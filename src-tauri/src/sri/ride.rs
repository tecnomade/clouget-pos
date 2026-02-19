use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use genpdf::elements::{Break, LinearLayout, PaddedElement, Paragraph, StyledElement, TableLayout};
use genpdf::style::{Style, Color};
use genpdf::{Alignment, Document, Element, Margins, SimplePageDecorator};
use std::collections::HashMap;

use crate::models::VentaCompleta;

/// Info del documento modificado (para notas de crédito)
pub struct DocModificado {
    pub tipo: String,         // "01" factura
    pub numero: String,       // "001-003-000000019"
    pub fecha_emision: String,
    pub motivo: String,
}

/// Datos del cliente para el RIDE
pub struct ClienteRide {
    pub nombre: String,
    pub identificacion: String,
    pub direccion: String,
    pub email: String,
    pub telefono: String,
    pub observacion: String,
}

/// Detalle con codigo de producto para el RIDE
pub struct DetalleRide {
    pub codigo: String,
    pub nombre: String,
    pub cantidad: f64,
    pub precio_unitario: f64,
    pub descuento: f64,
    pub iva_porcentaje: f64,
    pub precio_total_sin_impuesto: f64,
}

// ============================================
// HELPERS
// ============================================

/// Paragraph styled
fn p(text: &str, style: Style) -> StyledElement<Paragraph> {
    Paragraph::new(text).styled(style)
}

/// Paragraph aligned + styled
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

/// Paragraph con padding real (izquierdo 3mm) para celdas de tabla/secciones
fn pp(text: &str, style: Style) -> PaddedElement<StyledElement<Paragraph>> {
    Paragraph::new(text).styled(style).padded(Margins::trbl(1, 1, 1, 3))
}

/// Paragraph con padding real + alineado
fn pp_aligned(text: &str, style: Style, align: Alignment) -> impl Element {
    Paragraph::new(text).aligned(align).styled(style).padded(Margins::trbl(1, 1, 1, 3))
}

/// Paragraph alineado a la derecha con padding (para valores monetarios)
fn pp_right(text: &str, style: Style) -> impl Element {
    Paragraph::new(text).aligned(Alignment::Right).styled(style).padded(Margins::trbl(1, 3, 1, 1))
}

// ============================================
// GENERADOR RIDE PDF
// ============================================

