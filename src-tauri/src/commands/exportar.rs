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

// ============================================================================
// EXPORT INVENTARIO VALORIZADO: XLSX y PDF (v2.1.0)
// ============================================================================

/// Fila de inventario para export (estructura plana para evitar serde_json en rust)
#[derive(Debug)]
struct InventarioRow {
    codigo: String,
    nombre: String,
    categoria: String,
    stock_actual: f64,
    stock_minimo: f64,
    precio_costo: f64,
    precio_venta: f64,
    valor_costo: f64,
    valor_venta: f64,
    utilidad: f64,
    estado: String,
}

fn obtener_inventario_filtrado(
    db: &State<Database>,
    categoria_nombre: Option<String>,
    busqueda: Option<String>,
    estado_filtro: Option<String>,
) -> Result<(Vec<InventarioRow>, f64, f64), String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;

    let mut sql = String::from(
        "SELECT COALESCE(p.codigo, ''), p.nombre, COALESCE(c.nombre, ''),
                p.stock_actual, p.stock_minimo,
                p.precio_costo, p.precio_venta,
                (p.stock_actual * p.precio_costo) as valor_costo,
                (p.stock_actual * p.precio_venta) as valor_venta,
                CASE
                    WHEN p.stock_actual <= 0 THEN 'SIN_STOCK'
                    WHEN p.stock_actual <= p.stock_minimo THEN 'BAJO'
                    ELSE 'OK'
                END as estado_stock
         FROM productos p
         LEFT JOIN categorias c ON p.categoria_id = c.id
         WHERE p.activo = 1 AND COALESCE(p.es_servicio, 0) = 0 AND COALESCE(p.no_controla_stock, 0) = 0"
    );

    let mut params: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();
    if let Some(cat) = &categoria_nombre {
        if cat != "TODAS" && !cat.is_empty() {
            sql.push_str(" AND c.nombre = ?1");
            params.push(Box::new(cat.clone()));
        }
    }
    sql.push_str(" ORDER BY p.nombre");

    let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;
    let params_refs: Vec<&dyn rusqlite::ToSql> = params.iter().map(|b| b.as_ref()).collect();

    let mut total_costo = 0.0_f64;
    let mut total_venta = 0.0_f64;
    let filas: Vec<InventarioRow> = stmt.query_map(params_refs.as_slice(), |row| {
        let v_costo: f64 = row.get(7)?;
        let v_venta: f64 = row.get(8)?;
        Ok(InventarioRow {
            codigo: row.get(0)?,
            nombre: row.get(1)?,
            categoria: row.get(2)?,
            stock_actual: row.get(3)?,
            stock_minimo: row.get(4)?,
            precio_costo: row.get(5)?,
            precio_venta: row.get(6)?,
            valor_costo: v_costo,
            valor_venta: v_venta,
            utilidad: v_venta - v_costo,
            estado: row.get(9)?,
        })
    }).map_err(|e| e.to_string())?
    .filter_map(|r| r.ok())
    .filter(|r: &InventarioRow| {
        if let Some(f) = &estado_filtro {
            if f != "TODOS" && f != &r.estado { return false; }
        }
        true
    })
    .filter(|r| {
        if let Some(b) = &busqueda {
            if !b.is_empty() {
                let b_low = b.to_lowercase();
                return r.nombre.to_lowercase().contains(&b_low) ||
                       r.codigo.to_lowercase().contains(&b_low);
            }
        }
        true
    })
    .collect();

    for f in &filas {
        total_costo += f.valor_costo;
        total_venta += f.valor_venta;
    }
    Ok((filas, total_costo, total_venta))
}

