use rand::Rng;
use sha2::{Digest, Sha256};
use std::path::PathBuf;
use std::process::Command;

/// Escribe bytes a un PDF de forma robusta ante el bug de Windows **os error 32**
/// (ERROR_SHARING_VIOLATION): "el archivo está siendo utilizado por otro proceso",
/// que ocurre cuando un visor (Adobe Reader), un antivirus o un cliente de
/// sincronización sostiene el handle del archivo mientras se reintenta escribir.
///
/// Tres capas de defensa:
/// 1. **Nombre único** (uuid) por archivo → nunca colisiona con una copia que un
///    visor aún tenga abierta (causa principal del bug en el RIDE/factura).
/// 2. **Escribe a `.tmp` y renombra** (rename atómico) → no expone un archivo a
///    medio escribir al antivirus/sync.
/// 3. **Reintento con backoff** ante os error 32 → absorbe los bloqueos
///    transitorios de antivirus/sync sin depender del entorno del cliente.
///
/// Devuelve la ruta final del PDF escrito. `prefijo` ej. "RIDE", "RIDE-NC",
/// "TICKET"; `numero` se sanea de caracteres inválidos de nombre de archivo.
pub fn escribir_pdf_robusto(
    dir: &std::path::Path,
    prefijo: &str,
    numero: &str,
    bytes: &[u8],
) -> Result<PathBuf, String> {
    let safe = numero.replace(['/', '\\', ':', '*', '?', '"', '<', '>', '|'], "-");
    let uid = uuid::Uuid::new_v4();
    let final_path = dir.join(format!("{}-{}-{}.pdf", prefijo, safe, uid));
    let tmp_path = dir.join(format!(".{}-{}-{}.pdf.tmp", prefijo, safe, uid));

    // Capa 2+3: escribir a .tmp con reintentos ante locks transitorios.
    let mut intento = 0u32;
    loop {
        match std::fs::write(&tmp_path, bytes) {
            Ok(_) => break,
            Err(e) if e.raw_os_error() == Some(32) && intento < 5 => {
                intento += 1;
                std::thread::sleep(std::time::Duration::from_millis(120 * u64::from(intento)));
            }
            Err(e) => return Err(format!("Error guardando PDF: {}", e)),
        }
    }

    // Rename atómico al destino final, también con reintentos.
    intento = 0;
    loop {
        match std::fs::rename(&tmp_path, &final_path) {
            Ok(_) => return Ok(final_path),
            Err(e) if e.raw_os_error() == Some(32) && intento < 5 => {
                intento += 1;
                std::thread::sleep(std::time::Duration::from_millis(120 * u64::from(intento)));
            }
            Err(e) => {
                let _ = std::fs::remove_file(&tmp_path);
                return Err(format!("Error guardando PDF: {}", e));
            }
        }
    }
}

/// Convierte un Excel serial date a string ISO YYYY-MM-DD.
///
/// Excel almacena fechas como días desde 1899-12-30 (con un bug histórico:
/// trata 1900 como bisiesto aunque no lo fue). La fórmula compatible con
/// el bug es: para serial >= 60, fecha = 1899-12-30 + (serial - 1) días.
/// Para serial < 60 (raro, pre-marzo-1900), no aplicamos el ajuste.
///
/// Rango razonable de serials esperados:
///   25569 = 1970-01-01
///   46265 ≈ junio 2026
///   54789 ≈ enero 2050
///
/// Retorna None si el serial está fuera de rango razonable (1900-2200).
pub fn excel_serial_to_iso(serial: f64) -> Option<String> {
    use chrono::{Duration, NaiveDate};

    // Validar rango razonable: descartamos valores absurdos como 0, negativos,
    // o gigantescos que claramente no son fechas Excel.
    // Rango: 1 (1900-01-01) a ~110000 (2201-01-01).
    if !(1.0..=110000.0).contains(&serial) {
        return None;
    }

    let epoch = NaiveDate::from_ymd_opt(1899, 12, 30)?;
    // Bug del año 1900: Excel trata 1900 como bisiesto (29-feb-1900 que no
    // existió). Para serials >= 60 (post 1-mar-1900), restar 1 día compensa.
    let dias = if serial >= 60.0 {
        (serial as i64) - 1
    } else {
        serial as i64
    };

    epoch
        .checked_add_signed(Duration::days(dias))
        .map(|d| d.format("%Y-%m-%d").to_string())
}

/// Detecta si un string es un número puro (entero o decimal) que parece ser
/// un Excel serial date en rango razonable de fechas (1982-2173).
/// Si lo es, retorna el f64. Si no, None.
pub fn parse_posible_serial_excel(s: &str) -> Option<f64> {
    let trimmed = s.trim();
    if trimmed.is_empty() {
        return None;
    }
    // Solo acepta dígitos opcionalmente con un punto decimal
    if !trimmed.chars().all(|c| c.is_ascii_digit() || c == '.') {
        return None;
    }
    let f: f64 = trimmed.parse().ok()?;
    // Rango: 30000 (1982-03-15) a 100000 (2173-10-14). Cubre cualquier
    // fecha de caducidad razonable de un producto real.
    if (30000.0..=100000.0).contains(&f) {
        Some(f)
    } else {
        None
    }
}

/// Crea un Command que no muestra ventana de consola en Windows.
/// En otros sistemas, equivalente a Command::new.
pub fn silent_command(program: &str) -> Command {
    let cmd = Command::new(program);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x08000000;
        let mut cmd = cmd;
        cmd.creation_flags(CREATE_NO_WINDOW);
        return cmd;
    }
    #[cfg(not(windows))]
    cmd
}

/// Genera un salt aleatorio de 16 caracteres hexadecimales
pub fn generar_salt() -> String {
    let mut rng = rand::thread_rng();
    let salt: u64 = rng.gen();
    format!("{:016x}", salt)
}

/// Hash de PIN con salt usando SHA-256
/// Retorna el hash en formato hexadecimal
pub fn hash_pin(salt: &str, pin: &str) -> String {
    let input = format!("{}{}", salt, pin);
    let hash = Sha256::digest(input.as_bytes());
    format!("{:x}", hash)
}

/// Obtiene la ruta a la carpeta de fuentes.
/// Busca en múltiples ubicaciones para funcionar tanto en desarrollo como en producción.
pub fn obtener_ruta_fuentes() -> PathBuf {
    // 1. Ruta relativa al ejecutable (producción - instalado por Tauri)
    if let Ok(exe) = std::env::current_exe() {
        if let Some(exe_dir) = exe.parent() {
            // Tauri resources: junto al .exe
            let junto_exe = exe_dir.join("fonts");
            if junto_exe.exists() {
                return junto_exe;
            }
            // Tauri NSIS: _up_/fonts
            if let Some(parent) = exe_dir.parent() {
                let arriba = parent.join("fonts");
                if arriba.exists() {
                    return arriba;
                }
            }
            // Tauri resources alt: resources/fonts
            let resources = exe_dir.join("resources").join("fonts");
            if resources.exists() {
                return resources;
            }
        }
    }

    // 2. Ruta de desarrollo (CARGO_MANIFEST_DIR)
    let dev_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("fonts");
    if dev_dir.exists() {
        return dev_dir;
    }

    // 3. Fuentes del sistema Windows como fallback
    let windows_fonts = PathBuf::from("C:\\Windows\\Fonts");
    if windows_fonts.exists() {
        return windows_fonts;
    }

    // 4. Fallback final
    dev_dir
}
