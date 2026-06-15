//! Smoke tests — verifican que los cambios (presentaciones de compra, cierre de
//! caja con facturas PENDIENTE, Nota de Entrega) NO rompieron comportamiento
//! existente y SÍ funcionan como se diseñó.
//!
//! Cada test corre contra una BD SQLite en memoria efímera. Ejecutar con:
//!
//!   cargo test --test smoke_test --release

use clouget_pos_lib::commands::caja::calcular_monto_esperado_actual;
use clouget_pos_lib::db::schema;
use rusqlite::{params, Connection};

/// Setup helper — BD nueva en memoria, schema completo + migraciones idempotentes.
fn setup_db() -> Connection {
    let conn = Connection::open_in_memory().expect("open in-memory db");
    conn.execute_batch(
        "PRAGMA journal_mode = WAL;
         PRAGMA synchronous = NORMAL;
         PRAGMA foreign_keys = ON;",
    )
    .ok();
    schema::create_tables(&conn).expect("create_tables");
    // Migraciones de db/mod.rs::run_migrations — idempotentes (.ok()).
    conn.execute("ALTER TABLE compra_detalles ADD COLUMN presentacion_id INTEGER", []).ok();
    conn.execute("ALTER TABLE compra_detalles ADD COLUMN presentacion_nombre TEXT", []).ok();
    conn.execute("ALTER TABLE compra_detalles ADD COLUMN presentacion_factor REAL", []).ok();
    conn.execute("ALTER TABLE compra_detalles ADD COLUMN cantidad_presentacion REAL", []).ok();
    conn.execute("ALTER TABLE venta_detalles ADD COLUMN presentacion_id INTEGER", []).ok();
    conn.execute("ALTER TABLE venta_detalles ADD COLUMN presentacion_nombre TEXT", []).ok();
    conn.execute("ALTER TABLE venta_detalles ADD COLUMN presentacion_factor REAL", []).ok();
    conn.execute("ALTER TABLE venta_detalles ADD COLUMN cantidad_presentacion REAL", []).ok();
    conn.execute("ALTER TABLE venta_detalles ADD COLUMN lote_numero TEXT", []).ok();
    conn.execute("ALTER TABLE venta_detalles ADD COLUMN lote_fecha_caducidad TEXT", []).ok();
    conn
}

/// Abre una caja de prueba con monto inicial dado. Retorna el id.
fn abrir_caja(conn: &Connection, monto_inicial: f64) -> i64 {
    conn.execute(
        "INSERT INTO caja (monto_inicial, monto_esperado, estado, usuario)
         VALUES (?1, ?1, 'ABIERTA', 'tester')",
        params![monto_inicial],
    )
    .unwrap();
    conn.last_insert_rowid()
}

/// Inserta una venta de prueba. tipo_estado=None → venta normal;
/// tipo_estado=Some("GUIA_REMISION") → Nota de Entrega.
fn insertar_venta(
    conn: &Connection,
    numero: &str,
    forma_pago: &str,
    total: f64,
    estado: &str,
    tipo_estado: Option<&str>,
) -> i64 {
    let te = tipo_estado.unwrap_or("COMPLETADA");
    conn.execute(
        "INSERT INTO ventas (numero, total, forma_pago, estado, tipo_documento, tipo_estado)
         VALUES (?1, ?2, ?3, ?4, 'NOTA_VENTA', ?5)",
        params![numero, total, forma_pago, estado, te],
    )
    .unwrap();
    conn.last_insert_rowid()
}

// ── 1) CIERRE DE CAJA ───────────────────────────────────────────────────────

#[test]
fn cierre_caja_solo_inicial_sin_ventas() {
    let conn = setup_db();
    let caja_id = abrir_caja(&conn, 100.0);
    let esperado = calcular_monto_esperado_actual(&conn, caja_id);
    assert!((esperado - 100.0).abs() < 0.001, "Sin ventas: solo el inicial. Got {}", esperado);
}

