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
/// Refresca el access_token de Google Drive usando el refresh_token via Edge Function.
async fn refrescar_gdrive_token(db: &Database) -> Result<String, String> {
    let (refresh_token, api_url, api_key) = {
        let conn = db.conn.lock().map_err(|e| e.to_string())?;
        let get = |key: &str| -> String {
            conn.query_row("SELECT value FROM config WHERE key = ?1", rusqlite::params![key], |row| row.get(0)).unwrap_or_default()
        };
        (get("gdrive_refresh_token"), get("licencia_api_url"), get("licencia_api_key"))
    };

    if refresh_token.is_empty() {
        return Err("No hay refresh_token. Reconecte Google Drive.".to_string());
    }

    let client = reqwest::Client::new();
    let resp = client
        .post(format!("{}/gdrive-auth", api_url))
        .header("Authorization", format!("Bearer {}", api_key))
        .header("apikey", &api_key)
        .json(&serde_json::json!({ "refresh_token": refresh_token }))
        .timeout(std::time::Duration::from_secs(15))
        .send()
        .await
        .map_err(|e| format!("Error refrescando token: {}", e))?;

    let data: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;

    if data["ok"].as_bool() != Some(true) {
        return Err(format!("Error refrescando: {}", data["error"].as_str().unwrap_or("desconocido")));
    }

    let new_token = data["access_token"].as_str().unwrap_or("").to_string();
    if new_token.is_empty() {
        return Err("No se obtuvo nuevo access_token".to_string());
    }

    // Guardar nuevo token
    {
        let conn = db.conn.lock().map_err(|e| e.to_string())?;
        conn.execute("INSERT OR REPLACE INTO config (key, value) VALUES ('gdrive_access_token', ?1)", rusqlite::params![new_token]).ok();
        // Si viene nuevo refresh_token, guardarlo también
        if let Some(new_rt) = data["refresh_token"].as_str() {
            if !new_rt.is_empty() {
                conn.execute("INSERT OR REPLACE INTO config (key, value) VALUES ('gdrive_refresh_token', ?1)", rusqlite::params![new_rt]).ok();
            }
        }
    }

    Ok(new_token)
}

