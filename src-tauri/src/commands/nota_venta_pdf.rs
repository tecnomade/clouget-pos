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

fn pp(text: &str, style: Style) -> PaddedElement<StyledElement<Paragraph>> {
    Paragraph::new(text)
        .styled(style)
        .padded(Margins::trbl(1, 1, 1, 3))
}

fn pp_right(text: &str, style: Style) -> impl Element {
    Paragraph::new(text)
        .aligned(Alignment::Right)
        .styled(style)
        .padded(Margins::trbl(1, 3, 1, 1))
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

fn forma_pago_label(forma: &str) -> &str {
    match forma {
        "EFECTIVO" => "Efectivo",
        "TRANSFERENCIA" => "Transferencia",
        "TARJETA" | "TARJETA_CREDITO" | "TARJETA_DEBITO" => "Tarjeta",
        "CREDITO" => "Crédito",
        _ => forma,
    }
}

// ============================================
// GENERADOR NOTA DE VENTA PDF (A4)
// ============================================

fn generar_nota_venta_pdf_bytes(
    venta: &crate::models::Venta,
    detalles: &[(crate::models::VentaDetalle, String)], // (detalle, codigo_producto)
    cliente_nombre: &str,
    cliente_identificacion: &str,
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
    doc.set_title("Nota de Venta");

    let mut decorator = SimplePageDecorator::new();
    decorator.set_margins(Margins::trbl(15, 15, 15, 15));
    doc.set_page_decorator(decorator);

    // Estilos
    let s_normal = Style::new().with_font_size(9);
    let s_bold = Style::new().with_font_size(9).bold();
    let s_small = Style::new().with_font_size(8);
    let s_small_bold = Style::new().with_font_size(8).bold();
    let s_title = Style::new().with_font_size(14).bold();
    let s_doc_type = Style::new().with_font_size(16).bold();
    let s_doc_no = Style::new().with_font_size(12);
    let s_ruc = Style::new().with_font_size(10).bold();
    let s_total_bold = Style::new().with_font_size(11).bold();
    let s_pie = Style::new().with_font_size(7).with_color(Color::Greyscale(128));
    let s_regimen = Style::new().with_font_size(8).bold();
    let s_info_adicional = Style::new()
        .with_font_size(7)
        .with_color(Color::Greyscale(100));

    // --- Datos del config ---
    let nombre_negocio = config
        .get("nombre_negocio")
        .map(|s| s.as_str())
        .unwrap_or("MI NEGOCIO");
    let ruc = config.get("ruc").map(|s| s.as_str()).unwrap_or("");
    let direccion_neg = config.get("direccion").map(|s| s.as_str()).unwrap_or("");
    let telefono_neg = config.get("telefono").map(|s| s.as_str()).unwrap_or("");
    let regimen = config.get("regimen").map(|s| s.as_str()).unwrap_or("");

    let fecha_emision = venta.fecha.as_deref().unwrap_or("-");

    let regimen_label = match regimen {
        "RIMPE_POPULAR" => "CONTRIBUYENTE NEGOCIO POPULAR - REGIMEN RIMPE",
        "RIMPE_EMPRENDEDOR" => "CONTRIBUYENTE REGIMEN RIMPE",
        "GENERAL" => "REGIMEN GENERAL",
        _ => "",
    };

    // ===================================================================
    // SECCION 1: ENCABEZADO (2 columnas)
    // Izq: Logo + datos emisor
    // Der: NOTA DE VENTA titulo, numero, fecha, forma pago
    // ===================================================================
    let mut header_table = TableLayout::new(vec![1, 1]);
    header_table
        .set_cell_decorator(genpdf::elements::FrameCellDecorator::new(true, true, false));

    // --- Columna izquierda ---
    let mut col_izq = LinearLayout::vertical();

    // Logo
    let mut logo_element: Option<PaddedElement<genpdf::elements::Image>> = None;
    if let Some(logo_b64) = config.get("logo_negocio") {
        if !logo_b64.is_empty() {
            if let Ok(logo_bytes) = BASE64.decode(logo_b64) {
                let logo_temp = std::env::temp_dir().join("clouget_nv_logo.png");
                if std::fs::write(&logo_temp, &logo_bytes).is_ok() {
                    if let Ok(mut logo_img) = genpdf::elements::Image::from_path(&logo_temp) {
                        logo_img = logo_img.with_alignment(Alignment::Center);
                        let max_width_mm = 84.0_f64;
                        let max_height_mm = 35.0_f64;
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

    // Datos del emisor
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
        datos_emisor.push(pp(&format!("Tel: {}", telefono_neg), s_normal));
    }
    if !regimen_label.is_empty() {
        datos_emisor.push(Break::new(0.2));
        datos_emisor.push(pp(regimen_label, s_regimen));
    }
    datos_emisor.push(Break::new(0.5));
    col_izq.push(datos_emisor);

    // --- Columna derecha ---
    let mut col_der = LinearLayout::vertical();
    col_der.push(Break::new(2.0));
    col_der.push(pp("NOTA DE VENTA", s_doc_type));
    col_der.push(Break::new(0.5));
    col_der.push(pp(&format!("No. {}", venta.numero), s_doc_no));
    col_der.push(Break::new(0.5));
    col_der.push(pp(&format!("Fecha: {}", fecha_emision), s_bold));
    col_der.push(Break::new(0.3));
    col_der.push(pp(
        &format!("Forma de Pago: {}", forma_pago_label(&venta.forma_pago)),
        s_normal,
    ));
    if let Some(ref banco) = venta.banco_nombre {
        if !banco.is_empty() {
            col_der.push(pp(&format!("Banco: {}", banco), s_normal));
        }
    }
    if let Some(ref referencia) = venta.referencia_pago {
        if !referencia.is_empty() {
            col_der.push(pp(&format!("Ref: {}", referencia), s_normal));
        }
    }
    col_der.push(Break::new(0.5));

    header_table
        .row()
        .element(col_izq.padded(Margins::trbl(2, 3, 2, 3)))
        .element(col_der.padded(Margins::trbl(2, 3, 2, 3)))
        .push()
        .map_err(|e| format!("Error tabla header: {}", e))?;

    doc.push(header_table);
    doc.push(Break::new(1.0));

    // ===================================================================
    // SECCION 2: DATOS DEL CLIENTE
    // ===================================================================
    let es_consumidor_final =
        cliente_identificacion == "9999999999999" || cliente_identificacion.is_empty();

    let mut cliente_section = LinearLayout::vertical();
    cliente_section.push(Break::new(0.3));

    let mut fila_cli = TableLayout::new(vec![3, 2]);
    fila_cli
        .row()
        .element(pp(
            &format!("Cliente: {}", cliente_nombre),
            s_bold,
        ))
        .element(pp(
            if es_consumidor_final {
                "Consumidor Final".to_string()
            } else {
                let tipo_id = if cliente_identificacion.len() == 13 {
                    "RUC"
                } else if cliente_identificacion.len() == 10 {
                    "Cedula"
                } else {
                    "ID"
                };
                format!("{}: {}", tipo_id, cliente_identificacion)
            }
            .as_str(),
            s_normal,
        ))
        .push()
        .map_err(|e| format!("Error fila cliente: {}", e))?;

    cliente_section.push(fila_cli);
    cliente_section.push(Break::new(0.3));

    doc.push(
        cliente_section
            .padded(Margins::trbl(3, 2, 3, 2))
            .framed(),
    );
    doc.push(Break::new(1.0));

    // ===================================================================
    // SECCION 3: TABLA DE PRODUCTOS
    // Columnas: #, Codigo, Descripcion, Cantidad, P.Unit, Desc., Subtotal
    // ===================================================================
    let mut table = TableLayout::new(vec![1, 2, 6, 2, 2, 1, 2]);
    table.set_cell_decorator(genpdf::elements::FrameCellDecorator::new(true, true, false));

    // Header
    table
        .row()
        .element(pp("#", s_small_bold))
        .element(pp("Codigo", s_small_bold))
        .element(pp("Descripcion", s_small_bold))
        .element(pp_right("Cant.", s_small_bold))
        .element(pp_right("P.Unit.", s_small_bold))
        .element(pp_right("Desc.", s_small_bold))
        .element(pp_right("Subtotal", s_small_bold))
        .push()
        .map_err(|e| format!("Error tabla header: {}", e))?;

    // Filas de productos
    for (i, (det, codigo)) in detalles.iter().enumerate() {
        let nombre = det
            .nombre_producto
            .as_deref()
            .unwrap_or("?");
        let subtotal_linea = det.cantidad * det.precio_unitario - det.descuento;

        table
            .row()
            .element(pp(&format!("{}", i + 1), s_small))
            .element(pp(codigo, s_small))
            .element(pp(nombre, s_small))
            .element(pp_right(&format_cantidad(det.cantidad), s_small))
            .element(pp_right(&format_dinero(det.precio_unitario), s_small))
            .element(pp_right(&format_dinero(det.descuento), s_small))
            .element(pp_right(&format_dinero(subtotal_linea), s_small))
            .push()
            .map_err(|e| format!("Error tabla fila: {}", e))?;

        // Info adicional below product name if present
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
                    .element(pp("", s_small))
                    .push()
                    .map_err(|e| format!("Error tabla info adicional: {}", e))?;
            }
        }
    }

    doc.push(table);
    doc.push(Break::new(1.5));

    // ===================================================================
    // SECCION 4: TOTALES (right-aligned table)
    // ===================================================================
    let mut bottom_table = TableLayout::new(vec![12, 8]);

    // Columna izquierda: vacia (spacer)
    let spacer = LinearLayout::vertical();

    // Columna derecha: totales
    let mut totales_col = LinearLayout::vertical();

    // Calcular subtotales por tasa IVA
    let mut sub_iva_15 = 0.0_f64;
    let mut sub_iva_0 = 0.0_f64;

    for (det, _) in detalles {
        let linea = det.cantidad * det.precio_unitario - det.descuento;
        if det.iva_porcentaje > 0.0 {
            sub_iva_15 += linea;
        } else {
            sub_iva_0 += linea;
        }
    }

    let iva_valor = venta.iva;
    let descuento_total = venta.descuento;

    let mut totales_table = TableLayout::new(vec![4, 2]);
    totales_table
        .set_cell_decorator(genpdf::elements::FrameCellDecorator::new(true, true, false));

    let totales_lines: Vec<(&str, f64, Style)> = vec![
        ("Subtotal IVA 0%", sub_iva_0, s_small),
        ("Subtotal IVA 15%", sub_iva_15, s_small),
        ("IVA 15%", iva_valor, s_small),
        ("Descuento", descuento_total, s_small),
    ];

    for (label, valor, style) in &totales_lines {
        totales_table
            .row()
            .element(pp(label, *style))
            .element(pp_right(&format_dinero(*valor), *style))
            .push()
            .map_err(|e| format!("Error totales fila: {}", e))?;
    }

    // TOTAL (bold, grande)
    totales_table
        .row()
        .element(pp("TOTAL", s_total_bold))
        .element(pp_right(&format_dinero(venta.total), s_total_bold))
        .push()
        .map_err(|e| format!("Error totales total: {}", e))?;

    totales_col.push(totales_table);

    bottom_table
        .row()
        .element(spacer)
        .element(totales_col.padded(Margins::trbl(0, 0, 0, 2)))
        .push()
        .map_err(|e| format!("Error tabla bottom: {}", e))?;

    doc.push(bottom_table);
    doc.push(Break::new(1.5));

    // ===================================================================
    // SECCION 5: INFO DE PAGO
    // ===================================================================
    let mut pago_section = LinearLayout::vertical();
    pago_section.push(pp(
        &format!(
            "Forma de Pago: {}",
            forma_pago_label(&venta.forma_pago)
        ),
        s_bold,
    ));
    let es_credito = matches!(venta.forma_pago.as_str(), "CREDITO" | "CRÉDITO" | "FIADO");

    if venta.forma_pago == "EFECTIVO" {
        pago_section.push(pp(
            &format!("Monto Recibido: ${}", format_dinero(venta.monto_recibido)),
            s_normal,
        ));
        if venta.cambio > 0.0 {
            pago_section.push(pp(
                &format!("Cambio: ${}", format_dinero(venta.cambio)),
                s_normal,
            ));
        }
    } else if !es_credito {
        // Transferencia, tarjeta, débito, cheque: mostrar el valor pagado explícito
        pago_section.push(pp(
            &format!("Pagado: ${}", format_dinero(venta.total)),
            s_normal,
        ));
    } else {
        // Crédito: aclarar que queda pendiente
        pago_section.push(pp(
            &format!("Saldo pendiente (crédito): ${}", format_dinero(venta.total)),
            s_normal,
        ));
    }
    pago_section.push(Break::new(0.5));
    doc.push(pago_section);
    doc.push(Break::new(2.0));

    // ===================================================================
    // SECCION 6: PIE DE PAGINA
    // ===================================================================
    doc.push(p_aligned(
        "Gracias por su compra",
        s_bold,
        Alignment::Center,
    ));
    doc.push(Break::new(0.3));
    doc.push(p_aligned(nombre_negocio, s_normal, Alignment::Center));
    doc.push(Break::new(0.5));
    doc.push(p_aligned(
        "Generado por Clouget POS",
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
// TAURI COMMAND
// ============================================

#[tauri::command]
pub fn generar_nota_venta_pdf(db: State<Database>, venta_id: i64) -> Result<String, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;

    // Obtener venta
    let venta = conn
        .query_row(
            "SELECT v.id, v.numero, v.cliente_id, v.fecha, v.subtotal_sin_iva, v.subtotal_con_iva,
             v.descuento, v.iva, v.total, v.forma_pago, v.monto_recibido, v.cambio, v.estado,
             v.tipo_documento, v.estado_sri, v.autorizacion_sri, v.clave_acceso, v.observacion,
             v.numero_factura, v.establecimiento, v.punto_emision,
             v.banco_id, v.referencia_pago,
             COALESCE(cb.nombre, '') as banco_nombre
             FROM ventas v
             LEFT JOIN cuentas_banco cb ON v.banco_id = cb.id
             WHERE v.id = ?1",
            rusqlite::params![venta_id],
            |row| {
                Ok(crate::models::Venta {
                    id: Some(row.get(0)?),
                    numero: row.get(1)?,
                    cliente_id: row.get(2)?,
                    fecha: row.get(3)?,
                    subtotal_sin_iva: row.get(4)?,
                    subtotal_con_iva: row.get(5)?,
                    descuento: row.get(6)?,
                    iva: row.get(7)?,
                    total: row.get(8)?,
                    forma_pago: row.get(9)?,
                    monto_recibido: row.get(10)?,
                    cambio: row.get(11)?,
                    estado: row.get(12)?,
                    tipo_documento: row.get(13)?,
                    estado_sri: row
                        .get::<_, String>(14)
                        .unwrap_or_else(|_| "NO_APLICA".to_string()),
                    autorizacion_sri: row.get(15)?,
                    clave_acceso: row.get(16)?,
                    observacion: row.get(17)?,
                    numero_factura: row.get(18)?,
                    establecimiento: row.get(19).ok(),
                    punto_emision: row.get(20).ok(),
                    banco_id: row.get(21).ok(),
                    referencia_pago: row.get(22).ok(),
                    banco_nombre: {
                        let s: String = row.get(23)?;
                        if s.is_empty() { None } else { Some(s) }
                    },
                    comprobante_imagen: None,
                    caja_id: None,
                    tipo_estado: None,
                    anulada: None,
                    guia_placa: None,
                    guia_chofer: None,
                    guia_direccion_destino: None,
                })
            },
        )
        .map_err(|e| format!("Venta no encontrada: {}", e))?;

    // Verificar que sea NOTA_VENTA
    if venta.tipo_documento != "NOTA_VENTA" {
        return Err(format!(
            "Esta venta es de tipo '{}', no es una Nota de Venta",
            venta.tipo_documento
        ));
    }

    // Obtener detalles con codigo de producto e info_adicional
    let mut stmt = conn
        .prepare(
            "SELECT d.id, d.venta_id, d.producto_id, p.nombre, d.cantidad,
             d.precio_unitario, d.descuento, d.iva_porcentaje, d.subtotal,
             COALESCE(p.codigo, CAST(d.producto_id AS TEXT)) as codigo,
             d.info_adicional
             FROM venta_detalles d
             JOIN productos p ON d.producto_id = p.id
             WHERE d.venta_id = ?1",
        )
        .map_err(|e| e.to_string())?;

    let detalles: Vec<(crate::models::VentaDetalle, String)> = stmt
        .query_map(rusqlite::params![venta_id], |row| {
            let det = crate::models::VentaDetalle {
                id: Some(row.get(0)?),
                venta_id: Some(row.get(1)?),
                producto_id: row.get(2)?,
                nombre_producto: Some(row.get(3)?),
                cantidad: row.get(4)?,
                precio_unitario: row.get(5)?,
                descuento: row.get(6)?,
                iva_porcentaje: row.get(7)?,
                subtotal: row.get(8)?,
                info_adicional: row.get(10).ok(),
            unidad_id: None, unidad_nombre: None, factor_unidad: None, lote_id: None, combo_seleccion: None,
            };
            let codigo: String = row.get(9)?;
            Ok((det, codigo))
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    // Obtener cliente
    let (cliente_nombre, cliente_identificacion) = venta
        .cliente_id
        .and_then(|cid| {
            conn.query_row(
                "SELECT COALESCE(nombre, 'CONSUMIDOR FINAL'), COALESCE(identificacion, '9999999999999') FROM clientes WHERE id = ?1",
                rusqlite::params![cid],
                |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
            )
            .ok()
        })
        .unwrap_or_else(|| ("CONSUMIDOR FINAL".to_string(), "9999999999999".to_string()));

    // Config
    let mut cfg_stmt = conn
        .prepare("SELECT key, value FROM config")
        .map_err(|e| e.to_string())?;
    let config_map: std::collections::HashMap<String, String> = cfg_stmt
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
            ))
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<std::collections::HashMap<_, _>, _>>()
        .map_err(|e| e.to_string())?;

    drop(stmt);
    drop(cfg_stmt);
    drop(conn);

    // Generar PDF
    let pdf_bytes = generar_nota_venta_pdf_bytes(
        &venta,
        &detalles,
        &cliente_nombre,
        &cliente_identificacion,
        &config_map,
    )?;

    // Guardar en directorio temporal
    let temp_dir = std::env::temp_dir();
    let filename = format!(
        "NotaVenta-{}.pdf",
        venta.numero.replace(['/', '\\', ':'], "-")
    );
    let pdf_path = temp_dir.join(&filename);
    std::fs::write(&pdf_path, &pdf_bytes)
        .map_err(|e| format!("Error guardando PDF: {}", e))?;

    // Abrir con visor del sistema
    #[cfg(target_os = "windows")]
    {
        let _ = crate::utils::silent_command("cmd")
            .args(["/C", "start", "", &pdf_path.to_string_lossy()])
            .spawn();
    }

    #[cfg(target_os = "linux")]
    {
        let _ = std::process::Command::new("xdg-open")
            .arg(&pdf_path)
            .spawn();
    }

    #[cfg(target_os = "macos")]
    {
        let _ = std::process::Command::new("open")
            .arg(&pdf_path)
            .spawn();
    }

    Ok(pdf_path.to_string_lossy().to_string())
}
