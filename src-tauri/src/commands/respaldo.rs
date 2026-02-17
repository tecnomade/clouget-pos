use tauri::State;
use crate::db::Database;

/// Retorna la ruta actual de la base de datos
#[tauri::command]
pub fn obtener_ruta_db() -> Result<String, String> {
    let db_path = get_db_path();
    Ok(db_path.to_string_lossy().to_string())
}

/// Crea un respaldo de la base de datos en la ruta destino
#[tauri::command]
pub fn crear_respaldo(db: State<Database>, destino: String) -> Result<String, String> {
    let conn = db.conn.lock().unwrap();

    // Forzar checkpoint WAL para que todo esté en el archivo principal
    conn.execute_batch("PRAGMA wal_checkpoint(TRUNCATE);")
        .map_err(|e| format!("Error en WAL checkpoint: {}", e))?;

    drop(conn); // Liberar el lock antes de copiar

    let db_path = get_db_path();

    if !db_path.exists() {
        return Err("No se encontró el archivo de base de datos".to_string());
    }

    std::fs::copy(&db_path, &destino)
        .map_err(|e| format!("Error al copiar la base de datos: {}", e))?;

    Ok(destino)
}

/// Restaura un respaldo reemplazando la base de datos actual
#[tauri::command]
pub fn restaurar_respaldo(db: State<Database>, origen: String) -> Result<String, String> {
    let origen_path = std::path::PathBuf::from(&origen);

    if !origen_path.exists() {
        return Err("El archivo de respaldo no existe".to_string());
    }

    // Validar que es un archivo SQLite (header magic bytes)
    let header = std::fs::read(&origen_path)
        .map_err(|e| format!("Error al leer archivo: {}", e))?;

    if header.len() < 16 || &header[..16] != b"SQLite format 3\0" {
        return Err("El archivo seleccionado no es una base de datos SQLite válida".to_string());
    }

    let conn = db.conn.lock().unwrap();

    // Checkpoint WAL actual
    conn.execute_batch("PRAGMA wal_checkpoint(TRUNCATE);")
        .map_err(|e| format!("Error en WAL checkpoint: {}", e))?;

    drop(conn);

    let db_path = get_db_path();

    // Crear respaldo automático antes de restaurar
    let backup_auto = db_path.with_extension("db.pre-restore");
    std::fs::copy(&db_path, &backup_auto).ok(); // No fallar si no se puede

    // Copiar el respaldo sobre la BD actual
    std::fs::copy(&origen_path, &db_path)
        .map_err(|e| format!("Error al restaurar: {}", e))?;

    // Eliminar archivos WAL/SHM del respaldo anterior si existen
    let wal_path = db_path.with_extension("db-wal");
    let shm_path = db_path.with_extension("db-shm");
    std::fs::remove_file(&wal_path).ok();
    std::fs::remove_file(&shm_path).ok();

    Ok("Respaldo restaurado. Reinicie la aplicación para aplicar los cambios.".to_string())
}

fn get_db_path() -> std::path::PathBuf {
    #[cfg(target_os = "windows")]
    {
        std::env::var("LOCALAPPDATA")
            .map(|p| std::path::PathBuf::from(p).join("CloudgetPOS").join("clouget-pos.db"))
            .unwrap_or_else(|_| std::path::PathBuf::from("clouget-pos.db"))
    }
    #[cfg(not(target_os = "windows"))]
    {
        std::env::var("HOME")
            .map(|p| std::path::PathBuf::from(p).join(".clouget-pos").join("clouget-pos.db"))
            .unwrap_or_else(|_| std::path::PathBuf::from("clouget-pos.db"))
    }
}