#[test]
fn cierre_caja_cuenta_factura_completada() {
    let conn = setup_db();
    let caja_id = abrir_caja(&conn, 50.0);
    insertar_venta(&conn, "V-001", "EFECTIVO", 25.0, "COMPLETADA", None);
    let esperado = calcular_monto_esperado_actual(&conn, caja_id);
    assert!((esperado - 75.0).abs() < 0.001, "Factura COMPLETADA efectivo debe sumar. Got {}", esperado);
}

#[test]
fn cierre_caja_cuenta_factura_pendiente_sri() {
    // El bug que arreglamos: facturas PENDIENTE (SRI) tienen el efectivo en caja YA.
    let conn = setup_db();
    let caja_id = abrir_caja(&conn, 50.0);
    insertar_venta(&conn, "V-001", "EFECTIVO", 25.0, "PENDIENTE", None);
    let esperado = calcular_monto_esperado_actual(&conn, caja_id);
    assert!((esperado - 75.0).abs() < 0.001, "Factura PENDIENTE efectivo TAMBIEN debe sumar. Got {}", esperado);
}

#[test]
fn cierre_caja_excluye_notas_de_entrega() {
    let conn = setup_db();
    let caja_id = abrir_caja(&conn, 50.0);
    insertar_venta(&conn, "NE-001", "EFECTIVO", 100.0, "PENDIENTE", Some("GUIA_REMISION"));
    let esperado = calcular_monto_esperado_actual(&conn, caja_id);
    assert!((esperado - 50.0).abs() < 0.001, "Notas de Entrega NO cuentan. Got {}", esperado);
}

#[test]
fn cierre_caja_transferencia_no_suma_efectivo() {
    let conn = setup_db();
    let caja_id = abrir_caja(&conn, 50.0);
    insertar_venta(&conn, "V-001", "TRANSFERENCIA", 75.0, "COMPLETADA", None);
    let esperado = calcular_monto_esperado_actual(&conn, caja_id);
    assert!((esperado - 50.0).abs() < 0.001, "Transferencia NO entra a caja fisica. Got {}", esperado);
}

#[test]
fn cierre_caja_descuenta_retiros_y_gastos() {
    let conn = setup_db();
    let caja_id = abrir_caja(&conn, 100.0);
    insertar_venta(&conn, "V-001", "EFECTIVO", 50.0, "COMPLETADA", None);
    conn.execute(
        "INSERT INTO retiros_caja (caja_id, monto, motivo, usuario, estado, fecha)
         VALUES (?1, 30.0, 'Test', 'tester', 'SIN_DEPOSITO', datetime('now','localtime'))",
        params![caja_id],
    ).unwrap();
    conn.execute(
        "INSERT INTO gastos (descripcion, monto, categoria, caja_id, observacion, es_recurrente)
         VALUES ('Test', 10.0, 'Otros', ?1, NULL, 0)",
        params![caja_id],
    ).unwrap();
    let esperado = calcular_monto_esperado_actual(&conn, caja_id);
    assert!((esperado - 110.0).abs() < 0.001, "Esperaba 110 (100+50-30-10), got {}", esperado);
}

// ── 2) PRESENTACIONES DE COMPRA ─────────────────────────────────────────────

fn seed_producto(conn: &Connection, nombre: &str) -> i64 {
    conn.execute(
        "INSERT OR IGNORE INTO categorias (id, nombre, descripcion, activo) VALUES (1, 'Test', '', 1)",
        [],
    ).unwrap();
    conn.execute(
        "INSERT INTO productos (codigo, nombre, categoria_id, precio_costo, precio_venta, stock_actual, activo)
         VALUES (?1, ?2, 1, 1.0, 2.0, 0.0, 1)",
        params![format!("P-{}", nombre), nombre],
    ).unwrap();
    conn.last_insert_rowid()
}

