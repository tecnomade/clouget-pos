use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use image::GenericImageView;
use crate::models::VentaCompleta;
use std::collections::HashMap;

/// Genera el contenido de texto del ticket
pub fn generar_ticket(venta: &VentaCompleta, config: &HashMap<String, String>) -> Vec<u8> {
    let ancho = 48; // caracteres para impresora de 80mm (42 para 58mm)
    let mut ticket: Vec<u8> = Vec::new();

    // Comandos ESC/POS
    let esc_init: &[u8] = &[0x1B, 0x40]; // Inicializar impresora
    let esc_center: &[u8] = &[0x1B, 0x61, 0x01]; // Centrar texto
    let esc_left: &[u8] = &[0x1B, 0x61, 0x00]; // Alinear izquierda
    let esc_bold_on: &[u8] = &[0x1B, 0x45, 0x01]; // Negrita on
    let esc_bold_off: &[u8] = &[0x1B, 0x45, 0x00]; // Negrita off
    let esc_double_on: &[u8] = &[0x1B, 0x21, 0x30]; // Doble alto+ancho
    let esc_double_off: &[u8] = &[0x1B, 0x21, 0x00]; // Normal
    let esc_double_h: &[u8] = &[0x1B, 0x21, 0x10]; // Doble alto (sin doble ancho)
    let esc_cut: &[u8] = &[0x1D, 0x56, 0x00]; // Corte total
    let esc_feed: &[u8] = &[0x1B, 0x64, 0x04]; // Avanzar 4 líneas
    let esc_open_drawer: &[u8] = &[0x1B, 0x70, 0x00, 0x19, 0x78]; // Abrir cajón (pin 2)

    ticket.extend_from_slice(esc_init);

    // === LOGO ===
    // Si hay logo_negocio en config (base64 PNG), intentar imprimirlo como imagen raster
    if let Some(logo_b64) = config.get("logo_negocio") {
        if !logo_b64.is_empty() {
            if let Some(raster_bytes) = logo_to_raster(logo_b64, 300) {
                ticket.extend_from_slice(esc_center);
                ticket.extend_from_slice(&raster_bytes);
                ticket.push(b'\n');
            }
        }
    }

    // Encabezado - nombre del negocio
    ticket.extend_from_slice(esc_center);
    ticket.extend_from_slice(esc_bold_on);
    ticket.extend_from_slice(esc_double_on);
    let nombre = config.get("nombre_negocio").map(|s| s.as_str()).unwrap_or("MI NEGOCIO");
    ticket.extend_from_slice(nombre.as_bytes());
    ticket.push(b'\n');
    ticket.extend_from_slice(esc_double_off);
    ticket.extend_from_slice(esc_bold_off);

    // RUC
    if let Some(ruc) = config.get("ruc") {
        if !ruc.is_empty() {
            let regimen = config.get("regimen").map(|s| s.as_str()).unwrap_or("");
            let label = match regimen {
                "RIMPE_POPULAR" => "RIMPE - NEGOCIO POPULAR",
                "RIMPE_EMPRENDEDOR" => "RIMPE - EMPRENDEDOR",
                _ => "",
            };
            ticket.extend_from_slice(format!("RUC: {}\n", ruc).as_bytes());
            if !label.is_empty() {
                ticket.extend_from_slice(format!("{}\n", label).as_bytes());
            }
        }
    }

    // Dirección y teléfono
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
    ticket.extend_from_slice(linea_separador_doble(ancho).as_bytes());

    // === TIPO DE DOCUMENTO - prominente, doble alto, centrado ===
    let es_cotizacion = venta.venta.tipo_documento == "COTIZACION";
    let es_borrador = venta.venta.tipo_documento == "BORRADOR";
    let tipo_doc = match venta.venta.tipo_documento.as_str() {
        "FACTURA" => "FACTURA",
        "COTIZACION" => "COTIZACION",
        "BORRADOR" => "BORRADOR",
        _ => "NOTA DE VENTA",
    };
    ticket.extend_from_slice(esc_center);
    ticket.extend_from_slice(esc_bold_on);
    ticket.extend_from_slice(esc_double_h); // Doble alto para prominencia
    ticket.extend_from_slice(tipo_doc.as_bytes());
    ticket.push(b'\n');
    ticket.extend_from_slice(esc_double_off);

    // === NÚMERO DE DOCUMENTO - prominente ===
    ticket.extend_from_slice(format!("No. {}\n", venta.venta.numero).as_bytes());
    ticket.extend_from_slice(esc_bold_off);
    ticket.extend_from_slice(esc_left);

    ticket.extend_from_slice(linea_separador_simple(ancho).as_bytes());

    // Fecha y cliente
    let fecha = venta.venta.fecha.as_deref().unwrap_or("-");
    ticket.extend_from_slice(format!("Fecha: {}\n", fecha).as_bytes());
    let cliente = venta.cliente_nombre.as_deref().unwrap_or("CONSUMIDOR FINAL");
    ticket.extend_from_slice(format!("Cliente: {}\n", cliente).as_bytes());

    ticket.extend_from_slice(linea_separador_simple(ancho).as_bytes());

    // Cabecera de detalle
    ticket.extend_from_slice(esc_bold_on);
    ticket.extend_from_slice(
        format!("{:<22} {:>5} {:>8} {:>9}\n", "PRODUCTO", "CANT", "P.UNIT", "SUBTOT").as_bytes(),
    );
    ticket.extend_from_slice(esc_bold_off);
    ticket.extend_from_slice(linea_separador_simple(ancho).as_bytes());

    // Detalles
    for det in &venta.detalles {
        let nombre_base = det.nombre_producto.as_deref().unwrap_or("?");
        // Si vendio en presentacion (SIXPACK, JABA), agregar al nombre
        let nombre_prod: String = match (det.unidad_nombre.as_deref(), det.factor_unidad) {
            (Some(u), Some(f)) if !u.is_empty() && f > 1.0 => format!("{} ({})", nombre_base, u),
            (Some(u), _) if !u.is_empty() => format!("{} ({})", nombre_base, u),
            _ => nombre_base.to_string(),
        };
        // Si el nombre es muy largo, truncar
        let nombre_corto: String = if nombre_prod.len() > 22 {
            nombre_prod[..22].to_string()
        } else {
            nombre_prod.clone()
        };

        ticket.extend_from_slice(
            format!(
                "{:<22} {:>5} {:>8.2} {:>9.2}\n",
                nombre_corto,
                format_cantidad(det.cantidad),
                det.precio_unitario,
                det.subtotal
            )
            .as_bytes(),
        );

        if det.descuento > 0.0 {
            ticket.extend_from_slice(
                format!("  Desc: -{:.2}\n", det.descuento).as_bytes(),
            );
        }
        if let Some(ref info) = det.info_adicional {
            if !info.is_empty() {
                ticket.extend_from_slice(
                    format!("  {}\n", info).as_bytes(),
                );
            }
        }
    }

    ticket.extend_from_slice(linea_separador_doble(ancho).as_bytes());

    // Totales
    ticket.extend_from_slice(linea_monto("Subtotal 0%:", venta.venta.subtotal_sin_iva, ancho).as_bytes());
    ticket.extend_from_slice(linea_monto("Subtotal IVA:", venta.venta.subtotal_con_iva, ancho).as_bytes());
    ticket.extend_from_slice(linea_monto("IVA 15%:", venta.venta.iva, ancho).as_bytes());

    if venta.venta.descuento > 0.0 {
        ticket.extend_from_slice(linea_monto("Descuento:", venta.venta.descuento, ancho).as_bytes());
    }

    ticket.extend_from_slice(esc_bold_on);
    ticket.extend_from_slice(esc_double_on);
    ticket.extend_from_slice(esc_center);
    ticket.extend_from_slice(format!("TOTAL: ${:.2}\n", venta.venta.total).as_bytes());
    ticket.extend_from_slice(esc_double_off);
    ticket.extend_from_slice(esc_bold_off);
    ticket.extend_from_slice(esc_left);

    ticket.extend_from_slice(linea_separador_simple(ancho).as_bytes());

    // Pago (skip for cotizacion and borrador)
    if !es_cotizacion && !es_borrador {
        let forma = venta.venta.forma_pago.as_str();
        let forma_label = match forma {
            "EFECTIVO" => "Efectivo",
            "TRANSFERENCIA" | "TRANSFER" => "Transferencia",
            "TARJETA" => "Tarjeta",
            "CHEQUE" => "Cheque",
            "CREDITO" | "CRÉDITO" | "FIADO" => "Credito",
            other => other,
        };
        ticket.extend_from_slice(format!("Forma pago: {}\n", forma_label).as_bytes());

        let es_credito = matches!(forma, "CREDITO" | "CRÉDITO" | "FIADO");

        // Banco + referencia (transferencia / cheque / tarjeta)
        if let Some(ref banco) = venta.venta.banco_nombre {
            if !banco.is_empty() {
                ticket.extend_from_slice(format!("Banco: {}\n", banco).as_bytes());
            }
        }
        if let Some(ref refp) = venta.venta.referencia_pago {
            if !refp.is_empty() {
                ticket.extend_from_slice(format!("Ref: {}\n", refp).as_bytes());
            }
        }

        // Monto pagado (siempre visible para no-credito), cambio solo si efectivo
        if !es_credito {
            ticket.extend_from_slice(linea_monto("Pagado:", venta.venta.total, ancho).as_bytes());
        }
        if venta.venta.monto_recibido > 0.0 && forma == "EFECTIVO" {
            ticket.extend_from_slice(linea_monto("Recibido:", venta.venta.monto_recibido, ancho).as_bytes());
            if venta.venta.cambio > 0.0 {
                ticket.extend_from_slice(linea_monto("Cambio:", venta.venta.cambio, ancho).as_bytes());
            }
        }
        if es_credito {
            ticket.extend_from_slice(linea_monto("Saldo pendiente:", venta.venta.total, ancho).as_bytes());
        }
    }

    // Info SRI si fue autorizada
    if venta.venta.estado_sri == "AUTORIZADA" {
        ticket.extend_from_slice(linea_separador_doble(ancho).as_bytes());
        ticket.extend_from_slice(esc_center);
        ticket.extend_from_slice(esc_bold_on);
        ticket.extend_from_slice(b"FACTURA ELECTRONICA AUTORIZADA\n");
        ticket.extend_from_slice(esc_bold_off);
        ticket.extend_from_slice(esc_left);

        let num_factura = venta.venta.numero_factura.as_deref().unwrap_or(&venta.venta.numero);
        ticket.extend_from_slice(format!("Factura No: {}\n", num_factura).as_bytes());

        if let Some(ref aut) = venta.venta.autorizacion_sri {
            ticket.extend_from_slice(format!("No. Aut: {}\n", aut).as_bytes());
        }
        if let Some(ref clave) = venta.venta.clave_acceso {
            // Clave de acceso en 2 lineas (49 digitos)
            if clave.len() > 24 {
                ticket.extend_from_slice(format!("Clave: {}\n", &clave[..24]).as_bytes());
                ticket.extend_from_slice(format!("       {}\n", &clave[24..]).as_bytes());
            } else {
                ticket.extend_from_slice(format!("Clave: {}\n", clave).as_bytes());
            }
        }
        let ambiente = config.get("sri_ambiente").map(|s| s.as_str()).unwrap_or("pruebas");
        ticket.extend_from_slice(format!("Ambiente: {}\n",
            if ambiente == "produccion" { "PRODUCCION" } else { "PRUEBAS" }
        ).as_bytes());

        ticket.extend_from_slice(linea_separador_simple(ancho).as_bytes());
        ticket.extend_from_slice(esc_center);
        ticket.extend_from_slice(b"ESTE TICKET NO ES UN\n");
        ticket.extend_from_slice(b"DOCUMENTO TRIBUTARIO OFICIAL\n");
        ticket.extend_from_slice(b"Los comprobantes RIDE y XML seran\n");
        ticket.extend_from_slice(b"enviados a su email registrado\n");
        ticket.extend_from_slice(esc_left);
    }

    // Pie
    ticket.push(b'\n');
    ticket.extend_from_slice(esc_center);
    ticket.extend_from_slice(linea_separador_simple(ancho).as_bytes());
    if es_cotizacion {
        ticket.extend_from_slice(b"Validez: 30 dias\n");
        ticket.extend_from_slice(b"Esta cotizacion no es un\n");
        ticket.extend_from_slice(b"documento de venta\n");
    } else if es_borrador {
        ticket.extend_from_slice(b"DOCUMENTO NO FINALIZADO\n");
    } else {
        ticket.extend_from_slice(b"Gracias por su compra!\n");
    }
    ticket.extend_from_slice(b"CLOUGET PUNTO DE VENTA\n");

    ticket.extend_from_slice(esc_feed);
    ticket.extend_from_slice(esc_cut);

    // Abrir cajón de dinero si está habilitado
    let abrir_cajon = config.get("abrir_cajon").map(|v| v == "1").unwrap_or(true);
    if abrir_cajon {
        ticket.extend_from_slice(esc_open_drawer);
    }

    ticket
}

