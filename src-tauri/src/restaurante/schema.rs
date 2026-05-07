//! Esquema SQL del módulo Restaurante.
//!
//! Tablas:
//! - `rest_zonas`              — agrupa mesas (Salón, Terraza, Barra, etc.)
//! - `rest_mesas`              — mesas físicas con capacidad
//! - `rest_pedidos_abiertos`   — comanda activa por mesa (acumula items hasta cobrar)
//! - `rest_pedido_items`       — líneas del pedido con info adicional + estado cocina
//!
//! Todas las tablas usan prefijo `rest_` para no chocar con el resto del schema.

use rusqlite::{params, Connection};

/// Crea las tablas del módulo si no existen.
pub fn create_tables(conn: &Connection) -> Result<(), rusqlite::Error> {
    conn.execute_batch(
        "
        -- ─── Zonas (Salón, Terraza, Barra, etc.) ───────────────────
        CREATE TABLE IF NOT EXISTS rest_zonas (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            nombre TEXT NOT NULL,
            color TEXT NOT NULL DEFAULT '#3b82f6',
            orden INTEGER NOT NULL DEFAULT 0,
            activa INTEGER NOT NULL DEFAULT 1,
            created_at TEXT NOT NULL DEFAULT (datetime('now', 'localtime'))
        );

        -- ─── Mesas físicas ─────────────────────────────────────────
        CREATE TABLE IF NOT EXISTS rest_mesas (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            zona_id INTEGER,
            nombre TEXT NOT NULL,
            capacidad INTEGER NOT NULL DEFAULT 4,
            orden INTEGER NOT NULL DEFAULT 0,
            activa INTEGER NOT NULL DEFAULT 1,
            created_at TEXT NOT NULL DEFAULT (datetime('now', 'localtime')),
            FOREIGN KEY (zona_id) REFERENCES rest_zonas(id)
        );
        CREATE INDEX IF NOT EXISTS idx_rest_mesas_zona ON rest_mesas(zona_id);

        -- ─── Pedidos abiertos (comanda activa por mesa) ────────────
        -- Estado: ABIERTO | CUENTA_PEDIDA | COBRADO | CANCELADO
        CREATE TABLE IF NOT EXISTS rest_pedidos_abiertos (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            mesa_id INTEGER NOT NULL,
            mesero_id INTEGER,
            mesero_nombre TEXT,
            comensales INTEGER NOT NULL DEFAULT 1,
            estado TEXT NOT NULL DEFAULT 'ABIERTO',
            observacion TEXT,
            fecha_apertura TEXT NOT NULL DEFAULT (datetime('now', 'localtime')),
            fecha_cuenta TEXT,
            fecha_cierre TEXT,
            venta_id INTEGER,
            FOREIGN KEY (mesa_id) REFERENCES rest_mesas(id),
            FOREIGN KEY (mesero_id) REFERENCES usuarios(id),
            FOREIGN KEY (venta_id) REFERENCES ventas(id)
        );
        CREATE INDEX IF NOT EXISTS idx_rest_pedidos_mesa ON rest_pedidos_abiertos(mesa_id);
        CREATE INDEX IF NOT EXISTS idx_rest_pedidos_estado ON rest_pedidos_abiertos(estado);

        -- ─── Items de un pedido (lo que va consumiendo cada mesa) ──
        -- estado_cocina: PENDIENTE | EN_PREPARACION | LISTO | ENTREGADO
        -- enviado_cocina: 0=nuevo (no impreso aún) / 1=ya se envió a cocina
        CREATE TABLE IF NOT EXISTS rest_pedido_items (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            pedido_id INTEGER NOT NULL,
            producto_id INTEGER NOT NULL,
            cantidad REAL NOT NULL DEFAULT 1,
            precio_unit REAL NOT NULL DEFAULT 0,
            info_adicional TEXT,
            enviado_cocina INTEGER NOT NULL DEFAULT 0,
            estado_cocina TEXT NOT NULL DEFAULT 'PENDIENTE',
            fecha_creacion TEXT NOT NULL DEFAULT (datetime('now', 'localtime')),
            fecha_envio_cocina TEXT,
            FOREIGN KEY (pedido_id) REFERENCES rest_pedidos_abiertos(id) ON DELETE CASCADE,
            FOREIGN KEY (producto_id) REFERENCES productos(id)
        );
        CREATE INDEX IF NOT EXISTS idx_rest_pedido_items_pedido ON rest_pedido_items(pedido_id);
        CREATE INDEX IF NOT EXISTS idx_rest_pedido_items_estado ON rest_pedido_items(estado_cocina);

        -- ─── Mesas EXTRA unidas a un pedido (v2.3.68) ──────────────
        -- Para grupos grandes que ocupan varias mesas. La mesa
        -- 'principal' sigue siendo `pedido.mesa_id` (no rompe schema).
        -- Esta tabla M:N enumera las mesas SECUNDARIAS unidas al pedido.
        --
        -- Reglas:
        --  - Una mesa libre puede unirse a un pedido existente
        --  - Una mesa con pedido propio NO puede unirse a otro
        --  - Al COBRAR / CANCELAR el pedido, todas las mesas extra se
        --    liberan automáticamente (porque la query de mesas filtra
        --    pedidos ABIERTO/CUENTA_PEDIDA — el cambio de estado las
        --    suelta sin lógica adicional)
        CREATE TABLE IF NOT EXISTS rest_pedido_mesas_extra (
            pedido_id INTEGER NOT NULL,
            mesa_id INTEGER NOT NULL,
            fecha_union TEXT NOT NULL DEFAULT (datetime('now', 'localtime')),
            PRIMARY KEY (pedido_id, mesa_id),
            FOREIGN KEY (pedido_id) REFERENCES rest_pedidos_abiertos(id) ON DELETE CASCADE,
            FOREIGN KEY (mesa_id) REFERENCES rest_mesas(id)
        );
        CREATE INDEX IF NOT EXISTS idx_rest_pedido_mesas_extra_mesa ON rest_pedido_mesas_extra(mesa_id);
        CREATE INDEX IF NOT EXISTS idx_rest_pedido_mesas_extra_pedido ON rest_pedido_mesas_extra(pedido_id);
        ",
    )
}

/// Inserta zonas y mesas iniciales si la tabla está vacía.
/// Asumimos un restaurante simple: 1 zona "Salón" con 6 mesas de capacidad 4.
/// El dueño puede borrarlas y rehacer todo desde Configuración.
pub fn seed_default(conn: &Connection) -> Result<(), rusqlite::Error> {
    let count_zonas: i64 = conn
        .query_row("SELECT COUNT(*) FROM rest_zonas", params![], |row| row.get(0))
        .unwrap_or(0);

    if count_zonas == 0 {
        conn.execute(
            "INSERT INTO rest_zonas (nombre, color, orden) VALUES (?1, ?2, 0)",
            params!["Salón", "#3b82f6"],
        )?;

        let zona_id: i64 = conn.last_insert_rowid();

        for i in 1..=6 {
            conn.execute(
                "INSERT INTO rest_mesas (zona_id, nombre, capacidad, orden) VALUES (?1, ?2, 4, ?3)",
                params![zona_id, format!("Mesa {}", i), i],
            )?;
        }
    }

    Ok(())
}