#[test]
fn presentaciones_tabla_existe_y_acepta_inserts() {
    let conn = setup_db();
    let prod_id = seed_producto(&conn, "Cerveza");
    conn.execute(
        "INSERT INTO producto_presentaciones (producto_id, nombre, factor, activo, orden)
         VALUES (?1, 'Jaba x12', 12.0, 1, 0)",
        params![prod_id],
    ).expect("insertar presentación");
    let count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM producto_presentaciones WHERE producto_id = ?1",
        params![prod_id], |r| r.get(0),
    ).unwrap();
    assert_eq!(count, 1, "Debe haber 1 presentación creada");
}

#[test]
fn presentaciones_cascade_delete_al_borrar_producto() {
    let conn = setup_db();
    let prod_id = seed_producto(&conn, "Cerveza");
    conn.execute(
        "INSERT INTO producto_presentaciones (producto_id, nombre, factor) VALUES (?1, 'Jaba x12', 12.0)",
        params![prod_id],
    ).unwrap();
    conn.execute("DELETE FROM productos WHERE id = ?1", params![prod_id]).unwrap();
    let count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM producto_presentaciones", [], |r| r.get(0),
    ).unwrap();
    assert_eq!(count, 0, "ON DELETE CASCADE debe borrar las presentaciones del producto");
}

// ── 3) COMPRA_DETALLES — snapshot ───────────────────────────────────────────

#[test]
fn compra_detalles_acepta_snapshot_de_presentacion() {
    let conn = setup_db();
    conn.execute("INSERT INTO proveedores (id, nombre) VALUES (1, 'Test Prov')", []).unwrap();
    conn.execute(
        "INSERT INTO compras (numero, proveedor_id, total, subtotal, iva, forma_pago, es_credito)
         VALUES ('C-001', 1, 24.0, 24.0, 0.0, 'EFECTIVO', 0)",
        [],
    ).unwrap();
    let compra_id = conn.last_insert_rowid();
    let prod_id = seed_producto(&conn, "Cerveza");
    conn.execute(
        "INSERT INTO compra_detalles
         (compra_id, producto_id, descripcion, cantidad, precio_unitario, subtotal,
          presentacion_id, presentacion_nombre, presentacion_factor, cantidad_presentacion)
         VALUES (?1, ?2, 'Cerveza', 24.0, 1.0, 24.0, 999, 'Jaba x12', 12.0, 2.0)",
        params![compra_id, prod_id],
    ).expect("insertar detalle con snapshot");
    let (nombre, factor, cant_pres): (String, f64, f64) = conn.query_row(
        "SELECT presentacion_nombre, presentacion_factor, cantidad_presentacion
         FROM compra_detalles WHERE compra_id = ?1",
        params![compra_id], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
    ).unwrap();
    assert_eq!(nombre, "Jaba x12");
    assert_eq!(factor, 12.0);
    assert_eq!(cant_pres, 2.0);
}

// ── 4) VENTA_DETALLES — snapshot (Nota de Entrega) ──────────────────────────

#[test]
fn venta_detalles_acepta_snapshot_de_presentacion() {
    let conn = setup_db();
    let prod_id = seed_producto(&conn, "Cerveza");
    let venta_id = insertar_venta(&conn, "NE-001", "EFECTIVO", 24.0, "PENDIENTE", Some("GUIA_REMISION"));
    conn.execute(
        "INSERT INTO venta_detalles
         (venta_id, producto_id, cantidad, precio_unitario, subtotal,
          presentacion_id, presentacion_nombre, presentacion_factor, cantidad_presentacion)
         VALUES (?1, ?2, 24.0, 1.0, 24.0, 999, 'Jaba x12', 12.0, 2.0)",
        params![venta_id, prod_id],
    ).expect("insertar detalle NE con snapshot");
    let (cant, nombre, factor, cant_pres): (f64, String, f64, f64) = conn.query_row(
        "SELECT cantidad, presentacion_nombre, presentacion_factor, cantidad_presentacion
         FROM venta_detalles WHERE venta_id = ?1",
        params![venta_id], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
    ).unwrap();
    assert_eq!(cant, 24.0, "cantidad en unidad base = 2 * 12");
    assert_eq!(nombre, "Jaba x12");
    assert_eq!(factor, 12.0);
    assert_eq!(cant_pres, 2.0);
}

