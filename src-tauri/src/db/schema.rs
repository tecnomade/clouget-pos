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
        INSERT OR IGNORE INTO config (key, value) VALUES ('sri_facturas_gratis', '10');
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

    Ok(())
}
