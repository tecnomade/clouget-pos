//! Módulo App Móvil — base para `clouget-pos-app` (repo aparte).
//!
//! Este módulo va a alojar en Sprint 3:
//! - Endpoints HTTP `/api/v1/app/*` (auth PIN, catálogo, ventas, mesas, cocina, etc.)
//! - Discovery mDNS para que la app encuentre el servidor en LAN
//! - Generación de tokens de pareja (QR code) para emparejar dispositivos
//!
//! Por ahora (Sprint 2) solo expone el helper de validación de licencia que
//! usarán todos esos endpoints en runtime.
//!
//! # Activación (igual modelo que `restaurante`)
//!
//! 1. **Brand flag** (compile-time): `crate::branding::BRAND.tiene_modulo_app_movil()`.
//!    Por ahora siempre `true` — el módulo es transversal a Clouget y DigitalServer.
//! 2. **Licencia** (runtime): la licencia debe incluir `"app_movil"` en `modulos`.
//!    Validado por [`requiere_modulo_app_movil`].
//!
//! # Diferencia con `restaurante`
//!
//! - `restaurante` = mesas, comandas, cocina, dividir cuenta, unir mesas
//! - `app_movil`   = la app móvil en sí (puede usarse con O sin restaurante)
//!
//! Esto permite los 4 combos de licencia:
//! - `[]`                                  POS básico (perpetua)
//! - `["restaurante"]`                     POS + restaurante (sin app)
//! - `["app_movil"]`                       POS + app (vendedor piso, inventarista)
//! - `["restaurante", "app_movil"]`        POS + restaurante + app (caso completo)

pub mod schema;
pub mod http;
pub mod http_st;
pub mod commands;
pub mod discovery;
pub mod push;

use crate::db::Database;
use rusqlite::params;

/// Inicializa el módulo: corre migraciones SQL.
/// Llamar desde `lib.rs` durante el setup de la app.
pub fn init(db: &Database) -> Result<(), rusqlite::Error> {
    let conn = db.conn.lock().unwrap();
    schema::create_tables(&conn)?;
    Ok(())
}

/// Verifica que la licencia activa tenga el módulo `"app_movil"`.
///
/// Devuelve `Ok(())` si está activo, o `Err` con mensaje listo para propagar
/// al frontend o al cliente HTTP de la app móvil.
///
/// Idéntico patrón que `restaurante::requiere_modulo_restaurante` — los
/// endpoints HTTP de la app llamarán este helper como primera línea para
/// rechazar requests si la licencia no lo incluye.
#[allow(dead_code)] // Se empieza a usar en Sprint 3
pub fn requiere_modulo_app_movil(db: &Database) -> Result<(), String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;

    let modulos_json: String = conn
        .query_row(
            "SELECT value FROM config WHERE key = 'licencia_modulos'",
            params![],
            |row| row.get(0),
        )
        .unwrap_or_default();

    if modulos_json.is_empty() {
        return Err("Módulo App Móvil no incluido en su licencia".to_string());
    }

    let modulos: Vec<String> = serde_json::from_str(&modulos_json).unwrap_or_default();
    if !modulos.iter().any(|m| m == "app_movil") {
        return Err(
            "Módulo App Móvil no incluido en su licencia. Contacte a soporte para activarlo."
                .to_string(),
        );
    }

    Ok(())
}
