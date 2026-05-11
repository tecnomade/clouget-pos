//! Cliente de Expo Push Notifications.
//!
//! Cuando un pedido se envía a cocina, queremos avisar a los cocineros
//! conectados con la app móvil (que tienen permiso `ve_cocina`).
//!
//! Flujo:
//!  1. Buscamos en `app_tokens` los `push_token` de usuarios con `ve_cocina`.
//!  2. Para cada token, llamamos a `https://exp.host/--/api/v2/push/send`.
//!  3. Expo se encarga del routing a FCM (Android) o APNs (iOS).
//!
//! El envío es **fire-and-forget en background** — no bloqueamos el
//! request HTTP del mesero esperando que llegue la push.

use crate::db::Database;
use rusqlite::params;
use serde::{Deserialize, Serialize};

const EXPO_PUSH_URL: &str = "https://exp.host/--/api/v2/push/send";

#[derive(Debug, Serialize)]
struct ExpoPushMessage<'a> {
    to: &'a str,
    title: &'a str,
    body: &'a str,
    sound: &'a str,
    priority: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    data: Option<serde_json::Value>,
    /// Channel ID en Android (configurado en la app)
    #[serde(rename = "channelId", skip_serializing_if = "Option::is_none")]
    channel_id: Option<&'a str>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct ExpoPushReceipt {
    status: String,
    message: Option<String>,
}

/// Devuelve los push tokens de usuarios activos (no revocados) con un permiso dado.
/// Por ejemplo: `ve_cocina` para avisar a cocineros, `atiende_mesas` para meseros.
pub fn tokens_por_permiso(db: &Database, permiso: &str) -> Result<Vec<String>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let mut stmt = conn
        .prepare(
            "SELECT DISTINCT t.push_token
             FROM app_tokens t
             JOIN usuarios u ON u.id = t.usuario_id
             WHERE t.revoked = 0
               AND t.push_token IS NOT NULL
               AND t.push_token != ''
               AND (
                 u.es_admin = 1
                 OR EXISTS (
                   SELECT 1 FROM usuario_permisos up
                   WHERE up.usuario_id = u.id AND up.permiso = ?1
                 )
               )",
        )
        .map_err(|e| e.to_string())?;
    let tokens: Vec<String> = stmt
        .query_map(params![permiso], |r| r.get(0))
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();
    Ok(tokens)
}

/// Envía una push notification a uno o más tokens. **Fire-and-forget**:
/// se ejecuta en un task de Tokio; los errores se loggean pero no se propagan.
pub fn enviar_push_async(
    tokens: Vec<String>,
    title: String,
    body: String,
    data: Option<serde_json::Value>,
) {
    if tokens.is_empty() {
        return;
    }
    tokio::spawn(async move {
        if let Err(e) = enviar_push(&tokens, &title, &body, data).await {
            eprintln!("[push] Error enviando notificación: {}", e);
        }
    });
}

async fn enviar_push(
    tokens: &[String],
    title: &str,
    body: &str,
    data: Option<serde_json::Value>,
) -> Result<(), String> {
    let messages: Vec<ExpoPushMessage> = tokens
        .iter()
        .map(|t| ExpoPushMessage {
            to: t,
            title,
            body,
            sound: "default",
            priority: "high",
            data: data.clone(),
            channel_id: Some("cocina"), // canal con vibración + sonido en la app
        })
        .collect();

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|e| e.to_string())?;

    let res = client
        .post(EXPO_PUSH_URL)
        .header("Accept", "application/json")
        .header("Accept-encoding", "gzip, deflate")
        .header("Content-Type", "application/json")
        .json(&messages)
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if !res.status().is_success() {
        let status = res.status();
        let body_txt = res.text().await.unwrap_or_default();
        return Err(format!("Expo Push API HTTP {}: {}", status, body_txt));
    }

    Ok(())
}
