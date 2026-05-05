//! Módulo Restaurante — gestión de mesas, comandas, cocina y endpoints
//! HTTP para la app móvil de meseros (`clouget-mesero`, React Native + Expo).
//!
//! # Activación
//!
//! Para que un cliente vea estas features:
//!
//! 1. **Build debe ser de Clouget** (no DigitalServer) — chequeado en compile-time
//!    via `crate::branding::BRAND.tiene_modulo_restaurante()`. DigitalServer
//!    builds simplemente no llaman a `restaurante::*` desde `lib.rs`.
//! 2. **Su licencia debe tener el módulo `"restaurante"`** en `licencia.modulos`.
//!    Esto se valida en runtime en cada comando del módulo via
//!    [`requiere_modulo_restaurante`].
//!
//! # Estructura del módulo
//!
//! - [`schema`]   — migración SQL (rest_zonas, rest_mesas, rest_pedidos_abiertos, rest_pedido_items)
//! - [`models`]   — structs Rust serializables
//! - [`commands`] — comandos Tauri (CRUD + flujo de pedido)
//! - [`http`]     — endpoints HTTP para app móvil (stub en Fase 1, completo en Fase 3)

pub mod commands;
pub mod http;
pub mod models;
pub mod printing;
pub mod schema;

use crate::db::Database;
use rusqlite::params;

/// Inicializa el módulo: corre migraciones SQL.
/// Llamar SOLO desde `lib.rs` y SOLO si `branding::BRAND.tiene_modulo_restaurante()`.
pub fn init(db: &Database) -> Result<(), rusqlite::Error> {
    let conn = db.conn.lock().unwrap();
    schema::create_tables(&conn)?;
    schema::seed_default(&conn)?;
    Ok(())
}

/// Verifica que la licencia activa tenga el módulo "restaurante".
/// Si no, retorna error que se puede propagar al frontend.
///
/// Acepta licencias `"demo"` (todos los módulos en demo).
pub fn requiere_modulo_restaurante(db: &Database) -> Result<(), String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;

    let modulos_json: String = conn
        .query_row(
            "SELECT value FROM config WHERE key = 'licencia_modulos'",
            params![],
            |row| row.get(0),
        )
        .unwrap_or_default();

    if modulos_json.is_empty() {
        return Err("Módulo Restaurante no incluido en su licencia".to_string());
    }

    let modulos: Vec<String> = serde_json::from_str(&modulos_json).unwrap_or_default();
    if !modulos.iter().any(|m| m == "restaurante") {
        return Err("Módulo Restaurante no incluido en su licencia. Contacte a soporte para activarlo.".to_string());
    }

    Ok(())
}
