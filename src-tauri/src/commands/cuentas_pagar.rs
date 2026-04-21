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

    // Build UNION ALL query for all bank movement sources
    let mut sql = String::from(
        "SELECT tipo, referencia, monto, fecha, banco_nombre, detalle, banco_id FROM (
            -- Ventas con un solo pago tipo TRANSFER (forma_pago en ventas)
            SELECT 'VENTA' as tipo, v.numero as referencia, v.total as monto, v.fecha,
                   cb.nombre as banco_nombre,
                   COALESCE(cl.nombre, 'Consumidor Final') as detalle,
                   v.banco_id
            FROM ventas v
            LEFT JOIN cuentas_banco cb ON v.banco_id = cb.id
            LEFT JOIN clientes cl ON v.cliente_id = cl.id
            WHERE v.banco_id IS NOT NULL AND v.forma_pago = 'TRANSFER'
              AND v.anulada = 0 AND v.tipo_estado = 'COMPLETADA'
              AND v.tipo_documento IN ('NOTA_VENTA', 'FACTURA')
              AND date(v.fecha) >= date(?1) AND date(v.fecha) <= date(?2)

            UNION ALL

            -- Ventas MIXTAS: porcion bancaria desde pagos_venta
            SELECT 'VENTA' as tipo, v.numero as referencia, pv.monto as monto, v.fecha,
                   cb.nombre as banco_nombre,
                   COALESCE(cl.nombre, 'Consumidor Final') as detalle,
                   pv.banco_id
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
                   r.banco_id
            FROM retiros_caja r
            LEFT JOIN cuentas_banco cb ON r.banco_id = cb.id
            WHERE r.banco_id IS NOT NULL
              AND date(r.fecha) >= date(?1) AND date(r.fecha) <= date(?2)

            UNION ALL

            SELECT 'PAGO_PROVEEDOR' as tipo, COALESCE(pp.numero_comprobante, '') as referencia,
                   pp.monto, pp.fecha,
                   cb.nombre as banco_nombre,
                   p.nombre as detalle,
                   pp.banco_id
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
                   pc.banco_id
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

    let resultado = if let Some(bid) = banco_id {
        stmt.query_map(rusqlite::params![fecha_desde, fecha_hasta, bid], |row| {
            let tipo: String = row.get(0)?;
            let monto_raw: f64 = row.get(2)?;
            // Egresos son negativos (retiros y pagos proveedor)
            // INGRESOS al banco (positivos): VENTA por transfer, COBRO de credito, RETIRO de caja depositado al banco
            // EGRESOS del banco (negativos): PAGO a proveedor por transferencia
            let monto = if tipo == "VENTA" || tipo == "COBRO_CREDITO" || tipo == "RETIRO_CAJA" { monto_raw } else { -monto_raw };
            Ok(serde_json::json!({
                "tipo": tipo,
                "referencia": row.get::<_, String>(1)?,
                "monto": monto,
                "fecha": row.get::<_, String>(3)?,
                "banco_nombre": row.get::<_, Option<String>>(4)?,
                "detalle": row.get::<_, Option<String>>(5)?,
            }))
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?
    } else {
        stmt.query_map(rusqlite::params![fecha_desde, fecha_hasta], |row| {
            let tipo: String = row.get(0)?;
            let monto_raw: f64 = row.get(2)?;
            // INGRESOS al banco (positivos): VENTA por transfer, COBRO de credito, RETIRO de caja depositado al banco
            // EGRESOS del banco (negativos): PAGO a proveedor por transferencia
            let monto = if tipo == "VENTA" || tipo == "COBRO_CREDITO" || tipo == "RETIRO_CAJA" { monto_raw } else { -monto_raw };
            Ok(serde_json::json!({
                "tipo": tipo,
                "referencia": row.get::<_, String>(1)?,
                "monto": monto,
                "fecha": row.get::<_, String>(3)?,
                "banco_nombre": row.get::<_, Option<String>>(4)?,
                "detalle": row.get::<_, Option<String>>(5)?,
            }))
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?
    };

    Ok(resultado)
}