/// Exporta el reporte de inventario valorizado a XLSX con filtros aplicados
#[tauri::command]
pub fn exportar_inventario_xlsx(
    db: State<Database>,
    ruta: String,
    categoria_nombre: Option<String>,
    busqueda: Option<String>,
    estado_filtro: Option<String>,
) -> Result<(), String> {
    use rust_xlsxwriter::{Workbook, Format, Color};

    let (filas, total_costo, total_venta) = obtener_inventario_filtrado(
        &db, categoria_nombre.clone(), busqueda.clone(), estado_filtro.clone()
    )?;

    let mut workbook = Workbook::new();
    let worksheet = workbook.add_worksheet();
    worksheet.set_name("Inventario").map_err(|e| e.to_string())?;

    // Formatos
    let fmt_title = Format::new().set_bold().set_font_size(14).set_background_color(Color::RGB(0x3B82F6)).set_font_color(Color::White);
    let fmt_header = Format::new().set_bold().set_background_color(Color::RGB(0xE5E7EB)).set_border(rust_xlsxwriter::FormatBorder::Thin);
    let fmt_money = Format::new().set_num_format("$#,##0.00");
    let fmt_num = Format::new().set_num_format("#,##0.00");
    let fmt_bold = Format::new().set_bold();
    let fmt_bold_money = Format::new().set_bold().set_num_format("$#,##0.00").set_background_color(Color::RGB(0xDBEAFE));
    let fmt_sin_stock = Format::new().set_font_color(Color::Red).set_bold();
    let fmt_bajo = Format::new().set_font_color(Color::RGB(0xD97706));

    // Titulo
    worksheet.set_row_height(0, 24).ok();
    worksheet.merge_range(0, 0, 0, 10, "INVENTARIO VALORIZADO", &fmt_title).ok();

    // Info filtros
    let mut row = 1_u32;
    let fecha = chrono::Local::now().format("%d/%m/%Y %H:%M").to_string();
    worksheet.write_string(row, 0, format!("Generado: {}", fecha)).ok();
    row += 1;
    if let Some(c) = &categoria_nombre {
        if c != "TODAS" && !c.is_empty() {
            worksheet.write_string(row, 0, format!("Categoria: {}", c)).ok();
            row += 1;
        }
    }
    if let Some(e) = &estado_filtro {
        if e != "TODOS" {
            worksheet.write_string(row, 0, format!("Estado: {}", e)).ok();
            row += 1;
        }
    }
    row += 1;

    // Cabecera
    let headers = ["Codigo", "Producto", "Categoria", "Stock Actual", "Stock Minimo",
        "Precio Costo", "Precio Venta", "Valor Costo", "Valor Venta", "Utilidad Pot.", "Estado"];
    for (col, h) in headers.iter().enumerate() {
        worksheet.write_string_with_format(row, col as u16, *h, &fmt_header).ok();
    }
    row += 1;

    // Datos
    for f in &filas {
        worksheet.write_string(row, 0, &f.codigo).ok();
        worksheet.write_string(row, 1, &f.nombre).ok();
        worksheet.write_string(row, 2, &f.categoria).ok();
        worksheet.write_number_with_format(row, 3, f.stock_actual, &fmt_num).ok();
        worksheet.write_number_with_format(row, 4, f.stock_minimo, &fmt_num).ok();
        worksheet.write_number_with_format(row, 5, f.precio_costo, &fmt_money).ok();
        worksheet.write_number_with_format(row, 6, f.precio_venta, &fmt_money).ok();
        worksheet.write_number_with_format(row, 7, f.valor_costo, &fmt_money).ok();
        worksheet.write_number_with_format(row, 8, f.valor_venta, &fmt_money).ok();
        worksheet.write_number_with_format(row, 9, f.utilidad, &fmt_money).ok();
        let estado_label = match f.estado.as_str() {
            "SIN_STOCK" => "Sin stock",
            "BAJO" => "Bajo",
            _ => "OK",
        };
        let fmt_estado = match f.estado.as_str() {
            "SIN_STOCK" => &fmt_sin_stock,
            "BAJO" => &fmt_bajo,
            _ => &fmt_bold,
        };
        worksheet.write_string_with_format(row, 10, estado_label, fmt_estado).ok();
        row += 1;
    }

    // Totales
    row += 1;
    worksheet.write_string_with_format(row, 6, "TOTAL:", &fmt_bold).ok();
    worksheet.write_number_with_format(row, 7, total_costo, &fmt_bold_money).ok();
    worksheet.write_number_with_format(row, 8, total_venta, &fmt_bold_money).ok();
    worksheet.write_number_with_format(row, 9, total_venta - total_costo, &fmt_bold_money).ok();

    // Ajustar ancho de columnas
    worksheet.set_column_width(0, 12).ok();
    worksheet.set_column_width(1, 30).ok();
    worksheet.set_column_width(2, 18).ok();
    worksheet.set_column_width(3, 11).ok();
    worksheet.set_column_width(4, 11).ok();
    worksheet.set_column_width(5, 12).ok();
    worksheet.set_column_width(6, 12).ok();
    worksheet.set_column_width(7, 12).ok();
    worksheet.set_column_width(8, 12).ok();
    worksheet.set_column_width(9, 12).ok();
    worksheet.set_column_width(10, 12).ok();

    workbook.save(&ruta).map_err(|e| format!("Error guardando XLSX: {}", e))?;
    Ok(())
}

