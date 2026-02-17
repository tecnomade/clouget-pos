use serde::{Deserialize, Serialize};
use sha2::{Sha256, Digest};
use tauri::State;
use crate::db::Database;

/// Dias de gracia para operar sin conexion al servidor de validacion
const DIAS_GRACIA_OFFLINE: i64 = 7;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct LicenciaInfo {
    pub negocio: String,
    pub email: String,
    pub tipo: String,       // "perpetua", "anual", "trial"
    pub emitida: String,    // fecha ISO
    pub machine_id: String, // fingerprint del equipo
    pub activa: bool,
}

/// Respuesta del endpoint activar-licencia de Supabase
#[derive(Debug, Deserialize)]
struct RespuestaActivacion {
    ok: bool,
    negocio: Option<String>,
    email: Option<String>,
    tipo: Option<String>,
    emitida: Option<String>,
    mensaje: Option<String>,
}

/// Respuesta del endpoint validar-licencia de Supabase
#[derive(Debug, Deserialize)]
struct RespuestaValidacion {
    activa: bool,
    negocio: Option<String>,
    email: Option<String>,
    tipo: Option<String>,
    emitida: Option<String>,
    #[allow(dead_code)]
    mensaje: Option<String>,
}

// ─── Machine ID ────────────────────────────────────────────

/// Obtiene el Machine ID único de este equipo
/// Windows: SHA-256 de HKLM\SOFTWARE\Microsoft\Cryptography\MachineGuid
/// Linux/Mac: SHA-256 de /etc/machine-id
#[tauri::command]
pub fn obtener_machine_id() -> Result<String, String> {
    let raw = get_raw_machine_guid()?;
    let hash = Sha256::digest(raw.as_bytes());
    let hex = format!("{:x}", hash);
    // Primeros 8 caracteres en mayúsculas para fácil lectura por WhatsApp
    Ok(hex[..8].to_uppercase())
}

#[cfg(target_os = "windows")]
fn get_raw_machine_guid() -> Result<String, String> {
    use winreg::enums::*;
    use winreg::RegKey;

    let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
    let crypto_key = hklm
        .open_subkey("SOFTWARE\\Microsoft\\Cryptography")
        .map_err(|e| format!("No se pudo leer el registro de Windows: {}", e))?;

    let guid: String = crypto_key
        .get_value("MachineGuid")
        .map_err(|e| format!("No se encontró MachineGuid: {}", e))?;

    Ok(guid)
}

#[cfg(not(target_os = "windows"))]
fn get_raw_machine_guid() -> Result<String, String> {
    if let Ok(id) = std::fs::read_to_string("/etc/machine-id") {
        return Ok(id.trim().to_string());
    }
    if let Ok(id) = std::fs::read_to_string("/var/lib/dbus/machine-id") {
        return Ok(id.trim().to_string());
    }
    Err("No se pudo obtener el identificador de esta máquina".to_string())
}

// ─── Activar Licencia (online) ─────────────────────────────

