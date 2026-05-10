//! v2.4.13 — ST-5: Abonos (anticipos) en órdenes de servicio + cancelación.
//!
//! Cuando el cliente deja un equipo en el taller y paga adelantado, ese
//! dinero entra a caja pero queda en estado HOLDING (no es venta).
//! Cuando la orden se cobra completamente, los abonos se aplican (descuentan
//! del total). Si la orden se cancela, los abonos se devuelven.
//!
//! Comandos expuestos:
//! - `st_listar_abonos(orden_id)` — abonos de una orden con estado y forma de pago
//! - `st_recibir_abono(...)` — registra anticipo, requiere caja abierta
//! - `st_cancelar_orden(orden_id, observacion?)` — marca CANCELADA + devuelve abonos automáticamente
//! - `st_listar_holdings_caja(caja_id?)` — anticipos en HOLDING en la caja activa
//! - `st_total_abonos_orden(orden_id)` — suma de abonos en HOLDING para una orden

use crate::db::{Database, SesionState};
use rusqlite::params;
use serde::{Deserialize, Serialize};
use tauri::State;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AbonoServicio {
    pub id: Option<i64>,
    pub orden_id: i64,
    pub monto: f64,
    pub forma_pago: String,
    pub banco_id: Option<i64>,
    pub banco_nombre: Option<String>,
    pub referencia_pago: Option<String>,
    pub caja_id: Option<i64>,
    pub estado: String, // HOLDING | APLICADO | DEVUELTO
    pub venta_id_aplicado: Option<i64>,
    pub fecha: Option<String>,
    pub fecha_aplicado: Option<String>,
    pub fecha_devuelto: Option<String>,
    pub usuario_nombre: Option<String>,
    pub observacion: Option<String>,
}

#[derive(Debug, Serialize, Clone)]
pub struct HoldingCaja {
    pub abono_id: i64,
    pub orden_id: i64,
    pub orden_numero: String,
    pub cliente_nombre: Option<String>,
    pub equipo_descripcion: String,
    pub monto: f64,
    pub forma_pago: String,
    pub fecha: String,
    pub usuario_nombre: Option<String>,
}

fn requiere_modulo(db: &Database) -> Result<(), String> {
    crate::commands::servicio_tecnico::requiere_modulo_servicio_tecnico(db)
}

// ─── Listar abonos de una orden ──────────────────────────────────────────

#[tauri::command]
pub fn st_listar_abonos(
    db: State<'_, Database>,
    orden_id: i64,
) -> Result<Vec<AbonoServicio>, String> {
    requiere_modulo(&db)?;
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let mut stmt = conn.prepare(
        "SELECT a.id, a.orden_id, a.monto, a.forma_pago, a.banco_id, b.nombre,
                a.referencia_pago, a.caja_id, a.estado, a.venta_id_aplicado,
                a.fecha, a.fecha_aplicado, a.fecha_devuelto,
                a.usuario_nombre, a.observacion
         FROM st_abonos a
         LEFT JOIN cuentas_banco b ON a.banco_id = b.id
         WHERE a.orden_id = ?1
         ORDER BY a.fecha DESC"
    ).map_err(|e| e.to_string())?;
    let rows: Vec<AbonoServicio> = stmt.query_map(params![orden_id], |r| Ok(AbonoServicio {
        id: Some(r.get(0)?),
        orden_id: r.get(1)?,
        monto: r.get(2)?,
        forma_pago: r.get(3)?,
        banco_id: r.get(4)?,
        banco_nombre: r.get(5)?,
        referencia_pago: r.get(6)?,
        caja_id: r.get(7)?,
        estado: r.get(8)?,
        venta_id_aplicado: r.get(9)?,
        fecha: r.get(10)?,
        fecha_aplicado: r.get(11)?,
        fecha_devuelto: r.get(12)?,
        usuario_nombre: r.get(13)?,
        observacion: r.get(14)?,
    })).map_err(|e| e.to_string())?
       .collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())?;
    Ok(rows)
}

// ─── Recibir abono ───────────────────────────────────────────────────────

