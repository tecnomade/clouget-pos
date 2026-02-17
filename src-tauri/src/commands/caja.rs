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
             AND anulada = 0",
            rusqlite::params![caja_id],
            |row| row.get(0),
        )
        .unwrap_or(0.0);

    let num_ventas: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM ventas
             WHERE created_at >= (SELECT fecha_apertura FROM caja WHERE id = ?1)
             AND anulada = 0",
            rusqlite::params![caja_id],
            |row| row.get(0),
        )
        .unwrap_or(0);

    let total_efectivo: f64 = conn
        .query_row(
            "SELECT COALESCE(SUM(total), 0) FROM ventas
             WHERE created_at >= (SELECT fecha_apertura FROM caja WHERE id = ?1)
             AND forma_pago = 'EFECTIVO' AND anulada = 0",
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

    let monto_esperado = monto_inicial + total_efectivo - total_gastos;
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
    })
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
