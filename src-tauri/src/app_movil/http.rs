//! Endpoints HTTP del módulo App Móvil.
//!
//! Servidos en el mismo `axum` server que Multi-POS, montados bajo el prefijo
//! `/api/v1/app/*`. Usa el mismo `ServerState` (DB y SesionState compartidos)
//! pero con su PROPIO esquema de auth: en vez del token único de Multi-POS,
//! cada request lleva un token de la tabla `app_tokens` (uno por dispositivo).
//!
//! Rutas implementadas en Sprint 3a:
//! - `POST /api/v1/app/auth/pin`     — login con PIN, devuelve token
//! - `POST /api/v1/app/auth/logout`  — revoca el token actual
//! - `GET  /api/v1/app/me`           — usuario + permisos del token actual
//! - `GET  /api/v1/app/productos`    — catálogo (búsqueda opcional `?q=`)
//! - `GET  /api/v1/app/mesas`        — grid de mesas (requiere atiende_mesas o ve_cocina)
//! - `GET  /api/v1/app/ping`         — sin auth, prueba conectividad
//!
//! Próximos sprints (3b/3c):
//! - Endpoints de pedidos completos (abrir, items, cocina, dividir, unir, cobrar)
//! - mDNS discovery
//! - QR de emparejamiento
//!
//! # Auth flow
//!
//! 1. App: `POST /api/v1/app/auth/pin` con `{ usuario_id, pin, dispositivo_nombre, dispositivo_modelo, dispositivo_so }`
//! 2. Servidor: valida PIN contra `usuarios.pin_hash`, valida que el usuario tenga
//!    al menos un permiso de app (atiende_mesas, ve_cocina, vende_piso, inventaria, dueno_dashboard)
//!    o sea ADMIN, genera UUID v4, persiste en `app_tokens`, devuelve `{ token, usuario, permisos }`
//! 3. App: guarda token en `expo-secure-store` y lo manda en header `Authorization: Bearer <token>`
//! 4. Servidor: middleware `extract_app_session` valida el token en cada request

use crate::server::state::ServerState;
use axum::{
    extract::{Query, State as AxumState},
    http::{HeaderMap, StatusCode},
    routing::{get, post},
    Json, Router,
};
use rusqlite::params;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

// ─── Tipos request/response ──────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct LoginPinRequest {
    pub usuario_id: i64,
    pub pin: String,
    /// Nombre amigable del dispositivo (lo escribe el usuario o lo genera la app)
    pub dispositivo_nombre: Option<String>,
    /// Marca/modelo del dispositivo (Expo Constants)
    pub dispositivo_modelo: Option<String>,
    pub dispositivo_so: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct LoginPinResponse {
    pub token: String,
    pub usuario_id: i64,
    pub nombre: String,
    pub rol: String,
    pub permisos: Vec<String>,
    /// `true` si el usuario es ADMIN (bypass de permisos en frontend)
    pub es_admin: bool,
}

#[derive(Debug, Serialize)]
pub struct MeResponse {
    pub usuario_id: i64,
    pub nombre: String,
    pub rol: String,
    pub permisos: Vec<String>,
    pub es_admin: bool,
    pub negocio: String,
    pub modulos_licencia: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct ApiError {
    pub ok: bool,
    pub error: String,
}

impl ApiError {
    fn new(msg: impl Into<String>) -> Self {
        Self { ok: false, error: msg.into() }
    }
}

// ─── Sesión de app (resultado del middleware) ────────────────────────────

#[derive(Clone)]
pub struct AppSession {
    pub usuario_id: i64,
    pub nombre: String,
    pub rol: String,
    pub permisos: Vec<String>,
}

impl AppSession {
    /// Helper: ¿el usuario tiene este permiso? (ADMIN bypassa todo)
    pub fn tiene(&self, permiso: &str) -> bool {
        self.rol == "ADMIN" || self.permisos.iter().any(|p| p == permiso)
    }

