use crate::db::Database;
use std::sync::Arc;

/// Inicia el scheduler de backup automático en background.
/// Lee la frecuencia de config y ejecuta backup periódicamente.
pub fn start_backup_scheduler(db: Database) {
    std::thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new().expect("Failed to create backup scheduler runtime");
        rt.block_on(async move {
            loop {
                // Leer configuración
                let (activo, tipo, frecuencia_horas) = {
                    let conn = db.conn.lock().unwrap();
                    let get = |key: &str, default: &str| -> String {
                        conn.query_row(
                            "SELECT value FROM config WHERE key = ?1",
                            rusqlite::params![key],
                            |row| row.get(0),
                        )
                        .unwrap_or_else(|_| default.to_string())
                    };
                    (
                        get("backup_cloud_activo", "0") == "1",
                        get("backup_cloud_tipo", ""),
                        get("backup_cloud_frecuencia", "6").parse::<u64>().unwrap_or(6),
                    )
                };

                // Esperar el intervalo configurado
                tokio::time::sleep(std::time::Duration::from_secs(frecuencia_horas * 3600)).await;

                if !activo || tipo.is_empty() {
                    continue;
                }

                // Ejecutar backup
                eprintln!("[Clouget Backup] Ejecutando backup automático (tipo: {})", tipo);

                match ejecutar_backup_interno(&db, &tipo).await {
                    Ok(msg) => eprintln!("[Clouget Backup] {}", msg),
                    Err(err) => eprintln!("[Clouget Backup] Error: {}", err),
                }
            }
        });
    });
}

/// Ejecuta el backup internamente (sin State<>, para uso desde scheduler).
async fn ejecutar_backup_interno(db: &Database, tipo: &str) -> Result<String, String> {
    let (backup_data, _lic) = super::cloud::crear_backup_encriptado(db)?;

    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let get = |key: &str| -> String {
        conn.query_row("SELECT value FROM config WHERE key = ?1", rusqlite::params![key], |row| row.get(0))
            .unwrap_or_default()
    };

    let api_url = get("licencia_api_url");
    let api_key = get("licencia_api_key");
    let licencia_codigo = get("licencia_codigo");
    let ruc = get("ruc");
    let access_token = get("gdrive_access_token");
    let folder_id = get("gdrive_folder_id");
    drop(conn);

    let client = reqwest::Client::new();

    match tipo {
        "premium" => {
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
                guardar_timestamp(db);
                Ok("Backup premium subido exitosamente".to_string())
            } else {
                Err(format!("Error del servidor: {}", response.status()))
            }
        }
        "gdrive" => {
            if access_token.is_empty() {
                return Err("Token de Google Drive no configurado".to_string());
            }

            let filename = format!("clouget-backup-{}.enc.gz",
                chrono::Local::now().format("%Y%m%d-%H%M%S"));

            let metadata = serde_json::json!({
                "name": filename,
                "parents": [folder_id],
            });

            let boundary = "clouget_boundary_sched";
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
                guardar_timestamp(db);
                Ok("Backup subido a Google Drive".to_string())
            } else {
                Err(format!("Error de Google Drive: {}", resp.status()))
            }
        }
        _ => Err(format!("Tipo de backup desconocido: {}", tipo)),
    }
}

fn guardar_timestamp(db: &Database) {
    if let Ok(conn) = db.conn.lock() {
        let now = chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
        conn.execute(
            "INSERT OR REPLACE INTO config (key, value) VALUES ('backup_cloud_ultima', ?1)",
            rusqlite::params![now],
        ).ok();
    }
}
