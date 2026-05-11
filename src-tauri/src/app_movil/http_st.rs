//! Endpoints HTTP del módulo Servicio Técnico para la app móvil.
//!
//! Diseñado para el técnico de taller con su celular: ver órdenes asignadas,
//! actualizar estado/diagnóstico, subir fotos del equipo.
//!
//! Permiso requerido: `gestionar_servicio_tecnico` (o admin). Si el usuario
//! tiene asignación específica (tecnico_id), solo ve esas órdenes; si es
//! admin/coordinador ve todas.
//!
//! Endpoints:
//!  - `GET    /api/v1/app/st/mis-ordenes`              — lista órdenes activas del técnico
//!  - `GET    /api/v1/app/st/ordenes/:id`              — detalle de una orden
//!  - `POST   /api/v1/app/st/ordenes/:id/estado`       — cambia estado + observación
//!  - `POST   /api/v1/app/st/ordenes/:id/diagnostico`  — guarda diagnóstico/trabajo
//!  - `POST   /api/v1/app/st/ordenes/:id/imagen`       — sube imagen base64 (antes/después/general)

use super::http::{err400, err500, extract_app_session, ApiError};
use crate::server::state::ServerState;
use axum::{
    extract::{Path, State as AxumState},
    http::{HeaderMap, StatusCode},
    Json,
};
use rusqlite::params;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

// ─── Helpers ─────────────────────────────────────────────────────────────

fn requiere_modulo_st(state: &Arc<ServerState>) -> Result<(), (StatusCode, Json<ApiError>)> {
    if let Err(msg) = crate::commands::servicio_tecnico::requiere_modulo_servicio_tecnico(&state.db) {
        return Err((StatusCode::FORBIDDEN, Json(ApiError::new(msg))));
    }
    Ok(())
}

// ─── Tipos ───────────────────────────────────────────────────────────────