/// Exporta el reporte de inventario valorizado a PDF con filtros aplicados
#[tauri::command]
pub fn exportar_inventario_pdf(
    db: State<Database>,
    ruta: String,
    categoria_nombre: Option<String>,
    busqueda: Option<String>,
    estado_filtro: Option<String>,
) -> Result<(), String> {
    use genpdf::elements::{Break, Paragraph, TableLayout, FrameCellDecorator};
    use genpdf::style::{Style, Color};
    use genpdf::{Alignment, Element, Margins, SimplePageDecorator};

    let (filas, total_costo, total_venta) = obtener_inventario_filtrado(
        &db, categoria_nombre.clone(), busqueda.clone(), estado_filtro.clone()
    )?;

    let fonts_dir = crate::utils::obtener_ruta_fuentes();
    let font_family = genpdf::fonts::from_files(
        fonts_dir.to_str().unwrap_or("fonts"),
        "LiberationSans",
        None,
    ).map_err(|e| format!("Error cargando fuentes: {}", e))?;

    let mut doc = genpdf::Document::new(font_family);
    doc.set_title("Inventario Valorizado");
    doc.set_paper_size(genpdf::Size::new(297, 210)); // A4 landscape
    let mut decorator = SimplePageDecorator::new();
    decorator.set_margins(Margins::trbl(10, 10, 10, 10));
    doc.set_page_decorator(decorator);

    let s_title = Style::new().with_font_size(16).bold();
    let s_subtitle = Style::new().with_font_size(10).with_color(Color::Greyscale(100));
    let s_header = Style::new().with_font_size(9).bold();
    let s_cell = Style::new().with_font_size(8);
    let s_total = Style::new().with_font_size(10).bold();

    // Titulo
    doc.push(Paragraph::new("INVENTARIO VALORIZADO").aligned(Alignment::Center).styled(s_title));
    let fecha = chrono::Local::now().format("%d/%m/%Y %H:%M").to_string();
    doc.push(Paragraph::new(&format!("Generado: {}", fecha)).aligned(Alignment::Center).styled(s_subtitle));

    // Info filtros
    let mut filtros_txt = Vec::new();
    if let Some(c) = &categoria_nombre {
        if c != "TODAS" && !c.is_empty() {
            filtros_txt.push(format!("Categoria: {}", c));
        }
    }
    if let Some(e) = &estado_filtro {
        if e != "TODOS" {
            filtros_txt.push(format!("Estado: {}", e));
        }
    }
    if let Some(b) = &busqueda {
        if !b.is_empty() {
            filtros_txt.push(format!("Busqueda: {}", b));
        }
    }
    if !filtros_txt.is_empty() {
        doc.push(Paragraph::new(&format!("Filtros: {}", filtros_txt.join(" | "))).aligned(Alignment::Center).styled(s_subtitle));
    }
    doc.push(Break::new(1.0));

    // Tabla
    let mut table = TableLayout::new(vec![1, 4, 2, 1, 1, 2, 2, 2, 2]);
    table.set_cell_decorator(FrameCellDecorator::new(true, true, false));
    table.row()
        .element(Paragraph::new("Codigo").styled(s_header).padded(Margins::trbl(1, 2, 1, 2)))
        .element(Paragraph::new("Producto").styled(s_header).padded(Margins::trbl(1, 2, 1, 2)))
        .element(Paragraph::new("Categoria").styled(s_header).padded(Margins::trbl(1, 2, 1, 2)))
        .element(Paragraph::new("Stock").aligned(Alignment::Right).styled(s_header).padded(Margins::trbl(1, 2, 1, 2)))
        .element(Paragraph::new("Min").aligned(Alignment::Right).styled(s_header).padded(Margins::trbl(1, 2, 1, 2)))
        .element(Paragraph::new("P.Costo").aligned(Alignment::Right).styled(s_header).padded(Margins::trbl(1, 2, 1, 2)))
        .element(Paragraph::new("P.Venta").aligned(Alignment::Right).styled(s_header).padded(Margins::trbl(1, 2, 1, 2)))
        .element(Paragraph::new("V.Costo").aligned(Alignment::Right).styled(s_header).padded(Margins::trbl(1, 2, 1, 2)))
        .element(Paragraph::new("V.Venta").aligned(Alignment::Right).styled(s_header).padded(Margins::trbl(1, 2, 1, 2)))
        .push().map_err(|e| format!("Error tabla: {}", e))?;

    for f in &filas {
        table.row()
            .element(Paragraph::new(&f.codigo).styled(s_cell).padded(Margins::trbl(1, 2, 1, 2)))
            .element(Paragraph::new(&f.nombre).styled(s_cell).padded(Margins::trbl(1, 2, 1, 2)))
            .element(Paragraph::new(&f.categoria).styled(s_cell).padded(Margins::trbl(1, 2, 1, 2)))
            .element(Paragraph::new(&format!("{:.2}", f.stock_actual)).aligned(Alignment::Right).styled(s_cell).padded(Margins::trbl(1, 2, 1, 2)))
            .element(Paragraph::new(&format!("{:.2}", f.stock_minimo)).aligned(Alignment::Right).styled(s_cell).padded(Margins::trbl(1, 2, 1, 2)))
            .element(Paragraph::new(&format!("${:.2}", f.precio_costo)).aligned(Alignment::Right).styled(s_cell).padded(Margins::trbl(1, 2, 1, 2)))
            .element(Paragraph::new(&format!("${:.2}", f.precio_venta)).aligned(Alignment::Right).styled(s_cell).padded(Margins::trbl(1, 2, 1, 2)))
            .element(Paragraph::new(&format!("${:.2}", f.valor_costo)).aligned(Alignment::Right).styled(s_cell).padded(Margins::trbl(1, 2, 1, 2)))
            .element(Paragraph::new(&format!("${:.2}", f.valor_venta)).aligned(Alignment::Right).styled(s_cell).padded(Margins::trbl(1, 2, 1, 2)))
            .push().map_err(|e| format!("Error fila: {}", e))?;
    }

    doc.push(table);
    doc.push(Break::new(1.0));

    // Totales
    doc.push(Paragraph::new(&format!(
        "TOTAL AL COSTO: ${:.2}     TOTAL AL PRECIO VENTA: ${:.2}     UTILIDAD POTENCIAL: ${:.2}",
        total_costo, total_venta, total_venta - total_costo
    )).aligned(Alignment::Right).styled(s_total));

    doc.render_to_file(&ruta).map_err(|e| format!("Error PDF: {}", e))?;
    Ok(())
}

