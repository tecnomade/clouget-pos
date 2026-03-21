use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Nonce,
};
use flate2::write::{GzDecoder, GzEncoder};
use flate2::Compression;
use sha2::{Digest, Sha256};
use std::io::Write;

/// Comprime datos con gzip
pub fn compress(data: &[u8]) -> Result<Vec<u8>, String> {
    let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(data).map_err(|e| format!("Error comprimiendo: {}", e))?;
    encoder.finish().map_err(|e| format!("Error finalizando compresion: {}", e))
}

/// Descomprime datos gzip
pub fn decompress(data: &[u8]) -> Result<Vec<u8>, String> {
    let mut decoder = GzDecoder::new(Vec::new());
    decoder.write_all(data).map_err(|e| format!("Error descomprimiendo: {}", e))?;
    decoder.finish().map_err(|e| format!("Error finalizando descompresion: {}", e))
}

/// Deriva una clave AES-256 desde un string (licencia_codigo + machine_id)
fn derive_key(key_material: &str) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(key_material.as_bytes());
    hasher.update(b"clouget-pos-backup-v1"); // salt fijo
    let result = hasher.finalize();
    let mut key = [0u8; 32];
    key.copy_from_slice(&result);
    key
}

/// Encripta datos con AES-256-GCM
pub fn encrypt(data: &[u8], key_material: &str) -> Result<Vec<u8>, String> {
    let key = derive_key(key_material);
    let cipher = Aes256Gcm::new_from_slice(&key)
        .map_err(|e| format!("Error creando cipher: {}", e))?;

    // Generar nonce aleatorio de 12 bytes
    let mut nonce_bytes = [0u8; 12];
    use rand::RngCore;
    rand::thread_rng().fill_bytes(&mut nonce_bytes);
    let nonce = Nonce::from_slice(&nonce_bytes);

    let ciphertext = cipher
        .encrypt(nonce, data)
        .map_err(|e| format!("Error encriptando: {}", e))?;

    // Formato: [12 bytes nonce] + [ciphertext]
    let mut result = Vec::with_capacity(12 + ciphertext.len());
    result.extend_from_slice(&nonce_bytes);
    result.extend_from_slice(&ciphertext);
    Ok(result)
}

/// Desencripta datos con AES-256-GCM
pub fn decrypt(data: &[u8], key_material: &str) -> Result<Vec<u8>, String> {
    if data.len() < 13 {
        return Err("Datos encriptados demasiado cortos".to_string());
    }

    let key = derive_key(key_material);
    let cipher = Aes256Gcm::new_from_slice(&key)
        .map_err(|e| format!("Error creando cipher: {}", e))?;

    let nonce = Nonce::from_slice(&data[..12]);
    let ciphertext = &data[12..];

    cipher
        .decrypt(nonce, ciphertext)
        .map_err(|e| format!("Error desencriptando: {} — clave incorrecta o datos corruptos", e))
}
