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
    // Datos del negocio para encabezado de comprobantes desde la app
    pub ruc: String,
    pub direccion: String,
    pub telefono: String,
    pub email_negocio: String,
    pub pagina_web: String,
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
    ///
    /// v2.4.21: permisos implícitos por rol. El rol TECNICO ya implica
    /// acceso al módulo Servicio Técnico sin necesidad de asignar permisos
    /// manualmente (antes se creaba con permisos={} y no podía usar la app).
    pub fn tiene(&self, permiso: &str) -> bool {
        if self.rol == "ADMIN" { return true; }
        if self.rol == "TECNICO"
            && (permiso == "gestionar_servicio_tecnico" || permiso == "ver_servicio_tecnico")
        {
            return true;
        }
        self.permisos.iter().any(|p| p == permiso)
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

    // Modo de login configurado en el POS: "pin" | "password" | "ambos".
    let modo_login = conn
        .as_ref()
        .and_then(|c| c.query_row("SELECT value FROM config WHERE key = 'modo_login'", [], |r| r.get::<_, String>(0)).ok())
        .unwrap_or_else(|| "pin".to_string());

    Json(serde_json::json!({
        "ok": true,
        "service": "clouget-pos",
        "negocio": nombre_negocio,
        "app_movil_activo": app_movil_activo,
        "modo_login": modo_login,
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

    let permisos_app = ["atiende_mesas", "ve_cocina", "vende_piso", "inventaria", "dueno_dashboard", "cobra_caja", "gestionar_servicio_tecnico", "ver_servicio_tecnico", "recibir_abonos_st"];
    let usuarios: Vec<(i64, String, String, String)> = stmt
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
        .collect::<Vec<_>>();

    // Separar: con acceso de app vs sin acceso (para informar al admin cuántos
    // usuarios faltan habilitar, sin exponer sus datos completos).
    let mut con_acceso: Vec<serde_json::Value> = Vec::new();
    let mut sin_acceso = 0_i64;
    for (id, nombre, rol, permisos_json) in usuarios {
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
        if tiene_app {
            con_acceso.push(serde_json::json!({ "id": id, "nombre": nombre, "rol": rol, "es_admin": es_admin }));
        } else {
            sin_acceso += 1;
        }
    }

    Ok(Json(serde_json::json!({ "ok": true, "usuarios": con_acceso, "sin_acceso": sin_acceso })))
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
        "gestionar_servicio_tecnico",
        "ver_servicio_tecnico",
        "recibir_abonos_st",
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

/// Body para login por contraseña: se identifica por la contraseña (no por id).
#[derive(Debug, Deserialize)]
pub struct LoginPasswordRequest {
    pub password: String,
    /// Si se envía, valida la contraseña SOLO de ese usuario (más seguro y sin
    /// ambigüedad si dos usuarios comparten contraseña). Si falta, busca en todos.
    #[serde(default)]
    pub usuario_id: Option<i64>,
    #[serde(default)]
    pub dispositivo_nombre: Option<String>,
    #[serde(default)]
    pub dispositivo_modelo: Option<String>,
    #[serde(default)]
    pub dispositivo_so: Option<String>,
}

const PERMISOS_APP: [&str; 9] = [
    "atiende_mesas", "ve_cocina", "vende_piso", "inventaria", "dueno_dashboard",
    "cobra_caja", "gestionar_servicio_tecnico", "ver_servicio_tecnico", "recibir_abonos_st",
];

/// `POST /api/v1/app/auth/password` — login por contraseña (cuando el negocio
/// usa modo_login = password o ambos). Identifica al usuario por la contraseña.
pub async fn auth_password(
    AxumState(state): AxumState<Arc<ServerState>>,
    Json(req): Json<LoginPasswordRequest>,
) -> Result<Json<LoginPinResponse>, (StatusCode, Json<ApiError>)> {
    if let Err(msg) = super::requiere_modulo_app_movil(&state.db) {
        return Err((StatusCode::FORBIDDEN, Json(ApiError::new(msg))));
    }
    let password = req.password.trim();
    if password.is_empty() {
        return Err((StatusCode::BAD_REQUEST, Json(ApiError::new("Ingrese su contraseña"))));
    }

    let conn = state.db.conn.lock().map_err(err500)?;

    // Recorrer usuarios activos con password configurada y comparar el hash.
    let mut stmt = conn.prepare(
        "SELECT id, nombre, rol, COALESCE(permisos,'{}'), password_hash, password_salt
         FROM usuarios WHERE activo = 1 AND password_hash IS NOT NULL AND password_salt IS NOT NULL"
    ).map_err(err500)?;
    let filas: Vec<(i64, String, String, String, String, String)> = stmt.query_map([], |r| Ok((
        r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?, r.get(5)?,
    ))).map_err(err500)?.filter_map(|x| x.ok()).collect();
    drop(stmt);

    let mut encontrado: Option<(i64, String, String, String)> = None;
    for (id, nombre, rol, permisos_json, pw_hash, pw_salt) in filas {
        // Si la app envió usuario_id, validar SOLO ese usuario.
        if let Some(uid) = req.usuario_id {
            if uid != id { continue; }
        }
        if crate::utils::hash_pin(&pw_salt, password) == pw_hash {
            encontrado = Some((id, nombre, rol, permisos_json));
            break;
        }
    }
    let (id, nombre, rol, permisos_json) = encontrado.ok_or((
        StatusCode::UNAUTHORIZED, Json(ApiError::new("Contraseña incorrecta")),
    ))?;

    let permisos: Vec<String> = serde_json::from_str::<serde_json::Value>(&permisos_json)
        .ok()
        .and_then(|v| v.as_object().map(|m| m.iter().filter(|(_, v)| v.as_bool().unwrap_or(false)).map(|(k, _)| k.clone()).collect()))
        .unwrap_or_default();
    let es_admin = rol == "ADMIN";
    if !(es_admin || permisos.iter().any(|p| PERMISOS_APP.contains(&p.as_str()))) {
        return Err((StatusCode::FORBIDDEN, Json(ApiError::new(
            "Este usuario no tiene permisos para usar la app móvil."
        ))));
    }

    let token = uuid::Uuid::new_v4().to_string();
    conn.execute(
        "INSERT INTO app_tokens (usuario_id, token, dispositivo_nombre, dispositivo_modelo, dispositivo_so)
         VALUES (?1, ?2, ?3, ?4, ?5)",
        params![id, token, req.dispositivo_nombre, req.dispositivo_modelo, req.dispositivo_so],
    ).map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(ApiError::new(format!("Error guardando token: {}", e)))))?;

    Ok(Json(LoginPinResponse { token, usuario_id: id, nombre, rol, permisos, es_admin }))
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

    let cfg = |k: &str| -> String {
        conn.query_row("SELECT value FROM config WHERE key = ?1", [k], |r| r.get(0))
            .unwrap_or_default()
    };
    let ruc = cfg("ruc");
    let direccion = cfg("direccion");
    let telefono = cfg("telefono");
    let email_negocio = cfg("email_negocio");
    let pagina_web = cfg("pagina_web");

    let es_admin = session.rol == "ADMIN";
    Ok(Json(MeResponse {
        usuario_id: session.usuario_id,
        nombre: session.nombre,
        rol: session.rol,
        permisos: session.permisos,
        es_admin,
        negocio,
        modulos_licencia: modulos,
        ruc,
        direccion,
        telefono,
        email_negocio,
        pagina_web,
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

    // v2.5.91 — si la mesa tiene abonos (pagos parciales), la venta se arma como
    // MIXTO: cada abono con su forma de pago + el SALDO con la forma del cierre.
    // Así el arqueo refleja correctamente cada forma (efectivo/transfer/etc.).
    let abonos_holding: Vec<&crate::restaurante::models::AbonoPedido> =
        detalle.abonos.iter().filter(|a| a.estado == "HOLDING").collect();
    let total_abonado: f64 = abonos_holding.iter().map(|a| a.monto).sum();
    let saldo = (total - total_abonado).max(0.0);

    let venta_args = if total_abonado > 0.01 {
        // Pagos: abonos previos + saldo final (o crédito si es fiado el saldo).
        let mut pagos: Vec<serde_json::Value> = abonos_holding.iter().map(|a| serde_json::json!({
            "forma_pago": a.forma_pago,
            "monto": a.monto,
            "banco_id": a.banco_id,
            "referencia": a.referencia_pago,
        })).collect();
        if saldo > 0.01 {
            pagos.push(serde_json::json!({
                "forma_pago": if req.es_fiado { "CREDITO" } else { req.forma_pago.as_str() },
                "monto": saldo,
                "banco_id": req.banco_id,
                "referencia": req.referencia_pago,
            }));
        }
        serde_json::json!({
            "venta": {
                "items": items_venta,
                "forma_pago": "MIXTO",
                "monto_recibido": total,
                "descuento": 0.0,
                "tipo_documento": req.tipo_documento.unwrap_or_else(|| "NOTA_VENTA".to_string()),
                "es_fiado": false,
                "observacion": observacion,
                "cliente_id": req.cliente_id,
                "pagos": pagos,
            }
        })
    } else {
        serde_json::json!({
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
        })
    };

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
        // v2.5.91 — los abonos en HOLDING pasan a APLICADO (ya forman parte de la venta).
        conn.execute(
            "UPDATE rest_pedido_abonos
             SET estado = 'APLICADO', venta_id_aplicado = ?1, fecha_aplicado = datetime('now', 'localtime')
             WHERE pedido_id = ?2 AND estado = 'HOLDING'",
            params![venta_id, pedido_id],
        ).map_err(err500)?;
    }

    Ok(Json(serde_json::json!({
        "ok": true,
        "venta_id": venta_id,
        "venta": resultado.get("venta"),
    })))
}

/// `POST /api/v1/app/pedidos/:id/abono` — registra un pago parcial (abono) sobre
/// la mesa. El dinero entra a la caja como HOLDING; al cobrar el pedido se aplica.
#[derive(Debug, Deserialize)]
pub struct AbonoPedidoRequest {
    pub monto: f64,
    pub forma_pago: String,
    #[serde(default)]
    pub banco_id: Option<i64>,
    #[serde(default)]
    pub referencia_pago: Option<String>,
}

pub async fn pedidos_abono(
    AxumState(state): AxumState<Arc<ServerState>>,
    headers: HeaderMap,
    Path(pedido_id): Path<i64>,
    Json(req): Json<AbonoPedidoRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ApiError>)> {
    let session = extract_app_session(&headers, &state)?;
    requiere_restaurante(&state)?;
    session.requiere("cobra_caja")?;

    let conn = state.db.conn.lock().map_err(err500)?;
    let abono_id = crate::restaurante::commands::registrar_abono_pedido(
        &conn, pedido_id, req.monto, &req.forma_pago,
        req.banco_id, req.referencia_pago.as_deref(),
        Some(session.usuario_id), Some(&session.nombre),
    ).map_err(err400)?;

    let detalle = crate::restaurante::commands::obtener_pedido_detalle(&conn, pedido_id)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(ApiError::new(e))))?;

    Ok(Json(serde_json::json!({ "ok": true, "abono_id": abono_id, "detalle": detalle })))
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

// ─── v2.5.50 — Clientes (CRUD básico desde la app) ──────────────────────────

#[derive(Debug, Deserialize)]
pub struct ListarClientesQuery {
    pub q: Option<String>,
    pub limite: Option<i64>,
}

/// `GET /api/v1/app/clientes?q=&limite=` — lista clientes. Si `q` viene,
/// filtra por nombre/identificacion/email. Default 100, máx 500.
pub async fn listar_clientes(
    AxumState(state): AxumState<Arc<ServerState>>,
    headers: HeaderMap,
    Query(qp): Query<ListarClientesQuery>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ApiError>)> {
    let _session = extract_app_session(&headers, &state)?;

    let limite = qp.limite.unwrap_or(100).clamp(1, 500);
    let conn = state.db.conn.lock().map_err(err500)?;
    let busqueda = qp.q.unwrap_or_default();
    let busqueda_pat = format!("%{}%", busqueda);

    let (sql, params_dyn): (&str, Vec<Box<dyn rusqlite::ToSql>>) = if busqueda.is_empty() {
        (
            "SELECT id, COALESCE(tipo_identificacion, ''), COALESCE(identificacion, ''),
                    nombre, COALESCE(email, ''), COALESCE(telefono, ''), COALESCE(direccion, '')
             FROM clientes
             WHERE COALESCE(activo, 1) = 1
             ORDER BY nombre
             LIMIT ?1",
            vec![Box::new(limite)],
        )
    } else {
        (
            "SELECT id, COALESCE(tipo_identificacion, ''), COALESCE(identificacion, ''),
                    nombre, COALESCE(email, ''), COALESCE(telefono, ''), COALESCE(direccion, '')
             FROM clientes
             WHERE COALESCE(activo, 1) = 1
               AND (nombre LIKE ?1 OR identificacion LIKE ?1 OR email LIKE ?1)
             ORDER BY nombre
             LIMIT ?2",
            vec![Box::new(busqueda_pat), Box::new(limite)],
        )
    };

    let mut stmt = conn.prepare(sql).map_err(err500)?;
    let params_refs: Vec<&dyn rusqlite::ToSql> = params_dyn.iter().map(|b| b.as_ref()).collect();

    let clientes: Vec<serde_json::Value> = stmt
        .query_map(rusqlite::params_from_iter(params_refs.iter()), |r| {
            Ok(serde_json::json!({
                "id": r.get::<_, i64>(0)?,
                "tipo_identificacion": r.get::<_, String>(1)?,
                "identificacion": r.get::<_, String>(2)?,
                "nombre": r.get::<_, String>(3)?,
                "email": r.get::<_, String>(4)?,
                "telefono": r.get::<_, String>(5)?,
                "direccion": r.get::<_, String>(6)?,
            }))
        })
        .map_err(err500)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(err500)?;

    Ok(Json(serde_json::json!({
        "ok": true,
        "clientes": clientes,
        "total": clientes.len(),
    })))
}

/// `GET /api/v1/app/clientes/:id` — un cliente específico.
pub async fn obtener_cliente(
    AxumState(state): AxumState<Arc<ServerState>>,
    headers: HeaderMap,
    Path(id): Path<i64>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ApiError>)> {
    let _session = extract_app_session(&headers, &state)?;
    let conn = state.db.conn.lock().map_err(err500)?;
    let cliente: serde_json::Value = conn.query_row(
        "SELECT id, COALESCE(tipo_identificacion, ''), COALESCE(identificacion, ''),
                nombre, COALESCE(email, ''), COALESCE(telefono, ''), COALESCE(direccion, '')
         FROM clientes WHERE id = ?1",
        params![id],
        |r| Ok(serde_json::json!({
            "id": r.get::<_, i64>(0)?,
            "tipo_identificacion": r.get::<_, String>(1)?,
            "identificacion": r.get::<_, String>(2)?,
            "nombre": r.get::<_, String>(3)?,
            "email": r.get::<_, String>(4)?,
            "telefono": r.get::<_, String>(5)?,
            "direccion": r.get::<_, String>(6)?,
        })),
    ).map_err(|_| err400("Cliente no encontrado"))?;
    Ok(Json(serde_json::json!({ "ok": true, "cliente": cliente })))
}

/// `GET /api/v1/app/consultar-identificacion?id=<cedula_o_ruc>` — busca un
/// cliente por cédula/RUC. Si ya existe en la base local, lo devuelve para
/// autollenar el formulario. (No consulta el SRI externo; eso queda en el POS.)
pub async fn consultar_identificacion_app(
    AxumState(state): AxumState<Arc<ServerState>>,
    headers: HeaderMap,
    Query(qp): Query<std::collections::HashMap<String, String>>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ApiError>)> {
    let _session = extract_app_session(&headers, &state)?;
    let id = qp.get("id").map(|s| s.trim().to_string()).unwrap_or_default();
    if !id.chars().all(|c| c.is_ascii_digit()) || (id.len() != 10 && id.len() != 13) {
        return Err(err400("Ingrese una cédula (10 dígitos) o RUC (13 dígitos)"));
    }
    let conn = state.db.conn.lock().map_err(err500)?;
    let encontrado: Option<serde_json::Value> = conn.query_row(
        "SELECT id, COALESCE(tipo_identificacion,''), COALESCE(identificacion,''),
                nombre, COALESCE(email,''), COALESCE(telefono,''), COALESCE(direccion,'')
         FROM clientes WHERE identificacion = ?1 AND activo = 1 LIMIT 1",
        params![id],
        |r| Ok(serde_json::json!({
            "id": r.get::<_, i64>(0)?,
            "tipo_identificacion": r.get::<_, String>(1)?,
            "identificacion": r.get::<_, String>(2)?,
            "nombre": r.get::<_, String>(3)?,
            "email": r.get::<_, String>(4)?,
            "telefono": r.get::<_, String>(5)?,
            "direccion": r.get::<_, String>(6)?,
        })),
    ).ok();
    Ok(Json(serde_json::json!({
        "ok": true,
        "existente": encontrado.is_some(),
        "cliente": encontrado,
        "tipo_identificacion": if id.len() == 13 { "RUC" } else { "CEDULA" },
    })))
}

/// `GET /api/v1/app/st/tipos-equipo` — catálogo de tipos de equipo del ST.
pub async fn st_tipos_equipo(
    AxumState(state): AxumState<Arc<ServerState>>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ApiError>)> {
    let _session = extract_app_session(&headers, &state)?;
    let conn = state.db.conn.lock().map_err(err500)?;
    let mut stmt = conn.prepare(
        "SELECT id, nombre, COALESCE(icono,'') FROM st_tipos_equipo WHERE activo = 1 ORDER BY orden, nombre"
    ).map_err(err500)?;
    let rows: Vec<serde_json::Value> = stmt.query_map([], |r| Ok(serde_json::json!({
        "id": r.get::<_, i64>(0)?, "nombre": r.get::<_, String>(1)?, "icono": r.get::<_, String>(2)?,
    }))).map_err(err500)?.collect::<Result<Vec<_>, _>>().map_err(err500)?;
    Ok(Json(serde_json::json!({ "ok": true, "tipos": rows })))
}

/// `GET /api/v1/app/st/marcas?tipo_id=<id>` — marcas de un tipo de equipo.
pub async fn st_marcas(
    AxumState(state): AxumState<Arc<ServerState>>,
    headers: HeaderMap,
    Query(qp): Query<std::collections::HashMap<String, String>>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ApiError>)> {
    let _session = extract_app_session(&headers, &state)?;
    let tipo_id: i64 = qp.get("tipo_id").and_then(|s| s.parse().ok()).unwrap_or(0);
    let conn = state.db.conn.lock().map_err(err500)?;
    let mut stmt = conn.prepare(
        "SELECT id, nombre FROM st_marcas WHERE tipo_equipo_id = ?1 AND activo = 1 ORDER BY nombre"
    ).map_err(err500)?;
    let rows: Vec<serde_json::Value> = stmt.query_map(params![tipo_id], |r| Ok(serde_json::json!({
        "id": r.get::<_, i64>(0)?, "nombre": r.get::<_, String>(1)?,
    }))).map_err(err500)?.collect::<Result<Vec<_>, _>>().map_err(err500)?;
    Ok(Json(serde_json::json!({ "ok": true, "marcas": rows })))
}

/// `GET /api/v1/app/cuentas-banco` — cuentas bancarias activas para cobrar
/// con transferencia/depósito desde la app (sincroniza con el POS de escritorio).
pub async fn cuentas_banco_app(
    AxumState(state): AxumState<Arc<ServerState>>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ApiError>)> {
    let _session = extract_app_session(&headers, &state)?;
    let conn = state.db.conn.lock().map_err(err500)?;
    let mut stmt = conn.prepare(
        "SELECT id, nombre, COALESCE(tipo_cuenta,''), COALESCE(numero_cuenta,''), COALESCE(titular,'')
         FROM cuentas_banco WHERE activa = 1 ORDER BY nombre"
    ).map_err(err500)?;
    let rows: Vec<serde_json::Value> = stmt.query_map([], |r| Ok(serde_json::json!({
        "id": r.get::<_, i64>(0)?,
        "nombre": r.get::<_, String>(1)?,
        "tipo_cuenta": r.get::<_, String>(2)?,
        "numero_cuenta": r.get::<_, String>(3)?,
        "titular": r.get::<_, String>(4)?,
    }))).map_err(err500)?.collect::<Result<Vec<_>, _>>().map_err(err500)?;
    Ok(Json(serde_json::json!({ "ok": true, "cuentas": rows })))
}

/// `GET /api/v1/app/st/modelos?marca_id=<id>` — modelos de una marca.
pub async fn st_modelos(
    AxumState(state): AxumState<Arc<ServerState>>,
    headers: HeaderMap,
    Query(qp): Query<std::collections::HashMap<String, String>>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ApiError>)> {
    let _session = extract_app_session(&headers, &state)?;
    let marca_id: i64 = qp.get("marca_id").and_then(|s| s.parse().ok()).unwrap_or(0);
    let conn = state.db.conn.lock().map_err(err500)?;
    let mut stmt = conn.prepare(
        "SELECT id, nombre FROM st_modelos WHERE marca_id = ?1 AND activo = 1 ORDER BY nombre"
    ).map_err(err500)?;
    let rows: Vec<serde_json::Value> = stmt.query_map(params![marca_id], |r| Ok(serde_json::json!({
        "id": r.get::<_, i64>(0)?, "nombre": r.get::<_, String>(1)?,
    }))).map_err(err500)?.collect::<Result<Vec<_>, _>>().map_err(err500)?;
    Ok(Json(serde_json::json!({ "ok": true, "modelos": rows })))
}

/// `POST /api/v1/app/clientes` — crea un cliente (INSERT directo a BD).
/// Body: `{ tipo_identificacion, identificacion, nombre, email?, telefono?, direccion? }`
/// Si la identificación ya existe, lo busca y devuelve el existente (idempotente).
pub async fn crear_cliente(
    AxumState(state): AxumState<Arc<ServerState>>,
    headers: HeaderMap,
    Json(body): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ApiError>)> {
    let session = extract_app_session(&headers, &state)?;
    if !session.tiene("gestionar_clientes") && !session.tiene("vende_piso") {
        return Err((StatusCode::FORBIDDEN, Json(ApiError::new("Falta permiso gestionar_clientes o vende_piso"))));
    }

    let tipo_id = body.get("tipo_identificacion").and_then(|v| v.as_str()).unwrap_or("CEDULA").to_string();
    let identificacion = body.get("identificacion").and_then(|v| v.as_str()).map(|s| s.to_string());
    let nombre = body.get("nombre").and_then(|v| v.as_str()).unwrap_or("").trim().to_string();
    if nombre.is_empty() {
        return Err(err400("El nombre es obligatorio"));
    }
    let email = body.get("email").and_then(|v| v.as_str()).map(|s| s.to_string());
    let telefono = body.get("telefono").and_then(|v| v.as_str()).map(|s| s.to_string());
    let direccion = body.get("direccion").and_then(|v| v.as_str()).map(|s| s.to_string());

    let conn = state.db.conn.lock().map_err(err500)?;

    // Si ya existe por identificación, devolver el existente
    if let Some(ref id_str) = identificacion {
        if !id_str.is_empty() {
            if let Ok(existente_id) = conn.query_row(
                "SELECT id FROM clientes WHERE identificacion = ?1 LIMIT 1",
                params![id_str],
                |r| r.get::<_, i64>(0),
            ) {
                return Ok(Json(serde_json::json!({
                    "ok": true,
                    "id": existente_id,
                    "existente": true,
                    "mensaje": "Cliente ya existente, se devuelve el ID actual",
                })));
            }
        }
    }

    conn.execute(
        "INSERT INTO clientes (tipo_identificacion, identificacion, nombre, email, telefono, direccion, activo)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, 1)",
        params![tipo_id, identificacion, nombre, email, telefono, direccion],
    ).map_err(err500)?;
    let id = conn.last_insert_rowid();

    Ok(Json(serde_json::json!({
        "ok": true,
        "id": id,
        "existente": false,
    })))
}

// ─── v2.5.50 — Caja (estado, abrir, cerrar desde la app) ────────────────────

/// `GET /api/v1/app/caja/estado` — devuelve si hay caja abierta, monto inicial,
/// ventas del turno, monto esperado.
pub async fn caja_estado(
    AxumState(state): AxumState<Arc<ServerState>>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ApiError>)> {
    let _session = extract_app_session(&headers, &state)?;
    let resultado = crate::server::dispatch::dispatch_command(
        &state, "obtener_caja_abierta", serde_json::json!({})
    ).await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(ApiError::new(e))))?;
    Ok(Json(serde_json::json!({ "ok": true, "caja": resultado })))
}

#[derive(Debug, Deserialize)]
pub struct AbrirCajaRequest {
    pub monto_inicial: f64,
    pub observacion: Option<String>,
}

/// `POST /api/v1/app/caja/abrir` — abre una caja con monto inicial.
/// Requiere permiso `abre_caja` o `cobra_caja`.
pub async fn caja_abrir(
    AxumState(state): AxumState<Arc<ServerState>>,
    headers: HeaderMap,
    Json(req): Json<AbrirCajaRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ApiError>)> {
    let session = extract_app_session(&headers, &state)?;
    if !session.tiene("abre_caja") && !session.tiene("cobra_caja") {
        return Err((StatusCode::FORBIDDEN, Json(ApiError::new("Falta permiso abre_caja o cobra_caja"))));
    }
    let resultado = crate::server::dispatch::dispatch_command(
        &state, "abrir_caja", serde_json::json!({
            "montoInicial": req.monto_inicial,
            "observacion": req.observacion,
        })
    ).await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(ApiError::new(e))))?;
    Ok(Json(serde_json::json!({ "ok": true, "resultado": resultado })))
}

