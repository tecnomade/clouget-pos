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
    ];
    for (key, value) in &configs {
        conn.execute(
            "INSERT OR REPLACE INTO config (key, value) VALUES (?1, ?2)",
            rusqlite::params![key, value],
        )
        .map_err(|e| format!("Error configurando demo: {}", e))?;
    }

    // --- Categorías ---
    conn.execute_batch(
        "INSERT OR IGNORE INTO categorias (id, nombre, descripcion, activo) VALUES
            (1, 'Abarrotes', 'Productos de primera necesidad', 1),
            (2, 'Bebidas', 'Bebidas y refrescos', 1),
            (3, 'Higiene Personal', 'Productos de aseo y cuidado personal', 1),
            (4, 'Congelados', 'Carnes y productos congelados', 1);",
    )
    .map_err(|e| format!("Error creando categorías: {}", e))?;

    // --- Productos (12) ---
    let productos = [
        // (codigo, nombre, categoria_id, precio_costo, precio_venta, iva%, stock, unidad)
        ("ABR001", "Arroz Tipo 1 (500g)", 1, 1.10, 1.50, 0.0, 80.0, "UND"),
        ("ABR002", "Fideos Spaghetti (400g)", 1, 0.65, 0.90, 0.0, 60.0, "UND"),
        ("ABR003", "Aceite Vegetal (1L)", 1, 2.20, 3.00, 0.0, 40.0, "UND"),
        ("ABR004", "Harina de Trigo (1kg)", 1, 0.90, 1.35, 0.0, 50.0, "UND"),
        ("BEB001", "Agua Mineral 6-Pack", 2, 1.80, 2.50, 0.0, 45.0, "UND"),
        ("BEB002", "Jugo Natural Naranja (1L)", 2, 1.50, 2.25, 15.0, 30.0, "UND"),
        ("BEB003", "Refresco Cola (2L)", 2, 1.40, 2.00, 15.0, 55.0, "UND"),
        ("HIG001", "Jabon de Bano (3-pack)", 3, 1.00, 1.75, 15.0, 70.0, "UND"),
        ("HIG002", "Pasta Dental (75ml)", 3, 1.20, 1.95, 15.0, 50.0, "UND"),
        ("HIG003", "Shampoo (250ml)", 3, 2.50, 3.80, 15.0, 35.0, "UND"),
        ("CON001", "Pollo Entero", 4, 3.50, 5.50, 0.0, 20.0, "KG"),
        ("CON002", "Carne de Res", 4, 5.80, 8.50, 0.0, 15.0, "KG"),
    ];

    for (codigo, nombre, cat_id, costo, venta, iva, stock, unidad) in &productos {
        conn.execute(
            "INSERT OR IGNORE INTO productos (codigo, nombre, categoria_id, precio_costo, precio_venta, iva_porcentaje, incluye_iva, stock_actual, stock_minimo, unidad_medida, activo)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, 0, ?7, 5, ?8, 1)",
            rusqlite::params![codigo, nombre, cat_id, costo, venta, iva, stock, unidad],
        )
        .map_err(|e| format!("Error creando producto: {}", e))?;
    }

    // --- Clientes (3) ---
    conn.execute(
        "INSERT OR IGNORE INTO clientes (tipo_identificacion, identificacion, nombre, direccion, telefono)
         VALUES ('RUC', '0992345678001', 'Restaurante Don Jorge', 'Av. 9 de Octubre 456, Guayaquil', '+593 4 2567890')",
        [],
    ).map_err(|e| format!("Error creando cliente: {}", e))?;

    conn.execute(
        "INSERT OR IGNORE INTO clientes (tipo_identificacion, identificacion, nombre, direccion, telefono)
         VALUES ('RUC', '0987654321001', 'Tienda La Esquina', 'Calle Sucre 789, Cuenca', '+593 7 2891234')",
        [],
    ).map_err(|e| format!("Error creando cliente: {}", e))?;

    conn.execute(
        "INSERT OR IGNORE INTO clientes (tipo_identificacion, identificacion, nombre, direccion, email)
         VALUES ('CEDULA', '0912345678', 'Maria Lopez', 'Calle Bolivar 321, Quito', 'maria@email.com')",
        [],
    ).map_err(|e| format!("Error creando cliente: {}", e))?;

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

    // Borrar todos los datos de usuario
    conn.execute_batch(
        "DELETE FROM nota_credito_detalles;
         DELETE FROM notas_credito;
         DELETE FROM venta_detalles;
         DELETE FROM ventas;
         DELETE FROM movimientos_inventario;
         DELETE FROM precios_producto;
         DELETE FROM productos;
         DELETE FROM categorias;
         DELETE FROM clientes WHERE id != 1;
         DELETE FROM caja;
         DELETE FROM gastos;
         DELETE FROM usuarios;
         DELETE FROM cuentas_por_cobrar;
         DELETE FROM pagos_cuenta;
         DELETE FROM cuentas_banco;
         DELETE FROM email_log;
         DELETE FROM listas_precios WHERE es_default = 0;",
    ).map_err(|e| format!("Error limpiando datos demo: {}", e))?;

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
        ("licencia_activada", "0"),
        ("licencia_codigo", ""),
        ("licencia_negocio", ""),
        ("licencia_email", ""),
        ("licencia_tipo", ""),
    ];

    for (key, value) in &resets {
        conn.execute(
            "UPDATE config SET value = ?2 WHERE key = ?1",
            rusqlite::params![key, value],
        ).ok();
    }

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
