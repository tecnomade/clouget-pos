use rusqlite::Connection;

pub fn create_tables(conn: &Connection) -> Result<(), rusqlite::Error> {
    conn.execute_batch(
        "
        -- Configuración del negocio
        CREATE TABLE IF NOT EXISTS config (
            key TEXT PRIMARY KEY,
            value TEXT NOT NULL
        );

        -- Categorías de productos
        CREATE TABLE IF NOT EXISTS categorias (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            nombre TEXT NOT NULL,
            descripcion TEXT,
            activo INTEGER NOT NULL DEFAULT 1,
            created_at TEXT NOT NULL DEFAULT (datetime('now', 'localtime')),
            updated_at TEXT NOT NULL DEFAULT (datetime('now', 'localtime'))
        );

        -- Productos
        CREATE TABLE IF NOT EXISTS productos (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            codigo TEXT UNIQUE,
            codigo_barras TEXT UNIQUE,
            nombre TEXT NOT NULL,
            descripcion TEXT,
            categoria_id INTEGER,
            precio_costo REAL NOT NULL DEFAULT 0,
            precio_venta REAL NOT NULL DEFAULT 0,
            iva_porcentaje REAL NOT NULL DEFAULT 0,
            incluye_iva INTEGER NOT NULL DEFAULT 0,
            stock_actual REAL NOT NULL DEFAULT 0,
            stock_minimo REAL NOT NULL DEFAULT 0,
            unidad_medida TEXT NOT NULL DEFAULT 'UND',
            es_servicio INTEGER NOT NULL DEFAULT 0,
            activo INTEGER NOT NULL DEFAULT 1,
            created_at TEXT NOT NULL DEFAULT (datetime('now', 'localtime')),
            updated_at TEXT NOT NULL DEFAULT (datetime('now', 'localtime')),
            FOREIGN KEY (categoria_id) REFERENCES categorias(id)
        );

        CREATE INDEX IF NOT EXISTS idx_productos_codigo ON productos(codigo);
        CREATE INDEX IF NOT EXISTS idx_productos_codigo_barras ON productos(codigo_barras);
        CREATE INDEX IF NOT EXISTS idx_productos_nombre ON productos(nombre);
        CREATE INDEX IF NOT EXISTS idx_productos_categoria ON productos(categoria_id);

        -- Clientes
        CREATE TABLE IF NOT EXISTS clientes (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            tipo_identificacion TEXT NOT NULL DEFAULT 'CEDULA',
            identificacion TEXT UNIQUE,
            nombre TEXT NOT NULL,
            direccion TEXT,
            telefono TEXT,
            email TEXT,
            activo INTEGER NOT NULL DEFAULT 1,
            created_at TEXT NOT NULL DEFAULT (datetime('now', 'localtime')),
            updated_at TEXT NOT NULL DEFAULT (datetime('now', 'localtime'))
        );

        CREATE INDEX IF NOT EXISTS idx_clientes_identificacion ON clientes(identificacion);
        CREATE INDEX IF NOT EXISTS idx_clientes_nombre ON clientes(nombre);

        -- Ventas (cabecera)
        CREATE TABLE IF NOT EXISTS ventas (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            numero TEXT UNIQUE NOT NULL,
            cliente_id INTEGER,
            fecha TEXT NOT NULL DEFAULT (datetime('now', 'localtime')),
            subtotal_sin_iva REAL NOT NULL DEFAULT 0,
            subtotal_con_iva REAL NOT NULL DEFAULT 0,
            descuento REAL NOT NULL DEFAULT 0,
            iva REAL NOT NULL DEFAULT 0,
            total REAL NOT NULL DEFAULT 0,
            forma_pago TEXT NOT NULL DEFAULT 'EFECTIVO',
            monto_recibido REAL NOT NULL DEFAULT 0,
            cambio REAL NOT NULL DEFAULT 0,
            estado TEXT NOT NULL DEFAULT 'COMPLETADA',
            tipo_documento TEXT NOT NULL DEFAULT 'NOTA_VENTA',
            autorizacion_sri TEXT,
            clave_acceso TEXT,
            usuario TEXT,
            observacion TEXT,
            anulada INTEGER NOT NULL DEFAULT 0,
            created_at TEXT NOT NULL DEFAULT (datetime('now', 'localtime')),
            FOREIGN KEY (cliente_id) REFERENCES clientes(id)
        );

        CREATE INDEX IF NOT EXISTS idx_ventas_numero ON ventas(numero);
        CREATE INDEX IF NOT EXISTS idx_ventas_fecha ON ventas(fecha);
        CREATE INDEX IF NOT EXISTS idx_ventas_cliente ON ventas(cliente_id);
        CREATE INDEX IF NOT EXISTS idx_ventas_estado ON ventas(estado);

        -- Detalle de ventas
        CREATE TABLE IF NOT EXISTS venta_detalles (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            venta_id INTEGER NOT NULL,
            producto_id INTEGER NOT NULL,
            cantidad REAL NOT NULL,
            precio_unitario REAL NOT NULL,
            descuento REAL NOT NULL DEFAULT 0,
            iva_porcentaje REAL NOT NULL DEFAULT 0,
            subtotal REAL NOT NULL,
            FOREIGN KEY (venta_id) REFERENCES ventas(id) ON DELETE CASCADE,
            FOREIGN KEY (producto_id) REFERENCES productos(id)
        );

        CREATE INDEX IF NOT EXISTS idx_venta_detalles_venta ON venta_detalles(venta_id);

        -- Caja (apertura y cierre)
        CREATE TABLE IF NOT EXISTS caja (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            fecha_apertura TEXT NOT NULL DEFAULT (datetime('now', 'localtime')),
            fecha_cierre TEXT,
            monto_inicial REAL NOT NULL DEFAULT 0,
            monto_ventas REAL NOT NULL DEFAULT 0,
            monto_esperado REAL NOT NULL DEFAULT 0,
            monto_real REAL,
            diferencia REAL,
            estado TEXT NOT NULL DEFAULT 'ABIERTA',
            usuario TEXT,
            observacion TEXT
        );

        -- Fiados / Cuentas por cobrar
        CREATE TABLE IF NOT EXISTS cuentas_por_cobrar (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            cliente_id INTEGER NOT NULL,
            venta_id INTEGER NOT NULL,
            monto_total REAL NOT NULL,
            monto_pagado REAL NOT NULL DEFAULT 0,
            saldo REAL NOT NULL,
            estado TEXT NOT NULL DEFAULT 'PENDIENTE',
            fecha_vencimiento TEXT,
            created_at TEXT NOT NULL DEFAULT (datetime('now', 'localtime')),
            updated_at TEXT NOT NULL DEFAULT (datetime('now', 'localtime')),
            FOREIGN KEY (cliente_id) REFERENCES clientes(id),
            FOREIGN KEY (venta_id) REFERENCES ventas(id)
        );

        CREATE INDEX IF NOT EXISTS idx_cuentas_cliente ON cuentas_por_cobrar(cliente_id);
        CREATE INDEX IF NOT EXISTS idx_cuentas_estado ON cuentas_por_cobrar(estado);

        -- Pagos de fiados
        CREATE TABLE IF NOT EXISTS pagos_cuenta (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            cuenta_id INTEGER NOT NULL,
            monto REAL NOT NULL,
            fecha TEXT NOT NULL DEFAULT (datetime('now', 'localtime')),
            observacion TEXT,
            FOREIGN KEY (cuenta_id) REFERENCES cuentas_por_cobrar(id)
        );

        -- Gastos / Egresos
        CREATE TABLE IF NOT EXISTS gastos (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            descripcion TEXT NOT NULL,
            monto REAL NOT NULL,
            categoria TEXT,
            fecha TEXT NOT NULL DEFAULT (datetime('now', 'localtime')),
            caja_id INTEGER,
            observacion TEXT,
            FOREIGN KEY (caja_id) REFERENCES caja(id)
        );

        -- Usuarios / Cajeros
        CREATE TABLE IF NOT EXISTS usuarios (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            nombre TEXT NOT NULL UNIQUE,
            pin_hash TEXT NOT NULL,
            pin_salt TEXT NOT NULL,
            rol TEXT NOT NULL DEFAULT 'CAJERO',
            activo INTEGER NOT NULL DEFAULT 1,
            created_at TEXT NOT NULL DEFAULT (datetime('now', 'localtime'))
        );

        -- Insertar consumidor final por defecto
        INSERT OR IGNORE INTO clientes (id, tipo_identificacion, identificacion, nombre)
        VALUES (1, 'CEDULA', '9999999999999', 'CONSUMIDOR FINAL');

        -- Insertar configuración inicial
        INSERT OR IGNORE INTO config (key, value) VALUES ('nombre_negocio', 'Mi Negocio');
        INSERT OR IGNORE INTO config (key, value) VALUES ('ruc', '');
        INSERT OR IGNORE INTO config (key, value) VALUES ('direccion', '');
        INSERT OR IGNORE INTO config (key, value) VALUES ('telefono', '');
        INSERT OR IGNORE INTO config (key, value) VALUES ('regimen', 'RIMPE_POPULAR');
        INSERT OR IGNORE INTO config (key, value) VALUES ('iva_porcentaje', '15');
        INSERT OR IGNORE INTO config (key, value) VALUES ('moneda', 'USD');
        INSERT OR IGNORE INTO config (key, value) VALUES ('secuencial_nota_venta', '1');
        INSERT OR IGNORE INTO config (key, value) VALUES ('secuencial_factura', '1');
        INSERT OR IGNORE INTO config (key, value) VALUES ('secuencial_factura_pruebas', '1');
        INSERT OR IGNORE INTO config (key, value) VALUES ('secuencial_nota_credito', '1');
        INSERT OR IGNORE INTO config (key, value) VALUES ('secuencial_nota_credito_pruebas', '1');
        INSERT OR IGNORE INTO config (key, value) VALUES ('establecimiento', '001');
        INSERT OR IGNORE INTO config (key, value) VALUES ('punto_emision', '001');
        INSERT OR IGNORE INTO config (key, value) VALUES ('sri_modulo_activo', '0');
        INSERT OR IGNORE INTO config (key, value) VALUES ('sri_facturas_gratis', '30');
        INSERT OR IGNORE INTO config (key, value) VALUES ('sri_facturas_usadas', '0');
        INSERT OR IGNORE INTO config (key, value) VALUES ('timeout_inactividad', '15');
        INSERT OR IGNORE INTO config (key, value) VALUES ('sri_ambiente', 'pruebas');
        INSERT OR IGNORE INTO config (key, value) VALUES ('sri_certificado_cargado', '0');
        INSERT OR IGNORE INTO config (key, value) VALUES ('sri_emision_automatica', '0');

        -- Licencia online (caché local de validación Supabase)
        INSERT OR IGNORE INTO config (key, value) VALUES ('licencia_activada', '0');
        INSERT OR IGNORE INTO config (key, value) VALUES ('licencia_codigo', '');
        INSERT OR IGNORE INTO config (key, value) VALUES ('licencia_negocio', '');
        INSERT OR IGNORE INTO config (key, value) VALUES ('licencia_email', '');
        INSERT OR IGNORE INTO config (key, value) VALUES ('licencia_tipo', '');
        INSERT OR IGNORE INTO config (key, value) VALUES ('licencia_emitida', '');
        INSERT OR IGNORE INTO config (key, value) VALUES ('licencia_machine_id', '');
        INSERT OR IGNORE INTO config (key, value) VALUES ('licencia_ultima_validacion', '');
        INSERT OR IGNORE INTO config (key, value) VALUES ('licencia_api_url', 'https://zakquzflkvfqflqnxpxj.supabase.co/functions/v1');
        INSERT OR IGNORE INTO config (key, value) VALUES ('licencia_api_key', 'eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJpc3MiOiJzdXBhYmFzZSIsInJlZiI6Inpha3F1emZsa3ZmcWZscW54cHhqIiwicm9sZSI6ImFub24iLCJpYXQiOjE3NzA4MDc4NjAsImV4cCI6MjA4NjM4Mzg2MH0.dqdWxSYpyG2fKJt7VR2SjyX5lW__v7BuwQlVrm3ddGg');

        -- SRI Suscripción (caché local de validación online)
        INSERT OR IGNORE INTO config (key, value) VALUES ('sri_suscripcion_url', 'https://zakquzflkvfqflqnxpxj.supabase.co/functions/v1');
        INSERT OR IGNORE INTO config (key, value) VALUES ('sri_suscripcion_autorizado', '0');
        INSERT OR IGNORE INTO config (key, value) VALUES ('sri_suscripcion_plan', '');
        INSERT OR IGNORE INTO config (key, value) VALUES ('sri_suscripcion_hasta', '');
        INSERT OR IGNORE INTO config (key, value) VALUES ('sri_suscripcion_docs_restantes', '');
        INSERT OR IGNORE INTO config (key, value) VALUES ('sri_suscripcion_es_lifetime', '0');
        INSERT OR IGNORE INTO config (key, value) VALUES ('sri_suscripcion_ultima_validacion', '');
        INSERT OR IGNORE INTO config (key, value) VALUES ('sri_suscripcion_mensaje', '');

        -- Email service (configurado desde admin, cacheado localmente)
        INSERT OR IGNORE INTO config (key, value) VALUES ('email_service_url', 'https://email.clouget.com');
        INSERT OR IGNORE INTO config (key, value) VALUES ('email_service_api_key', 'clouget-email-dev-key');

        -- Cola de emails pendientes
        CREATE TABLE IF NOT EXISTS email_log (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            venta_id INTEGER NOT NULL,
            email TEXT NOT NULL,
            estado TEXT NOT NULL DEFAULT 'PENDIENTE',
            intentos INTEGER NOT NULL DEFAULT 0,
            ultimo_error TEXT,
            created_at TEXT NOT NULL DEFAULT (datetime('now','localtime')),
            enviado_at TEXT
        );

        -- Notas de crédito electrónicas
        CREATE TABLE IF NOT EXISTS notas_credito (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            numero TEXT UNIQUE NOT NULL,
            venta_id INTEGER NOT NULL REFERENCES ventas(id),
            cliente_id INTEGER NOT NULL,
            fecha TEXT NOT NULL DEFAULT (datetime('now','localtime')),
            motivo TEXT NOT NULL,
            subtotal_sin_iva REAL NOT NULL DEFAULT 0,
            subtotal_con_iva REAL NOT NULL DEFAULT 0,
            iva REAL NOT NULL DEFAULT 0,
            total REAL NOT NULL DEFAULT 0,
            estado_sri TEXT NOT NULL DEFAULT 'PENDIENTE',
            autorizacion_sri TEXT,
            clave_acceso TEXT,
            xml_firmado TEXT,
            usuario TEXT,
            usuario_id INTEGER,
            created_at TEXT NOT NULL DEFAULT (datetime('now','localtime'))
        );

        CREATE TABLE IF NOT EXISTS nota_credito_detalles (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            nota_credito_id INTEGER NOT NULL REFERENCES notas_credito(id),
            producto_id INTEGER NOT NULL,
            cantidad REAL NOT NULL,
            precio_unitario REAL NOT NULL,
            descuento REAL NOT NULL DEFAULT 0,
            iva_porcentaje REAL NOT NULL DEFAULT 0,
            subtotal REAL NOT NULL
        );

        -- Certificado digital P12 para facturación electrónica (máximo 1)
        CREATE TABLE IF NOT EXISTS sri_certificado (
            id INTEGER PRIMARY KEY CHECK (id = 1),
            p12_data BLOB NOT NULL,
            password TEXT NOT NULL,
            nombre TEXT,
            fecha_carga TEXT NOT NULL DEFAULT (datetime('now', 'localtime')),
            fecha_expiracion TEXT
        );

        -- Listas de precios (tarifas)
        CREATE TABLE IF NOT EXISTS listas_precios (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            nombre TEXT NOT NULL UNIQUE,
            descripcion TEXT,
            es_default INTEGER NOT NULL DEFAULT 0,
            activo INTEGER NOT NULL DEFAULT 1,
            created_at TEXT NOT NULL DEFAULT (datetime('now', 'localtime')),
            updated_at TEXT NOT NULL DEFAULT (datetime('now', 'localtime'))
        );

        -- Precios por producto y lista
        CREATE TABLE IF NOT EXISTS precios_producto (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            lista_precio_id INTEGER NOT NULL,
            producto_id INTEGER NOT NULL,
            precio REAL NOT NULL,
            FOREIGN KEY (lista_precio_id) REFERENCES listas_precios(id) ON DELETE CASCADE,
            FOREIGN KEY (producto_id) REFERENCES productos(id) ON DELETE CASCADE,
            UNIQUE(lista_precio_id, producto_id)
        );
        CREATE INDEX IF NOT EXISTS idx_precios_lista ON precios_producto(lista_precio_id);
        CREATE INDEX IF NOT EXISTS idx_precios_producto ON precios_producto(producto_id);

        -- Kardex: Movimientos de inventario
        CREATE TABLE IF NOT EXISTS movimientos_inventario (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            producto_id INTEGER NOT NULL,
            tipo TEXT NOT NULL,
            cantidad REAL NOT NULL,
            stock_anterior REAL NOT NULL,
            stock_nuevo REAL NOT NULL,
            costo_unitario REAL,
            referencia_id INTEGER,
            motivo TEXT,
            usuario TEXT,
            created_at TEXT NOT NULL DEFAULT (datetime('now','localtime')),
            FOREIGN KEY (producto_id) REFERENCES productos(id)
        );
        CREATE INDEX IF NOT EXISTS idx_mov_inv_producto ON movimientos_inventario(producto_id);
        CREATE INDEX IF NOT EXISTS idx_mov_inv_fecha ON movimientos_inventario(created_at);
        CREATE INDEX IF NOT EXISTS idx_mov_inv_tipo ON movimientos_inventario(tipo);

        -- Establecimientos (sucursales/locales)
        CREATE TABLE IF NOT EXISTS establecimientos (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            codigo TEXT NOT NULL UNIQUE,
            nombre TEXT NOT NULL,
            direccion TEXT,
            telefono TEXT,
            es_propio INTEGER NOT NULL DEFAULT 1,
            activo INTEGER NOT NULL DEFAULT 1,
            created_at TEXT NOT NULL DEFAULT (datetime('now','localtime'))
        );

        -- Puntos de emisión (cajas/terminales dentro de cada establecimiento)
        CREATE TABLE IF NOT EXISTS puntos_emision (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            establecimiento_id INTEGER NOT NULL,
            codigo TEXT NOT NULL,
            nombre TEXT,
            activo INTEGER NOT NULL DEFAULT 1,
            FOREIGN KEY (establecimiento_id) REFERENCES establecimientos(id),
            UNIQUE(establecimiento_id, codigo)
        );

        -- Secuenciales por establecimiento + punto de emisión + tipo de documento
        CREATE TABLE IF NOT EXISTS secuenciales (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            establecimiento_codigo TEXT NOT NULL,
            punto_emision_codigo TEXT NOT NULL,
            tipo_documento TEXT NOT NULL,
            secuencial INTEGER NOT NULL DEFAULT 1,
            UNIQUE(establecimiento_codigo, punto_emision_codigo, tipo_documento)
        );
        ",
    )?;

    // --- Migraciones incrementales ---
    // Agregar columna xml_firmado a ventas (para almacenar XML firmado del SRI)
    let _ = conn.execute("ALTER TABLE ventas ADD COLUMN xml_firmado TEXT", []);
    // Agregar columna estado_sri a ventas (si no existe por migracion anterior)
    let _ = conn.execute("ALTER TABLE ventas ADD COLUMN estado_sri TEXT NOT NULL DEFAULT 'NO_APLICA'", []);
    // Agregar columna fecha_autorizacion a ventas (fecha/hora que el SRI autorizo)
    let _ = conn.execute("ALTER TABLE ventas ADD COLUMN fecha_autorizacion TEXT", []);

    // Config: ticket como PDF (alternativa a impresion directa)
    conn.execute(
        "INSERT OR IGNORE INTO config (key, value) VALUES ('ticket_usar_pdf', '0')",
        [],
    )?;

    // Agregar columna numero_factura (secuencial SRI, solo se asigna al autorizar)
    let _ = conn.execute("ALTER TABLE ventas ADD COLUMN numero_factura TEXT", []);

    // --- Migración: Listas de precios ---
    // Agregar lista_precio_id a clientes
    let _ = conn.execute(
        "ALTER TABLE clientes ADD COLUMN lista_precio_id INTEGER REFERENCES listas_precios(id)",
        [],
    );

    // Seed: crear lista por defecto si no existe ninguna
    let lista_count: i64 = conn
        .query_row("SELECT COUNT(*) FROM listas_precios", [], |row| row.get(0))
        .unwrap_or(0);
    if lista_count == 0 {
        let _ = conn.execute(
            "INSERT INTO listas_precios (nombre, descripcion, es_default) VALUES ('Precio Publico', 'Lista de precios por defecto', 1)",
            [],
        );
        // Copiar precio_venta de productos existentes a la lista por defecto
        let _ = conn.execute(
            "INSERT OR IGNORE INTO precios_producto (lista_precio_id, producto_id, precio)
             SELECT 1, id, precio_venta FROM productos",
            [],
        );
    }

    // --- Migración: Cuentas bancarias + forma de pago en cobros ---
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS cuentas_banco (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            nombre TEXT NOT NULL,
            tipo_cuenta TEXT,
            numero_cuenta TEXT,
            titular TEXT,
            activa INTEGER NOT NULL DEFAULT 1,
            created_at TEXT NOT NULL DEFAULT (datetime('now', 'localtime'))
        );",
    )?;
    let _ = conn.execute("ALTER TABLE pagos_cuenta ADD COLUMN forma_pago TEXT NOT NULL DEFAULT 'EFECTIVO'", []);
    let _ = conn.execute("ALTER TABLE pagos_cuenta ADD COLUMN banco_id INTEGER REFERENCES cuentas_banco(id)", []);
    let _ = conn.execute("ALTER TABLE pagos_cuenta ADD COLUMN numero_comprobante TEXT", []);
    let _ = conn.execute("ALTER TABLE pagos_cuenta ADD COLUMN comprobante_imagen TEXT", []);

    // --- Migración: Imagen de productos ---
    let _ = conn.execute("ALTER TABLE productos ADD COLUMN imagen TEXT", []);

    // --- Migración: Estado de confirmación en pagos_cuenta ---
    let _ = conn.execute("ALTER TABLE pagos_cuenta ADD COLUMN estado TEXT NOT NULL DEFAULT 'CONFIRMADO'", []);
    let _ = conn.execute("ALTER TABLE pagos_cuenta ADD COLUMN confirmado_por INTEGER", []);
    let _ = conn.execute("ALTER TABLE pagos_cuenta ADD COLUMN fecha_confirmacion TEXT", []);

    // --- Migración: Modo demo ---
    conn.execute("INSERT OR IGNORE INTO config (key, value) VALUES ('demo_activo', '0')", [])?;

    // --- Migración: Aumentar facturas gratis de 10 a 30 ---
    let _ = conn.execute(
        "UPDATE config SET value = '30' WHERE key = 'sri_facturas_gratis' AND value = '10'",
        [],
    );

    // --- Migración: Config de terminal (establecimiento y punto de emisión local) ---
    conn.execute("INSERT OR IGNORE INTO config (key, value) VALUES ('terminal_establecimiento', '001')", [])?;
    conn.execute("INSERT OR IGNORE INTO config (key, value) VALUES ('terminal_punto_emision', '001')", [])?;
    conn.execute("INSERT OR IGNORE INTO config (key, value) VALUES ('modo_red', 'local')", [])?;
    conn.execute("INSERT OR IGNORE INTO config (key, value) VALUES ('servidor_puerto', '8847')", [])?;
    conn.execute("INSERT OR IGNORE INTO config (key, value) VALUES ('servidor_token', '')", [])?;
    conn.execute("INSERT OR IGNORE INTO config (key, value) VALUES ('servidor_url', '')", [])?;

    // --- Migración: Poblar establecimientos y puntos_emision desde config existente ---
    // Solo si la tabla está vacía (primera vez)
    let est_count: i64 = conn
        .query_row("SELECT COUNT(*) FROM establecimientos", [], |row| row.get(0))
        .unwrap_or(0);
    if est_count == 0 {
        let nombre_negocio: String = conn
            .query_row("SELECT value FROM config WHERE key = 'nombre_negocio'", [], |row| row.get(0))
            .unwrap_or_else(|_| "Mi Negocio".to_string());
        let direccion: String = conn
            .query_row("SELECT value FROM config WHERE key = 'direccion'", [], |row| row.get(0))
            .unwrap_or_default();
        let telefono: String = conn
            .query_row("SELECT value FROM config WHERE key = 'telefono'", [], |row| row.get(0))
            .unwrap_or_default();
        let est_codigo: String = conn
            .query_row("SELECT value FROM config WHERE key = 'establecimiento'", [], |row| row.get(0))
            .unwrap_or_else(|_| "001".to_string());
        let pe_codigo: String = conn
            .query_row("SELECT value FROM config WHERE key = 'punto_emision'", [], |row| row.get(0))
            .unwrap_or_else(|_| "001".to_string());

        conn.execute(
            "INSERT INTO establecimientos (codigo, nombre, direccion, telefono) VALUES (?1, ?2, ?3, ?4)",
            rusqlite::params![est_codigo, nombre_negocio, direccion, telefono],
        ).ok();

        let est_id: i64 = conn.last_insert_rowid();
        conn.execute(
            "INSERT INTO puntos_emision (establecimiento_id, codigo, nombre) VALUES (?1, ?2, 'Caja Principal')",
            rusqlite::params![est_id, pe_codigo],
        ).ok();

        // Migrar secuenciales desde config a tabla secuenciales
        let tipos_config = [
            ("NOTA_VENTA", "secuencial_nota_venta"),
            ("FACTURA", "secuencial_factura"),
            ("FACTURA_PRUEBAS", "secuencial_factura_pruebas"),
            ("NOTA_CREDITO", "secuencial_nota_credito"),
            ("NOTA_CREDITO_PRUEBAS", "secuencial_nota_credito_pruebas"),
        ];
        for (tipo, config_key) in &tipos_config {
            let sec: i64 = conn
                .query_row(
                    "SELECT CAST(value AS INTEGER) FROM config WHERE key = ?1",
                    rusqlite::params![config_key],
                    |row| row.get(0),
                )
                .unwrap_or(1);
            conn.execute(
                "INSERT OR IGNORE INTO secuenciales (establecimiento_codigo, punto_emision_codigo, tipo_documento, secuencial) VALUES (?1, ?2, ?3, ?4)",
                rusqlite::params![est_codigo, pe_codigo, tipo, sec],
            ).ok();
        }

        // Actualizar config de terminal con los valores existentes
        conn.execute(
            "UPDATE config SET value = ?1 WHERE key = 'terminal_establecimiento'",
            rusqlite::params![est_codigo],
        ).ok();
        conn.execute(
            "UPDATE config SET value = ?1 WHERE key = 'terminal_punto_emision'",
            rusqlite::params![pe_codigo],
        ).ok();
    }

    // --- Migración: columnas establecimiento y punto_emision en ventas y notas_credito ---
    let _ = conn.execute("ALTER TABLE ventas ADD COLUMN establecimiento TEXT NOT NULL DEFAULT '001'", []);
    let _ = conn.execute("ALTER TABLE ventas ADD COLUMN punto_emision TEXT NOT NULL DEFAULT '001'", []);
    let _ = conn.execute("ALTER TABLE notas_credito ADD COLUMN establecimiento TEXT NOT NULL DEFAULT '001'", []);
    let _ = conn.execute("ALTER TABLE notas_credito ADD COLUMN punto_emision TEXT NOT NULL DEFAULT '001'", []);

    // --- Migración: Multi-almacén ---
    conn.execute("INSERT OR IGNORE INTO config (key, value) VALUES ('multi_almacen_activo', '0')", [])?;
    conn.execute("INSERT OR IGNORE INTO config (key, value) VALUES ('venta_cross_store', '0')", [])?;

    // --- Migración: Backup Cloud ---
    conn.execute("INSERT OR IGNORE INTO config (key, value) VALUES ('backup_cloud_activo', '0')", [])?;
    conn.execute("INSERT OR IGNORE INTO config (key, value) VALUES ('backup_cloud_tipo', '')", [])?;
    conn.execute("INSERT OR IGNORE INTO config (key, value) VALUES ('backup_cloud_frecuencia', '6')", [])?;
    conn.execute("INSERT OR IGNORE INTO config (key, value) VALUES ('backup_cloud_ultima', '')", [])?;
    conn.execute("INSERT OR IGNORE INTO config (key, value) VALUES ('gdrive_access_token', '')", [])?;
    conn.execute("INSERT OR IGNORE INTO config (key, value) VALUES ('gdrive_refresh_token', '')", [])?;
    conn.execute("INSERT OR IGNORE INTO config (key, value) VALUES ('gdrive_folder_id', '')", [])?;
    conn.execute("INSERT OR IGNORE INTO config (key, value) VALUES ('gdrive_client_id', '419804426556-ple84m5nr8473fs32f9ma2a12gl2vcdl.apps.googleusercontent.com')", [])?;

    // Tabla stock por establecimiento
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS stock_establecimiento (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            producto_id INTEGER NOT NULL,
            establecimiento_id INTEGER NOT NULL,
            stock_actual REAL NOT NULL DEFAULT 0,
            stock_minimo REAL NOT NULL DEFAULT 0,
            FOREIGN KEY (producto_id) REFERENCES productos(id),
            FOREIGN KEY (establecimiento_id) REFERENCES establecimientos(id),
            UNIQUE(producto_id, establecimiento_id)
        );
        CREATE INDEX IF NOT EXISTS idx_stock_est_producto ON stock_establecimiento(producto_id);
        CREATE INDEX IF NOT EXISTS idx_stock_est_establecimiento ON stock_establecimiento(establecimiento_id);

        -- Transferencias de stock entre establecimientos
        CREATE TABLE IF NOT EXISTS transferencias_stock (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            producto_id INTEGER NOT NULL,
            origen_establecimiento_id INTEGER NOT NULL,
            destino_establecimiento_id INTEGER NOT NULL,
            cantidad REAL NOT NULL,
            estado TEXT NOT NULL DEFAULT 'PENDIENTE',
            usuario TEXT,
            created_at TEXT NOT NULL DEFAULT (datetime('now','localtime')),
            recibida_at TEXT,
            FOREIGN KEY (producto_id) REFERENCES productos(id),
            FOREIGN KEY (origen_establecimiento_id) REFERENCES establecimientos(id),
            FOREIGN KEY (destino_establecimiento_id) REFERENCES establecimientos(id)
        );",
    )?;

    // Columna establecimiento_origen en venta_detalles (para ventas cross-store)
    let _ = conn.execute("ALTER TABLE venta_detalles ADD COLUMN establecimiento_origen_id INTEGER", []);

    // Columna info_adicional en venta_detalles (número de serie, lote, observaciones por item)
    let _ = conn.execute("ALTER TABLE venta_detalles ADD COLUMN info_adicional TEXT", []);

    // Columna precio_costo en venta_detalles (snapshot del costo al momento de la venta)
    let _ = conn.execute("ALTER TABLE venta_detalles ADD COLUMN precio_costo REAL NOT NULL DEFAULT 0", []);

    // Columnas de transferencia bancaria en ventas
    let _ = conn.execute("ALTER TABLE ventas ADD COLUMN banco_id INTEGER", []);
    let _ = conn.execute("ALTER TABLE ventas ADD COLUMN referencia_pago TEXT", []);
    let _ = conn.execute("ALTER TABLE ventas ADD COLUMN comprobante_imagen TEXT", []);

    // Config: reglas de comprobante para cajero
    let _ = conn.execute("INSERT OR IGNORE INTO config (key, value) VALUES ('transferencia_requiere_referencia', '0')", []);
    let _ = conn.execute("INSERT OR IGNORE INTO config (key, value) VALUES ('transferencia_requiere_comprobante', '0')", []);

    // Config: por defecto el precio de venta de un producto incluye IVA (true por default)
    let _ = conn.execute("INSERT OR IGNORE INTO config (key, value) VALUES ('producto_incluye_iva_default', '1')", []);

    // Config: auto-imprimir ticket despues de cada venta (ON por defecto para flujo mas rapido)
    let _ = conn.execute("INSERT OR IGNORE INTO config (key, value) VALUES ('auto_imprimir', '1')", []);
    // Config: auto-imprimir ticket al autorizar en SRI (OFF por defecto, evita imprimir 2 veces)
    let _ = conn.execute("INSERT OR IGNORE INTO config (key, value) VALUES ('auto_imprimir_sri', '0')", []);

    // Config: canal de actualizaciones (stable | beta). Default stable.
    // Los testers reciben betas primero antes de liberar a stable.
    let _ = conn.execute("INSERT OR IGNORE INTO config (key, value) VALUES ('update_canal', 'stable')", []);

    // Columna establecimiento_id en movimientos_inventario
    let _ = conn.execute("ALTER TABLE movimientos_inventario ADD COLUMN establecimiento_id INTEGER", []);

    // --- Migración: Permisos por usuario (JSON) ---
    let _ = conn.execute("ALTER TABLE usuarios ADD COLUMN permisos TEXT NOT NULL DEFAULT '{}'", []);

    // Migración: tipo_estado en ventas (COMPLETADA, BORRADOR, COTIZACION, CONVERTIDA)
    let _ = conn.execute("ALTER TABLE ventas ADD COLUMN tipo_estado TEXT NOT NULL DEFAULT 'COMPLETADA'", []);
    let _ = conn.execute("CREATE INDEX IF NOT EXISTS idx_ventas_tipo_estado ON ventas(tipo_estado)", []);

    // Guías de Remisión
    let _ = conn.execute("ALTER TABLE ventas ADD COLUMN guia_origen_id INTEGER", []);
    let _ = conn.execute("ALTER TABLE ventas ADD COLUMN guia_placa TEXT", []);
    let _ = conn.execute("ALTER TABLE ventas ADD COLUMN guia_chofer TEXT", []);
    let _ = conn.execute("ALTER TABLE ventas ADD COLUMN guia_direccion_destino TEXT", []);

    // Tabla de choferes/transportistas (autocompletar)
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS choferes (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            nombre TEXT NOT NULL,
            placa TEXT,
            created_at TEXT DEFAULT (datetime('now','localtime')),
            UNIQUE(nombre)
        );"
    ).ok();

    // --- Migración: Proveedores, Compras y Cuentas por Pagar ---
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS proveedores (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            ruc TEXT,
            nombre TEXT NOT NULL,
            direccion TEXT,
            telefono TEXT,
            email TEXT,
            contacto TEXT,
            dias_credito INTEGER NOT NULL DEFAULT 0,
            activo INTEGER NOT NULL DEFAULT 1,
            created_at TEXT NOT NULL DEFAULT (datetime('now','localtime'))
        );

        CREATE TABLE IF NOT EXISTS compras (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            numero TEXT NOT NULL,
            proveedor_id INTEGER NOT NULL,
            fecha TEXT NOT NULL DEFAULT (datetime('now','localtime')),
            numero_factura TEXT,
            subtotal REAL NOT NULL DEFAULT 0,
            iva REAL NOT NULL DEFAULT 0,
            total REAL NOT NULL DEFAULT 0,
            estado TEXT NOT NULL DEFAULT 'REGISTRADA',
            forma_pago TEXT NOT NULL DEFAULT 'EFECTIVO',
            es_credito INTEGER NOT NULL DEFAULT 0,
            observacion TEXT,
            created_at TEXT NOT NULL DEFAULT (datetime('now','localtime')),
            FOREIGN KEY (proveedor_id) REFERENCES proveedores(id)
        );

        CREATE TABLE IF NOT EXISTS compra_detalles (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            compra_id INTEGER NOT NULL,
            producto_id INTEGER,
            descripcion TEXT,
            cantidad REAL NOT NULL DEFAULT 1,
            precio_unitario REAL NOT NULL DEFAULT 0,
            subtotal REAL NOT NULL DEFAULT 0,
            FOREIGN KEY (compra_id) REFERENCES compras(id) ON DELETE CASCADE,
            FOREIGN KEY (producto_id) REFERENCES productos(id)
        );

        CREATE TABLE IF NOT EXISTS cuentas_por_pagar (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            proveedor_id INTEGER NOT NULL,
            compra_id INTEGER,
            monto_total REAL NOT NULL,
            monto_pagado REAL NOT NULL DEFAULT 0,
            saldo REAL NOT NULL,
            estado TEXT NOT NULL DEFAULT 'PENDIENTE',
            fecha_vencimiento TEXT,
            observacion TEXT,
            created_at TEXT NOT NULL DEFAULT (datetime('now','localtime')),
            FOREIGN KEY (proveedor_id) REFERENCES proveedores(id),
            FOREIGN KEY (compra_id) REFERENCES compras(id)
        );

        CREATE TABLE IF NOT EXISTS pagos_proveedor (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            cuenta_id INTEGER NOT NULL,
            monto REAL NOT NULL,
            fecha TEXT NOT NULL DEFAULT (datetime('now','localtime')),
            forma_pago TEXT NOT NULL DEFAULT 'EFECTIVO',
            numero_comprobante TEXT,
            observacion TEXT,
            created_at TEXT NOT NULL DEFAULT (datetime('now','localtime')),
            FOREIGN KEY (cuenta_id) REFERENCES cuentas_por_pagar(id)
        );",
    )?;

    // --- Migración: Retiros de caja ---
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS retiros_caja (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            caja_id INTEGER NOT NULL,
            monto REAL NOT NULL,
            motivo TEXT NOT NULL DEFAULT '',
            banco_id INTEGER,
            referencia TEXT,
            usuario TEXT NOT NULL,
            usuario_id INTEGER,
            fecha TEXT NOT NULL DEFAULT (datetime('now','localtime')),
            FOREIGN KEY (caja_id) REFERENCES caja(id)
        );",
    )?;

    // --- Migración: Estado de depósito en retiros_caja ---
    let _ = conn.execute("ALTER TABLE retiros_caja ADD COLUMN estado TEXT NOT NULL DEFAULT 'SIN_DEPOSITO'", []);
    let _ = conn.execute("ALTER TABLE retiros_caja ADD COLUMN comprobante_imagen TEXT", []);

    // --- Migración: banco_id en pagos_proveedor ---
    let _ = conn.execute("ALTER TABLE pagos_proveedor ADD COLUMN banco_id INTEGER", []);

    // --- Migración: Columnas de contraseña en usuarios ---
    let _ = conn.execute("ALTER TABLE usuarios ADD COLUMN password_hash TEXT", []);
    let _ = conn.execute("ALTER TABLE usuarios ADD COLUMN password_salt TEXT", []);

    // Config: modo de login (pin, password, ambos)
    conn.execute("INSERT OR IGNORE INTO config (key, value) VALUES ('modo_login', 'pin')", [])?;

    // Secuencial para compras
    conn.execute("INSERT OR IGNORE INTO config (key, value) VALUES ('secuencial_compra', '1')", [])?;

    // --- Migración: Tipos de unidad ---
    let _ = conn.execute_batch("
        CREATE TABLE IF NOT EXISTS tipos_unidad (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            nombre TEXT NOT NULL UNIQUE,
            abreviatura TEXT NOT NULL
        );
        INSERT OR IGNORE INTO tipos_unidad (nombre, abreviatura) VALUES ('Unidad', 'UND');
        INSERT OR IGNORE INTO tipos_unidad (nombre, abreviatura) VALUES ('Kilogramo', 'KG');
        INSERT OR IGNORE INTO tipos_unidad (nombre, abreviatura) VALUES ('Libra', 'LB');
        INSERT OR IGNORE INTO tipos_unidad (nombre, abreviatura) VALUES ('Litro', 'LT');
        INSERT OR IGNORE INTO tipos_unidad (nombre, abreviatura) VALUES ('Metro', 'MT');
        INSERT OR IGNORE INTO tipos_unidad (nombre, abreviatura) VALUES ('Caja', 'CJ');
    ");

    // Migracion: columnas factor_default y es_agrupada en tipos_unidad (multi-unidad v1.9.8)
    let _ = conn.execute("ALTER TABLE tipos_unidad ADD COLUMN factor_default REAL NOT NULL DEFAULT 1", []);
    let _ = conn.execute("ALTER TABLE tipos_unidad ADD COLUMN es_agrupada INTEGER NOT NULL DEFAULT 0", []);

    // Semilla de unidades agrupadas comunes (para reventa: bebidas, farmacia, abarrotes)
    let _ = conn.execute_batch("
        INSERT OR IGNORE INTO tipos_unidad (nombre, abreviatura, factor_default, es_agrupada) VALUES ('Sixpack', '6PK', 6, 1);
        INSERT OR IGNORE INTO tipos_unidad (nombre, abreviatura, factor_default, es_agrupada) VALUES ('Doce Pack', '12PK', 12, 1);
        INSERT OR IGNORE INTO tipos_unidad (nombre, abreviatura, factor_default, es_agrupada) VALUES ('Jaba', 'JAB', 12, 1);
        INSERT OR IGNORE INTO tipos_unidad (nombre, abreviatura, factor_default, es_agrupada) VALUES ('Blister', 'BLI', 10, 1);
        INSERT OR IGNORE INTO tipos_unidad (nombre, abreviatura, factor_default, es_agrupada) VALUES ('Paquete', 'PAQ', 6, 1);
        INSERT OR IGNORE INTO tipos_unidad (nombre, abreviatura, factor_default, es_agrupada) VALUES ('Caja (24 und)', 'CJ24', 24, 1);
        INSERT OR IGNORE INTO tipos_unidad (nombre, abreviatura, factor_default, es_agrupada) VALUES ('Docena', 'DOC', 12, 1);
        INSERT OR IGNORE INTO tipos_unidad (nombre, abreviatura, factor_default, es_agrupada) VALUES ('Media docena', 'MDOC', 6, 1);
    ");

    // Migrar stock existente a stock_establecimiento (solo una vez)
    let stock_est_count: i64 = conn
        .query_row("SELECT COUNT(*) FROM stock_establecimiento", [], |row| row.get(0))
        .unwrap_or(0);
    if stock_est_count == 0 {
        // Obtener el primer establecimiento
        let primer_est_id: Option<i64> = conn
            .query_row("SELECT id FROM establecimientos ORDER BY id LIMIT 1", [], |row| row.get(0))
            .ok();

        if let Some(est_id) = primer_est_id {
            conn.execute(
                "INSERT INTO stock_establecimiento (producto_id, establecimiento_id, stock_actual, stock_minimo)
                 SELECT id, ?1, stock_actual, stock_minimo FROM productos WHERE es_servicio = 0",
                rusqlite::params![est_id],
            ).ok();
        }
    }

    // --- Migración: Números de serie ---
    let _ = conn.execute("ALTER TABLE productos ADD COLUMN requiere_serie INTEGER NOT NULL DEFAULT 0", []);

    let _ = conn.execute_batch("
        CREATE TABLE IF NOT EXISTS numeros_serie (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            producto_id INTEGER NOT NULL,
            serial TEXT NOT NULL,
            estado TEXT NOT NULL DEFAULT 'DISPONIBLE',
            compra_id INTEGER,
            venta_id INTEGER,
            venta_detalle_id INTEGER,
            cliente_id INTEGER,
            cliente_nombre TEXT,
            fecha_ingreso TEXT NOT NULL DEFAULT (datetime('now','localtime')),
            fecha_venta TEXT,
            observacion TEXT,
            FOREIGN KEY (producto_id) REFERENCES productos(id),
            UNIQUE(producto_id, serial)
        );
    ");

    // --- Migración: Caducidad / Lotes ---
    let _ = conn.execute("ALTER TABLE productos ADD COLUMN requiere_caducidad INTEGER NOT NULL DEFAULT 0", []);

    // --- Migración: no_controla_stock (productos a granel, digitales) ---
    let _ = conn.execute("ALTER TABLE productos ADD COLUMN no_controla_stock INTEGER NOT NULL DEFAULT 0", []);

    // --- Migración: gastos recurrentes ---
    let _ = conn.execute("ALTER TABLE gastos ADD COLUMN es_recurrente INTEGER NOT NULL DEFAULT 0", []);

    let _ = conn.execute_batch("
        CREATE TABLE IF NOT EXISTS lotes_caducidad (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            producto_id INTEGER NOT NULL,
            lote TEXT,
            fecha_caducidad TEXT NOT NULL,
            cantidad REAL NOT NULL,
            cantidad_inicial REAL NOT NULL,
            compra_id INTEGER,
            observacion TEXT,
            fecha_ingreso TEXT NOT NULL DEFAULT (datetime('now','localtime')),
            FOREIGN KEY (producto_id) REFERENCES productos(id)
        );
        CREATE INDEX IF NOT EXISTS idx_lotes_producto ON lotes_caducidad(producto_id);
        CREATE INDEX IF NOT EXISTS idx_lotes_caducidad ON lotes_caducidad(fecha_caducidad);
    ");

    let _ = conn.execute("INSERT OR IGNORE INTO config (key, value) VALUES ('modulo_caducidad', '0')", []);
    let _ = conn.execute("INSERT OR IGNORE INTO config (key, value) VALUES ('caducidad_dias_alerta', '7')", []);

    // Migracion: agregar fecha_elaboracion (fecha de expedicion/fabricacion) a lotes
    let _ = conn.execute("ALTER TABLE lotes_caducidad ADD COLUMN fecha_elaboracion TEXT", []);

    // Migracion: lote_id en venta_detalles (FIFO/FEFO - v2.2.0)
    // Permite saber de que lote especifico se vendio cada item
    let _ = conn.execute("ALTER TABLE venta_detalles ADD COLUMN lote_id INTEGER", []);
    let _ = conn.execute("CREATE INDEX IF NOT EXISTS idx_venta_detalles_lote ON venta_detalles(lote_id)", []);

    // Migracion: banco_id en compras (para pagos DEBITO/TRANSFERENCIA/CHEQUE)
    let _ = conn.execute("ALTER TABLE compras ADD COLUMN banco_id INTEGER", []);
    let _ = conn.execute("ALTER TABLE compras ADD COLUMN referencia_pago TEXT", []);

    // Sesion persistente entre reinicios de app (v2.3.8)
    // Default: NO persistir — el usuario debe loguearse cada vez que abre la app.
    // Si se activa, la sesion se restaura desde config al iniciar.
    let _ = conn.execute("INSERT OR IGNORE INTO config (key, value) VALUES ('sesion_persistente', '0')", []);

    // === CAJA ANTI-FRAUDE FASE 1 (v2.3.1) ===
    // Motivo de la diferencia entre apertura y cierre anterior (si aplica)
    let _ = conn.execute("ALTER TABLE caja ADD COLUMN motivo_diferencia_apertura TEXT", []);
    // Motivo del descuadre al cerrar (cuando monto_real != monto_esperado)
    let _ = conn.execute("ALTER TABLE caja ADD COLUMN motivo_descuadre TEXT", []);
    // Timestamp inmutable del cierre (separado de fecha_cierre por compatibilidad)
    let _ = conn.execute("ALTER TABLE caja ADD COLUMN cerrada_at TEXT", []);
    // Cierre anterior referenciado al abrir (para trazabilidad)
    let _ = conn.execute("ALTER TABLE caja ADD COLUMN caja_anterior_id INTEGER", []);
    // Desglose por denominacion (JSON) — opcional
    let _ = conn.execute("ALTER TABLE caja ADD COLUMN desglose_apertura TEXT", []);
    let _ = conn.execute("ALTER TABLE caja ADD COLUMN desglose_cierre TEXT", []);
    // Usuario que cerro (puede diferir del que abrio)
    let _ = conn.execute("ALTER TABLE caja ADD COLUMN usuario_cierre TEXT", []);

    // Tabla de eventos de caja (audit log inmutable)
    let _ = conn.execute_batch("
        CREATE TABLE IF NOT EXISTS caja_eventos (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            caja_id INTEGER NOT NULL,
            evento TEXT NOT NULL,           -- APERTURA | CIERRE | EDICION_INTENTADA | RETIRO | DEPOSITO
            usuario TEXT,
            usuario_id INTEGER,
            valor_anterior TEXT,            -- JSON con snapshot anterior (puede ser null)
            valor_nuevo TEXT,               -- JSON con snapshot nuevo
            motivo TEXT,
            metadatos TEXT,                 -- JSON adicional (ip, terminal, etc)
            timestamp TEXT NOT NULL DEFAULT (datetime('now', 'localtime')),
            FOREIGN KEY (caja_id) REFERENCES caja(id)
        );
        CREATE INDEX IF NOT EXISTS idx_caja_eventos_caja ON caja_eventos(caja_id);
        CREATE INDEX IF NOT EXISTS idx_caja_eventos_timestamp ON caja_eventos(timestamp);
    ");

    // Config: umbral de descuadre que requiere aprobacion (porcentaje del monto esperado)
    let _ = conn.execute("INSERT OR IGNORE INTO config (key, value) VALUES ('caja_descuadre_umbral_pct', '2')", []);
    // Config: si requiere PIN admin para cerrar con descuadre > umbral
    let _ = conn.execute("INSERT OR IGNORE INTO config (key, value) VALUES ('caja_requiere_pin_descuadre', '0')", []);

    // Control de stock negativo:
    //   PERMITIR (default): puede vender aunque deje stock < 0 (comportamiento historico)
    //   BLOQUEAR: no permite agregar al carrito ni vender si no alcanza stock
    //   BLOQUEAR_OCULTAR: igual a BLOQUEAR + oculta del grid POS productos con stock <= 0
    let _ = conn.execute("INSERT OR IGNORE INTO config (key, value) VALUES ('stock_negativo_modo', 'PERMITIR')", []);

    // Migracion: COMBOS / KITS - productos compuestos por otros productos
    // tipo_producto: 'SIMPLE' (default) | 'COMBO_FIJO' | 'COMBO_FLEXIBLE'
    let _ = conn.execute("ALTER TABLE productos ADD COLUMN tipo_producto TEXT NOT NULL DEFAULT 'SIMPLE'", []);
    // Grupos de componentes (solo COMBO_FLEXIBLE):
    //   Ejemplo "Combo Almuerzo": grupo "Plato" (escoger 1 de N), grupo "Bebida" (escoger 1 de N)
    let _ = conn.execute_batch("
        CREATE TABLE IF NOT EXISTS producto_componente_grupos (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            producto_padre_id INTEGER NOT NULL,
            nombre TEXT NOT NULL,
            minimo INTEGER NOT NULL DEFAULT 1,
            maximo INTEGER NOT NULL DEFAULT 1,
            orden INTEGER NOT NULL DEFAULT 0,
            FOREIGN KEY (producto_padre_id) REFERENCES productos(id) ON DELETE CASCADE
        );
        CREATE INDEX IF NOT EXISTS idx_combo_grupos_padre ON producto_componente_grupos(producto_padre_id);

        CREATE TABLE IF NOT EXISTS producto_componentes (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            producto_padre_id INTEGER NOT NULL,
            producto_hijo_id INTEGER NOT NULL,
            cantidad REAL NOT NULL DEFAULT 1,
            grupo_id INTEGER,
            orden INTEGER NOT NULL DEFAULT 0,
            FOREIGN KEY (producto_padre_id) REFERENCES productos(id) ON DELETE CASCADE,
            FOREIGN KEY (producto_hijo_id) REFERENCES productos(id),
            FOREIGN KEY (grupo_id) REFERENCES producto_componente_grupos(id) ON DELETE SET NULL,
            UNIQUE(producto_padre_id, producto_hijo_id, grupo_id)
        );
        CREATE INDEX IF NOT EXISTS idx_combo_comp_padre ON producto_componentes(producto_padre_id);
        CREATE INDEX IF NOT EXISTS idx_combo_comp_hijo ON producto_componentes(producto_hijo_id);
    ");

    // Selecciones de combo flexible en cada venta (que escogio el cajero)
    let _ = conn.execute_batch("
        CREATE TABLE IF NOT EXISTS venta_detalle_combo (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            venta_detalle_id INTEGER NOT NULL,    -- linea del combo padre en venta_detalles
            producto_hijo_id INTEGER NOT NULL,    -- el componente que se entrego
            cantidad REAL NOT NULL,
            grupo_id INTEGER,
            FOREIGN KEY (venta_detalle_id) REFERENCES venta_detalles(id) ON DELETE CASCADE,
            FOREIGN KEY (producto_hijo_id) REFERENCES productos(id)
        );
        CREATE INDEX IF NOT EXISTS idx_vd_combo_detalle ON venta_detalle_combo(venta_detalle_id);
    ");

    // Módulo Servicio Técnico
    let _ = conn.execute("INSERT OR IGNORE INTO config (key, value) VALUES ('modulo_servicio_tecnico', '0')", []);
    // Tipo de taller: MIXTO (default, permite escoger por orden), GENERAL, TECNOLOGIA, AUTOMOTRIZ, ELECTRODOMESTICO
    let _ = conn.execute("INSERT OR IGNORE INTO config (key, value) VALUES ('tipo_taller', 'MIXTO')", []);
    let _ = conn.execute_batch("
        CREATE TABLE IF NOT EXISTS ordenes_servicio (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            numero TEXT NOT NULL UNIQUE,
            cliente_id INTEGER,
            cliente_nombre TEXT,
            cliente_telefono TEXT,
            tipo_equipo TEXT NOT NULL DEFAULT 'GENERAL',
            equipo_descripcion TEXT NOT NULL,
            equipo_marca TEXT,
            equipo_modelo TEXT,
            equipo_serie TEXT,
            equipo_placa TEXT,
            equipo_kilometraje INTEGER,
            equipo_kilometraje_proximo INTEGER,
            accesorios TEXT,
            problema_reportado TEXT NOT NULL,
            diagnostico TEXT,
            trabajo_realizado TEXT,
            observaciones TEXT,
            tecnico_id INTEGER,
            tecnico_nombre TEXT,
            estado TEXT NOT NULL DEFAULT 'RECIBIDO',
            fecha_ingreso TEXT NOT NULL DEFAULT (datetime('now','localtime')),
            fecha_promesa TEXT,
            fecha_entrega TEXT,
            presupuesto REAL DEFAULT 0,
            monto_final REAL DEFAULT 0,
            garantia_dias INTEGER DEFAULT 0,
            venta_id INTEGER,
            usuario_creador TEXT
        );
        CREATE INDEX IF NOT EXISTS idx_ordenes_estado ON ordenes_servicio(estado);
        CREATE INDEX IF NOT EXISTS idx_ordenes_cliente ON ordenes_servicio(cliente_id);
        CREATE INDEX IF NOT EXISTS idx_ordenes_serie ON ordenes_servicio(equipo_serie);
        CREATE INDEX IF NOT EXISTS idx_ordenes_placa ON ordenes_servicio(equipo_placa);

        CREATE TABLE IF NOT EXISTS ordenes_servicio_movimientos (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            orden_id INTEGER NOT NULL,
            estado_anterior TEXT,
            estado_nuevo TEXT NOT NULL,
            observacion TEXT,
            usuario TEXT,
            fecha TEXT NOT NULL DEFAULT (datetime('now','localtime')),
            FOREIGN KEY (orden_id) REFERENCES ordenes_servicio(id) ON DELETE CASCADE
        );
    ");

    // Imágenes de órdenes de servicio
    let _ = conn.execute_batch("
        CREATE TABLE IF NOT EXISTS ordenes_servicio_imagenes (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            orden_id INTEGER NOT NULL,
            tipo TEXT NOT NULL DEFAULT 'GENERAL',
            imagen_base64 TEXT NOT NULL,
            descripcion TEXT,
            fecha TEXT NOT NULL DEFAULT (datetime('now','localtime')),
            FOREIGN KEY (orden_id) REFERENCES ordenes_servicio(id) ON DELETE CASCADE
        );
        CREATE INDEX IF NOT EXISTS idx_imagenes_orden ON ordenes_servicio_imagenes(orden_id);
    ");

    // --- Migracion: Pagos multiples por venta (pago mixto) ---
    // Una venta puede tener varios pagos: ej $100 EFECTIVO + $20 TRANSFER + $30 CREDITO
    let _ = conn.execute_batch("
        CREATE TABLE IF NOT EXISTS pagos_venta (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            venta_id INTEGER NOT NULL,
            forma_pago TEXT NOT NULL,
            monto REAL NOT NULL,
            banco_id INTEGER,
            referencia TEXT,
            comprobante_imagen TEXT,
            created_at TEXT NOT NULL DEFAULT (datetime('now','localtime')),
            FOREIGN KEY (venta_id) REFERENCES ventas(id) ON DELETE CASCADE,
            FOREIGN KEY (banco_id) REFERENCES cuentas_banco(id)
        );
        CREATE INDEX IF NOT EXISTS idx_pagos_venta_venta ON pagos_venta(venta_id);
    ");

    // Migracion: agregar comprobante_imagen a pagos_venta si no existe (tablas viejas)
    let _ = conn.execute("ALTER TABLE pagos_venta ADD COLUMN comprobante_imagen TEXT", []);

    // --- Migracion: Presentaciones / unidades multiples por producto ---
    // Un producto puede venderse en varias unidades (UND, SIXPACK=6, JABA=12, CAJA=24)
    // Cada presentacion tiene su factor de conversion a la unidad base y su precio propio.
    // Stock se descuenta como cantidad * factor (en unidades base).
    // Si el producto no tiene presentaciones, se vende como unidad base (factor=1).
    let _ = conn.execute_batch("
        CREATE TABLE IF NOT EXISTS unidades_producto (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            producto_id INTEGER NOT NULL,
            nombre TEXT NOT NULL,
            abreviatura TEXT,
            factor REAL NOT NULL DEFAULT 1,
            precio REAL NOT NULL DEFAULT 0,
            es_base INTEGER NOT NULL DEFAULT 0,
            orden INTEGER NOT NULL DEFAULT 0,
            activa INTEGER NOT NULL DEFAULT 1,
            created_at TEXT NOT NULL DEFAULT (datetime('now','localtime')),
            FOREIGN KEY (producto_id) REFERENCES productos(id) ON DELETE CASCADE
        );
        CREATE INDEX IF NOT EXISTS idx_unidades_producto ON unidades_producto(producto_id);

        -- Precios por unidad y lista (opcional, override del precio base de la unidad)
        CREATE TABLE IF NOT EXISTS precios_unidad_lista (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            unidad_id INTEGER NOT NULL,
            lista_precio_id INTEGER NOT NULL,
            precio REAL NOT NULL,
            FOREIGN KEY (unidad_id) REFERENCES unidades_producto(id) ON DELETE CASCADE,
            FOREIGN KEY (lista_precio_id) REFERENCES listas_precios(id) ON DELETE CASCADE,
            UNIQUE(unidad_id, lista_precio_id)
        );
    ");

    // Columna en venta_detalles para registrar la unidad de venta usada (factor multiplicador)
    let _ = conn.execute("ALTER TABLE venta_detalles ADD COLUMN unidad_id INTEGER", []);
    let _ = conn.execute("ALTER TABLE venta_detalles ADD COLUMN unidad_nombre TEXT", []);
    let _ = conn.execute("ALTER TABLE venta_detalles ADD COLUMN factor_unidad REAL DEFAULT 1", []);

    // unidades_producto: vincular con tipos_unidad maestros (v1.9.8)
    let _ = conn.execute("ALTER TABLE unidades_producto ADD COLUMN tipo_unidad_id INTEGER REFERENCES tipos_unidad(id)", []);

    Ok(())
}
