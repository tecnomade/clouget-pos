//! v2.4.13 — ST-5: Items presupuestados/aplicados a la orden de servicio.
//!
//! Permite armar el detalle de la orden ANTES de cobrar. Los abonos HOLDING
//! validan contra este total. Al cobrar, estos items se copian a la venta.
//!
//! Comandos:
//! - `st_listar_items_orden(orden_id)`
//! - `st_agregar_item_orden(...)` — producto del catalogo o servicio manual
//! - `st_actualizar_item_orden(...)`
//! - `st_eliminar_item_orden(item_id)`
//! - `st_total_orden(orden_id)` — devuelve subtotal_sin_iva, subtotal_con_iva, iva, total

use crate::db::Database;
use rusqlite::params;
use serde::{Deserialize, Serialize};
use tauri::State;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ItemOrden {
    pub id: Option<i64>,
    pub orden_id: i64,
    pub producto_id: Option<i64>,
    pub descripcion: String,
    pub cantidad: f64,
    pub precio_unitario: f64,
    pub iva_porcentaje: f64,
    pub subtotal: f64,
    pub es_servicio: i64,
}

#[derive(Debug, Serialize, Clone)]
pub struct TotalOrden {
    pub subtotal_sin_iva: f64,
    pub subtotal_con_iva: f64,
    pub iva: f64,
    pub total: f64,
    pub cantidad_items: i64,
}

fn requiere_modulo(db: &Database) -> Result<(), String> {
    crate::commands::servicio_tecnico::requiere_modulo_servicio_tecnico(db)
}

// ─── Listar items ────────────────────────────────────────────────────────

#[tauri::command]
pub fn st_listar_items_orden(
    db: State<'_, Database>,
    orden_id: i64,
) -> Result<Vec<ItemOrden>, String> {
    requiere_modulo(&db)?;
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let mut stmt = conn.prepare(
        "SELECT id, orden_id, producto_id, descripcion, cantidad, precio_unitario,
                iva_porcentaje, subtotal, es_servicio
         FROM orden_servicio_items
         WHERE orden_id = ?1
         ORDER BY id ASC"
    ).map_err(|e| e.to_string())?;
    let rows: Vec<ItemOrden> = stmt.query_map(params![orden_id], |r| Ok(ItemOrden {
        id: Some(r.get(0)?),
        orden_id: r.get(1)?,
        producto_id: r.get(2)?,
        descripcion: r.get(3)?,
        cantidad: r.get(4)?,
        precio_unitario: r.get(5)?,
        iva_porcentaje: r.get(6)?,
        subtotal: r.get(7)?,
        es_servicio: r.get(8)?,
    })).map_err(|e| e.to_string())?
       .collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())?;
    Ok(rows)
}

// ─── Agregar item ────────────────────────────────────────────────────────

#[tauri::command]
pub fn st_agregar_item_orden(
    db: State<'_, Database>,
    orden_id: i64,
    producto_id: Option<i64>,
    descripcion: String,
    cantidad: f64,
    precio_unitario: f64,
    iva_porcentaje: Option<f64>,
    es_servicio: Option<bool>,
) -> Result<i64, String> {
    requiere_modulo(&db)?;

    if cantidad <= 0.0 {
        return Err("La cantidad debe ser mayor a 0".to_string());
    }
    if precio_unitario < 0.0 {
        return Err("El precio no puede ser negativo".to_string());
    }
    if descripcion.trim().is_empty() {
        return Err("La descripcion es obligatoria".to_string());
    }

    let conn = db.conn.lock().map_err(|e| e.to_string())?;

    // Validar que la orden no este ENTREGADA o CANCELADA
    let estado: String = conn.query_row(
        "SELECT estado FROM ordenes_servicio WHERE id = ?1",
        params![orden_id], |r| r.get(0),
    ).map_err(|_| "Orden no encontrada".to_string())?;
    if estado == "ENTREGADO" {
        return Err("La orden ya fue entregada/cobrada. No se pueden modificar los items.".to_string());
    }
    if estado == "CANCELADA" {
        return Err("La orden esta cancelada. No se pueden modificar los items.".to_string());
    }

    // Si vino producto_id, traer iva del producto si no se paso explicito
    let iva_efectivo = match (iva_porcentaje, producto_id) {
        (Some(v), _) => v,
        (None, Some(pid)) => conn.query_row(
            "SELECT iva_porcentaje FROM productos WHERE id = ?1",
            params![pid], |r| r.get(0),
        ).unwrap_or(0.0),
        (None, None) => 0.0,
    };

    let es_servicio_flag = es_servicio.unwrap_or(producto_id.is_none()) as i64;
    let subtotal = cantidad * precio_unitario;

    conn.execute(
        "INSERT INTO orden_servicio_items
         (orden_id, producto_id, descripcion, cantidad, precio_unitario, iva_porcentaje, subtotal, es_servicio)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        params![
            orden_id, producto_id, descripcion.trim(), cantidad, precio_unitario,
            iva_efectivo, subtotal, es_servicio_flag,
        ],
    ).map_err(|e| e.to_string())?;

    Ok(conn.last_insert_rowid())
}