/// Activa una licencia usando un código de activación corto.
/// Llama al endpoint activar-licencia en Supabase.
#[tauri::command]
pub async fn verificar_licencia(
    db: State<'_, Database>,
    clave_licencia: String,
) -> Result<LicenciaInfo, String> {
    let codigo = clave_licencia.trim().to_uppercase();
    if codigo.is_empty() {
        return Err("Ingrese el código de activación".to_string());
    }

    let machine_id = obtener_machine_id()?;

    // Leer URL y API key
    let (api_url, api_key) = {
        let conn = db.conn.lock().map_err(|e| e.to_string())?;
        let get = |key: &str| -> String {
            conn.query_row(
                "SELECT value FROM config WHERE key = ?1",
                rusqlite::params![key],
                |row| row.get(0),
            )
            .unwrap_or_default()
        };
        (get("licencia_api_url"), get("licencia_api_key"))
    };

    if api_url.is_empty() {
        return Err("URL del servidor de licencias no configurada".to_string());
    }

    let endpoint = format!("{}/activar-licencia", api_url);

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .build()
        .map_err(|e| format!("Error creando cliente HTTP: {}", e))?;

    let body = serde_json::json!({
        "codigo": codigo,
        "machine_id": machine_id,
    });

    let resp = client
        .post(&endpoint)
        .header("Content-Type", "application/json")
        .header("Authorization", format!("Bearer {}", api_key))
        .header("apikey", &api_key)
        .json(&body)
        .send()
        .await
        .map_err(|_| "No se pudo conectar al servidor de licencias. Verifique su conexión a internet.".to_string())?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body_text = resp.text().await.unwrap_or_default();
        // Intentar parsear mensaje del servidor
        if let Ok(err_data) = serde_json::from_str::<serde_json::Value>(&body_text) {
            if let Some(msg) = err_data.get("mensaje").and_then(|v| v.as_str()) {
                return Err(msg.to_string());
            }
        }
        return Err(format!("Error del servidor (HTTP {})", status));
    }

    let data: RespuestaActivacion = resp
        .json()
        .await
        .map_err(|e| format!("Error parseando respuesta: {}", e))?;

    if !data.ok {
        return Err(data.mensaje.unwrap_or_else(|| "Código de activación inválido o ya utilizado".to_string()));
    }

    let negocio = data.negocio.unwrap_or_default();
    let email = data.email.unwrap_or_default();
    let tipo = data.tipo.unwrap_or_else(|| "perpetua".to_string());
    let emitida = data.emitida.unwrap_or_default();
    let hoy = chrono::Local::now().format("%Y-%m-%d").to_string();

    // Guardar en config
    {
        let conn = db.conn.lock().map_err(|e| e.to_string())?;
        let configs = [
            ("licencia_activada", "1"),
            ("licencia_codigo", &codigo),
            ("licencia_negocio", &negocio),
            ("licencia_email", &email),
            ("licencia_tipo", &tipo),
            ("licencia_emitida", &emitida),
            ("licencia_machine_id", &machine_id),
            ("licencia_ultima_validacion", &hoy),
        ];

        for (key, value) in &configs {
            conn.execute(
                "INSERT OR REPLACE INTO config (key, value) VALUES (?1, ?2)",
                rusqlite::params![key, value],
            )
            .map_err(|e| format!("Error guardando licencia: {}", e))?;
        }
    }

    Ok(LicenciaInfo {
        negocio,
        email,
        tipo,
        emitida,
        machine_id,
        activa: true,
    })
}

// ─── Obtener Estado de Licencia ────────────────────────────

