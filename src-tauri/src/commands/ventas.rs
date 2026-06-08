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
    // Permiso para que un cajero de confianza auto-confirme sus transferencias.
    let puede_autoconfirmar_transfer = es_admin
        || serde_json::from_str::<serde_json::Value>(&sesion_actual.permisos)
            .ok()
            .and_then(|v| v.get("autoconfirmar_transferencias")?.as_bool())
            .unwrap_or(false);
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
        // v2.5.20 BUG FIX: para COMBOS hay que validar stock de COMPONENTES,
        // no del padre (que siempre es 0 porque combos no tienen stock propio).
        //
        // Estrategia: armar un mapa de stock REQUERIDO por producto físico final.
        // - Producto simple: pid → cantidad * factor
        // - Combo fijo: cada componente → cantidad_combo_vendido * cantidad_componente
        // - Combo flexible: usar item.combo_seleccion
        let mut requerido: std::collections::HashMap<i64, f64> = std::collections::HashMap::new();
        for it in &venta.items {
            let Some(pid) = it.producto_id else { continue };
            let factor = it.factor_unidad.unwrap_or(1.0);
            let cant_total = it.cantidad * factor;

            // ¿Es combo? Chequear tipo_producto + presencia de componentes
            let (tipo_prod, tiene_componentes): (String, i64) = conn.query_row(
                "SELECT COALESCE(tipo_producto, 'SIMPLE'),
                        (SELECT COUNT(*) FROM producto_componentes WHERE producto_padre_id = productos.id)
                 FROM productos WHERE id = ?1",
                rusqlite::params![pid],
                |r| Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?)),
            ).unwrap_or(("SIMPLE".to_string(), 0));
            let es_combo = tipo_prod == "COMBO_FIJO" || tipo_prod == "COMBO_FLEXIBLE" || tiene_componentes > 0;

            if !es_combo {
                // Producto simple: cargar cantidad del producto base
                *requerido.entry(pid).or_insert(0.0) += cant_total;
            } else if tipo_prod == "COMBO_FLEXIBLE" {
                // Usar combo_seleccion (lo que el cajero escogió en el momento)
                if let Some(sel) = &it.combo_seleccion {
                    for c in sel {
                        if c.cantidad > 0.0 {
                            *requerido.entry(c.producto_hijo_id).or_insert(0.0) += c.cantidad * cant_total;
                        }
                    }
                }
            } else {
                // COMBO_FIJO (o producto con componentes detectados): leer de producto_componentes
                let mut stmt = conn.prepare(
                    "SELECT producto_hijo_id, cantidad FROM producto_componentes WHERE producto_padre_id = ?1"
                ).map_err(|e| e.to_string())?;
                let rows = stmt.query_map(rusqlite::params![pid], |r| Ok((r.get::<_, i64>(0)?, r.get::<_, f64>(1)?)))
                    .map_err(|e| e.to_string())?;
                for row in rows {
                    let (hijo_id, cant_componente) = row.map_err(|e| e.to_string())?;
                    *requerido.entry(hijo_id).or_insert(0.0) += cant_componente * cant_total;
                }
            }
        }

        for (pid, cant_req) in &requerido {
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

    // v2.3.52: si monto_recibido es 0 o menor en EFECTIVO/TARJETA y no es fiado/mixto,
    // asumir "monto exacto" (caso super comun cuando el cajero presiona Cobrar sin
    // tipear nada — espera que el sistema asuma que recibio el total exacto).
    // Antes la validacion v2.3.50 bloqueaba esto con error "monto < total" innecesariamente.
    let usa_mixto = venta.pagos.as_ref().map(|p| !p.is_empty()).unwrap_or(false);
    let forma_up = venta.forma_pago.to_uppercase();
    let monto_recibido_efectivo = r2(if !venta.es_fiado && !usa_mixto
        && matches!(forma_up.as_str(), "EFECTIVO" | "TARJETA")
        && venta.monto_recibido < 0.01
    {
        total // asumir monto exacto
    } else {
        venta.monto_recibido
    });

    // Redondear el cambio a centavos para evitar arrastre de float (ej. 2.9999 -> 3.00)
    let cambio = if monto_recibido_efectivo > total {
        r2(monto_recibido_efectivo - total)
    } else {
        0.0
    };

    // v2.3.50 VALIDACION (ajustada en v2.3.52): solo fallar si el cajero EXPLICITAMENTE
    // ingreso un monto > 0 pero menor al total. Si dejo en 0 → ya lo tratamos como exacto arriba.
    if !venta.es_fiado && !usa_mixto {
        if matches!(forma_up.as_str(), "EFECTIVO" | "TARJETA")
            && monto_recibido_efectivo > 0.01  // intentonalmente puso un valor
            && monto_recibido_efectivo + 0.01 < total
        {
            return Err(format!(
                "Monto recibido (${:.2}) es menor al total (${:.2}). Diferencia: ${:.2}. \
                 Si el cliente queda debiendo, marca la venta como CREDITO. \
                 Si paga con varios metodos, usa Pago Mixto. \
                 Si recibiste el monto exacto, deja el campo en 0 o vacio.",
                monto_recibido_efectivo, total, total - monto_recibido_efectivo
            ));
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
        else if puede_autoconfirmar_transfer { "VERIFICADO" }
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
            monto_recibido_efectivo,
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
                ).unwrap_or_else(|_| format!("ID {}", item.producto_id.unwrap_or(0)));
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

        // v2.5.18 SELF-HEALING: asegurar que la columna tipo_producto exista
        // (en BDs viejas el ALTER pudo no haberse ejecutado correctamente)
        let _ = conn.execute("ALTER TABLE productos ADD COLUMN tipo_producto TEXT NOT NULL DEFAULT 'SIMPLE'", []);

        // Obtener stock antes de descontar y verificar si es servicio / no_controla_stock / combo
        let (stock_antes, es_servicio, no_controla_stock, tipo_producto): (f64, bool, bool, String) = conn
            .query_row(
                "SELECT stock_actual, es_servicio, COALESCE(no_controla_stock, 0), COALESCE(tipo_producto, 'SIMPLE') FROM productos WHERE id = ?1",
                rusqlite::params![item.producto_id],
                |row| Ok((row.get::<_, f64>(0)?, row.get::<_, bool>(1)?, row.get::<_, i32>(2)? != 0, row.get::<_, String>(3)?)),
            )
            .unwrap_or((0.0, false, false, "SIMPLE".to_string()));

        // v2.5.18 DEFENSIVA: aunque tipo_producto sea "SIMPLE", si el producto TIENE
        // componentes registrados, tratarlo como COMBO_FIJO. Esto cubre casos donde
        // tipo_producto se perdió por bug de schema pero la estructura del combo está OK.
        let tiene_componentes: i64 = conn.query_row(
            "SELECT COUNT(*) FROM producto_componentes WHERE producto_padre_id = ?1",
            rusqlite::params![item.producto_id],
            |r| r.get(0),
        ).unwrap_or(0);
        let es_combo_efectivo = tipo_producto == "COMBO_FIJO"
            || tipo_producto == "COMBO_FLEXIBLE"
            || (tiene_componentes > 0 && tipo_producto != "COMBO_FLEXIBLE");
        let es_combo = es_combo_efectivo;
        // Si detectamos componentes pero tipo es SIMPLE, auto-corregir el tipo en BD
        // para que próximas ventas no necesiten esta heurística.
        if tiene_componentes > 0 && tipo_producto == "SIMPLE" {
            eprintln!("[Combo Auto-Fix] Producto {:?} tiene {} componentes pero tipo_producto='SIMPLE'. Auto-corrigiendo a COMBO_FIJO.", item.producto_id, tiene_componentes);
            let _ = conn.execute(
                "UPDATE productos SET tipo_producto = 'COMBO_FIJO' WHERE id = ?1",
                rusqlite::params![item.producto_id],
            );
        }
        // Para el resto del flujo, el tipo efectivo es el detectado
        let tipo_producto = if es_combo_efectivo && tipo_producto == "SIMPLE" {
            "COMBO_FIJO".to_string()
        } else {
            tipo_producto
        };
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
            // v2.5.28: grabar motivo con el numero visible (NV-XXXXXXXXX) para que
            // el kardex muestre "Venta NV-000000093" en lugar del id interno.
            let motivo_venta = format!("Venta {}", numero);
            let _ = conn.execute(
                "INSERT INTO movimientos_inventario (producto_id, tipo, cantidad, stock_anterior, stock_nuevo, costo_unitario, referencia_id, usuario, establecimiento_id, motivo)
                 VALUES (?1, 'VENTA', ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                rusqlite::params![
                    item.producto_id,
                    -(cantidad_base),
                    stock_antes,
                    stock_antes - cantidad_base,
                    precio_costo_prod,
                    venta_id,
                    usuario_nombre,
                    est_id,
                    motivo_venta,
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
                // v2.5.17 DIAGNOSTICO: si el combo no tiene componentes, registrar
                // en kardex un movimiento VENTA_COMBO_VACIO para que admin detecte
                // que el combo fue mal configurado (se vendio sin descontar nada).
                if componentes_a_descontar.is_empty() {
                    eprintln!("[Combo VACIO] Producto {:?} ({}) vendido como COMBO_FIJO pero no tiene componentes en producto_componentes. Stock NO descontado.", item.producto_id, nombre_prod);
                    let motivo_vacio = format!("Venta {} - COMBO SIN COMPONENTES ({})", numero, nombre_prod);
                    let _ = conn.execute(
                        "INSERT INTO movimientos_inventario (producto_id, tipo, cantidad, stock_anterior, stock_nuevo, costo_unitario, referencia_id, usuario, establecimiento_id, motivo)
                         VALUES (?1, 'VENTA_COMBO_VACIO', 0, 0, 0, 0, ?2, ?3, ?4, ?5)",
                        rusqlite::params![item.producto_id, venta_id, usuario_nombre, est_id, motivo_vacio],
                    );
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
                    let motivo_combo = format!("Venta {} (combo: {})", numero, nombre_prod);
                    let _ = conn.execute(
                        "INSERT INTO movimientos_inventario (producto_id, tipo, cantidad, stock_anterior, stock_nuevo, costo_unitario, referencia_id, usuario, establecimiento_id, motivo)
                         VALUES (?1, 'VENTA_COMBO', ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                        rusqlite::params![hijo_id, -cant_total, stock_h_antes, stock_h_antes - cant_total, costo_h, venta_id, usuario_nombre, est_id, motivo_combo],
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
            //
            // v2.5.15 SELF-HEALING: si la columna pago_estado no existe en la BD del cliente
            // (bug de migracion en instalaciones viejas), intentamos agregarla on-the-fly
            // antes del INSERT. Si igual falla, hacemos INSERT minimo sin esas columnas para
            // no perder la venta — luego UPDATE silencioso para setear pago_estado.
            let _ = conn.execute("ALTER TABLE pagos_venta ADD COLUMN pago_estado TEXT DEFAULT 'NO_APLICA'", []);
            let _ = conn.execute("ALTER TABLE pagos_venta ADD COLUMN verificado_por INTEGER", []);
            let _ = conn.execute("ALTER TABLE pagos_venta ADD COLUMN fecha_verificacion TEXT", []);
            let _ = conn.execute("ALTER TABLE pagos_venta ADD COLUMN motivo_verificacion TEXT", []);

            for p in pagos {
                let pf = p.forma_pago.to_uppercase();
                let es_pago_transfer = matches!(pf.as_str(), "TRANSFER" | "TRANSFERENCIA");
                let p_estado: &str = if !es_pago_transfer { "NO_APLICA" }
                    else if puede_autoconfirmar_transfer { "VERIFICADO" }
                    else { "REGISTRADO" };
                let p_verif_por: Option<i64> = if p_estado == "VERIFICADO" { Some(usuario_id) } else { None };
                let p_verif_fecha: Option<String> = if p_estado == "VERIFICADO" {
                    Some(chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string())
                } else { None };
                // Intento principal: INSERT con todas las columnas (caso normal)
                let res_full = conn.execute(
                    "INSERT INTO pagos_venta (venta_id, forma_pago, monto, banco_id, referencia, comprobante_imagen,
                                              pago_estado, verificado_por, fecha_verificacion)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                    rusqlite::params![venta_id, p.forma_pago, p.monto, p.banco_id, p.referencia, p.comprobante_imagen,
                                      p_estado, p_verif_por, p_verif_fecha],
                );
                if let Err(e) = res_full {
                    eprintln!("[VentaMixta] INSERT con pago_estado fallo: {} - intentando INSERT minimo", e);
                    // Fallback defensivo: INSERT sin las columnas nuevas (cliente con BD viejisima)
                    conn.execute(
                        "INSERT INTO pagos_venta (venta_id, forma_pago, monto, banco_id, referencia, comprobante_imagen)
                         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                        rusqlite::params![venta_id, p.forma_pago, p.monto, p.banco_id, p.referencia, p.comprobante_imagen],
                    ).map_err(|e2| format!("Error guardando pago (intento fallback tambien fallo): {}", e2))?;
                }
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
                despacho_estado: None,
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
                despacho_estado: None,
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
             v.caja_id, COALESCE(v.tipo_estado, 'COMPLETADA') as tipo_estado,
             COALESCE(v.anulada, 0) as anulada, v.despacho_estado
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
                    tipo_estado: row.get(26).ok(),
                    anulada: row.get::<_, i64>(27).ok(),
                    despacho_estado: row.get::<_, Option<String>>(28).ok().flatten(),
                    guia_placa: None, guia_chofer: None, guia_direccion_destino: None,
                })
            },
        )
        .map_err(|e| e.to_string())?;

    // v2.4.14: LEFT JOIN para incluir lineas de servicio (producto_id NULL).
    // Antes el INNER JOIN filtraba esas lineas y desaparecian del detalle.
    let mut stmt = conn
        .prepare(
            "SELECT d.id, d.venta_id, d.producto_id, p.nombre, d.cantidad,
             d.precio_unitario, d.descuento, d.iva_porcentaje, d.subtotal, d.info_adicional
             FROM venta_detalles d
             LEFT JOIN productos p ON d.producto_id = p.id
             WHERE d.venta_id = ?1",
        )
        .map_err(|e| e.to_string())?;

    let detalles = stmt
        .query_map(rusqlite::params![id], |row| {
            Ok(VentaDetalle {
                id: Some(row.get(0)?),
                venta_id: Some(row.get(1)?),
                producto_id: row.get(2)?,
                nombre_producto: row.get(3).ok(),
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

    // v2.5.16 BUG FIX: query incluye caja_id + cliente_nombre (antes hardcoded a None).
    // Tambien cambia WHERE a "date(fecha) = date('now')" para traer TODAS las ventas
    // del dia del cajero — el frontend filtra por chip "Solo sesion #X" opcionalmente.
    // Antes con WHERE v.fecha >= fecha_apertura, si fecha_apertura era de una sesion
    // vieja o si timezone divergia, las ventas nuevas no aparecian.
    let mut stmt = conn
        .prepare(
            "SELECT v.id, v.numero, v.cliente_id, v.fecha, v.subtotal_sin_iva, v.subtotal_con_iva,
             v.descuento, v.iva, v.total, v.forma_pago, v.monto_recibido, v.cambio, v.estado,
             v.tipo_documento, v.estado_sri, v.autorizacion_sri, v.clave_acceso, v.observacion,
             v.numero_factura, v.establecimiento, v.punto_emision,
             v.banco_id, v.referencia_pago, cb.nombre as banco_nombre,
             COALESCE(v.tipo_estado, 'COMPLETADA') as tipo_estado,
             v.caja_id, c.nombre as cliente_nombre
             FROM ventas v
             LEFT JOIN cuentas_banco cb ON v.banco_id = cb.id
             LEFT JOIN clientes c ON v.cliente_id = c.id
             WHERE (v.fecha >= ?1 OR date(v.fecha) = date('now','localtime'))
               AND COALESCE(v.anulada, 0) = 0
             ORDER BY v.fecha DESC, v.id DESC",
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
                caja_id: row.get(25).ok(),
                cliente_nombre: row.get(26).ok(),
                tipo_estado: row.get(24).ok(),
                guia_placa: None, guia_chofer: None, guia_direccion_destino: None,
                anulada: None,
                despacho_estado: None,
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

/// Resultado del cálculo + aplicación del reembolso de una NC/devolución.
/// Lo usan tanto `registrar_nota_credito` (SRI) como `crear_devolucion_interna`
/// para mantener una sola fuente de verdad sobre cómo se reembolsa el dinero
/// y cómo afecta la caja.
struct ReembolsoResultado {
    monto_efectivo: f64,
    monto_transfer: f64,
    monto_credito: f64,
    metodo_reembolso: String, // 'EFECTIVO' | 'TRANSFER' | 'CREDITO' | 'MIXTO'
    retiro_caja_id: Option<i64>,
}

/// Calcula el desglose del reembolso (efectivo/transfer/crédito) según la
/// forma de pago original de la venta, y si hay efectivo a devolver crea un
/// retiro automático en la caja abierta para mantener el cuadre.
///
/// Es la lógica COMPARTIDA entre NC SRI y devolución interna — antes solo la
/// devolución interna lo hacía, lo cual descuadraba la caja al hacer NC SRI
/// sobre ventas en efectivo (bug crítico v2.3.61 y anteriores).
fn calcular_y_aplicar_reembolso(
    conn: &rusqlite::Connection,
    venta_id: i64,
    venta_forma_pago: &str,
    venta_total: f64,
    total_nc: f64,
    nc_numero: &str,
    usuario_nombre: &str,
    usuario_id: i64,
) -> ReembolsoResultado {
    let proporcion = if venta_total > 0.01 { total_nc / venta_total } else { 1.0 };

    let (efectivo, transfer, credito) = if venta_forma_pago == "MIXTO" {
        // Sumar por forma desde pagos_venta y aplicar proporcion devuelta
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
        (efe * proporcion, tra * proporcion, cre * proporcion)
    } else {
        match venta_forma_pago.to_uppercase().as_str() {
            "EFECTIVO" => (total_nc, 0.0, 0.0),
            "TRANSFER" | "TRANSFERENCIA" => (0.0, total_nc, 0.0),
            "CREDITO" | "FIADO" => (0.0, 0.0, total_nc),
            _ => (total_nc, 0.0, 0.0),
        }
    };

    // Determinar método de reembolso textual
    let metodo = match (efectivo > 0.01, transfer > 0.01, credito > 0.01) {
        (true, false, false) => "EFECTIVO".to_string(),
        (false, true, false) => "TRANSFER".to_string(),
        (false, false, true) => "CREDITO".to_string(),
        _ => "MIXTO".to_string(),
    };

    // Si hay efectivo, crear retiro_caja automático para que la caja CUADRE.
    // Si no hay caja abierta, igual seguimos — la NC se persiste, el efectivo
    // simplemente no se compensa automáticamente (admin ajusta manual).
    let mut retiro_id: Option<i64> = None;
    if efectivo > 0.01 {
        if let Ok(caja_id) = conn.query_row(
            "SELECT id FROM caja WHERE estado = 'ABIERTA' LIMIT 1",
            [], |r| r.get::<_, i64>(0),
        ) {
            let motivo_retiro = format!("Devolución NC {} — efectivo a cliente", nc_numero);
            if conn.execute(
                "INSERT INTO retiros_caja (caja_id, monto, motivo, banco_id, referencia, usuario, usuario_id, estado)
                 VALUES (?1, ?2, ?3, NULL, NULL, ?4, ?5, 'SIN_DEPOSITO')",
                rusqlite::params![caja_id, efectivo, motivo_retiro, usuario_nombre, usuario_id],
            ).is_ok() {
                retiro_id = Some(conn.last_insert_rowid());
                let _ = conn.execute(
                    "UPDATE caja SET monto_esperado = monto_esperado - ?1 WHERE id = ?2",
                    rusqlite::params![efectivo, caja_id],
                );
            }
        }
    }

    ReembolsoResultado {
        monto_efectivo: efectivo,
        monto_transfer: transfer,
        monto_credito: credito,
        metodo_reembolso: metodo,
        retiro_caja_id: retiro_id,
    }
}

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

    // Validar que la venta original sea FACTURA AUTORIZADA + leer datos para reembolso
    let (factura_numero, cliente_id, venta_total, venta_forma_pago): (String, i64, f64, String) = conn
        .query_row(
            "SELECT numero, COALESCE(cliente_id, 1), total, COALESCE(forma_pago, 'EFECTIVO')
             FROM ventas WHERE id = ?1 AND tipo_documento = 'FACTURA' AND estado_sri = 'AUTORIZADA' AND anulada = 0",
            rusqlite::params![nota.venta_id],
            |row| Ok((row.get(0)?, row.get::<_, Option<i64>>(1)?.unwrap_or(1), row.get(2)?, row.get(3)?)),
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

    // === Reembolso (v2.3.62 fix critico) ===
    // Antes: NC SRI no tocaba caja → cierres descuadrados al hacer NC sobre venta efectivo.
    // Ahora: usamos el mismo helper que devolución interna para calcular desglose
    // efectivo/transfer/credito y crear retiro_caja automatico si aplica.
    let reembolso = calcular_y_aplicar_reembolso(
        &conn, nota.venta_id, &venta_forma_pago, venta_total, total,
        &numero, &usuario_nombre, usuario_id,
    );

    // Determinar tipo (PARCIAL si total NC < total venta, TOTAL si igual)
    let tipo_devolucion = if (total - venta_total).abs() < 0.01 { "TOTAL" } else { "PARCIAL" };

    // Persistir desglose de reembolso en notas_credito (v2.3.62)
    let _ = conn.execute(
        "UPDATE notas_credito SET
            tipo_devolucion = ?1,
            monto_efectivo_devuelto = ?2,
            monto_transfer_devuelto = ?3,
            monto_credito_devuelto = ?4,
            retiro_caja_id = ?5,
            metodo_reembolso = ?6
         WHERE id = ?7",
        rusqlite::params![
            tipo_devolucion,
            reembolso.monto_efectivo,
            reembolso.monto_transfer,
            reembolso.monto_credito,
            reembolso.retiro_caja_id,
            reembolso.metodo_reembolso,
            nc_id,
        ],
    );

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

    // === Devolución del dinero al cliente — efecto sobre caja segun forma_pago ===
    // Refactorizado v2.3.62: usa helper compartido `calcular_y_aplicar_reembolso`
    // que ahora tambien usa registrar_nota_credito (SRI). Misma logica, sin duplicar.
    let venta_total_real: f64 = conn.query_row(
        "SELECT total FROM ventas WHERE id = ?1",
        rusqlite::params![venta_id], |r| r.get(0),
    ).unwrap_or(0.0);

    let reembolso = calcular_y_aplicar_reembolso(
        &conn, venta_id, &venta_forma_pago, venta_total_real, total,
        &numero, &usuario_nombre, usuario_id,
    );

    // Determinar tipo (PARCIAL si NC < venta, TOTAL si igual)
    let tipo_devolucion = if (total - venta_total_real).abs() < 0.01 { "TOTAL" } else { "PARCIAL" };

    // Persistir desglose de reembolso en notas_credito (v2.3.62)
    // Antes esta info se perdia despues de cerrar la app.
    let _ = conn.execute(
        "UPDATE notas_credito SET
            tipo_devolucion = ?1,
            monto_efectivo_devuelto = ?2,
            monto_transfer_devuelto = ?3,
            monto_credito_devuelto = ?4,
            retiro_caja_id = ?5,
            metodo_reembolso = ?6
         WHERE id = ?7",
        rusqlite::params![
            tipo_devolucion,
            reembolso.monto_efectivo,
            reembolso.monto_transfer,
            reembolso.monto_credito,
            reembolso.retiro_caja_id,
            reembolso.metodo_reembolso,
            nc_id,
        ],
    );

    Ok(serde_json::json!({
        "id": nc_id,
        "numero": numero,
        "venta_id": venta_id,
        "venta_numero": venta_numero,
        "motivo": motivo.trim(),
        "total": total,
        "estado_sri": "NO_APLICA",
        // Info para el frontend: que pasar al usuario segun forma_pago
        "monto_efectivo_devuelto": reembolso.monto_efectivo,
        "monto_transfer_devuelto": reembolso.monto_transfer,
        "monto_credito_devuelto": reembolso.monto_credito,
        "retiro_caja_creado_id": reembolso.retiro_caja_id,
        "metodo_reembolso": reembolso.metodo_reembolso,
        "tipo_devolucion": tipo_devolucion,
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

    // v2.3.62: incluimos columnas nuevas (tipo_devolucion, montos reembolso, metodo)
    // para que el listado pueda mostrar la info de reembolso sin tener que abrir cada NC
    let base_query = "SELECT nc.id, nc.numero, nc.venta_id, nc.motivo, nc.total, nc.estado_sri,
             nc.fecha, nc.clave_acceso, nc.autorizacion_sri, nc.numero_factura_nc,
             COALESCE(v.numero_factura, v.numero) as venta_numero,
             COALESCE(cl.nombre, 'CONSUMIDOR FINAL') as cliente_nombre,
             COALESCE(nc.tipo_devolucion, 'TOTAL'),
             COALESCE(nc.monto_efectivo_devuelto, 0),
             COALESCE(nc.monto_transfer_devuelto, 0),
             COALESCE(nc.monto_credito_devuelto, 0),
             COALESCE(nc.metodo_reembolso, 'EFECTIVO'),
             nc.retiro_caja_id
             FROM notas_credito nc
             LEFT JOIN ventas v ON nc.venta_id = v.id
             LEFT JOIN clientes cl ON nc.cliente_id = cl.id
             WHERE date(nc.fecha) >= date(?1) AND date(nc.fecha) <= date(?2)";

    let query = if let Some(ref _est) = estado {
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
                "tipo_devolucion": row.get::<_, String>(12)?,
                "monto_efectivo_devuelto": row.get::<_, f64>(13)?,
                "monto_transfer_devuelto": row.get::<_, f64>(14)?,
                "monto_credito_devuelto": row.get::<_, f64>(15)?,
                "metodo_reembolso": row.get::<_, String>(16)?,
                "retiro_caja_id": row.get::<_, Option<i64>>(17)?,
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
                "tipo_devolucion": row.get::<_, String>(12)?,
                "monto_efectivo_devuelto": row.get::<_, f64>(13)?,
                "monto_transfer_devuelto": row.get::<_, f64>(14)?,
                "monto_credito_devuelto": row.get::<_, f64>(15)?,
                "metodo_reembolso": row.get::<_, String>(16)?,
                "retiro_caja_id": row.get::<_, Option<i64>>(17)?,
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
                despacho_estado: None,
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

    // Una nota de entrega va dirigida a un cliente específico (se le entrega la
    // mercadería), por lo que NO puede emitirse a Consumidor Final (id=1) ni sin cliente.
    match venta.cliente_id {
        None => return Err("La nota de entrega debe tener un cliente. No se permite Consumidor Final.".to_string()),
        Some(1) => return Err("La nota de entrega no puede ser a nombre de Consumidor Final. Seleccione un cliente.".to_string()),
        _ => {}
    }

    let conn = db.conn.lock().map_err(|e| e.to_string())?;

    // Leer establecimiento y punto de emisión del terminal
    let terminal_est: String = conn
        .query_row("SELECT value FROM config WHERE key = 'terminal_establecimiento'", [], |row| row.get(0))
        .unwrap_or_else(|_| "001".to_string());
    let terminal_pe: String = conn
        .query_row("SELECT value FROM config WHERE key = 'terminal_punto_emision'", [], |row| row.get(0))
        .unwrap_or_else(|_| "001".to_string());

    // REFACTOR INVENTARIO: ya no se descuenta stock en la creación, por lo que
    // no necesitamos el ID del establecimiento aquí (se calcula en la recepción).

    // Obtener secuencial para guía de remisión
    conn.execute(
        "INSERT OR IGNORE INTO secuenciales (establecimiento_codigo, punto_emision_codigo, tipo_documento, secuencial) VALUES ('001', '001', 'GUIA_REMISION_SEQ', 1)",
        [],
    ).map_err(|e| e.to_string())?;

    let secuencial: i64 = conn.query_row(
        "SELECT secuencial FROM secuenciales WHERE establecimiento_codigo = '001' AND punto_emision_codigo = '001' AND tipo_documento = 'GUIA_REMISION_SEQ'",
        [], |row| row.get(0),
    ).map_err(|e| e.to_string())?;

    // NE = Nota de Entrega (coincide con la etiqueta de la UI). Es solo la
    // referencia interna; el secuencial oficial del SRI (si se emite la Guía de
    // Remisión electrónica) es independiente.
    let numero = format!("NE-{:06}", secuencial);

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

    // Insertar detalles (SIN descontar stock).
    //
    // REFACTOR INVENTARIO: la creación de la nota de entrega (estado 'PENDIENTE')
    // ya NO mueve inventario. El stock se descuenta exactamente UNA vez, cuando el
    // cliente RECIBE la mercadería (cambiar_estado_guia → ENTREGADA), soportando
    // recepción parcial. Si la nota se RECHAZA, nunca se descuenta. Si se factura
    // directamente sin pasar por recepción, convertir_guia_a_venta descuenta ahí.
    let mut detalles_guardados = Vec::new();
    for item in &venta.items {
        let subtotal = item.cantidad * item.precio_unitario - item.descuento;
        conn.execute(
            "INSERT INTO venta_detalles (venta_id, producto_id, cantidad, precio_unitario, descuento, iva_porcentaje, subtotal, info_adicional)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
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

    // Incrementar secuencial
    conn.execute(
        "UPDATE secuenciales SET secuencial = secuencial + 1 WHERE establecimiento_codigo = '001' AND punto_emision_codigo = '001' AND tipo_documento = 'GUIA_REMISION_SEQ'",
        [],
    ).map_err(|e| e.to_string())?;

    // v2.5.67: aprender la asociacion placa<->chofer automaticamente
    if let (Some(pl), Some(ch)) = (venta.guia_placa.as_ref(), venta.guia_chofer.as_ref()) {
        let pl = pl.trim().to_uppercase();
        let ch = ch.trim();
        if !pl.is_empty() && !ch.is_empty() {
            let _ = conn.execute(
                "INSERT INTO placa_chofer_asoc (placa, chofer) VALUES (?1, ?2)
                 ON CONFLICT(placa, chofer) DO UPDATE SET veces = veces + 1, ultima_vez = datetime('now','localtime')",
                rusqlite::params![pl, ch],
            );
        }
    }

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
                despacho_estado: None,
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
         cl.nombre as cliente_nombre, v.despacho_estado
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
                despacho_estado: row.get::<_, Option<String>>(26).ok().flatten(),
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
    // REFACTOR INVENTARIO: ya_recibida indica que el stock YA se descontó en la
    // recepción (ENTREGADA). Si la guía aún está PENDIENTE, el stock NUNCA se movió
    // y debemos descontarlo ahora al facturar (más abajo).
    let ya_recibida = guia_estado_actual == "ENTREGADA";

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

    // === MOVIMIENTO DE STOCK AL FACTURAR (REFACTOR INVENTARIO) ===
    //
    // El stock se mueve EXACTAMENTE UNA VEZ. Dos caminos:
    //
    //   A) ya_recibida (guía ENTREGADA): el stock YA se descontó en la recepción.
    //      La cantidad es fija (no editable). NO se vuelve a tocar el stock aquí.
    //      `ajustes_stock` está vacío en este caso (sólo se llena si PENDIENTE).
    //
    //   B) PENDIENTE (facturada directo, sin paso de recepción): el stock NUNCA
    //      se movió. Debemos descontar AHORA las cantidades finales (con overrides
    //      aplicados), exactamente una vez, con kardex tipo 'GUIA_REMISION'.
    if !ya_recibida {
        // Multi-almacen: ID del establecimiento
        let multi_almacen: bool = conn.query_row(
            "SELECT value FROM config WHERE key = 'multi_almacen_activo'", [],
            |r| r.get::<_, String>(0)
        ).map(|v| v == "1").unwrap_or(false);
        let est_id_fact: Option<i64> = if multi_almacen {
            conn.query_row("SELECT id FROM establecimientos WHERE codigo = ?1",
                rusqlite::params![terminal_est], |r| r.get(0)).ok()
        } else { None };

        // Descontar cantidades finales (una sola vez). referencia = nueva venta.
        for (pid, cant, _pu, _desc, _iva, _sub, _info) in &items_finales {
            if *cant <= 0.0 { continue; }
            let (stock_antes, es_servicio, no_controla_stock): (f64, bool, bool) = conn
                .query_row(
                    "SELECT stock_actual, es_servicio, COALESCE(no_controla_stock, 0) FROM productos WHERE id = ?1",
                    rusqlite::params![pid],
                    |row| Ok((row.get::<_, f64>(0)?, row.get::<_, bool>(1)?, row.get::<_, i32>(2)? != 0)),
                )
                .unwrap_or((0.0, false, false));
            let omite_stock = es_servicio || no_controla_stock;
            if omite_stock { continue; }

            conn.execute(
                "UPDATE productos SET stock_actual = stock_actual - ?1, updated_at = datetime('now','localtime')
                 WHERE id = ?2",
                rusqlite::params![cant, pid],
            ).map_err(|e| e.to_string())?;
            if let Some(eid) = est_id_fact {
                conn.execute(
                    "UPDATE stock_establecimiento SET stock_actual = stock_actual - ?1
                     WHERE producto_id = ?2 AND establecimiento_id = ?3",
                    rusqlite::params![cant, pid, eid],
                ).ok();
            }
            let costo_snap: f64 = conn.query_row(
                "SELECT precio_costo FROM productos WHERE id = ?1",
                rusqlite::params![pid], |r| r.get(0),
            ).unwrap_or(0.0);
            // referencia_id = nueva venta facturada (la NV) para trazabilidad.
            let _ = conn.execute(
                "INSERT INTO movimientos_inventario (producto_id, tipo, cantidad, stock_anterior, stock_nuevo, costo_unitario, referencia_id, usuario, establecimiento_id)
                 VALUES (?1, 'GUIA_REMISION', ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                rusqlite::params![pid, -(*cant), stock_antes, stock_antes - cant, costo_snap, guia_id, usuario_nombre, est_id_fact],
            );
        }
    }

    // Si hubo cambios de cantidad (sólo PENDIENTE), actualizar venta_detalles de la
    // guía para mantener consistencia con lo facturado (el listado muestra los items
    // actualizados). NOTA: el descuento de stock arriba ya usa las cantidades finales.
    if !ajustes_stock.is_empty() {
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
    // El stock ya se manejó arriba (descontado en la recepción si ENTREGADA, o
    // descontado recién al facturar si PENDIENTE). NO se toca stock al insertar.
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
    // SIN tocar stock aquí — ya se descontó arriba (en recepción ENTREGADA, o recién
    // al facturar si la guía estaba PENDIENTE). Se descuenta exactamente una vez.
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
                despacho_estado: None,
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

// === Aprendizaje placa <-> chofer <-> transportista (v2.5.67) ===

#[derive(serde::Serialize)]
pub struct SugerenciaTransporte {
    pub placa: String,
    pub chofer: String,
    pub transportista_ruc: Option<String>,
    pub transportista_nombre: Option<String>,
    pub veces: i64,
}

/// Aprende (o refuerza) la asociacion placa<->chofer<->transportista.
/// Se llama automaticamente al guardar/emitir una guia. Idempotente: si el par
/// (placa, chofer) ya existe, incrementa el contador y refresca el transportista.
#[tauri::command]
pub fn aprender_placa_chofer(
    db: State<Database>,
    placa: String,
    chofer: String,
    transportista_ruc: Option<String>,
    transportista_nombre: Option<String>,
) -> Result<(), String> {
    let placa_n = placa.trim().to_uppercase();
    let chofer_n = chofer.trim().to_string();
    if placa_n.is_empty() || chofer_n.is_empty() {
        return Ok(()); // nada que aprender
    }
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    conn.execute(
        "INSERT INTO placa_chofer_asoc (placa, chofer, transportista_ruc, transportista_nombre)
         VALUES (?1, ?2, ?3, ?4)
         ON CONFLICT(placa, chofer) DO UPDATE SET
            veces = veces + 1,
            ultima_vez = datetime('now','localtime'),
            transportista_ruc = COALESCE(NULLIF(excluded.transportista_ruc, ''), transportista_ruc),
            transportista_nombre = COALESCE(NULLIF(excluded.transportista_nombre, ''), transportista_nombre)",
        rusqlite::params![
            placa_n,
            chofer_n,
            transportista_ruc.as_deref().map(|s| s.trim()),
            transportista_nombre.as_deref().map(|s| s.trim()),
        ],
    ).map_err(|e| e.to_string())?;
    Ok(())
}

/// Dado una placa (o prefijo), sugiere los choferes/transportistas que la han
/// conducido, ordenados por frecuencia. Devuelve uno o varios.
#[tauri::command]
pub fn sugerir_por_placa(db: State<Database>, placa: String) -> Result<Vec<SugerenciaTransporte>, String> {
    let placa_n = placa.trim().to_uppercase();
    if placa_n.is_empty() { return Ok(vec![]); }
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let mut stmt = conn.prepare(
        "SELECT placa, chofer, transportista_ruc, transportista_nombre, veces
         FROM placa_chofer_asoc
         WHERE placa LIKE ?1 || '%'
         ORDER BY veces DESC, ultima_vez DESC
         LIMIT 10"
    ).map_err(|e| e.to_string())?;
    let rows = stmt.query_map(rusqlite::params![placa_n], |row| {
        Ok(SugerenciaTransporte {
            placa: row.get(0)?,
            chofer: row.get(1)?,
            transportista_ruc: row.get(2)?,
            transportista_nombre: row.get(3)?,
            veces: row.get(4)?,
        })
    }).map_err(|e| e.to_string())?
    .collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())?;
    Ok(rows)
}

/// Dado un chofer (o fragmento del nombre), sugiere las placas que ha conducido,
/// ordenadas por frecuencia. Devuelve una o varias.
#[tauri::command]
pub fn sugerir_por_chofer(db: State<Database>, chofer: String) -> Result<Vec<SugerenciaTransporte>, String> {
    let chofer_n = chofer.trim().to_string();
    if chofer_n.is_empty() { return Ok(vec![]); }
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let mut stmt = conn.prepare(
        "SELECT placa, chofer, transportista_ruc, transportista_nombre, veces
         FROM placa_chofer_asoc
         WHERE chofer LIKE '%' || ?1 || '%'
         ORDER BY veces DESC, ultima_vez DESC
         LIMIT 10"
    ).map_err(|e| e.to_string())?;
    let rows = stmt.query_map(rusqlite::params![chofer_n], |row| {
        Ok(SugerenciaTransporte {
            placa: row.get(0)?,
            chofer: row.get(1)?,
            transportista_ruc: row.get(2)?,
            transportista_nombre: row.get(3)?,
            veces: row.get(4)?,
        })
    }).map_err(|e| e.to_string())?
    .collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())?;
    Ok(rows)
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

/// Cantidad efectivamente recibida por el cliente para un producto, en una
/// recepción parcial. Tauri lo recibe como `itemsRecibidos` (camelCase) desde JS.
#[derive(serde::Deserialize)]
pub struct ItemRecibido {
    pub producto_id: i64,
    pub cantidad: f64,
}

#[tauri::command]
pub fn cambiar_estado_guia(
    db: State<Database>,
    sesion: State<SesionState>,
    guia_id: i64,
    nuevo_estado: String,
    // REFACTOR INVENTARIO: cantidades realmente recibidas (recepción parcial).
    // Si es None → recepción total (se recibe la cantidad original de cada item).
    items_recibidos: Option<Vec<ItemRecibido>>,
) -> Result<(), String> {
    // Verificar sesión activa y capturar el nombre del usuario para el kardex
    // ANTES de soltar el guard (lo necesitamos en el movimiento de inventario).
    let sesion_guard = sesion.sesion.lock().map_err(|e| e.to_string())?;
    let sesion_actual = sesion_guard
        .as_ref()
        .ok_or("Debe iniciar sesión".to_string())?;
    let usuario_nombre = sesion_actual.nombre.clone();
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

    // Multi-almacén: obtener ID del establecimiento (necesario al descontar stock
    // en la recepción ENTREGADA).
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

    // REFACTOR INVENTARIO: el stock se mueve (descuenta) AQUÍ, en la recepción.
    if nuevo_estado == "ENTREGADA" {
        // Mapa producto_id -> cantidad recibida (sólo si se envió recepción parcial).
        let recibidos_map: Option<std::collections::HashMap<i64, f64>> = items_recibidos
            .as_ref()
            .map(|v| v.iter().map(|it| (it.producto_id, it.cantidad)).collect());
        let es_parcial = recibidos_map.is_some();

        // Cargar los detalles de la guía.
        let mut stmt = conn.prepare(
            "SELECT id, producto_id, cantidad, precio_unitario, descuento
             FROM venta_detalles WHERE venta_id = ?1"
        ).map_err(|e| e.to_string())?;
        let detalles: Vec<(i64, i64, f64, f64, f64)> = stmt.query_map(rusqlite::params![guia_id], |row| {
            Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?))
        }).map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;
        drop(stmt);

        for (detalle_id, producto_id, cantidad_orig, precio_unitario, descuento) in &detalles {
            // Cantidad recibida: del mapa si existe, si no la cantidad original.
            // Se acota a [0, cantidad_original].
            let recibida_raw = recibidos_map
                .as_ref()
                .and_then(|m| m.get(producto_id).copied())
                .unwrap_or(*cantidad_orig);
            let recibida = recibida_raw.max(0.0).min(*cantidad_orig);

            // Leer estado del producto (stock + flags de control).
            let (stock_antes, es_servicio, no_controla_stock): (f64, bool, bool) = conn
                .query_row(
                    "SELECT stock_actual, es_servicio, COALESCE(no_controla_stock, 0) FROM productos WHERE id = ?1",
                    rusqlite::params![producto_id],
                    |row| Ok((row.get::<_, f64>(0)?, row.get::<_, bool>(1)?, row.get::<_, i32>(2)? != 0)),
                )
                .unwrap_or((0.0, false, false));
            let omite_stock = es_servicio || no_controla_stock;

            // Descontar SOLO lo realmente recibido (y sólo productos físicos).
            if recibida > 0.0 && !omite_stock {
                conn.execute(
                    "UPDATE productos SET stock_actual = stock_actual - ?1,
                     updated_at = datetime('now','localtime')
                     WHERE id = ?2",
                    rusqlite::params![recibida, producto_id],
                ).map_err(|e| e.to_string())?;

                if let Some(eid) = est_id {
                    conn.execute(
                        "UPDATE stock_establecimiento SET stock_actual = stock_actual - ?1
                         WHERE producto_id = ?2 AND establecimiento_id = ?3",
                        rusqlite::params![recibida, producto_id, eid],
                    ).ok();
                }

                // Kardex: salida por recepción de nota de entrega.
                let costo_snap: f64 = conn.query_row(
                    "SELECT precio_costo FROM productos WHERE id = ?1",
                    rusqlite::params![producto_id],
                    |row| row.get(0)
                ).unwrap_or(0.0);
                let _ = conn.execute(
                    "INSERT INTO movimientos_inventario (producto_id, tipo, cantidad, stock_anterior, stock_nuevo, costo_unitario, referencia_id, usuario, establecimiento_id)
                     VALUES (?1, 'GUIA_REMISION', ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                    rusqlite::params![
                        producto_id,
                        -recibida,
                        stock_antes,
                        stock_antes - recibida,
                        costo_snap,
                        guia_id,
                        usuario_nombre,
                        est_id,
                    ],
                );
            }

            // Recepción parcial: si la cantidad recibida difiere de la original,
            // la nota refleja lo realmente entregado (ajustamos el detalle).
            if es_parcial && (recibida - cantidad_orig).abs() > 0.0001 {
                let nuevo_subtotal = (recibida * precio_unitario - descuento).max(0.0);
                conn.execute(
                    "UPDATE venta_detalles SET cantidad = ?1, subtotal = ?2 WHERE id = ?3",
                    rusqlite::params![recibida, nuevo_subtotal, detalle_id],
                ).map_err(|e| e.to_string())?;
            }
        }

        // Si fue recepción parcial, recalcular el total de la cabecera desde los
        // detalles ya actualizados. (IVA en estas notas suele ser 0.)
        if es_parcial {
            conn.execute(
                "UPDATE ventas SET total = (SELECT COALESCE(SUM(subtotal),0) FROM venta_detalles WHERE venta_id = ?1) WHERE id = ?1",
                rusqlite::params![guia_id],
            ).map_err(|e| e.to_string())?;
        }
    }

    // REFACTOR INVENTARIO: RECHAZADA no toca stock (nunca se descontó al crear).

    // Actualizar estado de la guía
    conn.execute(
        "UPDATE ventas SET estado = ?1 WHERE id = ?2 AND tipo_estado = 'GUIA_REMISION'",
        rusqlite::params![nuevo_estado, guia_id],
    ).map_err(|e| e.to_string())?;

    Ok(())
}

/// Datos SRI de una guía de remisión electrónica (codDoc 06).
/// Cada campo es opcional: solo se actualiza el que viene con valor.
#[derive(serde::Deserialize)]
pub struct GuiaDatosSri {
    pub transportista: Option<String>,
    pub ruc_transportista: Option<String>,
    pub tipo_id_transportista: Option<String>,
    pub dir_partida: Option<String>,
    pub fecha_inicio_transporte: Option<String>,
    pub fecha_fin_transporte: Option<String>,
    pub motivo_traslado: Option<String>,
    pub ruta: Option<String>,
    pub cod_doc_sustento: Option<String>,
    pub num_doc_sustento: Option<String>,
    pub num_aut_sustento: Option<String>,
    pub fecha_emision_sustento: Option<String>,
    pub placa: Option<String>,
    pub direccion_destino: Option<String>,
}

/// Guarda/actualiza los datos SRI de una guía de remisión (transportista, fechas,
/// motivo, ruta, doc sustento) antes de emitirla electrónicamente al SRI.
/// No toca el flujo de creación; solo hace UPDATE de las columnas guia_*.
#[tauri::command]
pub fn guia_guardar_datos_sri(
    db: State<Database>,
    guia_id: i64,
    datos: GuiaDatosSri,
) -> Result<(), String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let upd = |campo: &str, val: &Option<String>| -> Result<(), String> {
        if let Some(v) = val {
            // `campo` siempre es un literal hardcodeado (no input del usuario) → seguro.
            conn.execute(
                &format!("UPDATE ventas SET {} = ?1 WHERE id = ?2 AND tipo_estado = 'GUIA_REMISION'", campo),
                rusqlite::params![v, guia_id],
            ).map_err(|e| e.to_string())?;
        }
        Ok(())
    };
    upd("guia_transportista", &datos.transportista)?;
    upd("guia_ruc_transportista", &datos.ruc_transportista)?;
    upd("guia_tipo_id_transportista", &datos.tipo_id_transportista)?;
    upd("guia_dir_partida", &datos.dir_partida)?;
    upd("guia_fecha_inicio_transporte", &datos.fecha_inicio_transporte)?;
    upd("guia_fecha_fin_transporte", &datos.fecha_fin_transporte)?;
    upd("guia_motivo_traslado", &datos.motivo_traslado)?;
    upd("guia_ruta", &datos.ruta)?;
    upd("guia_cod_doc_sustento", &datos.cod_doc_sustento)?;
    upd("guia_num_doc_sustento", &datos.num_doc_sustento)?;
    upd("guia_num_aut_sustento", &datos.num_aut_sustento)?;
    upd("guia_fecha_emision_sustento", &datos.fecha_emision_sustento)?;
    upd("guia_placa", &datos.placa)?;
    upd("guia_direccion_destino", &datos.direccion_destino)?;

    // v2.5.67: aprender placa<->chofer<->transportista al guardar datos SRI
    let placa_l = datos.placa.as_deref().map(|s| s.trim().to_uppercase()).unwrap_or_default();
    if !placa_l.is_empty() {
        let chofer_l: String = conn.query_row(
            "SELECT COALESCE(guia_chofer,'') FROM ventas WHERE id = ?1",
            rusqlite::params![guia_id], |r| r.get(0),
        ).unwrap_or_default();
        if !chofer_l.trim().is_empty() {
            let _ = conn.execute(
                "INSERT INTO placa_chofer_asoc (placa, chofer, transportista_ruc, transportista_nombre)
                 VALUES (?1, ?2, ?3, ?4)
                 ON CONFLICT(placa, chofer) DO UPDATE SET
                    veces = veces + 1, ultima_vez = datetime('now','localtime'),
                    transportista_ruc = COALESCE(NULLIF(excluded.transportista_ruc,''), transportista_ruc),
                    transportista_nombre = COALESCE(NULLIF(excluded.transportista_nombre,''), transportista_nombre)",
                rusqlite::params![
                    placa_l,
                    chofer_l.trim(),
                    datos.ruc_transportista.as_deref().map(|s| s.trim()),
                    datos.transportista.as_deref().map(|s| s.trim()),
                ],
            );
        }
    }
    Ok(())
}

/// v2.5.68 (Fase C): cambia el estado logístico de despacho de una nota/guía.
/// Estados válidos: PREPARANDO, EN_TRANSITO, ENTREGADO, DEVUELTO, PARCIAL.
/// Sella automáticamente la fecha de salida (al pasar a EN_TRANSITO) y la de
/// entrega (al pasar a ENTREGADO). No toca stock ni estado comercial/tributario.
#[tauri::command]
pub fn guia_cambiar_despacho(
    db: State<Database>,
    guia_id: i64,
    nuevo_estado: String,
    observacion: Option<String>,
) -> Result<(), String> {
    let estado = nuevo_estado.trim().to_uppercase();
    let validos = ["PREPARANDO", "EN_TRANSITO", "ENTREGADO", "DEVUELTO", "PARCIAL"];
    if !validos.contains(&estado.as_str()) {
        return Err(format!("Estado de despacho inválido: {}", estado));
    }
    let conn = db.conn.lock().map_err(|e| e.to_string())?;

    // Gating: el despacho logístico es inventario avanzado → requiere multi_almacen.
    {
        let getc = |k: &str| -> String {
            conn.query_row("SELECT value FROM config WHERE key=?1", rusqlite::params![k], |r| r.get(0))
                .unwrap_or_default()
        };
        let demo = getc("demo_activo") == "1";
        let mods = getc("licencia_modulos");
        if !demo && !mods.contains("multi_almacen") {
            return Err("El control de despacho requiere el módulo de inventario avanzado (multi-almacén). Actívelo en su licencia.".to_string());
        }
    }

    // Verificar que es una guía/nota de entrega
    let existe: i64 = conn.query_row(
        "SELECT COUNT(*) FROM ventas WHERE id = ?1 AND tipo_estado = 'GUIA_REMISION'",
        rusqlite::params![guia_id], |r| r.get(0),
    ).map_err(|e| e.to_string())?;
    if existe == 0 {
        return Err("Nota de entrega no encontrada".to_string());
    }

    // Sellar fechas según el estado destino (solo si aún no están puestas)
    match estado.as_str() {
        "EN_TRANSITO" => {
            conn.execute(
                "UPDATE ventas SET despacho_estado = ?1,
                     despacho_fecha_salida = COALESCE(despacho_fecha_salida, datetime('now','localtime')),
                     despacho_observacion = COALESCE(?2, despacho_observacion)
                 WHERE id = ?3",
                rusqlite::params![estado, observacion, guia_id],
            ).map_err(|e| e.to_string())?;
        }
        "ENTREGADO" => {
            conn.execute(
                "UPDATE ventas SET despacho_estado = ?1,
                     despacho_fecha_entrega = COALESCE(despacho_fecha_entrega, datetime('now','localtime')),
                     despacho_observacion = COALESCE(?2, despacho_observacion)
                 WHERE id = ?3",
                rusqlite::params![estado, observacion, guia_id],
            ).map_err(|e| e.to_string())?;
        }
        _ => {
            conn.execute(
                "UPDATE ventas SET despacho_estado = ?1,
                     despacho_observacion = COALESCE(?2, despacho_observacion)
                 WHERE id = ?3",
                rusqlite::params![estado, observacion, guia_id],
            ).map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}

/// Lee los datos SRI guardados de una guía (para precargar el modal de emisión).
#[tauri::command]
pub fn guia_obtener_datos_sri(
    db: State<Database>,
    guia_id: i64,
) -> Result<std::collections::HashMap<String, String>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let mut m = std::collections::HashMap::new();
    conn.query_row(
        "SELECT COALESCE(guia_transportista,''), COALESCE(guia_ruc_transportista,''),
                COALESCE(guia_tipo_id_transportista,''), COALESCE(guia_dir_partida,''),
                COALESCE(guia_fecha_inicio_transporte,''), COALESCE(guia_fecha_fin_transporte,''),
                COALESCE(guia_motivo_traslado,''), COALESCE(guia_ruta,''),
                COALESCE(guia_cod_doc_sustento,''), COALESCE(guia_num_doc_sustento,''),
                COALESCE(guia_num_aut_sustento,''), COALESCE(guia_fecha_emision_sustento,''),
                COALESCE(guia_placa,''), COALESCE(guia_chofer,''), COALESCE(guia_direccion_destino,''),
                COALESCE(estado_sri,''), COALESCE(numero_factura,'')
         FROM ventas WHERE id = ?1 AND tipo_estado = 'GUIA_REMISION'",
        rusqlite::params![guia_id],
        |row| {
            m.insert("transportista".to_string(), row.get::<_, String>(0)?);
            m.insert("ruc_transportista".to_string(), row.get::<_, String>(1)?);
            m.insert("tipo_id_transportista".to_string(), row.get::<_, String>(2)?);
            m.insert("dir_partida".to_string(), row.get::<_, String>(3)?);
            m.insert("fecha_inicio_transporte".to_string(), row.get::<_, String>(4)?);
            m.insert("fecha_fin_transporte".to_string(), row.get::<_, String>(5)?);
            m.insert("motivo_traslado".to_string(), row.get::<_, String>(6)?);
            m.insert("ruta".to_string(), row.get::<_, String>(7)?);
            m.insert("cod_doc_sustento".to_string(), row.get::<_, String>(8)?);
            m.insert("num_doc_sustento".to_string(), row.get::<_, String>(9)?);
            m.insert("num_aut_sustento".to_string(), row.get::<_, String>(10)?);
            m.insert("fecha_emision_sustento".to_string(), row.get::<_, String>(11)?);
            m.insert("placa".to_string(), row.get::<_, String>(12)?);
            m.insert("chofer".to_string(), row.get::<_, String>(13)?);
            m.insert("direccion_destino".to_string(), row.get::<_, String>(14)?);
            m.insert("estado_sri".to_string(), row.get::<_, String>(15)?);
            m.insert("numero_sri".to_string(), row.get::<_, String>(16)?);
            Ok(())
        },
    ).map_err(|e| format!("Guía no encontrada: {}", e))?;
    Ok(m)
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

    // Leer venta (incluye estado comercial y tipo_estado para el manejo de stock
    // de notas de entrega / guías de remisión).
    let (estado_sri, _tipo_documento, anulada, numero, total, estado_comercial, tipo_estado): (String, String, i32, String, f64, String, Option<String>) = conn.query_row(
        "SELECT COALESCE(estado_sri, 'NO_APLICA'), tipo_documento, anulada, numero, total, COALESCE(estado,''), tipo_estado FROM ventas WHERE id = ?1",
        rusqlite::params![venta_id],
        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?, row.get(5)?, row.get(6)?))
    ).map_err(|_| "Venta no encontrada".to_string())?;

    // REFACTOR INVENTARIO: para una nota de entrega / guía de remisión, el stock
    // sólo se descontó si fue ENTREGADA (recibida) o COMPLETADA/FACTURADA
    // (facturada). Si está PENDIENTE (en tránsito, nunca recibida) o RECHAZADA,
    // NUNCA se movió inventario → al anular NO se debe devolver stock.
    let es_guia = tipo_estado.as_deref() == Some("GUIA_REMISION");
    let guia_movio_stock = matches!(estado_comercial.as_str(), "ENTREGADA" | "COMPLETADA" | "FACTURADA");
    let omitir_reintegro_stock = es_guia && !guia_movio_stock;

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
    let items: Vec<(i64, f64, f64, Option<i64>)> = if omitir_reintegro_stock {
        // Nota de entrega PENDIENTE/RECHAZADA: nunca descontó stock → no reintegrar.
        Vec::new()
    } else {
        stmt.query_map(rusqlite::params![venta_id], |row| {
            Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?))
        }).map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect()
    };
    drop(stmt);

    // v2.5.62: refactor del loop de reintegración para NO silenciar errores
    // críticos. Antes se usaba .ok() en todos los UPDATE, lo que ocultaba
    // fallos (ej. columna updated_at faltante en BDs viejas) → venta quedaba
    // anulada pero stock no se actualizaba. Ahora:
    //   - UPDATE de productos.stock_actual → fail-fast con map_err
    //   - INSERT a movimientos_inventario → fail-fast (auditoría obligatoria)
    //   - UPDATE de updated_at QUITADO (no es crítico y rompe en BDs viejas)
    //   - lote_caducidad y stock_establecimiento → seguir como best-effort
    //     porque son secundarios (multi-almacén/caducidad pueden estar off)
    for (prod_id, cant, factor, lote_id) in &items {
        let cant_base = cant * factor;
        let omite: bool = conn.query_row(
            "SELECT (COALESCE(es_servicio, 0) + COALESCE(no_controla_stock, 0)) > 0 FROM productos WHERE id = ?1",
            rusqlite::params![prod_id],
            |row| row.get::<_, i64>(0).map(|v| v > 0)
        ).unwrap_or(false);
        if omite { continue; }

        // Best-effort: reversar al lote (no crítico, módulo caducidad puede no estar)
        if let Some(lid) = lote_id {
            let _ = conn.execute(
                "UPDATE lotes_caducidad SET cantidad = cantidad + ?1 WHERE id = ?2",
                rusqlite::params![cant_base, lid],
            );
        }

        let stock_antes: f64 = conn.query_row(
            "SELECT stock_actual FROM productos WHERE id = ?1",
            rusqlite::params![prod_id], |row| row.get(0)
        ).unwrap_or(0.0);

        // CRÍTICO: UPDATE stock (sin updated_at para compatibilidad con BDs viejas)
        conn.execute(
            "UPDATE productos SET stock_actual = stock_actual + ?1 WHERE id = ?2",
            rusqlite::params![cant_base, prod_id]
        ).map_err(|e| format!(
            "Error reintegrando stock del producto {}: {}. La anulación NO se aplicó para evitar inconsistencia.",
            prod_id, e
        ))?;

        // Best-effort: stock por establecimiento (multi-almacén puede no estar)
        if let Some(eid) = est_id {
            let _ = conn.execute(
                "UPDATE stock_establecimiento SET stock_actual = stock_actual + ?1
                 WHERE producto_id = ?2 AND establecimiento_id = ?3",
                rusqlite::params![cant_base, prod_id, eid]
            );
        }

        // CRÍTICO: movimiento auditable (sin esto no podemos auto-reparar en el futuro)
        conn.execute(
            "INSERT INTO movimientos_inventario (producto_id, tipo, cantidad, stock_anterior, stock_nuevo, motivo, usuario, referencia_id, establecimiento_id)
             VALUES (?1, 'ANULACION_VENTA', ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            rusqlite::params![prod_id, cant_base, stock_antes, stock_antes + cant_base,
                format!("Anulacion venta {} - {}", numero, motivo.trim()), usuario_nombre, venta_id, est_id]
        ).map_err(|e| format!(
            "Error registrando movimiento de auditoría para producto {}: {}. La anulación NO se aplicó.",
            prod_id, e
        ))?;
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

    // v2.5.63: CRÍTICO — UPDATE de caja ya NO usa .ok() (silenciaba errores).
    // Mismo bug que el de stock: si este UPDATE fallaba (ej. por trigger o
    // tipo de columna), la venta quedaba marcada anulada pero la caja seguía
    // contando el dinero. Ahora si falla, devolvemos error real al user.
    //
    // Solo descontamos la caja ABIERTA. Si la caja en que se hizo la venta
    // ya fue cerrada, ese cierre NO se altera (caso ya cerrado/cuadrado).
    conn.execute(
        "UPDATE caja SET monto_ventas = monto_ventas - ?1, monto_esperado = monto_esperado - ?2
         WHERE estado = 'ABIERTA'",
        rusqlite::params![total, efectivo_a_restar]
    ).map_err(|e| format!(
        "Error actualizando caja al anular: {}. La anulación NO se aplicó. \
         Stock ya fue revertido si llegó hasta aquí — revisa logs.",
        e
    ))?;

    Ok(())
}

// ─── v2.5.61: Reparación de anulaciones que dejaron stock inconsistente ─────
//
// Algunos clientes reportaron que después de anular una venta, el stock no
// volvía al producto. El motivo más común: los `.ok()` en `anular_venta`
// silenciaban un UPDATE fallido (ej. la columna `updated_at` no existía en
// instalaciones muy viejas, o un trigger DB fallaba).
//
// Estos comandos permiten al admin diagnosticar y reparar después del hecho.

#[derive(serde::Serialize)]
pub struct DiagnosticoAnulacion {
    pub venta_numero: String,
    pub anulada: bool,
    pub fecha_anulacion: Option<String>,
    pub items: Vec<DiagnosticoItemAnulacion>,
    pub todo_correcto: bool,
}

#[derive(serde::Serialize)]
pub struct DiagnosticoItemAnulacion {
    pub producto_id: i64,
    pub producto_nombre: String,
    pub cantidad_vendida: f64,
    pub factor_unidad: f64,
    pub cantidad_base: f64,
    pub stock_actual_ahora: f64,
    pub es_servicio_o_no_controla: bool,
    pub tiene_movimiento_anulacion: bool,
    pub necesita_reparacion: bool,
}

/// Diagnóstico: revisa una venta anulada y reporta qué items quedaron sin
/// reintegrar al stock. Es read-only, no modifica nada.
#[tauri::command]
pub fn verificar_anulacion(
    db: State<Database>,
    venta_id: i64,
) -> Result<DiagnosticoAnulacion, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;

    let (numero, anulada, _estado): (String, i32, String) = conn.query_row(
        "SELECT numero, anulada, COALESCE(estado, '') FROM ventas WHERE id = ?1",
        rusqlite::params![venta_id],
        |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
    ).map_err(|_| "Venta no encontrada".to_string())?;

    if anulada == 0 {
        return Err("Esta venta NO está anulada. Solo se puede verificar anulaciones.".into());
    }

    // Items de la venta
    let mut stmt = conn.prepare(
        "SELECT vd.producto_id, COALESCE(p.nombre, '?'),
                vd.cantidad, COALESCE(vd.factor_unidad, 1) as factor,
                p.stock_actual,
                (COALESCE(p.es_servicio, 0) + COALESCE(p.no_controla_stock, 0)) > 0 as es_servicio
         FROM venta_detalles vd
         LEFT JOIN productos p ON vd.producto_id = p.id
         WHERE vd.venta_id = ?1 AND vd.producto_id IS NOT NULL"
    ).map_err(|e| e.to_string())?;
    let items_raw: Vec<(i64, String, f64, f64, f64, bool)> = stmt.query_map(
        rusqlite::params![venta_id],
        |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?, r.get(5)?))
    ).map_err(|e| e.to_string())?
    .filter_map(Result::ok)
    .collect();
    drop(stmt);

    let mut items: Vec<DiagnosticoItemAnulacion> = Vec::new();
    let mut todo_correcto = true;

    for (pid, nombre, cant, factor, stock_ahora, es_serv) in items_raw {
        let cant_base = cant * factor;
        // ¿Existe movimiento ANULACION_VENTA para este producto y esta venta?
        let tiene_mov: bool = conn.query_row(
            "SELECT COUNT(*) > 0 FROM movimientos_inventario
             WHERE referencia_id = ?1 AND producto_id = ?2 AND tipo = 'ANULACION_VENTA'",
            rusqlite::params![venta_id, pid],
            |r| r.get(0),
        ).unwrap_or(false);

        // Necesita reparación si: NO es servicio Y NO tiene movimiento de anulación
        let necesita = !es_serv && !tiene_mov;
        if necesita { todo_correcto = false; }

        items.push(DiagnosticoItemAnulacion {
            producto_id: pid,
            producto_nombre: nombre,
            cantidad_vendida: cant,
            factor_unidad: factor,
            cantidad_base: cant_base,
            stock_actual_ahora: stock_ahora,
            es_servicio_o_no_controla: es_serv,
            tiene_movimiento_anulacion: tiene_mov,
            necesita_reparacion: necesita,
        });
    }

    Ok(DiagnosticoAnulacion {
        venta_numero: numero,
        anulada: true,
        fecha_anulacion: None,
        items,
        todo_correcto,
    })
}

#[derive(serde::Serialize)]
pub struct ReparacionAnulacion {
    pub venta_numero: String,
    pub items_reparados: Vec<ItemReparado>,
    pub items_ya_correctos: usize,
    pub items_omitidos_servicio: usize,
}

#[derive(serde::Serialize)]
pub struct ItemReparado {
    pub producto_id: i64,
    pub producto_nombre: String,
    pub cantidad_sumada: f64,
    pub stock_antes: f64,
    pub stock_despues: f64,
}

/// Repara una anulación que NO revirtió correctamente el stock.
/// Solo aplica a items que NO tienen movimiento `ANULACION_VENTA` registrado.
/// Crea el movimiento ahora con motivo "REPARACION".
#[tauri::command]
pub fn reparar_anulacion_venta(
    db: State<Database>,
    sesion: State<SesionState>,
    venta_id: i64,
) -> Result<ReparacionAnulacion, String> {
    let sesion_guard = sesion.sesion.lock().map_err(|e| e.to_string())?;
    let sesion_actual = sesion_guard.as_ref().ok_or("Debe iniciar sesion".to_string())?;
    let usuario_rol = sesion_actual.rol.clone();
    let usuario_nombre = sesion_actual.nombre.clone();
    drop(sesion_guard);

    if usuario_rol != "ADMIN" {
        return Err("Solo el administrador puede reparar anulaciones".into());
    }

    let conn = db.conn.lock().map_err(|e| e.to_string())?;

    let (numero, anulada): (String, i32) = conn.query_row(
        "SELECT numero, anulada FROM ventas WHERE id = ?1",
        rusqlite::params![venta_id],
        |r| Ok((r.get(0)?, r.get(1)?)),
    ).map_err(|_| "Venta no encontrada".to_string())?;

    if anulada == 0 {
        return Err("La venta NO está anulada. No hay nada que reparar.".into());
    }

    // Cargar items
    let mut stmt = conn.prepare(
        "SELECT vd.producto_id, COALESCE(p.nombre, '?'),
                vd.cantidad, COALESCE(vd.factor_unidad, 1) as factor,
                vd.lote_id,
                (COALESCE(p.es_servicio, 0) + COALESCE(p.no_controla_stock, 0)) > 0 as es_servicio
         FROM venta_detalles vd
         LEFT JOIN productos p ON vd.producto_id = p.id
         WHERE vd.venta_id = ?1 AND vd.producto_id IS NOT NULL"
    ).map_err(|e| e.to_string())?;
    let items_raw: Vec<(i64, String, f64, f64, Option<i64>, bool)> = stmt.query_map(
        rusqlite::params![venta_id],
        |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?, r.get(5)?))
    ).map_err(|e| e.to_string())?
    .filter_map(Result::ok)
    .collect();
    drop(stmt);

    let mut reparados: Vec<ItemReparado> = Vec::new();
    let mut ya_correctos = 0usize;
    let mut omitidos = 0usize;

    for (pid, nombre, cant, factor, lote_id, es_serv) in items_raw {
        if es_serv {
            omitidos += 1;
            continue;
        }

        let tiene_mov: bool = conn.query_row(
            "SELECT COUNT(*) > 0 FROM movimientos_inventario
             WHERE referencia_id = ?1 AND producto_id = ?2 AND tipo = 'ANULACION_VENTA'",
            rusqlite::params![venta_id, pid],
            |r| r.get(0),
        ).unwrap_or(false);

        if tiene_mov {
            ya_correctos += 1;
            continue;
        }

        let cant_base = cant * factor;
        let stock_antes: f64 = conn.query_row(
            "SELECT stock_actual FROM productos WHERE id = ?1",
            rusqlite::params![pid], |r| r.get(0),
        ).unwrap_or(0.0);

        conn.execute(
            "UPDATE productos SET stock_actual = stock_actual + ?1 WHERE id = ?2",
            rusqlite::params![cant_base, pid],
        ).map_err(|e| format!("Error reintegrando stock producto {}: {}", pid, e))?;

        // Reversar lote si aplica
        if let Some(lid) = lote_id {
            let _ = conn.execute(
                "UPDATE lotes_caducidad SET cantidad = cantidad + ?1 WHERE id = ?2",
                rusqlite::params![cant_base, lid],
            );
        }

        // Registrar movimiento de reparación (importante: tipo ANULACION_VENTA
        // para que un re-verificar marque ya_correcto). Motivo distintivo.
        let stock_despues = stock_antes + cant_base;
        let _ = conn.execute(
            "INSERT INTO movimientos_inventario
                (producto_id, tipo, cantidad, stock_anterior, stock_nuevo, motivo, usuario, referencia_id)
             VALUES (?1, 'ANULACION_VENTA', ?2, ?3, ?4, ?5, ?6, ?7)",
            rusqlite::params![
                pid, cant_base, stock_antes, stock_despues,
                format!("REPARACION manual anulacion {}", numero),
                usuario_nombre, venta_id
            ],
        );

        reparados.push(ItemReparado {
            producto_id: pid,
            producto_nombre: nombre,
            cantidad_sumada: cant_base,
            stock_antes,
            stock_despues,
        });
    }

    Ok(ReparacionAnulacion {
        venta_numero: numero,
        items_reparados: reparados,
        items_ya_correctos: ya_correctos,
        items_omitidos_servicio: omitidos,
    })
}

// ─── Detalle completo de NC (v2.3.62) ────────────────────────────────────
// Para que el modal de detalle pueda mostrar todo: header + items + venta original
// + desglose de reembolso + retiro_caja vinculado.

#[tauri::command]
pub fn obtener_nota_credito(
    db: State<Database>,
    nc_id: i64,
) -> Result<serde_json::Value, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;

    // 1. Header de la NC con TODOS los campos nuevos
    let header: serde_json::Value = conn
        .query_row(
            "SELECT nc.id, nc.numero, nc.venta_id, nc.cliente_id, nc.motivo, nc.fecha,
                    nc.subtotal_sin_iva, nc.subtotal_con_iva, nc.iva, nc.total,
                    nc.estado_sri, nc.autorizacion_sri, nc.clave_acceso, nc.numero_factura_nc,
                    nc.usuario, nc.establecimiento, nc.punto_emision,
                    COALESCE(nc.tipo_devolucion, 'TOTAL'),
                    COALESCE(nc.monto_efectivo_devuelto, 0),
                    COALESCE(nc.monto_transfer_devuelto, 0),
                    COALESCE(nc.monto_credito_devuelto, 0),
                    COALESCE(nc.metodo_reembolso, 'EFECTIVO'),
                    nc.retiro_caja_id,
                    COALESCE(cl.nombre, 'CONSUMIDOR FINAL') as cliente_nombre,
                    COALESCE(cl.identificacion, '') as cliente_identificacion,
                    COALESCE(v.numero_factura, v.numero) as venta_numero,
                    COALESCE(v.forma_pago, 'EFECTIVO') as venta_forma_pago,
                    v.fecha as venta_fecha,
                    v.total as venta_total,
                    v.tipo_documento as venta_tipo
             FROM notas_credito nc
             LEFT JOIN clientes cl ON nc.cliente_id = cl.id
             LEFT JOIN ventas v ON nc.venta_id = v.id
             WHERE nc.id = ?1",
            rusqlite::params![nc_id],
            |row| {
                Ok(serde_json::json!({
                    "id": row.get::<_, i64>(0)?,
                    "numero": row.get::<_, String>(1)?,
                    "venta_id": row.get::<_, i64>(2)?,
                    "cliente_id": row.get::<_, Option<i64>>(3)?,
                    "motivo": row.get::<_, String>(4)?,
                    "fecha": row.get::<_, String>(5)?,
                    "subtotal_sin_iva": row.get::<_, f64>(6)?,
                    "subtotal_con_iva": row.get::<_, f64>(7)?,
                    "iva": row.get::<_, f64>(8)?,
                    "total": row.get::<_, f64>(9)?,
                    "estado_sri": row.get::<_, String>(10).unwrap_or_else(|_| "NO_APLICA".to_string()),
                    "autorizacion_sri": row.get::<_, Option<String>>(11)?,
                    "clave_acceso": row.get::<_, Option<String>>(12)?,
                    "numero_factura_nc": row.get::<_, Option<String>>(13)?,
                    "usuario": row.get::<_, Option<String>>(14)?,
                    "establecimiento": row.get::<_, Option<String>>(15)?,
                    "punto_emision": row.get::<_, Option<String>>(16)?,
                    "tipo_devolucion": row.get::<_, String>(17)?,
                    "monto_efectivo_devuelto": row.get::<_, f64>(18)?,
                    "monto_transfer_devuelto": row.get::<_, f64>(19)?,
                    "monto_credito_devuelto": row.get::<_, f64>(20)?,
                    "metodo_reembolso": row.get::<_, String>(21)?,
                    "retiro_caja_id": row.get::<_, Option<i64>>(22)?,
                    "cliente_nombre": row.get::<_, String>(23)?,
                    "cliente_identificacion": row.get::<_, String>(24)?,
                    "venta_numero": row.get::<_, String>(25)?,
                    "venta_forma_pago": row.get::<_, String>(26)?,
                    "venta_fecha": row.get::<_, Option<String>>(27)?,
                    "venta_total": row.get::<_, Option<f64>>(28)?,
                    "venta_tipo": row.get::<_, Option<String>>(29)?,
                }))
            },
        )
        .map_err(|_| "Nota de crédito no encontrada".to_string())?;

    // 2. Items con nombre de producto
    let mut stmt = conn
        .prepare(
            "SELECT d.id, d.producto_id, p.nombre, d.cantidad, d.precio_unitario,
                    d.descuento, d.iva_porcentaje, d.subtotal
             FROM nota_credito_detalles d
             JOIN productos p ON d.producto_id = p.id
             WHERE d.nota_credito_id = ?1
             ORDER BY d.id",
        )
        .map_err(|e| e.to_string())?;
    let items: Vec<serde_json::Value> = stmt
        .query_map(rusqlite::params![nc_id], |row| {
            Ok(serde_json::json!({
                "id": row.get::<_, i64>(0)?,
                "producto_id": row.get::<_, i64>(1)?,
                "nombre_producto": row.get::<_, String>(2)?,
                "cantidad": row.get::<_, f64>(3)?,
                "precio_unitario": row.get::<_, f64>(4)?,
                "descuento": row.get::<_, f64>(5)?,
                "iva_porcentaje": row.get::<_, f64>(6)?,
                "subtotal": row.get::<_, f64>(7)?,
            }))
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    Ok(serde_json::json!({
        "header": header,
        "items": items,
    }))
}