// ── 5) MIGRACIÓN IDEMPOTENTE ────────────────────────────────────────────────

#[test]
fn migraciones_son_idempotentes() {
    let conn = setup_db();
    let result1 = conn.execute("ALTER TABLE compra_detalles ADD COLUMN presentacion_id INTEGER", []);
    let result2 = conn.execute("ALTER TABLE venta_detalles ADD COLUMN presentacion_id INTEGER", []);
    assert!(result1.is_err(), "Segundo ALTER en compra_detalles debe fallar (columna ya existe)");
    assert!(result2.is_err(), "Segundo ALTER en venta_detalles debe fallar (columna ya existe)");
    let count: i64 = conn.query_row("SELECT COUNT(*) FROM compra_detalles", [], |r| r.get(0))
        .expect("compra_detalles sigue funcional");
    assert_eq!(count, 0);
}

// ── 6) AJUSTE DE STOCK CON PRESENTACIÓN ─────────────────────────────────────

#[test]
fn ajuste_stock_con_presentacion_calcula_unidades_base() {
    let conn = setup_db();
    let prod_id = seed_producto(&conn, "Cerveza");
    conn.execute(
        "INSERT INTO producto_presentaciones (producto_id, nombre, factor) VALUES (?1, 'Jaba x12', 12.0)",
        params![prod_id],
    ).unwrap();
    let cant_pres = 2.0_f64;
    let factor = 12.0_f64;
    let stock_nuevo = cant_pres * factor;
    conn.execute("UPDATE productos SET stock_actual = ?1 WHERE id = ?2", params![stock_nuevo, prod_id]).unwrap();
    let stock_persistido: f64 = conn.query_row(
        "SELECT stock_actual FROM productos WHERE id = ?1", params![prod_id], |r| r.get(0),
    ).unwrap();
    assert_eq!(stock_persistido, 24.0, "Ajuste de '2 jabas x12' debe persistir como 24 unidades base");
}

// ── 7) EXPORT PDF — celdas con palabras largas no deben perderse ────────────

#[test]
fn exportar_tabla_pdf_no_pierde_celdas_largas() {
    // Bug real: genpdf descartaba palabras más anchas que la columna, dejando
    // vacías las celdas "CONSUMIDOR FINAL" y "9999999999999" del reporte de ventas.
    let ruta = std::env::temp_dir().join("test_reporte_consumidor_final.pdf");
    let encabezados: Vec<String> = vec![
        "Fecha", "Número", "Cliente", "Identif.", "Cajero",
        "Forma pago", "Tipo doc.", "Subtotal", "IVA", "Descuento", "Total", "Estado",
    ].into_iter().map(String::from).collect();
    let filas: Vec<Vec<String>> = vec![
        vec!["2026-06-10 19:30", "NV-000000158", "CONSUMIDOR FINAL", "9999999999999",
             "ISRAEL", "TRANSFERENCIA", "NOTA_VENTA", "3.00", "0.00", "0.00", "3.00", "COMPLETADA"]
            .into_iter().map(String::from).collect(),
    ];
    clouget_pos_lib::commands::exportar::exportar_tabla_pdf(
        ruta.to_string_lossy().to_string(),
        "Reporte de Ventas".to_string(),
        Some("Test consumidor final".to_string()),
        encabezados,
        filas,
        Some(true),
    ).expect("el PDF debe generarse sin error");
    let bytes = std::fs::metadata(&ruta).expect("PDF debe existir").len();
    assert!(bytes > 1000, "PDF sospechosamente pequeño: {} bytes", bytes);
}

