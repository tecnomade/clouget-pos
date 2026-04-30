// Verificacion de transferencias de pago (v2.3.33+)
//
// Flujo:
// 1. Cuando una venta se cobra con TRANSFER (pura o como parte de MIXTO):
//    - Si el cajero es ADMIN -> pago_estado = 'VERIFICADO' automaticamente
//    - Si es cajero comun     -> pago_estado = 'REGISTRADO' (pendiente revision)
// 2. Admin entra a "Verificar Transferencias" y ve la lista de REGISTRADAS.
// 3. Admin marca cada una como VERIFICADO o RECHAZADO (con motivo).
// 4. Esto NO afecta el cuadre de caja: las transferencias nunca entran al
//    efectivo. Solo es trazabilidad.

use crate::db::{Database, SesionState};
use serde::{Deserialize, Serialize};
use tauri::State;

#[derive(Debug, Serialize, Deserialize)]
pub struct TransferenciaPago {
    pub origen: String, // "VENTA" o "PAGO_MIXTO"
    pub origen_id: i64, // venta.id o pagos_venta.id
    pub venta_id: i64,
    pub venta_numero: String,
    pub fecha: String,
    pub cliente_nombre: Option<String>,
    pub cajero: Option<String>,
    pub monto: f64,
    pub banco_id: Option<i64>,
    pub banco_nombre: Option<String>,
    pub referencia: Option<String>,
    pub comprobante_imagen: Option<String>,
    pub pago_estado: String,
    pub verificado_por: Option<i64>,
    pub verificado_por_nombre: Option<String>,
    pub fecha_verificacion: Option<String>,
    pub motivo_verificacion: Option<String>,
}

/// Lista todas las transferencias asociadas a ventas, con filtros opcionales.
/// Si `solo_pendientes=true`, retorna solo estado='REGISTRADO'.
/// Si no, retorna todas (REGISTRADO + VERIFICADO + RECHAZADO).
#[tauri::command]
pub fn listar_transferencias_verificacion(
    db: State<Database>,
    solo_pendientes: Option<bool>,
    fecha_desde: Option<String>,
    fecha_hasta: Option<String>,
) -> Result<Vec<TransferenciaPago>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let solo_pend = solo_pendientes.unwrap_or(false);
    let desde = fecha_desde.unwrap_or_else(|| "1970-01-01".to_string());
    let hasta = fecha_hasta.unwrap_or_else(|| "2999-12-31 23:59:59".to_string());

    let mut out: Vec<TransferenciaPago> = Vec::new();

    // === Origen 1: ventas con forma_pago = TRANSFER (puras) ===
    let estado_filter = if solo_pend { " AND v.pago_estado = 'REGISTRADO'" } else { " AND v.pago_estado != 'NO_APLICA'" };
    let sql = format!(
        "SELECT v.id, v.numero, v.fecha, c.nombre as cliente_nombre, v.usuario, v.total,
                v.banco_id, cb.nombre as banco_nombre, v.referencia_pago, v.comprobante_imagen,
                COALESCE(v.pago_estado, 'NO_APLICA'), v.verificado_por, u.nombre as verif_nombre,
                v.fecha_verificacion, v.motivo_verificacion
         FROM ventas v
         LEFT JOIN clientes c ON v.cliente_id = c.id
         LEFT JOIN cuentas_banco cb ON v.banco_id = cb.id
         LEFT JOIN usuarios u ON v.verificado_por = u.id
         WHERE UPPER(v.forma_pago) IN ('TRANSFER','TRANSFERENCIA')
           AND v.anulada = 0
           AND DATE(v.fecha) BETWEEN DATE(?1) AND DATE(?2)
           {}
         ORDER BY v.fecha DESC",
        estado_filter
    );
    let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;
    let rows = stmt.query_map(rusqlite::params![desde, hasta], |r| {
        Ok(TransferenciaPago {
            origen: "VENTA".to_string(),
            origen_id: r.get(0)?,
            venta_id: r.get(0)?,
            venta_numero: r.get(1)?,
            fecha: r.get(2)?,
            cliente_nombre: r.get(3)?,
            cajero: r.get(4)?,
            monto: r.get(5)?,
            banco_id: r.get(6)?,
            banco_nombre: r.get(7)?,
            referencia: r.get(8)?,
            comprobante_imagen: r.get(9)?,
            pago_estado: r.get(10)?,
            verificado_por: r.get(11)?,
            verificado_por_nombre: r.get(12)?,
            fecha_verificacion: r.get(13)?,
            motivo_verificacion: r.get(14)?,
        })
    }).map_err(|e| e.to_string())?;
    for row in rows {
        if let Ok(r) = row { out.push(r); }
    }

    // === Origen 2: pagos_venta TRANSFER (componentes de MIXTO) ===
    let estado_filter_p = if solo_pend { " AND pv.pago_estado = 'REGISTRADO'" } else { " AND pv.pago_estado != 'NO_APLICA'" };
    let sql_p = format!(
        "SELECT pv.id, v.id, v.numero, v.fecha, c.nombre as cliente_nombre, v.usuario, pv.monto,
                pv.banco_id, cb.nombre as banco_nombre, pv.referencia, pv.comprobante_imagen,
                COALESCE(pv.pago_estado, 'NO_APLICA'), pv.verificado_por, u.nombre as verif_nombre,
                pv.fecha_verificacion, pv.motivo_verificacion
         FROM pagos_venta pv
         JOIN ventas v ON v.id = pv.venta_id
         LEFT JOIN clientes c ON v.cliente_id = c.id
         LEFT JOIN cuentas_banco cb ON pv.banco_id = cb.id
         LEFT JOIN usuarios u ON pv.verificado_por = u.id
         WHERE UPPER(pv.forma_pago) IN ('TRANSFER','TRANSFERENCIA')
           AND v.anulada = 0
           AND DATE(v.fecha) BETWEEN DATE(?1) AND DATE(?2)
           {}
         ORDER BY v.fecha DESC",
        estado_filter_p
    );
    let mut stmt2 = conn.prepare(&sql_p).map_err(|e| e.to_string())?;
    let rows2 = stmt2.query_map(rusqlite::params![desde, hasta], |r| {
        Ok(TransferenciaPago {
            origen: "PAGO_MIXTO".to_string(),
            origen_id: r.get(0)?,
            venta_id: r.get(1)?,
            venta_numero: r.get(2)?,
            fecha: r.get(3)?,
            cliente_nombre: r.get(4)?,
            cajero: r.get(5)?,
            monto: r.get(6)?,
            banco_id: r.get(7)?,
            banco_nombre: r.get(8)?,
            referencia: r.get(9)?,
            comprobante_imagen: r.get(10)?,
            pago_estado: r.get(11)?,
            verificado_por: r.get(12)?,
            verificado_por_nombre: r.get(13)?,
            fecha_verificacion: r.get(14)?,
            motivo_verificacion: r.get(15)?,
        })
    }).map_err(|e| e.to_string())?;
    for row in rows2 {
        if let Ok(r) = row { out.push(r); }
    }

    // Ordenar todo por fecha desc
    out.sort_by(|a, b| b.fecha.cmp(&a.fecha));

    Ok(out)
}

