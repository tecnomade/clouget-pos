use crate::db::{Database, SesionState};
use crate::models::{NuevaVenta, NuevaNotaCredito, NotaCreditoInfo, Venta, VentaCompleta, VentaDetalle, DocumentoReciente, ResumenGuias};
use tauri::State;

#[tauri::command]
pub fn registrar_venta(
    db: State<Database>,
    sesion: State<SesionState>,
    venta: NuevaVenta,
) -> Result<VentaCompleta, String> {
    // Verificar sesión activa
    let sesion_guard = sesion.sesion.lock().map_err(|e| e.to_string())?;
    let sesion_actual = sesion_guard
        .as_ref()
        .ok_or("Debe iniciar sesión para registrar ventas".to_string())?;
    let usuario_nombre = sesion_actual.nombre.clone();
    let usuario_id = sesion_actual.usuario_id;
    let es_admin = sesion_actual.rol == "ADMIN";
    drop(sesion_guard);

    let conn = db.conn.lock().map_err(|e| e.to_string())?;

    // Verificar que haya caja abierta y obtener su ID para vincular la venta a la sesion
    let caja_id_actual: Option<i64> = conn
        .query_row(
            "SELECT id FROM caja WHERE estado = 'ABIERTA' LIMIT 1",
            [],
            |row| row.get::<_, i64>(0),
        )
        .ok();

    if caja_id_actual.is_none() {
        return Err("Debe abrir la caja antes de realizar ventas".to_string());
    }

    // Validar stock segun config 'stock_negativo_modo'
    // PERMITIR (default): permite vender aunque deje stock < 0
    // BLOQUEAR | BLOQUEAR_OCULTAR: bloquea si la cantidad supera el stock disponible
    let stock_modo: String = conn
        .query_row("SELECT value FROM config WHERE key = 'stock_negativo_modo'", [], |r| r.get(0))
        .unwrap_or_else(|_| "PERMITIR".to_string());
    if stock_modo == "BLOQUEAR" || stock_modo == "BLOQUEAR_OCULTAR" {
        // Acumular cantidad necesaria por producto (sumar lineas duplicadas + factor unidad)
        let mut requerido: std::collections::HashMap<i64, f64> = std::collections::HashMap::new();
        for it in &venta.items {
            let factor = it.factor_unidad.unwrap_or(1.0);
            *requerido.entry(it.producto_id).or_insert(0.0) += it.cantidad * factor;
        }
        for (pid, cant_req) in &requerido {
            // Saltar productos sin control de stock o servicios
            let (stock_actual, es_serv, no_ctrl): (f64, bool, bool) = conn.query_row(
                "SELECT stock_actual, COALESCE(es_servicio,0), COALESCE(no_controla_stock,0) FROM productos WHERE id = ?1",
                rusqlite::params![pid],
                |r| Ok((r.get::<_, f64>(0)?, r.get::<_, i32>(1)? != 0, r.get::<_, i32>(2)? != 0)),
            ).unwrap_or((0.0, false, false));
            if es_serv || no_ctrl { continue; }
            if *cant_req > stock_actual + 1e-9 {
                let nombre: String = conn.query_row(
                    "SELECT nombre FROM productos WHERE id = ?1",
                    rusqlite::params![pid], |r| r.get(0)
                ).unwrap_or_else(|_| format!("ID {}", pid));
                return Err(format!(
                    "Stock insuficiente para '{}': requiere {:.2}, disponible {:.2}. Active 'Permitir stock negativo' en Configuracion o registre una compra.",
                    nombre, cant_req, stock_actual
                ));
            }
        }
    }

    // Leer establecimiento y punto de emisión del terminal
    let terminal_est: String = conn
        .query_row("SELECT value FROM config WHERE key = 'terminal_establecimiento'", [], |row| row.get(0))
        .unwrap_or_else(|_| "001".to_string());
    let terminal_pe: String = conn
        .query_row("SELECT value FROM config WHERE key = 'terminal_punto_emision'", [], |row| row.get(0))
        .unwrap_or_else(|_| "001".to_string());

    // Multi-almacén: obtener ID del establecimiento para descontar stock
    let multi_almacen: bool = conn
        .query_row("SELECT value FROM config WHERE key = 'multi_almacen_activo'", [], |row| row.get::<_, String>(0))
        .map(|v| v == "1")
        .unwrap_or(false);
    let est_id: Option<i64> = if multi_almacen {
        conn.query_row("SELECT id FROM establecimientos WHERE codigo = ?1", rusqlite::params![terminal_est], |row| row.get(0)).ok()
    } else {
        None
    };

    // Obtener secuencial interno de tabla secuenciales
    conn.execute(
        "INSERT OR IGNORE INTO secuenciales (establecimiento_codigo, punto_emision_codigo, tipo_documento, secuencial) VALUES (?1, ?2, 'NOTA_VENTA', 1)",
        rusqlite::params![terminal_est, terminal_pe],
    ).map_err(|e| e.to_string())?;

    let mut secuencial: i64 = conn
        .query_row(
            "SELECT secuencial FROM secuenciales WHERE establecimiento_codigo = ?1 AND punto_emision_codigo = ?2 AND tipo_documento = 'NOTA_VENTA'",
            rusqlite::params![terminal_est, terminal_pe],
            |row| row.get(0),
        )
        .map_err(|e| e.to_string())?;

    // Safety: si hay ventas existentes con números más altos (ej. desde demo),
    // saltar al siguiente número disponible para evitar UNIQUE constraint failed
    let max_existente: i64 = conn
        .query_row(
            "SELECT COALESCE(MAX(CAST(SUBSTR(numero, 4) AS INTEGER)), 0) FROM ventas WHERE numero LIKE 'NV-%'",
            [],
            |row| row.get(0),
        )
        .unwrap_or(0);
    if max_existente >= secuencial {
        secuencial = max_existente + 1;
        // Actualizar tabla para futuros secuenciales
        conn.execute(
            "UPDATE secuenciales SET secuencial = ?1 WHERE establecimiento_codigo = ?2 AND punto_emision_codigo = ?3 AND tipo_documento = 'NOTA_VENTA'",
            rusqlite::params![secuencial, terminal_est, terminal_pe],
        ).ok();
    }

    // Formato interno: NV-000000017
    let numero = format!("NV-{:09}", secuencial);

    // Calcular totales con redondeo a 2 decimales (consistencia con XSD del SRI)
    let r2 = |n: f64| (n * 100.0).round() / 100.0;
    let mut subtotal_sin_iva = 0.0_f64;
    let mut subtotal_con_iva = 0.0_f64;
    let mut iva_total = 0.0_f64;

    for item in &venta.items {
        let subtotal_item = r2(item.cantidad * item.precio_unitario - item.descuento);
        if item.iva_porcentaje > 0.0 {
            subtotal_con_iva += subtotal_item;
            iva_total += r2(subtotal_item * (item.iva_porcentaje / 100.0));
        } else {
            subtotal_sin_iva += subtotal_item;
        }
    }
    subtotal_sin_iva = r2(subtotal_sin_iva);
    subtotal_con_iva = r2(subtotal_con_iva);
    iva_total = r2(iva_total);

    let total = r2(subtotal_sin_iva + subtotal_con_iva + iva_total - venta.descuento);
    let cambio = if venta.monto_recibido > total {
        venta.monto_recibido - total
    } else {
        0.0
    };

    // v2.3.50 VALIDACION: anti-fraude — si la venta NO es fiada y NO usa pagos mixtos,
    // el monto_recibido debe cubrir el total. Antes el backend permitia ventas con
    // monto_recibido < total sin fiado, dejando "deudas fantasma" sin registro de CXC.
    let usa_mixto = venta.pagos.as_ref().map(|p| !p.is_empty()).unwrap_or(false);
    if !venta.es_fiado && !usa_mixto {
        // Para EFECTIVO y TARJETA exigimos cubrir el total. TRANSFER/CREDITO en single-method
        // pueden pasar (transfer = banco, no efectivo; credito = se asume fiado).
        let forma_up = venta.forma_pago.to_uppercase();
        if matches!(forma_up.as_str(), "EFECTIVO" | "TARJETA") {
            if venta.monto_recibido + 0.01 < total {
                return Err(format!(
                    "Monto recibido (${:.2}) es menor al total (${:.2}). Diferencia: ${:.2}. \
                     Si el cliente queda debiendo, marca la venta como CREDITO. \
                     Si paga con varios metodos, usa Pago Mixto.",
                    venta.monto_recibido, total, total - venta.monto_recibido
                ));
            }
        }
    }

    // Determinar estado_sri segun tipo de documento
    let estado_sri = match venta.tipo_documento.as_str() {
        "FACTURA" => "PENDIENTE",
        _ => "NO_APLICA",
    };

    // === Verificacion de transferencia (v2.3.33+) ===
    // Si la venta es TRANSFER (puro o como parte de un MIXTO), determinar el pago_estado:
    //  - Si el usuario es ADMIN: 'VERIFICADO' automaticamente (admin se valida a si mismo)
    //  - Si es cajero:           'REGISTRADO' (queda pendiente para que admin revise despues)
    // Si NO es transferencia: 'NO_APLICA' (no requiere verificacion)
    let es_transfer_venta = matches!(venta.forma_pago.to_uppercase().as_str(), "TRANSFER" | "TRANSFERENCIA");
    let es_transfer_mixto = venta.pagos.as_ref()
        .map(|pgs| pgs.iter().any(|p| matches!(p.forma_pago.to_uppercase().as_str(), "TRANSFER" | "TRANSFERENCIA")))
        .unwrap_or(false);
    let requiere_verificacion = es_transfer_venta || es_transfer_mixto;
    let pago_estado_inicial: &str = if !requiere_verificacion { "NO_APLICA" }
        else if es_admin { "VERIFICADO" }
        else { "REGISTRADO" };
    let verificado_por_inicial: Option<i64> = if pago_estado_inicial == "VERIFICADO" { Some(usuario_id) } else { None };
    let fecha_verificacion_inicial: Option<String> = if pago_estado_inicial == "VERIFICADO" {
        Some(chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string())
    } else { None };

    // Insertar cabecera de venta
    conn.execute(
        "INSERT INTO ventas (numero, cliente_id, subtotal_sin_iva, subtotal_con_iva,
         descuento, iva, total, forma_pago, monto_recibido, cambio, estado,
         tipo_documento, estado_sri, observacion, usuario, usuario_id, establecimiento, punto_emision,
         banco_id, referencia_pago, comprobante_imagen,
         pago_estado, verificado_por, fecha_verificacion, caja_id)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21, ?22, ?23, ?24, ?25)",
        rusqlite::params![
            numero,
            venta.cliente_id.unwrap_or(1),
            subtotal_sin_iva,
            subtotal_con_iva,
            venta.descuento,
            iva_total,
            total,
            venta.forma_pago,
            venta.monto_recibido,
            cambio,
            "COMPLETADA",
            venta.tipo_documento,
            estado_sri,
            venta.observacion,
            usuario_nombre,
            usuario_id,
            terminal_est,
            terminal_pe,
            venta.banco_id,
            venta.referencia_pago,
            venta.comprobante_imagen,
            pago_estado_inicial,
            verificado_por_inicial,
            fecha_verificacion_inicial,
            caja_id_actual,
        ],
    )
    .map_err(|e| e.to_string())?;

    let venta_id = conn.last_insert_rowid();

    // Insertar detalles y actualizar stock
    let mut detalles_guardados = Vec::new();
    for item in &venta.items {
        let subtotal = r2(item.cantidad * item.precio_unitario - item.descuento);

        // Obtener precio_costo del producto para snapshot en venta_detalles
        let precio_costo_prod: f64 = conn
            .query_row(
                "SELECT precio_costo FROM productos WHERE id = ?1",
                rusqlite::params![item.producto_id],
                |row| row.get(0),
            )
            .unwrap_or(0.0);

        // Multi-unidad: factor de conversion (default 1 = unidad base)
        let factor_unidad = item.factor_unidad.unwrap_or(1.0);
        let cantidad_base = item.cantidad * factor_unidad; // cantidad real a descontar del stock

        // Lote de caducidad (v2.2.0): si producto requiere caducidad y no viene lote_id,
        // aplicar FEFO automatico (el que vence primero con stock disponible)
        let requiere_caducidad: bool = conn.query_row(
            "SELECT COALESCE(requiere_caducidad, 0) FROM productos WHERE id = ?1",
            rusqlite::params![item.producto_id],
            |row| row.get::<_, i32>(0).map(|v| v != 0)
        ).unwrap_or(false);

        let lote_id_final: Option<i64> = if let Some(lid) = item.lote_id {
            // Validar que el lote tenga stock suficiente — anti-fraude/error UX
            let stock_lote: f64 = conn.query_row(
                "SELECT cantidad FROM lotes_caducidad WHERE id = ?1 AND producto_id = ?2",
                rusqlite::params![lid, item.producto_id],
                |r| r.get(0),
            ).unwrap_or(0.0);
            if cantidad_base > stock_lote + 1e-9 {
                let nombre_prod: String = conn.query_row(
                    "SELECT nombre FROM productos WHERE id = ?1",
                    rusqlite::params![item.producto_id], |r| r.get(0)
                ).unwrap_or_else(|_| format!("ID {}", item.producto_id));
                return Err(format!(
                    "El lote #{} de '{}' solo tiene {:.2} unidades. Intentas vender {:.2}. Reduce la cantidad o agrega otro lote/sin lote.",
                    lid, nombre_prod, stock_lote, cantidad_base
                ));
            }
            Some(lid)
        } else if requiere_caducidad {
            // Auto FEFO: lote con fecha_caducidad mas proxima que tenga stock
            conn.query_row(
                "SELECT id FROM lotes_caducidad WHERE producto_id = ?1 AND cantidad >= ?2
                 ORDER BY fecha_caducidad ASC LIMIT 1",
                rusqlite::params![item.producto_id, cantidad_base],
                |row| row.get(0)
            ).ok()
        } else {
            None
        };

        conn.execute(
            "INSERT INTO venta_detalles (venta_id, producto_id, cantidad, precio_unitario,
             descuento, iva_porcentaje, subtotal, info_adicional, precio_costo,
             unidad_id, unidad_nombre, factor_unidad, lote_id)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
            rusqlite::params![
                venta_id,
                item.producto_id,
                item.cantidad,
                item.precio_unitario,
                item.descuento,
                item.iva_porcentaje,
                subtotal,
                item.info_adicional,
                precio_costo_prod,
                item.unidad_id,
                item.unidad_nombre,
                factor_unidad,
                lote_id_final,
            ],
        )
        .map_err(|e| e.to_string())?;

        // Descontar del lote (si aplica)
        if let Some(lid) = lote_id_final {
            conn.execute(
                "UPDATE lotes_caducidad SET cantidad = MAX(cantidad - ?1, 0) WHERE id = ?2",
                rusqlite::params![cantidad_base, lid],
            ).ok();
        }

        // Obtener stock antes de descontar y verificar si es servicio / no_controla_stock / combo
        let (stock_antes, es_servicio, no_controla_stock, tipo_producto): (f64, bool, bool, String) = conn
            .query_row(
                "SELECT stock_actual, es_servicio, COALESCE(no_controla_stock, 0), COALESCE(tipo_producto, 'SIMPLE') FROM productos WHERE id = ?1",
                rusqlite::params![item.producto_id],
                |row| Ok((row.get::<_, f64>(0)?, row.get::<_, bool>(1)?, row.get::<_, i32>(2)? != 0, row.get::<_, String>(3)?)),
            )
            .unwrap_or((0.0, false, false, "SIMPLE".to_string()));

        let es_combo = tipo_producto == "COMBO_FIJO" || tipo_producto == "COMBO_FLEXIBLE";
        // Los combos no descuentan stock del padre — se gestiona via componentes mas abajo
        let omite_stock = es_servicio || no_controla_stock || es_combo;

        // Descontar stock (cantidad_base = cantidad x factor de la unidad de venta)
        if !omite_stock {
            conn.execute(
                "UPDATE productos SET stock_actual = stock_actual - ?1,
                 updated_at = datetime('now','localtime')
                 WHERE id = ?2",
                rusqlite::params![cantidad_base, item.producto_id],
            )
            .map_err(|e| e.to_string())?;
        }

        // Multi-almacén: también descontar de stock_establecimiento
        if let Some(eid) = est_id {
            if !omite_stock {
                conn.execute(
                    "UPDATE stock_establecimiento SET stock_actual = stock_actual - ?1
                     WHERE producto_id = ?2 AND establecimiento_id = ?3",
                    rusqlite::params![cantidad_base, item.producto_id, eid],
                ).ok();
            }
        }

        // Registrar movimiento de inventario (kardex) para productos físicos
        // costo_unitario = precio_costo snapshot del momento de la venta (NO precio de venta)
        if !omite_stock {
            let _ = conn.execute(
                "INSERT INTO movimientos_inventario (producto_id, tipo, cantidad, stock_anterior, stock_nuevo, costo_unitario, referencia_id, usuario, establecimiento_id)
                 VALUES (?1, 'VENTA', ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                rusqlite::params![
                    item.producto_id,
                    -(cantidad_base),
                    stock_antes,
                    stock_antes - cantidad_base,
                    precio_costo_prod,
                    venta_id,
                    usuario_nombre,
                    est_id,
                ],
            );
        }

        // Obtener nombre del producto
        let nombre_prod: String = conn
            .query_row(
                "SELECT nombre FROM productos WHERE id = ?1",
                rusqlite::params![item.producto_id],
                |row| row.get(0),
            )
            .unwrap_or_default();

        let detalle_id = conn.last_insert_rowid();

        // === COMBOS: descontar stock de componentes ===
        // Para COMBO_FIJO: usa la definicion guardada en producto_componentes
        // Para COMBO_FLEXIBLE: usa item.combo_seleccion (lo que el cajero escogio)
        if es_combo {
            let mut componentes_a_descontar: Vec<(i64, f64)> = Vec::new();
            if tipo_producto == "COMBO_FIJO" {
                let mut stmt = conn.prepare(
                    "SELECT producto_hijo_id, cantidad FROM producto_componentes WHERE producto_padre_id = ?1"
                ).map_err(|e| e.to_string())?;
                let rows = stmt.query_map(rusqlite::params![item.producto_id], |r| Ok((r.get::<_, i64>(0)?, r.get::<_, f64>(1)?)))
                    .map_err(|e| e.to_string())?;
                for row in rows {
                    let (hijo_id, cant) = row.map_err(|e| e.to_string())?;
                    // Multiplica por la cantidad de combos vendidos
                    componentes_a_descontar.push((hijo_id, cant * item.cantidad));
                }
            } else {
                // COMBO_FLEXIBLE: requiere combo_seleccion
                if let Some(sel) = &item.combo_seleccion {
                    for c in sel {
                        if c.cantidad > 0.0 {
                            componentes_a_descontar.push((c.producto_hijo_id, c.cantidad * item.cantidad));
                        }
                    }
                }
                if componentes_a_descontar.is_empty() {
                    return Err(format!("Combo flexible '{}' sin seleccion de componentes", nombre_prod));
                }
            }

            for (hijo_id, cant_total) in componentes_a_descontar {
                // Stock anterior del hijo + flags
                let (stock_h_antes, es_serv_h, no_ctrl_h, costo_h): (f64, bool, bool, f64) = conn.query_row(
                    "SELECT stock_actual, COALESCE(es_servicio,0), COALESCE(no_controla_stock,0), precio_costo
                     FROM productos WHERE id = ?1",
                    rusqlite::params![hijo_id],
                    |r| Ok((r.get::<_, f64>(0)?, r.get::<_, i32>(1)? != 0, r.get::<_, i32>(2)? != 0, r.get::<_, f64>(3)?)),
                ).unwrap_or((0.0, false, false, 0.0));

                let omite_h = es_serv_h || no_ctrl_h;

                if !omite_h {
                    conn.execute(
                        "UPDATE productos SET stock_actual = stock_actual - ?1, updated_at = datetime('now','localtime') WHERE id = ?2",
                        rusqlite::params![cant_total, hijo_id],
                    ).map_err(|e| e.to_string())?;
                    if let Some(eid) = est_id {
                        conn.execute(
                            "UPDATE stock_establecimiento SET stock_actual = stock_actual - ?1
                             WHERE producto_id = ?2 AND establecimiento_id = ?3",
                            rusqlite::params![cant_total, hijo_id, eid],
                        ).ok();
                    }
                    let _ = conn.execute(
                        "INSERT INTO movimientos_inventario (producto_id, tipo, cantidad, stock_anterior, stock_nuevo, costo_unitario, referencia_id, usuario, establecimiento_id)
                         VALUES (?1, 'VENTA_COMBO', ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                        rusqlite::params![hijo_id, -cant_total, stock_h_antes, stock_h_antes - cant_total, costo_h, venta_id, usuario_nombre, est_id],
                    );
                }

                // Registrar la entrega del componente
                let _ = conn.execute(
                    "INSERT INTO venta_detalle_combo (venta_detalle_id, producto_hijo_id, cantidad, grupo_id) VALUES (?1, ?2, ?3, NULL)",
                    rusqlite::params![detalle_id, hijo_id, cant_total],
                );
            }
        }

        detalles_guardados.push(VentaDetalle {
            id: Some(detalle_id),
            venta_id: Some(venta_id),
            producto_id: item.producto_id,
            nombre_producto: Some(nombre_prod),
            cantidad: item.cantidad,
            precio_unitario: item.precio_unitario,
            descuento: item.descuento,
            iva_porcentaje: item.iva_porcentaje,
            subtotal,
            info_adicional: item.info_adicional.clone(),
            unidad_id: None, unidad_nombre: None, factor_unidad: None, lote_id: None, combo_seleccion: None,
        });
    }

    // Actualizar secuencial interno en tabla secuenciales
    conn.execute(
        "UPDATE secuenciales SET secuencial = secuencial + 1 WHERE establecimiento_codigo = ?1 AND punto_emision_codigo = ?2 AND tipo_documento = 'NOTA_VENTA'",
        rusqlite::params![terminal_est, terminal_pe],
    )
    .map_err(|e| e.to_string())?;

    // Actualizar monto de caja si hay una abierta.
    // - monto_ventas suma TODAS las ventas (para reportes/dashboard).
    // - monto_esperado solo suma la porcion EFECTIVO (lo que entra a caja fisica).
    //   TRANSFER, CREDITO, etc. NO afectan el efectivo en caja.
    let efectivo_de_esta_venta: f64 = if let Some(ref pagos) = venta.pagos {
        // Pagos mixtos: sumar solo los EFECTIVO
        pagos.iter().filter(|p| p.forma_pago == "EFECTIVO").map(|p| p.monto).sum()
    } else if venta.forma_pago == "EFECTIVO" && !venta.es_fiado {
        total
    } else {
        0.0
    };
    conn.execute(
        "UPDATE caja SET monto_ventas = monto_ventas + ?1,
         monto_esperado = monto_esperado + ?2
         WHERE estado = 'ABIERTA'",
        rusqlite::params![total, efectivo_de_esta_venta],
    )
    .ok();

    // Si es fiado, crear cuenta por cobrar (legacy: forma_pago unica = CREDITO)
    if venta.es_fiado {
        conn.execute(
            "INSERT INTO cuentas_por_cobrar (cliente_id, venta_id, monto_total, saldo, estado)
             VALUES (?1, ?2, ?3, ?3, 'PENDIENTE')",
            rusqlite::params![venta.cliente_id.unwrap_or(1), venta_id, total],
        )
        .map_err(|e| e.to_string())?;
    }

    // --- PAGO MIXTO ---
    // Si el frontend mando un array de pagos, los registramos individualmente.
    // Tambien si hay un pago tipo CREDITO, creamos la cuenta por cobrar parcial.
    if let Some(pagos) = &venta.pagos {
        if !pagos.is_empty() {
            // Validar que la suma de pagos sea igual al total (con tolerancia 0.01)
            let suma: f64 = pagos.iter().map(|p| p.monto).sum();
            if (suma - total).abs() > 0.02 {
                return Err(format!("La suma de pagos (${:.2}) no coincide con el total (${:.2})", suma, total));
            }

            // Insertar cada pago en pagos_venta. Para componentes TRANSFER, marcar el
            // estado de verificacion segun rol del usuario (igual que la venta misma).
            for p in pagos {
                let pf = p.forma_pago.to_uppercase();
                let es_pago_transfer = matches!(pf.as_str(), "TRANSFER" | "TRANSFERENCIA");
                let p_estado: &str = if !es_pago_transfer { "NO_APLICA" }
                    else if es_admin { "VERIFICADO" }
                    else { "REGISTRADO" };
                let p_verif_por: Option<i64> = if p_estado == "VERIFICADO" { Some(usuario_id) } else { None };
                let p_verif_fecha: Option<String> = if p_estado == "VERIFICADO" {
                    Some(chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string())
                } else { None };
                conn.execute(
                    "INSERT INTO pagos_venta (venta_id, forma_pago, monto, banco_id, referencia, comprobante_imagen,
                                              pago_estado, verificado_por, fecha_verificacion)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                    rusqlite::params![venta_id, p.forma_pago, p.monto, p.banco_id, p.referencia, p.comprobante_imagen,
                                      p_estado, p_verif_por, p_verif_fecha],
                ).map_err(|e| format!("Error guardando pago: {}", e))?;
            }

            // Si hay un pago tipo CREDITO y no se creo CXC arriba (es_fiado=false), crearla por el monto credito
            if !venta.es_fiado {
                let monto_credito: f64 = pagos.iter()
                    .filter(|p| p.forma_pago.eq_ignore_ascii_case("CREDITO"))
                    .map(|p| p.monto).sum();
                if monto_credito > 0.01 {
                    conn.execute(
                        "INSERT INTO cuentas_por_cobrar (cliente_id, venta_id, monto_total, saldo, estado)
                         VALUES (?1, ?2, ?3, ?3, 'PENDIENTE')",
                        rusqlite::params![venta.cliente_id.unwrap_or(1), venta_id, monto_credito],
                    ).map_err(|e| e.to_string())?;
                }
            }

            // Si hay mas de 1 pago o el unico pago es distinto al forma_pago de la venta, marcar la venta como MIXTO
            let formas_unicas: std::collections::HashSet<String> = pagos.iter().map(|p| p.forma_pago.to_uppercase()).collect();
            if formas_unicas.len() > 1 {
                conn.execute(
                    "UPDATE ventas SET forma_pago = 'MIXTO' WHERE id = ?1",
                    rusqlite::params![venta_id],
                ).ok();
            } else if let Some(unica) = formas_unicas.iter().next() {
                // Solo una forma → usar esa
                conn.execute(
                    "UPDATE ventas SET forma_pago = ?1 WHERE id = ?2",
                    rusqlite::params![unica, venta_id],
                ).ok();
            }
        }
    }

    // Obtener nombre del cliente
    let cliente_nombre: Option<String> = conn
        .query_row(
            "SELECT nombre FROM clientes WHERE id = ?1",
            rusqlite::params![venta.cliente_id.unwrap_or(1)],
            |row| row.get(0),
        )
        .ok();

    Ok(VentaCompleta {
        venta: Venta {
            id: Some(venta_id),
            numero: numero.clone(),
            cliente_id: venta.cliente_id,
            fecha: None,
            subtotal_sin_iva,
            subtotal_con_iva,
            descuento: venta.descuento,
            iva: iva_total,
            total,
            forma_pago: venta.forma_pago,
            monto_recibido: venta.monto_recibido,
            cambio,
            estado: "COMPLETADA".to_string(),
            tipo_documento: venta.tipo_documento,
            estado_sri: estado_sri.to_string(),
            autorizacion_sri: None,
            clave_acceso: None,
            observacion: venta.observacion,
            numero_factura: None,
            establecimiento: Some(terminal_est),
            punto_emision: Some(terminal_pe),
            banco_id: None,
            referencia_pago: None,
            banco_nombre: None,
            comprobante_imagen: None,
            caja_id: None,
            cliente_nombre: None,
            tipo_estado: None,
            guia_placa: None, guia_chofer: None, guia_direccion_destino: None,
                anulada: None,
        },
        detalles: detalles_guardados,
        cliente_nombre,
    })
}