// ── 8) ESCRITURA ROBUSTA DE PDF — nombre único anti os error 32 ─────────────

#[test]
fn escribir_pdf_robusto_genera_nombres_unicos() {
    // El fix del "os error 32": dos generaciones del MISMO comprobante deben
    // escribir archivos DISTINTOS (no pisar uno que un visor tenga abierto).
    let dir = std::env::temp_dir();
    let bytes = b"%PDF-1.4 contenido de prueba";
    let p1 = clouget_pos_lib::utils::escribir_pdf_robusto(&dir, "RIDE", "001-001-000000003", bytes)
        .expect("primer write");
    let p2 = clouget_pos_lib::utils::escribir_pdf_robusto(&dir, "RIDE", "001-001-000000003", bytes)
        .expect("segundo write del mismo numero");
    assert_ne!(p1, p2, "Dos RIDE del mismo numero deben tener rutas distintas (uuid)");
    assert!(p1.exists() && p2.exists(), "Ambos PDF deben existir en disco");
    assert!(p1.file_name().unwrap().to_string_lossy().starts_with("RIDE-001-001-000000003-"));
    assert_eq!(std::fs::read(&p1).unwrap(), bytes, "El contenido debe ser exacto");
    // No debe quedar el .tmp intermedio
    let tmp = dir.join(format!(".{}.tmp", p1.file_name().unwrap().to_string_lossy()));
    assert!(!tmp.exists(), "El .tmp intermedio no debe quedar tras el rename");
    let _ = std::fs::remove_file(&p1);
    let _ = std::fs::remove_file(&p2);
}

#[test]
fn escribir_pdf_robusto_sanea_caracteres_invalidos() {
    let dir = std::env::temp_dir();
    let p = clouget_pos_lib::utils::escribir_pdf_robusto(&dir, "RIDE-NC", "001/001:0003", b"x")
        .expect("write con caracteres invalidos en numero");
    let name = p.file_name().unwrap().to_string_lossy();
    assert!(!name.contains('/') && !name.contains(':'), "Caracteres invalidos saneados: {}", name);
    let _ = std::fs::remove_file(&p);
}

// ── 9) TRAZABILIDAD DE LOTE → CLIENTE (recall farmacéutico) ─────────────────

/// Reproduce la query de clientes_por_lote para verificar la cadena de recall
/// lote → venta_detalle → venta → cliente, incluyendo que el SNAPSHOT del lote
/// sobreviva al borrado del lote del inventario.
fn recall_buscar(conn: &Connection, termino: &str) -> Vec<(String, Option<String>, Option<String>)> {
    let mut stmt = conn.prepare(
        "SELECT COALESCE(NULLIF(c.nombre,''), 'CONSUMIDOR FINAL') as cliente_nombre,
                c.telefono,
                COALESCE(d.lote_numero, l.lote) as lote
         FROM venta_detalles d
         JOIN ventas v ON d.venta_id = v.id
         LEFT JOIN clientes c ON v.cliente_id = c.id
         LEFT JOIN lotes_caducidad l ON d.lote_id = l.id
         WHERE COALESCE(d.lote_numero, l.lote) LIKE ?1
           AND COALESCE(v.anulada, 0) = 0",
    ).unwrap();
    let busq = format!("%{}%", termino.trim());
    stmt.query_map(params![busq], |r| {
        Ok((r.get::<_, String>(0)?, r.get::<_, Option<String>>(1)?, r.get::<_, Option<String>>(2)?))
    }).unwrap().collect::<Result<Vec<_>, _>>().unwrap()
}

