use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use crate::db::{Database, SesionState};
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

    let tipo_producto = match producto.tipo_producto.as_str() {
        "COMBO_FIJO" | "COMBO_FLEXIBLE" => producto.tipo_producto.clone(),
        _ => "SIMPLE".to_string(),
    };

    // Restaurante: validar destino_preparacion (default COCINA si vacio o invalido)
    let destino_preparacion = match producto.destino_preparacion.as_str() {
        "BARRA" | "DIRECTO" => producto.destino_preparacion.clone(),
        _ => "COCINA".to_string(),
    };

    conn.execute(
        "INSERT INTO productos (codigo, codigo_barras, nombre, descripcion, categoria_id,
         precio_costo, precio_venta, precio_minimo, iva_porcentaje, incluye_iva, stock_actual, stock_minimo,
         unidad_medida, es_servicio, activo, imagen, requiere_serie, requiere_caducidad, no_controla_stock,
         tipo_producto, destino_preparacion)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21)",
        rusqlite::params![
            codigo,
            codigo_barras,
            producto.nombre,
            producto.descripcion,
            producto.categoria_id,
            producto.precio_costo,
            producto.precio_venta,
            producto.precio_minimo,
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
            tipo_producto,
            destino_preparacion,
        ],
    )
    .map_err(|e| e.to_string())?;

    let nuevo_id = conn.last_insert_rowid();

    // Movimiento INICIAL en el kardex para que el stock inicial tenga origen
    // trazable (stock_anterior=0 → stock_nuevo=stock_actual). Solo si el
    // producto controla stock y tiene cantidad inicial > 0.
    let controla_stock = !producto.no_controla_stock && !producto.es_servicio
        && tipo_producto == "SIMPLE";
    if controla_stock && producto.stock_actual > 0.0 {
        conn.execute(
            "INSERT INTO movimientos_inventario
                (producto_id, tipo, cantidad, stock_anterior, stock_nuevo, costo_unitario, motivo, usuario)
             VALUES (?1, 'INICIAL', ?2, 0, ?2, ?3, 'Stock inicial', NULL)",
            rusqlite::params![nuevo_id, producto.stock_actual, producto.precio_costo],
        )
        .map_err(|e| e.to_string())?;
    }

    Ok(nuevo_id)
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

    let tipo_producto = match producto.tipo_producto.as_str() {
        "COMBO_FIJO" | "COMBO_FLEXIBLE" => producto.tipo_producto.clone(),
        _ => "SIMPLE".to_string(),
    };

    // Restaurante: validar destino_preparacion
    let destino_preparacion = match producto.destino_preparacion.as_str() {
        "BARRA" | "DIRECTO" => producto.destino_preparacion.clone(),
        _ => "COCINA".to_string(),
    };

    // IMPORTANTE: NO se actualiza `stock_actual` desde la edición del producto.
    // El stock solo cambia por movimientos de kardex (Ajuste/Entrada/Salida,
    // compras, ventas) para mantener la trazabilidad del inventario. El valor
    // que envíe el frontend se ignora aquí a propósito.
    conn.execute(
        "UPDATE productos SET codigo=?1, codigo_barras=?2, nombre=?3, descripcion=?4,
         categoria_id=?5, precio_costo=?6, precio_venta=?7, precio_minimo=?8, iva_porcentaje=?9,
         incluye_iva=?10, stock_minimo=?11, unidad_medida=?12,
         es_servicio=?13, activo=?14, imagen=?15, requiere_serie=?16, requiere_caducidad=?17,
         no_controla_stock=?18, tipo_producto=?19, destino_preparacion=?20,
         updated_at=datetime('now','localtime')
         WHERE id=?21",
        rusqlite::params![
            producto.codigo,
            codigo_barras,
            producto.nombre,
            producto.descripcion,
            producto.categoria_id,
            producto.precio_costo,
            producto.precio_venta,
            producto.precio_minimo,
            producto.iva_porcentaje,
            producto.incluye_iva as i32,
            producto.stock_minimo,
            producto.unidad_medida,
            producto.es_servicio as i32,
            producto.activo as i32,
            producto.imagen,
            producto.requiere_serie as i32,
            producto.requiere_caducidad as i32,
            producto.no_controla_stock as i32,
            tipo_producto,
            destino_preparacion,
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

    // Busqueda en: nombre, codigo, codigo_barras Y descripcion (info adicional).
    // Ordena por relevancia: 1=nombre, 2=codigo/cb, 3=descripcion. Asi los matches
    // del titulo aparecen primero y los de descripcion al final como fallback.
    // v2.5.21: tambien traemos es_servicio y no_controla_stock para el calculo
    // de stock disponible de combos en el frontend (excluir servicios)
    let mut stmt = conn
        .prepare(
            "SELECT p.id, p.codigo, p.codigo_barras, p.nombre, p.precio_venta, p.precio_costo, p.iva_porcentaje,
                    p.incluye_iva, p.stock_actual, p.stock_minimo, c.nombre as cat_nombre,
                    pp.precio as precio_lista,
                    COALESCE(p.es_servicio, 0), COALESCE(p.no_controla_stock, 0),
                    p.precio_minimo,
                    CASE
                        WHEN p.nombre LIKE ?1 THEN 1
                        WHEN p.codigo LIKE ?1 OR p.codigo_barras LIKE ?1 THEN 2
                        WHEN p.descripcion LIKE ?1 THEN 3
                        ELSE 4
                    END as match_score
             FROM productos p
             LEFT JOIN categorias c ON p.categoria_id = c.id
             LEFT JOIN precios_producto pp ON pp.producto_id = p.id AND pp.lista_precio_id = ?2
             WHERE p.activo = 1
             AND (p.nombre LIKE ?1 OR p.codigo LIKE ?1 OR p.codigo_barras LIKE ?1 OR p.descripcion LIKE ?1)
             ORDER BY match_score, p.nombre
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
                precio_minimo: row.get(14)?,
                precio_costo: row.get(5)?,
                iva_porcentaje: row.get(6)?,
                incluye_iva: row.get::<_, i32>(7)? != 0,
                stock_actual: row.get(8)?,
                stock_minimo: row.get(9)?,
                categoria_nombre: row.get(10)?,
                precio_lista: row.get(11)?,
                tiene_imagen: false, // no aplicable en esta query
                es_servicio: row.get::<_, i32>(12)? != 0,
                no_controla_stock: row.get::<_, i32>(13)? != 0,
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

    // v2.5.21: tambien traemos es_servicio y no_controla_stock (igual que la version sin multi-almacen)
    let mut stmt = conn
        .prepare(
            "SELECT p.id, p.codigo, p.nombre, p.precio_venta, p.iva_porcentaje, p.incluye_iva,
                    COALESCE(se.stock_actual, 0) as stock_local,
                    COALESCE(se.stock_minimo, p.stock_minimo) as stock_min,
                    c.nombre as cat_nombre,
                    pp.precio as precio_lista,
                    COALESCE(p.es_servicio, 0), COALESCE(p.no_controla_stock, 0),
                    p.precio_minimo,
                    CASE
                        WHEN p.nombre LIKE ?1 THEN 1
                        WHEN p.codigo LIKE ?1 OR p.codigo_barras LIKE ?1 THEN 2
                        WHEN p.descripcion LIKE ?1 THEN 3
                        ELSE 4
                    END as match_score
             FROM productos p
             LEFT JOIN categorias c ON p.categoria_id = c.id
             LEFT JOIN precios_producto pp ON pp.producto_id = p.id AND pp.lista_precio_id = ?2
             LEFT JOIN stock_establecimiento se ON se.producto_id = p.id AND se.establecimiento_id = ?3
             WHERE p.activo = 1
             AND (p.nombre LIKE ?1 OR p.codigo LIKE ?1 OR p.codigo_barras LIKE ?1 OR p.descripcion LIKE ?1)
             ORDER BY match_score, p.nombre
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
                precio_minimo: row.get(12)?,
                iva_porcentaje: row.get(4)?,
                incluye_iva: row.get::<_, i32>(5)? != 0,
                stock_actual: row.get(6)?,
                stock_minimo: row.get(7)?,
                categoria_nombre: row.get(8)?,
                precio_lista: row.get(9)?,
                tiene_imagen: false,
                es_servicio: row.get::<_, i32>(10)? != 0,
                no_controla_stock: row.get::<_, i32>(11)? != 0,
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
         no_controla_stock, COALESCE(tipo_producto, 'SIMPLE') as tipo_producto,
         COALESCE(destino_preparacion, 'COCINA') as destino_preparacion, precio_minimo
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
                precio_minimo: row.get(21)?,
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
                tipo_producto: row.get(19)?,
                destino_preparacion: row.get(20)?,
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

    // v2.4.14: incluimos flag `tiene_imagen` (no la imagen completa) para mostrar
    // miniatura en el listado. El frontend hace lazy-load por viewport.
    let sql = if solo_activos {
        "SELECT p.id, p.codigo, p.nombre, p.precio_venta, p.iva_porcentaje, p.incluye_iva,
                p.stock_actual, p.stock_minimo, c.nombre, pp.precio, p.precio_costo, p.codigo_barras,
                CASE WHEN p.imagen IS NOT NULL AND p.imagen != '' THEN 1 ELSE 0 END
         FROM productos p
         LEFT JOIN categorias c ON p.categoria_id = c.id
         LEFT JOIN precios_producto pp ON pp.producto_id = p.id AND pp.lista_precio_id = ?1
         WHERE p.activo = 1 ORDER BY p.nombre"
    } else {
        "SELECT p.id, p.codigo, p.nombre, p.precio_venta, p.iva_porcentaje, p.incluye_iva,
                p.stock_actual, p.stock_minimo, c.nombre, pp.precio, p.precio_costo, p.codigo_barras,
                CASE WHEN p.imagen IS NOT NULL AND p.imagen != '' THEN 1 ELSE 0 END
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
                codigo_barras: row.get(11)?,
                nombre: row.get(2)?,
                precio_venta: row.get(3)?,
                precio_minimo: None,
                precio_costo: row.get::<_, Option<f64>>(10)?.unwrap_or(0.0),
                iva_porcentaje: row.get(4)?,
                incluye_iva: row.get::<_, i32>(5)? != 0,
                stock_actual: row.get(6)?,
                stock_minimo: row.get(7)?,
                categoria_nombre: row.get(8)?,
                precio_lista: row.get(9)?,
                tiene_imagen: row.get::<_, i32>(12)? != 0,
                es_servicio: false, // listar_productos no incluye este flag, default false
                no_controla_stock: false,
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
                precio_minimo: None,
                iva_porcentaje: row.get(4)?,
                incluye_iva: row.get::<_, i32>(5)? != 0,
                stock_actual: row.get(6)?,
                stock_minimo: row.get(7)?,
                categoria_nombre: row.get(8)?,
                precio_lista: None,
                tiene_imagen: false,
                es_servicio: false,
                no_controla_stock: false,
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    Ok(productos)
}

// --- Imagen de producto ---

/// Tamaño máximo aceptado de input (5 MB). Imágenes más grandes se rechazan.
const MAX_BYTES_INPUT: usize = 5 * 1024 * 1024;
/// Tamaño máximo final almacenado en DB (500 KB). Imágenes más grandes se
/// redimensionan/recomprimen automáticamente.
const MAX_BYTES_FINAL: usize = 500_000;
/// Dimensión máxima en píxeles del lado mayor para resize automático.
const MAX_LADO_PX: u32 = 1024;

/// v2.4.2 — Optimiza una imagen para ajustarla al límite de 500 KB.
///
/// Si la imagen ya pesa <= 500 KB, la devuelve tal cual.
/// Si pesa más:
///   1. Decodifica con `image` crate (soporta PNG, JPG, GIF, BMP, WebP, etc.)
///   2. Redimensiona si el lado mayor > 1024 px (Lanczos3, mantiene aspect)
///   3. Re-encode como JPEG con calidad descendente (85→75→65→50→35) hasta entrar
///   4. Si tras todo eso sigue > 500 KB, devuelve error
///
/// Formatos exóticos que `image` no decodifica (SVG, HEIC):
///   - Si pesan <= 500 KB pasan
///   - Si pesan más, error pidiendo al usuario que reduzca manualmente
fn optimizar_imagen(bytes: Vec<u8>) -> Result<Vec<u8>, String> {
    if bytes.len() <= MAX_BYTES_FINAL {
        return Ok(bytes);
    }

    // Intentar decodificar
    let img_dyn = match image::load_from_memory(&bytes) {
        Ok(i) => i,
        Err(_) => {
            // Formato no decodificable (SVG/HEIC/etc.) y > 500KB → error
            return Err(format!(
                "La imagen pesa {:.1} KB y no se puede optimizar automáticamente \
                 (formato no soportado para resize). Máximo: {} KB. Reducila manualmente.",
                bytes.len() as f64 / 1024.0,
                MAX_BYTES_FINAL / 1024
            ));
        }
    };

    // Redimensionar si el lado mayor supera el límite
    use image::GenericImageView;
    let (w, h) = img_dyn.dimensions();
    let img_redim = if w > MAX_LADO_PX || h > MAX_LADO_PX {
        img_dyn.resize(MAX_LADO_PX, MAX_LADO_PX, image::imageops::FilterType::Lanczos3)
    } else {
        img_dyn
    };

    // Re-encode como JPEG con calidad descendente
    for quality in &[85u8, 75, 65, 50, 35] {
        let mut buf: Vec<u8> = Vec::new();
        let rgb = img_redim.to_rgb8();
        let mut encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut buf, *quality);
        if encoder.encode(&rgb, rgb.width(), rgb.height(), image::ColorType::Rgb8).is_ok()
            && buf.len() <= MAX_BYTES_FINAL
        {
            return Ok(buf);
        }
    }

    Err(format!(
        "No se pudo reducir la imagen a {} KB ni con calidad mínima. \
         Reducila manualmente y volvé a intentar.",
        MAX_BYTES_FINAL / 1024
    ))
}

/// Lee y codifica una imagen en base64 SIN tocar la DB.
/// v2.4.2: acepta hasta 5MB de input y redimensiona automáticamente si es
/// necesario para ajustar a 500 KB finales en DB.
#[tauri::command]
pub fn leer_imagen_archivo(imagen_path: String) -> Result<String, String> {
    let bytes = std::fs::read(&imagen_path)
        .map_err(|e| format!("Error leyendo imagen: {}", e))?;

    if bytes.len() > MAX_BYTES_INPUT {
        return Err(format!(
            "La imagen pesa {:.1} MB. Máximo {} MB. Reducila antes de cargar.",
            bytes.len() as f64 / (1024.0 * 1024.0),
            MAX_BYTES_INPUT / (1024 * 1024)
        ));
    }

    let optimizada = optimizar_imagen(bytes)?;
    Ok(BASE64.encode(&optimizada))
}

#[tauri::command]
pub fn cargar_imagen_producto(db: State<Database>, id: i64, imagen_path: String) -> Result<String, String> {
    let bytes = std::fs::read(&imagen_path)
        .map_err(|e| format!("Error leyendo imagen: {}", e))?;

    if bytes.len() > MAX_BYTES_INPUT {
        return Err(format!(
            "La imagen pesa {:.1} MB. Máximo {} MB. Reducila antes de cargar.",
            bytes.len() as f64 / (1024.0 * 1024.0),
            MAX_BYTES_INPUT / (1024 * 1024)
        ));
    }

    let optimizada = optimizar_imagen(bytes)?;
    let b64 = BASE64.encode(&optimizada);

    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    conn.execute(
        "UPDATE productos SET imagen = ?1, updated_at = datetime('now','localtime') WHERE id = ?2",
        rusqlite::params![b64, id],
    )
    .map_err(|e| e.to_string())?;

    Ok(b64)
}

/// v2.4.1 — Guarda imagen para un producto existente recibiendo el base64
/// directamente (en vez de leer un archivo del disco). Útil para soportar
/// **pegar imagen** desde el portapapeles o **drag & drop** desde el navegador.
///
/// El base64 puede venir con o sin el prefijo `data:image/xxx;base64,`. El
/// comando lo limpia automáticamente. Valida que el tamaño decodificado no
/// supere 500 KB.
#[tauri::command]
pub fn guardar_imagen_producto_b64(
    db: State<Database>,
    id: i64,
    base64: String,
) -> Result<String, String> {
    // Limpiar prefijo data URL si vino: "data:image/png;base64,iVBORw..."
    let limpio = if let Some(idx) = base64.find(",") {
        if base64[..idx].contains("base64") {
            &base64[idx + 1..]
        } else {
            base64.as_str()
        }
    } else {
        base64.as_str()
    };

    // Decodificar y validar tamaño bruto (input)
    let bytes = BASE64
        .decode(limpio.as_bytes())
        .map_err(|e| format!("Base64 inválido: {}", e))?;
    if bytes.is_empty() {
        return Err("La imagen está vacía".to_string());
    }
    if bytes.len() > MAX_BYTES_INPUT {
        return Err(format!(
            "La imagen pesa {:.1} MB. Máximo {} MB. Reducila antes de cargar.",
            bytes.len() as f64 / (1024.0 * 1024.0),
            MAX_BYTES_INPUT / (1024 * 1024)
        ));
    }

    // v2.4.2 — Optimizar (resize + recompress) si > 500 KB
    let optimizada = optimizar_imagen(bytes)?;
    let b64_final = BASE64.encode(&optimizada);

    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    conn.execute(
        "UPDATE productos SET imagen = ?1, updated_at = datetime('now','localtime') WHERE id = ?2",
        rusqlite::params![b64_final, id],
    )
    .map_err(|e| e.to_string())?;

    Ok(b64_final)
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
                    COALESCE(p.es_servicio, 0), COALESCE(p.no_controla_stock, 0),
                    COALESCE(p.tipo_producto, 'SIMPLE') as tipo_producto,
                    p.descripcion, p.codigo, p.codigo_barras, p.precio_minimo
             FROM productos p
             LEFT JOIN categorias c ON p.categoria_id = c.id
             WHERE p.activo = 1
             ORDER BY p.nombre",
        )
        .map_err(|e| e.to_string())?;

    let mut productos: Vec<ProductoTactil> = stmt
        .query_map([], |row| {
            Ok(ProductoTactil {
                id: row.get(0)?,
                nombre: row.get(1)?,
                precio_venta: row.get(2)?,
                precio_minimo: row.get(15)?,
                iva_porcentaje: row.get(3)?,
                incluye_iva: row.get::<_, i32>(4)? != 0,
                stock_actual: row.get(5)?,
                categoria_id: row.get(6)?,
                categoria_nombre: row.get(7)?,
                imagen: row.get(8)?,
                es_servicio: row.get::<_, i32>(9)? != 0,
                no_controla_stock: row.get::<_, i32>(10)? != 0,
                tipo_producto: row.get(11)?,
                stock_combo: None,
                descripcion: row.get(12)?,
                codigo: row.get(13)?,
                codigo_barras: row.get(14)?,
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    // Calcular stock dinamico para COMBO_FIJO (MIN(stock_hijo / cantidad_componente))
    // Saltar componentes que son servicios o sin control de stock.
    for prod in productos.iter_mut() {
        if prod.tipo_producto == "COMBO_FIJO" {
            let stock = crate::commands::combos::calcular_stock_combo(&conn, prod.id).ok().flatten();
            prod.stock_combo = stock;
        }
    }

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
pub fn eliminar_categoria(db: State<Database>, sesion: State<SesionState>, id: i64, accion: Option<String>, mover_a: Option<i64>) -> Result<serde_json::Value, String> {
    // Mismo permiso que productos: solo ADMIN o 'eliminar_productos'.
    {
        let sesion_guard = sesion.sesion.lock().map_err(|e| e.to_string())?;
        if let Some(s) = sesion_guard.as_ref() {
            if s.rol != "ADMIN" {
                let tiene = serde_json::from_str::<serde_json::Value>(&s.permisos)
                    .ok()
                    .and_then(|v| v.get("eliminar_productos")?.as_bool())
                    .unwrap_or(false);
                if !tiene {
                    return Err("No tiene permiso para eliminar categorías.".to_string());
                }
            }
        }
    }

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
                // v2.4.1: Eliminar producto por producto usando el helper que cae
                // a soft delete si hay referencias (venta_detalles, compras, etc.)
                // El DELETE masivo viejo fallaba con FK al primer producto referenciado.
                let mut ids_stmt = conn
                    .prepare("SELECT id FROM productos WHERE categoria_id = ?1")
                    .map_err(|e| e.to_string())?;
                let ids: Vec<i64> = ids_stmt
                    .query_map(rusqlite::params![id], |r| r.get::<_, i64>(0))
                    .map_err(|e| e.to_string())?
                    .collect::<Result<Vec<_>, _>>()
                    .map_err(|e| e.to_string())?;
                drop(ids_stmt);
                for pid in ids {
                    eliminar_producto_interno(&conn, pid)?;
                }
            }
            _ => {
                // Sin acción: retornar conteo para que el frontend pregunte
                return Ok(serde_json::json!({ "requiere_accion": true, "productos": count }));
            }
        }
    }

    // v2.4.1: intentar DELETE; si falla por FK (productos soft-deleted que siguen
    // apuntando a esta categoria_id), hacer SET categoria_id = NULL en sus filas
    // y reintentar. Así la categoría queda eliminada limpiamente.
    let resultado = conn.execute("DELETE FROM categorias WHERE id = ?1", rusqlite::params![id]);
    if resultado.is_err() {
        // Liberar referencias y reintentar
        conn.execute(
            "UPDATE productos SET categoria_id = NULL WHERE categoria_id = ?1",
            rusqlite::params![id],
        )
        .map_err(|e| format!("No se pudo liberar referencias de categoría: {}", e))?;
        conn.execute("DELETE FROM categorias WHERE id = ?1", rusqlite::params![id])
            .map_err(|e| format!("No se pudo eliminar categoría: {}", e))?;
    }
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

/// Elimina un producto. Estrategia v2.4.1:
///
/// SIEMPRE intenta soft delete (`activo = 0`) primero. Esto es lo correcto
/// porque los productos pueden estar referenciados desde MUCHAS tablas:
/// `venta_detalles`, `compra_detalles`, `kardex_movimientos`, `combo_componentes`,
/// `series_producto`, `lotes_producto`, `precios_producto`, `producto_warehouse_stock`,
/// `unidades_producto`, etc. Cualquiera de esas FK rompería el DELETE físico.
///
/// Solo si el producto está completamente "huérfano" (sin referencias en
/// ninguna tabla) intentamos DELETE físico para mantener la DB limpia.
///
/// El soft delete:
///   - marca `activo = 0`
///   - "libera" el `codigo` y `codigo_barras` apendiéndoles `_DEL{id}` para
///     que el usuario pueda crear otro producto con el mismo código
#[tauri::command]
pub fn eliminar_producto(db: State<Database>, sesion: State<SesionState>, id: i64) -> Result<(), String> {
    // Permiso: solo ADMIN o usuarios con 'eliminar_productos'. Por defecto los
    // usuarios nuevos (ej. cajeros) NO tienen este permiso → no pueden borrar.
    {
        let sesion_guard = sesion.sesion.lock().map_err(|e| e.to_string())?;
        if let Some(s) = sesion_guard.as_ref() {
            if s.rol != "ADMIN" {
                let tiene = serde_json::from_str::<serde_json::Value>(&s.permisos)
                    .ok()
                    .and_then(|v| v.get("eliminar_productos")?.as_bool())
                    .unwrap_or(false);
                if !tiene {
                    return Err("No tiene permiso para eliminar productos.".to_string());
                }
            }
        }
    }

    let conn = db.conn.lock().map_err(|e| e.to_string())?;

    // Bloquear borrado si el producto todavía controla stock y tiene cantidad > 0.
    // Así no se "evapora" inventario sin rastro: primero hay que ajustar a 0
    // (lo que deja un movimiento de kardex). Productos sin control de stock o
    // servicios sí pueden eliminarse directamente.
    let (stock, no_controla, es_servicio): (f64, i32, i32) = conn
        .query_row(
            "SELECT COALESCE(stock_actual, 0), COALESCE(no_controla_stock, 0), COALESCE(es_servicio, 0)
             FROM productos WHERE id = ?1",
            rusqlite::params![id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )
        .map_err(|_| "Producto no encontrado".to_string())?;
    if no_controla == 0 && es_servicio == 0 && stock.abs() > 0.0001 {
        // Prefijo BLOCK_DELETE_STOCK para que el frontend muestre un mensaje claro.
        return Err(format!("BLOCK_DELETE_STOCK:{}", stock));
    }

    eliminar_producto_interno(&conn, id)
}

/// Helper interno reutilizable por `eliminar_producto` y por `eliminar_categoria`
/// cuando el usuario elige "eliminar todos los productos de la categoría".
///
/// Estrategia: intenta DELETE físico primero. Si falla por FK constraint
/// (porque hay venta_detalles, compra_detalles, kardex, combos, series, etc.
/// que apuntan al producto), automáticamente hace soft delete liberando
/// `codigo` y `codigo_barras` para que puedan reusarse en un producto nuevo.
fn eliminar_producto_interno(conn: &rusqlite::Connection, id: i64) -> Result<(), String> {
    // Intento 1: DELETE físico (limpia DB si el producto está huérfano)
    let resultado_delete = conn.execute(
        "DELETE FROM productos WHERE id = ?1",
        rusqlite::params![id],
    );

    if resultado_delete.is_ok() {
        return Ok(());
    }

    // Falló DELETE → tiene referencias → soft delete liberando códigos
    conn.execute(
        "UPDATE productos
         SET activo = 0,
             codigo = COALESCE(codigo, '') || '_DEL' || id,
             codigo_barras = NULL
         WHERE id = ?1",
        rusqlite::params![id],
    )
    .map_err(|e| format!("No se pudo eliminar producto: {}", e))?;

    Ok(())
}

// --- Excel Import/Export ---

#[tauri::command]
pub fn exportar_plantilla_productos(db: State<Database>) -> Result<Vec<u8>, String> {
    use rust_xlsxwriter::*;

    // v2.5.37: cargar listas de precios para agregar una columna por cada una
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let listas: Vec<String> = {
        let mut stmt = conn.prepare(
            "SELECT nombre FROM listas_precios WHERE activo = 1 AND es_default = 0 ORDER BY id"
        ).map_err(|e| e.to_string())?;
        let rows: Vec<String> = stmt.query_map([], |r| r.get::<_, String>(0))
            .map_err(|e| e.to_string())?
            .filter_map(Result::ok)
            .collect();
        rows
    };
    drop(conn);

    let mut workbook = Workbook::new();
    let sheet = workbook.add_worksheet();
    sheet.set_name("Productos").map_err(|e| e.to_string())?;

    let header_fmt = Format::new().set_bold().set_background_color(Color::RGB(0x2563EB)).set_font_color(Color::White).set_border(FormatBorder::Thin);
    // Header destacado para columnas opcionales (precio_lista_X, incluye_iva)
    let header_opt_fmt = Format::new().set_bold().set_background_color(Color::RGB(0x16A34A)).set_font_color(Color::White).set_border(FormatBorder::Thin);

    // Columnas base (siempre)
    let headers_base = ["codigo", "codigo_barras", "nombre", "descripcion", "categoria", "precio_costo", "precio_venta", "iva_porcentaje", "incluye_iva", "stock_actual", "stock_minimo", "unidad_medida", "es_servicio", "requiere_serie", "requiere_caducidad", "lote", "fecha_caducidad"];
    for (i, h) in headers_base.iter().enumerate() {
        // incluye_iva (idx 8) en color verde como "campo opcional importante"
        let fmt = if *h == "incluye_iva" { &header_opt_fmt } else { &header_fmt };
        sheet.write_string_with_format(0, i as u16, *h, fmt).map_err(|e| e.to_string())?;
    }

    // v2.5.37: una columna por cada lista de precios adicional
    // formato: precio_<nombre_lista_normalizado>
    let base_count = headers_base.len() as u16;
    for (i, nombre_lista) in listas.iter().enumerate() {
        let col_name = format!("precio_{}", nombre_lista.replace(' ', "_"));
        sheet.write_string_with_format(0, base_count + i as u16, &col_name, &header_opt_fmt).map_err(|e| e.to_string())?;
    }

    // Ejemplo 1: producto normal sin caducidad, precio sin IVA, IVA 15%, NO incluye iva
    let example1 = ["P0001", "", "Producto ejemplo", "Descripcion", "General", "1.00", "2.50", "15", "0", "100", "5", "UND", "0", "0", "0", "", ""];
    for (i, v) in example1.iter().enumerate() {
        sheet.write_string(1, i as u16, *v).map_err(|e| e.to_string())?;
    }
    // Para listas adicionales en ejemplo 1: usar 2.30, 2.10, etc.
    for (i, _) in listas.iter().enumerate() {
        let precio = format!("{:.2}", 2.50 - 0.20 * (i + 1) as f64);
        sheet.write_string(1, base_count + i as u16, &precio).ok();
    }

    // Ejemplo 2: producto con precio bruto (incluye IVA), IVA 15%
    let example2 = ["P0002", "", "Producto IVA incluido", "Precio ya trae IVA", "General", "0.87", "1.00", "15", "1", "50", "5", "UND", "0", "0", "0", "", ""];
    for (i, v) in example2.iter().enumerate() {
        sheet.write_string(2, i as u16, *v).map_err(|e| e.to_string())?;
    }

    // Ejemplo 3: producto exento de IVA (iva_porcentaje vacío = sin IVA / 0%)
    let example3 = ["P0003", "", "Producto exento IVA", "Lacteos basicos", "Alimentos", "0.50", "0.80", "0", "0", "100", "10", "UND", "0", "0", "0", "", ""];
    for (i, v) in example3.iter().enumerate() {
        sheet.write_string(3, i as u16, *v).map_err(|e| e.to_string())?;
    }

    // Ejemplo 4: producto CON caducidad
    let example4 = ["P0004", "", "Yogurt", "Yogurt sabor fresa", "Lacteos", "0.50", "1.00", "15", "0", "20", "5", "UND", "0", "0", "1", "LOT-001", "2026-12-31"];
    for (i, v) in example4.iter().enumerate() {
        sheet.write_string(4, i as u16, *v).map_err(|e| e.to_string())?;
    }

    // Auto-fit columns en plantilla — hacerlo aquí, antes de soltar el borrow de `sheet`
    let total_cols = base_count + listas.len() as u16;
    for i in 0..total_cols {
        sheet.set_column_width(i, 15).map_err(|e| e.to_string())?;
    }
    sheet.set_column_width(2, 30).map_err(|e| e.to_string())?;
    sheet.set_column_width(3, 25).map_err(|e| e.to_string())?;
    sheet.set_column_width(15, 15).map_err(|e| e.to_string())?;
    sheet.set_column_width(16, 18).map_err(|e| e.to_string())?;
    // FIN del bloque sheet — necesario para soltar el borrow mutable de workbook
    let _ = sheet;

    // Hoja de instrucciones (segundo worksheet — requiere que el anterior haya salido de scope)
    {
        let inst_sheet = workbook.add_worksheet();
        inst_sheet.set_name("Instrucciones").map_err(|e| e.to_string())?;
        let bold_fmt = Format::new().set_bold();
        inst_sheet.write_string_with_format(0, 0, "Como llenar esta plantilla", &bold_fmt).ok();
        let instrucciones = vec![
            "",
            "COLUMNAS OBLIGATORIAS: nombre",
            "COLUMNAS RECOMENDADAS: codigo, categoria, precio_costo, precio_venta",
            "",
            "iva_porcentaje: 0 (exento), 5, 12, 15. Si dejas vacio = 0 (sin IVA).",
            "incluye_iva: 0 = el precio NO incluye IVA (se suma al cobrar). 1 = precio ya trae IVA incluido.",
            "",
            "LISTAS DE PRECIOS:",
            "  - Las columnas verdes 'precio_<NombreLista>' permiten poner precio por cada lista.",
            "  - Si solo llenas precio_venta y dejas las demas vacias, ese precio se replica en todas las listas.",
            "  - Si llenas una lista especifica, ese precio rige para esa lista (precio_venta se usa para la lista DEFAULT).",
            "",
            "es_servicio: 1 = no descuenta stock, 0 = producto fisico.",
            "requiere_serie: 1 = pide numero de serie al vender (electronicos, vehiculos).",
            "requiere_caducidad: 1 = el producto vence, se debe controlar por lotes.",
            "lote / fecha_caducidad: solo si requiere_caducidad=1.",
            "",
            "El codigo se autogenera si lo dejas vacio (P0001, P0002, ...).",
            "Si un codigo ya existe en el sistema, ese producto se ACTUALIZA en vez de duplicarse.",
        ];
        for (i, txt) in instrucciones.iter().enumerate() {
            inst_sheet.write_string((i + 1) as u32, 0, *txt).ok();
        }
        inst_sheet.set_column_width(0, 90).ok();
    }

    let buf = workbook.save_to_buffer().map_err(|e| e.to_string())?;
    Ok(buf)
}

#[tauri::command]
pub fn exportar_productos_excel(db: State<Database>) -> Result<Vec<u8>, String> {
    use rust_xlsxwriter::*;

    let conn = db.conn.lock().map_err(|e| e.to_string())?;

    // v2.5.37: cargar listas de precios adicionales (no la DEFAULT)
    let listas: Vec<(i64, String)> = {
        let mut stmt = conn.prepare(
            "SELECT id, nombre FROM listas_precios WHERE activo = 1 AND es_default = 0 ORDER BY id"
        ).map_err(|e| e.to_string())?;
        let rows: Vec<(i64, String)> = stmt.query_map([], |r| Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?)))
            .map_err(|e| e.to_string())?
            .filter_map(Result::ok)
            .collect();
        rows
    };

    let mut stmt = conn.prepare(
        "SELECT p.id, p.codigo, p.codigo_barras, p.nombre, p.descripcion,
                COALESCE(c.nombre, ''), p.precio_costo, p.precio_venta,
                p.iva_porcentaje, COALESCE(p.incluye_iva, 0),
                p.stock_actual, p.stock_minimo,
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
    let header_opt_fmt = Format::new().set_bold().set_background_color(Color::RGB(0x16A34A)).set_font_color(Color::White).set_border(FormatBorder::Thin);
    let money_fmt = Format::new().set_num_format("$#,##0.00");

    let headers_base = ["codigo", "codigo_barras", "nombre", "descripcion", "categoria", "precio_costo", "precio_venta", "iva_porcentaje", "incluye_iva", "stock_actual", "stock_minimo", "unidad_medida", "es_servicio", "requiere_serie", "requiere_caducidad", "lote", "fecha_caducidad"];
    for (i, h) in headers_base.iter().enumerate() {
        let fmt = if *h == "incluye_iva" { &header_opt_fmt } else { &header_fmt };
        sheet.write_string_with_format(0, i as u16, *h, fmt).map_err(|e| e.to_string())?;
    }
    let base_count = headers_base.len() as u16;
    for (i, (_, nombre_lista)) in listas.iter().enumerate() {
        let col_name = format!("precio_{}", nombre_lista.replace(' ', "_"));
        sheet.write_string_with_format(0, base_count + i as u16, &col_name, &header_opt_fmt).map_err(|e| e.to_string())?;
    }

    let mut row = 1u32;
    let rows = stmt.query_map([], |r| {
        Ok((
            r.get::<_, i64>(0)?,
            r.get::<_, Option<String>>(1)?.unwrap_or_default(),
            r.get::<_, Option<String>>(2)?.unwrap_or_default(),
            r.get::<_, String>(3)?,
            r.get::<_, Option<String>>(4)?.unwrap_or_default(),
            r.get::<_, String>(5)?,
            r.get::<_, f64>(6)?,
            r.get::<_, f64>(7)?,
            r.get::<_, f64>(8)?,
            r.get::<_, i32>(9)?,
            r.get::<_, f64>(10)?,
            r.get::<_, f64>(11)?,
            r.get::<_, Option<String>>(12)?.unwrap_or_default(),
            r.get::<_, i32>(13)?,
            r.get::<_, i32>(14)?,
            r.get::<_, i32>(15)?,
            r.get::<_, Option<String>>(16)?.unwrap_or_default(),
            r.get::<_, Option<String>>(17)?.unwrap_or_default(),
        ))
    }).map_err(|e| e.to_string())?;

    // Cargar precios por lista en lookup map: (producto_id, lista_id) → precio
    let precios_map: std::collections::HashMap<(i64, i64), f64> = {
        let mut stmt2 = conn.prepare(
            "SELECT producto_id, lista_precio_id, precio FROM precios_producto"
        ).map_err(|e| e.to_string())?;
        let mut map = std::collections::HashMap::new();
        let it = stmt2.query_map([], |r| Ok((r.get::<_, i64>(0)?, r.get::<_, i64>(1)?, r.get::<_, f64>(2)?)))
            .map_err(|e| e.to_string())?;
        for x in it.flatten() { map.insert((x.0, x.1), x.2); }
        map
    };

    for r in rows {
        let (pid, codigo, barras, nombre, desc, cat, costo, venta, iva, incl_iva, stock, stock_min, unidad, servicio, req_serie, req_caducidad, lote, fecha_cad) = r.map_err(|e| e.to_string())?;
        sheet.write_string(row, 0, &codigo).ok();
        sheet.write_string(row, 1, &barras).ok();
        sheet.write_string(row, 2, &nombre).ok();
        sheet.write_string(row, 3, &desc).ok();
        sheet.write_string(row, 4, &cat).ok();
        sheet.write_number_with_format(row, 5, costo, &money_fmt).ok();
        sheet.write_number_with_format(row, 6, venta, &money_fmt).ok();
        sheet.write_number(row, 7, iva).ok();
        sheet.write_number(row, 8, incl_iva as f64).ok();
        sheet.write_number(row, 9, stock).ok();
        sheet.write_number(row, 10, stock_min).ok();
        sheet.write_string(row, 11, &unidad).ok();
        sheet.write_number(row, 12, servicio as f64).ok();
        sheet.write_number(row, 13, req_serie as f64).ok();
        sheet.write_number(row, 14, req_caducidad as f64).ok();
        sheet.write_string(row, 15, &lote).ok();
        sheet.write_string(row, 16, &fecha_cad).ok();

        // Columnas de precios por lista
        for (i, (lista_id, _)) in listas.iter().enumerate() {
            if let Some(precio) = precios_map.get(&(pid, *lista_id)) {
                sheet.write_number_with_format(row, base_count + i as u16, *precio, &money_fmt).ok();
            }
        }

        row += 1;
    }

    // Column widths
    let total_cols = base_count + listas.len() as u16;
    for i in 0..total_cols { sheet.set_column_width(i, 15).ok(); }
    sheet.set_column_width(2, 35).ok();
    sheet.set_column_width(3, 25).ok();
    sheet.set_column_width(15, 15).ok();
    sheet.set_column_width(16, 18).ok();

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
    // v2.5.37: incluye_iva opcional. Si vacío en el archivo, se usa el default del producto.
    let col_incluye_iva = find_col("incluye_iva");
    let col_stock = find_col("stock_actual");
    let col_stock_min = find_col("stock_minimo");
    let col_unidad = find_col("unidad_medida");
    let col_servicio = find_col("es_servicio");
    let col_requiere_serie = find_col("requiere_serie");
    let col_requiere_caducidad = find_col("requiere_caducidad");
    let col_lote = find_col("lote");
    let col_fecha_caducidad = find_col("fecha_caducidad");

    // v2.5.37: detectar columnas precio_<NombreLista> y mapearlas a lista_id de la BD
    // (formato del header: precio_<nombre_normalizado> donde espacios se reemplazaron por _)
    let listas_db: Vec<(i64, String)> = {
        let mut stmt = conn.prepare(
            "SELECT id, nombre FROM listas_precios WHERE activo = 1 AND es_default = 0"
        ).map_err(|e| e.to_string())?;
        let rows: Vec<(i64, String)> = stmt.query_map([], |r| Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?)))
            .map_err(|e| e.to_string())?
            .filter_map(Result::ok)
            .collect();
        rows
    };
    // (lista_id, header_normalizado, col_idx)
    let mut cols_listas: Vec<(i64, String, usize)> = Vec::new();
    for (lid, nombre) in &listas_db {
        let header_esperado = format!("precio_{}", nombre.replace(' ', "_").to_lowercase());
        if let Some(idx) = headers.iter().position(|h| h == &header_esperado) {
            cols_listas.push((*lid, nombre.clone(), idx));
        }
    }

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
        // Lectura específica para celdas de FECHA: detecta si Excel guardó la
        // celda con formato Fecha (calamine entrega Float/DateTime con días serial)
        // o como Texto (String "YYYY-MM-DD"). Convierte ambos casos a YYYY-MM-DD.
        // Esto arregla el bug donde fechas formato Excel quedaban como "46265".
        let get_fecha = |idx: Option<usize>| -> String {
            let cell = match idx.and_then(|i| row.get(i)) {
                Some(c) => c,
                None => return String::new(),
            };
            match cell {
                calamine::Data::DateTime(dt) => {
                    crate::utils::excel_serial_to_iso(dt.as_f64()).unwrap_or_default()
                }
                calamine::Data::DateTimeIso(s) => {
                    // Ya viene en formato ISO (raro pero posible). Tomamos solo YYYY-MM-DD.
                    s.trim().chars().take(10).collect()
                }
                calamine::Data::Float(f) => {
                    // Si es un número en rango de Excel serial dates → convertir
                    if let Some(iso) = crate::utils::excel_serial_to_iso(*f) {
                        if (30000.0..=100000.0).contains(f) {
                            return iso;
                        }
                    }
                    // Sino, devolver representación textual (raro caso)
                    cell.to_string().trim().to_string()
                }
                calamine::Data::Int(i) => {
                    let f = *i as f64;
                    if let Some(iso) = crate::utils::excel_serial_to_iso(f) {
                        if (30000.0..=100000.0).contains(&f) {
                            return iso;
                        }
                    }
                    i.to_string()
                }
                calamine::Data::String(s) => {
                    let t = s.trim();
                    // Si es un string que contiene un número en rango Excel serial → convertir
                    if let Some(serial) = crate::utils::parse_posible_serial_excel(t) {
                        if let Some(iso) = crate::utils::excel_serial_to_iso(serial) {
                            return iso;
                        }
                    }
                    // Sino, asumimos que ya es YYYY-MM-DD u otro formato textual válido
                    t.to_string()
                }
                _ => String::new(),
            }
        };

        let nombre = get_str(Some(col_nombre));
        if nombre.is_empty() { continue; } // Skip empty rows

        let codigo = get_str(col_codigo);
        let codigo_barras = get_str(col_codigo_barras);
        let descripcion = get_str(col_descripcion);
        let categoria_nombre = get_str(col_categoria);
        let precio_costo = get_f64(col_precio_costo, 0.0);

        // v2.5.37: leer precios por lista ANTES de decidir precio_venta principal
        // (porque si precio_venta está vacío pero hay precios por lista, usamos el menor de ellos)
        let precios_por_lista: Vec<(i64, f64)> = cols_listas.iter()
            .map(|(lid, _, idx)| (*lid, get_f64(Some(*idx), 0.0)))
            .filter(|(_, p)| *p > 0.0)
            .collect();

        let precio_venta_raw = get_f64(col_precio_venta, 0.0);
        // Si precio_venta vacío Y hay al menos una lista llena → usar el primero como precio base
        let precio_venta = if precio_venta_raw > 0.0 {
            precio_venta_raw
        } else if let Some((_, p)) = precios_por_lista.first() {
            *p
        } else {
            0.0
        };

        // v2.5.37: iva_porcentaje. Si vacío (col existe pero celda blanca) → 0 (sin IVA).
        // Si la columna no existe en el archivo → 0 también.
        let iva = get_f64(col_iva, 0.0);

        // v2.5.37: incluye_iva. Si vacío en archivo, default = 0 (precio NO incluye IVA).
        // Si la columna existe y tiene 1 → precio_venta es bruto (ya trae IVA).
        let incluye_iva: i32 = if col_incluye_iva.is_some() {
            let s = get_str(col_incluye_iva);
            if s == "1" || s.to_lowercase() == "si" || s.to_lowercase() == "yes" || s.to_lowercase() == "true" {
                1
            } else if get_f64(col_incluye_iva, 0.0) == 1.0 {
                1
            } else {
                0
            }
        } else { 0 };

        let mut stock = get_f64(col_stock, 0.0);
        let stock_min = get_f64(col_stock_min, 0.0);
        let unidad = get_str(col_unidad);
        let es_servicio = get_str(col_servicio) == "1" || get_f64(col_servicio, 0.0) == 1.0;
        let requiere_serie = get_str(col_requiere_serie) == "1" || get_f64(col_requiere_serie, 0.0) == 1.0;
        let requiere_caducidad = get_str(col_requiere_caducidad) == "1" || get_f64(col_requiere_caducidad, 0.0) == 1.0;
        let lote_str = get_str(col_lote);
        // IMPORTANTE: usar get_fecha (no get_str) para que celdas Excel formato
        // Fecha se conviertan correctamente a YYYY-MM-DD en vez de quedar como
        // serial Excel "46265" — bug histórico que causaba lotes con
        // "vencimiento" en el año 1900 y -2,400,000 días restantes.
        let fecha_caducidad_str = get_fecha(col_fecha_caducidad);

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
            // v2.5.37: ahora actualiza tambien incluye_iva desde Excel
            match conn.execute(
                "UPDATE productos SET nombre=?1, descripcion=?2, categoria_id=?3, precio_costo=?4, precio_venta=?5, iva_porcentaje=?6, incluye_iva=?7, stock_actual=?8, stock_minimo=?9, unidad_medida=?10, es_servicio=?11, codigo_barras=?12, requiere_serie=?13, requiere_caducidad=?14 WHERE id=?15",
                rusqlite::params![nombre, descripcion, categoria_id, precio_costo, precio_venta, iva, incluye_iva, stock, stock_min, if unidad.is_empty() { "UND" } else { &unidad }, es_servicio as i32, if codigo_barras.is_empty() { None } else { Some(&codigo_barras) }, requiere_serie as i32, requiere_caducidad as i32, id]
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
                "INSERT INTO productos (codigo, codigo_barras, nombre, descripcion, categoria_id, precio_costo, precio_venta, iva_porcentaje, stock_actual, stock_minimo, unidad_medida, es_servicio, activo, incluye_iva, requiere_serie, requiere_caducidad) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, 1, ?13, ?14, ?15)",
                rusqlite::params![final_codigo, if codigo_barras.is_empty() { None } else { Some(&codigo_barras) }, nombre, descripcion, categoria_id, precio_costo, precio_venta, iva, stock, stock_min, if unidad.is_empty() { "UND".to_string() } else { unidad }, es_servicio as i32, incluye_iva, requiere_serie as i32, requiere_caducidad as i32]
            ) {
                Ok(_) => { creados += 1; Some(conn.last_insert_rowid()) }
                Err(e) => { errores += 1; msgs.push(format!("Fila {}: {}", line_idx + 2, e)); None }
            }
        };

        // v2.5.37: sync de precios por lista (precios_producto). Para cada columna
        // precio_<NombreLista> que vino llena, hacer UPSERT en precios_producto.
        // Si la celda venía vacía pero precio_venta tiene valor, replicar precio_venta
        // a TODAS las listas que existen pero no fueron llenadas (regla "rige sobre los demas").
        if let Some(pid) = producto_id_afectado {
            for (lista_id, _nombre_lista, col_idx) in &cols_listas {
                let precio_celda = get_f64(Some(*col_idx), 0.0);
                let precio_final = if precio_celda > 0.0 {
                    precio_celda
                } else if precio_venta > 0.0 {
                    // celda vacía pero precio_venta tiene valor → replicar
                    precio_venta
                } else {
                    continue; // ambos vacíos → no insertar
                };
                conn.execute(
                    "INSERT INTO precios_producto (producto_id, lista_precio_id, precio) VALUES (?1, ?2, ?3)
                     ON CONFLICT(producto_id, lista_precio_id) DO UPDATE SET precio = excluded.precio",
                    rusqlite::params![pid, lista_id, precio_final],
                ).ok();
            }
        }

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
    if cantidad <= 0.0 {
        return Err("La cantidad debe ser mayor a 0".into());
    }

    // Validar que fecha_caducidad sea YYYY-MM-DD válido.
    // Esto previene el bug histórico donde se guardaban Excel serial dates
    // crudos (ej. "46265") como si fueran fechas, generando lotes con
    // -2,400,000 días de "vida útil" basura.
    if chrono::NaiveDate::parse_from_str(fecha_caducidad.trim(), "%Y-%m-%d").is_err() {
        return Err(format!(
            "Fecha de caducidad invalida: '{}'. Formato esperado: YYYY-MM-DD",
            fecha_caducidad.trim()
        ));
    }
    if let Some(ref fe) = fecha_elaboracion {
        let fe_t = fe.trim();
        if !fe_t.is_empty() && chrono::NaiveDate::parse_from_str(fe_t, "%Y-%m-%d").is_err() {
            return Err(format!(
                "Fecha de elaboracion invalida: '{}'. Formato esperado: YYYY-MM-DD",
                fe_t
            ));
        }
    }

    let conn = db.conn.lock().map_err(|e| e.to_string())?;

    // Validacion: la suma de lotes no puede superar el stock actual.
    // Si compra_id viene seteado (lote auto-creado por una compra que ya sumo al stock),
    // se omite la validacion para no rechazar el flujo legitimo de compras.
    if compra_id.is_none() {
        let stock_actual: f64 = conn.query_row(
            "SELECT COALESCE(stock_actual, 0) FROM productos WHERE id = ?1",
            rusqlite::params![producto_id],
            |r| r.get(0),
        ).map_err(|e| format!("Producto no encontrado: {}", e))?;

        let suma_lotes: f64 = conn.query_row(
            "SELECT COALESCE(SUM(cantidad), 0) FROM lotes_caducidad WHERE producto_id = ?1 AND cantidad > 0",
            rusqlite::params![producto_id],
            |r| r.get(0),
        ).unwrap_or(0.0);

        let disponible = stock_actual - suma_lotes;
        if cantidad > disponible {
            return Err(format!(
                "No se puede agregar lote: stock actual {:.0}, ya asignados a lotes {:.0}, disponible para asignar {:.0}. Si recibio mas unidades, registre una COMPRA — esto crea el lote y suma al stock automaticamente.",
                stock_actual, suma_lotes, disponible.max(0.0)
            ));
        }
    }

    conn.execute(
        "INSERT INTO lotes_caducidad (producto_id, lote, fecha_caducidad, cantidad, cantidad_inicial, compra_id, observacion, fecha_elaboracion) VALUES (?1, ?2, ?3, ?4, ?4, ?5, ?6, ?7)",
        rusqlite::params![producto_id, lote, fecha_caducidad, cantidad, compra_id, observacion, fecha_elaboracion],
    ).map_err(|e| e.to_string())?;
    Ok(conn.last_insert_rowid())
}

/// Repara lotes con fechas guardadas como Excel serial date crudo (ej. "46265").
///
/// Este bug ocurría cuando el cliente importaba productos desde Excel y la
/// columna fecha_caducidad estaba formateada como "Fecha" en Excel — calamine
/// devolvía el serial Excel y se guardaba como string crudo.
///
/// Recorre `lotes_caducidad` y `lotes_caducidad.fecha_elaboracion`, detecta
/// strings que son números puros entre 30000-100000 (rango Excel serial dates
/// 1982-2173), los convierte a YYYY-MM-DD usando `excel_serial_to_iso()` y
/// hace UPDATE atómico.
///
/// Es idempotente: re-ejecutarlo no causa problema (los ya arreglados ya no
/// matchean el patrón "número puro").
///
/// Retorna `{ revisados, reparados, ejemplos: [{id, antes, despues}] }`.
#[tauri::command]
pub fn reparar_fechas_caducidad(db: State<Database>) -> Result<serde_json::Value, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;

    // Leer todos los lotes (id, fecha_caducidad, fecha_elaboracion)
    let mut stmt = conn
        .prepare("SELECT id, fecha_caducidad, COALESCE(fecha_elaboracion, '') FROM lotes_caducidad")
        .map_err(|e| e.to_string())?;

    let lotes: Vec<(i64, String, String)> = stmt
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;
    drop(stmt);

    let revisados = lotes.len();
    let mut reparados = 0i64;
    let mut ejemplos: Vec<serde_json::Value> = Vec::new();

    for (id, fecha_cad, fecha_elab) in lotes {
        // Reparar fecha_caducidad si es serial Excel
        let nueva_fecha_cad = crate::utils::parse_posible_serial_excel(&fecha_cad)
            .and_then(crate::utils::excel_serial_to_iso);
        // Reparar fecha_elaboracion si es serial Excel
        let nueva_fecha_elab = if !fecha_elab.is_empty() {
            crate::utils::parse_posible_serial_excel(&fecha_elab)
                .and_then(crate::utils::excel_serial_to_iso)
        } else {
            None
        };

        let cambio_cad = nueva_fecha_cad.is_some();
        let cambio_elab = nueva_fecha_elab.is_some();

        if !cambio_cad && !cambio_elab {
            continue;
        }

        // Update atómico
        let resultado = match (nueva_fecha_cad.as_deref(), nueva_fecha_elab.as_deref()) {
            (Some(nc), Some(ne)) => conn.execute(
                "UPDATE lotes_caducidad SET fecha_caducidad = ?1, fecha_elaboracion = ?2 WHERE id = ?3",
                rusqlite::params![nc, ne, id],
            ),
            (Some(nc), None) => conn.execute(
                "UPDATE lotes_caducidad SET fecha_caducidad = ?1 WHERE id = ?2",
                rusqlite::params![nc, id],
            ),
            (None, Some(ne)) => conn.execute(
                "UPDATE lotes_caducidad SET fecha_elaboracion = ?1 WHERE id = ?2",
                rusqlite::params![ne, id],
            ),
            (None, None) => unreachable!(),
        };

        if resultado.is_ok() {
            reparados += 1;
            // Guardar primeros 10 ejemplos para el toast/log
            if ejemplos.len() < 10 {
                ejemplos.push(serde_json::json!({
                    "id": id,
                    "fecha_caducidad_antes": fecha_cad,
                    "fecha_caducidad_despues": nueva_fecha_cad,
                    "fecha_elaboracion_antes": fecha_elab,
                    "fecha_elaboracion_despues": nueva_fecha_elab,
                }));
            }
        }
    }

    Ok(serde_json::json!({
        "revisados": revisados,
        "reparados": reparados,
        "ejemplos": ejemplos,
    }))
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

/// Lista TODOS los lotes con cantidad > 0 (no solo los en alerta), con filtros opcionales.
/// Retorna producto enriched + estado calculado en base a dias_alerta.
#[tauri::command]
pub fn listar_todos_lotes(
    db: State<Database>,
    filtro_estado: Option<String>,    // "TODOS" | "OK" | "POR_VENCER" | "VENCIDO"
    busqueda_producto: Option<String>, // busca en nombre/codigo
    incluir_agotados: Option<bool>,   // por default false (cantidad > 0)
) -> Result<serde_json::Value, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let dias_alerta: i64 = conn.query_row("SELECT value FROM config WHERE key = 'caducidad_dias_alerta'", [], |r| r.get::<_, String>(0))
        .ok().and_then(|s| s.parse().ok()).unwrap_or(7);

    let incluir_agotados = incluir_agotados.unwrap_or(false);
    let busqueda = busqueda_producto.unwrap_or_default().trim().to_lowercase();
    let filtro_estado = filtro_estado.unwrap_or_else(|| "TODOS".to_string()).to_uppercase();

    let mut where_parts: Vec<String> = Vec::new();
    if !incluir_agotados {
        where_parts.push("l.cantidad > 0".to_string());
    }
    let where_clause = if where_parts.is_empty() { String::new() } else { format!("WHERE {}", where_parts.join(" AND ")) };

    let sql = format!(
        "SELECT l.id, l.lote, l.fecha_caducidad, l.cantidad, l.cantidad_inicial,
                p.id, p.nombre, p.codigo, p.unidad_medida,
                CASE
                    WHEN date(l.fecha_caducidad) < date('now', 'localtime') THEN 'VENCIDO'
                    WHEN date(l.fecha_caducidad) <= date('now', 'localtime', '+' || ?1 || ' days') THEN 'POR_VENCER'
                    ELSE 'OK'
                END as estado,
                CAST(julianday(l.fecha_caducidad) - julianday(date('now','localtime')) AS INTEGER) as dias_restantes,
                l.fecha_elaboracion, l.fecha_ingreso, l.observacion, l.compra_id
         FROM lotes_caducidad l
         JOIN productos p ON l.producto_id = p.id
         {}
         ORDER BY l.fecha_caducidad ASC",
        where_clause
    );

    let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;
    let rows: Vec<serde_json::Value> = stmt.query_map(rusqlite::params![dias_alerta], |row| {
        Ok(serde_json::json!({
            "id": row.get::<_, i64>(0)?,
            "lote": row.get::<_, Option<String>>(1)?,
            "fecha_caducidad": row.get::<_, String>(2)?,
            "cantidad": row.get::<_, f64>(3)?,
            "cantidad_inicial": row.get::<_, f64>(4)?,
            "producto_id": row.get::<_, i64>(5)?,
            "producto_nombre": row.get::<_, String>(6)?,
            "producto_codigo": row.get::<_, Option<String>>(7)?,
            "producto_unidad": row.get::<_, Option<String>>(8)?,
            "estado": row.get::<_, String>(9)?,
            "dias_restantes": row.get::<_, i64>(10)?,
            "fecha_elaboracion": row.get::<_, Option<String>>(11)?,
            "fecha_ingreso": row.get::<_, String>(12)?,
            "observacion": row.get::<_, Option<String>>(13)?,
            "compra_id": row.get::<_, Option<i64>>(14)?,
        }))
    }).map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())?;

    // Filtrado en memoria para busqueda y estado (mas simple que armar SQL dinamico complejo)
    let rows: Vec<serde_json::Value> = rows.into_iter().filter(|r| {
        if filtro_estado != "TODOS" && r["estado"] != serde_json::Value::String(filtro_estado.clone()) {
            return false;
        }
        if !busqueda.is_empty() {
            let nombre = r["producto_nombre"].as_str().unwrap_or("").to_lowercase();
            let codigo = r["producto_codigo"].as_str().unwrap_or("").to_lowercase();
            if !nombre.contains(&busqueda) && !codigo.contains(&busqueda) {
                return false;
            }
        }
        true
    }).collect();

    let vencidos = rows.iter().filter(|r| r["estado"] == "VENCIDO").count();
    let por_vencer = rows.iter().filter(|r| r["estado"] == "POR_VENCER").count();
    let ok = rows.iter().filter(|r| r["estado"] == "OK").count();
    let total_unidades: f64 = rows.iter().filter_map(|r| r["cantidad"].as_f64()).sum();

    Ok(serde_json::json!({
        "lotes": rows,
        "vencidos": vencidos,
        "por_vencer": por_vencer,
        "ok": ok,
        "total_unidades": total_unidades,
        "dias_alerta": dias_alerta
    }))
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


// ─────────────────────────────────────────────────────────────────────────────
// v2.6.25 — Presentaciones de compra por producto (jaba x12, six-pack, etc.)
// ─────────────────────────────────────────────────────────────────────────────

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct ProductoPresentacion {
    #[serde(default)]
    pub id: Option<i64>,
    pub producto_id: i64,
    pub nombre: String,
    pub factor: f64,
    #[serde(default)]
    pub precio_costo: Option<f64>,
    #[serde(default)]
    pub codigo_barras: Option<String>,
    #[serde(default = "default_true")]
    pub activo: bool,
    #[serde(default)]
    pub orden: i64,
}

fn default_true() -> bool { true }

#[tauri::command]
pub fn listar_presentaciones_producto(
    db: tauri::State<Database>,
    producto_id: i64,
) -> Result<Vec<ProductoPresentacion>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let mut stmt = conn.prepare(
        "SELECT id, producto_id, nombre, factor, precio_costo, codigo_barras, activo, orden
         FROM producto_presentaciones
         WHERE producto_id = ?1
         ORDER BY orden ASC, id ASC"
    ).map_err(|e| e.to_string())?;

    let rows = stmt.query_map(rusqlite::params![producto_id], |r| {
        Ok(ProductoPresentacion {
            id: Some(r.get(0)?),
            producto_id: r.get(1)?,
            nombre: r.get(2)?,
            factor: r.get(3)?,
            precio_costo: r.get(4).ok(),
            codigo_barras: r.get(5).ok(),
            activo: r.get::<_, i64>(6)? != 0,
            orden: r.get(7)?,
        })
    }).map_err(|e| e.to_string())?;

    let mut out = Vec::new();
    for r in rows { out.push(r.map_err(|e| e.to_string())?); }
    Ok(out)
}

