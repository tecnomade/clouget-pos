use crate::sri::soap;
use std::io::Write;
use std::process::{Command, Stdio};

/// Resultado de firmar un XML
pub struct XmlFirmado {
    pub xml: String,
}

/// Firma un XML de comprobante electronico con XAdES-BES para el SRI de Ecuador.
///
/// Usa ec-sri-invoice-signer (Node.js) via proceso hijo para garantizar
/// compatibilidad total con el SRI. La libreria esta probada en produccion
/// con miles de comprobantes.
///
/// Flujo:
/// 1. Escribe el P12 a un archivo temporal
/// 2. Ejecuta scripts/firmar-xml.cjs via Node.js
/// 3. Envia el XML por stdin, recibe el XML firmado por stdout
/// 4. Limpia el archivo temporal
pub fn firmar_comprobante(
    xml_sin_firma: &str,
    p12_data: &[u8],
    p12_password: &str,
    root_tag: &str, // "factura", "notaCredito"
) -> Result<XmlFirmado, String> {
    soap::log_sri("=== FIRMA: Iniciando firma via ec-sri-invoice-signer ===");

    // 1. Escribir P12 a archivo temporal
    let temp_dir = std::env::var("LOCALAPPDATA")
        .map(|app_data| std::path::PathBuf::from(app_data).join("CloudgetPOS"))
        .unwrap_or_else(|_| std::env::temp_dir().join("CloudgetPOS"));
    let _ = std::fs::create_dir_all(&temp_dir);
    let p12_temp_path = temp_dir.join("_temp_cert.p12");

    std::fs::write(&p12_temp_path, p12_data)
        .map_err(|e| format!("Error escribiendo P12 temporal: {}", e))?;

    // 2. Determinar la ruta del script
    let script_path = encontrar_script_firma()?;

    soap::log_sri(&format!("Script firma: {}", script_path.display()));
    soap::log_sri(&format!("P12 temp: {}", p12_temp_path.display()));
    soap::log_sri(&format!("Root tag: {}", root_tag));

    // 3. Ejecutar Node.js con el script
    let mut cmd = Command::new("node");
    cmd.arg(&script_path)
        .arg(root_tag)
        .arg(&p12_temp_path)
        .arg(p12_password)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    // En Windows, ocultar la ventana de consola
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW
    }

    let mut child = cmd.spawn()
        .map_err(|e| {
            let _ = std::fs::remove_file(&p12_temp_path);
            format!("Error ejecutando Node.js para firma: {}. Verifique que Node.js esta instalado.", e)
        })?;

    // 4. Enviar XML por stdin
    if let Some(mut stdin) = child.stdin.take() {
        stdin.write_all(xml_sin_firma.as_bytes())
            .map_err(|e| {
                let _ = std::fs::remove_file(&p12_temp_path);
                format!("Error enviando XML a firma: {}", e)
            })?;
        // Drop stdin para cerrar el pipe y que el script lea EOF
    }

    // 5. Esperar resultado
    let output = child.wait_with_output()
        .map_err(|e| {
            let _ = std::fs::remove_file(&p12_temp_path);
            format!("Error esperando resultado de firma: {}", e)
        })?;

    // 6. Limpiar archivo temporal
    let _ = std::fs::remove_file(&p12_temp_path);

    // 7. Verificar resultado
    let stderr_text = String::from_utf8_lossy(&output.stderr);
    if !output.status.success() {
        soap::log_sri(&format!("ERROR firma Node.js (exit {}): {}", output.status, stderr_text));
        return Err(format!("Error en firma digital: {}", stderr_text.trim()));
    }

    let xml_firmado = String::from_utf8(output.stdout)
        .map_err(|e| format!("Error decodificando XML firmado: {}", e))?;

    if xml_firmado.is_empty() {
        soap::log_sri(&format!("ERROR: XML firmado vacio. stderr: {}", stderr_text));
        return Err("La firma digital no genero resultado. Verifique el certificado P12.".to_string());
    }

    soap::log_sri(&format!("FIRMA OK: XML firmado {} bytes", xml_firmado.len()));

    Ok(XmlFirmado { xml: xml_firmado })
}

/// Encuentra la ruta del script firmar-xml.cjs
/// Busca en varios lugares segun si es dev o produccion
fn encontrar_script_firma() -> Result<std::path::PathBuf, String> {
    // Opcion 1: Variable de entorno (para override)
    if let Ok(path) = std::env::var("CLOUGET_FIRMA_SCRIPT") {
        let p = std::path::PathBuf::from(path);
        if p.exists() {
            return Ok(p);
        }
    }

    // Opcion 2: Relativo al directorio de trabajo (desarrollo)
    let dev_path = std::path::PathBuf::from("scripts/firmar-xml.cjs");
    if dev_path.exists() {
        return Ok(dev_path);
    }

    // Opcion 3: Relativo al ejecutable (produccion)
    if let Ok(exe_path) = std::env::current_exe() {
        if let Some(exe_dir) = exe_path.parent() {
            let prod_path = exe_dir.join("scripts").join("firmar-xml.cjs");
            if prod_path.exists() {
                return Ok(prod_path);
            }
            // Tambien buscar un nivel arriba
            if let Some(parent_dir) = exe_dir.parent() {
                let prod_path2 = parent_dir.join("scripts").join("firmar-xml.cjs");
                if prod_path2.exists() {
                    return Ok(prod_path2);
                }
            }
        }
    }

    // Opcion 4: Directorio del proyecto hardcoded (solo desarrollo Windows)
    let hardcoded = std::path::PathBuf::from("C:/proyectos/clouget-pos/scripts/firmar-xml.cjs");
    if hardcoded.exists() {
        return Ok(hardcoded);
    }

    Err("No se encontro el script de firma (scripts/firmar-xml.cjs). Verifique la instalacion.".to_string())
}
