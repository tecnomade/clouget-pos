//! v2.5.47 — RIDE PDF del Comprobante de Retención SRI Ecuador.
//!
//! Genera el PDF tamaño A4 conforme a la ficha técnica del SRI:
//! - Encabezado con logo + datos del agente + número/autorización/clave/barcode
//! - Datos del sujeto retenido (proveedor)
//! - Tabla de impuestos retenidos (RENTA + IVA) con código, base, %, valor, doc sustento
//! - Total retenido
//! - Información adicional
//!
//! Reutiliza el patrón de `ride.rs` (helpers `pp`, `pp_right`, barcoders, genpdf).

use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use genpdf::elements::{Break, LinearLayout, PaddedElement, Paragraph, StyledElement, TableLayout};
use genpdf::style::{Style, Color};
use genpdf::{Alignment, Document, Element, Margins, SimplePageDecorator};
use std::collections::HashMap;

/// Datos de la cabecera de la retención para el RIDE.
pub struct DatosRetencionRide {
    pub numero: String,                 // "001-001-000000001"
    pub clave_acceso: String,           // 49 dígitos
    pub numero_autorizacion: String,    // viene del SRI (puede ser igual a clave_acceso)
    pub fecha_emision: String,          // "dd/mm/yyyy"
    pub fecha_autorizacion: String,
    pub ambiente: String,               // "1" o "2"
    pub periodo_fiscal: String,         // "MM/YYYY"
    pub sujeto_nombre: String,
    pub sujeto_identificacion: String,
    pub sujeto_tipo_id: String,         // "04"=RUC, "05"=cedula, "06"=pasaporte
    pub sujeto_direccion: Option<String>,
    pub sujeto_email: Option<String>,
    pub total_retenido: f64,
}

/// Una línea de retención dentro del comprobante.
pub struct ItemRetencionRide {
    pub tipo_label: String,           // "RENTA" o "IVA"
    pub codigo_retencion: String,     // ej. "304"
    pub base_imponible: f64,
    pub porcentaje: f64,
    pub valor_retenido: f64,
    pub cod_doc_sustento: String,     // "01"=factura
    pub num_doc_sustento: String,     // 15 dígitos
    pub fecha_doc_sustento: String,   // dd/mm/yyyy
}

// ============================================
// HELPERS (mismos patrones que ride.rs)
// ============================================

fn p_aligned(text: &str, style: Style, align: Alignment) -> impl Element {
    Paragraph::new(text).aligned(align).styled(style)
}

fn format_dinero(val: f64) -> String { format!("{:.2}", val) }
fn format_pct(val: f64) -> String {
    if val == val.floor() { format!("{:.0}%", val) } else { format!("{:.2}%", val) }
}

fn pp(text: &str, style: Style) -> PaddedElement<StyledElement<Paragraph>> {
    Paragraph::new(text).styled(style).padded(Margins::trbl(1, 1, 1, 3))
}

fn pp_right(text: &str, style: Style) -> impl Element {
    Paragraph::new(text).aligned(Alignment::Right).styled(style).padded(Margins::trbl(1, 3, 1, 1))
}

fn pp_center(text: &str, style: Style) -> impl Element {
    Paragraph::new(text).aligned(Alignment::Center).styled(style).padded(Margins::trbl(1, 1, 1, 1))
}

// ============================================
// LABEL maps
// ============================================

fn tipo_id_label(cod: &str) -> &'static str {
    match cod {
        "04" => "RUC",
        "05" => "Cédula",
        "06" => "Pasaporte",
        "07" => "Consumidor Final",
        "08" => "Identificación del exterior",
        _ => "Identificación",
    }
}

fn cod_doc_sustento_label(cod: &str) -> &'static str {
    match cod {
        "01" => "Factura",
        "02" => "Nota de Venta",
        "03" => "Liquidación de compra",
        "04" => "Nota de Crédito",
        "05" => "Nota de Débito",
        "11" => "Pasajes",
        "12" => "Comprobante de venta emitido por máquinas registradoras",
        "20" => "Estado de cuenta",
        "21" => "Carta de porte aéreo",
        "47" => "Nota de Crédito - liquidación de impuesto",
        "48" => "Nota de Débito - liquidación de impuesto",
        _ => "Documento",
    }
}

