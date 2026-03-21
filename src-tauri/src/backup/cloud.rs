use crate::db::Database;
use super::encrypt;
use tauri::State;

/// Crea un backup comprimido y encriptado de la base de datos.
pub fn crear_backup_encriptado(db: &Database) -> Result<(Vec<u8>, String), String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;

    // WAL checkpoint
    conn.execute_batch("PRAGMA wal_checkpoint(TRUNCATE)").ok();

    let licencia: String = conn
        .query_row("SELECT value FROM config WHERE key = 'licencia_codigo'", [], |row| row.get(0))
        .unwrap_or_default();

    drop(conn);

    let db_path = Database::get_db_path_pub();
    let db_bytes = std::fs::read(&db_path)
        .map_err(|e| format!("Error leyendo BD: {}", e))?;

    let compressed = encrypt::compress(&db_bytes)?;

    let encrypted = if licencia.is_empty() {
        compressed
    } else {
        encrypt::encrypt(&compressed, &licencia)?
    };

    Ok((encrypted, licencia))
}

/// Sube un backup al servidor de Clouget (premium).
#[tauri::command]
pub async fn backup_cloud_premium(db: State<'_, Database>) -> Result<String, String> {
    // Extraer todo lo sincrónico antes de cualquier .await
    let (backup_data, api_url, api_key, licencia_codigo, ruc) = {
        let (data, _lic) = crear_backup_encriptado(&db)?;
        let conn = db.conn.lock().map_err(|e| e.to_string())?;
        let get = |key: &str| -> String {
            conn.query_row("SELECT value FROM config WHERE key = ?1", rusqlite::params![key], |row| row.get(0))
                .unwrap_or_default()
        };
        (data, get("licencia_api_url"), get("licencia_api_key"), get("licencia_codigo"), get("ruc"))
    };

    if api_url.is_empty() || licencia_codigo.is_empty() {
        return Err("Configure la licencia antes de usar backup cloud".to_string());
    }

    let client = reqwest::Client::new();
    let response = client
        .post(format!("{}/backup-upload", api_url))
        .header("Authorization", format!("Bearer {}", licencia_codigo))
        .header("apikey", &api_key)
        .header("x-ruc", &ruc)
        .header("Content-Type", "application/octet-stream")
        .body(backup_data)
        .timeout(std::time::Duration::from_secs(120))
        .send()
        .await
        .map_err(|e| format!("Error subiendo backup: {}", e))?;

    if response.status().is_success() {
        let now = chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
        let conn = db.conn.lock().map_err(|e| e.to_string())?;
        conn.execute(
            "INSERT OR REPLACE INTO config (key, value) VALUES ('backup_cloud_ultima', ?1)",
            rusqlite::params![now],
        ).ok();
        Ok(format!("Backup subido exitosamente ({})", now))
    } else {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        Err(format!("Error del servidor ({}): {}", status, body))
    }
}