#[derive(Debug, Serialize)]
pub struct OrdenResumen {
    pub id: i64,
    pub numero: String,
    pub estado: String,
    pub cliente_nombre: Option<String>,
    pub cliente_telefono: Option<String>,
    pub equipo_descripcion: String,
    pub equipo_marca: Option<String>,
    pub equipo_modelo: Option<String>,
    pub equipo_serie: Option<String>,
    pub equipo_placa: Option<String>,
    pub problema_reportado: String,
    pub presupuesto: f64,
    pub monto_final: f64,
    pub fecha_ingreso: String,
    pub fecha_promesa: Option<String>,
    pub tecnico_id: Option<i64>,
    pub tecnico_nombre: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct OrdenDetalle {
    #[serde(flatten)]
    pub resumen: OrdenResumen,
    pub diagnostico: Option<String>,
    pub trabajo_realizado: Option<String>,
    pub observaciones: Option<String>,
    pub accesorios: Option<String>,
    pub garantia_dias: i64,
    pub imagenes: Vec<ImagenOrden>,
}

#[derive(Debug, Serialize)]
pub struct ImagenOrden {
    pub id: i64,
    pub tipo: String,
    pub imagen_base64: String,
    pub descripcion: Option<String>,
}

// ─── GET /api/v1/app/st/mis-ordenes ──────────────────────────────────────

pub async fn st_mis_ordenes(
    AxumState(state): AxumState<Arc<ServerState>>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ApiError>)> {
    let session = extract_app_session(&headers, &state)?;
    requiere_modulo_st(&state)?;

    // Permiso: gestionar_servicio_tecnico O ver_servicio_tecnico
    if !session.tiene("gestionar_servicio_tecnico") && !session.tiene("ver_servicio_tecnico") {
        return Err((StatusCode::FORBIDDEN, Json(ApiError::new("Sin permiso de servicio técnico"))));
    }

    let conn = state.db.conn.lock().map_err(err500)?;

    // Si NO es admin/coordinador → solo SUS órdenes asignadas (tecnico_id = self)
    // Si es admin → todas las activas (no entregadas/canceladas)
    let solo_propias = !session.rol.eq_ignore_ascii_case("ADMIN")
        && !session.tiene("coordina_servicio_tecnico");

    let ordenes: Vec<OrdenResumen> = {
        let sql = if solo_propias {
            "SELECT id, numero, estado, cliente_nombre, cliente_telefono,
                    equipo_descripcion, equipo_marca, equipo_modelo, equipo_serie, equipo_placa,
                    problema_reportado, COALESCE(presupuesto,0), COALESCE(monto_final,0),
                    fecha_ingreso, fecha_promesa, tecnico_id, tecnico_nombre
             FROM ordenes_servicio
             WHERE tecnico_id = ?1
               AND estado NOT IN ('ENTREGADO', 'ENTREGADO_PARCIAL', 'CANCELADA', 'CANCELADO')
             ORDER BY
               CASE estado
                 WHEN 'EN_REPARACION' THEN 1
                 WHEN 'DIAGNOSTICANDO' THEN 2
                 WHEN 'ESPERANDO_REPUESTOS' THEN 3
                 WHEN 'RECIBIDO' THEN 4
                 WHEN 'LISTO' THEN 5
                 ELSE 6
               END,
               fecha_promesa ASC NULLS LAST,
               id DESC"
        } else {
            "SELECT id, numero, estado, cliente_nombre, cliente_telefono,
                    equipo_descripcion, equipo_marca, equipo_modelo, equipo_serie, equipo_placa,
                    problema_reportado, COALESCE(presupuesto,0), COALESCE(monto_final,0),
                    fecha_ingreso, fecha_promesa, tecnico_id, tecnico_nombre
             FROM ordenes_servicio
             WHERE estado NOT IN ('ENTREGADO', 'ENTREGADO_PARCIAL', 'CANCELADA', 'CANCELADO')
             ORDER BY
               CASE estado
                 WHEN 'EN_REPARACION' THEN 1
                 WHEN 'DIAGNOSTICANDO' THEN 2
                 WHEN 'ESPERANDO_REPUESTOS' THEN 3
                 WHEN 'RECIBIDO' THEN 4
                 WHEN 'LISTO' THEN 5
                 ELSE 6
               END,
               fecha_promesa ASC NULLS LAST,
               id DESC"
        };
        let mut stmt = conn.prepare(sql).map_err(err500)?;
        let mapper = |r: &rusqlite::Row| -> rusqlite::Result<OrdenResumen> {
            Ok(OrdenResumen {
                id: r.get(0)?, numero: r.get(1)?, estado: r.get(2)?,
                cliente_nombre: r.get(3)?, cliente_telefono: r.get(4)?,
                equipo_descripcion: r.get(5)?,
                equipo_marca: r.get(6)?, equipo_modelo: r.get(7)?,
                equipo_serie: r.get(8)?, equipo_placa: r.get(9)?,
                problema_reportado: r.get(10)?,
                presupuesto: r.get(11)?, monto_final: r.get(12)?,
                fecha_ingreso: r.get(13)?, fecha_promesa: r.get(14)?,
                tecnico_id: r.get(15)?, tecnico_nombre: r.get(16)?,
            })
        };
        let rows = if solo_propias {
            stmt.query_map(params![session.usuario_id], mapper)
                .map_err(err500)?.collect::<Result<Vec<_>, _>>().map_err(err500)?
        } else {
            stmt.query_map([], mapper)
                .map_err(err500)?.collect::<Result<Vec<_>, _>>().map_err(err500)?
        };
        rows
    };

    Ok(Json(serde_json::json!({
        "ok": true,
        "ordenes": ordenes,
        "total": ordenes.len(),
        "solo_propias": solo_propias,
    })))
}

// ─── GET /api/v1/app/st/ordenes/:id ──────────────────────────────────────

pub async fn st_obtener_orden(
    AxumState(state): AxumState<Arc<ServerState>>,
    headers: HeaderMap,
    Path(id): Path<i64>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ApiError>)> {
    let session = extract_app_session(&headers, &state)?;
    requiere_modulo_st(&state)?;

    if !session.tiene("gestionar_servicio_tecnico") && !session.tiene("ver_servicio_tecnico") {
        return Err((StatusCode::FORBIDDEN, Json(ApiError::new("Sin permiso de servicio técnico"))));
    }

    let conn = state.db.conn.lock().map_err(err500)?;

    let detalle: OrdenDetalle = {
        let resumen = conn.query_row(
            "SELECT id, numero, estado, cliente_nombre, cliente_telefono,
                    equipo_descripcion, equipo_marca, equipo_modelo, equipo_serie, equipo_placa,
                    problema_reportado, COALESCE(presupuesto,0), COALESCE(monto_final,0),
                    fecha_ingreso, fecha_promesa, tecnico_id, tecnico_nombre,
                    diagnostico, trabajo_realizado, observaciones, accesorios,
                    COALESCE(garantia_dias, 0)
             FROM ordenes_servicio WHERE id = ?1",
            params![id],
            |r| Ok(OrdenDetalle {
                resumen: OrdenResumen {
                    id: r.get(0)?, numero: r.get(1)?, estado: r.get(2)?,
                    cliente_nombre: r.get(3)?, cliente_telefono: r.get(4)?,
                    equipo_descripcion: r.get(5)?,
                    equipo_marca: r.get(6)?, equipo_modelo: r.get(7)?,
                    equipo_serie: r.get(8)?, equipo_placa: r.get(9)?,
                    problema_reportado: r.get(10)?,
                    presupuesto: r.get(11)?, monto_final: r.get(12)?,
                    fecha_ingreso: r.get(13)?, fecha_promesa: r.get(14)?,
                    tecnico_id: r.get(15)?, tecnico_nombre: r.get(16)?,
                },
                diagnostico: r.get(17)?,
                trabajo_realizado: r.get(18)?,
                observaciones: r.get(19)?,
                accesorios: r.get(20)?,
                garantia_dias: r.get(21)?,
                imagenes: vec![],
            }),
        ).map_err(|_| err400("Orden no encontrada"))?;
        resumen
    };

    // Cargar imágenes
    let imagenes: Vec<ImagenOrden> = {
        let mut stmt = conn.prepare(
            "SELECT id, tipo, imagen_base64, descripcion
             FROM ordenes_servicio_imagenes
             WHERE orden_id = ?1
             ORDER BY id ASC",
        ).map_err(err500)?;
        let rows = stmt.query_map(params![id], |r| Ok(ImagenOrden {
            id: r.get(0)?, tipo: r.get(1)?,
            imagen_base64: r.get(2)?, descripcion: r.get(3)?,
        })).map_err(err500)?
          .collect::<Result<Vec<_>, _>>().map_err(err500)?;
        rows
    };

    let mut detalle_full = detalle;
    detalle_full.imagenes = imagenes;

    Ok(Json(serde_json::json!({ "ok": true, "orden": detalle_full })))
}

// ─── POST /api/v1/app/st/ordenes/:id/estado ──────────────────────────────

#[derive(Deserialize)]
pub struct CambiarEstadoReq {
    pub nuevo_estado: String,
    pub observacion: Option<String>,
}

pub async fn st_cambiar_estado(
    AxumState(state): AxumState<Arc<ServerState>>,
    headers: HeaderMap,
    Path(id): Path<i64>,
    Json(req): Json<CambiarEstadoReq>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ApiError>)> {
    let session = extract_app_session(&headers, &state)?;
    requiere_modulo_st(&state)?;
    session.requiere("gestionar_servicio_tecnico")?;

    let conn = state.db.conn.lock().map_err(err500)?;
    let estado_actual: String = conn.query_row(
        "SELECT estado FROM ordenes_servicio WHERE id = ?1",
        params![id], |r| r.get(0),
    ).map_err(|_| err400("Orden no encontrada"))?;

    conn.execute(
        "UPDATE ordenes_servicio SET estado = ?1 WHERE id = ?2",
        params![req.nuevo_estado, id],
    ).map_err(err500)?;

    let _ = conn.execute(
        "INSERT INTO ordenes_servicio_movimientos (orden_id, estado_anterior, estado_nuevo, observacion, usuario)
         VALUES (?1, ?2, ?3, ?4, ?5)",
        params![id, estado_actual, req.nuevo_estado, req.observacion, session.nombre],
    );

    Ok(Json(serde_json::json!({ "ok": true })))
}

// ─── POST /api/v1/app/st/ordenes/:id/diagnostico ─────────────────────────

#[derive(Deserialize)]
pub struct GuardarDiagnosticoReq {
    pub diagnostico: Option<String>,
    pub trabajo_realizado: Option<String>,
    pub observaciones: Option<String>,
}

pub async fn st_guardar_diagnostico(
    AxumState(state): AxumState<Arc<ServerState>>,
    headers: HeaderMap,
    Path(id): Path<i64>,
    Json(req): Json<GuardarDiagnosticoReq>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ApiError>)> {
    let session = extract_app_session(&headers, &state)?;
    requiere_modulo_st(&state)?;
    session.requiere("gestionar_servicio_tecnico")?;

    let conn = state.db.conn.lock().map_err(err500)?;
    conn.execute(
        "UPDATE ordenes_servicio
         SET diagnostico = COALESCE(?1, diagnostico),
             trabajo_realizado = COALESCE(?2, trabajo_realizado),
             observaciones = COALESCE(?3, observaciones)
         WHERE id = ?4",
        params![req.diagnostico, req.trabajo_realizado, req.observaciones, id],
    ).map_err(err500)?;

    Ok(Json(serde_json::json!({ "ok": true })))
}

// ─── POST /api/v1/app/st/ordenes (crear orden nueva desde el móvil) ──────

#[derive(Deserialize)]
pub struct CrearOrdenReq {
    pub cliente_nombre: String,
    pub cliente_telefono: Option<String>,
    pub cliente_identificacion: Option<String>,
    pub tipo_equipo: String,
    pub equipo_descripcion: String,
    pub equipo_marca: Option<String>,
    pub equipo_modelo: Option<String>,
    pub equipo_serie: Option<String>,
    pub equipo_placa: Option<String>,
    pub accesorios: Option<String>,
    pub problema_reportado: String,
    pub presupuesto: Option<f64>,
}

pub async fn st_crear_orden(
    AxumState(state): AxumState<Arc<ServerState>>,
    headers: HeaderMap,
    Json(req): Json<CrearOrdenReq>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ApiError>)> {
    let session = extract_app_session(&headers, &state)?;
    requiere_modulo_st(&state)?;
    session.requiere("gestionar_servicio_tecnico")?;

    if req.cliente_nombre.trim().is_empty() {
        return Err(err400("El nombre del cliente es obligatorio"));
    }
    if req.equipo_descripcion.trim().is_empty() {
        return Err(err400("La descripción del equipo es obligatoria"));
    }
    if req.problema_reportado.trim().is_empty() {
        return Err(err400("El problema reportado es obligatorio"));
    }

    let conn = state.db.conn.lock().map_err(err500)?;

    // v2.4.27: prefijo OT (Orden de Trabajo) — sequencial continuo con OS- antiguos.
    let next_seq: i64 = conn.query_row(
        "SELECT COALESCE(MAX(CAST(SUBSTR(numero, 4) AS INTEGER)), 0) + 1
         FROM ordenes_servicio WHERE numero LIKE 'OS-%' OR numero LIKE 'OT-%'",
        [], |r| r.get(0),
    ).unwrap_or(1);
    let numero = format!("OT-{:06}", next_seq);

    // Buscar/crear cliente si no existe (por identificación o teléfono)
    let cliente_id: Option<i64> = {
        let ident = req.cliente_identificacion.as_deref().unwrap_or("").trim();
        let tel = req.cliente_telefono.as_deref().unwrap_or("").trim();
        let nombre = req.cliente_nombre.trim();
        if !ident.is_empty() {
            conn.query_row(
                "SELECT id FROM clientes WHERE identificacion = ?1",
                params![ident], |r| r.get(0),
            ).ok()
        } else if !tel.is_empty() {
            conn.query_row(
                "SELECT id FROM clientes WHERE telefono = ?1",
                params![tel], |r| r.get(0),
            ).ok()
        } else {
            conn.query_row(
                "SELECT id FROM clientes WHERE LOWER(nombre) = LOWER(?1)",
                params![nombre], |r| r.get(0),
            ).ok()
        }
    };

    conn.execute(
        "INSERT INTO ordenes_servicio (
            numero, cliente_id, cliente_nombre, cliente_telefono,
            tipo_equipo, equipo_descripcion, equipo_marca, equipo_modelo,
            equipo_serie, equipo_placa, accesorios, problema_reportado,
            presupuesto, estado, tecnico_id, tecnico_nombre, fecha_ingreso
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, 'RECIBIDO', ?14, ?15, datetime('now', 'localtime'))",
        params![
            numero, cliente_id, req.cliente_nombre.trim(), req.cliente_telefono,
            req.tipo_equipo, req.equipo_descripcion.trim(), req.equipo_marca, req.equipo_modelo,
            req.equipo_serie, req.equipo_placa, req.accesorios, req.problema_reportado.trim(),
            req.presupuesto.unwrap_or(0.0), session.usuario_id, session.nombre,
        ],
    ).map_err(err500)?;
    let id = conn.last_insert_rowid();