/// Lista todos los pagos asociados a una venta (pago mixto)
#[tauri::command]
pub fn listar_pagos_venta(db: State<Database>, venta_id: i64) -> Result<Vec<serde_json::Value>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let mut stmt = conn.prepare(
        "SELECT pv.id, pv.forma_pago, pv.monto, pv.banco_id, pv.referencia, pv.comprobante_imagen, cb.nombre as banco_nombre
         FROM pagos_venta pv
         LEFT JOIN cuentas_banco cb ON pv.banco_id = cb.id
         WHERE pv.venta_id = ?1
         ORDER BY pv.id"
    ).map_err(|e| e.to_string())?;

    let pagos = stmt.query_map(rusqlite::params![venta_id], |row| {
        Ok(serde_json::json!({
            "id": row.get::<_, i64>(0)?,
            "forma_pago": row.get::<_, String>(1)?,
            "monto": row.get::<_, f64>(2)?,
            "banco_id": row.get::<_, Option<i64>>(3)?,
            "referencia": row.get::<_, Option<String>>(4)?,
            "comprobante_imagen": row.get::<_, Option<String>>(5)?,
            "banco_nombre": row.get::<_, Option<String>>(6)?,
        }))
    }).map_err(|e| e.to_string())?
    .collect::<Result<Vec<_>, _>>()
    .map_err(|e| e.to_string())?;

    Ok(pagos)
}