// ============================================
// GENERADOR RIDE PDF DE RETENCIÓN
// ============================================

/// Genera el RIDE (PDF A4) del comprobante de retención conforme ficha SRI Ecuador.
///
/// `config` debe contener: `nombre_negocio`, `ruc`, `direccion`, `telefono`,
/// `regimen`, `sri_ambiente`, `logo_negocio` (b64 opcional).
/// `contabilidad_obligado` y `contabilidad_resolucion` se pasan aparte porque
/// viven en `contabilidad_config`, no en `config`.
#[allow(clippy::too_many_arguments)]
pub fn generar_ride_retencion_pdf(
    datos: &DatosRetencionRide,
    items: &[ItemRetencionRide],
    config: &HashMap<String, String>,
    contabilidad_obligado: bool,
    contabilidad_resolucion: Option<&str>,
) -> Result<Vec<u8>, String> {
    let fonts_dir = crate::utils::obtener_ruta_fuentes();
    let font_family = genpdf::fonts::from_files(
        fonts_dir.to_str().unwrap_or("fonts"),
        "LiberationSans",
        None,
    )
    .map_err(|e| format!("Error cargando fuentes: {}", e))?;

    let mut doc = Document::new(font_family);
    doc.set_title("RIDE - Comprobante de Retencion");

    let mut decorator = SimplePageDecorator::new();
    decorator.set_margins(Margins::trbl(15, 15, 15, 15));
    doc.set_page_decorator(decorator);

    // Estilos
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

    // Datos del agente (config global)
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

    // ===================================================================
    // SECCIÓN 1: ENCABEZADO (table 2 col con bordes alineados)
    // ===================================================================
    let mut header_table = TableLayout::new(vec![1, 1]);
    header_table.set_cell_decorator(genpdf::elements::FrameCellDecorator::new(true, true, false));

    // --- Columna izquierda: Logo + Datos emisor ---
    let mut col_izq = LinearLayout::vertical();

    let mut logo_element: Option<PaddedElement<genpdf::elements::Image>> = None;
    if let Some(logo_b64) = config.get("logo_negocio") {
        if !logo_b64.is_empty() {
            if let Ok(logo_bytes) = BASE64.decode(logo_b64) {
                let logo_temp = std::env::temp_dir().join("clouget_ride_ret_logo.png");
                if std::fs::write(&logo_temp, &logo_bytes).is_ok() {
                    if let Ok(mut logo_img) = genpdf::elements::Image::from_path(&logo_temp) {
                        logo_img = logo_img.with_alignment(Alignment::Center);
                        let max_width_mm = 84.0_f64;
                        let (img_w, img_h) = if logo_bytes.len() > 24 && &logo_bytes[0..4] == b"\x89PNG" {
                            let w = u32::from_be_bytes([logo_bytes[16], logo_bytes[17], logo_bytes[18], logo_bytes[19]]) as f64;
                            let h = u32::from_be_bytes([logo_bytes[20], logo_bytes[21], logo_bytes[22], logo_bytes[23]]) as f64;
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
                        logo_img = logo_img.with_scale(genpdf::Scale::new(final_scale, final_scale));
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

    let mut datos_emisor = LinearLayout::vertical();
    datos_emisor.push(Break::new(0.3));
    datos_emisor.push(pp(nombre_negocio, s_title));
    datos_emisor.push(Break::new(0.3));
    if !direccion_neg.is_empty() {
        datos_emisor.push(pp(&format!("Dirección Matriz: {}", direccion_neg), s_normal));
        datos_emisor.push(pp(&format!("Dirección Sucursal: {}", direccion_neg), s_normal));
    }
    if !telefono_neg.is_empty() {
        datos_emisor.push(pp(&format!("Tel: {}", telefono_neg), s_normal));
    }
    datos_emisor.push(Break::new(0.3));
    datos_emisor.push(pp(
        &format!("OBLIGADO A LLEVAR CONTABILIDAD: {}", if contabilidad_obligado { "SI" } else { "NO" }),
        s_bold,
    ));
    if let Some(res) = contabilidad_resolucion {
        if !res.is_empty() {
            datos_emisor.push(pp(&format!("AGENTE DE RETENCIÓN Res. No.: {}", res), s_bold));
        }
    }
    if !regimen_label.is_empty() {
        datos_emisor.push(Break::new(0.2));
        datos_emisor.push(pp(regimen_label, s_regimen));
    }
    datos_emisor.push(Break::new(0.5));
    col_izq.push(datos_emisor);

    // --- Columna derecha ---
    let mut col_der = LinearLayout::vertical();
    col_der.push(Break::new(0.3));
    col_der.push(pp(&format!("R.U.C.:  {}", ruc), s_ruc));
    col_der.push(Break::new(0.3));
    col_der.push(pp("COMPROBANTE DE RETENCIÓN", s_doc_type));
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
            Err(e) => {
                eprintln!("Warning: No se pudo generar barcode Code128 (retención): {}", e);
            }
        }
    }
    col_der.push(Break::new(0.3));
    col_der.push(p_aligned(&datos.clave_acceso, s_clave_small, Alignment::Center));
    col_der.push(Break::new(0.3));

    header_table
        .row()
        .element(col_izq.padded(Margins::trbl(2, 3, 2, 3)))
        .element(col_der.padded(Margins::trbl(2, 3, 2, 3)))
        .push()
        .map_err(|e| format!("Error tabla header: {}", e))?;

    doc.push(header_table);
    doc.push(Break::new(1.0));

    // ===================================================================
    // SECCIÓN 2: DATOS DEL SUJETO RETENIDO (recuadro full width)
    // ===================================================================
    let mut sujeto_section = LinearLayout::vertical();

    let mut fila1 = TableLayout::new(vec![3, 2]);
    fila1
        .row()
        .element(pp(
            &format!("Razón Social / Nombres Apellidos: {}", datos.sujeto_nombre),
            s_normal,
        ))
        .element(pp(
            &format!("{}: {}", tipo_id_label(&datos.sujeto_tipo_id), datos.sujeto_identificacion),
            s_normal,
        ))
        .push()
        .map_err(|e| format!("Error fila sujeto 1: {}", e))?;
    sujeto_section.push(fila1);

    let mut fila2 = TableLayout::new(vec![3, 2]);
    let dir_text = match datos.sujeto_direccion.as_deref() {
        Some(d) if !d.is_empty() => format!("Dirección: {}", d),
        _ => String::new(),
    };
    fila2
        .row()
        .element(pp(&format!("Fecha de Emisión: {}", datos.fecha_emision), s_bold))
        .element(pp(&dir_text, s_normal))
        .push()
        .map_err(|e| format!("Error fila sujeto 2: {}", e))?;
    sujeto_section.push(fila2);

    let mut fila3 = TableLayout::new(vec![3, 2]);
    fila3
        .row()
        .element(pp(&format!("Período Fiscal: {}", datos.periodo_fiscal), s_bold))
        .element(pp("", s_normal))
        .push()
        .map_err(|e| format!("Error fila sujeto 3: {}", e))?;
    sujeto_section.push(fila3);

    doc.push(sujeto_section.padded(Margins::trbl(3, 2, 3, 2)).framed());
    doc.push(Break::new(1.0));

    // ===================================================================
    // SECCIÓN 3: TABLA DE IMPUESTOS RETENIDOS
    // Columnas: Comprob | No.Comp | Fecha | Imp. Ret. | Cod | Base | % | Valor
    // Pesos: 3, 3, 2, 2, 1, 2, 1, 2 = 16
    // ===================================================================
    let mut table = TableLayout::new(vec![3, 3, 2, 2, 1, 2, 1, 2]);
    table.set_cell_decorator(genpdf::elements::FrameCellDecorator::new(true, true, false));

    table
        .row()
        .element(pp("Comprobante", s_small_bold))
        .element(pp("Número", s_small_bold))
        .element(pp("Fecha", s_small_bold))
        .element(pp("Impuesto", s_small_bold))
        .element(pp_center("Cód.", s_small_bold))
        .element(pp_right("Base Imp.", s_small_bold))
        .element(pp_center("%", s_small_bold))
        .element(pp_right("Valor Ret.", s_small_bold))
        .push()
        .map_err(|e| format!("Error tabla header retencion: {}", e))?;

    for it in items {
        // Formatear num doc sustento "001001000000001" → "001-001-000000001"
        let num_formateado = if it.num_doc_sustento.len() == 15 {
            format!("{}-{}-{}",
                &it.num_doc_sustento[0..3],
                &it.num_doc_sustento[3..6],
                &it.num_doc_sustento[6..15])
        } else {
            it.num_doc_sustento.clone()
        };

        table
            .row()
            .element(pp(cod_doc_sustento_label(&it.cod_doc_sustento), s_small))
            .element(pp(&num_formateado, s_small))
            .element(pp(&it.fecha_doc_sustento, s_small))
            .element(pp(&it.tipo_label, s_small))
            .element(pp_center(&it.codigo_retencion, s_small))
            .element(pp_right(&format_dinero(it.base_imponible), s_small))
            .element(pp_center(&format_pct(it.porcentaje), s_small))
            .element(pp_right(&format_dinero(it.valor_retenido), s_small))
            .push()
            .map_err(|e| format!("Error tabla fila retencion: {}", e))?;
    }

    doc.push(table);
    doc.push(Break::new(0.8));

    // ===================================================================
    // SECCIÓN 4: TOTAL RETENIDO (alineado a la derecha)
    // ===================================================================
    let mut total_table = TableLayout::new(vec![10, 4]);
    total_table.set_cell_decorator(genpdf::elements::FrameCellDecorator::new(true, true, false));
    total_table
        .row()
        .element(pp_right("VALOR TOTAL RETENIDO", s_total_bold))
        .element(pp_right(&format!("$ {}", format_dinero(datos.total_retenido)), s_total_bold))
        .push()
        .map_err(|e| format!("Error tabla total: {}", e))?;
    doc.push(total_table);
    doc.push(Break::new(1.0));

    // ===================================================================
    // SECCIÓN 5: INFORMACIÓN ADICIONAL (email del sujeto)
    // ===================================================================
    if let Some(email) = datos.sujeto_email.as_deref() {
        if !email.is_empty() {
            let mut info = LinearLayout::vertical();
            info.push(pp("INFORMACIÓN ADICIONAL", s_bold));
            info.push(pp(&format!("Email: {}", email), s_normal));
            doc.push(info.padded(Margins::trbl(3, 2, 3, 2)).framed());
            doc.push(Break::new(0.5));
        }
    }

    // ===================================================================
    // SECCIÓN 6: PIE DE PÁGINA
    // ===================================================================
    doc.push(Break::new(1.0));
    doc.push(p_aligned(
        "Este documento es una Representación Impresa de un Comprobante Electrónico (RIDE)",
        s_pie,
        Alignment::Center,
    ));
    doc.push(p_aligned(
        "Conforme a la normativa del Servicio de Rentas Internas - SRI Ecuador",
        s_pie,
        Alignment::Center,
    ));
    doc.push(p_aligned(
        &format!("Generado por Clouget POS — Ambiente: {}", ambiente_label),
        s_pie,
        Alignment::Center,
    ));

    // ===================================================================
    // Renderizar
    // ===================================================================
    let mut buffer = Vec::new();
    doc.render(&mut buffer)
        .map_err(|e| format!("Error renderizando PDF: {}", e))?;
    Ok(buffer)
}
