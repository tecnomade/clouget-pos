use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use crate::db::Database;
use crate::models::{Categoria, Producto, ProductoBusqueda, ProductoTactil};
use tauri::State;

#[tauri::command]
pub fn crear_producto(db: State<Database>, producto: Producto) -> Result<i64, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;

    // Auto-generar código si viene vacío
    let codigo = match &producto.codigo {
        Some(c) if !c.trim().is_empty() => Some(c.trim().to_string()),
        _ => {
            let next: i64 = conn
                .query_row(
                    "SELECT COALESCE(MAX(CAST(REPLACE(codigo, 'P', '') AS INTEGER)), 0) + 1
                     FROM productos WHERE codigo LIKE 'P%' AND LENGTH(codigo) <= 8",
                    [],
                    |row| row.get(0),
                )
                .unwrap_or(1);
            Some(format!("P{:04}", next))
        }
    };

    // Normalizar codigo_barras: vacío -> NULL para evitar UNIQUE collision
    let codigo_barras = match &producto.codigo_barras {
        Some(c) if !c.trim().is_empty() => Some(c.trim().to_string()),
        _ => None,
    };

    // Verificar duplicado de codigo_barras
    // Si el conflicto es solo con productos INACTIVOS, libera el código automáticamente
    if let Some(cb) = &codigo_barras {
        let mut stmt = conn.prepare("SELECT id, nombre, activo FROM productos WHERE codigo_barras = ?1").map_err(|e| e.to_string())?;
        let existentes: Vec<(i64, String, i32)> = stmt.query_map(rusqlite::params![cb], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))
            .map_err(|e| e.to_string())?
            .collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())?;
        drop(stmt);

        let activos_conflicto: Vec<&(i64, String, i32)> = existentes.iter().filter(|(_, _, a)| *a == 1).collect();
        let inactivos_conflicto: Vec<&(i64, String, i32)> = existentes.iter().filter(|(_, _, a)| *a == 0).collect();

        if !activos_conflicto.is_empty() {
            let nombres: Vec<String> = activos_conflicto.iter().map(|(_, n, _)| n.clone()).collect();
            return Err(format!("DUPLICATE_BARCODE:{}:{}", cb, nombres.join(", ")));
        }

        // Si solo hay conflicto con inactivos, liberar el código de esos
        for (inactivo_id, _, _) in &inactivos_conflicto {
            conn.execute("UPDATE productos SET codigo_barras = NULL WHERE id = ?1", rusqlite::params![inactivo_id]).ok();
        }
    }

    conn.execute(
        "INSERT INTO productos (codigo, codigo_barras, nombre, descripcion, categoria_id,
         precio_costo, precio_venta, iva_porcentaje, incluye_iva, stock_actual, stock_minimo,
         unidad_medida, es_servicio, activo, imagen, requiere_serie, requiere_caducidad, no_controla_stock)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18)",
        rusqlite::params![
            codigo,
            codigo_barras,
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
            producto.imagen,
            producto.requiere_serie as i32,
            producto.requiere_caducidad as i32,
            producto.no_controla_stock as i32,
        ],
    )
    .map_err(|e| e.to_string())?;

    Ok(conn.last_insert_rowid())
}

#[tauri::command]
pub fn actualizar_producto(db: State<Database>, producto: Producto) -> Result<(), String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let id = producto.id.ok_or("ID requerido para actualizar")?;

    // Normalizar codigo_barras: vacío -> NULL para evitar UNIQUE collision
    let codigo_barras = match &producto.codigo_barras {
        Some(c) if !c.trim().is_empty() => Some(c.trim().to_string()),
        _ => None,
    };

    // Verificar duplicado de codigo_barras (excluyendo el mismo producto)
    // Si el conflicto es solo con productos INACTIVOS, libera el código automáticamente
    if let Some(cb) = &codigo_barras {
        let mut stmt = conn.prepare("SELECT id, nombre, activo FROM productos WHERE codigo_barras = ?1 AND id != ?2").map_err(|e| e.to_string())?;
        let existentes: Vec<(i64, String, i32)> = stmt.query_map(rusqlite::params![cb, id], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))
            .map_err(|e| e.to_string())?
            .collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())?;
        drop(stmt);

        let activos_conflicto: Vec<&(i64, String, i32)> = existentes.iter().filter(|(_, _, a)| *a == 1).collect();
        let inactivos_conflicto: Vec<&(i64, String, i32)> = existentes.iter().filter(|(_, _, a)| *a == 0).collect();

        if !activos_conflicto.is_empty() {
            // Bloquear solo si hay conflicto con producto activo
            let nombres: Vec<String> = activos_conflicto.iter().map(|(_, n, _)| n.clone()).collect();
            return Err(format!("DUPLICATE_BARCODE:{}:{}", cb, nombres.join(", ")));
        }

        // Si solo hay conflicto con inactivos, liberar el código de esos
        for (inactivo_id, _, _) in &inactivos_conflicto {
            conn.execute("UPDATE productos SET codigo_barras = NULL WHERE id = ?1", rusqlite::params![inactivo_id]).ok();
        }
    }

    conn.execute(
        "UPDATE productos SET codigo=?1, codigo_barras=?2, nombre=?3, descripcion=?4,
         categoria_id=?5, precio_costo=?6, precio_venta=?7, iva_porcentaje=?8,
         incluye_iva=?9, stock_actual=?10, stock_minimo=?11, unidad_medida=?12,
         es_servicio=?13, activo=?14, imagen=?15, requiere_serie=?16, requiere_caducidad=?17,
         no_controla_stock=?18, updated_at=datetime('now','localtime')
         WHERE id=?19",
        rusqlite::params![
            producto.codigo,
            codigo_barras,
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
            producto.imagen,
            producto.requiere_serie as i32,
            producto.requiere_caducidad as i32,
            producto.no_controla_stock as i32,
            id,
        ],
    )
    .map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
