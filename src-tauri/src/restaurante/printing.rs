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

use super::models::{PedidoDetalle, PedidoItem};
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

// ─── Comanda de Cocina (v2.3.67) ────────────────────────────────────────
// Ticket impreso automáticamente al "Enviar cocina" para que el cocinero
// vea qué preparar. Diseñado simple y grande:
//   - SIN precios (cocina no necesita verlos)
//   - Cantidades en negrita y grandes
//   - Observaciones (sin cebolla, etc.) muy visibles
//   - Cabecera grande con # mesa para que se lea desde lejos

/// Filtro: qué tipo de comanda generar.
/// COCINA = items con destino_preparacion='COCINA'
/// BARRA  = items con destino_preparacion='BARRA'
/// AMBOS  = todos (1 ticket combinado)
pub enum DestinoComanda {
    Cocina,
    Barra,
    Ambos,
}

/// v2.4.5 — Genera la comanda de cocina como PDF (formato 80mm) usando genpdf.
/// Equivalente PDF de `generar_comanda_cocina` (que genera ESC/POS).
/// Se usa cuando la "impresora de cocina" configurada es virtual
/// (Microsoft Print to PDF, OneNote, etc.) o no hay impresora.
///
/// Retorna `Ok(None)` si el filtro deja la lista vacía (todos DIRECTO o
/// destino opuesto) — caller debe omitir la generación silenciosamente.
pub fn generar_comanda_cocina_pdf(
    pedido_id: i64,
    mesa_nombre: &str,
    zona_nombre: Option<&str>,
    mesero: Option<&str>,
    items: &[PedidoItem],
    filtro: DestinoComanda,
    config: &HashMap<String, String>,
) -> Result<Option<Vec<u8>>, String> {
    let items_filtrados: Vec<&PedidoItem> = items.iter().filter(|i| match filtro {
        DestinoComanda::Cocina => i.destino_preparacion == "COCINA",
        DestinoComanda::Barra  => i.destino_preparacion == "BARRA",
        DestinoComanda::Ambos  => i.destino_preparacion != "DIRECTO",
    }).collect();

    if items_filtrados.is_empty() {
        return Ok(None);
    }

    let fonts_dir = crate::utils::obtener_ruta_fuentes();
    let font_family = genpdf::fonts::from_files(
        fonts_dir.to_str().unwrap_or("fonts"),
        "LiberationSans",
        None,
    )
    .map_err(|e| format!("Error cargando fuentes: {}", e))?;

    let mut doc = Document::new(font_family);
    let titulo_doc = format!("Comanda Mesa {} (Ped #{})", mesa_nombre, pedido_id);
    doc.set_title(titulo_doc);
    // 80mm width, 250mm alto generoso
    doc.set_paper_size(genpdf::Size::new(80, 250));

    let mut decorator = SimplePageDecorator::new();
    decorator.set_margins(Margins::trbl(3, 3, 3, 3));
    doc.set_page_decorator(decorator);

    let s_titulo_destino = Style::new().with_font_size(14).bold();
    let s_mesa_grande   = Style::new().with_font_size(18).bold();
    let s_normal = Style::new().with_font_size(8);
    let s_bold   = Style::new().with_font_size(8).bold();
    let s_item   = Style::new().with_font_size(11).bold();
    let s_obs    = Style::new().with_font_size(9).bold();
    let s_small  = Style::new().with_font_size(7);
    let s_barra  = Style::new().with_font_size(8).bold();

    // ── Título según destino ──
    let titulo = match filtro {
        DestinoComanda::Cocina => "COCINA",
        DestinoComanda::Barra  => "BARRA",
        DestinoComanda::Ambos  => "COMANDA",
    };
    doc.push(p_aligned_pdf(titulo, s_titulo_destino, Alignment::Center));
    doc.push(p_aligned_pdf("================", s_normal, Alignment::Center));

    // ── Mesa cabecera GRANDE para lectura desde lejos ──
    let mesa_full = if let Some(z) = zona_nombre {
        format!("MESA: {} ({})", mesa_nombre, z)
    } else {
        format!("MESA: {}", mesa_nombre)
    };
    doc.push(p_aligned_pdf(&mesa_full, s_mesa_grande, Alignment::Center));
    doc.push(p_aligned_pdf("================", s_normal, Alignment::Center));

    // ── Datos pedido ──
    if let Some(m) = mesero {
        if !m.is_empty() {
            doc.push(p_aligned_pdf(&format!("Mesero: {}", m), s_normal, Alignment::Left));
        }
    }
    let hora = chrono::Local::now().format("%H:%M:%S").to_string();
    doc.push(p_aligned_pdf(
        &format!("Hora: {} · Pedido #{}", hora, pedido_id),
        s_normal,
        Alignment::Left,
    ));
    if let Some(neg) = config.get("nombre_negocio") {
        if !neg.is_empty() {
            doc.push(p_aligned_pdf(&format!("({})", neg), s_small, Alignment::Left));
        }
    }
    doc.push(p_aligned_pdf("----------------", s_normal, Alignment::Center));

    // ── Items: cantidad x nombre + obs/info_adicional + tag [BARRA] si Ambos ──
    for it in &items_filtrados {
        let nombre = it.producto_nombre.as_deref().unwrap_or("(sin nombre)");
        let cant = if it.cantidad.fract() == 0.0 {
            format!("{}", it.cantidad as i64)
        } else {
            format!("{:.2}", it.cantidad)
        };

        // En modo Ambos, marcar items de BARRA con tag visible
        let tag_barra = if matches!(filtro, DestinoComanda::Ambos) && it.destino_preparacion == "BARRA" {
            doc.push(p_aligned_pdf("[BARRA]", s_barra, Alignment::Left));
            true
        } else {
            false
        };
        let _ = tag_barra; // placeholder (visual ya añadido)

        doc.push(p_aligned_pdf(
            &format!("{}x  {}", cant, nombre),
            s_item,
            Alignment::Left,
        ));

        if let Some(info) = it.info_adicional.as_deref() {
            if !info.is_empty() {
                doc.push(p_aligned_pdf(&format!("  -> {}", info), s_obs, Alignment::Left));
            }
        }
        doc.push(Break::new(0.3));
    }

    doc.push(p_aligned_pdf("================", s_normal, Alignment::Center));
    doc.push(p_aligned_pdf(
        &format!("Total items: {}", items_filtrados.len()),
        s_bold,
        Alignment::Center,
    ));

    let mut buf: Vec<u8> = Vec::new();
    doc.render(&mut buf).map_err(|e| format!("Error generando PDF comanda: {}", e))?;
    Ok(Some(buf))
}