/// Genera el RIDE (PDF A4) para una factura o nota de crédito autorizada.
/// Formato estandar SRI Ecuador basado en la referencia oficial.
pub fn generar_ride_pdf(
    venta: &VentaCompleta,
    detalles_ride: &[DetalleRide],
    cliente: &ClienteRide,
    config: &HashMap<String, String>,
    fecha_autorizacion: Option<&str>,
    tipo_doc: &str,                        // "FACTURA" o "NOTA DE CREDITO"
    doc_modificado: Option<&DocModificado>, // Solo para NC
) -> Result<Vec<u8>, String> {
    let fonts_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("fonts");

    let font_family = genpdf::fonts::from_files(
        fonts_dir.to_str().unwrap_or("fonts"),
        "LiberationSans",
        None,
    )
    .map_err(|e| format!("Error cargando fuentes: {}. Asegurese de que los archivos LiberationSans-*.ttf estan en src-tauri/fonts/", e))?;

    let mut doc = Document::new(font_family);
    doc.set_title(if tipo_doc == "FACTURA" { "RIDE - Factura Electronica" } else { "RIDE - Nota de Credito Electronica" });

    // Page margins
    let mut decorator = SimplePageDecorator::new();
    decorator.set_margins(Margins::trbl(15, 15, 15, 15));
    doc.set_page_decorator(decorator);

    // Estilos (tamaños aumentados para mejor legibilidad)
    let s_normal = Style::new().with_font_size(9);
    let s_bold = Style::new().with_font_size(9).bold();
    let s_small = Style::new().with_font_size(8);
    let s_small_bold = Style::new().with_font_size(8).bold();
    let s_title = Style::new().with_font_size(14).bold();
    let s_doc_type = Style::new().with_font_size(16).bold();
    let s_doc_no = Style::new().with_font_size(12);
    let s_ruc = Style::new().with_font_size(10).bold();
    let s_clave = Style::new().with_font_size(7);
    let s_clave_small = Style::new().with_font_size(7);
    let s_total_bold = Style::new().with_font_size(11).bold();
    let s_pie = Style::new().with_font_size(7).with_color(Color::Greyscale(128));
    let s_regimen = Style::new().with_font_size(8).bold();

    // --- Datos del config ---
    let nombre_negocio = config.get("nombre_negocio").map(|s| s.as_str()).unwrap_or("MI NEGOCIO");
    let ruc = config.get("ruc").map(|s| s.as_str()).unwrap_or("");
    let direccion_neg = config.get("direccion").map(|s| s.as_str()).unwrap_or("");
    let telefono_neg = config.get("telefono").map(|s| s.as_str()).unwrap_or("");
    let regimen = config.get("regimen").map(|s| s.as_str()).unwrap_or("");
    let ambiente = config.get("sri_ambiente").map(|s| s.as_str()).unwrap_or("pruebas");
    let ambiente_label = if ambiente == "produccion" { "PRODUCCION" } else { "PRUEBAS" };

    let clave_acceso = venta.venta.clave_acceso.as_deref().unwrap_or("");
    let autorizacion = venta.venta.autorizacion_sri.as_deref().unwrap_or(clave_acceso);
    let fecha_emision = venta.venta.fecha.as_deref().unwrap_or("-");
    let fecha_aut_str = fecha_autorizacion.unwrap_or(fecha_emision);

    let regimen_label = match regimen {
        "RIMPE_POPULAR" => "CONTRIBUYENTE NEGOCIO POPULAR - REGIMEN RIMPE",
        "RIMPE_EMPRENDEDOR" => "CONTRIBUYENTE REGIMEN RIMPE",
        "GENERAL" => "REGIMEN GENERAL",
        _ => "",
    };

    // ===================================================================
    // SECCION 1: ENCABEZADO
    // Estructura: 1 fila, 2 columnas en TableLayout (alinea bordes abajo)
    // Izq: Logo (framed, menos alto) + Datos emisor (framed) apilados
    // Der: Una sola celda framed con RUC, FACTURA, barcode, clave
    // ===================================================================
    let mut header_table = TableLayout::new(vec![1, 1]);
    // FrameCellDecorator dibuja bordes de celda que se extienden al alto completo de la fila
    // => bordes inferiores de ambas columnas quedan alineados automáticamente
    header_table.set_cell_decorator(genpdf::elements::FrameCellDecorator::new(true, true, false));

    // --- Columna izquierda: Logo arriba + Datos emisor abajo ---
    // Estructura: [logo o espacio vacío] + [datos emisor]
    // El espacio libre queda arriba (entre logo y datos) gracias al FrameCellDecorator
    let mut col_izq = LinearLayout::vertical();

    // Preparar logo si existe (se insertará después de los datos emisor, al final)
    let mut logo_element: Option<genpdf::elements::PaddedElement<genpdf::elements::Image>> = None;
    if let Some(logo_b64) = config.get("logo_negocio") {
        if !logo_b64.is_empty() {
            if let Ok(logo_bytes) = BASE64.decode(logo_b64) {
                let logo_temp = std::env::temp_dir().join("clouget_ride_logo.png");
                if std::fs::write(&logo_temp, &logo_bytes).is_ok() {
                    if let Ok(mut logo_img) = genpdf::elements::Image::from_path(&logo_temp) {
                        logo_img = logo_img.with_alignment(Alignment::Center);
                        // Escala dinámica: siempre llenar el ancho de la columna
                        // Fórmula genpdf: rendered_mm = 25.4 * (scale * pixels) / 300
                        // Despejando: scale = (target_mm * 300) / (25.4 * pixels)
                        let max_width_mm = 84.0_f64; // ancho útil columna (~90mm - 6mm padding)
                        // Leer dimensiones del PNG desde el header (bytes 16-23)
                        let (img_w, img_h) = if logo_bytes.len() > 24 && &logo_bytes[0..4] == b"\x89PNG" {
                            let w = u32::from_be_bytes([logo_bytes[16], logo_bytes[17], logo_bytes[18], logo_bytes[19]]) as f64;
                            let h = u32::from_be_bytes([logo_bytes[20], logo_bytes[21], logo_bytes[22], logo_bytes[23]]) as f64;
                            (w, h)
                        } else {
                            (200.0, 100.0)
                        };
                        // Escalar para llenar el ancho, pero limitar alto al espacio libre
                        // La col derecha mide ~55mm; datos emisor ~25mm → espacio libre ~30mm
                        let max_height_mm = 35.0_f64;
                        let scale_by_w = (max_width_mm * 300.0) / (25.4 * img_w);
                        let rendered_h = 25.4 * (scale_by_w * img_h) / 300.0;
                        let final_scale = if rendered_h > max_height_mm {
                            // Logo cuadrado/vertical: limitar por alto para no agrandar la fila
                            (max_height_mm * 300.0) / (25.4 * img_h)
                        } else {
                            // Logo horizontal: llenar todo el ancho
                            scale_by_w
                        };
                        logo_img = logo_img.with_scale(genpdf::Scale::new(final_scale, final_scale));
                        // Sin padding lateral; solo 1mm arriba/abajo
                        logo_element = Some(logo_img.padded(Margins::trbl(1, 0, 1, 0)));
                    }
                    let _ = std::fs::remove_file(&logo_temp);
                }
            }
        }
    }

    // Logo va primero arriba; si no hay logo, espacio vacío
    if let Some(logo) = logo_element {
        col_izq.push(logo);
    } else {
        col_izq.push(Break::new(8.0));
    }

    // Sub-seccion 2: Datos del emisor (con borde)
    let mut datos_emisor = LinearLayout::vertical();
    datos_emisor.push(Break::new(0.3));
    datos_emisor.push(pp(nombre_negocio, s_title));
    datos_emisor.push(Break::new(0.3));
    if !direccion_neg.is_empty() {
        datos_emisor.push(pp(&format!("Direccion Matriz: {}", direccion_neg), s_normal));
        datos_emisor.push(pp(&format!("Direccion Sucursal: {}", direccion_neg), s_normal));
    }
    if !telefono_neg.is_empty() {
        datos_emisor.push(pp(&format!("Tel: {}", telefono_neg), s_normal));
    }
    datos_emisor.push(Break::new(0.3));
    datos_emisor.push(pp("OBLIGADO A LLEVAR CONTABILIDAD: NO", s_bold));
    if !regimen_label.is_empty() {
        datos_emisor.push(Break::new(0.2));
        datos_emisor.push(pp(regimen_label, s_regimen));
    }
    datos_emisor.push(Break::new(0.5));
    col_izq.push(datos_emisor);

    // --- Columna derecha: Una sola celda con todo ---
    let mut col_der = LinearLayout::vertical();
    col_der.push(Break::new(0.3));
    col_der.push(pp(&format!("R.U.C.:  {}", ruc), s_ruc));
    col_der.push(Break::new(0.3));
    col_der.push(pp(tipo_doc, s_doc_type));
    let num_factura_ride = venta.venta.numero_factura.as_deref().unwrap_or(&venta.venta.numero);
    col_der.push(pp(&format!("No. {}", num_factura_ride), s_doc_no));
    col_der.push(Break::new(0.3));
    col_der.push(pp("NUMERO DE AUTORIZACION", s_bold));
    col_der.push(pp(autorizacion, s_clave));
    col_der.push(Break::new(0.3));
    col_der.push(pp("FECHA Y HORA DE AUTORIZACION", s_bold));
    col_der.push(pp(fecha_aut_str, s_normal));
    col_der.push(Break::new(0.3));
    col_der.push(pp(&format!("AMBIENTE:    {}", ambiente_label), s_normal));
    col_der.push(pp("EMISION:     NORMAL", s_normal));
    col_der.push(Break::new(0.3));

    // Clave de acceso + código de barras
    col_der.push(pp("CLAVE DE ACCESO:", s_bold));
    col_der.push(Break::new(0.3));
    if !clave_acceso.is_empty() {
        match generar_barcode128_image(clave_acceso) {
            Ok(barcode_path) => {
                if let Ok(mut barcode_img) = genpdf::elements::Image::from_path(&barcode_path) {
                    barcode_img = barcode_img.with_alignment(Alignment::Center);
                    // PNG ~331px ancho (Code128 de 49 dígitos, scale_x=1, ~321 módulos + quiet_zone)
                    // ancho: 25.4*(1.8*331)/300 = 50.4mm | alto: 25.4*(2.0*80)/300 = 13.5mm
                    // Columna ~85mm útiles con márgenes 15mm => cabe bien
                    barcode_img = barcode_img.with_scale(genpdf::Scale::new(1.8, 2.0));
                    col_der.push(barcode_img);
                }
                let _ = std::fs::remove_file(&barcode_path);
            }
            Err(e) => {
                eprintln!("Warning: No se pudo generar barcode Code128: {}", e);
            }
        }
    }
    col_der.push(Break::new(0.3));
    col_der.push(p_aligned(clave_acceso, s_clave_small, Alignment::Center));
    col_der.push(Break::new(0.3));

    // Una fila: FrameCellDecorator dibuja bordes alineados al alto de la fila
    header_table
        .row()
        .element(col_izq.padded(Margins::trbl(2, 3, 2, 3)))
        .element(col_der.padded(Margins::trbl(2, 3, 2, 3)))
        .push()
        .map_err(|e| format!("Error tabla header: {}", e))?;

    doc.push(header_table);
    doc.push(Break::new(1.0));

    // (Clave de acceso integrada en columna derecha del encabezado)

    // ===================================================================
    // SECCION 2: DOCUMENTO MODIFICADO (solo para Notas de Crédito)
    // ===================================================================
    if let Some(dm) = doc_modificado {
        let mut dm_section = LinearLayout::vertical();
        dm_section.push(Break::new(0.3));
        let tipo_doc_mod = if dm.tipo == "01" { "FACTURA" } else { &dm.tipo };
        dm_section.push(pp(&format!("COMPROBANTE QUE SE MODIFICA: {}", tipo_doc_mod), s_bold));
        dm_section.push(pp(&format!("No. {}    Fecha: {}", dm.numero, dm.fecha_emision), s_normal));
        dm_section.push(pp(&format!("RAZON DE MODIFICACION: {}", dm.motivo), s_bold));
        dm_section.push(Break::new(0.3));
        doc.push(dm_section.framed());
        doc.push(Break::new(0.5));
    }

    // ===================================================================
    // SECCION 3: DATOS DEL COMPRADOR (recuadro full width)
    // ===================================================================
    let mut comprador_section = LinearLayout::vertical();

    // Fila 1: Razon Social + Identificacion (2 columnas)
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
            &format!("Razon Social / Nombres Apellidos: {}", cliente.nombre),
            s_normal,
        ))
        .element(pp(
            &format!("{}: {}", tipo_id_label, cliente.identificacion),
            s_normal,
        ))
        .push()
        .map_err(|e| format!("Error fila comprador 1: {}", e))?;
    comprador_section.push(fila1);

    // Fila 2: Fecha Emision + Direccion (2 columnas)
    let mut fila2 = TableLayout::new(vec![3, 2]);
    let dir_text = if !cliente.direccion.is_empty() {
        format!("Direccion: {}", cliente.direccion)
    } else {
        String::new()
    };
    fila2
        .row()
        .element(pp(&format!("Fecha de emision: {}", fecha_emision), s_bold))
        .element(pp(&dir_text, s_normal))
        .push()
        .map_err(|e| format!("Error fila comprador 2: {}", e))?;
    comprador_section.push(fila2);

    doc.push(comprador_section.padded(Margins::trbl(3, 2, 3, 2)).framed());
    doc.push(Break::new(1.0));

    // ===================================================================
    // SECCION 4: TABLA DE PRODUCTOS
    // ===================================================================
    let mut table = TableLayout::new(vec![2, 1, 6, 2, 1, 2]);
    table.set_cell_decorator(genpdf::elements::FrameCellDecorator::new(true, true, false));

    // Header
    table
        .row()
        .element(pp("Codigo", s_small_bold))
        .element(pp("Cant.", s_small_bold))
        .element(pp("Descripcion", s_small_bold))
        .element(pp_right("P. Unit.", s_small_bold))
        .element(pp_right("Desc.", s_small_bold))
        .element(pp_right("Subtotal", s_small_bold))
        .push()
        .map_err(|e| format!("Error tabla header: {}", e))?;

    // Filas de productos
    for det in detalles_ride {
        table
            .row()
            .element(pp(&det.codigo, s_small))
            .element(pp(&format_cantidad(det.cantidad), s_small))
            .element(pp(&det.nombre, s_small))
            .element(pp_right(&format_dinero(det.precio_unitario), s_small))
            .element(pp_right(&format_dinero(det.descuento), s_small))
            .element(pp_right(&format_dinero(det.precio_total_sin_impuesto), s_small))
            .push()
            .map_err(|e| format!("Error tabla fila: {}", e))?;
    }

    doc.push(table);
    doc.push(Break::new(1.5));

    // ===================================================================
    // SECCION 5: INFO ADICIONAL + TOTALES (2 columnas)
    // ===================================================================
    let mut bottom_table = TableLayout::new(vec![12, 8]);

    // --- Columna izquierda: Info adicional + Forma de pago ---
    let mut info_col = LinearLayout::vertical();
    info_col.push(Break::new(0.8));
    info_col.push(pp("Informacion Adicional", s_bold));
    info_col.push(Break::new(1.0));

    if !cliente.direccion.is_empty() {
        info_col.push(pp(&format!("Direccion:  {}", cliente.direccion), s_small));
        info_col.push(Break::new(0.3));
    }
    if !cliente.email.is_empty() {
        info_col.push(pp(&format!("Email:  {}", cliente.email), s_small));
        info_col.push(Break::new(0.3));
    }
    if !cliente.telefono.is_empty() {
        info_col.push(pp(&format!("Telefono:  {}", cliente.telefono), s_small));
        info_col.push(Break::new(0.3));
    }
    if !cliente.observacion.is_empty() {
        info_col.push(pp(&format!("Observacion:  {}", cliente.observacion), s_small));
        info_col.push(Break::new(0.3));
    }

    info_col.push(Break::new(2.0));

    // Forma de pago (sub-tabla dentro de info adicional)
    let mut pago_table = TableLayout::new(vec![5, 2]);
    pago_table.set_cell_decorator(genpdf::elements::FrameCellDecorator::new(true, true, false));

    pago_table
        .row()
        .element(pp("Forma de Pago", s_small_bold))
        .element(pp("Valor", s_small_bold))
        .push()
        .map_err(|e| format!("Error forma pago header: {}", e))?;

    let forma_pago_desc = match venta.venta.forma_pago.as_str() {
        "EFECTIVO" => "SIN UTILIZACION DEL SISTEMA FINANCIERO",
        "TARJETA" | "TARJETA_CREDITO" | "TARJETA_DEBITO" => "TARJETA DE CREDITO",
        "TRANSFERENCIA" => "OTROS CON UTILIZACION DEL SISTEMA FINANCIERO",
        _ => "SIN UTILIZACION DEL SISTEMA FINANCIERO",
    };

    pago_table
        .row()
        .element(pp(forma_pago_desc, s_small))
        .element(pp_right(&format_dinero(venta.venta.total), s_small))
        .push()
        .map_err(|e| format!("Error forma pago fila: {}", e))?;

    info_col.push(pago_table);
    info_col.push(Break::new(1.0));

    // --- Columna derecha: Totales desglosados ---
    let mut totales_col = LinearLayout::vertical();

    // Calcular subtotales por tasa IVA
    let mut sub_iva_15 = 0.0_f64;
    let mut sub_iva_5 = 0.0_f64;
    let mut sub_iva_0 = 0.0_f64;
    let descuento_total = venta.venta.descuento;

    for det in detalles_ride {
        if det.iva_porcentaje >= 14.0 {
            sub_iva_15 += det.precio_total_sin_impuesto;
        } else if det.iva_porcentaje > 0.0 && det.iva_porcentaje < 14.0 {
            sub_iva_5 += det.precio_total_sin_impuesto;
        } else {
            sub_iva_0 += det.precio_total_sin_impuesto;
        }
    }

    let subtotal_sin_impuestos = sub_iva_0 + sub_iva_5 + sub_iva_15;
    let iva_15_valor = venta.venta.iva; // IVA calculado en la venta

    // Tabla de totales con bordes
    let mut totales_table = TableLayout::new(vec![4, 2]);
    totales_table.set_cell_decorator(genpdf::elements::FrameCellDecorator::new(true, true, false));

    // Agregar cada linea de totales
    let totales_lines: Vec<(&str, f64, Style)> = vec![
        ("SUBTOTAL 5%", sub_iva_5, s_small),
        ("SUBTOTAL 15%", sub_iva_15, s_small),
        ("SUBTOTAL IVA 0%", sub_iva_0, s_small),
        ("SUBTOTAL NO OBJETO IVA", 0.0, s_small),
        ("SUBTOTAL EXENTO IVA", 0.0, s_small),
        ("SUBTOTAL SIN IMPUESTO", subtotal_sin_impuestos, s_small),
        ("DESCUENTO", descuento_total, s_small),
        ("ICE", 0.0, s_small),
        ("IVA 5%", 0.0, s_small),
        ("IVA 15%", iva_15_valor, s_small),
        ("IRBPNR", 0.0, s_small),
        ("PROPINA", 0.0, s_small),
    ];

    for (label, valor, style) in &totales_lines {
        totales_table
            .row()
            .element(pp(label, *style))
            .element(pp_right(&format_dinero(*valor), *style))
            .push()
            .map_err(|e| format!("Error totales fila: {}", e))?;
    }

    // VALOR TOTAL (bold, mas grande)
    totales_table
        .row()
        .element(pp("VALOR TOTAL", s_total_bold))
        .element(pp_right(&format_dinero(venta.venta.total), s_total_bold))
        .push()
        .map_err(|e| format!("Error totales valor total: {}", e))?;

    totales_col.push(totales_table);

    // Juntar las dos columnas
    bottom_table
        .row()
        .element(info_col.padded(Margins::trbl(2, 2, 2, 2)).framed())
        .element(totales_col.padded(Margins::trbl(0, 0, 0, 2)))
        .push()
        .map_err(|e| format!("Error tabla bottom: {}", e))?;

    doc.push(bottom_table);
    doc.push(Break::new(2.5));

    // ===================================================================
    // PIE DE PAGINA
    // ===================================================================
    doc.push(p_aligned(
        "Representacion Impresa de Documento Electronico - SRI Ecuador",
        s_pie,
        Alignment::Center,
    ));

    // --- RENDERIZAR A BYTES ---
    let mut buffer = Vec::new();
    doc.render(&mut buffer)
        .map_err(|e| format!("Error generando PDF: {}", e))?;

    Ok(buffer)
}