/// Verifica (o rechaza) una transferencia. Solo admin puede llamar este comando.
/// Para origen='VENTA' actualiza ventas.pago_estado.
/// Para origen='PAGO_MIXTO' actualiza pagos_venta.pago_estado.
#[tauri::command]
pub fn verificar_transferencia(
    db: State<Database>,
    sesion: State<SesionState>,
    origen: String,
    origen_id: i64,
    aprobar: bool,
    motivo: Option<String>,
) -> Result<(), String> {
    let sesion_guard = sesion.sesion.lock().map_err(|e| e.to_string())?;
    let sesion_actual = sesion_guard
        .as_ref()
        .ok_or("Debe iniciar sesion".to_string())?;
    if sesion_actual.rol != "ADMIN" {
        return Err("Solo administradores pueden verificar transferencias".to_string());
    }
    let usuario_id = sesion_actual.usuario_id;
    drop(sesion_guard);

    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let nuevo_estado = if aprobar { "VERIFICADO" } else { "RECHAZADO" };
    let fecha = chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string();

    let tabla = match origen.as_str() {
        "VENTA" => "ventas",
        "PAGO_MIXTO" => "pagos_venta",
        _ => return Err("Origen invalido (debe ser 'VENTA' o 'PAGO_MIXTO')".to_string()),
    };

    let sql = format!(
        "UPDATE {} SET pago_estado = ?1, verificado_por = ?2, fecha_verificacion = ?3, motivo_verificacion = ?4
         WHERE id = ?5",
        tabla
    );
    let n = conn.execute(
        &sql,
        rusqlite::params![nuevo_estado, usuario_id, fecha, motivo, origen_id],
    ).map_err(|e| e.to_string())?;

    if n == 0 {
        return Err("Transferencia no encontrada".to_string());
    }

    Ok(())
}

/// Cuenta cuantas transferencias estan pendientes de verificar.
/// Util para mostrar un badge en el sidebar/menu del admin.
#[tauri::command]
pub fn contar_transferencias_pendientes(db: State<Database>) -> Result<i64, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let from_ventas: i64 = conn.query_row(
        "SELECT COUNT(*) FROM ventas
         WHERE UPPER(forma_pago) IN ('TRANSFER','TRANSFERENCIA')
           AND COALESCE(pago_estado, 'NO_APLICA') = 'REGISTRADO'
           AND anulada = 0",
        [], |r| r.get(0),
    ).unwrap_or(0);
    let from_pagos: i64 = conn.query_row(
        "SELECT COUNT(*) FROM pagos_venta pv
         JOIN ventas v ON v.id = pv.venta_id
         WHERE UPPER(pv.forma_pago) IN ('TRANSFER','TRANSFERENCIA')
           AND COALESCE(pv.pago_estado, 'NO_APLICA') = 'REGISTRADO'
           AND v.anulada = 0",
        [], |r| r.get(0),
    ).unwrap_or(0);
    Ok(from_ventas + from_pagos)
}
