use crate::db::Database;
use tauri::State;
use genpdf::elements::{Break, LinearLayout, Paragraph, TableLayout};
use genpdf::style::Style;
use genpdf::{Alignment, Element, Margins, SimplePageDecorator};

#[derive(Debug, serde::Deserialize)]
pub struct EtiquetaConfig {
    pub producto_ids: Vec<i64>,
    pub cantidad_por_producto: i64,
    pub columnas: i64,
    pub mostrar_precio: bool,
    pub mostrar_codigo: bool,
    #[serde(default)]
    pub lista_precio_id: Option<i64>,
    #[serde(default)]
    pub preset: Option<String>,
    #[serde(default)]
    pub ancho_mm: Option<f64>,
    #[serde(default)]
    pub alto_mm: Option<f64>,
    #[serde(default)]
    pub margen_top_mm: Option<f64>,
    #[serde(default)]
    pub margen_left_mm: Option<f64>,
}

struct ProductoEtiqueta {
    nombre: String,
    codigo: String,
    codigo_barras: Option<String>,
    precio: f64,
}

struct CeldaData {
    barcode_path: Option<String>,
    barcode_text: String,
    nombre: String,
    precio: f64,
    mostrar_precio: bool,
    mostrar_codigo: bool,
    cols: usize,
}

struct PresetInfo {
    page_width_mm: f64,
    page_height_mm: f64,
    columnas: Option<usize>,
    margen_top: f64,
    margen_left: f64,
}

fn resolver_preset(config: &EtiquetaConfig) -> PresetInfo {
    let preset = config.preset.as_deref().unwrap_or("a4");
    let mt = config.margen_top_mm.unwrap_or(5.0);
    let ml = config.margen_left_mm.unwrap_or(5.0);

    match preset {
        "zebra_50x25" => PresetInfo { page_width_mm: 50.0, page_height_mm: 25.0, columnas: Some(1), margen_top: mt.min(3.0), margen_left: ml.min(3.0) },
        "zebra_50x30" => PresetInfo { page_width_mm: 50.0, page_height_mm: 30.0, columnas: Some(1), margen_top: mt.min(3.0), margen_left: ml.min(3.0) },
        "zebra_100x50" => PresetInfo { page_width_mm: 100.0, page_height_mm: 50.0, columnas: Some(1), margen_top: mt.min(3.0), margen_left: ml.min(3.0) },
        "zebra_100x150" => PresetInfo { page_width_mm: 100.0, page_height_mm: 150.0, columnas: Some(1), margen_top: mt.min(5.0), margen_left: ml.min(5.0) },
        "rollo_80" => PresetInfo { page_width_mm: 80.0, page_height_mm: 297.0, columnas: None, margen_top: mt, margen_left: ml },
        "avery_65" => PresetInfo { page_width_mm: 210.0, page_height_mm: 297.0, columnas: Some(5), margen_top: mt, margen_left: ml },
        "avery_24" => PresetInfo { page_width_mm: 210.0, page_height_mm: 297.0, columnas: Some(3), margen_top: mt, margen_left: ml },
        "personalizado" => PresetInfo {
            page_width_mm: config.ancho_mm.unwrap_or(210.0),
            page_height_mm: config.alto_mm.unwrap_or(297.0),
            columnas: None,
            margen_top: mt,
            margen_left: ml,
        },
        _ => PresetInfo { page_width_mm: 210.0, page_height_mm: 297.0, columnas: None, margen_top: mt, margen_left: ml }, // a4
    }
}

