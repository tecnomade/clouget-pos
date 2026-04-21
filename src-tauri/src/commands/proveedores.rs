use crate::db::Database;
use crate::models::Proveedor;
use tauri::State;

#[tauri::command]
pub fn crear_proveedor(db: State<Database>, proveedor: Proveedor) -> Result<Proveedor, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;

    conn.execute(
        "INSERT INTO proveedores (ruc, nombre, direccion, telefono, email, contacto, dias_credito, activo)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        rusqlite::params![
            proveedor.ruc,
            proveedor.nombre,
            proveedor.direccion,
            proveedor.telefono,
            proveedor.email,
            proveedor.contacto,
            proveedor.dias_credito,
            proveedor.activo,
        ],
    )
    .map_err(|e| e.to_string())?;

    let id = conn.last_insert_rowid();

    Ok(Proveedor {
        id: Some(id),
        ..proveedor
    })
}

#[tauri::command]
pub fn actualizar_proveedor(
    db: State<Database>,
    id: i64,
    proveedor: Proveedor,
) -> Result<(), String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;

    conn.execute(
        "UPDATE proveedores SET ruc = ?1, nombre = ?2, direccion = ?3, telefono = ?4,
         email = ?5, contacto = ?6, dias_credito = ?7, activo = ?8
         WHERE id = ?9",
        rusqlite::params![
            proveedor.ruc,
            proveedor.nombre,
            proveedor.direccion,
            proveedor.telefono,
            proveedor.email,
            proveedor.contacto,
            proveedor.dias_credito,
            proveedor.activo,
            id,
        ],
    )
    .map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
pub fn listar_proveedores(db: State<Database>) -> Result<Vec<Proveedor>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;

    let mut stmt = conn
        .prepare(
            "SELECT id, ruc, nombre, direccion, telefono, email, contacto, dias_credito, activo
             FROM proveedores WHERE activo = 1 ORDER BY nombre",
        )
        .map_err(|e| e.to_string())?;

    let resultado = stmt
        .query_map([], |row| {
            Ok(Proveedor {
                id: Some(row.get(0)?),
                ruc: row.get(1)?,
                nombre: row.get(2)?,
                direccion: row.get(3)?,
                telefono: row.get(4)?,
                email: row.get(5)?,
                contacto: row.get(6)?,
                dias_credito: row.get(7)?,
                activo: row.get(8)?,
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    Ok(resultado)
}

#[tauri::command]
pub fn buscar_proveedores(db: State<Database>, termino: String) -> Result<Vec<Proveedor>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;

    let busqueda = format!("%{}%", termino);
    let mut stmt = conn
        .prepare(
            "SELECT id, ruc, nombre, direccion, telefono, email, contacto, dias_credito, activo
             FROM proveedores
             WHERE activo = 1 AND (nombre LIKE ?1 OR ruc LIKE ?1 OR contacto LIKE ?1)
             ORDER BY nombre
             LIMIT 50",
        )
        .map_err(|e| e.to_string())?;

    let resultado = stmt
        .query_map(rusqlite::params![busqueda], |row| {
            Ok(Proveedor {
                id: Some(row.get(0)?),
                ruc: row.get(1)?,
                nombre: row.get(2)?,
                direccion: row.get(3)?,
                telefono: row.get(4)?,
                email: row.get(5)?,
                contacto: row.get(6)?,
                dias_credito: row.get(7)?,
                activo: row.get(8)?,
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    Ok(resultado)
}

#[tauri::command]
pub fn eliminar_proveedor(db: State<Database>, id: i64) -> Result<(), String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;

    // Soft delete: marcar como inactivo
    conn.execute(
        "UPDATE proveedores SET activo = 0 WHERE id = ?1",
        rusqlite::params![id],
    )
    .map_err(|e| e.to_string())?;

    Ok(())
}