// ============================================
// GENERADOR TICKET PDF (80mm)
// ============================================

/// Genera un ticket de venta como PDF en formato 80mm para abrir en visor del sistema.
pub fn generar_ticket_pdf(
    venta: &VentaCompleta,
    config: &HashMap<String, String>,
) -> Result<Vec<u8>, String> {
    let fonts_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("fonts");

    let font_family = genpdf::fonts::from_files(
        fonts_dir.to_str().unwrap_or("fonts"),
        "LiberationSans",
        None,
    )
    .map_err(|e| format!("Error cargando fuentes: {}", e))?;

    let mut doc = Document::new(font_family);
    doc.set_title("Ticket de Venta");
    // 80mm width, ~250mm height (estimado generoso)
    doc.set_paper_size(genpdf::Size::new(80, 250));

    let mut decorator = SimplePageDecorator::new();
    decorator.set_margins(Margins::trbl(3, 3, 3, 3));
    doc.set_page_decorator(decorator);

    let s_title = Style::new().with_font_size(10).bold();
    let s_normal = Style::new().with_font_size(7);
    let s_bold = Style::new().with_font_size(7).bold();
    let s_small = Style::new().with_font_size(6);
    let s_total = Style::new().with_font_size(9).bold();
    let nombre_negocio = config.get("nombre_negocio").map(|s| s.as_str()).unwrap_or("MI NEGOCIO");
    let ruc = config.get("ruc").map(|s| s.as_str()).unwrap_or("");
    let direccion = config.get("direccion").map(|s| s.as_str()).unwrap_or("");

    // Encabezado
    doc.push(p_aligned(nombre_negocio, s_title, Alignment::Center));
    if !ruc.is_empty() {
        doc.push(p_aligned(&format!("RUC: {}", ruc), s_normal, Alignment::Center));
    }
    if !direccion.is_empty() {
        doc.push(p_aligned(direccion, s_small, Alignment::Center));
    }

    doc.push(Break::new(0.5));

    // Info venta
    doc.push(p(&format!("No: {}", venta.venta.numero), s_bold));
    if let Some(ref fecha) = venta.venta.fecha {
        doc.push(p(&format!("Fecha: {}", fecha), s_normal));
    }
    if let Some(ref cliente) = venta.cliente_nombre {
        doc.push(p(&format!("Cliente: {}", cliente), s_normal));
    }

    doc.push(Break::new(0.5));

    // Productos
    let mut prod_table = TableLayout::new(vec![1, 4, 2]);
    prod_table
        .row()
        .element(p("Cant", s_bold))
        .element(p("Producto", s_bold))
        .element(p_aligned("Total", s_bold, Alignment::Right))
        .push()
        .map_err(|e| format!("Error ticket header: {}", e))?;

    for det in &venta.detalles {
        let nombre = det.nombre_producto.as_deref().unwrap_or("?");
        let total = det.cantidad * det.precio_unitario - det.descuento;
        prod_table
            .row()
            .element(p(&format_cantidad(det.cantidad), s_normal))
            .element(p(nombre, s_normal))
            .element(p_aligned(&format_dinero(total), s_normal, Alignment::Right))
            .push()
            .map_err(|e| format!("Error ticket fila: {}", e))?;
    }
    doc.push(prod_table);

    doc.push(Break::new(0.5));

    // Totales
    let mut total_table = TableLayout::new(vec![3, 2]);

    if venta.venta.subtotal_sin_iva > 0.0 {
        total_table.row()
            .element(p("Subtotal 0%:", s_normal))
            .element(p_aligned(&format_dinero(venta.venta.subtotal_sin_iva), s_normal, Alignment::Right))
            .push().map_err(|e| format!("Error: {}", e))?;
    }
    if venta.venta.subtotal_con_iva > 0.0 {
        total_table.row()
            .element(p("Subtotal 15%:", s_normal))
            .element(p_aligned(&format_dinero(venta.venta.subtotal_con_iva), s_normal, Alignment::Right))
            .push().map_err(|e| format!("Error: {}", e))?;
    }
    if venta.venta.iva > 0.0 {
        total_table.row()
            .element(p("IVA 15%:", s_normal))
            .element(p_aligned(&format_dinero(venta.venta.iva), s_normal, Alignment::Right))
            .push().map_err(|e| format!("Error: {}", e))?;
    }
    if venta.venta.descuento > 0.0 {
        total_table.row()
            .element(p("Descuento:", s_normal))
            .element(p_aligned(&format_dinero(venta.venta.descuento), s_normal, Alignment::Right))
            .push().map_err(|e| format!("Error: {}", e))?;
    }
    total_table.row()
        .element(p("TOTAL:", s_total))
        .element(p_aligned(&format!("${}", format_dinero(venta.venta.total)), s_total, Alignment::Right))
        .push().map_err(|e| format!("Error: {}", e))?;

    doc.push(total_table);

    doc.push(Break::new(0.3));

    // Forma de pago
    doc.push(p(&format!("Pago: {}", venta.venta.forma_pago), s_normal));
    if venta.venta.monto_recibido > 0.0 {
        doc.push(p(&format!("Recibido: ${}", format_dinero(venta.venta.monto_recibido)), s_normal));
        doc.push(p(&format!("Cambio:   ${}", format_dinero(venta.venta.cambio)), s_normal));
    }

    // Info SRI si fue autorizada
    if venta.venta.estado_sri == "AUTORIZADA" {
        doc.push(Break::new(0.5));
        doc.push(p_aligned("FACTURA ELECTRONICA AUTORIZADA", s_bold, Alignment::Center));
        let num_factura = venta.venta.numero_factura.as_deref().unwrap_or(&venta.venta.numero);
        doc.push(p(&format!("Factura No: {}", num_factura), s_normal));
        if let Some(ref aut) = venta.venta.autorizacion_sri {
            doc.push(p(&format!("No. Aut: {}", aut), s_small));
        }
        let ambiente = config.get("sri_ambiente").map(|s| s.as_str()).unwrap_or("pruebas");
        let ambiente_label = if ambiente == "produccion" { "PRODUCCION" } else { "PRUEBAS" };
        doc.push(p(&format!("Ambiente: {}", ambiente_label), s_normal));

        // QR de clave de acceso
        if let Some(ref clave) = venta.venta.clave_acceso {
            if !clave.is_empty() {
                if let Ok(qr_path) = generar_qr_image(clave) {
                    if let Ok(mut qr_img) = genpdf::elements::Image::from_path(&qr_path) {
                        qr_img = qr_img.with_alignment(Alignment::Center);
                        qr_img = qr_img.with_scale(genpdf::Scale::new(0.25, 0.25));
                        doc.push(qr_img);
                    }
                    let _ = std::fs::remove_file(&qr_path);
                }
            }
        }

        doc.push(Break::new(0.3));
        doc.push(p_aligned("ESTE TICKET NO ES UN", s_bold, Alignment::Center));
        doc.push(p_aligned("DOCUMENTO TRIBUTARIO OFICIAL", s_bold, Alignment::Center));
        doc.push(p_aligned("Los comprobantes RIDE y XML seran", s_small, Alignment::Center));
        doc.push(p_aligned("enviados a su email registrado", s_small, Alignment::Center));
    }

    doc.push(Break::new(1));
    doc.push(p_aligned("Gracias por su compra!", s_bold, Alignment::Center));

    // Renderizar
    let mut buffer = Vec::new();
    doc.render(&mut buffer)
        .map_err(|e| format!("Error generando ticket PDF: {}", e))?;

    Ok(buffer)
}