    /// Helper para handlers: rechaza con 403 si no tiene el permiso
    pub fn requiere(&self, permiso: &str) -> Result<(), (StatusCode, Json<ApiError>)> {
        if self.tiene(permiso) {
            Ok(())
        } else {
            Err((
                StatusCode::FORBIDDEN,
                Json(ApiError::new(format!("Falta permiso: {}", permiso))),
            ))
        }
    }
}

/// Extrae token del header `Authorization: Bearer <token>` y valida contra DB.
/// Llamado al inicio de cada handler protegido.
pub fn extract_app_session(
    headers: &HeaderMap,
    state: &Arc<ServerState>,
) -> Result<AppSession, (StatusCode, Json<ApiError>)> {
    // 1. Validar licencia primero — si no tiene `app_movil` rechazamos en bloque
    if let Err(msg) = super::requiere_modulo_app_movil(&state.db) {
        return Err((StatusCode::FORBIDDEN, Json(ApiError::new(msg))));
    }

    // 2. Extraer header
    let auth = headers
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    let token = auth.strip_prefix("Bearer ").unwrap_or("").trim();
    if token.is_empty() {
        return Err((
            StatusCode::UNAUTHORIZED,
            Json(ApiError::new("Falta header Authorization: Bearer <token>")),
        ));
    }

    // 3. Buscar token en DB y traer datos del usuario en una sola query
    let conn = state.db.conn.lock().map_err(|e| {
        (StatusCode::INTERNAL_SERVER_ERROR, Json(ApiError::new(e.to_string())))
    })?;

    let row: Result<(i64, String, String, String, i64), rusqlite::Error> = conn.query_row(
        "SELECT u.id, u.nombre, u.rol, COALESCE(u.permisos, '{}') AS permisos,
                t.id AS token_id
         FROM app_tokens t
         JOIN usuarios u ON t.usuario_id = u.id
         WHERE t.token = ?1
           AND t.revoked = 0
           AND u.activo = 1",
        params![token],
        |r| {
            Ok((
                r.get::<_, i64>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, String>(2)?,
                r.get::<_, String>(3)?,
                r.get::<_, i64>(4)?,
            ))
        },
    );

    let (usuario_id, nombre, rol, permisos_json, token_id) = row.map_err(|_| {
        (
            StatusCode::UNAUTHORIZED,
            Json(ApiError::new("Token inválido o revocado")),
        )
    })?;

    // 4. Tocar last_used_at (best-effort, no falla la request si no se puede)
    let _ = conn.execute(
        "UPDATE app_tokens SET last_used_at = datetime('now','localtime') WHERE id = ?1",
        params![token_id],
    );

    // 5. Parsear permisos JSON → Vec<String>
    let permisos: Vec<String> = serde_json::from_str::<serde_json::Value>(&permisos_json)
        .ok()
        .and_then(|v| {
            v.as_object().map(|map| {
                map.iter()
                    .filter(|(_, v)| v.as_bool().unwrap_or(false))
                    .map(|(k, _)| k.clone())
                    .collect()
            })
        })
        .unwrap_or_default();

    Ok(AppSession {
        usuario_id,
        nombre,
        rol,
        permisos,
    })
}

// ─── Handlers ────────────────────────────────────────────────────────────

/// `GET /api/v1/app/ping` — sin auth. Para que la app verifique conectividad
/// y validar que apuntó al servidor correcto. Devuelve algo identificable.
pub async fn ping(
    AxumState(state): AxumState<Arc<ServerState>>,
) -> Json<serde_json::Value> {
    let conn = state.db.conn.lock().ok();
    let nombre_negocio = conn
        .as_ref()
        .and_then(|c| {
            c.query_row(
                "SELECT value FROM config WHERE key = 'nombre_negocio'",
                [],
                |r| r.get::<_, String>(0),
            )
            .ok()
        })
        .unwrap_or_else(|| "Clouget POS".to_string());

    let modulos_json = conn
        .as_ref()
        .and_then(|c| {
            c.query_row(
                "SELECT value FROM config WHERE key = 'licencia_modulos'",
                [],
                |r| r.get::<_, String>(0),
            )
            .ok()
        })
        .unwrap_or_default();
    let modulos: Vec<String> = serde_json::from_str(&modulos_json).unwrap_or_default();
    let app_movil_activo = modulos.iter().any(|m| m == "app_movil");

    Json(serde_json::json!({
        "ok": true,
        "service": "clouget-pos",
        "negocio": nombre_negocio,
        "app_movil_activo": app_movil_activo,
        "version": env!("CARGO_PKG_VERSION"),
    }))
}

/// `POST /api/v1/app/auth/pin` — login con PIN, devuelve token.
pub async fn auth_pin(
    AxumState(state): AxumState<Arc<ServerState>>,
    Json(req): Json<LoginPinRequest>,
) -> Result<Json<LoginPinResponse>, (StatusCode, Json<ApiError>)> {
    // 1. Validar licencia
    if let Err(msg) = super::requiere_modulo_app_movil(&state.db) {
        return Err((StatusCode::FORBIDDEN, Json(ApiError::new(msg))));
    }

    let pin = req.pin.trim();
    if pin.len() < 4 {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ApiError::new("PIN debe tener al menos 4 caracteres")),
        ));
    }

    let conn = state.db.conn.lock().map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiError::new(e.to_string())),
        )
    })?;

    // 2. Buscar usuario y validar PIN
    let row: Result<(String, String, String, String, String), rusqlite::Error> = conn
        .query_row(
            "SELECT nombre, pin_hash, pin_salt, rol, COALESCE(permisos, '{}')
             FROM usuarios WHERE id = ?1 AND activo = 1",
            params![req.usuario_id],
            |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, String>(2)?,
                    r.get::<_, String>(3)?,
                    r.get::<_, String>(4)?,
                ))
            },
        );

    let (nombre, pin_hash, pin_salt, rol, permisos_json) = row.map_err(|_| {
        (
            StatusCode::UNAUTHORIZED,
            Json(ApiError::new("Usuario no encontrado o inactivo")),
        )
    })?;

    // 3. Verificar PIN (mismo método que el comando local de login)
    let pin_calc = crate::utils::hash_pin(&pin_salt, pin);
    if pin_calc != pin_hash {
        return Err((
            StatusCode::UNAUTHORIZED,
            Json(ApiError::new("PIN incorrecto")),
        ));
    }

    // 4. Verificar que el usuario tenga al menos un permiso de app O sea ADMIN
    let permisos: Vec<String> = serde_json::from_str::<serde_json::Value>(&permisos_json)
        .ok()
        .and_then(|v| {
            v.as_object().map(|map| {
                map.iter()
                    .filter(|(_, v)| v.as_bool().unwrap_or(false))
                    .map(|(k, _)| k.clone())
                    .collect()
            })
        })
        .unwrap_or_default();

    let es_admin = rol == "ADMIN";
    let permisos_app = [
        "atiende_mesas",
        "ve_cocina",
        "vende_piso",
        "inventaria",
        "dueno_dashboard",
        "cobra_caja",
    ];
    let tiene_permiso_app = es_admin || permisos.iter().any(|p| permisos_app.contains(&p.as_str()));
    if !tiene_permiso_app {
        return Err((
            StatusCode::FORBIDDEN,
            Json(ApiError::new(
                "Este usuario no tiene permisos para usar la app móvil. \
                 Pídele al admin que le asigne al menos uno: atiende_mesas, ve_cocina, \
                 vende_piso, inventaria, dueno_dashboard.",
            )),
        ));
    }

    // 5. Generar token (UUID v4) y persistir
    let token = uuid::Uuid::new_v4().to_string();
    conn.execute(
        "INSERT INTO app_tokens
         (usuario_id, token, dispositivo_nombre, dispositivo_modelo, dispositivo_so)
         VALUES (?1, ?2, ?3, ?4, ?5)",
        params![
            req.usuario_id,
            token,
            req.dispositivo_nombre,
            req.dispositivo_modelo,
            req.dispositivo_so
        ],
    )
    .map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiError::new(format!("Error guardando token: {}", e))),
        )
    })?;

    Ok(Json(LoginPinResponse {
        token,
        usuario_id: req.usuario_id,
        nombre,
        rol,
        permisos,
        es_admin,
    }))
}