#[tauri::command]
pub async fn backup_cloud_gdrive(db: State<'_, Database>) -> Result<String, String> {
    // Extraer todo lo sincrónico
    let (backup_data, mut access_token, folder_id) = {
        let (data, _lic) = crear_backup_encriptado(&db)?;
        let conn = db.conn.lock().map_err(|e| e.to_string())?;
        let at: String = conn.query_row("SELECT value FROM config WHERE key = 'gdrive_access_token'", [], |row| row.get(0)).unwrap_or_default();
        let fi: String = conn.query_row("SELECT value FROM config WHERE key = 'gdrive_folder_id'", [], |row| row.get(0)).unwrap_or_default();
        (data, at, fi)
    };

    if access_token.is_empty() {
        // Intentar refrescar automáticamente
        access_token = refrescar_gdrive_token(&db).await?;
    }

    let client = reqwest::Client::new();

    // Crear carpeta si no existe
    let folder = if folder_id.is_empty() {
        let resp = client
            .post("https://www.googleapis.com/drive/v3/files")
            .bearer_auth(&access_token)
            .json(&serde_json::json!({
                "name": "Clouget POS Backups",
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
            // Intentar refrescar token y reintentar
            match refrescar_gdrive_token(&db).await {
                Ok(_) => return Err("Token refrescado. Intente de nuevo.".to_string()),
                Err(_) => return Err("Token de Google Drive expirado. Reconecte su cuenta.".to_string()),
            }
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

/// Inicia el flujo OAuth2 de Google Drive:
/// 1. Abre un mini-servidor HTTP temporal en puerto 8847 para capturar el callback
/// 2. Abre el navegador con la URL de autorización
/// 3. Espera que Google redirija con el código
/// 4. Intercambia el código por tokens via la Edge Function gdrive-auth
/// 5. Guarda los tokens en config
#[tauri::command]
pub async fn conectar_gdrive(db: State<'_, Database>) -> Result<String, String> {
    // Constantes de respaldo (valores públicos de Supabase)
    const DEFAULT_CLIENT_ID: &str = "419804426556-ple84m5nr8473fs32f9ma2a12gl2vcdl.apps.googleusercontent.com";
    const DEFAULT_API_URL: &str = "https://zakquzflkvfqflqnxpxj.supabase.co/functions/v1";
    const DEFAULT_API_KEY: &str = "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJpc3MiOiJzdXBhYmFzZSIsInJlZiI6Inpha3F1emZsa3ZmcWZscW54cHhqIiwicm9sZSI6ImFub24iLCJpYXQiOjE3MzY2MDcxNjQsImV4cCI6MjA1MjE4MzE2NH0.sxaKNMkNguqQnvmUXh2JVRjqXDDqgsKb2LKPSGFp9bE";

    let (client_id, api_url, api_key) = {
        let conn = db.conn.lock().map_err(|e| e.to_string())?;
        let get = |key: &str| -> String {
            conn.query_row("SELECT value FROM config WHERE key = ?1", rusqlite::params![key], |row| row.get(0))
                .unwrap_or_default()
        };
        let cid = get("gdrive_client_id");
        let url = get("licencia_api_url");
        let key = get("licencia_api_key");
        (
            if cid.is_empty() { DEFAULT_CLIENT_ID.to_string() } else { cid },
            if url.is_empty() { DEFAULT_API_URL.to_string() } else { url },
            if key.is_empty() { DEFAULT_API_KEY.to_string() } else { key },
        )
    };

    // Iniciar mini-servidor para capturar el callback OAuth
    let (tx, rx) = tokio::sync::oneshot::channel::<String>();
    let tx = std::sync::Arc::new(tokio::sync::Mutex::new(Some(tx)));

    let tx_clone = tx.clone();
    let server = tokio::spawn(async move {
        let app = axum::Router::new().route("/oauth/callback", axum::routing::get(
            move |query: axum::extract::Query<std::collections::HashMap<String, String>>| {
                let tx = tx_clone.clone();
                async move {
                    let code = query.get("code").cloned().unwrap_or_default();
                    if let Some(sender) = tx.lock().await.take() {
                        sender.send(code).ok();
                    }
                    axum::response::Html(
                        "<html><body style='font-family:sans-serif;text-align:center;padding:60px;'>\
                        <h2>Google Drive conectado</h2>\
                        <p>Puede cerrar esta ventana y volver a Clouget POS.</p>\
                        </body></html>"
                    )
                }
            },
        ));

        let listener = tokio::net::TcpListener::bind("127.0.0.1:8847").await
            .expect("No se pudo iniciar servidor OAuth en puerto 8847");

        // Servir solo una request y terminar
        axum::serve(listener, app)
            .with_graceful_shutdown(async {
                tokio::time::sleep(std::time::Duration::from_secs(120)).await;
            })
            .await
            .ok();
    });

    // Abrir navegador con URL de autorización
    let redirect_uri = "http://localhost:8847/oauth/callback";
    let scope = "https://www.googleapis.com/auth/drive.file";
    let auth_url = format!(
        "https://accounts.google.com/o/oauth2/v2/auth?client_id={}&redirect_uri={}&response_type=code&scope={}&access_type=offline&prompt=consent",
        client_id,
        urlencoding(&redirect_uri),
        urlencoding(&scope),
    );

    // Abrir en el navegador del sistema
    #[cfg(target_os = "windows")]
    crate::utils::silent_command("cmd").args(["/C", "start", &auth_url.replace("&", "^&")]).spawn().ok();
    #[cfg(target_os = "linux")]
    std::process::Command::new("xdg-open").arg(&auth_url).spawn().ok();
    #[cfg(target_os = "macos")]
    std::process::Command::new("open").arg(&auth_url).spawn().ok();

    // Esperar el código (timeout 2 minutos)
    let code = tokio::time::timeout(std::time::Duration::from_secs(120), rx)
        .await
        .map_err(|_| "Tiempo agotado esperando autorización de Google Drive".to_string())?
        .map_err(|_| "Error recibiendo código OAuth".to_string())?;

    server.abort(); // Detener el mini-servidor

    if code.is_empty() {
        return Err("No se recibió código de autorización".to_string());
    }

    // Intercambiar código por tokens via Edge Function
    let client = reqwest::Client::new();
    let resp = client
        .post(format!("{}/gdrive-auth", api_url))
        .header("Authorization", format!("Bearer {}", api_key))
        .header("apikey", &api_key)
        .json(&serde_json::json!({ "code": code }))
        .timeout(std::time::Duration::from_secs(15))
        .send()
        .await
        .map_err(|e| format!("Error contactando servidor: {}", e))?;

    let data: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;

    if data["ok"].as_bool() != Some(true) {
        return Err(format!("Error de Google: {}", data["error"].as_str().unwrap_or("desconocido")));
    }

    let access_token = data["access_token"].as_str().unwrap_or("").to_string();
    let refresh_token = data["refresh_token"].as_str().unwrap_or("").to_string();

    // Guardar tokens
    {
        let conn = db.conn.lock().map_err(|e| e.to_string())?;
        conn.execute("INSERT OR REPLACE INTO config (key, value) VALUES ('gdrive_access_token', ?1)", rusqlite::params![access_token]).ok();
        conn.execute("INSERT OR REPLACE INTO config (key, value) VALUES ('gdrive_refresh_token', ?1)", rusqlite::params![refresh_token]).ok();
    }

    Ok("Google Drive conectado exitosamente".to_string())
}

fn urlencoding(s: &str) -> String {
    s.replace(":", "%3A").replace("/", "%2F").replace("?", "%3F").replace("=", "%3D").replace("&", "%26")
}