// ============================================
// QR CODE GENERATOR (usado en ticket 80mm)
// ============================================

/// Genera un QR code como imagen PNG en archivo temporal.
fn generar_qr_image(data: &str) -> Result<String, String> {
    use qrcode::QrCode;

    let code = QrCode::new(data.as_bytes())
        .map_err(|e| format!("Error creando QR: {}", e))?;

    let modules = code.to_colors();
    let width = code.width() as u32;
    let scale = 4_u32;
    let border = 4_u32;
    let img_size = (width + border * 2) * scale;

    let mut img_buf = vec![255u8; (img_size * img_size) as usize];

    for (i, color) in modules.iter().enumerate() {
        let x = (i as u32) % width;
        let y = (i as u32) / width;
        if *color == qrcode::types::Color::Dark {
            let px = (x + border) * scale;
            let py = (y + border) * scale;
            for dy in 0..scale {
                for dx in 0..scale {
                    let idx = ((py + dy) * img_size + (px + dx)) as usize;
                    if idx < img_buf.len() {
                        img_buf[idx] = 0;
                    }
                }
            }
        }
    }

    let gray_img = image::GrayImage::from_raw(img_size, img_size, img_buf)
        .ok_or("Error creando imagen QR")?;

    let temp_dir = std::env::temp_dir();
    let qr_path = temp_dir.join("clouget_ride_qr.png");
    gray_img
        .save(&qr_path)
        .map_err(|e| format!("Error guardando QR: {}", e))?;

    Ok(qr_path.to_string_lossy().to_string())
}

