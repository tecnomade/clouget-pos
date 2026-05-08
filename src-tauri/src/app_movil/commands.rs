//! Comandos Tauri admin del módulo App Móvil.
//!
//! Estos comandos los consume Configuración → 📱 App Móvil del POS escritorio
//! para que el admin pueda ver dispositivos emparejados y revocarlos
//! (ej. mesero perdió el celular, despido, etc.).
//!
//! No los consume la app móvil — la app habla solo HTTP, no Tauri.

use crate::db::Database;
use rusqlite::params;
use serde::Serialize;
use tauri::State;

#[derive(Debug, Serialize, Clone)]
pub struct DispositivoApp {
    pub id: i64,
    pub usuario_id: i64,
    pub usuario_nombre: String,
    pub dispositivo_nombre: Option<String>,
    pub dispositivo_modelo: Option<String>,
    pub dispositivo_so: Option<String>,
    pub created_at: String,
    pub last_used_at: String,
    pub revoked: bool,
    /// Cuántos minutos hace que se usó por última vez (para mostrar "hace X min")
    pub minutos_inactivo: i64,
}

/// Lista todos los dispositivos emparejados — tanto activos como revocados —
/// con datos del usuario JOIN, ordenados por uso reciente.
#[tauri::command]
pub fn app_listar_dispositivos(db: State<'_, Database>) -> Result<Vec<DispositivoApp>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let mut stmt = conn
        .prepare(
            "SELECT t.id, t.usuario_id, u.nombre,
                    t.dispositivo_nombre, t.dispositivo_modelo, t.dispositivo_so,
                    t.created_at, t.last_used_at, t.revoked,
                    CAST((julianday('now', 'localtime') - julianday(t.last_used_at)) * 24 * 60 AS INTEGER) AS mins
             FROM app_tokens t
             JOIN usuarios u ON t.usuario_id = u.id
             ORDER BY t.revoked ASC, t.last_used_at DESC",
        )
        .map_err(|e| e.to_string())?;

    let dispositivos: Vec<DispositivoApp> = stmt
        .query_map([], |r| {
            Ok(DispositivoApp {
                id: r.get(0)?,
                usuario_id: r.get(1)?,
                usuario_nombre: r.get(2)?,
                dispositivo_nombre: r.get(3)?,
                dispositivo_modelo: r.get(4)?,
                dispositivo_so: r.get(5)?,
                created_at: r.get(6)?,
                last_used_at: r.get(7)?,
                revoked: r.get::<_, i64>(8)? != 0,
                minutos_inactivo: r.get::<_, Option<i64>>(9)?.unwrap_or(0),
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    Ok(dispositivos)
}

/// Revoca un dispositivo (no lo borra — solo marca `revoked = 1` para auditoría).
/// El próximo request del dispositivo recibirá 401 y la app deberá hacer login otra vez.
#[tauri::command]
pub fn app_revocar_dispositivo(db: State<'_, Database>, id: i64) -> Result<(), String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let filas = conn
        .execute(
            "UPDATE app_tokens SET revoked = 1 WHERE id = ?1",
            params![id],
        )
        .map_err(|e| e.to_string())?;
    if filas == 0 {
        return Err("Dispositivo no encontrado".to_string());
    }
    Ok(())
}

/// Borra físicamente un dispositivo de la tabla — para limpieza de tokens
/// viejos revocados. No revoca, lo elimina.
#[tauri::command]
pub fn app_eliminar_dispositivo(db: State<'_, Database>, id: i64) -> Result<(), String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    conn.execute("DELETE FROM app_tokens WHERE id = ?1", params![id])
        .map_err(|e| e.to_string())?;
    Ok(())
}

// ─── v2.4.4 — Sprint 3c: QR de emparejamiento ─────────────────────────────

#[derive(Debug, Serialize, Clone)]
pub struct QrEmparejamiento {
    /// PNG del código QR encoded en base64 (sin prefijo `data:image/png;base64,`)
    pub qr_png_b64: String,
    /// JSON que el QR contiene (la app lo parsea al escanear)
    pub payload: String,
    /// IP local detectada
    pub ip: String,
    /// Puerto del servidor HTTP
    pub port: u16,
    /// Nombre del negocio (para confirmación visual en la app)
    pub negocio: String,
    /// `true` si el módulo `restaurante` está activo
    pub tiene_restaurante: bool,
}

/// Genera el código QR para que la app móvil escanee y se autoconfigure.
///
/// El QR contiene un JSON con `{ ip, port, service, negocio, restaurante }`.
/// La app al escanear guarda esta info y luego pide PIN normal — el QR NO
/// contiene credenciales (más seguro: si alguien fotografía el QR no puede
/// loguearse sin saber el PIN).
#[tauri::command]
pub fn app_generar_qr_emparejamiento(
    db: State<'_, Database>,
) -> Result<QrEmparejamiento, String> {
    use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
    use qrcode::QrCode;

    let conn = db.conn.lock().map_err(|e| e.to_string())?;

    let port: u16 = conn
        .query_row(
            "SELECT value FROM config WHERE key = 'servidor_puerto'",
            [],
            |r| r.get::<_, String>(0),
        )
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(8847);

    let negocio: String = conn
        .query_row(
            "SELECT value FROM config WHERE key = 'nombre_negocio'",
            [],
            |r| r.get(0),
        )
        .unwrap_or_else(|_| "Clouget POS".to_string());

    let modulos_json: String = conn
        .query_row(
            "SELECT value FROM config WHERE key = 'licencia_modulos'",
            [],
            |r| r.get(0),
        )
        .unwrap_or_default();
    drop(conn);

    let modulos: Vec<String> = serde_json::from_str(&modulos_json).unwrap_or_default();
    let tiene_restaurante = modulos.iter().any(|m| m == "restaurante");
    let tiene_app_movil = modulos.iter().any(|m| m == "app_movil");

    if !tiene_app_movil {
        return Err("El módulo 'app_movil' no está activo en su licencia".to_string());
    }

    let ip = super::discovery::obtener_ip_local()
        .ok_or_else(|| "No se pudo detectar la IP local de esta PC. Verifique conexión a la red.".to_string())?;

    // Payload JSON que va dentro del QR (compacto)
    let payload = serde_json::json!({
        "service": "clouget-pos",
        "ip": ip,
        "port": port,
        "negocio": negocio,
        "restaurante": tiene_restaurante,
        "version": env!("CARGO_PKG_VERSION"),
    });
    let payload_str = serde_json::to_string(&payload).map_err(|e| e.to_string())?;

    // Generar QR — usamos to_colors() y armamos el bitmap manual (mismo
    // patrón que sri/ride.rs para no depender de la feature `image` de qrcode)
    let code = QrCode::new(payload_str.as_bytes())
        .map_err(|e| format!("Error generando QR: {}", e))?;
    let modules = code.to_colors();
    let width = code.width() as u32;
    let scale: u32 = 8; // 8 px por módulo → ~280 px lado para un QR pequeño
    let border: u32 = 4;
    let img_size = (width + border * 2) * scale;

    // Buffer luma8 (1 byte por píxel, 0=negro, 255=blanco)
    let mut img_buf = vec![255u8; (img_size * img_size) as usize];
    for (i, color) in modules.iter().enumerate() {
        let x = (i as u32) % width;
        let y = (i as u32) / width;
        if *color == qrcode::types::Color::Dark {
            let px = (x + border) * scale;
            let py = (y + border) * scale;
            for dy in 0..scale {
                for dx in 0..scale {
                    let idx = ((py + dy) * img_size + (px + dx)) as usize;
                    if idx < img_buf.len() {
                        img_buf[idx] = 0;
                    }
                }
            }
        }
    }

    // Convertir Luma8 buffer → PNG en memoria
    let img_buffer: image::ImageBuffer<image::Luma<u8>, Vec<u8>> =
        image::ImageBuffer::from_raw(img_size, img_size, img_buf)
            .ok_or_else(|| "Error creando ImageBuffer del QR".to_string())?;
    let mut buf: Vec<u8> = Vec::new();
    image::DynamicImage::ImageLuma8(img_buffer)
        .write_to(&mut buf, image::ImageOutputFormat::Png)
        .map_err(|e| format!("Error encoding PNG: {}", e))?;
    let qr_png_b64 = BASE64.encode(&buf);

    Ok(QrEmparejamiento {
        qr_png_b64,
        payload: payload_str,
        ip,
        port,
        negocio,
        tiene_restaurante,
    })
}