// ============================================================================
// EXPORT GENERICO XLSX/PDF: cualquier reporte pasa titulo + headers + filas
// ============================================================================

#[tauri::command]
pub fn exportar_tabla_xlsx(
    ruta: String,
    titulo: String,
    subtitulo: Option<String>,
    encabezados: Vec<String>,
    filas: Vec<Vec<String>>,
    columnas_numericas: Option<Vec<usize>>,
) -> Result<(), String> {
    use rust_xlsxwriter::{Workbook, Format, Color};

    let mut workbook = Workbook::new();
    let worksheet = workbook.add_worksheet();
    worksheet.set_name("Reporte").map_err(|e| e.to_string())?;

    let fmt_title = Format::new().set_bold().set_font_size(14)
        .set_background_color(Color::RGB(0x3B82F6)).set_font_color(Color::White);
    let fmt_subtitle = Format::new().set_font_size(10).set_italic();
    let fmt_header = Format::new().set_bold()
        .set_background_color(Color::RGB(0xE5E7EB))
        .set_border(rust_xlsxwriter::FormatBorder::Thin);
    let fmt_money = Format::new().set_num_format("$#,##0.00");

    // Titulo
    let max_col = encabezados.len().max(1) as u16 - 1;
    worksheet.set_row_height(0, 24).ok();
    worksheet.merge_range(0, 0, 0, max_col, &titulo, &fmt_title).ok();

    let mut row = 1_u32;
    if let Some(sub) = &subtitulo {
        worksheet.write_string_with_format(row, 0, sub.as_str(), &fmt_subtitle).ok();
        row += 1;
    }
    let fecha = chrono::Local::now().format("%d/%m/%Y %H:%M").to_string();
    worksheet.write_string(row, 0, format!("Generado: {}", fecha)).ok();
    row += 2;

    // Cabecera
    for (col, h) in encabezados.iter().enumerate() {
        worksheet.write_string_with_format(row, col as u16, h.as_str(), &fmt_header).ok();
    }
    row += 1;

    let columnas_num: Vec<usize> = columnas_numericas.unwrap_or_default();

    // Datos
    for fila in &filas {
        for (col, val) in fila.iter().enumerate() {
            if columnas_num.contains(&col) {
                // Intentar parsear como numero
                let clean = val.replace('$', "").replace(',', "").replace('%', "").trim().to_string();
                if let Ok(n) = clean.parse::<f64>() {
                    worksheet.write_number_with_format(row, col as u16, n, &fmt_money).ok();
                    continue;
                }
            }
            worksheet.write_string(row, col as u16, val.as_str()).ok();
        }
        row += 1;
    }

    // Ajustar anchos: primer columna amplia, resto normal
    worksheet.set_column_width(0, 20).ok();
    for col in 1..encabezados.len() as u16 {
        worksheet.set_column_width(col, 15).ok();
    }

    workbook.save(&ruta).map_err(|e| format!("Error guardando XLSX: {}", e))?;
    Ok(())
}

