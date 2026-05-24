//! v2.5.43 — Módulo Contabilidad (Agente de Retención + ATS)
//!
//! Este módulo es OPCIONAL: solo accesible si la licencia incluye `contabilidad`.
//! Funciona como container para todo lo relacionado con ser AGENTE DE RETENCIÓN
//! (lo opuesto a `retenciones_recibidas` que ya existe — esas son las que clientes
//! me hacen a mí).
//!
//! Funcionalidades planeadas (en orden de release):
//!   - v2.5.43: Foundation — config del agente, schema, página base
//!   - v2.5.44: Captura manual de retenciones emitidas al registrar/editar compra
//!   - v2.5.45: Generación XML SRI + envío + autorización del comprobante
//!   - v2.5.46: RIDE PDF del comprobante de retención
//!   - v2.5.47: Generador ATS mensual + XML completo
//!
//! La activación efectiva se controla desde admin.clouget.com (campo
//! `licencia.modulos` debe incluir `"contabilidad"`).

use crate::db::{Database, SesionState};
use rusqlite::params;
use serde::{Deserialize, Serialize};
use tauri::State;

// ─── Configuración del agente de retención ───────────────────────────────────

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ContabilidadConfig {
    pub es_agente_retencion: bool,
    pub resolucion_designacion: Option<String>,
    pub fecha_designacion: Option<String>,
    pub tipo_contribuyente: Option<String>,
    pub obligado_contabilidad: bool,
    pub codigo_retencion_renta_default: Option<String>,
    pub codigo_retencion_iva_default: Option<String>,
    pub contador_ruc: Option<String>,
    pub contador_nombre: Option<String>,
    pub observacion: Option<String>,
}

#[tauri::command]
pub fn contabilidad_obtener_config(db: State<'_, Database>) -> Result<ContabilidadConfig, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let cfg = conn.query_row(
        "SELECT es_agente_retencion, resolucion_designacion, fecha_designacion,
                tipo_contribuyente, obligado_contabilidad,
                codigo_retencion_renta_default, codigo_retencion_iva_default,
                contador_ruc, contador_nombre, observacion
         FROM contabilidad_config WHERE id = 1",
        [],
        |r| Ok(ContabilidadConfig {
            es_agente_retencion: r.get::<_, i32>(0)? != 0,
            resolucion_designacion: r.get(1).ok(),
            fecha_designacion: r.get(2).ok(),
            tipo_contribuyente: r.get(3).ok(),
            obligado_contabilidad: r.get::<_, i32>(4)? != 0,
            codigo_retencion_renta_default: r.get(5).ok(),
            codigo_retencion_iva_default: r.get(6).ok(),
            contador_ruc: r.get(7).ok(),
            contador_nombre: r.get(8).ok(),
            observacion: r.get(9).ok(),
        }),
    ).unwrap_or(ContabilidadConfig {
        es_agente_retencion: false,
        resolucion_designacion: None,
        fecha_designacion: None,
        tipo_contribuyente: None,
        obligado_contabilidad: false,
        codigo_retencion_renta_default: None,
        codigo_retencion_iva_default: None,
        contador_ruc: None,
        contador_nombre: None,
        observacion: None,
    });
    Ok(cfg)
}

#[tauri::command]
pub fn contabilidad_guardar_config(
    db: State<'_, Database>,
    config: ContabilidadConfig,
) -> Result<(), String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    conn.execute(
        "UPDATE contabilidad_config SET
            es_agente_retencion = ?1,
            resolucion_designacion = ?2,
            fecha_designacion = ?3,
            tipo_contribuyente = ?4,
            obligado_contabilidad = ?5,
            codigo_retencion_renta_default = ?6,
            codigo_retencion_iva_default = ?7,
            contador_ruc = ?8,
            contador_nombre = ?9,
            observacion = ?10,
            updated_at = datetime('now','localtime')
         WHERE id = 1",
        params![
            config.es_agente_retencion as i32,
            config.resolucion_designacion,
            config.fecha_designacion,
            config.tipo_contribuyente,
            config.obligado_contabilidad as i32,
            config.codigo_retencion_renta_default,
            config.codigo_retencion_iva_default,
            config.contador_ruc,
            config.contador_nombre,
            config.observacion,
        ],
    ).map_err(|e| e.to_string())?;
    Ok(())
}

// ─── Retenciones EMITIDAS (placeholders, se implementan en v2.5.44+) ─────────

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RetencionEmitidaResumen {
    pub id: i64,
    pub numero: String,
    pub fecha_emision: String,
    pub proveedor_nombre: String,
    pub proveedor_ruc: Option<String>,
    pub numero_documento_referencia: Option<String>,
    pub total: f64,
    pub estado_sri: String,
    pub anulada: bool,
}

#[tauri::command]
pub fn contabilidad_listar_retenciones(
    db: State<'_, Database>,
    fecha_desde: Option<String>,
    fecha_hasta: Option<String>,
) -> Result<Vec<RetencionEmitidaResumen>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let desde = fecha_desde.unwrap_or_else(|| "1970-01-01".to_string());
    let hasta = fecha_hasta.unwrap_or_else(|| "2999-12-31".to_string());

    let mut stmt = conn.prepare(
        "SELECT re.id, re.numero, re.fecha_emision, p.nombre, p.ruc,
                re.numero_documento_referencia, re.total, re.estado_sri, re.anulada
         FROM retenciones_emitidas re
         JOIN proveedores p ON re.proveedor_id = p.id
         WHERE date(re.fecha_emision) >= date(?1) AND date(re.fecha_emision) <= date(?2)
         ORDER BY re.fecha_emision DESC"
    ).map_err(|e| e.to_string())?;

    let rows: Vec<RetencionEmitidaResumen> = stmt.query_map(params![desde, hasta], |r| {
        Ok(RetencionEmitidaResumen {
            id: r.get(0)?,
            numero: r.get(1)?,
            fecha_emision: r.get(2)?,
            proveedor_nombre: r.get(3)?,
            proveedor_ruc: r.get(4).ok(),
            numero_documento_referencia: r.get(5).ok(),
            total: r.get(6)?,
            estado_sri: r.get(7)?,
            anulada: r.get::<_, i32>(8)? != 0,
        })
    }).map_err(|e| e.to_string())?
    .filter_map(Result::ok)
    .collect();

    Ok(rows)
}

/// Stub para v2.5.44 — registrar retención emitida (manual o al registrar compra).
/// Por ahora retorna placeholder para que el frontend no rompa.
#[tauri::command]
pub fn contabilidad_registrar_retencion(
    _db: State<'_, Database>,
    _sesion: State<'_, SesionState>,
) -> Result<i64, String> {
    Err("Función disponible en v2.5.44 (próxima release)".to_string())
}