#[tauri::command]
pub fn guardar_presentaciones_producto(
    db: tauri::State<Database>,
    producto_id: i64,
    presentaciones: Vec<ProductoPresentacion>,
) -> Result<Vec<ProductoPresentacion>, String> {
    let mut conn = db.conn.lock().map_err(|e| e.to_string())?;
    let tx = conn.transaction().map_err(|e| e.to_string())?;

    // IDs que vienen en el payload — los que tenian id quedan; los que faltan se borran
    let ids_payload: Vec<i64> = presentaciones.iter().filter_map(|p| p.id).collect();

    // Borrar las presentaciones que ya no estan en el payload (UI de "eliminar")
    {
        let placeholders = if ids_payload.is_empty() {
            "(NULL)".to_string()
        } else {
            format!("({})", ids_payload.iter().map(|_| "?").collect::<Vec<_>>().join(","))
        };
        let sql = format!(
            "DELETE FROM producto_presentaciones WHERE producto_id = ? AND id NOT IN {}",
            placeholders
        );
        let mut params: Vec<rusqlite::types::Value> = Vec::new();
        params.push(producto_id.into());
        for id in &ids_payload { params.push((*id).into()); }
        tx.execute(&sql, rusqlite::params_from_iter(params.iter()))
            .map_err(|e| e.to_string())?;
    }

    // Validar: nombre no vacio, factor > 0
    for (i, p) in presentaciones.iter().enumerate() {
        if p.nombre.trim().is_empty() {
            return Err(format!("Presentacion #{}: el nombre es obligatorio", i + 1));
        }
        if p.factor <= 0.0 {
            return Err(format!("Presentacion '{}': el factor debe ser mayor a 0", p.nombre));
        }
    }

    // UPSERT por id (o INSERT si id is None)
    for (idx, p) in presentaciones.iter().enumerate() {
        let orden = if p.orden != 0 { p.orden } else { idx as i64 };
        let activo = if p.activo { 1 } else { 0 };
        match p.id {
            Some(id) => {
                tx.execute(
                    "UPDATE producto_presentaciones
                     SET nombre = ?1, factor = ?2, precio_costo = ?3,
                         codigo_barras = ?4, activo = ?5, orden = ?6
                     WHERE id = ?7 AND producto_id = ?8",
                    rusqlite::params![
                        p.nombre.trim(),
                        p.factor,
                        p.precio_costo,
                        p.codigo_barras.as_deref().map(|s| s.trim()),
                        activo,
                        orden,
                        id,
                        producto_id,
                    ],
                ).map_err(|e| e.to_string())?;
            }
            None => {
                tx.execute(
                    "INSERT INTO producto_presentaciones
                     (producto_id, nombre, factor, precio_costo, codigo_barras, activo, orden)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                    rusqlite::params![
                        producto_id,
                        p.nombre.trim(),
                        p.factor,
                        p.precio_costo,
                        p.codigo_barras.as_deref().map(|s| s.trim()),
                        activo,
                        orden,
                    ],
                ).map_err(|e| e.to_string())?;
            }
        }
    }

    tx.commit().map_err(|e| e.to_string())?;
    drop(conn);

    // Devolver la lista actualizada (incluyendo los nuevos ids)
    listar_presentaciones_producto(db, producto_id)
}