#[tauri::command]
pub fn exportar_tabla_pdf(
    ruta: String,
    titulo: String,
    subtitulo: Option<String>,
    encabezados: Vec<String>,
    filas: Vec<Vec<String>>,
    orientacion_horizontal: Option<bool>,
) -> Result<(), String> {
    use genpdf::elements::{Break, Paragraph, TableLayout, FrameCellDecorator};
    use genpdf::style::{Style, Color};
    use genpdf::{Alignment, Element, Margins, SimplePageDecorator};

    let fonts_dir = crate::utils::obtener_ruta_fuentes();
    let font_family = genpdf::fonts::from_files(
        fonts_dir.to_str().unwrap_or("fonts"),
        "LiberationSans",
        None,
    ).map_err(|e| format!("Error fuentes: {}", e))?;

    let mut doc = genpdf::Document::new(font_family);
    doc.set_title(&titulo);
    let horizontal = orientacion_horizontal.unwrap_or(true);
    if horizontal {
        doc.set_paper_size(genpdf::Size::new(297, 210));
    } else {
        doc.set_paper_size(genpdf::Size::new(210, 297));
    }
    let mut decorator = SimplePageDecorator::new();
    decorator.set_margins(Margins::trbl(10, 10, 10, 10));
    doc.set_page_decorator(decorator);

    let s_title = Style::new().with_font_size(16).bold();
    let s_subtitle = Style::new().with_font_size(10).with_color(Color::Greyscale(100));
    let s_header = Style::new().with_font_size(9).bold();
    let s_cell = Style::new().with_font_size(8);

    doc.push(Paragraph::new(&titulo).aligned(Alignment::Center).styled(s_title));
    if let Some(sub) = &subtitulo {
        doc.push(Paragraph::new(sub.as_str()).aligned(Alignment::Center).styled(s_subtitle));
    }
    let fecha = chrono::Local::now().format("%d/%m/%Y %H:%M").to_string();
    doc.push(Paragraph::new(&format!("Generado: {}", fecha)).aligned(Alignment::Center).styled(s_subtitle));
    doc.push(Break::new(1.0));

    // Tabla con columnas iguales
    let n_cols = encabezados.len();
    let widths: Vec<usize> = (0..n_cols).map(|i| if i == 0 || i == 1 { 2 } else { 1 }).collect();
    let mut table = TableLayout::new(widths);
    table.set_cell_decorator(FrameCellDecorator::new(true, true, false));

    // Header
    let mut row = table.row();
    for h in &encabezados {
        row = row.element(Paragraph::new(h.as_str()).styled(s_header).padded(Margins::trbl(1, 2, 1, 2)));
    }
    row.push().map_err(|e| format!("Error header: {}", e))?;

    // Datos
    for fila in &filas {
        let mut r = table.row();
        for val in fila {
            r = r.element(Paragraph::new(val.as_str()).styled(s_cell).padded(Margins::trbl(1, 2, 1, 2)));
        }
        r.push().map_err(|e| format!("Error fila: {}", e))?;
    }

    doc.push(table);
    doc.render_to_file(&ruta).map_err(|e| format!("Error PDF: {}", e))?;
    Ok(())
}