/// Genera el ticket ESC/POS de comanda de cocina.
/// Recibe los items YA enviados a cocina (después de UPDATE enviado_cocina=1).
///
/// Si el filtro es Cocina/Barra, solo incluye items de ese destino.
/// Items DIRECTO se IGNORAN (el mesero los entrega del mostrador).
///
/// Retorna None si después del filtro no hay items que imprimir
/// (ej: pedido solo tiene items DIRECTO o solo items del otro destino).
pub fn generar_comanda_cocina(
    pedido_id: i64,
    mesa_nombre: &str,
    zona_nombre: Option<&str>,
    mesero: Option<&str>,
    items: &[PedidoItem],
    filtro: DestinoComanda,
    config: &HashMap<String, String>,
) -> Option<Vec<u8>> {
    // Filtrar items según el destino de la comanda
    let items_filtrados: Vec<&PedidoItem> = items.iter().filter(|i| {
        match filtro {
            DestinoComanda::Cocina => i.destino_preparacion == "COCINA",
            DestinoComanda::Barra  => i.destino_preparacion == "BARRA",
            DestinoComanda::Ambos  => i.destino_preparacion != "DIRECTO", // todo excepto directo
        }
    }).collect();

    if items_filtrados.is_empty() {
        return None;
    }

    let mut ticket: Vec<u8> = Vec::new();

    // Comandos ESC/POS
    let esc_init: &[u8] = &[0x1B, 0x40];
    let esc_center: &[u8] = &[0x1B, 0x61, 0x01];
    let esc_left: &[u8] = &[0x1B, 0x61, 0x00];
    let esc_bold_on: &[u8] = &[0x1B, 0x45, 0x01];
    let esc_bold_off: &[u8] = &[0x1B, 0x45, 0x00];
    let esc_double_on: &[u8] = &[0x1B, 0x21, 0x30]; // doble alto+ancho (HUGE)
    let esc_double_off: &[u8] = &[0x1B, 0x21, 0x00];
    let esc_double_h: &[u8] = &[0x1B, 0x21, 0x10]; // solo doble alto
    let esc_cut: &[u8] = &[0x1D, 0x56, 0x00];
    let esc_feed: &[u8] = &[0x1B, 0x64, 0x05];

    ticket.extend_from_slice(esc_init);

    // === TÍTULO según destino ===
    let titulo = match filtro {
        DestinoComanda::Cocina => "🍳 COCINA",
        DestinoComanda::Barra  => "🍷 BARRA",
        DestinoComanda::Ambos  => "🍽 COMANDA",
    };
    ticket.extend_from_slice(esc_center);
    ticket.extend_from_slice(esc_bold_on);
    ticket.extend_from_slice(esc_double_on);
    ticket.extend_from_slice(titulo.as_bytes());
    ticket.push(b'\n');
    ticket.extend_from_slice(esc_double_off);
    ticket.extend_from_slice(esc_bold_off);
    ticket.extend_from_slice(esc_left);

    ticket.extend_from_slice(linea_separador_doble(ANCHO).as_bytes());

    // === Cabecera grande: MESA (lo más importante de leer rápido) ===
    ticket.extend_from_slice(esc_center);
    ticket.extend_from_slice(esc_bold_on);
    ticket.extend_from_slice(esc_double_h);
    let mesa_label = match zona_nombre {
        Some(z) => format!("MESA: {} ({})", mesa_nombre, z),
        None => format!("MESA: {}", mesa_nombre),
    };
    ticket.extend_from_slice(mesa_label.as_bytes());
    ticket.push(b'\n');
    ticket.extend_from_slice(esc_double_off);
    ticket.extend_from_slice(esc_bold_off);
    ticket.extend_from_slice(esc_left);

    // === Info contexto (más pequeño) ===
    if let Some(m) = mesero {
        ticket.extend_from_slice(format!("Mesero: {}\n", m).as_bytes());
    }
    let hora = chrono::Local::now().format("%H:%M:%S").to_string();
    let nombre_negocio = config.get("nombre_negocio").map(|s| s.as_str()).unwrap_or("");
    ticket.extend_from_slice(format!("Hora: {} · Pedido #{}\n", hora, pedido_id).as_bytes());
    if !nombre_negocio.is_empty() {
        ticket.extend_from_slice(format!("({})\n", nombre_negocio).as_bytes());
    }

    ticket.extend_from_slice(linea_separador_simple(ANCHO).as_bytes());

    // === Items: agrupados por nombre+observación, sin precios ===
    use std::collections::BTreeMap;
    #[derive(Clone)]
    struct LineaCocina {
        nombre: String,
        info: Option<String>,
        cantidad: f64,
        destino: String,
    }
    let mut grupos: BTreeMap<String, LineaCocina> = BTreeMap::new();
    for it in &items_filtrados {
        let nombre = it.producto_nombre.clone().unwrap_or_else(|| "?".into());
        let info_key = it.info_adicional.clone().unwrap_or_default();
        let key = format!("{}|{}|{}", nombre, info_key, it.destino_preparacion);
        grupos
            .entry(key)
            .and_modify(|g| g.cantidad += it.cantidad)
            .or_insert(LineaCocina {
                nombre: nombre.clone(),
                info: it.info_adicional.clone(),
                cantidad: it.cantidad,
                destino: it.destino_preparacion.clone(),
            });
    }

    for grupo in grupos.values() {
        // Cantidad en negrita doble alto, nombre normal
        ticket.extend_from_slice(esc_bold_on);
        ticket.extend_from_slice(esc_double_h);
        ticket.extend_from_slice(format!("{}x  ", format_cantidad(grupo.cantidad)).as_bytes());
        ticket.extend_from_slice(esc_double_off);
        ticket.extend_from_slice(grupo.nombre.as_bytes());
        // Si es comanda combinada AMBOS, marcar de qué destino es
        if matches!(filtro, DestinoComanda::Ambos) && grupo.destino == "BARRA" {
            ticket.extend_from_slice(b" [BARRA]");
        }
        ticket.push(b'\n');
        ticket.extend_from_slice(esc_bold_off);

        // Observación destacada con flecha + indentada
        if let Some(ref info) = grupo.info {
            if !info.is_empty() {
                ticket.extend_from_slice(esc_bold_on);
                ticket.extend_from_slice(format!("    ↳ {}\n", info).as_bytes());
                ticket.extend_from_slice(esc_bold_off);
            }
        }
        // Espacio entre items para mejor lectura
        ticket.push(b'\n');
    }

    ticket.extend_from_slice(linea_separador_doble(ANCHO).as_bytes());

    // Pie pequeño
    ticket.extend_from_slice(esc_center);
    ticket.extend_from_slice(format!("{} item(s) — {}\n", grupos.len(), hora).as_bytes());
    ticket.extend_from_slice(esc_left);

    // Cortar
    ticket.extend_from_slice(esc_feed);
    ticket.extend_from_slice(esc_cut);

    Some(ticket)
}

