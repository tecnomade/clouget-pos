pub mod schema;

use crate::models::SesionActiva;
use rusqlite::Connection;
use std::path::PathBuf;
use std::sync::Mutex;

pub struct Database {
    pub conn: Mutex<Connection>,
}

pub struct SesionState {
    pub sesion: Mutex<Option<SesionActiva>>,
}

impl Database {
    pub fn new() -> Result<Self, rusqlite::Error> {
        let db_path = Self::get_db_path();

        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent).ok();
        }

        let conn = Connection::open(&db_path)?;

        // Optimizaciones SQLite para POS
        conn.execute_batch(
            "PRAGMA journal_mode = WAL;
             PRAGMA synchronous = NORMAL;
             PRAGMA cache_size = -8000;
             PRAGMA foreign_keys = ON;
             PRAGMA busy_timeout = 5000;",
        )?;

        let db = Database {
            conn: Mutex::new(conn),
        };

        db.run_migrations()?;

        Ok(db)
    }

    fn get_db_path() -> PathBuf {
        let mut path = dirs_next().unwrap_or_else(|| PathBuf::from("."));
        path.push("clouget-pos.db");
        path
    }

    fn run_migrations(&self) -> Result<(), rusqlite::Error> {
        let conn = self.conn.lock().unwrap();
        schema::create_tables(&conn)?;

        // Migraciones incrementales (safe: .ok() ignora si columna ya existe)
        conn.execute("ALTER TABLE caja ADD COLUMN usuario_id INTEGER", [])
            .ok();
        conn.execute("ALTER TABLE ventas ADD COLUMN usuario_id INTEGER", [])
            .ok();
        conn.execute(
            "ALTER TABLE ventas ADD COLUMN estado_sri TEXT NOT NULL DEFAULT 'NO_APLICA'",
            [],
        )
        .ok();

        // Migración: columna email_enviado en ventas
        conn.execute(
            "ALTER TABLE ventas ADD COLUMN email_enviado INTEGER NOT NULL DEFAULT 0",
            [],
        )
        .ok();

        // Migración: columna numero_factura_nc en notas_credito (número SRI asignado)
        conn.execute(
            "ALTER TABLE notas_credito ADD COLUMN numero_factura_nc TEXT",
            [],
        )
        .ok();

        // Migración: configurar email service si está vacío
        conn.execute(
            "UPDATE config SET value = 'https://email.clouget.com' WHERE key = 'email_service_url' AND value = ''",
            [],
        )
        .ok();
        conn.execute(
            "UPDATE config SET value = 'clouget-email-dev-key' WHERE key = 'email_service_api_key' AND value = ''",
            [],
        )
        .ok();

        // Seed admin por defecto si no hay usuarios
        seed_default_admin(&conn);

        Ok(())
    }
}

/// Inserta el usuario ADMINISTRADOR con PIN 0000 si no hay usuarios
fn seed_default_admin(conn: &Connection) {
    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM usuarios", [], |row| row.get(0))
        .unwrap_or(0);

    if count == 0 {
        let salt = crate::utils::generar_salt();
        let pin_hash = crate::utils::hash_pin(&salt, "0000");
        conn.execute(
            "INSERT INTO usuarios (nombre, pin_hash, pin_salt, rol, activo)
             VALUES ('ADMINISTRADOR', ?1, ?2, 'ADMIN', 1)",
            rusqlite::params![pin_hash, salt],
        )
        .ok();
    }
}

/// Retorna el directorio de datos de la aplicación
fn dirs_next() -> Option<PathBuf> {
    #[cfg(target_os = "windows")]
    {
        std::env::var("LOCALAPPDATA")
            .ok()
            .map(|p| PathBuf::from(p).join("CloudgetPOS"))
    }
    #[cfg(not(target_os = "windows"))]
    {
        std::env::var("HOME")
            .ok()
            .map(|p| PathBuf::from(p).join(".clouget-pos"))
    }
}
