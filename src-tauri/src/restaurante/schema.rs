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

        -- ─── Abonos / pagos parciales sobre la mesa (v2.5.91) ──────────
        -- Espejo de st_abonos: el cliente puede pagar por partes mientras la
        -- mesa sigue consumiendo. El dinero entra a la caja como HOLDING
        -- (anticipo) y al COBRAR el pedido pasa a APLICADO (se descuenta del
        -- total). Así el arqueo cuadra igual que con los anticipos de ST.
        --   estado: HOLDING | APLICADO | DEVUELTO
        CREATE TABLE IF NOT EXISTS rest_pedido_abonos (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            pedido_id INTEGER NOT NULL,
            monto REAL NOT NULL,
            forma_pago TEXT NOT NULL DEFAULT 'EFECTIVO',
            banco_id INTEGER,
            referencia_pago TEXT,
            caja_id INTEGER,
            estado TEXT NOT NULL DEFAULT 'HOLDING',
            venta_id_aplicado INTEGER,
            fecha TEXT NOT NULL DEFAULT (datetime('now', 'localtime')),
            fecha_aplicado TEXT,
            usuario_id INTEGER,
            usuario_nombre TEXT,
            observacion TEXT,
            FOREIGN KEY (pedido_id) REFERENCES rest_pedidos_abiertos(id) ON DELETE CASCADE,
            FOREIGN KEY (banco_id) REFERENCES cuentas_banco(id),
            FOREIGN KEY (caja_id) REFERENCES caja(id),
            FOREIGN KEY (venta_id_aplicado) REFERENCES ventas(id)
        );
        CREATE INDEX IF NOT EXISTS idx_rest_pedido_abonos_pedido ON rest_pedido_abonos(pedido_id);
        CREATE INDEX IF NOT EXISTS idx_rest_pedido_abonos_estado ON rest_pedido_abonos(estado);

        -- ─── Sub-cuentas (división de cuenta v2.3.69) ──────────────
        -- Cuando un grupo decide pagar por separado, el pedido se divide en N
        -- sub-cuentas. Cada una se cobra de forma independiente (su propia
        -- forma de pago), generando una venta REAL apuntando a un producto
        -- especial '_DIVISION_CUENTA_' (es_servicio=1, IVA 0%).
        --
        -- Estado:  PENDIENTE | COBRADA
        -- numero:  1, 2, 3, ... N (orden visible al mesero)
        -- venta_id: NULL hasta que se cobra; luego apunta a la venta generada
        --
        -- Cuando TODAS las sub-cuentas de un pedido están COBRADAS, el sistema
        -- marca el pedido como COBRADO (vinculándolo con la venta de la PRIMERA
        -- sub-cuenta cobrada) y libera la(s) mesa(s).
        --
        -- LIMITACIÓN MVP: el stock de los items reales NO se descuenta porque
        -- cada venta es por monto plano. Aceptable para restaurantes pequeños.
        CREATE TABLE IF NOT EXISTS rest_subcuentas (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            pedido_id INTEGER NOT NULL,
            numero INTEGER NOT NULL,
            total REAL NOT NULL,
            estado TEXT NOT NULL DEFAULT 'PENDIENTE',
            forma_pago TEXT,
            banco_id INTEGER,
            referencia_pago TEXT,
            venta_id INTEGER,
            fecha_cobro TEXT,
            FOREIGN KEY (pedido_id) REFERENCES rest_pedidos_abiertos(id) ON DELETE CASCADE,
            FOREIGN KEY (banco_id) REFERENCES cuentas_banco(id),
            FOREIGN KEY (venta_id) REFERENCES ventas(id)
        );
        CREATE INDEX IF NOT EXISTS idx_rest_subcuentas_pedido ON rest_subcuentas(pedido_id);
        CREATE INDEX IF NOT EXISTS idx_rest_subcuentas_estado ON rest_subcuentas(estado);
        ",
    )
}

/// Inserta zonas y mesas iniciales si la tabla está vacía.
/// Asumimos un restaurante simple: 1 zona "Salón" con 6 mesas de capacidad 4.
/// El dueño puede borrarlas y rehacer todo desde Configuración.
///
/// v2.3.69: También garantiza la existencia del producto especial
/// `_DIVISION_CUENTA_` (es_servicio=1, IVA 0%, oculto en POS) usado al cobrar
/// sub-cuentas — cada sub-cuenta se materializa como una venta de ese producto
/// con precio_unitario = monto de la sub-cuenta.
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

    // v2.3.69 — Producto especial "_DIVISION_CUENTA_" usado por el cobro de
    // sub-cuentas. Es un servicio (no descuenta stock), IVA 0% (el IVA real ya
    // se contabilizó en los items del pedido — la división es solo un cobro
    // partido). Se identifica por su `codigo` único '_DIVISION_CUENTA_' para
    // que sea fácil filtrarlo en reportes.
    let existe: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM productos WHERE codigo = '_DIVISION_CUENTA_'",
            params![],
            |row| row.get(0),
        )
        .unwrap_or(0);
    if existe == 0 {
        // INSERT OR IGNORE por si en concurrencia se intenta crear dos veces
        let _ = conn.execute(
            "INSERT OR IGNORE INTO productos
             (codigo, nombre, descripcion, precio_costo, precio_venta,
              iva_porcentaje, incluye_iva, stock_actual, stock_minimo,
              unidad_medida, es_servicio, activo)
             VALUES ('_DIVISION_CUENTA_', 'Cuota Mesa Restaurante', 'Producto interno para división de cuenta — no se vende manualmente',
                     0, 0, 0, 0, 0, 0, 'UND', 1, 1)",
            params![],
        );
    }

    Ok(())
}
