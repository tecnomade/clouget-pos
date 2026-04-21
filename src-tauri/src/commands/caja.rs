use crate::db::{Database, SesionState};
use crate::models::{Caja, ResumenCaja};
use tauri::State;

#[tauri::command]
pub fn abrir_caja(
    db: State<Database>,
    sesion: State<SesionState>,
    monto_inicial: f64,
) -> Result<Caja, String> {
    // Obtener usuario de la sesión
    let sesion_guard = sesion.sesion.lock().map_err(|e| e.to_string())?;
    let sesion_actual = sesion_guard
        .as_ref()
        .ok_or("Debe iniciar sesión para abrir la caja".to_string())?;
    let usuario_nombre = sesion_actual.nombre.clone();
    let usuario_id = sesion_actual.usuario_id;
    drop(sesion_guard);

    let conn = db.conn.lock().map_err(|e| e.to_string())?;

    // Verificar que no haya caja abierta
    let caja_abierta: bool = conn
        .query_row(
            "SELECT COUNT(*) FROM caja WHERE estado = 'ABIERTA'",
            [],
            |row| row.get::<_, i64>(0),
        )
        .map(|count| count > 0)
        .unwrap_or(false);

    if caja_abierta {
        return Err("Ya existe una caja abierta. Ciérrela primero.".to_string());
    }

    conn.execute(
        "INSERT INTO caja (monto_inicial, monto_esperado, estado, usuario, usuario_id)
         VALUES (?1, ?1, 'ABIERTA', ?2, ?3)",
        rusqlite::params![monto_inicial, usuario_nombre, usuario_id],
    )
    .map_err(|e| e.to_string())?;

    let id = conn.last_insert_rowid();

    Ok(Caja {
        id: Some(id),
        fecha_apertura: None,
        fecha_cierre: None,
        monto_inicial,
        monto_ventas: 0.0,
        monto_esperado: monto_inicial,
        monto_real: None,
        diferencia: None,
        estado: "ABIERTA".to_string(),
        usuario: Some(usuario_nombre),
        usuario_id: Some(usuario_id),
        observacion: None,
    })
}

