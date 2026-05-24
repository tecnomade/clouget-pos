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

// ─── v2.5.45: Captura de retenciones emitidas ────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct ItemRetencionEmitida {
    pub tipo: String,           // "RENTA" o "IVA"
    pub codigo_sri: String,     // Tabla 304 o 21
    pub base_imponible: f64,
    pub porcentaje: f64,
    pub valor: f64,
}

#[derive(Debug, Deserialize)]
pub struct NuevaRetencionEmitida {
    pub compra_id: i64,
    pub numero_documento_referencia: Option<String>,
    pub fecha_documento_referencia: Option<String>,
    pub items: Vec<ItemRetencionEmitida>,
    #[serde(default)]
    pub observacion: Option<String>,
    /// Opcional — para generación SRI (estab + pto + secuencial)
    #[serde(default)]
    pub establecimiento: Option<String>,
    #[serde(default)]
    pub punto_emision: Option<String>,
    #[serde(default)]
    pub secuencial: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct RetencionEmitidaCreada {
    pub id: i64,
    pub numero: String,
    pub total: f64,
    pub subtotal_renta: f64,
    pub subtotal_iva: f64,
}

#[tauri::command]
pub fn contabilidad_crear_retencion(
    db: State<'_, Database>,
    sesion: State<'_, SesionState>,
    input: NuevaRetencionEmitida,
) -> Result<RetencionEmitidaCreada, String> {
    let usuario = {
        let s = sesion.sesion.lock().map_err(|e| e.to_string())?;
        s.as_ref().map(|s| s.nombre.clone()).unwrap_or_else(|| "?".to_string())
    };

    let mut conn = db.conn.lock().map_err(|e| e.to_string())?;

    // Validaciones básicas
    if input.items.is_empty() {
        return Err("Debes agregar al menos una línea de retención (RENTA o IVA)".into());
    }
    for it in &input.items {
        let t = it.tipo.to_uppercase();
        if t != "RENTA" && t != "IVA" {
            return Err(format!("Tipo inválido: '{}'. Solo RENTA o IVA.", it.tipo));
        }
        if it.codigo_sri.trim().is_empty() {
            return Err("Cada línea requiere código SRI".into());
        }
        if it.valor < 0.0 || it.base_imponible < 0.0 {
            return Err("Base y valor deben ser positivos".into());
        }
    }

    // Validar compra existe y no anulada
    let (compra_numero, compra_estado, proveedor_id, compra_fecha): (String, String, i64, String) = conn.query_row(
        "SELECT numero, estado, proveedor_id, COALESCE(fecha, '') FROM compras WHERE id = ?1",
        params![input.compra_id],
        |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
    ).map_err(|_| "Compra no encontrada".to_string())?;
    if compra_estado == "ANULADA" {
        return Err("No se puede emitir retención sobre una compra anulada".into());
    }

    let subtotal_renta: f64 = input.items.iter().filter(|i| i.tipo.to_uppercase() == "RENTA").map(|i| i.valor).sum();
    let subtotal_iva: f64 = input.items.iter().filter(|i| i.tipo.to_uppercase() == "IVA").map(|i| i.valor).sum();
    let total = subtotal_renta + subtotal_iva;

    // Generar numero interno RET-XXXXXX (auto-incrementable)
    let next_seq: i64 = conn.query_row(
        "SELECT COALESCE(MAX(CAST(SUBSTR(numero, 5) AS INTEGER)), 0) + 1
         FROM retenciones_emitidas WHERE numero LIKE 'RET-%'",
        [], |r| r.get(0),
    ).unwrap_or(1);
    let numero = format!("RET-{:06}", next_seq);

    let fecha_doc_ref = input.fecha_documento_referencia.unwrap_or_else(|| compra_fecha.clone());

    let tx = conn.transaction().map_err(|e| e.to_string())?;

    tx.execute(
        "INSERT INTO retenciones_emitidas
            (numero, compra_id, proveedor_id, tipo_documento_referencia,
             numero_documento_referencia, fecha_documento_referencia,
             establecimiento, punto_emision, secuencial,
             subtotal_renta, subtotal_iva, total,
             estado_sri, usuario, observacion)
         VALUES (?1, ?2, ?3, '01', ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, 'NO_APLICA', ?12, ?13)",
        params![
            numero, input.compra_id, proveedor_id,
            input.numero_documento_referencia,
            fecha_doc_ref,
            input.establecimiento,
            input.punto_emision,
            input.secuencial,
            subtotal_renta, subtotal_iva, total,
            usuario, input.observacion,
        ],
    ).map_err(|e| e.to_string())?;
    let ret_id = tx.last_insert_rowid();

    // Detalles
    for it in &input.items {
        tx.execute(
            "INSERT INTO retencion_emitida_detalles
                (retencion_id, tipo, codigo_sri, base_imponible, porcentaje, valor)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![ret_id, it.tipo.to_uppercase(), it.codigo_sri.trim(),
                    it.base_imponible, it.porcentaje, it.valor],
        ).map_err(|e| e.to_string())?;
    }

    // Ajustar saldo de cuenta_por_pagar (le pagas menos al proveedor por las retenciones)
    let cxp: Option<(i64, f64)> = tx.query_row(
        "SELECT id, saldo FROM cuentas_por_pagar WHERE compra_id = ?1 AND estado != 'ANULADA' LIMIT 1",
        params![input.compra_id], |r| Ok((r.get(0)?, r.get(1)?)),
    ).ok();
    if let Some((cxp_id, saldo_actual)) = cxp {
        let nuevo_saldo = (saldo_actual - total).max(0.0);
        let nuevo_estado = if nuevo_saldo <= 0.01 { "PAGADA" } else { "PENDIENTE" };
        let _ = tx.execute(
            "UPDATE cuentas_por_pagar SET saldo = ?1, estado = ?2 WHERE id = ?3",
            params![nuevo_saldo, nuevo_estado, cxp_id],
        );
    }

    tx.commit().map_err(|e| e.to_string())?;

    eprintln!("[Contabilidad] Retención {} emitida sobre compra {} por ${:.2}", numero, compra_numero, total);
    Ok(RetencionEmitidaCreada {
        id: ret_id, numero, total, subtotal_renta, subtotal_iva,
    })
}

#[derive(Debug, Serialize)]
pub struct RetencionEmitidaDetalle {
    pub id: i64,
    pub numero: String,
    pub fecha_emision: String,
    pub compra_id: i64,
    pub compra_numero: String,
    pub proveedor_id: i64,
    pub proveedor_nombre: String,
    pub proveedor_ruc: Option<String>,
    pub numero_documento_referencia: Option<String>,
    pub fecha_documento_referencia: Option<String>,
    pub subtotal_renta: f64,
    pub subtotal_iva: f64,
    pub total: f64,
    pub estado_sri: String,
    pub anulada: bool,
    pub observacion: Option<String>,
    pub items: Vec<RetencionEmitidaItem>,
}

#[derive(Debug, Serialize)]
pub struct RetencionEmitidaItem {
    pub tipo: String,
    pub codigo_sri: String,
    pub base_imponible: f64,
    pub porcentaje: f64,
    pub valor: f64,
}

#[tauri::command]
pub fn contabilidad_obtener_retencion(
    db: State<'_, Database>,
    id: i64,
) -> Result<RetencionEmitidaDetalle, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let cab = conn.query_row(
        "SELECT re.id, re.numero, re.fecha_emision, re.compra_id, c.numero,
                re.proveedor_id, p.nombre, p.ruc,
                re.numero_documento_referencia, re.fecha_documento_referencia,
                re.subtotal_renta, re.subtotal_iva, re.total,
                re.estado_sri, re.anulada, re.observacion
         FROM retenciones_emitidas re
         JOIN compras c ON re.compra_id = c.id
         JOIN proveedores p ON re.proveedor_id = p.id
         WHERE re.id = ?1",
        params![id],
        |r| Ok(RetencionEmitidaDetalle {
            id: r.get(0)?,
            numero: r.get(1)?,
            fecha_emision: r.get(2)?,
            compra_id: r.get(3)?,
            compra_numero: r.get(4)?,
            proveedor_id: r.get(5)?,
            proveedor_nombre: r.get(6)?,
            proveedor_ruc: r.get(7).ok(),
            numero_documento_referencia: r.get(8).ok(),
            fecha_documento_referencia: r.get(9).ok(),
            subtotal_renta: r.get(10)?,
            subtotal_iva: r.get(11)?,
            total: r.get(12)?,
            estado_sri: r.get(13)?,
            anulada: r.get::<_, i32>(14)? != 0,
            observacion: r.get(15).ok(),
            items: Vec::new(),
        }),
    ).map_err(|_| "Retención no encontrada".to_string())?;

