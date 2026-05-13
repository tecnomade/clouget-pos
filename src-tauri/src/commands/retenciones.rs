//! v2.5.4 — Retenciones recibidas (SRI Ecuador).
//!
//! Cuando vendemos a una empresa, esa empresa puede actuar como agente de
//! retención y descontar parte del pago según normativa SRI:
//! - Retención de IVA (Tabla 21): 10%, 20%, 30%, 70%, 100% del IVA
//! - Retención de Renta (Tabla 304): 1%, 1.75%, 2%, 8%, 10% del subtotal
//!
//! Estas retenciones REDUCEN el saldo pendiente de la factura. Al registrarlas
//! aquí, el cobro de la factura se considera completado (sin descuadre contable).
//!
//! Comandos:
//! - `listar_retenciones_venta(venta_id)` — lista retenciones de una factura
//! - `total_retenciones_venta(venta_id)` — suma total de retenciones aplicadas
//! - `registrar_retencion(...)` — registra una nueva retención
//! - `eliminar_retencion(id)` — elimina una retención (corregir typos)

use crate::db::{Database, SesionState};
use rusqlite::params;
use serde::{Deserialize, Serialize};
use tauri::State;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RetencionRecibida {
    pub id: Option<i64>,
    pub venta_id: i64,
    pub tipo: String,                    // RENTA | IVA
    pub codigo_sri: String,
    pub base_imponible: f64,
    pub porcentaje: f64,
    pub valor: f64,
    pub numero_comprobante: String,
    pub fecha_emision: String,           // YYYY-MM-DD
    pub fecha_registro: Option<String>,
    pub usuario: Option<String>,
    pub observacion: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct TotalesRetencion {
    pub total_renta: f64,
    pub total_iva: f64,
    pub total: f64,
    pub cantidad: i64,
}

// ─── LISTAR ───────────────────────────────────────────────────────────────

#[tauri::command]
pub fn listar_retenciones_venta(
    db: State<'_, Database>,
    venta_id: i64,
) -> Result<Vec<RetencionRecibida>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let mut stmt = conn.prepare(
        "SELECT id, venta_id, tipo, codigo_sri, base_imponible, porcentaje, valor,
                numero_comprobante, fecha_emision, fecha_registro, usuario, observacion
         FROM retenciones_recibidas
         WHERE venta_id = ?1
         ORDER BY tipo ASC, fecha_registro ASC"
    ).map_err(|e| e.to_string())?;
    let rows: Vec<RetencionRecibida> = stmt.query_map(params![venta_id], |r| Ok(RetencionRecibida {
        id: Some(r.get(0)?),
        venta_id: r.get(1)?,
        tipo: r.get(2)?,
        codigo_sri: r.get(3)?,
        base_imponible: r.get(4)?,
        porcentaje: r.get(5)?,
        valor: r.get(6)?,
        numero_comprobante: r.get(7)?,
        fecha_emision: r.get(8)?,
        fecha_registro: r.get(9)?,
        usuario: r.get(10)?,
        observacion: r.get(11)?,
    })).map_err(|e| e.to_string())?
       .collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())?;
    Ok(rows)
}

// ─── TOTALES ──────────────────────────────────────────────────────────────

#[tauri::command]
pub fn total_retenciones_venta(
    db: State<'_, Database>,
    venta_id: i64,
) -> Result<TotalesRetencion, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let total_renta: f64 = conn.query_row(
        "SELECT COALESCE(SUM(valor), 0) FROM retenciones_recibidas WHERE venta_id = ?1 AND tipo = 'RENTA'",
        params![venta_id], |r| r.get(0),
    ).unwrap_or(0.0);
    let total_iva: f64 = conn.query_row(
        "SELECT COALESCE(SUM(valor), 0) FROM retenciones_recibidas WHERE venta_id = ?1 AND tipo = 'IVA'",
        params![venta_id], |r| r.get(0),
    ).unwrap_or(0.0);
    let cantidad: i64 = conn.query_row(
        "SELECT COUNT(*) FROM retenciones_recibidas WHERE venta_id = ?1",
        params![venta_id], |r| r.get(0),
    ).unwrap_or(0);
    Ok(TotalesRetencion {
        total_renta,
        total_iva,
        total: total_renta + total_iva,
        cantidad,
    })
}

