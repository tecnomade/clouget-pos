use crate::db::{Database, SesionState};
use crate::models::{CuentaBanco, CuentaConCliente, CuentaDetalle, CuentaPorCobrar, PagoCuenta, ResumenCliente};
use crate::commands::usuarios::verificar_admin;
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
            "SELECT p.id, p.cuenta_id, p.monto, p.fecha, p.observacion,
                    p.forma_pago, p.banco_id, p.numero_comprobante, p.comprobante_imagen,
                    b.nombre, p.estado, p.confirmado_por, p.fecha_confirmacion
             FROM pagos_cuenta p
             LEFT JOIN cuentas_banco b ON p.banco_id = b.id
             WHERE p.cuenta_id = ?1 ORDER BY p.fecha DESC",
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
                forma_pago: row.get::<_, Option<String>>(5)?.unwrap_or_else(|| "EFECTIVO".to_string()),
                banco_id: row.get(6)?,
                numero_comprobante: row.get(7)?,
                comprobante_imagen: row.get(8)?,
                banco_nombre: row.get(9)?,
                estado: row.get(10)?,
                confirmado_por: row.get(11)?,
                fecha_confirmacion: row.get(12)?,
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

    let forma_pago = if pago.forma_pago.is_empty() { "EFECTIVO" } else { &pago.forma_pago };

    // Para transferencias, descontar pagos pendientes del saldo disponible
    let saldo_disponible = if forma_pago != "EFECTIVO" {
        let pendientes: f64 = conn
            .query_row(
                "SELECT COALESCE(SUM(monto), 0) FROM pagos_cuenta
                 WHERE cuenta_id = ?1 AND estado = 'PENDIENTE'",
                rusqlite::params![pago.cuenta_id],
                |row| row.get(0),
            )
            .unwrap_or(0.0);
        saldo_actual - pendientes
    } else {
        saldo_actual
    };

    if pago.monto > saldo_disponible + 0.01 {
        return Err(format!(
            "El monto (${:.2}) excede el saldo disponible (${:.2})",
            pago.monto, saldo_disponible
        ));
    }

    // EFECTIVO → CONFIRMADO inmediato; TRANSFERENCIA → PENDIENTE hasta que admin confirme
    let estado_pago = if forma_pago == "EFECTIVO" { "CONFIRMADO" } else { "PENDIENTE" };

    conn.execute(
        "INSERT INTO pagos_cuenta (cuenta_id, monto, observacion, forma_pago, banco_id, numero_comprobante, comprobante_imagen, estado)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        rusqlite::params![
            pago.cuenta_id,
            pago.monto,
            pago.observacion,
            forma_pago,
            pago.banco_id,
            pago.numero_comprobante,
            pago.comprobante_imagen,
            estado_pago,
        ],
    )
    .map_err(|e| e.to_string())?;

    // Solo aplicar saldo y caja para pagos CONFIRMADOS (efectivo)
    if forma_pago == "EFECTIVO" {
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

        // Sumar al monto esperado de caja abierta
        let _ = conn.execute(
            "UPDATE caja SET monto_ventas = monto_ventas + ?1, monto_esperado = monto_esperado + ?1
             WHERE estado = 'ABIERTA'",
            rusqlite::params![pago.monto],
        );
    }
    // Transferencias: no tocan saldo ni caja hasta confirmación de admin

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

// --- CRUD Cuentas Banco ---

