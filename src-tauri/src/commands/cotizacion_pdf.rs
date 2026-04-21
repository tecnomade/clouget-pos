use crate::db::Database;
use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use genpdf::elements::{Break, LinearLayout, PaddedElement, Paragraph, StyledElement, TableLayout};
use genpdf::style::{Color, Style};
use genpdf::{Alignment, Document, Element, Margins, SimplePageDecorator};
use tauri::State;

// ============================================
// HELPERS
// ============================================

fn p(text: &str, style: Style) -> StyledElement<Paragraph> {
    Paragraph::new(text).styled(style)
}

fn p_aligned(text: &str, style: Style, align: Alignment) -> impl Element {
    Paragraph::new(text).aligned(align).styled(style)
}

fn format_cantidad(cant: f64) -> String {
    if cant == cant.floor() {
        format!("{:.0}", cant)
    } else {
        format!("{:.2}", cant)
    }
}

fn format_dinero(val: f64) -> String {
    format!("{:.2}", val)
}

/// Paragraph con padding (izquierdo 3mm) para celdas de tabla
fn pp(text: &str, style: Style) -> PaddedElement<StyledElement<Paragraph>> {
    Paragraph::new(text)
        .styled(style)
        .padded(Margins::trbl(1, 1, 1, 3))
}

/// Paragraph alineado a la derecha con padding
fn pp_right(text: &str, style: Style) -> impl Element {
    Paragraph::new(text)
        .aligned(Alignment::Right)
        .styled(style)
        .padded(Margins::trbl(1, 3, 1, 1))
}

/// Paragraph con padding + alineado al centro
fn pp_center(text: &str, style: Style) -> impl Element {
    Paragraph::new(text)
        .aligned(Alignment::Center)
        .styled(style)
        .padded(Margins::trbl(1, 1, 1, 1))
}

// ============================================
// COMMAND: generar_cotizacion_pdf
// ============================================