#[derive(Debug, Deserialize)]
pub struct CerrarCajaRequest {
    pub monto_real: f64,
    pub observacion: Option<String>,
}

/// `POST /api/v1/app/caja/cerrar` — cierra la caja activa.
/// Implementación inline: UPDATE caja con monto_real, diferencia, fecha_cierre.
pub async fn caja_cerrar(
    AxumState(state): AxumState<Arc<ServerState>>,
    headers: HeaderMap,
    Json(req): Json<CerrarCajaRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ApiError>)> {
    let session = extract_app_session(&headers, &state)?;
    if !session.tiene("cierra_caja") && !session.tiene("cobra_caja") && !session.tiene("abre_caja") {
        return Err((StatusCode::FORBIDDEN, Json(ApiError::new("Falta permiso cierra_caja o cobra_caja"))));
    }

    let conn = state.db.conn.lock().map_err(err500)?;

    // Obtener la caja abierta + recalcular monto_esperado en base a ventas reales
    let (caja_id, monto_inicial): (i64, f64) = conn.query_row(
        "SELECT id, monto_inicial FROM caja WHERE estado = 'ABIERTA' ORDER BY id DESC LIMIT 1",
        [],
        |r| Ok((r.get(0)?, r.get(1)?)),
    ).map_err(|_| err400("No hay caja abierta para cerrar"))?;

    // Sumar ventas en efectivo del turno (forma_pago=EFECTIVO y no anuladas)
    let monto_ventas: f64 = conn.query_row(
        "SELECT COALESCE(SUM(total), 0) FROM ventas
         WHERE forma_pago = 'EFECTIVO' AND anulada = 0
           AND fecha >= (SELECT fecha_apertura FROM caja WHERE id = ?1)",
        params![caja_id],
        |r| r.get(0),
    ).unwrap_or(0.0);

    let monto_esperado = monto_inicial + monto_ventas;
    let diferencia = req.monto_real - monto_esperado;

    conn.execute(
        "UPDATE caja SET estado = 'CERRADA',
                         fecha_cierre = datetime('now','localtime'),
                         monto_ventas = ?1,
                         monto_esperado = ?2,
                         monto_real = ?3,
                         diferencia = ?4,
                         observacion = COALESCE(?5, observacion)
         WHERE id = ?6",
        params![monto_ventas, monto_esperado, req.monto_real, diferencia, req.observacion, caja_id],
    ).map_err(err500)?;

    Ok(Json(serde_json::json!({
        "ok": true,
        "caja_id": caja_id,
        "monto_inicial": monto_inicial,
        "monto_ventas": monto_ventas,
        "monto_esperado": monto_esperado,
        "monto_real": req.monto_real,
        "diferencia": diferencia,
    })))
}

