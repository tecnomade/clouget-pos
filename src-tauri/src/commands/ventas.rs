use crate::db::{Database, SesionState};
use crate::models::{NuevaVenta, NuevaNotaCredito, NotaCreditoInfo, Venta, VentaCompleta, VentaDetalle};
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

    // Obtener secuencial interno (todas las ventas usan secuencial_nota_venta)
    let secuencial: i64 = conn
        .query_row(
            "SELECT CAST(value AS INTEGER) FROM config WHERE key = 'secuencial_nota_venta'",
            [],
            |row| row.get(0),
        )
        .map_err(|e| e.to_string())?;

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
         tipo_documento, estado_sri, observacion, usuario, usuario_id)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16)",
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
        ],
    )
    .map_err(|e| e.to_string())?;

    let venta_id = conn.last_insert_rowid();

    // Insertar detalles y actualizar stock
    let mut detalles_guardados = Vec::new();
    for item in &venta.items {
        let subtotal = item.cantidad * item.precio_unitario - item.descuento;

        conn.execute(
            "INSERT INTO venta_detalles (venta_id, producto_id, cantidad, precio_unitario,
             descuento, iva_porcentaje, subtotal)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            rusqlite::params![
                venta_id,
                item.producto_id,
                item.cantidad,
                item.precio_unitario,
                item.descuento,
                item.iva_porcentaje,
                subtotal,
            ],
        )
        .map_err(|e| e.to_string())?;

        // Descontar stock (si no es servicio)
        conn.execute(
            "UPDATE productos SET stock_actual = stock_actual - ?1,
             updated_at = datetime('now','localtime')
             WHERE id = ?2 AND es_servicio = 0",
            rusqlite::params![item.cantidad, item.producto_id],
        )
        .map_err(|e| e.to_string())?;

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
        });
    }

    // Actualizar secuencial interno
    conn.execute(
        "UPDATE config SET value = CAST(?1 AS TEXT) WHERE key = 'secuencial_nota_venta'",
        rusqlite::params![secuencial + 1],
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
            "SELECT id, numero, cliente_id, fecha, subtotal_sin_iva, subtotal_con_iva,
             descuento, iva, total, forma_pago, monto_recibido, cambio, estado,
             tipo_documento, estado_sri, autorizacion_sri, clave_acceso, observacion,
             numero_factura
             FROM ventas
             WHERE date(fecha) = date(?1) AND anulada = 0
             ORDER BY fecha DESC",
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
            "SELECT id, numero, cliente_id, fecha, subtotal_sin_iva, subtotal_con_iva,
             descuento, iva, total, forma_pago, monto_recibido, cambio, estado,
             tipo_documento, estado_sri, autorizacion_sri, clave_acceso, observacion,
             numero_factura
             FROM ventas WHERE id = ?1",
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
                })
            },
        )
        .map_err(|e| e.to_string())?;

    let mut stmt = conn
        .prepare(
            "SELECT d.id, d.venta_id, d.producto_id, p.nombre, d.cantidad,
             d.precio_unitario, d.descuento, d.iva_porcentaje, d.subtotal
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
            "SELECT id, numero, cliente_id, fecha, subtotal_sin_iva, subtotal_con_iva,
             descuento, iva, total, forma_pago, monto_recibido, cambio, estado,
             tipo_documento, estado_sri, autorizacion_sri, clave_acceso, observacion,
             numero_factura
             FROM ventas
             WHERE fecha >= ?1 AND anulada = 0
             ORDER BY fecha DESC",
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
            "SELECT COALESCE(SUM(vd.subtotal - (p.precio_costo * vd.cantidad)), 0)
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
    drop(sesion_guard);

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

    // Generar número secuencial
    let secuencial: i64 = conn
        .query_row(
            "SELECT CAST(value AS INTEGER) FROM config WHERE key = 'secuencial_nota_credito'",
            [],
            |row| row.get(0),
        )
        .unwrap_or(1);

    let establecimiento: String = conn
        .query_row("SELECT value FROM config WHERE key = 'establecimiento'", [], |row| row.get(0))
        .unwrap_or_else(|_| "001".to_string());

    let punto_emision: String = conn
        .query_row("SELECT value FROM config WHERE key = 'punto_emision'", [], |row| row.get(0))
        .unwrap_or_else(|_| "001".to_string());

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
         subtotal_sin_iva, subtotal_con_iva, iva, total, usuario, usuario_id)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
        rusqlite::params![
            numero, nota.venta_id, cliente_id, nota.motivo.trim(),
            subtotal_sin_iva, subtotal_con_iva, iva_total, total,
            usuario_nombre, usuario_id,
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
    }

    // Incrementar secuencial
    conn.execute(
        "UPDATE config SET value = CAST(?1 AS TEXT) WHERE key = 'secuencial_nota_credito'",
        rusqlite::params![secuencial + 1],
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
