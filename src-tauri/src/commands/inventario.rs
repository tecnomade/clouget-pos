use crate::db::Database;
use serde::{Deserialize, Serialize};
use tauri::State;

#[derive(Debug, Serialize, Deserialize)]
pub struct MovimientoInventario {
    pub id: Option<i64>,
    pub producto_id: i64,
    pub producto_nombre: Option<String>,
    pub producto_codigo: Option<String>,
    pub tipo: String,
    pub cantidad: f64,
    pub stock_anterior: f64,
    pub stock_nuevo: f64,
    pub costo_unitario: Option<f64>,
    pub referencia_id: Option<i64>,
    pub motivo: Option<String>,
    pub usuario: Option<String>,
    pub created_at: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ResumenInventario {
    pub total_productos: i64,
    pub total_entradas_mes: f64,
    pub total_salidas_mes: f64,
    pub total_ajustes_mes: i64,
    pub valor_inventario: f64,
}

/// Registra un movimiento de inventario (ENTRADA, SALIDA, AJUSTE)
/// Actualiza stock_actual automáticamente
#[tauri::command]
pub fn registrar_movimiento(
    db: State<Database>,
    producto_id: i64,
    tipo: String,
    cantidad: f64,
    motivo: Option<String>,
    costo_unitario: Option<f64>,
    usuario: Option<String>,
) -> Result<MovimientoInventario, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;

    // Obtener stock actual
    let stock_actual: f64 = conn
        .query_row(
            "SELECT stock_actual FROM productos WHERE id = ?1",
            rusqlite::params![producto_id],
            |row| row.get(0),
        )
        .map_err(|_| "Producto no encontrado".to_string())?;

    // Calcular nuevo stock según tipo
    let stock_nuevo = match tipo.as_str() {
        "ENTRADA" => stock_actual + cantidad.abs(),
        "SALIDA" | "VENTA" => stock_actual - cantidad.abs(),
        "AJUSTE" => cantidad, // cantidad ES el nuevo stock
        "DEVOLUCION" => stock_actual + cantidad.abs(),
        _ => return Err(format!("Tipo de movimiento no valido: {}", tipo)),
    };

    // Para AJUSTE, la cantidad real del movimiento es la diferencia
    let cantidad_movimiento = match tipo.as_str() {
        "AJUSTE" => cantidad - stock_actual,
        _ => if tipo == "SALIDA" || tipo == "VENTA" { -cantidad.abs() } else { cantidad.abs() },
    };

    // Registrar movimiento
    conn.execute(
        "INSERT INTO movimientos_inventario (producto_id, tipo, cantidad, stock_anterior, stock_nuevo, costo_unitario, motivo, usuario)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        rusqlite::params![
            producto_id,
            tipo,
            cantidad_movimiento,
            stock_actual,
            stock_nuevo,
            costo_unitario,
            motivo,
            usuario,
        ],
    )
    .map_err(|e| e.to_string())?;

    let mov_id = conn.last_insert_rowid();

    // Actualizar stock del producto
    conn.execute(
        "UPDATE productos SET stock_actual = ?1, updated_at = datetime('now','localtime') WHERE id = ?2",
        rusqlite::params![stock_nuevo, producto_id],
    )
    .map_err(|e| e.to_string())?;

    Ok(MovimientoInventario {
        id: Some(mov_id),
        producto_id,
        producto_nombre: None,
        producto_codigo: None,
        tipo,
        cantidad: cantidad_movimiento,
        stock_anterior: stock_actual,
        stock_nuevo,
        costo_unitario,
        referencia_id: None,
        motivo,
        usuario,
        created_at: None,
    })
}

