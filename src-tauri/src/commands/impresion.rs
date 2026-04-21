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
             numero_factura, establecimiento, punto_emision
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
                    establecimiento: row.get(19).ok(),
                    punto_emision: row.get(20).ok(),
                    banco_id: None,
                    referencia_pago: None,
                    banco_nombre: None,
                    tipo_estado: None,
                    guia_placa: None, guia_chofer: None, guia_direccion_destino: None,
                anulada: None,
                })
            },
        )
        .map_err(|e| e.to_string())?;

    let mut stmt = conn
        .prepare(
            "SELECT d.id, d.venta_id, d.producto_id, p.nombre, d.cantidad,
             d.precio_unitario, d.descuento, d.iva_porcentaje, d.subtotal, d.info_adicional
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
                info_adicional: row.get(9).ok(),
            unidad_id: None, unidad_nombre: None, factor_unidad: None,
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
             numero_factura, establecimiento, punto_emision
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
                    establecimiento: row.get(19).ok(),
                    punto_emision: row.get(20).ok(),
                    banco_id: None,
                    referencia_pago: None,
                    banco_nombre: None,
                    tipo_estado: None,
                    guia_placa: None, guia_chofer: None, guia_direccion_destino: None,
                anulada: None,
                })
            },
        )
        .map_err(|e| e.to_string())?;

    let mut stmt = conn
        .prepare(
            "SELECT d.id, d.venta_id, d.producto_id, p.nombre, d.cantidad,
             d.precio_unitario, d.descuento, d.iva_porcentaje, d.subtotal, d.info_adicional
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
                info_adicional: row.get(9).ok(),
            unidad_id: None, unidad_nombre: None, factor_unidad: None,
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

    // Cobros de cuentas por cobrar
    let total_cobros_efectivo: f64 = conn
        .query_row(
            "SELECT COALESCE(SUM(monto), 0) FROM pagos_cuenta
             WHERE fecha >= ?1 AND forma_pago = 'EFECTIVO' AND estado = 'CONFIRMADO'",
            rusqlite::params![fecha_apertura],
            |row| row.get(0),
        )
        .unwrap_or(0.0);

    let total_cobros_banco: f64 = conn
        .query_row(
            "SELECT COALESCE(SUM(monto), 0) FROM pagos_cuenta
             WHERE fecha >= ?1 AND forma_pago = 'TRANSFERENCIA' AND estado = 'CONFIRMADO'",
            rusqlite::params![fecha_apertura],
            |row| row.get(0),
        )
        .unwrap_or(0.0);

    // Retiros de caja
    let total_retiros: f64 = conn
        .query_row(
            "SELECT COALESCE(SUM(monto), 0) FROM retiros_caja WHERE caja_id = ?1",
            rusqlite::params![caja_id],
            |row| row.get(0),
        )
        .unwrap_or(0.0);

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

    // Ventas por categoría
    let ventas_por_categoria: Vec<(String, f64)> = {
        let mut stmt = conn.prepare(
            "SELECT COALESCE(c.nombre, 'Sin categoria') as cat, COALESCE(SUM(vd.subtotal), 0) as total
             FROM ventas v
             JOIN venta_detalles vd ON v.id = vd.venta_id
             LEFT JOIN productos p ON vd.producto_id = p.id
             LEFT JOIN categorias c ON p.categoria_id = c.id
             WHERE v.created_at >= ?1 AND v.anulada = 0
               AND v.tipo_documento IN ('NOTA_VENTA', 'FACTURA')
               AND v.tipo_estado = 'COMPLETADA'
             GROUP BY cat ORDER BY total DESC"
        ).map_err(|e| e.to_string())?;
        let rows: Vec<(String, f64)> = stmt.query_map(rusqlite::params![fecha_apertura], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, f64>(1)?))
        }).map_err(|e| e.to_string())?
          .collect::<Result<Vec<_>, _>>()
          .unwrap_or_default();
        rows
    };

    Ok(crate::models::ResumenCajaReporte {
        caja,
        total_ventas,
        num_ventas,
        total_efectivo,
        total_transferencia,
        total_fiado,
        total_gastos,
        total_cobros_efectivo,
        total_cobros_banco,
        total_retiros,
        total_notas_credito,
        num_notas_credito,
        nombre_negocio,
        ruc,
        direccion,
        ventas_por_categoria,
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

    // Retiros
    if r.total_retiros > 0.0 {
        t.extend_from_slice(esc_bold_on);
        t.extend_from_slice(b"RETIROS\n");
        t.extend_from_slice(esc_bold_off);
        t.extend_from_slice(linea_monto_r("Total Retiros:", &format!("-${:.2}", r.total_retiros), ancho).as_bytes());
        t.push(b'\n');
    }

    // Cobros de cuentas por cobrar
    if r.total_cobros_efectivo > 0.0 || r.total_cobros_banco > 0.0 {
        t.extend_from_slice(esc_bold_on);
        t.extend_from_slice(b"COBROS CUENTAS POR COBRAR\n");
        t.extend_from_slice(esc_bold_off);
        if r.total_cobros_efectivo > 0.0 {
            t.extend_from_slice(linea_monto_r("En efectivo:", &format!("${:.2}", r.total_cobros_efectivo), ancho).as_bytes());
        }
        if r.total_cobros_banco > 0.0 {
            t.extend_from_slice(linea_monto_r("En banco:", &format!("${:.2}", r.total_cobros_banco), ancho).as_bytes());
        }
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

    // Ventas por categoria
    if !r.ventas_por_categoria.is_empty() {
        t.extend_from_slice(linea_sep(ancho, '=').as_bytes());
        t.extend_from_slice(esc_bold_on);
        t.extend_from_slice(b"VENTAS POR CATEGORIA\n");
        t.extend_from_slice(esc_bold_off);
        t.extend_from_slice(linea_sep(ancho, '-').as_bytes());
        for (cat, total) in &r.ventas_por_categoria {
            t.extend_from_slice(linea_monto_r(&format!("{}:", cat), &format!("${:.2}", total), ancho).as_bytes());
        }
    }

    t.extend_from_slice(linea_sep(ancho, '=').as_bytes());

    // Cuadre de caja
    t.extend_from_slice(esc_bold_on);
    t.extend_from_slice(b"CUADRE DE CAJA\n");
    t.extend_from_slice(esc_bold_off);
    t.extend_from_slice(linea_monto_r("Monto Inicial:", &format!("${:.2}", r.caja.monto_inicial), ancho).as_bytes());
    t.extend_from_slice(linea_monto_r("(+) Efectivo ventas:", &format!("${:.2}", r.total_efectivo), ancho).as_bytes());
    if r.total_cobros_efectivo > 0.0 {
        t.extend_from_slice(linea_monto_r("(+) Cobros efectivo:", &format!("${:.2}", r.total_cobros_efectivo), ancho).as_bytes());
    }
    t.extend_from_slice(linea_monto_r("(-) Gastos:", &format!("${:.2}", r.total_gastos), ancho).as_bytes());
    if r.total_retiros > 0.0 {
        t.extend_from_slice(linea_monto_r("(-) Retiros:", &format!("${:.2}", r.total_retiros), ancho).as_bytes());
    }
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

    let fonts_dir = crate::utils::obtener_ruta_fuentes();
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

    // Retiros
    if r.total_retiros > 0.0 {
        doc.push(Paragraph::new("RETIROS").styled(s_subtitle));
        doc.push(Break::new(0.3));
        let mut retiros_table = TableLayout::new(vec![3, 2]);
        retiros_table
            .row()
            .element(Paragraph::new("Total Retiros:").styled(s_normal))
            .element(
                Paragraph::new(format!("-${:.2}", r.total_retiros))
                    .aligned(Alignment::Right)
                    .styled(s_bold),
            )
            .push()
            .map_err(|e| format!("Error: {}", e))?;
        doc.push(retiros_table);
        doc.push(Break::new(0.5));
    }

    // Cobros de cuentas por cobrar
    if r.total_cobros_efectivo > 0.0 || r.total_cobros_banco > 0.0 {
        doc.push(Paragraph::new("COBROS CUENTAS POR COBRAR").styled(s_subtitle));
        doc.push(Break::new(0.3));
        let mut cobros_table = TableLayout::new(vec![3, 2]);
        if r.total_cobros_efectivo > 0.0 {
            cobros_table
                .row()
                .element(Paragraph::new("En efectivo:").styled(s_normal))
                .element(
                    Paragraph::new(format!("${:.2}", r.total_cobros_efectivo))
                        .aligned(Alignment::Right)
                        .styled(s_normal),
                )
                .push()
                .map_err(|e| format!("Error: {}", e))?;
        }
        if r.total_cobros_banco > 0.0 {
            cobros_table
                .row()
                .element(Paragraph::new("En banco/transferencia:").styled(s_normal))
                .element(
                    Paragraph::new(format!("${:.2}", r.total_cobros_banco))
                        .aligned(Alignment::Right)
                        .styled(s_normal),
                )
                .push()
                .map_err(|e| format!("Error: {}", e))?;
        }
        doc.push(cobros_table);
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

    // Ventas por categoria
    if !r.ventas_por_categoria.is_empty() {
        doc.push(Paragraph::new("VENTAS POR CATEGORIA").styled(s_subtitle));
        doc.push(Break::new(0.3));
        let mut cat_table = TableLayout::new(vec![3, 2]);
        for (cat, total) in &r.ventas_por_categoria {
            cat_table.row()
                .element(Paragraph::new(format!("{}:", cat)).styled(s_normal))
                .element(Paragraph::new(format!("${:.2}", total)).aligned(Alignment::Right).styled(s_normal))
                .push().ok();
        }
        doc.push(cat_table);
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
        .element(Paragraph::new("(+) Efectivo ventas:").styled(s_normal))
        .element(
            Paragraph::new(format!("${:.2}", r.total_efectivo))
                .aligned(Alignment::Right)
                .styled(s_normal),
        )
        .push()
        .map_err(|e| format!("Error: {}", e))?;
    if r.total_cobros_efectivo > 0.0 {
        cuadre_table
            .row()
            .element(Paragraph::new("(+) Cobros efectivo:").styled(s_normal))
            .element(
                Paragraph::new(format!("${:.2}", r.total_cobros_efectivo))
                    .aligned(Alignment::Right)
                    .styled(s_normal),
            )
            .push()
            .map_err(|e| format!("Error: {}", e))?;
    }
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
    if r.total_retiros > 0.0 {
        cuadre_table
            .row()
            .element(Paragraph::new("(-) Retiros:").styled(s_normal))
            .element(
                Paragraph::new(format!("${:.2}", r.total_retiros))
                    .aligned(Alignment::Right)
                    .styled(s_normal),
            )
            .push()
            .map_err(|e| format!("Error: {}", e))?;
    }
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
        let output = crate::utils::silent_command("powershell")
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

// ============================================
// GUIA DE REMISION PDF
// ============================================

/// Genera PDF A4 de Guía de Remisión y lo abre con el visor del sistema.
#[tauri::command]
pub fn imprimir_guia_remision_pdf(db: State<Database>, venta_id: i64) -> Result<String, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;

    // Cargar venta con campos de guía
    let venta = conn
        .query_row(
            "SELECT id, numero, cliente_id, fecha, subtotal_sin_iva, subtotal_con_iva,
             descuento, iva, total, forma_pago, monto_recibido, cambio, estado,
             tipo_documento, estado_sri, autorizacion_sri, clave_acceso, observacion,
             numero_factura, establecimiento, punto_emision,
             tipo_estado, guia_placa, guia_chofer, guia_direccion_destino
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
                    establecimiento: row.get(19).ok(),
                    punto_emision: row.get(20).ok(),
                    banco_id: None,
                    referencia_pago: None,
                    banco_nombre: None,
                    tipo_estado: row.get(21).ok(),
                    guia_placa: row.get(22).ok(),
                    guia_chofer: row.get(23).ok(),
                    guia_direccion_destino: row.get(24).ok(),
                anulada: None,
                })
            },
        )
        .map_err(|e| format!("Venta no encontrada: {}", e))?;

    // Verificar que sea guía de remisión
    let tipo_estado = venta.tipo_estado.as_deref().unwrap_or("");
    if tipo_estado != "GUIA_REMISION" {
        return Err("Esta venta no es una Guia de Remision".to_string());
    }

    // Cargar detalles con nombre de producto
    let mut stmt = conn
        .prepare(
            "SELECT d.id, d.venta_id, d.producto_id, p.nombre, d.cantidad,
             d.precio_unitario, d.descuento, d.iva_porcentaje, d.subtotal, d.info_adicional
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
                info_adicional: row.get(9).ok(),
            unidad_id: None, unidad_nombre: None, factor_unidad: None,
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    // Cargar datos del cliente (nombre, identificacion, direccion, telefono)
    let (cliente_nombre, cliente_identificacion, cliente_direccion, cliente_telefono) =
        if let Some(cid) = venta.cliente_id {
            conn.query_row(
                "SELECT nombre, identificacion, direccion, telefono FROM clientes WHERE id = ?1",
                rusqlite::params![cid],
                |row| {
                    Ok((
                        row.get::<_, String>(0).ok(),
                        row.get::<_, Option<String>>(1).unwrap_or(None),
                        row.get::<_, Option<String>>(2).unwrap_or(None),
                        row.get::<_, Option<String>>(3).unwrap_or(None),
                    ))
                },
            )
            .unwrap_or((None, None, None, None))
        } else {
            (None, None, None, None)
        };

    // Cargar config del negocio
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

    // Generar PDF
    let pdf_bytes = generar_guia_remision_pdf(
        &venta,
        &detalles,
        &config,
        cliente_nombre.as_deref(),
        cliente_identificacion.as_deref(),
        cliente_direccion.as_deref(),
        cliente_telefono.as_deref(),
    )?;

    // Guardar en temp
    let temp_dir = std::env::temp_dir();
    let filename = format!(
        "GuiaRemision-{}.pdf",
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

/// Genera los bytes del PDF A4 para Guía de Remisión (formato profesional)
fn generar_guia_remision_pdf(
    venta: &crate::models::Venta,
    detalles: &[crate::models::VentaDetalle],
    config: &std::collections::HashMap<String, String>,
    cliente_nombre: Option<&str>,
    cliente_identificacion: Option<&str>,
    cliente_direccion: Option<&str>,
    cliente_telefono: Option<&str>,
) -> Result<Vec<u8>, String> {
    use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
    use genpdf::elements::{Break, LinearLayout, PaddedElement, Paragraph, StyledElement, TableLayout};
    use genpdf::style::{Color, Style};
    use genpdf::{Alignment, Document, Element, Margins, SimplePageDecorator};

    let fonts_dir = crate::utils::obtener_ruta_fuentes();
    let font_family = genpdf::fonts::from_files(
        fonts_dir.to_str().unwrap_or("fonts"),
        "LiberationSans",
        None,
    )
    .map_err(|e| format!("Error cargando fuentes: {}", e))?;

    let mut doc = Document::new(font_family);
    doc.set_title("Guia de Remision");
    doc.set_paper_size(genpdf::Size::new(210, 297)); // A4

    let mut decorator = SimplePageDecorator::new();
    decorator.set_margins(Margins::trbl(15, 15, 15, 15));
    doc.set_page_decorator(decorator);

    // --- Helpers para celdas con padding ---
    fn pp(text: &str, style: Style) -> PaddedElement<StyledElement<Paragraph>> {
        Paragraph::new(text).styled(style).padded(Margins::trbl(2, 2, 2, 4))
    }

    fn pp_right(text: &str, style: Style) -> impl Element {
        Paragraph::new(text).aligned(Alignment::Right).styled(style).padded(Margins::trbl(2, 4, 2, 2))
    }

    fn pp_center(text: &str, style: Style) -> impl Element {
        Paragraph::new(text).aligned(Alignment::Center).styled(style).padded(Margins::trbl(2, 2, 2, 2))
    }

    // Estilos
    let s_normal = Style::new().with_font_size(9);
    let s_bold = Style::new().with_font_size(9).bold();
    let s_small = Style::new().with_font_size(8);
    let s_small_bold = Style::new().with_font_size(8).bold();
    let s_small_italic = Style::new().with_font_size(7).italic();
    let s_doc_title = Style::new().with_font_size(16).bold();
    let s_doc_no = Style::new().with_font_size(11);
    let s_negocio = Style::new().with_font_size(11).bold();
    let s_table_header = Style::new().with_font_size(8).bold();
    let s_table_cell = Style::new().with_font_size(8);
    let s_total_bold = Style::new().with_font_size(10).bold();
    let s_firma = Style::new().with_font_size(9);
    let s_firma_label = Style::new().with_font_size(8).bold();
    let s_firma_field = Style::new().with_font_size(8);
    let s_footer = Style::new().with_font_size(7).with_color(Color::Greyscale(128));

    // --- Datos del negocio ---
    let nombre_negocio = config.get("nombre_negocio").map(|s| s.as_str()).unwrap_or("MI NEGOCIO");
    let ruc = config.get("ruc").map(|s| s.as_str()).unwrap_or("");
    let direccion_neg = config.get("direccion").map(|s| s.as_str()).unwrap_or("");
    let telefono_neg = config.get("telefono").map(|s| s.as_str()).unwrap_or("");

    let fecha = venta.fecha.as_deref().unwrap_or("-");

    // ===================================================================
    // SECCION 1: ENCABEZADO (2 columnas con bordes)
    // Izq: Logo + datos del negocio
    // Der: Titulo "GUIA DE REMISION" + numero + fecha
    // ===================================================================
    let mut header_table = TableLayout::new(vec![1, 1]);
    header_table.set_cell_decorator(genpdf::elements::FrameCellDecorator::new(true, true, false));

    // --- Columna izquierda: Logo + datos emisor ---
    let mut col_izq = LinearLayout::vertical();

    // Logo del negocio (misma lógica que ride.rs)
    if let Some(logo_b64) = config.get("logo_negocio") {
        if !logo_b64.is_empty() {
            if let Ok(logo_bytes) = BASE64.decode(logo_b64) {
                let logo_temp = std::env::temp_dir().join("clouget_guia_logo.png");
                if std::fs::write(&logo_temp, &logo_bytes).is_ok() {
                    if let Ok(mut logo_img) = genpdf::elements::Image::from_path(&logo_temp) {
                        logo_img = logo_img.with_alignment(Alignment::Center);
                        // Escala dinámica: llenar ancho de columna
                        let max_width_mm = 84.0_f64;
                        let max_height_mm = 30.0_f64;
                        let (img_w, img_h) = if logo_bytes.len() > 24 && &logo_bytes[0..4] == b"\x89PNG" {
                            let w = u32::from_be_bytes([logo_bytes[16], logo_bytes[17], logo_bytes[18], logo_bytes[19]]) as f64;
                            let h = u32::from_be_bytes([logo_bytes[20], logo_bytes[21], logo_bytes[22], logo_bytes[23]]) as f64;
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
                        logo_img = logo_img.with_scale(genpdf::Scale::new(final_scale, final_scale));
                        col_izq.push(logo_img.padded(Margins::trbl(2, 3, 2, 3)));
                    }
                    let _ = std::fs::remove_file(&logo_temp);
                }
            }
        }
    }

    // Datos del negocio
    col_izq.push(pp(nombre_negocio, s_negocio));
    if !ruc.is_empty() {
        col_izq.push(pp(&format!("RUC: {}", ruc), s_bold));
    }
    if !direccion_neg.is_empty() {
        col_izq.push(pp(&format!("Dir: {}", direccion_neg), s_small));
    }
    if !telefono_neg.is_empty() {
        col_izq.push(pp(&format!("Tel: {}", telefono_neg), s_small));
    }
    col_izq.push(Break::new(0.5));

    // --- Columna derecha: Titulo documento + numero + fecha ---
    let mut col_der = LinearLayout::vertical();
    col_der.push(Break::new(2));
    col_der.push(
        Paragraph::new("GUIA DE REMISION")
            .aligned(Alignment::Center)
            .styled(s_doc_title)
            .padded(Margins::trbl(1, 3, 1, 3)),
    );
    col_der.push(Break::new(1));
    col_der.push(
        Paragraph::new(format!("Nro: {}", venta.numero))
            .aligned(Alignment::Center)
            .styled(s_doc_no)
            .padded(Margins::trbl(1, 3, 1, 3)),
    );
    col_der.push(Break::new(0.5));
    col_der.push(
        Paragraph::new(format!("Fecha: {}", fecha))
            .aligned(Alignment::Center)
            .styled(s_normal)
            .padded(Margins::trbl(1, 3, 1, 3)),
    );
    col_der.push(Break::new(1));

    // Estado
    let estado_label = match venta.estado.as_str() {
        "PENDIENTE" => "PENDIENTE DE ENTREGA",
        "COMPLETADA" => "ENTREGADA",
        "ANULADA" => "ANULADA",
        _ => &venta.estado,
    };
    col_der.push(
        Paragraph::new(estado_label)
            .aligned(Alignment::Center)
            .styled(s_bold)
            .padded(Margins::trbl(1, 3, 2, 3)),
    );

    header_table
        .row()
        .element(col_izq.padded(Margins::trbl(0, 0, 0, 0)))
        .element(col_der.padded(Margins::trbl(0, 0, 0, 0)))
        .push()
        .map_err(|e| format!("Error header: {}", e))?;

    doc.push(header_table);
    doc.push(Break::new(0.8));

    // ===================================================================
    // SECCION 2: DATOS DE TRANSPORTE (con bordes)
    // ===================================================================
    let placa = venta.guia_placa.as_deref().unwrap_or("-");
    let chofer = venta.guia_chofer.as_deref().unwrap_or("-");
    let destino = venta.guia_direccion_destino.as_deref().unwrap_or("-");

    let mut transporte_table = TableLayout::new(vec![1, 2, 1, 2]);
    transporte_table.set_cell_decorator(genpdf::elements::FrameCellDecorator::new(true, true, false));

    // Fila 1: Placa + Chofer
    transporte_table
        .row()
        .element(pp("Placa:", s_small_bold))
        .element(pp(placa, s_normal))
        .element(pp("Chofer:", s_small_bold))
        .element(pp(chofer, s_normal))
        .push()
        .map_err(|e| format!("Error transporte: {}", e))?;

    // Fila 2: Direccion destino (span completo)
    transporte_table
        .row()
        .element(pp("Dir. Destino:", s_small_bold))
        .element(pp(destino, s_normal))
        .element(pp("", s_normal))
        .element(pp("", s_normal))
        .push()
        .map_err(|e| format!("Error transporte: {}", e))?;

    doc.push(transporte_table);
    doc.push(Break::new(0.5));

    // ===================================================================
    // SECCION 3: DATOS DEL CLIENTE (con bordes)
    // ===================================================================
    let mut cliente_table = TableLayout::new(vec![1, 2, 1, 2]);
    cliente_table.set_cell_decorator(genpdf::elements::FrameCellDecorator::new(true, true, false));

    // Fila 1: Nombre + Identificacion
    cliente_table
        .row()
        .element(pp("Cliente:", s_small_bold))
        .element(pp(cliente_nombre.unwrap_or("Consumidor Final"), s_normal))
        .element(pp("Identificacion:", s_small_bold))
        .element(pp(cliente_identificacion.unwrap_or("9999999999999"), s_normal))
        .push()
        .map_err(|e| format!("Error cliente: {}", e))?;

    // Fila 2: Direccion + Telefono
    let cli_dir = cliente_direccion.unwrap_or("-");
    let cli_tel = cliente_telefono.unwrap_or("-");
    cliente_table
        .row()
        .element(pp("Direccion:", s_small_bold))
        .element(pp(cli_dir, s_normal))
        .element(pp("Telefono:", s_small_bold))
        .element(pp(cli_tel, s_normal))
        .push()
        .map_err(|e| format!("Error cliente: {}", e))?;

    doc.push(cliente_table);
    doc.push(Break::new(0.8));

    // ===================================================================
    // SECCION 4: TABLA DE PRODUCTOS (con bordes FrameCellDecorator)
    // Columnas: # | Codigo | Descripcion | Cantidad | P.Unit | Subtotal
    // ===================================================================
    let mut items_table = TableLayout::new(vec![1, 2, 5, 2, 2, 2]);
    items_table.set_cell_decorator(genpdf::elements::FrameCellDecorator::new(true, true, false));

    // Header row
    items_table
        .row()
        .element(pp_center("#", s_table_header))
        .element(pp("Codigo", s_table_header))
        .element(pp("Descripcion", s_table_header))
        .element(pp_center("Cantidad", s_table_header))
        .element(pp_right("P.Unit", s_table_header))
        .element(pp_right("Subtotal", s_table_header))
        .push()
        .map_err(|e| format!("Error items header: {}", e))?;

    for (i, det) in detalles.iter().enumerate() {
        let nombre = det.nombre_producto.as_deref().unwrap_or("Producto");
        let cant_str = if det.cantidad == det.cantidad.floor() {
            format!("{:.0}", det.cantidad)
        } else {
            format!("{:.2}", det.cantidad)
        };
        let codigo = format!("{}", det.producto_id);

        items_table
            .row()
            .element(pp_center(&format!("{}", i + 1), s_table_cell))
            .element(pp(&codigo, s_table_cell))
            .element(pp(nombre, s_table_cell))
            .element(pp_center(&cant_str, s_table_cell))
            .element(pp_right(&format!("${:.2}", det.precio_unitario), s_table_cell))
            .element(pp_right(&format!("${:.2}", det.subtotal), s_table_cell))
            .push()
            .map_err(|e| format!("Error item: {}", e))?;

        // Info adicional (S/N, lote, observacion) en fila aparte
        if let Some(ref info) = det.info_adicional {
            if !info.is_empty() {
                items_table
                    .row()
                    .element(pp("", s_small))
                    .element(pp("", s_small))
                    .element(
                        Paragraph::new(format!("  >> {}", info))
                            .styled(s_small_italic)
                            .padded(Margins::trbl(0, 1, 1, 5)),
                    )
                    .element(pp("", s_small))
                    .element(pp("", s_small))
                    .element(pp("", s_small))
                    .push()
                    .map_err(|e| format!("Error info: {}", e))?;
            }
        }
    }

    doc.push(items_table);
    doc.push(Break::new(0.5));

    // ===================================================================
    // SECCION 5: TOTALES (alineados a la derecha)
    // ===================================================================
    let mut totales_outer = TableLayout::new(vec![4, 2]);

    // Espacio izquierdo vacío + bloque totales a la derecha
    let mut totales_block = LinearLayout::vertical();

    // Subtotal
    {
        let mut row_table = TableLayout::new(vec![1, 1]);
        row_table.set_cell_decorator(genpdf::elements::FrameCellDecorator::new(true, true, false));
        row_table
            .row()
            .element(pp("Subtotal:", s_bold))
            .element(pp_right(
                &format!("${:.2}", venta.subtotal_sin_iva + venta.subtotal_con_iva),
                s_normal,
            ))
            .push()
            .map_err(|e| format!("Error totales: {}", e))?;
        totales_block.push(row_table);
    }

    // Descuento (si aplica)
    if venta.descuento > 0.0 {
        let mut row_table = TableLayout::new(vec![1, 1]);
        row_table.set_cell_decorator(genpdf::elements::FrameCellDecorator::new(true, true, false));
        row_table
            .row()
            .element(pp("Descuento:", s_bold))
            .element(pp_right(&format!("-${:.2}", venta.descuento), s_normal))
            .push()
            .map_err(|e| format!("Error totales: {}", e))?;
        totales_block.push(row_table);
    }

    // IVA
    {
        let mut row_table = TableLayout::new(vec![1, 1]);
        row_table.set_cell_decorator(genpdf::elements::FrameCellDecorator::new(true, true, false));
        row_table
            .row()
            .element(pp("IVA:", s_bold))
            .element(pp_right(&format!("${:.2}", venta.iva), s_normal))
            .push()
            .map_err(|e| format!("Error totales: {}", e))?;
        totales_block.push(row_table);
    }

    // TOTAL
    {
        let mut row_table = TableLayout::new(vec![1, 1]);
        row_table.set_cell_decorator(genpdf::elements::FrameCellDecorator::new(true, true, false));
        row_table
            .row()
            .element(pp("TOTAL:", s_total_bold))
            .element(pp_right(&format!("${:.2}", venta.total), s_total_bold))
            .push()
            .map_err(|e| format!("Error totales: {}", e))?;
        totales_block.push(row_table);
    }

    totales_outer
        .row()
        .element(Paragraph::new("").styled(s_normal))
        .element(totales_block)
        .push()
        .map_err(|e| format!("Error totales outer: {}", e))?;

    doc.push(totales_outer);

    // Observacion
    if let Some(ref obs) = venta.observacion {
        if !obs.is_empty() {
            doc.push(Break::new(0.5));
            let mut obs_table = TableLayout::new(vec![1]);
            obs_table.set_cell_decorator(genpdf::elements::FrameCellDecorator::new(true, true, false));
            obs_table
                .row()
                .element(pp(&format!("Observacion: {}", obs), s_normal))
                .push()
                .map_err(|e| format!("Error obs: {}", e))?;
            doc.push(obs_table);
        }
    }

    doc.push(Break::new(3));

    // ===================================================================
    // SECCION 6: FIRMA "RECIBI CONFORME" (caja unica, ancho completo)
    // ===================================================================
    // Layout: 2 columnas — izquierda vacía (spacer), derecha con cuadro de firma
    let mut firmas_table = TableLayout::new(vec![1, 1]);
    firmas_table.set_cell_decorator(genpdf::elements::FrameCellDecorator::new(false, false, false));

    // Columna izquierda: vacía
    let spacer = LinearLayout::vertical();

    // Columna derecha: cuadro con borde
    let mut firma_outer = TableLayout::new(vec![1]);
    firma_outer.set_cell_decorator(genpdf::elements::FrameCellDecorator::new(true, true, false));

    let mut firma_box = LinearLayout::vertical();
    firma_box.push(
        Paragraph::new("RECIBI CONFORME")
            .aligned(Alignment::Center)
            .styled(s_firma_label)
            .padded(Margins::trbl(4, 4, 2, 4)),
    );
    firma_box.push(Break::new(5));
    firma_box.push(
        Paragraph::new("________________________________")
            .aligned(Alignment::Center)
            .styled(s_firma)
            .padded(Margins::trbl(0, 4, 0, 4)),
    );
    firma_box.push(
        Paragraph::new("Firma")
            .aligned(Alignment::Center)
            .styled(s_firma_field)
            .padded(Margins::trbl(0, 4, 3, 4)),
    );
    firma_box.push(
        Paragraph::new("Nombre: ___________________________________")
            .aligned(Alignment::Left)
            .styled(s_firma_field)
            .padded(Margins::trbl(2, 8, 1, 8)),
    );
    firma_box.push(
        Paragraph::new("Cedula:   ___________________________________")
            .aligned(Alignment::Left)
            .styled(s_firma_field)
            .padded(Margins::trbl(1, 8, 4, 8)),
    );

    firma_outer
        .row()
        .element(firma_box)
        .push()
        .map_err(|e| format!("Error firma: {}", e))?;

    firmas_table
        .row()
        .element(spacer)
        .element(firma_outer)
        .push()
        .map_err(|e| format!("Error firmas: {}", e))?;

    doc.push(firmas_table);

    doc.push(Break::new(1.5));

    // ===================================================================
    // SECCION 7: NOTAS Y FOOTER
    // ===================================================================
    doc.push(
        Paragraph::new("Nota: Este documento NO tiene valor tributario. Es una guia de remision interna para control de mercaderia.")
            .aligned(Alignment::Center)
            .styled(s_footer),
    );
    doc.push(Break::new(0.5));
    doc.push(
        Paragraph::new("Generado por Clouget POS")
            .aligned(Alignment::Center)
            .styled(s_footer),
    );

    let mut buffer = Vec::new();
    doc.render(&mut buffer)
        .map_err(|e| format!("Error generando PDF: {}", e))?;

    Ok(buffer)
}
