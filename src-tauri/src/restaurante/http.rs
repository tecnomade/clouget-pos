//! Endpoints HTTP del módulo Restaurante para la app móvil de meseros.
//!
//! **Estado actual: STUB (Fase 1).**
//!
//! En Fase 3 implementaremos aquí:
//!
//! - `POST /api/v1/rest/auth/pin`           → mesero login con PIN, devuelve token
//! - `GET  /api/v1/rest/mesas`              → grid de mesas con estado
//! - `POST /api/v1/rest/pedidos/abrir`      → abre pedido en una mesa
//! - `POST /api/v1/rest/pedidos/:id/items`  → agrega item
//! - `POST /api/v1/rest/pedidos/:id/cocina` → envía a cocina (imprime ticket)
//! - `POST /api/v1/rest/pedidos/:id/cuenta` → marca cuenta pedida
//! - `GET  /api/v1/rest/cocina/items`       → vista cocina (TV/tablet)
//! - `POST /api/v1/rest/cocina/items/:id`   → cambia estado cocina
//!
//! Todos los endpoints requerirán:
//! 1. `branding::BRAND.tiene_app_movil_meseros()` (compile-time)
//! 2. Header `Authorization: Bearer <token-mesero>` (runtime)
//! 3. Licencia con módulo `restaurante` activo
//!
//! Discovery: la app móvil encontrará el servidor automáticamente via mDNS
//! (servicio `_clouget-pos._tcp.local.`) — implementación pendiente Fase 3.

// Cuando arranquemos Fase 3:
// use axum::{routing::{get, post}, Router};
// use std::sync::Arc;
// use crate::server::state::ServerState;
//
// pub fn rutas() -> Router<Arc<ServerState>> {
//     Router::new()
//         .route("/api/v1/rest/auth/pin", post(auth_pin))
//         .route("/api/v1/rest/mesas", get(listar_mesas))
//         // ... etc
// }
