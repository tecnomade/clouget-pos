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
use crate::sri::{clave_acceso, firma, ride_retencion, soap, xml};
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

// ─── v2.5.46: Emisión SRI del comprobante de retención ───────────────────────

/// Resultado de la emisión SRI de un comprobante de retención.
#[derive(Debug, Serialize)]
pub struct ResultadoEmisionRetencion {
    pub exito: bool,
    pub estado_sri: String,
    pub clave_acceso: Option<String>,
    pub numero_autorizacion: Option<String>,
    pub fecha_autorizacion: Option<String>,
    pub numero_comprobante: Option<String>,
    pub mensaje: String,
}

/// Convierte fecha "YYYY-MM-DD ..." a "DD/MM/YYYY" (formato SRI).
fn fmt_fecha_sri(fecha_bd: &str) -> Result<String, String> {
    let parte = fecha_bd.split(' ').next().unwrap_or(fecha_bd).trim();
    let partes: Vec<&str> = parte.split('-').collect();
    if partes.len() != 3 {
        return Err(format!("Fecha inválida: {}", fecha_bd));
    }
    Ok(format!("{}/{}/{}", partes[2], partes[1], partes[0]))
}

/// Normaliza "001-002-000000123" → "001002000000123" (15 dígitos sin guiones).
/// Si no tiene formato esperado, intenta zero-pad a 15 dígitos.
fn fmt_num_doc_sustento(numero: &str) -> String {
    let limpio: String = numero.chars().filter(|c| c.is_ascii_digit()).collect();
    if limpio.len() >= 15 {
        limpio.chars().rev().take(15).collect::<String>().chars().rev().collect()
    } else {
        format!("{:0>15}", limpio)
    }
}

