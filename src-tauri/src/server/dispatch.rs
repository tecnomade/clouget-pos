use super::state::ServerState;
use crate::models::*;
use serde_json::Value;

/// Despacha un comando remoto a la función Rust correspondiente.
/// Retorna el resultado serializado como JSON o un error.
pub async fn dispatch_command(
    state: &ServerState,
    command: &str,
    args: Value,
) -> Result<Value, String> {
    match command {
        // --- Productos ---
        "buscar_productos" => {
            let termino: String = extract(&args, "termino")?;
            let cliente_id: Option<i64> = args.get("clienteId").and_then(|v| v.as_i64());
            let conn = state.db.conn.lock().map_err(|e| e.to_string())?;
            let mut stmt = conn
                .prepare(
                    "SELECT p.id, p.codigo, p.nombre, p.precio_venta, p.iva_porcentaje,
                     p.stock_actual, p.stock_minimo,
                     COALESCE(c.nombre, '') as categoria_nombre
                     FROM productos p
                     LEFT JOIN categorias c ON p.categoria_id = c.id
                     WHERE p.activo = 1 AND (
                         p.nombre LIKE ?1 OR p.codigo LIKE ?1 OR p.codigo_barras LIKE ?1
                     )
                     ORDER BY p.nombre LIMIT 50",
                )
                .map_err(|e| e.to_string())?;

            let busqueda = format!("%{}%", termino);
            let productos: Vec<ProductoBusqueda> = stmt
                .query_map(rusqlite::params![busqueda], |row| {
                    Ok(ProductoBusqueda {
                        id: row.get(0)?,
                        codigo: row.get(1)?,
                        nombre: row.get(2)?,
                        precio_venta: row.get(3)?,
                        iva_porcentaje: row.get(4)?,
                        stock_actual: row.get(5)?,
                        stock_minimo: row.get(6)?,
                        categoria_nombre: row.get(7)?,
                        precio_lista: None,
                    })
                })
                .map_err(|e| e.to_string())?
                .collect::<Result<Vec<_>, _>>()
                .map_err(|e| e.to_string())?;

            // Si hay cliente_id, resolver precio de lista
            if let Some(cid) = cliente_id {
                let lista_id: Option<i64> = conn
                    .query_row(
                        "SELECT lista_precio_id FROM clientes WHERE id = ?1",
                        rusqlite::params![cid],
                        |row| row.get(0),
                    )
                    .ok()
                    .flatten();

                if let Some(lid) = lista_id {
                    let mut result = productos;
                    for p in &mut result {
                        let precio: Option<f64> = conn
                            .query_row(
                                "SELECT precio FROM precios_producto WHERE lista_precio_id = ?1 AND producto_id = ?2",
                                rusqlite::params![lid, p.id],
                                |row| row.get(0),
                            )
                            .ok();
                        p.precio_lista = precio;
                    }
                    return to_json(&result);
                }
            }

            to_json(&productos)
        }

        "listar_productos" => {
            let conn = state.db.conn.lock().map_err(|e| e.to_string())?;
            let mut stmt = conn
                .prepare(
                    "SELECT id, codigo, codigo_barras, nombre, descripcion, categoria_id,
                     precio_costo, precio_venta, iva_porcentaje, incluye_iva,
                     stock_actual, stock_minimo, unidad_medida, es_servicio, activo, imagen
                     FROM productos ORDER BY nombre",
                )
                .map_err(|e| e.to_string())?;

            let productos: Vec<Producto> = stmt
                .query_map([], |row| {
                    Ok(Producto {
                        id: Some(row.get(0)?),
                        codigo: row.get(1)?,
                        codigo_barras: row.get(2)?,
                        nombre: row.get(3)?,
                        descripcion: row.get(4)?,
                        categoria_id: row.get(5)?,
                        precio_costo: row.get(6)?,
                        precio_venta: row.get(7)?,
                        iva_porcentaje: row.get(8)?,
                        incluye_iva: row.get(9)?,
                        stock_actual: row.get(10)?,
                        stock_minimo: row.get(11)?,
                        unidad_medida: row.get(12)?,
                        es_servicio: row.get(13)?,
                        activo: row.get(14)?,
                        imagen: row.get(15)?,
                    })
                })
                .map_err(|e| e.to_string())?
                .collect::<Result<Vec<_>, _>>()
                .map_err(|e| e.to_string())?;

            to_json(&productos)
        }

        // --- Clientes ---
        "buscar_clientes" => {
            let termino: String = extract(&args, "termino")?;
            let conn = state.db.conn.lock().map_err(|e| e.to_string())?;
            let busqueda = format!("%{}%", termino);
            let mut stmt = conn
                .prepare(
                    "SELECT id, tipo_identificacion, identificacion, nombre, direccion,
                     telefono, email, activo, lista_precio_id
                     FROM clientes
                     WHERE nombre LIKE ?1 OR identificacion LIKE ?1
                     ORDER BY nombre LIMIT 50",
                )
                .map_err(|e| e.to_string())?;

            let clientes: Vec<Value> = stmt
                .query_map(rusqlite::params![busqueda], |row| {
                    Ok(serde_json::json!({
                        "id": row.get::<_, i64>(0)?,
                        "tipo_identificacion": row.get::<_, String>(1)?,
                        "identificacion": row.get::<_, Option<String>>(2)?,
                        "nombre": row.get::<_, String>(3)?,
                        "direccion": row.get::<_, Option<String>>(4)?,
                        "telefono": row.get::<_, Option<String>>(5)?,
                        "email": row.get::<_, Option<String>>(6)?,
                        "activo": row.get::<_, bool>(7)?,
                        "lista_precio_id": row.get::<_, Option<i64>>(8)?,
                    }))
                })
                .map_err(|e| e.to_string())?
                .collect::<Result<Vec<_>, _>>()
                .map_err(|e| e.to_string())?;

            to_json(&clientes)
        }

        "listar_clientes" => {
            let conn = state.db.conn.lock().map_err(|e| e.to_string())?;
            let mut stmt = conn
                .prepare(
                    "SELECT c.id, c.tipo_identificacion, c.identificacion, c.nombre,
                     c.direccion, c.telefono, c.email, c.activo, c.lista_precio_id,
                     lp.nombre as lista_nombre
                     FROM clientes c
                     LEFT JOIN listas_precios lp ON c.lista_precio_id = lp.id
                     ORDER BY c.nombre",
                )
                .map_err(|e| e.to_string())?;

            let clientes: Vec<Value> = stmt
                .query_map([], |row| {
                    Ok(serde_json::json!({
                        "id": row.get::<_, i64>(0)?,
                        "tipo_identificacion": row.get::<_, String>(1)?,
                        "identificacion": row.get::<_, Option<String>>(2)?,
                        "nombre": row.get::<_, String>(3)?,
                        "direccion": row.get::<_, Option<String>>(4)?,
                        "telefono": row.get::<_, Option<String>>(5)?,
                        "email": row.get::<_, Option<String>>(6)?,
                        "activo": row.get::<_, bool>(7)?,
                        "lista_precio_id": row.get::<_, Option<i64>>(8)?,
                        "lista_precio_nombre": row.get::<_, Option<String>>(9)?,
                    }))
                })
                .map_err(|e| e.to_string())?
                .collect::<Result<Vec<_>, _>>()
                .map_err(|e| e.to_string())?;

            to_json(&clientes)
        }

        // --- Configuración ---
        "obtener_config" => {
            let conn = state.db.conn.lock().map_err(|e| e.to_string())?;
            let mut stmt = conn
                .prepare("SELECT key, value FROM config")
                .map_err(|e| e.to_string())?;

            let mut config = std::collections::HashMap::new();
            let rows = stmt
                .query_map([], |row| {
                    Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
                })
                .map_err(|e| e.to_string())?;

            for row in rows {
                let (k, v) = row.map_err(|e| e.to_string())?;
                config.insert(k, v);
            }

            to_json(&config)
        }

        "guardar_config" => {
            let config: std::collections::HashMap<String, String> =
                serde_json::from_value(args.get("config").cloned().unwrap_or(Value::Null))
                    .map_err(|e| e.to_string())?;

            let conn = state.db.conn.lock().map_err(|e| e.to_string())?;
            for (key, value) in &config {
                conn.execute(
                    "INSERT OR REPLACE INTO config (key, value) VALUES (?1, ?2)",
                    rusqlite::params![key, value],
                )
                .map_err(|e| e.to_string())?;
            }

            to_json(&true)
        }

        // --- Caja ---
        "obtener_caja_abierta" => {
            let conn = state.db.conn.lock().map_err(|e| e.to_string())?;
            let caja: Option<Value> = conn
                .query_row(
                    "SELECT id, fecha_apertura, fecha_cierre, monto_inicial, monto_ventas,
                     monto_esperado, monto_real, diferencia, estado, usuario, usuario_id, observacion
                     FROM caja WHERE estado = 'ABIERTA' ORDER BY id DESC LIMIT 1",
                    [],
                    |row| {
                        Ok(serde_json::json!({
                            "id": row.get::<_, i64>(0)?,
                            "fecha_apertura": row.get::<_, Option<String>>(1)?,
                            "fecha_cierre": row.get::<_, Option<String>>(2)?,
                            "monto_inicial": row.get::<_, f64>(3)?,
                            "monto_ventas": row.get::<_, f64>(4)?,
                            "monto_esperado": row.get::<_, f64>(5)?,
                            "monto_real": row.get::<_, Option<f64>>(6)?,
                            "diferencia": row.get::<_, Option<f64>>(7)?,
                            "estado": row.get::<_, String>(8)?,
                            "usuario": row.get::<_, Option<String>>(9)?,
                            "usuario_id": row.get::<_, Option<i64>>(10)?,
                            "observacion": row.get::<_, Option<String>>(11)?,
                        }))
                    },
                )
                .ok();

            to_json(&caja)
        }

        "abrir_caja" => {
            let monto_inicial: f64 = extract(&args, "montoInicial")?;
            let sesion_guard = state.sesion.sesion.lock().map_err(|e| e.to_string())?;
            let sesion = sesion_guard
                .as_ref()
                .ok_or("Debe iniciar sesión".to_string())?;
            let usuario = sesion.nombre.clone();
            let usuario_id = sesion.usuario_id;
            drop(sesion_guard);

            let conn = state.db.conn.lock().map_err(|e| e.to_string())?;

            // Verificar que no haya caja abierta
            let abierta: i64 = conn
                .query_row(
                    "SELECT COUNT(*) FROM caja WHERE estado = 'ABIERTA'",
                    [],
                    |row| row.get(0),
                )
                .unwrap_or(0);

            if abierta > 0 {
                return Err("Ya hay una caja abierta".to_string());
            }

            conn.execute(
                "INSERT INTO caja (monto_inicial, monto_esperado, estado, usuario, usuario_id) VALUES (?1, ?1, 'ABIERTA', ?2, ?3)",
                rusqlite::params![monto_inicial, usuario, usuario_id],
            )
            .map_err(|e| e.to_string())?;

            let id = conn.last_insert_rowid();
            to_json(&serde_json::json!({
                "id": id,
                "monto_inicial": monto_inicial,
                "estado": "ABIERTA",
                "usuario": usuario,
            }))
        }

        // --- Usuarios / Sesión ---
        "iniciar_sesion" => {
            let pin: String = extract(&args, "pin")?;

            let usuarios: Vec<(i64, String, String, String, String)> = (|| -> Result<Vec<_>, String> {
                let conn = state.db.conn.lock().map_err(|e| e.to_string())?;
                let mut stmt = conn
                    .prepare("SELECT id, nombre, pin_hash, pin_salt, rol FROM usuarios WHERE activo = 1")
                    .map_err(|e| e.to_string())?;

                let result = stmt.query_map([], |row| {
                    Ok((
                        row.get(0)?,
                        row.get(1)?,
                        row.get(2)?,
                        row.get(3)?,
                        row.get(4)?,
                    ))
                })
                .map_err(|e| e.to_string())?
                .collect::<Result<Vec<_>, _>>()
                .map_err(|e| e.to_string())?;
                Ok(result)
            })()?;

            for (id, nombre, hash, salt, rol) in &usuarios {
                let pin_hash = crate::utils::hash_pin(salt, &pin);
                if &pin_hash == hash {
                    let sesion = crate::models::SesionActiva {
                        usuario_id: *id,
                        nombre: nombre.clone(),
                        rol: rol.clone(),
                    };
                    let mut sesion_guard =
                        state.sesion.sesion.lock().map_err(|e| e.to_string())?;
                    *sesion_guard = Some(sesion.clone());
                    return to_json(&sesion);
                }
            }

            Err("PIN incorrecto".to_string())
        }

        "cerrar_sesion" => {
            let mut sesion_guard = state.sesion.sesion.lock().map_err(|e| e.to_string())?;
            *sesion_guard = None;
            to_json(&true)
        }

        "obtener_sesion_actual" => {
            let sesion_guard = state.sesion.sesion.lock().map_err(|e| e.to_string())?;
            to_json(&*sesion_guard)
        }

        // --- Ventas ---
        "registrar_venta" => {
            // Deserializar venta y llamar la función interna
            let venta: NuevaVenta =
                serde_json::from_value(args.get("venta").cloned().unwrap_or(Value::Null))
                    .map_err(|e| format!("Error deserializando venta: {}", e))?;

            // Simular sesion via State wrapper — la lógica de venta necesita sesion
            let sesion_guard = state.sesion.sesion.lock().map_err(|e| e.to_string())?;
            let sesion_actual = sesion_guard
                .as_ref()
                .ok_or("Debe iniciar sesión para registrar ventas".to_string())?;
            let usuario_nombre = sesion_actual.nombre.clone();
            let usuario_id = sesion_actual.usuario_id;
            drop(sesion_guard);

            let conn = state.db.conn.lock().map_err(|e| e.to_string())?;

            // Verificar caja abierta
            let caja_abierta: bool = conn
                .query_row(
                    "SELECT COUNT(*) FROM caja WHERE estado = 'ABIERTA'",
                    [],
                    |row| row.get::<_, i64>(0),
                )
                .map(|c| c > 0)
                .unwrap_or(false);

            if !caja_abierta {
                return Err("Debe abrir la caja antes de realizar ventas".to_string());
            }

            // Leer terminal
            let terminal_est: String = conn
                .query_row("SELECT value FROM config WHERE key = 'terminal_establecimiento'", [], |row| row.get(0))
                .unwrap_or_else(|_| "001".to_string());
            let terminal_pe: String = conn
                .query_row("SELECT value FROM config WHERE key = 'terminal_punto_emision'", [], |row| row.get(0))
                .unwrap_or_else(|_| "001".to_string());

            // Secuencial
            conn.execute(
                "INSERT OR IGNORE INTO secuenciales (establecimiento_codigo, punto_emision_codigo, tipo_documento, secuencial) VALUES (?1, ?2, 'NOTA_VENTA', 1)",
                rusqlite::params![terminal_est, terminal_pe],
            ).map_err(|e| e.to_string())?;

            let secuencial: i64 = conn
                .query_row(
                    "SELECT secuencial FROM secuenciales WHERE establecimiento_codigo = ?1 AND punto_emision_codigo = ?2 AND tipo_documento = 'NOTA_VENTA'",
                    rusqlite::params![terminal_est, terminal_pe],
                    |row| row.get(0),
                )
                .map_err(|e| e.to_string())?;

            let numero = format!("NV-{:09}", secuencial);

            // Calcular totales
            let mut subtotal_sin_iva = 0.0_f64;
            let mut subtotal_con_iva = 0.0_f64;
            let mut iva_total = 0.0_f64;

            for item in &venta.items {
                let subtotal_item = item.cantidad * item.precio_unitario - item.descuento;
                if item.iva_porcentaje > 0.0 {
                    subtotal_con_iva += subtotal_item;
                    iva_total += subtotal_item * (item.iva_porcentaje / 100.0);
                } else {
                    subtotal_sin_iva += subtotal_item;
                }
            }

            let total = subtotal_sin_iva + subtotal_con_iva + iva_total - venta.descuento;
            let cambio = if venta.monto_recibido > total { venta.monto_recibido - total } else { 0.0 };

            let estado_sri = match venta.tipo_documento.as_str() {
                "FACTURA" => "PENDIENTE",
                _ => "NO_APLICA",
            };

            // INSERT venta
            conn.execute(
                "INSERT INTO ventas (numero, cliente_id, subtotal_sin_iva, subtotal_con_iva,
                 descuento, iva, total, forma_pago, monto_recibido, cambio, estado,
                 tipo_documento, estado_sri, observacion, usuario, usuario_id, establecimiento, punto_emision)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18)",
                rusqlite::params![
                    numero, venta.cliente_id.unwrap_or(1),
                    subtotal_sin_iva, subtotal_con_iva, venta.descuento,
                    iva_total, total, venta.forma_pago, venta.monto_recibido,
                    cambio, "COMPLETADA", venta.tipo_documento, estado_sri,
                    venta.observacion, usuario_nombre, usuario_id, terminal_est, terminal_pe,
                ],
            ).map_err(|e| e.to_string())?;

            let venta_id = conn.last_insert_rowid();

            // Detalles + stock
            for item in &venta.items {
                let subtotal = item.cantidad * item.precio_unitario - item.descuento;
                conn.execute(
                    "INSERT INTO venta_detalles (venta_id, producto_id, cantidad, precio_unitario, descuento, iva_porcentaje, subtotal)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                    rusqlite::params![venta_id, item.producto_id, item.cantidad, item.precio_unitario, item.descuento, item.iva_porcentaje, subtotal],
                ).map_err(|e| e.to_string())?;

                conn.execute(
                    "UPDATE productos SET stock_actual = stock_actual - ?1, updated_at = datetime('now','localtime') WHERE id = ?2 AND es_servicio = 0",
                    rusqlite::params![item.cantidad, item.producto_id],
                ).map_err(|e| e.to_string())?;
            }

            // Incrementar secuencial
            conn.execute(
                "UPDATE secuenciales SET secuencial = secuencial + 1 WHERE establecimiento_codigo = ?1 AND punto_emision_codigo = ?2 AND tipo_documento = 'NOTA_VENTA'",
                rusqlite::params![terminal_est, terminal_pe],
            ).map_err(|e| e.to_string())?;

            // Actualizar caja
            conn.execute(
                "UPDATE caja SET monto_ventas = monto_ventas + ?1, monto_esperado = monto_inicial + monto_ventas + ?1 WHERE estado = 'ABIERTA'",
                rusqlite::params![total],
            ).ok();

            // Fiado
            if venta.es_fiado {
                conn.execute(
                    "INSERT INTO cuentas_por_cobrar (cliente_id, venta_id, monto_total, saldo, estado) VALUES (?1, ?2, ?3, ?3, 'PENDIENTE')",
                    rusqlite::params![venta.cliente_id.unwrap_or(1), venta_id, total],
                ).map_err(|e| e.to_string())?;
            }

            to_json(&serde_json::json!({
                "venta": {
                    "id": venta_id,
                    "numero": numero,
                    "total": total,
                    "estado": "COMPLETADA",
                    "tipo_documento": venta.tipo_documento,
                    "estado_sri": estado_sri,
                    "establecimiento": terminal_est,
                    "punto_emision": terminal_pe,
                },
                "detalles": [],
                "cliente_nombre": null,
            }))
        }

        "listar_ventas_dia" => {
            let fecha: String = extract(&args, "fecha")?;
            let conn = state.db.conn.lock().map_err(|e| e.to_string())?;
            let mut stmt = conn
                .prepare(
                    "SELECT id, numero, cliente_id, fecha, subtotal_sin_iva, subtotal_con_iva,
                     descuento, iva, total, forma_pago, monto_recibido, cambio, estado,
                     tipo_documento, estado_sri, autorizacion_sri, clave_acceso, observacion,
                     numero_factura, establecimiento, punto_emision
                     FROM ventas WHERE date(fecha) = date(?1) AND anulada = 0 ORDER BY fecha DESC",
                )
                .map_err(|e| e.to_string())?;

            let ventas: Vec<Value> = stmt
                .query_map(rusqlite::params![fecha], |row| {
                    Ok(serde_json::json!({
                        "id": row.get::<_, i64>(0)?,
                        "numero": row.get::<_, String>(1)?,
                        "cliente_id": row.get::<_, Option<i64>>(2)?,
                        "fecha": row.get::<_, Option<String>>(3)?,
                        "subtotal_sin_iva": row.get::<_, f64>(4)?,
                        "subtotal_con_iva": row.get::<_, f64>(5)?,
                        "descuento": row.get::<_, f64>(6)?,
                        "iva": row.get::<_, f64>(7)?,
                        "total": row.get::<_, f64>(8)?,
                        "forma_pago": row.get::<_, String>(9)?,
                        "monto_recibido": row.get::<_, f64>(10)?,
                        "cambio": row.get::<_, f64>(11)?,
                        "estado": row.get::<_, String>(12)?,
                        "tipo_documento": row.get::<_, String>(13)?,
                        "estado_sri": row.get::<_, String>(14).unwrap_or_else(|_| "NO_APLICA".to_string()),
                        "autorizacion_sri": row.get::<_, Option<String>>(15)?,
                        "clave_acceso": row.get::<_, Option<String>>(16)?,
                        "observacion": row.get::<_, Option<String>>(17)?,
                        "numero_factura": row.get::<_, Option<String>>(18)?,
                        "establecimiento": row.get::<_, Option<String>>(19)?,
                        "punto_emision": row.get::<_, Option<String>>(20)?,
                    }))
                })
                .map_err(|e| e.to_string())?
                .collect::<Result<Vec<_>, _>>()
                .map_err(|e| e.to_string())?;

            to_json(&ventas)
        }

        // --- Establecimientos ---
        "listar_establecimientos" => {
            let conn = state.db.conn.lock().map_err(|e| e.to_string())?;
            let mut stmt = conn
                .prepare("SELECT id, codigo, nombre, direccion, telefono, es_propio, activo FROM establecimientos ORDER BY codigo")
                .map_err(|e| e.to_string())?;

            let items: Vec<Value> = stmt
                .query_map([], |row| {
                    Ok(serde_json::json!({
                        "id": row.get::<_, i64>(0)?,
                        "codigo": row.get::<_, String>(1)?,
                        "nombre": row.get::<_, String>(2)?,
                        "direccion": row.get::<_, Option<String>>(3)?,
                        "telefono": row.get::<_, Option<String>>(4)?,
                        "es_propio": row.get::<_, bool>(5)?,
                        "activo": row.get::<_, bool>(6)?,
                    }))
                })
                .map_err(|e| e.to_string())?
                .collect::<Result<Vec<_>, _>>()
                .map_err(|e| e.to_string())?;

            to_json(&items)
        }

        "listar_puntos_emision" => {
            let establecimiento_id: i64 = extract(&args, "establecimientoId")?;
            let conn = state.db.conn.lock().map_err(|e| e.to_string())?;
            let mut stmt = conn
                .prepare("SELECT id, establecimiento_id, codigo, nombre, activo FROM puntos_emision WHERE establecimiento_id = ?1 ORDER BY codigo")
                .map_err(|e| e.to_string())?;

            let items: Vec<Value> = stmt
                .query_map(rusqlite::params![establecimiento_id], |row| {
                    Ok(serde_json::json!({
                        "id": row.get::<_, i64>(0)?,
                        "establecimiento_id": row.get::<_, i64>(1)?,
                        "codigo": row.get::<_, String>(2)?,
                        "nombre": row.get::<_, Option<String>>(3)?,
                        "activo": row.get::<_, bool>(4)?,
                    }))
                })
                .map_err(|e| e.to_string())?
                .collect::<Result<Vec<_>, _>>()
                .map_err(|e| e.to_string())?;

            to_json(&items)
        }

        // --- Categorías ---
        "listar_categorias" => {
            let conn = state.db.conn.lock().map_err(|e| e.to_string())?;
            let mut stmt = conn
                .prepare("SELECT id, nombre, descripcion, activo FROM categorias ORDER BY nombre")
                .map_err(|e| e.to_string())?;

            let cats: Vec<Value> = stmt
                .query_map([], |row| {
                    Ok(serde_json::json!({
                        "id": row.get::<_, i64>(0)?,
                        "nombre": row.get::<_, String>(1)?,
                        "descripcion": row.get::<_, Option<String>>(2)?,
                        "activo": row.get::<_, bool>(3)?,
                    }))
                })
                .map_err(|e| e.to_string())?
                .collect::<Result<Vec<_>, _>>()
                .map_err(|e| e.to_string())?;

            to_json(&cats)
        }

        // --- Listas de Precios ---
        "resolver_precio_producto" => {
            let producto_id: i64 = extract(&args, "productoId")?;
            let cliente_id: Option<i64> = args.get("clienteId").and_then(|v| v.as_i64());

            let conn = state.db.conn.lock().map_err(|e| e.to_string())?;

            // Precio base
            let precio_base: f64 = conn
                .query_row(
                    "SELECT precio_venta FROM productos WHERE id = ?1",
                    rusqlite::params![producto_id],
                    |row| row.get(0),
                )
                .map_err(|e| e.to_string())?;

            if let Some(cid) = cliente_id {
                let lista_id: Option<i64> = conn
                    .query_row(
                        "SELECT lista_precio_id FROM clientes WHERE id = ?1",
                        rusqlite::params![cid],
                        |row| row.get(0),
                    )
                    .ok()
                    .flatten();

                if let Some(lid) = lista_id {
                    let precio: Option<f64> = conn
                        .query_row(
                            "SELECT precio FROM precios_producto WHERE lista_precio_id = ?1 AND producto_id = ?2",
                            rusqlite::params![lid, producto_id],
                            |row| row.get(0),
                        )
                        .ok();

                    if let Some(p) = precio {
                        return to_json(&p);
                    }
                }
            }

            to_json(&precio_base)
        }

        // Comando no implementado en el servidor
        // --- Secuenciales: Reservar rango para modo offline ---
        "reservar_secuenciales" => {
            let establecimiento: String = extract(&args, "establecimiento")?;
            let punto_emision: String = extract(&args, "puntoEmision")?;
            let tipo_documento: String = extract(&args, "tipoDocumento")?;
            let cantidad: i64 = extract(&args, "cantidad")?;

            let conn = state.db.conn.lock().map_err(|e| e.to_string())?;

            // Asegurar que existe
            conn.execute(
                "INSERT OR IGNORE INTO secuenciales (establecimiento_codigo, punto_emision_codigo, tipo_documento, secuencial) VALUES (?1, ?2, ?3, 1)",
                rusqlite::params![establecimiento, punto_emision, tipo_documento],
            ).map_err(|e| e.to_string())?;

            // Leer actual
            let desde: i64 = conn
                .query_row(
                    "SELECT secuencial FROM secuenciales WHERE establecimiento_codigo = ?1 AND punto_emision_codigo = ?2 AND tipo_documento = ?3",
                    rusqlite::params![establecimiento, punto_emision, tipo_documento],
                    |row| row.get(0),
                )
                .map_err(|e| e.to_string())?;

            let hasta = desde + cantidad - 1;

            // Avanzar el secuencial en el servidor
            conn.execute(
                "UPDATE secuenciales SET secuencial = ?1 WHERE establecimiento_codigo = ?2 AND punto_emision_codigo = ?3 AND tipo_documento = ?4",
                rusqlite::params![hasta + 1, establecimiento, punto_emision, tipo_documento],
            ).map_err(|e| e.to_string())?;

            to_json(&serde_json::json!({
                "desde": desde,
                "hasta": hasta,
            }))
        }

        _ => Err(format!("Comando '{}' no disponible en modo red", command)),
    }
}

/// Helper: extrae un campo del JSON args
fn extract<T: serde::de::DeserializeOwned>(args: &Value, key: &str) -> Result<T, String> {
    let val = args
        .get(key)
        .ok_or_else(|| format!("Parámetro '{}' requerido", key))?;
    serde_json::from_value(val.clone()).map_err(|e| format!("Error en parámetro '{}': {}", key, e))
}

/// Helper: serializa a JSON Value
fn to_json<T: serde::Serialize>(data: &T) -> Result<Value, String> {
    serde_json::to_value(data).map_err(|e| e.to_string())
}
