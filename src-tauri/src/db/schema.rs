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
        -- v2.4.15: producto_id es NULLABLE para soportar servicios manuales
        -- (ej: mano de obra de orden de servicio tecnico). Antes era NOT NULL
        -- y los INSERT con NULL fallaban silenciosamente.
        CREATE TABLE IF NOT EXISTS venta_detalles (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            venta_id INTEGER NOT NULL,
            producto_id INTEGER,
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

    // Verificacion de transferencias (v2.3.33+):
    // pago_estado:
    //   - 'NO_APLICA'   → no es transferencia (efectivo/credito puro)
    //   - 'REGISTRADO'  → cajero la ingreso, pendiente de revision admin
    //   - 'VERIFICADO'  → admin confirmo (visto el comprobante en banco)
    //   - 'RECHAZADO'   → admin marco como invalida (con motivo en obs)
    // verificado_por: usuario_id del admin que verifico
    // fecha_verificacion: timestamp de la verificacion
    // motivo_verificacion: nota libre (especialmente cuando se rechaza)
    let _ = conn.execute("ALTER TABLE ventas ADD COLUMN pago_estado TEXT DEFAULT 'NO_APLICA'", []);
    let _ = conn.execute("ALTER TABLE ventas ADD COLUMN verificado_por INTEGER", []);
    let _ = conn.execute("ALTER TABLE ventas ADD COLUMN fecha_verificacion TEXT", []);
    let _ = conn.execute("ALTER TABLE ventas ADD COLUMN motivo_verificacion TEXT", []);

    // Vincular cada venta con la sesion de caja en la que se hizo (v2.3.34+).
    // Permite mostrar al usuario "esta venta fue de la sesion #42" y filtrar por sesion.
    let _ = conn.execute("ALTER TABLE ventas ADD COLUMN caja_id INTEGER", []);
    // Backfill: para ventas viejas sin caja_id, deducir desde fechas (apertura <= fecha < cierre)
    let _ = conn.execute(
        "UPDATE ventas SET caja_id = (
            SELECT c.id FROM caja c
            WHERE c.fecha_apertura <= ventas.fecha
              AND (c.fecha_cierre IS NULL OR ventas.fecha < c.fecha_cierre)
            ORDER BY c.id DESC LIMIT 1
         )
         WHERE caja_id IS NULL",
        [],
    );
    // Marcar todas las TRANSFER existentes (anteriores a esta migracion) como VERIFICADO
    // para no contaminar el panel admin con historico que no fue revisado.
    let _ = conn.execute(
        "UPDATE ventas SET pago_estado = 'VERIFICADO'
         WHERE pago_estado IS NULL OR pago_estado = ''
         OR (UPPER(forma_pago) IN ('TRANSFER','TRANSFERENCIA') AND pago_estado = 'NO_APLICA')",
        [],
    );

    // v2.5.12 BUG FIX: las ALTER TABLE de pagos_venta movidas mas abajo, despues
    // del CREATE TABLE pagos_venta (~linea 1232). Antes corrian aca y fallaban
    // silenciosamente en instalaciones nuevas porque la tabla aun no existia,
    // dejando pagos_venta SIN la columna pago_estado. El INSERT en cobro mixto
    // fallaba con "table pagos_venta has no column named pago_estado".

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

    // v2.5.67: columnas SRI para guía de remisión electrónica (codDoc 06).
    // estado_sri, clave_acceso, autorizacion_sri y xml_firmado se reutilizan de las
    // columnas compartidas de ventas (mismas que factura). Estas son específicas de la guía:
    let _ = conn.execute("ALTER TABLE ventas ADD COLUMN guia_transportista TEXT", []);
    let _ = conn.execute("ALTER TABLE ventas ADD COLUMN guia_ruc_transportista TEXT", []);
    let _ = conn.execute("ALTER TABLE ventas ADD COLUMN guia_tipo_id_transportista TEXT", []);
    let _ = conn.execute("ALTER TABLE ventas ADD COLUMN guia_dir_partida TEXT", []);
    let _ = conn.execute("ALTER TABLE ventas ADD COLUMN guia_fecha_inicio_transporte TEXT", []);
    let _ = conn.execute("ALTER TABLE ventas ADD COLUMN guia_fecha_fin_transporte TEXT", []);
    let _ = conn.execute("ALTER TABLE ventas ADD COLUMN guia_motivo_traslado TEXT", []);
    let _ = conn.execute("ALTER TABLE ventas ADD COLUMN guia_ruta TEXT", []);
    let _ = conn.execute("ALTER TABLE ventas ADD COLUMN guia_cod_doc_sustento TEXT", []);
    let _ = conn.execute("ALTER TABLE ventas ADD COLUMN guia_num_doc_sustento TEXT", []);
    let _ = conn.execute("ALTER TABLE ventas ADD COLUMN guia_num_aut_sustento TEXT", []);
    let _ = conn.execute("ALTER TABLE ventas ADD COLUMN guia_fecha_emision_sustento TEXT", []);

    // v2.5.68 (Fase C): ciclo de vida logístico de despacho de la nota/guía.
    // Estados: PREPARANDO -> EN_TRANSITO -> ENTREGADO  (+ DEVUELTO / PARCIAL).
    // Es el estado OPERATIVO real del movimiento físico, independiente del estado
    // comercial (venta) y tributario (SRI). NULL = sin despacho gestionado (notas viejas).
    let _ = conn.execute("ALTER TABLE ventas ADD COLUMN despacho_estado TEXT", []);
    let _ = conn.execute("ALTER TABLE ventas ADD COLUMN despacho_fecha_salida TEXT", []);
    let _ = conn.execute("ALTER TABLE ventas ADD COLUMN despacho_fecha_entrega TEXT", []);
    let _ = conn.execute("ALTER TABLE ventas ADD COLUMN despacho_observacion TEXT", []);

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

    // Tabla de vehiculos guardados (autocompletar placas, v2.3.39+).
    // Separada de choferes porque a veces se conoce solo la placa, no el chofer,
    // y un mismo vehiculo puede ser conducido por distintos choferes.
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS vehiculos_transporte (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            placa TEXT NOT NULL UNIQUE,
            descripcion TEXT,
            created_at TEXT DEFAULT (datetime('now','localtime'))
        );
        CREATE INDEX IF NOT EXISTS idx_vehiculos_placa ON vehiculos_transporte(placa);"
    ).ok();

    // v2.5.67: Asociacion aprendida placa <-> chofer <-> transportista.
    // Modela la relacion muchos-a-muchos (una placa puede tener varios choferes y
    // viceversa) con un contador de frecuencia para sugerir el mas usado primero.
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS placa_chofer_asoc (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            placa TEXT NOT NULL,
            chofer TEXT NOT NULL,
            transportista_ruc TEXT,
            transportista_nombre TEXT,
            veces INTEGER NOT NULL DEFAULT 1,
            ultima_vez TEXT DEFAULT (datetime('now','localtime')),
            UNIQUE(placa, chofer)
        );
        CREATE INDEX IF NOT EXISTS idx_pca_placa ON placa_chofer_asoc(placa);
        CREATE INDEX IF NOT EXISTS idx_pca_chofer ON placa_chofer_asoc(chofer);"
    ).ok();

    // Backfill (idempotente via INSERT OR IGNORE + flag): sembrar la asociacion desde
    // las guias historicas que ya tienen placa y chofer.
    {
        let ya: String = conn
            .query_row("SELECT value FROM config WHERE key = 'migracion_v2_5_67_placa_chofer'", [], |r| r.get(0))
            .unwrap_or_default();
        if ya != "1" {
            let _ = conn.execute(
                "INSERT OR IGNORE INTO placa_chofer_asoc (placa, chofer, veces)
                 SELECT UPPER(TRIM(guia_placa)), TRIM(guia_chofer), COUNT(*)
                 FROM ventas
                 WHERE tipo_estado = 'GUIA_REMISION'
                   AND guia_placa IS NOT NULL AND TRIM(guia_placa) <> ''
                   AND guia_chofer IS NOT NULL AND TRIM(guia_chofer) <> ''
                 GROUP BY UPPER(TRIM(guia_placa)), TRIM(guia_chofer)",
                [],
            );
            let _ = conn.execute(
                "INSERT OR REPLACE INTO config (key, value) VALUES ('migracion_v2_5_67_placa_chofer', '1')",
                [],
            );
        }
    }

    // Direcciones de entrega por cliente (autocompletar para guias, v2.3.39+).
    // Un cliente puede tener varias direcciones (casa, oficina, sucursal, etc.).
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS direcciones_cliente (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            cliente_id INTEGER NOT NULL,
            direccion TEXT NOT NULL,
            etiqueta TEXT,
            contacto_nombre TEXT,
            contacto_telefono TEXT,
            referencia TEXT,
            created_at TEXT DEFAULT (datetime('now','localtime')),
            FOREIGN KEY (cliente_id) REFERENCES clientes(id) ON DELETE CASCADE
        );
        CREATE INDEX IF NOT EXISTS idx_dir_cli ON direcciones_cliente(cliente_id);"
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

    // v2.3.46: tipo de movimiento — 'RETIRO' (saca dinero, default) o 'INGRESO' (mete dinero).
    // Permite registrar ingresos manuales (ej: ajustes, devoluciones de gastos erroneos
    // de cajas cerradas, aporte de socio, etc.) sin afectar la integridad del flujo
    // de retiros existentes. La columna existing tabla retiros_caja se reusa para no
    // duplicar logica — solo cambia el signo en el calculo de monto_esperado.
    let _ = conn.execute("ALTER TABLE retiros_caja ADD COLUMN tipo TEXT DEFAULT 'RETIRO'", []);

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
    // v2.3.47: trazabilidad — quien registro el gasto y nombre cacheado
    let _ = conn.execute("ALTER TABLE gastos ADD COLUMN usuario_id INTEGER", []);
    let _ = conn.execute("ALTER TABLE gastos ADD COLUMN usuario_nombre TEXT", []);

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

    // v2.5.22: PMP (Promedio Ponderado Movil) para valuacion de inventario.
    // - precio_costo: ultimo precio de compra (modo "reposicion")
    // - costo_promedio: PMP recalculado en cada compra
    //   formula: (stock_actual * costo_promedio + nueva_cantidad * precio_compra) / (stock_actual + nueva_cantidad)
    // Si la columna ya existe, ALTER silently fails y queda lo que estaba.
    let _ = conn.execute("ALTER TABLE productos ADD COLUMN costo_promedio REAL NOT NULL DEFAULT 0", []);
    // En productos existentes, inicializar costo_promedio = precio_costo (asumimos que ese es el costo de su stock inicial)
    let _ = conn.execute("UPDATE productos SET costo_promedio = precio_costo WHERE costo_promedio = 0 AND precio_costo > 0", []);
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

    // ─── ST-2 (v2.4.9): Catálogo jerárquico tipos→marcas→modelos ──────────
    // Permite estructurar equipos/vehículos para autocompletar en la orden y
    // facilitar búsquedas/historial filtrable. Soft-delete (activo=0) para
    // preservar referencias históricas en órdenes ya creadas.
    let _ = conn.execute_batch("
        CREATE TABLE IF NOT EXISTS st_tipos_equipo (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            nombre TEXT NOT NULL UNIQUE,
            icono TEXT DEFAULT '🔧',
            requiere_placa INTEGER NOT NULL DEFAULT 0,
            requiere_kilometraje INTEGER NOT NULL DEFAULT 0,
            requiere_serie INTEGER NOT NULL DEFAULT 0,
            orden INTEGER NOT NULL DEFAULT 0,
            activo INTEGER NOT NULL DEFAULT 1,
            created_at TEXT NOT NULL DEFAULT (datetime('now','localtime'))
        );

        CREATE TABLE IF NOT EXISTS st_marcas (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            tipo_equipo_id INTEGER NOT NULL,
            nombre TEXT NOT NULL,
            activo INTEGER NOT NULL DEFAULT 1,
            created_at TEXT NOT NULL DEFAULT (datetime('now','localtime')),
            UNIQUE(tipo_equipo_id, nombre),
            FOREIGN KEY (tipo_equipo_id) REFERENCES st_tipos_equipo(id) ON DELETE CASCADE
        );
        CREATE INDEX IF NOT EXISTS idx_st_marcas_tipo ON st_marcas(tipo_equipo_id);

        CREATE TABLE IF NOT EXISTS st_modelos (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            marca_id INTEGER NOT NULL,
            nombre TEXT NOT NULL,
            anio_desde INTEGER,
            anio_hasta INTEGER,
            activo INTEGER NOT NULL DEFAULT 1,
            created_at TEXT NOT NULL DEFAULT (datetime('now','localtime')),
            UNIQUE(marca_id, nombre),
            FOREIGN KEY (marca_id) REFERENCES st_marcas(id) ON DELETE CASCADE
        );
        CREATE INDEX IF NOT EXISTS idx_st_modelos_marca ON st_modelos(marca_id);
    ");

    // Seed inicial de tipos comunes (se inserta solo si la tabla está vacía).
    // El admin puede borrar/agregar libremente desde Configuración → Servicio Técnico.
    let count_tipos: i64 = conn.query_row("SELECT COUNT(*) FROM st_tipos_equipo", [], |r| r.get(0)).unwrap_or(0);
    if count_tipos == 0 {
        let _ = conn.execute_batch("
            INSERT INTO st_tipos_equipo (nombre, icono, requiere_placa, requiere_kilometraje, requiere_serie, orden) VALUES
              ('Vehículo',         '🚗', 1, 1, 0, 1),
              ('Motocicleta',      '🏍️', 1, 1, 0, 2),
              ('Computadora',      '💻', 0, 0, 1, 3),
              ('Celular',          '📱', 0, 0, 1, 4),
              ('Electrodoméstico', '🔌', 0, 0, 1, 5),
              ('General',          '🔧', 0, 0, 0, 99);
        ");
    }

    // Migración v2.4.9: agregar FKs opcionales al catálogo en ordenes_servicio.
    // Si el user elige del catálogo, guardamos los IDs (mejor filtrado/historial).
    // Si escribe libre, los IDs quedan NULL pero los TEXT (equipo_marca, etc) se mantienen.
    let _ = conn.execute("ALTER TABLE ordenes_servicio ADD COLUMN tipo_equipo_id INTEGER REFERENCES st_tipos_equipo(id)", []);
    let _ = conn.execute("ALTER TABLE ordenes_servicio ADD COLUMN marca_id INTEGER REFERENCES st_marcas(id)", []);
    let _ = conn.execute("ALTER TABLE ordenes_servicio ADD COLUMN modelo_id INTEGER REFERENCES st_modelos(id)", []);
    let _ = conn.execute("CREATE INDEX IF NOT EXISTS idx_ordenes_tipo ON ordenes_servicio(tipo_equipo_id)", []);
    let _ = conn.execute("CREATE INDEX IF NOT EXISTS idx_ordenes_marca ON ordenes_servicio(marca_id)", []);
    let _ = conn.execute("CREATE INDEX IF NOT EXISTS idx_ordenes_modelo ON ordenes_servicio(modelo_id)", []);

    // ─── ST-5 (v2.4.13): Abonos en órdenes con holding en caja ────────────
    // Cuando un cliente deja un equipo y paga adelantado (anticipo), ese
    // dinero entra físicamente a caja PERO no es venta cobrada todavía. Se
    // mantiene en estado HOLDING hasta que la orden se cobra (APLICADO) o
    // se cancela (DEVUELTO).
    //
    // Estados:
    //   HOLDING  — anticipo recibido, en caja, no aplicado
    //   APLICADO — la orden se cobró y este abono se descontó del total
    //   DEVUELTO — la orden se canceló y el dinero se devolvió al cliente
    let _ = conn.execute_batch("
        CREATE TABLE IF NOT EXISTS st_abonos (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            orden_id INTEGER NOT NULL,
            monto REAL NOT NULL,
            forma_pago TEXT NOT NULL DEFAULT 'EFECTIVO',
            banco_id INTEGER,
            referencia_pago TEXT,
            caja_id INTEGER,                              -- a qué sesión de caja entró
            estado TEXT NOT NULL DEFAULT 'HOLDING',       -- HOLDING | APLICADO | DEVUELTO
            venta_id_aplicado INTEGER,                    -- venta donde se descontó (NULL hasta APLICADO)
            fecha TEXT NOT NULL DEFAULT (datetime('now', 'localtime')),
            fecha_aplicado TEXT,
            fecha_devuelto TEXT,
            usuario_id INTEGER,
            usuario_nombre TEXT,
            observacion TEXT,
            FOREIGN KEY (orden_id) REFERENCES ordenes_servicio(id) ON DELETE CASCADE,
            FOREIGN KEY (banco_id) REFERENCES cuentas_banco(id),
            FOREIGN KEY (caja_id) REFERENCES caja(id),
            FOREIGN KEY (venta_id_aplicado) REFERENCES ventas(id)
        );
        CREATE INDEX IF NOT EXISTS idx_st_abonos_orden ON st_abonos(orden_id);
        CREATE INDEX IF NOT EXISTS idx_st_abonos_estado ON st_abonos(estado);
        CREATE INDEX IF NOT EXISTS idx_st_abonos_caja ON st_abonos(caja_id);
    ");

    // --- Migracion v2.4.13: items presupuestados/aplicados a la orden de servicio ---
    // Permite armar el detalle (productos del catalogo + servicios manuales) ANTES de cobrar.
    // El total de la orden se calcula desde aqui; los abonos HOLDING no pueden exceder este total.
    // Al cobrar, estos items se copian a venta_detalles.
    //
    // - producto_id puede ser NULL (servicio manual / mano de obra sin codigo).
    // - es_servicio = 1 evita descontar stock al cobrar.
    let _ = conn.execute_batch("
        CREATE TABLE IF NOT EXISTS orden_servicio_items (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            orden_id INTEGER NOT NULL,
            producto_id INTEGER,
            descripcion TEXT NOT NULL,
            cantidad REAL NOT NULL DEFAULT 1,
            precio_unitario REAL NOT NULL DEFAULT 0,
            iva_porcentaje REAL NOT NULL DEFAULT 0,
            subtotal REAL NOT NULL DEFAULT 0,
            es_servicio INTEGER NOT NULL DEFAULT 0,
            created_at TEXT NOT NULL DEFAULT (datetime('now','localtime')),
            FOREIGN KEY (orden_id) REFERENCES ordenes_servicio(id) ON DELETE CASCADE,
            FOREIGN KEY (producto_id) REFERENCES productos(id)
        );
        CREATE INDEX IF NOT EXISTS idx_orden_servicio_items_orden ON orden_servicio_items(orden_id);
    ");

    // --- Migracion v2.4.14: cobranza parcial / saldo pendiente en orden de servicio ---
    // Permite entregar el equipo aunque el cliente haya pagado solo una parte.
    // El estado pasa a ENTREGADO_PARCIAL y queda registrado el saldo pendiente.
    let _ = conn.execute("ALTER TABLE ordenes_servicio ADD COLUMN saldo_pendiente REAL DEFAULT 0", []);

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

    // v2.5.12 BUG FIX: migraciones de verificacion de transferencias para pagos_venta.
    // Antes estaban arriba (~linea 620) ANTES del CREATE TABLE, lo cual causaba que
    // fallaran silenciosamente en instalaciones nuevas y la columna pago_estado nunca
    // se agregara. Movidas aca, despues del CREATE TABLE, garantizando que se ejecuten
    // sobre la tabla recien creada (idempotente: si ya existen, los ALTER fallan silent
    // y no pasa nada).
    let _ = conn.execute("ALTER TABLE pagos_venta ADD COLUMN pago_estado TEXT DEFAULT 'NO_APLICA'", []);
    let _ = conn.execute("ALTER TABLE pagos_venta ADD COLUMN verificado_por INTEGER", []);
    let _ = conn.execute("ALTER TABLE pagos_venta ADD COLUMN fecha_verificacion TEXT", []);
    let _ = conn.execute("ALTER TABLE pagos_venta ADD COLUMN motivo_verificacion TEXT", []);
    let _ = conn.execute(
        "UPDATE pagos_venta SET pago_estado = 'VERIFICADO'
         WHERE UPPER(forma_pago) IN ('TRANSFER','TRANSFERENCIA') AND (pago_estado IS NULL OR pago_estado = 'NO_APLICA')",
        [],
    );

    // --- Migracion v2.5.4: Retenciones recibidas (SRI Ecuador) ---
    // Cuando un cliente (empresa) compra y nos paga, puede retener IVA y/o Renta
    // segun la normativa SRI. Esto reduce el saldo pendiente de la factura.
    // Ej: Factura $1150 → cliente retiene 30% IVA ($45) + 2% Renta ($20) = $65
    // → cliente paga $1085 + nos entrega 2 comprobantes de retencion
    // → registramos las 2 retenciones aqui → saldo pasa a 0 (cancelado)
    //
    // tipo: 'RENTA' o 'IVA'
    // codigo_sri: codigo de la tabla SRI (Renta tabla 304, IVA tabla 21)
    // numero_comprobante: el N° del comprobante de retencion fisico/electronico del cliente
    let _ = conn.execute_batch("
        CREATE TABLE IF NOT EXISTS retenciones_recibidas (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            venta_id INTEGER NOT NULL,
            tipo TEXT NOT NULL,
            codigo_sri TEXT NOT NULL,
            base_imponible REAL NOT NULL,
            porcentaje REAL NOT NULL,
            valor REAL NOT NULL,
            numero_comprobante TEXT NOT NULL,
            fecha_emision TEXT NOT NULL,
            fecha_registro TEXT NOT NULL DEFAULT (datetime('now','localtime')),
            usuario TEXT,
            observacion TEXT,
            FOREIGN KEY (venta_id) REFERENCES ventas(id) ON DELETE CASCADE
        );
        CREATE INDEX IF NOT EXISTS idx_retenciones_venta ON retenciones_recibidas(venta_id);
        CREATE INDEX IF NOT EXISTS idx_retenciones_fecha ON retenciones_recibidas(fecha_registro);
        CREATE INDEX IF NOT EXISTS idx_retenciones_tipo ON retenciones_recibidas(tipo);
    ");

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

    // v2.4.25: kilometraje de salida (al entregar un vehiculo) + intervalo recomendado.
    // - intervalo: cada cuánto km se recomienda mantenimiento (ej: 5000)
    // - salida: km que tiene el vehiculo al ser entregado (post-trabajo)
    // Próximo mantenimiento se calcula = (salida || entrada) + intervalo.
    let _ = conn.execute("ALTER TABLE ordenes_servicio ADD COLUMN equipo_kilometraje_intervalo INTEGER", []);
    let _ = conn.execute("ALTER TABLE ordenes_servicio ADD COLUMN equipo_kilometraje_salida INTEGER", []);

    // ─── v2.4.15: producto_id NULLABLE en venta_detalles ────────────────────
    // Bug grave: en BDs existentes, producto_id era NOT NULL. Cuando una orden
    // de servicio tecnico se cobraba con servicios manuales (mano de obra,
    // diagnostico, etc.) el INSERT tenia producto_id = NULL y FALLABA
    // silenciosamente por el `.ok()` en cobrar_orden_servicio. Resultado:
    // la linea del servicio NUNCA se insertaba — el detalle de venta solo
    // mostraba los productos del catalogo, total no cuadraba con detalle.
    //
    // SQLite no permite cambiar NULL/NOT NULL via ALTER TABLE. Hay que
    // recrear la tabla. Solo lo hacemos si producto_id sigue siendo NOT NULL.
    let producto_id_es_not_null: bool = conn.query_row(
        "SELECT \"notnull\" FROM pragma_table_info('venta_detalles') WHERE name = 'producto_id'",
        [], |r| r.get::<_, i64>(0).map(|n| n != 0),
    ).unwrap_or(false);
    if producto_id_es_not_null {
        let res = conn.execute_batch("
            PRAGMA foreign_keys = OFF;
            BEGIN;
            CREATE TABLE venta_detalles_new (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                venta_id INTEGER NOT NULL,
                producto_id INTEGER,
                cantidad REAL NOT NULL,
                precio_unitario REAL NOT NULL,
                descuento REAL NOT NULL DEFAULT 0,
                iva_porcentaje REAL NOT NULL DEFAULT 0,
                subtotal REAL NOT NULL,
                establecimiento_origen_id INTEGER,
                info_adicional TEXT,
                precio_costo REAL NOT NULL DEFAULT 0,
                lote_id INTEGER,
                unidad_id INTEGER,
                unidad_nombre TEXT,
                factor_unidad REAL DEFAULT 1,
                FOREIGN KEY (venta_id) REFERENCES ventas(id) ON DELETE CASCADE,
                FOREIGN KEY (producto_id) REFERENCES productos(id)
            );
            INSERT INTO venta_detalles_new
                (id, venta_id, producto_id, cantidad, precio_unitario, descuento,
                 iva_porcentaje, subtotal, establecimiento_origen_id, info_adicional,
                 precio_costo, lote_id, unidad_id, unidad_nombre, factor_unidad)
            SELECT id, venta_id, producto_id, cantidad, precio_unitario, descuento,
                   iva_porcentaje, subtotal, establecimiento_origen_id, info_adicional,
                   precio_costo, lote_id, unidad_id, unidad_nombre, factor_unidad
            FROM venta_detalles;
            DROP TABLE venta_detalles;
            ALTER TABLE venta_detalles_new RENAME TO venta_detalles;
            CREATE INDEX IF NOT EXISTS idx_venta_detalles_venta ON venta_detalles(venta_id);
            COMMIT;
            PRAGMA foreign_keys = ON;
        ");
        if let Err(e) = res {
            eprintln!("[migracion v2.4.15] Error migrando venta_detalles a producto_id NULL: {}", e);
        } else {
            eprintln!("[migracion v2.4.15] venta_detalles.producto_id ahora es NULLABLE");
        }
    }

    // ─── v2.5.30: Mejoras al módulo de Compras ───────────────────────────────
    // 1. tipo_documento (FACTURA / NOTA_VENTA / INFORMAL) — distingue tipo SRI
    // 2. estado_sri (AUTORIZADA / NULL) y clave_acceso — para facturas validadas
    // 3. UNIQUE INDEX para evitar duplicados por proveedor+tipo+numero_factura
    // 4. cantidad_devuelta en compra_detalles + tabla compra_devoluciones
    let _ = conn.execute(
        "ALTER TABLE compras ADD COLUMN tipo_documento TEXT NOT NULL DEFAULT 'INFORMAL'",
        [],
    );
    let _ = conn.execute("ALTER TABLE compras ADD COLUMN estado_sri TEXT", []);
    let _ = conn.execute("ALTER TABLE compras ADD COLUMN clave_acceso TEXT", []);
    let _ = conn.execute("ALTER TABLE compras ADD COLUMN fecha_emision TEXT", []);
    let _ = conn.execute("ALTER TABLE compras ADD COLUMN usuario TEXT", []);

    // Migrar compras viejas — si tiene numero_factura → asume FACTURA (sin autorizar),
    // si no → queda como INFORMAL (default). El usuario puede editar después.
    let _ = conn.execute(
        "UPDATE compras SET tipo_documento = 'FACTURA' WHERE tipo_documento = 'INFORMAL' AND numero_factura IS NOT NULL AND TRIM(numero_factura) != ''",
        [],
    );

    // UNIQUE INDEX parcial: solo aplica cuando numero_factura y clave_acceso no son NULL
    // Para clave_acceso es unique global (jamas pueden repetirse — son las 49 digit del SRI)
    let _ = conn.execute(
        "CREATE UNIQUE INDEX IF NOT EXISTS idx_compras_clave_acceso_unique
         ON compras(clave_acceso) WHERE clave_acceso IS NOT NULL AND clave_acceso != ''",
        [],
    );
    // Para numero_factura: unique por proveedor+tipo (un proveedor no puede tener dos facturas con mismo numero)
    let _ = conn.execute(
        "CREATE UNIQUE INDEX IF NOT EXISTS idx_compras_factura_proveedor_unique
         ON compras(proveedor_id, tipo_documento, numero_factura)
         WHERE numero_factura IS NOT NULL AND numero_factura != '' AND estado != 'ANULADA'",
        [],
    );

    // cantidad_devuelta en compra_detalles para tracking de devoluciones parciales
    let _ = conn.execute(
        "ALTER TABLE compra_detalles ADD COLUMN cantidad_devuelta REAL NOT NULL DEFAULT 0",
        [],
    );

    // Tabla de devoluciones de compra (notas de crédito de proveedor)
    let _ = conn.execute_batch("
        CREATE TABLE IF NOT EXISTS compra_devoluciones (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            compra_id INTEGER NOT NULL,
            numero TEXT NOT NULL,
            fecha TEXT NOT NULL DEFAULT (datetime('now','localtime')),
            motivo TEXT,
            subtotal REAL NOT NULL DEFAULT 0,
            iva REAL NOT NULL DEFAULT 0,
            total REAL NOT NULL DEFAULT 0,
            es_total INTEGER NOT NULL DEFAULT 0,
            usuario TEXT,
            observacion TEXT,
            created_at TEXT NOT NULL DEFAULT (datetime('now','localtime')),
            FOREIGN KEY (compra_id) REFERENCES compras(id)
        );
        CREATE TABLE IF NOT EXISTS compra_devolucion_detalles (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            devolucion_id INTEGER NOT NULL,
            compra_detalle_id INTEGER NOT NULL,
            producto_id INTEGER,
            cantidad REAL NOT NULL,
            precio_unitario REAL NOT NULL,
            subtotal REAL NOT NULL,
            FOREIGN KEY (devolucion_id) REFERENCES compra_devoluciones(id) ON DELETE CASCADE,
            FOREIGN KEY (compra_detalle_id) REFERENCES compra_detalles(id),
            FOREIGN KEY (producto_id) REFERENCES productos(id)
        );
        CREATE INDEX IF NOT EXISTS idx_compra_dev_compra ON compra_devoluciones(compra_id);
        CREATE INDEX IF NOT EXISTS idx_compra_dev_det_dev ON compra_devolucion_detalles(devolucion_id);
    ");

    // ─── v2.5.39: Categorías de clientes con defaults preconfigurados ─────────
    // Permite agrupar clientes (ej "Consumidor Final", "Mayorista", "Empresarial")
    // y preconfigurar valores que se asignan automáticamente al crear cliente:
    // dias_credito, limite_credito, descuento_pct, lista_precios por defecto.
    let _ = conn.execute_batch("
        CREATE TABLE IF NOT EXISTS categorias_clientes (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            nombre TEXT NOT NULL UNIQUE,
            descripcion TEXT,
            permite_credito INTEGER NOT NULL DEFAULT 0,
            dias_credito INTEGER NOT NULL DEFAULT 0,
            limite_credito REAL NOT NULL DEFAULT 0,
            descuento_pct REAL NOT NULL DEFAULT 0,
            lista_precio_id INTEGER,
            requiere_ruc INTEGER NOT NULL DEFAULT 0,
            es_default INTEGER NOT NULL DEFAULT 0,
            activo INTEGER NOT NULL DEFAULT 1,
            created_at TEXT NOT NULL DEFAULT (datetime('now','localtime')),
            FOREIGN KEY (lista_precio_id) REFERENCES listas_precios(id)
        );
        CREATE INDEX IF NOT EXISTS idx_cat_cli_nombre ON categorias_clientes(nombre);
    ");

    // Columnas extra en clientes: categoria + valores que overridean el default de su categoria.
    // OJO: lista_precio_id (singular) ya existe en clientes — no la agregamos de nuevo.
    let _ = conn.execute("ALTER TABLE clientes ADD COLUMN categoria_id INTEGER REFERENCES categorias_clientes(id)", []);
    let _ = conn.execute("ALTER TABLE clientes ADD COLUMN permite_credito INTEGER NOT NULL DEFAULT 0", []);
    let _ = conn.execute("ALTER TABLE clientes ADD COLUMN dias_credito INTEGER NOT NULL DEFAULT 0", []);
    let _ = conn.execute("ALTER TABLE clientes ADD COLUMN limite_credito REAL NOT NULL DEFAULT 0", []);
    let _ = conn.execute("ALTER TABLE clientes ADD COLUMN descuento_pct REAL NOT NULL DEFAULT 0", []);

    // Seed categoría default "General" si no hay ninguna (idempotente)
    let _ = conn.execute(
        "INSERT INTO categorias_clientes (nombre, descripcion, es_default, activo, permite_credito, dias_credito, limite_credito, descuento_pct)
         SELECT 'General', 'Categoria por defecto', 1, 1, 0, 0, 0, 0
         WHERE NOT EXISTS (SELECT 1 FROM categorias_clientes WHERE es_default = 1)",
        [],
    );

    // ─── v2.5.43/44: Módulo Contabilidad (Agente de Retención + ATS) ─────────
    // Módulo OPCIONAL — solo accesible si la licencia tiene el flag `contabilidad`
    // activado desde admin.clouget.com. Encierra:
    //   - Configuración separada del agente de retención (resolución, fecha, etc.)
    //   - Retenciones EMITIDAS a proveedores (yo soy el agente que retiene)
    //   - Generación XML SRI + autorización + RIDE del comprobante de retención
    //   - Generador ATS (Anexo Transaccional Simplificado) mensual
    //
    // NO se confunde con `retenciones_recibidas` que ya existe (lo que clientes
    // me retienen a mí). Ambas tablas conviven; cada una se maneja por separado.
    //
    // v2.5.44: rename de `sri_avanzado_config` a `contabilidad_config`. Si la tabla
    // vieja existe (de v2.5.43 BETA), se migran los datos automáticamente.
    let _ = conn.execute_batch("
        CREATE TABLE IF NOT EXISTS contabilidad_config (
            id INTEGER PRIMARY KEY CHECK (id = 1),
            es_agente_retencion INTEGER NOT NULL DEFAULT 0,
            resolucion_designacion TEXT,
            fecha_designacion TEXT,
            tipo_contribuyente TEXT,
            obligado_contabilidad INTEGER NOT NULL DEFAULT 0,
            codigo_retencion_renta_default TEXT,
            codigo_retencion_iva_default TEXT,
            contador_ruc TEXT,
            contador_nombre TEXT,
            observacion TEXT,
            updated_at TEXT NOT NULL DEFAULT (datetime('now','localtime'))
        );
        INSERT OR IGNORE INTO contabilidad_config (id) VALUES (1);

        CREATE TABLE IF NOT EXISTS retenciones_emitidas (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            numero TEXT NOT NULL UNIQUE,
            compra_id INTEGER NOT NULL,
            proveedor_id INTEGER NOT NULL,
            fecha_emision TEXT NOT NULL DEFAULT (datetime('now','localtime')),
            tipo_documento_referencia TEXT NOT NULL DEFAULT '01',
            numero_documento_referencia TEXT,
            fecha_documento_referencia TEXT,
            establecimiento TEXT,
            punto_emision TEXT,
            secuencial TEXT,
            numero_factura TEXT,
            clave_acceso TEXT,
            estado_sri TEXT NOT NULL DEFAULT 'NO_APLICA',
            autorizacion_sri TEXT,
            fecha_autorizacion TEXT,
            xml_firmado TEXT,
            subtotal_renta REAL NOT NULL DEFAULT 0,
            subtotal_iva REAL NOT NULL DEFAULT 0,
            total REAL NOT NULL DEFAULT 0,
            usuario TEXT,
            observacion TEXT,
            anulada INTEGER NOT NULL DEFAULT 0,
            created_at TEXT NOT NULL DEFAULT (datetime('now','localtime')),
            FOREIGN KEY (compra_id) REFERENCES compras(id),
            FOREIGN KEY (proveedor_id) REFERENCES proveedores(id)
        );
        CREATE INDEX IF NOT EXISTS idx_ret_emit_compra ON retenciones_emitidas(compra_id);
        CREATE INDEX IF NOT EXISTS idx_ret_emit_proveedor ON retenciones_emitidas(proveedor_id);
        CREATE INDEX IF NOT EXISTS idx_ret_emit_fecha ON retenciones_emitidas(fecha_emision);
        CREATE UNIQUE INDEX IF NOT EXISTS idx_ret_emit_clave_acceso_unique
            ON retenciones_emitidas(clave_acceso)
            WHERE clave_acceso IS NOT NULL AND clave_acceso != '';

        CREATE TABLE IF NOT EXISTS retencion_emitida_detalles (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            retencion_id INTEGER NOT NULL,
            tipo TEXT NOT NULL,
            codigo_sri TEXT NOT NULL,
            base_imponible REAL NOT NULL,
            porcentaje REAL NOT NULL,
            valor REAL NOT NULL,
            FOREIGN KEY (retencion_id) REFERENCES retenciones_emitidas(id) ON DELETE CASCADE,
            CHECK (tipo IN ('RENTA', 'IVA'))
        );
        CREATE INDEX IF NOT EXISTS idx_ret_emit_det_ret ON retencion_emitida_detalles(retencion_id);
    ");

    // v2.5.69: Liquidaciones de Compra (codDoc 03). La emite el negocio cuando
    // compra a un proveedor que no puede facturar (agricultor, reciclador, etc.).
    let _ = conn.execute_batch("
        CREATE TABLE IF NOT EXISTS liquidaciones_compra (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            numero TEXT,
            proveedor_id INTEGER NOT NULL,
            fecha_emision TEXT NOT NULL DEFAULT (datetime('now','localtime')),
            establecimiento TEXT,
            punto_emision TEXT,
            secuencial TEXT,
            numero_factura TEXT,
            clave_acceso TEXT,
            estado_sri TEXT NOT NULL DEFAULT 'NO_APLICA',
            autorizacion_sri TEXT,
            fecha_autorizacion TEXT,
            xml_firmado TEXT,
            subtotal_sin_impuestos REAL NOT NULL DEFAULT 0,
            total_descuento REAL NOT NULL DEFAULT 0,
            iva REAL NOT NULL DEFAULT 0,
            total REAL NOT NULL DEFAULT 0,
            forma_pago TEXT NOT NULL DEFAULT 'EFECTIVO',
            usuario TEXT,
            observacion TEXT,
            anulada INTEGER NOT NULL DEFAULT 0,
            created_at TEXT NOT NULL DEFAULT (datetime('now','localtime')),
            FOREIGN KEY (proveedor_id) REFERENCES proveedores(id)
        );
        CREATE INDEX IF NOT EXISTS idx_liq_compra_prov ON liquidaciones_compra(proveedor_id);
        CREATE INDEX IF NOT EXISTS idx_liq_compra_fecha ON liquidaciones_compra(fecha_emision);

        CREATE TABLE IF NOT EXISTS liquidacion_compra_detalles (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            liquidacion_id INTEGER NOT NULL,
            codigo TEXT,
            descripcion TEXT NOT NULL,
            cantidad REAL NOT NULL DEFAULT 1,
            precio_unitario REAL NOT NULL DEFAULT 0,
            descuento REAL NOT NULL DEFAULT 0,
            iva_porcentaje REAL NOT NULL DEFAULT 0,
            FOREIGN KEY (liquidacion_id) REFERENCES liquidaciones_compra(id) ON DELETE CASCADE
        );
        CREATE INDEX IF NOT EXISTS idx_liq_compra_det ON liquidacion_compra_detalles(liquidacion_id);
    ");

    // v2.5.69: Notas de Débito (codDoc 05). Cobra un valor adicional (interés,
    // recargo) sobre una factura ya emitida al cliente.
    let _ = conn.execute_batch("
        CREATE TABLE IF NOT EXISTS notas_debito (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            numero TEXT,
            cliente_id INTEGER NOT NULL,
            venta_id INTEGER,
            cod_doc_modificado TEXT NOT NULL DEFAULT '01',
            num_doc_modificado TEXT NOT NULL,
            fecha_doc_modificado TEXT,
            fecha_emision TEXT NOT NULL DEFAULT (datetime('now','localtime')),
            establecimiento TEXT,
            punto_emision TEXT,
            secuencial TEXT,
            numero_factura TEXT,
            clave_acceso TEXT,
            estado_sri TEXT NOT NULL DEFAULT 'NO_APLICA',
            autorizacion_sri TEXT,
            fecha_autorizacion TEXT,
            xml_firmado TEXT,
            total_sin_impuestos REAL NOT NULL DEFAULT 0,
            iva REAL NOT NULL DEFAULT 0,
            valor_total REAL NOT NULL DEFAULT 0,
            aplica_iva INTEGER NOT NULL DEFAULT 0,
            usuario TEXT,
            observacion TEXT,
            anulada INTEGER NOT NULL DEFAULT 0,
            created_at TEXT NOT NULL DEFAULT (datetime('now','localtime')),
            FOREIGN KEY (cliente_id) REFERENCES clientes(id)
        );
        CREATE INDEX IF NOT EXISTS idx_nd_cliente ON notas_debito(cliente_id);
        CREATE INDEX IF NOT EXISTS idx_nd_fecha ON notas_debito(fecha_emision);

        CREATE TABLE IF NOT EXISTS nota_debito_motivos (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            nota_debito_id INTEGER NOT NULL,
            razon TEXT NOT NULL,
            valor REAL NOT NULL DEFAULT 0,
            FOREIGN KEY (nota_debito_id) REFERENCES notas_debito(id) ON DELETE CASCADE
        );
        CREATE INDEX IF NOT EXISTS idx_nd_motivos ON nota_debito_motivos(nota_debito_id);
    ");

    // v2.5.44: si existe la tabla vieja sri_avanzado_config (de v2.5.43 BETA),
    // migrar los datos y borrarla. Ignora errores si no existe (caso normal).
    let tabla_vieja_existe: bool = conn.query_row(
        "SELECT 1 FROM sqlite_master WHERE type='table' AND name='sri_avanzado_config'",
        [], |_r| Ok(true),
    ).unwrap_or(false);
    if tabla_vieja_existe {
        let _ = conn.execute(
            "INSERT OR REPLACE INTO contabilidad_config
                (id, es_agente_retencion, resolucion_designacion, fecha_designacion,
                 tipo_contribuyente, obligado_contabilidad,
                 codigo_retencion_renta_default, codigo_retencion_iva_default,
                 contador_ruc, contador_nombre, observacion, updated_at)
             SELECT id, es_agente_retencion, resolucion_designacion, fecha_designacion,
                    tipo_contribuyente, obligado_contabilidad,
                    codigo_retencion_renta_default, codigo_retencion_iva_default,
                    contador_ruc, contador_nombre, observacion, updated_at
             FROM sri_avanzado_config WHERE id = 1",
            [],
        );
        let _ = conn.execute("DROP TABLE IF EXISTS sri_avanzado_config", []);
    }

    // ─── v2.5.53: Cuentas OAuth Gmail per-cliente para envio de emails ──────
    // Cada POS puede conectar su propia cuenta Gmail (sin pasar por
    // notificaciones@clouget.com centralizada). El refresh_token se persiste
    // localmente y se envia al microservicio email.clouget.com puntualmente
    // en cada envio (stateless desde el lado del microservicio).
    //
    // Si hay al menos 1 cuenta activa aqui, enviar_email_interno usa OAuth.
    // Si no, hace fallback al flow tradicional (cuentas centralizadas).
    let _ = conn.execute_batch("
        CREATE TABLE IF NOT EXISTS oauth_email_cuentas (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            proveedor TEXT NOT NULL DEFAULT 'gmail',
            email TEXT NOT NULL,
            refresh_token TEXT NOT NULL,
            from_name TEXT,
            activa INTEGER NOT NULL DEFAULT 1,
            created_at TEXT NOT NULL DEFAULT (datetime('now', 'localtime'))
        );
        CREATE UNIQUE INDEX IF NOT EXISTS idx_oauth_email_proveedor_email
            ON oauth_email_cuentas(proveedor, email);
    ");

    // ─── v2.5.48: Migración correctiva — ventas a crédito mal guardadas ─────
    // En versiones previas a v2.5.48, si el usuario seleccionaba "Transferencia"
    // y después tocaba "Crédito" en POS, el state quedaba con formaPago="TRANSFER"
    // + esFiado=true. La venta se guardaba con forma_pago="TRANSFER" en BD aunque
    // realmente era crédito (porque se creaba la CXC asociada).
    //
    // Esta migración detecta esas ventas (TRANSFER + tiene CXC activa) y las
    // reclasifica a CREDITO. Solo se ejecuta una vez al arrancar v2.5.48+.
    let _ = conn.execute(
        "UPDATE ventas
         SET forma_pago = 'CREDITO'
         WHERE forma_pago = 'TRANSFER'
           AND id IN (
               SELECT venta_id FROM cuentas_por_cobrar
               WHERE venta_id IS NOT NULL AND estado != 'ANULADA'
           )",
        [],
    );

    // ─── v2.5.46: Datos fiscales del proveedor para retenciones SRI ──────────
    // Para emitir un comprobante de retención al SRI necesitamos saber:
    //   - tipo_identificacion del sujeto retenido (RUC/CEDULA/PASAPORTE)
    //   - si está obligado a llevar contabilidad (SI/NO)
    //   - tipo de sujeto (01=Persona Natural, 02=Sociedad)
    // Estos campos son opcionales en la app — si no están, el comando intenta
    // inferirlos desde el RUC (largo + tercer dígito).
    let _ = conn.execute("ALTER TABLE proveedores ADD COLUMN tipo_identificacion TEXT", []);
    let _ = conn.execute("ALTER TABLE proveedores ADD COLUMN obligado_contabilidad INTEGER NOT NULL DEFAULT 0", []);
    let _ = conn.execute("ALTER TABLE proveedores ADD COLUMN tipo TEXT", []); // "01"=PN, "02"=Sociedad

    // ─── v2.5.35: Datos del comprobante NC del proveedor ─────────────────────
    // La tabla compra_devoluciones almacena devoluciones internas. Ahora también
    // puede guardar los datos del comprobante NC SRI que el proveedor emitió
    // (importado por XML o ingresado manualmente). Esto da trazabilidad fiscal
    // completa: la devolución contable está respaldada por el documento del SRI.
    let _ = conn.execute("ALTER TABLE compra_devoluciones ADD COLUMN numero_nc TEXT", []);
    let _ = conn.execute("ALTER TABLE compra_devoluciones ADD COLUMN clave_acceso_nc TEXT", []);
    let _ = conn.execute("ALTER TABLE compra_devoluciones ADD COLUMN estado_sri_nc TEXT", []);
    let _ = conn.execute("ALTER TABLE compra_devoluciones ADD COLUMN fecha_emision_nc TEXT", []);
    let _ = conn.execute("ALTER TABLE compra_devoluciones ADD COLUMN xml_nc_firmado TEXT", []);
    // v2.5.42: tipo_nc — MERCANCIA (revierte stock) o AJUSTE_PRECIO (no toca stock, ajusta CXP + precio_costo)
    let _ = conn.execute("ALTER TABLE compra_devoluciones ADD COLUMN tipo_nc TEXT NOT NULL DEFAULT 'MERCANCIA'", []);
    // v2.5.42: config global para permitir stock negativo al anular/devolver
    let _ = conn.execute(
        "INSERT OR IGNORE INTO config (key, value) VALUES ('permitir_anulacion_stock_negativo', '0')",
        [],
    );
    // UNIQUE INDEX parcial sobre clave_acceso_nc (49 dig SRI) — evita re-importar misma NC
    let _ = conn.execute(
        "CREATE UNIQUE INDEX IF NOT EXISTS idx_compra_dev_clave_nc_unique
         ON compra_devoluciones(clave_acceso_nc)
         WHERE clave_acceso_nc IS NOT NULL AND clave_acceso_nc != ''",
        [],
    );

    // ─── v2.5.32: Gastos importados desde XML SRI ────────────────────────────
    // Para evitar duplicar la importación de un mismo XML cuando TODO el contenido
    // va como gasto (sin crear compra). También trazar el origen.
    let _ = conn.execute("ALTER TABLE gastos ADD COLUMN clave_acceso TEXT", []);
    let _ = conn.execute("ALTER TABLE gastos ADD COLUMN numero_factura_xml TEXT", []);
    let _ = conn.execute("ALTER TABLE gastos ADD COLUMN proveedor_id INTEGER", []);
    // UNIQUE INDEX parcial sobre clave_acceso del gasto — bloquea reimportación XML
    let _ = conn.execute(
        "CREATE UNIQUE INDEX IF NOT EXISTS idx_gastos_clave_acceso_unique
         ON gastos(clave_acceso) WHERE clave_acceso IS NOT NULL AND clave_acceso != ''",
        [],
    );

    // ─── v2.5.31: One-shot migration — recalcular CXC con retenciones ────────
    // Las versiones anteriores registraban retenciones recibidas en
    // `retenciones_recibidas` pero NO actualizaban `cuentas_por_cobrar.saldo`.
    // Esto genera CXC con saldo > 0 que en realidad están saldadas por la retención.
    // Este UPDATE corrige los registros existentes una sola vez (idempotente: si ya
    // se ejecutó, el UPDATE no cambia nada porque los saldos ya son correctos).
    let _ = conn.execute("
        UPDATE cuentas_por_cobrar
        SET saldo = MAX(0, monto_total - monto_pagado -
            COALESCE((SELECT SUM(valor) FROM retenciones_recibidas
                      WHERE venta_id = cuentas_por_cobrar.venta_id), 0)),
            estado = CASE
                WHEN (monto_total - monto_pagado -
                      COALESCE((SELECT SUM(valor) FROM retenciones_recibidas
                                WHERE venta_id = cuentas_por_cobrar.venta_id), 0)) <= 0.01
                THEN 'PAGADA'
                ELSE estado
            END,
            updated_at = datetime('now','localtime')
        WHERE estado IN ('PENDIENTE', 'PAGADA')
          AND EXISTS (SELECT 1 FROM retenciones_recibidas
                      WHERE venta_id = cuentas_por_cobrar.venta_id)
    ", []);

    // ─── v2.5.62: One-shot auto-repair — anulaciones con stock no revertido ──
    // Reportado: anular_venta usaba .ok() en los UPDATE de stock, silenciando
    // errores. En instalaciones viejas (columna updated_at faltante, triggers
    // rotos, etc.) la venta quedaba marcada anulada=1 pero el stock no volvía.
    //
    // Esta migración corre al arrancar la app:
    //   1. Busca ventas con anulada=1
    //   2. Para cada item de esas ventas, verifica si existe movimiento
    //      'ANULACION_VENTA' en movimientos_inventario
    //   3. Si NO existe → suma cant*factor al stock del producto y crea el
    //      movimiento ahora (auditable). Skip si es servicio/no_controla_stock.
    //
    // Es idempotente: tras correrla una vez, todos los items quedan marcados
    // y futuras corridas no hacen nada.
    {
        // Cargar items huérfanos en memoria primero (para evitar locks anidados)
        let mut stmt = conn.prepare("
            SELECT vd.id, vd.venta_id, vd.producto_id,
                   vd.cantidad, COALESCE(vd.factor_unidad, 1) as factor,
                   vd.lote_id, p.stock_actual,
                   v.numero
            FROM venta_detalles vd
            JOIN ventas v ON vd.venta_id = v.id
            JOIN productos p ON vd.producto_id = p.id
            WHERE v.anulada = 1
              AND vd.producto_id IS NOT NULL
              AND COALESCE(p.es_servicio, 0) = 0
              AND COALESCE(p.no_controla_stock, 0) = 0
              AND NOT EXISTS (
                  SELECT 1 FROM movimientos_inventario mi
                  WHERE mi.referencia_id = vd.venta_id
                    AND mi.producto_id = vd.producto_id
                    AND mi.tipo = 'ANULACION_VENTA'
              )
        ");
        if let Ok(mut stmt) = stmt {
            let rows = stmt.query_map([], |r| Ok((
                r.get::<_, i64>(0)?,       // detalle_id (no usado, solo dedup)
                r.get::<_, i64>(1)?,       // venta_id
                r.get::<_, i64>(2)?,       // producto_id
                r.get::<_, f64>(3)?,       // cantidad
                r.get::<_, f64>(4)?,       // factor
                r.get::<_, Option<i64>>(5)?, // lote_id
                r.get::<_, f64>(6)?,       // stock_actual
                r.get::<_, String>(7)?,    // venta numero
            )));

            if let Ok(rows) = rows {
                let items: Vec<(i64, i64, i64, f64, f64, Option<i64>, f64, String)> =
                    rows.filter_map(|r| r.ok()).collect();
                // Nota: NO hacemos drop(stmt) — sale de scope al final del
                // `if let Ok(mut stmt) = stmt {`. Hacerlo manualmente confunde
                // al borrow checker porque `rows` borrowea `stmt` aunque ya
                // colectamos en `items`.

                let cuantos = items.len();
                if cuantos > 0 {
                    eprintln!(
                        "[Migración v2.5.62] Detectados {} item(s) de venta(s) anuladas sin reversión de stock. Reparando...",
                        cuantos
                    );

                    for (_det_id, venta_id, prod_id, cant, factor, lote_id, stock_antes, numero) in &items {
                        let cant_base = cant * factor;
                        let stock_despues = stock_antes + cant_base;

                        // Reintegrar stock
                        let upd_stock = conn.execute(
                            "UPDATE productos SET stock_actual = stock_actual + ?1 WHERE id = ?2",
                            rusqlite::params![cant_base, prod_id],
                        );
                        if let Err(e) = upd_stock {
                            eprintln!("[Migración v2.5.62] Error reintegrando stock producto {}: {}", prod_id, e);
                            continue;
                        }

                        // Reintegrar lote si aplica
                        if let Some(lid) = lote_id {
                            let _ = conn.execute(
                                "UPDATE lotes_caducidad SET cantidad = cantidad + ?1 WHERE id = ?2",
                                rusqlite::params![cant_base, lid],
                            );
                        }

                        // Crear movimiento auditable
                        let _ = conn.execute(
                            "INSERT INTO movimientos_inventario
                                (producto_id, tipo, cantidad, stock_anterior, stock_nuevo,
                                 motivo, usuario, referencia_id)
                             VALUES (?1, 'ANULACION_VENTA', ?2, ?3, ?4, ?5, ?6, ?7)",
                            rusqlite::params![
                                prod_id, cant_base, stock_antes, stock_despues,
                                format!("AUTO-REPARACION migracion v2.5.62 (anulacion {} no habia revertido stock)", numero),
                                "sistema", venta_id
                            ],
                        );
                    }
                    eprintln!("[Migración v2.5.62] Auto-reparación completada para {} item(s).", cuantos);
                }
            }
        }
    }

    // ─── v2.5.63: Auto-reparación de caja — anulaciones que no descontaron ───
    // Reportado: al anular venta EFECTIVO, el monto_esperado de caja NO se
    // descontaba (mismo bug del .ok() silenciando errores).
    //
    // Solo CAJAS ABIERTAS. Cajas cerradas ya están cuadradas (el cierre asumió
    // el monto que tenía en su momento y no se debe re-ajustar).
    //
    // Idempotente vía flag en `config`: corre 1 sola vez por instalación.
    let migracion_aplicada: bool = conn.query_row(
        "SELECT 1 FROM config WHERE key = 'migracion_v2_5_63_caja_anulada_aplicada'",
        [], |_| Ok(true),
    ).unwrap_or(false);
    if !migracion_aplicada {
        // Resta del monto_esperado de cajas abiertas el total de cada venta
        // EFECTIVO anulada que tiene la columna caja_id apuntando a esa caja.
        // Esto compensa el descuento que originalmente debía haber hecho
        // anular_venta pero que falló silenciosamente.
        let mut stmt = conn.prepare("
            SELECT v.id, v.numero, v.total, v.caja_id
            FROM ventas v
            JOIN caja c ON v.caja_id = c.id
            WHERE v.anulada = 1
              AND v.forma_pago = 'EFECTIVO'
              AND c.estado = 'ABIERTA'
        ");
        if let Ok(mut stmt) = stmt {
            let rows = stmt.query_map([], |r| Ok((
                r.get::<_, i64>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, f64>(2)?,
                r.get::<_, i64>(3)?,
            )));

            if let Ok(rows) = rows {
                let items: Vec<(i64, String, f64, i64)> = rows.filter_map(|r| r.ok()).collect();
                let cuantos = items.len();

                if cuantos > 0 {
                    eprintln!(
                        "[Migración v2.5.63] {} anulación(es) EFECTIVO en caja abierta sin descuento. Compensando...",
                        cuantos
                    );

                    for (_venta_id, numero, total, caja_id) in &items {
                        let _ = conn.execute(
                            "UPDATE caja SET monto_esperado = monto_esperado - ?1,
                                              monto_ventas   = monto_ventas - ?1
                             WHERE id = ?2",
                            rusqlite::params![total, caja_id],
                        );
                        eprintln!("  · Venta {} ${:.2} compensada en caja #{}", numero, total, caja_id);
                    }
                    eprintln!("[Migración v2.5.63] Compensación completada.");
                }
            }
        }
        let _ = conn.execute(
            "INSERT OR REPLACE INTO config (key, value) VALUES ('migracion_v2_5_63_caja_anulada_aplicada', '1')",
            [],
        );
    }

    Ok(())
}
