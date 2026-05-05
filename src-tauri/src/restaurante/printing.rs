//! Generador de tickets para el módulo Restaurante.
//!
//! Dos formatos:
//! - `generar_pre_cuenta`: ESC/POS para impresora térmica real (80mm).
//! - `generar_pre_cuenta_pdf`: PDF nativo (genpdf) para abrir en visor cuando
//!   la "impresora" configurada es virtual (Microsoft Print to PDF, OneNote,
//!   XPS, Fax) o cuando no hay impresora configurada.
//!
//! En ambos casos la pre-cuenta NO es comprobante fiscal — solo informativa.
//! La factura/nota de venta real se genera al cobrar vía `registrar_venta`.

use super::models::PedidoDetalle;
use crate::printing::{
    format_cantidad, linea_monto, linea_separador_doble, linea_separador_simple,
    logo_to_raster_pub,
};
use genpdf::elements::{Break, Paragraph, StyledElement, TableLayout};
use genpdf::style::Style;
use genpdf::{Alignment, Document, Element, Margins, SimplePageDecorator};
use std::collections::HashMap;

/// Ancho de impresora térmica de 80mm (48 caracteres) — mismo que tickets de venta.
const ANCHO: usize = 48;

/// Genera el ticket ESC/POS de pre-cuenta para una mesa.
/// Reutiliza la cabecera (logo + datos negocio) del ticket normal y agrega
/// info específica de mesa: nombre, mesero, comensales, hora apertura.
pub fn generar_pre_cuenta(
    detalle: &PedidoDetalle,
    config: &HashMap<String, String>,
) -> Vec<u8> {
    let mut ticket: Vec<u8> = Vec::new();

    // Comandos ESC/POS
    let esc_init: &[u8] = &[0x1B, 0x40];
    let esc_center: &[u8] = &[0x1B, 0x61, 0x01];
    let esc_left: &[u8] = &[0x1B, 0x61, 0x00];
    let esc_bold_on: &[u8] = &[0x1B, 0x45, 0x01];
    let esc_bold_off: &[u8] = &[0x1B, 0x45, 0x00];
    let esc_double_on: &[u8] = &[0x1B, 0x21, 0x30];
    let esc_double_off: &[u8] = &[0x1B, 0x21, 0x00];
    let esc_double_h: &[u8] = &[0x1B, 0x21, 0x10];
    let esc_cut: &[u8] = &[0x1D, 0x56, 0x00];
    let esc_feed: &[u8] = &[0x1B, 0x64, 0x04];

    ticket.extend_from_slice(esc_init);

    // === LOGO ===
    if let Some(logo_b64) = config.get("logo_negocio") {
        if !logo_b64.is_empty() {
            if let Some(raster_bytes) = logo_to_raster_pub(logo_b64, 300) {
                ticket.extend_from_slice(esc_center);
                ticket.extend_from_slice(&raster_bytes);
                ticket.push(b'\n');
            }
        }
    }

    // === Encabezado negocio ===
    ticket.extend_from_slice(esc_center);
    ticket.extend_from_slice(esc_bold_on);
    ticket.extend_from_slice(esc_double_on);
    let nombre = config
        .get("nombre_negocio")
        .map(|s| s.as_str())
        .unwrap_or("MI NEGOCIO");
    ticket.extend_from_slice(nombre.as_bytes());
    ticket.push(b'\n');
    ticket.extend_from_slice(esc_double_off);
    ticket.extend_from_slice(esc_bold_off);

    if let Some(dir) = config.get("direccion") {
        if !dir.is_empty() {
            ticket.extend_from_slice(format!("{}\n", dir).as_bytes());
        }
    }
    if let Some(tel) = config.get("telefono") {
        if !tel.is_empty() {
            ticket.extend_from_slice(format!("Tel: {}\n", tel).as_bytes());
        }
    }

    ticket.extend_from_slice(esc_left);
    ticket.extend_from_slice(linea_separador_doble(ANCHO).as_bytes());

    // === TÍTULO PRE-CUENTA — prominente, centrado, doble alto ===
    ticket.extend_from_slice(esc_center);
    ticket.extend_from_slice(esc_bold_on);
    ticket.extend_from_slice(esc_double_h);
    ticket.extend_from_slice(b"PRE-CUENTA\n");
    ticket.extend_from_slice(esc_double_off);
    ticket.extend_from_slice(esc_bold_off);
    ticket.extend_from_slice(esc_left);
    ticket.extend_from_slice(linea_separador_simple(ANCHO).as_bytes());

    // === Info mesa ===
    let zona = detalle
        .zona_nombre
        .as_deref()
        .map(|z| format!(" ({})", z))
        .unwrap_or_default();
    ticket.extend_from_slice(format!("Mesa: {}{}\n", detalle.mesa_nombre, zona).as_bytes());
    if let Some(ref mesero) = detalle.pedido.mesero_nombre {
        ticket.extend_from_slice(format!("Mesero: {}\n", mesero).as_bytes());
    }
    ticket.extend_from_slice(
        format!("Comensales: {}\n", detalle.pedido.comensales).as_bytes(),
    );
    if let Some(ref apertura) = detalle.pedido.fecha_apertura {
        ticket.extend_from_slice(format!("Apertura: {}\n", apertura).as_bytes());
    }
    ticket.extend_from_slice(
        format!(
            "Pedido: #{}\n",
            detalle.pedido.id.unwrap_or(0)
        )
        .as_bytes(),
    );

    ticket.extend_from_slice(linea_separador_simple(ANCHO).as_bytes());

    // === Cabecera detalle ===
    ticket.extend_from_slice(esc_bold_on);
    ticket.extend_from_slice(
        format!(
            "{:<22} {:>5} {:>8} {:>9}\n",
            "PRODUCTO", "CANT", "P.UNIT", "SUBTOT"
        )
        .as_bytes(),
    );
    ticket.extend_from_slice(esc_bold_off);
    ticket.extend_from_slice(linea_separador_simple(ANCHO).as_bytes());

    // === Items (agrupados por producto+observación) ===
    // Para mejor lectura, agrupamos items idénticos (mismo producto + misma obs)
    use std::collections::BTreeMap;
    #[derive(Clone)]
    struct Linea {
        nombre: String,
        info: Option<String>,
        cantidad: f64,
        precio: f64,
        subtotal: f64,
    }
    let mut grupos: BTreeMap<String, Linea> = BTreeMap::new();
    for it in &detalle.items {
        let nombre = it.producto_nombre.clone().unwrap_or_else(|| "?".into());
        let info_key = it.info_adicional.clone().unwrap_or_default();
        let key = format!("{}|{}", nombre, info_key);
        grupos
            .entry(key)
            .and_modify(|g| {
                g.cantidad += it.cantidad;
                g.subtotal += it.cantidad * it.precio_unit;
            })
            .or_insert(Linea {
                nombre: nombre.clone(),
                info: it.info_adicional.clone(),
                cantidad: it.cantidad,
                precio: it.precio_unit,
                subtotal: it.cantidad * it.precio_unit,
            });
    }

    for grupo in grupos.values() {
        let nombre_corto: String = if grupo.nombre.len() > 22 {
            grupo.nombre[..22].to_string()
        } else {
            grupo.nombre.clone()
        };

        ticket.extend_from_slice(
            format!(
                "{:<22} {:>5} {:>8.2} {:>9.2}\n",
                nombre_corto,
                format_cantidad(grupo.cantidad),
                grupo.precio,
                grupo.subtotal
            )
            .as_bytes(),
        );

        if let Some(ref info) = grupo.info {
            if !info.is_empty() {
                ticket.extend_from_slice(format!("  {}\n", info).as_bytes());
            }
        }
    }

    ticket.extend_from_slice(linea_separador_doble(ANCHO).as_bytes());

    // === Totales (sin desglose IVA — la pre-cuenta es informativa) ===
    ticket.extend_from_slice(linea_monto("Subtotal:", detalle.subtotal, ANCHO).as_bytes());
    if detalle.iva > 0.0 {
        ticket.extend_from_slice(linea_monto("IVA 15%:", detalle.iva, ANCHO).as_bytes());
    }

    ticket.extend_from_slice(esc_bold_on);
    ticket.extend_from_slice(esc_double_on);
    ticket.extend_from_slice(esc_center);
    ticket.extend_from_slice(format!("TOTAL: ${:.2}\n", detalle.total).as_bytes());
    ticket.extend_from_slice(esc_double_off);
    ticket.extend_from_slice(esc_bold_off);
    ticket.extend_from_slice(esc_left);

    ticket.extend_from_slice(linea_separador_simple(ANCHO).as_bytes());

    // === Aviso fiscal ===
    ticket.extend_from_slice(esc_center);
    ticket.extend_from_slice(esc_bold_on);
    ticket.extend_from_slice(b"ESTE DOCUMENTO NO ES UN\n");
    ticket.extend_from_slice(b"COMPROBANTE FISCAL\n");
    ticket.extend_from_slice(esc_bold_off);
    ticket.extend_from_slice(b"Solicite su factura al pagar\n");
    ticket.extend_from_slice(esc_left);

    ticket.extend_from_slice(linea_separador_doble(ANCHO).as_bytes());

    // Pie
    ticket.extend_from_slice(esc_center);
    ticket.extend_from_slice(b"Gracias por su visita!\n");
    ticket.extend_from_slice(esc_left);

    // Cortar
    ticket.extend_from_slice(esc_feed);
    ticket.extend_from_slice(esc_cut);

    ticket
}

