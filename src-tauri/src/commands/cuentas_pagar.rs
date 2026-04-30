use crate::db::Database;
use crate::models::{CuentaPorPagar, PagoProveedor, ResumenAcreedor};
use tauri::State;

#[tauri::command]
pub fn alertas_pagos_vencidos(
    db: State<Database>,
) -> Result<Vec<serde_json::Value>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let hoy = chrono::Local::now().format("%Y-%m-%d").to_string();
    let mut stmt = conn
        .prepare(
            "SELECT cp.id, c.numero, p.nombre, cp.monto_total, cp.monto_pagado,
                    cp.saldo, cp.fecha_vencimiento,
                    julianday(?1) - julianday(cp.fecha_vencimiento) as dias_vencido
             FROM cuentas_por_pagar cp
             JOIN proveedores p ON cp.proveedor_id = p.id
             LEFT JOIN compras c ON cp.compra_id = c.id
             WHERE cp.estado = 'PENDIENTE' AND cp.fecha_vencimiento < ?1
             ORDER BY cp.fecha_vencimiento ASC",
        )
        .map_err(|e| e.to_string())?;

    let resultado = stmt
        .query_map(rusqlite::params![hoy], |row| {
            Ok(serde_json::json!({
                "id": row.get::<_, i64>(0)?,
                "numero_compra": row.get::<_, Option<String>>(1)?,
                "proveedor_nombre": row.get::<_, String>(2)?,
                "total": row.get::<_, f64>(3)?,
                "pagado": row.get::<_, f64>(4)?,
                "saldo": row.get::<_, f64>(5)?,
                "fecha_vencimiento": row.get::<_, Option<String>>(6)?,
                "dias_vencido": row.get::<_, f64>(7)?,
            }))
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    Ok(resultado)
}

#[tauri::command]
pub fn resumen_acreedores(db: State<Database>) -> Result<Vec<ResumenAcreedor>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;

    let mut stmt = conn
        .prepare(
            "SELECT cp.proveedor_id, p.nombre, SUM(cp.saldo) as total_deuda, COUNT(*) as num_cuentas
             FROM cuentas_por_pagar cp
             JOIN proveedores p ON cp.proveedor_id = p.id
             WHERE cp.estado = 'PENDIENTE'
             GROUP BY cp.proveedor_id
             ORDER BY total_deuda DESC",
        )
        .map_err(|e| e.to_string())?;

    let resultado = stmt
        .query_map([], |row| {
            Ok(ResumenAcreedor {
                proveedor_id: row.get(0)?,
                proveedor_nombre: row.get(1)?,
                total_deuda: row.get(2)?,
                num_cuentas: row.get(3)?,
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    Ok(resultado)
}

#[tauri::command]
pub fn listar_cuentas_pagar(
    db: State<Database>,
    proveedor_id: Option<i64>,
) -> Result<Vec<CuentaPorPagar>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;

    let sql = if proveedor_id.is_some() {
        "SELECT cp.id, cp.proveedor_id, cp.compra_id, cp.monto_total, cp.monto_pagado,
                cp.saldo, cp.estado, cp.fecha_vencimiento, cp.observacion, cp.created_at,
                p.nombre, c.numero
         FROM cuentas_por_pagar cp
         JOIN proveedores p ON cp.proveedor_id = p.id
         LEFT JOIN compras c ON cp.compra_id = c.id
         WHERE cp.estado = 'PENDIENTE' AND cp.proveedor_id = ?1
         ORDER BY cp.created_at DESC"
    } else {
        "SELECT cp.id, cp.proveedor_id, cp.compra_id, cp.monto_total, cp.monto_pagado,
                cp.saldo, cp.estado, cp.fecha_vencimiento, cp.observacion, cp.created_at,
                p.nombre, c.numero
         FROM cuentas_por_pagar cp
         JOIN proveedores p ON cp.proveedor_id = p.id
         LEFT JOIN compras c ON cp.compra_id = c.id
         WHERE cp.estado = 'PENDIENTE'
         ORDER BY p.nombre, cp.created_at DESC"
    };

    let mut stmt = conn.prepare(sql).map_err(|e| e.to_string())?;

    let map_row = |row: &rusqlite::Row| -> rusqlite::Result<CuentaPorPagar> {
        Ok(CuentaPorPagar {
            id: Some(row.get(0)?),
            proveedor_id: row.get(1)?,
            compra_id: row.get(2)?,
            monto_total: row.get(3)?,
            monto_pagado: row.get(4)?,
            saldo: row.get(5)?,
            estado: row.get(6)?,
            fecha_vencimiento: row.get(7)?,
            observacion: row.get(8)?,
            created_at: row.get(9)?,
            proveedor_nombre: row.get(10)?,
            compra_numero: row.get(11)?,
        })
    };

    let resultado = if let Some(pid) = proveedor_id {
        stmt.query_map(rusqlite::params![pid], map_row)
            .map_err(|e| e.to_string())?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())?
    } else {
        stmt.query_map([], map_row)
            .map_err(|e| e.to_string())?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())?
    };

    Ok(resultado)
}

#[tauri::command]
pub fn registrar_pago_proveedor(
    db: State<Database>,
    cuenta_id: i64,
    monto: f64,
    forma_pago: String,
    numero_comprobante: Option<String>,
    observacion: Option<String>,
    banco_id: Option<i64>,
) -> Result<CuentaPorPagar, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;

    // Obtener saldo actual
    let saldo_actual: f64 = conn
        .query_row(
            "SELECT saldo FROM cuentas_por_pagar WHERE id = ?1 AND estado = 'PENDIENTE'",
            rusqlite::params![cuenta_id],
            |row| row.get(0),
        )
        .map_err(|_| "Cuenta no encontrada o ya pagada".to_string())?;

    if monto <= 0.0 {
        return Err("El monto debe ser mayor a 0".to_string());
    }

    if monto > saldo_actual + 0.01 {
        return Err(format!(
            "El monto (${:.2}) excede el saldo pendiente (${:.2})",
            monto, saldo_actual
        ));
    }

    // Insertar pago
    conn.execute(
        "INSERT INTO pagos_proveedor (cuenta_id, monto, forma_pago, numero_comprobante, observacion, banco_id)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        rusqlite::params![cuenta_id, monto, forma_pago, numero_comprobante, observacion, banco_id],
    )
    .map_err(|e| e.to_string())?;

    // Actualizar saldo de la cuenta
    let nuevo_saldo = (saldo_actual - monto).max(0.0);
    let nuevo_estado = if nuevo_saldo <= 0.01 {
        "PAGADA"
    } else {
        "PENDIENTE"
    };

    conn.execute(
        "UPDATE cuentas_por_pagar SET monto_pagado = monto_pagado + ?1, saldo = ?2, estado = ?3
         WHERE id = ?4",
        rusqlite::params![monto, nuevo_saldo, nuevo_estado, cuenta_id],
    )
    .map_err(|e| e.to_string())?;

    // Retornar cuenta actualizada
    conn.query_row(
        "SELECT cp.id, cp.proveedor_id, cp.compra_id, cp.monto_total, cp.monto_pagado,
                cp.saldo, cp.estado, cp.fecha_vencimiento, cp.observacion, cp.created_at,
                p.nombre, c.numero
         FROM cuentas_por_pagar cp
         JOIN proveedores p ON cp.proveedor_id = p.id
         LEFT JOIN compras c ON cp.compra_id = c.id
         WHERE cp.id = ?1",
        rusqlite::params![cuenta_id],
        |row| {
            Ok(CuentaPorPagar {
                id: Some(row.get(0)?),
                proveedor_id: row.get(1)?,
                compra_id: row.get(2)?,
                monto_total: row.get(3)?,
                monto_pagado: row.get(4)?,
                saldo: row.get(5)?,
                estado: row.get(6)?,
                fecha_vencimiento: row.get(7)?,
                observacion: row.get(8)?,
                created_at: row.get(9)?,
                proveedor_nombre: row.get(10)?,
                compra_numero: row.get(11)?,
            })
        },
    )
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn historial_pagos_proveedor(
    db: State<Database>,
    cuenta_id: i64,
) -> Result<Vec<PagoProveedor>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;

    let mut stmt = conn
        .prepare(
            "SELECT pp.id, pp.cuenta_id, pp.monto, pp.fecha, pp.forma_pago, pp.numero_comprobante, pp.observacion,
                    pp.banco_id, cb.nombre as banco_nombre
             FROM pagos_proveedor pp
             LEFT JOIN cuentas_banco cb ON pp.banco_id = cb.id
             WHERE pp.cuenta_id = ?1
             ORDER BY pp.fecha DESC",
        )
        .map_err(|e| e.to_string())?;

    let resultado = stmt
        .query_map(rusqlite::params![cuenta_id], |row| {
            Ok(PagoProveedor {
                id: Some(row.get(0)?),
                cuenta_id: row.get(1)?,
                monto: row.get(2)?,
                fecha: row.get(3)?,
                forma_pago: row.get(4)?,
                numero_comprobante: row.get(5)?,
                observacion: row.get(6)?,
                banco_id: row.get(7)?,
                banco_nombre: row.get(8)?,
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    Ok(resultado)
}

#[tauri::command]
pub fn listar_movimientos_bancarios(
    db: State<Database>,
    banco_id: Option<i64>,
    fecha_desde: String,
    fecha_hasta: String,
) -> Result<Vec<serde_json::Value>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;

    // Build UNION ALL query for all bank movement sources.
    // origen_id = ID de la fila en la tabla origen (venta.id, retiro.id, etc.)
    //   permite al frontend abrir el detalle del documento al expandir la fila.
    // pago_estado = solo aplica a VENTA/PAGO_VENTA: REGISTRADO/VERIFICADO/RECHAZADO/NO_APLICA
    // tiene_comprobante = bool, indica si hay imagen adjunta para mostrar boton "Ver comprobante"
    let mut sql = String::from(
        "SELECT tipo, referencia, monto, fecha, banco_nombre, detalle, banco_id,
                origen_id, pago_estado, tiene_comprobante FROM (
            -- Ventas con un solo pago tipo TRANSFER (forma_pago en ventas)
            SELECT 'VENTA' as tipo, v.numero as referencia, v.total as monto, v.fecha,
                   cb.nombre as banco_nombre,
                   COALESCE(cl.nombre, 'Consumidor Final') as detalle,
                   v.banco_id,
                   v.id as origen_id,
                   COALESCE(v.pago_estado, 'NO_APLICA') as pago_estado,
                   CASE WHEN v.comprobante_imagen IS NOT NULL AND v.comprobante_imagen != '' THEN 1 ELSE 0 END as tiene_comprobante
            FROM ventas v
            LEFT JOIN cuentas_banco cb ON v.banco_id = cb.id
            LEFT JOIN clientes cl ON v.cliente_id = cl.id
            WHERE v.banco_id IS NOT NULL AND v.forma_pago = 'TRANSFER'
              AND v.anulada = 0 AND v.tipo_estado = 'COMPLETADA'
              AND v.tipo_documento IN ('NOTA_VENTA', 'FACTURA')
              AND date(v.fecha) >= date(?1) AND date(v.fecha) <= date(?2)

            UNION ALL

            -- Ventas MIXTAS: porcion bancaria desde pagos_venta
            SELECT 'PAGO_VENTA' as tipo, v.numero as referencia, pv.monto as monto, v.fecha,
                   cb.nombre as banco_nombre,
                   COALESCE(cl.nombre, 'Consumidor Final') || ' (mixto)' as detalle,
                   pv.banco_id,
                   pv.id as origen_id,
                   COALESCE(pv.pago_estado, 'NO_APLICA') as pago_estado,
                   CASE WHEN pv.comprobante_imagen IS NOT NULL AND pv.comprobante_imagen != '' THEN 1 ELSE 0 END as tiene_comprobante
            FROM pagos_venta pv
            INNER JOIN ventas v ON v.id = pv.venta_id
            LEFT JOIN cuentas_banco cb ON pv.banco_id = cb.id
            LEFT JOIN clientes cl ON v.cliente_id = cl.id
            WHERE pv.banco_id IS NOT NULL AND pv.forma_pago = 'TRANSFER'
              AND v.anulada = 0 AND v.tipo_estado = 'COMPLETADA'
              AND v.tipo_documento IN ('NOTA_VENTA', 'FACTURA')
              AND date(v.fecha) >= date(?1) AND date(v.fecha) <= date(?2)

            UNION ALL

            SELECT 'RETIRO_CAJA' as tipo, COALESCE(r.referencia, r.motivo) as referencia,
                   r.monto, r.fecha,
                   cb.nombre as banco_nombre,
                   r.motivo as detalle,
                   r.banco_id,
                   r.id as origen_id,
                   'NO_APLICA' as pago_estado,
                   CASE WHEN r.comprobante_imagen IS NOT NULL AND r.comprobante_imagen != '' THEN 1 ELSE 0 END as tiene_comprobante
            FROM retiros_caja r
            LEFT JOIN cuentas_banco cb ON r.banco_id = cb.id
            WHERE r.banco_id IS NOT NULL
              AND date(r.fecha) >= date(?1) AND date(r.fecha) <= date(?2)

            UNION ALL

            SELECT 'PAGO_PROVEEDOR' as tipo, COALESCE(pp.numero_comprobante, '') as referencia,
                   pp.monto, pp.fecha,
                   cb.nombre as banco_nombre,
                   p.nombre as detalle,
                   pp.banco_id,
                   pp.id as origen_id,
                   'NO_APLICA' as pago_estado,
                   0 as tiene_comprobante
            FROM pagos_proveedor pp
            LEFT JOIN cuentas_banco cb ON pp.banco_id = cb.id
            LEFT JOIN cuentas_por_pagar cp ON pp.cuenta_id = cp.id
            LEFT JOIN proveedores p ON cp.proveedor_id = p.id
            WHERE pp.banco_id IS NOT NULL AND pp.forma_pago = 'TRANSFERENCIA'
              AND date(pp.fecha) >= date(?1) AND date(pp.fecha) <= date(?2)

            UNION ALL

            SELECT 'COBRO_CREDITO' as tipo, COALESCE(pc.numero_comprobante, '') as referencia,
                   pc.monto, pc.fecha,
                   cb.nombre as banco_nombre,
                   cl.nombre as detalle,
                   pc.banco_id,
                   pc.id as origen_id,
                   'NO_APLICA' as pago_estado,
                   CASE WHEN pc.comprobante_imagen IS NOT NULL AND pc.comprobante_imagen != '' THEN 1 ELSE 0 END as tiene_comprobante
            FROM pagos_cuenta pc
            LEFT JOIN cuentas_banco cb ON pc.banco_id = cb.id
            LEFT JOIN cuentas_por_cobrar cc ON pc.cuenta_id = cc.id
            LEFT JOIN clientes cl ON cc.cliente_id = cl.id
            WHERE pc.banco_id IS NOT NULL AND pc.forma_pago = 'TRANSFERENCIA'
              AND date(pc.fecha) >= date(?1) AND date(pc.fecha) <= date(?2)
        ) movimientos",
    );

    if banco_id.is_some() {
        sql.push_str(" WHERE banco_id = ?3");
    }

    sql.push_str(" ORDER BY fecha DESC");

    let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;

    let mapper = |row: &rusqlite::Row| -> rusqlite::Result<serde_json::Value> {
        let tipo: String = row.get(0)?;
        let monto_raw: f64 = row.get(2)?;
        // INGRESOS al banco (positivos): VENTA / PAGO_VENTA por transfer, COBRO de credito,
        // RETIRO de caja depositado al banco
        // EGRESOS del banco (negativos): PAGO a proveedor por transferencia
        let es_ingreso = matches!(tipo.as_str(), "VENTA" | "PAGO_VENTA" | "COBRO_CREDITO" | "RETIRO_CAJA");
        let monto = if es_ingreso { monto_raw } else { -monto_raw };
        Ok(serde_json::json!({
            "tipo": tipo,
            "referencia": row.get::<_, String>(1)?,
            "monto": monto,
            "fecha": row.get::<_, String>(3)?,
            "banco_nombre": row.get::<_, Option<String>>(4)?,
            "detalle": row.get::<_, Option<String>>(5)?,
            "origen_id": row.get::<_, i64>(7)?,
            "pago_estado": row.get::<_, String>(8)?,
            "tiene_comprobante": row.get::<_, i64>(9)? != 0,
        }))
    };
    let resultado: Vec<serde_json::Value> = if let Some(bid) = banco_id {
        stmt.query_map(rusqlite::params![fecha_desde, fecha_hasta, bid], mapper)
            .map_err(|e| e.to_string())?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())?
    } else {
        stmt.query_map(rusqlite::params![fecha_desde, fecha_hasta], mapper)
            .map_err(|e| e.to_string())?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())?
    };

    Ok(resultado)
}

/// Devuelve el detalle completo de un movimiento bancario para mostrarlo al expandir
/// la fila. Cada `tipo` tiene su propia estructura. El frontend renderiza segun el tipo.
#[tauri::command]
pub fn obtener_detalle_movimiento_bancario(
    db: State<Database>,
    tipo: String,
    origen_id: i64,
) -> Result<serde_json::Value, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    match tipo.as_str() {
        "VENTA" => {
            let v: serde_json::Value = conn.query_row(
                "SELECT v.id, v.numero, v.fecha, v.subtotal_sin_iva, v.subtotal_con_iva, v.iva,
                        v.descuento, v.total, v.forma_pago, v.tipo_documento, v.estado_sri,
                        v.observacion, v.usuario, v.banco_id, cb.nombre as banco_nombre,
                        v.referencia_pago, v.comprobante_imagen,
                        COALESCE(v.pago_estado, 'NO_APLICA'), v.verificado_por,
                        u.nombre as verificador, v.fecha_verificacion, v.motivo_verificacion,
                        cl.nombre as cliente_nombre, cl.identificacion as cliente_cedula, cl.telefono, cl.email
                 FROM ventas v
                 LEFT JOIN cuentas_banco cb ON v.banco_id = cb.id
                 LEFT JOIN clientes cl ON v.cliente_id = cl.id
                 LEFT JOIN usuarios u ON v.verificado_por = u.id
                 WHERE v.id = ?1",
                rusqlite::params![origen_id],
                |r| Ok(serde_json::json!({
                    "id": r.get::<_, i64>(0)?,
                    "numero": r.get::<_, String>(1)?,
                    "fecha": r.get::<_, String>(2)?,
                    "subtotal_sin_iva": r.get::<_, f64>(3)?,
                    "subtotal_con_iva": r.get::<_, f64>(4)?,
                    "iva": r.get::<_, f64>(5)?,
                    "descuento": r.get::<_, f64>(6)?,
                    "total": r.get::<_, f64>(7)?,
                    "forma_pago": r.get::<_, String>(8)?,
                    "tipo_documento": r.get::<_, String>(9)?,
                    "estado_sri": r.get::<_, Option<String>>(10)?,
                    "observacion": r.get::<_, Option<String>>(11)?,
                    "usuario": r.get::<_, Option<String>>(12)?,
                    "banco_id": r.get::<_, Option<i64>>(13)?,
                    "banco_nombre": r.get::<_, Option<String>>(14)?,
                    "referencia_pago": r.get::<_, Option<String>>(15)?,
                    "comprobante_imagen": r.get::<_, Option<String>>(16)?,
                    "pago_estado": r.get::<_, String>(17)?,
                    "verificado_por": r.get::<_, Option<i64>>(18)?,
                    "verificador_nombre": r.get::<_, Option<String>>(19)?,
                    "fecha_verificacion": r.get::<_, Option<String>>(20)?,
                    "motivo_verificacion": r.get::<_, Option<String>>(21)?,
                    "cliente_nombre": r.get::<_, Option<String>>(22)?,
                    "cliente_cedula": r.get::<_, Option<String>>(23)?,
                    "cliente_telefono": r.get::<_, Option<String>>(24)?,
                    "cliente_email": r.get::<_, Option<String>>(25)?,
                })),
            ).map_err(|e| format!("Venta no encontrada: {}", e))?;
            // Items de la venta
            let mut stmt = conn.prepare(
                "SELECT p.nombre, vd.cantidad, vd.precio_unitario, vd.subtotal
                 FROM venta_detalles vd JOIN productos p ON p.id = vd.producto_id
                 WHERE vd.venta_id = ?1"
            ).map_err(|e| e.to_string())?;
            let items: Vec<serde_json::Value> = stmt.query_map(
                rusqlite::params![origen_id], |r| Ok(serde_json::json!({
                    "nombre": r.get::<_, String>(0)?,
                    "cantidad": r.get::<_, f64>(1)?,
                    "precio_unitario": r.get::<_, f64>(2)?,
                    "subtotal": r.get::<_, f64>(3)?,
                }))
            ).map_err(|e| e.to_string())?
              .collect::<Result<Vec<_>, _>>()
              .unwrap_or_default();
            let mut out = v;
            out["items"] = serde_json::Value::Array(items);
            Ok(out)
        }
        "PAGO_VENTA" => {
            // Componente TRANSFER de venta MIXTO
            let p: serde_json::Value = conn.query_row(
                "SELECT pv.id, pv.venta_id, v.numero as venta_numero, v.fecha as venta_fecha,
                        v.total as venta_total, pv.monto, pv.forma_pago,
                        pv.banco_id, cb.nombre as banco_nombre, pv.referencia, pv.comprobante_imagen,
                        COALESCE(pv.pago_estado, 'NO_APLICA'), pv.verificado_por,
                        u.nombre as verificador, pv.fecha_verificacion, pv.motivo_verificacion,
                        cl.nombre as cliente_nombre, v.usuario as cajero
                 FROM pagos_venta pv
                 JOIN ventas v ON v.id = pv.venta_id
                 LEFT JOIN cuentas_banco cb ON pv.banco_id = cb.id
                 LEFT JOIN clientes cl ON v.cliente_id = cl.id
                 LEFT JOIN usuarios u ON pv.verificado_por = u.id
                 WHERE pv.id = ?1",
                rusqlite::params![origen_id],
                |r| Ok(serde_json::json!({
                    "id": r.get::<_, i64>(0)?,
                    "venta_id": r.get::<_, i64>(1)?,
                    "venta_numero": r.get::<_, String>(2)?,
                    "venta_fecha": r.get::<_, String>(3)?,
                    "venta_total": r.get::<_, f64>(4)?,
                    "monto": r.get::<_, f64>(5)?,
                    "forma_pago": r.get::<_, String>(6)?,
                    "banco_id": r.get::<_, Option<i64>>(7)?,
                    "banco_nombre": r.get::<_, Option<String>>(8)?,
                    "referencia": r.get::<_, Option<String>>(9)?,
                    "comprobante_imagen": r.get::<_, Option<String>>(10)?,
                    "pago_estado": r.get::<_, String>(11)?,
                    "verificado_por": r.get::<_, Option<i64>>(12)?,
                    "verificador_nombre": r.get::<_, Option<String>>(13)?,
                    "fecha_verificacion": r.get::<_, Option<String>>(14)?,
                    "motivo_verificacion": r.get::<_, Option<String>>(15)?,
                    "cliente_nombre": r.get::<_, Option<String>>(16)?,
                    "cajero": r.get::<_, Option<String>>(17)?,
                })),
            ).map_err(|e| format!("Pago no encontrado: {}", e))?;
            Ok(p)
        }
        "RETIRO_CAJA" => {
            let r: serde_json::Value = conn.query_row(
                "SELECT r.id, r.caja_id, r.fecha, r.monto, r.motivo, r.usuario,
                        r.banco_id, cb.nombre as banco_nombre, r.referencia, r.comprobante_imagen,
                        COALESCE(r.estado, 'SIN_DEPOSITO')
                 FROM retiros_caja r
                 LEFT JOIN cuentas_banco cb ON r.banco_id = cb.id
                 WHERE r.id = ?1",
                rusqlite::params![origen_id],
                |r| Ok(serde_json::json!({
                    "id": r.get::<_, i64>(0)?,
                    "caja_id": r.get::<_, i64>(1)?,
                    "fecha": r.get::<_, String>(2)?,
                    "monto": r.get::<_, f64>(3)?,
                    "motivo": r.get::<_, Option<String>>(4)?,
                    "usuario": r.get::<_, Option<String>>(5)?,
                    "banco_id": r.get::<_, Option<i64>>(6)?,
                    "banco_nombre": r.get::<_, Option<String>>(7)?,
                    "referencia": r.get::<_, Option<String>>(8)?,
                    "comprobante_imagen": r.get::<_, Option<String>>(9)?,
                    "estado": r.get::<_, String>(10)?,
                })),
            ).map_err(|e| format!("Retiro no encontrado: {}", e))?;
            Ok(r)
        }
        "PAGO_PROVEEDOR" => {
            // v2.3.51 FIX: cuentas_por_pagar NO tiene factura_numero ni fecha_factura.
            // Esos datos viven en `compras` (compra_id es la FK). LEFT JOIN compras.
            let p: serde_json::Value = conn.query_row(
                "SELECT pp.id, pp.cuenta_id, pp.fecha, pp.monto, pp.forma_pago,
                        pp.numero_comprobante, pp.banco_id, cb.nombre as banco_nombre,
                        pr.nombre as proveedor_nombre, pr.ruc as proveedor_ruc,
                        pr.telefono as proveedor_tel,
                        co.numero_factura, co.fecha as fecha_factura, cp.monto_total
                 FROM pagos_proveedor pp
                 LEFT JOIN cuentas_banco cb ON pp.banco_id = cb.id
                 LEFT JOIN cuentas_por_pagar cp ON pp.cuenta_id = cp.id
                 LEFT JOIN proveedores pr ON cp.proveedor_id = pr.id
                 LEFT JOIN compras co ON cp.compra_id = co.id
                 WHERE pp.id = ?1",
                rusqlite::params![origen_id],
                |r| Ok(serde_json::json!({
                    "id": r.get::<_, i64>(0)?,
                    "cuenta_id": r.get::<_, Option<i64>>(1)?,
                    "fecha": r.get::<_, String>(2)?,
                    "monto": r.get::<_, f64>(3)?,
                    "forma_pago": r.get::<_, String>(4)?,
                    "numero_comprobante": r.get::<_, Option<String>>(5)?,
                    "banco_id": r.get::<_, Option<i64>>(6)?,
                    "banco_nombre": r.get::<_, Option<String>>(7)?,
                    "proveedor_nombre": r.get::<_, Option<String>>(8)?,
                    "proveedor_ruc": r.get::<_, Option<String>>(9)?,
                    "proveedor_telefono": r.get::<_, Option<String>>(10)?,
                    "factura_numero": r.get::<_, Option<String>>(11)?,
                    "fecha_factura": r.get::<_, Option<String>>(12)?,
                    "factura_total": r.get::<_, Option<f64>>(13)?,
                })),
            ).map_err(|e| format!("Pago a proveedor no encontrado: {}", e))?;
            Ok(p)
        }
        "COBRO_CREDITO" => {
            let p: serde_json::Value = conn.query_row(
                "SELECT pc.id, pc.cuenta_id, pc.fecha, pc.monto, pc.forma_pago,
                        pc.numero_comprobante, pc.banco_id, cb.nombre as banco_nombre,
                        pc.comprobante_imagen, pc.observacion,
                        cl.nombre as cliente_nombre, cl.identificacion as cliente_cedula,
                        cl.telefono as cliente_telefono,
                        cc.venta_id, vv.numero as venta_numero, cc.monto_total, cc.saldo
                 FROM pagos_cuenta pc
                 LEFT JOIN cuentas_banco cb ON pc.banco_id = cb.id
                 LEFT JOIN cuentas_por_cobrar cc ON pc.cuenta_id = cc.id
                 LEFT JOIN clientes cl ON cc.cliente_id = cl.id
                 LEFT JOIN ventas vv ON cc.venta_id = vv.id
                 WHERE pc.id = ?1",
                rusqlite::params![origen_id],
                |r| Ok(serde_json::json!({
                    "id": r.get::<_, i64>(0)?,
                    "cuenta_id": r.get::<_, Option<i64>>(1)?,
                    "fecha": r.get::<_, String>(2)?,
                    "monto": r.get::<_, f64>(3)?,
                    "forma_pago": r.get::<_, String>(4)?,
                    "numero_comprobante": r.get::<_, Option<String>>(5)?,
                    "banco_id": r.get::<_, Option<i64>>(6)?,
                    "banco_nombre": r.get::<_, Option<String>>(7)?,
                    "comprobante_imagen": r.get::<_, Option<String>>(8)?,
                    "observacion": r.get::<_, Option<String>>(9)?,
                    "cliente_nombre": r.get::<_, Option<String>>(10)?,
                    "cliente_cedula": r.get::<_, Option<String>>(11)?,
                    "cliente_telefono": r.get::<_, Option<String>>(12)?,
                    "venta_id": r.get::<_, Option<i64>>(13)?,
                    "venta_numero": r.get::<_, Option<String>>(14)?,
                    "credito_total": r.get::<_, Option<f64>>(15)?,
                    "credito_saldo": r.get::<_, Option<f64>>(16)?,
                })),
            ).map_err(|e| format!("Cobro no encontrado: {}", e))?;
            Ok(p)
        }
        _ => Err(format!("Tipo de movimiento no soportado: {}", tipo)),
    }
}
