//! Schema SQL del módulo App Móvil.
//!
//! Tablas:
//! - `app_tokens` — sesiones activas de dispositivos móviles emparejados
//!
//! Cada vez que un mesero/cocinero/vendedor hace login PIN en la app, se
//! genera un token único (UUID v4) y se persiste aquí. El token se envía
//! en el header `Authorization: Bearer <token>` en cada request.
//!
//! El admin puede revocar un dispositivo desde Configuración → App Móvil
//! sin afectar otros dispositivos del mismo usuario (cada dispositivo tiene
//! su propio token).

use rusqlite::Connection;

/// Crea las tablas del módulo si no existen.
pub fn create_tables(conn: &Connection) -> Result<(), rusqlite::Error> {
    conn.execute_batch(
        "
        -- ─── Tokens de sesión por dispositivo móvil ───────────────────────
        -- Un usuario puede tener N dispositivos activos (un mesero con su
        -- teléfono + un tablet compartido). Cada uno con su token.
        --
        -- revoked = 1 → bloqueado (admin lo revocó). El middleware lo rechaza.
        CREATE TABLE IF NOT EXISTS app_tokens (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            usuario_id INTEGER NOT NULL,
            token TEXT NOT NULL UNIQUE,
            dispositivo_nombre TEXT,                  -- 'iPad cocina', 'iPhone Juan', etc. (lo manda la app al loguear)
            dispositivo_modelo TEXT,                  -- 'iPad Pro 11', 'Galaxy S21', etc. (Expo Constants)
            dispositivo_so TEXT,                      -- 'iOS 17.1', 'Android 14', etc.
            push_token TEXT,                          -- Expo Push Token para notifs (opcional, se setea después)
            created_at TEXT NOT NULL DEFAULT (datetime('now', 'localtime')),
            last_used_at TEXT NOT NULL DEFAULT (datetime('now', 'localtime')),
            revoked INTEGER NOT NULL DEFAULT 0,
            FOREIGN KEY (usuario_id) REFERENCES usuarios(id) ON DELETE CASCADE
        );
        CREATE INDEX IF NOT EXISTS idx_app_tokens_token ON app_tokens(token);
        CREATE INDEX IF NOT EXISTS idx_app_tokens_usuario ON app_tokens(usuario_id);
        CREATE INDEX IF NOT EXISTS idx_app_tokens_revoked ON app_tokens(revoked);
        ",
    )
}
