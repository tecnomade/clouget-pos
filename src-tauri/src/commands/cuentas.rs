use crate::db::Database;
use crate::models::{CuentaConCliente, CuentaDetalle, CuentaPorCobrar, PagoCuenta, ResumenCliente};
use tauri::State;

#[tauri::command]
pub fn resumen_deudores(db: State<Database>) -> Result<Vec<ResumenCliente>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;

    let mut stmt = conn
        .prepare(
            "SELECT c.cliente_id, cl.nombre, SUM(c.saldo) as total_deuda, COUNT(*) as num_cuentas
             FROM cuentas_por_cobrar c
             JOIN clientes cl ON c.cliente_id = cl.id
             WHERE c.estado = 'PENDIENTE'
             GROUP BY c.cliente_id
             ORDER BY total_deuda DESC",
        )
        .map_err(|e| e.to_string())?;

    let resultado = stmt
        .query_map([], |row| {
            Ok(ResumenCliente {
                cliente_id: row.get(0)?,
                cliente_nombre: row.get(1)?,
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
pub fn listar_cuentas_pendientes(
    db: State<Database>,
    cliente_id: Option<i64>,
) -> Result<Vec<CuentaConCliente>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;

    let sql = if cliente_id.is_some() {
        "SELECT c.id, c.cliente_id, c.venta_id, c.monto_total, c.monto_pagado, c.saldo,
                c.estado, c.fecha_vencimiento, c.created_at, cl.nombre, v.numero
         FROM cuentas_por_cobrar c
         JOIN clientes cl ON c.cliente_id = cl.id
         JOIN ventas v ON c.venta_id = v.id
         WHERE c.estado = 'PENDIENTE' AND c.cliente_id = ?1
         ORDER BY c.created_at DESC"
    } else {
        "SELECT c.id, c.cliente_id, c.venta_id, c.monto_total, c.monto_pagado, c.saldo,
                c.estado, c.fecha_vencimiento, c.created_at, cl.nombre, v.numero
         FROM cuentas_por_cobrar c
         JOIN clientes cl ON c.cliente_id = cl.id
         JOIN ventas v ON c.venta_id = v.id
         WHERE c.estado = 'PENDIENTE'
         ORDER BY cl.nombre, c.created_at DESC"
    };

    let mut stmt = conn.prepare(sql).map_err(|e| e.to_string())?;

    let map_row = |row: &rusqlite::Row| -> rusqlite::Result<CuentaConCliente> {
        Ok(CuentaConCliente {
            cuenta: CuentaPorCobrar {
                id: Some(row.get(0)?),
                cliente_id: row.get(1)?,
                venta_id: row.get(2)?,
                monto_total: row.get(3)?,
                monto_pagado: row.get(4)?,
                saldo: row.get(5)?,
                estado: row.get(6)?,
                fecha_vencimiento: row.get(7)?,
                created_at: row.get(8)?,
            },
            cliente_nombre: row.get(9)?,
            venta_numero: row.get(10)?,
        })
    };

    let resultado = if let Some(cid) = cliente_id {
        stmt.query_map(rusqlite::params![cid], map_row)
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
pub fn obtener_cuenta_detalle(db: State<Database>, id: i64) -> Result<CuentaDetalle, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;

    let cuenta_con_cliente = conn
        .query_row(
            "SELECT c.id, c.cliente_id, c.venta_id, c.monto_total, c.monto_pagado, c.saldo,
                    c.estado, c.fecha_vencimiento, c.created_at, cl.nombre, v.numero
             FROM cuentas_por_cobrar c
             JOIN clientes cl ON c.cliente_id = cl.id
             JOIN ventas v ON c.venta_id = v.id
             WHERE c.id = ?1",
            rusqlite::params![id],
            |row| {
                Ok((
                    CuentaPorCobrar {
                        id: Some(row.get(0)?),
                        cliente_id: row.get(1)?,
                        venta_id: row.get(2)?,
                        monto_total: row.get(3)?,
                        monto_pagado: row.get(4)?,
                        saldo: row.get(5)?,
                        estado: row.get(6)?,
                        fecha_vencimiento: row.get(7)?,
                        created_at: row.get(8)?,
                    },
                    row.get::<_, String>(9)?,
                    row.get::<_, String>(10)?,
                ))
            },
        )
        .map_err(|e| e.to_string())?;

    let mut stmt = conn
        .prepare(
            "SELECT id, cuenta_id, monto, fecha, observacion
             FROM pagos_cuenta WHERE cuenta_id = ?1 ORDER BY fecha DESC",
        )
        .map_err(|e| e.to_string())?;

    let pagos = stmt
        .query_map(rusqlite::params![id], |row| {
            Ok(PagoCuenta {
                id: Some(row.get(0)?),
                cuenta_id: row.get(1)?,
                monto: row.get(2)?,
                fecha: row.get(3)?,
                observacion: row.get(4)?,
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    Ok(CuentaDetalle {
        cuenta: cuenta_con_cliente.0,
        cliente_nombre: cuenta_con_cliente.1,
        venta_numero: cuenta_con_cliente.2,
        pagos,
    })
}

#[tauri::command]
pub fn registrar_pago_cuenta(db: State<Database>, pago: PagoCuenta) -> Result<CuentaPorCobrar, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;

    // Obtener saldo actual
    let saldo_actual: f64 = conn
        .query_row(
            "SELECT saldo FROM cuentas_por_cobrar WHERE id = ?1 AND estado = 'PENDIENTE'",
            rusqlite::params![pago.cuenta_id],
            |row| row.get(0),
        )
        .map_err(|_| "Cuenta no encontrada o ya pagada".to_string())?;

    if pago.monto <= 0.0 {
        return Err("El monto debe ser mayor a 0".to_string());
    }

    if pago.monto > saldo_actual + 0.01 {
        return Err(format!(
            "El monto (${:.2}) excede el saldo pendiente (${:.2})",
            pago.monto, saldo_actual
        ));
    }

    // Insertar pago
    conn.execute(
        "INSERT INTO pagos_cuenta (cuenta_id, monto, observacion) VALUES (?1, ?2, ?3)",
        rusqlite::params![pago.cuenta_id, pago.monto, pago.observacion],
    )
    .map_err(|e| e.to_string())?;

    // Actualizar cuenta
    let nuevo_saldo = saldo_actual - pago.monto;
    let nuevo_estado = if nuevo_saldo <= 0.01 { "PAGADA" } else { "PENDIENTE" };

    conn.execute(
        "UPDATE cuentas_por_cobrar
         SET monto_pagado = monto_pagado + ?1, saldo = ?2, estado = ?3,
             updated_at = datetime('now','localtime')
         WHERE id = ?4",
        rusqlite::params![pago.monto, nuevo_saldo.max(0.0), nuevo_estado, pago.cuenta_id],
    )
    .map_err(|e| e.to_string())?;

    // Si hay caja abierta, sumar el pago como ingreso de efectivo
    let _ = conn.execute(
        "UPDATE caja SET monto_ventas = monto_ventas + ?1, monto_esperado = monto_esperado + ?1
         WHERE estado = 'ABIERTA'",
        rusqlite::params![pago.monto],
    );

    // Retornar cuenta actualizada
    conn.query_row(
        "SELECT id, cliente_id, venta_id, monto_total, monto_pagado, saldo, estado,
                fecha_vencimiento, created_at
         FROM cuentas_por_cobrar WHERE id = ?1",
        rusqlite::params![pago.cuenta_id],
        |row| {
            Ok(CuentaPorCobrar {
                id: Some(row.get(0)?),
                cliente_id: row.get(1)?,
                venta_id: row.get(2)?,
                monto_total: row.get(3)?,
                monto_pagado: row.get(4)?,
                saldo: row.get(5)?,
                estado: row.get(6)?,
                fecha_vencimiento: row.get(7)?,
                created_at: row.get(8)?,
            })
        },
    )
    .map_err(|e| e.to_string())
}