#[tauri::command]
pub fn listar_ventas_dia(db: State<Database>, fecha: String) -> Result<Vec<Venta>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;

    let mut stmt = conn
        .prepare(
            "SELECT v.id, v.numero, v.cliente_id, v.fecha, v.subtotal_sin_iva, v.subtotal_con_iva,
             v.descuento, v.iva, v.total, v.forma_pago, v.monto_recibido, v.cambio, v.estado,
             v.tipo_documento, v.estado_sri, v.autorizacion_sri, v.clave_acceso, v.observacion,
             v.numero_factura, v.establecimiento, v.punto_emision,
             v.banco_id, v.referencia_pago, cb.nombre as banco_nombre,
             COALESCE(v.tipo_estado, 'COMPLETADA') as tipo_estado,
             COALESCE(v.anulada, 0) as anulada,
             v.caja_id
             FROM ventas v
             LEFT JOIN cuentas_banco cb ON v.banco_id = cb.id
             WHERE date(v.fecha) = date(?1)
             ORDER BY v.anulada ASC, v.fecha DESC",
        )
        .map_err(|e| e.to_string())?;

    let ventas = stmt
        .query_map(rusqlite::params![fecha], |row| {
            Ok(Venta {
                id: Some(row.get(0)?),
                numero: row.get(1)?,
                cliente_id: row.get(2)?,
                fecha: row.get(3)?,
                subtotal_sin_iva: row.get(4)?,
                subtotal_con_iva: row.get(5)?,
                descuento: row.get(6)?,
                iva: row.get(7)?,
                total: row.get(8)?,
                forma_pago: row.get(9)?,
                monto_recibido: row.get(10)?,
                cambio: row.get(11)?,
                estado: row.get(12)?,
                tipo_documento: row.get(13)?,
                estado_sri: row.get::<_, String>(14).unwrap_or_else(|_| "NO_APLICA".to_string()),
                autorizacion_sri: row.get(15)?,
                clave_acceso: row.get(16)?,
                observacion: row.get(17)?,
                numero_factura: row.get(18)?,
                establecimiento: row.get(19).ok(),
                punto_emision: row.get(20).ok(),
                banco_id: row.get(21).ok(),
                referencia_pago: row.get(22).ok(),
                banco_nombre: row.get(23).ok(),
                comprobante_imagen: None,
                tipo_estado: row.get(24).ok(),
                anulada: row.get::<_, i64>(25).ok(),
                caja_id: row.get(26).ok(),
                cliente_nombre: None,
                guia_placa: None, guia_chofer: None, guia_direccion_destino: None,
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    Ok(ventas)
}

#[tauri::command]
pub fn obtener_venta(db: State<Database>, id: i64) -> Result<VentaCompleta, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;

    let venta = conn
        .query_row(
            "SELECT v.id, v.numero, v.cliente_id, v.fecha, v.subtotal_sin_iva, v.subtotal_con_iva,
             v.descuento, v.iva, v.total, v.forma_pago, v.monto_recibido, v.cambio, v.estado,
             v.tipo_documento, v.estado_sri, v.autorizacion_sri, v.clave_acceso, v.observacion,
             v.numero_factura, v.establecimiento, v.punto_emision,
             v.banco_id, v.referencia_pago, cb.nombre as banco_nombre, v.comprobante_imagen,
             v.caja_id
             FROM ventas v
             LEFT JOIN cuentas_banco cb ON v.banco_id = cb.id
             WHERE v.id = ?1",
            rusqlite::params![id],
            |row| {
                Ok(Venta {
                    id: Some(row.get(0)?),
                    numero: row.get(1)?,
                    cliente_id: row.get(2)?,
                    fecha: row.get(3)?,
                    subtotal_sin_iva: row.get(4)?,
                    subtotal_con_iva: row.get(5)?,
                    descuento: row.get(6)?,
                    iva: row.get(7)?,
                    total: row.get(8)?,
                    forma_pago: row.get(9)?,
                    monto_recibido: row.get(10)?,
                    cambio: row.get(11)?,
                    estado: row.get(12)?,
                    tipo_documento: row.get(13)?,
                    estado_sri: row.get::<_, String>(14).unwrap_or_else(|_| "NO_APLICA".to_string()),
                    autorizacion_sri: row.get(15)?,
                    clave_acceso: row.get(16)?,
                    observacion: row.get(17)?,
                    numero_factura: row.get(18)?,
                    establecimiento: row.get(19).ok(),
                    punto_emision: row.get(20).ok(),
                    banco_id: row.get(21).ok(),
                    referencia_pago: row.get(22).ok(),
                    banco_nombre: row.get(23).ok(),
                    comprobante_imagen: row.get(24).ok(),
                    caja_id: row.get(25).ok(),
                    cliente_nombre: None,
                    tipo_estado: None,
                    anulada: None,
                    guia_placa: None, guia_chofer: None, guia_direccion_destino: None,
                })
            },
        )
        .map_err(|e| e.to_string())?;

    let mut stmt = conn
        .prepare(
            "SELECT d.id, d.venta_id, d.producto_id, p.nombre, d.cantidad,
             d.precio_unitario, d.descuento, d.iva_porcentaje, d.subtotal, d.info_adicional
             FROM venta_detalles d
             JOIN productos p ON d.producto_id = p.id
             WHERE d.venta_id = ?1",
        )
        .map_err(|e| e.to_string())?;

    let detalles = stmt
        .query_map(rusqlite::params![id], |row| {
            Ok(VentaDetalle {
                id: Some(row.get(0)?),
                venta_id: Some(row.get(1)?),
                producto_id: row.get(2)?,
                nombre_producto: Some(row.get(3)?),
                cantidad: row.get(4)?,
                precio_unitario: row.get(5)?,
                descuento: row.get(6)?,
                iva_porcentaje: row.get(7)?,
                subtotal: row.get(8)?,
                info_adicional: row.get(9).ok(),
            unidad_id: None, unidad_nombre: None, factor_unidad: None, lote_id: None, combo_seleccion: None,
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    let cliente_nombre: Option<String> = venta.cliente_id.and_then(|cid| {
        conn.query_row(
            "SELECT nombre FROM clientes WHERE id = ?1",
            rusqlite::params![cid],
            |row| row.get(0),
        )
        .ok()
    });

    Ok(VentaCompleta {
        venta,
        detalles,
        cliente_nombre,
    })
}

// --- Ventas filtradas por sesión de caja (para cajeros) ---

#[tauri::command]
pub fn listar_ventas_sesion_caja(
    db: State<Database>,
    sesion: State<SesionState>,
) -> Result<Vec<Venta>, String> {
    let sesion_guard = sesion.sesion.lock().map_err(|e| e.to_string())?;
    let sesion_actual = sesion_guard
        .as_ref()
        .ok_or("Debe iniciar sesión".to_string())?;
    let usuario_id = sesion_actual.usuario_id;
    drop(sesion_guard);

    let conn = db.conn.lock().map_err(|e| e.to_string())?;

    // Buscar la caja abierta de este usuario (o la más reciente cerrada hoy).
    // Si no hay ninguna, fallback a "ventas del dia hoy del usuario" para que el
    // cajero NUNCA vea pantalla en blanco — incluso si aun no abrio caja, debe
    // poder revisar sus ventas anteriores del dia.
    let fecha_apertura: String = conn
        .query_row(
            "SELECT fecha_apertura FROM caja
             WHERE usuario_id = ?1 AND (estado = 'ABIERTA' OR date(fecha_apertura) = date('now','localtime'))
             ORDER BY id DESC LIMIT 1",
            rusqlite::params![usuario_id],
            |row| row.get(0),
        )
        .unwrap_or_else(|_| {
            // Fallback: inicio del dia de hoy para mostrar ventas del dia del cajero
            chrono::Local::now().format("%Y-%m-%d 00:00:00").to_string()
        });

    let mut stmt = conn
        .prepare(
            "SELECT v.id, v.numero, v.cliente_id, v.fecha, v.subtotal_sin_iva, v.subtotal_con_iva,
             v.descuento, v.iva, v.total, v.forma_pago, v.monto_recibido, v.cambio, v.estado,
             v.tipo_documento, v.estado_sri, v.autorizacion_sri, v.clave_acceso, v.observacion,
             v.numero_factura, v.establecimiento, v.punto_emision,
             v.banco_id, v.referencia_pago, cb.nombre as banco_nombre,
             COALESCE(v.tipo_estado, 'COMPLETADA') as tipo_estado
             FROM ventas v
             LEFT JOIN cuentas_banco cb ON v.banco_id = cb.id
             WHERE v.fecha >= ?1 AND v.anulada = 0
             ORDER BY v.fecha DESC",
        )
        .map_err(|e| e.to_string())?;

    let ventas = stmt
        .query_map(rusqlite::params![fecha_apertura], |row| {
            Ok(Venta {
                id: Some(row.get(0)?),
                numero: row.get(1)?,
                cliente_id: row.get(2)?,
                fecha: row.get(3)?,
                subtotal_sin_iva: row.get(4)?,
                subtotal_con_iva: row.get(5)?,
                descuento: row.get(6)?,
                iva: row.get(7)?,
                total: row.get(8)?,
                forma_pago: row.get(9)?,
                monto_recibido: row.get(10)?,
                cambio: row.get(11)?,
                estado: row.get(12)?,
                tipo_documento: row.get(13)?,
                estado_sri: row.get::<_, String>(14).unwrap_or_else(|_| "NO_APLICA".to_string()),
                autorizacion_sri: row.get(15)?,
                clave_acceso: row.get(16)?,
                observacion: row.get(17)?,
                numero_factura: row.get(18)?,
                establecimiento: row.get(19).ok(),
                punto_emision: row.get(20).ok(),
                banco_id: row.get(21).ok(),
                referencia_pago: row.get(22).ok(),
                banco_nombre: row.get(23).ok(),
                comprobante_imagen: None,
                caja_id: None,
                cliente_nombre: None,
                tipo_estado: row.get(24).ok(),
                guia_placa: None, guia_chofer: None, guia_direccion_destino: None,
                anulada: None,
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    Ok(ventas)
}

#[tauri::command]
pub fn resumen_sesion_caja(
    db: State<Database>,
    sesion: State<SesionState>,
) -> Result<crate::commands::reportes::ResumenDiario, String> {
    let sesion_guard = sesion.sesion.lock().map_err(|e| e.to_string())?;
    let sesion_actual = sesion_guard
        .as_ref()
        .ok_or("Debe iniciar sesión".to_string())?;
    let usuario_id = sesion_actual.usuario_id;
    drop(sesion_guard);

    let conn = db.conn.lock().map_err(|e| e.to_string())?;

    let fecha_apertura: String = conn
        .query_row(
            "SELECT fecha_apertura FROM caja
             WHERE usuario_id = ?1 AND (estado = 'ABIERTA' OR date(fecha_apertura) = date('now','localtime'))
             ORDER BY id DESC LIMIT 1",
            rusqlite::params![usuario_id],
            |row| row.get(0),
        )
        .unwrap_or_else(|_| {
            chrono::Local::now().format("%Y-%m-%d 00:00:00").to_string()
        });

    let total_ventas: f64 = conn
        .query_row(
            "SELECT COALESCE(SUM(total), 0) FROM ventas WHERE fecha >= ?1 AND anulada = 0",
            rusqlite::params![fecha_apertura],
            |row| row.get(0),
        )
        .unwrap_or(0.0);

    let num_ventas: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM ventas WHERE fecha >= ?1 AND anulada = 0",
            rusqlite::params![fecha_apertura],
            |row| row.get(0),
        )
        .unwrap_or(0);

    let total_efectivo: f64 = conn
        .query_row(
            "SELECT COALESCE(SUM(total), 0) FROM ventas WHERE fecha >= ?1 AND forma_pago = 'EFECTIVO' AND anulada = 0",
            rusqlite::params![fecha_apertura],
            |row| row.get(0),
        )
        .unwrap_or(0.0);

    let total_transferencia: f64 = conn
        .query_row(
            "SELECT COALESCE(SUM(total), 0) FROM ventas WHERE fecha >= ?1 AND forma_pago = 'TRANSFER' AND anulada = 0",
            rusqlite::params![fecha_apertura],
            |row| row.get(0),
        )
        .unwrap_or(0.0);

    let total_fiado: f64 = conn
        .query_row(
            "SELECT COALESCE(SUM(monto_total), 0) FROM cuentas_por_cobrar WHERE created_at >= ?1",
            rusqlite::params![fecha_apertura],
            |row| row.get(0),
        )
        .unwrap_or(0.0);

    let utilidad_bruta: f64 = conn
        .query_row(
            "SELECT COALESCE(SUM(vd.subtotal - (CASE WHEN COALESCE(vd.precio_costo, 0) > 0 THEN vd.precio_costo ELSE p.precio_costo END * vd.cantidad)), 0)
             FROM venta_detalles vd
             JOIN ventas v ON vd.venta_id = v.id
             JOIN productos p ON vd.producto_id = p.id
             WHERE v.fecha >= ?1 AND v.anulada = 0",
            rusqlite::params![fecha_apertura],
            |row| row.get(0),
        )
        .unwrap_or(0.0);

    let total_notas_credito: f64 = conn
        .query_row(
            "SELECT COALESCE(SUM(total), 0) FROM notas_credito WHERE fecha >= ?1",
            rusqlite::params![fecha_apertura],
            |row| row.get(0),
        )
        .unwrap_or(0.0);

    let num_notas_credito: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM notas_credito WHERE fecha >= ?1",
            rusqlite::params![fecha_apertura],
            |row| row.get(0),
        )
        .unwrap_or(0);

    Ok(crate::commands::reportes::ResumenDiario {
        total_ventas,
        num_ventas,
        total_efectivo,
        total_transferencia,
        total_fiado,
        utilidad_bruta,
        total_notas_credito,
        num_notas_credito,
    })
}

#[tauri::command]
pub fn listar_notas_credito_sesion_caja(
    db: State<Database>,
    sesion: State<SesionState>,
) -> Result<Vec<NotaCreditoInfo>, String> {
    let sesion_guard = sesion.sesion.lock().map_err(|e| e.to_string())?;
    let sesion_actual = sesion_guard
        .as_ref()
        .ok_or("Debe iniciar sesión".to_string())?;
    let usuario_id = sesion_actual.usuario_id;
    drop(sesion_guard);

    let conn = db.conn.lock().map_err(|e| e.to_string())?;

    let fecha_apertura: String = conn
        .query_row(
            "SELECT fecha_apertura FROM caja
             WHERE usuario_id = ?1 AND (estado = 'ABIERTA' OR date(fecha_apertura) = date('now','localtime'))
             ORDER BY id DESC LIMIT 1",
            rusqlite::params![usuario_id],
            |row| row.get(0),
        )
        .unwrap_or_else(|_| {
            // Fallback: inicio del dia hoy si no hay caja
            chrono::Local::now().format("%Y-%m-%d 00:00:00").to_string()
        });

    let mut stmt = conn
        .prepare(
            "SELECT nc.id, nc.numero, nc.venta_id, COALESCE(v.numero_factura, v.numero), nc.motivo,
             nc.total, nc.fecha, nc.estado_sri, nc.autorizacion_sri, nc.clave_acceso, nc.numero_factura_nc
             FROM notas_credito nc
             JOIN ventas v ON nc.venta_id = v.id
             WHERE nc.fecha >= ?1
             ORDER BY nc.fecha DESC",
        )
        .map_err(|e| e.to_string())?;

    let notas = stmt
        .query_map(rusqlite::params![fecha_apertura], |row| {
            Ok(NotaCreditoInfo {
                id: row.get(0)?,
                numero: row.get(1)?,
                venta_id: row.get(2)?,
                factura_numero: row.get(3)?,
                motivo: row.get(4)?,
                total: row.get(5)?,
                fecha: row.get(6)?,
                estado_sri: row.get::<_, String>(7).unwrap_or_else(|_| "PENDIENTE".to_string()),
                autorizacion_sri: row.get(8)?,
                clave_acceso: row.get(9)?,
                numero_factura_nc: row.get(10)?,
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    Ok(notas)
}

// --- Notas de Crédito ---

#[tauri::command]
pub fn registrar_nota_credito(
    db: State<Database>,
    sesion: State<SesionState>,
    nota: NuevaNotaCredito,
) -> Result<NotaCreditoInfo, String> {
    let sesion_guard = sesion.sesion.lock().map_err(|e| e.to_string())?;
    let sesion_actual = sesion_guard
        .as_ref()
        .ok_or("Debe iniciar sesión".to_string())?;
    let usuario_nombre = sesion_actual.nombre.clone();
    let usuario_id = sesion_actual.usuario_id;
    let usuario_rol = sesion_actual.rol.clone();
    let usuario_permisos = sesion_actual.permisos.clone();
    drop(sesion_guard);

    // Verificar permiso: admin o tiene crear_nota_credito
    if usuario_rol != "ADMIN" {
        let tiene_permiso = serde_json::from_str::<serde_json::Value>(&usuario_permisos)
            .ok()
            .and_then(|v| v.get("crear_nota_credito")?.as_bool())
            .unwrap_or(false);
        if !tiene_permiso {
            return Err("No tiene permisos para crear notas de crédito".to_string());
        }
    }

    let conn = db.conn.lock().map_err(|e| e.to_string())?;

    // Validar que la venta original sea FACTURA AUTORIZADA
    let (factura_numero, cliente_id): (String, i64) = conn
        .query_row(
            "SELECT numero, cliente_id FROM ventas WHERE id = ?1 AND tipo_documento = 'FACTURA' AND estado_sri = 'AUTORIZADA' AND anulada = 0",
            rusqlite::params![nota.venta_id],
            |row| Ok((row.get(0)?, row.get::<_, Option<i64>>(1)?.unwrap_or(1))),
        )
        .map_err(|_| "La venta no existe, no es factura o no está autorizada por el SRI".to_string())?;

    // Validar que no exista ya una NC para esta factura
    let nc_existente: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM notas_credito WHERE venta_id = ?1",
            rusqlite::params![nota.venta_id],
            |row| row.get(0),
        )
        .unwrap_or(0);

    if nc_existente > 0 {
        return Err("Ya existe una nota de credito para esta factura".to_string());
    }

    if nota.items.is_empty() {
        return Err("Debe seleccionar al menos un item para la nota de crédito".to_string());
    }
    if nota.motivo.trim().is_empty() {
        return Err("Debe ingresar un motivo para la nota de crédito".to_string());
    }

    // Leer establecimiento y punto de emisión del terminal
    let establecimiento: String = conn
        .query_row("SELECT value FROM config WHERE key = 'terminal_establecimiento'", [], |row| row.get(0))
        .unwrap_or_else(|_| "001".to_string());
    let punto_emision: String = conn
        .query_row("SELECT value FROM config WHERE key = 'terminal_punto_emision'", [], |row| row.get(0))
        .unwrap_or_else(|_| "001".to_string());

    // Generar número secuencial desde tabla secuenciales
    conn.execute(
        "INSERT OR IGNORE INTO secuenciales (establecimiento_codigo, punto_emision_codigo, tipo_documento, secuencial) VALUES (?1, ?2, 'NOTA_CREDITO', 1)",
        rusqlite::params![establecimiento, punto_emision],
    ).map_err(|e| e.to_string())?;

    let secuencial: i64 = conn
        .query_row(
            "SELECT secuencial FROM secuenciales WHERE establecimiento_codigo = ?1 AND punto_emision_codigo = ?2 AND tipo_documento = 'NOTA_CREDITO'",
            rusqlite::params![establecimiento, punto_emision],
            |row| row.get(0),
        )
        .unwrap_or(1);

    let numero = format!("{}-{}-{:09}", establecimiento, punto_emision, secuencial);

    // Calcular totales
    let mut subtotal_sin_iva = 0.0_f64;
    let mut subtotal_con_iva = 0.0_f64;
    let mut iva_total = 0.0_f64;

    for item in &nota.items {
        let subtotal_item = item.cantidad * item.precio_unitario - item.descuento;
        if item.iva_porcentaje > 0.0 {
            subtotal_con_iva += subtotal_item;
            iva_total += subtotal_item * (item.iva_porcentaje / 100.0);
        } else {
            subtotal_sin_iva += subtotal_item;
        }
    }

    let total = subtotal_sin_iva + subtotal_con_iva + iva_total;

    // Insertar nota de crédito
    conn.execute(
        "INSERT INTO notas_credito (numero, venta_id, cliente_id, motivo,
         subtotal_sin_iva, subtotal_con_iva, iva, total, usuario, usuario_id, establecimiento, punto_emision)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
        rusqlite::params![
            numero, nota.venta_id, cliente_id, nota.motivo.trim(),
            subtotal_sin_iva, subtotal_con_iva, iva_total, total,
            usuario_nombre, usuario_id, establecimiento, punto_emision,
        ],
    )
    .map_err(|e| e.to_string())?;

    let nc_id = conn.last_insert_rowid();

    // Insertar detalles y devolver stock
    for item in &nota.items {
        let subtotal = item.cantidad * item.precio_unitario - item.descuento;
        conn.execute(
            "INSERT INTO nota_credito_detalles (nota_credito_id, producto_id, cantidad,
             precio_unitario, descuento, iva_porcentaje, subtotal)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            rusqlite::params![
                nc_id, item.producto_id, item.cantidad,
                item.precio_unitario, item.descuento, item.iva_porcentaje, subtotal,
            ],
        )
        .map_err(|e| e.to_string())?;

        // Stock anterior + flag servicio para kardex
        let stock_antes: f64 = conn.query_row(
            "SELECT COALESCE(stock_actual, 0) FROM productos WHERE id = ?1",
            rusqlite::params![item.producto_id], |r| r.get(0),
        ).unwrap_or(0.0);
        let es_serv: bool = conn.query_row(
            "SELECT COALESCE(es_servicio, 0) FROM productos WHERE id = ?1",
            rusqlite::params![item.producto_id], |r| r.get::<_, i32>(0),
        ).map(|v| v != 0).unwrap_or(false);

        // Devolver stock (si no es servicio)
        conn.execute(
            "UPDATE productos SET stock_actual = stock_actual + ?1,
             updated_at = datetime('now','localtime')
             WHERE id = ?2 AND es_servicio = 0",
            rusqlite::params![item.cantidad, item.producto_id],
        )
        .ok();

        // Multi-almacén: devolver stock a stock_establecimiento
        let nc_est_id: Option<i64> = conn
            .query_row("SELECT id FROM establecimientos WHERE codigo = ?1", rusqlite::params![establecimiento], |row| row.get(0))
            .ok();
        if let Some(eid) = nc_est_id {
            conn.execute(
                "UPDATE stock_establecimiento SET stock_actual = stock_actual + ?1
                 WHERE producto_id = ?2 AND establecimiento_id = ?3",
                rusqlite::params![item.cantidad, item.producto_id, eid],
            ).ok();
        }

        // v2.3.49 FIX: registrar movimiento kardex tipo 'NOTA_CREDITO' para trazabilidad
        if !es_serv {
            let costo_snap: f64 = conn.query_row(
                "SELECT COALESCE(precio_costo, 0) FROM productos WHERE id = ?1",
                rusqlite::params![item.producto_id], |r| r.get(0),
            ).unwrap_or(0.0);
            let motivo_kardex = format!("NC {} (motivo: {})", numero, nota.motivo.trim());
            let _ = conn.execute(
                "INSERT INTO movimientos_inventario (producto_id, tipo, cantidad, stock_anterior, stock_nuevo, costo_unitario, referencia_id, usuario, establecimiento_id, motivo)
                 VALUES (?1, 'NOTA_CREDITO', ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                rusqlite::params![
                    item.producto_id, item.cantidad,
                    stock_antes, stock_antes + item.cantidad,
                    costo_snap, nc_id, usuario_nombre, nc_est_id, motivo_kardex,
                ],
            );
        }
    }

    // Incrementar secuencial en tabla secuenciales
    conn.execute(
        "UPDATE secuenciales SET secuencial = secuencial + 1 WHERE establecimiento_codigo = ?1 AND punto_emision_codigo = ?2 AND tipo_documento = 'NOTA_CREDITO'",
        rusqlite::params![establecimiento, punto_emision],
    )
    .map_err(|e| e.to_string())?;

    Ok(NotaCreditoInfo {
        id: nc_id,
        numero: numero.clone(),
        venta_id: nota.venta_id,
        factura_numero: factura_numero,
        motivo: nota.motivo.trim().to_string(),
        total,
        fecha: String::new(),
        estado_sri: "PENDIENTE".to_string(),
        autorizacion_sri: None,
        clave_acceso: None,
        numero_factura_nc: None,
    })
}

/// Crea una devolución interna (NC sin SRI) para ventas tipo NOTA_VENTA.
#[tauri::command]
pub fn crear_devolucion_interna(
    db: State<Database>,
    sesion: State<SesionState>,
    venta_id: i64,
    motivo: String,
    items: Vec<serde_json::Value>,
) -> Result<serde_json::Value, String> {
    let sesion_guard = sesion.sesion.lock().map_err(|e| e.to_string())?;
    let sesion_actual = sesion_guard
        .as_ref()
        .ok_or("Debe iniciar sesión".to_string())?;
    let usuario_nombre = sesion_actual.nombre.clone();
    let usuario_id = sesion_actual.usuario_id;
    let usuario_rol = sesion_actual.rol.clone();
    let usuario_permisos = sesion_actual.permisos.clone();
    drop(sesion_guard);

    // Verificar permiso: admin o tiene crear_nota_credito
    if usuario_rol != "ADMIN" {
        let tiene_permiso = serde_json::from_str::<serde_json::Value>(&usuario_permisos)
            .ok()
            .and_then(|v| v.get("crear_nota_credito")?.as_bool())
            .unwrap_or(false);
        if !tiene_permiso {
            return Err("No tiene permisos para crear notas de crédito".to_string());
        }
    }

    if items.is_empty() {
        return Err("Debe seleccionar al menos un item para la devolución".to_string());
    }
    if motivo.trim().is_empty() {
        return Err("Debe ingresar un motivo para la devolución".to_string());
    }

    let conn = db.conn.lock().map_err(|e| e.to_string())?;

    // Validar que la venta existe y NO esta autorizada por SRI
    // Politica: NOTA_VENTA y FACTURA_NO_AUTORIZADA se tratan igual (devolucion interna).
    // Solo FACTURA AUTORIZADA requiere NC electronica SRI.
    let (venta_numero, cliente_id, tipo_doc, estado_sri, venta_forma_pago): (String, i64, String, String, String) = conn
        .query_row(
            "SELECT numero, COALESCE(cliente_id, 1), tipo_documento, COALESCE(estado_sri, 'NO_APLICA'), COALESCE(forma_pago, 'EFECTIVO')
             FROM ventas WHERE id = ?1 AND anulada = 0",
            rusqlite::params![venta_id],
            |row| Ok((row.get(0)?, row.get::<_, Option<i64>>(1)?.unwrap_or(1), row.get(2)?, row.get(3)?, row.get(4)?)),
        )
        .map_err(|_| "Venta no encontrada o anulada".to_string())?;

    // Solo bloquear si es FACTURA YA AUTORIZADA por el SRI
    if tipo_doc == "FACTURA" && estado_sri == "AUTORIZADA" {
        return Err("Esta factura ya fue autorizada por el SRI. Debe crear una Nota de Credito electronica.".to_string());
    }

    // Validar que no exista ya una NC/devolución para esta venta
    let nc_existente: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM notas_credito WHERE venta_id = ?1",
            rusqlite::params![venta_id],
            |row| row.get(0),
        )
        .unwrap_or(0);

    if nc_existente > 0 {
        return Err("Ya existe una devolución para esta venta".to_string());
    }

    // Leer establecimiento y punto de emisión
    let establecimiento: String = conn
        .query_row("SELECT value FROM config WHERE key = 'terminal_establecimiento'", [], |row| row.get(0))
        .unwrap_or_else(|_| "001".to_string());
    let punto_emision: String = conn
        .query_row("SELECT value FROM config WHERE key = 'terminal_punto_emision'", [], |row| row.get(0))
        .unwrap_or_else(|_| "001".to_string());

    // Generar número secuencial
    conn.execute(
        "INSERT OR IGNORE INTO secuenciales (establecimiento_codigo, punto_emision_codigo, tipo_documento, secuencial) VALUES (?1, ?2, 'NOTA_CREDITO', 1)",
        rusqlite::params![establecimiento, punto_emision],
    ).map_err(|e| e.to_string())?;

    let secuencial: i64 = conn
        .query_row(
            "SELECT secuencial FROM secuenciales WHERE establecimiento_codigo = ?1 AND punto_emision_codigo = ?2 AND tipo_documento = 'NOTA_CREDITO'",
            rusqlite::params![establecimiento, punto_emision],
            |row| row.get(0),
        )
        .unwrap_or(1);

    let numero = format!("{}-{}-{:09}", establecimiento, punto_emision, secuencial);

    // Calcular totales
    let mut subtotal_sin_iva = 0.0_f64;
    let mut subtotal_con_iva = 0.0_f64;
    let mut iva_total = 0.0_f64;

    for item in &items {
        let cantidad = item.get("cantidad").and_then(|v| v.as_f64()).unwrap_or(0.0);
        let precio_unitario = item.get("precio_unitario").and_then(|v| v.as_f64()).unwrap_or(0.0);
        let descuento = item.get("descuento").and_then(|v| v.as_f64()).unwrap_or(0.0);
        let iva_porcentaje = item.get("iva_porcentaje").and_then(|v| v.as_f64()).unwrap_or(0.0);

        let subtotal_item = cantidad * precio_unitario - descuento;
        if iva_porcentaje > 0.0 {
            subtotal_con_iva += subtotal_item;
            iva_total += subtotal_item * (iva_porcentaje / 100.0);
        } else {
            subtotal_sin_iva += subtotal_item;
        }
    }

    let total = subtotal_sin_iva + subtotal_con_iva + iva_total;

    // Insertar nota de crédito con estado_sri = 'NO_APLICA'
    conn.execute(
        "INSERT INTO notas_credito (numero, venta_id, cliente_id, motivo,
         subtotal_sin_iva, subtotal_con_iva, iva, total, usuario, usuario_id,
         establecimiento, punto_emision, estado_sri)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, 'NO_APLICA')",
        rusqlite::params![
            numero, venta_id, cliente_id, motivo.trim(),
            subtotal_sin_iva, subtotal_con_iva, iva_total, total,
            usuario_nombre, usuario_id, establecimiento, punto_emision,
        ],
    )
    .map_err(|e| e.to_string())?;

    let nc_id = conn.last_insert_rowid();

    // Insertar detalles y devolver stock (si aplica)
    for item in &items {
        let producto_id = item.get("producto_id").and_then(|v| v.as_i64()).unwrap_or(0);
        let cantidad = item.get("cantidad").and_then(|v| v.as_f64()).unwrap_or(0.0);
        let precio_unitario = item.get("precio_unitario").and_then(|v| v.as_f64()).unwrap_or(0.0);
        let descuento = item.get("descuento").and_then(|v| v.as_f64()).unwrap_or(0.0);
        let iva_porcentaje = item.get("iva_porcentaje").and_then(|v| v.as_f64()).unwrap_or(0.0);
        let subtotal = cantidad * precio_unitario - descuento;
        // v2.3.48: flag opcional "devolver_stock" — si false, solo se descuenta valor
        // (cliente conserva el producto, solo se le devuelve dinero por defecto/dano).
        // Default true para mantener compatibilidad con flujo anterior.
        let devolver_stock_item = item.get("devolver_stock").and_then(|v| v.as_bool()).unwrap_or(true);

        conn.execute(
            "INSERT INTO nota_credito_detalles (nota_credito_id, producto_id, cantidad,
             precio_unitario, descuento, iva_porcentaje, subtotal)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            rusqlite::params![
                nc_id, producto_id, cantidad,
                precio_unitario, descuento, iva_porcentaje, subtotal,
            ],
        )
        .map_err(|e| e.to_string())?;

        // Solo tocamos stock si el cajero pidio "devolver al inventario".
        // Si devolver_stock_item=false → solo se devuelve dinero (NC), el cliente
        // conserva el producto. Util para descuento/compensacion sin retornar fisicamente.
        if devolver_stock_item {
            // Leer stock anterior para el kardex
            let stock_antes: f64 = conn.query_row(
                "SELECT COALESCE(stock_actual, 0) FROM productos WHERE id = ?1",
                rusqlite::params![producto_id], |r| r.get(0),
            ).unwrap_or(0.0);
            let es_servicio: bool = conn.query_row(
                "SELECT COALESCE(es_servicio, 0) FROM productos WHERE id = ?1",
                rusqlite::params![producto_id], |r| r.get::<_, i32>(0),
            ).map(|v| v != 0).unwrap_or(false);

            // Devolver stock (si no es servicio)
            conn.execute(
                "UPDATE productos SET stock_actual = stock_actual + ?1,
                 updated_at = datetime('now','localtime')
                 WHERE id = ?2 AND es_servicio = 0",
                rusqlite::params![cantidad, producto_id],
            )
            .ok();

            // Multi-almacén: devolver stock a stock_establecimiento
            let est_id: Option<i64> = conn
                .query_row("SELECT id FROM establecimientos WHERE codigo = ?1", rusqlite::params![establecimiento], |row| row.get(0))
                .ok();
            if let Some(eid) = est_id {
                conn.execute(
                    "UPDATE stock_establecimiento SET stock_actual = stock_actual + ?1
                     WHERE producto_id = ?2 AND establecimiento_id = ?3",
                    rusqlite::params![cantidad, producto_id, eid],
                ).ok();
            }

            // v2.3.48 FIX: registrar movimiento en kardex (tipo DEVOLUCION) para trazabilidad.
            // Sin esto el stock subia en productos.stock_actual pero el usuario no veia
            // el movimiento en la pantalla de Inventario / Kardex.
            if !es_servicio {
                let costo_snap: f64 = conn.query_row(
                    "SELECT COALESCE(precio_costo, 0) FROM productos WHERE id = ?1",
                    rusqlite::params![producto_id], |r| r.get(0),
                ).unwrap_or(0.0);
                let motivo_kardex = format!("Devolucion NC {} (motivo: {})", numero, motivo.trim());
                let _ = conn.execute(
                    "INSERT INTO movimientos_inventario (producto_id, tipo, cantidad, stock_anterior, stock_nuevo, costo_unitario, referencia_id, usuario, establecimiento_id, motivo)
                     VALUES (?1, 'DEVOLUCION', ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                    rusqlite::params![
                        producto_id,
                        cantidad,                  // positivo (entra al stock)
                        stock_antes,
                        stock_antes + cantidad,
                        costo_snap,
                        nc_id,                     // referencia a la NC
                        usuario_nombre,
                        est_id,
                        motivo_kardex,
                    ],
                );
            }
        }
        // Si devolver_stock_item == false: NO se toca productos.stock_actual,
        // NO se registra kardex. Solo queda la NC con el monto a devolver.
    }

    // Incrementar secuencial
    conn.execute(
        "UPDATE secuenciales SET secuencial = secuencial + 1 WHERE establecimiento_codigo = ?1 AND punto_emision_codigo = ?2 AND tipo_documento = 'NOTA_CREDITO'",
        rusqlite::params![establecimiento, punto_emision],
    )
    .map_err(|e| e.to_string())?;

    // === Devolución del dinero al cliente — efecto sobre caja según forma_pago ===
    // Para que el cierre de caja CUADRE despues de una devolucion, calculamos cuanto
    // efectivo realmente sale de la caja (porcion EFECTIVO de la venta) y registramos
    // un retiro automatico con motivo claro. Para TRANSFER no tocamos caja (admin
    // hace la devolucion por su app del banco).
    //
    // Casos:
    //   forma_pago='EFECTIVO' o sin pagos_venta: devolucion EFECTIVO completa
    //   forma_pago='MIXTO': leer pagos_venta, devolver proporcional al EFECTIVO
    //   forma_pago='TRANSFER' / 'CREDITO': no afecta caja (mensaje informativo)
    //
    // Calcular proporcion EFECTIVO si MIXTO. Si no, asumir 100% del forma_pago.
    let (monto_efectivo_devolver, monto_transfer_devolver, monto_credito_devolver) = {
        let venta_total: f64 = conn.query_row(
            "SELECT total FROM ventas WHERE id = ?1",
            rusqlite::params![venta_id], |r| r.get(0),
        ).unwrap_or(0.0);
        let proporcion_devuelta = if venta_total > 0.01 { total / venta_total } else { 1.0 };

        if venta_forma_pago == "MIXTO" {
            // Sumar por forma desde pagos_venta y aplicar proporcion
            let efe: f64 = conn.query_row(
                "SELECT COALESCE(SUM(monto), 0) FROM pagos_venta
                 WHERE venta_id = ?1 AND UPPER(forma_pago) = 'EFECTIVO'",
                rusqlite::params![venta_id], |r| r.get(0),
            ).unwrap_or(0.0);
            let tra: f64 = conn.query_row(
                "SELECT COALESCE(SUM(monto), 0) FROM pagos_venta
                 WHERE venta_id = ?1 AND UPPER(forma_pago) IN ('TRANSFER','TRANSFERENCIA')",
                rusqlite::params![venta_id], |r| r.get(0),
            ).unwrap_or(0.0);
            let cre: f64 = conn.query_row(
                "SELECT COALESCE(SUM(monto), 0) FROM pagos_venta
                 WHERE venta_id = ?1 AND UPPER(forma_pago) IN ('CREDITO','FIADO')",
                rusqlite::params![venta_id], |r| r.get(0),
            ).unwrap_or(0.0);
            (efe * proporcion_devuelta, tra * proporcion_devuelta, cre * proporcion_devuelta)
        } else {
            match venta_forma_pago.to_uppercase().as_str() {
                "EFECTIVO" => (total, 0.0, 0.0),
                "TRANSFER" | "TRANSFERENCIA" => (0.0, total, 0.0),
                "CREDITO" | "FIADO" => (0.0, 0.0, total),
                _ => (total, 0.0, 0.0), // fallback: tratar como efectivo
            }
        }
    };

    // Si hay efectivo devuelto, registrar retiro automatico para que la caja cuadre
    let mut retiro_creado: Option<i64> = None;
    if monto_efectivo_devolver > 0.01 {
        // Buscar caja abierta — si no hay, no creamos retiro pero igual seguimos
        // (la devolucion se registro bien, el efectivo simplemente no se compensa
        // automaticamente, y al abrir nueva caja se podria ajustar).
        if let Ok(caja_id) = conn.query_row(
            "SELECT id FROM caja WHERE estado = 'ABIERTA' LIMIT 1",
            [], |r| r.get::<_, i64>(0),
        ) {
            let motivo_retiro = format!("Devolución NC {} — efectivo a cliente", numero);
            // Insertamos directo sin pasar por la validacion de "no permitir negativo",
            // porque la devolucion es un evento real que ya paso. Si la caja queda
            // negativa, es responsabilidad del admin hacer ajuste manual despues.
            if conn.execute(
                "INSERT INTO retiros_caja (caja_id, monto, motivo, banco_id, referencia, usuario, usuario_id, estado)
                 VALUES (?1, ?2, ?3, NULL, NULL, ?4, ?5, 'SIN_DEPOSITO')",
                rusqlite::params![caja_id, monto_efectivo_devolver, motivo_retiro, usuario_nombre, usuario_id],
            ).is_ok() {
                retiro_creado = Some(conn.last_insert_rowid());
                // Actualizar monto_esperado stored para consistencia inmediata
                let _ = conn.execute(
                    "UPDATE caja SET monto_esperado = monto_esperado - ?1 WHERE id = ?2",
                    rusqlite::params![monto_efectivo_devolver, caja_id],
                );
            }
        }
    }

    Ok(serde_json::json!({
        "id": nc_id,
        "numero": numero,
        "venta_id": venta_id,
        "venta_numero": venta_numero,
        "motivo": motivo.trim(),
        "total": total,
        "estado_sri": "NO_APLICA",
        // Info para el frontend: que pasar al usuario segun forma_pago
        "monto_efectivo_devuelto": monto_efectivo_devolver,
        "monto_transfer_devuelto": monto_transfer_devolver,
        "monto_credito_devuelto": monto_credito_devolver,
        "retiro_caja_creado_id": retiro_creado,
        "venta_forma_pago": venta_forma_pago,
    }))
}

#[tauri::command]
pub fn listar_notas_credito_dia(
    db: State<Database>,
    fecha: String,
) -> Result<Vec<NotaCreditoInfo>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;

    let mut stmt = conn
        .prepare(
            "SELECT nc.id, nc.numero, nc.venta_id, COALESCE(v.numero_factura, v.numero), nc.motivo,
             nc.total, nc.fecha, nc.estado_sri, nc.autorizacion_sri, nc.clave_acceso, nc.numero_factura_nc
             FROM notas_credito nc
             JOIN ventas v ON nc.venta_id = v.id
             WHERE date(nc.fecha) = date(?1)
             ORDER BY nc.fecha DESC",
        )
        .map_err(|e| e.to_string())?;

    let notas = stmt
        .query_map(rusqlite::params![fecha], |row| {
            Ok(NotaCreditoInfo {
                id: row.get(0)?,
                numero: row.get(1)?,
                venta_id: row.get(2)?,
                factura_numero: row.get(3)?,
                motivo: row.get(4)?,
                total: row.get(5)?,
                fecha: row.get(6)?,
                estado_sri: row.get::<_, String>(7).unwrap_or_else(|_| "PENDIENTE".to_string()),
                autorizacion_sri: row.get(8)?,
                clave_acceso: row.get(9)?,
                numero_factura_nc: row.get(10)?,
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    Ok(notas)
}

#[tauri::command]
pub fn listar_notas_credito(
    db: State<Database>,
    fecha_desde: String,
    fecha_hasta: String,
    estado: Option<String>,
) -> Result<Vec<serde_json::Value>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;

    let base_query = "SELECT nc.id, nc.numero, nc.venta_id, nc.motivo, nc.total, nc.estado_sri,
             nc.fecha, nc.clave_acceso, nc.autorizacion_sri, nc.numero_factura_nc,
             COALESCE(v.numero_factura, v.numero) as venta_numero,
             COALESCE(cl.nombre, 'CONSUMIDOR FINAL') as cliente_nombre
             FROM notas_credito nc
             LEFT JOIN ventas v ON nc.venta_id = v.id
             LEFT JOIN clientes cl ON nc.cliente_id = cl.id
             WHERE date(nc.fecha) >= date(?1) AND date(nc.fecha) <= date(?2)";

    let query = if let Some(ref est) = estado {
        format!("{} AND nc.estado_sri = ?3 ORDER BY nc.fecha DESC", base_query)
    } else {
        format!("{} ORDER BY nc.fecha DESC", base_query)
    };

    let mut stmt = conn.prepare(&query).map_err(|e| e.to_string())?;

    let rows: Vec<serde_json::Value> = if let Some(ref est) = estado {
        stmt.query_map(rusqlite::params![fecha_desde, fecha_hasta, est], |row| {
            Ok(serde_json::json!({
                "id": row.get::<_, i64>(0)?,
                "numero": row.get::<_, String>(1)?,
                "venta_id": row.get::<_, i64>(2)?,
                "motivo": row.get::<_, String>(3)?,
                "total": row.get::<_, f64>(4)?,
                "estado_sri": row.get::<_, String>(5).unwrap_or_else(|_| "PENDIENTE".to_string()),
                "fecha": row.get::<_, String>(6)?,
                "clave_acceso": row.get::<_, Option<String>>(7)?,
                "autorizacion_sri": row.get::<_, Option<String>>(8)?,
                "numero_factura_nc": row.get::<_, Option<String>>(9)?,
                "venta_numero": row.get::<_, String>(10)?,
                "cliente_nombre": row.get::<_, String>(11)?,
            }))
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?
    } else {
        stmt.query_map(rusqlite::params![fecha_desde, fecha_hasta], |row| {
            Ok(serde_json::json!({
                "id": row.get::<_, i64>(0)?,
                "numero": row.get::<_, String>(1)?,
                "venta_id": row.get::<_, i64>(2)?,
                "motivo": row.get::<_, String>(3)?,
                "total": row.get::<_, f64>(4)?,
                "estado_sri": row.get::<_, String>(5).unwrap_or_else(|_| "PENDIENTE".to_string()),
                "fecha": row.get::<_, String>(6)?,
                "clave_acceso": row.get::<_, Option<String>>(7)?,
                "autorizacion_sri": row.get::<_, Option<String>>(8)?,
                "numero_factura_nc": row.get::<_, Option<String>>(9)?,
                "venta_numero": row.get::<_, String>(10)?,
                "cliente_nombre": row.get::<_, String>(11)?,
            }))
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?
    };

    Ok(rows)
}

// ==========================================
// Borradores y Cotizaciones
// ==========================================

#[tauri::command]
pub fn guardar_borrador(
    db: State<Database>,
    sesion: State<SesionState>,
    venta: NuevaVenta,
) -> Result<VentaCompleta, String> {
    guardar_documento_pendiente(db, sesion, venta, "BORRADOR", "BR")
}

#[tauri::command]
pub fn guardar_cotizacion(
    db: State<Database>,
    sesion: State<SesionState>,
    venta: NuevaVenta,
) -> Result<VentaCompleta, String> {
    guardar_documento_pendiente(db, sesion, venta, "COTIZACION", "COT")
}

fn guardar_documento_pendiente(
    db: State<Database>,
    sesion: State<SesionState>,
    venta: NuevaVenta,
    tipo_estado: &str,
    prefijo: &str,
) -> Result<VentaCompleta, String> {
    let sesion_guard = sesion.sesion.lock().map_err(|e| e.to_string())?;
    let sesion_actual = sesion_guard.as_ref().ok_or("Debe iniciar sesion".to_string())?;
    let usuario_nombre = sesion_actual.nombre.clone();
    let usuario_id = sesion_actual.usuario_id;
    drop(sesion_guard);

    let conn = db.conn.lock().map_err(|e| e.to_string())?;

    let seq_tipo = format!("{}_SEQ", tipo_estado);
    conn.execute(
        "INSERT OR IGNORE INTO secuenciales (establecimiento_codigo, punto_emision_codigo, tipo_documento, secuencial) VALUES ('001', '001', ?1, 1)",
        rusqlite::params![seq_tipo],
    ).map_err(|e| e.to_string())?;

    let secuencial: i64 = conn.query_row(
        "SELECT secuencial FROM secuenciales WHERE establecimiento_codigo = '001' AND punto_emision_codigo = '001' AND tipo_documento = ?1",
        rusqlite::params![seq_tipo], |row| row.get(0),
    ).map_err(|e| e.to_string())?;

    let numero = format!("{}-{:06}", prefijo, secuencial);

    let mut subtotal_sin_iva = 0.0_f64;
    let mut subtotal_con_iva = 0.0_f64;
    let mut iva_total = 0.0_f64;
    for item in &venta.items {
        let s = item.cantidad * item.precio_unitario - item.descuento;
        if item.iva_porcentaje > 0.0 { subtotal_con_iva += s; iva_total += s * (item.iva_porcentaje / 100.0); }
        else { subtotal_sin_iva += s; }
    }
    let total = subtotal_sin_iva + subtotal_con_iva + iva_total - venta.descuento;

    conn.execute(
        "INSERT INTO ventas (numero, cliente_id, subtotal_sin_iva, subtotal_con_iva, descuento, iva, total, forma_pago, monto_recibido, cambio, estado, tipo_documento, estado_sri, observacion, usuario, usuario_id, tipo_estado) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, 0, 0, 'PENDIENTE', ?9, 'NO_APLICA', ?10, ?11, ?12, ?13)",
        rusqlite::params![numero, venta.cliente_id.unwrap_or(1), subtotal_sin_iva, subtotal_con_iva, venta.descuento, iva_total, total, venta.forma_pago, venta.tipo_documento, venta.observacion, usuario_nombre, usuario_id, tipo_estado],
    ).map_err(|e| e.to_string())?;

    let venta_id = conn.last_insert_rowid();
    let mut detalles_guardados = Vec::new();
    for item in &venta.items {
        let subtotal = item.cantidad * item.precio_unitario - item.descuento;
        conn.execute(
            "INSERT INTO venta_detalles (venta_id, producto_id, cantidad, precio_unitario, descuento, iva_porcentaje, subtotal, info_adicional) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            rusqlite::params![venta_id, item.producto_id, item.cantidad, item.precio_unitario, item.descuento, item.iva_porcentaje, subtotal, item.info_adicional],
        ).map_err(|e| e.to_string())?;
        let nombre_prod: String = conn.query_row("SELECT nombre FROM productos WHERE id = ?1", rusqlite::params![item.producto_id], |row| row.get(0)).unwrap_or_default();
        detalles_guardados.push(VentaDetalle {
            id: Some(conn.last_insert_rowid()), venta_id: Some(venta_id), producto_id: item.producto_id,
            nombre_producto: Some(nombre_prod), cantidad: item.cantidad, precio_unitario: item.precio_unitario,
            descuento: item.descuento, iva_porcentaje: item.iva_porcentaje, subtotal, info_adicional: item.info_adicional.clone(),
            unidad_id: None, unidad_nombre: None, factor_unidad: None, lote_id: None, combo_seleccion: None,
        });
    }

    conn.execute("UPDATE secuenciales SET secuencial = secuencial + 1 WHERE establecimiento_codigo = '001' AND punto_emision_codigo = '001' AND tipo_documento = ?1", rusqlite::params![seq_tipo]).map_err(|e| e.to_string())?;
    let cliente_nombre: Option<String> = conn.query_row("SELECT nombre FROM clientes WHERE id = ?1", rusqlite::params![venta.cliente_id.unwrap_or(1)], |row| row.get(0)).ok();

    Ok(VentaCompleta {
        venta: Venta {
            id: Some(venta_id), numero, cliente_id: Some(venta.cliente_id.unwrap_or(1)), fecha: None,
            subtotal_sin_iva, subtotal_con_iva, descuento: venta.descuento, iva: iva_total, total,
            forma_pago: venta.forma_pago, monto_recibido: 0.0, cambio: 0.0, estado: "PENDIENTE".to_string(),
            tipo_documento: venta.tipo_documento, estado_sri: "NO_APLICA".to_string(),
            autorizacion_sri: None, clave_acceso: None, observacion: venta.observacion,
            numero_factura: None, establecimiento: None, punto_emision: None,
            banco_id: None, referencia_pago: None, banco_nombre: None,
            comprobante_imagen: None,
            caja_id: None,
            cliente_nombre: None,
            tipo_estado: Some(tipo_estado.to_string()),
            guia_placa: None, guia_chofer: None, guia_direccion_destino: None,
                anulada: None,
        },
        detalles: detalles_guardados, cliente_nombre,
    })
}

#[tauri::command]
pub fn eliminar_borrador(db: State<Database>, id: i64) -> Result<(), String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let tipo_estado: String = conn.query_row("SELECT COALESCE(tipo_estado, 'COMPLETADA') FROM ventas WHERE id = ?1", rusqlite::params![id], |row| row.get(0)).map_err(|_| "Documento no encontrado".to_string())?;
    if tipo_estado != "BORRADOR" && tipo_estado != "COTIZACION" { return Err("Solo se pueden eliminar borradores y cotizaciones".to_string()); }
    conn.execute("DELETE FROM venta_detalles WHERE venta_id = ?1", rusqlite::params![id]).map_err(|e| e.to_string())?;
    conn.execute("DELETE FROM ventas WHERE id = ?1", rusqlite::params![id]).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn listar_documentos_recientes(db: State<Database>, limite: Option<i64>) -> Result<Vec<DocumentoReciente>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let lim = limite.unwrap_or(15);
    let mut stmt = conn.prepare(
        "SELECT v.id, v.numero, COALESCE(v.tipo_estado, 'COMPLETADA'), v.tipo_documento, c.nombre, v.total, COALESCE(v.fecha, datetime('now','localtime')) FROM ventas v LEFT JOIN clientes c ON v.cliente_id = c.id ORDER BY v.id DESC LIMIT ?1"
    ).map_err(|e| e.to_string())?;
    let docs = stmt.query_map(rusqlite::params![lim], |row| {
        Ok(DocumentoReciente { id: row.get(0)?, numero: row.get(1)?, tipo_estado: row.get(2)?, tipo_documento: row.get(3)?, cliente_nombre: row.get(4)?, total: row.get(5)?, fecha: row.get(6)? })
    }).map_err(|e| e.to_string())?.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())?;
    Ok(docs)
}

// --- Guías de Remisión ---

#[tauri::command]
pub fn guardar_guia_remision(
    db: State<Database>,
    sesion: State<SesionState>,
    venta: NuevaVenta,
) -> Result<VentaCompleta, String> {
    let sesion_guard = sesion.sesion.lock().map_err(|e| e.to_string())?;
    let sesion_actual = sesion_guard.as_ref().ok_or("Debe iniciar sesion".to_string())?;
    let usuario_nombre = sesion_actual.nombre.clone();
    let usuario_id = sesion_actual.usuario_id;
    drop(sesion_guard);

    let conn = db.conn.lock().map_err(|e| e.to_string())?;

    // Leer establecimiento y punto de emisión del terminal
    let terminal_est: String = conn
        .query_row("SELECT value FROM config WHERE key = 'terminal_establecimiento'", [], |row| row.get(0))
        .unwrap_or_else(|_| "001".to_string());
    let terminal_pe: String = conn
        .query_row("SELECT value FROM config WHERE key = 'terminal_punto_emision'", [], |row| row.get(0))
        .unwrap_or_else(|_| "001".to_string());

    // Multi-almacén: obtener ID del establecimiento para descontar stock
    let multi_almacen: bool = conn
        .query_row("SELECT value FROM config WHERE key = 'multi_almacen_activo'", [], |row| row.get::<_, String>(0))
        .map(|v| v == "1")
        .unwrap_or(false);
    let est_id: Option<i64> = if multi_almacen {
        conn.query_row("SELECT id FROM establecimientos WHERE codigo = ?1", rusqlite::params![terminal_est], |row| row.get(0)).ok()
    } else {
        None
    };

    // Obtener secuencial para guía de remisión
    conn.execute(
        "INSERT OR IGNORE INTO secuenciales (establecimiento_codigo, punto_emision_codigo, tipo_documento, secuencial) VALUES ('001', '001', 'GUIA_REMISION_SEQ', 1)",
        [],
    ).map_err(|e| e.to_string())?;

    let secuencial: i64 = conn.query_row(
        "SELECT secuencial FROM secuenciales WHERE establecimiento_codigo = '001' AND punto_emision_codigo = '001' AND tipo_documento = 'GUIA_REMISION_SEQ'",
        [], |row| row.get(0),
    ).map_err(|e| e.to_string())?;

    let numero = format!("GR-{:06}", secuencial);

    // Calcular totales
    let mut subtotal_sin_iva = 0.0_f64;
    let mut subtotal_con_iva = 0.0_f64;
    let mut iva_total = 0.0_f64;
    for item in &venta.items {
        let s = item.cantidad * item.precio_unitario - item.descuento;
        if item.iva_porcentaje > 0.0 { subtotal_con_iva += s; iva_total += s * (item.iva_porcentaje / 100.0); }
        else { subtotal_sin_iva += s; }
    }
    let total = subtotal_sin_iva + subtotal_con_iva + iva_total - venta.descuento;

    // Insertar cabecera
    conn.execute(
        "INSERT INTO ventas (numero, cliente_id, subtotal_sin_iva, subtotal_con_iva, descuento, iva, total, forma_pago, monto_recibido, cambio, estado, tipo_documento, estado_sri, observacion, usuario, usuario_id, establecimiento, punto_emision, tipo_estado, guia_placa, guia_chofer, guia_direccion_destino)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, 0, 0, 'PENDIENTE', ?9, 'NO_APLICA', ?10, ?11, ?12, ?13, ?14, 'GUIA_REMISION', ?15, ?16, ?17)",
        rusqlite::params![
            numero, venta.cliente_id.unwrap_or(1), subtotal_sin_iva, subtotal_con_iva,
            venta.descuento, iva_total, total, venta.forma_pago, venta.tipo_documento,
            venta.observacion, usuario_nombre, usuario_id, terminal_est, terminal_pe,
            venta.guia_placa, venta.guia_chofer, venta.guia_direccion_destino,
        ],
    ).map_err(|e| e.to_string())?;

    let venta_id = conn.last_insert_rowid();

    // Insertar detalles, descontar stock, crear kardex
    let mut detalles_guardados = Vec::new();
    for item in &venta.items {
        let subtotal = item.cantidad * item.precio_unitario - item.descuento;
        conn.execute(
            "INSERT INTO venta_detalles (venta_id, producto_id, cantidad, precio_unitario, descuento, iva_porcentaje, subtotal, info_adicional)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            rusqlite::params![venta_id, item.producto_id, item.cantidad, item.precio_unitario, item.descuento, item.iva_porcentaje, subtotal, item.info_adicional],
        ).map_err(|e| e.to_string())?;

        // Obtener stock antes de descontar y verificar si es servicio o no controla stock
        let (stock_antes, es_servicio, no_controla_stock): (f64, bool, bool) = conn
            .query_row(
                "SELECT stock_actual, es_servicio, COALESCE(no_controla_stock, 0) FROM productos WHERE id = ?1",
                rusqlite::params![item.producto_id],
                |row| Ok((row.get::<_, f64>(0)?, row.get::<_, bool>(1)?, row.get::<_, i32>(2)? != 0)),
            )
            .unwrap_or((0.0, false, false));

        let omite_stock = es_servicio || no_controla_stock;

        // Descontar stock (si no es servicio y controla stock)
        if !omite_stock {
            conn.execute(
                "UPDATE productos SET stock_actual = stock_actual - ?1,
                 updated_at = datetime('now','localtime')
                 WHERE id = ?2",
                rusqlite::params![item.cantidad, item.producto_id],
            )
            .map_err(|e| e.to_string())?;
        }

        // Multi-almacén: también descontar de stock_establecimiento
        if let Some(eid) = est_id {
            if !omite_stock {
                conn.execute(
                    "UPDATE stock_establecimiento SET stock_actual = stock_actual - ?1
                     WHERE producto_id = ?2 AND establecimiento_id = ?3",
                    rusqlite::params![item.cantidad, item.producto_id, eid],
                ).ok();
            }
        }

        // Registrar movimiento de inventario (kardex) para productos físicos
        // costo_unitario = precio_costo snapshot, NO precio de venta
        if !omite_stock {
            let costo_snap: f64 = conn.query_row(
                "SELECT precio_costo FROM productos WHERE id = ?1",
                rusqlite::params![item.producto_id],
                |row| row.get(0)
            ).unwrap_or(0.0);
            let _ = conn.execute(
                "INSERT INTO movimientos_inventario (producto_id, tipo, cantidad, stock_anterior, stock_nuevo, costo_unitario, referencia_id, usuario, establecimiento_id)
                 VALUES (?1, 'GUIA_REMISION', ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                rusqlite::params![
                    item.producto_id,
                    -(item.cantidad),
                    stock_antes,
                    stock_antes - item.cantidad,
                    costo_snap,
                    venta_id,
                    usuario_nombre,
                    est_id,
                ],
            );
        }

        let nombre_prod: String = conn.query_row("SELECT nombre FROM productos WHERE id = ?1", rusqlite::params![item.producto_id], |row| row.get(0)).unwrap_or_default();
        detalles_guardados.push(VentaDetalle {
            id: Some(conn.last_insert_rowid()), venta_id: Some(venta_id), producto_id: item.producto_id,
            nombre_producto: Some(nombre_prod), cantidad: item.cantidad, precio_unitario: item.precio_unitario,
            descuento: item.descuento, iva_porcentaje: item.iva_porcentaje, subtotal, info_adicional: item.info_adicional.clone(),
            unidad_id: None, unidad_nombre: None, factor_unidad: None, lote_id: None, combo_seleccion: None,
        });
    }

    // Incrementar secuencial
    conn.execute(
        "UPDATE secuenciales SET secuencial = secuencial + 1 WHERE establecimiento_codigo = '001' AND punto_emision_codigo = '001' AND tipo_documento = 'GUIA_REMISION_SEQ'",
        [],
    ).map_err(|e| e.to_string())?;

    let cliente_nombre: Option<String> = conn.query_row("SELECT nombre FROM clientes WHERE id = ?1", rusqlite::params![venta.cliente_id.unwrap_or(1)], |row| row.get(0)).ok();

    Ok(VentaCompleta {
        venta: Venta {
            id: Some(venta_id), numero, cliente_id: Some(venta.cliente_id.unwrap_or(1)), fecha: None,
            subtotal_sin_iva, subtotal_con_iva, descuento: venta.descuento, iva: iva_total, total,
            forma_pago: venta.forma_pago, monto_recibido: 0.0, cambio: 0.0, estado: "PENDIENTE".to_string(),
            tipo_documento: venta.tipo_documento, estado_sri: "NO_APLICA".to_string(),
            autorizacion_sri: None, clave_acceso: None, observacion: venta.observacion,
            numero_factura: None, establecimiento: Some(terminal_est), punto_emision: Some(terminal_pe),
            banco_id: None, referencia_pago: None, banco_nombre: None,
            comprobante_imagen: None,
            caja_id: None,
            cliente_nombre: None,
            tipo_estado: Some("GUIA_REMISION".to_string()),
            guia_placa: venta.guia_placa, guia_chofer: venta.guia_chofer,
            guia_direccion_destino: venta.guia_direccion_destino,
                anulada: None,
        },
        detalles: detalles_guardados, cliente_nombre,
    })
}

#[tauri::command]
pub fn listar_guias_remision(
    db: State<Database>,
    fecha_desde: Option<String>,
    fecha_hasta: Option<String>,
    cliente_id: Option<i64>,
    estado: Option<String>,
) -> Result<Vec<Venta>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;

    let mut sql = String::from(
        "SELECT v.id, v.numero, v.cliente_id, v.fecha, v.subtotal_sin_iva, v.subtotal_con_iva,
         v.descuento, v.iva, v.total, v.forma_pago, v.monto_recibido, v.cambio, v.estado,
         v.tipo_documento, v.estado_sri, v.autorizacion_sri, v.clave_acceso, v.observacion,
         v.numero_factura, v.establecimiento, v.punto_emision,
         v.banco_id, v.referencia_pago, cb.nombre as banco_nombre, v.tipo_estado,
         cl.nombre as cliente_nombre
         FROM ventas v
         LEFT JOIN cuentas_banco cb ON v.banco_id = cb.id
         LEFT JOIN clientes cl ON v.cliente_id = cl.id
         WHERE v.tipo_estado = 'GUIA_REMISION'"
    );

    let mut params: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();
    let mut param_idx = 1;

    if let Some(ref fd) = fecha_desde {
        sql.push_str(&format!(" AND date(v.fecha) >= date(?{})", param_idx));
        params.push(Box::new(fd.clone()));
        param_idx += 1;
    }
    if let Some(ref fh) = fecha_hasta {
        sql.push_str(&format!(" AND date(v.fecha) <= date(?{})", param_idx));
        params.push(Box::new(fh.clone()));
        param_idx += 1;
    }
    if let Some(cid) = cliente_id {
        sql.push_str(&format!(" AND v.cliente_id = ?{}", param_idx));
        params.push(Box::new(cid));
        param_idx += 1;
    }
    if let Some(ref est) = estado {
        sql.push_str(&format!(" AND v.estado = ?{}", param_idx));
        params.push(Box::new(est.clone()));
        let _ = param_idx;
    }

    sql.push_str(" ORDER BY v.fecha DESC");

    let param_refs: Vec<&dyn rusqlite::types::ToSql> = params.iter().map(|p| p.as_ref()).collect();
    let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;

    let ventas = stmt.query_map(param_refs.as_slice(), |row| {
        Ok(Venta {
            id: Some(row.get(0)?),
            numero: row.get(1)?,
            cliente_id: row.get(2)?,
            fecha: row.get(3)?,
            subtotal_sin_iva: row.get(4)?,
            subtotal_con_iva: row.get(5)?,
            descuento: row.get(6)?,
            iva: row.get(7)?,
            total: row.get(8)?,
            forma_pago: row.get(9)?,
            monto_recibido: row.get(10)?,
            cambio: row.get(11)?,
            estado: row.get(12)?,
            tipo_documento: row.get(13)?,
            estado_sri: row.get::<_, String>(14).unwrap_or_else(|_| "NO_APLICA".to_string()),
            autorizacion_sri: row.get(15)?,
            clave_acceso: row.get(16)?,
            observacion: row.get(17)?,
            numero_factura: row.get(18)?,
            establecimiento: row.get(19).ok(),
            punto_emision: row.get(20).ok(),
            banco_id: row.get(21).ok(),
            referencia_pago: row.get(22).ok(),
            banco_nombre: row.get(23).ok(),
            comprobante_imagen: None,
            caja_id: None,
            cliente_nombre: row.get(25).ok(),
            tipo_estado: row.get(24).ok(),
            guia_placa: None, guia_chofer: None, guia_direccion_destino: None,
                anulada: None,
        })
    }).map_err(|e| e.to_string())?
    .collect::<Result<Vec<_>, _>>()
    .map_err(|e| e.to_string())?;

    Ok(ventas)
}

#[tauri::command]
pub fn resumen_guias_remision(
    db: State<Database>,
    fecha_desde: String,
    fecha_hasta: String,
) -> Result<ResumenGuias, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;

    let resumen = conn.query_row(
        "SELECT
            COALESCE(SUM(CASE WHEN estado = 'PENDIENTE' THEN 1 ELSE 0 END), 0),
            COALESCE(SUM(CASE WHEN estado = 'COMPLETADA' THEN 1 ELSE 0 END), 0),
            COALESCE(SUM(CASE WHEN estado = 'PENDIENTE' THEN total ELSE 0 END), 0.0),
            COALESCE(SUM(CASE WHEN estado = 'COMPLETADA' THEN total ELSE 0 END), 0.0)
         FROM ventas
         WHERE tipo_estado = 'GUIA_REMISION'
           AND date(fecha) >= date(?1)
           AND date(fecha) <= date(?2)",
        rusqlite::params![fecha_desde, fecha_hasta],
        |row| {
            Ok(ResumenGuias {
                abiertas: row.get(0)?,
                cerradas: row.get(1)?,
                total_pendiente: row.get(2)?,
                total_cerrado: row.get(3)?,
            })
        },
    ).map_err(|e| e.to_string())?;

    Ok(resumen)
}

/// Item editado en el modal Facturar.
/// - precio_unitario y descuento: siempre editables.
/// - cantidad: SOLO se aplica si la guia esta en estado PENDIENTE
///   (todavia no entregada al cliente). Si esta ENTREGADA, cambiar cantidad
///   no se permite — debe ser devolucion parcial despues.
/// El backend ajusta stock si cantidad cambia (decrementa mas o devuelve).
#[derive(Debug, serde::Deserialize)]
pub struct ItemOverride {
    pub producto_id: i64,
    pub precio_unitario: f64,
    pub descuento: f64,
    #[serde(default)]
    pub cantidad: Option<f64>,
}

#[tauri::command]
pub fn convertir_guia_a_venta(
    db: State<Database>,
    sesion: State<SesionState>,
    guia_id: i64,
    forma_pago: String,
    monto_recibido: f64,
    es_fiado: Option<bool>,
    banco_id: Option<i64>,
    referencia_pago: Option<String>,
    // Si presente, sobrescribe precios/descuentos por item al facturar.
    // El cajero puede corregir precios mal cotizados antes de facturar.
    items_override: Option<Vec<ItemOverride>>,
) -> Result<VentaCompleta, String> {
    let sesion_guard = sesion.sesion.lock().map_err(|e| e.to_string())?;
    let sesion_actual = sesion_guard.as_ref().ok_or("Debe iniciar sesion".to_string())?;
    let usuario_nombre = sesion_actual.nombre.clone();
    let usuario_id = sesion_actual.usuario_id;
    drop(sesion_guard);

    let conn = db.conn.lock().map_err(|e| e.to_string())?;

    // Verificar que haya caja abierta
    let caja_abierta: bool = conn
        .query_row(
            "SELECT COUNT(*) FROM caja WHERE estado = 'ABIERTA'",
            [],
            |row| row.get::<_, i64>(0),
        )
        .map(|count| count > 0)
        .unwrap_or(false);

    if !caja_abierta {
        return Err("Debe abrir la caja antes de convertir una guía en venta".to_string());
    }

    // Verificar que la guía existe y está PENDIENTE o ENTREGADA (no FACTURADA ni RECHAZADA)
    // v2.3.36: aceptar ENTREGADA permite convertir despues de marcar entregada al cliente.
    let (guia_numero, _total_guia, cliente_id_val, _subtotal_sin_iva_g, _subtotal_con_iva_g, _iva_g, descuento_g, tipo_doc_g, observacion_g, guia_estado_actual): (String, f64, i64, f64, f64, f64, f64, String, Option<String>, String) = conn.query_row(
        "SELECT numero, total, cliente_id, subtotal_sin_iva, subtotal_con_iva, iva, descuento, tipo_documento, observacion, estado
         FROM ventas WHERE id = ?1 AND tipo_estado = 'GUIA_REMISION'
         AND estado IN ('PENDIENTE', 'ENTREGADA')",
        rusqlite::params![guia_id],
        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?, row.get(5)?, row.get(6)?, row.get(7)?, row.get(8)?, row.get(9)?)),
    ).map_err(|_| "Guía no encontrada, ya fue facturada, o fue rechazada".to_string())?;
    let guia_es_pendiente = guia_estado_actual == "PENDIENTE";

    // Leer establecimiento y punto de emisión del terminal
    let terminal_est: String = conn
        .query_row("SELECT value FROM config WHERE key = 'terminal_establecimiento'", [], |row| row.get(0))
        .unwrap_or_else(|_| "001".to_string());
    let terminal_pe: String = conn
        .query_row("SELECT value FROM config WHERE key = 'terminal_punto_emision'", [], |row| row.get(0))
        .unwrap_or_else(|_| "001".to_string());

    // Caja actual para vincular la nueva venta
    let caja_id_actual: Option<i64> = conn
        .query_row("SELECT id FROM caja WHERE estado = 'ABIERTA' LIMIT 1", [], |r| r.get(0))
        .ok();

    // Generar nuevo secuencial NV
    conn.execute(
        "INSERT OR IGNORE INTO secuenciales (establecimiento_codigo, punto_emision_codigo, tipo_documento, secuencial) VALUES (?1, ?2, 'NOTA_VENTA', 1)",
        rusqlite::params![terminal_est, terminal_pe],
    ).map_err(|e| e.to_string())?;

    let mut secuencial: i64 = conn.query_row(
        "SELECT secuencial FROM secuenciales WHERE establecimiento_codigo = ?1 AND punto_emision_codigo = ?2 AND tipo_documento = 'NOTA_VENTA'",
        rusqlite::params![terminal_est, terminal_pe],
        |row| row.get(0),
    ).map_err(|e| e.to_string())?;
    let max_existente: i64 = conn
        .query_row("SELECT COALESCE(MAX(CAST(SUBSTR(numero, 4) AS INTEGER)), 0) FROM ventas WHERE numero LIKE 'NV-%'", [], |row| row.get(0))
        .unwrap_or(0);
    if max_existente >= secuencial { secuencial = max_existente + 1; }

    let nuevo_numero = format!("NV-{:09}", secuencial);

    // === Leer items de la guia y aplicar overrides de precio si se enviaron ===
    let mut stmt_d = conn.prepare(
        "SELECT producto_id, cantidad, precio_unitario, descuento, iva_porcentaje, info_adicional
         FROM venta_detalles WHERE venta_id = ?1"
    ).map_err(|e| e.to_string())?;
    let items_originales: Vec<(i64, f64, f64, f64, f64, Option<String>)> = stmt_d.query_map(
        rusqlite::params![guia_id],
        |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?, r.get(5)?))
    ).map_err(|e| e.to_string())?
     .collect::<Result<Vec<_>, _>>()
     .map_err(|e| e.to_string())?;
    drop(stmt_d);

    // Aplicar overrides (precio/descuento por producto_id, y cantidad solo si guia PENDIENTE).
    // Tuple: (precio_unitario, descuento, cantidad_override)
    let overrides_map: std::collections::HashMap<i64, (f64, f64, Option<f64>)> = items_override
        .as_ref()
        .map(|ovs| ovs.iter().map(|o| (o.producto_id, (o.precio_unitario, o.descuento, o.cantidad))).collect())
        .unwrap_or_default();

    let r2 = |n: f64| (n * 100.0).round() / 100.0;
    let mut items_finales: Vec<(i64, f64, f64, f64, f64, f64, Option<String>)> = Vec::new();
    let mut sum_sin_iva = 0.0_f64;
    let mut sum_con_iva = 0.0_f64;
    let mut sum_iva = 0.0_f64;
    // Track stock adjustments (+ = devolver al inventario, - = decrementar mas).
    // Solo aplica si guia PENDIENTE — si ENTREGADA cantidad nunca cambia.
    let mut ajustes_stock: Vec<(i64, f64)> = Vec::new();
    for (pid, cant_orig, pu_orig, desc_orig, iva_p, info) in &items_originales {
        let ov = overrides_map.get(pid).copied();
        let (pu, desc, cant_override) = ov.unwrap_or((*pu_orig, *desc_orig, None));
        let cant = if guia_es_pendiente {
            cant_override.unwrap_or(*cant_orig)
        } else {
            *cant_orig // ENTREGADA: cantidad fija
        };
        if guia_es_pendiente && (cant - cant_orig).abs() > 0.0001 {
            // Diferencia: si cant nueva > cant_orig, decrementar mas stock (negativo)
            //             si cant nueva < cant_orig, devolver stock (positivo)
            ajustes_stock.push((*pid, *cant_orig - cant)); // ajuste = orig - nueva (positivo = devolver)
        }
        let subtotal_item = r2(cant * pu - desc);
        if *iva_p > 0.0 {
            sum_con_iva += subtotal_item;
            sum_iva += r2(subtotal_item * (iva_p / 100.0));
        } else {
            sum_sin_iva += subtotal_item;
        }
        items_finales.push((*pid, cant, pu, desc, *iva_p, subtotal_item, info.clone()));
    }
    sum_sin_iva = r2(sum_sin_iva);
    sum_con_iva = r2(sum_con_iva);
    sum_iva = r2(sum_iva);
    let total_recalculado = r2(sum_sin_iva + sum_con_iva + sum_iva - descuento_g);

    // Aplicar ajustes de stock si los hubo (solo PENDIENTE).
    // ajuste positivo = devolver stock; ajuste negativo = decrementar (mas).
    if !ajustes_stock.is_empty() {
        // Multi-almacen: ID del establecimiento
        let multi_almacen: bool = conn.query_row(
            "SELECT value FROM config WHERE key = 'multi_almacen_activo'", [],
            |r| r.get::<_, String>(0)
        ).map(|v| v == "1").unwrap_or(false);
        let est_id_ajuste: Option<i64> = if multi_almacen {
            conn.query_row("SELECT id FROM establecimientos WHERE codigo = ?1",
                rusqlite::params![terminal_est], |r| r.get(0)).ok()
        } else { None };

        for (pid, ajuste) in &ajustes_stock {
            // ajuste positivo = devolver, negativo = decrementar
            conn.execute(
                "UPDATE productos SET stock_actual = stock_actual + ?1, updated_at = datetime('now','localtime')
                 WHERE id = ?2 AND es_servicio = 0",
                rusqlite::params![ajuste, pid],
            ).ok();
            if let Some(eid) = est_id_ajuste {
                conn.execute(
                    "UPDATE stock_establecimiento SET stock_actual = stock_actual + ?1
                     WHERE producto_id = ?2 AND establecimiento_id = ?3",
                    rusqlite::params![ajuste, pid, eid],
                ).ok();
            }
            // Kardex: registrar el ajuste
            let stock_ant: f64 = conn.query_row(
                "SELECT stock_actual FROM productos WHERE id = ?1",
                rusqlite::params![pid], |r| r.get(0),
            ).unwrap_or(0.0);
            let _ = conn.execute(
                "INSERT INTO movimientos_inventario (producto_id, tipo, cantidad, stock_anterior, stock_nuevo, costo_unitario, referencia_id, usuario, establecimiento_id)
                 VALUES (?1, 'AJUSTE_GUIA', ?2, ?3, ?4, 0, ?5, ?6, ?7)",
                rusqlite::params![pid, ajuste, stock_ant - ajuste, stock_ant, guia_id, usuario_nombre, est_id_ajuste],
            );
        }

        // Actualizar venta_detalles de la guia con las nuevas cantidades para mantener consistencia
        // (el listado de la guia muestra los items actualizados)
        for (pid, cant, _pu, _desc, _iva, _sub, _info) in &items_finales {
            conn.execute(
                "UPDATE venta_detalles SET cantidad = ?1 WHERE venta_id = ?2 AND producto_id = ?3",
                rusqlite::params![cant, guia_id, pid],
            ).ok();
        }
    }

    let cambio = if monto_recibido > total_recalculado { monto_recibido - total_recalculado } else { 0.0 };

    // Determinar pago_estado para verificacion (igual que crear_venta normal)
    let es_admin = {
        let s = sesion.sesion.lock().map_err(|e| e.to_string())?;
        s.as_ref().map(|sa| sa.rol == "ADMIN").unwrap_or(false)
    };
    let es_transfer = matches!(forma_pago.to_uppercase().as_str(), "TRANSFER" | "TRANSFERENCIA");
    let pago_estado_inicial: &str = if !es_transfer { "NO_APLICA" }
        else if es_admin { "VERIFICADO" } else { "REGISTRADO" };
    let verif_por: Option<i64> = if pago_estado_inicial == "VERIFICADO" { Some(usuario_id) } else { None };
    let verif_fecha: Option<String> = if pago_estado_inicial == "VERIFICADO" {
        Some(chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string())
    } else { None };

    // Insertar NUEVA venta como COMPLETADA, vinculada a la guia origen.
    // NO se vuelve a descontar stock (ya se descontó al crear la guía).
    // Si hubo overrides de precio, los totales son los recalculados (no los originales de la guia).
    let estado_sri_nuevo = match tipo_doc_g.as_str() { "FACTURA" => "PENDIENTE", _ => "NO_APLICA" };
    let observacion_extra = if items_override.is_some() {
        format!(" | Precios editados al facturar")
    } else { String::new() };
    let nueva_observacion = match observacion_g {
        Some(ref o) if !o.is_empty() => format!("{} | Origen: {}{}", o, guia_numero, observacion_extra),
        _ => format!("Origen: {}{}", guia_numero, observacion_extra),
    };

    conn.execute(
        "INSERT INTO ventas (numero, cliente_id, subtotal_sin_iva, subtotal_con_iva,
         descuento, iva, total, forma_pago, monto_recibido, cambio, estado,
         tipo_documento, estado_sri, observacion, usuario, usuario_id, establecimiento, punto_emision,
         banco_id, referencia_pago, comprobante_imagen,
         pago_estado, verificado_por, fecha_verificacion, caja_id,
         tipo_estado, guia_origen_id)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, 'COMPLETADA', ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, NULL,
                 ?20, ?21, ?22, ?23, 'COMPLETADA', ?24)",
        rusqlite::params![
            nuevo_numero, cliente_id_val,
            sum_sin_iva, sum_con_iva, descuento_g, sum_iva, total_recalculado,
            forma_pago, monto_recibido, cambio,
            tipo_doc_g, estado_sri_nuevo, nueva_observacion,
            usuario_nombre, usuario_id, terminal_est, terminal_pe,
            banco_id, referencia_pago,
            pago_estado_inicial, verif_por, verif_fecha, caja_id_actual,
            guia_id,
        ],
    ).map_err(|e| e.to_string())?;
    let nueva_venta_id = conn.last_insert_rowid();

    // Insertar detalles en la nueva venta usando items_finales (con overrides aplicados).
    // SIN tocar stock — ya descontado al crear la guia (cantidad nunca cambia,
    // si necesitan cambiar cantidad → devolucion parcial despues).
    for (pid, cant, pu, desc, iva_p, sub, info) in &items_finales {
        conn.execute(
            "INSERT INTO venta_detalles (venta_id, producto_id, cantidad, precio_unitario, descuento, iva_porcentaje, subtotal, info_adicional)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            rusqlite::params![nueva_venta_id, pid, cant, pu, desc, iva_p, sub, info],
        ).map_err(|e| e.to_string())?;
    }

    // Marcar la guia origen como FACTURADA (queda visible en pestaña Facturadas)
    conn.execute(
        "UPDATE ventas SET estado = 'FACTURADA' WHERE id = ?1 AND tipo_estado = 'GUIA_REMISION'",
        rusqlite::params![guia_id],
    ).map_err(|e| e.to_string())?;

    // Incrementar secuencial NV
    conn.execute(
        "UPDATE secuenciales SET secuencial = ?1 + 1 WHERE establecimiento_codigo = ?2 AND punto_emision_codigo = ?3 AND tipo_documento = 'NOTA_VENTA'",
        rusqlite::params![secuencial, terminal_est, terminal_pe],
    ).map_err(|e| e.to_string())?;

    // === Actualizar caja: solo PORCION EFECTIVO de la nueva venta afecta el efectivo ===
    // Usar total_recalculado (con overrides aplicados) en vez del total original de la guia.
    let efectivo_de_esta_venta: f64 = if forma_pago.to_uppercase() == "EFECTIVO" && !es_fiado.unwrap_or(false) {
        total_recalculado
    } else {
        0.0
    };
    conn.execute(
        "UPDATE caja SET monto_ventas = monto_ventas + ?1, monto_esperado = monto_esperado + ?2 WHERE estado = 'ABIERTA'",
        rusqlite::params![total_recalculado, efectivo_de_esta_venta],
    ).ok();

    // Si es fiado, crear cuenta por cobrar (vinculada a la NUEVA venta, no a la guia)
    if es_fiado.unwrap_or(false) {
        conn.execute(
            "INSERT INTO cuentas_por_cobrar (cliente_id, venta_id, monto_total, saldo, estado)
             VALUES (?1, ?2, ?3, ?3, 'PENDIENTE')",
            rusqlite::params![cliente_id_val, nueva_venta_id, total_recalculado],
        ).map_err(|e| e.to_string())?;
    }

    // Obtener la nueva venta para retornar
    let venta_result = conn.query_row(
        "SELECT v.id, v.numero, v.cliente_id, v.fecha, v.subtotal_sin_iva, v.subtotal_con_iva,
         v.descuento, v.iva, v.total, v.forma_pago, v.monto_recibido, v.cambio, v.estado,
         v.tipo_documento, v.estado_sri, v.autorizacion_sri, v.clave_acceso, v.observacion,
         v.numero_factura, v.establecimiento, v.punto_emision,
         v.banco_id, v.referencia_pago, cb.nombre as banco_nombre, v.tipo_estado
         FROM ventas v
         LEFT JOIN cuentas_banco cb ON v.banco_id = cb.id
         WHERE v.id = ?1",
        rusqlite::params![nueva_venta_id],
        |row| {
            Ok(Venta {
                id: Some(row.get(0)?),
                numero: row.get(1)?,
                cliente_id: row.get(2)?,
                fecha: row.get(3)?,
                subtotal_sin_iva: row.get(4)?,
                subtotal_con_iva: row.get(5)?,
                descuento: row.get(6)?,
                iva: row.get(7)?,
                total: row.get(8)?,
                forma_pago: row.get(9)?,
                monto_recibido: row.get(10)?,
                cambio: row.get(11)?,
                estado: row.get(12)?,
                tipo_documento: row.get(13)?,
                estado_sri: row.get::<_, String>(14).unwrap_or_else(|_| "NO_APLICA".to_string()),
                autorizacion_sri: row.get(15)?,
                clave_acceso: row.get(16)?,
                observacion: row.get(17)?,
                numero_factura: row.get(18)?,
                establecimiento: row.get(19).ok(),
                punto_emision: row.get(20).ok(),
                banco_id: row.get(21).ok(),
                referencia_pago: row.get(22).ok(),
                banco_nombre: row.get(23).ok(),
                comprobante_imagen: None,
                caja_id: caja_id_actual,
                cliente_nombre: None,
                tipo_estado: row.get(24).ok(),
                guia_placa: None, guia_chofer: None, guia_direccion_destino: None,
                anulada: None,
            })
        },
    ).map_err(|e| e.to_string())?;

    // Obtener detalles
    let mut stmt = conn.prepare(
        "SELECT vd.id, vd.venta_id, vd.producto_id, p.nombre, vd.cantidad, vd.precio_unitario,
         vd.descuento, vd.iva_porcentaje, vd.subtotal, vd.info_adicional
         FROM venta_detalles vd
         LEFT JOIN productos p ON vd.producto_id = p.id
         WHERE vd.venta_id = ?1"
    ).map_err(|e| e.to_string())?;

    let detalles = stmt.query_map(rusqlite::params![nueva_venta_id], |row| {
        Ok(VentaDetalle {
            id: row.get(0)?,
            venta_id: row.get(1)?,
            producto_id: row.get(2)?,
            nombre_producto: row.get(3)?,
            cantidad: row.get(4)?,
            precio_unitario: row.get(5)?,
            descuento: row.get(6)?,
            iva_porcentaje: row.get(7)?,
            subtotal: row.get(8)?,
            info_adicional: row.get(9)?,
            unidad_id: None, unidad_nombre: None, factor_unidad: None, lote_id: None, combo_seleccion: None,
        })
    }).map_err(|e| e.to_string())?
    .collect::<Result<Vec<_>, _>>()
    .map_err(|e| e.to_string())?;

    let cliente_nombre: Option<String> = conn.query_row(
        "SELECT nombre FROM clientes WHERE id = ?1",
        rusqlite::params![cliente_id_val],
        |row| row.get(0),
    ).ok();

    Ok(VentaCompleta {
        venta: venta_result,
        detalles,
        cliente_nombre,
    })
}

// --- Choferes (autocompletar) ---

#[tauri::command]
pub fn listar_choferes(db: State<Database>) -> Result<Vec<(i64, String, Option<String>)>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let mut stmt = conn.prepare("SELECT id, nombre, placa FROM choferes ORDER BY nombre")
        .map_err(|e| e.to_string())?;
    let rows = stmt.query_map([], |row| {
        Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?, row.get::<_, Option<String>>(2)?))
    }).map_err(|e| e.to_string())?
    .collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())?;
    Ok(rows)
}