// ─── v2.5.50 — Retenciones recibidas (cliente me retiene al pagar) ──────────

/// `GET /api/v1/app/ventas/:id/retenciones` — lista las retenciones recibidas
/// asociadas a una venta. Implementación inline (SELECT directo).
pub async fn ventas_listar_retenciones(
    AxumState(state): AxumState<Arc<ServerState>>,
    headers: HeaderMap,
    Path(venta_id): Path<i64>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ApiError>)> {
    let _session = extract_app_session(&headers, &state)?;
    let conn = state.db.conn.lock().map_err(err500)?;
    let mut stmt = conn.prepare(
        "SELECT id, tipo, codigo_sri, base_imponible, porcentaje, valor,
                numero_comprobante, fecha_emision
         FROM retenciones_recibidas WHERE venta_id = ?1 ORDER BY id"
    ).map_err(err500)?;
    let rows: Vec<serde_json::Value> = stmt.query_map(params![venta_id], |r| {
        Ok(serde_json::json!({
            "id": r.get::<_, i64>(0)?,
            "tipo": r.get::<_, String>(1)?,
            "codigo_sri": r.get::<_, String>(2)?,
            "base_imponible": r.get::<_, f64>(3)?,
            "porcentaje": r.get::<_, f64>(4)?,
            "valor": r.get::<_, f64>(5)?,
            "numero_comprobante": r.get::<_, String>(6)?,
            "fecha_emision": r.get::<_, String>(7)?,
        }))
    }).map_err(err500)?
    .collect::<Result<Vec<_>, _>>()
    .map_err(err500)?;
    Ok(Json(serde_json::json!({ "ok": true, "retenciones": rows })))
}