/// Obtiene el estado actual de la licencia desde la caché local.
/// Si hay conexión, re-valida online en silencio.
#[tauri::command]
pub async fn obtener_estado_licencia(
    db: State<'_, Database>,
) -> Result<Option<LicenciaInfo>, String> {
    // Leer datos de cache
    let (activada, codigo, negocio, email, tipo, emitida, machine_id_cache, ultima_validacion, api_url, api_key) = {
        let conn = db.conn.lock().map_err(|e| e.to_string())?;
        let get = |key: &str| -> String {
            conn.query_row(
                "SELECT value FROM config WHERE key = ?1",
                rusqlite::params![key],
                |row| row.get(0),
            )
            .unwrap_or_default()
        };
        (
            get("licencia_activada"),
            get("licencia_codigo"),
            get("licencia_negocio"),
            get("licencia_email"),
            get("licencia_tipo"),
            get("licencia_emitida"),
            get("licencia_machine_id"),
            get("licencia_ultima_validacion"),
            get("licencia_api_url"),
            get("licencia_api_key"),
        )
    };

    // Si nunca se activó, no hay licencia
    if activada != "1" || codigo.is_empty() {
        return Ok(None);
    }

    // Verificar machine_id coincide
    let mi_machine_id = obtener_machine_id().unwrap_or_default();
    if !machine_id_cache.is_empty() && machine_id_cache != mi_machine_id {
        return Ok(None); // Licencia de otro equipo
    }

    // Intentar re-validar online (silencioso)
    let mut activa = true;
    let mut negocio_actual = negocio.clone();
    let mut email_actual = email.clone();
    let mut tipo_actual = tipo.clone();
    let mut emitida_actual = emitida.clone();

    if !api_url.is_empty() {
        match revalidar_online(&mi_machine_id, &api_url, &api_key).await {
            Ok(Some(resp)) => {
                activa = resp.activa;
                if let Some(n) = resp.negocio { negocio_actual = n; }
                if let Some(e) = resp.email { email_actual = e; }
                if let Some(t) = resp.tipo { tipo_actual = t; }
                if let Some(em) = resp.emitida { emitida_actual = em; }

                // Actualizar cache
                let hoy = chrono::Local::now().format("%Y-%m-%d").to_string();
                let conn = db.conn.lock().map_err(|e| e.to_string())?;
                let configs = [
                    ("licencia_activada", if activa { "1" } else { "0" }),
                    ("licencia_negocio", &negocio_actual),
                    ("licencia_email", &email_actual),
                    ("licencia_tipo", &tipo_actual),
                    ("licencia_emitida", &emitida_actual),
                    ("licencia_ultima_validacion", &hoy),
                ];
                for (key, value) in &configs {
                    conn.execute(
                        "INSERT OR REPLACE INTO config (key, value) VALUES (?1, ?2)",
                        rusqlite::params![key, value],
                    ).ok();
                }
            }
            Ok(None) => {
                // Respuesta inválida — usar cache
            }
            Err(_) => {
                // Sin conexión — verificar gracia offline
                activa = evaluar_gracia_offline(&ultima_validacion);
            }
        }
    }

    if !activa {
        return Ok(None);
    }

    Ok(Some(LicenciaInfo {
        negocio: negocio_actual,
        email: email_actual,
        tipo: tipo_actual,
        emitida: emitida_actual,
        machine_id: mi_machine_id,
        activa: true,
    }))
}

// ─── Helpers ───────────────────────────────────────────────

/// Re-valida la licencia online llamando al endpoint validar-licencia.
async fn revalidar_online(machine_id: &str, api_url: &str, api_key: &str) -> Result<Option<RespuestaValidacion>, String> {
    let endpoint = format!("{}/validar-licencia", api_url);

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(8))
        .build()
        .map_err(|e| format!("Error HTTP: {}", e))?;

    let body = serde_json::json!({
        "machine_id": machine_id,
    });

    let resp = client
        .post(&endpoint)
        .header("Content-Type", "application/json")
        .header("Authorization", format!("Bearer {}", api_key))
        .header("apikey", api_key)
        .json(&body)
        .send()
        .await
        .map_err(|_| "SIN_CONEXION".to_string())?;

    if !resp.status().is_success() {
        return Ok(None);
    }

    let data: RespuestaValidacion = resp
        .json()
        .await
        .map_err(|_| "Error parseando".to_string())?;

    Ok(Some(data))
}

/// Evalúa si la caché offline es válida (dentro de 7 días de gracia).
fn evaluar_gracia_offline(ultima_validacion: &str) -> bool {
    if ultima_validacion.is_empty() {
        return false;
    }

    use chrono::NaiveDate;
    let hoy = chrono::Local::now().format("%Y-%m-%d").to_string();

    let desde = NaiveDate::parse_from_str(ultima_validacion, "%Y-%m-%d");
    let hasta = NaiveDate::parse_from_str(&hoy, "%Y-%m-%d");

    match (desde, hasta) {
        (Ok(d), Ok(h)) => {
            let dias = (h - d).num_days().max(0);
            dias <= DIAS_GRACIA_OFFLINE
        }
        _ => false,
    }
}