#[tauri::command]
pub fn generar_etiquetas_pdf(
    db: State<Database>,
    config: EtiquetaConfig,
) -> Result<String, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;

    let mut productos: Vec<ProductoEtiqueta> = Vec::new();
    for pid in &config.producto_ids {
        let mut prod = conn.query_row(
            "SELECT nombre, codigo, codigo_barras, precio_venta FROM productos WHERE id = ?1",
            rusqlite::params![pid],
            |row| {
                Ok(ProductoEtiqueta {
                    nombre: row.get(0)?,
                    codigo: row.get::<_, Option<String>>(1)?.unwrap_or_default(),
                    codigo_barras: row.get(2)?,
                    precio: row.get(3)?,
                })
            },
        ).map_err(|e| format!("Producto {} no encontrado: {}", pid, e))?;

        // Si hay lista de precios, buscar el precio específico
        if let Some(lista_id) = config.lista_precio_id {
            if let Ok(precio_lista) = conn.query_row(
                "SELECT precio FROM precios_producto WHERE producto_id = ?1 AND lista_precio_id = ?2",
                rusqlite::params![pid, lista_id],
                |row| row.get::<_, f64>(0),
            ) {
                prod.precio = precio_lista;
            }
            // Si no existe precio en esa lista, mantiene precio_venta como fallback
        }

        productos.push(prod);
    }

    let preset_info = resolver_preset(&config);
    let cols = preset_info.columnas.unwrap_or(config.columnas.max(1).min(6) as usize);

    let font_family = genpdf::fonts::from_files("C:\\Windows\\Fonts", "arial", None)
        .unwrap_or_else(|_| {
            genpdf::fonts::from_files("C:\\Windows\\Fonts", "calibri", None)
                .unwrap_or_else(|_| genpdf::fonts::from_files("C:\\Windows\\Fonts", "consola", None).unwrap())
        });

    let mut doc = genpdf::Document::new(font_family);
    let mut decorator = SimplePageDecorator::new();
    decorator.set_margins(Margins::trbl(
        preset_info.margen_top as f32,
        preset_info.margen_left as f32,
        preset_info.margen_top as f32,
        preset_info.margen_left as f32,
    ));
    doc.set_paper_size(genpdf::Size::new(
        preset_info.page_width_mm as f32,
        preset_info.page_height_mm as f32,
    ));
    doc.set_page_decorator(decorator);

    // Calcular ancho disponible por celda para escalar dinámicamente
    let usable_width = preset_info.page_width_mm - 2.0 * preset_info.margen_left;
    let cell_width_mm = usable_width / cols as f64;

    // Pre-generate all cell data
    let mut all_cells: Vec<CeldaData> = Vec::new();
    for prod in &productos {
        for _ in 0..config.cantidad_por_producto {
            let barcode_data = prod.codigo_barras.as_deref()
                .filter(|s| !s.is_empty())
                .unwrap_or(&prod.codigo);

            let barcode_path = if !barcode_data.is_empty() {
                generar_barcode_etiqueta(barcode_data).ok()
            } else {
                None
            };

            // Nombre: truncar según ancho disponible
            let max_chars = if cell_width_mm < 30.0 { 15 } else if cell_width_mm < 50.0 { 22 } else { 30 };
            let nombre_corto = if prod.nombre.len() > max_chars {
                format!("{}...", &prod.nombre[..max_chars.saturating_sub(3)])
            } else {
                prod.nombre.clone()
            };

            all_cells.push(CeldaData {
                barcode_path,
                barcode_text: barcode_data.to_string(),
                nombre: nombre_corto,
                precio: prod.precio,
                mostrar_precio: config.mostrar_precio,
                mostrar_codigo: config.mostrar_codigo,
                cols,
            });
        }
    }

    // Build table row by row
    let col_widths: Vec<usize> = vec![1; cols];
    let mut table = TableLayout::new(col_widths);

    let total_cells = all_cells.len();
    let total_rows = (total_cells + cols - 1) / cols;

    for row_idx in 0..total_rows {
        let mut row = table.row();
        for col in 0..cols {
            let cell_idx = row_idx * cols + col;
            if cell_idx < total_cells {
                let cell = &all_cells[cell_idx];
                let mut content = LinearLayout::vertical();

                // Barcode image - escala dinámica según ancho de celda
                if let Some(ref bp) = cell.barcode_path {
                    if let Ok(img) = genpdf::elements::Image::from_path(bp) {
                        let scale = calcular_escala_barcode(cell_width_mm);
                        content.push(
                            img.with_scale(genpdf::Scale::new(scale.0 as f32, scale.1 as f32))
                                .with_alignment(Alignment::Center),
                        );
                    }
                }

                // Code text
                if cell.mostrar_codigo && !cell.barcode_text.is_empty() {
                    let code_size = calcular_font_size(cell_width_mm, "code");
                    content.push(Paragraph::new(&cell.barcode_text).styled(Style::new().with_font_size(code_size)));
                }

                // Product name
                let nombre_size = calcular_font_size(cell_width_mm, "nombre");
                content.push(Paragraph::new(&cell.nombre).styled(Style::new().bold().with_font_size(nombre_size)));

                // Price
                if cell.mostrar_precio {
                    let precio_size = calcular_font_size(cell_width_mm, "precio");
                    content.push(Paragraph::new(format!("${:.2}", cell.precio)).styled(Style::new().bold().with_font_size(precio_size)));
                }

                let padding = if cell_width_mm < 30.0 { 1.0 } else if cell_width_mm < 50.0 { 2.0 } else { 3.0 };
                content.push(Break::new(0.3));
                row = row.element(content.padded(Margins::trbl(padding as f32, padding as f32, padding as f32, padding as f32)));
            } else {
                row = row.element(Paragraph::new(""));
            }
        }
        row.push().map_err(|e| format!("Error fila: {}", e))?;
    }

    // Cleanup barcode temp files
    for cell in &all_cells {
        if let Some(ref bp) = cell.barcode_path {
            let _ = std::fs::remove_file(bp);
        }
    }

    doc.push(table);

    // Save to Desktop
    let userprofile = std::env::var("USERPROFILE").unwrap_or_else(|_| ".".to_string());
    let desktop = std::path::PathBuf::from(&userprofile).join("Desktop");
    let filename = format!("Etiquetas_{}.pdf", chrono::Local::now().format("%Y%m%d_%H%M%S"));
    let path = desktop.join(&filename);

    doc.render_to_file(&path)
        .map_err(|e| format!("Error generando PDF: {}", e))?;

    // Open PDF
    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("cmd")
            .args(["/C", "start", "", &path.to_string_lossy()])
            .spawn()
            .ok();
    }

    Ok(path.to_string_lossy().to_string())
}

