use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use genpdf::elements::{Break, LinearLayout, Paragraph, StyledElement, TableLayout};
use genpdf::style::{Style, Color};
use genpdf::{Alignment, Document, Element, Margins, SimplePageDecorator};
use std::collections::HashMap;

use crate::models::VentaCompleta;

/// Prefijo de padding lateral para textos dentro de secciones enmarcadas
const PAD: &str = "  ";

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

/// Paragraph con padding lateral (prefijo de espacios)
fn pp(text: &str, style: Style) -> StyledElement<Paragraph> {
    Paragraph::new(format!("{}{}", PAD, text)).styled(style)
}

/// Paragraph con padding lateral + alineado
fn pp_aligned(text: &str, style: Style, align: Alignment) -> impl Element {
    Paragraph::new(format!("{}{}", PAD, text)).aligned(align).styled(style)
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
    decorator.set_margins(Margins::trbl(20, 25, 20, 25));
    doc.set_page_decorator(decorator);

    // Estilos
    let s_normal = Style::new().with_font_size(8);
    let s_bold = Style::new().with_font_size(8).bold();
    let s_small = Style::new().with_font_size(7);
    let s_small_bold = Style::new().with_font_size(7).bold();
    let s_title = Style::new().with_font_size(12).bold();
    let s_doc_type = Style::new().with_font_size(14).bold();
    let s_doc_no = Style::new().with_font_size(11);
    let s_ruc = Style::new().with_font_size(9).bold();
    let s_clave = Style::new().with_font_size(7);
    let s_clave_small = Style::new().with_font_size(6);
    let s_total_bold = Style::new().with_font_size(10).bold();
    let s_pie = Style::new().with_font_size(7).with_color(Color::Greyscale(128));
    let s_regimen = Style::new().with_font_size(7).bold();

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
    // SECCION 1: ENCABEZADO - Dos recuadros lado a lado
    // ===================================================================
    let mut header_table = TableLayout::new(vec![1, 1]);

    // --- Columna izquierda: Datos del emisor ---
    let mut col_izq = LinearLayout::vertical();
    col_izq.push(Break::new(0.5));

    // Logo del negocio (si existe en config como base64) - formato horizontal
    if let Some(logo_b64) = config.get("logo_negocio") {
        if !logo_b64.is_empty() {
            if let Ok(logo_bytes) = BASE64.decode(logo_b64) {
                let logo_temp = std::env::temp_dir().join("clouget_ride_logo.png");
                if std::fs::write(&logo_temp, &logo_bytes).is_ok() {
                    if let Ok(mut logo_img) = genpdf::elements::Image::from_path(&logo_temp) {
                        logo_img = logo_img.with_alignment(Alignment::Center);
                        // Escala mayor para aprovechar espacio disponible
                        logo_img = logo_img.with_scale(genpdf::Scale::new(0.6, 0.45));
                        col_izq.push(logo_img);
                        col_izq.push(Break::new(0.3));
                    }
                    let _ = std::fs::remove_file(&logo_temp);
                }
            }
        }
    }

    col_izq.push(pp(nombre_negocio, s_title));
    col_izq.push(Break::new(0.5));
    if !ruc.is_empty() {
        col_izq.push(pp(&format!("RUC: {}", ruc), s_bold));
    }
    if !direccion_neg.is_empty() {
        col_izq.push(Break::new(0.5));
        col_izq.push(pp(&format!("Direccion Matriz: {}", direccion_neg), s_normal));
        col_izq.push(pp(&format!("Direccion Sucursal: {}", direccion_neg), s_normal));
    }
    if !telefono_neg.is_empty() {
        col_izq.push(pp(&format!("Tel: {}", telefono_neg), s_normal));
    }
    col_izq.push(Break::new(0.5));
    col_izq.push(pp("OBLIGADO A LLEVAR CONTABILIDAD: NO", s_bold));
    if !regimen_label.is_empty() {
        col_izq.push(Break::new(0.5));
        col_izq.push(pp(regimen_label, s_regimen));
    }
    col_izq.push(Break::new(0.5));

    // --- Columna derecha: Datos del documento + clave de acceso ---
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

    // Clave de acceso + código de barras integrados en columna derecha
    col_der.push(pp("CLAVE DE ACCESO:", s_bold));
    if !clave_acceso.is_empty() {
        match generar_barcode128_image(clave_acceso) {
            Ok(barcode_path) => {
                if let Ok(mut barcode_img) = genpdf::elements::Image::from_path(&barcode_path) {
                    barcode_img = barcode_img.with_alignment(Alignment::Center);
                    barcode_img = barcode_img.with_scale(genpdf::Scale::new(0.55, 0.5));
                    col_der.push(barcode_img);
                }
                let _ = std::fs::remove_file(&barcode_path);
            }
            Err(e) => {
                eprintln!("Warning: No se pudo generar barcode Code128: {}", e);
            }
        }
    }
    col_der.push(p_aligned(clave_acceso, s_clave_small, Alignment::Center));
    col_der.push(Break::new(0.3));

    // Envolver cada columna en un borde (framed)
    header_table
        .row()
        .element(col_izq.framed())
        .element(col_der.framed())
        .push()
        .map_err(|e| format!("Error tabla header: {}", e))?;

    doc.push(header_table);
    doc.push(Break::new(0.5));

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
    comprador_section.push(Break::new(0.3));

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
    comprador_section.push(Break::new(0.3));

    doc.push(comprador_section.framed());
    doc.push(Break::new(0.5));

    // ===================================================================
    // SECCION 4: TABLA DE PRODUCTOS
    // ===================================================================
    let mut table = TableLayout::new(vec![2, 1, 5, 2, 1, 2]);
    table.set_cell_decorator(genpdf::elements::FrameCellDecorator::new(true, true, false));

    // Header
    table
        .row()
        .element(pp("Codigo", s_small_bold))
        .element(pp("Cant.", s_small_bold))
        .element(pp("Descripcion", s_small_bold))
        .element(pp("P. Unit.", s_small_bold))
        .element(pp("Desc.", s_small_bold))
        .element(pp("Subtotal", s_small_bold))
        .push()
        .map_err(|e| format!("Error tabla header: {}", e))?;

    // Filas de productos
    for det in detalles_ride {
        table
            .row()
            .element(pp(&det.codigo, s_small))
            .element(pp(&format_cantidad(det.cantidad), s_small))
            .element(pp(&det.nombre, s_small))
            .element(pp(&format_dinero(det.precio_unitario), s_small))
            .element(pp(&format_dinero(det.descuento), s_small))
            .element(pp(&format_dinero(det.precio_total_sin_impuesto), s_small))
            .push()
            .map_err(|e| format!("Error tabla fila: {}", e))?;
    }

    doc.push(table);
    doc.push(Break::new(1));

    // ===================================================================
    // SECCION 5: INFO ADICIONAL + TOTALES (2 columnas)
    // ===================================================================
    let mut bottom_table = TableLayout::new(vec![11, 9]);

    // --- Columna izquierda: Info adicional + Forma de pago ---
    let mut info_col = LinearLayout::vertical();
    info_col.push(Break::new(0.5));
    info_col.push(pp("Informacion Adicional", s_bold));

    if !cliente.direccion.is_empty() {
        info_col.push(pp(&format!("Direccion:  {}", cliente.direccion), s_small));
    }
    if !cliente.email.is_empty() {
        info_col.push(pp(&format!("Email:  {}", cliente.email), s_small));
    }
    if !cliente.telefono.is_empty() {
        info_col.push(pp(&format!("Telefono:  {}", cliente.telefono), s_small));
    }
    if !cliente.observacion.is_empty() {
        info_col.push(pp(&format!("Observacion:  {}", cliente.observacion), s_small));
    }

    info_col.push(Break::new(1));

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
        .element(pp(&format_dinero(venta.venta.total), s_small))
        .push()
        .map_err(|e| format!("Error forma pago fila: {}", e))?;

    info_col.push(pago_table);
    info_col.push(Break::new(0.5));

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
            .element(pp(&format_dinero(*valor), *style))
            .push()
            .map_err(|e| format!("Error totales fila: {}", e))?;
    }

    // VALOR TOTAL (bold, mas grande)
    totales_table
        .row()
        .element(pp("VALOR TOTAL", s_total_bold))
        .element(pp(&format_dinero(venta.venta.total), s_total_bold))
        .push()
        .map_err(|e| format!("Error totales valor total: {}", e))?;

    totales_col.push(totales_table);

    // Juntar las dos columnas
    bottom_table
        .row()
        .element(info_col.framed())
        .element(totales_col)
        .push()
        .map_err(|e| format!("Error tabla bottom: {}", e))?;

    doc.push(bottom_table);
    doc.push(Break::new(2));

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
    let height = 60_u32;
    let scale_x = 3_u32;
    let quiet_zone = 10_u32; // margen lateral blanco
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
