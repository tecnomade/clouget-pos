use super::OfflineDb;
use serde_json::Value;
use tauri::State;

/// Guarda una operación en la cola offline para sincronizar después.
#[tauri::command]
pub fn encolar_operacion(
    offline: State<Option<OfflineDb>>,
    comando: String,
    params_json: String,
) -> Result<i64, String> {
    let db = offline
        .inner()
        .as_ref()
        .ok_or("Base de datos offline no disponible")?;
    let conn = db.conn.lock().map_err(|e| e.to_string())?;

    conn.execute(
        "INSERT INTO cola_operaciones (comando, params_json) VALUES (?1, ?2)",
        rusqlite::params![comando, params_json],
    )
    .map_err(|e| e.to_string())?;

    Ok(conn.last_insert_rowid())
}

/// Lista operaciones pendientes en la cola offline.
#[tauri::command]
pub fn listar_cola_offline(
    offline: State<Option<OfflineDb>>,
) -> Result<Vec<Value>, String> {
    let db = offline
        .inner()
        .as_ref()
        .ok_or("Base de datos offline no disponible")?;
    let conn = db.conn.lock().map_err(|e| e.to_string())?;

    let mut stmt = conn
        .prepare("SELECT id, comando, params_json, created_at, estado, intentos, ultimo_error FROM cola_operaciones WHERE estado = 'PENDIENTE' ORDER BY id ASC")
        .map_err(|e| e.to_string())?;

    let ops: Vec<Value> = stmt
        .query_map([], |row| {
            Ok(serde_json::json!({
                "id": row.get::<_, i64>(0)?,
                "comando": row.get::<_, String>(1)?,
                "params_json": row.get::<_, String>(2)?,
                "created_at": row.get::<_, String>(3)?,
                "estado": row.get::<_, String>(4)?,
                "intentos": row.get::<_, i64>(5)?,
                "ultimo_error": row.get::<_, Option<String>>(6)?,
            }))
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    Ok(ops)
}

/// Marca una operación de la cola como enviada (sincronizada).
#[tauri::command]
pub fn marcar_operacion_enviada(
    offline: State<Option<OfflineDb>>,
    id: i64,
) -> Result<(), String> {
    let db = offline
        .inner()
        .as_ref()
        .ok_or("Base de datos offline no disponible")?;
    let conn = db.conn.lock().map_err(|e| e.to_string())?;

    conn.execute(
        "UPDATE cola_operaciones SET estado = 'ENVIADA' WHERE id = ?1",
        rusqlite::params![id],
    )
    .map_err(|e| e.to_string())?;

    Ok(())
}

/// Marca una operación como error.
#[tauri::command]
pub fn marcar_operacion_error(
    offline: State<Option<OfflineDb>>,
    id: i64,
    error: String,
) -> Result<(), String> {
    let db = offline
        .inner()
        .as_ref()
        .ok_or("Base de datos offline no disponible")?;
    let conn = db.conn.lock().map_err(|e| e.to_string())?;

    conn.execute(
        "UPDATE cola_operaciones SET estado = 'ERROR', ultimo_error = ?1, intentos = intentos + 1 WHERE id = ?2",
        rusqlite::params![error, id],
    )
    .map_err(|e| e.to_string())?;

    Ok(())
}

/// Cuenta operaciones pendientes en la cola.
#[tauri::command]
pub fn contar_cola_offline(
    offline: State<Option<OfflineDb>>,
) -> Result<i64, String> {
    let db = offline
        .inner()
        .as_ref()
        .ok_or("Base de datos offline no disponible")?;
    let conn = db.conn.lock().map_err(|e| e.to_string())?;

    conn.query_row(
        "SELECT COUNT(*) FROM cola_operaciones WHERE estado = 'PENDIENTE'",
        [],
        |row| row.get(0),
    )
    .map_err(|e| e.to_string())
}

/// Sincroniza el cache de productos desde el servidor.
/// Se llama cuando el terminal está online.
#[tauri::command]
pub async fn sincronizar_cache_productos(
    offline: State<'_, Option<OfflineDb>>,
    productos_json: String,
) -> Result<i64, String> {
    let db = offline
        .inner()
        .as_ref()
        .ok_or("Base de datos offline no disponible")?;

    let productos: Vec<Value> = serde_json::from_str(&productos_json)
        .map_err(|e| format!("Error parseando productos: {}", e))?;

    let conn = db.conn.lock().map_err(|e| e.to_string())?;

    // Reemplazar cache completo
    conn.execute("DELETE FROM cache_productos", []).ok();

    let mut count: i64 = 0;
    for p in &productos {
        conn.execute(
            "INSERT INTO cache_productos (id, codigo, codigo_barras, nombre, precio_venta, iva_porcentaje, stock_actual, stock_minimo, categoria_nombre, es_servicio)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            rusqlite::params![
                p["id"].as_i64(),
                p["codigo"].as_str(),
                p["codigo_barras"].as_str(),
                p["nombre"].as_str().unwrap_or(""),
                p["precio_venta"].as_f64().unwrap_or(0.0),
                p["iva_porcentaje"].as_f64().unwrap_or(0.0),
                p["stock_actual"].as_f64().unwrap_or(0.0),
                p["stock_minimo"].as_f64().unwrap_or(0.0),
                p["categoria_nombre"].as_str(),
                p["es_servicio"].as_bool().unwrap_or(false),
            ],
        ).ok();
        count += 1;
    }

    Ok(count)
}

/// Busca productos en el cache offline local.
#[tauri::command]
pub fn buscar_productos_offline(
    offline: State<Option<OfflineDb>>,
    termino: String,
) -> Result<Vec<Value>, String> {
    let db = offline
        .inner()
        .as_ref()
        .ok_or("Base de datos offline no disponible")?;
    let conn = db.conn.lock().map_err(|e| e.to_string())?;

    let busqueda = format!("%{}%", termino);
    let mut stmt = conn
        .prepare(
            "SELECT id, codigo, nombre, precio_venta, iva_porcentaje, stock_actual, stock_minimo, categoria_nombre
             FROM cache_productos
             WHERE nombre LIKE ?1 OR codigo LIKE ?1 OR codigo_barras LIKE ?1
             LIMIT 50",
        )
        .map_err(|e| e.to_string())?;

    let productos: Vec<Value> = stmt
        .query_map(rusqlite::params![busqueda], |row| {
            Ok(serde_json::json!({
                "id": row.get::<_, i64>(0)?,
                "codigo": row.get::<_, Option<String>>(1)?,
                "nombre": row.get::<_, String>(2)?,
                "precio_venta": row.get::<_, f64>(3)?,
                "iva_porcentaje": row.get::<_, f64>(4)?,
                "stock_actual": row.get::<_, f64>(5)?,
                "stock_minimo": row.get::<_, f64>(6)?,
                "categoria_nombre": row.get::<_, Option<String>>(7)?,
            }))
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    Ok(productos)
}

/// Reserva un rango de secuenciales del servidor para uso offline.
#[tauri::command]
pub fn guardar_secuenciales_reservados(
    offline: State<Option<OfflineDb>>,
    tipo_documento: String,
    desde: i64,
    hasta: i64,
) -> Result<(), String> {
    let db = offline
        .inner()
        .as_ref()
        .ok_or("Base de datos offline no disponible")?;
    let conn = db.conn.lock().map_err(|e| e.to_string())?;

    conn.execute(
        "INSERT OR REPLACE INTO secuenciales_reservados (tipo_documento, desde, hasta, actual) VALUES (?1, ?2, ?3, ?2)",
        rusqlite::params![tipo_documento, desde, hasta],
    )
    .map_err(|e| e.to_string())?;

    Ok(())
}

/// Obtiene y consume el siguiente secuencial reservado para uso offline.
#[tauri::command]
pub fn obtener_secuencial_offline(
    offline: State<Option<OfflineDb>>,
    tipo_documento: String,
) -> Result<i64, String> {
    let db = offline
        .inner()
        .as_ref()
        .ok_or("Base de datos offline no disponible")?;
    let conn = db.conn.lock().map_err(|e| e.to_string())?;

    let (actual, hasta): (i64, i64) = conn
        .query_row(
            "SELECT actual, hasta FROM secuenciales_reservados WHERE tipo_documento = ?1",
            rusqlite::params![tipo_documento],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .map_err(|_| format!("No hay secuenciales reservados para {}", tipo_documento))?;

    if actual > hasta {
        return Err("Se agotaron los secuenciales reservados. Reconecte al servidor.".to_string());
    }

    // Incrementar
    conn.execute(
        "UPDATE secuenciales_reservados SET actual = actual + 1 WHERE tipo_documento = ?1",
        rusqlite::params![tipo_documento],
    )
    .map_err(|e| e.to_string())?;

    Ok(actual)
}