/// `POST /api/v1/app/ventas/:id/retencion` — registra una retención recibida
/// (cliente me retiene al pagar). Reduce saldo de CXC si corresponde.
/// Payload: `{ tipo: "RENTA"|"IVA", codigo_retencion, base_imponible, porcentaje, valor, numero_comprobante?, fecha_emision? }`
pub async fn ventas_registrar_retencion(
    AxumState(state): AxumState<Arc<ServerState>>,
    headers: HeaderMap,
    Path(venta_id): Path<i64>,
    Json(body): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ApiError>)> {
    let session = extract_app_session(&headers, &state)?;
    if !session.tiene("cobra_caja") && !session.tiene("gestionar_cobranzas") {
        return Err((StatusCode::FORBIDDEN, Json(ApiError::new("Falta permiso cobra_caja o gestionar_cobranzas"))));
    }
    let tipo = body.get("tipo").and_then(|v| v.as_str()).unwrap_or("RENTA").to_string();
    let codigo = body.get("codigo_sri")
        .or_else(|| body.get("codigo_retencion"))
        .and_then(|v| v.as_str()).unwrap_or("").to_string();
    let base: f64 = body.get("base_imponible").and_then(|v| v.as_f64()).unwrap_or(0.0);
    let pct: f64 = body.get("porcentaje").and_then(|v| v.as_f64()).unwrap_or(0.0);
    let valor: f64 = body.get("valor").and_then(|v| v.as_f64()).unwrap_or(0.0);
    // numero_comprobante y fecha_emision son NOT NULL en la tabla — usar defaults
    let numero_comprobante = body.get("numero_comprobante").and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .unwrap_or_else(|| "S/N".to_string());
    let fecha_emision = body.get("fecha_emision").and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .unwrap_or_else(|| chrono::Local::now().format("%Y-%m-%d").to_string());

    if codigo.is_empty() {
        return Err(err400("codigo_sri es obligatorio"));
    }
    if valor <= 0.0 {
        return Err(err400("valor debe ser > 0"));
    }

    let conn = state.db.conn.lock().map_err(err500)?;
    conn.execute(
        "INSERT INTO retenciones_recibidas
            (venta_id, tipo, codigo_sri, base_imponible, porcentaje, valor,
             numero_comprobante, fecha_emision, usuario)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
        params![venta_id, tipo, codigo, base, pct, valor, numero_comprobante, fecha_emision, session.nombre],
    ).map_err(err500)?;
    let id = conn.last_insert_rowid();

    // Si la venta tiene CXC activa, reducir saldo (igual que retención emitida sobre compra)
    let _ = conn.execute(
        "UPDATE cuentas_por_cobrar
         SET saldo = MAX(0, saldo - ?1),
             estado = CASE WHEN saldo - ?1 <= 0.01 THEN 'PAGADA' ELSE 'PENDIENTE' END
         WHERE venta_id = ?2 AND estado != 'ANULADA'",
        params![valor, venta_id],
    );

    Ok(Json(serde_json::json!({
        "ok": true,
        "id": id,
        "mensaje": format!("Retención {} registrada por ${:.2}", tipo, valor),
    })))
}