/// Sube un backup a Google Drive del usuario (gratis).
#[tauri::command]
pub async fn backup_cloud_gdrive(db: State<'_, Database>) -> Result<String, String> {
    // Extraer todo lo sincrónico
    let (backup_data, access_token, folder_id) = {
        let (data, _lic) = crear_backup_encriptado(&db)?;
        let conn = db.conn.lock().map_err(|e| e.to_string())?;
        let at: String = conn.query_row("SELECT value FROM config WHERE key = 'gdrive_access_token'", [], |row| row.get(0)).unwrap_or_default();
        let fi: String = conn.query_row("SELECT value FROM config WHERE key = 'gdrive_folder_id'", [], |row| row.get(0)).unwrap_or_default();
        (data, at, fi)
    };

    if access_token.is_empty() {
        return Err("Conecte su cuenta de Google Drive primero".to_string());
    }

    let client = reqwest::Client::new();

    // Crear carpeta si no existe
    let folder = if folder_id.is_empty() {
        let resp = client
            .post("https://www.googleapis.com/drive/v3/files")
            .bearer_auth(&access_token)
            .json(&serde_json::json!({
                "name": "CloudgetPOS-Backups",
                "mimeType": "application/vnd.google-apps.folder"
            }))
            .send()
            .await
            .map_err(|e| format!("Error creando carpeta: {}", e))?;

        let body: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;
        let fid = body["id"].as_str().unwrap_or("").to_string();

        let conn = db.conn.lock().map_err(|e| e.to_string())?;
        conn.execute(
            "INSERT OR REPLACE INTO config (key, value) VALUES ('gdrive_folder_id', ?1)",
            rusqlite::params![fid],
        ).ok();

        fid
    } else {
        folder_id
    };

    let filename = format!("clouget-backup-{}.enc.gz",
        chrono::Local::now().format("%Y%m%d-%H%M%S"));

    // Upload multipart
    let metadata = serde_json::json!({
        "name": filename,
        "parents": [folder],
    });

    let boundary = "clouget_boundary_2026";
    let mut body = Vec::new();
    body.extend_from_slice(format!("--{}\r\n", boundary).as_bytes());
    body.extend_from_slice(b"Content-Type: application/json; charset=UTF-8\r\n\r\n");
    body.extend_from_slice(metadata.to_string().as_bytes());
    body.extend_from_slice(b"\r\n");
    body.extend_from_slice(format!("--{}\r\n", boundary).as_bytes());
    body.extend_from_slice(b"Content-Type: application/octet-stream\r\n\r\n");
    body.extend_from_slice(&backup_data);
    body.extend_from_slice(b"\r\n");
    body.extend_from_slice(format!("--{}--", boundary).as_bytes());

    let resp = client
        .post("https://www.googleapis.com/upload/drive/v3/files?uploadType=multipart")
        .bearer_auth(&access_token)
        .header("Content-Type", format!("multipart/related; boundary={}", boundary))
        .body(body)
        .timeout(std::time::Duration::from_secs(120))
        .send()
        .await
        .map_err(|e| format!("Error subiendo a Drive: {}", e))?;

    if resp.status().is_success() {
        let now = chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
        let conn = db.conn.lock().map_err(|e| e.to_string())?;
        conn.execute(
            "INSERT OR REPLACE INTO config (key, value) VALUES ('backup_cloud_ultima', ?1)",
            rusqlite::params![now],
        ).ok();
        Ok(format!("Backup subido a Google Drive ({})", now))
    } else {
        let status = resp.status();
        if status.as_u16() == 401 {
            return Err("Token de Google Drive expirado. Reconecte su cuenta.".to_string());
        }
        let body_text = resp.text().await.unwrap_or_default();
        Err(format!("Error de Google Drive ({}): {}", status, body_text))
    }
}

/// Ejecuta backup según la configuración.
#[tauri::command]
pub async fn ejecutar_backup_cloud(db: State<'_, Database>) -> Result<String, String> {
    let tipo: String = {
        let conn = db.conn.lock().map_err(|e| e.to_string())?;
        conn.query_row("SELECT value FROM config WHERE key = 'backup_cloud_tipo'", [], |row| row.get(0))
            .unwrap_or_default()
    };

    match tipo.as_str() {
        "premium" => backup_cloud_premium(db).await,
        "gdrive" => backup_cloud_gdrive(db).await,
        _ => Err("Tipo de backup no configurado".to_string()),
    }
}

/// Obtiene el estado del backup cloud.
#[tauri::command]
pub fn estado_backup_cloud(db: State<Database>) -> Result<serde_json::Value, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let get = |key: &str| -> String {
        conn.query_row("SELECT value FROM config WHERE key = ?1", rusqlite::params![key], |row| row.get(0))
            .unwrap_or_default()
    };

    Ok(serde_json::json!({
        "activo": get("backup_cloud_activo") == "1",
        "tipo": get("backup_cloud_tipo"),
        "frecuencia_horas": get("backup_cloud_frecuencia").parse::<i64>().unwrap_or(6),
        "ultima": get("backup_cloud_ultima"),
        "gdrive_conectado": !get("gdrive_access_token").is_empty(),
    }))
}

/// Guarda tokens de Google Drive OAuth2.
#[tauri::command]
pub fn guardar_gdrive_tokens(
    db: State<Database>,
    access_token: String,
    refresh_token: String,
) -> Result<(), String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    conn.execute("INSERT OR REPLACE INTO config (key, value) VALUES ('gdrive_access_token', ?1)", rusqlite::params![access_token]).ok();
    conn.execute("INSERT OR REPLACE INTO config (key, value) VALUES ('gdrive_refresh_token', ?1)", rusqlite::params![refresh_token]).ok();
    Ok(())
}

/// Desconecta Google Drive.
#[tauri::command]
pub fn desconectar_gdrive(db: State<Database>) -> Result<(), String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    conn.execute("DELETE FROM config WHERE key IN ('gdrive_access_token', 'gdrive_refresh_token', 'gdrive_folder_id')", []).ok();
    Ok(())
}