/// Lista movimientos de inventario de un producto con filtros opcionales
#[tauri::command]
pub fn listar_movimientos(
    db: State<Database>,
    producto_id: Option<i64>,
    fecha_inicio: Option<String>,
    fecha_fin: Option<String>,
    tipo: Option<String>,
    limite: Option<i64>,
) -> Result<Vec<MovimientoInventario>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;

    let mut sql = String::from(
        "SELECT m.id, m.producto_id, p.nombre, p.codigo, m.tipo, m.cantidad,
                m.stock_anterior, m.stock_nuevo, m.costo_unitario, m.referencia_id,
                m.motivo, m.usuario, m.created_at
         FROM movimientos_inventario m
         JOIN productos p ON m.producto_id = p.id
         WHERE 1=1"
    );
    let mut params: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();

    if let Some(pid) = producto_id {
        sql.push_str(&format!(" AND m.producto_id = ?{}", params.len() + 1));
        params.push(Box::new(pid));
    }

    if let Some(ref fi) = fecha_inicio {
        sql.push_str(&format!(" AND date(m.created_at) >= date(?{})", params.len() + 1));
        params.push(Box::new(fi.clone()));
    }

    if let Some(ref ff) = fecha_fin {
        sql.push_str(&format!(" AND date(m.created_at) <= date(?{})", params.len() + 1));
        params.push(Box::new(ff.clone()));
    }

    if let Some(ref t) = tipo {
        sql.push_str(&format!(" AND m.tipo = ?{}", params.len() + 1));
        params.push(Box::new(t.clone()));
    }

    sql.push_str(" ORDER BY m.created_at DESC");

    let lim = limite.unwrap_or(200);
    sql.push_str(&format!(" LIMIT ?{}", params.len() + 1));
    params.push(Box::new(lim));

    let param_refs: Vec<&dyn rusqlite::types::ToSql> = params.iter().map(|p| p.as_ref()).collect();

    let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;
    let movimientos = stmt
        .query_map(param_refs.as_slice(), |row| {
            Ok(MovimientoInventario {
                id: row.get(0)?,
                producto_id: row.get(1)?,
                producto_nombre: row.get(2)?,
                producto_codigo: row.get(3)?,
                tipo: row.get(4)?,
                cantidad: row.get(5)?,
                stock_anterior: row.get(6)?,
                stock_nuevo: row.get(7)?,
                costo_unitario: row.get(8)?,
                referencia_id: row.get(9)?,
                motivo: row.get(10)?,
                usuario: row.get(11)?,
                created_at: row.get(12)?,
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    Ok(movimientos)
}

/// Resumen general de inventario para el dashboard
#[tauri::command]
pub fn resumen_inventario(db: State<Database>) -> Result<ResumenInventario, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;

    let total_productos: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM productos WHERE activo = 1 AND es_servicio = 0",
            [],
            |row| row.get(0),
        )
        .unwrap_or(0);

    let total_entradas_mes: f64 = conn
        .query_row(
            "SELECT COALESCE(SUM(cantidad), 0) FROM movimientos_inventario
             WHERE tipo = 'ENTRADA' AND date(created_at) >= date('now', 'start of month', 'localtime')",
            [],
            |row| row.get(0),
        )
        .unwrap_or(0.0);

    let total_salidas_mes: f64 = conn
        .query_row(
            "SELECT COALESCE(SUM(ABS(cantidad)), 0) FROM movimientos_inventario
             WHERE tipo IN ('SALIDA', 'VENTA') AND date(created_at) >= date('now', 'start of month', 'localtime')",
            [],
            |row| row.get(0),
        )
        .unwrap_or(0.0);

    let total_ajustes_mes: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM movimientos_inventario
             WHERE tipo = 'AJUSTE' AND date(created_at) >= date('now', 'start of month', 'localtime')",
            [],
            |row| row.get(0),
        )
        .unwrap_or(0);

    let valor_inventario: f64 = conn
        .query_row(
            "SELECT COALESCE(SUM(stock_actual * precio_costo), 0) FROM productos WHERE activo = 1 AND es_servicio = 0",
            [],
            |row| row.get(0),
        )
        .unwrap_or(0.0);

    Ok(ResumenInventario {
        total_productos,
        total_entradas_mes,
        total_salidas_mes,
        total_ajustes_mes,
        valor_inventario,
    })
}
