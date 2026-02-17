use crate::db::Database;
use std::io::Write;
use tauri::State;

/// BOM UTF-8 para que Excel abra correctamente caracteres especiales
const BOM: &[u8] = b"\xEF\xBB\xBF";
/// Separador de columnas (punto y coma para Excel en español)
const SEP: &str = ";";

fn escapar_csv(valor: &str) -> String {
    if valor.contains(';') || valor.contains('"') || valor.contains('\n') {
        format!("\"{}\"", valor.replace('"', "\"\""))
    } else {
        valor.to_string()
    }
}

#[tauri::command]
pub fn exportar_ventas_csv(
    db: State<Database>,
    fecha_inicio: String,
    fecha_fin: String,
    ruta: String,
) -> Result<String, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;

    let mut stmt = conn
        .prepare(
            "SELECT v.numero, v.fecha, c.nombre, v.tipo_documento, v.forma_pago,
             v.subtotal_sin_iva, v.subtotal_con_iva, v.iva, v.descuento, v.total,
             v.monto_recibido, v.cambio, v.estado_sri
             FROM ventas v
             LEFT JOIN clientes c ON v.cliente_id = c.id
             WHERE date(v.fecha) BETWEEN date(?1) AND date(?2) AND v.anulada = 0
             ORDER BY v.fecha DESC",
        )
        .map_err(|e| e.to_string())?;

    let filas: Vec<Vec<String>> = stmt
        .query_map(rusqlite::params![fecha_inicio, fecha_fin], |row| {
            Ok(vec![
                row.get::<_, String>(0).unwrap_or_default(),
                row.get::<_, String>(1).unwrap_or_default(),
                row.get::<_, String>(2).unwrap_or("CONSUMIDOR FINAL".to_string()),
                row.get::<_, String>(3).unwrap_or_default(),
                row.get::<_, String>(4).unwrap_or_default(),
                format!("{:.2}", row.get::<_, f64>(5).unwrap_or(0.0)),
                format!("{:.2}", row.get::<_, f64>(6).unwrap_or(0.0)),
                format!("{:.2}", row.get::<_, f64>(7).unwrap_or(0.0)),
                format!("{:.2}", row.get::<_, f64>(8).unwrap_or(0.0)),
                format!("{:.2}", row.get::<_, f64>(9).unwrap_or(0.0)),
                format!("{:.2}", row.get::<_, f64>(10).unwrap_or(0.0)),
                format!("{:.2}", row.get::<_, f64>(11).unwrap_or(0.0)),
                row.get::<_, String>(12).unwrap_or("NO_APLICA".to_string()),
            ])
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    let mut file = std::fs::File::create(&ruta).map_err(|e| e.to_string())?;
    file.write_all(BOM).map_err(|e| e.to_string())?;

    // Header
    let headers = [
        "Numero", "Fecha", "Cliente", "Tipo Doc", "Forma Pago",
        "Subtotal 0%", "Subtotal IVA", "IVA", "Descuento", "Total",
        "Recibido", "Cambio", "Estado SRI",
    ];
    writeln!(file, "{}", headers.join(SEP)).map_err(|e| e.to_string())?;

    for fila in &filas {
        let linea: Vec<String> = fila.iter().map(|v| escapar_csv(v)).collect();
        writeln!(file, "{}", linea.join(SEP)).map_err(|e| e.to_string())?;
    }

    Ok(format!("{} ventas exportadas", filas.len()))
}

#[tauri::command]
pub fn exportar_gastos_csv(
    db: State<Database>,
    fecha_inicio: String,
    fecha_fin: String,
    ruta: String,
) -> Result<String, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;

    let mut stmt = conn
        .prepare(
            "SELECT descripcion, monto, categoria, fecha, observacion
             FROM gastos
             WHERE date(fecha) BETWEEN date(?1) AND date(?2)
             ORDER BY fecha DESC",
        )
        .map_err(|e| e.to_string())?;

    let filas: Vec<Vec<String>> = stmt
        .query_map(rusqlite::params![fecha_inicio, fecha_fin], |row| {
            Ok(vec![
                row.get::<_, String>(0).unwrap_or_default(),
                format!("{:.2}", row.get::<_, f64>(1).unwrap_or(0.0)),
                row.get::<_, String>(2).unwrap_or_default(),
                row.get::<_, String>(3).unwrap_or_default(),
                row.get::<_, String>(4).unwrap_or_default(),
            ])
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    let mut file = std::fs::File::create(&ruta).map_err(|e| e.to_string())?;
    file.write_all(BOM).map_err(|e| e.to_string())?;

    let headers = ["Descripcion", "Monto", "Categoria", "Fecha", "Observacion"];
    writeln!(file, "{}", headers.join(SEP)).map_err(|e| e.to_string())?;

    for fila in &filas {
        let linea: Vec<String> = fila.iter().map(|v| escapar_csv(v)).collect();
        writeln!(file, "{}", linea.join(SEP)).map_err(|e| e.to_string())?;
    }

    Ok(format!("{} gastos exportados", filas.len()))
}

#[tauri::command]
pub fn exportar_inventario_csv(
    db: State<Database>,
    ruta: String,
) -> Result<String, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;

    let mut stmt = conn
        .prepare(
            "SELECT p.codigo, p.nombre, COALESCE(c.nombre, ''),
             p.precio_costo, p.precio_venta, p.iva_porcentaje,
             p.stock_actual, p.stock_minimo, p.unidad_medida,
             CASE WHEN p.es_servicio = 1 THEN 'Si' ELSE 'No' END
             FROM productos p
             LEFT JOIN categorias c ON p.categoria_id = c.id
             WHERE p.activo = 1
             ORDER BY p.nombre",
        )
        .map_err(|e| e.to_string())?;

    let filas: Vec<Vec<String>> = stmt
        .query_map([], |row| {
            Ok(vec![
                row.get::<_, String>(0).unwrap_or_default(),
                row.get::<_, String>(1).unwrap_or_default(),
                row.get::<_, String>(2).unwrap_or_default(),
                format!("{:.2}", row.get::<_, f64>(3).unwrap_or(0.0)),
                format!("{:.2}", row.get::<_, f64>(4).unwrap_or(0.0)),
                format!("{:.0}", row.get::<_, f64>(5).unwrap_or(0.0)),
                format!("{:.2}", row.get::<_, f64>(6).unwrap_or(0.0)),
                format!("{:.2}", row.get::<_, f64>(7).unwrap_or(0.0)),
                row.get::<_, String>(8).unwrap_or_default(),
                row.get::<_, String>(9).unwrap_or_default(),
            ])
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    let mut file = std::fs::File::create(&ruta).map_err(|e| e.to_string())?;
    file.write_all(BOM).map_err(|e| e.to_string())?;

    let headers = [
        "Codigo", "Nombre", "Categoria", "P. Costo", "P. Venta",
        "IVA %", "Stock Actual", "Stock Minimo", "Unidad", "Es Servicio",
    ];
    writeln!(file, "{}", headers.join(SEP)).map_err(|e| e.to_string())?;

    for fila in &filas {
        let linea: Vec<String> = fila.iter().map(|v| escapar_csv(v)).collect();
        writeln!(file, "{}", linea.join(SEP)).map_err(|e| e.to_string())?;
    }

    Ok(format!("{} productos exportados", filas.len()))
}

/// Guarda un texto en un archivo (usado para exportar XML firmado, etc.)
#[tauri::command]
pub fn guardar_archivo_texto(ruta: String, contenido: String) -> Result<(), String> {
    std::fs::write(&ruta, contenido.as_bytes())
        .map_err(|e| format!("Error guardando archivo: {}", e))
}