#[tauri::command]
pub fn listar_cuentas_banco(db: State<Database>) -> Result<Vec<CuentaBanco>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;

    let mut stmt = conn
        .prepare(
            "SELECT id, nombre, tipo_cuenta, numero_cuenta, titular, activa
             FROM cuentas_banco WHERE activa = 1 ORDER BY nombre",
        )
        .map_err(|e| e.to_string())?;

    let resultado = stmt
        .query_map([], |row| {
            Ok(CuentaBanco {
                id: Some(row.get(0)?),
                nombre: row.get(1)?,
                tipo_cuenta: row.get(2)?,
                numero_cuenta: row.get(3)?,
                titular: row.get(4)?,
                activa: row.get(5)?,
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    Ok(resultado)
}

#[tauri::command]
pub fn crear_cuenta_banco(
    db: State<Database>,
    sesion: State<SesionState>,
    cuenta: CuentaBanco,
) -> Result<CuentaBanco, String> {
    verificar_admin(&sesion)?;
    let conn = db.conn.lock().map_err(|e| e.to_string())?;

    conn.execute(
        "INSERT INTO cuentas_banco (nombre, tipo_cuenta, numero_cuenta, titular)
         VALUES (?1, ?2, ?3, ?4)",
        rusqlite::params![cuenta.nombre, cuenta.tipo_cuenta, cuenta.numero_cuenta, cuenta.titular],
    )
    .map_err(|e| e.to_string())?;

    let id = conn.last_insert_rowid();

    Ok(CuentaBanco {
        id: Some(id),
        nombre: cuenta.nombre,
        tipo_cuenta: cuenta.tipo_cuenta,
        numero_cuenta: cuenta.numero_cuenta,
        titular: cuenta.titular,
        activa: true,
    })
}

#[tauri::command]
pub fn actualizar_cuenta_banco(
    db: State<Database>,
    sesion: State<SesionState>,
    id: i64,
    cuenta: CuentaBanco,
) -> Result<(), String> {
    verificar_admin(&sesion)?;
    let conn = db.conn.lock().map_err(|e| e.to_string())?;

    conn.execute(
        "UPDATE cuentas_banco SET nombre = ?1, tipo_cuenta = ?2, numero_cuenta = ?3, titular = ?4
         WHERE id = ?5",
        rusqlite::params![cuenta.nombre, cuenta.tipo_cuenta, cuenta.numero_cuenta, cuenta.titular, id],
    )
    .map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
pub fn desactivar_cuenta_banco(
    db: State<Database>,
    sesion: State<SesionState>,
    id: i64,
) -> Result<(), String> {
    verificar_admin(&sesion)?;
    let conn = db.conn.lock().map_err(|e| e.to_string())?;

    conn.execute(
        "UPDATE cuentas_banco SET activa = 0 WHERE id = ?1",
        rusqlite::params![id],
    )
    .map_err(|e| e.to_string())?;

    Ok(())
}

// --- Confirmación de pagos por transferencia ---

#[tauri::command]
pub fn confirmar_pago_cuenta(
    db: State<Database>,
    sesion: State<SesionState>,
    pago_id: i64,
) -> Result<CuentaDetalle, String> {
    verificar_admin(&sesion)?;
    let conn = db.conn.lock().map_err(|e| e.to_string())?;

    // Obtener pago pendiente
    let (cuenta_id, monto): (i64, f64) = conn
        .query_row(
            "SELECT cuenta_id, monto FROM pagos_cuenta WHERE id = ?1 AND estado = 'PENDIENTE'",
            rusqlite::params![pago_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .map_err(|_| "Pago no encontrado o ya procesado".to_string())?;

    // Obtener admin_id de la sesión
    let admin_id = {
        let sesion_guard = sesion.sesion.lock().map_err(|e| e.to_string())?;
        sesion_guard.as_ref().map(|s| s.usuario_id).unwrap_or(0)
    };

    // Marcar como CONFIRMADO
    conn.execute(
        "UPDATE pagos_cuenta SET estado = 'CONFIRMADO', confirmado_por = ?1,
         fecha_confirmacion = datetime('now','localtime') WHERE id = ?2",
        rusqlite::params![admin_id, pago_id],
    )
    .map_err(|e| e.to_string())?;

    // Ahora aplicar reducción de saldo
    let saldo_actual: f64 = conn
        .query_row(
            "SELECT saldo FROM cuentas_por_cobrar WHERE id = ?1",
            rusqlite::params![cuenta_id],
            |row| row.get(0),
        )
        .map_err(|e| e.to_string())?;

    let nuevo_saldo = (saldo_actual - monto).max(0.0);
    let nuevo_estado = if nuevo_saldo <= 0.01 { "PAGADA" } else { "PENDIENTE" };

    conn.execute(
        "UPDATE cuentas_por_cobrar SET monto_pagado = monto_pagado + ?1, saldo = ?2, estado = ?3,
         updated_at = datetime('now','localtime') WHERE id = ?4",
        rusqlite::params![monto, nuevo_saldo, nuevo_estado, cuenta_id],
    )
    .map_err(|e| e.to_string())?;

    // Transferencias NO van a caja (no afectan arqueo físico)
    drop(conn);
    obtener_cuenta_detalle(db, cuenta_id)
}

#[tauri::command]
pub fn rechazar_pago_cuenta(
    db: State<Database>,
    sesion: State<SesionState>,
    pago_id: i64,
    motivo: Option<String>,
) -> Result<CuentaDetalle, String> {
    verificar_admin(&sesion)?;
    let conn = db.conn.lock().map_err(|e| e.to_string())?;

    // Obtener pago pendiente
    let cuenta_id: i64 = conn
        .query_row(
            "SELECT cuenta_id FROM pagos_cuenta WHERE id = ?1 AND estado = 'PENDIENTE'",
            rusqlite::params![pago_id],
            |row| row.get(0),
        )
        .map_err(|_| "Pago no encontrado o ya procesado".to_string())?;

    let admin_id = {
        let sesion_guard = sesion.sesion.lock().map_err(|e| e.to_string())?;
        sesion_guard.as_ref().map(|s| s.usuario_id).unwrap_or(0)
    };

    // Marcar como RECHAZADO — saldo no cambia porque nunca se aplicó
    conn.execute(
        "UPDATE pagos_cuenta SET estado = 'RECHAZADO', confirmado_por = ?1,
         fecha_confirmacion = datetime('now','localtime'),
         observacion = CASE WHEN ?2 IS NOT NULL THEN ?2 ELSE observacion END
         WHERE id = ?3",
        rusqlite::params![admin_id, motivo, pago_id],
    )
    .map_err(|e| e.to_string())?;

    drop(conn);
    obtener_cuenta_detalle(db, cuenta_id)
}

#[tauri::command]
pub fn contar_pagos_pendientes(db: State<Database>) -> Result<i64, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    conn.query_row(
        "SELECT COUNT(*) FROM pagos_cuenta WHERE estado = 'PENDIENTE'",
        [],
        |row| row.get(0),
    )
    .map_err(|e| e.to_string())
}