/// Emite al SRI el comprobante de retención: genera XML, firma con XAdES-BES,
/// envía via SOAP, consulta autorización y persiste resultado en BD.
///
/// Reutiliza la infra existente de facturas (clave_acceso, firma, soap).
///
/// Requisitos:
/// - Licencia con módulo `contabilidad` activa
/// - Certificado P12 cargado (mismo que facturas)
/// - Config: ruc, sri_ambiente, terminal_establecimiento/punto_emision o establecimiento/punto_emision
/// - contabilidad_config: es_agente_retencion = true
#[tauri::command]
pub async fn contabilidad_emitir_retencion_sri(
    db: State<'_, Database>,
    id: i64,
) -> Result<ResultadoEmisionRetencion, String> {
    // ── 1. Leer todo lo necesario en un solo lock ────────────────────────────
    #[allow(dead_code)]
    struct DatosRet {
        numero_interno: String,
        compra_id: i64,
        proveedor_nombre: String,
        proveedor_ruc: Option<String>,
        proveedor_tipo_identificacion: Option<String>,
        proveedor_obligado_contabilidad: i32,
        proveedor_tipo: Option<String>, // "01"=PN, "02"=Sociedad
        compra_numero: String,
        compra_fecha: String,
        num_doc_referencia: Option<String>,
        fecha_doc_referencia: Option<String>,
        anulada: i32,
        estado_sri: String,
        clave_acceso_previa: Option<String>,
        xml_firmado_previo: Option<String>,
        establecimiento_prev: Option<String>,
        punto_emision_prev: Option<String>,
        secuencial_prev: Option<String>,
        numero_comprobante_prev: Option<String>,
    }
    struct DetRet {
        tipo: String,        // "RENTA" o "IVA"
        codigo_sri: String,
        base_imponible: f64,
        porcentaje: f64,
        valor: f64,
    }

    let (datos, detalles, config, p12_data, p12_password, es_agente, obligado_contabilidad_cfg, contribuyente_especial_cfg) = {
        let conn = db.conn.lock().map_err(|e| e.to_string())?;

        // Cabecera + JOIN proveedor + compra
        let datos: DatosRet = conn.query_row(
            "SELECT re.numero, re.compra_id, p.nombre, p.ruc, p.tipo_identificacion,
                    COALESCE(p.obligado_contabilidad, 0),
                    p.tipo,
                    c.numero, COALESCE(c.fecha, re.fecha_emision),
                    re.numero_documento_referencia, re.fecha_documento_referencia,
                    re.anulada, re.estado_sri,
                    re.clave_acceso, re.xml_firmado,
                    re.establecimiento, re.punto_emision, re.secuencial, re.numero_factura
             FROM retenciones_emitidas re
             JOIN proveedores p ON re.proveedor_id = p.id
             JOIN compras c ON re.compra_id = c.id
             WHERE re.id = ?1",
            params![id],
            |r| Ok(DatosRet {
                numero_interno: r.get(0)?,
                compra_id: r.get(1)?,
                proveedor_nombre: r.get(2)?,
                proveedor_ruc: r.get(3).ok(),
                proveedor_tipo_identificacion: r.get(4).ok(),
                proveedor_obligado_contabilidad: r.get::<_, i32>(5).unwrap_or(0),
                proveedor_tipo: r.get(6).ok(),
                compra_numero: r.get(7)?,
                compra_fecha: r.get(8)?,
                num_doc_referencia: r.get(9).ok(),
                fecha_doc_referencia: r.get(10).ok(),
                anulada: r.get(11)?,
                estado_sri: r.get(12)?,
                clave_acceso_previa: r.get(13).ok(),
                xml_firmado_previo: r.get(14).ok(),
                establecimiento_prev: r.get(15).ok(),
                punto_emision_prev: r.get(16).ok(),
                secuencial_prev: r.get(17).ok(),
                numero_comprobante_prev: r.get(18).ok(),
            }),
        ).map_err(|_| "Retención no encontrada".to_string())?;

        if datos.anulada != 0 {
            return Err("La retención está anulada".into());
        }
        if datos.estado_sri == "AUTORIZADA" {
            return Err("Esta retención ya fue autorizada por el SRI".into());
        }

        // Detalles
        let mut stmt = conn.prepare(
            "SELECT tipo, codigo_sri, base_imponible, porcentaje, valor
             FROM retencion_emitida_detalles WHERE retencion_id = ?1 ORDER BY tipo, id"
        ).map_err(|e| e.to_string())?;
        let detalles: Vec<DetRet> = stmt.query_map(params![id], |r| Ok(DetRet {
            tipo: r.get(0)?,
            codigo_sri: r.get(1)?,
            base_imponible: r.get(2)?,
            porcentaje: r.get(3)?,
            valor: r.get(4)?,
        })).map_err(|e| e.to_string())?
        .filter_map(Result::ok)
        .collect();
        drop(stmt);

        if detalles.is_empty() {
            return Err("La retención no tiene líneas".into());
        }

        // Config global (RUC, ambiente, etc.)
        let mut config: std::collections::HashMap<String, String> = std::collections::HashMap::new();
        let mut stmt_cfg = conn.prepare("SELECT key, value FROM config").map_err(|e| e.to_string())?;
        let rows = stmt_cfg.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?))).map_err(|e| e.to_string())?;
        for row in rows {
            let (k, v) = row.map_err(|e| e.to_string())?;
            config.insert(k, v);
        }
        drop(stmt_cfg);

        // contabilidad_config: es_agente_retencion + obligado_contabilidad + contribuyente_especial (vía resolución)
        let (es_agente, obligado, contrib_esp): (i32, i32, Option<String>) = conn.query_row(
            "SELECT es_agente_retencion, obligado_contabilidad,
                    NULLIF(TRIM(COALESCE(resolucion_designacion, '')), '')
             FROM contabilidad_config WHERE id = 1",
            [], |r| Ok((r.get(0)?, r.get(1)?, r.get(2).ok())),
        ).unwrap_or((0, 0, None));

        // Certificado P12
        let (p12_blob, p12_pass): (Vec<u8>, String) = conn.query_row(
            "SELECT p12_data, password FROM sri_certificado WHERE id = 1",
            [], |r| Ok((r.get(0)?, r.get(1)?)),
        ).map_err(|_| "No hay certificado digital cargado. Cárguelo en Configuración → SRI.".to_string())?;

        (datos, detalles, config, p12_blob, p12_pass, es_agente != 0, obligado != 0, contrib_esp)
    };

    if !es_agente {
        return Err("Active 'Es agente de retención' en Contabilidad → Configuración antes de emitir.".into());
    }

    // ── 2. Resolver config ───────────────────────────────────────────────────
    let cfg = |k: &str| config.get(k).cloned().unwrap_or_default();
    let ruc = cfg("ruc");
    if ruc.len() != 13 {
        return Err("Configure el RUC del negocio (13 dígitos) antes de emitir.".into());
    }
    let ambiente = match cfg("sri_ambiente").as_str() {
        "produccion" => "2",
        _ => "1",
    };
    let establecimiento_cfg = {
        let term = cfg("terminal_establecimiento");
        if term.is_empty() { cfg("establecimiento") } else { term }
    };
    let punto_emision_cfg = {
        let term = cfg("terminal_punto_emision");
        if term.is_empty() { cfg("punto_emision") } else { term }
    };
    let regimen = cfg("regimen");

    let tipo_doc_sec = if ambiente == "1" { "RETENCION_PRUEBAS" } else { "RETENCION" };

    // Verificar suscripción SRI (mismo enforcement que facturas, mismo cupo).
    // En modo demo, se salta.
    let modo_demo = cfg("demo_activo") == "1";

    // ── 3. Reenvío o primera emisión ─────────────────────────────────────────
    let mut secuencial_sri: i64 = 0;
    let mut numero_comprobante = datos.numero_comprobante_prev.clone().unwrap_or_default();
    let mut establecimiento_usado = datos.establecimiento_prev.clone().unwrap_or_else(|| establecimiento_cfg.clone());
    let mut punto_emision_usado = datos.punto_emision_prev.clone().unwrap_or_else(|| punto_emision_cfg.clone());
    let mut es_primera_emision = false;

    let (clave_final, xml_firmado_final, resultado_sri) = if datos.estado_sri == "PENDIENTE"
        && datos.clave_acceso_previa.is_some()
        && datos.xml_firmado_previo.is_some()
        && !modo_demo
    {
        let clave_prev = datos.clave_acceso_previa.clone().unwrap();
        let xml_prev = datos.xml_firmado_previo.clone().unwrap();
        soap::log_sri(&format!("=== REENVIO RETENCION: clave previa {} ===", clave_prev));

        // Primero consultar autorización
        let consulta = soap::consultar_autorizacion(&clave_prev, ambiente).await;
        match consulta {
            Ok(ref res) if res.exito => (clave_prev, xml_prev, consulta.unwrap()),
            _ => {
                soap::log_sri("Clave previa no autorizada, reenviando XML firmado de retención...");
                let r = soap::enviar_comprobante(&xml_prev, &clave_prev, ambiente).await?;
                (clave_prev, xml_prev, r)
            }
        }
    } else {
        es_primera_emision = true;

        // Secuencial nuevo
        secuencial_sri = {
            let conn = db.conn.lock().map_err(|e| e.to_string())?;
            // Asegurar registro y leer
            conn.execute(
                "INSERT OR IGNORE INTO secuenciales (establecimiento_codigo, punto_emision_codigo, tipo_documento, secuencial) VALUES (?1, ?2, ?3, 1)",
                params![establecimiento_cfg, punto_emision_cfg, tipo_doc_sec],
            ).map_err(|e| format!("Error creando secuencial retención: {}", e))?;
            conn.query_row(
                "SELECT secuencial FROM secuenciales WHERE establecimiento_codigo = ?1 AND punto_emision_codigo = ?2 AND tipo_documento = ?3",
                params![establecimiento_cfg, punto_emision_cfg, tipo_doc_sec],
                |r| r.get::<_, i64>(0),
            ).map_err(|e| format!("Error leyendo secuencial: {}", e))?
        };
        establecimiento_usado = establecimiento_cfg.clone();
        punto_emision_usado = punto_emision_cfg.clone();
        let secuencial = format!("{:09}", secuencial_sri);
        numero_comprobante = format!("{}-{}-{}", establecimiento_usado, punto_emision_usado, secuencial);

        let fecha_emision = fmt_fecha_sri(&datos.compra_fecha).unwrap_or_else(|_| {
            // fallback: hoy
            chrono::Local::now().format("%d/%m/%Y").to_string()
        });

        let clave = clave_acceso::generar_clave_acceso(
            &fecha_emision,
            "07", // comprobante de retención
            &ruc,
            ambiente,
            &establecimiento_usado,
            &punto_emision_usado,
            &secuencial,
            "1",
        );

        // Identificación del sujeto retenido (proveedor)
        let id_sujeto = datos.proveedor_ruc.clone().unwrap_or_default();
        if id_sujeto.is_empty() {
            return Err("El proveedor no tiene RUC/identificación configurada".into());
        }
        let tipo_id_sujeto = match datos.proveedor_tipo_identificacion.as_deref().unwrap_or("") {
            "RUC" => "04",
            "CEDULA" => "05",
            "PASAPORTE" => "06",
            _ => {
                // Inferir por longitud
                if id_sujeto.len() == 13 { "04" }
                else if id_sujeto.len() == 10 { "05" }
                else { "06" }
            }
        };

        // tipo_sujeto_retenido: "01"=PN, "02"=Sociedad. Si no está claro, default según largo del RUC.
        let tipo_sujeto = datos.proveedor_tipo.clone().or_else(|| {
            if id_sujeto.len() == 13 && id_sujeto.ends_with("001") {
                let tercero = id_sujeto.chars().nth(2).and_then(|c| c.to_digit(10)).unwrap_or(0);
                if tercero == 9 { Some("02".to_string()) } // RUC sociedad
                else if tercero == 6 { Some("01".to_string()) } // RUC público (tratar como PN)
                else { Some("01".to_string()) } // RUC persona natural
            } else { Some("01".to_string()) }
        });

        // Período fiscal = MM/YYYY de la fecha de emisión (DD/MM/YYYY)
        let periodo_fiscal = {
            let partes: Vec<&str> = fecha_emision.split('/').collect();
            if partes.len() == 3 {
                format!("{}/{}", partes[1], partes[2])
            } else {
                chrono::Local::now().format("%m/%Y").to_string()
            }
        };

        // Documento sustento (la compra/factura del proveedor)
        let num_doc_sustento = datos.num_doc_referencia.as_deref().unwrap_or(&datos.compra_numero);
        let num_doc_sustento_fmt = fmt_num_doc_sustento(num_doc_sustento);
        let fecha_doc_sustento = datos.fecha_doc_referencia.as_deref().unwrap_or(&datos.compra_fecha);
        let fecha_doc_sustento_fmt = fmt_fecha_sri(fecha_doc_sustento).unwrap_or(fecha_emision.clone());

        // Mapear detalles → ImpuestoRetenido
        let impuestos: Vec<xml::ImpuestoRetenido> = detalles.iter().map(|d| {
            let codigo = if d.tipo.eq_ignore_ascii_case("RENTA") { "1" } else { "2" }; // 1=Renta, 2=IVA
            xml::ImpuestoRetenido {
                codigo: codigo.to_string(),
                codigo_retencion: d.codigo_sri.trim().to_string(),
                base_imponible: d.base_imponible,
                porcentaje_retener: d.porcentaje,
                valor_retenido: d.valor,
                cod_doc_sustento: "01".to_string(), // factura (compra del proveedor)
                num_doc_sustento: num_doc_sustento_fmt.clone(),
                fecha_emision_doc_sustento: fecha_doc_sustento_fmt.clone(),
                numero_autorizacion_doc_sustento: None,
            }
        }).collect();

        let contribuyente_rimpe = match regimen.as_str() {
            "RIMPE_EMPRENDEDOR" => Some("CONTRIBUYENTE RÉGIMEN RIMPE".to_string()),
            "RIMPE_POPULAR" => Some("CONTRIBUYENTE NEGOCIO POPULAR - RÉGIMEN RIMPE".to_string()),
            _ => None,
        };

        let datos_xml = xml::DatosRetencion {
            ambiente: ambiente.to_string(),
            tipo_emision: "1".to_string(),
            razon_social: cfg("nombre_negocio"),
            nombre_comercial: cfg("nombre_negocio"),
            ruc: ruc.clone(),
            clave_acceso: clave.clone(),
            estab: establecimiento_usado.clone(),
            pto_emi: punto_emision_usado.clone(),
            secuencial: secuencial.clone(),
            dir_matriz: cfg("direccion"),
            contribuyente_rimpe,
            fecha_emision,
            dir_establecimiento: cfg("direccion"),
            contribuyente_especial: contribuyente_especial_cfg.clone(),
            obligado_contabilidad: if obligado_contabilidad_cfg { "SI".to_string() } else { "NO".to_string() },
            tipo_identificacion_sujeto_retenido: tipo_id_sujeto.to_string(),
            razon_social_sujeto_retenido: datos.proveedor_nombre.clone(),
            tipo_sujeto_retenido: tipo_sujeto,
            identificacion_sujeto_retenido: id_sujeto,
            periodo_fiscal,
            impuestos,
        };

        let _ = datos.proveedor_obligado_contabilidad; // suprimido warn

        let xml_sin_firma = xml::generar_xml_retencion(&datos_xml);
        soap::log_sri(&format!("XML retención sin firma ({} bytes):\n{}", xml_sin_firma.len(), xml_sin_firma));

        if modo_demo {
            // Demo mode: simular autorización sin enviar al SRI
            let fake_auth = clave.clone();
            let now = chrono::Local::now().format("%Y-%m-%dT%H:%M:%S").to_string();
            return persistir_y_responder(
                &db, id, &clave, &xml_sin_firma, true, "AUTORIZADA",
                Some(fake_auth), Some(now), &numero_comprobante,
                &establecimiento_usado, &punto_emision_usado, secuencial_sri,
                tipo_doc_sec, es_primera_emision,
                "Demo: retención autorizada simulada (no enviada al SRI)",
            );
        }

        let firmado = firma::firmar_comprobante(
            &xml_sin_firma,
            &p12_data,
            &p12_password,
            "comprobanteRetencion",
        )?;
        let r = soap::enviar_comprobante(&firmado.xml, &clave, ambiente).await?;
        (clave, firmado.xml, r)
    };

    // ── 4. Persistir resultado y responder ───────────────────────────────────
    persistir_y_responder(
        &db, id,
        &clave_final, &xml_firmado_final,
        resultado_sri.exito,
        &resultado_sri.estado,
        resultado_sri.numero_autorizacion.clone(),
        resultado_sri.fecha_autorizacion.clone(),
        &numero_comprobante,
        &establecimiento_usado, &punto_emision_usado, secuencial_sri,
        tipo_doc_sec, es_primera_emision,
        resultado_sri.mensaje.as_deref().unwrap_or(""),
    )
}

