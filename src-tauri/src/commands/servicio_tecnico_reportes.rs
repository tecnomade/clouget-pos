//! v2.4.14 — Reportes específicos del módulo de Servicio Técnico.
//!
//! Por ahora: reporte de cancelaciones (qué órdenes se cancelaron, quién, por qué,
//! cuánto se devolvió en abonos). En el futuro puede crecer con: garantías
//! activas, productividad por técnico, tiempos promedio, etc.

use crate::db::Database;
use rusqlite::params;
use serde::{Deserialize, Serialize};
use tauri::State;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct OrdenCancelada {
    pub orden_id: i64,
    pub numero: String,
    pub fecha_ingreso: String,
    pub fecha_cancelacion: Option<String>, // viene de movimientos
    pub cliente_nombre: Option<String>,
    pub cliente_telefono: Option<String>,
    pub equipo_descripcion: String,
    pub equipo_marca: Option<String>,
    pub equipo_modelo: Option<String>,
    pub usuario_cancelacion: Option<String>,
    pub observacion: Option<String>,
    pub abonos_devueltos: i64,
    pub monto_devuelto: f64,
}

#[derive(Debug, Serialize, Clone)]
pub struct ResumenCancelaciones {
    pub total_canceladas: i64,
    pub total_abonos_devueltos: i64,
    pub monto_total_devuelto: f64,
    pub ordenes: Vec<OrdenCancelada>,
}

fn requiere_modulo(db: &Database) -> Result<(), String> {
    crate::commands::servicio_tecnico::requiere_modulo_servicio_tecnico(db)
}

/// Lista órdenes canceladas en un rango. Si no se pasan fechas, devuelve los
/// últimos 30 días.
#[tauri::command]
pub fn st_reporte_cancelaciones(
    db: State<'_, Database>,
    fecha_desde: Option<String>,
    fecha_hasta: Option<String>,
) -> Result<ResumenCancelaciones, String> {
    requiere_modulo(&db)?;
    let conn = db.conn.lock().map_err(|e| e.to_string())?;

    // Default: últimos 30 días
    let desde = fecha_desde.unwrap_or_else(|| "date('now', 'localtime', '-30 days')".to_string());
    let hasta = fecha_hasta.unwrap_or_else(|| "date('now', 'localtime')".to_string());

    // Si las fechas vinieron como literal (YYYY-MM-DD), entrecomillamos.
    // Si vinieron como expresión SQL (date(...)), las dejamos crudas.
    // Para simplicidad: detectamos si arrancan con 'date(' o no.
    let desde_sql = if desde.starts_with("date(") { desde } else { format!("'{}'", desde) };
    let hasta_sql = if hasta.starts_with("date(") { hasta } else { format!("'{}'", hasta) };

    let sql = format!(
        "SELECT o.id, o.numero, o.fecha_ingreso,
                o.cliente_nombre, o.cliente_telefono,
                o.equipo_descripcion, COALESCE(o.equipo_marca, ''), COALESCE(o.equipo_modelo, ''),
                m.fecha, m.usuario, m.observacion,
                COALESCE((SELECT COUNT(*) FROM st_abonos a WHERE a.orden_id = o.id AND a.estado = 'DEVUELTO'), 0),
                COALESCE((SELECT SUM(a.monto) FROM st_abonos a WHERE a.orden_id = o.id AND a.estado = 'DEVUELTO'), 0)
         FROM ordenes_servicio o
         LEFT JOIN ordenes_servicio_movimientos m
             ON m.orden_id = o.id AND m.estado_nuevo = 'CANCELADA'
         WHERE o.estado = 'CANCELADA'
           AND date(COALESCE(m.fecha, o.fecha_ingreso)) BETWEEN date({}) AND date({})
         GROUP BY o.id
         ORDER BY COALESCE(m.fecha, o.fecha_ingreso) DESC",
        desde_sql, hasta_sql
    );

    let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;
    let rows: Vec<OrdenCancelada> = stmt.query_map([], |r| Ok(OrdenCancelada {
        orden_id: r.get(0)?,
        numero: r.get(1)?,
        fecha_ingreso: r.get(2)?,
        cliente_nombre: r.get(3)?,
        cliente_telefono: r.get(4)?,
        equipo_descripcion: r.get(5)?,
        equipo_marca: {
            let s: String = r.get(6)?;
            if s.is_empty() { None } else { Some(s) }
        },
        equipo_modelo: {
            let s: String = r.get(7)?;
            if s.is_empty() { None } else { Some(s) }
        },
        fecha_cancelacion: r.get(8)?,
        usuario_cancelacion: r.get(9)?,
        observacion: r.get(10)?,
        abonos_devueltos: r.get(11)?,
        monto_devuelto: r.get(12)?,
    })).map_err(|e| e.to_string())?
       .collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())?;

    let total_canceladas = rows.len() as i64;
    let total_abonos_devueltos = rows.iter().map(|o| o.abonos_devueltos).sum();
    let monto_total_devuelto = rows.iter().map(|o| o.monto_devuelto).sum();

    Ok(ResumenCancelaciones {
        total_canceladas,
        total_abonos_devueltos,
        monto_total_devuelto,
        ordenes: rows,
    })
}