/// `POST /api/v1/app/auth/logout` — revoca el token actual.
pub async fn auth_logout(
    AxumState(state): AxumState<Arc<ServerState>>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ApiError>)> {
    let auth = headers
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    let token = auth.strip_prefix("Bearer ").unwrap_or("").trim();

    if !token.is_empty() {
        let conn = state.db.conn.lock().map_err(|e| {
            (StatusCode::INTERNAL_SERVER_ERROR, Json(ApiError::new(e.to_string())))
        })?;
        let _ = conn.execute(
            "UPDATE app_tokens SET revoked = 1 WHERE token = ?1",
            params![token],
        );
    }

    Ok(Json(serde_json::json!({ "ok": true })))
}

/// `GET /api/v1/app/me` — datos del usuario logueado.
pub async fn me(
    AxumState(state): AxumState<Arc<ServerState>>,
    headers: HeaderMap,
) -> Result<Json<MeResponse>, (StatusCode, Json<ApiError>)> {
    let session = extract_app_session(&headers, &state)?;

    // Datos extra (negocio + módulos de licencia) para que la app sepa qué mostrar
    let conn = state.db.conn.lock().map_err(|e| {
        (StatusCode::INTERNAL_SERVER_ERROR, Json(ApiError::new(e.to_string())))
    })?;
    let negocio: String = conn
        .query_row(
            "SELECT value FROM config WHERE key = 'nombre_negocio'",
            [],
            |r| r.get(0),
        )
        .unwrap_or_else(|_| "Clouget POS".to_string());
    let modulos_json: String = conn
        .query_row(
            "SELECT value FROM config WHERE key = 'licencia_modulos'",
            [],
            |r| r.get(0),
        )
        .unwrap_or_default();
    let modulos: Vec<String> = serde_json::from_str(&modulos_json).unwrap_or_default();

    let es_admin = session.rol == "ADMIN";
    Ok(Json(MeResponse {
        usuario_id: session.usuario_id,
        nombre: session.nombre,
        rol: session.rol,
        permisos: session.permisos,
        es_admin,
        negocio,
        modulos_licencia: modulos,
    }))
}

