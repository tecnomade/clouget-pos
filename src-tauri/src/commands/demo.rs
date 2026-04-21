use tauri::State;
use crate::db::Database;
use crate::commands::licencia::LicenciaInfo;
use crate::utils;

/// Activa el modo demo con datos ficticios precargados.
/// Solo disponible si no hay licencia activada.
#[tauri::command]
pub fn activar_demo(db: State<Database>) -> Result<LicenciaInfo, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;

    let get_cfg = |key: &str| -> String {
        conn.query_row(
            "SELECT value FROM config WHERE key = ?1",
            rusqlite::params![key],
            |row| row.get(0),
        )
        .unwrap_or_default()
    };

    // No permitir demo si ya tiene licencia activa
    if get_cfg("licencia_activada") == "1" {
        return Err("No se puede activar el demo con una licencia activa".to_string());
    }

    // Configurar negocio demo
    let configs = [
        ("nombre_negocio", "Tienda El Bosque"),
        ("ruc", "1790016919001"),
        ("direccion", "Av. Amazonas N25-78, Quito, Ecuador"),
        ("telefono", "+593 2 2345678"),
        ("regimen", "RIMPE_EMPRENDEDOR"),
        ("sri_modulo_activo", "1"),
        ("sri_ambiente", "pruebas"),
        ("sri_certificado_cargado", "1"),
        ("sri_facturas_gratis", "999999"),
        ("sri_facturas_usadas", "0"),
        ("demo_activo", "1"),
        // Activar todos los módulos opcionales en demo
        ("modulo_series_activo", "1"),
        ("modulo_caducidad", "1"),
        ("caducidad_dias_alerta", "7"),
    ];
    for (key, value) in &configs {
        conn.execute(
            "INSERT OR REPLACE INTO config (key, value) VALUES (?1, ?2)",
            rusqlite::params![key, value],
        )
        .map_err(|e| format!("Error configurando demo: {}", e))?;
    }

    // --- Categorías (8) ---
    conn.execute_batch(
        "INSERT OR IGNORE INTO categorias (id, nombre, descripcion, activo) VALUES
            (1, 'Abarrotes', 'Productos de primera necesidad', 1),
            (2, 'Bebidas', 'Bebidas y refrescos', 1),
            (3, 'Higiene Personal', 'Productos de aseo y cuidado personal', 1),
            (4, 'Congelados', 'Carnes y productos congelados', 1),
            (5, 'Tecnologia', 'Equipos electronicos con numero de serie', 1),
            (6, 'Lacteos', 'Productos lacteos con caducidad', 1),
            (7, 'Panaderia', 'Pan y reposteria del dia', 1),
            (8, 'Limpieza Hogar', 'Productos de limpieza para el hogar', 1);",
    )
    .map_err(|e| format!("Error creando categorías: {}", e))?;

    // --- Productos (24) ---
    let productos = [
        // (codigo, nombre, categoria_id, precio_costo, precio_venta, iva%, stock, unidad)
        ("ABR001", "Arroz Tipo 1 (500g)", 1, 1.10, 1.50, 0.0, 80.0, "UND"),
        ("ABR002", "Fideos Spaghetti (400g)", 1, 0.65, 0.90, 0.0, 60.0, "UND"),
        ("ABR003", "Aceite Vegetal (1L)", 1, 2.20, 3.00, 0.0, 40.0, "UND"),
        ("ABR004", "Harina de Trigo (1kg)", 1, 0.90, 1.35, 0.0, 50.0, "UND"),
        ("ABR005", "Azucar Blanca (2kg)", 1, 1.80, 2.40, 0.0, 35.0, "UND"),
        ("ABR006", "Sal de Mesa (500g)", 1, 0.40, 0.65, 0.0, 90.0, "UND"),
        ("ABR007", "Atun en lata (170g)", 1, 1.20, 1.80, 0.0, 65.0, "UND"),
        ("ABR008", "Lentejas (500g)", 1, 0.95, 1.40, 0.0, 25.0, "UND"),
        ("BEB001", "Agua Mineral 6-Pack", 2, 1.80, 2.50, 0.0, 45.0, "UND"),
        ("BEB002", "Jugo Natural Naranja (1L)", 2, 1.50, 2.25, 15.0, 30.0, "UND"),
        ("BEB003", "Refresco Cola (2L)", 2, 1.40, 2.00, 15.0, 55.0, "UND"),
        ("BEB004", "Cerveza Lata 350ml", 2, 0.85, 1.30, 15.0, 4.0, "UND"),
        ("BEB005", "Energizante 300ml", 2, 1.30, 2.10, 15.0, 28.0, "UND"),
        ("HIG001", "Jabon de Bano (3-pack)", 3, 1.00, 1.75, 15.0, 70.0, "UND"),
        ("HIG002", "Pasta Dental (75ml)", 3, 1.20, 1.95, 15.0, 50.0, "UND"),
        ("HIG003", "Shampoo (250ml)", 3, 2.50, 3.80, 15.0, 35.0, "UND"),
        ("HIG004", "Papel Higienico 4 rollos", 3, 1.60, 2.50, 15.0, 80.0, "UND"),
        ("HIG005", "Detergente (1kg)", 3, 2.10, 3.20, 15.0, 3.0, "UND"),
        ("CON001", "Pollo Entero", 4, 3.50, 5.50, 0.0, 20.0, "KG"),
        ("CON002", "Carne de Res", 4, 5.80, 8.50, 0.0, 15.0, "KG"),
        ("CON003", "Pescado Tilapia", 4, 4.20, 6.80, 0.0, 8.0, "KG"),
        ("CON004", "Queso Fresco (500g)", 4, 2.50, 4.00, 15.0, 12.0, "UND"),
        ("SRV001", "Servicio de Delivery", 1, 0.0, 2.50, 15.0, 0.0, "UND"),
        ("SRV002", "Empaque para Regalo", 1, 0.0, 1.00, 15.0, 0.0, "UND"),
        // Tecnologia (cat 5) — requieren serie
        ("TEC001", "Laptop HP 15.6\" i5 8GB 512SSD", 5, 480.00, 650.00, 15.0, 5.0, "UND"),
        ("TEC002", "Celular Samsung A15 128GB", 5, 180.00, 245.00, 15.0, 8.0, "UND"),
        ("TEC003", "Tablet Lenovo M10 64GB", 5, 145.00, 199.00, 15.0, 4.0, "UND"),
        ("TEC004", "Audifonos Bluetooth JBL", 5, 18.00, 32.00, 15.0, 15.0, "UND"),
        ("TEC005", "Teclado mecanico RGB", 5, 22.00, 38.50, 15.0, 7.0, "UND"),
        // Lacteos (cat 6) — requieren caducidad
        ("LAC001", "Leche Entera (1L)", 6, 0.85, 1.20, 0.0, 60.0, "UND"),
        ("LAC002", "Yogurt Frutilla (200g)", 6, 0.45, 0.80, 0.0, 80.0, "UND"),
        ("LAC003", "Mantequilla (250g)", 6, 1.40, 2.20, 15.0, 25.0, "UND"),
        ("LAC004", "Queso Mozzarella (400g)", 6, 2.80, 4.50, 15.0, 18.0, "UND"),
        // Panaderia (cat 7) — requieren caducidad (corta)
        ("PAN001", "Pan de Yema (UND)", 7, 0.10, 0.20, 0.0, 120.0, "UND"),
        ("PAN002", "Pan Integral (500g)", 7, 0.95, 1.50, 0.0, 30.0, "UND"),
        ("PAN003", "Torta Chocolate (porc)", 7, 1.20, 2.50, 15.0, 12.0, "UND"),
        // Limpieza Hogar (cat 8)
        ("LIM001", "Cloro 4L", 8, 1.80, 2.80, 15.0, 22.0, "UND"),
        ("LIM002", "Desinfectante Floral 1L", 8, 1.20, 1.95, 15.0, 30.0, "UND"),
        ("LIM003", "Esponja Multiuso 3-pack", 8, 0.65, 1.10, 15.0, 50.0, "UND"),
        ("LIM004", "Bolsa Basura 30L 10UN", 8, 0.85, 1.45, 15.0, 40.0, "UND"),
    ];

    for (codigo, nombre, cat_id, costo, venta, iva, stock, unidad) in &productos {
        conn.execute(
            "INSERT OR IGNORE INTO productos (codigo, nombre, categoria_id, precio_costo, precio_venta, iva_porcentaje, incluye_iva, stock_actual, stock_minimo, unidad_medida, activo)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, 0, ?7, 5, ?8, 1)",
            rusqlite::params![codigo, nombre, cat_id, costo, venta, iva, stock, unidad],
        )
        .map_err(|e| format!("Error creando producto: {}", e))?;
    }

    // --- Clientes (15) ---
    conn.execute_batch(
        "INSERT OR IGNORE INTO clientes (tipo_identificacion, identificacion, nombre, direccion, telefono, email) VALUES
            ('RUC', '0992345678001', 'Restaurante Don Jorge', 'Av. 9 de Octubre 456, Guayaquil', '+593 4 2567890', 'donjorge@restaurant.com'),
            ('RUC', '0987654321001', 'Tienda La Esquina', 'Calle Sucre 789, Cuenca', '+593 7 2891234', 'tienda@laesquina.com'),
            ('CEDULA', '0912345678', 'Maria Lopez', 'Calle Bolivar 321, Quito', '0991234567', 'maria@email.com'),
            ('CEDULA', '0923456789', 'Juan Perez', 'Av. Amazonas N32-15, Quito', '0987654321', 'juan.perez@gmail.com'),
            ('RUC', '1791234567001', 'Comercial Andes S.A.', 'Av. Eloy Alfaro y NN.UU., Quito', '+593 2 2456789', 'compras@andes.com.ec'),
            ('CEDULA', '0934567890', 'Ana Garcia', 'Cdla. Kennedy Norte, Guayaquil', '0998877665', 'ana.garcia@hotmail.com'),
            ('CEDULA', '0945678901', 'Carlos Vega', 'Av. Solano y 12 de Abril, Cuenca', '0976543210', NULL),
            ('RUC', '0193456789001', 'Hostería Las Cabañas', 'Via a Banos km 3, Banos', '+593 3 2740555', 'reservas@lascabanas.com'),
            ('CEDULA', '1712345670', 'Sofia Mendoza', 'Cdla. Kennedy Vieja, Guayaquil', '0992223344', 'sofiam@yahoo.com'),
            ('CEDULA', '1798765430', 'Roberto Castillo', 'Calle Larga 4-23, Cuenca', '0995556677', 'rcastillo@hotmail.com'),
            ('PASAPORTE', 'AB123456', 'Michael Brown', 'Hotel Quito Plaza, Quito', '0987887665', 'michael.brown@gmail.com'),
            ('CEDULA', '0987654322', 'Lucia Paredes', 'Sector Iñaquito, Quito', '0998120000', NULL),
            ('RUC', '1715678900001', 'Distribuidora Los Andes', 'Av. America N20-45, Quito', '+593 2 2345099', 'compras@losandes.com'),
            ('CEDULA', '1723456789', 'Pablo Salazar', 'La Mariscal, Quito', '0998340120', 'psalazar@outlook.com'),
            ('CEDULA', '0934567891', 'Mireya Palacios', 'Cdla. Urdesa, Guayaquil', '0991122334', 'mireyapal@gmail.com');"
    ).map_err(|e| format!("Error creando clientes: {}", e))?;

    // --- Usuarios (2) ---
    let salt_admin = utils::generar_salt();
    let hash_admin = utils::hash_pin(&salt_admin, "1234");
    conn.execute(
        "INSERT OR IGNORE INTO usuarios (nombre, pin_hash, pin_salt, rol, activo)
         VALUES ('Admin', ?1, ?2, 'ADMIN', 1)",
        rusqlite::params![hash_admin, salt_admin],
    ).map_err(|e| format!("Error creando usuario admin: {}", e))?;

    let salt_cajero = utils::generar_salt();
    let hash_cajero = utils::hash_pin(&salt_cajero, "0000");
    conn.execute(
        "INSERT OR IGNORE INTO usuarios (nombre, pin_hash, pin_salt, rol, activo)
         VALUES ('Cajero', ?1, ?2, 'CAJERO', 1)",
        rusqlite::params![hash_cajero, salt_cajero],
    ).map_err(|e| format!("Error creando usuario cajero: {}", e))?;

    // Tecnico (PIN 5555)
    let salt_tec = utils::generar_salt();
    let hash_tec = utils::hash_pin(&salt_tec, "5555");
    conn.execute(
        "INSERT OR IGNORE INTO usuarios (nombre, pin_hash, pin_salt, rol, activo)
         VALUES ('Tecnico', ?1, ?2, 'TECNICO', 1)",
        rusqlite::params![hash_tec, salt_tec],
    ).ok();

    // Cajero adicional (PIN 1111)
    let salt_c2 = utils::generar_salt();
    let hash_c2 = utils::hash_pin(&salt_c2, "1111");
    conn.execute(
        "INSERT OR IGNORE INTO usuarios (nombre, pin_hash, pin_salt, rol, activo)
         VALUES ('Cajero 2', ?1, ?2, 'CAJERO', 1)",
        rusqlite::params![hash_c2, salt_c2],
    ).ok();

    // --- Cuentas bancarias (2) ---
    conn.execute(
        "INSERT OR IGNORE INTO cuentas_banco (nombre, tipo_cuenta, numero_cuenta, titular, activa)
         VALUES ('Banco Pichincha', 'Ahorros', '2205678901', 'Tienda El Bosque', 1)",
        [],
    ).map_err(|e| format!("Error creando banco: {}", e))?;

    conn.execute(
        "INSERT OR IGNORE INTO cuentas_banco (nombre, tipo_cuenta, numero_cuenta, titular, activa)
         VALUES ('Cooperativa JEP', 'Corriente', '1100987654', 'Tienda El Bosque', 1)",
        [],
    ).map_err(|e| format!("Error creando banco: {}", e))?;

    // --- Lista de precios "Mayorista" ---
    let lista_mayorista_id: i64 = conn.query_row(
        "SELECT id FROM listas_precios WHERE nombre = 'Mayorista'",
        [],
        |row| row.get(0),
    ).unwrap_or_else(|_| {
        conn.execute(
            "INSERT INTO listas_precios (nombre, descripcion, es_default, activo) VALUES ('Mayorista', 'Precios para compras al por mayor', 0, 1)",
            [],
        ).ok();
        conn.last_insert_rowid()
    });

    // Asignar precios mayoristas (15% menos que precio_venta)
    let prods: Vec<(i64, f64)> = {
        let mut stmt = conn.prepare("SELECT id, precio_venta FROM productos WHERE activo = 1")
            .map_err(|e| e.to_string())?;
        let results = stmt.query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
            .map_err(|e| e.to_string())?
            .filter_map(|r| r.ok())
            .collect();
        results
    };
    for (prod_id, precio) in &prods {
        let precio_mayorista = (precio * 0.85 * 100.0).round() / 100.0;
        conn.execute(
            "INSERT OR IGNORE INTO precios_producto (lista_precio_id, producto_id, precio) VALUES (?1, ?2, ?3)",
            rusqlite::params![lista_mayorista_id, prod_id, precio_mayorista],
        ).ok();
    }

    // Asignar lista Mayorista a clientes con RUC (negocios)
    conn.execute(
        "UPDATE clientes SET lista_precio_id = ?1 WHERE tipo_identificacion = 'RUC' AND identificacion != '9999999999999'",
        rusqlite::params![lista_mayorista_id],
    ).ok();

    // Asignar precios de lista por defecto (Precio Publico) a todos los productos
    conn.execute(
        "INSERT OR IGNORE INTO precios_producto (lista_precio_id, producto_id, precio)
         SELECT 1, id, precio_venta FROM productos WHERE activo = 1",
        [],
    ).ok();

    // --- Proveedores (3) ---
    conn.execute_batch(
        "INSERT OR IGNORE INTO proveedores (ruc, nombre, contacto, telefono, email, direccion, dias_credito, activo) VALUES
            ('0990012345001', 'Distribuidora El Sol', 'Carlos Mendez', '+593 4 2345678', 'ventas@elsol.com', 'Km 5 Via Daule, Guayaquil', 30, 1),
            ('1790034567001', 'Importadora Central', 'Ana Torres', '+593 2 2876543', 'pedidos@importcentral.com', 'Av. 10 de Agosto N45-12, Quito', 15, 1),
            ('0190078901001', 'Lacteos del Sur', 'Pedro Ramos', '+593 7 2834567', 'info@lacteosdelsur.com', 'Av. Remigio Crespo 3-42, Cuenca', 45, 1);"
    ).map_err(|e| format!("Error creando proveedores: {}", e))?;

    // --- Gastos demo (variados con diferentes fechas y categorías) ---
    conn.execute_batch(
        "INSERT OR IGNORE INTO gastos (descripcion, monto, categoria, observacion, fecha, es_recurrente) VALUES
            ('Pago arriendo local', 350.00, 'Arriendo', 'Mensual', datetime('now', 'localtime', '-15 days'), 1),
            ('Compra bolsas plasticas', 15.00, 'Suministros', 'Stock para el mes', datetime('now', 'localtime', '-10 days'), 0),
            ('Servicio de internet', 35.00, 'Servicios', 'CNT plan negocio', datetime('now', 'localtime', '-7 days'), 1),
            ('Mantenimiento refrigeradora', 80.00, 'Mantenimiento', 'Cambio de filtro', datetime('now', 'localtime', '-5 days'), 0),
            ('Pago luz electrica', 45.00, 'Servicios', 'Consumo del mes', datetime('now', 'localtime', '-3 days'), 1),
            ('Combustible vehiculo', 25.00, 'Transporte', 'Repartidor', datetime('now', 'localtime', '-2 days'), 0),
            ('Compra papel impresora', 8.50, 'Suministros', NULL, datetime('now', 'localtime', '-1 days'), 0),
            ('Pago agua potable', 18.00, 'Servicios', 'Mensual', datetime('now', 'localtime', '-1 days'), 1),
            ('Reparacion computadora', 60.00, 'Mantenimiento', 'Cambio de disco', datetime('now', 'localtime'), 0),
            ('Sueldo cajero quincenal', 200.00, 'Sueldos', NULL, datetime('now', 'localtime'), 1);"
    ).map_err(|e| format!("Error creando gastos: {}", e))?;

    // --- Abrir caja demo ---
    conn.execute(
        "INSERT OR IGNORE INTO caja (id, monto_inicial, fecha_apertura, estado, usuario)
         VALUES (1, 50.00, datetime('now', 'localtime', '-7 days'), 'ABIERTA', 'Admin')",
        [],
    ).ok();

    // --- Ventas demo (últimos 7 días) ---
    for day_offset in (0..7).rev() {
        let num_ventas = if day_offset == 0 { 3 } else { 2 };
        for v in 0..num_ventas {
            let offset_str = format!("-{} days", day_offset);
            let hora = format!("{:02}:{}:00", 9 + v * 3, if v % 2 == 0 { "15" } else { "45" });

            // Get next secuencial
            let sec: i64 = conn.query_row(
                "SELECT COALESCE(MAX(CAST(SUBSTR(numero, 4) AS INTEGER)), 0) + 1 FROM ventas WHERE numero LIKE 'NV-%'",
                [], |r| r.get(0)
            ).unwrap_or(1);
            let numero = format!("NV-{:09}", sec);

            // Pick products based on day and venta index
            let prod_idx1 = ((day_offset + v) % 12) as i64 + 1;
            let prod_idx2 = ((day_offset + v + 3) % 12) as i64 + 1;
            let cant1 = (v + 1) as f64;
            let cant2 = if v % 2 == 0 { 2.0 } else { 1.0 };

            // Get product prices
            let (p1_precio, p1_iva): (f64, f64) = conn.query_row(
                "SELECT precio_venta, iva_porcentaje FROM productos WHERE id = ?1",
                rusqlite::params![prod_idx1], |r| Ok((r.get(0)?, r.get(1)?))
            ).unwrap_or((2.0, 0.0));
            let (p2_precio, p2_iva): (f64, f64) = conn.query_row(
                "SELECT precio_venta, iva_porcentaje FROM productos WHERE id = ?1",
                rusqlite::params![prod_idx2], |r| Ok((r.get(0)?, r.get(1)?))
            ).unwrap_or((1.5, 0.0));

            let sub1 = cant1 * p1_precio;
            let sub2 = cant2 * p2_precio;
            // Split by IVA
            let subtotal_sin_iva = if p1_iva == 0.0 { sub1 } else { 0.0 } + if p2_iva == 0.0 { sub2 } else { 0.0 };
            let subtotal_con_iva = if p1_iva > 0.0 { sub1 } else { 0.0 } + if p2_iva > 0.0 { sub2 } else { 0.0 };
            let iva_total = sub1 * (p1_iva / 100.0) + sub2 * (p2_iva / 100.0);
            let total = subtotal_sin_iva + subtotal_con_iva + iva_total;
            let forma_pago = if v % 3 == 0 { "TRANSFER" } else { "EFECTIVO" };

            conn.execute(
                "INSERT INTO ventas (numero, cliente_id, subtotal_sin_iva, subtotal_con_iva, iva, descuento, total, forma_pago, monto_recibido, cambio, tipo_documento, estado, estado_sri, usuario, fecha)
                 VALUES (?1, 1, ?2, ?3, ?4, 0, ?5, ?6, ?5, 0, 'NOTA_VENTA', 'COMPLETADA', 'NO_APLICA', 'Admin', datetime('now', 'localtime', ?7, ?8))",
                rusqlite::params![numero, subtotal_sin_iva, subtotal_con_iva, iva_total, total, forma_pago, offset_str, hora],
            ).ok();

            let venta_id = conn.last_insert_rowid();

            // Insert venta_detalles
            let p1_costo: f64 = conn.query_row("SELECT precio_costo FROM productos WHERE id = ?1", rusqlite::params![prod_idx1], |r| r.get(0)).unwrap_or(1.0);
            let p2_costo: f64 = conn.query_row("SELECT precio_costo FROM productos WHERE id = ?1", rusqlite::params![prod_idx2], |r| r.get(0)).unwrap_or(0.8);

            conn.execute(
                "INSERT INTO venta_detalles (venta_id, producto_id, cantidad, precio_unitario, descuento, iva_porcentaje, subtotal, precio_costo)
                 VALUES (?1, ?2, ?3, ?4, 0, ?5, ?6, ?7)",
                rusqlite::params![venta_id, prod_idx1, cant1, p1_precio, p1_iva, sub1, p1_costo],
            ).ok();
            conn.execute(
                "INSERT INTO venta_detalles (venta_id, producto_id, cantidad, precio_unitario, descuento, iva_porcentaje, subtotal, precio_costo)
                 VALUES (?1, ?2, ?3, ?4, 0, ?5, ?6, ?7)",
                rusqlite::params![venta_id, prod_idx2, cant2, p2_precio, p2_iva, sub2, p2_costo],
            ).ok();

            // Decrease stock
            conn.execute("UPDATE productos SET stock_actual = stock_actual - ?1 WHERE id = ?2", rusqlite::params![cant1, prod_idx1]).ok();
            conn.execute("UPDATE productos SET stock_actual = stock_actual - ?1 WHERE id = ?2", rusqlite::params![cant2, prod_idx2]).ok();
        }
    }

    // --- Compra demo ---
    conn.execute(
        "INSERT OR IGNORE INTO compras (numero, proveedor_id, subtotal, iva, total, forma_pago, es_credito, numero_factura, observacion, fecha, estado)
         VALUES ('CMP-000001', 1, 147.50, 9.98, 157.48, 'EFECTIVO', 1, 'FAC-001-2345', 'Compra semanal abarrotes', datetime('now', 'localtime', '-3 days'), 'REGISTRADA')",
        [],
    ).ok();
    let compra_id = conn.last_insert_rowid();
    conn.execute(
        "INSERT OR IGNORE INTO compra_detalles (compra_id, producto_id, cantidad, precio_unitario, subtotal)
         VALUES (?1, 1, 50, 1.10, 55.00)",
        rusqlite::params![compra_id],
    ).ok();
    conn.execute(
        "INSERT OR IGNORE INTO compra_detalles (compra_id, producto_id, cantidad, precio_unitario, subtotal)
         VALUES (?1, 2, 40, 0.65, 26.00)",
        rusqlite::params![compra_id],
    ).ok();
    conn.execute(
        "INSERT OR IGNORE INTO compra_detalles (compra_id, producto_id, cantidad, precio_unitario, subtotal)
         VALUES (?1, 3, 20, 2.20, 44.00)",
        rusqlite::params![compra_id],
    ).ok();
    conn.execute(
        "INSERT OR IGNORE INTO compra_detalles (compra_id, producto_id, cantidad, precio_unitario, subtotal)
         VALUES (?1, 6, 15, 1.50, 22.50)",
        rusqlite::params![compra_id],
    ).ok();

    // Cuenta por pagar for the credit purchase
    conn.execute(
        "INSERT OR IGNORE INTO cuentas_por_pagar (compra_id, proveedor_id, monto_total, monto_pagado, saldo, estado, fecha_vencimiento)
         VALUES (?1, 1, 157.48, 0, 157.48, 'PENDIENTE', date('now', '+27 days'))",
        rusqlite::params![compra_id],
    ).ok();

    // --- Cotizacion demo ---
    conn.execute(
        "INSERT OR IGNORE INTO secuenciales (establecimiento_codigo, punto_emision_codigo, tipo_documento, secuencial) VALUES ('001', '001', 'COTIZACION_SEQ', 1)",
        [],
    ).ok();
    conn.execute(
        "INSERT INTO ventas (numero, cliente_id, subtotal_sin_iva, subtotal_con_iva, iva, descuento, total, forma_pago, monto_recibido, cambio, tipo_documento, estado, estado_sri, tipo_estado, fecha)
         VALUES ('COT-000001', 2, 13.70, 11.40, 1.71, 0, 26.81, 'EFECTIVO', 0, 0, 'COTIZACION', 'PENDIENTE', 'NO_APLICA', 'COTIZACION', datetime('now', 'localtime', '-1 day'))",
        [],
    ).ok();
    let cot_id = conn.last_insert_rowid();
    conn.execute(
        "INSERT INTO venta_detalles (venta_id, producto_id, cantidad, precio_unitario, descuento, iva_porcentaje, subtotal, precio_costo)
         VALUES (?1, 10, 3, 3.80, 0, 15, 11.40, 2.50)",
        rusqlite::params![cot_id],
    ).ok();
    conn.execute(
        "INSERT INTO venta_detalles (venta_id, producto_id, cantidad, precio_unitario, descuento, iva_porcentaje, subtotal, precio_costo)
         VALUES (?1, 11, 2, 5.50, 0, 0, 11.00, 3.50)",
        rusqlite::params![cot_id],
    ).ok();
    conn.execute(
        "INSERT INTO venta_detalles (venta_id, producto_id, cantidad, precio_unitario, descuento, iva_porcentaje, subtotal, precio_costo)
         VALUES (?1, 4, 2, 1.35, 0, 0, 2.70, 0.90)",
        rusqlite::params![cot_id],
    ).ok();

    // --- Guias de remision demo (3) ---
    conn.execute(
        "INSERT INTO ventas (numero, cliente_id, subtotal_sin_iva, subtotal_con_iva, iva, descuento, total, forma_pago, monto_recibido, cambio, tipo_documento, estado, estado_sri, guia_placa, guia_chofer, guia_direccion_destino, fecha)
         VALUES ('GR-000001', 3, 6.00, 6.00, 0.90, 0, 12.90, 'EFECTIVO', 0, 0, 'GUIA_REMISION', 'PENDIENTE', 'NO_APLICA', 'PBQ-1234', 'Pedro Vasquez', 'Calle Bolivar 321, Quito', datetime('now', 'localtime'))",
        [],
    ).ok();
    let gr_id = conn.last_insert_rowid();
    conn.execute(
        "INSERT INTO venta_detalles (venta_id, producto_id, cantidad, precio_unitario, descuento, iva_porcentaje, subtotal, precio_costo)
         VALUES (?1, 7, 3, 2.00, 0, 15, 6.00, 1.40)",
        rusqlite::params![gr_id],
    ).ok();
    conn.execute(
        "INSERT INTO venta_detalles (venta_id, producto_id, cantidad, precio_unitario, descuento, iva_porcentaje, subtotal, precio_costo)
         VALUES (?1, 1, 4, 1.50, 0, 0, 6.00, 1.10)",
        rusqlite::params![gr_id],
    ).ok();

    // Guia entregada (cerrada)
    conn.execute(
        "INSERT INTO ventas (numero, cliente_id, subtotal_sin_iva, subtotal_con_iva, iva, descuento, total, forma_pago, monto_recibido, cambio, tipo_documento, estado, estado_sri, guia_placa, guia_chofer, guia_direccion_destino, fecha)
         VALUES ('GR-000002', 5, 25.50, 0, 0, 0, 25.50, 'EFECTIVO', 0, 0, 'GUIA_REMISION', 'ENTREGADA', 'NO_APLICA', 'GBC-5678', 'Luis Mora', 'Av. Eloy Alfaro y NN.UU., Quito', datetime('now', 'localtime', '-3 days'))",
        [],
    ).ok();
    let gr2_id = conn.last_insert_rowid();
    conn.execute(
        "INSERT INTO venta_detalles (venta_id, producto_id, cantidad, precio_unitario, descuento, iva_porcentaje, subtotal, precio_costo)
         VALUES (?1, 5, 10, 2.40, 0, 0, 24.00, 1.80)",
        rusqlite::params![gr2_id],
    ).ok();

    // --- Borradores (2) ---
    conn.execute(
        "INSERT INTO ventas (numero, cliente_id, subtotal_sin_iva, subtotal_con_iva, iva, descuento, total, forma_pago, monto_recibido, cambio, tipo_documento, estado, estado_sri, tipo_estado, fecha)
         VALUES ('BR-000001', 4, 8.40, 0, 0, 0, 8.40, 'EFECTIVO', 0, 0, 'BORRADOR', 'PENDIENTE', 'NO_APLICA', 'BORRADOR', datetime('now', 'localtime', '-1 hours'))",
        [],
    ).ok();
    let br_id = conn.last_insert_rowid();
    conn.execute(
        "INSERT INTO venta_detalles (venta_id, producto_id, cantidad, precio_unitario, descuento, iva_porcentaje, subtotal, precio_costo)
         VALUES (?1, 1, 4, 1.50, 0, 0, 6.00, 1.10), (?1, 2, 2, 0.90, 0, 0, 1.80, 0.65), (?1, 6, 1, 0.65, 0, 0, 0.65, 0.40)",
        rusqlite::params![br_id],
    ).ok();

    // --- Cuentas bancarias adicionales ---
    conn.execute_batch(
        "INSERT OR IGNORE INTO cuentas_banco (nombre, tipo_cuenta, numero_cuenta, titular, activa) VALUES
            ('Banco Guayaquil', 'Ahorros', '0033567890', 'Tienda El Bosque', 1),
            ('PayPhone', 'Billetera', '0991234567', 'Tienda El Bosque', 1);"
    ).ok();

    // --- Retiros de caja demo ---
    conn.execute_batch(
        "INSERT INTO retiros_caja (caja_id, monto, motivo, banco_id, referencia, usuario, estado, fecha) VALUES
            (1, 200.00, 'Deposito banco al cierre del dia', 1, 'DEP-2024001', 'Admin', 'DEPOSITADO', datetime('now', 'localtime', '-3 days')),
            (1, 50.00, 'Pago a proveedor de pan', NULL, NULL, 'Admin', 'SIN_DEPOSITO', datetime('now', 'localtime', '-2 days')),
            (1, 150.00, 'Deposito en Pichincha', 1, NULL, 'Admin', 'EN_TRANSITO', datetime('now', 'localtime', '-1 days'));"
    ).ok();

    // --- Notas de Credito demo ---
    let factura_para_nc: i64 = conn.query_row(
        "SELECT id FROM ventas WHERE tipo_documento = 'NOTA_VENTA' AND total > 5 ORDER BY id LIMIT 1",
        [], |r| r.get(0)
    ).unwrap_or(1);
    conn.execute(
        "INSERT INTO notas_credito (numero, venta_id, motivo, subtotal_sin_iva, subtotal_con_iva, iva, total, estado_sri, fecha)
         VALUES ('NC-000001', ?1, 'Devolucion - producto defectuoso', 3.00, 0, 0, 3.00, 'NO_APLICA', datetime('now', 'localtime', '-2 days'))",
        rusqlite::params![factura_para_nc],
    ).ok();

    // --- Compras adicionales ---
    conn.execute(
        "INSERT INTO compras (numero, proveedor_id, subtotal, iva, total, forma_pago, es_credito, numero_factura, observacion, fecha, estado)
         VALUES ('CMP-000002', 2, 85.20, 12.78, 97.98, 'TRANSFERENCIA', 0, 'FAC-002-1234', 'Pedido especial bebidas', datetime('now', 'localtime', '-5 days'), 'REGISTRADA')",
        [],
    ).ok();
    let cmp2 = conn.last_insert_rowid();
    conn.execute(
        "INSERT INTO compra_detalles (compra_id, producto_id, cantidad, precio_unitario, subtotal) VALUES
            (?1, 9, 30, 1.80, 54.00),
            (?1, 11, 15, 1.40, 21.00),
            (?1, 12, 12, 0.85, 10.20)",
        rusqlite::params![cmp2],
    ).ok();

    // --- Pago a cuenta por pagar (demuestra historial) ---
    conn.execute(
        "INSERT INTO pagos_proveedor (cuenta_id, monto, forma_pago, banco_id, comprobante, observacion, usuario, fecha)
         VALUES (1, 50.00, 'EFECTIVO', NULL, NULL, 'Abono parcial', 'Admin', datetime('now', 'localtime', '-1 days'))",
        [],
    ).ok();
    conn.execute("UPDATE cuentas_por_pagar SET monto_pagado = 50.00, saldo = 107.48 WHERE id = 1", []).ok();

    // --- Cuentas por Cobrar (venta a credito) ---
    let venta_credito_id: i64 = conn.query_row(
        "SELECT id FROM ventas WHERE forma_pago != 'CREDITO' ORDER BY id DESC LIMIT 1",
        [], |r| r.get(0)
    ).unwrap_or(0);
    if venta_credito_id > 0 {
        conn.execute(
            "INSERT INTO cuentas_por_cobrar (cliente_id, venta_id, monto_total, monto_pagado, saldo, estado, fecha_vencimiento)
             VALUES (1, ?1, 25.00, 0, 25.00, 'PENDIENTE', date('now', '+15 days'))",
            rusqlite::params![venta_credito_id],
        ).ok();
    }

    // --- Ordenes de Servicio Tecnico demo (5) ---
    let _ = conn.execute(
        "INSERT INTO ordenes_servicio (numero, cliente_id, cliente_nombre, cliente_telefono, tipo_equipo, equipo_descripcion, equipo_marca, equipo_modelo, equipo_serie, problema_reportado, diagnostico, tecnico_id, tecnico_nombre, estado, fecha_ingreso, fecha_promesa, presupuesto, monto_final, garantia_dias, usuario_creador)
         VALUES ('OS-000001', 3, 'Maria Lopez', '0991234567', 'TECNOLOGIA', 'Laptop HP Pavilion 15', 'HP', 'Pavilion 15-eh1xxx', 'CND2345ABC', 'No enciende, posible falla de fuente', 'Falla en cargador detectada', 3, 'Tecnico', 'EN_REPARACION', datetime('now', 'localtime', '-5 days'), datetime('now', 'localtime', '+2 days'), 45.00, 0, 30, 'Admin')",
        [],
    );
    let _ = conn.execute(
        "INSERT INTO ordenes_servicio (numero, cliente_id, cliente_nombre, cliente_telefono, tipo_equipo, equipo_descripcion, equipo_marca, equipo_modelo, equipo_serie, problema_reportado, tecnico_id, tecnico_nombre, estado, fecha_ingreso, fecha_promesa, presupuesto, garantia_dias, usuario_creador)
         VALUES ('OS-000002', 4, 'Juan Perez', '0987654321', 'TECNOLOGIA', 'Celular Samsung Galaxy A52', 'Samsung', 'A52', 'IMEI-358291098765432', 'Pantalla rota', 3, 'Tecnico', 'ESPERANDO_REPUESTOS', datetime('now', 'localtime', '-3 days'), datetime('now', 'localtime', '+5 days'), 80.00, 30, 'Admin')",
        [],
    );
    let _ = conn.execute(
        "INSERT INTO ordenes_servicio (numero, cliente_id, cliente_nombre, cliente_telefono, tipo_equipo, equipo_descripcion, equipo_marca, equipo_modelo, equipo_placa, equipo_kilometraje, equipo_kilometraje_proximo, problema_reportado, diagnostico, trabajo_realizado, tecnico_id, tecnico_nombre, estado, fecha_ingreso, fecha_entrega, monto_final, garantia_dias, usuario_creador)
         VALUES ('OS-000003', 6, 'Ana Garcia', '0998877665', 'AUTOMOTRIZ', 'Chevrolet Sail 1.4', 'Chevrolet', 'Sail', 'PBA-1234', 65000, 70000, 'Cambio de aceite y filtros', 'Aceite quemado, filtros muy sucios', 'Cambio de aceite, 4 filtros y revision general', 3, 'Tecnico', 'ENTREGADO', datetime('now', 'localtime', '-7 days'), datetime('now', 'localtime', '-5 days'), 65.00, 0, 'Admin')",
        [],
    );
    let _ = conn.execute(
        "INSERT INTO ordenes_servicio (numero, cliente_id, cliente_nombre, cliente_telefono, tipo_equipo, equipo_descripcion, equipo_marca, equipo_modelo, equipo_placa, equipo_kilometraje, problema_reportado, tecnico_id, tecnico_nombre, estado, fecha_ingreso, presupuesto, usuario_creador)
         VALUES ('OS-000004', 7, 'Carlos Vega', '0976543210', 'AUTOMOTRIZ', 'Hyundai Accent 2018', 'Hyundai', 'Accent', 'GBA-9876', 82000, 'Frenos chillan al frenar', 3, 'Tecnico', 'DIAGNOSTICANDO', datetime('now', 'localtime', '-1 days'), 50.00, 'Admin')",
        [],
    );
    let _ = conn.execute(
        "INSERT INTO ordenes_servicio (numero, cliente_nombre, cliente_telefono, tipo_equipo, equipo_descripcion, equipo_marca, problema_reportado, estado, fecha_ingreso, presupuesto, usuario_creador)
         VALUES ('OS-000005', 'Cliente Walk-in', '0995551234', 'ELECTRODOMESTICO', 'Refrigeradora Mabe 14 pies', 'Mabe', 'No enfria adecuadamente', 'RECIBIDO', datetime('now', 'localtime'), 35.00, 'Admin')",
        [],
    );

    // Movimientos para que el historial se vea poblado
    let _ = conn.execute_batch(
        "INSERT INTO ordenes_servicio_movimientos (orden_id, estado_anterior, estado_nuevo, observacion, usuario, fecha) VALUES
            (1, NULL, 'RECIBIDO', 'Equipo recibido', 'Admin', datetime('now', 'localtime', '-5 days')),
            (1, 'RECIBIDO', 'DIAGNOSTICANDO', NULL, 'Tecnico', datetime('now', 'localtime', '-4 days')),
            (1, 'DIAGNOSTICANDO', 'EN_REPARACION', 'Cargador a reemplazar', 'Tecnico', datetime('now', 'localtime', '-3 days')),
            (3, NULL, 'RECIBIDO', NULL, 'Admin', datetime('now', 'localtime', '-7 days')),
            (3, 'RECIBIDO', 'EN_REPARACION', NULL, 'Tecnico', datetime('now', 'localtime', '-7 days')),
            (3, 'EN_REPARACION', 'LISTO', 'Listo para entrega', 'Tecnico', datetime('now', 'localtime', '-5 days')),
            (3, 'LISTO', 'ENTREGADO', 'Entregado al cliente', 'Admin', datetime('now', 'localtime', '-5 days'));"
    );

    // ==========================================================================
    // BLOQUE EXPANDIDO: Datos adicionales para que TODAS las secciones tengan
    // ejemplos visibles en modo demo
    // ==========================================================================

    // --- Marcar productos que requieren serie y caducidad ---
    let _ = conn.execute(
        "UPDATE productos SET requiere_serie = 1 WHERE codigo IN ('TEC001', 'TEC002', 'TEC003')",
        [],
    );
    let _ = conn.execute(
        "UPDATE productos SET requiere_caducidad = 1
         WHERE codigo IN ('LAC001', 'LAC002', 'LAC003', 'LAC004', 'PAN001', 'PAN002', 'PAN003')",
        [],
    );
    // Servicios no controlan stock
    let _ = conn.execute(
        "UPDATE productos SET es_servicio = 1, no_controla_stock = 1 WHERE codigo IN ('SRV001', 'SRV002')",
        [],
    );

    // --- Choferes / Transportistas ---
    let _ = conn.execute_batch(
        "INSERT OR IGNORE INTO choferes (nombre, placa) VALUES
            ('Pedro Vasquez', 'PBQ-1234'),
            ('Luis Mora', 'GBC-5678'),
            ('Diego Rivera', 'PCA-7890'),
            ('Marco Naranjo', 'GBA-3344');"
    );

    // --- Establecimiento adicional + punto emisión (Multi-Almacén / Multi-POS) ---
    let est2_id: Option<i64> = {
        conn.execute(
            "INSERT OR IGNORE INTO establecimientos (codigo, nombre, direccion, telefono, es_propio, activo)
             VALUES ('002', 'Sucursal Norte', 'Av. 6 de Diciembre y Whymper, Quito', '+593 2 2456000', 1, 1)",
            [],
        ).ok();
        conn.query_row("SELECT id FROM establecimientos WHERE codigo = '002'", [], |r| r.get(0)).ok()
    };
    if let Some(eid) = est2_id {
        let _ = conn.execute(
            "INSERT OR IGNORE INTO puntos_emision (establecimiento_id, codigo, nombre, activo)
             VALUES (?1, '001', 'Caja Norte', 1)",
            rusqlite::params![eid],
        );
        // Stock dividido: 70% sucursal principal, 30% sucursal norte
        // (insertar en stock_establecimiento usando el id 1 = primera)
        let est1_id: i64 = conn.query_row(
            "SELECT id FROM establecimientos ORDER BY id LIMIT 1", [], |r| r.get(0)
        ).unwrap_or(1);
        // Repartir stock en sucursal norte (30%)
        let prods_para_repartir: Vec<(i64, f64)> = {
            let mut stmt = conn.prepare(
                "SELECT id, stock_actual FROM productos WHERE activo = 1 AND es_servicio = 0 AND stock_actual > 0"
            ).map_err(|e| e.to_string())?;
            let r = stmt.query_map([], |row| Ok((row.get::<_, i64>(0)?, row.get::<_, f64>(1)?)))
                .map_err(|e| e.to_string())?
                .filter_map(|x| x.ok())
                .collect();
            r
        };
        for (pid, stock) in &prods_para_repartir {
            let stock_norte = (stock * 0.30).round();
            let stock_principal = stock - stock_norte;
            // Asegurar fila para el establecimiento principal (puede que ya exista por migración)
            let _ = conn.execute(
                "INSERT OR REPLACE INTO stock_establecimiento (producto_id, establecimiento_id, stock_actual, stock_minimo)
                 VALUES (?1, ?2, ?3, 5)",
                rusqlite::params![pid, est1_id, stock_principal],
            );
            let _ = conn.execute(
                "INSERT OR REPLACE INTO stock_establecimiento (producto_id, establecimiento_id, stock_actual, stock_minimo)
                 VALUES (?1, ?2, ?3, 3)",
                rusqlite::params![pid, eid, stock_norte],
            );
        }
        // Una transferencia entre sucursales
        let _ = conn.execute(
            "INSERT INTO transferencias_stock (producto_id, origen_establecimiento_id, destino_establecimiento_id, cantidad, estado, usuario, created_at, recibida_at)
             VALUES (1, ?1, ?2, 10, 'RECIBIDA', 'Admin', datetime('now', 'localtime', '-2 days'), datetime('now', 'localtime', '-1 days'))",
            rusqlite::params![est1_id, eid],
        );
        let _ = conn.execute(
            "INSERT INTO transferencias_stock (producto_id, origen_establecimiento_id, destino_establecimiento_id, cantidad, estado, usuario, created_at)
             VALUES (3, ?1, ?2, 5, 'PENDIENTE', 'Admin', datetime('now', 'localtime'))",
            rusqlite::params![est1_id, eid],
        );
    }

    // --- Números de serie demo ---
    // TEC001 (Laptop) - 5 series, 1 vendida
    let laptop_id: i64 = conn.query_row("SELECT id FROM productos WHERE codigo = 'TEC001'", [], |r| r.get(0)).unwrap_or(0);
    if laptop_id > 0 {
        let series_laptop = ["HP-CND2345ABC", "HP-CND2346XYZ", "HP-CND2347MNO", "HP-CND2348PQR", "HP-CND2349STU"];
        for (i, s) in series_laptop.iter().enumerate() {
            let estado = if i == 0 { "VENDIDO" } else { "DISPONIBLE" };
            let _ = conn.execute(
                "INSERT OR IGNORE INTO numeros_serie (producto_id, serial, estado, fecha_ingreso) VALUES (?1, ?2, ?3, datetime('now','localtime', '-10 days'))",
                rusqlite::params![laptop_id, s, estado],
            );
        }
    }
    // TEC002 (Celular Samsung) - 8 series, 2 vendidas
    let cel_id: i64 = conn.query_row("SELECT id FROM productos WHERE codigo = 'TEC002'", [], |r| r.get(0)).unwrap_or(0);
    if cel_id > 0 {
        let series_cel = [
            "IMEI-358291098765432", "IMEI-358291098765433", "IMEI-358291098765434",
            "IMEI-358291098765435", "IMEI-358291098765436", "IMEI-358291098765437",
            "IMEI-358291098765438", "IMEI-358291098765439",
        ];
        for (i, s) in series_cel.iter().enumerate() {
            let estado = if i < 2 { "VENDIDO" } else { "DISPONIBLE" };
            let _ = conn.execute(
                "INSERT OR IGNORE INTO numeros_serie (producto_id, serial, estado, fecha_ingreso) VALUES (?1, ?2, ?3, datetime('now','localtime', '-7 days'))",
                rusqlite::params![cel_id, s, estado],
            );
        }
    }
    // TEC003 (Tablet) - 4 series
    let tab_id: i64 = conn.query_row("SELECT id FROM productos WHERE codigo = 'TEC003'", [], |r| r.get(0)).unwrap_or(0);
    if tab_id > 0 {
        for s in ["LEN-M10-001", "LEN-M10-002", "LEN-M10-003", "LEN-M10-004"] {
            let _ = conn.execute(
                "INSERT OR IGNORE INTO numeros_serie (producto_id, serial, estado) VALUES (?1, ?2, 'DISPONIBLE')",
                rusqlite::params![tab_id, s],
            );
        }
    }

    // --- Lotes con caducidad ---
    let lacteos_caducidad = [
        ("LAC001", "L-ECH-2024A", 30,  60.0, "Lote enero"),       // 30 dias - OK
        ("LAC001", "L-ECH-2024B", 5,   20.0, "Por vencer pronto"), // 5 dias - alerta
        ("LAC002", "L-YOG-100",   45,  80.0, ""),                 // OK
        ("LAC002", "L-YOG-101",   2,   15.0, "VENCE PRONTO"),     // 2 dias - urgente
        ("LAC003", "L-MAN-22",    90,  25.0, ""),                 // OK
        ("LAC004", "L-QSO-505",   15,  18.0, ""),                 // OK
        ("PAN001", "PYM-DIA",     1,  120.0, "Pan del dia"),      // 1 dia
        ("PAN002", "INT-340",     7,   30.0, ""),                 // 7 dias
        ("PAN003", "TC-CHO-09",   3,   12.0, ""),                 // 3 dias
    ];
    for (cod, lote, dias, qty, obs) in &lacteos_caducidad {
        let pid: i64 = conn.query_row(
            "SELECT id FROM productos WHERE codigo = ?1",
            rusqlite::params![cod], |r| r.get(0),
        ).unwrap_or(0);
        if pid > 0 {
            let fecha_cad = format!("+{} days", dias);
            let _ = conn.execute(
                "INSERT INTO lotes_caducidad (producto_id, lote, fecha_caducidad, cantidad, cantidad_inicial, observacion)
                 VALUES (?1, ?2, date('now', ?3), ?4, ?4, ?5)",
                rusqlite::params![pid, lote, fecha_cad, qty, obs],
            );
        }
    }

    // --- Movimientos de Inventario (Kardex) demo ---
    let _ = conn.execute_batch(
        "INSERT INTO movimientos_inventario (producto_id, tipo, cantidad, stock_anterior, stock_nuevo, costo_unitario, motivo, usuario, created_at) VALUES
            (1, 'INGRESO_COMPRA', 50, 30, 80, 1.10, 'Compra a Distribuidora El Sol', 'Admin', datetime('now', 'localtime', '-3 days')),
            (2, 'INGRESO_COMPRA', 40, 20, 60, 0.65, 'Compra a Distribuidora El Sol', 'Admin', datetime('now', 'localtime', '-3 days')),
            (3, 'INGRESO_COMPRA', 20, 20, 40, 2.20, 'Compra a Distribuidora El Sol', 'Admin', datetime('now', 'localtime', '-3 days')),
            (1, 'AJUSTE_NEGATIVO', -2, 80, 78, NULL, 'Producto danado', 'Admin', datetime('now', 'localtime', '-2 days')),
            (5, 'AJUSTE_POSITIVO', 5, 30, 35, NULL, 'Conteo fisico', 'Admin', datetime('now', 'localtime', '-1 days')),
            (12, 'INGRESO_COMPRA', 12, 0, 12, 0.85, 'Pedido especial', 'Admin', datetime('now', 'localtime', '-5 days'));"
    );

    // --- Más venta a crédito (genera CXC pendiente) ---
    let _ = conn.execute(
        "INSERT INTO ventas (numero, cliente_id, subtotal_sin_iva, subtotal_con_iva, iva, descuento, total, forma_pago, monto_recibido, cambio, tipo_documento, estado, estado_sri, usuario, fecha)
         VALUES ('NV-000050', 9, 32.50, 0, 0, 0, 32.50, 'CREDITO', 0, 0, 'NOTA_VENTA', 'COMPLETADA', 'NO_APLICA', 'Admin', datetime('now', 'localtime', '-2 days'))",
        [],
    );
    let venta_credito = conn.last_insert_rowid();
    let _ = conn.execute(
        "INSERT INTO venta_detalles (venta_id, producto_id, cantidad, precio_unitario, descuento, iva_porcentaje, subtotal, precio_costo)
         VALUES (?1, 1, 10, 1.50, 0, 0, 15.00, 1.10), (?1, 5, 5, 2.40, 0, 0, 12.00, 1.80), (?1, 6, 8, 0.65, 0, 0, 5.20, 0.40)",
        rusqlite::params![venta_credito],
    );
    let _ = conn.execute(
        "INSERT INTO cuentas_por_cobrar (cliente_id, venta_id, monto_total, monto_pagado, saldo, estado, fecha_vencimiento)
         VALUES (9, ?1, 32.50, 0, 32.50, 'PENDIENTE', date('now', '+15 days'))",
        rusqlite::params![venta_credito],
    );

    // Otra venta crédito ya parcialmente pagada
    let _ = conn.execute(
        "INSERT INTO ventas (numero, cliente_id, subtotal_sin_iva, subtotal_con_iva, iva, descuento, total, forma_pago, monto_recibido, cambio, tipo_documento, estado, estado_sri, usuario, fecha)
         VALUES ('NV-000051', 13, 75.00, 0, 0, 0, 75.00, 'CREDITO', 0, 0, 'NOTA_VENTA', 'COMPLETADA', 'NO_APLICA', 'Admin', datetime('now', 'localtime', '-10 days'))",
        [],
    );
    let venta_cred2 = conn.last_insert_rowid();
    let _ = conn.execute(
        "INSERT INTO venta_detalles (venta_id, producto_id, cantidad, precio_unitario, descuento, iva_porcentaje, subtotal, precio_costo)
         VALUES (?1, 3, 25, 3.00, 0, 0, 75.00, 2.20)",
        rusqlite::params![venta_cred2],
    );
    let _ = conn.execute(
        "INSERT INTO cuentas_por_cobrar (cliente_id, venta_id, monto_total, monto_pagado, saldo, estado, fecha_vencimiento)
         VALUES (13, ?1, 75.00, 30.00, 45.00, 'ABONADA', date('now', '+5 days'))",
        rusqlite::params![venta_cred2],
    );
    let cxc2_id = conn.last_insert_rowid();
    // Pago parcial registrado
    let _ = conn.execute(
        "INSERT INTO pagos_cuenta (cuenta_id, monto, forma_pago, banco_id, fecha)
         VALUES (?1, 30.00, 'EFECTIVO', NULL, datetime('now', 'localtime', '-3 days'))",
        rusqlite::params![cxc2_id],
    );

    // CXC vencida (atraso de 5 días)
    let _ = conn.execute(
        "INSERT INTO ventas (numero, cliente_id, subtotal_sin_iva, subtotal_con_iva, iva, descuento, total, forma_pago, monto_recibido, cambio, tipo_documento, estado, estado_sri, usuario, fecha)
         VALUES ('NV-000052', 6, 18.00, 0, 0, 0, 18.00, 'CREDITO', 0, 0, 'NOTA_VENTA', 'COMPLETADA', 'NO_APLICA', 'Admin', datetime('now', 'localtime', '-20 days'))",
        [],
    );
    let venta_vencida = conn.last_insert_rowid();
    let _ = conn.execute(
        "INSERT INTO venta_detalles (venta_id, producto_id, cantidad, precio_unitario, descuento, iva_porcentaje, subtotal, precio_costo)
         VALUES (?1, 9, 12, 1.50, 0, 0, 18.00, 0.95)",
        rusqlite::params![venta_vencida],
    );
    let _ = conn.execute(
        "INSERT INTO cuentas_por_cobrar (cliente_id, venta_id, monto_total, monto_pagado, saldo, estado, fecha_vencimiento)
         VALUES (6, ?1, 18.00, 0, 18.00, 'VENCIDA', date('now', '-5 days'))",
        rusqlite::params![venta_vencida],
    );

    // --- Más Notas de Crédito ---
    let _ = conn.execute(
        "INSERT INTO notas_credito (numero, venta_id, cliente_id, motivo, subtotal_sin_iva, subtotal_con_iva, iva, total, estado_sri, fecha)
         VALUES ('NC-000002', ?1, 13, 'Devolucion parcial - articulo equivocado', 6.00, 0, 0, 6.00, 'NO_APLICA', datetime('now', 'localtime', '-1 days'))",
        rusqlite::params![venta_cred2],
    );

    // --- Compras adicionales (varias formas y estados) ---
    let _ = conn.execute(
        "INSERT INTO compras (numero, proveedor_id, subtotal, iva, total, forma_pago, es_credito, numero_factura, observacion, fecha, estado)
         VALUES ('CMP-000003', 3, 120.00, 18.00, 138.00, 'CHEQUE', 1, 'FAC-003-9988', 'Pedido lacteos semanal', datetime('now', 'localtime', '-7 days'), 'REGISTRADA')",
        [],
    );
    let cmp3 = conn.last_insert_rowid();
    let _ = conn.execute(
        "INSERT INTO compra_detalles (compra_id, producto_id, cantidad, precio_unitario, subtotal) VALUES
            (?1, 25, 60, 0.85, 51.00),
            (?1, 26, 80, 0.45, 36.00),
            (?1, 27, 25, 1.40, 35.00)",
        rusqlite::params![cmp3],
    );
    let _ = conn.execute(
        "INSERT INTO cuentas_por_pagar (compra_id, proveedor_id, monto_total, monto_pagado, saldo, estado, fecha_vencimiento)
         VALUES (?1, 3, 138.00, 0, 138.00, 'PENDIENTE', date('now', '+38 days'))",
        rusqlite::params![cmp3],
    );

    // Compra de tecnologia (con series)
    let _ = conn.execute(
        "INSERT INTO compras (numero, proveedor_id, subtotal, iva, total, forma_pago, es_credito, numero_factura, observacion, fecha, estado)
         VALUES ('CMP-000004', 2, 2400.00, 360.00, 2760.00, 'TRANSFERENCIA', 0, 'FAC-002-1500', 'Stock tecnologia', datetime('now', 'localtime', '-10 days'), 'REGISTRADA')",
        [],
    );
    let cmp4 = conn.last_insert_rowid();
    let _ = conn.execute(
        "INSERT INTO compra_detalles (compra_id, producto_id, cantidad, precio_unitario, subtotal) VALUES
            (?1, 21, 5, 480.00, 2400.00)",
        rusqlite::params![cmp4],
    );

    // CXP totalmente pagada (historial)
    let _ = conn.execute(
        "INSERT INTO compras (numero, proveedor_id, subtotal, iva, total, forma_pago, es_credito, numero_factura, observacion, fecha, estado)
         VALUES ('CMP-000005', 1, 60.00, 9.00, 69.00, 'EFECTIVO', 1, 'FAC-001-5500', 'Reposicion limpieza', datetime('now', 'localtime', '-15 days'), 'REGISTRADA')",
        [],
    );
    let cmp5 = conn.last_insert_rowid();
    let _ = conn.execute(
        "INSERT INTO compra_detalles (compra_id, producto_id, cantidad, precio_unitario, subtotal) VALUES (?1, 33, 30, 2.00, 60.00)",
        rusqlite::params![cmp5],
    );
    let _ = conn.execute(
        "INSERT INTO cuentas_por_pagar (compra_id, proveedor_id, monto_total, monto_pagado, saldo, estado, fecha_vencimiento)
         VALUES (?1, 1, 69.00, 69.00, 0, 'PAGADA', date('now', '-2 days'))",
        rusqlite::params![cmp5],
    );
    let cxp_pagada = conn.last_insert_rowid();
    let _ = conn.execute(
        "INSERT INTO pagos_proveedor (cuenta_id, monto, forma_pago, banco_id, observacion, fecha)
         VALUES (?1, 69.00, 'TRANSFERENCIA', 1, 'Pago completo', datetime('now', 'localtime', '-2 days'))",
        rusqlite::params![cxp_pagada],
    );

    // --- Email log demo (cola de envíos) ---
    let venta_para_mail: i64 = conn.query_row(
        "SELECT id FROM ventas WHERE tipo_documento = 'NOTA_VENTA' ORDER BY id DESC LIMIT 1",
        [], |r| r.get(0),
    ).unwrap_or(1);
    let _ = conn.execute_batch(&format!(
        "INSERT INTO email_log (venta_id, email, estado, intentos, enviado_at, created_at) VALUES
            ({0}, 'cliente1@email.com', 'ENVIADO', 1, datetime('now','localtime','-1 days'), datetime('now','localtime','-1 days'));
         INSERT INTO email_log (venta_id, email, estado, intentos, ultimo_error, created_at) VALUES
            ({0}, 'cliente2@error.com', 'FALLIDO', 3, 'Email rebotado', datetime('now','localtime','-2 hours'));
         INSERT INTO email_log (venta_id, email, estado, intentos, created_at) VALUES
            ({0}, 'cliente3@email.com', 'PENDIENTE', 0, datetime('now','localtime'));",
        venta_para_mail
    ));

    // --- Orden de Servicio entregada y cobrada (con venta vinculada) ---
    let _ = conn.execute(
        "INSERT INTO ventas (numero, cliente_id, subtotal_sin_iva, subtotal_con_iva, iva, descuento, total, forma_pago, monto_recibido, cambio, tipo_documento, estado, estado_sri, usuario, fecha, observacion)
         VALUES ('NV-000060', 6, 0, 65.00, 9.75, 0, 65.00, 'EFECTIVO', 65.00, 0, 'NOTA_VENTA', 'COMPLETADA', 'NO_APLICA', 'Admin', datetime('now', 'localtime', '-5 days'), 'Cobro orden OS-000003 - Cambio de aceite Sail')",
        [],
    );
    let venta_os = conn.last_insert_rowid();
    let _ = conn.execute(
        "UPDATE ordenes_servicio SET venta_id = ?1, monto_final = 65.00 WHERE numero = 'OS-000003'",
        rusqlite::params![venta_os],
    );

    // Una orden mas: ELECTRODOMESTICO LISTA (para mostrar 6 estados)
    let _ = conn.execute(
        "INSERT INTO ordenes_servicio (numero, cliente_id, cliente_nombre, cliente_telefono, tipo_equipo, equipo_descripcion, equipo_marca, equipo_modelo, equipo_serie, problema_reportado, diagnostico, trabajo_realizado, tecnico_id, tecnico_nombre, estado, fecha_ingreso, presupuesto, monto_final, garantia_dias, usuario_creador)
         VALUES ('OS-000006', 11, 'Lucia Paredes', '0998120000', 'GENERAL', 'Bicicleta Gw Hyena', 'GW', 'Hyena 29\"', 'GW-HY-3344', 'Cambio de cadena y ajuste frenos', 'Cadena rota, frenos descalibrados', 'Cambio cadena, ajuste frenos delanteros y traseros', 3, 'Tecnico', 'LISTO', datetime('now', 'localtime', '-2 days'), 25.00, 25.00, 15, 'Admin')",
        [],
    );

    // --- Conteo final ventas demo: completar resumen del dia ---
    // (las primeras ventas ya se generaron, dejamos esto como referencia)

    // Activar modulos en config para que se vean
    let _ = conn.execute_batch(
        "UPDATE config SET value = '1' WHERE key = 'modulo_servicio_tecnico';
         UPDATE config SET value = '1' WHERE key = 'modulo_caducidad';
         UPDATE config SET value = '1' WHERE key = 'modulo_series_activo';
         UPDATE config SET value = '1' WHERE key = 'multi_almacen_activo';"
    );

    let machine_id = crate::commands::licencia::obtener_machine_id().unwrap_or_default();

    Ok(LicenciaInfo {
        negocio: "Tienda El Bosque".to_string(),
        email: "demo@clouget.com".to_string(),
        tipo: "demo".to_string(),
        emitida: chrono::Local::now().format("%Y-%m-%d").to_string(),
        machine_id,
        activa: true,
        modulos: vec![
            "multi_pos".to_string(),
            "multi_almacen".to_string(),
            "backup_cloud".to_string(),
            "backup_premium".to_string(),
            "series".to_string(),
            "servicio_tecnico".to_string(),
            "sri_ilimitado".to_string(),
        ],
    })
}

