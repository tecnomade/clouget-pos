use crate::db::Database;
use crate::models::{Categoria, Producto, ProductoBusqueda};
use tauri::State;

#[tauri::command]
pub fn crear_producto(db: State<Database>, producto: Producto) -> Result<i64, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;

    conn.execute(
        "INSERT INTO productos (codigo, codigo_barras, nombre, descripcion, categoria_id,
         precio_costo, precio_venta, iva_porcentaje, incluye_iva, stock_actual, stock_minimo,
         unidad_medida, es_servicio, activo)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)",
        rusqlite::params![
            producto.codigo,
            producto.codigo_barras,
            producto.nombre,
            producto.descripcion,
            producto.categoria_id,
            producto.precio_costo,
            producto.precio_venta,
            producto.iva_porcentaje,
            producto.incluye_iva as i32,
            producto.stock_actual,
            producto.stock_minimo,
            producto.unidad_medida,
            producto.es_servicio as i32,
            producto.activo as i32,
        ],
    )
    .map_err(|e| e.to_string())?;

    Ok(conn.last_insert_rowid())
}

#[tauri::command]
pub fn actualizar_producto(db: State<Database>, producto: Producto) -> Result<(), String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let id = producto.id.ok_or("ID requerido para actualizar")?;

    conn.execute(
        "UPDATE productos SET codigo=?1, codigo_barras=?2, nombre=?3, descripcion=?4,
         categoria_id=?5, precio_costo=?6, precio_venta=?7, iva_porcentaje=?8,
         incluye_iva=?9, stock_actual=?10, stock_minimo=?11, unidad_medida=?12,
         es_servicio=?13, activo=?14, updated_at=datetime('now','localtime')
         WHERE id=?15",
        rusqlite::params![
            producto.codigo,
            producto.codigo_barras,
            producto.nombre,
            producto.descripcion,
            producto.categoria_id,
            producto.precio_costo,
            producto.precio_venta,
            producto.iva_porcentaje,
            producto.incluye_iva as i32,
            producto.stock_actual,
            producto.stock_minimo,
            producto.unidad_medida,
            producto.es_servicio as i32,
            producto.activo as i32,
            id,
        ],
    )
    .map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