#[tauri::command]
pub fn cerrar_caja(
    db: State<Database>,
    sesion: State<SesionState>,
    monto_real: f64,
    observacion: Option<String>,
) -> Result<ResumenCaja, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;

    let caja_id: i64 = conn
        .query_row(
            "SELECT id FROM caja WHERE estado = 'ABIERTA' LIMIT 1",
            [],
            |row| row.get(0),
        )
        .map_err(|_| "No hay caja abierta".to_string())?;

    // Calcular totales
    let total_ventas: f64 = conn
        .query_row(
            "SELECT COALESCE(SUM(total), 0) FROM ventas
             WHERE created_at >= (SELECT fecha_apertura FROM caja WHERE id = ?1)
             AND anulada = 0 AND estado = 'COMPLETADA'",
            rusqlite::params![caja_id],
            |row| row.get(0),
        )
        .unwrap_or(0.0);

    let num_ventas: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM ventas
             WHERE created_at >= (SELECT fecha_apertura FROM caja WHERE id = ?1)
             AND anulada = 0 AND estado = 'COMPLETADA'",
            rusqlite::params![caja_id],
            |row| row.get(0),
        )
        .unwrap_or(0);

    let total_efectivo: f64 = conn
        .query_row(
            "SELECT COALESCE(SUM(total), 0) FROM ventas
             WHERE created_at >= (SELECT fecha_apertura FROM caja WHERE id = ?1)
             AND forma_pago = 'EFECTIVO' AND anulada = 0 AND estado = 'COMPLETADA'",
            rusqlite::params![caja_id],
            |row| row.get(0),
        )
        .unwrap_or(0.0);

    let total_gastos: f64 = conn
        .query_row(
            "SELECT COALESCE(SUM(monto), 0) FROM gastos WHERE caja_id = ?1",
            rusqlite::params![caja_id],
            |row| row.get(0),
        )
        .unwrap_or(0.0);

    let monto_inicial: f64 = conn
        .query_row(
            "SELECT monto_inicial FROM caja WHERE id = ?1",
            rusqlite::params![caja_id],
            |row| row.get(0),
        )
        .unwrap_or(0.0);

    // Cobros de cuentas por cobrar en EFECTIVO (cuenta para arqueo de caja)
    let total_cobros_efectivo: f64 = conn
        .query_row(
            "SELECT COALESCE(SUM(p.monto), 0) FROM pagos_cuenta p
             WHERE p.fecha >= (SELECT fecha_apertura FROM caja WHERE id = ?1)
             AND p.forma_pago = 'EFECTIVO' AND p.estado = 'CONFIRMADO'",
            rusqlite::params![caja_id],
            |row| row.get(0),
        )
        .unwrap_or(0.0);

    // Cobros de cuentas por cobrar en TRANSFERENCIA/BANCO (NO cuenta para arqueo)
    let total_cobros_banco: f64 = conn
        .query_row(
            "SELECT COALESCE(SUM(p.monto), 0) FROM pagos_cuenta p
             WHERE p.fecha >= (SELECT fecha_apertura FROM caja WHERE id = ?1)
             AND p.forma_pago = 'TRANSFERENCIA' AND p.estado = 'CONFIRMADO'",
            rusqlite::params![caja_id],
            |row| row.get(0),
        )
        .unwrap_or(0.0);

    let total_retiros: f64 = conn
        .query_row(
            "SELECT COALESCE(SUM(monto), 0) FROM retiros_caja WHERE caja_id = ?1",
            rusqlite::params![caja_id],
            |row| row.get(0),
        )
        .unwrap_or(0.0);

    let monto_esperado = monto_inicial + total_efectivo + total_cobros_efectivo - total_gastos - total_retiros;
    let diferencia = monto_real - monto_esperado;

    conn.execute(
        "UPDATE caja SET fecha_cierre = datetime('now','localtime'),
         monto_ventas = ?1, monto_esperado = ?2, monto_real = ?3,
         diferencia = ?4, estado = 'CERRADA', observacion = ?5
         WHERE id = ?6",
        rusqlite::params![total_ventas, monto_esperado, monto_real, diferencia, observacion, caja_id],
    )
    .map_err(|e| e.to_string())?;

    // Auto-cerrar sesión al cerrar caja
    drop(conn);
    let mut sesion_guard = sesion.sesion.lock().map_err(|e| e.to_string())?;
    *sesion_guard = None;

    let caja = Caja {
        id: Some(caja_id),
        fecha_apertura: None,
        fecha_cierre: None,
        monto_inicial,
        monto_ventas: total_ventas,
        monto_esperado,
        monto_real: Some(monto_real),
        diferencia: Some(diferencia),
        estado: "CERRADA".to_string(),
        usuario: None,
        usuario_id: None,
        observacion,
    };

    Ok(ResumenCaja {
        caja,
        total_ventas,
        num_ventas,
        total_efectivo,
        total_gastos,
        total_cobros_efectivo,
        total_cobros_banco,
        total_retiros,
    })
}

#[tauri::command]
pub fn registrar_retiro(
    db: State<Database>,
    sesion: State<SesionState>,
    monto: f64,
    motivo: String,
    banco_id: Option<i64>,
    referencia: Option<String>,
) -> Result<serde_json::Value, String> {
    // Get session user
    let sesion_guard = sesion.sesion.lock().map_err(|e| e.to_string())?;
    let sesion_actual = sesion_guard
        .as_ref()
        .ok_or("Debe iniciar sesión para registrar un retiro".to_string())?;
    let usuario_nombre = sesion_actual.nombre.clone();
    let usuario_id = sesion_actual.usuario_id;
    drop(sesion_guard);

    if monto <= 0.0 {
        return Err("El monto del retiro debe ser mayor a 0".to_string());
    }

    let conn = db.conn.lock().map_err(|e| e.to_string())?;

    // Find open caja
    let caja_id: i64 = conn
        .query_row(
            "SELECT id FROM caja WHERE estado = 'ABIERTA' LIMIT 1",
            [],
            |row| row.get(0),
        )
        .map_err(|_| "No hay caja abierta para registrar el retiro".to_string())?;

    let estado = if banco_id.is_some() { "EN_TRANSITO" } else { "SIN_DEPOSITO" };

    conn.execute(
        "INSERT INTO retiros_caja (caja_id, monto, motivo, banco_id, referencia, usuario, usuario_id, estado)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        rusqlite::params![caja_id, monto, motivo, banco_id, referencia, usuario_nombre, usuario_id, estado],
    )
    .map_err(|e| e.to_string())?;

    let id = conn.last_insert_rowid();
    let fecha: String = conn
        .query_row(
            "SELECT fecha FROM retiros_caja WHERE id = ?1",
            rusqlite::params![id],
            |row| row.get(0),
        )
        .unwrap_or_default();

    Ok(serde_json::json!({
        "id": id,
        "monto": monto,
        "motivo": motivo,
        "fecha": fecha,
        "usuario": usuario_nombre,
        "estado": estado,
    }))
}

