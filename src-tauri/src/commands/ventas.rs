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
        return Err("Debe abrir la caja antes de realizar ventas".to_string());
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

    // Calcular totales
    let mut subtotal_sin_iva = 0.0_f64;
    let mut subtotal_con_iva = 0.0_f64;
    let mut iva_total = 0.0_f64;

    for item in &venta.items {
        let subtotal_item = item.cantidad * item.precio_unitario - item.descuento;
        if item.iva_porcentaje > 0.0 {
            subtotal_con_iva += subtotal_item;
            iva_total += subtotal_item * (item.iva_porcentaje / 100.0);
        } else {
            subtotal_sin_iva += subtotal_item;
        }
    }

    let total = subtotal_sin_iva + subtotal_con_iva + iva_total - venta.descuento;
    let cambio = if venta.monto_recibido > total {
        venta.monto_recibido - total
    } else {
        0.0
    };

    // Determinar estado_sri segun tipo de documento
    let estado_sri = match venta.tipo_documento.as_str() {
        "FACTURA" => "PENDIENTE",
        _ => "NO_APLICA",
    };

    // Insertar cabecera de venta
    conn.execute(
        "INSERT INTO ventas (numero, cliente_id, subtotal_sin_iva, subtotal_con_iva,
         descuento, iva, total, forma_pago, monto_recibido, cambio, estado,
         tipo_documento, estado_sri, observacion, usuario, usuario_id, establecimiento, punto_emision,
         banco_id, referencia_pago, comprobante_imagen)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21)",
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
        ],
    )
    .map_err(|e| e.to_string())?;

    let venta_id = conn.last_insert_rowid();

    // Insertar detalles y actualizar stock
    let mut detalles_guardados = Vec::new();
    for item in &venta.items {
        let subtotal = item.cantidad * item.precio_unitario - item.descuento;

        // Obtener precio_costo del producto para snapshot en venta_detalles
        let precio_costo_prod: f64 = conn
            .query_row(
                "SELECT precio_costo FROM productos WHERE id = ?1",
                rusqlite::params![item.producto_id],
                |row| row.get(0),
            )
            .unwrap_or(0.0);

        conn.execute(
            "INSERT INTO venta_detalles (venta_id, producto_id, cantidad, precio_unitario,
             descuento, iva_porcentaje, subtotal, info_adicional, precio_costo)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
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
            ],
        )
        .map_err(|e| e.to_string())?;

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
        if !omite_stock {
            let _ = conn.execute(
                "INSERT INTO movimientos_inventario (producto_id, tipo, cantidad, stock_anterior, stock_nuevo, costo_unitario, referencia_id, usuario, establecimiento_id)
                 VALUES (?1, 'VENTA', ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                rusqlite::params![
                    item.producto_id,
                    -(item.cantidad),
                    stock_antes,
                    stock_antes - item.cantidad,
                    item.precio_unitario,
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

        detalles_guardados.push(VentaDetalle {
            id: Some(conn.last_insert_rowid()),
            venta_id: Some(venta_id),
            producto_id: item.producto_id,
            nombre_producto: Some(nombre_prod),
            cantidad: item.cantidad,
            precio_unitario: item.precio_unitario,
            descuento: item.descuento,
            iva_porcentaje: item.iva_porcentaje,
            subtotal,
            info_adicional: item.info_adicional.clone(),
        });
    }

    // Actualizar secuencial interno en tabla secuenciales
    conn.execute(
        "UPDATE secuenciales SET secuencial = secuencial + 1 WHERE establecimiento_codigo = ?1 AND punto_emision_codigo = ?2 AND tipo_documento = 'NOTA_VENTA'",
        rusqlite::params![terminal_est, terminal_pe],
    )
    .map_err(|e| e.to_string())?;

    // Actualizar monto de caja si hay una abierta
    conn.execute(
        "UPDATE caja SET monto_ventas = monto_ventas + ?1,
         monto_esperado = monto_inicial + monto_ventas + ?1
         WHERE estado = 'ABIERTA'",
        rusqlite::params![total],
    )
    .ok();

    // Si es fiado, crear cuenta por cobrar
    if venta.es_fiado {
        conn.execute(
            "INSERT INTO cuentas_por_cobrar (cliente_id, venta_id, monto_total, saldo, estado)
             VALUES (?1, ?2, ?3, ?3, 'PENDIENTE')",
            rusqlite::params![venta.cliente_id.unwrap_or(1), venta_id, total],
        )
        .map_err(|e| e.to_string())?;
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
            tipo_estado: None,
            guia_placa: None, guia_chofer: None, guia_direccion_destino: None,
        },
        detalles: detalles_guardados,
        cliente_nombre,
    })
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
             COALESCE(v.tipo_estado, 'COMPLETADA') as tipo_estado
             FROM ventas v
             LEFT JOIN cuentas_banco cb ON v.banco_id = cb.id
             WHERE date(v.fecha) = date(?1) AND v.anulada = 0
             ORDER BY v.fecha DESC",
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
                tipo_estado: row.get(24).ok(),
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
             v.banco_id, v.referencia_pago, cb.nombre as banco_nombre
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
                    tipo_estado: None,
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

    // Buscar la caja abierta de este usuario (o la más reciente cerrada hoy)
    let fecha_apertura: String = conn
        .query_row(
            "SELECT fecha_apertura FROM caja
             WHERE usuario_id = ?1 AND (estado = 'ABIERTA' OR date(fecha_apertura) = date('now','localtime'))
             ORDER BY id DESC LIMIT 1",
            rusqlite::params![usuario_id],
            |row| row.get(0),
        )
        .map_err(|_| "No se encontró una sesión de caja para este usuario".to_string())?;

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
                tipo_estado: row.get(24).ok(),
                guia_placa: None, guia_chofer: None, guia_direccion_destino: None,
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
        .map_err(|_| "No se encontró una sesión de caja para este usuario".to_string())?;

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
        .map_err(|_| "No se encontró una sesión de caja para este usuario".to_string())?;

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

    // Validar que la venta existe y es NOTA_VENTA (no FACTURA)
    let (venta_numero, cliente_id, tipo_doc): (String, i64, String) = conn
        .query_row(
            "SELECT numero, COALESCE(cliente_id, 1), tipo_documento FROM ventas WHERE id = ?1 AND anulada = 0",
            rusqlite::params![venta_id],
            |row| Ok((row.get(0)?, row.get::<_, Option<i64>>(1)?.unwrap_or(1), row.get(2)?)),
        )
        .map_err(|_| "Venta no encontrada o anulada".to_string())?;

    if tipo_doc == "FACTURA" {
        return Err("Para facturas electrónicas use la opción de Nota de Crédito SRI".to_string());
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

    // Insertar detalles y devolver stock
    for item in &items {
        let producto_id = item.get("producto_id").and_then(|v| v.as_i64()).unwrap_or(0);
        let cantidad = item.get("cantidad").and_then(|v| v.as_f64()).unwrap_or(0.0);
        let precio_unitario = item.get("precio_unitario").and_then(|v| v.as_f64()).unwrap_or(0.0);
        let descuento = item.get("descuento").and_then(|v| v.as_f64()).unwrap_or(0.0);
        let iva_porcentaje = item.get("iva_porcentaje").and_then(|v| v.as_f64()).unwrap_or(0.0);
        let subtotal = cantidad * precio_unitario - descuento;

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
    }

    // Incrementar secuencial
    conn.execute(
        "UPDATE secuenciales SET secuencial = secuencial + 1 WHERE establecimiento_codigo = ?1 AND punto_emision_codigo = ?2 AND tipo_documento = 'NOTA_CREDITO'",
        rusqlite::params![establecimiento, punto_emision],
    )
    .map_err(|e| e.to_string())?;

    Ok(serde_json::json!({
        "id": nc_id,
        "numero": numero,
        "venta_id": venta_id,
        "venta_numero": venta_numero,
        "motivo": motivo.trim(),
        "total": total,
        "estado_sri": "NO_APLICA",
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
            tipo_estado: Some(tipo_estado.to_string()),
            guia_placa: None, guia_chofer: None, guia_direccion_destino: None,
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
        if !omite_stock {
            let _ = conn.execute(
                "INSERT INTO movimientos_inventario (producto_id, tipo, cantidad, stock_anterior, stock_nuevo, costo_unitario, referencia_id, usuario, establecimiento_id)
                 VALUES (?1, 'GUIA_REMISION', ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                rusqlite::params![
                    item.producto_id,
                    -(item.cantidad),
                    stock_antes,
                    stock_antes - item.cantidad,
                    item.precio_unitario,
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
            tipo_estado: Some("GUIA_REMISION".to_string()),
            guia_placa: venta.guia_placa, guia_chofer: venta.guia_chofer,
            guia_direccion_destino: venta.guia_direccion_destino,
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
         v.banco_id, v.referencia_pago, cb.nombre as banco_nombre, v.tipo_estado
         FROM ventas v
         LEFT JOIN cuentas_banco cb ON v.banco_id = cb.id
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
            tipo_estado: row.get(24).ok(),
            guia_placa: None, guia_chofer: None, guia_direccion_destino: None,
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

    // Verificar que la guía existe y está pendiente
    let (guia_numero, total, cliente_id_val): (String, f64, i64) = conn.query_row(
        "SELECT numero, total, cliente_id FROM ventas WHERE id = ?1 AND tipo_estado = 'GUIA_REMISION' AND estado = 'PENDIENTE'",
        rusqlite::params![guia_id],
        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
    ).map_err(|_| "Guía de remisión no encontrada o ya fue convertida".to_string())?;

    // Leer establecimiento y punto de emisión del terminal
    let terminal_est: String = conn
        .query_row("SELECT value FROM config WHERE key = 'terminal_establecimiento'", [], |row| row.get(0))
        .unwrap_or_else(|_| "001".to_string());
    let terminal_pe: String = conn
        .query_row("SELECT value FROM config WHERE key = 'terminal_punto_emision'", [], |row| row.get(0))
        .unwrap_or_else(|_| "001".to_string());

    // Generar nuevo secuencial NV
    conn.execute(
        "INSERT OR IGNORE INTO secuenciales (establecimiento_codigo, punto_emision_codigo, tipo_documento, secuencial) VALUES (?1, ?2, 'NOTA_VENTA', 1)",
        rusqlite::params![terminal_est, terminal_pe],
    ).map_err(|e| e.to_string())?;

    let secuencial: i64 = conn.query_row(
        "SELECT secuencial FROM secuenciales WHERE establecimiento_codigo = ?1 AND punto_emision_codigo = ?2 AND tipo_documento = 'NOTA_VENTA'",
        rusqlite::params![terminal_est, terminal_pe],
        |row| row.get(0),
    ).map_err(|e| e.to_string())?;

    let nuevo_numero = format!("NV-{:09}", secuencial);
    let cambio = if monto_recibido > total { monto_recibido - total } else { 0.0 };

    // Actualizar la guía para convertirla en venta completada
    let nueva_observacion = format!("Origen: {}", guia_numero);
    conn.execute(
        "UPDATE ventas SET
            tipo_estado = 'COMPLETADA',
            estado = 'COMPLETADA',
            numero = ?1,
            forma_pago = ?2,
            monto_recibido = ?3,
            cambio = ?4,
            banco_id = ?5,
            referencia_pago = ?6,
            observacion = CASE WHEN observacion IS NOT NULL AND observacion != '' THEN observacion || ' | ' || ?7 ELSE ?7 END,
            fecha = datetime('now','localtime'),
            usuario = ?8,
            usuario_id = ?9,
            guia_origen_id = ?10
         WHERE id = ?11",
        rusqlite::params![
            nuevo_numero, forma_pago, monto_recibido, cambio,
            banco_id, referencia_pago, nueva_observacion,
            usuario_nombre, usuario_id, guia_id, guia_id,
        ],
    ).map_err(|e| e.to_string())?;

    // Incrementar secuencial NV
    conn.execute(
        "UPDATE secuenciales SET secuencial = secuencial + 1 WHERE establecimiento_codigo = ?1 AND punto_emision_codigo = ?2 AND tipo_documento = 'NOTA_VENTA'",
        rusqlite::params![terminal_est, terminal_pe],
    ).map_err(|e| e.to_string())?;

    // Actualizar monto de caja
    conn.execute(
        "UPDATE caja SET monto_ventas = monto_ventas + ?1,
         monto_esperado = monto_inicial + monto_ventas + ?1
         WHERE estado = 'ABIERTA'",
        rusqlite::params![total],
    ).ok();

    // Si es fiado, crear cuenta por cobrar
    if es_fiado.unwrap_or(false) {
        conn.execute(
            "INSERT INTO cuentas_por_cobrar (cliente_id, venta_id, monto_total, saldo, estado)
             VALUES (?1, ?2, ?3, ?3, 'PENDIENTE')",
            rusqlite::params![cliente_id_val, guia_id, total],
        ).map_err(|e| e.to_string())?;
    }

    // Obtener la venta actualizada
    let venta_result = conn.query_row(
        "SELECT v.id, v.numero, v.cliente_id, v.fecha, v.subtotal_sin_iva, v.subtotal_con_iva,
         v.descuento, v.iva, v.total, v.forma_pago, v.monto_recibido, v.cambio, v.estado,
         v.tipo_documento, v.estado_sri, v.autorizacion_sri, v.clave_acceso, v.observacion,
         v.numero_factura, v.establecimiento, v.punto_emision,
         v.banco_id, v.referencia_pago, cb.nombre as banco_nombre, v.tipo_estado
         FROM ventas v
         LEFT JOIN cuentas_banco cb ON v.banco_id = cb.id
         WHERE v.id = ?1",
        rusqlite::params![guia_id],
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
                tipo_estado: row.get(24).ok(),
                guia_placa: None, guia_chofer: None, guia_direccion_destino: None,
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

    let detalles = stmt.query_map(rusqlite::params![guia_id], |row| {
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
