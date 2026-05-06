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

    // v2.3.63 FIX: si verificamos una VENTA con forma_pago=TRANSFER pero la venta
    // ALSO tiene filas en pagos_venta (caso raro pero posible), marcar tambien
    // esas filas para evitar el contador "fantasma" que persistia despues de
    // verificar manualmente.
    if origen == "VENTA" {
        let _ = conn.execute(
            "UPDATE pagos_venta
             SET pago_estado = ?1, verificado_por = ?2, fecha_verificacion = ?3, motivo_verificacion = ?4
             WHERE venta_id = ?5
               AND UPPER(forma_pago) IN ('TRANSFER','TRANSFERENCIA')
               AND COALESCE(pago_estado, 'NO_APLICA') = 'REGISTRADO'",
            rusqlite::params![nuevo_estado, usuario_id, fecha, motivo, origen_id],
        );
    }
    // Caso simetrico: si verificamos un PAGO_MIXTO y todos los demas pagos de
    // esa venta tambien estan VERIFICADOS, marcar la venta padre tambien.
    if origen == "PAGO_MIXTO" {
        let _ = conn.execute(
            "UPDATE ventas
             SET pago_estado = ?1, verificado_por = ?2, fecha_verificacion = ?3
             WHERE id = (SELECT venta_id FROM pagos_venta WHERE id = ?4)
               AND COALESCE(pago_estado, 'NO_APLICA') = 'REGISTRADO'
               AND NOT EXISTS (
                   SELECT 1 FROM pagos_venta pv
                   WHERE pv.venta_id = ventas.id
                     AND UPPER(pv.forma_pago) IN ('TRANSFER','TRANSFERENCIA')
                     AND COALESCE(pv.pago_estado, 'NO_APLICA') = 'REGISTRADO'
               )",
            rusqlite::params![nuevo_estado, usuario_id, fecha, origen_id],
        );
    }

    Ok(())
}

/// Cuenta cuantas transferencias estan pendientes de verificar.
/// Util para mostrar un badge en el sidebar/menu del admin.
///
/// IMPORTANTE: filtra por los ultimos 60 dias para mantener consistencia con lo
/// que se muestra en /movimientos-bancarios (filtro "Este mes" por defecto).
/// Sin este filtro, transferencias viejas olvidadas se cuentan eternamente
/// y no se ven al hacer click en la alerta — confunde al usuario.
#[tauri::command]
pub fn contar_transferencias_pendientes(db: State<Database>) -> Result<i64, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;

    // v2.3.63 FIX: cleanup retroactivo de huerfanos.
    // Antes de v2.3.63 si verificabas una venta MIXTA, solo se actualizaba
    // ventas.pago_estado pero los pagos_venta correspondientes quedaban en
    // 'REGISTRADO'. Esto causaba el contador "fantasma" que persistia.
    // Esta limpieza es idempotente — si no hay huerfanos no hace nada.
    let _ = conn.execute(
        "UPDATE pagos_venta
         SET pago_estado = 'VERIFICADO'
         WHERE COALESCE(pago_estado, 'NO_APLICA') = 'REGISTRADO'
           AND venta_id IN (
               SELECT id FROM ventas
               WHERE COALESCE(pago_estado, 'NO_APLICA') = 'VERIFICADO'
                 AND anulada = 0
           )",
        [],
    );
    // Tambien limpiar ventas anuladas que aun figuran como REGISTRADO
    let _ = conn.execute(
        "UPDATE ventas SET pago_estado = 'NO_APLICA'
         WHERE anulada = 1 AND COALESCE(pago_estado, 'NO_APLICA') = 'REGISTRADO'",
        [],
    );

    let from_ventas: i64 = conn.query_row(
        "SELECT COUNT(*) FROM ventas
         WHERE UPPER(forma_pago) IN ('TRANSFER','TRANSFERENCIA')
           AND COALESCE(pago_estado, 'NO_APLICA') = 'REGISTRADO'
           AND anulada = 0
           AND DATE(fecha) >= DATE('now', '-60 days', 'localtime')",
        [], |r| r.get(0),
    ).unwrap_or(0);
    let from_pagos: i64 = conn.query_row(
        "SELECT COUNT(*) FROM pagos_venta pv
         JOIN ventas v ON v.id = pv.venta_id
         WHERE UPPER(pv.forma_pago) IN ('TRANSFER','TRANSFERENCIA')
           AND COALESCE(pv.pago_estado, 'NO_APLICA') = 'REGISTRADO'
           AND v.anulada = 0
           AND DATE(v.fecha) >= DATE('now', '-60 days', 'localtime')",
        [], |r| r.get(0),
    ).unwrap_or(0);
    Ok(from_ventas + from_pagos)
}