#[tauri::command]
pub fn guardar_chofer(db: State<Database>, nombre: String, placa: Option<String>) -> Result<(), String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    conn.execute(
        "INSERT OR REPLACE INTO choferes (nombre, placa) VALUES (?1, ?2)",
        rusqlite::params![nombre.trim(), placa.as_deref().map(|p| p.trim())],
    ).map_err(|e| e.to_string())?;
    Ok(())
}

// === Vehiculos guardados (autocomplete de placas en guias) ===
#[tauri::command]
pub fn listar_vehiculos(db: State<Database>) -> Result<Vec<(i64, String, Option<String>)>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let mut stmt = conn.prepare("SELECT id, placa, descripcion FROM vehiculos_transporte ORDER BY placa")
        .map_err(|e| e.to_string())?;
    let rows = stmt.query_map([], |row| {
        Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?, row.get::<_, Option<String>>(2)?))
    }).map_err(|e| e.to_string())?
    .collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())?;
    Ok(rows)
}

#[tauri::command]
pub fn guardar_vehiculo(db: State<Database>, placa: String, descripcion: Option<String>) -> Result<(), String> {
    let placa = placa.trim().to_uppercase();
    if placa.is_empty() { return Err("Placa vacia".to_string()); }
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    // INSERT OR REPLACE no funciona limpio por el UNIQUE — usamos INSERT OR IGNORE + UPDATE descripcion
    let _ = conn.execute(
        "INSERT OR IGNORE INTO vehiculos_transporte (placa, descripcion) VALUES (?1, ?2)",
        rusqlite::params![placa, descripcion.as_deref().map(|d| d.trim())],
    );
    if let Some(desc) = descripcion {
        if !desc.trim().is_empty() {
            let _ = conn.execute(
                "UPDATE vehiculos_transporte SET descripcion = ?1 WHERE placa = ?2",
                rusqlite::params![desc.trim(), placa],
            );
        }
    }
    Ok(())
}