#[derive(Debug, Deserialize)]
pub struct ListarProductosQuery {
    pub q: Option<String>,
    pub limite: Option<i64>,
}

/// `GET /api/v1/app/productos?q=&limite=` — catálogo simple para la app.
/// Devuelve productos activos. Si `q` viene, filtra por nombre/codigo/codigo_barras.
pub async fn listar_productos(
    AxumState(state): AxumState<Arc<ServerState>>,
    headers: HeaderMap,
    Query(qp): Query<ListarProductosQuery>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ApiError>)> {
    let _session = extract_app_session(&headers, &state)?;

    let limite = qp.limite.unwrap_or(200).clamp(1, 1000);
    let conn = state.db.conn.lock().map_err(|e| {
        (StatusCode::INTERNAL_SERVER_ERROR, Json(ApiError::new(e.to_string())))
    })?;

    let busqueda = qp.q.unwrap_or_default();
    let busqueda_pat = format!("%{}%", busqueda);

    let (sql, params_dyn): (&str, Vec<Box<dyn rusqlite::ToSql>>) = if busqueda.is_empty() {
        (
            "SELECT p.id, p.codigo, p.codigo_barras, p.nombre, p.precio_venta,
                    p.iva_porcentaje, COALESCE(p.incluye_iva, 0) AS incluye_iva,
                    p.stock_actual, COALESCE(c.nombre, '') AS categoria
             FROM productos p
             LEFT JOIN categorias c ON p.categoria_id = c.id
             WHERE p.activo = 1
             ORDER BY p.nombre
             LIMIT ?1",
            vec![Box::new(limite)],
        )
    } else {
        (
            "SELECT p.id, p.codigo, p.codigo_barras, p.nombre, p.precio_venta,
                    p.iva_porcentaje, COALESCE(p.incluye_iva, 0) AS incluye_iva,
                    p.stock_actual, COALESCE(c.nombre, '') AS categoria
             FROM productos p
             LEFT JOIN categorias c ON p.categoria_id = c.id
             WHERE p.activo = 1
               AND (p.nombre LIKE ?1 OR p.codigo LIKE ?1 OR p.codigo_barras LIKE ?1)
             ORDER BY p.nombre
             LIMIT ?2",
            vec![Box::new(busqueda_pat), Box::new(limite)],
        )
    };

    let mut stmt = conn.prepare(sql).map_err(|e| {
        (StatusCode::INTERNAL_SERVER_ERROR, Json(ApiError::new(e.to_string())))
    })?;
    let params_refs: Vec<&dyn rusqlite::ToSql> = params_dyn.iter().map(|b| b.as_ref()).collect();

    let productos: Vec<serde_json::Value> = stmt
        .query_map(rusqlite::params_from_iter(params_refs.iter()), |r| {
            Ok(serde_json::json!({
                "id": r.get::<_, i64>(0)?,
                "codigo": r.get::<_, Option<String>>(1)?,
                "codigo_barras": r.get::<_, Option<String>>(2)?,
                "nombre": r.get::<_, String>(3)?,
                "precio_venta": r.get::<_, f64>(4)?,
                "iva_porcentaje": r.get::<_, f64>(5)?,
                "incluye_iva": r.get::<_, i64>(6)? != 0,
                "stock_actual": r.get::<_, f64>(7)?,
                "categoria": r.get::<_, String>(8)?,
            }))
        })
        .map_err(|e| {
            (StatusCode::INTERNAL_SERVER_ERROR, Json(ApiError::new(e.to_string())))
        })?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| {
            (StatusCode::INTERNAL_SERVER_ERROR, Json(ApiError::new(e.to_string())))
        })?;

    Ok(Json(serde_json::json!({
        "ok": true,
        "productos": productos,
        "total": productos.len(),
    })))
}

