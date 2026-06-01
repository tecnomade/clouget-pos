//! v2.5.70 — RIDE PDF genérico para comprobantes SRI con estructura de documento
//! "tipo factura" (emisor + receptor + tabla de líneas + total).
//!
//! Reutilizado por:
//!   - Liquidación de Compra (03): receptor = proveedor, líneas = productos
//!   - Nota de Débito (05): receptor = cliente, líneas = motivos
//!
//! Imita el layout del RIDE de retención (`ride_retencion.rs`): encabezado de
//! 2 columnas (logo+emisor / RUC+doc+autorización+barcode), recuadro del
//! receptor, tabla de líneas y total. No reimplementa la factura.

use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use genpdf::elements::{Break, LinearLayout, PaddedElement, Paragraph, StyledElement, TableLayout};
use genpdf::style::{Color, Style};
use genpdf::{Alignment, Document, Element, Margins, SimplePageDecorator};
use std::collections::HashMap;

/// Cabecera común del RIDE genérico.
pub struct DatosRideGenerico {
    pub tipo_doc_titulo: String,        // "LIQUIDACIÓN DE COMPRA" / "NOTA DE DÉBITO"
    pub numero: String,                 // "001-001-000000001"
    pub clave_acceso: String,
    pub numero_autorizacion: String,
    pub fecha_emision: String,
    pub fecha_autorizacion: String,
    pub ambiente: String,               // "1" / "2"
    pub receptor_label: String,         // "Proveedor" / "Cliente"
    pub receptor_nombre: String,
    pub receptor_identificacion: String,
    pub receptor_tipo_id: String,       // "04"=RUC, "05"=cédula...
    pub receptor_direccion: Option<String>,
    pub receptor_email: Option<String>,
    /// Línea extra opcional en el recuadro del receptor (p.ej. "Modifica: FACT 001-...").
    pub linea_extra: Option<String>,
    pub total: f64,
    pub total_label: String,            // "VALOR TOTAL" / "IMPORTE TOTAL"
}

/// Una fila de la tabla del RIDE genérico: descripción | cantidad | precio | total.
pub struct FilaRideGenerico {
    pub descripcion: String,
    pub cantidad: Option<f64>,    // None = no mostrar cantidad (p.ej. motivos de ND)
    pub precio: Option<f64>,      // None = no mostrar precio unitario
    pub valor: f64,               // total de la línea
}

fn p_aligned(text: &str, style: Style, align: Alignment) -> impl Element {
    Paragraph::new(text).aligned(align).styled(style)
}
fn format_dinero(val: f64) -> String { format!("{:.2}", val) }
fn pp(text: &str, style: Style) -> PaddedElement<StyledElement<Paragraph>> {
    Paragraph::new(text).styled(style).padded(Margins::trbl(1, 1, 1, 3))
}
fn pp_right(text: &str, style: Style) -> impl Element {
    Paragraph::new(text).aligned(Alignment::Right).styled(style).padded(Margins::trbl(1, 3, 1, 1))
}

fn tipo_id_label(cod: &str) -> &'static str {
    match cod {
        "04" => "RUC", "05" => "Cédula", "06" => "Pasaporte",
        "07" => "Consumidor Final", "08" => "Identificación del exterior",
        _ => "Identificación",
    }
}