// === Direcciones de entrega del cliente (autocomplete en guias) ===
#[tauri::command]
pub fn listar_direcciones_cliente(db: State<Database>, cliente_id: i64) -> Result<Vec<serde_json::Value>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let mut stmt = conn.prepare(
        "SELECT id, direccion, etiqueta, contacto_nombre, contacto_telefono, referencia
         FROM direcciones_cliente WHERE cliente_id = ?1
         ORDER BY id DESC"
    ).map_err(|e| e.to_string())?;
    let rows = stmt.query_map(rusqlite::params![cliente_id], |row| {
        Ok(serde_json::json!({
            "id": row.get::<_, i64>(0)?,
            "direccion": row.get::<_, String>(1)?,
            "etiqueta": row.get::<_, Option<String>>(2)?,
            "contacto_nombre": row.get::<_, Option<String>>(3)?,
            "contacto_telefono": row.get::<_, Option<String>>(4)?,
            "referencia": row.get::<_, Option<String>>(5)?,
        }))
    }).map_err(|e| e.to_string())?
    .collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())?;
    Ok(rows)
}

#[tauri::command]
pub fn guardar_direccion_cliente(
    db: State<Database>,
    cliente_id: i64,
    direccion: String,
    etiqueta: Option<String>,
) -> Result<i64, String> {
    let dir = direccion.trim();
    if dir.is_empty() { return Err("Direccion vacia".to_string()); }
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    // Evitar duplicados exactos para el mismo cliente
    let existe: Option<i64> = conn.query_row(
        "SELECT id FROM direcciones_cliente WHERE cliente_id = ?1 AND direccion = ?2 LIMIT 1",
        rusqlite::params![cliente_id, dir], |r| r.get(0),
    ).ok();
    if let Some(id) = existe {
        // Actualizar etiqueta si vino una nueva
        if let Some(et) = etiqueta {
            if !et.trim().is_empty() {
                let _ = conn.execute(
                    "UPDATE direcciones_cliente SET etiqueta = ?1 WHERE id = ?2",
                    rusqlite::params![et.trim(), id],
                );
            }
        }
        return Ok(id);
    }
    conn.execute(
        "INSERT INTO direcciones_cliente (cliente_id, direccion, etiqueta) VALUES (?1, ?2, ?3)",
        rusqlite::params![cliente_id, dir, etiqueta.as_deref().map(|e| e.trim())],
    ).map_err(|e| e.to_string())?;
    Ok(conn.last_insert_rowid())
}