// ─── v2.5.51 — Emisión SRI desde la app (ACTIVA) ────────────────────────────

/// `POST /api/v1/app/ventas/:id/emitir-sri` — autoriza una venta ya registrada
/// como FACTURA ante el SRI. Genera XML, firma con XAdES-BES, envía via SOAP,
/// consulta autorización y persiste el resultado en la venta.
///
/// Body opcional: `{ forma_pago_credito_sri?: "20"|"21"|... }` para sobreescribir
/// el código de forma de pago SRI cuando la venta es a crédito.
///
/// Requiere permiso `vende_piso` o `cobra_caja`. Valida también la suscripción
/// SRI (mismo enforcement que el POS desktop: trial gratuito + planes pagados).
pub async fn ventas_emitir_sri(
    AxumState(state): AxumState<Arc<ServerState>>,
    headers: HeaderMap,
    Path(venta_id): Path<i64>,
    Json(body): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ApiError>)> {
    let session = extract_app_session(&headers, &state)?;
    if !session.tiene("vende_piso") && !session.tiene("cobra_caja") {
        return Err((StatusCode::FORBIDDEN, Json(ApiError::new("Falta permiso vende_piso o cobra_caja"))));
    }
    let forma_pago_credito_sri = body.get("forma_pago_credito_sri")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());
    let resultado = crate::server::dispatch::dispatch_command(
        &state, "emitir_factura_sri", serde_json::json!({
            "ventaId": venta_id,
            "formaPagoCreditoSri": forma_pago_credito_sri,
        })
    ).await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(ApiError::new(e))))?;
    Ok(Json(serde_json::json!({ "ok": true, "resultado": resultado })))
}

/// `GET /api/v1/app/ventas?fecha=YYYY-MM-DD` — lista las ventas de un día
/// (por defecto hoy). Reusa la lógica del POS (incluye estado_sri, tipo_documento,
/// numero_factura, etc.) para la pantalla de Ventas de la app.
#[derive(Debug, Deserialize)]
pub struct VentasListarQuery {
    pub fecha: Option<String>,
}

pub async fn ventas_listar(
    AxumState(state): AxumState<Arc<ServerState>>,
    headers: HeaderMap,
    axum::extract::Query(q): axum::extract::Query<VentasListarQuery>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ApiError>)> {
    let session = extract_app_session(&headers, &state)?;
    if !session.tiene("vende_piso") && !session.tiene("cobra_caja") && !session.tiene("dueno_dashboard") {
        return Err((StatusCode::FORBIDDEN, Json(ApiError::new("Falta permiso para ver ventas"))));
    }
    let fecha = q.fecha.unwrap_or_else(|| chrono::Local::now().format("%Y-%m-%d").to_string());
    let resultado = crate::server::dispatch::dispatch_command(
        &state, "listar_ventas_dia", serde_json::json!({ "fecha": fecha })
    ).await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(ApiError::new(e))))?;
    Ok(Json(serde_json::json!({ "ok": true, "fecha": fecha, "ventas": resultado })))
}

/// `GET /api/v1/app/sri/estado` — indica si el POS está listo para emitir
/// comprobantes electrónicos (módulo activo + certificado), para que la app
/// muestre u oculte el botón "Autorizar SRI".
pub async fn sri_estado_app(
    AxumState(state): AxumState<Arc<ServerState>>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ApiError>)> {
    let _session = extract_app_session(&headers, &state)?;
    let conn = state.db.conn.lock().map_err(err500)?;
    let cfg = |k: &str| -> String {
        conn.query_row("SELECT value FROM config WHERE key = ?1", [k], |r| r.get(0)).unwrap_or_default()
    };
    let modulo_activo = cfg("sri_modulo_activo") == "1";
    let certificado_cargado = cfg("sri_certificado_cargado") == "1";
    let ambiente = cfg("sri_ambiente");
    let suscripcion_autorizada = cfg("sri_suscripcion_autorizado") == "1";
    // El POS puede emitir si el módulo está activo y el certificado cargado.
    // La cuota/suscripción se valida realmente al emitir (server-side).
    let puede_emitir = modulo_activo && certificado_cargado;
    Ok(Json(serde_json::json!({
        "ok": true,
        "puede_emitir": puede_emitir,
        "modulo_activo": modulo_activo,
        "certificado_cargado": certificado_cargado,
        "suscripcion_autorizada": suscripcion_autorizada,
        "ambiente": ambiente,
    })))
}

// ─── v2.5.52 — Proveedores (listar/crear/obtener) ───────────────────────────

#[derive(Debug, Deserialize)]
pub struct ListarProveedoresQuery {
    pub q: Option<String>,
    pub limite: Option<i64>,
}