// ============================================
// CODE128 BARCODE GENERATOR (usado en RIDE A4)
// ============================================

/// Genera un codigo de barras Code128 como imagen PNG en archivo temporal.
/// Intenta Code128-C (numerico) primero, luego Code128-B como fallback.
fn generar_barcode128_image(data: &str) -> Result<String, String> {
    use barcoders::sym::code128::Code128;

    // Code128-C (Ć = U+0106): optimo para datos numericos puros
    // Code128-B (Ɓ = U+0181): alfanumerico general (fallback)
    let data_c = format!("\u{0106}{}", data);
    let barcode = Code128::new(&data_c)
        .or_else(|_| {
            let data_b = format!("\u{0181}{}", data);
            Code128::new(&data_b)
        })
        .map_err(|e| format!("Error creando Code128: {}", e))?;
    let encoded: Vec<u8> = barcode.encode();

    // Generar imagen PNG manualmente (sin feature "image" de barcoders)
    // scale_x=1: cada barra = 1 pixel (barcode compacto, genpdf escala después)
    let height = 80_u32;
    let scale_x = 1_u32;
    let quiet_zone = 5_u32; // margen lateral blanco minimo
    let width = (encoded.len() as u32) * scale_x + quiet_zone * 2;

    let mut img_buf = vec![255u8; (width * height) as usize];

    for (i, &bar) in encoded.iter().enumerate() {
        if bar == 1 {
            for x_offset in 0..scale_x {
                let px = quiet_zone + (i as u32) * scale_x + x_offset;
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

    let temp_path = std::env::temp_dir().join("clouget_ride_barcode.png");
    gray_img
        .save(&temp_path)
        .map_err(|e| format!("Error guardando barcode: {}", e))?;

    Ok(temp_path.to_string_lossy().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{Venta, VentaDetalle, VentaCompleta};

    #[test]
    fn test_generar_ride_prueba() {
        let venta = VentaCompleta {
            venta: Venta {
                id: Some(1),
                numero: "001-003-000000001".to_string(),
                cliente_id: Some(2),
                fecha: Some("2026-02-18 10:30:00".to_string()),
                subtotal_sin_iva: 5.00,
                subtotal_con_iva: 10.00,
                descuento: 0.0,
                iva: 1.50,
                total: 16.50,
                forma_pago: "EFECTIVO".to_string(),
                monto_recibido: 20.00,
                cambio: 3.50,
                estado: "COMPLETADA".to_string(),
                tipo_documento: "FACTURA".to_string(),
                estado_sri: "AUTORIZADO".to_string(),
                autorizacion_sri: Some("1802202601179245326800110010030000000011234567816".to_string()),
                clave_acceso: Some("1802202601179245326800110010030000000011234567816".to_string()),
                observacion: None,
                numero_factura: Some("001-003-000000001".to_string()),
            },
            detalles: vec![
                VentaDetalle {
                    id: Some(1),
                    venta_id: Some(1),
                    producto_id: 1,
                    nombre_producto: Some("Coca Cola 500ml".to_string()),
                    cantidad: 2.0,
                    precio_unitario: 1.25,
                    descuento: 0.0,
                    iva_porcentaje: 15.0,
                    subtotal: 2.50,
                },
                VentaDetalle {
                    id: Some(2),
                    venta_id: Some(1),
                    producto_id: 2,
                    nombre_producto: Some("Pan de agua".to_string()),
                    cantidad: 5.0,
                    precio_unitario: 0.15,
                    descuento: 0.0,
                    iva_porcentaje: 0.0,
                    subtotal: 0.75,
                },
                VentaDetalle {
                    id: Some(3),
                    venta_id: Some(1),
                    producto_id: 3,
                    nombre_producto: Some("Arroz Flor de Oro 1kg".to_string()),
                    cantidad: 1.0,
                    precio_unitario: 1.15,
                    descuento: 0.0,
                    iva_porcentaje: 0.0,
                    subtotal: 1.15,
                },
            ],
            cliente_nombre: Some("Juan Perez".to_string()),
        };

        let detalles_ride = vec![
            DetalleRide {
                codigo: "001".to_string(),
                nombre: "Coca Cola 500ml".to_string(),
                cantidad: 2.0,
                precio_unitario: 1.25,
                descuento: 0.0,
                iva_porcentaje: 15.0,
                precio_total_sin_impuesto: 2.50,
            },
            DetalleRide {
                codigo: "002".to_string(),
                nombre: "Pan de agua".to_string(),
                cantidad: 5.0,
                precio_unitario: 0.15,
                descuento: 0.0,
                iva_porcentaje: 0.0,
                precio_total_sin_impuesto: 0.75,
            },
            DetalleRide {
                codigo: "003".to_string(),
                nombre: "Arroz Flor de Oro 1kg".to_string(),
                cantidad: 1.0,
                precio_unitario: 1.15,
                descuento: 0.0,
                iva_porcentaje: 0.0,
                precio_total_sin_impuesto: 1.15,
            },
        ];

        let cliente = ClienteRide {
            nombre: "Juan Carlos Perez Lopez".to_string(),
            identificacion: "0102030405".to_string(),
            direccion: "Av. 10 de Agosto y Colon, Quito".to_string(),
            email: "juan.perez@email.com".to_string(),
            telefono: "0991234567".to_string(),
            observacion: "Cliente frecuente".to_string(),
        };

        let mut config = HashMap::new();
        config.insert("nombre_negocio".to_string(), "ABARROTES DON PEPE".to_string());
        config.insert("ruc".to_string(), "1792453268001".to_string());
        config.insert("direccion".to_string(), "Calle Sucre 123 y Bolivar, Ambato".to_string());
        config.insert("telefono".to_string(), "032-456789".to_string());
        config.insert("regimen".to_string(), "RIMPE_EMPRENDEDOR".to_string());
        config.insert("sri_ambiente".to_string(), "pruebas".to_string());

        let userprofile = std::env::var("USERPROFILE").unwrap_or_else(|_| ".".to_string());
        let desktop = std::path::PathBuf::from(&userprofile).join("Desktop");

        // Helper: crear PNG sólido de color (sin dependencias externas)
        fn crear_png_solido(ancho: u32, alto: u32, r: u8, g: u8, b: u8) -> Vec<u8> {
            let mut buf = Vec::new();
            // PNG signature
            buf.extend_from_slice(b"\x89PNG\r\n\x1a\n");
            // IHDR chunk
            let mut ihdr_data = Vec::new();
            ihdr_data.extend_from_slice(&ancho.to_be_bytes());
            ihdr_data.extend_from_slice(&alto.to_be_bytes());
            ihdr_data.push(8); // bit depth
            ihdr_data.push(2); // color type: RGB
            ihdr_data.push(0); // compression
            ihdr_data.push(0); // filter
            ihdr_data.push(0); // interlace
            let ihdr_crc = crc32_png(b"IHDR", &ihdr_data);
            buf.extend_from_slice(&(13u32).to_be_bytes()); // length
            buf.extend_from_slice(b"IHDR");
            buf.extend_from_slice(&ihdr_data);
            buf.extend_from_slice(&ihdr_crc.to_be_bytes());
            // IDAT chunk - raw image data con zlib
            let mut raw_data = Vec::new();
            for _ in 0..alto {
                raw_data.push(0); // filter: None
                for _ in 0..ancho {
                    raw_data.extend_from_slice(&[r, g, b]);
                }
            }
            // zlib: deflate stored (no compression)
            let mut zlib_data = Vec::new();
            zlib_data.push(0x78); // CMF
            zlib_data.push(0x01); // FLG
            // Split raw_data into blocks of max 65535 bytes
            let chunks: Vec<&[u8]> = raw_data.chunks(65535).collect();
            for (i, chunk) in chunks.iter().enumerate() {
                let is_last = i == chunks.len() - 1;
                zlib_data.push(if is_last { 1 } else { 0 }); // BFINAL
                let len = chunk.len() as u16;
                zlib_data.extend_from_slice(&len.to_le_bytes());
                zlib_data.extend_from_slice(&(!len).to_le_bytes());
                zlib_data.extend_from_slice(chunk);
            }
            // Adler32 checksum
            let adler = adler32(&raw_data);
            zlib_data.extend_from_slice(&adler.to_be_bytes());
            let idat_crc = crc32_png(b"IDAT", &zlib_data);
            buf.extend_from_slice(&(zlib_data.len() as u32).to_be_bytes());
            buf.extend_from_slice(b"IDAT");
            buf.extend_from_slice(&zlib_data);
            buf.extend_from_slice(&idat_crc.to_be_bytes());
            // IEND chunk
            let iend_crc = crc32_png(b"IEND", &[]);
            buf.extend_from_slice(&0u32.to_be_bytes());
            buf.extend_from_slice(b"IEND");
            buf.extend_from_slice(&iend_crc.to_be_bytes());
            buf
        }
        fn crc32_png(chunk_type: &[u8], data: &[u8]) -> u32 {
            let mut crc: u32 = 0xFFFFFFFF;
            for &byte in chunk_type.iter().chain(data.iter()) {
                let idx = ((crc ^ byte as u32) & 0xFF) as usize;
                let mut val = idx as u32;
                for _ in 0..8 {
                    if val & 1 != 0 { val = 0xEDB88320 ^ (val >> 1); }
                    else { val >>= 1; }
                }
                crc = val ^ (crc >> 8);
            }
            crc ^ 0xFFFFFFFF
        }
        fn adler32(data: &[u8]) -> u32 {
            let mut a: u32 = 1;
            let mut b: u32 = 0;
            for &byte in data {
                a = (a + byte as u32) % 65521;
                b = (b + a) % 65521;
            }
            (b << 16) | a
        }

        // --- 1. PDF con logo HORIZONTAL (300x100, azul) ---
        let logo_h = crear_png_solido(300, 100, 41, 98, 255); // azul
        config.insert("logo_negocio".to_string(), BASE64.encode(&logo_h));
        let result = generar_ride_pdf(&venta, &detalles_ride, &cliente, &config,
            Some("18/02/2026 10:35:00"), "FACTURA", None);
        assert!(result.is_ok(), "Error RIDE logo horizontal: {:?}", result.err());
        let path_h = desktop.join("RIDE_logo_horizontal.pdf");
        std::fs::write(&path_h, result.unwrap()).expect("Error guardando PDF");
        println!("1. Logo horizontal: {}", path_h.display());

        // --- 2. PDF con logo CUADRADO (150x150, verde) ---
        let logo_v = crear_png_solido(150, 150, 34, 197, 94); // verde
        config.insert("logo_negocio".to_string(), BASE64.encode(&logo_v));
        let result = generar_ride_pdf(&venta, &detalles_ride, &cliente, &config,
            Some("18/02/2026 10:35:00"), "FACTURA", None);
        assert!(result.is_ok(), "Error RIDE logo cuadrado: {:?}", result.err());
        let path_v = desktop.join("RIDE_logo_cuadrado.pdf");
        std::fs::write(&path_v, result.unwrap()).expect("Error guardando PDF");
        println!("2. Logo cuadrado:   {}", path_v.display());

        // --- 3. PDF con logo 300x150 (horizontal 2:1, rojo) ---
        let logo_2_1 = crear_png_solido(300, 150, 220, 38, 38); // rojo
        config.insert("logo_negocio".to_string(), BASE64.encode(&logo_2_1));
        let result = generar_ride_pdf(&venta, &detalles_ride, &cliente, &config,
            Some("18/02/2026 10:35:00"), "FACTURA", None);
        assert!(result.is_ok(), "Error RIDE logo 300x150: {:?}", result.err());
        let path_r = desktop.join("RIDE_logo_300x150.pdf");
        std::fs::write(&path_r, result.unwrap()).expect("Error guardando PDF");
        println!("3. Logo 300x150:    {}", path_r.display());

        // --- 4. PDF con logo 512x512 (cuadrado grande, naranja) ---
        let logo_512 = crear_png_solido(512, 512, 255, 152, 0); // naranja
        config.insert("logo_negocio".to_string(), BASE64.encode(&logo_512));
        let result = generar_ride_pdf(&venta, &detalles_ride, &cliente, &config,
            Some("18/02/2026 10:35:00"), "FACTURA", None);
        assert!(result.is_ok(), "Error RIDE logo 512x512: {:?}", result.err());
        let path_512 = desktop.join("RIDE_logo_512x512.pdf");
        std::fs::write(&path_512, result.unwrap()).expect("Error guardando PDF");
        println!("4. Logo 512x512:    {}", path_512.display());

        // --- 5. PDF SIN logo ---
        config.remove("logo_negocio");
        let result = generar_ride_pdf(&venta, &detalles_ride, &cliente, &config,
            Some("18/02/2026 10:35:00"), "FACTURA", None);
        assert!(result.is_ok(), "Error RIDE sin logo: {:?}", result.err());
        let path_s = desktop.join("RIDE_sin_logo.pdf");
        std::fs::write(&path_s, result.unwrap()).expect("Error guardando PDF");
        println!("5. Sin logo:        {}", path_s.display());

        // --- 6. PDF para PÁGINA WEB (logo real "Empresa de prueba") ---
        config.insert("nombre_negocio".to_string(), "EMPRESA DE PRUEBA".to_string());
        config.insert("ruc".to_string(), "0990012345001".to_string());
        config.insert("direccion".to_string(), "Av. Principal 456 y Secundaria, Guayaquil".to_string());
        config.insert("telefono".to_string(), "04-2567890".to_string());
        let logo_web_path = std::path::PathBuf::from(std::env::var("USERPROFILE").unwrap_or_else(|_| ".".to_string()))
            .join("Downloads").join("Empresa de prueba.png");
        let logo_web_bytes = std::fs::read(&logo_web_path).expect("No se encontró Empresa de prueba.png en Downloads");
        config.insert("logo_negocio".to_string(), BASE64.encode(&logo_web_bytes));
        let result = generar_ride_pdf(&venta, &detalles_ride, &cliente, &config,
            Some("18/02/2026 10:35:00"), "FACTURA", None);
        assert!(result.is_ok(), "Error RIDE web: {:?}", result.err());
        let path_web = desktop.join("RIDE_web_demo.pdf");
        std::fs::write(&path_web, result.unwrap()).expect("Error guardando PDF");
        println!("6. Demo web:        {}", path_web.display());
    }
}
