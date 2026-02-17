use crate::db::Database;
use crate::models::Cliente;
use tauri::State;

#[tauri::command]
pub fn crear_cliente(db: State<Database>, cliente: Cliente) -> Result<i64, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;

    conn.execute(
        "INSERT INTO clientes (tipo_identificacion, identificacion, nombre, direccion, telefono, email, activo, lista_precio_id)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        rusqlite::params![
            cliente.tipo_identificacion,
            cliente.identificacion,
            cliente.nombre,
            cliente.direccion,
            cliente.telefono,
            cliente.email,
            cliente.activo as i32,
            cliente.lista_precio_id,
        ],
    )
    .map_err(|e| e.to_string())?;

    Ok(conn.last_insert_rowid())
}

#[tauri::command]
pub fn actualizar_cliente(db: State<Database>, cliente: Cliente) -> Result<(), String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let id = cliente.id.ok_or("ID requerido para actualizar")?;

    conn.execute(
        "UPDATE clientes SET tipo_identificacion=?1, identificacion=?2, nombre=?3,
         direccion=?4, telefono=?5, email=?6, activo=?7, lista_precio_id=?8,
         updated_at=datetime('now','localtime')
         WHERE id=?9",
        rusqlite::params![
            cliente.tipo_identificacion,
            cliente.identificacion,
            cliente.nombre,
            cliente.direccion,
            cliente.telefono,
            cliente.email,
            cliente.activo as i32,
            cliente.lista_precio_id,
            id,
        ],
    )
    .map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
pub fn buscar_clientes(db: State<Database>, termino: String) -> Result<Vec<Cliente>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let busqueda = format!("%{}%", termino);

    let mut stmt = conn
        .prepare(
            "SELECT c.id, c.tipo_identificacion, c.identificacion, c.nombre, c.direccion,
                    c.telefono, c.email, c.activo, c.lista_precio_id, lp.nombre
             FROM clientes c
             LEFT JOIN listas_precios lp ON lp.id = c.lista_precio_id
             WHERE c.activo = 1
             AND (c.nombre LIKE ?1 OR c.identificacion LIKE ?1)
             ORDER BY c.nombre LIMIT 30",
        )
        .map_err(|e| e.to_string())?;

    let clientes = stmt
        .query_map(rusqlite::params![busqueda], |row| {
            Ok(Cliente {
                id: Some(row.get(0)?),
                tipo_identificacion: row.get(1)?,
                identificacion: row.get(2)?,
                nombre: row.get(3)?,
                direccion: row.get(4)?,
                telefono: row.get(5)?,
                email: row.get(6)?,
                activo: row.get::<_, i32>(7)? != 0,
                lista_precio_id: row.get(8)?,
                lista_precio_nombre: row.get(9)?,
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    Ok(clientes)
}

#[tauri::command]
pub fn listar_clientes(db: State<Database>) -> Result<Vec<Cliente>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;

    let mut stmt = conn
        .prepare(
            "SELECT c.id, c.tipo_identificacion, c.identificacion, c.nombre, c.direccion,
                    c.telefono, c.email, c.activo, c.lista_precio_id, lp.nombre
             FROM clientes c
             LEFT JOIN listas_precios lp ON lp.id = c.lista_precio_id
             WHERE c.activo = 1 ORDER BY c.nombre",
        )
        .map_err(|e| e.to_string())?;

    let clientes = stmt
        .query_map([], |row| {
            Ok(Cliente {
                id: Some(row.get(0)?),
                tipo_identificacion: row.get(1)?,
                identificacion: row.get(2)?,
                nombre: row.get(3)?,
                direccion: row.get(4)?,
                telefono: row.get(5)?,
                email: row.get(6)?,
                activo: row.get::<_, i32>(7)? != 0,
                lista_precio_id: row.get(8)?,
                lista_precio_nombre: row.get(9)?,
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    Ok(clientes)
}