/// Persiste el resultado de la emisión SRI y devuelve respuesta para el frontend.
#[allow(clippy::too_many_arguments)]
fn persistir_y_responder(
    db: &State<'_, Database>,
    retencion_id: i64,
    clave: &str,
    xml_firmado: &str,
    exito: bool,
    estado_raw: &str,
    numero_autorizacion: Option<String>,
    fecha_autorizacion: Option<String>,
    numero_comprobante: &str,
    establecimiento: &str,
    punto_emision: &str,
    secuencial_int: i64,
    tipo_doc_sec: &str,
    es_primera_emision: bool,
    mensaje_extra: &str,
) -> Result<ResultadoEmisionRetencion, String> {
    let nuevo_estado = if exito {
        "AUTORIZADA"
    } else if estado_raw == "EN_PROCESO" {
        "PENDIENTE"
    } else {
        "RECHAZADA"
    };

    let xml_para_guardar = if exito || nuevo_estado == "PENDIENTE" {
        Some(xml_firmado.to_string())
    } else {
        None
    };

    {
        let conn = db.conn.lock().map_err(|e| e.to_string())?;
        let secuencial_str = if secuencial_int > 0 {
            Some(format!("{:09}", secuencial_int))
        } else { None };

        conn.execute(
            "UPDATE retenciones_emitidas SET
                estado_sri = ?1,
                clave_acceso = ?2,
                autorizacion_sri = ?3,
                fecha_autorizacion = ?4,
                xml_firmado = COALESCE(?5, xml_firmado),
                numero_factura = COALESCE(?6, numero_factura),
                establecimiento = COALESCE(?7, establecimiento),
                punto_emision = COALESCE(?8, punto_emision),
                secuencial = COALESCE(?9, secuencial)
             WHERE id = ?10",
            params![
                nuevo_estado,
                clave,
                numero_autorizacion,
                fecha_autorizacion,
                xml_para_guardar,
                if numero_comprobante.is_empty() { None } else { Some(numero_comprobante.to_string()) },
                if establecimiento.is_empty() { None } else { Some(establecimiento.to_string()) },
                if punto_emision.is_empty() { None } else { Some(punto_emision.to_string()) },
                secuencial_str,
                retencion_id,
            ],
        ).map_err(|e| format!("Error actualizando retención: {}", e))?;

        if exito && es_primera_emision && secuencial_int > 0 {
            let _ = conn.execute(
                "UPDATE secuenciales SET secuencial = secuencial + 1
                 WHERE establecimiento_codigo = ?1 AND punto_emision_codigo = ?2 AND tipo_documento = ?3",
                params![establecimiento, punto_emision, tipo_doc_sec],
            );
        }
    }

    let mensaje = if !mensaje_extra.is_empty() {
        mensaje_extra.to_string()
    } else if exito {
        format!("Retención autorizada — Nº {}", numero_comprobante)
    } else {
        format!("Estado: {}", nuevo_estado)
    };

    Ok(ResultadoEmisionRetencion {
        exito,
        estado_sri: nuevo_estado.to_string(),
        clave_acceso: Some(clave.to_string()),
        numero_autorizacion,
        fecha_autorizacion,
        numero_comprobante: if numero_comprobante.is_empty() { None } else { Some(numero_comprobante.to_string()) },
        mensaje,
    })
}

