pub mod dispatch;
pub mod state;

use crate::db::{Database, SesionState};
use axum::{
    extract::State as AxumState,
    http::{HeaderMap, StatusCode},
    routing::post,
    Json, Router,
};
use state::ServerState;
use std::sync::Arc;
use tower_http::cors::{Any, CorsLayer};

/// Inicia el servidor HTTP embebido para multi-POS en red.
/// Se ejecuta en un thread separado con su propio runtime tokio.
/// Recibe clones de Database y SesionState que comparten la misma conexión.
pub fn start_server(db: Database, sesion: SesionState, port: u16, token: String) {
    std::thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new().expect("Failed to create tokio runtime for server");
        rt.block_on(async move {
            let state = ServerState {
                db,
                sesion,
                token,
            };

            // CORS: la app móvil corre en otro origen (Expo Dev en :8081, app
            // empacada en el dispositivo). Permitimos cualquier origen porque
            // el servidor está en LAN privada y la auth la garantiza el token.
            let cors = CorsLayer::new()
                .allow_origin(Any)
                .allow_methods(Any)
                .allow_headers(Any);

            let app = Router::new()
                .route("/api/v1/invoke", post(handle_invoke))
                .route("/api/v1/ping", axum::routing::get(handle_ping))
                // v2.4.2 — Sprint 3a: rutas de la app móvil mergeadas
                .merge(crate::app_movil::http::rutas())
                .layer(cors)
                .with_state(Arc::new(state));

            let addr = format!("0.0.0.0:{}", port);
            eprintln!("[Clouget Server] Iniciando servidor en {}", addr);

            let listener = tokio::net::TcpListener::bind(&addr)
                .await
                .expect(&format!("No se pudo iniciar servidor en puerto {}", port));

            eprintln!("[Clouget Server] Servidor activo en puerto {}", port);

            axum::serve(listener, app)
                .await
                .expect("Server error");
        });
    });
}

/// Ping endpoint para verificar conectividad
async fn handle_ping() -> &'static str {
    "clouget-pos-server"
}

/// Request body para invocación remota
#[derive(serde::Deserialize)]
struct InvokeRequest {
    command: String,
    #[serde(default)]
    args: serde_json::Value,
}

/// Response body
#[derive(serde::Serialize)]
struct InvokeResponse {
    ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    data: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
}

/// Handler principal: recibe comando + args, valida token, despacha
async fn handle_invoke(
    AxumState(state): AxumState<Arc<ServerState>>,
    headers: HeaderMap,
    Json(req): Json<InvokeRequest>,
) -> (StatusCode, Json<InvokeResponse>) {
    // Validar token
    let auth = headers
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    let expected = format!("Bearer {}", state.token);
    if auth != expected {
        return (
            StatusCode::UNAUTHORIZED,
            Json(InvokeResponse {
                ok: false,
                data: None,
                error: Some("Token inválido".to_string()),
            }),
        );
    }

    // Despachar comando
    match dispatch::dispatch_command(&state, &req.command, req.args).await {
        Ok(data) => (
            StatusCode::OK,
            Json(InvokeResponse {
                ok: true,
                data: Some(data),
                error: None,
            }),
        ),
        Err(err) => (
            StatusCode::OK,
            Json(InvokeResponse {
                ok: false,
                data: None,
                error: Some(err),
            }),
        ),
    }
}