/// `GET /api/v1/app/proveedores?q=&limite=` — lista proveedores activos.
/// Búsqueda por nombre/RUC/email. Default 100, máx 500.
pub async fn listar_proveedores(
    AxumState(state): AxumState<Arc<ServerState>>,
    headers: HeaderMap,
    Query(qp): Query<ListarProveedoresQuery>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ApiError>)> {
    let _session = extract_app_session(&headers, &state)?;
    let limite = qp.limite.unwrap_or(100).clamp(1, 500);
    let conn = state.db.conn.lock().map_err(err500)?;
    let busqueda = qp.q.unwrap_or_default();
    let busqueda_pat = format!("%{}%", busqueda);

    let (sql, params_dyn): (&str, Vec<Box<dyn rusqlite::ToSql>>) = if busqueda.is_empty() {
        (
            "SELECT id, COALESCE(ruc, ''), nombre, COALESCE(email, ''),
                    COALESCE(telefono, ''), COALESCE(direccion, ''), dias_credito
             FROM proveedores WHERE COALESCE(activo, 1) = 1 ORDER BY nombre LIMIT ?1",
            vec![Box::new(limite)],
        )
    } else {
        (
            "SELECT id, COALESCE(ruc, ''), nombre, COALESCE(email, ''),
                    COALESCE(telefono, ''), COALESCE(direccion, ''), dias_credito
             FROM proveedores WHERE COALESCE(activo, 1) = 1
               AND (nombre LIKE ?1 OR ruc LIKE ?1 OR email LIKE ?1)
             ORDER BY nombre LIMIT ?2",
            vec![Box::new(busqueda_pat), Box::new(limite)],
        )
    };

    let mut stmt = conn.prepare(sql).map_err(err500)?;
    let params_refs: Vec<&dyn rusqlite::ToSql> = params_dyn.iter().map(|b| b.as_ref()).collect();
    let proveedores: Vec<serde_json::Value> = stmt
        .query_map(rusqlite::params_from_iter(params_refs.iter()), |r| {
            Ok(serde_json::json!({
                "id": r.get::<_, i64>(0)?,
                "ruc": r.get::<_, String>(1)?,
                "nombre": r.get::<_, String>(2)?,
                "email": r.get::<_, String>(3)?,
                "telefono": r.get::<_, String>(4)?,
                "direccion": r.get::<_, String>(5)?,
                "dias_credito": r.get::<_, i64>(6)?,
            }))
        }).map_err(err500)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(err500)?;

    Ok(Json(serde_json::json!({
        "ok": true, "proveedores": proveedores, "total": proveedores.len(),
    })))
}

/// `GET /api/v1/app/proveedores/:id` — un proveedor.
pub async fn obtener_proveedor(
    AxumState(state): AxumState<Arc<ServerState>>,
    headers: HeaderMap,
    Path(id): Path<i64>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ApiError>)> {
    let _session = extract_app_session(&headers, &state)?;
    let conn = state.db.conn.lock().map_err(err500)?;
    let prov: serde_json::Value = conn.query_row(
        "SELECT id, COALESCE(ruc, ''), nombre, COALESCE(email, ''),
                COALESCE(telefono, ''), COALESCE(direccion, ''),
                COALESCE(contacto, ''), dias_credito
         FROM proveedores WHERE id = ?1",
        params![id],
        |r| Ok(serde_json::json!({
            "id": r.get::<_, i64>(0)?,
            "ruc": r.get::<_, String>(1)?,
            "nombre": r.get::<_, String>(2)?,
            "email": r.get::<_, String>(3)?,
            "telefono": r.get::<_, String>(4)?,
            "direccion": r.get::<_, String>(5)?,
            "contacto": r.get::<_, String>(6)?,
            "dias_credito": r.get::<_, i64>(7)?,
        })),
    ).map_err(|_| err400("Proveedor no encontrado"))?;
    Ok(Json(serde_json::json!({ "ok": true, "proveedor": prov })))
}

/// `POST /api/v1/app/proveedores` — crea un proveedor. Idempotente por RUC.
pub async fn crear_proveedor(
    AxumState(state): AxumState<Arc<ServerState>>,
    headers: HeaderMap,
    Json(body): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ApiError>)> {
    let session = extract_app_session(&headers, &state)?;
    if !session.tiene("gestionar_compras") && !session.tiene("vende_piso") {
        return Err((StatusCode::FORBIDDEN, Json(ApiError::new("Falta permiso gestionar_compras o vende_piso"))));
    }
    let nombre = body.get("nombre").and_then(|v| v.as_str()).unwrap_or("").trim().to_string();
    if nombre.is_empty() {
        return Err(err400("El nombre es obligatorio"));
    }
    let ruc = body.get("ruc").and_then(|v| v.as_str()).map(|s| s.to_string());
    let email = body.get("email").and_then(|v| v.as_str()).map(|s| s.to_string());
    let telefono = body.get("telefono").and_then(|v| v.as_str()).map(|s| s.to_string());
    let direccion = body.get("direccion").and_then(|v| v.as_str()).map(|s| s.to_string());
    let contacto = body.get("contacto").and_then(|v| v.as_str()).map(|s| s.to_string());
    let dias_credito = body.get("dias_credito").and_then(|v| v.as_i64()).unwrap_or(0);

    let conn = state.db.conn.lock().map_err(err500)?;

    if let Some(ref r) = ruc {
        if !r.is_empty() {
            if let Ok(existente_id) = conn.query_row(
                "SELECT id FROM proveedores WHERE ruc = ?1 LIMIT 1",
                params![r],
                |r| r.get::<_, i64>(0),
            ) {
                return Ok(Json(serde_json::json!({
                    "ok": true, "id": existente_id, "existente": true,
                    "mensaje": "Proveedor ya existente, se devuelve el ID actual",
                })));
            }
        }
    }

    conn.execute(
        "INSERT INTO proveedores (ruc, nombre, email, telefono, direccion, contacto, dias_credito, activo)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 1)",
        params![ruc, nombre, email, telefono, direccion, contacto, dias_credito],
    ).map_err(err500)?;
    let id = conn.last_insert_rowid();
    Ok(Json(serde_json::json!({ "ok": true, "id": id, "existente": false })))
}

// ─── v2.5.52 — Compras (listar + crear básica INFORMAL desde la app) ────────

#[derive(Debug, Deserialize)]
pub struct ListarComprasQuery {
    pub desde: Option<String>,    // YYYY-MM-DD
    pub hasta: Option<String>,
    pub proveedor_id: Option<i64>,
    pub limite: Option<i64>,
}

/// `GET /api/v1/app/compras?desde=&hasta=&proveedor_id=&limite=` — lista compras.
pub async fn listar_compras(
    AxumState(state): AxumState<Arc<ServerState>>,
    headers: HeaderMap,
    Query(qp): Query<ListarComprasQuery>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ApiError>)> {
    let _session = extract_app_session(&headers, &state)?;
    let limite = qp.limite.unwrap_or(100).clamp(1, 500);
    let desde = qp.desde.unwrap_or_else(|| "1970-01-01".to_string());
    let hasta = qp.hasta.unwrap_or_else(|| "2999-12-31".to_string());
    let conn = state.db.conn.lock().map_err(err500)?;

    let mut sql = String::from(
        "SELECT c.id, c.numero, c.fecha, COALESCE(c.numero_factura, ''),
                c.subtotal, c.iva, c.total, c.forma_pago, c.estado,
                c.tipo_documento, COALESCE(c.estado_sri, ''),
                c.proveedor_id, p.nombre, COALESCE(p.ruc, '')
         FROM compras c
         JOIN proveedores p ON c.proveedor_id = p.id
         WHERE date(COALESCE(c.fecha_emision, c.fecha)) >= date(?1)
           AND date(COALESCE(c.fecha_emision, c.fecha)) <= date(?2)"
    );
    let mut params_dyn: Vec<Box<dyn rusqlite::ToSql>> = vec![Box::new(desde), Box::new(hasta)];
    if let Some(pid) = qp.proveedor_id {
        sql.push_str(" AND c.proveedor_id = ?3");
        params_dyn.push(Box::new(pid));
    }
    sql.push_str(" ORDER BY c.fecha DESC LIMIT ?");
    sql.push_str(&(params_dyn.len() + 1).to_string());
    params_dyn.push(Box::new(limite));

    let mut stmt = conn.prepare(&sql).map_err(err500)?;
    let params_refs: Vec<&dyn rusqlite::ToSql> = params_dyn.iter().map(|b| b.as_ref()).collect();
    let compras: Vec<serde_json::Value> = stmt
        .query_map(rusqlite::params_from_iter(params_refs.iter()), |r| {
            Ok(serde_json::json!({
                "id": r.get::<_, i64>(0)?,
                "numero": r.get::<_, String>(1)?,
                "fecha": r.get::<_, String>(2)?,
                "numero_factura": r.get::<_, String>(3)?,
                "subtotal": r.get::<_, f64>(4)?,
                "iva": r.get::<_, f64>(5)?,
                "total": r.get::<_, f64>(6)?,
                "forma_pago": r.get::<_, String>(7)?,
                "estado": r.get::<_, String>(8)?,
                "tipo_documento": r.get::<_, String>(9)?,
                "estado_sri": r.get::<_, String>(10)?,
                "proveedor_id": r.get::<_, i64>(11)?,
                "proveedor_nombre": r.get::<_, String>(12)?,
                "proveedor_ruc": r.get::<_, String>(13)?,
            }))
        }).map_err(err500)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(err500)?;

    Ok(Json(serde_json::json!({
        "ok": true, "compras": compras, "total": compras.len(),
    })))
}