// ─── v2.5.47: RIDE PDF del comprobante de retención ──────────────────────────

/// Genera el RIDE (PDF) del comprobante de retención y lo devuelve como bytes.
/// El frontend recibe el Vec<u8> y lo guarda con un dialog Save / lo abre.
///
/// Funciona aunque la retención no esté autorizada (se marca como "PRUEBAS" /
/// "PENDIENTE" en el encabezado), pero idealmente se imprime después de tener
/// `autorizacion_sri` válido del SRI.
#[tauri::command]
pub fn contabilidad_generar_ride_pdf(
    db: State<'_, Database>,
    id: i64,
) -> Result<Vec<u8>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;

    // Cabecera + datos del proveedor + compra
    struct Cab {
        numero_comprobante: Option<String>,
        clave_acceso: Option<String>,
        autorizacion_sri: Option<String>,
        fecha_emision: String,
        fecha_autorizacion: Option<String>,
        fecha_doc_referencia: Option<String>,
        num_doc_referencia: Option<String>,
        compra_numero: String,
        compra_fecha: String,
        estab: Option<String>,
        pto: Option<String>,
        sec: Option<String>,
        prov_nombre: String,
        prov_ruc: Option<String>,
        prov_tipo_id: Option<String>,
        prov_direccion: Option<String>,
        prov_email: Option<String>,
        total: f64,
        anulada: i32,
    }

    let cab: Cab = conn.query_row(
        "SELECT re.numero_factura, re.clave_acceso, re.autorizacion_sri,
                re.fecha_emision, re.fecha_autorizacion,
                re.fecha_documento_referencia, re.numero_documento_referencia,
                c.numero, COALESCE(c.fecha, re.fecha_emision),
                re.establecimiento, re.punto_emision, re.secuencial,
                p.nombre, p.ruc, p.tipo_identificacion, p.direccion, p.email,
                re.total, re.anulada
         FROM retenciones_emitidas re
         JOIN compras c ON re.compra_id = c.id
         JOIN proveedores p ON re.proveedor_id = p.id
         WHERE re.id = ?1",
        params![id],
        |r| Ok(Cab {
            numero_comprobante: r.get(0).ok(),
            clave_acceso: r.get(1).ok(),
            autorizacion_sri: r.get(2).ok(),
            fecha_emision: r.get(3)?,
            fecha_autorizacion: r.get(4).ok(),
            fecha_doc_referencia: r.get(5).ok(),
            num_doc_referencia: r.get(6).ok(),
            compra_numero: r.get(7)?,
            compra_fecha: r.get(8)?,
            estab: r.get(9).ok(),
            pto: r.get(10).ok(),
            sec: r.get(11).ok(),
            prov_nombre: r.get(12)?,
            prov_ruc: r.get(13).ok(),
            prov_tipo_id: r.get(14).ok(),
            prov_direccion: r.get(15).ok(),
            prov_email: r.get(16).ok(),
            total: r.get(17)?,
            anulada: r.get(18)?,
        }),
    ).map_err(|_| "Retención no encontrada".to_string())?;

    if cab.anulada != 0 {
        return Err("La retención está anulada — no se puede imprimir RIDE".into());
    }

    // Detalles
    let mut stmt = conn.prepare(
        "SELECT tipo, codigo_sri, base_imponible, porcentaje, valor
         FROM retencion_emitida_detalles WHERE retencion_id = ?1 ORDER BY tipo, id"
    ).map_err(|e| e.to_string())?;
    let items_raw: Vec<(String, String, f64, f64, f64)> = stmt.query_map(params![id], |r| Ok((
        r.get::<_, String>(0)?,
        r.get::<_, String>(1)?,
        r.get::<_, f64>(2)?,
        r.get::<_, f64>(3)?,
        r.get::<_, f64>(4)?,
    ))).map_err(|e| e.to_string())?
    .filter_map(Result::ok)
    .collect();
    drop(stmt);

    // Config global
    let mut config: std::collections::HashMap<String, String> = std::collections::HashMap::new();
    let mut stmt_cfg = conn.prepare("SELECT key, value FROM config").map_err(|e| e.to_string())?;
    let rows = stmt_cfg.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?))).map_err(|e| e.to_string())?;
    for row in rows {
        let (k, v) = row.map_err(|e| e.to_string())?;
        config.insert(k, v);
    }
    drop(stmt_cfg);

    // Config contabilidad (obligado + resolución)
    let (obligado, resolucion): (i32, Option<String>) = conn.query_row(
        "SELECT obligado_contabilidad,
                NULLIF(TRIM(COALESCE(resolucion_designacion, '')), '')
         FROM contabilidad_config WHERE id = 1",
        [], |r| Ok((r.get(0)?, r.get(1).ok())),
    ).unwrap_or((0, None));

    // ── Armar datos ─────────────────────────────────────────────────────────
    let ambiente = config.get("sri_ambiente").map(|s| s.as_str()).unwrap_or("pruebas");
    let ambiente_cod = if ambiente == "produccion" { "2" } else { "1" };

    let numero = cab.numero_comprobante.clone().unwrap_or_else(|| {
        // Fallback: armar desde estab-pto-sec o desde fecha si nada
        let estab = cab.estab.as_deref().unwrap_or("001");
        let pto = cab.pto.as_deref().unwrap_or("001");
        let sec = cab.sec.as_deref().unwrap_or("000000000");
        format!("{}-{}-{}", estab, pto, sec)
    });

    let fecha_emision_fmt = formatear_fecha_dmy(&cab.fecha_emision);
    let fecha_autorizacion_fmt = cab.fecha_autorizacion
        .as_deref()
        .map(formatear_fecha_dmy)
        .unwrap_or_else(|| fecha_emision_fmt.clone());

    // Período fiscal: MM/YYYY
    let periodo_fiscal = periodo_fiscal_de_fecha(&cab.fecha_emision);

    let tipo_id_sujeto = cab.prov_tipo_id.as_deref().map(|t| match t {
        "RUC" => "04".to_string(),
        "CEDULA" => "05".to_string(),
        "PASAPORTE" => "06".to_string(),
        _ => "07".to_string(),
    }).unwrap_or_else(|| {
        // Inferir por longitud del RUC
        let r = cab.prov_ruc.as_deref().unwrap_or("");
        if r.len() == 13 { "04".to_string() }
        else if r.len() == 10 { "05".to_string() }
        else { "06".to_string() }
    });

    let datos = ride_retencion::DatosRetencionRide {
        numero,
        clave_acceso: cab.clave_acceso.clone().unwrap_or_default(),
        numero_autorizacion: cab.autorizacion_sri.clone().or_else(|| cab.clave_acceso.clone()).unwrap_or_default(),
        fecha_emision: fecha_emision_fmt.clone(),
        fecha_autorizacion: fecha_autorizacion_fmt,
        ambiente: ambiente_cod.to_string(),
        periodo_fiscal,
        sujeto_nombre: cab.prov_nombre,
        sujeto_identificacion: cab.prov_ruc.unwrap_or_default(),
        sujeto_tipo_id: tipo_id_sujeto,
        sujeto_direccion: cab.prov_direccion,
        sujeto_email: cab.prov_email,
        total_retenido: cab.total,
    };

    // Documento sustento (la compra)
    let num_doc_sust_raw = cab.num_doc_referencia.unwrap_or(cab.compra_numero);
    let num_doc_sust_fmt = {
        let limpio: String = num_doc_sust_raw.chars().filter(|c| c.is_ascii_digit()).collect();
        if limpio.len() >= 15 {
            limpio.chars().rev().take(15).collect::<String>().chars().rev().collect()
        } else {
            format!("{:0>15}", limpio)
        }
    };
    let fecha_doc_sust = cab.fecha_doc_referencia.as_deref().unwrap_or(&cab.compra_fecha);
    let fecha_doc_sust_fmt = formatear_fecha_dmy(fecha_doc_sust);

    let items: Vec<ride_retencion::ItemRetencionRide> = items_raw.into_iter().map(|(tipo, codigo, base, pct, valor)| {
        ride_retencion::ItemRetencionRide {
            tipo_label: tipo,
            codigo_retencion: codigo,
            base_imponible: base,
            porcentaje: pct,
            valor_retenido: valor,
            cod_doc_sustento: "01".to_string(),
            num_doc_sustento: num_doc_sust_fmt.clone(),
            fecha_doc_sustento: fecha_doc_sust_fmt.clone(),
        }
    }).collect();

    ride_retencion::generar_ride_retencion_pdf(&datos, &items, &config, obligado != 0, resolucion.as_deref())
}

/// Convierte "YYYY-MM-DD ..." a "dd/mm/yyyy". Si ya viene en otro formato,
/// devuelve el string tal cual (el ride lo muestra como recibió).
fn formatear_fecha_dmy(fecha_bd: &str) -> String {
    let parte = fecha_bd.split(' ').next().unwrap_or(fecha_bd).trim();
    let partes: Vec<&str> = parte.split('-').collect();
    if partes.len() == 3 && partes[0].len() == 4 {
        format!("{}/{}/{}", partes[2], partes[1], partes[0])
    } else {
        fecha_bd.to_string()
    }
}

/// "2026-05-24 ..." → "05/2026"
fn periodo_fiscal_de_fecha(fecha_bd: &str) -> String {
    let parte = fecha_bd.split(' ').next().unwrap_or(fecha_bd).trim();
    let partes: Vec<&str> = parte.split('-').collect();
    if partes.len() == 3 && partes[0].len() == 4 {
        format!("{}/{}", partes[1], partes[0])
    } else {
        chrono::Local::now().format("%m/%Y").to_string()
    }
}