// ─── PDF nativo (genpdf) ────────────────────────────────────────────────
// Para casos donde la "impresora" es virtual (Microsoft Print to PDF, etc.)
// o no hay impresora configurada. Genera un PDF legible en formato 80mm
// que se puede abrir con el visor del sistema o enviar por WhatsApp.

/// Helper local: párrafo con estilo
fn p_pdf(text: &str, style: Style) -> StyledElement<Paragraph> {
    Paragraph::new(text).styled(style)
}

/// Helper local: párrafo alineado + estilo
fn p_aligned_pdf(text: &str, style: Style, align: Alignment) -> impl Element {
    Paragraph::new(text).aligned(align).styled(style)
}

fn fmt_dinero(val: f64) -> String {
    format!("{:.2}", val)
}

/// Genera la pre-cuenta como PDF (formato 80mm) usando genpdf.
/// Retorna los bytes del PDF para guardar en disco y abrir con el visor del sistema.
pub fn generar_pre_cuenta_pdf(
    detalle: &PedidoDetalle,
    config: &HashMap<String, String>,
) -> Result<Vec<u8>, String> {
    let fonts_dir = crate::utils::obtener_ruta_fuentes();

    let font_family = genpdf::fonts::from_files(
        fonts_dir.to_str().unwrap_or("fonts"),
        "LiberationSans",
        None,
    )
    .map_err(|e| format!("Error cargando fuentes: {}", e))?;

    let mut doc = Document::new(font_family);
    doc.set_title(format!("Pre-cuenta {}", detalle.mesa_nombre));
    // 80mm width, 250mm alto generoso
    doc.set_paper_size(genpdf::Size::new(80, 250));

    let mut decorator = SimplePageDecorator::new();
    decorator.set_margins(Margins::trbl(3, 3, 3, 3));
    doc.set_page_decorator(decorator);

    let s_title = Style::new().with_font_size(12).bold();
    let s_normal = Style::new().with_font_size(8);
    let s_bold = Style::new().with_font_size(8).bold();
    let s_small = Style::new().with_font_size(7);
    let s_total = Style::new().with_font_size(11).bold();
    let s_aviso = Style::new().with_font_size(7).bold();

    // Encabezado negocio
    let nombre_negocio = config
        .get("nombre_negocio")
        .map(|s| s.as_str())
        .unwrap_or("MI NEGOCIO");
    doc.push(p_aligned_pdf(nombre_negocio, s_title, Alignment::Center));
    if let Some(dir) = config.get("direccion") {
        if !dir.is_empty() {
            doc.push(p_aligned_pdf(dir, s_small, Alignment::Center));
        }
    }
    if let Some(tel) = config.get("telefono") {
        if !tel.is_empty() {
            doc.push(p_aligned_pdf(
                &format!("Tel: {}", tel),
                s_small,
                Alignment::Center,
            ));
        }
    }

    doc.push(Break::new(0.3));

    // Título PRE-CUENTA
    doc.push(p_aligned_pdf("PRE-CUENTA", s_total, Alignment::Center));

    doc.push(Break::new(0.3));

    // Info mesa
    let zona_str = detalle
        .zona_nombre
        .as_deref()
        .map(|z| format!(" ({})", z))
        .unwrap_or_default();
    doc.push(p_pdf(
        &format!("Mesa: {}{}", detalle.mesa_nombre, zona_str),
        s_bold,
    ));
    if let Some(ref mesero) = detalle.pedido.mesero_nombre {
        doc.push(p_pdf(&format!("Mesero: {}", mesero), s_normal));
    }
    doc.push(p_pdf(
        &format!("Comensales: {}", detalle.pedido.comensales),
        s_normal,
    ));
    if let Some(ref apertura) = detalle.pedido.fecha_apertura {
        doc.push(p_pdf(&format!("Apertura: {}", apertura), s_normal));
    }
    doc.push(p_pdf(
        &format!("Pedido: #{}", detalle.pedido.id.unwrap_or(0)),
        s_normal,
    ));

    doc.push(p_aligned_pdf(
        &"-".repeat(160),
        s_small,
        Alignment::Left,
    ));
    doc.push(Break::new(0.2));

    // Tabla de items: Cant | Producto | Total
    let mut prod_table = TableLayout::new(vec![1, 4, 2]);
    prod_table
        .row()
        .element(p_pdf("Cant", s_bold))
        .element(p_pdf("Producto", s_bold))
        .element(p_aligned_pdf("Total", s_bold, Alignment::Right))
        .push()
        .map_err(|e| format!("Error tabla header: {}", e))?;

    // Agrupar items idénticos (mismo producto + misma observación)
    use std::collections::BTreeMap;
    #[derive(Clone)]
    struct Linea {
        nombre: String,
        info: Option<String>,
        cantidad: f64,
        subtotal: f64,
    }
    let mut grupos: BTreeMap<String, Linea> = BTreeMap::new();
    for it in &detalle.items {
        let nombre = it.producto_nombre.clone().unwrap_or_else(|| "?".into());
        let info_key = it.info_adicional.clone().unwrap_or_default();
        let key = format!("{}|{}", nombre, info_key);
        grupos
            .entry(key)
            .and_modify(|g| {
                g.cantidad += it.cantidad;
                g.subtotal += it.cantidad * it.precio_unit;
            })
            .or_insert(Linea {
                nombre: nombre.clone(),
                info: it.info_adicional.clone(),
                cantidad: it.cantidad,
                subtotal: it.cantidad * it.precio_unit,
            });
    }

    for grupo in grupos.values() {
        prod_table
            .row()
            .element(p_pdf(&format_cantidad(grupo.cantidad), s_normal))
            .element(p_pdf(&grupo.nombre, s_normal))
            .element(p_aligned_pdf(
                &fmt_dinero(grupo.subtotal),
                s_normal,
                Alignment::Right,
            ))
            .push()
            .map_err(|e| format!("Error tabla fila: {}", e))?;

        if let Some(ref info) = grupo.info {
            if !info.is_empty() {
                prod_table
                    .row()
                    .element(p_pdf("", s_small))
                    .element(p_pdf(&format!("  ↳ {}", info), s_small))
                    .element(p_pdf("", s_small))
                    .push()
                    .map_err(|e| format!("Error tabla info: {}", e))?;
            }
        }
    }
    doc.push(prod_table);

    doc.push(Break::new(0.3));
    doc.push(p_aligned_pdf(
        &"=".repeat(160),
        s_small,
        Alignment::Left,
    ));

    // Totales
    if detalle.iva > 0.0 {
        doc.push(p_pdf(
            &format!("Subtotal: {}", fmt_dinero(detalle.subtotal)),
            s_normal,
        ));
        doc.push(p_pdf(
            &format!("IVA 15%: {}", fmt_dinero(detalle.iva)),
            s_normal,
        ));
    }

    doc.push(Break::new(0.2));
    doc.push(p_aligned_pdf(
        &format!("TOTAL: ${}", fmt_dinero(detalle.total)),
        s_total,
        Alignment::Center,
    ));

    doc.push(Break::new(0.5));

    // Aviso fiscal
    doc.push(p_aligned_pdf(
        "ESTE DOCUMENTO NO ES UN",
        s_aviso,
        Alignment::Center,
    ));
    doc.push(p_aligned_pdf(
        "COMPROBANTE FISCAL",
        s_aviso,
        Alignment::Center,
    ));
    doc.push(p_aligned_pdf(
        "Solicite su factura al pagar",
        s_small,
        Alignment::Center,
    ));

    doc.push(Break::new(0.3));
    doc.push(p_aligned_pdf(
        "Gracias por su visita!",
        s_small,
        Alignment::Center,
    ));

    // Renderizar a bytes
    let mut buffer: Vec<u8> = Vec::new();
    doc.render(&mut buffer)
        .map_err(|e| format!("Error generando PDF: {}", e))?;

    Ok(buffer)
}
