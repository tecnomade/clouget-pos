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

    // Desactivar FK temporalmente para hacer un truncate en cascada limpio.
    let _ = conn.execute("PRAGMA foreign_keys = OFF", []);

    // Borrar en orden dependiente (hijos primero, luego padres).
    // Cada DELETE va con `let _ =` para que tablas opcionales (no en todas las DBs)
    // no rompan el reset si no existen — el error se ignora y continua con la siguiente.
    let tablas_a_borrar = [
        // Combos en ventas (hijo de venta_detalles)
        "venta_detalle_combo",
        // Series asignadas a ventas
        "serie_venta",
        // Pagos de cuentas (CXC + CXP)
        "pagos_cuenta",
        "pagos_proveedor",
        // Cuentas por cobrar y pagar (deben ir antes de ventas/compras)
        "cuentas_por_cobrar",
        "cuentas_por_pagar",
        // Notas de credito (deben ir antes de ventas)
        "nota_credito_detalles",
        "notas_credito",
        // Detalles de venta y venta
        "venta_detalles",
        "ventas",
        // Lotes de caducidad (FK hacia compras)
        "lotes_caducidad",
        // Detalles de compra y compra
        "compra_detalles",
        "compras",
        // Movimientos / kardex
        "movimientos_inventario",
        // Multi-almacen
        "transferencias_detalles",
        "transferencias",
        "stock_establecimiento",
        // Caja: eventos (FK hacia caja), retiros, gastos, caja
        "caja_eventos",
        "retiros_caja",
        "gastos",
        "caja",
        // Servicio Tecnico
        "orden_servicio_movimientos",
        "orden_servicio_imagenes",
        "orden_servicio_repuestos",
        "ordenes_servicio",
        // Otros catalogos transaccionales
        "choferes",
        "proveedores",
        "series_producto",
    ];

    let mut errores: Vec<String> = Vec::new();
    for tabla in &tablas_a_borrar {
        // Verificar si la tabla existe primero
        let existe: bool = conn.query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name=?1",
            rusqlite::params![tabla],
            |r| r.get::<_, i64>(0).map(|c| c > 0),
        ).unwrap_or(false);
        if !existe { continue; }
        if let Err(e) = conn.execute(&format!("DELETE FROM {}", tabla), []) {
            errores.push(format!("{}: {}", tabla, e));
        }
    }

    // Reactivar FK siempre
    let _ = conn.execute("PRAGMA foreign_keys = ON", []);

    if !errores.is_empty() {
        return Err(format!("Errores al resetear: {}", errores.join("; ")));
    }

    // Stock = 0 (no eliminar productos, solo resetear inventario)
    let _ = conn.execute("UPDATE productos SET stock_actual = 0", []);

    // Reiniciar secuenciales
    let _ = conn.execute("DELETE FROM secuenciales", []);
    let _ = conn.execute("UPDATE config SET value = '1' WHERE key LIKE 'secuencial_%'", []);
    let _ = conn.execute("UPDATE config SET value = '0' WHERE key = 'secuencial_compra'", []);

    Ok("Base de datos reseteada exitosamente".to_string())
}