/// Sale del modo demo: limpia todos los datos y resetea la configuración.
#[tauri::command]
pub fn salir_demo(db: State<Database>) -> Result<(), String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;

    let demo = conn.query_row(
        "SELECT value FROM config WHERE key = 'demo_activo'",
        [],
        |row| row.get::<_, String>(0),
    ).unwrap_or_default();

    if demo != "1" {
        return Err("El modo demo no está activo".to_string());
    }

    // Borrar todos los datos de usuario - desactivar FKs temporalmente para evitar errores de orden
    conn.execute("PRAGMA foreign_keys = OFF", []).ok();

    let tablas_borrar = [
        "DELETE FROM ordenes_servicio_imagenes",
        "DELETE FROM ordenes_servicio_movimientos",
        "DELETE FROM ordenes_servicio",
        "DELETE FROM lotes_caducidad",
        "DELETE FROM numeros_serie",
        "DELETE FROM presentaciones_producto",
        "DELETE FROM pagos_proveedor",
        "DELETE FROM cuentas_por_pagar",
        "DELETE FROM compra_detalles",
        "DELETE FROM compras",
        "DELETE FROM proveedores",
        "DELETE FROM retiros_caja",
        "DELETE FROM choferes",
        "DELETE FROM nota_credito_detalles",
        "DELETE FROM notas_credito",
        "DELETE FROM venta_detalles",
        "DELETE FROM ventas",
        "DELETE FROM movimientos_inventario",
        "DELETE FROM transferencias_stock",
        "DELETE FROM stock_establecimiento",
        "DELETE FROM precios_producto",
        "DELETE FROM productos",
        "DELETE FROM categorias",
        "DELETE FROM clientes WHERE id != 1",
        "DELETE FROM caja",
        "DELETE FROM gastos",
        "DELETE FROM usuarios",
        "DELETE FROM cuentas_por_cobrar",
        "DELETE FROM pagos_cuenta",
        "DELETE FROM cuentas_banco",
        "DELETE FROM email_log",
        "DELETE FROM puntos_emision WHERE establecimiento_id != 1",
        "DELETE FROM establecimientos WHERE id != 1",
        "DELETE FROM secuenciales WHERE tipo_documento NOT IN ('NOTA_VENTA','FACTURA','FACTURA_PRUEBAS','NOTA_CREDITO','NOTA_CREDITO_PRUEBAS')",
        "DELETE FROM listas_precios WHERE es_default = 0",
        "DELETE FROM tipos_unidad WHERE id > 6",
    ];
    let mut errores: Vec<String> = Vec::new();
    for sql in tablas_borrar.iter() {
        if let Err(e) = conn.execute(sql, []) {
            // Solo registrar errores que no sean "no such table"
            let es = e.to_string();
            if !es.contains("no such table") {
                errores.push(format!("{}: {}", sql, es));
            }
        }
    }
    conn.execute("PRAGMA foreign_keys = ON", []).ok();

    if !errores.is_empty() {
        eprintln!("Advertencias en salir_demo: {:?}", errores);
    }

    // Resetear config
    let resets = [
        ("nombre_negocio", "Mi Negocio"),
        ("ruc", ""),
        ("direccion", ""),
        ("telefono", ""),
        ("regimen", "RIMPE_POPULAR"),
        ("sri_modulo_activo", "0"),
        ("sri_certificado_cargado", "0"),
        ("sri_facturas_gratis", "30"),
        ("sri_facturas_usadas", "0"),
        ("sri_ambiente", "pruebas"),
        ("demo_activo", "0"),
        ("secuencial_compra", "0"),
        ("licencia_activada", "0"),
        ("licencia_codigo", ""),
        ("licencia_negocio", ""),
        ("licencia_email", ""),
        ("licencia_tipo", ""),
        ("modulo_servicio_tecnico", "0"),
        ("modulo_caducidad", "0"),
        ("modulo_series_activo", "0"),
        ("multi_almacen_activo", "0"),
    ];

    for (key, value) in &resets {
        conn.execute(
            "UPDATE config SET value = ?2 WHERE key = ?1",
            rusqlite::params![key, value],
        ).ok();
    }

    // Limpiar sesión persistida (ya que se borraron los usuarios)
    conn.execute("DELETE FROM config WHERE key = 'sesion_activa'", []).ok();

    // Forzar demo_activo = 0
    conn.execute("UPDATE config SET value = '0' WHERE key = 'demo_activo'", []).ok();
    conn.execute("INSERT OR REPLACE INTO config (key, value) VALUES ('demo_activo', '0')", []).ok();

    Ok(())
}

/// Consulta si el modo demo está activo.
#[tauri::command]
pub fn es_demo(db: State<Database>) -> Result<bool, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let demo = conn.query_row(
        "SELECT value FROM config WHERE key = 'demo_activo'",
        [],
        |row| row.get::<_, String>(0),
    ).unwrap_or_default();
    Ok(demo == "1")
}
