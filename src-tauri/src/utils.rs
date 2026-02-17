use rand::Rng;
use sha2::{Digest, Sha256};

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
