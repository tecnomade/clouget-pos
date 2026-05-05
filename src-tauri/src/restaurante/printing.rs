//! Generador de tickets ESC/POS para el módulo Restaurante.
//!
//! - `generar_pre_cuenta`: ticket de cortesía que el cliente revisa ANTES de pagar.
//!   NO es comprobante fiscal — solo informativo. La factura/nota de venta real
//!   se genera al cobrar (vía `registrar_venta`).

use super::models::PedidoDetalle;
use crate::printing::{
    format_cantidad, linea_monto, linea_separador_doble, linea_separador_simple,
    logo_to_raster_pub,
};
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