pub fn buscar_productos(
    db: State<Database>,
    termino: String,
    lista_precio_id: Option<i64>,
) -> Result<Vec<ProductoBusqueda>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let busqueda = format!("%{}%", termino);

    // Verificar si multi-almacén está activo
    let multi_almacen: bool = conn
        .query_row("SELECT value FROM config WHERE key = 'multi_almacen_activo'", [], |row| row.get::<_, String>(0))
        .map(|v| v == "1")
        .unwrap_or(false);

    if multi_almacen {
        return buscar_productos_multi_almacen(&conn, &busqueda, lista_precio_id);
    }

    let mut stmt = conn
        .prepare(
            "SELECT p.id, p.codigo, p.codigo_barras, p.nombre, p.precio_venta, p.precio_costo, p.iva_porcentaje,
                    p.incluye_iva, p.stock_actual, p.stock_minimo, c.nombre as cat_nombre,
                    pp.precio as precio_lista
             FROM productos p
             LEFT JOIN categorias c ON p.categoria_id = c.id
             LEFT JOIN precios_producto pp ON pp.producto_id = p.id AND pp.lista_precio_id = ?2
             WHERE p.activo = 1
             AND (p.nombre LIKE ?1 OR p.codigo LIKE ?1 OR p.codigo_barras LIKE ?1)
             ORDER BY p.nombre
             LIMIT 50",
        )
        .map_err(|e| e.to_string())?;

    let productos = stmt
        .query_map(rusqlite::params![busqueda, lista_precio_id], |row| {
            Ok(ProductoBusqueda {
                id: row.get(0)?,
                codigo: row.get(1)?,
                codigo_barras: row.get(2)?,
                nombre: row.get(3)?,
                precio_venta: row.get(4)?,
                precio_costo: row.get(5)?,
                iva_porcentaje: row.get(6)?,
                incluye_iva: row.get::<_, i32>(7)? != 0,
                stock_actual: row.get(8)?,
                stock_minimo: row.get(9)?,
                categoria_nombre: row.get(10)?,
                precio_lista: row.get(11)?,
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    Ok(productos)
}

/// Búsqueda de productos con stock por establecimiento (multi-almacén)
fn buscar_productos_multi_almacen(
    conn: &std::sync::MutexGuard<rusqlite::Connection>,
    busqueda: &str,
    lista_precio_id: Option<i64>,
) -> Result<Vec<ProductoBusqueda>, String> {
    let est_codigo: String = conn
        .query_row("SELECT value FROM config WHERE key = 'terminal_establecimiento'", [], |row| row.get(0))
        .unwrap_or_else(|_| "001".to_string());
    let est_id: Option<i64> = conn
        .query_row("SELECT id FROM establecimientos WHERE codigo = ?1", rusqlite::params![est_codigo], |row| row.get(0))
        .ok();

    let mut stmt = conn
        .prepare(
            "SELECT p.id, p.codigo, p.nombre, p.precio_venta, p.iva_porcentaje, p.incluye_iva,
                    COALESCE(se.stock_actual, 0) as stock_local,
                    COALESCE(se.stock_minimo, p.stock_minimo) as stock_min,
                    c.nombre as cat_nombre,
                    pp.precio as precio_lista
             FROM productos p
             LEFT JOIN categorias c ON p.categoria_id = c.id
             LEFT JOIN precios_producto pp ON pp.producto_id = p.id AND pp.lista_precio_id = ?2
             LEFT JOIN stock_establecimiento se ON se.producto_id = p.id AND se.establecimiento_id = ?3
             WHERE p.activo = 1
             AND (p.nombre LIKE ?1 OR p.codigo LIKE ?1 OR p.codigo_barras LIKE ?1)
             ORDER BY p.nombre
             LIMIT 50",
        )
        .map_err(|e| e.to_string())?;

    let productos = stmt
        .query_map(rusqlite::params![busqueda, lista_precio_id, est_id], |row| {
            Ok(ProductoBusqueda {
                id: row.get(0)?,
                codigo: row.get(1)?,
                codigo_barras: None,
                nombre: row.get(2)?,
                precio_venta: row.get(3)?, precio_costo: 0.0,
                iva_porcentaje: row.get(4)?,
                incluye_iva: row.get::<_, i32>(5)? != 0,
                stock_actual: row.get(6)?,
                stock_minimo: row.get(7)?,
                categoria_nombre: row.get(8)?,
                precio_lista: row.get(9)?,
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
         stock_minimo, unidad_medida, es_servicio, activo, imagen, requiere_serie, requiere_caducidad,
         no_controla_stock
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
                imagen: row.get(15)?,
                requiere_serie: row.get::<_, i32>(16)? != 0,
                requiere_caducidad: row.get::<_, i32>(17)? != 0,
                no_controla_stock: row.get::<_, i32>(18)? != 0,
            })
        },
    )
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn listar_productos(
    db: State<Database>,
    solo_activos: bool,
    lista_precio_id: Option<i64>,
) -> Result<Vec<ProductoBusqueda>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;

    let sql = if solo_activos {
        "SELECT p.id, p.codigo, p.nombre, p.precio_venta, p.iva_porcentaje, p.incluye_iva,
                p.stock_actual, p.stock_minimo, c.nombre, pp.precio
         FROM productos p
         LEFT JOIN categorias c ON p.categoria_id = c.id
         LEFT JOIN precios_producto pp ON pp.producto_id = p.id AND pp.lista_precio_id = ?1
         WHERE p.activo = 1 ORDER BY p.nombre"
    } else {
        "SELECT p.id, p.codigo, p.nombre, p.precio_venta, p.iva_porcentaje, p.incluye_iva,
                p.stock_actual, p.stock_minimo, c.nombre, pp.precio
         FROM productos p
         LEFT JOIN categorias c ON p.categoria_id = c.id
         LEFT JOIN precios_producto pp ON pp.producto_id = p.id AND pp.lista_precio_id = ?1
         ORDER BY p.nombre"
    };

    let mut stmt = conn.prepare(sql).map_err(|e| e.to_string())?;

    let productos = stmt
        .query_map(rusqlite::params![lista_precio_id], |row| {
            Ok(ProductoBusqueda {
                id: row.get(0)?,
                codigo: row.get(1)?,
                codigo_barras: None,
                nombre: row.get(2)?,
                precio_venta: row.get(3)?, precio_costo: 0.0,
                iva_porcentaje: row.get(4)?,
                incluye_iva: row.get::<_, i32>(5)? != 0,
                stock_actual: row.get(6)?,
                stock_minimo: row.get(7)?,
                categoria_nombre: row.get(8)?,
                precio_lista: row.get(9)?,
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
            "SELECT p.id, p.codigo, p.nombre, p.precio_venta, p.iva_porcentaje, p.incluye_iva,
                    p.stock_actual, p.stock_minimo, c.nombre
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
                codigo_barras: None,
                nombre: row.get(2)?,
                precio_venta: row.get(3)?, precio_costo: 0.0,
                iva_porcentaje: row.get(4)?,
                incluye_iva: row.get::<_, i32>(5)? != 0,
                stock_actual: row.get(6)?,
                stock_minimo: row.get(7)?,
                categoria_nombre: row.get(8)?,
                precio_lista: None,
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    Ok(productos)
}

// --- Imagen de producto ---

#[tauri::command]
pub fn cargar_imagen_producto(db: State<Database>, id: i64, imagen_path: String) -> Result<String, String> {
    let bytes = std::fs::read(&imagen_path)
        .map_err(|e| format!("Error leyendo imagen: {}", e))?;

    if bytes.len() > 500_000 {
        return Err("La imagen es demasiado grande. Maximo 500KB.".to_string());
    }

    let b64 = BASE64.encode(&bytes);

    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    conn.execute(
        "UPDATE productos SET imagen = ?1, updated_at = datetime('now','localtime') WHERE id = ?2",
        rusqlite::params![b64, id],
    )
    .map_err(|e| e.to_string())?;

    Ok(b64)
}

#[tauri::command]
pub fn eliminar_imagen_producto(db: State<Database>, id: i64) -> Result<(), String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    conn.execute(
        "UPDATE productos SET imagen = NULL, updated_at = datetime('now','localtime') WHERE id = ?1",
        rusqlite::params![id],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

// --- Productos para modo tactil ---

#[tauri::command]
pub fn listar_productos_tactil(db: State<Database>) -> Result<Vec<ProductoTactil>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;

    let mut stmt = conn
        .prepare(
            "SELECT p.id, p.nombre, p.precio_venta, p.iva_porcentaje, p.incluye_iva, p.stock_actual,
                    p.categoria_id, c.nombre, p.imagen,
                    COALESCE(p.es_servicio, 0), COALESCE(p.no_controla_stock, 0)
             FROM productos p
             LEFT JOIN categorias c ON p.categoria_id = c.id
             WHERE p.activo = 1
             ORDER BY p.nombre",
        )
        .map_err(|e| e.to_string())?;

    let productos = stmt
        .query_map([], |row| {
            Ok(ProductoTactil {
                id: row.get(0)?,
                nombre: row.get(1)?,
                precio_venta: row.get(2)?,
                iva_porcentaje: row.get(3)?,
                incluye_iva: row.get::<_, i32>(4)? != 0,
                stock_actual: row.get(5)?,
                categoria_id: row.get(6)?,
                categoria_nombre: row.get(7)?,
                imagen: row.get(8)?,
                es_servicio: row.get::<_, i32>(9)? != 0,
                no_controla_stock: row.get::<_, i32>(10)? != 0,
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

#[tauri::command]
pub fn actualizar_categoria(db: State<Database>, id: i64, nombre: String) -> Result<(), String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    conn.execute("UPDATE categorias SET nombre = ?1 WHERE id = ?2", rusqlite::params![nombre.trim(), id])
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn eliminar_categoria(db: State<Database>, id: i64, accion: Option<String>, mover_a: Option<i64>) -> Result<serde_json::Value, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let count: i64 = conn.query_row("SELECT COUNT(*) FROM productos WHERE categoria_id = ?1", rusqlite::params![id], |r| r.get(0)).unwrap_or(0);

    if count > 0 {
        match accion.as_deref() {
            Some("mover") => {
                // Mover productos a otra categoría (o General si mover_a es None)
                let destino = mover_a.unwrap_or_else(|| {
                    conn.query_row("SELECT id FROM categorias WHERE LOWER(nombre) = 'general'", [], |r| r.get(0))
                        .unwrap_or_else(|_| {
                            conn.execute("INSERT INTO categorias (nombre) VALUES ('General')", []).ok();
                            conn.last_insert_rowid()
                        })
                });
                conn.execute("UPDATE productos SET categoria_id = ?1 WHERE categoria_id = ?2", rusqlite::params![destino, id])
                    .map_err(|e| e.to_string())?;
            }
            Some("eliminar_productos") => {
                // Eliminar todos los productos de esta categoría
                conn.execute("DELETE FROM productos WHERE categoria_id = ?1", rusqlite::params![id])
                    .map_err(|e| e.to_string())?;
            }
            _ => {
                // Sin acción: retornar conteo para que el frontend pregunte
                return Ok(serde_json::json!({ "requiere_accion": true, "productos": count }));
            }
        }
    }

    conn.execute("DELETE FROM categorias WHERE id = ?1", rusqlite::params![id]).map_err(|e| e.to_string())?;
    Ok(serde_json::json!({ "eliminada": true, "productos_afectados": count }))
}

#[tauri::command]
pub fn listar_tipos_unidad(db: State<Database>) -> Result<Vec<serde_json::Value>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let mut stmt = conn.prepare(
        "SELECT id, nombre, abreviatura,
                COALESCE(factor_default, 1) as factor_default,
                COALESCE(es_agrupada, 0) as es_agrupada
         FROM tipos_unidad ORDER BY es_agrupada, nombre"
    ).map_err(|e| e.to_string())?;
    let result: Vec<serde_json::Value> = stmt.query_map([], |row| {
        Ok(serde_json::json!({
            "id": row.get::<_, i64>(0)?,
            "nombre": row.get::<_, String>(1)?,
            "abreviatura": row.get::<_, String>(2)?,
            "factor_default": row.get::<_, f64>(3)?,
            "es_agrupada": row.get::<_, i32>(4)? != 0,
        }))
    }).map_err(|e| e.to_string())?
      .collect::<Result<Vec<_>, _>>()
      .map_err(|e| e.to_string())?;
    Ok(result)
}

#[tauri::command]
pub fn crear_tipo_unidad(
    db: State<Database>,
    nombre: String,
    abreviatura: String,
    factor_default: Option<f64>,
    es_agrupada: Option<bool>,
) -> Result<i64, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let factor = factor_default.unwrap_or(1.0);
    let agrupada = es_agrupada.unwrap_or(false);
    conn.execute(
        "INSERT INTO tipos_unidad (nombre, abreviatura, factor_default, es_agrupada) VALUES (?1, ?2, ?3, ?4)",
        rusqlite::params![nombre.trim(), abreviatura.trim().to_uppercase(), factor, agrupada as i32]
    ).map_err(|e| e.to_string())?;
    Ok(conn.last_insert_rowid())
}

#[tauri::command]
pub fn actualizar_tipo_unidad(
    db: State<Database>,
    id: i64,
    nombre: String,
    abreviatura: String,
    factor_default: Option<f64>,
    es_agrupada: Option<bool>,
) -> Result<(), String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let factor = factor_default.unwrap_or(1.0);
    let agrupada = es_agrupada.unwrap_or(false);
    conn.execute(
        "UPDATE tipos_unidad SET nombre = ?1, abreviatura = ?2, factor_default = ?3, es_agrupada = ?4 WHERE id = ?5",
        rusqlite::params![nombre.trim(), abreviatura.trim().to_uppercase(), factor, agrupada as i32, id]
    ).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn eliminar_tipo_unidad(db: State<Database>, id: i64) -> Result<(), String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    conn.execute("DELETE FROM tipos_unidad WHERE id = ?1", rusqlite::params![id]).map_err(|e| e.to_string())?;
    Ok(())
}

// --- Eliminar producto ---

#[tauri::command]
pub fn eliminar_producto(db: State<Database>, id: i64) -> Result<(), String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    // Check if product has been used in sales
    let ventas: i64 = conn.query_row(
        "SELECT COUNT(*) FROM venta_detalles WHERE producto_id = ?1",
        rusqlite::params![id], |r| r.get(0)
    ).unwrap_or(0);
    if ventas > 0 {
        // Soft delete - mark as inactive y liberar códigos para que puedan reutilizarse
        conn.execute(
            "UPDATE productos SET activo = 0, codigo = codigo || '_DEL' || id, codigo_barras = NULL WHERE id = ?1",
            rusqlite::params![id]
        ).map_err(|e| e.to_string())?;
    } else {
        // Hard delete - no sales reference
        conn.execute("DELETE FROM productos WHERE id = ?1", rusqlite::params![id])
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}

// --- Excel Import/Export ---

#[tauri::command]
pub fn exportar_plantilla_productos() -> Result<Vec<u8>, String> {
    use rust_xlsxwriter::*;

    let mut workbook = Workbook::new();
    let sheet = workbook.add_worksheet();
    sheet.set_name("Productos").map_err(|e| e.to_string())?;

    // Header format
    let header_fmt = Format::new().set_bold().set_background_color(Color::RGB(0x2563EB)).set_font_color(Color::White).set_border(FormatBorder::Thin);

    let headers = ["codigo", "codigo_barras", "nombre", "descripcion", "categoria", "precio_costo", "precio_venta", "iva_porcentaje", "stock_actual", "stock_minimo", "unidad_medida", "es_servicio", "requiere_serie", "requiere_caducidad", "lote", "fecha_caducidad"];
    for (i, h) in headers.iter().enumerate() {
        sheet.write_string_with_format(0, i as u16, *h, &header_fmt).map_err(|e| e.to_string())?;
    }

    // Example row 1: producto normal (sin caducidad)
    let example1 = ["P0001", "", "Producto ejemplo", "Descripcion", "General", "1.00", "2.50", "15", "100", "5", "UND", "0", "0", "0", "", ""];
    for (i, v) in example1.iter().enumerate() {
        sheet.write_string(1, i as u16, *v).map_err(|e| e.to_string())?;
    }

    // Example row 2: producto CON caducidad
    let example2 = ["P0002", "", "Yogurt", "Yogurt sabor fresa", "Lacteos", "0.50", "1.00", "15", "20", "5", "UND", "0", "0", "1", "LOT-001", "2026-12-31"];
    for (i, v) in example2.iter().enumerate() {
        sheet.write_string(2, i as u16, *v).map_err(|e| e.to_string())?;
    }

    // Auto-fit columns
    for i in 0..16u16 {
        sheet.set_column_width(i, 15).map_err(|e| e.to_string())?;
    }
    sheet.set_column_width(2, 30).map_err(|e| e.to_string())?; // nombre wider
    sheet.set_column_width(3, 25).map_err(|e| e.to_string())?; // descripcion wider
    sheet.set_column_width(14, 15).map_err(|e| e.to_string())?; // lote
    sheet.set_column_width(15, 18).map_err(|e| e.to_string())?; // fecha_caducidad

    let buf = workbook.save_to_buffer().map_err(|e| e.to_string())?;
    Ok(buf)
}

#[tauri::command]
pub fn exportar_productos_excel(db: State<Database>) -> Result<Vec<u8>, String> {
    use rust_xlsxwriter::*;

    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let mut stmt = conn.prepare(
        "SELECT p.codigo, p.codigo_barras, p.nombre, p.descripcion,
                COALESCE(c.nombre, ''), p.precio_costo, p.precio_venta,
                p.iva_porcentaje, p.stock_actual, p.stock_minimo,
                p.unidad_medida, p.es_servicio,
                p.requiere_serie, p.requiere_caducidad,
                (SELECT lote FROM lotes_caducidad WHERE producto_id = p.id AND cantidad > 0 ORDER BY fecha_caducidad ASC LIMIT 1) as lote,
                (SELECT fecha_caducidad FROM lotes_caducidad WHERE producto_id = p.id AND cantidad > 0 ORDER BY fecha_caducidad ASC LIMIT 1) as fecha_cad
         FROM productos p LEFT JOIN categorias c ON p.categoria_id = c.id
         WHERE p.activo = 1 ORDER BY p.nombre"
    ).map_err(|e| e.to_string())?;

    let mut workbook = Workbook::new();
    let sheet = workbook.add_worksheet();
    sheet.set_name("Productos").map_err(|e| e.to_string())?;

    let header_fmt = Format::new().set_bold().set_background_color(Color::RGB(0x2563EB)).set_font_color(Color::White).set_border(FormatBorder::Thin);
    let money_fmt = Format::new().set_num_format("$#,##0.00");

    let headers = ["codigo", "codigo_barras", "nombre", "descripcion", "categoria", "precio_costo", "precio_venta", "iva_porcentaje", "stock_actual", "stock_minimo", "unidad_medida", "es_servicio", "requiere_serie", "requiere_caducidad", "lote", "fecha_caducidad"];
    for (i, h) in headers.iter().enumerate() {
        sheet.write_string_with_format(0, i as u16, *h, &header_fmt).map_err(|e| e.to_string())?;
    }

    let mut row = 1u32;
    let rows = stmt.query_map([], |r| {
        Ok((
            r.get::<_, Option<String>>(0)?.unwrap_or_default(),
            r.get::<_, Option<String>>(1)?.unwrap_or_default(),
            r.get::<_, String>(2)?,
            r.get::<_, Option<String>>(3)?.unwrap_or_default(),
            r.get::<_, String>(4)?,
            r.get::<_, f64>(5)?,
            r.get::<_, f64>(6)?,
            r.get::<_, f64>(7)?,
            r.get::<_, f64>(8)?,
            r.get::<_, f64>(9)?,
            r.get::<_, Option<String>>(10)?.unwrap_or_default(),
            r.get::<_, i32>(11)?,
            r.get::<_, i32>(12)?,
            r.get::<_, i32>(13)?,
            r.get::<_, Option<String>>(14)?.unwrap_or_default(),
            r.get::<_, Option<String>>(15)?.unwrap_or_default(),
        ))
    }).map_err(|e| e.to_string())?;

    for r in rows {
        let (codigo, barras, nombre, desc, cat, costo, venta, iva, stock, stock_min, unidad, servicio, req_serie, req_caducidad, lote, fecha_cad) = r.map_err(|e| e.to_string())?;
        sheet.write_string(row, 0, &codigo).ok();
        sheet.write_string(row, 1, &barras).ok();
        sheet.write_string(row, 2, &nombre).ok();
        sheet.write_string(row, 3, &desc).ok();
        sheet.write_string(row, 4, &cat).ok();
        sheet.write_number_with_format(row, 5, costo, &money_fmt).ok();
        sheet.write_number_with_format(row, 6, venta, &money_fmt).ok();
        sheet.write_number(row, 7, iva).ok();
        sheet.write_number(row, 8, stock).ok();
        sheet.write_number(row, 9, stock_min).ok();
        sheet.write_string(row, 10, &unidad).ok();
        sheet.write_number(row, 11, servicio as f64).ok();
        sheet.write_number(row, 12, req_serie as f64).ok();
        sheet.write_number(row, 13, req_caducidad as f64).ok();
        sheet.write_string(row, 14, &lote).ok();
        sheet.write_string(row, 15, &fecha_cad).ok();
        row += 1;
    }

    // Column widths
    for i in 0..16u16 { sheet.set_column_width(i, 15).ok(); }
    sheet.set_column_width(2, 35).ok();
    sheet.set_column_width(3, 25).ok();
    sheet.set_column_width(14, 15).ok();
    sheet.set_column_width(15, 18).ok();

    let buf = workbook.save_to_buffer().map_err(|e| e.to_string())?;
    Ok(buf)
}

#[tauri::command]
pub fn importar_productos_excel(db: State<Database>, archivo_bytes: Vec<u8>) -> Result<serde_json::Value, String> {
    use calamine::{Reader, Xlsx, open_workbook_from_rs};
    use std::io::Cursor;

    let conn = db.conn.lock().map_err(|e| e.to_string())?;

    let cursor = Cursor::new(&archivo_bytes);
    let mut workbook: Xlsx<_> = open_workbook_from_rs(cursor).map_err(|e| format!("Error abriendo Excel: {}", e))?;

    // Get first sheet
    let sheet_names = workbook.sheet_names().to_vec();
    let sheet_name = sheet_names.first().ok_or("No se encontraron hojas en el archivo")?;
    let range = workbook.worksheet_range(sheet_name).map_err(|e| format!("Error leyendo hoja: {}", e))?;

    let mut rows_iter = range.rows();
    let header_row = rows_iter.next().ok_or("Archivo vacío")?;

    // Parse headers
    let headers: Vec<String> = header_row.iter().map(|c| c.to_string().trim().to_lowercase()).collect();
    let find_col = |name: &str| -> Option<usize> { headers.iter().position(|h| h == name) };

    let col_nombre = find_col("nombre").ok_or("Columna 'nombre' es requerida")?;
    let col_codigo = find_col("codigo");
    let col_codigo_barras = find_col("codigo_barras");
    let col_descripcion = find_col("descripcion");
    let col_categoria = find_col("categoria");
    let col_precio_costo = find_col("precio_costo");
    let col_precio_venta = find_col("precio_venta");
    let col_iva = find_col("iva_porcentaje");
    let col_stock = find_col("stock_actual");
    let col_stock_min = find_col("stock_minimo");
    let col_unidad = find_col("unidad_medida");
    let col_servicio = find_col("es_servicio");
    let col_requiere_serie = find_col("requiere_serie");
    let col_requiere_caducidad = find_col("requiere_caducidad");
    let col_lote = find_col("lote");
    let col_fecha_caducidad = find_col("fecha_caducidad");

    let mut creados = 0i64;
    let mut actualizados = 0i64;
    let mut errores = 0i64;
    let mut lotes_creados = 0i64;
    let mut warnings_caducidad: Vec<String> = Vec::new();
    let mut msgs: Vec<String> = Vec::new();

    for (line_idx, row) in rows_iter.enumerate() {
        let get_str = |idx: Option<usize>| -> String {
            idx.and_then(|i| row.get(i)).map(|c| c.to_string().trim().to_string()).unwrap_or_default()
        };
        let get_f64 = |idx: Option<usize>, default: f64| -> f64 {
            idx.and_then(|i| row.get(i)).and_then(|c| match c {
                calamine::Data::Float(f) => Some(*f),
                calamine::Data::Int(i) => Some(*i as f64),
                calamine::Data::String(s) => s.trim().parse::<f64>().ok(),
                _ => None,
            }).unwrap_or(default)
        };

        let nombre = get_str(Some(col_nombre));
        if nombre.is_empty() { continue; } // Skip empty rows

        let codigo = get_str(col_codigo);
        let codigo_barras = get_str(col_codigo_barras);
        let descripcion = get_str(col_descripcion);
        let categoria_nombre = get_str(col_categoria);
        let precio_costo = get_f64(col_precio_costo, 0.0);
        let precio_venta = get_f64(col_precio_venta, 0.0);
        let iva = get_f64(col_iva, 0.0);
        let mut stock = get_f64(col_stock, 0.0);
        let stock_min = get_f64(col_stock_min, 0.0);
        let unidad = get_str(col_unidad);
        let es_servicio = get_str(col_servicio) == "1" || get_f64(col_servicio, 0.0) == 1.0;
        let requiere_serie = get_str(col_requiere_serie) == "1" || get_f64(col_requiere_serie, 0.0) == 1.0;
        let requiere_caducidad = get_str(col_requiere_caducidad) == "1" || get_f64(col_requiere_caducidad, 0.0) == 1.0;
        let lote_str = get_str(col_lote);
        let fecha_caducidad_str = get_str(col_fecha_caducidad);

        // Si requiere_caducidad pero no hay fecha, marcar warning y forzar stock=0
        let mut warn_sin_fecha = false;
        if requiere_caducidad && fecha_caducidad_str.is_empty() {
            warn_sin_fecha = true;
            stock = 0.0;
        }

        // Find or create category (default: "General")
        let cat_nombre = if categoria_nombre.is_empty() { "General".to_string() } else { categoria_nombre };
        let categoria_id: Option<i64> = {
            let existing: Option<i64> = conn.query_row(
                "SELECT id FROM categorias WHERE LOWER(nombre) = LOWER(?1)", rusqlite::params![cat_nombre], |r| r.get(0)
            ).ok();
            Some(match existing {
                Some(id) => id,
                None => {
                    conn.execute("INSERT INTO categorias (nombre) VALUES (?1)", rusqlite::params![cat_nombre]).ok();
                    conn.last_insert_rowid()
                }
            })
        };

        // Check if product exists by codigo
        let existing_id: Option<i64> = if !codigo.is_empty() {
            conn.query_row("SELECT id FROM productos WHERE codigo = ?1", rusqlite::params![codigo], |r| r.get(0)).ok()
        } else { None };

        let producto_id_afectado: Option<i64> = if let Some(id) = existing_id {
            match conn.execute(
                "UPDATE productos SET nombre=?1, descripcion=?2, categoria_id=?3, precio_costo=?4, precio_venta=?5, iva_porcentaje=?6, stock_actual=?7, stock_minimo=?8, unidad_medida=?9, es_servicio=?10, codigo_barras=?11, requiere_serie=?12, requiere_caducidad=?13 WHERE id=?14",
                rusqlite::params![nombre, descripcion, categoria_id, precio_costo, precio_venta, iva, stock, stock_min, if unidad.is_empty() { "UND" } else { &unidad }, es_servicio as i32, if codigo_barras.is_empty() { None } else { Some(&codigo_barras) }, requiere_serie as i32, requiere_caducidad as i32, id]
            ) {
                Ok(_) => { actualizados += 1; Some(id) }
                Err(e) => { errores += 1; msgs.push(format!("Fila {}: {}", line_idx + 2, e)); None }
            }
        } else {
            let final_codigo = if codigo.is_empty() {
                let next: i64 = conn.query_row("SELECT COALESCE(MAX(CAST(REPLACE(codigo, 'P', '') AS INTEGER)), 0) + 1 FROM productos WHERE codigo LIKE 'P%'", [], |r| r.get(0)).unwrap_or(1);
                format!("P{:04}", next)
            } else { codigo };

            match conn.execute(
                "INSERT INTO productos (codigo, codigo_barras, nombre, descripcion, categoria_id, precio_costo, precio_venta, iva_porcentaje, stock_actual, stock_minimo, unidad_medida, es_servicio, activo, incluye_iva, requiere_serie, requiere_caducidad) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, 1, 0, ?13, ?14)",
                rusqlite::params![final_codigo, if codigo_barras.is_empty() { None } else { Some(&codigo_barras) }, nombre, descripcion, categoria_id, precio_costo, precio_venta, iva, stock, stock_min, if unidad.is_empty() { "UND".to_string() } else { unidad }, es_servicio as i32, requiere_serie as i32, requiere_caducidad as i32]
            ) {
                Ok(_) => { creados += 1; Some(conn.last_insert_rowid()) }
                Err(e) => { errores += 1; msgs.push(format!("Fila {}: {}", line_idx + 2, e)); None }
            }
        };

        // Manejo de caducidad: si se especificó fecha, crear lote; si no hay fecha, warning.
        if let Some(pid) = producto_id_afectado {
            if requiere_caducidad {
                if !fecha_caducidad_str.is_empty() {
                    let lote_value: Option<String> = if lote_str.is_empty() { None } else { Some(lote_str.clone()) };
                    let stock_lote = if stock > 0.0 { stock } else { 0.0 };
                    if stock_lote > 0.0 {
                        if conn.execute(
                            "INSERT INTO lotes_caducidad (producto_id, lote, fecha_caducidad, cantidad, cantidad_inicial) VALUES (?1, ?2, ?3, ?4, ?4)",
                            rusqlite::params![pid, lote_value, fecha_caducidad_str, stock_lote]
                        ).is_ok() {
                            lotes_creados += 1;
                        }
                    }
                } else if warn_sin_fecha {
                    // Asegurar stock=0 en BD (el UPDATE/INSERT ya usó stock=0, pero por seguridad)
                    conn.execute("UPDATE productos SET stock_actual = 0 WHERE id = ?1", rusqlite::params![pid]).ok();
                    warnings_caducidad.push(nombre.clone());
                }
            }
        }
    }

    Ok(serde_json::json!({
        "creados": creados,
        "actualizados": actualizados,
        "errores": errores,
        "lotes_creados": lotes_creados,
        "warnings_caducidad": warnings_caducidad,
        "mensajes": msgs
    }))
}

// --- Números de Serie ---

#[tauri::command]
pub fn registrar_series(
    db: State<Database>,
    producto_id: i64,
    seriales: Vec<String>,
    compra_id: Option<i64>,
) -> Result<serde_json::Value, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let mut insertados = 0i64;
    let mut duplicados = 0i64;
    for serial in &seriales {
        let s = serial.trim();
        if s.is_empty() { continue; }
        match conn.execute(
            "INSERT INTO numeros_serie (producto_id, serial, compra_id) VALUES (?1, ?2, ?3)",
            rusqlite::params![producto_id, s.to_uppercase(), compra_id],
        ) {
            Ok(_) => insertados += 1,
            Err(_) => duplicados += 1,
        }
    }
    Ok(serde_json::json!({"insertados": insertados, "duplicados": duplicados}))
}

#[tauri::command]
pub fn listar_series_producto(
    db: State<Database>,
    producto_id: i64,
    estado: Option<String>,
) -> Result<Vec<serde_json::Value>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let query = if let Some(ref est) = estado {
        format!("SELECT ns.id, ns.serial, ns.estado, ns.compra_id, ns.venta_id, ns.cliente_nombre, ns.fecha_ingreso, ns.fecha_venta, ns.observacion FROM numeros_serie ns WHERE ns.producto_id = ?1 AND ns.estado = '{}' ORDER BY ns.fecha_ingreso DESC", est)
    } else {
        "SELECT ns.id, ns.serial, ns.estado, ns.compra_id, ns.venta_id, ns.cliente_nombre, ns.fecha_ingreso, ns.fecha_venta, ns.observacion FROM numeros_serie ns WHERE ns.producto_id = ?1 ORDER BY ns.fecha_ingreso DESC".to_string()
    };
    let mut stmt = conn.prepare(&query).map_err(|e| e.to_string())?;
    let rows = stmt.query_map(rusqlite::params![producto_id], |row| {
        Ok(serde_json::json!({
            "id": row.get::<_, i64>(0)?,
            "serial": row.get::<_, String>(1)?,
            "estado": row.get::<_, String>(2)?,
            "compra_id": row.get::<_, Option<i64>>(3)?,
            "venta_id": row.get::<_, Option<i64>>(4)?,
            "cliente_nombre": row.get::<_, Option<String>>(5)?,
            "fecha_ingreso": row.get::<_, String>(6)?,
            "fecha_venta": row.get::<_, Option<String>>(7)?,
            "observacion": row.get::<_, Option<String>>(8)?
        }))
    }).map_err(|e| e.to_string())?;
    rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn series_disponibles(
    db: State<Database>,
    producto_id: i64,
) -> Result<Vec<serde_json::Value>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let mut stmt = conn.prepare(
        "SELECT id, serial FROM numeros_serie WHERE producto_id = ?1 AND estado = 'DISPONIBLE' ORDER BY serial"
    ).map_err(|e| e.to_string())?;
    let rows = stmt.query_map(rusqlite::params![producto_id], |row| {
        Ok(serde_json::json!({"id": row.get::<_, i64>(0)?, "serial": row.get::<_, String>(1)?}))
    }).map_err(|e| e.to_string())?;
    rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn marcar_serie_vendida(
    db: State<Database>,
    serie_id: i64,
    venta_id: i64,
    venta_detalle_id: Option<i64>,
    cliente_id: Option<i64>,
    cliente_nombre: Option<String>,
) -> Result<(), String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    conn.execute(
        "UPDATE numeros_serie SET estado = 'VENDIDO', venta_id = ?1, venta_detalle_id = ?2, cliente_id = ?3, cliente_nombre = ?4, fecha_venta = datetime('now','localtime') WHERE id = ?5 AND estado = 'DISPONIBLE'",
        rusqlite::params![venta_id, venta_detalle_id, cliente_id, cliente_nombre, serie_id],
    ).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn buscar_serie(
    db: State<Database>,
    serial: String,
) -> Result<Vec<serde_json::Value>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let mut stmt = conn.prepare(
        "SELECT ns.id, ns.serial, ns.estado, ns.fecha_ingreso, ns.fecha_venta, ns.cliente_nombre, ns.observacion, p.nombre as producto_nombre, p.id as producto_id
         FROM numeros_serie ns
         JOIN productos p ON ns.producto_id = p.id
         WHERE ns.serial LIKE ?1
         ORDER BY ns.fecha_ingreso DESC LIMIT 50"
    ).map_err(|e| e.to_string())?;
    let busq = format!("%{}%", serial.trim().to_uppercase());
    let rows = stmt.query_map(rusqlite::params![busq], |row| {
        Ok(serde_json::json!({
            "id": row.get::<_, i64>(0)?,
            "serial": row.get::<_, String>(1)?,
            "estado": row.get::<_, String>(2)?,
            "fecha_ingreso": row.get::<_, String>(3)?,
            "fecha_venta": row.get::<_, Option<String>>(4)?,
            "cliente_nombre": row.get::<_, Option<String>>(5)?,
            "observacion": row.get::<_, Option<String>>(6)?,
            "producto_nombre": row.get::<_, String>(7)?,
            "producto_id": row.get::<_, i64>(8)?
        }))
    }).map_err(|e| e.to_string())?;
    rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn devolver_serie(
    db: State<Database>,
    serie_id: i64,
) -> Result<(), String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    conn.execute(
        "UPDATE numeros_serie SET estado = 'DISPONIBLE', venta_id = NULL, venta_detalle_id = NULL, cliente_id = NULL, cliente_nombre = NULL, fecha_venta = NULL WHERE id = ?1",
        rusqlite::params![serie_id],
    ).map_err(|e| e.to_string())?;
    Ok(())
}

// --- Caducidad / Lotes ---

#[tauri::command]
pub fn registrar_lote_caducidad(
    db: State<Database>,
    producto_id: i64,
    lote: Option<String>,
    fecha_caducidad: String,
    cantidad: f64,
    compra_id: Option<i64>,
    observacion: Option<String>,
    fecha_elaboracion: Option<String>,
) -> Result<i64, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    conn.execute(
        "INSERT INTO lotes_caducidad (producto_id, lote, fecha_caducidad, cantidad, cantidad_inicial, compra_id, observacion, fecha_elaboracion) VALUES (?1, ?2, ?3, ?4, ?4, ?5, ?6, ?7)",
        rusqlite::params![producto_id, lote, fecha_caducidad, cantidad, compra_id, observacion, fecha_elaboracion],
    ).map_err(|e| e.to_string())?;
    Ok(conn.last_insert_rowid())
}

#[tauri::command]
pub fn listar_lotes_producto(db: State<Database>, producto_id: i64) -> Result<Vec<serde_json::Value>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let mut stmt = conn.prepare(
        "SELECT id, lote, fecha_caducidad, cantidad, cantidad_inicial, observacion, fecha_ingreso, fecha_elaboracion
         FROM lotes_caducidad WHERE producto_id = ?1 AND cantidad > 0 ORDER BY fecha_caducidad ASC"
    ).map_err(|e| e.to_string())?;
    let rows = stmt.query_map(rusqlite::params![producto_id], |row| {
        Ok(serde_json::json!({
            "id": row.get::<_, i64>(0)?,
            "lote": row.get::<_, Option<String>>(1)?,
            "fecha_caducidad": row.get::<_, String>(2)?,
            "cantidad": row.get::<_, f64>(3)?,
            "cantidad_inicial": row.get::<_, f64>(4)?,
            "observacion": row.get::<_, Option<String>>(5)?,
            "fecha_ingreso": row.get::<_, String>(6)?,
            "fecha_elaboracion": row.get::<_, Option<String>>(7)?,
        }))
    }).map_err(|e| e.to_string())?;
    rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn alertas_caducidad(db: State<Database>) -> Result<serde_json::Value, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let dias_alerta: i64 = conn.query_row("SELECT value FROM config WHERE key = 'caducidad_dias_alerta'", [], |r| r.get::<_, String>(0))
        .ok().and_then(|s| s.parse().ok()).unwrap_or(7);

    let mut stmt = conn.prepare(
        "SELECT l.id, l.lote, l.fecha_caducidad, l.cantidad, p.id, p.nombre, p.codigo,
                CASE
                    WHEN date(l.fecha_caducidad) < date('now', 'localtime') THEN 'VENCIDO'
                    WHEN date(l.fecha_caducidad) <= date('now', 'localtime', '+' || ?1 || ' days') THEN 'POR_VENCER'
                    ELSE 'OK'
                END as estado,
                CAST(julianday(l.fecha_caducidad) - julianday(date('now','localtime')) AS INTEGER) as dias_restantes,
                l.fecha_elaboracion
         FROM lotes_caducidad l
         JOIN productos p ON l.producto_id = p.id
         WHERE l.cantidad > 0
           AND date(l.fecha_caducidad) <= date('now', 'localtime', '+' || ?1 || ' days')
         ORDER BY l.fecha_caducidad ASC"
    ).map_err(|e| e.to_string())?;

    let rows: Vec<serde_json::Value> = stmt.query_map(rusqlite::params![dias_alerta], |row| {
        Ok(serde_json::json!({
            "id": row.get::<_, i64>(0)?,
            "lote": row.get::<_, Option<String>>(1)?,
            "fecha_caducidad": row.get::<_, String>(2)?,
            "cantidad": row.get::<_, f64>(3)?,
            "producto_id": row.get::<_, i64>(4)?,
            "producto_nombre": row.get::<_, String>(5)?,
            "producto_codigo": row.get::<_, Option<String>>(6)?,
            "estado": row.get::<_, String>(7)?,
            "dias_restantes": row.get::<_, i64>(8)?,
            "fecha_elaboracion": row.get::<_, Option<String>>(9)?,
        }))
    }).map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())?;

    let vencidos = rows.iter().filter(|r| r["estado"] == "VENCIDO").count();
    let por_vencer = rows.iter().filter(|r| r["estado"] == "POR_VENCER").count();

    Ok(serde_json::json!({
        "lotes": rows,
        "vencidos": vencidos,
        "por_vencer": por_vencer,
        "dias_alerta": dias_alerta
    }))
}

#[tauri::command]
pub fn eliminar_lote_caducidad(db: State<Database>, lote_id: i64) -> Result<(), String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    conn.execute("DELETE FROM lotes_caducidad WHERE id = ?1", rusqlite::params![lote_id])
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn ajustar_cantidad_lote(db: State<Database>, lote_id: i64, cantidad: f64) -> Result<(), String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    conn.execute(
        "UPDATE lotes_caducidad SET cantidad = ?1 WHERE id = ?2",
        rusqlite::params![cantidad.max(0.0), lote_id],
    ).map_err(|e| e.to_string())?;
    Ok(())
}

// ============================================================================
// PRESENTACIONES / UNIDADES MULTIPLES POR PRODUCTO (v1.9.7)
// ============================================================================

/// Lista las unidades/presentaciones de un producto, incluyendo sus precios por lista.
/// Cada unidad trae un array `precios_lista: [{ lista_precio_id, precio }]`.
#[tauri::command]
pub fn listar_unidades_producto(db: State<Database>, producto_id: i64) -> Result<Vec<serde_json::Value>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let mut stmt = conn.prepare(
        "SELECT id, nombre, abreviatura, factor, precio, es_base, orden, activa,
                COALESCE(tipo_unidad_id, 0) as tipo_unidad_id
         FROM unidades_producto
         WHERE producto_id = ?1 AND activa = 1
         ORDER BY orden, factor"
    ).map_err(|e| e.to_string())?;

    let filas: Vec<(i64, serde_json::Value)> = stmt.query_map(rusqlite::params![producto_id], |row| {
        let id: i64 = row.get(0)?;
        let tid: i64 = row.get(8)?;
        let v = serde_json::json!({
            "id": id,
            "nombre": row.get::<_, String>(1)?,
            "abreviatura": row.get::<_, Option<String>>(2)?,
            "factor": row.get::<_, f64>(3)?,
            "precio": row.get::<_, f64>(4)?,
            "es_base": row.get::<_, i32>(5)? != 0,
            "orden": row.get::<_, i32>(6)?,
            "activa": row.get::<_, i32>(7)? != 0,
            "tipo_unidad_id": if tid == 0 { serde_json::Value::Null } else { serde_json::Value::Number(tid.into()) },
        });
        Ok((id, v))
    }).map_err(|e| e.to_string())?
    .collect::<Result<Vec<_>, _>>()
    .map_err(|e| e.to_string())?;
    drop(stmt);

    // Cargar precios por lista para cada unidad
    let mut unidades: Vec<serde_json::Value> = Vec::new();
    for (uid, mut u) in filas {
        let mut stmt2 = conn.prepare(
            "SELECT lista_precio_id, precio FROM precios_unidad_lista WHERE unidad_id = ?1"
        ).map_err(|e| e.to_string())?;
        let precios: Vec<serde_json::Value> = stmt2.query_map(rusqlite::params![uid], |row| {
            Ok(serde_json::json!({
                "lista_precio_id": row.get::<_, i64>(0)?,
                "precio": row.get::<_, f64>(1)?,
            }))
        }).map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;
        u["precios_lista"] = serde_json::Value::Array(precios);
        unidades.push(u);
    }
    Ok(unidades)
}

/// Reemplaza todas las unidades de un producto (operación atómica)
/// Cada unidad puede incluir `precios_lista: [{ lista_precio_id, precio }]` para overrides por lista.
#[tauri::command]
pub fn guardar_unidades_producto(
    db: State<Database>,
    producto_id: i64,
    unidades: Vec<serde_json::Value>,
) -> Result<(), String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;

    // Borrar las existentes (CASCADE limpia precios_unidad_lista)
    conn.execute("DELETE FROM unidades_producto WHERE producto_id = ?1",
        rusqlite::params![producto_id]).map_err(|e| e.to_string())?;

    for (i, u) in unidades.iter().enumerate() {
        let nombre = u.get("nombre").and_then(|v| v.as_str()).unwrap_or("").to_string();
        let abreviatura = u.get("abreviatura").and_then(|v| v.as_str()).map(|s| s.to_string());
        let factor = u.get("factor").and_then(|v| v.as_f64()).unwrap_or(1.0);
        let precio = u.get("precio").and_then(|v| v.as_f64()).unwrap_or(0.0);
        let es_base = u.get("es_base").and_then(|v| v.as_bool()).unwrap_or(false);
        let tipo_unidad_id = u.get("tipo_unidad_id").and_then(|v| v.as_i64());

        if nombre.trim().is_empty() { continue; }
        if factor <= 0.0 { return Err(format!("Factor invalido para {}", nombre)); }

        conn.execute(
            "INSERT INTO unidades_producto (producto_id, nombre, abreviatura, factor, precio, es_base, orden, activa, tipo_unidad_id)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 1, ?8)",
            rusqlite::params![producto_id, nombre.trim(), abreviatura, factor, precio, es_base as i32, i as i32, tipo_unidad_id],
        ).map_err(|e| e.to_string())?;
        let unidad_id = conn.last_insert_rowid();

        // Precios por lista (opcional)
        if let Some(precios_lista) = u.get("precios_lista").and_then(|v| v.as_array()) {
            for pl in precios_lista {
                let lista_id = pl.get("lista_precio_id").and_then(|v| v.as_i64()).unwrap_or(0);
                let pl_precio = pl.get("precio").and_then(|v| v.as_f64()).unwrap_or(0.0);
                if lista_id > 0 && pl_precio > 0.0 {
                    conn.execute(
                        "INSERT OR REPLACE INTO precios_unidad_lista (unidad_id, lista_precio_id, precio) VALUES (?1, ?2, ?3)",
                        rusqlite::params![unidad_id, lista_id, pl_precio],
                    ).ok();
                }
            }
        }
    }
    Ok(())
}