#[tauri::command]
pub fn st_recibir_abono(
    db: State<'_, Database>,
    sesion: State<'_, SesionState>,
    orden_id: i64,
    monto: f64,
    forma_pago: String,
    banco_id: Option<i64>,
    referencia_pago: Option<String>,
    observacion: Option<String>,
) -> Result<i64, String> {
    requiere_modulo(&db)?;

    if monto <= 0.0 {
        return Err("El monto del abono debe ser mayor a 0".to_string());
    }

    let (usuario_nombre, usuario_id) = {
        let s = sesion.sesion.lock().map_err(|e| e.to_string())?;
        (
            s.as_ref().map(|s| s.nombre.clone()),
            s.as_ref().map(|s| s.usuario_id),
        )
    };

    let conn = db.conn.lock().map_err(|e| e.to_string())?;

    // Validar que la orden exista y no esté cobrada/cancelada
    let estado: String = conn.query_row(
        "SELECT estado FROM ordenes_servicio WHERE id = ?1",
        params![orden_id], |r| r.get(0),
    ).map_err(|_| "Orden no encontrada".to_string())?;
    if estado == "ENTREGADO" {
        return Err("La orden ya fue entregada y cobrada. No se pueden recibir más abonos.".to_string());
    }
    if estado == "CANCELADA" {
        return Err("La orden está cancelada. No se pueden recibir abonos.".to_string());
    }

    // Caja abierta (necesario para registrar holding)
    let caja_id: Option<i64> = conn.query_row(
        "SELECT id FROM caja WHERE estado = 'ABIERTA' LIMIT 1",
        [], |r| r.get(0),
    ).ok();
    if caja_id.is_none() {
        return Err("Debes abrir la caja antes de recibir abonos".to_string());
    }

    // Validar que el holding total no exceda el total de items / presupuesto.
    // Tope: si hay items, usa la suma; si no, cae al presupuesto registrado.
    let total_items = crate::commands::servicio_tecnico_items::calcular_total_orden(&conn, orden_id)
        .map(|t| t.total).unwrap_or(0.0);
    let presupuesto: f64 = conn.query_row(
        "SELECT COALESCE(presupuesto, 0) FROM ordenes_servicio WHERE id = ?1",
        params![orden_id], |r| r.get(0),
    ).unwrap_or(0.0);
    let tope = if total_items > 0.0 { total_items } else { presupuesto };
    if tope > 0.0 {
        let holding_actual: f64 = conn.query_row(
            "SELECT COALESCE(SUM(monto), 0) FROM st_abonos WHERE orden_id = ?1 AND estado = 'HOLDING'",
            params![orden_id], |r| r.get(0),
        ).unwrap_or(0.0);
        if holding_actual + monto > tope + 0.001 {
            let restante = (tope - holding_actual).max(0.0);
            return Err(format!(
                "El abono excede el total de la orden (${:.2}). Ya hay ${:.2} en holding. Maximo a recibir: ${:.2}",
                tope, holding_actual, restante
            ));
        }
    }

    conn.execute(
        "INSERT INTO st_abonos (orden_id, monto, forma_pago, banco_id, referencia_pago,
                                 caja_id, estado, usuario_id, usuario_nombre, observacion)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, 'HOLDING', ?7, ?8, ?9)",
        params![
            orden_id, monto, forma_pago, banco_id, referencia_pago,
            caja_id, usuario_id, usuario_nombre, observacion,
        ],
    ).map_err(|e| e.to_string())?;

    Ok(conn.last_insert_rowid())
}

// ─── Total de abonos en HOLDING para una orden ───────────────────────────

#[tauri::command]
pub fn st_total_abonos_orden(
    db: State<'_, Database>,
    orden_id: i64,
) -> Result<f64, String> {
    requiere_modulo(&db)?;
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let total: f64 = conn.query_row(
        "SELECT COALESCE(SUM(monto), 0) FROM st_abonos WHERE orden_id = ?1 AND estado = 'HOLDING'",
        params![orden_id], |r| r.get(0),
    ).unwrap_or(0.0);
    Ok(total)
}

// ─── Cancelar orden + devolver abonos ────────────────────────────────────