    let mut stmt = conn.prepare(
        "SELECT tipo, codigo_sri, base_imponible, porcentaje, valor
         FROM retencion_emitida_detalles WHERE retencion_id = ?1
         ORDER BY tipo, id"
    ).map_err(|e| e.to_string())?;
    let items: Vec<RetencionEmitidaItem> = stmt.query_map(params![id], |r| Ok(RetencionEmitidaItem {
        tipo: r.get(0)?,
        codigo_sri: r.get(1)?,
        base_imponible: r.get(2)?,
        porcentaje: r.get(3)?,
        valor: r.get(4)?,
    })).map_err(|e| e.to_string())?
    .filter_map(Result::ok)
    .collect();

    Ok(RetencionEmitidaDetalle { items, ..cab })
}

#[tauri::command]
pub fn contabilidad_anular_retencion(
    db: State<'_, Database>,
    sesion: State<'_, SesionState>,
    id: i64,
    motivo: Option<String>,
) -> Result<(), String> {
    let usuario = {
        let s = sesion.sesion.lock().map_err(|e| e.to_string())?;
        s.as_ref().map(|s| s.nombre.clone()).unwrap_or_else(|| "?".to_string())
    };
    let conn = db.conn.lock().map_err(|e| e.to_string())?;

    // Obtener compra y total para revertir CXP
    let (compra_id, total, ya_anulada): (i64, f64, i32) = conn.query_row(
        "SELECT compra_id, total, anulada FROM retenciones_emitidas WHERE id = ?1",
        params![id], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
    ).map_err(|_| "Retención no encontrada".to_string())?;
    if ya_anulada != 0 {
        return Err("La retención ya está anulada".into());
    }

    // Marcar anulada
    let motivo_str = motivo.unwrap_or_else(|| "Sin motivo".to_string());
    conn.execute(
        "UPDATE retenciones_emitidas SET anulada = 1,
         observacion = COALESCE(observacion || ' · ', '') || 'ANULADA: ' || ?2
         WHERE id = ?1",
        params![id, motivo_str],
    ).map_err(|e| e.to_string())?;

    // Revertir saldo de CXP (sumar de vuelta el total de la retención)
    if total > 0.0 {
        let _ = conn.execute(
            "UPDATE cuentas_por_pagar
             SET saldo = saldo + ?1,
                 estado = CASE WHEN saldo + ?1 > 0.01 THEN 'PENDIENTE' ELSE estado END
             WHERE compra_id = ?2 AND estado != 'ANULADA'",
            params![total, compra_id],
        );
    }
    eprintln!("[Contabilidad] Retención #{} anulada por {} — motivo: {}", id, usuario, motivo_str);
    Ok(())
}

/// Stub para v2.5.46 — emitir retención al SRI. Por ahora retorna error claro.
#[tauri::command]
pub fn contabilidad_registrar_retencion(
    _db: State<'_, Database>,
    _sesion: State<'_, SesionState>,
) -> Result<i64, String> {
    Err("Función disponible en v2.5.44 (próxima release)".to_string())
}