#[tauri::command]
pub fn eliminar_direccion_cliente(db: State<Database>, id: i64) -> Result<(), String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    conn.execute("DELETE FROM direcciones_cliente WHERE id = ?1", rusqlite::params![id])
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn cambiar_estado_guia(
    db: State<Database>,
    sesion: State<SesionState>,
    guia_id: i64,
    nuevo_estado: String,
) -> Result<(), String> {
    // Verificar sesión activa
    let sesion_guard = sesion.sesion.lock().map_err(|e| e.to_string())?;
    let _sesion_actual = sesion_guard
        .as_ref()
        .ok_or("Debe iniciar sesión".to_string())?;
    drop(sesion_guard);

    // Validar nuevo estado
    if nuevo_estado != "ENTREGADA" && nuevo_estado != "RECHAZADA" {
        return Err("Estado no válido. Use ENTREGADA o RECHAZADA".to_string());
    }

    let conn = db.conn.lock().map_err(|e| e.to_string())?;

    // Verificar que la guía existe y es una guía de remisión pendiente
    let estado_actual: String = conn
        .query_row(
            "SELECT estado FROM ventas WHERE id = ?1 AND tipo_estado = 'GUIA_REMISION'",
            rusqlite::params![guia_id],
            |row| row.get(0),
        )
        .map_err(|_| "Guía de remisión no encontrada".to_string())?;

    // No permitir cambiar desde COMPLETADA (cerrada/convertida)
    if estado_actual == "COMPLETADA" {
        return Err("No se puede cambiar el estado de una guía cerrada/convertida".to_string());
    }

    // No permitir cambiar si ya está en un estado final
    if estado_actual == "ENTREGADA" || estado_actual == "RECHAZADA" {
        return Err(format!("La guía ya está en estado {}", estado_actual));
    }

    // Si RECHAZADA: devolver stock
    if nuevo_estado == "RECHAZADA" {
        // Multi-almacén: obtener ID del establecimiento
        let terminal_est: String = conn
            .query_row("SELECT value FROM config WHERE key = 'terminal_establecimiento'", [], |row| row.get(0))
            .unwrap_or_else(|_| "001".to_string());
        let multi_almacen: bool = conn
            .query_row("SELECT value FROM config WHERE key = 'multi_almacen_activo'", [], |row| row.get::<_, String>(0))
            .map(|v| v == "1")
            .unwrap_or(false);
        let est_id: Option<i64> = if multi_almacen {
            conn.query_row("SELECT id FROM establecimientos WHERE codigo = ?1", rusqlite::params![terminal_est], |row| row.get(0)).ok()
        } else {
            None
        };

        // Obtener items de la guía y devolver stock
        let mut stmt = conn.prepare(
            "SELECT producto_id, cantidad FROM venta_detalles WHERE venta_id = ?1"
        ).map_err(|e| e.to_string())?;

        let items: Vec<(i64, f64)> = stmt.query_map(rusqlite::params![guia_id], |row| {
            Ok((row.get::<_, i64>(0)?, row.get::<_, f64>(1)?))
        }).map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

        for (producto_id, cantidad) in &items {
            // Devolver stock al producto
            conn.execute(
                "UPDATE productos SET stock_actual = stock_actual + ?1,
                 updated_at = datetime('now','localtime')
                 WHERE id = ?2 AND es_servicio = 0",
                rusqlite::params![cantidad, producto_id],
            ).ok();

            // Multi-almacén: devolver stock a stock_establecimiento
            if let Some(eid) = est_id {
                conn.execute(
                    "UPDATE stock_establecimiento SET stock_actual = stock_actual + ?1
                     WHERE producto_id = ?2 AND establecimiento_id = ?3",
                    rusqlite::params![cantidad, producto_id, eid],
                ).ok();
            }
        }
    }

    // Actualizar estado de la guía
    conn.execute(
        "UPDATE ventas SET estado = ?1 WHERE id = ?2 AND tipo_estado = 'GUIA_REMISION'",
        rusqlite::params![nuevo_estado, guia_id],
    ).map_err(|e| e.to_string())?;

    Ok(())
}