/// Separador doble: ═══════════
fn linea_separador_doble(ancho: usize) -> String {
    let mut s = String::with_capacity(ancho + 1);
    for _ in 0..ancho {
        s.push('=');
    }
    s.push('\n');
    s
}

/// Separador simple: ----------------
fn linea_separador_simple(ancho: usize) -> String {
    let mut s = String::with_capacity(ancho + 1);
    for _ in 0..ancho {
        s.push('-');
    }
    s.push('\n');
    s
}

fn linea_monto(label: &str, monto: f64, ancho: usize) -> String {
    let valor = format!("${:.2}", monto);
    let espacios = ancho.saturating_sub(label.len() + valor.len());
    format!("{}{}{}\n", label, " ".repeat(espacios), valor)
}

fn format_cantidad(cant: f64) -> String {
    if cant == cant.floor() {
        format!("{:.0}", cant)
    } else {
        format!("{:.2}", cant)
    }
}

/// Convierte un logo base64-PNG a bytes ESC/POS raster (GS v 0).
/// Retorna None si falla cualquier paso (decodificar, parsear imagen, etc.)
/// max_width: ancho máximo en píxeles (debe ser múltiplo de 8, se ajusta)
fn logo_to_raster(logo_b64: &str, max_width: u32) -> Option<Vec<u8>> {
    // Decodificar base64
    let png_bytes = BASE64.decode(logo_b64).ok()?;

    // Cargar imagen con el crate image
    let img = image::load_from_memory(&png_bytes).ok()?;

    // Escalar si es más ancho que max_width, manteniendo proporción
    let (orig_w, orig_h) = (img.width(), img.height());
    let (w, h) = if orig_w > max_width {
        let scale = max_width as f64 / orig_w as f64;
        let new_h = (orig_h as f64 * scale) as u32;
        (max_width, new_h.max(1))
    } else {
        (orig_w, orig_h)
    };

    let resized = img.resize_exact(w, h, image::imageops::FilterType::Lanczos3);

    // Convertir a escala de grises y luego a 1-bit B&W con umbral
    let gray = resized.to_luma8();

    // El ancho en bytes debe ser múltiplo de 8 bits
    let width_pixels = gray.width();
    let height_pixels = gray.height();
    // Ancho en bytes (cada byte = 8 píxeles horizontales)
    let bytes_per_row = ((width_pixels + 7) / 8) as usize;

    // Construir bitmap 1-bit: 1 = negro (punto), 0 = blanco
    let mut bitmap: Vec<u8> = Vec::with_capacity(bytes_per_row * height_pixels as usize);
    for y in 0..height_pixels {
        for bx in 0..bytes_per_row as u32 {
            let mut byte_val: u8 = 0;
            for bit in 0..8u32 {
                let px = bx * 8 + bit;
                if px < width_pixels {
                    let luma = gray.get_pixel(px, y)[0];
                    // Umbral: < 128 = negro (bit 1), >= 128 = blanco (bit 0)
                    if luma < 128 {
                        byte_val |= 0x80 >> bit;
                    }
                }
            }
            bitmap.push(byte_val);
        }
    }

    // Construir comando GS v 0: 0x1D 0x76 0x30 0x00 xL xH yL yH [data]
    // xL xH = bytes por fila (little-endian)
    // yL yH = número de filas (little-endian)
    let xl = (bytes_per_row & 0xFF) as u8;
    let xh = ((bytes_per_row >> 8) & 0xFF) as u8;
    let yl = (height_pixels & 0xFF) as u8;
    let yh = ((height_pixels >> 8) & 0xFF) as u8;

    let mut cmd: Vec<u8> = Vec::with_capacity(8 + bitmap.len());
    cmd.extend_from_slice(&[0x1D, 0x76, 0x30, 0x00, xl, xh, yl, yh]);
    cmd.extend_from_slice(&bitmap);

    Some(cmd)
}

