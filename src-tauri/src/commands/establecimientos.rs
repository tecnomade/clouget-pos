use crate::db::Database;
use crate::models::{Establecimiento, PuntoEmision};
use tauri::State;

#[tauri::command]
pub fn listar_establecimientos(db: State<Database>) -> Result<Vec<Establecimiento>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;

    let mut stmt = conn
        .prepare("SELECT id, codigo, nombre, direccion, telefono, es_propio, activo FROM establecimientos ORDER BY codigo")
        .map_err(|e| e.to_string())?;

    let items = stmt
        .query_map([], |row| {
            Ok(Establecimiento {
                id: Some(row.get(0)?),
                codigo: row.get(1)?,
                nombre: row.get(2)?,
                direccion: row.get(3)?,
                telefono: row.get(4)?,
                es_propio: row.get(5)?,
                activo: row.get(6)?,
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    Ok(items)
}

#[tauri::command]
pub fn crear_establecimiento(
    db: State<Database>,
    establecimiento: Establecimiento,
) -> Result<Establecimiento, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;

    // Validar código de 3 dígitos
    if establecimiento.codigo.len() != 3 || !establecimiento.codigo.chars().all(|c| c.is_ascii_digit()) {
        return Err("El código de establecimiento debe ser de 3 dígitos (ej: 001, 002)".to_string());
    }

    conn.execute(
        "INSERT INTO establecimientos (codigo, nombre, direccion, telefono, es_propio, activo) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        rusqlite::params![
            establecimiento.codigo,
            establecimiento.nombre,
            establecimiento.direccion,
            establecimiento.telefono,
            establecimiento.es_propio,
            establecimiento.activo,
        ],
    )
    .map_err(|e| {
        if e.to_string().contains("UNIQUE") {
            "Ya existe un establecimiento con ese código".to_string()
        } else {
            e.to_string()
        }
    })?;

    let id = conn.last_insert_rowid();

    Ok(Establecimiento {
        id: Some(id),
        ..establecimiento
    })
}

#[tauri::command]
pub fn actualizar_establecimiento(
    db: State<Database>,
    establecimiento: Establecimiento,
) -> Result<(), String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;

    conn.execute(
        "UPDATE establecimientos SET nombre = ?1, direccion = ?2, telefono = ?3, es_propio = ?4, activo = ?5 WHERE id = ?6",
        rusqlite::params![
            establecimiento.nombre,
            establecimiento.direccion,
            establecimiento.telefono,
            establecimiento.es_propio,
            establecimiento.activo,
            establecimiento.id,
        ],
    )
    .map_err(|e| e.to_string())?;

    Ok(())
}

// --- Puntos de Emisión ---

#[tauri::command]
pub fn listar_puntos_emision(
    db: State<Database>,
    establecimiento_id: i64,
) -> Result<Vec<PuntoEmision>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;

    let mut stmt = conn
        .prepare("SELECT id, establecimiento_id, codigo, nombre, activo FROM puntos_emision WHERE establecimiento_id = ?1 ORDER BY codigo")
        .map_err(|e| e.to_string())?;

    let items = stmt
        .query_map(rusqlite::params![establecimiento_id], |row| {
            Ok(PuntoEmision {
                id: Some(row.get(0)?),
                establecimiento_id: row.get(1)?,
                codigo: row.get(2)?,
                nombre: row.get(3)?,
                activo: row.get(4)?,
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    Ok(items)
}

#[tauri::command]
pub fn crear_punto_emision(
    db: State<Database>,
    punto: PuntoEmision,
) -> Result<PuntoEmision, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;

    // Validar código de 3 dígitos
    if punto.codigo.len() != 3 || !punto.codigo.chars().all(|c| c.is_ascii_digit()) {
        return Err("El código de punto de emisión debe ser de 3 dígitos (ej: 001, 002)".to_string());
    }

    conn.execute(
        "INSERT INTO puntos_emision (establecimiento_id, codigo, nombre, activo) VALUES (?1, ?2, ?3, ?4)",
        rusqlite::params![punto.establecimiento_id, punto.codigo, punto.nombre, punto.activo],
    )
    .map_err(|e| {
        if e.to_string().contains("UNIQUE") {
            "Ya existe un punto de emisión con ese código en este establecimiento".to_string()
        } else {
            e.to_string()
        }
    })?;

    let id = conn.last_insert_rowid();

    // Crear secuenciales para el nuevo punto de emisión
    let est_codigo: String = conn
        .query_row(
            "SELECT codigo FROM establecimientos WHERE id = ?1",
            rusqlite::params![punto.establecimiento_id],
            |row| row.get(0),
        )
        .map_err(|e| e.to_string())?;

    let tipos = ["NOTA_VENTA", "FACTURA", "FACTURA_PRUEBAS", "NOTA_CREDITO", "NOTA_CREDITO_PRUEBAS"];
    for tipo in &tipos {
        conn.execute(
            "INSERT OR IGNORE INTO secuenciales (establecimiento_codigo, punto_emision_codigo, tipo_documento, secuencial) VALUES (?1, ?2, ?3, 1)",
            rusqlite::params![est_codigo, punto.codigo, tipo],
        ).ok();
    }

    Ok(PuntoEmision {
        id: Some(id),
        ..punto
    })
}

#[tauri::command]
pub fn actualizar_punto_emision(
    db: State<Database>,
    punto: PuntoEmision,
) -> Result<(), String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;

    conn.execute(
        "UPDATE puntos_emision SET nombre = ?1, activo = ?2 WHERE id = ?3",
        rusqlite::params![punto.nombre, punto.activo, punto.id],
    )
    .map_err(|e| e.to_string())?;

    Ok(())
}
