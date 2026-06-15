pub mod schema;

use crate::models::SesionActiva;
use rusqlite::Connection;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

/// Base de datos SQLite compartida. Clonable gracias a Arc<Mutex<Connection>>.
#[derive(Clone)]
pub struct Database {
    pub conn: Arc<Mutex<Connection>>,
}

/// Estado de sesión compartido. Clonable gracias a Arc<Mutex<...>>.
#[derive(Clone)]
pub struct SesionState {
    pub sesion: Arc<Mutex<Option<SesionActiva>>>,
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
            conn: Arc::new(Mutex::new(conn)),
        };

        db.run_migrations()?;

        Ok(db)
    }

    /// Retorna la ruta de la base de datos (accesible desde otros módulos)
    pub fn get_db_path_pub() -> PathBuf {
        Self::get_db_path()
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

        // Migración: destino_preparacion en productos (Restaurante v2.3.55)
        // Valores: 'COCINA' (default) | 'BARRA' | 'DIRECTO' (mesero despacha sin pasar por cocina)
        // Default 'COCINA' es safe: productos existentes mantienen el comportamiento anterior.
        conn.execute(
            "ALTER TABLE productos ADD COLUMN destino_preparacion TEXT NOT NULL DEFAULT 'COCINA'",
            [],
        )
        .ok();

        // Migración: precio_minimo en productos (piso de precio opcional).
        // NULL = sin piso (comportamiento actual). Si se define, nadie puede
        // vender por debajo, ni con permiso para editar el precio en la venta.
        let _ = conn.execute("ALTER TABLE productos ADD COLUMN precio_minimo REAL", []);

        // Migración: trazabilidad de reembolso en notas_credito (v2.3.62)
        // Antes solo se calculaba el desglose efectivo/transfer/credito y se mostraba
        // al usuario, pero no se persistía. Si volvías a buscar la NC mañana no sabías
        // cómo se devolvió el dinero. Estas columnas guardan esa info para auditoría.
        conn.execute(
            "ALTER TABLE notas_credito ADD COLUMN tipo_devolucion TEXT NOT NULL DEFAULT 'TOTAL'",
            [],
        )
        .ok();
        conn.execute(
            "ALTER TABLE notas_credito ADD COLUMN monto_efectivo_devuelto REAL NOT NULL DEFAULT 0",
            [],
        )
        .ok();
        conn.execute(
            "ALTER TABLE notas_credito ADD COLUMN monto_transfer_devuelto REAL NOT NULL DEFAULT 0",
            [],
        )
        .ok();
        conn.execute(
            "ALTER TABLE notas_credito ADD COLUMN monto_credito_devuelto REAL NOT NULL DEFAULT 0",
            [],
        )
        .ok();
        conn.execute(
            "ALTER TABLE notas_credito ADD COLUMN retiro_caja_id INTEGER",
            [],
        )
        .ok();
        conn.execute(
            "ALTER TABLE notas_credito ADD COLUMN metodo_reembolso TEXT NOT NULL DEFAULT 'EFECTIVO'",
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

        // Migracion v2.6.25: presentaciones de compra por producto.
        // Columnas snapshot en compra_detalles. Idempotentes — .ok() ignora si la
        // columna ya existe.
        conn.execute("ALTER TABLE compra_detalles ADD COLUMN presentacion_id INTEGER", []).ok();
        conn.execute("ALTER TABLE compra_detalles ADD COLUMN presentacion_nombre TEXT", []).ok();
        conn.execute("ALTER TABLE compra_detalles ADD COLUMN presentacion_factor REAL", []).ok();
        conn.execute("ALTER TABLE compra_detalles ADD COLUMN cantidad_presentacion REAL", []).ok();

        // Migracion v2.6.26: extender snapshot a venta_detalles para Notas de Entrega.
        conn.execute("ALTER TABLE venta_detalles ADD COLUMN presentacion_id INTEGER", []).ok();
        conn.execute("ALTER TABLE venta_detalles ADD COLUMN presentacion_nombre TEXT", []).ok();
        conn.execute("ALTER TABLE venta_detalles ADD COLUMN presentacion_factor REAL", []).ok();
        conn.execute("ALTER TABLE venta_detalles ADD COLUMN cantidad_presentacion REAL", []).ok();
        // v2.6.32: snapshot del lote vendido (trazabilidad/recall)
        conn.execute("ALTER TABLE venta_detalles ADD COLUMN lote_numero TEXT", []).ok();
        conn.execute("ALTER TABLE venta_detalles ADD COLUMN lote_fecha_caducidad TEXT", []).ok();

        // Seed admin por defecto si no hay usuarios
        seed_default_admin(&conn);

        // Migracion v2.3.25: arreglar demo descuadrado de v2.3.23/v2.3.24.
        // El demo viejo sembraba retiros de $200+$50+$150=$400 que dejaban la
        // caja con descuadre permanente. Ahora sembramos $25+$15+$20=$60.
        // Si detectamos los retiros viejos en caja_id=1 (caja demo), los
        // borramos y re-insertamos los correctos. Solo se ejecuta UNA vez:
        // las re-ejecuciones detectaran que los retiros viejos ya no estan.
        let demo_activo: String = conn
            .query_row(
                "SELECT value FROM config WHERE key = 'demo_activo'",
                [],
                |row| row.get(0),
            )
            .unwrap_or_default();
        if demo_activo == "1" {
            // Borrar SOLO los retiros viejos del seed demo (identificables por
            // monto + motivo + caja_id=1). No tocamos retiros que el usuario
            // haya creado manualmente.
            let borrados = conn
                .execute(
                    "DELETE FROM retiros_caja
                     WHERE caja_id = 1
                       AND ((monto = 200.00 AND motivo = 'Deposito banco al cierre del dia')
                         OR (monto = 50.00 AND motivo = 'Pago a proveedor de pan')
                         OR (monto = 150.00 AND motivo = 'Deposito en Pichincha'))",
                    [],
                )
                .unwrap_or(0);
            if borrados > 0 {
                // Re-sembrar los retiros corregidos (mismos motivos, montos chicos).
                let _ = conn.execute_batch(
                    "INSERT INTO retiros_caja (caja_id, monto, motivo, banco_id, referencia, usuario, estado, fecha) VALUES
                        (1, 25.00, 'Deposito banco al cierre del dia', 1, 'DEP-2024001', 'Admin', 'DEPOSITADO', datetime('now', 'localtime', '-3 days')),
                        (1, 15.00, 'Pago a proveedor de pan', NULL, NULL, 'Admin', 'SIN_DEPOSITO', datetime('now', 'localtime', '-2 days')),
                        (1, 20.00, 'Deposito en Pichincha', 1, NULL, 'Admin', 'EN_TRANSITO', datetime('now', 'localtime', '-1 days'));"
                );
                // Reset monto_esperado para que obtener_caja_abierta lo
                // recalcule en el proximo read (calcular_monto_esperado_actual).
                let _ = conn.execute(
                    "UPDATE caja SET monto_esperado = monto_inicial WHERE id = 1",
                    [],
                );
            }
        }

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
