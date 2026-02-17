use crate::db::Database;
use crate::printing;
use tauri::State;

#[tauri::command]
pub fn imprimir_ticket(db: State<Database>, venta_id: i64) -> Result<String, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;

    // Obtener venta completa
    let venta = conn
        .query_row(
            "SELECT id, numero, cliente_id, fecha, subtotal_sin_iva, subtotal_con_iva,
             descuento, iva, total, forma_pago, monto_recibido, cambio, estado,
             tipo_documento, estado_sri, autorizacion_sri, clave_acceso, observacion,
             numero_factura
             FROM ventas WHERE id = ?1",
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
                    estado_sri: row.get::<_, String>(14).unwrap_or_else(|_| "NO_APLICA".to_string()),
                    autorizacion_sri: row.get(15)?,
                    clave_acceso: row.get(16)?,
                    observacion: row.get(17)?,
                    numero_factura: row.get(18)?,
                })
            },
        )
        .map_err(|e| e.to_string())?;

    let mut stmt = conn
        .prepare(
            "SELECT d.id, d.venta_id, d.producto_id, p.nombre, d.cantidad,
             d.precio_unitario, d.descuento, d.iva_porcentaje, d.subtotal
             FROM venta_detalles d
             JOIN productos p ON d.producto_id = p.id
             WHERE d.venta_id = ?1",
        )
        .map_err(|e| e.to_string())?;

    let detalles = stmt
        .query_map(rusqlite::params![venta_id], |row| {
            Ok(crate::models::VentaDetalle {
                id: Some(row.get(0)?),
                venta_id: Some(row.get(1)?),
                producto_id: row.get(2)?,
                nombre_producto: Some(row.get(3)?),
                cantidad: row.get(4)?,
                precio_unitario: row.get(5)?,
                descuento: row.get(6)?,
                iva_porcentaje: row.get(7)?,
                subtotal: row.get(8)?,
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    let cliente_nombre: Option<String> = venta.cliente_id.and_then(|cid| {
        conn.query_row(
            "SELECT nombre FROM clientes WHERE id = ?1",
            rusqlite::params![cid],
            |row| row.get(0),
        )
        .ok()
    });

    // Obtener config
    let mut cfg_stmt = conn
        .prepare("SELECT key, value FROM config")
        .map_err(|e| e.to_string())?;

    let config: std::collections::HashMap<String, String> = cfg_stmt
        .query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<std::collections::HashMap<_, _>, _>>()
        .map_err(|e| e.to_string())?;

    let venta_completa = crate::models::VentaCompleta {
        venta,
        detalles,
        cliente_nombre,
    };

    let ticket_data = printing::generar_ticket(&venta_completa, &config);

    // Obtener nombre de impresora de la config
    let impresora = config
        .get("impresora")
        .map(|s| s.to_string())
        .unwrap_or_default();

    if impresora.is_empty() {
        return Err("No hay impresora configurada. Vaya a Configuración.".to_string());
    }

    printing::imprimir_raw_windows(&impresora, &ticket_data)?;

    Ok("Ticket impreso correctamente".to_string())
}

/// Genera el ticket de venta como PDF y lo abre con el visor del sistema.
/// Alternativa a la impresion directa por ESC/POS cuando la termica no esta disponible.
#[tauri::command]
pub fn imprimir_ticket_pdf(db: State<Database>, venta_id: i64) -> Result<String, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;

    // Obtener venta completa
    let venta = conn
        .query_row(
            "SELECT id, numero, cliente_id, fecha, subtotal_sin_iva, subtotal_con_iva,
             descuento, iva, total, forma_pago, monto_recibido, cambio, estado,
             tipo_documento, estado_sri, autorizacion_sri, clave_acceso, observacion,
             numero_factura
             FROM ventas WHERE id = ?1",
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
                    estado_sri: row.get::<_, String>(14).unwrap_or_else(|_| "NO_APLICA".to_string()),
                    autorizacion_sri: row.get(15)?,
                    clave_acceso: row.get(16)?,
                    observacion: row.get(17)?,
                    numero_factura: row.get(18)?,
                })
            },
        )
        .map_err(|e| e.to_string())?;

    let mut stmt = conn
        .prepare(
            "SELECT d.id, d.venta_id, d.producto_id, p.nombre, d.cantidad,
             d.precio_unitario, d.descuento, d.iva_porcentaje, d.subtotal
             FROM venta_detalles d
             JOIN productos p ON d.producto_id = p.id
             WHERE d.venta_id = ?1",
        )
        .map_err(|e| e.to_string())?;

    let detalles = stmt
        .query_map(rusqlite::params![venta_id], |row| {
            Ok(crate::models::VentaDetalle {
                id: Some(row.get(0)?),
                venta_id: Some(row.get(1)?),
                producto_id: row.get(2)?,
                nombre_producto: Some(row.get(3)?),
                cantidad: row.get(4)?,
                precio_unitario: row.get(5)?,
                descuento: row.get(6)?,
                iva_porcentaje: row.get(7)?,
                subtotal: row.get(8)?,
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    let cliente_nombre: Option<String> = venta.cliente_id.and_then(|cid| {
        conn.query_row(
            "SELECT nombre FROM clientes WHERE id = ?1",
            rusqlite::params![cid],
            |row| row.get(0),
        )
        .ok()
    });

    // Obtener config
    let mut cfg_stmt = conn
        .prepare("SELECT key, value FROM config")
        .map_err(|e| e.to_string())?;

    let config: std::collections::HashMap<String, String> = cfg_stmt
        .query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<std::collections::HashMap<_, _>, _>>()
        .map_err(|e| e.to_string())?;

    drop(stmt);
    drop(cfg_stmt);
    drop(conn);

    let venta_completa = crate::models::VentaCompleta {
        venta,
        detalles,
        cliente_nombre,
    };

    let pdf_bytes = crate::sri::ride::generar_ticket_pdf(&venta_completa, &config)?;

    // Guardar en directorio temporal
    let temp_dir = std::env::temp_dir();
    let filename = format!(
        "Ticket-{}.pdf",
        venta_completa.venta.numero.replace(['/', '\\', ':'], "-")
    );
    let pdf_path = temp_dir.join(&filename);
    std::fs::write(&pdf_path, &pdf_bytes)
        .map_err(|e| format!("Error guardando ticket PDF: {}", e))?;

    // Abrir con visor del sistema
    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("cmd")
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
// REPORTE CIERRE DE CAJA
// ============================================

/// Obtiene datos completos de una caja cerrada para el reporte
fn obtener_datos_reporte_caja(
    conn: &rusqlite::Connection,
    caja_id: i64,
) -> Result<crate::models::ResumenCajaReporte, String> {
    let caja = conn
        .query_row(
            "SELECT id, fecha_apertura, fecha_cierre, monto_inicial, monto_ventas,
             monto_esperado, monto_real, diferencia, estado, usuario, observacion, usuario_id
             FROM caja WHERE id = ?1",
            rusqlite::params![caja_id],
            |row| {
                Ok(crate::models::Caja {
                    id: Some(row.get(0)?),
                    fecha_apertura: row.get(1)?,
                    fecha_cierre: row.get(2)?,
                    monto_inicial: row.get(3)?,
                    monto_ventas: row.get(4)?,
                    monto_esperado: row.get(5)?,
                    monto_real: row.get(6)?,
                    diferencia: row.get(7)?,
                    estado: row.get(8)?,
                    usuario: row.get(9)?,
                    observacion: row.get(10)?,
                    usuario_id: row.get(11)?,
                })
            },
        )
        .map_err(|e| format!("Caja no encontrada: {}", e))?;

    let fecha_apertura = caja.fecha_apertura.as_deref().unwrap_or("");

    let total_ventas: f64 = conn
        .query_row(
            "SELECT COALESCE(SUM(total), 0) FROM ventas
             WHERE created_at >= ?1 AND anulada = 0",
            rusqlite::params![fecha_apertura],
            |row| row.get(0),
        )
        .unwrap_or(0.0);

    let num_ventas: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM ventas
             WHERE created_at >= ?1 AND anulada = 0",
            rusqlite::params![fecha_apertura],
            |row| row.get(0),
        )
        .unwrap_or(0);

    let total_efectivo: f64 = conn
        .query_row(
            "SELECT COALESCE(SUM(total), 0) FROM ventas
             WHERE created_at >= ?1 AND forma_pago = 'EFECTIVO' AND anulada = 0",
            rusqlite::params![fecha_apertura],
            |row| row.get(0),
        )
        .unwrap_or(0.0);

    let total_transferencia: f64 = conn
        .query_row(
            "SELECT COALESCE(SUM(total), 0) FROM ventas
             WHERE created_at >= ?1 AND forma_pago = 'TRANSFERENCIA' AND anulada = 0",
            rusqlite::params![fecha_apertura],
            |row| row.get(0),
        )
        .unwrap_or(0.0);

    let total_fiado: f64 = conn
        .query_row(
            "SELECT COALESCE(SUM(total), 0) FROM ventas
             WHERE created_at >= ?1 AND forma_pago = 'FIADO' AND anulada = 0",
            rusqlite::params![fecha_apertura],
            |row| row.get(0),
        )
        .unwrap_or(0.0);

    let total_gastos: f64 = conn
        .query_row(
            "SELECT COALESCE(SUM(monto), 0) FROM gastos WHERE caja_id = ?1",
            rusqlite::params![caja_id],
            |row| row.get(0),
        )
        .unwrap_or(0.0);

    let total_notas_credito: f64 = conn
        .query_row(
            "SELECT COALESCE(SUM(total), 0) FROM notas_credito
             WHERE fecha >= ?1",
            rusqlite::params![fecha_apertura],
            |row| row.get(0),
        )
        .unwrap_or(0.0);

    let num_notas_credito: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM notas_credito WHERE fecha >= ?1",
            rusqlite::params![fecha_apertura],
            |row| row.get(0),
        )
        .unwrap_or(0);

    // Config del negocio
    let nombre_negocio: String = conn
        .query_row(
            "SELECT value FROM config WHERE key = 'nombre_negocio'",
            [],
            |row| row.get(0),
        )
        .unwrap_or_else(|_| "MI NEGOCIO".to_string());

    let ruc: String = conn
        .query_row("SELECT value FROM config WHERE key = 'ruc'", [], |row| {
            row.get(0)
        })
        .unwrap_or_default();

    let direccion: String = conn
        .query_row(
            "SELECT value FROM config WHERE key = 'direccion'",
            [],
            |row| row.get(0),
        )
        .unwrap_or_default();

    Ok(crate::models::ResumenCajaReporte {
        caja,
        total_ventas,
        num_ventas,
        total_efectivo,
        total_transferencia,
        total_fiado,
        total_gastos,
        total_notas_credito,
        num_notas_credito,
        nombre_negocio,
        ruc,
        direccion,
    })
}

/// Genera reporte de cierre de caja como ESC/POS y lo imprime en la térmica
#[tauri::command]
pub fn imprimir_reporte_caja(db: State<Database>, caja_id: i64) -> Result<String, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let r = obtener_datos_reporte_caja(&conn, caja_id)?;

    let impresora: String = conn
        .query_row(
            "SELECT value FROM config WHERE key = 'impresora'",
            [],
            |row| row.get(0),
        )
        .unwrap_or_default();

    if impresora.is_empty() {
        return Err("No hay impresora configurada. Vaya a Configuración.".to_string());
    }

    drop(conn);

    let ticket_data = generar_ticket_reporte_caja(&r);
    printing::imprimir_raw_windows(&impresora, &ticket_data)?;

    Ok("Reporte de caja impreso correctamente".to_string())
}

/// Genera reporte de cierre de caja como PDF y lo abre con el visor del sistema
#[tauri::command]
pub fn imprimir_reporte_caja_pdf(db: State<Database>, caja_id: i64) -> Result<String, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let r = obtener_datos_reporte_caja(&conn, caja_id)?;
    drop(conn);

    let pdf_bytes = generar_reporte_caja_pdf(&r)?;

    let temp_dir = std::env::temp_dir();
    let filename = format!("ReporteCaja-{}.pdf", caja_id);
    let pdf_path = temp_dir.join(&filename);
    std::fs::write(&pdf_path, &pdf_bytes)
        .map_err(|e| format!("Error guardando PDF: {}", e))?;

    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("cmd")
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

/// Genera bytes ESC/POS para el reporte de cierre de caja (impresora térmica 80mm)
fn generar_ticket_reporte_caja(r: &crate::models::ResumenCajaReporte) -> Vec<u8> {
    let ancho = 48;
    let mut t: Vec<u8> = Vec::new();

    let esc_init: &[u8] = &[0x1B, 0x40];
    let esc_center: &[u8] = &[0x1B, 0x61, 0x01];
    let esc_left: &[u8] = &[0x1B, 0x61, 0x00];
    let esc_bold_on: &[u8] = &[0x1B, 0x45, 0x01];
    let esc_bold_off: &[u8] = &[0x1B, 0x45, 0x00];
    let esc_double_on: &[u8] = &[0x1B, 0x21, 0x30];
    let esc_double_off: &[u8] = &[0x1B, 0x21, 0x00];
    let esc_cut: &[u8] = &[0x1D, 0x56, 0x00];
    let esc_feed: &[u8] = &[0x1B, 0x64, 0x04];

    t.extend_from_slice(esc_init);

    // Encabezado
    t.extend_from_slice(esc_center);
    t.extend_from_slice(esc_bold_on);
    t.extend_from_slice(r.nombre_negocio.as_bytes());
    t.push(b'\n');
    t.extend_from_slice(esc_bold_off);
    if !r.ruc.is_empty() {
        t.extend_from_slice(format!("RUC: {}\n", r.ruc).as_bytes());
    }
    if !r.direccion.is_empty() {
        t.extend_from_slice(format!("{}\n", r.direccion).as_bytes());
    }

    t.extend_from_slice(esc_left);
    t.extend_from_slice(linea_sep(ancho, '-').as_bytes());

    t.extend_from_slice(esc_center);
    t.extend_from_slice(esc_double_on);
    t.extend_from_slice(esc_bold_on);
    t.extend_from_slice(b"CIERRE DE CAJA\n");
    t.extend_from_slice(esc_double_off);
    t.extend_from_slice(esc_bold_off);
    t.extend_from_slice(esc_left);

    t.extend_from_slice(linea_sep(ancho, '-').as_bytes());

    // Info de sesion
    if let Some(ref usuario) = r.caja.usuario {
        t.extend_from_slice(format!("Cajero: {}\n", usuario).as_bytes());
    }
    if let Some(ref apertura) = r.caja.fecha_apertura {
        t.extend_from_slice(format!("Apertura: {}\n", apertura).as_bytes());
    }
    if let Some(ref cierre) = r.caja.fecha_cierre {
        t.extend_from_slice(format!("Cierre:   {}\n", cierre).as_bytes());
    }

    t.extend_from_slice(linea_sep(ancho, '=').as_bytes());

    // Resumen de ventas
    t.extend_from_slice(esc_bold_on);
    t.extend_from_slice(b"VENTAS\n");
    t.extend_from_slice(esc_bold_off);
    t.extend_from_slice(linea_monto_r("Num. Ventas:", &r.num_ventas.to_string(), ancho).as_bytes());
    t.extend_from_slice(linea_monto_r("Total Ventas:", &format!("${:.2}", r.total_ventas), ancho).as_bytes());
    t.push(b'\n');

    // Desglose por forma de pago
    t.extend_from_slice(esc_bold_on);
    t.extend_from_slice(b"FORMAS DE PAGO\n");
    t.extend_from_slice(esc_bold_off);
    t.extend_from_slice(linea_monto_r("Efectivo:", &format!("${:.2}", r.total_efectivo), ancho).as_bytes());
    t.extend_from_slice(linea_monto_r("Transferencia:", &format!("${:.2}", r.total_transferencia), ancho).as_bytes());
    if r.total_fiado > 0.0 {
        t.extend_from_slice(linea_monto_r("Fiado:", &format!("${:.2}", r.total_fiado), ancho).as_bytes());
    }
    t.push(b'\n');

    // Gastos
    if r.total_gastos > 0.0 {
        t.extend_from_slice(esc_bold_on);
        t.extend_from_slice(b"GASTOS\n");
        t.extend_from_slice(esc_bold_off);
        t.extend_from_slice(linea_monto_r("Total Gastos:", &format!("-${:.2}", r.total_gastos), ancho).as_bytes());
        t.push(b'\n');
    }

    // Notas de credito
    if r.num_notas_credito > 0 {
        t.extend_from_slice(esc_bold_on);
        t.extend_from_slice(b"DEVOLUCIONES\n");
        t.extend_from_slice(esc_bold_off);
        t.extend_from_slice(linea_monto_r("Num. NC:", &r.num_notas_credito.to_string(), ancho).as_bytes());
        t.extend_from_slice(linea_monto_r("Total NC:", &format!("-${:.2}", r.total_notas_credito), ancho).as_bytes());
        t.push(b'\n');
    }

    t.extend_from_slice(linea_sep(ancho, '=').as_bytes());

    // Cuadre de caja
    t.extend_from_slice(esc_bold_on);
    t.extend_from_slice(b"CUADRE DE CAJA\n");
    t.extend_from_slice(esc_bold_off);
    t.extend_from_slice(linea_monto_r("Monto Inicial:", &format!("${:.2}", r.caja.monto_inicial), ancho).as_bytes());
    t.extend_from_slice(linea_monto_r("(+) Efectivo:", &format!("${:.2}", r.total_efectivo), ancho).as_bytes());
    t.extend_from_slice(linea_monto_r("(-) Gastos:", &format!("${:.2}", r.total_gastos), ancho).as_bytes());
    t.extend_from_slice(linea_sep(ancho, '-').as_bytes());
    t.extend_from_slice(esc_bold_on);
    t.extend_from_slice(linea_monto_r("Monto Esperado:", &format!("${:.2}", r.caja.monto_esperado), ancho).as_bytes());
    t.extend_from_slice(esc_bold_off);
    t.extend_from_slice(linea_monto_r("Monto Real:", &format!("${:.2}", r.caja.monto_real.unwrap_or(0.0)), ancho).as_bytes());

    t.extend_from_slice(linea_sep(ancho, '-').as_bytes());
    let dif = r.caja.diferencia.unwrap_or(0.0);
    t.extend_from_slice(esc_bold_on);
    t.extend_from_slice(esc_double_on);
    t.extend_from_slice(esc_center);
    let dif_str = if dif >= 0.0 {
        format!("DIFERENCIA: ${:.2}\n", dif)
    } else {
        format!("DIFERENCIA: -${:.2}\n", dif.abs())
    };
    t.extend_from_slice(dif_str.as_bytes());
    t.extend_from_slice(esc_double_off);
    t.extend_from_slice(esc_bold_off);
    t.extend_from_slice(esc_left);

    // Observacion
    if let Some(ref obs) = r.caja.observacion {
        if !obs.is_empty() {
            t.push(b'\n');
            t.extend_from_slice(format!("Obs: {}\n", obs).as_bytes());
        }
    }

    t.push(b'\n');
    t.extend_from_slice(esc_center);
    t.extend_from_slice(b"CLOUGET PUNTO DE VENTA\n");

    t.extend_from_slice(esc_feed);
    t.extend_from_slice(esc_cut);

    t
}

fn linea_sep(ancho: usize, ch: char) -> String {
    format!("{}\n", std::iter::repeat(ch).take(ancho).collect::<String>())
}

fn linea_monto_r(label: &str, valor: &str, ancho: usize) -> String {
    let espacios = ancho.saturating_sub(label.len() + valor.len());
    format!("{}{}{}\n", label, " ".repeat(espacios), valor)
}

/// Genera PDF A4 del reporte de cierre de caja
fn generar_reporte_caja_pdf(
    r: &crate::models::ResumenCajaReporte,
) -> Result<Vec<u8>, String> {
    use genpdf::elements::{Break, Paragraph, TableLayout};
    use genpdf::style::Style;
    use genpdf::{Alignment, Document, Element, Margins, SimplePageDecorator};

    let fonts_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("fonts");
    let font_family = genpdf::fonts::from_files(
        fonts_dir.to_str().unwrap_or("fonts"),
        "LiberationSans",
        None,
    )
    .map_err(|e| format!("Error cargando fuentes: {}", e))?;

    let mut doc = Document::new(font_family);
    doc.set_title("Reporte Cierre de Caja");
    doc.set_paper_size(genpdf::Size::new(210, 297)); // A4

    let mut decorator = SimplePageDecorator::new();
    decorator.set_margins(Margins::trbl(20, 20, 20, 20));
    doc.set_page_decorator(decorator);

    let s_title = Style::new().with_font_size(16).bold();
    let s_subtitle = Style::new().with_font_size(12).bold();
    let s_normal = Style::new().with_font_size(10);
    let s_bold = Style::new().with_font_size(10).bold();
    let s_small = Style::new().with_font_size(9);
    let s_big = Style::new().with_font_size(14).bold();

    // Encabezado
    doc.push(
        Paragraph::new(&r.nombre_negocio)
            .aligned(Alignment::Center)
            .styled(s_title),
    );
    if !r.ruc.is_empty() {
        doc.push(
            Paragraph::new(format!("RUC: {}", r.ruc))
                .aligned(Alignment::Center)
                .styled(s_normal),
        );
    }
    if !r.direccion.is_empty() {
        doc.push(
            Paragraph::new(&r.direccion)
                .aligned(Alignment::Center)
                .styled(s_small),
        );
    }

    doc.push(Break::new(1));
    doc.push(
        Paragraph::new("REPORTE DE CIERRE DE CAJA")
            .aligned(Alignment::Center)
            .styled(s_subtitle),
    );
    doc.push(Break::new(1));

    // Info de sesion
    let mut info_table = TableLayout::new(vec![1, 2]);
    if let Some(ref usuario) = r.caja.usuario {
        info_table
            .row()
            .element(Paragraph::new("Cajero:").styled(s_bold))
            .element(Paragraph::new(usuario).styled(s_normal))
            .push()
            .map_err(|e| format!("Error: {}", e))?;
    }
    if let Some(ref apertura) = r.caja.fecha_apertura {
        info_table
            .row()
            .element(Paragraph::new("Apertura:").styled(s_bold))
            .element(Paragraph::new(apertura).styled(s_normal))
            .push()
            .map_err(|e| format!("Error: {}", e))?;
    }
    if let Some(ref cierre) = r.caja.fecha_cierre {
        info_table
            .row()
            .element(Paragraph::new("Cierre:").styled(s_bold))
            .element(Paragraph::new(cierre).styled(s_normal))
            .push()
            .map_err(|e| format!("Error: {}", e))?;
    }
    doc.push(info_table);
    doc.push(Break::new(1));

    // Resumen de ventas
    doc.push(Paragraph::new("VENTAS").styled(s_subtitle));
    doc.push(Break::new(0.3));

    let mut ventas_table = TableLayout::new(vec![3, 2]);
    ventas_table
        .row()
        .element(Paragraph::new("Numero de Ventas:").styled(s_normal))
        .element(
            Paragraph::new(r.num_ventas.to_string())
                .aligned(Alignment::Right)
                .styled(s_bold),
        )
        .push()
        .map_err(|e| format!("Error: {}", e))?;
    ventas_table
        .row()
        .element(Paragraph::new("Total Ventas:").styled(s_normal))
        .element(
            Paragraph::new(format!("${:.2}", r.total_ventas))
                .aligned(Alignment::Right)
                .styled(s_bold),
        )
        .push()
        .map_err(|e| format!("Error: {}", e))?;
    doc.push(ventas_table);
    doc.push(Break::new(0.5));

    // Formas de pago
    doc.push(Paragraph::new("FORMAS DE PAGO").styled(s_subtitle));
    doc.push(Break::new(0.3));

    let mut pago_table = TableLayout::new(vec![3, 2]);
    pago_table
        .row()
        .element(Paragraph::new("Efectivo:").styled(s_normal))
        .element(
            Paragraph::new(format!("${:.2}", r.total_efectivo))
                .aligned(Alignment::Right)
                .styled(s_normal),
        )
        .push()
        .map_err(|e| format!("Error: {}", e))?;
    pago_table
        .row()
        .element(Paragraph::new("Transferencia:").styled(s_normal))
        .element(
            Paragraph::new(format!("${:.2}", r.total_transferencia))
                .aligned(Alignment::Right)
                .styled(s_normal),
        )
        .push()
        .map_err(|e| format!("Error: {}", e))?;
    if r.total_fiado > 0.0 {
        pago_table
            .row()
            .element(Paragraph::new("Fiado:").styled(s_normal))
            .element(
                Paragraph::new(format!("${:.2}", r.total_fiado))
                    .aligned(Alignment::Right)
                    .styled(s_normal),
            )
            .push()
            .map_err(|e| format!("Error: {}", e))?;
    }
    doc.push(pago_table);
    doc.push(Break::new(0.5));

    // Gastos
    if r.total_gastos > 0.0 {
        doc.push(Paragraph::new("GASTOS").styled(s_subtitle));
        doc.push(Break::new(0.3));
        let mut gastos_table = TableLayout::new(vec![3, 2]);
        gastos_table
            .row()
            .element(Paragraph::new("Total Gastos:").styled(s_normal))
            .element(
                Paragraph::new(format!("-${:.2}", r.total_gastos))
                    .aligned(Alignment::Right)
                    .styled(s_bold),
            )
            .push()
            .map_err(|e| format!("Error: {}", e))?;
        doc.push(gastos_table);
        doc.push(Break::new(0.5));
    }

    // Notas de credito
    if r.num_notas_credito > 0 {
        doc.push(Paragraph::new("DEVOLUCIONES").styled(s_subtitle));
        doc.push(Break::new(0.3));
        let mut nc_table = TableLayout::new(vec![3, 2]);
        nc_table
            .row()
            .element(Paragraph::new("Num. NC:").styled(s_normal))
            .element(
                Paragraph::new(r.num_notas_credito.to_string())
                    .aligned(Alignment::Right)
                    .styled(s_normal),
            )
            .push()
            .map_err(|e| format!("Error: {}", e))?;
        nc_table
            .row()
            .element(Paragraph::new("Total NC:").styled(s_normal))
            .element(
                Paragraph::new(format!("-${:.2}", r.total_notas_credito))
                    .aligned(Alignment::Right)
                    .styled(s_bold),
            )
            .push()
            .map_err(|e| format!("Error: {}", e))?;
        doc.push(nc_table);
        doc.push(Break::new(0.5));
    }

    // Cuadre de caja
    doc.push(Paragraph::new("CUADRE DE CAJA").styled(s_subtitle));
    doc.push(Break::new(0.3));

    let mut cuadre_table = TableLayout::new(vec![3, 2]);
    cuadre_table
        .row()
        .element(Paragraph::new("Monto Inicial:").styled(s_normal))
        .element(
            Paragraph::new(format!("${:.2}", r.caja.monto_inicial))
                .aligned(Alignment::Right)
                .styled(s_normal),
        )
        .push()
        .map_err(|e| format!("Error: {}", e))?;
    cuadre_table
        .row()
        .element(Paragraph::new("(+) Efectivo:").styled(s_normal))
        .element(
            Paragraph::new(format!("${:.2}", r.total_efectivo))
                .aligned(Alignment::Right)
                .styled(s_normal),
        )
        .push()
        .map_err(|e| format!("Error: {}", e))?;
    cuadre_table
        .row()
        .element(Paragraph::new("(-) Gastos:").styled(s_normal))
        .element(
            Paragraph::new(format!("${:.2}", r.total_gastos))
                .aligned(Alignment::Right)
                .styled(s_normal),
        )
        .push()
        .map_err(|e| format!("Error: {}", e))?;
    cuadre_table
        .row()
        .element(Paragraph::new("Monto Esperado:").styled(s_bold))
        .element(
            Paragraph::new(format!("${:.2}", r.caja.monto_esperado))
                .aligned(Alignment::Right)
                .styled(s_bold),
        )
        .push()
        .map_err(|e| format!("Error: {}", e))?;
    cuadre_table
        .row()
        .element(Paragraph::new("Monto Real:").styled(s_normal))
        .element(
            Paragraph::new(format!(
                "${:.2}",
                r.caja.monto_real.unwrap_or(0.0)
            ))
            .aligned(Alignment::Right)
            .styled(s_normal),
        )
        .push()
        .map_err(|e| format!("Error: {}", e))?;
    doc.push(cuadre_table);
    doc.push(Break::new(0.5));

    // Diferencia - grande y destacada
    let dif = r.caja.diferencia.unwrap_or(0.0);
    let dif_str = if dif >= 0.0 {
        format!("DIFERENCIA: ${:.2}", dif)
    } else {
        format!("DIFERENCIA: -${:.2}", dif.abs())
    };
    doc.push(
        Paragraph::new(&dif_str)
            .aligned(Alignment::Center)
            .styled(s_big),
    );

    // Observacion
    if let Some(ref obs) = r.caja.observacion {
        if !obs.is_empty() {
            doc.push(Break::new(0.5));
            doc.push(Paragraph::new(format!("Observacion: {}", obs)).styled(s_small));
        }
    }

    doc.push(Break::new(2));
    doc.push(
        Paragraph::new("CLOUGET PUNTO DE VENTA")
            .aligned(Alignment::Center)
            .styled(s_small),
    );

    let mut buffer = Vec::new();
    doc.render(&mut buffer)
        .map_err(|e| format!("Error generando PDF: {}", e))?;

    Ok(buffer)
}

/// Enumera impresoras del sistema (lento, usa PowerShell)
fn enumerar_impresoras_sistema() -> Result<Vec<String>, String> {
    #[cfg(target_os = "windows")]
    {
        use std::process::Command;
        let output = Command::new("powershell")
            .args(["-Command", "Get-Printer | Select-Object -ExpandProperty Name"])
            .output()
            .map_err(|e| e.to_string())?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let impresoras: Vec<String> = stdout
            .lines()
            .map(|l| l.trim().to_string())
            .filter(|l| !l.is_empty())
            .collect();

        Ok(impresoras)
    }

    #[cfg(not(target_os = "windows"))]
    {
        Ok(vec!["No disponible en este sistema".to_string()])
    }
}

#[tauri::command]
pub fn listar_impresoras() -> Result<Vec<String>, String> {
    enumerar_impresoras_sistema()
}

/// Lee impresoras desde cache en config. Si no hay cache, enumera y guarda.
#[tauri::command]
pub fn listar_impresoras_cached(db: State<Database>) -> Result<Vec<String>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let cache: String = conn
        .query_row(
            "SELECT value FROM config WHERE key = 'impresoras_cache'",
            [],
            |row| row.get(0),
        )
        .unwrap_or_default();

    if !cache.is_empty() {
        let lista: Vec<String> = cache.split('\n').map(|s| s.to_string()).collect();
        return Ok(lista);
    }
    drop(conn);

    // No hay cache: enumerar y guardar
    let impresoras = enumerar_impresoras_sistema()?;
    let valor = impresoras.join("\n");
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    conn.execute(
        "INSERT OR REPLACE INTO config (key, value) VALUES ('impresoras_cache', ?1)",
        rusqlite::params![valor],
    ).ok();

    Ok(impresoras)
}

/// Fuerza re-enumeracion de impresoras y actualiza el cache.
#[tauri::command]
pub fn refrescar_impresoras(db: State<Database>) -> Result<Vec<String>, String> {
    let impresoras = enumerar_impresoras_sistema()?;
    let valor = impresoras.join("\n");
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    conn.execute(
        "INSERT OR REPLACE INTO config (key, value) VALUES ('impresoras_cache', ?1)",
        rusqlite::params![valor],
    ).ok();

    Ok(impresoras)
}