#[tauri::command]
pub fn st_cancelar_orden(
    db: State<'_, Database>,
    sesion: State<'_, SesionState>,
    orden_id: i64,
    observacion: Option<String>,
) -> Result<serde_json::Value, String> {
    requiere_modulo(&db)?;

    let usuario_nombre = {
        let s = sesion.sesion.lock().map_err(|e| e.to_string())?;
        s.as_ref().map(|s| s.nombre.clone()).unwrap_or_else(|| "Sistema".to_string())
    };

    let mut conn = db.conn.lock().map_err(|e| e.to_string())?;

    // Validar estado actual
    let estado_actual: String = conn.query_row(
        "SELECT estado FROM ordenes_servicio WHERE id = ?1",
        params![orden_id], |r| r.get(0),
    ).map_err(|_| "Orden no encontrada".to_string())?;
    if estado_actual == "ENTREGADO" {
        return Err("La orden ya fue entregada/cobrada. No se puede cancelar (anular la venta correspondiente desde Ventas).".to_string());
    }
    if estado_actual == "CANCELADA" {
        return Err("La orden ya estaba cancelada".to_string());
    }

    // Transacción: cancelar + devolver abonos + log de movimiento
    let tx = conn.transaction().map_err(|e| e.to_string())?;

    // 1. Marcar orden CANCELADA
    tx.execute(
        "UPDATE ordenes_servicio SET estado = 'CANCELADA' WHERE id = ?1",
        params![orden_id],
    ).map_err(|e| e.to_string())?;

    // 2. Devolver abonos en HOLDING → DEVUELTO
    let abonos_devueltos = tx.execute(
        "UPDATE st_abonos
         SET estado = 'DEVUELTO', fecha_devuelto = datetime('now', 'localtime')
         WHERE orden_id = ?1 AND estado = 'HOLDING'",
        params![orden_id],
    ).map_err(|e| e.to_string())?;

    let monto_devuelto: f64 = tx.query_row(
        "SELECT COALESCE(SUM(monto), 0) FROM st_abonos
         WHERE orden_id = ?1 AND estado = 'DEVUELTO'
           AND date(fecha_devuelto) = date('now', 'localtime')",
        params![orden_id], |r| r.get(0),
    ).unwrap_or(0.0);

    // 3. Movimiento en historial
    let obs_completa = match observacion {
        Some(o) if !o.trim().is_empty() => format!("Cancelada: {}{}",
            o,
            if abonos_devueltos > 0 { format!(" · {} abono(s) devuelto(s) (${:.2})", abonos_devueltos, monto_devuelto) } else { String::new() }
        ),
        _ => format!("Orden cancelada{}",
            if abonos_devueltos > 0 { format!(" · {} abono(s) devuelto(s) (${:.2})", abonos_devueltos, monto_devuelto) } else { String::new() }
        ),
    };
    tx.execute(
        "INSERT INTO ordenes_servicio_movimientos (orden_id, estado_anterior, estado_nuevo, observacion, usuario)
         VALUES (?1, ?2, 'CANCELADA', ?3, ?4)",
        params![orden_id, estado_actual, obs_completa, usuario_nombre],
    ).ok();

    tx.commit().map_err(|e| e.to_string())?;

    Ok(serde_json::json!({
        "ok": true,
        "abonos_devueltos": abonos_devueltos,
        "monto_devuelto": monto_devuelto,
    }))
}

// ─── Listar holdings (en caja activa o todos abiertos) ───────────────────

#[tauri::command]
pub fn st_listar_holdings_caja(
    db: State<'_, Database>,
    caja_id: Option<i64>,
) -> Result<Vec<HoldingCaja>, String> {
    requiere_modulo(&db)?;
    let conn = db.conn.lock().map_err(|e| e.to_string())?;

    // Si no se especifica caja_id, usar la caja abierta actual
    let caja_efectiva: Option<i64> = match caja_id {
        Some(id) => Some(id),
        None => conn.query_row(
            "SELECT id FROM caja WHERE estado = 'ABIERTA' LIMIT 1",
            [], |r| r.get(0),
        ).ok(),
    };

    let sql = if caja_efectiva.is_some() {
        "SELECT a.id, a.orden_id, o.numero, o.cliente_nombre, o.equipo_descripcion,
                a.monto, a.forma_pago, a.fecha, a.usuario_nombre
         FROM st_abonos a
         JOIN ordenes_servicio o ON a.orden_id = o.id
         WHERE a.estado = 'HOLDING' AND a.caja_id = ?1
         ORDER BY a.fecha DESC"
    } else {
        "SELECT a.id, a.orden_id, o.numero, o.cliente_nombre, o.equipo_descripcion,
                a.monto, a.forma_pago, a.fecha, a.usuario_nombre
         FROM st_abonos a
         JOIN ordenes_servicio o ON a.orden_id = o.id
         WHERE a.estado = 'HOLDING'
         ORDER BY a.fecha DESC"
    };

    let mut stmt = conn.prepare(sql).map_err(|e| e.to_string())?;
    let mapper = |r: &rusqlite::Row| -> rusqlite::Result<HoldingCaja> {
        Ok(HoldingCaja {
            abono_id: r.get(0)?,
            orden_id: r.get(1)?,
            orden_numero: r.get(2)?,
            cliente_nombre: r.get(3)?,
            equipo_descripcion: r.get(4)?,
            monto: r.get(5)?,
            forma_pago: r.get(6)?,
            fecha: r.get(7)?,
            usuario_nombre: r.get(8)?,
        })
    };
    let rows: Vec<HoldingCaja> = if let Some(cid) = caja_efectiva {
        stmt.query_map(params![cid], mapper)
            .map_err(|e| e.to_string())?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())?
    } else {
        stmt.query_map([], mapper)
            .map_err(|e| e.to_string())?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())?
    };
    Ok(rows)
}