/// `POST /api/v1/app/compras` — registra una compra INFORMAL básica (cabecera).
/// Para registro rápido desde la calle. Sin items detallados (el desktop maneja
/// las compras formales con kardex + IVA por item).
///
/// Body: `{ proveedor_id, total, forma_pago?, observacion?, fecha? }`
pub async fn crear_compra_simple(
    AxumState(state): AxumState<Arc<ServerState>>,
    headers: HeaderMap,
    Json(body): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ApiError>)> {
    let session = extract_app_session(&headers, &state)?;
    if !session.tiene("gestionar_compras") && !session.tiene("vende_piso") {
        return Err((StatusCode::FORBIDDEN, Json(ApiError::new("Falta permiso gestionar_compras o vende_piso"))));
    }
    let proveedor_id = body.get("proveedor_id").and_then(|v| v.as_i64())
        .ok_or_else(|| err400("proveedor_id es obligatorio"))?;
    let total: f64 = body.get("total").and_then(|v| v.as_f64())
        .ok_or_else(|| err400("total es obligatorio"))?;
    if total <= 0.0 {
        return Err(err400("total debe ser > 0"));
    }
    let forma_pago = body.get("forma_pago").and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .unwrap_or_else(|| "EFECTIVO".to_string());
    let observacion = body.get("observacion").and_then(|v| v.as_str()).map(|s| s.to_string());
    let es_credito = forma_pago.eq_ignore_ascii_case("CREDITO");

    let conn = state.db.conn.lock().map_err(err500)?;

    // Generar número interno COMP-XXXXXXXXX
    let next: i64 = conn.query_row(
        "SELECT COALESCE(CAST(value AS INTEGER), 1) FROM config WHERE key = 'secuencial_compra'",
        [], |r| r.get(0),
    ).unwrap_or(1);
    let _ = conn.execute(
        "INSERT OR IGNORE INTO config (key, value) VALUES ('secuencial_compra', ?1)",
        params![next.to_string()],
    );
    let _ = conn.execute(
        "UPDATE config SET value = CAST(?1 AS TEXT) WHERE key = 'secuencial_compra'",
        params![next + 1],
    );
    let numero = format!("COMP-{:09}", next);

    conn.execute(
        "INSERT INTO compras
            (numero, proveedor_id, subtotal, iva, total, forma_pago, es_credito,
             observacion, tipo_documento, estado, usuario)
         VALUES (?1, ?2, ?3, 0, ?3, ?4, ?5, ?6, 'INFORMAL', 'REGISTRADA', ?7)",
        params![numero, proveedor_id, total, forma_pago, es_credito as i32, observacion, session.nombre],
    ).map_err(err500)?;
    let id = conn.last_insert_rowid();

    // Si es a crédito, crear CXP
    if es_credito {
        let _ = conn.execute(
            "INSERT INTO cuentas_por_pagar (proveedor_id, compra_id, monto_total, saldo, estado)
             VALUES (?1, ?2, ?3, ?3, 'PENDIENTE')",
            params![proveedor_id, id, total],
        );
    }

    Ok(Json(serde_json::json!({
        "ok": true,
        "id": id,
        "numero": numero,
        "total": total,
    })))
}

/// `GET /api/v1/app/compras/:id` — detalle de una compra (cabecera).
pub async fn obtener_compra(
    AxumState(state): AxumState<Arc<ServerState>>,
    headers: HeaderMap,
    Path(id): Path<i64>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ApiError>)> {
    let _session = extract_app_session(&headers, &state)?;
    let conn = state.db.conn.lock().map_err(err500)?;
    let compra: serde_json::Value = conn.query_row(
        "SELECT c.id, c.numero, c.fecha, COALESCE(c.numero_factura, ''),
                c.subtotal, c.iva, c.total, c.forma_pago, c.estado,
                c.tipo_documento, COALESCE(c.observacion, ''),
                c.proveedor_id, p.nombre, COALESCE(p.ruc, '')
         FROM compras c JOIN proveedores p ON c.proveedor_id = p.id
         WHERE c.id = ?1",
        params![id],
        |r| Ok(serde_json::json!({
            "id": r.get::<_, i64>(0)?,
            "numero": r.get::<_, String>(1)?,
            "fecha": r.get::<_, String>(2)?,
            "numero_factura": r.get::<_, String>(3)?,
            "subtotal": r.get::<_, f64>(4)?,
            "iva": r.get::<_, f64>(5)?,
            "total": r.get::<_, f64>(6)?,
            "forma_pago": r.get::<_, String>(7)?,
            "estado": r.get::<_, String>(8)?,
            "tipo_documento": r.get::<_, String>(9)?,
            "observacion": r.get::<_, String>(10)?,
            "proveedor_id": r.get::<_, i64>(11)?,
            "proveedor_nombre": r.get::<_, String>(12)?,
            "proveedor_ruc": r.get::<_, String>(13)?,
        })),
    ).map_err(|_| err400("Compra no encontrada"))?;

    // Items (si hay)
    let mut stmt_items = conn.prepare(
        "SELECT id, COALESCE(producto_id, 0), COALESCE(descripcion, ''),
                cantidad, precio_unitario, subtotal
         FROM compra_detalles WHERE compra_id = ?1 ORDER BY id"
    ).map_err(err500)?;
    let items: Vec<serde_json::Value> = stmt_items.query_map(params![id], |r| {
        Ok(serde_json::json!({
            "id": r.get::<_, i64>(0)?,
            "producto_id": r.get::<_, i64>(1)?,
            "descripcion": r.get::<_, String>(2)?,
            "cantidad": r.get::<_, f64>(3)?,
            "precio_unitario": r.get::<_, f64>(4)?,
            "subtotal": r.get::<_, f64>(5)?,
        }))
    }).map_err(err500)?
    .collect::<Result<Vec<_>, _>>()
    .map_err(err500)?;

    Ok(Json(serde_json::json!({
        "ok": true, "compra": compra, "items": items,
    })))
}

// ─── v2.5.53 — Dashboard KPIs del día ────────────────────────────────────────