/// v2.3.64: detalle EXACTO de qué transferencias está contando el badge.
/// Útil para diagnóstico cuando el usuario reporta "dice 1 pero ya verifiqué todas".
/// Retorna lista con id, número, fecha, monto, origen (VENTA o PAGO_MIXTO) — sin
/// filtro de fecha (ve TODO lo que el query de contar suma).
#[tauri::command]
pub fn detalle_transferencias_pendientes(db: State<Database>) -> Result<Vec<serde_json::Value>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let mut out: Vec<serde_json::Value> = Vec::new();

    // De ventas (pago_estado='REGISTRADO')
    let mut stmt = conn.prepare(
        "SELECT v.id, v.numero, v.fecha, v.total, v.forma_pago,
                COALESCE(c.nombre, 'CONSUMIDOR FINAL') as cliente_nombre,
                COALESCE(v.pago_estado, 'NO_APLICA') as estado,
                v.anulada
         FROM ventas v
         LEFT JOIN clientes c ON v.cliente_id = c.id
         WHERE UPPER(v.forma_pago) IN ('TRANSFER','TRANSFERENCIA')
           AND COALESCE(v.pago_estado, 'NO_APLICA') = 'REGISTRADO'
         ORDER BY v.fecha DESC"
    ).map_err(|e| e.to_string())?;

    let rows = stmt.query_map([], |row| Ok(serde_json::json!({
        "origen": "VENTA",
        "id": row.get::<_, i64>(0)?,
        "numero": row.get::<_, String>(1)?,
        "fecha": row.get::<_, String>(2)?,
        "monto": row.get::<_, f64>(3)?,
        "forma_pago": row.get::<_, String>(4)?,
        "cliente": row.get::<_, String>(5)?,
        "pago_estado": row.get::<_, String>(6)?,
        "anulada": row.get::<_, i32>(7)?,
    }))).map_err(|e| e.to_string())?;
    for r in rows { if let Ok(v) = r { out.push(v); } }

    // De pagos_venta (de ventas mixtas)
    let mut stmt2 = conn.prepare(
        "SELECT pv.id, v.numero, v.fecha, pv.monto, pv.forma_pago,
                COALESCE(c.nombre, 'CONSUMIDOR FINAL') as cliente_nombre,
                COALESCE(pv.pago_estado, 'NO_APLICA') as estado,
                v.anulada, v.id as venta_id
         FROM pagos_venta pv
         JOIN ventas v ON v.id = pv.venta_id
         LEFT JOIN clientes c ON v.cliente_id = c.id
         WHERE UPPER(pv.forma_pago) IN ('TRANSFER','TRANSFERENCIA')
           AND COALESCE(pv.pago_estado, 'NO_APLICA') = 'REGISTRADO'
         ORDER BY v.fecha DESC"
    ).map_err(|e| e.to_string())?;

    let rows2 = stmt2.query_map([], |row| Ok(serde_json::json!({
        "origen": "PAGO_MIXTO",
        "id": row.get::<_, i64>(0)?,
        "numero": row.get::<_, String>(1)?,
        "fecha": row.get::<_, String>(2)?,
        "monto": row.get::<_, f64>(3)?,
        "forma_pago": row.get::<_, String>(4)?,
        "cliente": row.get::<_, String>(5)?,
        "pago_estado": row.get::<_, String>(6)?,
        "anulada": row.get::<_, i32>(7)?,
        "venta_id": row.get::<_, i64>(8)?,
    }))).map_err(|e| e.to_string())?;
    for r in rows2 { if let Ok(v) = r { out.push(v); } }

    Ok(out)
}

/// v2.3.64: marcar como VERIFICADA una transferencia específica que está
/// "atrapada" en estado REGISTRADO. Útil cuando el cleanup automático no
/// la pesca (p.ej. la venta padre está REGISTRADO también, no VERIFICADO).
/// Solo admin. Sirve como "última opción" para limpiar el badge fantasma.
#[tauri::command]
pub fn forzar_marcar_transferencia_verificada(
    db: State<Database>,
    sesion: State<SesionState>,
    origen: String,  // "VENTA" o "PAGO_MIXTO"
    id: i64,
    motivo: String,
) -> Result<(), String> {
    let sesion_guard = sesion.sesion.lock().map_err(|e| e.to_string())?;
    let sesion_actual = sesion_guard.as_ref()
        .ok_or("Debe iniciar sesión".to_string())?;
    if sesion_actual.rol != "ADMIN" {
        return Err("Solo administradores pueden forzar verificación".to_string());
    }
    let usuario_id = sesion_actual.usuario_id;
    drop(sesion_guard);

    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let fecha = chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
    let motivo_full = format!("[FORZADO] {}", motivo);

    let tabla = match origen.as_str() {
        "VENTA" => "ventas",
        "PAGO_MIXTO" => "pagos_venta",
        _ => return Err("Origen invalido".to_string()),
    };
    let sql = format!(
        "UPDATE {} SET pago_estado = 'VERIFICADO', verificado_por = ?1,
         fecha_verificacion = ?2, motivo_verificacion = ?3 WHERE id = ?4",
        tabla
    );
    let n = conn.execute(&sql, rusqlite::params![usuario_id, fecha, motivo_full, id])
        .map_err(|e| e.to_string())?;
    if n == 0 {
        return Err("Transferencia no encontrada".to_string());
    }
    Ok(())
}