/// Anula una venta NO autorizada por el SRI (FACTURA PENDIENTE/RECHAZADA o NOTA_VENTA).
/// Reintegra el stock, elimina CXC/pagos asociados y marca anulada=1.
/// Si la factura esta AUTORIZADA por SRI, NO permite anular (debe usar Nota de Credito).
#[tauri::command]
pub fn anular_venta(
    db: State<Database>,
    sesion: State<SesionState>,
    venta_id: i64,
    motivo: String,
    // v2.3.50: si la venta fue EFECTIVO/MIXTO con efectivo, este flag indica si el cajero
    // YA devolvio el dinero al cliente (caso normal: cliente reclamó). Si es false, el
    // cajero conservó el efectivo (caso: anulación por error contable, cliente nunca llegó).
    // Default true para mantener compatibilidad con el flujo anterior.
    efectivo_devuelto: Option<bool>,
) -> Result<(), String> {
    // Verificar sesion y permisos (solo admin o con permiso crear_nota_credito)
    let sesion_guard = sesion.sesion.lock().map_err(|e| e.to_string())?;
    let sesion_actual = sesion_guard
        .as_ref()
        .ok_or("Debe iniciar sesion".to_string())?;
    let usuario_rol = sesion_actual.rol.clone();
    let usuario_permisos = sesion_actual.permisos.clone();
    let usuario_nombre = sesion_actual.nombre.clone();
    drop(sesion_guard);

    if usuario_rol != "ADMIN" {
        let tiene_permiso = serde_json::from_str::<serde_json::Value>(&usuario_permisos)
            .ok()
            .and_then(|v| v.get("crear_nota_credito")?.as_bool())
            .unwrap_or(false);
        if !tiene_permiso {
            return Err("No tiene permisos para anular ventas".to_string());
        }
    }

    if motivo.trim().is_empty() {
        return Err("Debe ingresar un motivo para la anulacion".to_string());
    }

    let conn = db.conn.lock().map_err(|e| e.to_string())?;

    // Leer venta
    let (estado_sri, _tipo_documento, anulada, numero, total): (String, String, i32, String, f64) = conn.query_row(
        "SELECT COALESCE(estado_sri, 'NO_APLICA'), tipo_documento, anulada, numero, total FROM ventas WHERE id = ?1",
        rusqlite::params![venta_id],
        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?))
    ).map_err(|_| "Venta no encontrada".to_string())?;

    if anulada != 0 {
        return Err("La venta ya esta anulada".to_string());
    }
    if estado_sri == "AUTORIZADA" {
        return Err("No se puede anular una factura AUTORIZADA por el SRI. Debe crear una Nota de Credito.".to_string());
    }

    // v2.3.49 FIX CRITICO: si la venta ya tiene una nota de credito (devolucion),
    // NO permitir anular. Anular ademas reintegraria stock que ya devolvio la NC,
    // causando duplicacion. Si el usuario quiere "anular", debe primero revertir
    // la NC o aceptar que la NC ya hizo el efecto contable.
    let nc_count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM notas_credito WHERE venta_id = ?1",
        rusqlite::params![venta_id], |r| r.get(0),
    ).unwrap_or(0);
    if nc_count > 0 {
        return Err(format!(
            "No se puede anular esta venta — ya tiene {} nota(s) de credito (devolucion). \
             La devolucion ya revirtio el stock y el dinero. Si necesitas hacer mas ajustes, \
             registra otra devolucion (NC) en lugar de anular.",
            nc_count
        ));
    }

    // Multi-almacen: obtener est_id para revertir stock por establecimiento
    let multi_almacen: bool = conn
        .query_row("SELECT value FROM config WHERE key = 'multi_almacen_activo'", [], |row| row.get::<_, String>(0))
        .map(|v| v == "1")
        .unwrap_or(false);
    let est_id: Option<i64> = if multi_almacen {
        let terminal_est: String = conn.query_row(
            "SELECT value FROM config WHERE key = 'terminal_establecimiento'", [],
            |row| row.get(0)
        ).unwrap_or_else(|_| "001".to_string());
        conn.query_row(
            "SELECT id FROM establecimientos WHERE codigo = ?1",
            rusqlite::params![terminal_est], |row| row.get(0)
        ).ok()
    } else { None };

    // Reintegrar stock de cada item (considerando factor_unidad para multi-unidad)
    // y lote_id para reintegrar al lote correspondiente
    let mut stmt = conn.prepare(
        "SELECT producto_id, cantidad, COALESCE(factor_unidad, 1) as factor, lote_id FROM venta_detalles WHERE venta_id = ?1"
    ).map_err(|e| e.to_string())?;
    let items: Vec<(i64, f64, f64, Option<i64>)> = stmt.query_map(rusqlite::params![venta_id], |row| {
        Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?))
    }).map_err(|e| e.to_string())?
    .filter_map(|r| r.ok())
    .collect();
    drop(stmt);

    for (prod_id, cant, factor, lote_id) in &items {
        let cant_base = cant * factor;
        let omite: bool = conn.query_row(
            "SELECT (COALESCE(es_servicio, 0) + COALESCE(no_controla_stock, 0)) > 0 FROM productos WHERE id = ?1",
            rusqlite::params![prod_id],
            |row| row.get::<_, i64>(0).map(|v| v > 0)
        ).unwrap_or(false);
        if omite { continue; }

        // Reversar al lote si existe
        if let Some(lid) = lote_id {
            conn.execute(
                "UPDATE lotes_caducidad SET cantidad = cantidad + ?1 WHERE id = ?2",
                rusqlite::params![cant_base, lid],
            ).ok();
        }

        let stock_antes: f64 = conn.query_row(
            "SELECT stock_actual FROM productos WHERE id = ?1",
            rusqlite::params![prod_id], |row| row.get(0)
        ).unwrap_or(0.0);
        conn.execute(
            "UPDATE productos SET stock_actual = stock_actual + ?1, updated_at = datetime('now','localtime') WHERE id = ?2",
            rusqlite::params![cant_base, prod_id]
        ).ok();

        if let Some(eid) = est_id {
            conn.execute(
                "UPDATE stock_establecimiento SET stock_actual = stock_actual + ?1
                 WHERE producto_id = ?2 AND establecimiento_id = ?3",
                rusqlite::params![cant_base, prod_id, eid]
            ).ok();
        }

        conn.execute(
            "INSERT INTO movimientos_inventario (producto_id, tipo, cantidad, stock_anterior, stock_nuevo, motivo, usuario, referencia_id, establecimiento_id)
             VALUES (?1, 'ANULACION_VENTA', ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            rusqlite::params![prod_id, cant_base, stock_antes, stock_antes + cant_base,
                format!("Anulacion venta {} - {}", numero, motivo.trim()), usuario_nombre, venta_id, est_id]
        ).ok();
    }

    // v2.3.49: calcular EFECTIVO real de la venta ANTES de borrar pagos_venta.
    // Necesario para revertir monto_esperado abajo segun la porcion efectivo real.
    let (forma_pago_v, es_fiado_v): (String, i32) = conn.query_row(
        "SELECT forma_pago, COALESCE(estado, '') = 'PENDIENTE' AS es_fiado FROM ventas WHERE id = ?1",
        rusqlite::params![venta_id], |r| Ok((r.get(0)?, r.get::<_, bool>(1)? as i32)),
    ).unwrap_or(("EFECTIVO".to_string(), 0));
    let efectivo_de_venta: f64 = if forma_pago_v == "MIXTO" {
        conn.query_row(
            "SELECT COALESCE(SUM(monto), 0) FROM pagos_venta
             WHERE venta_id = ?1 AND UPPER(forma_pago) = 'EFECTIVO'",
            rusqlite::params![venta_id], |r| r.get(0),
        ).unwrap_or(0.0)
    } else if forma_pago_v == "EFECTIVO" && es_fiado_v == 0 {
        total
    } else {
        0.0
    };
    // Si el cajero devolvio el efectivo al cliente (default true) → restar de
    // monto_esperado (caja sale el dinero). Si NO lo devolvio (anulación por
    // error contable, cliente nunca llegó) → no restar; el efectivo queda en
    // caja como sobrante explicable al cierre.
    let efectivo_a_restar: f64 = if efectivo_devuelto.unwrap_or(true) {
        efectivo_de_venta
    } else {
        0.0
    };

    // Eliminar CXC y sus pagos
    conn.execute("DELETE FROM pagos_cuenta WHERE cuenta_id IN (SELECT id FROM cuentas_por_cobrar WHERE venta_id = ?1)",
        rusqlite::params![venta_id]).ok();
    conn.execute("DELETE FROM cuentas_por_cobrar WHERE venta_id = ?1",
        rusqlite::params![venta_id]).ok();

    // Eliminar pagos mixtos de esta venta
    conn.execute("DELETE FROM pagos_venta WHERE venta_id = ?1",
        rusqlite::params![venta_id]).ok();

    // Marcar venta como anulada
    let obs_anulacion = format!("ANULADA por {}: {}", usuario_nombre, motivo.trim());
    conn.execute(
        "UPDATE ventas SET anulada = 1, estado = 'ANULADA',
         observacion = COALESCE(observacion || ' | ', '') || ?1
         WHERE id = ?2",
        rusqlite::params![obs_anulacion, venta_id]
    ).map_err(|e| e.to_string())?;

    // v2.3.49 FIX: revertir monto_ventas (todas las formas) y monto_esperado
    // (solo porcion EFECTIVO, calculada antes de borrar pagos_venta arriba).
    // Antes solo se restaba monto_ventas, dejando monto_esperado inflado por
    // efectivo fantasma de ventas anuladas → cierre con descuadre falso.
    conn.execute(
        "UPDATE caja SET monto_ventas = monto_ventas - ?1, monto_esperado = monto_esperado - ?2
         WHERE estado = 'ABIERTA'",
        rusqlite::params![total, efectivo_a_restar]
    ).ok();

    Ok(())
}