/// Genera el RIDE (PDF A4) de un comprobante "tipo factura" (Liquidación / ND).
pub fn generar_ride_generico(
    datos: &DatosRideGenerico,
    filas: &[FilaRideGenerico],
    config: &HashMap<String, String>,
    obligado_contabilidad: bool,
) -> Result<Vec<u8>, String> {
    let fonts_dir = crate::utils::obtener_ruta_fuentes();
    let font_family = genpdf::fonts::from_files(fonts_dir.to_str().unwrap_or("fonts"), "LiberationSans", None)
        .map_err(|e| format!("Error cargando fuentes: {}", e))?;

    let mut doc = Document::new(font_family);
    doc.set_title(&format!("RIDE - {}", datos.tipo_doc_titulo));
    let mut decorator = SimplePageDecorator::new();
    decorator.set_margins(Margins::trbl(15, 15, 15, 15));
    doc.set_page_decorator(decorator);

    let s_normal = Style::new().with_font_size(9);
    let s_bold = Style::new().with_font_size(9).bold();
    let s_small = Style::new().with_font_size(8);
    let s_small_bold = Style::new().with_font_size(8).bold();
    let s_title = Style::new().with_font_size(14).bold();
    let s_doc_type = Style::new().with_font_size(13).bold();
    let s_doc_no = Style::new().with_font_size(11);
    let s_ruc = Style::new().with_font_size(10).bold();
    let s_clave_small = Style::new().with_font_size(7);
    let s_total_bold = Style::new().with_font_size(11).bold();
    let s_pie = Style::new().with_font_size(7).with_color(Color::Greyscale(128));
    let s_regimen = Style::new().with_font_size(8).bold();

    let nombre_negocio = config.get("nombre_negocio").map(|s| s.as_str()).unwrap_or("MI NEGOCIO");
    let ruc = config.get("ruc").map(|s| s.as_str()).unwrap_or("");
    let direccion_neg = config.get("direccion").map(|s| s.as_str()).unwrap_or("");
    let telefono_neg = config.get("telefono").map(|s| s.as_str()).unwrap_or("");
    let regimen = config.get("regimen").map(|s| s.as_str()).unwrap_or("");
    let ambiente_label = if datos.ambiente == "2" { "PRODUCCION" } else { "PRUEBAS" };
    let regimen_label = match regimen {
        "RIMPE_POPULAR" => "CONTRIBUYENTE NEGOCIO POPULAR - REGIMEN RIMPE",
        "RIMPE_EMPRENDEDOR" => "CONTRIBUYENTE REGIMEN RIMPE",
        "GENERAL" => "REGIMEN GENERAL",
        _ => "",
    };

    // ── SECCIÓN 1: ENCABEZADO ──
    let mut header_table = TableLayout::new(vec![1, 1]);
    header_table.set_cell_decorator(genpdf::elements::FrameCellDecorator::new(true, true, false));

    let mut col_izq = LinearLayout::vertical();
    let mut logo_element: Option<PaddedElement<genpdf::elements::Image>> = None;
    if let Some(logo_b64) = config.get("logo_negocio") {
        if !logo_b64.is_empty() {
            if let Ok(logo_bytes) = BASE64.decode(logo_b64) {
                let logo_temp = std::env::temp_dir().join("clouget_ride_gen_logo.png");
                if std::fs::write(&logo_temp, &logo_bytes).is_ok() {
                    if let Ok(mut logo_img) = genpdf::elements::Image::from_path(&logo_temp) {
                        logo_img = logo_img.with_alignment(Alignment::Center);
                        let max_width_mm = 84.0_f64;
                        let (img_w, img_h) = if logo_bytes.len() > 24 && &logo_bytes[0..4] == b"\x89PNG" {
                            let w = u32::from_be_bytes([logo_bytes[16], logo_bytes[17], logo_bytes[18], logo_bytes[19]]) as f64;
                            let h = u32::from_be_bytes([logo_bytes[20], logo_bytes[21], logo_bytes[22], logo_bytes[23]]) as f64;
                            (w, h)
                        } else { (200.0, 100.0) };
                        let max_height_mm = 35.0_f64;
                        let scale_by_w = (max_width_mm * 300.0) / (25.4 * img_w);
                        let rendered_h = 25.4 * (scale_by_w * img_h) / 300.0;
                        let final_scale = if rendered_h > max_height_mm {
                            (max_height_mm * 300.0) / (25.4 * img_h)
                        } else { scale_by_w };
                        logo_img = logo_img.with_scale(genpdf::Scale::new(final_scale, final_scale));
                        logo_element = Some(logo_img.padded(Margins::trbl(1, 0, 1, 0)));
                    }
                    let _ = std::fs::remove_file(&logo_temp);
                }
            }
        }
    }
    if let Some(logo) = logo_element { col_izq.push(logo); } else { col_izq.push(Break::new(8.0)); }

    let mut datos_emisor = LinearLayout::vertical();
    datos_emisor.push(Break::new(0.3));
    datos_emisor.push(pp(nombre_negocio, s_title));
    datos_emisor.push(Break::new(0.3));
    if !direccion_neg.is_empty() {
        datos_emisor.push(pp(&format!("Dirección Matriz: {}", direccion_neg), s_normal));
    }
    if !telefono_neg.is_empty() {
        datos_emisor.push(pp(&format!("Tel: {}", telefono_neg), s_normal));
    }
    datos_emisor.push(Break::new(0.3));
    datos_emisor.push(pp(
        &format!("OBLIGADO A LLEVAR CONTABILIDAD: {}", if obligado_contabilidad { "SI" } else { "NO" }),
        s_bold,
    ));
    if !regimen_label.is_empty() {
        datos_emisor.push(Break::new(0.2));
        datos_emisor.push(pp(regimen_label, s_regimen));
    }
    datos_emisor.push(Break::new(0.5));
    col_izq.push(datos_emisor);

    let mut col_der = LinearLayout::vertical();
    col_der.push(Break::new(0.3));
    col_der.push(pp(&format!("R.U.C.:  {}", ruc), s_ruc));
    col_der.push(Break::new(0.3));
    col_der.push(pp(&datos.tipo_doc_titulo, s_doc_type));
    col_der.push(pp(&format!("No. {}", datos.numero), s_doc_no));
    col_der.push(Break::new(0.3));
    col_der.push(pp("NÚMERO DE AUTORIZACIÓN", s_bold));
    col_der.push(pp(&datos.numero_autorizacion, s_clave_small));
    col_der.push(Break::new(0.3));
    col_der.push(pp("FECHA Y HORA DE AUTORIZACIÓN", s_bold));
    col_der.push(pp(&datos.fecha_autorizacion, s_normal));
    col_der.push(Break::new(0.3));
    col_der.push(pp(&format!("AMBIENTE:    {}", ambiente_label), s_normal));
    col_der.push(pp("EMISIÓN:     NORMAL", s_normal));
    col_der.push(Break::new(0.3));
    col_der.push(pp("CLAVE DE ACCESO:", s_bold));
    col_der.push(Break::new(0.3));
    if !datos.clave_acceso.is_empty() {
        match crate::sri::ride::generar_barcode128_image(&datos.clave_acceso) {
            Ok(barcode_path) => {
                if let Ok(mut barcode_img) = genpdf::elements::Image::from_path(&barcode_path) {
                    barcode_img = barcode_img.with_alignment(Alignment::Center);
                    barcode_img = barcode_img.with_scale(genpdf::Scale::new(1.8, 2.0));
                    col_der.push(barcode_img);
                }
                let _ = std::fs::remove_file(&barcode_path);
            }
            Err(e) => eprintln!("Warning: barcode Code128 (RIDE genérico): {}", e),
        }
    }
    col_der.push(Break::new(0.3));
    col_der.push(p_aligned(&datos.clave_acceso, s_clave_small, Alignment::Center));
    col_der.push(Break::new(0.3));

    header_table.row()
        .element(col_izq.padded(Margins::trbl(2, 3, 2, 3)))
        .element(col_der.padded(Margins::trbl(2, 3, 2, 3)))
        .push()
        .map_err(|e| format!("Error header: {}", e))?;
    doc.push(header_table);
    doc.push(Break::new(1.0));

    // ── SECCIÓN 2: RECEPTOR ──
    let mut receptor = LinearLayout::vertical();
    let mut fila1 = TableLayout::new(vec![3, 2]);
    fila1.row()
        .element(pp(&format!("{}: {}", datos.receptor_label, datos.receptor_nombre), s_normal))
        .element(pp(&format!("{}: {}", tipo_id_label(&datos.receptor_tipo_id), datos.receptor_identificacion), s_normal))
        .push().map_err(|e| format!("Error receptor 1: {}", e))?;
    receptor.push(fila1);

    let mut fila2 = TableLayout::new(vec![3, 2]);
    let dir_text = match datos.receptor_direccion.as_deref() {
        Some(d) if !d.is_empty() => format!("Dirección: {}", d),
        _ => String::new(),
    };
    fila2.row()
        .element(pp(&format!("Fecha de Emisión: {}", datos.fecha_emision), s_bold))
        .element(pp(&dir_text, s_normal))
        .push().map_err(|e| format!("Error receptor 2: {}", e))?;
    receptor.push(fila2);

    if let Some(ref extra) = datos.linea_extra {
        if !extra.is_empty() {
            let mut fila3 = TableLayout::new(vec![1]);
            fila3.row().element(pp(extra, s_bold)).push().map_err(|e| format!("Error receptor 3: {}", e))?;
            receptor.push(fila3);
        }
    }
    doc.push(receptor.padded(Margins::trbl(3, 2, 3, 2)).framed());
    doc.push(Break::new(1.0));

    // ── SECCIÓN 3: TABLA DE LÍNEAS ──
    let mut table = TableLayout::new(vec![8, 2, 3, 3]);
    table.set_cell_decorator(genpdf::elements::FrameCellDecorator::new(true, true, false));
    table.row()
        .element(pp("Descripción", s_small_bold))
        .element(pp_right("Cant.", s_small_bold))
        .element(pp_right("P. Unit.", s_small_bold))
        .element(pp_right("Valor", s_small_bold))
        .push().map_err(|e| format!("Error tabla header: {}", e))?;

    for f in filas {
        table.row()
            .element(pp(&f.descripcion, s_small))
            .element(pp_right(&f.cantidad.map(|c| if c == c.floor() { format!("{:.0}", c) } else { format!("{:.2}", c) }).unwrap_or_default(), s_small))
            .element(pp_right(&f.precio.map(format_dinero).unwrap_or_default(), s_small))
            .element(pp_right(&format_dinero(f.valor), s_small))
            .push().map_err(|e| format!("Error tabla fila: {}", e))?;
    }
    doc.push(table);
    doc.push(Break::new(0.8));

    // ── SECCIÓN 4: TOTAL ──
    let mut total_table = TableLayout::new(vec![10, 4]);
    total_table.set_cell_decorator(genpdf::elements::FrameCellDecorator::new(true, true, false));
    total_table.row()
        .element(pp_right(&datos.total_label, s_total_bold))
        .element(pp_right(&format!("$ {}", format_dinero(datos.total)), s_total_bold))
        .push().map_err(|e| format!("Error total: {}", e))?;
    doc.push(total_table);
    doc.push(Break::new(1.0));

    // ── SECCIÓN 5: INFO ADICIONAL ──
    if let Some(email) = datos.receptor_email.as_deref() {
        if !email.is_empty() {
            let mut info = LinearLayout::vertical();
            info.push(pp("INFORMACIÓN ADICIONAL", s_bold));
            info.push(pp(&format!("Email: {}", email), s_normal));
            doc.push(info.padded(Margins::trbl(3, 2, 3, 2)).framed());
            doc.push(Break::new(0.5));
        }
    }

    // ── SECCIÓN 6: PIE ──
    doc.push(Break::new(1.0));
    doc.push(p_aligned("Este documento es una Representación Impresa de un Comprobante Electrónico (RIDE)", s_pie, Alignment::Center));
    doc.push(p_aligned("Conforme a la normativa del Servicio de Rentas Internas - SRI Ecuador", s_pie, Alignment::Center));
    doc.push(p_aligned(&format!("Generado por Clouget POS — Ambiente: {}", ambiente_label), s_pie, Alignment::Center));

    let mut buffer = Vec::new();
    doc.render(&mut buffer).map_err(|e| format!("Error renderizando PDF: {}", e))?;
    Ok(buffer)
}