    let _ = conn.execute(
        "INSERT INTO ordenes_servicio_movimientos (orden_id, estado_anterior, estado_nuevo, observacion, usuario)
         VALUES (?1, NULL, 'RECIBIDO', 'Creada desde app móvil', ?2)",
        params![id, session.nombre],
    );

    Ok(Json(serde_json::json!({ "ok": true, "id": id, "numero": numero })))
}

// ─── POST /api/v1/app/st/ordenes/:id/imagen ──────────────────────────────

#[derive(Deserialize)]
pub struct SubirImagenReq {
    /// Base64 PUR (sin data URI prefix)
    pub imagen_base64: String,
    /// "ANTES" | "DESPUES" | "GENERAL"
    pub tipo: String,
    pub descripcion: Option<String>,
}

pub async fn st_subir_imagen(
    AxumState(state): AxumState<Arc<ServerState>>,
    headers: HeaderMap,
    Path(id): Path<i64>,
    Json(req): Json<SubirImagenReq>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ApiError>)> {
    let session = extract_app_session(&headers, &state)?;
    requiere_modulo_st(&state)?;
    session.requiere("gestionar_servicio_tecnico")?;

    if req.imagen_base64.is_empty() {
        return Err(err400("Imagen vacía"));
    }
    let tipo_ok = matches!(req.tipo.as_str(), "ANTES" | "DESPUES" | "GENERAL");
    if !tipo_ok {
        return Err(err400("Tipo inválido (use ANTES, DESPUES o GENERAL)"));
    }

    let conn = state.db.conn.lock().map_err(err500)?;
    // Validar que la orden existe
    let _: i64 = conn.query_row(
        "SELECT id FROM ordenes_servicio WHERE id = ?1",
        params![id], |r| r.get(0),
    ).map_err(|_| err400("Orden no encontrada"))?;

    conn.execute(
        "INSERT INTO ordenes_servicio_imagenes (orden_id, tipo, imagen_base64, descripcion)
         VALUES (?1, ?2, ?3, ?4)",
        params![id, req.tipo, req.imagen_base64, req.descripcion],
    ).map_err(err500)?;

    let _ = session; // placeholder para futura auditoría por usuario
    Ok(Json(serde_json::json!({
        "ok": true,
        "id": conn.last_insert_rowid(),
    })))
}
