//! v2.5.53 — Cuentas OAuth Gmail per-cliente para envío de emails.
//!
//! Modelo: cada POS conecta SU PROPIA cuenta Gmail desde Configuración. El
//! refresh_token se guarda LOCAL en SQLite. Cuando se va a enviar un email,
//! `commands::sri::enviar_email_interno` consulta primero si hay alguna
//! cuenta OAuth activa aquí, y si la hay envía con ella vía el endpoint
//! `/enviar-email-oauth` de email.clouget.com (que no guarda nada, sólo
//! usa los tokens al vuelo).
//!
//! Beneficios vs cuentas centralizadas:
//!   - El cliente final recibe email DESDE el negocio que le vendió,
//!     no desde notificaciones@clouget.com
//!   - Mejor reputación / entregabilidad (dominio del propio negocio)
//!   - Escala sin límite (cada cliente con su Gmail)
//!   - Privacidad: nosotros no vemos los emails

use crate::db::Database;
use rusqlite::params;
use serde::{Deserialize, Serialize};
use tauri::State;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct OauthEmailCuenta {
    pub id: i64,
    pub proveedor: String,
    pub email: String,
    pub from_name: Option<String>,
    pub activa: bool,
    pub created_at: String,
    // refresh_token NUNCA se expone al frontend (solo se usa internamente)
}

#[derive(Debug, Deserialize)]
pub struct GuardarOauthCuenta {
    pub proveedor: Option<String>, // default "gmail"
    pub email: String,
    pub refresh_token: String,
    pub from_name: Option<String>,
}

/// Lista cuentas OAuth email configuradas (sin exponer refresh_token).
#[tauri::command]
pub fn listar_oauth_email_cuentas(
    db: State<'_, Database>,
) -> Result<Vec<OauthEmailCuenta>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let mut stmt = conn
        .prepare(
            "SELECT id, proveedor, email, from_name, activa, created_at
             FROM oauth_email_cuentas ORDER BY id",
        )
        .map_err(|e| e.to_string())?;
    let rows: Vec<OauthEmailCuenta> = stmt
        .query_map([], |r| {
            Ok(OauthEmailCuenta {
                id: r.get(0)?,
                proveedor: r.get(1)?,
                email: r.get(2)?,
                from_name: r.get(3).ok(),
                activa: r.get::<_, i32>(4)? != 0,
                created_at: r.get(5)?,
            })
        })
        .map_err(|e| e.to_string())?
        .filter_map(Result::ok)
        .collect();
    Ok(rows)
}

/// Persiste una nueva cuenta OAuth (típicamente llamado desde el handler del
/// deep link OAuth callback con los datos que vienen de Google). Idempotente:
/// si ya existe el email, actualiza el refresh_token.
#[tauri::command]
pub fn guardar_oauth_email_cuenta(
    db: State<'_, Database>,
    cuenta: GuardarOauthCuenta,
) -> Result<i64, String> {
    let proveedor = cuenta.proveedor.unwrap_or_else(|| "gmail".to_string());
    if cuenta.email.is_empty() || cuenta.refresh_token.is_empty() {
        return Err("email y refresh_token son obligatorios".into());
    }
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    // INSERT OR REPLACE para idempotencia (actualiza si existe el par proveedor+email)
    conn.execute(
        "INSERT INTO oauth_email_cuentas (proveedor, email, refresh_token, from_name, activa)
         VALUES (?1, ?2, ?3, ?4, 1)
         ON CONFLICT(proveedor, email) DO UPDATE SET
            refresh_token = excluded.refresh_token,
            from_name = COALESCE(excluded.from_name, oauth_email_cuentas.from_name),
            activa = 1",
        params![proveedor, cuenta.email, cuenta.refresh_token, cuenta.from_name],
    ).map_err(|e| format!("Error guardando cuenta OAuth: {}", e))?;
    let id: i64 = conn.query_row(
        "SELECT id FROM oauth_email_cuentas WHERE proveedor = ?1 AND email = ?2",
        params![proveedor, cuenta.email],
        |r| r.get(0),
    ).map_err(|e| e.to_string())?;
    eprintln!("[OAuth-Email] Cuenta {} guardada/actualizada (id {})", cuenta.email, id);
    Ok(id)
}

/// Elimina (o desactiva) una cuenta OAuth.
#[tauri::command]
pub fn eliminar_oauth_email_cuenta(
    db: State<'_, Database>,
    id: i64,
) -> Result<(), String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    conn.execute("DELETE FROM oauth_email_cuentas WHERE id = ?1", params![id])
        .map_err(|e| e.to_string())?;
    Ok(())
}

/// Activa o desactiva una cuenta OAuth (sin borrarla).
#[tauri::command]
pub fn toggle_oauth_email_cuenta(
    db: State<'_, Database>,
    id: i64,
    activa: bool,
) -> Result<(), String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    conn.execute(
        "UPDATE oauth_email_cuentas SET activa = ?1 WHERE id = ?2",
        params![activa as i32, id],
    ).map_err(|e| e.to_string())?;
    Ok(())
}

/// Inicia el flow OAuth abriendo el navegador del SO con la URL de
/// email.clouget.com/oauth/cliente/init. El callback de Google llama a
/// email.clouget.com/oauth/google/callback?state=cliente que muestra
/// página de éxito + dispara deep link `clouget://oauth-email-callback?...`
/// que reabre Clouget POS y emite evento "deep-link://new-url" al frontend.
#[tauri::command]
pub fn iniciar_oauth_email_gmail(db: State<'_, Database>) -> Result<String, String> {
    // Leer URL del servicio email desde config (mismo que se usa para enviar email)
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let email_url: String = conn
        .query_row(
            "SELECT value FROM config WHERE key = 'email_service_url'",
            [],
            |r| r.get(0),
        )
        .unwrap_or_else(|_| "https://email.clouget.com".to_string());
    drop(conn);

    let oauth_url = format!("{}/oauth/cliente/init", email_url.trim_end_matches('/'));

    // Abrir en navegador del SO
    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("cmd")
            .args(&["/C", "start", "", &oauth_url])
            .spawn()
            .map_err(|e| format!("No se pudo abrir navegador: {}", e))?;
    }
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg(&oauth_url)
            .spawn()
            .map_err(|e| format!("No se pudo abrir navegador: {}", e))?;
    }
    #[cfg(target_os = "linux")]
    {
        std::process::Command::new("xdg-open")
            .arg(&oauth_url)
            .spawn()
            .map_err(|e| format!("No se pudo abrir navegador: {}", e))?;
    }

    Ok(oauth_url)
}

// ─── Helpers usados por commands::sri::enviar_email_interno ──────────────────

/// Devuelve la primera cuenta OAuth activa (con su refresh_token) si existe.
/// Si no hay ninguna activa, devuelve None y el envío usa el flow tradicional.
pub fn obtener_cuenta_oauth_activa(db: &Database) -> Option<CuentaOauthEnvio> {
    let conn = db.conn.lock().ok()?;
    conn.query_row(
        "SELECT email, refresh_token, from_name FROM oauth_email_cuentas
         WHERE activa = 1 ORDER BY id LIMIT 1",
        [],
        |r| {
            Ok(CuentaOauthEnvio {
                email: r.get(0)?,
                refresh_token: r.get(1)?,
                from_name: r.get(2).ok(),
            })
        },
    )
    .ok()
}

#[derive(Debug, Clone)]
pub struct CuentaOauthEnvio {
    pub email: String,
    pub refresh_token: String,
    pub from_name: Option<String>,
}