// ─── Actualizar item ─────────────────────────────────────────────────────

#[tauri::command]
pub fn st_actualizar_item_orden(
    db: State<'_, Database>,
    item_id: i64,
    descripcion: String,
    cantidad: f64,
    precio_unitario: f64,
    iva_porcentaje: f64,
) -> Result<(), String> {
    requiere_modulo(&db)?;

    if cantidad <= 0.0 {
        return Err("La cantidad debe ser mayor a 0".to_string());
    }
    if precio_unitario < 0.0 {
        return Err("El precio no puede ser negativo".to_string());
    }
    if descripcion.trim().is_empty() {
        return Err("La descripcion es obligatoria".to_string());
    }

    let conn = db.conn.lock().map_err(|e| e.to_string())?;

    // Validar estado de la orden propietaria
    let (orden_id, estado): (i64, String) = conn.query_row(
        "SELECT i.orden_id, o.estado
         FROM orden_servicio_items i
         JOIN ordenes_servicio o ON i.orden_id = o.id
         WHERE i.id = ?1",
        params![item_id], |r| Ok((r.get(0)?, r.get(1)?)),
    ).map_err(|_| "Item no encontrado".to_string())?;
    if estado == "ENTREGADO" || estado == "CANCELADA" {
        return Err("La orden ya esta cerrada. No se pueden modificar los items.".to_string());
    }

    let subtotal = cantidad * precio_unitario;
    conn.execute(
        "UPDATE orden_servicio_items
         SET descripcion = ?1, cantidad = ?2, precio_unitario = ?3,
             iva_porcentaje = ?4, subtotal = ?5
         WHERE id = ?6",
        params![descripcion.trim(), cantidad, precio_unitario, iva_porcentaje, subtotal, item_id],
    ).map_err(|e| e.to_string())?;

    let _ = orden_id; // silenciar unused
    Ok(())
}

// ─── Eliminar item ───────────────────────────────────────────────────────

#[tauri::command]
pub fn st_eliminar_item_orden(
    db: State<'_, Database>,
    item_id: i64,
) -> Result<(), String> {
    requiere_modulo(&db)?;
    let conn = db.conn.lock().map_err(|e| e.to_string())?;

    let estado: String = conn.query_row(
        "SELECT o.estado FROM orden_servicio_items i
         JOIN ordenes_servicio o ON i.orden_id = o.id
         WHERE i.id = ?1",
        params![item_id], |r| r.get(0),
    ).map_err(|_| "Item no encontrado".to_string())?;
    if estado == "ENTREGADO" || estado == "CANCELADA" {
        return Err("La orden ya esta cerrada. No se pueden eliminar items.".to_string());
    }

    conn.execute("DELETE FROM orden_servicio_items WHERE id = ?1", params![item_id])
        .map_err(|e| e.to_string())?;
    Ok(())
}

// ─── Total de la orden (calculado desde items) ───────────────────────────

#[tauri::command]
pub fn st_total_orden(
    db: State<'_, Database>,
    orden_id: i64,
) -> Result<TotalOrden, String> {
    requiere_modulo(&db)?;
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    calcular_total_orden(&conn, orden_id)
}

/// Helper compartido para calcular total desde items (uso interno).
pub fn calcular_total_orden(
    conn: &rusqlite::Connection,
    orden_id: i64,
) -> Result<TotalOrden, String> {
    let mut stmt = conn.prepare(
        "SELECT cantidad, precio_unitario, iva_porcentaje
         FROM orden_servicio_items
         WHERE orden_id = ?1"
    ).map_err(|e| e.to_string())?;
    let rows = stmt.query_map(params![orden_id], |r| {
        Ok((r.get::<_, f64>(0)?, r.get::<_, f64>(1)?, r.get::<_, f64>(2)?))
    }).map_err(|e| e.to_string())?;

    let mut subtotal_sin_iva = 0.0;
    let mut subtotal_con_iva = 0.0;
    let mut iva = 0.0;
    let mut count: i64 = 0;
    for row in rows {
        let (cant, precio, iva_pct) = row.map_err(|e| e.to_string())?;
        let sub = cant * precio;
        if iva_pct > 0.0 {
            subtotal_con_iva += sub;
            iva += sub * (iva_pct / 100.0);
        } else {
            subtotal_sin_iva += sub;
        }
        count += 1;
    }

    Ok(TotalOrden {
        subtotal_sin_iva,
        subtotal_con_iva,
        iva,
        total: subtotal_sin_iva + subtotal_con_iva + iva,
        cantidad_items: count,
    })
}