// ─── REGISTRAR ────────────────────────────────────────────────────────────

#[tauri::command]
pub fn registrar_retencion(
    db: State<'_, Database>,
    sesion: State<'_, SesionState>,
    venta_id: i64,
    tipo: String,
    codigo_sri: String,
    base_imponible: f64,
    porcentaje: f64,
    valor: f64,
    numero_comprobante: String,
    fecha_emision: String,
    observacion: Option<String>,
) -> Result<i64, String> {
    // Validaciones
    let tipo_upper = tipo.to_uppercase();
    if tipo_upper != "RENTA" && tipo_upper != "IVA" {
        return Err("Tipo debe ser RENTA o IVA".to_string());
    }
    if base_imponible < 0.0 || valor < 0.0 || porcentaje < 0.0 {
        return Err("Base, valor y porcentaje deben ser positivos".to_string());
    }
    if numero_comprobante.trim().is_empty() {
        return Err("El número de comprobante es obligatorio".to_string());
    }
    if fecha_emision.trim().is_empty() {
        return Err("La fecha de emisión es obligatoria".to_string());
    }

    let usuario_nombre = {
        let s = sesion.sesion.lock().map_err(|e| e.to_string())?;
        s.as_ref().map(|s| s.nombre.clone())
    };

    let conn = db.conn.lock().map_err(|e| e.to_string())?;

    // Validar que la venta exista y obtener total para chequear que no se exceda
    let total_venta: f64 = conn.query_row(
        "SELECT total FROM ventas WHERE id = ?1",
        params![venta_id], |r| r.get(0),
    ).map_err(|_| "Venta no encontrada".to_string())?;

    // Calcular cuánto ya hay retenido + cuánto se ha cobrado para no exceder el total
    let ya_retenido: f64 = conn.query_row(
        "SELECT COALESCE(SUM(valor), 0) FROM retenciones_recibidas WHERE venta_id = ?1",
        params![venta_id], |r| r.get(0),
    ).unwrap_or(0.0);
    let ya_cobrado: f64 = conn.query_row(
        "SELECT COALESCE(SUM(monto), 0) FROM pagos_venta WHERE venta_id = ?1",
        params![venta_id], |r| r.get(0),
    ).unwrap_or(0.0);
    // Fallback: si no hay pagos_venta (venta legacy), usar ventas.monto_recibido
    let ya_cobrado = if ya_cobrado > 0.0 {
        ya_cobrado
    } else {
        conn.query_row(
            "SELECT COALESCE(monto_recibido, 0) FROM ventas WHERE id = ?1",
            params![venta_id], |r| r.get(0),
        ).unwrap_or(0.0)
    };

    if ya_retenido + valor > total_venta + 0.001 {
        let max = (total_venta - ya_retenido).max(0.0);
        return Err(format!(
            "El valor de retención (${:.2}) excede el saldo de la factura. Total: ${:.2}, ya retenido: ${:.2}, máximo permitido: ${:.2}",
            valor, total_venta, ya_retenido, max
        ));
    }

    let _ = ya_cobrado; // por ahora solo lo usamos para futuro reporte

    conn.execute(
        "INSERT INTO retenciones_recibidas
         (venta_id, tipo, codigo_sri, base_imponible, porcentaje, valor,
          numero_comprobante, fecha_emision, usuario, observacion)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
        params![
            venta_id, tipo_upper, codigo_sri, base_imponible, porcentaje, valor,
            numero_comprobante.trim(), fecha_emision, usuario_nombre, observacion,
        ],
    ).map_err(|e| e.to_string())?;

    Ok(conn.last_insert_rowid())
}

// ─── ELIMINAR ─────────────────────────────────────────────────────────────

#[tauri::command]
pub fn eliminar_retencion(
    db: State<'_, Database>,
    retencion_id: i64,
) -> Result<(), String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let n = conn.execute(
        "DELETE FROM retenciones_recibidas WHERE id = ?1",
        params![retencion_id],
    ).map_err(|e| e.to_string())?;
    if n == 0 {
        return Err("Retención no encontrada".to_string());
    }
    Ok(())
}