#[tauri::command]
pub fn generar_cotizacion_pdf(db: State<Database>, venta_id: i64) -> Result<String, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;

    // --- Obtener la venta (cotización) ---
    let venta = conn
        .query_row(
            "SELECT id, numero, cliente_id, fecha, subtotal_sin_iva, subtotal_con_iva,
             descuento, iva, total, forma_pago, monto_recibido, cambio, estado,
             tipo_documento, observacion, establecimiento, punto_emision
             FROM ventas WHERE id = ?1",
            rusqlite::params![venta_id],
            |row| {
                Ok(CotizacionVenta {
                    id: row.get(0)?,
                    numero: row.get(1)?,
                    cliente_id: row.get(2)?,
                    fecha: row.get::<_, Option<String>>(3)?.unwrap_or_default(),
                    subtotal_sin_iva: row.get(4)?,
                    subtotal_con_iva: row.get(5)?,
                    descuento: row.get(6)?,
                    iva: row.get(7)?,
                    total: row.get(8)?,
                    observacion: row.get::<_, Option<String>>(14)?,
                })
            },
        )
        .map_err(|e| format!("Cotizacion no encontrada: {}", e))?;

    // --- Obtener detalles con código de producto e info_adicional ---
    let mut stmt = conn
        .prepare(
            "SELECT d.producto_id, p.nombre, d.cantidad, d.precio_unitario,
             d.descuento, d.iva_porcentaje, d.subtotal,
             COALESCE(p.codigo, CAST(d.producto_id AS TEXT)) as codigo,
             d.info_adicional
             FROM venta_detalles d
             JOIN productos p ON d.producto_id = p.id
             WHERE d.venta_id = ?1",
        )
        .map_err(|e| e.to_string())?;

    let detalles: Vec<CotizacionDetalle> = stmt
        .query_map(rusqlite::params![venta_id], |row| {
            Ok(CotizacionDetalle {
                codigo: row.get(7)?,
                nombre: row.get(1)?,
                cantidad: row.get(2)?,
                precio_unitario: row.get(3)?,
                descuento: row.get(4)?,
                iva_porcentaje: row.get(5)?,
                subtotal: row.get(6)?,
                info_adicional: row.get(8)?,
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    // --- Obtener datos del cliente ---
    let cliente = venta
        .cliente_id
        .and_then(|cid| {
            conn.query_row(
                "SELECT nombre, COALESCE(identificacion,''), COALESCE(direccion,''),
                 COALESCE(telefono,''), COALESCE(email,'')
                 FROM clientes WHERE id = ?1",
                rusqlite::params![cid],
                |row| {
                    Ok(CotizacionCliente {
                        nombre: row.get(0)?,
                        identificacion: row.get(1)?,
                        direccion: row.get(2)?,
                        telefono: row.get(3)?,
                        email: row.get(4)?,
                    })
                },
            )
            .ok()
        })
        .unwrap_or(CotizacionCliente {
            nombre: "CONSUMIDOR FINAL".to_string(),
            identificacion: "9999999999999".to_string(),
            direccion: String::new(),
            telefono: String::new(),
            email: String::new(),
        });

    // --- Obtener config ---
    let mut cfg_stmt = conn
        .prepare("SELECT key, value FROM config")
        .map_err(|e| e.to_string())?;
    let config: std::collections::HashMap<String, String> = cfg_stmt
        .query_map([], |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)))
        .map_err(|e| e.to_string())?
        .collect::<Result<std::collections::HashMap<_, _>, _>>()
        .map_err(|e| e.to_string())?;

    drop(stmt);
    drop(cfg_stmt);
    drop(conn);

    // --- Generar PDF ---
    let pdf_bytes =
        generar_pdf_cotizacion(&venta, &detalles, &cliente, &config)?;

    // Guardar en temp
    let temp_dir = std::env::temp_dir();
    let filename = format!(
        "Cotizacion-{}.pdf",
        venta.numero.replace(['/', '\\', ':'], "-")
    );
    let pdf_path = temp_dir.join(&filename);
    std::fs::write(&pdf_path, &pdf_bytes)
        .map_err(|e| format!("Error guardando PDF: {}", e))?;

    // Abrir con visor del sistema
    #[cfg(target_os = "windows")]
    {
        crate::utils::silent_command("cmd")
            .args(["/C", "start", "", &pdf_path.to_string_lossy()])
            .spawn()
            .map_err(|e| format!("Error abriendo PDF: {}", e))?;
    }

    #[cfg(not(target_os = "windows"))]
    {
        std::process::Command::new("xdg-open")
            .arg(&pdf_path.to_string_lossy().to_string())
            .spawn()
            .map_err(|e| format!("Error abriendo PDF: {}", e))?;
    }

    Ok(pdf_path.to_string_lossy().to_string())
}

// ============================================
// STRUCTS INTERNOS
// ============================================

struct CotizacionVenta {
    id: i64,
    numero: String,
    cliente_id: Option<i64>,
    fecha: String,
    subtotal_sin_iva: f64,
    subtotal_con_iva: f64,
    descuento: f64,
    iva: f64,
    total: f64,
    observacion: Option<String>,
}

struct CotizacionDetalle {
    codigo: String,
    nombre: String,
    cantidad: f64,
    precio_unitario: f64,
    descuento: f64,
    iva_porcentaje: f64,
    subtotal: f64,
    info_adicional: Option<String>,
}

struct CotizacionCliente {
    nombre: String,
    identificacion: String,
    direccion: String,
    telefono: String,
    email: String,
}

// ============================================
// PDF GENERATION
// ============================================

fn generar_pdf_cotizacion(
    venta: &CotizacionVenta,
    detalles: &[CotizacionDetalle],
    cliente: &CotizacionCliente,
    config: &std::collections::HashMap<String, String>,
) -> Result<Vec<u8>, String> {
    let fonts_dir = crate::utils::obtener_ruta_fuentes();

    let font_family = genpdf::fonts::from_files(
        fonts_dir.to_str().unwrap_or("fonts"),
        "LiberationSans",
        None,
    )
    .map_err(|e| {
        format!(
            "Error cargando fuentes: {}. Asegurese de que los archivos LiberationSans-*.ttf estan en src-tauri/fonts/",
            e
        )
    })?;

    let mut doc = Document::new(font_family);
    doc.set_title("Cotizacion");

    let mut decorator = SimplePageDecorator::new();
    decorator.set_margins(Margins::trbl(15, 15, 15, 15));
    doc.set_page_decorator(decorator);

    // Estilos
    let s_normal = Style::new().with_font_size(9);
    let s_bold = Style::new().with_font_size(9).bold();
    let s_small = Style::new().with_font_size(8);
    let s_small_bold = Style::new().with_font_size(8).bold();
    let s_title = Style::new().with_font_size(14).bold();
    let s_doc_type = Style::new().with_font_size(18).bold();
    let s_doc_no = Style::new().with_font_size(11);
    let s_ruc = Style::new().with_font_size(10).bold();
    let s_total_bold = Style::new().with_font_size(11).bold();
    let s_pie = Style::new().with_font_size(7).with_color(Color::Greyscale(128));
    let s_note = Style::new().with_font_size(8).italic();
    let s_info_adicional = Style::new().with_font_size(7).with_color(Color::Greyscale(100));
    let s_section_title = Style::new().with_font_size(9).bold().with_color(Color::Greyscale(60));

    // --- Datos del config ---
    let nombre_negocio = config
        .get("nombre_negocio")
        .map(|s| s.as_str())
        .unwrap_or("MI NEGOCIO");
    let ruc = config.get("ruc").map(|s| s.as_str()).unwrap_or("");
    let direccion_neg = config.get("direccion").map(|s| s.as_str()).unwrap_or("");
    let telefono_neg = config.get("telefono").map(|s| s.as_str()).unwrap_or("");

    // ===================================================================
    // SECCION 1: ENCABEZADO (2 columnas)
    // ===================================================================
    let mut header_table = TableLayout::new(vec![1, 1]);
    header_table.set_cell_decorator(genpdf::elements::FrameCellDecorator::new(true, true, false));

    // --- Columna izquierda: Logo + datos del negocio ---
    let mut col_izq = LinearLayout::vertical();

    // Logo (misma lógica que ride.rs)
    let mut logo_element: Option<PaddedElement<genpdf::elements::Image>> = None;
    if let Some(logo_b64) = config.get("logo_negocio") {
        if !logo_b64.is_empty() {
            if let Ok(logo_bytes) = BASE64.decode(logo_b64) {
                let logo_temp = std::env::temp_dir().join("clouget_cotizacion_logo.png");
                if std::fs::write(&logo_temp, &logo_bytes).is_ok() {
                    if let Ok(mut logo_img) = genpdf::elements::Image::from_path(&logo_temp) {
                        logo_img = logo_img.with_alignment(Alignment::Center);
                        let max_width_mm = 84.0_f64;
                        let (img_w, img_h) =
                            if logo_bytes.len() > 24 && &logo_bytes[0..4] == b"\x89PNG" {
                                let w = u32::from_be_bytes([
                                    logo_bytes[16],
                                    logo_bytes[17],
                                    logo_bytes[18],
                                    logo_bytes[19],
                                ]) as f64;
                                let h = u32::from_be_bytes([
                                    logo_bytes[20],
                                    logo_bytes[21],
                                    logo_bytes[22],
                                    logo_bytes[23],
                                ]) as f64;
                                (w, h)
                            } else {
                                (200.0, 100.0)
                            };
                        let max_height_mm = 35.0_f64;
                        let scale_by_w = (max_width_mm * 300.0) / (25.4 * img_w);
                        let rendered_h = 25.4 * (scale_by_w * img_h) / 300.0;
                        let final_scale = if rendered_h > max_height_mm {
                            (max_height_mm * 300.0) / (25.4 * img_h)
                        } else {
                            scale_by_w
                        };
                        logo_img =
                            logo_img.with_scale(genpdf::Scale::new(final_scale, final_scale));
                        logo_element = Some(logo_img.padded(Margins::trbl(1, 0, 1, 0)));
                    }
                    let _ = std::fs::remove_file(&logo_temp);
                }
            }
        }
    }

    if let Some(logo) = logo_element {
        col_izq.push(logo);
    } else {
        col_izq.push(Break::new(8.0));
    }

    // Datos del negocio
    let mut datos_emisor = LinearLayout::vertical();
    datos_emisor.push(Break::new(0.3));
    datos_emisor.push(pp(nombre_negocio, s_title));
    datos_emisor.push(Break::new(0.3));
    if !ruc.is_empty() {
        datos_emisor.push(pp(&format!("RUC: {}", ruc), s_ruc));
        datos_emisor.push(Break::new(0.2));
    }
    if !direccion_neg.is_empty() {
        datos_emisor.push(pp(&format!("Direccion: {}", direccion_neg), s_normal));
    }
    if !telefono_neg.is_empty() {
        datos_emisor.push(pp(&format!("Telefono: {}", telefono_neg), s_normal));
    }
    datos_emisor.push(Break::new(0.5));
    col_izq.push(datos_emisor);

    // --- Columna derecha: Título COTIZACIÓN + número + fecha + validez ---
    let mut col_der = LinearLayout::vertical();
    col_der.push(Break::new(2.0));
    col_der.push(pp_center("COTIZACION", s_doc_type));
    col_der.push(Break::new(1.0));
    col_der.push(pp(&format!("No. {}", venta.numero), s_doc_no));
    col_der.push(Break::new(0.5));
    col_der.push(pp(&format!("Fecha: {}", venta.fecha), s_bold));
    col_der.push(Break::new(0.3));
    col_der.push(pp("Validez: 30 dias", s_bold));
    col_der.push(Break::new(2.0));

    header_table
        .row()
        .element(col_izq.padded(Margins::trbl(2, 3, 2, 3)))
        .element(col_der.padded(Margins::trbl(2, 3, 2, 3)))
        .push()
        .map_err(|e| format!("Error tabla header: {}", e))?;

    doc.push(header_table);
    doc.push(Break::new(1.5));

    // ===================================================================
    // SECCION 2: DATOS DEL CLIENTE
    // ===================================================================
    let mut cliente_section = LinearLayout::vertical();
    cliente_section.push(Break::new(0.3));
    cliente_section.push(pp("DATOS DEL CLIENTE", s_section_title));
    cliente_section.push(Break::new(0.5));

    // Fila 1: Nombre + Identificación
    let tipo_id_label = if cliente.identificacion == "9999999999999" {
        "Consumidor Final"
    } else if cliente.identificacion.len() == 13 {
        "RUC"
    } else if cliente.identificacion.len() == 10 {
        "Cedula"
    } else {
        "Identificacion"
    };

    let mut fila1 = TableLayout::new(vec![3, 2]);
    fila1
        .row()
        .element(pp(
            &format!("Cliente: {}", cliente.nombre),
            s_bold,
        ))
        .element(pp(
            &format!("{}: {}", tipo_id_label, cliente.identificacion),
            s_normal,
        ))
        .push()
        .map_err(|e| format!("Error fila cliente 1: {}", e))?;
    cliente_section.push(fila1);

    // Fila 2: Dirección + Teléfono
    if !cliente.direccion.is_empty() || !cliente.telefono.is_empty() {
        let mut fila2 = TableLayout::new(vec![3, 2]);
        fila2
            .row()
            .element(pp(
                &if !cliente.direccion.is_empty() {
                    format!("Direccion: {}", cliente.direccion)
                } else {
                    String::new()
                },
                s_normal,
            ))
            .element(pp(
                &if !cliente.telefono.is_empty() {
                    format!("Telefono: {}", cliente.telefono)
                } else {
                    String::new()
                },
                s_normal,
            ))
            .push()
            .map_err(|e| format!("Error fila cliente 2: {}", e))?;
        cliente_section.push(fila2);
    }

    // Fila 3: Email
    if !cliente.email.is_empty() {
        cliente_section.push(pp(&format!("Email: {}", cliente.email), s_normal));
    }

    cliente_section.push(Break::new(0.3));
    doc.push(
        cliente_section
            .padded(Margins::trbl(3, 2, 3, 2))
            .framed(),
    );
    doc.push(Break::new(1.5));

    // ===================================================================
    // SECCION 3: TABLA DE PRODUCTOS
    // ===================================================================
    // Columnas: #, Código, Descripción, Cantidad, P.Unitario, Subtotal
    let mut table = TableLayout::new(vec![1, 2, 7, 2, 2, 2]);
    table.set_cell_decorator(genpdf::elements::FrameCellDecorator::new(true, true, false));

    // Header
    table
        .row()
        .element(pp("#", s_small_bold))
        .element(pp("Codigo", s_small_bold))
        .element(pp("Descripcion", s_small_bold))
        .element(pp_right("Cant.", s_small_bold))
        .element(pp_right("P.Unitario", s_small_bold))
        .element(pp_right("Subtotal", s_small_bold))
        .push()
        .map_err(|e| format!("Error tabla header: {}", e))?;

    // Filas de productos
    for (i, det) in detalles.iter().enumerate() {
        let subtotal_item = det.cantidad * det.precio_unitario - det.descuento;
        table
            .row()
            .element(pp(&format!("{}", i + 1), s_small))
            .element(pp(&det.codigo, s_small))
            .element(pp(&det.nombre, s_small))
            .element(pp_right(&format_cantidad(det.cantidad), s_small))
            .element(pp_right(&format_dinero(det.precio_unitario), s_small))
            .element(pp_right(&format_dinero(subtotal_item), s_small))
            .push()
            .map_err(|e| format!("Error tabla fila: {}", e))?;

        // Info adicional debajo del nombre si existe
        if let Some(ref info) = det.info_adicional {
            if !info.is_empty() {
                table
                    .row()
                    .element(pp("", s_small))
                    .element(pp("", s_small))
                    .element(pp(&format!("  {}", info), s_info_adicional))
                    .element(pp("", s_small))
                    .element(pp("", s_small))
                    .element(pp("", s_small))
                    .push()
                    .map_err(|e| format!("Error tabla info adicional: {}", e))?;
            }
        }
    }

    doc.push(table);
    doc.push(Break::new(1.5));

    // ===================================================================
    // SECCION 4: TOTALES (alineados a la derecha)
    // ===================================================================
    let mut bottom_table = TableLayout::new(vec![10, 6]);

    // Columna izquierda: vacía (espacio)
    let spacer = LinearLayout::vertical();

    // Columna derecha: totales
    let mut totales_col = LinearLayout::vertical();

    let mut totales_table = TableLayout::new(vec![4, 2]);
    totales_table.set_cell_decorator(genpdf::elements::FrameCellDecorator::new(true, true, false));

    // Subtotal 0%
    totales_table
        .row()
        .element(pp("SUBTOTAL 0%", s_small))
        .element(pp_right(&format_dinero(venta.subtotal_sin_iva), s_small))
        .push()
        .map_err(|e| format!("Error totales: {}", e))?;

    // Subtotal IVA
    totales_table
        .row()
        .element(pp("SUBTOTAL IVA", s_small))
        .element(pp_right(&format_dinero(venta.subtotal_con_iva), s_small))
        .push()
        .map_err(|e| format!("Error totales: {}", e))?;

    // Descuento (si hay)
    if venta.descuento > 0.0 {
        totales_table
            .row()
            .element(pp("DESCUENTO", s_small))
            .element(pp_right(&format_dinero(venta.descuento), s_small))
            .push()
            .map_err(|e| format!("Error totales: {}", e))?;
    }

    // IVA 15%
    totales_table
        .row()
        .element(pp("IVA 15%", s_small))
        .element(pp_right(&format_dinero(venta.iva), s_small))
        .push()
        .map_err(|e| format!("Error totales: {}", e))?;

    // TOTAL
    totales_table
        .row()
        .element(pp("TOTAL", s_total_bold))
        .element(pp_right(&format!("${}", format_dinero(venta.total)), s_total_bold))
        .push()
        .map_err(|e| format!("Error totales: {}", e))?;

    totales_col.push(totales_table);

    bottom_table
        .row()
        .element(spacer)
        .element(totales_col)
        .push()
        .map_err(|e| format!("Error bottom table: {}", e))?;

    doc.push(bottom_table);
    doc.push(Break::new(2.0));

    // ===================================================================
    // SECCION 5: NOTAS / OBSERVACIONES
    // ===================================================================
    let mut notas_section = LinearLayout::vertical();
    notas_section.push(pp("NOTAS", s_section_title));
    notas_section.push(Break::new(0.5));
    notas_section.push(pp(
        "Esta cotizacion tiene una validez de 30 dias a partir de la fecha de emision.",
        s_note,
    ));
    notas_section.push(Break::new(0.3));
    notas_section.push(pp(
        "Los precios incluyen IVA donde aplique. Sujeto a disponibilidad de stock.",
        s_note,
    ));

    // Observación de la venta si existe
    if let Some(ref obs) = venta.observacion {
        if !obs.is_empty() {
            notas_section.push(Break::new(0.5));
            notas_section.push(pp(&format!("Observacion: {}", obs), s_note));
        }
    }

    notas_section.push(Break::new(0.3));
    doc.push(
        notas_section
            .padded(Margins::trbl(3, 3, 3, 3))
            .framed(),
    );
    doc.push(Break::new(3.0));

    // ===================================================================
    // SECCION 6: PIE DE PÁGINA
    // ===================================================================
    let mut footer = LinearLayout::vertical();
    let footer_text = if !telefono_neg.is_empty() {
        format!("{} | Tel: {}", nombre_negocio, telefono_neg)
    } else {
        nombre_negocio.to_string()
    };
    footer.push(p_aligned(&footer_text, s_pie, Alignment::Center));
    footer.push(Break::new(0.3));
    footer.push(p_aligned(
        "Generado por Clouget POS",
        s_pie,
        Alignment::Center,
    ));
    doc.push(footer);

    // Renderizar
    let mut buf = Vec::new();
    doc.render(&mut buf)
        .map_err(|e| format!("Error renderizando PDF: {}", e))?;

    Ok(buf)
}
