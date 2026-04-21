use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use crate::db::Database;
use std::collections::HashMap;
use tauri::State;

#[tauri::command]
pub fn obtener_config(db: State<Database>) -> Result<HashMap<String, String>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;

    let mut stmt = conn
        .prepare("SELECT key, value FROM config")
        .map_err(|e| e.to_string())?;

    let config = stmt
        .query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<HashMap<_, _>, _>>()
        .map_err(|e| e.to_string())?;

    Ok(config)
}

#[tauri::command]
pub fn guardar_config(db: State<Database>, configs: HashMap<String, String>) -> Result<(), String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;

    for (key, value) in configs {
        conn.execute(
            "INSERT OR REPLACE INTO config (key, value) VALUES (?1, ?2)",
            rusqlite::params![key, value],
        )
        .map_err(|e| e.to_string())?;
    }

    Ok(())
}

#[tauri::command]
pub fn cargar_logo_negocio(db: State<Database>, logo_path: String) -> Result<String, String> {
    let bytes = std::fs::read(&logo_path)
        .map_err(|e| format!("Error leyendo imagen: {}", e))?;

    // Validar tamaño máximo (500KB)
    if bytes.len() > 500_000 {
        return Err("La imagen es demasiado grande. Máximo 500KB.".to_string());
    }

    // Guardar como base64 en config
    let b64 = BASE64.encode(&bytes);

    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    conn.execute(
        "INSERT OR REPLACE INTO config (key, value) VALUES ('logo_negocio', ?1)",
        rusqlite::params![b64],
    )
    .map_err(|e| e.to_string())?;

    Ok("Logo cargado correctamente".to_string())
}

#[tauri::command]
pub fn eliminar_logo_negocio(db: State<Database>) -> Result<String, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    conn.execute(
        "DELETE FROM config WHERE key = 'logo_negocio'",
        [],
    )
    .map_err(|e| e.to_string())?;

    Ok("Logo eliminado".to_string())
}

/// Genera un token aleatorio para el servidor de red y lo guarda en config.
#[tauri::command]
pub fn generar_token_servidor(db: State<Database>) -> Result<String, String> {
    use rand::Rng;
    let mut rng = rand::thread_rng();
    let token: String = (0..32)
        .map(|_| {
            let idx = rng.gen_range(0..36);
            if idx < 10 {
                (b'0' + idx) as char
            } else {
                (b'a' + idx - 10) as char
            }
        })
        .collect();

    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    conn.execute(
        "INSERT OR REPLACE INTO config (key, value) VALUES ('servidor_token', ?1)",
        rusqlite::params![token],
    )
    .map_err(|e| e.to_string())?;

    Ok(token)
}

/// Obtiene los secuenciales actuales de la tabla `secuenciales` para mostrar en Config.
#[tauri::command]
pub fn obtener_secuenciales(db: State<Database>) -> Result<HashMap<String, i64>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;

    let mut result = HashMap::new();

    let mut stmt = conn
        .prepare("SELECT establecimiento_codigo, punto_emision_codigo, tipo_documento, secuencial FROM secuenciales")
        .map_err(|e| e.to_string())?;

    let rows = stmt
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, i64>(3)?,
            ))
        })
        .map_err(|e| e.to_string())?;

    for r in rows {
        let (est, pe, tipo, sec) = r.map_err(|e| e.to_string())?;
        let key = format!("{}_{}_{}_{}", est, pe, tipo, "secuencial");
        result.insert(key, sec);
        // Also insert simplified keys for current establecimiento/punto_emision
        match tipo.as_str() {
            "FACTURA" => { result.insert("secuencial_factura".to_string(), sec); },
            "NOTA_CREDITO" => { result.insert("secuencial_nc".to_string(), sec); },
            _ => {}
        }
    }

    Ok(result)
}

/// Actualiza un secuencial en la tabla `secuenciales`.
#[tauri::command]
pub fn actualizar_secuencial(
    db: State<Database>,
    establecimiento: String,
    punto_emision: String,
    tipo_documento: String,
    secuencial: i64,
) -> Result<(), String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;

    conn.execute(
        "INSERT OR REPLACE INTO secuenciales (establecimiento_codigo, punto_emision_codigo, tipo_documento, secuencial) VALUES (?1, ?2, ?3, ?4)",
        rusqlite::params![establecimiento, punto_emision, tipo_documento, secuencial],
    )
    .map_err(|e| e.to_string())?;

    Ok(())
}

/// Prueba la conexión a un servidor remoto de Clouget POS.
#[tauri::command]
pub async fn probar_conexion_servidor(url: String, token: String) -> Result<String, String> {
    let client = reqwest::Client::new();
    let response = client
        .get(format!("{}/api/v1/ping", url))
        .header("Authorization", format!("Bearer {}", token))
        .timeout(std::time::Duration::from_secs(5))
        .send()
        .await
        .map_err(|e| format!("No se pudo conectar: {}", e))?;

    let body = response
        .text()
        .await
        .map_err(|e| format!("Error leyendo respuesta: {}", e))?;

    if body == "clouget-pos-server" {
        Ok("Conexión exitosa".to_string())
    } else {
        Err(format!("Respuesta inesperada: {}", body))
    }
}

#[tauri::command]
pub fn resetear_base_datos(
    db: State<Database>,
    sesion: State<crate::db::SesionState>,
    confirmacion: String,
) -> Result<String, String> {
    let sesion_guard = sesion.sesion.lock().map_err(|e| e.to_string())?;
    let ses = sesion_guard.as_ref().ok_or("No hay sesion activa")?;
    if ses.rol != "ADMIN" {
        return Err("Solo el administrador puede resetear la base de datos".to_string());
    }
    drop(sesion_guard);

    if confirmacion != "RESETEAR" {
        return Err("Debe escribir RESETEAR para confirmar".to_string());
    }

    let conn = db.conn.lock().map_err(|e| e.to_string())?;

    conn.execute_batch("
        DELETE FROM pagos_proveedor;
        DELETE FROM cuentas_por_pagar;
        DELETE FROM compra_detalles;
        DELETE FROM compras;
        DELETE FROM nota_credito_detalles;
        DELETE FROM notas_credito;
        DELETE FROM venta_detalles;
        DELETE FROM ventas;
        DELETE FROM retiros_caja;
        DELETE FROM gastos;
        DELETE FROM caja;
        DELETE FROM choferes;
        DELETE FROM proveedores;
    ").map_err(|e| format!("Error reseteando datos: {}", e))?;

    conn.execute("UPDATE productos SET stock_actual = 0", [])
        .map_err(|e| format!("Error reseteando stock: {}", e))?;

    conn.execute_batch("
        DELETE FROM secuenciales;
        UPDATE config SET value = '0' WHERE key = 'secuencial_compra';
    ").map_err(|e| format!("Error reseteando secuenciales: {}", e))?;

    Ok("Base de datos reseteada exitosamente".to_string())
}