/// Imprime bytes RAW a una impresora de Windows por nombre
/// Usa la API Win32 de impresión (WritePrinter) via PowerShell para enviar datos RAW
#[cfg(target_os = "windows")]
pub fn imprimir_raw_windows(nombre_impresora: &str, datos: &[u8]) -> Result<(), String> {
    use std::io::Write;

    // Escribir datos a archivo temporal
    let temp_path = std::env::temp_dir().join("clouget_ticket.bin");
    let mut file = std::fs::File::create(&temp_path).map_err(|e| e.to_string())?;
    file.write_all(datos).map_err(|e| e.to_string())?;
    drop(file);

    let temp_str = temp_path.to_string_lossy().to_string();

    // Usar PowerShell con la API .NET RawPrinterHelper para enviar RAW data
    // Esto funciona con cualquier nombre de impresora de Windows sin necesitar ruta de share
    let ps_script = format!(
        r#"
Add-Type @'
using System;
using System.Runtime.InteropServices;
public class RawPrint {{
    [StructLayout(LayoutKind.Sequential)] public struct DOCINFO {{
        [MarshalAs(UnmanagedType.LPStr)] public string pDocName;
        [MarshalAs(UnmanagedType.LPStr)] public string pOutputFile;
        [MarshalAs(UnmanagedType.LPStr)] public string pDataType;
    }}
    [DllImport("winspool.drv", SetLastError=true, CharSet=CharSet.Auto)]
    public static extern bool OpenPrinter(string pPrinterName, out IntPtr phPrinter, IntPtr pDefault);
    [DllImport("winspool.drv", SetLastError=true)]
    public static extern bool StartDocPrinter(IntPtr hPrinter, int Level, ref DOCINFO pDocInfo);
    [DllImport("winspool.drv", SetLastError=true)]
    public static extern bool StartPagePrinter(IntPtr hPrinter);
    [DllImport("winspool.drv", SetLastError=true)]
    public static extern bool WritePrinter(IntPtr hPrinter, IntPtr pBytes, int dwCount, out int dwWritten);
    [DllImport("winspool.drv", SetLastError=true)]
    public static extern bool EndPagePrinter(IntPtr hPrinter);
    [DllImport("winspool.drv", SetLastError=true)]
    public static extern bool EndDocPrinter(IntPtr hPrinter);
    [DllImport("winspool.drv", SetLastError=true)]
    public static extern bool ClosePrinter(IntPtr hPrinter);

    public static bool SendRaw(string printer, byte[] data) {{
        IntPtr hPrinter;
        if (!OpenPrinter(printer, out hPrinter, IntPtr.Zero)) return false;
        var di = new DOCINFO {{ pDocName = "Clouget Ticket", pDataType = "RAW" }};
        StartDocPrinter(hPrinter, 1, ref di);
        StartPagePrinter(hPrinter);
        IntPtr pUnmanagedBytes = Marshal.AllocCoTaskMem(data.Length);
        Marshal.Copy(data, 0, pUnmanagedBytes, data.Length);
        int written;
        WritePrinter(hPrinter, pUnmanagedBytes, data.Length, out written);
        Marshal.FreeCoTaskMem(pUnmanagedBytes);
        EndPagePrinter(hPrinter);
        EndDocPrinter(hPrinter);
        ClosePrinter(hPrinter);
        return written == data.Length;
    }}
}}
'@
$bytes = [System.IO.File]::ReadAllBytes("{temp_str}")
$ok = [RawPrint]::SendRaw("{nombre_impresora}", $bytes)
if (-not $ok) {{ throw "No se pudo enviar datos a la impresora '{nombre_impresora}'" }}
Write-Output "OK"
"#
    );

    let mut cmd = crate::utils::silent_command("powershell");
    cmd.args(["-NoProfile", "-ExecutionPolicy", "Bypass", "-Command", &ps_script]);
    let output = cmd.output()
        .map_err(|e| format!("Error al ejecutar PowerShell: {}", e))?;

    // Limpiar archivo temporal
    std::fs::remove_file(&temp_path).ok();

    if output.status.success() {
        Ok(())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let msg = if !stderr.trim().is_empty() { stderr } else { stdout };
        Err(format!("Error al imprimir: {}", msg.trim()))
    }
}

#[cfg(not(target_os = "windows"))]
pub fn imprimir_raw_windows(_nombre_impresora: &str, _datos: &[u8]) -> Result<(), String> {
    Err("Impresión solo disponible en Windows".to_string())
}
