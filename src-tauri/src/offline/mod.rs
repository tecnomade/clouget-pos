pub mod cache;

use rusqlite::Connection;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

/// Base de datos offline local para terminales cliente.
/// Almacena cache de productos y cola de operaciones pendientes.
#[derive(Clone)]
pub struct OfflineDb {
    pub conn: Arc<Mutex<Connection>>,
}

impl OfflineDb {
    pub fn new() -> Result<Self, rusqlite::Error> {
        let db_path = Self::get_path();
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent).ok();
        }

        let conn = Connection::open(&db_path)?;
        conn.execute_batch(
            "PRAGMA journal_mode = WAL;
             PRAGMA synchronous = NORMAL;
             PRAGMA foreign_keys = ON;",
        )?;

        // Crear tablas de cache y cola
        conn.execute_batch(
            "
            CREATE TABLE IF NOT EXISTS cache_productos (
                id INTEGER PRIMARY KEY,
                codigo TEXT,
                codigo_barras TEXT,
                nombre TEXT NOT NULL,
                precio_venta REAL NOT NULL,
                iva_porcentaje REAL NOT NULL DEFAULT 0,
                stock_actual REAL NOT NULL DEFAULT 0,
                stock_minimo REAL NOT NULL DEFAULT 0,
                categoria_nombre TEXT,
                es_servicio INTEGER NOT NULL DEFAULT 0,
                updated_at TEXT
            );

            CREATE TABLE IF NOT EXISTS cache_clientes (
                id INTEGER PRIMARY KEY,
                tipo_identificacion TEXT,
                identificacion TEXT,
                nombre TEXT NOT NULL,
                direccion TEXT,
                telefono TEXT,
                email TEXT,
                lista_precio_id INTEGER
            );

            CREATE TABLE IF NOT EXISTS cola_operaciones (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                comando TEXT NOT NULL,
                params_json TEXT NOT NULL,
                created_at TEXT NOT NULL DEFAULT (datetime('now','localtime')),
                estado TEXT NOT NULL DEFAULT 'PENDIENTE',
                intentos INTEGER NOT NULL DEFAULT 0,
                ultimo_error TEXT,
                resultado_json TEXT
            );

            CREATE TABLE IF NOT EXISTS cache_config (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS secuenciales_reservados (
                tipo_documento TEXT PRIMARY KEY,
                desde INTEGER NOT NULL,
                hasta INTEGER NOT NULL,
                actual INTEGER NOT NULL
            );
            ",
        )?;

        Ok(OfflineDb {
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    fn get_path() -> PathBuf {
        #[cfg(target_os = "windows")]
        {
            std::env::var("LOCALAPPDATA")
                .ok()
                .map(|p| PathBuf::from(p).join("CloudgetPOS").join("offline.db"))
                .unwrap_or_else(|| PathBuf::from("offline.db"))
        }
        #[cfg(not(target_os = "windows"))]
        {
            std::env::var("HOME")
                .ok()
                .map(|p| PathBuf::from(p).join(".clouget-pos").join("offline.db"))
                .unwrap_or_else(|| PathBuf::from("offline.db"))
        }
    }
}