pub fn buscar_productos(db: State<Database>, termino: String) -> Result<Vec<ProductoBusqueda>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let busqueda = format!("%{}%", termino);

    let mut stmt = conn
        .prepare(
            "SELECT p.id, p.codigo, p.nombre, p.precio_venta, p.iva_porcentaje, p.stock_actual, p.stock_minimo, c.nombre as cat_nombre
             FROM productos p
             LEFT JOIN categorias c ON p.categoria_id = c.id
             WHERE p.activo = 1
             AND (p.nombre LIKE ?1 OR p.codigo LIKE ?1 OR p.codigo_barras LIKE ?1)
             ORDER BY p.nombre
             LIMIT 50",
        )
        .map_err(|e| e.to_string())?;

    let productos = stmt
        .query_map(rusqlite::params![busqueda], |row| {
            Ok(ProductoBusqueda {
                id: row.get(0)?,
                codigo: row.get(1)?,
                nombre: row.get(2)?,
                precio_venta: row.get(3)?,
                iva_porcentaje: row.get(4)?,
                stock_actual: row.get(5)?,
                stock_minimo: row.get(6)?,
                categoria_nombre: row.get(7)?,
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    Ok(productos)
}

#[tauri::command]
pub fn obtener_producto(db: State<Database>, id: i64) -> Result<Producto, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;

    conn.query_row(
        "SELECT id, codigo, codigo_barras, nombre, descripcion, categoria_id,
         precio_costo, precio_venta, iva_porcentaje, incluye_iva, stock_actual,
         stock_minimo, unidad_medida, es_servicio, activo
         FROM productos WHERE id = ?1",
        rusqlite::params![id],
        |row| {
            Ok(Producto {
                id: Some(row.get(0)?),
                codigo: row.get(1)?,
                codigo_barras: row.get(2)?,
                nombre: row.get(3)?,
                descripcion: row.get(4)?,
                categoria_id: row.get(5)?,
                precio_costo: row.get(6)?,
                precio_venta: row.get(7)?,
                iva_porcentaje: row.get(8)?,
                incluye_iva: row.get::<_, i32>(9)? != 0,
                stock_actual: row.get(10)?,
                stock_minimo: row.get(11)?,
                unidad_medida: row.get(12)?,
                es_servicio: row.get::<_, i32>(13)? != 0,
                activo: row.get::<_, i32>(14)? != 0,
            })
        },
    )
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn listar_productos(db: State<Database>, solo_activos: bool) -> Result<Vec<ProductoBusqueda>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;

    let sql = if solo_activos {
        "SELECT p.id, p.codigo, p.nombre, p.precio_venta, p.iva_porcentaje, p.stock_actual, p.stock_minimo, c.nombre
         FROM productos p LEFT JOIN categorias c ON p.categoria_id = c.id
         WHERE p.activo = 1 ORDER BY p.nombre"
    } else {
        "SELECT p.id, p.codigo, p.nombre, p.precio_venta, p.iva_porcentaje, p.stock_actual, p.stock_minimo, c.nombre
         FROM productos p LEFT JOIN categorias c ON p.categoria_id = c.id
         ORDER BY p.nombre"
    };

    let mut stmt = conn.prepare(sql).map_err(|e| e.to_string())?;

    let productos = stmt
        .query_map([], |row| {
            Ok(ProductoBusqueda {
                id: row.get(0)?,
                codigo: row.get(1)?,
                nombre: row.get(2)?,
                precio_venta: row.get(3)?,
                iva_porcentaje: row.get(4)?,
                stock_actual: row.get(5)?,
                stock_minimo: row.get(6)?,
                categoria_nombre: row.get(7)?,
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    Ok(productos)
}

#[tauri::command]
pub fn productos_mas_vendidos(db: State<Database>, limite: i64) -> Result<Vec<ProductoBusqueda>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;

    let mut stmt = conn
        .prepare(
            "SELECT p.id, p.codigo, p.nombre, p.precio_venta, p.iva_porcentaje, p.stock_actual, p.stock_minimo, c.nombre
             FROM productos p
             LEFT JOIN categorias c ON p.categoria_id = c.id
             INNER JOIN venta_detalles vd ON vd.producto_id = p.id
             WHERE p.activo = 1
             GROUP BY p.id
             ORDER BY SUM(vd.cantidad) DESC
             LIMIT ?1",
        )
        .map_err(|e| e.to_string())?;

    let productos = stmt
        .query_map(rusqlite::params![limite], |row| {
            Ok(ProductoBusqueda {
                id: row.get(0)?,
                codigo: row.get(1)?,
                nombre: row.get(2)?,
                precio_venta: row.get(3)?,
                iva_porcentaje: row.get(4)?,
                stock_actual: row.get(5)?,
                stock_minimo: row.get(6)?,
                categoria_nombre: row.get(7)?,
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    Ok(productos)
}

// --- Categorías ---

#[tauri::command]
pub fn crear_categoria(db: State<Database>, categoria: Categoria) -> Result<i64, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;

    conn.execute(
        "INSERT INTO categorias (nombre, descripcion, activo) VALUES (?1, ?2, ?3)",
        rusqlite::params![categoria.nombre, categoria.descripcion, categoria.activo as i32],
    )
    .map_err(|e| e.to_string())?;

    Ok(conn.last_insert_rowid())
}

#[tauri::command]
pub fn listar_categorias(db: State<Database>) -> Result<Vec<Categoria>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;

    let mut stmt = conn
        .prepare("SELECT id, nombre, descripcion, activo FROM categorias WHERE activo = 1 ORDER BY nombre")
        .map_err(|e| e.to_string())?;

    let categorias = stmt
        .query_map([], |row| {
            Ok(Categoria {
                id: Some(row.get(0)?),
                nombre: row.get(1)?,
                descripcion: row.get(2)?,
                activo: row.get::<_, i32>(3)? != 0,
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    Ok(categorias)
}
