use crate::db::Database;
use serde::{Deserialize, Serialize};
use tauri::State;

#[derive(Debug, Serialize, Deserialize)]
pub struct TransferenciaStock {
    pub id: Option<i64>,
    pub producto_id: i64,
    pub producto_nombre: Option<String>,
    pub origen_establecimiento_id: i64,
    pub origen_nombre: Option<String>,
    pub destino_establecimiento_id: i64,
    pub destino_nombre: Option<String>,
    pub cantidad: f64,
    pub estado: String,
    pub usuario: Option<String>,
    pub created_at: Option<String>,
    pub recibida_at: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct StockEstablecimiento {
    pub establecimiento_id: i64,
    pub establecimiento_nombre: String,
    pub establecimiento_codigo: String,
    pub stock_actual: f64,
    pub stock_minimo: f64,
}

/// Crea una transferencia de stock entre establecimientos.
/// Descuenta stock del origen inmediatamente.
#[tauri::command]
pub fn crear_transferencia(
    db: State<Database>,
    producto_id: i64,
    origen_establecimiento_id: i64,
    destino_establecimiento_id: i64,
    cantidad: f64,
    usuario: Option<String>,
) -> Result<TransferenciaStock, String> {
    if cantidad <= 0.0 {
        return Err("La cantidad debe ser mayor a 0".to_string());
    }
    if origen_establecimiento_id == destino_establecimiento_id {
        return Err("Origen y destino no pueden ser el mismo establecimiento".to_string());
    }

    let conn = db.conn.lock().map_err(|e| e.to_string())?;

    // Verificar stock disponible en origen
    let stock_origen: f64 = conn
        .query_row(
            "SELECT COALESCE(stock_actual, 0) FROM stock_establecimiento WHERE producto_id = ?1 AND establecimiento_id = ?2",
            rusqlite::params![producto_id, origen_establecimiento_id],
            |row| row.get(0),
        )
        .unwrap_or(0.0);

    if stock_origen < cantidad {
        return Err(format!("Stock insuficiente en origen. Disponible: {:.2}", stock_origen));
    }

    // Descontar del origen
    conn.execute(
        "UPDATE stock_establecimiento SET stock_actual = stock_actual - ?1 WHERE producto_id = ?2 AND establecimiento_id = ?3",
        rusqlite::params![cantidad, producto_id, origen_establecimiento_id],
    ).map_err(|e| e.to_string())?;

    // Crear registro de transferencia
    conn.execute(
        "INSERT INTO transferencias_stock (producto_id, origen_establecimiento_id, destino_establecimiento_id, cantidad, estado, usuario)
         VALUES (?1, ?2, ?3, ?4, 'PENDIENTE', ?5)",
        rusqlite::params![producto_id, origen_establecimiento_id, destino_establecimiento_id, cantidad, usuario],
    ).map_err(|e| e.to_string())?;

    let id = conn.last_insert_rowid();

    Ok(TransferenciaStock {
        id: Some(id),
        producto_id,
        producto_nombre: None,
        origen_establecimiento_id,
        origen_nombre: None,
        destino_establecimiento_id,
        destino_nombre: None,
        cantidad,
        estado: "PENDIENTE".to_string(),
        usuario,
        created_at: None,
        recibida_at: None,
    })
}

/// Marca una transferencia como recibida. Incrementa stock en destino.
#[tauri::command]
pub fn recibir_transferencia(
    db: State<Database>,
    id: i64,
) -> Result<(), String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;

    let (producto_id, destino_id, cantidad, estado): (i64, i64, f64, String) = conn
        .query_row(
            "SELECT producto_id, destino_establecimiento_id, cantidad, estado FROM transferencias_stock WHERE id = ?1",
            rusqlite::params![id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
        )
        .map_err(|_| "Transferencia no encontrada".to_string())?;

    if estado != "PENDIENTE" {
        return Err("Esta transferencia ya fue procesada".to_string());
    }

    // Incrementar stock en destino (crear registro si no existe)
    conn.execute(
        "INSERT INTO stock_establecimiento (producto_id, establecimiento_id, stock_actual, stock_minimo)
         VALUES (?1, ?2, ?3, 0)
         ON CONFLICT(producto_id, establecimiento_id) DO UPDATE SET stock_actual = stock_actual + ?3",
        rusqlite::params![producto_id, destino_id, cantidad],
    ).map_err(|e| e.to_string())?;

    // Marcar como recibida
    conn.execute(
        "UPDATE transferencias_stock SET estado = 'RECIBIDA', recibida_at = datetime('now','localtime') WHERE id = ?1",
        rusqlite::params![id],
    ).map_err(|e| e.to_string())?;

    Ok(())
}

/// Lista transferencias (filtradas opcionalmente por establecimiento y estado).
#[tauri::command]
pub fn listar_transferencias(
    db: State<Database>,
    establecimiento_id: Option<i64>,
    estado: Option<String>,
) -> Result<Vec<TransferenciaStock>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;

