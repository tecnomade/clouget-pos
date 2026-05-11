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
    pub fn new(msg: impl Into<String>) -> Self {
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
    /// ID en `app_tokens` (para actualizar push_token, etc.)
    pub token_id: i64,
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
        token_id,
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

/// `GET /api/v1/app/auth/usuarios-disponibles` — lista usuarios activos
/// (id + nombre) para mostrar en el selector de login. SIN AUTH (es la
/// puerta de entrada — el user no tiene token aún). NO devuelve hashes ni
/// permisos — solo lo mínimo para presentar avatares y nombres.
///
/// Filtra a usuarios con al menos un permiso de app (atiende_mesas, ve_cocina,
/// vende_piso, inventaria, dueno_dashboard, cobra_caja) o ADMIN. Así no
/// aparecen usuarios "solo desktop" que no podrían usar la app.
pub async fn auth_usuarios_disponibles(
    AxumState(state): AxumState<Arc<ServerState>>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ApiError>)> {
    if let Err(msg) = super::requiere_modulo_app_movil(&state.db) {
        return Err((StatusCode::FORBIDDEN, Json(ApiError::new(msg))));
    }
    let conn = state.db.conn.lock().map_err(|e| {
        (StatusCode::INTERNAL_SERVER_ERROR, Json(ApiError::new(e.to_string())))
    })?;
    let mut stmt = conn
        .prepare(
            "SELECT id, nombre, rol, COALESCE(permisos, '{}') FROM usuarios WHERE activo = 1 ORDER BY nombre",
        )
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(ApiError::new(e.to_string()))))?;

    let permisos_app = ["atiende_mesas", "ve_cocina", "vende_piso", "inventaria", "dueno_dashboard", "cobra_caja"];
    let usuarios: Vec<serde_json::Value> = stmt
        .query_map([], |r| {
            Ok((
                r.get::<_, i64>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, String>(2)?,
                r.get::<_, String>(3)?,
            ))
        })
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(ApiError::new(e.to_string()))))?
        .filter_map(|res| res.ok())
        .filter_map(|(id, nombre, rol, permisos_json)| {
            // Filtrar: solo ADMIN o usuarios con al menos un permiso de app
            let es_admin = rol == "ADMIN";
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
            let tiene_app = es_admin || permisos.iter().any(|p| permisos_app.contains(&p.as_str()));
            if !tiene_app {
                return None;
            }
            Some(serde_json::json!({ "id": id, "nombre": nombre, "rol": rol, "es_admin": es_admin }))
        })
        .collect();

    Ok(Json(serde_json::json!({ "ok": true, "usuarios": usuarios })))
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

/// `POST /api/v1/app/auth/push-token` — actualiza el Expo Push Token del dispositivo
/// actual (la app lo registra al loguear y/o cuando el usuario acepta permiso de notifs).
///
/// Body: `{ "push_token": "ExponentPushToken[xxxxx]" }`
#[derive(serde::Deserialize)]
pub struct SetPushTokenReq {
    pub push_token: String,
}

pub async fn auth_set_push_token(
    AxumState(state): AxumState<Arc<ServerState>>,
    headers: HeaderMap,
    Json(req): Json<SetPushTokenReq>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ApiError>)> {
    let session = extract_app_session(&headers, &state)?;
    let conn = state.db.conn.lock().map_err(err500)?;

    let pt = req.push_token.trim();
    // Validación mínima: token de Expo empieza con ExponentPushToken[ o ExpoPushToken[
    if !pt.starts_with("ExponentPushToken[") && !pt.starts_with("ExpoPushToken[") {
        return Err(err400("Push token inválido (formato Expo)"));
    }

    conn.execute(
        "UPDATE app_tokens SET push_token = ?1 WHERE id = ?2",
        params![pt, session.token_id],
    ).map_err(err500)?;

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

    let conn = state.db.conn.lock().map_err(|e| {
        (StatusCode::INTERNAL_SERVER_ERROR, Json(ApiError::new(e.to_string())))
    })?;

    let mesas = crate::restaurante::commands::listar_mesas_con_estado_internal(&conn)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(ApiError::new(e))))?;

    Ok(Json(serde_json::json!({
        "ok": true,
        "mesas": mesas,
    })))
}

// ─── Pedidos (Sprint 3b — v2.4.3) ────────────────────────────────────────
//
// Helpers para convertir resultados a JSON con manejo de errores uniforme.

use axum::extract::Path;

pub fn err500(e: impl std::fmt::Display) -> (StatusCode, Json<ApiError>) {
    (StatusCode::INTERNAL_SERVER_ERROR, Json(ApiError::new(e.to_string())))
}
pub fn err400(e: impl std::fmt::Display) -> (StatusCode, Json<ApiError>) {
    (StatusCode::BAD_REQUEST, Json(ApiError::new(e.to_string())))
}

/// Macro helper: validar que el módulo restaurante esté en la licencia.
/// Si no, retorna 403 con mensaje legible.
fn requiere_restaurante(state: &Arc<ServerState>) -> Result<(), (StatusCode, Json<ApiError>)> {
    crate::restaurante::requiere_modulo_restaurante(&state.db)
        .map_err(|m| (StatusCode::FORBIDDEN, Json(ApiError::new(m))))
}

#[derive(Debug, Deserialize)]
pub struct AbrirPedidoRequest {
    pub mesa_id: i64,
    pub comensales: Option<i32>,
}