/// `GET /api/v1/app/mesas` — grid de mesas con estado para mesero/cocinero.
/// Requiere permiso `atiende_mesas` o `ve_cocina` (o ADMIN).
pub async fn listar_mesas(
    AxumState(state): AxumState<Arc<ServerState>>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ApiError>)> {
    let session = extract_app_session(&headers, &state)?;

    if !session.tiene("atiende_mesas") && !session.tiene("ve_cocina") {
        return Err((
            StatusCode::FORBIDDEN,
            Json(ApiError::new(
                "Falta permiso atiende_mesas o ve_cocina".to_string(),
            )),
        ));
    }

    // Validar también que el módulo restaurante esté activo
    if let Err(msg) = crate::restaurante::requiere_modulo_restaurante(&state.db) {
        return Err((StatusCode::FORBIDDEN, Json(ApiError::new(msg))));
    }

    // Reusar el comando interno del POS escritorio (devuelve Vec<MesaConEstado>)
    // Lo invocamos vía dispatch para no duplicar la query gigante.
    let payload = crate::server::dispatch::dispatch_command(
        &state,
        "rest_listar_mesas_con_estado",
        serde_json::json!({}),
    )
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(ApiError::new(e))))?;

    Ok(Json(serde_json::json!({
        "ok": true,
        "mesas": payload,
    })))
}

// ─── Router builder ──────────────────────────────────────────────────────

/// Devuelve el router con todas las rutas del módulo, listo para `merge` en
/// el server principal.
pub fn rutas() -> Router<Arc<ServerState>> {
    Router::new()
        // Sin auth
        .route("/api/v1/app/ping", get(ping))
        .route("/api/v1/app/auth/pin", post(auth_pin))
        .route("/api/v1/app/auth/logout", post(auth_logout))
        // Con auth
        .route("/api/v1/app/me", get(me))
        .route("/api/v1/app/productos", get(listar_productos))
        .route("/api/v1/app/mesas", get(listar_mesas))
}
