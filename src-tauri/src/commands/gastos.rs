use crate::db::Database;
use crate::models::Gasto;
use tauri::State;

#[tauri::command]
pub fn crear_gasto(db: State<Database>, gasto: Gasto) -> Result<Gasto, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;

    // Obtener caja abierta si existe
    let caja_id: Option<i64> = conn
        .query_row(
            "SELECT id FROM caja WHERE estado = 'ABIERTA' LIMIT 1",
            [],
            |row| row.get(0),
        )
        .ok();

    conn.execute(
        "INSERT INTO gastos (descripcion, monto, categoria, caja_id, observacion)
         VALUES (?1, ?2, ?3, ?4, ?5)",
        rusqlite::params![
            gasto.descripcion,
            gasto.monto,
            gasto.categoria,
            caja_id,
            gasto.observacion,
        ],
    )
    .map_err(|e| e.to_string())?;

    let id = conn.last_insert_rowid();

    // Obtener el gasto insertado con su fecha
    let resultado = conn
        .query_row(
            "SELECT id, descripcion, monto, categoria, fecha, caja_id, observacion
             FROM gastos WHERE id = ?1",
            rusqlite::params![id],
            |row| {
                Ok(Gasto {
                    id: Some(row.get(0)?),
                    descripcion: row.get(1)?,
                    monto: row.get(2)?,
                    categoria: row.get(3)?,
                    fecha: row.get(4)?,
                    caja_id: row.get(5)?,
                    observacion: row.get(6)?,
                })
            },
        )
        .map_err(|e| e.to_string())?;

    Ok(resultado)
}

#[tauri::command]
pub fn listar_gastos_dia(db: State<Database>, fecha: String) -> Result<Vec<Gasto>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;

    let mut stmt = conn
        .prepare(
            "SELECT id, descripcion, monto, categoria, fecha, caja_id, observacion
             FROM gastos
             WHERE date(fecha) = date(?1)
             ORDER BY fecha DESC",
        )
        .map_err(|e| e.to_string())?;

    let gastos = stmt
        .query_map(rusqlite::params![fecha], |row| {
            Ok(Gasto {
                id: Some(row.get(0)?),
                descripcion: row.get(1)?,
                monto: row.get(2)?,
                categoria: row.get(3)?,
                fecha: row.get(4)?,
                caja_id: row.get(5)?,
                observacion: row.get(6)?,
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    Ok(gastos)
}

#[tauri::command]
pub fn eliminar_gasto(db: State<Database>, id: i64) -> Result<(), String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;

    // Verificar que el gasto existe
    conn.query_row(
        "SELECT id FROM gastos WHERE id = ?1",
        rusqlite::params![id],
        |row| row.get::<_, i64>(0),
    )
    .map_err(|_| "Gasto no encontrado".to_string())?;

    conn.execute("DELETE FROM gastos WHERE id = ?1", rusqlite::params![id])
        .map_err(|e| e.to_string())?;

    Ok(())
}