/// v2.6.30–31 — Catalogo de presentaciones unicas en uso (jaba x12, six-pack x6, ...).
/// Une TRES fuentes:
///   1. producto_presentaciones (compra-side, asignadas a productos)
///   2. unidades_producto       (venta-side, asignadas a productos, es_base=0)
///   3. tipos_unidad            (catalogo global, es_agrupada=1, factor_default>1)
/// Devuelve dedup por (LOWER(nombre), factor) ordenado por uso.
/// `usos` solo cuenta fuentes 1 y 2 — las del catalogo global tienen usos=0
/// y se muestran igual para que el usuario pueda elegirlas la primera vez.
#[derive(serde::Serialize, Debug, Clone)]
pub struct PresentacionSugerida {
    pub nombre: String,
    pub factor: f64,
    pub usos: i64,
}

#[tauri::command]
pub fn listar_presentaciones_unicas(
    db: tauri::State<Database>,
) -> Result<Vec<PresentacionSugerida>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let mut stmt = conn.prepare(
        "SELECT nombre, factor, SUM(usos_total) AS usos FROM (
             -- Fuente 1: presentaciones de compra ya asignadas a productos
             SELECT TRIM(nombre) AS nombre, factor, 1 AS usos_total
             FROM producto_presentaciones
             WHERE activo = 1 AND TRIM(nombre) != '' AND factor > 0
             UNION ALL
             -- Fuente 2: unidades de venta multi-unidad ya asignadas a productos
             SELECT TRIM(nombre) AS nombre, factor, 1 AS usos_total
             FROM unidades_producto
             WHERE COALESCE(es_base, 0) = 0
               AND COALESCE(activa, 1) = 1
               AND TRIM(nombre) != '' AND factor > 0
             UNION ALL
             -- Fuente 3: catalogo global de unidades agrupadas
             SELECT TRIM(nombre) AS nombre, factor_default AS factor, 0 AS usos_total
             FROM tipos_unidad
             WHERE COALESCE(es_agrupada, 0) = 1
               AND TRIM(nombre) != ''
               AND factor_default > 1
         )
         GROUP BY LOWER(nombre), factor
         ORDER BY usos DESC, nombre ASC
         LIMIT 100"
    ).map_err(|e| e.to_string())?;

    let rows = stmt.query_map([], |r| {
        Ok(PresentacionSugerida {
            nombre: r.get(0)?,
            factor: r.get(1)?,
            usos: r.get(2)?,
        })
    }).map_err(|e| e.to_string())?;

    let mut out = Vec::new();
    for r in rows { out.push(r.map_err(|e| e.to_string())?); }
    Ok(out)
}