    let mut sql = String::from(
        "SELECT t.id, t.producto_id, p.nombre, t.origen_establecimiento_id, eo.nombre,
                t.destino_establecimiento_id, ed.nombre, t.cantidad, t.estado, t.usuario,
                t.created_at, t.recibida_at
         FROM transferencias_stock t
         JOIN productos p ON t.producto_id = p.id
         JOIN establecimientos eo ON t.origen_establecimiento_id = eo.id
         JOIN establecimientos ed ON t.destino_establecimiento_id = ed.id
         WHERE 1=1"
    );

    if let Some(ref eid) = establecimiento_id {
        sql.push_str(&format!(" AND (t.origen_establecimiento_id = {} OR t.destino_establecimiento_id = {})", eid, eid));
    }
    if let Some(ref est) = estado {
        sql.push_str(&format!(" AND t.estado = '{}'", est));
    }
    sql.push_str(" ORDER BY t.created_at DESC LIMIT 100");

    let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;

    let transferencias = stmt
        .query_map([], |row| {
            Ok(TransferenciaStock {
                id: Some(row.get(0)?),
                producto_id: row.get(1)?,
                producto_nombre: row.get(2)?,
                origen_establecimiento_id: row.get(3)?,
                origen_nombre: row.get(4)?,
                destino_establecimiento_id: row.get(5)?,
                destino_nombre: row.get(6)?,
                cantidad: row.get(7)?,
                estado: row.get(8)?,
                usuario: row.get(9)?,
                created_at: row.get(10)?,
                recibida_at: row.get(11)?,
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    Ok(transferencias)
}

/// Consulta el stock de un producto en todos los establecimientos.
#[tauri::command]
pub fn stock_por_establecimiento(
    db: State<Database>,
    producto_id: i64,
) -> Result<Vec<StockEstablecimiento>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;

    let mut stmt = conn
        .prepare(
            "SELECT e.id, e.nombre, e.codigo,
                    COALESCE(se.stock_actual, 0), COALESCE(se.stock_minimo, 0)
             FROM establecimientos e
             LEFT JOIN stock_establecimiento se ON se.establecimiento_id = e.id AND se.producto_id = ?1
             WHERE e.activo = 1
             ORDER BY e.codigo",
        )
        .map_err(|e| e.to_string())?;

    let stocks = stmt
        .query_map(rusqlite::params![producto_id], |row| {
            Ok(StockEstablecimiento {
                establecimiento_id: row.get(0)?,
                establecimiento_nombre: row.get(1)?,
                establecimiento_codigo: row.get(2)?,
                stock_actual: row.get(3)?,
                stock_minimo: row.get(4)?,
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    Ok(stocks)
}

/// Actualiza el stock de un producto en un establecimiento específico.
#[tauri::command]
pub fn actualizar_stock_establecimiento(
    db: State<Database>,
    producto_id: i64,
    establecimiento_id: i64,
    stock_actual: f64,
    stock_minimo: f64,
) -> Result<(), String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;

    conn.execute(
        "INSERT INTO stock_establecimiento (producto_id, establecimiento_id, stock_actual, stock_minimo)
         VALUES (?1, ?2, ?3, ?4)
         ON CONFLICT(producto_id, establecimiento_id) DO UPDATE SET stock_actual = ?3, stock_minimo = ?4",
        rusqlite::params![producto_id, establecimiento_id, stock_actual, stock_minimo],
    ).map_err(|e| e.to_string())?;

    // Actualizar stock total en productos (suma de todos los establecimientos)
    conn.execute(
        "UPDATE productos SET stock_actual = (SELECT COALESCE(SUM(stock_actual), 0) FROM stock_establecimiento WHERE producto_id = ?1)
         WHERE id = ?1",
        rusqlite::params![producto_id],
    ).map_err(|e| e.to_string())?;

    Ok(())
}