/// `POST /api/v1/app/pedidos/abrir` — abre pedido en una mesa libre.
/// Mesero = usuario del session.
pub async fn pedidos_abrir(
    AxumState(state): AxumState<Arc<ServerState>>,
    headers: HeaderMap,
    Json(req): Json<AbrirPedidoRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ApiError>)> {
    let session = extract_app_session(&headers, &state)?;
    requiere_restaurante(&state)?;
    session.requiere("atiende_mesas")?;

    let conn = state.db.conn.lock().map_err(err500)?;

    // Validar mesa exista
    let mesa_existe: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM rest_mesas WHERE id = ?1 AND activa = 1",
            params![req.mesa_id],
            |row| row.get(0),
        )
        .unwrap_or(0);
    if mesa_existe == 0 {
        return Err(err400("Mesa no encontrada o inactiva"));
    }

    // Validar que no haya pedido activo en esta mesa
    let abierto: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM rest_pedidos_abiertos WHERE mesa_id = ?1 AND estado IN ('ABIERTO', 'CUENTA_PEDIDA')",
            params![req.mesa_id],
            |row| row.get(0),
        )
        .unwrap_or(0);
    if abierto > 0 {
        return Err(err400("Ya existe un pedido abierto en esta mesa"));
    }

    conn.execute(
        "INSERT INTO rest_pedidos_abiertos (mesa_id, mesero_id, mesero_nombre, comensales, estado)
         VALUES (?1, ?2, ?3, ?4, 'ABIERTO')",
        params![req.mesa_id, session.usuario_id, session.nombre, req.comensales.unwrap_or(1)],
    )
    .map_err(err500)?;

    let pedido_id = conn.last_insert_rowid();
    Ok(Json(serde_json::json!({ "ok": true, "pedido_id": pedido_id })))
}

/// `GET /api/v1/app/pedidos/:id` — detalle del pedido.
pub async fn pedidos_obtener(
    AxumState(state): AxumState<Arc<ServerState>>,
    headers: HeaderMap,
    Path(id): Path<i64>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ApiError>)> {
    let session = extract_app_session(&headers, &state)?;
    requiere_restaurante(&state)?;
    if !session.tiene("atiende_mesas") && !session.tiene("ve_cocina") {
        return Err((StatusCode::FORBIDDEN, Json(ApiError::new("Falta permiso atiende_mesas o ve_cocina"))));
    }

    let conn = state.db.conn.lock().map_err(err500)?;
    let detalle = crate::restaurante::commands::obtener_pedido_detalle(&conn, id)
        .map_err(|e| (StatusCode::NOT_FOUND, Json(ApiError::new(e))))?;
    Ok(Json(serde_json::json!({ "ok": true, "detalle": detalle })))
}

/// `GET /api/v1/app/pedidos/mesa/:mesa_id` — pedido activo en la mesa (o null).
pub async fn pedidos_de_mesa(
    AxumState(state): AxumState<Arc<ServerState>>,
    headers: HeaderMap,
    Path(mesa_id): Path<i64>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ApiError>)> {
    let session = extract_app_session(&headers, &state)?;
    requiere_restaurante(&state)?;
    if !session.tiene("atiende_mesas") && !session.tiene("ve_cocina") {
        return Err((StatusCode::FORBIDDEN, Json(ApiError::new("Falta permiso"))));
    }

    let conn = state.db.conn.lock().map_err(err500)?;
    let pedido_id: Option<i64> = conn
        .query_row(
            "SELECT id FROM rest_pedidos_abiertos
             WHERE mesa_id = ?1 AND estado IN ('ABIERTO', 'CUENTA_PEDIDA')
             ORDER BY id DESC LIMIT 1",
            params![mesa_id],
            |r| r.get(0),
        )
        .ok();

    match pedido_id {
        Some(pid) => {
            let detalle = crate::restaurante::commands::obtener_pedido_detalle(&conn, pid)
                .map_err(err500)?;
            Ok(Json(serde_json::json!({ "ok": true, "detalle": detalle })))
        }
        None => Ok(Json(serde_json::json!({ "ok": true, "detalle": null }))),
    }
}

#[derive(Debug, Deserialize)]
pub struct AgregarItemRequest {
    pub producto_id: i64,
    pub cantidad: Option<f64>,
    pub info_adicional: Option<String>,
}