#[tauri::command]
pub fn listar_retiros_caja(
    db: State<Database>,
    caja_id: i64,
) -> Result<Vec<serde_json::Value>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;

    let mut stmt = conn
        .prepare(
            "SELECT r.id, r.caja_id, r.monto, r.motivo, r.banco_id, r.referencia,
                    r.usuario, r.usuario_id, r.fecha, cb.nombre as banco_nombre,
                    r.estado, r.comprobante_imagen
             FROM retiros_caja r
             LEFT JOIN cuentas_banco cb ON r.banco_id = cb.id
             WHERE r.caja_id = ?1
             ORDER BY r.fecha DESC",
        )
        .map_err(|e| e.to_string())?;

    let retiros = stmt
        .query_map(rusqlite::params![caja_id], |row| {
            Ok(serde_json::json!({
                "id": row.get::<_, i64>(0)?,
                "caja_id": row.get::<_, i64>(1)?,
                "monto": row.get::<_, f64>(2)?,
                "motivo": row.get::<_, String>(3)?,
                "banco_id": row.get::<_, Option<i64>>(4)?,
                "referencia": row.get::<_, Option<String>>(5)?,
                "usuario": row.get::<_, String>(6)?,
                "usuario_id": row.get::<_, Option<i64>>(7)?,
                "fecha": row.get::<_, String>(8)?,
                "banco_nombre": row.get::<_, Option<String>>(9)?,
                "estado": row.get::<_, Option<String>>(10)?.unwrap_or_else(|| "SIN_DEPOSITO".to_string()),
                "comprobante_imagen": row.get::<_, Option<String>>(11)?,
            }))
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    Ok(retiros)
}

#[tauri::command]
pub fn confirmar_deposito(
    db: State<Database>,
    retiro_id: i64,
    referencia: String,
    comprobante_imagen: Option<String>,
) -> Result<(), String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let rows = conn.execute(
        "UPDATE retiros_caja SET estado = 'DEPOSITADO', referencia = ?1, comprobante_imagen = ?2 WHERE id = ?3 AND estado = 'EN_TRANSITO'",
        rusqlite::params![referencia, comprobante_imagen, retiro_id],
    ).map_err(|e| e.to_string())?;
    if rows == 0 {
        return Err("No se encontró el retiro en tránsito".to_string());
    }
    Ok(())
}

#[tauri::command]
pub fn obtener_caja_abierta(db: State<Database>) -> Result<Option<Caja>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;

    let result = conn.query_row(
        "SELECT id, fecha_apertura, fecha_cierre, monto_inicial, monto_ventas,
         monto_esperado, monto_real, diferencia, estado, usuario, observacion, usuario_id
         FROM caja WHERE estado = 'ABIERTA' LIMIT 1",
        [],
        |row| {
            Ok(Caja {
                id: Some(row.get(0)?),
                fecha_apertura: row.get(1)?,
                fecha_cierre: row.get(2)?,
                monto_inicial: row.get(3)?,
                monto_ventas: row.get(4)?,
                monto_esperado: row.get(5)?,
                monto_real: row.get(6)?,
                diferencia: row.get(7)?,
                estado: row.get(8)?,
                usuario: row.get(9)?,
                observacion: row.get(10)?,
                usuario_id: row.get(11)?,
            })
        },
    );

    match result {
        Ok(caja) => Ok(Some(caja)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(e.to_string()),
    }
}
