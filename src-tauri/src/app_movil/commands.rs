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