/// `POST /api/v1/app/pedidos/:id/items` — agrega item al pedido.
pub async fn pedidos_agregar_item(
    AxumState(state): AxumState<Arc<ServerState>>,
    headers: HeaderMap,
    Path(pedido_id): Path<i64>,
    Json(req): Json<AgregarItemRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ApiError>)> {
    let session = extract_app_session(&headers, &state)?;
    requiere_restaurante(&state)?;
    session.requiere("atiende_mesas")?;

    let conn = state.db.conn.lock().map_err(err500)?;

    // Validar pedido abierto
    let estado: String = conn
        .query_row(
            "SELECT estado FROM rest_pedidos_abiertos WHERE id = ?1",
            params![pedido_id],
            |r| r.get(0),
        )
        .map_err(|_| err400("Pedido no encontrado"))?;
    if estado != "ABIERTO" && estado != "CUENTA_PEDIDA" {
        return Err(err400(format!("No se pueden agregar items a un pedido {}", estado)));
    }

    // Producto y destino
    let (precio, destino): (f64, String) = conn
        .query_row(
            "SELECT precio_venta, COALESCE(destino_preparacion, 'COCINA')
             FROM productos WHERE id = ?1 AND activo = 1",
            params![req.producto_id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .map_err(|_| err400("Producto no encontrado o inactivo"))?;

    let cantidad = req.cantidad.unwrap_or(1.0);
    let es_directo = destino == "DIRECTO";

    if es_directo {
        conn.execute(
            "INSERT INTO rest_pedido_items
             (pedido_id, producto_id, cantidad, precio_unit, info_adicional,
              enviado_cocina, estado_cocina, fecha_envio_cocina)
             VALUES (?1, ?2, ?3, ?4, ?5, 1, 'ENTREGADO', datetime('now', 'localtime'))",
            params![pedido_id, req.producto_id, cantidad, precio, req.info_adicional],
        ).map_err(err500)?;
    } else {
        conn.execute(
            "INSERT INTO rest_pedido_items (pedido_id, producto_id, cantidad, precio_unit, info_adicional)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![pedido_id, req.producto_id, cantidad, precio, req.info_adicional],
        ).map_err(err500)?;
    }

    let item_id = conn.last_insert_rowid();
    Ok(Json(serde_json::json!({ "ok": true, "item_id": item_id })))
}

/// `DELETE /api/v1/app/pedidos/items/:item_id` — elimina item (si no enviado a cocina).
pub async fn pedidos_eliminar_item(
    AxumState(state): AxumState<Arc<ServerState>>,
    headers: HeaderMap,
    Path(item_id): Path<i64>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ApiError>)> {
    let session = extract_app_session(&headers, &state)?;
    requiere_restaurante(&state)?;
    session.requiere("atiende_mesas")?;

    let conn = state.db.conn.lock().map_err(err500)?;

    // Validar que se pueda borrar
    let (enviado, destino): (i32, String) = conn
        .query_row(
            "SELECT i.enviado_cocina, COALESCE(p.destino_preparacion, 'COCINA')
             FROM rest_pedido_items i JOIN productos p ON i.producto_id = p.id
             WHERE i.id = ?1",
            params![item_id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .map_err(|_| err400("Item no encontrado"))?;
    if enviado != 0 && destino != "DIRECTO" {
        return Err(err400("No se puede eliminar un item ya enviado a cocina"));
    }

    conn.execute("DELETE FROM rest_pedido_items WHERE id = ?1", params![item_id])
        .map_err(err500)?;
    Ok(Json(serde_json::json!({ "ok": true })))
}

/// `POST /api/v1/app/pedidos/:id/enviar-cocina` — marca items pendientes como enviados.
/// Devuelve la lista de items recién enviados (la app los puede mostrar al mesero).
pub async fn pedidos_enviar_cocina(
    AxumState(state): AxumState<Arc<ServerState>>,
    headers: HeaderMap,
    Path(pedido_id): Path<i64>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ApiError>)> {
    let session = extract_app_session(&headers, &state)?;
    requiere_restaurante(&state)?;
    session.requiere("atiende_mesas")?;

    let conn = state.db.conn.lock().map_err(err500)?;

    // Items pendientes (en scope para que stmt se libere antes del execute/drop)
    let items: Vec<serde_json::Value> = {
        let mut stmt = conn.prepare(
            "SELECT i.id, p.nombre, i.cantidad, i.info_adicional,
                    COALESCE(p.destino_preparacion, 'COCINA') as destino
             FROM rest_pedido_items i JOIN productos p ON i.producto_id = p.id
             WHERE i.pedido_id = ?1 AND i.enviado_cocina = 0
             ORDER BY i.id",
        ).map_err(err500)?;
        let rows = stmt.query_map(params![pedido_id], |r| {
            Ok(serde_json::json!({
                "id": r.get::<_, i64>(0)?,
                "producto_nombre": r.get::<_, String>(1)?,
                "cantidad": r.get::<_, f64>(2)?,
                "info_adicional": r.get::<_, Option<String>>(3)?,
                "destino_preparacion": r.get::<_, String>(4)?,
            }))
        }).map_err(err500)?
          .collect::<Result<Vec<_>, _>>().map_err(err500)?;
        rows
    };

    if items.is_empty() {
        return Err(err400("No hay items nuevos para enviar a cocina"));
    }

    conn.execute(
        "UPDATE rest_pedido_items
         SET enviado_cocina = 1, fecha_envio_cocina = datetime('now', 'localtime')
         WHERE pedido_id = ?1 AND enviado_cocina = 0",
        params![pedido_id],
    ).map_err(err500)?;

    // Mesa nombre para mostrar en la push (opcional pero útil al cocinero)
    let mesa_nombre: String = conn.query_row(
        "SELECT m.nombre FROM rest_mesas m
         JOIN rest_pedidos p ON p.mesa_id = m.id
         WHERE p.id = ?1",
        params![pedido_id],
        |r| r.get(0),
    ).unwrap_or_else(|_| format!("Pedido #{}", pedido_id));

    drop(conn); // soltamos el lock antes del spawn async para evitar bloquear

    // v0.2 Sprint 6.2: push notification a cocineros con permiso ve_cocina
    if let Ok(tokens) = super::push::tokens_por_permiso(&state.db, "ve_cocina") {
        let total_items: f64 = items.iter()
            .filter_map(|i| i.get("cantidad").and_then(|c| c.as_f64()))
            .sum();
        let body = if items.len() == 1 {
            let nombre = items[0].get("producto_nombre").and_then(|n| n.as_str()).unwrap_or("Item");
            format!("{} · {} (x{:.0})", mesa_nombre, nombre, total_items)
        } else {
            format!("{} · {} items nuevos", mesa_nombre, items.len())
        };
        super::push::enviar_push_async(
            tokens,
            "🍳 Nueva comanda".to_string(),
            body,
            Some(serde_json::json!({
                "tipo": "cocina_nueva",
                "pedido_id": pedido_id,
                "mesa": mesa_nombre,
            })),
        );
    }

    Ok(Json(serde_json::json!({
        "ok": true, "items": items, "total": items.len()
    })))
}

/// `POST /api/v1/app/pedidos/:id/pedir-cuenta` — marca CUENTA_PEDIDA.
pub async fn pedidos_pedir_cuenta(
    AxumState(state): AxumState<Arc<ServerState>>,
    headers: HeaderMap,
    Path(pedido_id): Path<i64>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ApiError>)> {
    let session = extract_app_session(&headers, &state)?;
    requiere_restaurante(&state)?;
    session.requiere("atiende_mesas")?;

    let conn = state.db.conn.lock().map_err(err500)?;
    conn.execute(
        "UPDATE rest_pedidos_abiertos
         SET estado = 'CUENTA_PEDIDA', fecha_cuenta = datetime('now', 'localtime')
         WHERE id = ?1 AND estado = 'ABIERTO'",
        params![pedido_id],
    ).map_err(err500)?;
    Ok(Json(serde_json::json!({ "ok": true })))
}

/// `POST /api/v1/app/pedidos/:id/cancelar` — cancela pedido sin cobrar.
pub async fn pedidos_cancelar(
    AxumState(state): AxumState<Arc<ServerState>>,
    headers: HeaderMap,
    Path(pedido_id): Path<i64>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ApiError>)> {
    let session = extract_app_session(&headers, &state)?;
    requiere_restaurante(&state)?;
    session.requiere("cancela_pedido")?;

    let conn = state.db.conn.lock().map_err(err500)?;
    conn.execute(
        "UPDATE rest_pedidos_abiertos
         SET estado = 'CANCELADO', fecha_cierre = datetime('now', 'localtime')
         WHERE id = ?1",
        params![pedido_id],
    ).map_err(err500)?;
    Ok(Json(serde_json::json!({ "ok": true })))
}

#[derive(Debug, Deserialize)]
pub struct CobrarPedidoRequest {
    pub forma_pago: String,
    pub banco_id: Option<i64>,
    pub referencia_pago: Option<String>,
    #[serde(default)]
    pub es_fiado: bool,
    pub cliente_id: Option<i64>,
    pub tipo_documento: Option<String>,
}

/// `POST /api/v1/app/pedidos/:id/cobrar` — combo: registrar venta + cerrar pedido.
/// Reusa `dispatch_command("registrar_venta")` y luego marca el pedido como COBRADO.
pub async fn pedidos_cobrar(
    AxumState(state): AxumState<Arc<ServerState>>,
    headers: HeaderMap,
    Path(pedido_id): Path<i64>,
    Json(req): Json<CobrarPedidoRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ApiError>)> {
    let session = extract_app_session(&headers, &state)?;
    requiere_restaurante(&state)?;
    session.requiere("cobra_caja")?;

    // 1. Cargar items del pedido + datos de mesa para el payload de venta
    let detalle = {
        let conn = state.db.conn.lock().map_err(err500)?;
        crate::restaurante::commands::obtener_pedido_detalle(&conn, pedido_id)
            .map_err(|e| (StatusCode::NOT_FOUND, Json(ApiError::new(e))))?
    };

    if detalle.items.is_empty() {
        return Err(err400("El pedido no tiene items para cobrar"));
    }

    // 2. Construir payload de venta
    let total: f64 = detalle.items.iter().map(|i| i.cantidad * i.precio_unit).sum();
    let observacion = format!(
        "Mesa: {}{} · Pedido #{} · App móvil ({})",
        detalle.mesa_nombre,
        detalle.zona_nombre.as_ref().map(|z| format!(" ({})", z)).unwrap_or_default(),
        pedido_id,
        session.nombre
    );
    let items_venta: Vec<serde_json::Value> = detalle.items.iter().map(|i| {
        serde_json::json!({
            "producto_id": i.producto_id,
            "cantidad": i.cantidad,
            "precio_unitario": i.precio_unit,
            "descuento": 0.0,
            "iva_porcentaje": 0.0,
            "subtotal": i.cantidad * i.precio_unit,
            "info_adicional": i.info_adicional,
        })
    }).collect();

    let venta_args = serde_json::json!({
        "venta": {
            "items": items_venta,
            "forma_pago": req.forma_pago,
            "monto_recibido": if req.es_fiado { 0.0 } else { total },
            "descuento": 0.0,
            "tipo_documento": req.tipo_documento.unwrap_or_else(|| "NOTA_VENTA".to_string()),
            "es_fiado": req.es_fiado,
            "observacion": observacion,
            "banco_id": req.banco_id,
            "referencia_pago": req.referencia_pago,
            "cliente_id": req.cliente_id,
        }
    });

    // 3. Registrar venta vía dispatch (reusa toda la lógica del POS: SRI, secuencial, kardex, etc.)
    let resultado = crate::server::dispatch::dispatch_command(
        &state, "registrar_venta", venta_args
    ).await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(ApiError::new(e))))?;

    // Extraer venta_id de la respuesta
    let venta_id = resultado.get("venta")
        .and_then(|v| v.get("id"))
        .and_then(|v| v.as_i64())
        .ok_or_else(|| err500("Respuesta de venta sin id"))?;

    // 4. Marcar pedido como COBRADO (libera la mesa principal y todas las extras automáticamente)
    {
        let conn = state.db.conn.lock().map_err(err500)?;
        conn.execute(
            "UPDATE rest_pedidos_abiertos
             SET estado = 'COBRADO', venta_id = ?1, fecha_cierre = datetime('now', 'localtime')
             WHERE id = ?2",
            params![venta_id, pedido_id],
        ).map_err(err500)?;
    }

    Ok(Json(serde_json::json!({
        "ok": true,
        "venta_id": venta_id,
        "venta": resultado.get("venta"),
    })))
}

// ─── Cocina (Sprint 3b) ──────────────────────────────────────────────────

/// `GET /api/v1/app/cocina/items` — lista items pendientes en cocina.
pub async fn cocina_listar(
    AxumState(state): AxumState<Arc<ServerState>>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ApiError>)> {
    let session = extract_app_session(&headers, &state)?;
    requiere_restaurante(&state)?;
    session.requiere("ve_cocina")?;

    let conn = state.db.conn.lock().map_err(err500)?;
    let mut stmt = conn.prepare(
        "SELECT i.id, i.pedido_id, m.nombre, z.nombre, p.mesero_nombre,
                pr.nombre, i.cantidad, i.info_adicional, i.estado_cocina, i.fecha_envio_cocina,
                CAST((julianday('now', 'localtime') - julianday(i.fecha_envio_cocina)) * 24 * 60 AS INTEGER) AS mins
         FROM rest_pedido_items i
         JOIN rest_pedidos_abiertos p ON i.pedido_id = p.id
         JOIN rest_mesas m ON p.mesa_id = m.id
         LEFT JOIN rest_zonas z ON m.zona_id = z.id
         JOIN productos pr ON i.producto_id = pr.id
         WHERE i.enviado_cocina = 1
           AND i.estado_cocina IN ('PENDIENTE', 'EN_PREPARACION', 'LISTO')
         ORDER BY i.fecha_envio_cocina ASC",
    ).map_err(err500)?;

    let items: Vec<serde_json::Value> = stmt.query_map([], |r| {
        Ok(serde_json::json!({
            "id": r.get::<_, i64>(0)?,
            "pedido_id": r.get::<_, i64>(1)?,
            "mesa_nombre": r.get::<_, String>(2)?,
            "zona_nombre": r.get::<_, Option<String>>(3)?,
            "mesero_nombre": r.get::<_, Option<String>>(4)?,
            "producto_nombre": r.get::<_, String>(5)?,
            "cantidad": r.get::<_, f64>(6)?,
            "info_adicional": r.get::<_, Option<String>>(7)?,
            "estado_cocina": r.get::<_, String>(8)?,
            "fecha_envio_cocina": r.get::<_, Option<String>>(9)?,
            "minutos_en_cocina": r.get::<_, Option<i64>>(10)?,
        }))
    }).map_err(err500)?.collect::<Result<Vec<_>, _>>().map_err(err500)?;

    Ok(Json(serde_json::json!({ "ok": true, "items": items, "total": items.len() })))
}

#[derive(Debug, Deserialize)]
pub struct CocinaEstadoRequest {
    pub estado: String,
}

/// `POST /api/v1/app/cocina/items/:id/estado` — cambia estado del item (PENDIENTE/EN_PREPARACION/LISTO/ENTREGADO).
pub async fn cocina_marcar(
    AxumState(state): AxumState<Arc<ServerState>>,
    headers: HeaderMap,
    Path(item_id): Path<i64>,
    Json(req): Json<CocinaEstadoRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ApiError>)> {
    let session = extract_app_session(&headers, &state)?;
    requiere_restaurante(&state)?;
    session.requiere("ve_cocina")?;

    let estado_valido = matches!(req.estado.as_str(), "PENDIENTE" | "EN_PREPARACION" | "LISTO" | "ENTREGADO");
    if !estado_valido {
        return Err(err400("Estado de cocina inválido"));
    }

    let conn = state.db.conn.lock().map_err(err500)?;
    conn.execute(
        "UPDATE rest_pedido_items SET estado_cocina = ?1 WHERE id = ?2",
        params![req.estado, item_id],
    ).map_err(err500)?;
    Ok(Json(serde_json::json!({ "ok": true })))
}

// ─── Unir mesas + dividir cuenta (Sprint 3b) ─────────────────────────────

#[derive(Debug, Deserialize)]
pub struct UnirMesasRequest { pub mesas_ids: Vec<i64> }

pub async fn pedidos_unir_mesas(
    AxumState(state): AxumState<Arc<ServerState>>,
    headers: HeaderMap,
    Path(pedido_id): Path<i64>,
    Json(req): Json<UnirMesasRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ApiError>)> {
    let session = extract_app_session(&headers, &state)?;
    requiere_restaurante(&state)?;
    session.requiere("une_mesas")?;

    if req.mesas_ids.is_empty() {
        return Err(err400("Debe seleccionar al menos una mesa"));
    }

    let mut conn = state.db.conn.lock().map_err(err500)?;
    let (mesa_principal, estado): (i64, String) = conn
        .query_row(
            "SELECT mesa_id, estado FROM rest_pedidos_abiertos WHERE id = ?1",
            params![pedido_id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .map_err(|_| err400("Pedido no encontrado"))?;
    if estado != "ABIERTO" && estado != "CUENTA_PEDIDA" {
        return Err(err400(format!("No se pueden unir mesas a un pedido {}", estado)));
    }

    let tx = conn.transaction().map_err(err500)?;
    for mesa_id in &req.mesas_ids {
        if *mesa_id == mesa_principal { return Err(err400("No se puede unir la mesa principal a sí misma")); }
        let activa: i64 = tx.query_row("SELECT COUNT(*) FROM rest_mesas WHERE id = ?1 AND activa = 1", params![mesa_id], |r| r.get(0)).unwrap_or(0);
        if activa == 0 { return Err(err400(format!("Mesa {} no encontrada", mesa_id))); }
        let propio: i64 = tx.query_row(
            "SELECT COUNT(*) FROM rest_pedidos_abiertos WHERE mesa_id = ?1 AND estado IN ('ABIERTO', 'CUENTA_PEDIDA')",
            params![mesa_id], |r| r.get(0)
        ).unwrap_or(0);
        if propio > 0 { return Err(err400(format!("Mesa {} ya tiene pedido propio", mesa_id))); }
        let unida: i64 = tx.query_row(
            "SELECT COUNT(*) FROM rest_pedido_mesas_extra pe
             JOIN rest_pedidos_abiertos p ON pe.pedido_id = p.id
             WHERE pe.mesa_id = ?1 AND pe.pedido_id != ?2 AND p.estado IN ('ABIERTO', 'CUENTA_PEDIDA')",
            params![mesa_id, pedido_id], |r| r.get(0)
        ).unwrap_or(0);
        if unida > 0 { return Err(err400(format!("Mesa {} ya unida a otro pedido", mesa_id))); }
        tx.execute(
            "INSERT OR IGNORE INTO rest_pedido_mesas_extra (pedido_id, mesa_id) VALUES (?1, ?2)",
            params![pedido_id, mesa_id]
        ).map_err(err500)?;
    }
    tx.commit().map_err(err500)?;
    Ok(Json(serde_json::json!({ "ok": true })))
}

/// `DELETE /api/v1/app/pedidos/:pedido_id/mesas-extra/:mesa_id`
pub async fn pedidos_desunir_mesa(
    AxumState(state): AxumState<Arc<ServerState>>,
    headers: HeaderMap,
    Path((pedido_id, mesa_id)): Path<(i64, i64)>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ApiError>)> {
    let session = extract_app_session(&headers, &state)?;
    requiere_restaurante(&state)?;
    session.requiere("une_mesas")?;

    let conn = state.db.conn.lock().map_err(err500)?;
    let filas = conn.execute(
        "DELETE FROM rest_pedido_mesas_extra WHERE pedido_id = ?1 AND mesa_id = ?2",
        params![pedido_id, mesa_id]
    ).map_err(err500)?;
    if filas == 0 { return Err(err400("Esa mesa no estaba unida")); }
    Ok(Json(serde_json::json!({ "ok": true })))
}

#[derive(Debug, Deserialize)]
pub struct DividirCuentaRequest { pub n_partes: i32 }

pub async fn pedidos_dividir(
    AxumState(state): AxumState<Arc<ServerState>>,
    headers: HeaderMap,
    Path(pedido_id): Path<i64>,
    Json(req): Json<DividirCuentaRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ApiError>)> {
    let session = extract_app_session(&headers, &state)?;
    requiere_restaurante(&state)?;
    session.requiere("divide_cuenta")?;

    if !(2..=20).contains(&req.n_partes) {
        return Err(err400("n_partes debe ser entre 2 y 20"));
    }
    let mut conn = state.db.conn.lock().map_err(err500)?;
    let estado: String = conn.query_row(
        "SELECT estado FROM rest_pedidos_abiertos WHERE id = ?1",
        params![pedido_id], |r| r.get(0)
    ).map_err(|_| err400("Pedido no encontrado"))?;
    if estado != "ABIERTO" && estado != "CUENTA_PEDIDA" {
        return Err(err400(format!("No se puede dividir un pedido {}", estado)));
    }
    let existentes: i64 = conn.query_row(
        "SELECT COUNT(*) FROM rest_subcuentas WHERE pedido_id = ?1",
        params![pedido_id], |r| r.get(0)
    ).unwrap_or(0);
    if existentes > 0 { return Err(err400("Pedido ya dividido")); }

    let total: f64 = conn.query_row(
        "SELECT COALESCE(SUM(cantidad * precio_unit), 0) FROM rest_pedido_items WHERE pedido_id = ?1",
        params![pedido_id], |r| r.get(0)
    ).unwrap_or(0.0);
    if total <= 0.0 { return Err(err400("Pedido sin items para dividir")); }

    let total_centavos: i64 = (total * 100.0).round() as i64;
    let parte_centavos: i64 = total_centavos / (req.n_partes as i64);
    let residuo: i64 = total_centavos - parte_centavos * (req.n_partes as i64);

    let tx = conn.transaction().map_err(err500)?;
    for i in 1..=req.n_partes {
        let centavos = if i == req.n_partes { parte_centavos + residuo } else { parte_centavos };
        let monto = (centavos as f64) / 100.0;
        tx.execute(
            "INSERT INTO rest_subcuentas (pedido_id, numero, total) VALUES (?1, ?2, ?3)",
            params![pedido_id, i, monto]
        ).map_err(err500)?;
    }
    tx.commit().map_err(err500)?;

    let subs = crate::restaurante::commands::listar_subcuentas_internal(&conn, pedido_id)
        .map_err(err500)?;
    Ok(Json(serde_json::json!({ "ok": true, "subcuentas": subs })))
}

pub async fn pedidos_listar_subcuentas(
    AxumState(state): AxumState<Arc<ServerState>>,
    headers: HeaderMap,
    Path(pedido_id): Path<i64>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ApiError>)> {
    let _session = extract_app_session(&headers, &state)?;
    requiere_restaurante(&state)?;

    let conn = state.db.conn.lock().map_err(err500)?;
    let subs = crate::restaurante::commands::listar_subcuentas_internal(&conn, pedido_id)
        .map_err(err500)?;
    Ok(Json(serde_json::json!({ "ok": true, "subcuentas": subs })))
}

pub async fn pedidos_cancelar_division(
    AxumState(state): AxumState<Arc<ServerState>>,
    headers: HeaderMap,
    Path(pedido_id): Path<i64>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ApiError>)> {
    let session = extract_app_session(&headers, &state)?;
    requiere_restaurante(&state)?;
    session.requiere("divide_cuenta")?;

    let conn = state.db.conn.lock().map_err(err500)?;
    let cobradas: i64 = conn.query_row(
        "SELECT COUNT(*) FROM rest_subcuentas WHERE pedido_id = ?1 AND estado = 'COBRADA'",
        params![pedido_id], |r| r.get(0)
    ).unwrap_or(0);
    if cobradas > 0 { return Err(err400(format!("Hay {} sub-cuenta(s) ya cobradas", cobradas))); }
    conn.execute("DELETE FROM rest_subcuentas WHERE pedido_id = ?1", params![pedido_id])
        .map_err(err500)?;
    Ok(Json(serde_json::json!({ "ok": true })))
}

#[derive(Debug, Deserialize)]
pub struct CobrarSubcuentaRequest {
    pub forma_pago: String,
    pub banco_id: Option<i64>,
    pub referencia_pago: Option<String>,
}

/// `POST /api/v1/app/subcuentas/:id/cobrar` — combo: registrar venta del producto especial + marcar cobrada.
pub async fn subcuentas_cobrar(
    AxumState(state): AxumState<Arc<ServerState>>,
    headers: HeaderMap,
    Path(subcuenta_id): Path<i64>,
    Json(req): Json<CobrarSubcuentaRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ApiError>)> {
    let session = extract_app_session(&headers, &state)?;
    requiere_restaurante(&state)?;
    session.requiere("cobra_caja")?;

    // 1. Cargar datos sub-cuenta + pedido
    let (pedido_id, monto, estado, total_subs): (i64, f64, String, i64) = {
        let conn = state.db.conn.lock().map_err(err500)?;
        let row: (i64, f64, String) = conn.query_row(
            "SELECT pedido_id, total, estado FROM rest_subcuentas WHERE id = ?1",
            params![subcuenta_id], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?))
        ).map_err(|_| err400("Sub-cuenta no encontrada"))?;
        let total_subs: i64 = conn.query_row(
            "SELECT COUNT(*) FROM rest_subcuentas WHERE pedido_id = ?1",
            params![row.0], |r| r.get(0)
        ).unwrap_or(0);
        (row.0, row.1, row.2, total_subs)
    };
    if estado == "COBRADA" { return Err(err400("Esta sub-cuenta ya fue cobrada")); }

    // 2. Producto especial
    let prod_id: i64 = {
        let conn = state.db.conn.lock().map_err(err500)?;
        conn.query_row(
            "SELECT id FROM productos WHERE codigo = '_DIVISION_CUENTA_'",
            [], |r| r.get(0)
        ).map_err(|_| err500("Producto _DIVISION_CUENTA_ no existe"))?
    };

    // 3. Detalle pedido para observación
    let detalle = {
        let conn = state.db.conn.lock().map_err(err500)?;
        crate::restaurante::commands::obtener_pedido_detalle(&conn, pedido_id)
            .map_err(err500)?
    };
    let numero_sub = detalle.items.len(); // placeholder, lo corregimos abajo
    let _ = numero_sub;

    // 4. Registrar venta del producto especial
    let observacion = format!(
        "Mesa: {}{} · Pedido #{} · Sub-cuenta de {} · App móvil",
        detalle.mesa_nombre,
        detalle.zona_nombre.as_ref().map(|z| format!(" ({})", z)).unwrap_or_default(),
        pedido_id, total_subs
    );
    let venta_args = serde_json::json!({
        "venta": {
            "items": [{
                "producto_id": prod_id,
                "cantidad": 1.0,
                "precio_unitario": monto,
                "descuento": 0.0,
                "iva_porcentaje": 0.0,
                "subtotal": monto,
                "info_adicional": format!("Items consumidos: {}",
                    detalle.items.iter().map(|i|
                        format!("{}x {}", i.cantidad, i.producto_nombre.clone().unwrap_or_default())
                    ).collect::<Vec<_>>().join(", ")
                ),
            }],
            "forma_pago": req.forma_pago.clone(),
            "monto_recibido": monto,
            "descuento": 0.0,
            "tipo_documento": "NOTA_VENTA",
            "es_fiado": false,
            "observacion": observacion,
            "banco_id": req.banco_id,
            "referencia_pago": req.referencia_pago,
        }
    });
    let resultado = crate::server::dispatch::dispatch_command(
        &state, "registrar_venta", venta_args
    ).await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(ApiError::new(e))))?;

    let venta_id = resultado.get("venta")
        .and_then(|v| v.get("id")).and_then(|v| v.as_i64())
        .ok_or_else(|| err500("Respuesta de venta sin id"))?;

    // 5. Marcar sub-cuenta cobrada + ¿todas pagas? → cerrar pedido
    let (todas_cobradas, pendientes) = {
        let conn = state.db.conn.lock().map_err(err500)?;
        conn.execute(
            "UPDATE rest_subcuentas
             SET estado = 'COBRADA', forma_pago = ?1, banco_id = ?2, referencia_pago = ?3,
                 venta_id = ?4, fecha_cobro = datetime('now', 'localtime')
             WHERE id = ?5",
            params![req.forma_pago, req.banco_id, req.referencia_pago, venta_id, subcuenta_id]
        ).map_err(err500)?;
        let pend: i32 = conn.query_row(
            "SELECT COUNT(*) FROM rest_subcuentas WHERE pedido_id = ?1 AND estado = 'PENDIENTE'",
            params![pedido_id], |r| r.get(0)
        ).unwrap_or(0);
        let todas = pend == 0;
        if todas {
            let primera_venta_id: i64 = conn.query_row(
                "SELECT venta_id FROM rest_subcuentas WHERE pedido_id = ?1 AND venta_id IS NOT NULL ORDER BY numero LIMIT 1",
                params![pedido_id], |r| r.get(0)
            ).unwrap_or(venta_id);
            conn.execute(
                "UPDATE rest_pedidos_abiertos
                 SET estado = 'COBRADO', venta_id = ?1, fecha_cierre = datetime('now', 'localtime')
                 WHERE id = ?2",
                params![primera_venta_id, pedido_id]
            ).map_err(err500)?;
        }
        (todas, pend)
    };

    Ok(Json(serde_json::json!({
        "ok": true,
        "venta_id": venta_id,
        "todas_cobradas": todas_cobradas,
        "pendientes": pendientes,
    })))
}