/// `GET /api/v1/app/dashboard/hoy` — KPIs rápidos para el dueño:
/// ventas del día, ticket promedio, top 5 productos, estado caja,
/// fiados pendientes, stock crítico.
pub async fn dashboard_hoy(
    AxumState(state): AxumState<Arc<ServerState>>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ApiError>)> {
    let _session = extract_app_session(&headers, &state)?;
    let conn = state.db.conn.lock().map_err(err500)?;

    // Ventas del día (no anuladas)
    let (ventas_count, ventas_total, ventas_iva): (i64, f64, f64) = conn.query_row(
        "SELECT COUNT(*), COALESCE(SUM(total), 0), COALESCE(SUM(iva), 0)
         FROM ventas WHERE date(fecha) = date('now', 'localtime') AND anulada = 0",
        [], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
    ).unwrap_or((0, 0.0, 0.0));

    let ticket_promedio = if ventas_count > 0 { ventas_total / ventas_count as f64 } else { 0.0 };

    // Ventas por forma de pago (hoy)
    let mut stmt_fp = conn.prepare(
        "SELECT forma_pago, COUNT(*), COALESCE(SUM(total), 0)
         FROM ventas WHERE date(fecha) = date('now', 'localtime') AND anulada = 0
         GROUP BY forma_pago ORDER BY 3 DESC"
    ).map_err(err500)?;
    let formas_pago: Vec<serde_json::Value> = stmt_fp.query_map([], |r| {
        Ok(serde_json::json!({
            "forma_pago": r.get::<_, String>(0)?,
            "count": r.get::<_, i64>(1)?,
            "total": r.get::<_, f64>(2)?,
        }))
    }).map_err(err500)?.filter_map(Result::ok).collect();
    drop(stmt_fp);

    // Top 5 productos del día
    let mut stmt_top = conn.prepare(
        "SELECT COALESCE(p.nombre, vd.observacion, 'Producto') as nombre,
                SUM(vd.cantidad) as unidades,
                SUM(vd.cantidad * vd.precio_unitario - COALESCE(vd.descuento, 0)) as importe
         FROM venta_detalles vd
         JOIN ventas v ON vd.venta_id = v.id
         LEFT JOIN productos p ON vd.producto_id = p.id
         WHERE date(v.fecha) = date('now', 'localtime') AND v.anulada = 0
         GROUP BY COALESCE(p.id, -vd.id)
         ORDER BY unidades DESC
         LIMIT 5"
    ).map_err(err500)?;
    let top_productos: Vec<serde_json::Value> = stmt_top.query_map([], |r| {
        Ok(serde_json::json!({
            "nombre": r.get::<_, String>(0)?,
            "unidades": r.get::<_, f64>(1)?,
            "importe": r.get::<_, f64>(2)?,
        }))
    }).map_err(err500)?.filter_map(Result::ok).collect();
    drop(stmt_top);

    // Estado caja
    let caja: Option<serde_json::Value> = conn.query_row(
        "SELECT id, fecha_apertura, monto_inicial, monto_ventas, monto_esperado, usuario
         FROM caja WHERE estado = 'ABIERTA' ORDER BY id DESC LIMIT 1",
        [], |r| Ok(serde_json::json!({
            "id": r.get::<_, i64>(0)?,
            "fecha_apertura": r.get::<_, String>(1)?,
            "monto_inicial": r.get::<_, f64>(2)?,
            "monto_ventas": r.get::<_, f64>(3)?,
            "monto_esperado": r.get::<_, f64>(4)?,
            "usuario": r.get::<_, Option<String>>(5)?,
        })),
    ).ok();

    // Fiados pendientes (CXC abiertas)
    let (cxc_count, cxc_total): (i64, f64) = conn.query_row(
        "SELECT COUNT(*), COALESCE(SUM(saldo), 0)
         FROM cuentas_por_cobrar WHERE estado != 'PAGADA' AND estado != 'ANULADA'",
        [], |r| Ok((r.get(0)?, r.get(1)?)),
    ).unwrap_or((0, 0.0));

    // Stock crítico (productos con stock <= stock_minimo > 0)
    let stock_critico: i64 = conn.query_row(
        "SELECT COUNT(*) FROM productos
         WHERE activo = 1 AND COALESCE(no_controla_stock, 0) = 0
           AND stock_minimo > 0 AND stock_actual <= stock_minimo",
        [], |r| r.get(0),
    ).unwrap_or(0);

    // Comparación vs ayer (ventas)
    let ayer_total: f64 = conn.query_row(
        "SELECT COALESCE(SUM(total), 0)
         FROM ventas WHERE date(fecha) = date('now', '-1 day', 'localtime') AND anulada = 0",
        [], |r| r.get(0),
    ).unwrap_or(0.0);
    let diferencia_pct = if ayer_total > 0.01 {
        ((ventas_total - ayer_total) / ayer_total) * 100.0
    } else { 0.0 };

    Ok(Json(serde_json::json!({
        "ok": true,
        "ventas_hoy": {
            "count": ventas_count,
            "total": ventas_total,
            "iva": ventas_iva,
            "ticket_promedio": ticket_promedio,
            "vs_ayer_pct": diferencia_pct,
            "ayer_total": ayer_total,
        },
        "formas_pago": formas_pago,
        "top_productos": top_productos,
        "caja": caja,
        "cxc": { "count": cxc_count, "total": cxc_total },
        "stock_critico_count": stock_critico,
    })))
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
        .route("/api/v1/app/auth/password", post(auth_password))
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
        .route("/api/v1/app/pedidos/:id/abono", post(pedidos_abono))
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
        .route("/api/v1/app/ventas", post(ventas_registrar).get(ventas_listar))
        // ── v2.5.50: Clientes CRUD básico ───────────────────────────────
        .route("/api/v1/app/clientes", get(listar_clientes).post(crear_cliente))
        .route("/api/v1/app/clientes/:id", get(obtener_cliente))
        .route("/api/v1/app/consultar-identificacion", get(consultar_identificacion_app))
        .route("/api/v1/app/st/tipos-equipo", get(st_tipos_equipo))
        .route("/api/v1/app/st/marcas", get(st_marcas))
        .route("/api/v1/app/st/modelos", get(st_modelos))
        .route("/api/v1/app/cuentas-banco", get(cuentas_banco_app))
        // ── v2.5.50: Caja (estado/abrir/cerrar) ─────────────────────────
        .route("/api/v1/app/caja/estado", get(caja_estado))
        .route("/api/v1/app/caja/abrir", post(caja_abrir))
        .route("/api/v1/app/caja/cerrar", post(caja_cerrar))
        // ── v2.5.50: Emisión SRI + retenciones recibidas ────────────────
        .route("/api/v1/app/sri/estado", get(sri_estado_app))
        .route("/api/v1/app/ventas/:id/emitir-sri", post(ventas_emitir_sri))
        .route("/api/v1/app/ventas/:id/retencion", post(ventas_registrar_retencion))
        .route("/api/v1/app/ventas/:id/retenciones", get(ventas_listar_retenciones))
        // ── v2.5.52: Proveedores ────────────────────────────────────────
        .route("/api/v1/app/proveedores", get(listar_proveedores).post(crear_proveedor))
        .route("/api/v1/app/proveedores/:id", get(obtener_proveedor))
        // ── v2.5.52: Compras (listar/crear INFORMAL/detalle) ────────────
        .route("/api/v1/app/compras", get(listar_compras).post(crear_compra_simple))
        .route("/api/v1/app/compras/:id", get(obtener_compra))
        // ── v2.5.53: Dashboard KPIs del día ─────────────────────────────
        .route("/api/v1/app/dashboard/hoy", get(dashboard_hoy))
        // ── Servicio Técnico (Sprint 6.4 — técnico móvil) ───────────────
        .route("/api/v1/app/st/mis-ordenes", get(super::http_st::st_mis_ordenes))
        .route("/api/v1/app/st/ordenes", post(super::http_st::st_crear_orden))
        .route("/api/v1/app/st/ordenes/:id", get(super::http_st::st_obtener_orden))
        .route("/api/v1/app/st/ordenes/:id/estado", post(super::http_st::st_cambiar_estado))
        .route("/api/v1/app/st/ordenes/:id/diagnostico", post(super::http_st::st_guardar_diagnostico))
        .route("/api/v1/app/st/ordenes/:id/imagen", post(super::http_st::st_subir_imagen))
}