/// Calcula escala del barcode según el ancho disponible de la celda en mm
fn calcular_escala_barcode(cell_width_mm: f64) -> (f64, f64) {
    if cell_width_mm < 25.0 {
        (0.6, 0.5)       // Zebra 50mm / 2 cols o muy pequeño
    } else if cell_width_mm < 40.0 {
        (1.0, 0.7)       // Zebra single col ~46mm
    } else if cell_width_mm < 55.0 {
        (1.3, 0.8)       // 4 cols A4
    } else if cell_width_mm < 70.0 {
        (1.8, 1.0)       // 3 cols A4
    } else if cell_width_mm < 100.0 {
        (2.5, 1.2)       // 2 cols A4 o Zebra 100mm
    } else {
        (3.0, 1.5)       // 1 col A4
    }
}

/// Calcula el tamaño de fuente según el ancho de celda y tipo de texto
fn calcular_font_size(cell_width_mm: f64, tipo: &str) -> u8 {
    match tipo {
        "code" => {
            if cell_width_mm < 25.0 { 4 }
            else if cell_width_mm < 40.0 { 5 }
            else if cell_width_mm < 55.0 { 6 }
            else { 7 }
        }
        "nombre" => {
            if cell_width_mm < 25.0 { 5 }
            else if cell_width_mm < 40.0 { 6 }
            else if cell_width_mm < 55.0 { 7 }
            else if cell_width_mm < 70.0 { 8 }
            else if cell_width_mm < 100.0 { 9 }
            else { 10 }
        }
        "precio" => {
            if cell_width_mm < 25.0 { 6 }
            else if cell_width_mm < 40.0 { 7 }
            else if cell_width_mm < 55.0 { 8 }
            else if cell_width_mm < 70.0 { 9 }
            else if cell_width_mm < 100.0 { 10 }
            else { 12 }
        }
        _ => 7,
    }
}

fn generar_barcode_etiqueta(data: &str) -> Result<String, String> {
    use barcoders::sym::code128::Code128;

    let data_c = format!("\u{0106}{}", data);
    let barcode = Code128::new(&data_c)
        .or_else(|_| {
            let data_b = format!("\u{0181}{}", data);
            Code128::new(&data_b)
        })
        .map_err(|e| format!("Error Code128: {}", e))?;
    let encoded: Vec<u8> = barcode.encode();

    let height = 60_u32;
    let scale_x = 2_u32;
    let quiet_zone = 10_u32;
    let width = (encoded.len() as u32) * scale_x + quiet_zone * 2;

    let mut img_buf = vec![255u8; (width * height) as usize];

    for (i, &bar) in encoded.iter().enumerate() {
        if bar == 1 {
            for x_off in 0..scale_x {
                let px = quiet_zone + (i as u32) * scale_x + x_off;
                for y in 0..height {
                    let idx = (y * width + px) as usize;
                    if idx < img_buf.len() {
                        img_buf[idx] = 0;
                    }
                }
            }
        }
    }

    let gray_img = image::GrayImage::from_raw(width, height, img_buf)
        .ok_or("Error creando imagen barcode")?;

    let tmp = std::env::temp_dir().join(format!("barcode_etq_{}.png", uuid::Uuid::new_v4()));
    gray_img.save(&tmp).map_err(|e| format!("Error guardando barcode: {}", e))?;

    Ok(tmp.to_string_lossy().to_string())
}