// ─── Vendedor de piso: venta directa (Sprint 3b) ─────────────────────────

/// `POST /api/v1/app/ventas` — registra una venta directa (vendedor de piso o cobro
/// fuera de mesa). El payload es el mismo que `registrar_venta` desktop.
/// Requiere permiso `vende_piso` o `cobra_caja`.
pub async fn ventas_registrar(
    AxumState(state): AxumState<Arc<ServerState>>,
    headers: HeaderMap,
    Json(venta): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ApiError>)> {
    let session = extract_app_session(&headers, &state)?;
    if !session.tiene("vende_piso") && !session.tiene("cobra_caja") {
        return Err((StatusCode::FORBIDDEN, Json(ApiError::new("Falta permiso vende_piso o cobra_caja"))));
    }

    // Delegar al dispatcher (reusa toda la lógica del POS: SRI, kardex, secuenciales)
    let resultado = crate::server::dispatch::dispatch_command(
        &state, "registrar_venta", serde_json::json!({ "venta": venta })
    ).await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(ApiError::new(e))))?;

    Ok(Json(serde_json::json!({ "ok": true, "resultado": resultado })))
}

/// `GET /api/v1/app/pedidos/:id/mesas-libres-para-unir` — lista mesas LIBRES
/// disponibles para unir al pedido.
pub async fn pedidos_mesas_libres(
    AxumState(state): AxumState<Arc<ServerState>>,
    headers: HeaderMap,
    Path(pedido_id): Path<i64>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ApiError>)> {
    let session = extract_app_session(&headers, &state)?;
    requiere_restaurante(&state)?;
    session.requiere("une_mesas")?;

    let conn = state.db.conn.lock().map_err(err500)?;
    let mesa_principal: i64 = conn.query_row(
        "SELECT mesa_id FROM rest_pedidos_abiertos WHERE id = ?1",
        params![pedido_id], |r| r.get(0)
    ).map_err(|_| err400("Pedido no encontrado"))?;

    let mut stmt = conn.prepare(
        "SELECT m.id, m.nombre, m.capacidad, z.nombre
         FROM rest_mesas m
         LEFT JOIN rest_zonas z ON m.zona_id = z.id
         WHERE m.activa = 1 AND m.id != ?1
           AND NOT EXISTS (SELECT 1 FROM rest_pedidos_abiertos p
                           WHERE p.mesa_id = m.id AND p.estado IN ('ABIERTO', 'CUENTA_PEDIDA'))
           AND NOT EXISTS (SELECT 1 FROM rest_pedido_mesas_extra pe
                           JOIN rest_pedidos_abiertos p2 ON pe.pedido_id = p2.id
                           WHERE pe.mesa_id = m.id AND p2.estado IN ('ABIERTO', 'CUENTA_PEDIDA'))
         ORDER BY z.orden NULLS LAST, m.orden, m.nombre"
    ).map_err(err500)?;
    let mesas: Vec<serde_json::Value> = stmt.query_map(params![mesa_principal], |r| {
        Ok(serde_json::json!({
            "id": r.get::<_, i64>(0)?,
            "nombre": r.get::<_, String>(1)?,
            "capacidad": r.get::<_, i32>(2)?,
            "zona_nombre": r.get::<_, Option<String>>(3)?,
        }))
    }).map_err(err500)?.collect::<Result<Vec<_>, _>>().map_err(err500)?;

    Ok(Json(serde_json::json!({ "ok": true, "mesas": mesas })))
}

