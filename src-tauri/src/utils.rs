use rand::Rng;
use sha2::{Digest, Sha256};
use std::path::PathBuf;
use std::process::Command;

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
