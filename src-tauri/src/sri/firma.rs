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

    // 3. Ejecutar Node.js con el script (preferir Node bundleado)
    let node_bin = encontrar_node();
    soap::log_sri(&format!("Node bin: {}", node_bin.display()));
    let mut cmd = Command::new(&node_bin);
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
            format!("Error ejecutando Node.js para firma ({}): {}. Reinstale la aplicacion.", node_bin.display(), e)
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

/// Encuentra la ruta del script firmar-xml.cjs (version autocontenida).
///
/// En produccion se empaqueta como recurso Tauri en `firma/firmar-xml.cjs`
/// (junto al .exe). En desarrollo se resuelve via CARGO_MANIFEST_DIR o el repo.
fn encontrar_script_firma() -> Result<std::path::PathBuf, String> {
    // Opcion 1: Variable de entorno (para override / pruebas)
    if let Ok(path) = std::env::var("CLOUGET_FIRMA_SCRIPT") {
        let p = std::path::PathBuf::from(path);
        if p.exists() {
            return Ok(p);
        }
    }

    // Candidatos en orden de preferencia (produccion primero)
    let mut candidatos: Vec<std::path::PathBuf> = Vec::new();

    // Opcion 2: Recurso Tauri junto al ejecutable (produccion)
    if let Ok(exe_path) = std::env::current_exe() {
        if let Some(exe_dir) = exe_path.parent() {
            candidatos.push(exe_dir.join("firma").join("firmar-xml.cjs"));
            candidatos.push(exe_dir.join("resources").join("firma").join("firmar-xml.cjs"));
            if let Some(parent_dir) = exe_dir.parent() {
                candidatos.push(parent_dir.join("firma").join("firmar-xml.cjs"));
            }
            // Compatibilidad con instalaciones viejas que usaban scripts/
            candidatos.push(exe_dir.join("scripts").join("firmar-xml.cjs"));
        }
    }

    // Opcion 3: Desarrollo (relativo a src-tauri y al repo)
    candidatos.push(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("firma")
            .join("firmar-xml.cjs"),
    );
    candidatos.push(std::path::PathBuf::from("src-tauri/firma/firmar-xml.cjs"));
    candidatos.push(std::path::PathBuf::from("scripts/firmar-xml.cjs"));

    for c in &candidatos {
        if c.exists() {
            return Ok(c.clone());
        }
    }

    Err("No se encontro el script de firma (firma/firmar-xml.cjs). Reinstale la aplicacion.".to_string())
}

/// Encuentra el ejecutable de Node.js a usar para la firma.
///
/// Prefiere el Node bundleado como recurso (`firma/node.exe`) para no depender
/// de que el cliente tenga Node instalado. Si no existe (p.ej. en desarrollo),
/// cae al `node` del PATH del sistema.
fn encontrar_node() -> std::path::PathBuf {
    #[cfg(windows)]
    let node_name = "node.exe";
    #[cfg(not(windows))]
    let node_name = "node";

    // 1. Node bundleado junto al ejecutable (produccion)
    if let Ok(exe_path) = std::env::current_exe() {
        if let Some(exe_dir) = exe_path.parent() {
            let bundled = exe_dir.join("firma").join(node_name);
            if bundled.exists() {
                return bundled;
            }
            let bundled_res = exe_dir.join("resources").join("firma").join(node_name);
            if bundled_res.exists() {
                return bundled_res;
            }
        }
    }

    // 2. Node bundleado en desarrollo (src-tauri/firma)
    let dev_bundled = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("firma")
        .join(node_name);
    if dev_bundled.exists() {
        return dev_bundled;
    }

    // 3. Fallback: node del PATH del sistema
    std::path::PathBuf::from("node")
}