// ─── Router builder ──────────────────────────────────────────────────────

/// Devuelve el router con todas las rutas del módulo, listo para `merge` en
/// el server principal.
pub fn rutas() -> Router<Arc<ServerState>> {
    use axum::routing::delete;
    Router::new()
        // ── Sin auth ────────────────────────────────────────────────────
        .route("/api/v1/app/ping", get(ping))
        .route("/api/v1/app/auth/pin", post(auth_pin))
        .route("/api/v1/app/auth/logout", post(auth_logout))
        .route("/api/v1/app/auth/push-token", post(auth_set_push_token))
        .route("/api/v1/app/auth/usuarios-disponibles", get(auth_usuarios_disponibles))
        // ── Datos del usuario / catálogo ────────────────────────────────
        .route("/api/v1/app/me", get(me))
        .route("/api/v1/app/productos", get(listar_productos))
        // ── Mesas (restaurante) ─────────────────────────────────────────
        .route("/api/v1/app/mesas", get(listar_mesas))
        // ── Pedidos (Sprint 3b) ─────────────────────────────────────────
        .route("/api/v1/app/pedidos/abrir", post(pedidos_abrir))
        .route("/api/v1/app/pedidos/:id", get(pedidos_obtener))
        .route("/api/v1/app/pedidos/mesa/:mesa_id", get(pedidos_de_mesa))
        .route("/api/v1/app/pedidos/:id/items", post(pedidos_agregar_item))
        .route("/api/v1/app/pedidos/items/:item_id", delete(pedidos_eliminar_item))
        .route("/api/v1/app/pedidos/:id/enviar-cocina", post(pedidos_enviar_cocina))
        .route("/api/v1/app/pedidos/:id/pedir-cuenta", post(pedidos_pedir_cuenta))
        .route("/api/v1/app/pedidos/:id/cancelar", post(pedidos_cancelar))
        .route("/api/v1/app/pedidos/:id/cobrar", post(pedidos_cobrar))
        // ── Unir mesas (Sprint 3b) ──────────────────────────────────────
        .route("/api/v1/app/pedidos/:id/unir-mesas", post(pedidos_unir_mesas))
        .route("/api/v1/app/pedidos/:pedido_id/mesas-extra/:mesa_id", delete(pedidos_desunir_mesa))
        .route("/api/v1/app/pedidos/:id/mesas-libres-para-unir", get(pedidos_mesas_libres))
        // ── Dividir cuenta (Sprint 3b) ──────────────────────────────────
        .route("/api/v1/app/pedidos/:id/dividir", post(pedidos_dividir))
        .route("/api/v1/app/pedidos/:id/subcuentas", get(pedidos_listar_subcuentas))
        .route("/api/v1/app/pedidos/:id/cancelar-division", post(pedidos_cancelar_division))
        .route("/api/v1/app/subcuentas/:id/cobrar", post(subcuentas_cobrar))
        // ── Cocina (Sprint 3b) ──────────────────────────────────────────
        .route("/api/v1/app/cocina/items", get(cocina_listar))
        .route("/api/v1/app/cocina/items/:id/estado", post(cocina_marcar))
        // ── Vendedor de piso (Sprint 3b) ────────────────────────────────
        .route("/api/v1/app/ventas", post(ventas_registrar))
        // ── Servicio Técnico (Sprint 6.4 — técnico móvil) ───────────────
        .route("/api/v1/app/st/mis-ordenes", get(super::http_st::st_mis_ordenes))
        .route("/api/v1/app/st/ordenes", post(super::http_st::st_crear_orden))
        .route("/api/v1/app/st/ordenes/:id", get(super::http_st::st_obtener_orden))
        .route("/api/v1/app/st/ordenes/:id/estado", post(super::http_st::st_cambiar_estado))
        .route("/api/v1/app/st/ordenes/:id/diagnostico", post(super::http_st::st_guardar_diagnostico))
        .route("/api/v1/app/st/ordenes/:id/imagen", post(super::http_st::st_subir_imagen))
}