#[test]
fn recall_encuentra_cliente_por_lote_y_sobrevive_borrado() {
    let conn = setup_db();
    let prod_id = seed_producto(&conn, "Paracetamol");
    // Cliente real
    conn.execute(
        "INSERT INTO clientes (id, tipo_identificacion, identificacion, nombre, telefono)
         VALUES (50, 'CEDULA', '0925479925', 'Lucia Bueno', '0991112233')",
        [],
    ).unwrap();
    // Lote con caducidad
    conn.execute(
        "INSERT INTO lotes_caducidad (id, producto_id, lote, fecha_caducidad, cantidad, cantidad_inicial)
         VALUES (7, ?1, 'L01', '2026-12-01', 100, 100)",
        params![prod_id],
    ).unwrap();
    // Venta al cliente con ese lote (snapshot guardado como en registrar_venta)
    let venta_id = insertar_venta(&conn, "NV-500", "EFECTIVO", 5.0, "COMPLETADA", None);
    conn.execute("UPDATE ventas SET cliente_id = 50 WHERE id = ?1", params![venta_id]).unwrap();
    conn.execute(
        "INSERT INTO venta_detalles (venta_id, producto_id, cantidad, precio_unitario, subtotal,
         lote_id, lote_numero, lote_fecha_caducidad)
         VALUES (?1, ?2, 2.0, 2.5, 5.0, 7, 'L01', '2026-12-01')",
        params![venta_id, prod_id],
    ).unwrap();

    // Recall por número de lote → debe encontrar al cliente
    let r1 = recall_buscar(&conn, "L01");
    assert_eq!(r1.len(), 1, "Debe encontrar 1 venta del lote L01");
    assert_eq!(r1[0].0, "Lucia Bueno");
    assert_eq!(r1[0].1.as_deref(), Some("0991112233"), "Debe traer el telefono para contactar");
    assert_eq!(r1[0].2.as_deref(), Some("L01"));

    // Borrar el lote del inventario: el snapshot en venta_detalles debe seguir rastreando
    conn.execute("DELETE FROM lotes_caducidad WHERE id = 7", []).unwrap();
    let r2 = recall_buscar(&conn, "L01");
    assert_eq!(r2.len(), 1, "El snapshot debe sobrevivir al borrado del lote");
    assert_eq!(r2[0].0, "Lucia Bueno");

    // Una venta anulada NO debe aparecer en el recall
    conn.execute("UPDATE ventas SET anulada = 1 WHERE id = ?1", params![venta_id]).unwrap();
    assert_eq!(recall_buscar(&conn, "L01").len(), 0, "Ventas anuladas excluidas del recall");
}

// ── 10) ESCENARIO MIXTO COMPLEJO — test de aceptación ───────────────────────

#[test]
fn escenario_mixto_caja_cierra_correctamente() {
    // 100 inicial + 10 NV + 25 F-PENDIENTE + 0 transfer + 0 NE - 15 retiro - 5 gasto = 115
    let conn = setup_db();
    let caja_id = abrir_caja(&conn, 100.0);
    insertar_venta(&conn, "NV-001", "EFECTIVO", 10.0, "COMPLETADA", None);
    insertar_venta(&conn, "F-001", "EFECTIVO", 25.0, "PENDIENTE", None);
    insertar_venta(&conn, "F-002", "TRANSFERENCIA", 50.0, "COMPLETADA", None);
    insertar_venta(&conn, "NE-001", "EFECTIVO", 100.0, "PENDIENTE", Some("GUIA_REMISION"));
    conn.execute(
        "INSERT INTO retiros_caja (caja_id, monto, motivo, usuario, estado, fecha)
         VALUES (?1, 15.0, 'Test', 'tester', 'SIN_DEPOSITO', datetime('now','localtime'))",
        params![caja_id],
    ).unwrap();
    conn.execute(
        "INSERT INTO gastos (descripcion, monto, categoria, caja_id, observacion, es_recurrente)
         VALUES ('Test', 5.0, 'Otros', ?1, NULL, 0)",
        params![caja_id],
    ).unwrap();
    let esperado = calcular_monto_esperado_actual(&conn, caja_id);
    assert!((esperado - 115.0).abs() < 0.001,
        "Escenario mixto: esperaba 115, got {} (100+10+25+0+0-15-5)", esperado);
}
