use crate::db::Database;
use crate::models::{ListaPrecio, PrecioProducto, PrecioProductoDetalle};
use tauri::State;

#[tauri::command]
pub fn listar_listas_precios(db: State<Database>) -> Result<Vec<ListaPrecio>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;

    let mut stmt = conn
        .prepare(
            "SELECT id, nombre, descripcion, es_default, activo
             FROM listas_precios WHERE activo = 1 ORDER BY es_default DESC, nombre",
        )
        .map_err(|e| e.to_string())?;

    let listas = stmt
        .query_map([], |row| {
            Ok(ListaPrecio {
                id: Some(row.get(0)?),
                nombre: row.get(1)?,
                descripcion: row.get(2)?,
                es_default: row.get::<_, i32>(3)? != 0,
                activo: row.get::<_, i32>(4)? != 0,
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    Ok(listas)
}

#[tauri::command]
pub fn crear_lista_precio(db: State<Database>, lista: ListaPrecio) -> Result<i64, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;

    conn.execute(
        "INSERT INTO listas_precios (nombre, descripcion, es_default, activo) VALUES (?1, ?2, ?3, ?4)",
        rusqlite::params![
            lista.nombre,
            lista.descripcion,
            lista.es_default as i32,
            lista.activo as i32,
        ],
    )
    .map_err(|e| e.to_string())?;

    Ok(conn.last_insert_rowid())
}

#[tauri::command]
pub fn actualizar_lista_precio(db: State<Database>, lista: ListaPrecio) -> Result<(), String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let id = lista.id.ok_or("ID requerido para actualizar")?;

    conn.execute(
        "UPDATE listas_precios SET nombre=?1, descripcion=?2, activo=?3,
         updated_at=datetime('now','localtime') WHERE id=?4",
        rusqlite::params![lista.nombre, lista.descripcion, lista.activo as i32, id],
    )
    .map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
pub fn establecer_lista_default(db: State<Database>, id: i64) -> Result<(), String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;

    // Quitar default de todas
    conn.execute("UPDATE listas_precios SET es_default = 0", [])
        .map_err(|e| e.to_string())?;

    // Establecer la nueva default
    conn.execute(
        "UPDATE listas_precios SET es_default = 1 WHERE id = ?1",
        rusqlite::params![id],
    )
    .map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
pub fn guardar_precios_producto(
    db: State<Database>,
    producto_id: i64,
    precios: Vec<PrecioProducto>,
) -> Result<(), String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;

    // Eliminar precios anteriores del producto
    conn.execute(
        "DELETE FROM precios_producto WHERE producto_id = ?1",
        rusqlite::params![producto_id],
    )
    .map_err(|e| e.to_string())?;

    // Insertar nuevos precios
    for precio in &precios {
        conn.execute(
            "INSERT INTO precios_producto (lista_precio_id, producto_id, precio) VALUES (?1, ?2, ?3)",
            rusqlite::params![precio.lista_precio_id, producto_id, precio.precio],
        )
        .map_err(|e| e.to_string())?;
    }

    Ok(())
}

#[tauri::command]
pub fn obtener_precios_producto(
    db: State<Database>,
    producto_id: i64,
) -> Result<Vec<PrecioProductoDetalle>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;

    let mut stmt = conn
        .prepare(
            "SELECT pp.lista_precio_id, lp.nombre, pp.precio
             FROM precios_producto pp
             JOIN listas_precios lp ON lp.id = pp.lista_precio_id
             WHERE pp.producto_id = ?1 AND lp.activo = 1
             ORDER BY lp.es_default DESC, lp.nombre",
        )
        .map_err(|e| e.to_string())?;

    let precios = stmt
        .query_map(rusqlite::params![producto_id], |row| {
            Ok(PrecioProductoDetalle {
                lista_precio_id: row.get(0)?,
                lista_nombre: row.get(1)?,
                precio: row.get(2)?,
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    Ok(precios)
}

#[tauri::command]
pub fn resolver_precio_producto(
    db: State<Database>,
    producto_id: i64,
    cliente_id: Option<i64>,
) -> Result<f64, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;

    // Waterfall: lista del cliente → lista default → precio_venta base
    let precio: f64 = conn
        .query_row(
            "SELECT COALESCE(
                (SELECT pp.precio FROM precios_producto pp
                 JOIN clientes c ON c.lista_precio_id = pp.lista_precio_id
                 WHERE pp.producto_id = ?1 AND c.id = ?2),
                (SELECT pp.precio FROM precios_producto pp
                 JOIN listas_precios lp ON lp.id = pp.lista_precio_id
                 WHERE pp.producto_id = ?1 AND lp.es_default = 1),
                (SELECT precio_venta FROM productos WHERE id = ?1)
            ) AS precio_final",
            rusqlite::params![producto_id, cliente_id],
            |row| row.get(0),
        )
        .map_err(|e| e.to_string())?;

    Ok(precio)
}
