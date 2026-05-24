use crate::db::{Database, SesionState};
use crate::models::{Compra, CompraCompleta, CompraDetalle, NuevaCompra};
use tauri::State;

/// v2.5.30: helper para obtener nombre de usuario actual desde sesion
fn usuario_actual(sesion: &SesionState) -> String {
    sesion.sesion.lock().ok()
        .and_then(|g| g.as_ref().map(|u| u.nombre.clone()))
        .unwrap_or_else(|| "ADMIN".to_string())
}

/// v2.5.30: valida que numero_factura+tipo_documento no este duplicado para el proveedor.
/// Retorna Err con mensaje amigable si ya existe. Tambien valida clave_acceso unica.
fn validar_factura_unica(
    conn: &rusqlite::Connection,
    proveedor_id: i64,
    numero_factura: Option<&str>,
    tipo_documento: &str,
    clave_acceso: Option<&str>,
    excluir_compra_id: Option<i64>,
) -> Result<(), String> {
    // Validar clave_acceso unica (global) — solo si viene
    if let Some(ca) = clave_acceso {
        if !ca.trim().is_empty() && ca.trim().len() == 49 {
            let exists: Option<i64> = match excluir_compra_id {
                Some(eid) => conn.query_row(
                    "SELECT id FROM compras WHERE clave_acceso = ?1 AND id != ?2 AND estado != 'ANULADA' LIMIT 1",
                    rusqlite::params![ca.trim(), eid], |r| r.get(0),
                ).ok(),
                None => conn.query_row(
                    "SELECT id FROM compras WHERE clave_acceso = ?1 AND estado != 'ANULADA' LIMIT 1",
                    rusqlite::params![ca.trim()], |r| r.get(0),
                ).ok(),
            };
            if exists.is_some() {
                return Err(format!(
                    "Esta factura ya fue importada (clave de acceso SRI duplicada: {}…)",
                    &ca.trim()[..ca.trim().len().min(20)]
                ));
            }
            // v2.5.32: también chequear si la clave fue importada como GASTO
            // (cuando todos los items del XML se mapearon a gastos, no se crea
            // ninguna compra, pero la clave_acceso queda registrada en gastos)
            let exists_gasto: Option<i64> = conn.query_row(
                "SELECT id FROM gastos WHERE clave_acceso = ?1 LIMIT 1",
                rusqlite::params![ca.trim()], |r| r.get(0),
            ).ok();
            if exists_gasto.is_some() {
                return Err(format!(
                    "Esta factura ya fue importada anteriormente como GASTO (clave SRI: {}…). Si necesitas re-importarla como compra, primero elimina el gasto.",
                    &ca.trim()[..ca.trim().len().min(20)]
                ));
            }
        }
    }
    // Validar numero_factura + tipo + proveedor — solo si viene numero_factura
    if let Some(nf) = numero_factura {
        let nf_trim = nf.trim();
        if !nf_trim.is_empty() {
            let exists: Option<(i64, String)> = match excluir_compra_id {
                Some(eid) => conn.query_row(
                    "SELECT id, COALESCE(numero, '') FROM compras
                     WHERE proveedor_id = ?1 AND tipo_documento = ?2 AND numero_factura = ?3
                       AND id != ?4 AND estado != 'ANULADA' LIMIT 1",
                    rusqlite::params![proveedor_id, tipo_documento, nf_trim, eid],
                    |r| Ok((r.get(0)?, r.get(1)?)),
                ).ok(),
                None => conn.query_row(
                    "SELECT id, COALESCE(numero, '') FROM compras
                     WHERE proveedor_id = ?1 AND tipo_documento = ?2 AND numero_factura = ?3
                       AND estado != 'ANULADA' LIMIT 1",
                    rusqlite::params![proveedor_id, tipo_documento, nf_trim],
                    |r| Ok((r.get(0)?, r.get(1)?)),
                ).ok(),
            };
            if let Some((_id, num_existente)) = exists {
                return Err(format!(
                    "Ya existe una compra de este proveedor con número de {} '{}' (compra interna {}). Si fue anulada puede registrar otra; de lo contrario verifique el número.",
                    if tipo_documento == "FACTURA" { "factura" }
                    else if tipo_documento == "NOTA_VENTA" { "nota de venta" }
                    else { "documento" },
                    nf_trim, num_existente
                ));
            }
        }
    }
    Ok(())
}

/// v2.5.30: auto-genera el numero interno de compra (COMP-XXXXXXXXX, 9 dig).
/// Usa el MAX existente + 1 (independiente del config 'secuencial_compra' viejo).
fn proximo_numero_compra(conn: &rusqlite::Connection) -> String {
    let next: i64 = conn.query_row(
        "SELECT COALESCE(MAX(CAST(SUBSTR(numero, 6) AS INTEGER)), 0) + 1
         FROM compras WHERE numero LIKE 'COMP-%'",
        [], |r| r.get(0),
    ).unwrap_or(1);
    // Sincronizamos tambien el config viejo para compatibilidad
    let _ = conn.execute(
        "UPDATE config SET value = CAST(?1 AS TEXT) WHERE key = 'secuencial_compra'",
        rusqlite::params![next + 1],
    );
    format!("COMP-{:09}", next)
}

#[tauri::command]
pub fn registrar_compra(
    db: State<Database>,
    sesion: State<SesionState>,
    compra: NuevaCompra,
) -> Result<CompraCompleta, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let usuario = usuario_actual(&sesion);

    // v2.5.30: tipo de documento — default INFORMAL si no se especifica
    let tipo_documento = compra.tipo_documento.clone()
        .map(|s| s.trim().to_uppercase())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "INFORMAL".to_string());
    if !["FACTURA", "NOTA_VENTA", "INFORMAL"].contains(&tipo_documento.as_str()) {
        return Err(format!("tipo_documento invalido '{}' (esperado: FACTURA, NOTA_VENTA o INFORMAL)", tipo_documento));
    }

    // v2.5.30: numero_factura es opcional. Si tipo es FACTURA y viene vacio dejarlo NULL.
    // Si tipo es INFORMAL forzamos numero_factura a NULL (no aplica).
    let numero_factura: Option<String> = if tipo_documento == "INFORMAL" {
        None
    } else {
        compra.numero_factura.as_ref()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
    };
    let clave_acceso: Option<String> = compra.clave_acceso.as_ref()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());

    // v2.5.30: validar duplicado por numero_factura+proveedor+tipo y clave_acceso
    validar_factura_unica(
        &conn, compra.proveedor_id,
        numero_factura.as_deref(), &tipo_documento,
        clave_acceso.as_deref(), None,
    )?;

    // v2.5.30: numero interno SIEMPRE autogenerado (formato COMP-XXXXXXXXX, 9 dig)
    let numero = proximo_numero_compra(&conn);

    // Calcular totales
    let mut subtotal_total = 0.0;
    let mut iva_total = 0.0;

    for item in &compra.items {
        let item_subtotal = item.cantidad * item.precio_unitario;
        let item_iva = item_subtotal * (item.iva_porcentaje / 100.0);
        subtotal_total += item_subtotal;
        iva_total += item_iva;
    }

    let total = subtotal_total + iva_total;

    // Validar que si la forma de pago requiere banco, se especifique
    let req_banco = matches!(compra.forma_pago.as_str(), "DEBITO" | "TRANSFERENCIA" | "CHEQUE");
    if req_banco && compra.banco_id.is_none() {
        return Err("Debe seleccionar una cuenta bancaria para esta forma de pago".into());
    }

    // fecha_emision: si viene del frontend en formato YYYY-MM-DD, normalizar a ISO
    let fecha_emision_norm = compra.fecha_emision.as_ref()
        .map(|s| convertir_fecha_sri(s).unwrap_or_else(|| s.clone()))
        .filter(|s| !s.trim().is_empty());

    // Insertar compra
    conn.execute(
        "INSERT INTO compras (numero, proveedor_id, numero_factura, subtotal, iva, total,
                              forma_pago, es_credito, observacion, banco_id, referencia_pago,
                              tipo_documento, estado_sri, clave_acceso, fecha_emision, usuario)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16)",
        rusqlite::params![
            numero,
            compra.proveedor_id,
            numero_factura,
            subtotal_total,
            iva_total,
            total,
            compra.forma_pago,
            compra.es_credito,
            compra.observacion,
            compra.banco_id,
            compra.referencia_pago,
            tipo_documento,
            None::<String>, // estado_sri: manual no es autorizada; solo XML lo marca
            clave_acceso,
            fecha_emision_norm,
            usuario,
        ],
    )
    .map_err(|e| {
        // Si el UNIQUE INDEX fue violado, devolver mensaje amigable
        let msg = e.to_string();
        if msg.contains("UNIQUE") && msg.contains("clave_acceso") {
            "Esta factura ya fue registrada (clave de acceso SRI duplicada)".to_string()
        } else if msg.contains("UNIQUE") && msg.contains("factura_proveedor") {
            "Ya existe una compra de este proveedor con ese numero de factura".to_string()
        } else { msg }
    })?;

    let compra_id = conn.last_insert_rowid();

    // Insertar detalles y actualizar stock/costo
    let mut detalles = Vec::new();
    for item in &compra.items {
        let item_subtotal = item.cantidad * item.precio_unitario;

        let descripcion = if let Some(pid) = item.producto_id {
            // Obtener nombre del producto
            let nombre: String = conn
                .query_row(
                    "SELECT nombre FROM productos WHERE id = ?1",
                    rusqlite::params![pid],
                    |row| row.get(0),
                )
                .unwrap_or_else(|_| "Producto desconocido".to_string());

            // v2.5.22 PMP: recalcular costo_promedio (Promedio Ponderado Movil)
            // antes de actualizar stock. Fórmula:
            //   nuevo_promedio = (stock_actual * costo_promedio + nueva_cant * precio_compra) / (stock_actual + nueva_cant)
            // Si stock_actual <= 0, el nuevo promedio es directamente el precio_compra.
            let (stock_actual, costo_promedio_actual): (f64, f64) = conn
                .query_row(
                    "SELECT stock_actual, COALESCE(costo_promedio, precio_costo) FROM productos WHERE id = ?1",
                    rusqlite::params![pid],
                    |row| Ok((row.get::<_, f64>(0)?, row.get::<_, f64>(1)?)),
                )
                .unwrap_or((0.0, item.precio_unitario));

            let nuevo_costo_promedio: f64 = if stock_actual <= 0.0 {
                // Sin stock previo (o negativo) → nuevo costo = precio de esta compra
                item.precio_unitario
            } else {
                let stock_total = stock_actual + item.cantidad;
                if stock_total <= 0.0 {
                    item.precio_unitario
                } else {
                    (stock_actual * costo_promedio_actual + item.cantidad * item.precio_unitario) / stock_total
                }
            };

            // Stock anterior antes del UPDATE — para kardex
            let stock_antes_kardex = stock_actual;

            // Actualizar stock + precio_costo (último) + costo_promedio (PMP)
            conn.execute(
                "UPDATE productos SET stock_actual = stock_actual + ?1, precio_costo = ?2,
                 costo_promedio = ?3, updated_at = datetime('now','localtime') WHERE id = ?4",
                rusqlite::params![item.cantidad, item.precio_unitario, nuevo_costo_promedio, pid],
            )
            .map_err(|e| e.to_string())?;

            // v2.5.30: registrar movimiento en kardex (INGRESO_COMPRA)
            let motivo_kardex = format!("Compra {} - {}", numero, &nombre);
            let _ = conn.execute(
                "INSERT INTO movimientos_inventario (producto_id, tipo, cantidad, stock_anterior, stock_nuevo, costo_unitario, referencia_id, usuario, motivo)
                 VALUES (?1, 'INGRESO_COMPRA', ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                rusqlite::params![pid, item.cantidad, stock_antes_kardex, stock_antes_kardex + item.cantidad, item.precio_unitario, compra_id, usuario, motivo_kardex],
            );

            Some(nombre)
        } else {
            item.descripcion.clone()
        };

        conn.execute(
            "INSERT INTO compra_detalles (compra_id, producto_id, descripcion, cantidad, precio_unitario, subtotal)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            rusqlite::params![
                compra_id,
                item.producto_id,
                descripcion,
                item.cantidad,
                item.precio_unitario,
                item_subtotal,
            ],
        )
        .map_err(|e| e.to_string())?;

        // v2.2.0: si viene fecha_caducidad, crear lote automaticamente
        if let Some(prod_id) = item.producto_id {
            if let Some(fecha_cad) = &item.lote_fecha_caducidad {
                if !fecha_cad.trim().is_empty() {
                    let lote_num = item.lote_numero.clone().filter(|s| !s.trim().is_empty())
                        .unwrap_or_else(|| {
                            // Auto-generar: LOT-YYYYMMDD-{compra_id}
                            let fecha_hoy = chrono::Local::now().format("%Y%m%d").to_string();
                            format!("LOT-{}-{}", fecha_hoy, compra_id)
                        });
                    conn.execute(
                        "INSERT INTO lotes_caducidad (producto_id, lote, fecha_caducidad, cantidad, cantidad_inicial, compra_id, fecha_elaboracion)
                         VALUES (?1, ?2, ?3, ?4, ?4, ?5, ?6)",
                        rusqlite::params![prod_id, lote_num, fecha_cad, item.cantidad, compra_id, item.lote_fecha_elaboracion],
                    ).ok();
                }
            }
        }

        let detalle_id = conn.last_insert_rowid();
        detalles.push(CompraDetalle {
            id: Some(detalle_id),
            compra_id: Some(compra_id),
            producto_id: item.producto_id,
            descripcion: descripcion.clone(),
            cantidad: item.cantidad,
            precio_unitario: item.precio_unitario,
            subtotal: item_subtotal,
            nombre_producto: descripcion,
            cantidad_devuelta: 0.0,
        });
    }

    // Si es crédito, crear cuenta por pagar
    if compra.es_credito {
        if let Some(dias) = compra.dias_credito {
            conn.execute(
                "INSERT INTO cuentas_por_pagar (proveedor_id, compra_id, monto_total, saldo, fecha_vencimiento)
                 VALUES (?1, ?2, ?3, ?4, datetime('now','localtime','+'||?5||' days'))",
                rusqlite::params![
                    compra.proveedor_id,
                    compra_id,
                    total,
                    total,
                    dias,
                ],
            )
            .map_err(|e| e.to_string())?;
        } else {
            conn.execute(
                "INSERT INTO cuentas_por_pagar (proveedor_id, compra_id, monto_total, saldo)
                 VALUES (?1, ?2, ?3, ?4)",
                rusqlite::params![
                    compra.proveedor_id,
                    compra_id,
                    total,
                    total,
                ],
            )
            .map_err(|e| e.to_string())?;
        }
    }

    // Obtener nombre del proveedor
    let proveedor_nombre: Option<String> = conn
        .query_row(
            "SELECT nombre FROM proveedores WHERE id = ?1",
            rusqlite::params![compra.proveedor_id],
            |row| row.get(0),
        )
        .ok();

    // Obtener fecha generada
    let fecha: Option<String> = conn
        .query_row(
            "SELECT fecha FROM compras WHERE id = ?1",
            rusqlite::params![compra_id],
            |row| row.get(0),
        )
        .ok();

    let banco_id_saved = compra.banco_id;
    let referencia_pago_saved = compra.referencia_pago.clone();
    let banco_nombre_saved: Option<String> = if let Some(bid) = banco_id_saved {
        conn.query_row("SELECT nombre FROM cuentas_banco WHERE id = ?1", rusqlite::params![bid], |r| r.get(0)).ok()
    } else { None };

    Ok(CompraCompleta {
        compra: Compra {
            id: Some(compra_id),
            numero: numero.clone(),
            proveedor_id: compra.proveedor_id,
            fecha,
            numero_factura,
            subtotal: subtotal_total,
            iva: iva_total,
            total,
            estado: "REGISTRADA".to_string(),
            forma_pago: compra.forma_pago,
            es_credito: compra.es_credito,
            observacion: compra.observacion,
            proveedor_nombre,
            banco_id: banco_id_saved,
            referencia_pago: referencia_pago_saved,
            banco_nombre: banco_nombre_saved,
            tipo_documento: Some(tipo_documento),
            estado_sri: None,
            clave_acceso,
            fecha_emision: fecha_emision_norm,
            total_devuelto: 0.0,
        },
        detalles,
    })
}

#[tauri::command]
pub fn listar_compras(
    db: State<Database>,
    fecha_desde: Option<String>,
    fecha_hasta: Option<String>,
) -> Result<Vec<Compra>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;

    let sql = match (&fecha_desde, &fecha_hasta) {
        (Some(_), Some(_)) => {
            "SELECT c.id, c.numero, c.proveedor_id, c.fecha, c.numero_factura,
                    c.subtotal, c.iva, c.total, c.estado, c.forma_pago, c.es_credito,
                    c.observacion, p.nombre, c.banco_id, c.referencia_pago, b.nombre as banco_nombre,
                    COALESCE(c.tipo_documento, 'INFORMAL'), c.estado_sri, c.clave_acceso, c.fecha_emision,
                    COALESCE((SELECT SUM(total) FROM compra_devoluciones WHERE compra_id = c.id), 0)
             FROM compras c
             JOIN proveedores p ON c.proveedor_id = p.id
             LEFT JOIN cuentas_banco b ON c.banco_id = b.id
             WHERE date(c.fecha) >= date(?1) AND date(c.fecha) <= date(?2)
             ORDER BY c.fecha DESC"
        }
        _ => {
            "SELECT c.id, c.numero, c.proveedor_id, c.fecha, c.numero_factura,
                    c.subtotal, c.iva, c.total, c.estado, c.forma_pago, c.es_credito,
                    c.observacion, p.nombre, c.banco_id, c.referencia_pago, b.nombre as banco_nombre,
                    COALESCE(c.tipo_documento, 'INFORMAL'), c.estado_sri, c.clave_acceso, c.fecha_emision,
                    COALESCE((SELECT SUM(total) FROM compra_devoluciones WHERE compra_id = c.id), 0)
             FROM compras c
             JOIN proveedores p ON c.proveedor_id = p.id
             LEFT JOIN cuentas_banco b ON c.banco_id = b.id
             ORDER BY c.fecha DESC
             LIMIT 100"
        }
    };

    let mut stmt = conn.prepare(sql).map_err(|e| e.to_string())?;

    let map_row = |row: &rusqlite::Row| -> rusqlite::Result<Compra> {
        Ok(Compra {
            id: Some(row.get(0)?),
            numero: row.get(1)?,
            proveedor_id: row.get(2)?,
            fecha: row.get(3)?,
            numero_factura: row.get(4)?,
            subtotal: row.get(5)?,
            iva: row.get(6)?,
            total: row.get(7)?,
            estado: row.get(8)?,
            forma_pago: row.get(9)?,
            es_credito: row.get(10)?,
            observacion: row.get(11)?,
            proveedor_nombre: row.get(12)?,
            banco_id: row.get(13).ok(),
            referencia_pago: row.get(14).ok(),
            banco_nombre: row.get(15).ok(),
            tipo_documento: Some(row.get::<_, String>(16)?),
            estado_sri: row.get(17).ok(),
            clave_acceso: row.get(18).ok(),
            fecha_emision: row.get(19).ok(),
            total_devuelto: row.get(20).unwrap_or(0.0),
        })
    };

    let resultado = match (&fecha_desde, &fecha_hasta) {
        (Some(desde), Some(hasta)) => {
            stmt.query_map(rusqlite::params![desde, hasta], map_row)
                .map_err(|e| e.to_string())?
                .collect::<Result<Vec<_>, _>>()
                .map_err(|e| e.to_string())?
        }
        _ => {
            stmt.query_map([], map_row)
                .map_err(|e| e.to_string())?
                .collect::<Result<Vec<_>, _>>()
                .map_err(|e| e.to_string())?
        }
    };

    Ok(resultado)
}

#[tauri::command]
pub fn obtener_compra(db: State<Database>, id: i64) -> Result<CompraCompleta, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;

    let compra = conn
        .query_row(
            "SELECT c.id, c.numero, c.proveedor_id, c.fecha, c.numero_factura,
                    c.subtotal, c.iva, c.total, c.estado, c.forma_pago, c.es_credito,
                    c.observacion, p.nombre, c.banco_id, c.referencia_pago, b.nombre as banco_nombre,
                    COALESCE(c.tipo_documento, 'INFORMAL'), c.estado_sri, c.clave_acceso, c.fecha_emision,
                    COALESCE((SELECT SUM(total) FROM compra_devoluciones WHERE compra_id = c.id), 0)
             FROM compras c
             JOIN proveedores p ON c.proveedor_id = p.id
             LEFT JOIN cuentas_banco b ON c.banco_id = b.id
             WHERE c.id = ?1",
            rusqlite::params![id],
            |row| {
                Ok(Compra {
                    id: Some(row.get(0)?),
                    numero: row.get(1)?,
                    proveedor_id: row.get(2)?,
                    fecha: row.get(3)?,
                    numero_factura: row.get(4)?,
                    subtotal: row.get(5)?,
                    iva: row.get(6)?,
                    total: row.get(7)?,
                    estado: row.get(8)?,
                    forma_pago: row.get(9)?,
                    es_credito: row.get(10)?,
                    observacion: row.get(11)?,
                    proveedor_nombre: row.get(12)?,
                    banco_id: row.get(13).ok(),
                    referencia_pago: row.get(14).ok(),
                    banco_nombre: row.get(15).ok(),
                    tipo_documento: Some(row.get::<_, String>(16)?),
                    estado_sri: row.get(17).ok(),
                    clave_acceso: row.get(18).ok(),
                    fecha_emision: row.get(19).ok(),
                    total_devuelto: row.get(20).unwrap_or(0.0),
                })
            },
        )
        .map_err(|_| "Compra no encontrada".to_string())?;

    let mut stmt = conn
        .prepare(
            "SELECT cd.id, cd.compra_id, cd.producto_id, cd.descripcion,
                    cd.cantidad, cd.precio_unitario, cd.subtotal, p.nombre,
                    COALESCE(cd.cantidad_devuelta, 0)
             FROM compra_detalles cd
             LEFT JOIN productos p ON cd.producto_id = p.id
             WHERE cd.compra_id = ?1",
        )
        .map_err(|e| e.to_string())?;

    let detalles = stmt
        .query_map(rusqlite::params![id], |row| {
            Ok(CompraDetalle {
                id: Some(row.get(0)?),
                compra_id: row.get(1)?,
                producto_id: row.get(2)?,
                descripcion: row.get(3)?,
                cantidad: row.get(4)?,
                precio_unitario: row.get(5)?,
                subtotal: row.get(6)?,
                nombre_producto: row.get(7)?,
                cantidad_devuelta: row.get(8).unwrap_or(0.0),
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    Ok(CompraCompleta { compra, detalles })
}

/// v2.5.42: helper de trazabilidad — para un item de compra (producto_id, cantidad_comprada,
/// cantidad_devuelta), calcula cuántas unidades se pueden todavía devolver o anular sin
/// generar stock negativo. Fórmula:
///     pendiente_de_la_compra = cantidad_comprada - cantidad_devuelta
///     stock_actual = lo que el producto tiene ahora
///     devolvible = min(pendiente_de_la_compra, stock_actual)
///
/// Retorna (devolvible, vendido_post_compra) donde vendido_post_compra =
/// pendiente_de_la_compra - devolvible (lo que se vendió y por tanto no se puede devolver).
fn calcular_devolvible(
    conn: &rusqlite::Connection,
    producto_id: i64,
    cantidad_comprada: f64,
    cantidad_devuelta: f64,
) -> (f64, f64) {
    let pendiente = (cantidad_comprada - cantidad_devuelta).max(0.0);
    let stock_actual: f64 = conn.query_row(
        "SELECT stock_actual FROM productos WHERE id = ?1",
        rusqlite::params![producto_id], |r| r.get(0),
    ).unwrap_or(0.0);
    let devolvible = pendiente.min(stock_actual.max(0.0));
    let vendido = (pendiente - devolvible).max(0.0);
    (devolvible, vendido)
}

#[tauri::command]
pub fn anular_compra(
    db: State<Database>,
    sesion: State<SesionState>,
    id: i64,
    motivo: Option<String>,
    forzar_stock_negativo: Option<bool>,
) -> Result<serde_json::Value, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let usuario = usuario_actual(&sesion);
    let forzar = forzar_stock_negativo.unwrap_or(false);

    // Verificar que la compra existe y no está ya anulada
    let (estado, numero, total_devuelto): (String, String, f64) = conn
        .query_row(
            "SELECT estado, numero,
                    COALESCE((SELECT SUM(total) FROM compra_devoluciones WHERE compra_id = compras.id), 0)
             FROM compras WHERE id = ?1",
            rusqlite::params![id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .map_err(|_| "Compra no encontrada".to_string())?;

    if estado == "ANULADA" {
        return Err("La compra ya está anulada".to_string());
    }

    // v2.5.30: no se puede anular si ya tiene devoluciones aplicadas
    if total_devuelto > 0.001 {
        return Err(format!(
            "No se puede anular: esta compra ya tiene devoluciones aplicadas por ${:.2}. Reverse las devoluciones primero o use Devolver Total en su lugar.",
            total_devuelto
        ));
    }

    // v2.5.42: validar trazabilidad — items ya vendidos no permiten anular sin override
    let mut stmt = conn
        .prepare(
            "SELECT cd.producto_id, cd.cantidad, COALESCE(p.nombre, '?'),
                    COALESCE(cd.cantidad_devuelta, 0)
             FROM compra_detalles cd
             LEFT JOIN productos p ON cd.producto_id = p.id
             WHERE cd.compra_id = ?1 AND cd.producto_id IS NOT NULL",
        )
        .map_err(|e| e.to_string())?;

    let detalles: Vec<(i64, f64, String, f64)> = stmt
        .query_map(rusqlite::params![id], |row| {
            Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?))
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    // Detectar items con stock_actual menor a la cantidad que habría que revertir
    let mut conflictos: Vec<(String, f64, f64)> = Vec::new(); // (nombre, vendido, devolvible)
    for (pid, cant_comprada, nombre, cant_dev) in &detalles {
        let (devolvible, vendido) = calcular_devolvible(&conn, *pid, *cant_comprada, *cant_dev);
        if vendido > 0.001 {
            conflictos.push((nombre.clone(), vendido, devolvible));
        }
    }

    // Si hay conflictos y NO se forza, bloquear con mensaje detallado
    if !conflictos.is_empty() && !forzar {
        // Chequear flag global de config: permitir_anulacion_stock_negativo
        let permitido_global: bool = conn.query_row(
            "SELECT value FROM config WHERE key = 'permitir_anulacion_stock_negativo'",
            [], |r| r.get::<_, String>(0),
        ).map(|v| v == "1").unwrap_or(false);

        if !permitido_global {
            let detalle_str = conflictos.iter()
                .map(|(n, vendido, devolvible)| format!("  - {}: vendiste {} unidad(es), solo puedes devolver {}", n, vendido, devolvible))
                .collect::<Vec<_>>().join("\n");
            return Err(format!(
                "TRAZABILIDAD: No se puede anular completa porque algunos items YA SE VENDIERON:\n{}\n\nOpciones:\n  1) Usa 'Devolver' para regresar SOLO las cantidades disponibles\n  2) Activa 'Permitir anulación con stock negativo' en Configuración (admin)\n  3) Reenvía con la opción 'Forzar' marcada (admin)",
                detalle_str
            ));
        }
    }

    let motivo_str = motivo.unwrap_or_else(|| "Sin motivo".to_string());
    let mut items_con_negativo = 0i64;
    for (producto_id, cantidad, _nombre, _cant_dev) in detalles {
        // Stock antes (para kardex)
        let stock_antes: f64 = conn.query_row(
            "SELECT stock_actual FROM productos WHERE id = ?1",
            rusqlite::params![producto_id], |r| r.get(0),
        ).unwrap_or(0.0);

        conn.execute(
            "UPDATE productos SET stock_actual = stock_actual - ?1,
             updated_at = datetime('now','localtime') WHERE id = ?2",
            rusqlite::params![cantidad, producto_id],
        )
        .map_err(|e| e.to_string())?;

        let stock_nuevo = stock_antes - cantidad;
        if stock_nuevo < 0.0 { items_con_negativo += 1; }

        // v2.5.30: kardex inverso. v2.5.42: marca si quedó stock negativo
        let motivo_kardex = if stock_nuevo < 0.0 {
            format!("Anulacion compra {} - {} ⚠ STOCK NEGATIVO (items vendidos)", numero, motivo_str)
        } else {
            format!("Anulacion compra {} - {}", numero, motivo_str)
        };
        let _ = conn.execute(
            "INSERT INTO movimientos_inventario (producto_id, tipo, cantidad, stock_anterior, stock_nuevo, costo_unitario, referencia_id, usuario, motivo)
             VALUES (?1, 'ANULACION_COMPRA', ?2, ?3, ?4, 0, ?5, ?6, ?7)",
            rusqlite::params![producto_id, -cantidad, stock_antes, stock_nuevo, id, usuario, motivo_kardex],
        );
    }

    // Marcar compra como anulada (guardar motivo en observacion)
    conn.execute(
        "UPDATE compras SET estado = 'ANULADA',
                            observacion = COALESCE(observacion || ' · ', '') || 'ANULADA: ' || ?2
         WHERE id = ?1",
        rusqlite::params![id, motivo_str],
    )
    .map_err(|e| e.to_string())?;

    // Anular cuenta por pagar asociada si existe
    conn.execute(
        "UPDATE cuentas_por_pagar SET estado = 'ANULADA' WHERE compra_id = ?1 AND estado = 'PENDIENTE'",
        rusqlite::params![id],
    )
    .map_err(|e| e.to_string())?;

    Ok(serde_json::json!({
        "anulada": true,
        "items_con_stock_negativo": items_con_negativo,
        "advertencia": if items_con_negativo > 0 {
            format!("Anulación forzada: {} producto(s) quedó/quedaron con stock negativo porque ya se habían vendido. Revisa el kardex.", items_con_negativo)
        } else { String::new() },
    }))
}

// ================================================================
// Importación de XML de Factura Electrónica (SRI Ecuador)
// ================================================================

#[derive(serde::Serialize)]
pub struct PreviewXmlCompra {
    pub proveedor_ruc: String,
    pub proveedor_nombre: String,
    pub proveedor_existe: bool,
    pub proveedor_id: Option<i64>,
    pub numero_factura: String,
    pub fecha_emision: String,
    pub clave_acceso: String,
    pub subtotal_0: f64,
    pub subtotal_15: f64,
    pub iva: f64,
    pub total: f64,
    pub items: Vec<PreviewItemXml>,
    /// v2.5.30: si el XML viene envuelto en <autorizacion><estado>AUTORIZADO</estado>
    /// significa que el SRI ya validó esta factura. Si no, es un XML sin firma o no autorizada.
    pub autorizada: bool,
    /// v2.5.30: estado SRI exacto leido del XML ("AUTORIZADO", "PPR", "RECHAZADO", etc.)
    pub estado_sri: Option<String>,
    /// v2.5.30: si la clave_acceso ya existe en otra compra registrada, devuelve su id
    pub compra_duplicada_id: Option<i64>,
}

#[derive(serde::Serialize)]
pub struct PreviewItemXml {
    pub codigo_principal: Option<String>,
    pub descripcion: String,
    pub cantidad: f64,
    pub precio_unitario: f64,
    pub descuento: f64,
    pub iva_porcentaje: f64,
    pub subtotal: f64,
    pub producto_existente_id: Option<i64>,
    pub producto_existente_nombre: Option<String>,
}

#[tauri::command]
pub fn preview_xml_compra(
    db: State<Database>,
    xml_contenido: String,
) -> Result<PreviewXmlCompra, String> {
    use quick_xml::events::Event;
    use quick_xml::reader::Reader;

    // v2.5.30: detectar si el XML viene envuelto en <autorizacion><estado>AUTORIZADO</estado>
    // (formato de autorización del SRI). Si sí → factura autorizada legítima.
    // Si no → puede ser un XML sin autorizar / generado / manipulado → tratar como NOTA_VENTA.
    let estado_sri_xml: Option<String> = {
        let bytes = xml_contenido.as_bytes();
        let mut reader = Reader::from_reader(bytes);
        reader.config_mut().trim_text(true);
        let mut buf = Vec::new();
        let mut inside_estado = false;
        let mut inside_autorizacion = false;
        let mut estado_text = String::new();
        loop {
            match reader.read_event_into(&mut buf) {
                Ok(Event::Start(e)) => {
                    let name = String::from_utf8_lossy(e.name().as_ref()).to_string();
                    if name == "autorizacion" { inside_autorizacion = true; }
                    if name == "estado" && inside_autorizacion { inside_estado = true; }
                }
                Ok(Event::End(e)) => {
                    let name = String::from_utf8_lossy(e.name().as_ref()).to_string();
                    if name == "estado" { inside_estado = false; }
                    if name == "autorizacion" { inside_autorizacion = false; }
                }
                Ok(Event::Text(t)) => {
                    if inside_estado {
                        if let Ok(s) = t.unescape() { estado_text.push_str(&s); }
                    }
                }
                Ok(Event::Eof) => break,
                Err(_) => break,
                _ => {}
            }
            buf.clear();
        }
        let t = estado_text.trim().to_string();
        if t.is_empty() { None } else { Some(t) }
    };
    let autorizada = estado_sri_xml.as_deref().map(|s| s.eq_ignore_ascii_case("AUTORIZADO")).unwrap_or(false);

    // Algunos XMLs del SRI envuelven la factura dentro de <autorizacion><comprobante><![CDATA[...]]></comprobante>
    // Intentamos detectar esto y desenrollar el contenido real de la factura.
    let xml_real: String = {
        let bytes = xml_contenido.as_bytes();
        let mut reader = Reader::from_reader(bytes);
        reader.config_mut().trim_text(true);
        let mut buf = Vec::new();
        let mut inside_comprobante = false;
        let mut found: Option<String> = None;
        loop {
            match reader.read_event_into(&mut buf) {
                Ok(Event::Start(e)) => {
                    if e.name().as_ref() == b"comprobante" {
                        inside_comprobante = true;
                    }
                }
                Ok(Event::End(e)) => {
                    if e.name().as_ref() == b"comprobante" {
                        inside_comprobante = false;
                    }
                }
                Ok(Event::CData(c)) => {
                    if inside_comprobante {
                        if let Ok(s) = std::str::from_utf8(c.as_ref()) {
                            found = Some(s.to_string());
                            break;
                        }
                    }
                }
                Ok(Event::Text(t)) => {
                    if inside_comprobante {
                        if let Ok(s) = t.unescape() {
                            let trimmed = s.trim();
                            if trimmed.starts_with("<factura") || trimmed.starts_with("<?xml") {
                                found = Some(trimmed.to_string());
                                break;
                            }
                        }
                    }
                }
                Ok(Event::Eof) => break,
                Err(_) => break,
                _ => {}
            }
            buf.clear();
        }
        found.unwrap_or(xml_contenido.clone())
    };

    let mut reader = Reader::from_str(&xml_real);
    reader.config_mut().trim_text(true);

    let mut buf = Vec::new();
    let mut path_stack: Vec<String> = Vec::new();
    let mut current_text = String::new();

    let mut proveedor_ruc = String::new();
    let mut proveedor_nombre = String::new();
    let mut estab = String::new();
    let mut pto_emi = String::new();
    let mut secuencial = String::new();
    let mut fecha_emision = String::new();
    let mut clave_acceso = String::new();
    let mut subtotal_0 = 0.0_f64;
    let mut subtotal_15 = 0.0_f64;
    let mut iva_total = 0.0_f64;
    let mut importe_total: f64 = 0.0;

    let mut items: Vec<PreviewItemXml> = Vec::new();
    let mut current_item: Option<PreviewItemXml> = None;

    // Para impuestos dentro de <detalle><impuestos><impuesto>
    let mut current_imp_codigo: i64 = 0;
    let mut current_imp_tarifa: f64 = 0.0;

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) => {
                let name = String::from_utf8_lossy(e.name().as_ref()).to_string();
                path_stack.push(name.clone());
                current_text.clear();
                if name == "detalle" {
                    current_item = Some(PreviewItemXml {
                        codigo_principal: None,
                        descripcion: String::new(),
                        cantidad: 0.0,
                        precio_unitario: 0.0,
                        descuento: 0.0,
                        iva_porcentaje: 0.0,
                        subtotal: 0.0,
                        producto_existente_id: None,
                        producto_existente_nombre: None,
                    });
                }
                if name == "impuesto" {
                    current_imp_codigo = 0;
                    current_imp_tarifa = 0.0;
                }
            }
            Ok(Event::Text(e)) => {
                if let Ok(s) = e.unescape() {
                    current_text.push_str(&s);
                }
            }
            Ok(Event::CData(c)) => {
                if let Ok(s) = std::str::from_utf8(c.as_ref()) {
                    current_text.push_str(s);
                }
            }
            Ok(Event::End(e)) => {
                let name = String::from_utf8_lossy(e.name().as_ref()).to_string();
                let val = current_text.trim().to_string();

                let in_detalle = path_stack.iter().any(|s| s == "detalle");
                let in_info_factura = path_stack.iter().any(|s| s == "infoFactura");
                let in_info_tributaria = path_stack.iter().any(|s| s == "infoTributaria");
                let in_total_impuestos =
                    path_stack.iter().any(|s| s == "totalConImpuestos");
                let in_detalle_impuesto = in_detalle && path_stack.iter().any(|s| s == "impuesto");

                if !in_detalle {
                    // Cabecera
                    if in_info_tributaria {
                        match name.as_str() {
                            "ruc" => {
                                if proveedor_ruc.is_empty() {
                                    proveedor_ruc = val.clone();
                                }
                            }
                            "razonSocial" => {
                                if proveedor_nombre.is_empty() {
                                    proveedor_nombre = val.clone();
                                }
                            }
                            "nombreComercial" => {
                                if proveedor_nombre.is_empty() {
                                    proveedor_nombre = val.clone();
                                }
                            }
                            "estab" => estab = val.clone(),
                            "ptoEmi" => pto_emi = val.clone(),
                            "secuencial" => secuencial = val.clone(),
                            "claveAcceso" => clave_acceso = val.clone(),
                            _ => {}
                        }
                    }
                    if in_info_factura {
                        match name.as_str() {
                            "fechaEmision" => {
                                if fecha_emision.is_empty() {
                                    fecha_emision = val.clone();
                                }
                            }
                            "importeTotal" => {
                                importe_total = val.parse().unwrap_or(importe_total);
                            }
                            _ => {}
                        }
                    }
                    // Impuestos del totalConImpuestos (sumatoria IVA global por si la necesitamos)
                    if in_total_impuestos && name == "valor" {
                        // No se usa; calculamos desde items
                    }
                }

                if in_detalle {
                    if let Some(item) = current_item.as_mut() {
                        // Impuesto dentro del detalle
                        if in_detalle_impuesto {
                            match name.as_str() {
                                "codigo" => {
                                    current_imp_codigo = val.parse().unwrap_or(0);
                                }
                                "tarifa" => {
                                    current_imp_tarifa = val.parse().unwrap_or(0.0);
                                }
                                "impuesto" => {
                                    // Fin de un impuesto: si es IVA (codigo 2) guardamos la tarifa
                                    if current_imp_codigo == 2 {
                                        item.iva_porcentaje = current_imp_tarifa;
                                    }
                                }
                                _ => {}
                            }
                        } else {
                            match name.as_str() {
                                "codigoPrincipal" => {
                                    item.codigo_principal = Some(val.clone());
                                }
                                "codigoAuxiliar" => {
                                    if item.codigo_principal.is_none() {
                                        item.codigo_principal = Some(val.clone());
                                    }
                                }
                                "descripcion" => item.descripcion = val.clone(),
                                "cantidad" => {
                                    item.cantidad = val.parse().unwrap_or(0.0);
                                }
                                "precioUnitario" => {
                                    item.precio_unitario = val.parse().unwrap_or(0.0);
                                }
                                "descuento" => {
                                    item.descuento = val.parse().unwrap_or(0.0);
                                }
                                "precioTotalSinImpuesto" => {
                                    item.subtotal = val.parse().unwrap_or(0.0);
                                }
                                _ => {}
                            }
                        }
                    }
                }

                if name == "detalle" {
                    if let Some(mut item) = current_item.take() {
                        // Buscar producto existente por código
                        if let Some(cod) = &item.codigo_principal {
                            if let Ok(conn) = db.conn.lock() {
                                let res: Result<(i64, String), _> = conn.query_row(
                                    "SELECT id, nombre FROM productos WHERE codigo = ?1 OR codigo_barras = ?1 LIMIT 1",
                                    rusqlite::params![cod],
                                    |row| Ok((row.get(0)?, row.get(1)?)),
                                );
                                if let Ok((id, nom)) = res {
                                    item.producto_existente_id = Some(id);
                                    item.producto_existente_nombre = Some(nom);
                                }
                            }
                        }
                        // Acumular subtotales por tarifa
                        if item.iva_porcentaje > 0.0 {
                            subtotal_15 += item.subtotal;
                            iva_total += item.subtotal * (item.iva_porcentaje / 100.0);
                        } else {
                            subtotal_0 += item.subtotal;
                        }
                        items.push(item);
                    }
                }

                path_stack.pop();
                current_text.clear();
            }
            Ok(Event::Eof) => break,
            Err(e) => return Err(format!("Error parseando XML: {}", e)),
            _ => {}
        }
        buf.clear();
    }

    // Construir numero_factura
    let numero_factura = if !estab.is_empty() && !pto_emi.is_empty() && !secuencial.is_empty() {
        format!("{}-{}-{}", estab, pto_emi, secuencial)
    } else {
        secuencial.clone()
    };

    let total = if importe_total > 0.0 {
        importe_total
    } else {
        subtotal_0 + subtotal_15 + iva_total
    };

    // Verificar si proveedor existe
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let proveedor_existente: Option<i64> = conn
        .query_row(
            "SELECT id FROM proveedores WHERE ruc = ?1 LIMIT 1",
            rusqlite::params![&proveedor_ruc],
            |row| row.get(0),
        )
        .ok();

    // v2.5.30: chequear si esta clave_acceso ya fue importada previamente
    let compra_duplicada_id: Option<i64> = if !clave_acceso.is_empty() {
        conn.query_row(
            "SELECT id FROM compras WHERE clave_acceso = ?1 AND estado != 'ANULADA' LIMIT 1",
            rusqlite::params![&clave_acceso], |r| r.get(0),
        ).ok()
    } else { None };

    Ok(PreviewXmlCompra {
        proveedor_ruc,
        proveedor_nombre,
        proveedor_existe: proveedor_existente.is_some(),
        proveedor_id: proveedor_existente,
        numero_factura,
        fecha_emision,
        clave_acceso,
        subtotal_0,
        subtotal_15,
        iva: iva_total,
        total,
        items,
        autorizada,
        estado_sri: estado_sri_xml,
        compra_duplicada_id,
    })
}

#[derive(serde::Deserialize)]
pub struct NuevoProductoSimple {
    pub codigo: Option<String>,
    pub nombre: String,
    pub categoria_id: Option<i64>,
    pub iva_porcentaje: f64,
}

#[derive(serde::Deserialize)]
pub struct ItemMapeado {
    pub accion: String, // "producto_nuevo" | "producto_existente" | "gasto" | "ignorar"
    pub producto_id: Option<i64>,
    pub producto_nuevo: Option<NuevoProductoSimple>,
    pub gasto_categoria: Option<String>,
    pub descripcion: String,
    pub cantidad: f64,
    pub precio_unitario: f64,
    pub iva_porcentaje: f64,
    pub subtotal: f64,
}

#[derive(serde::Deserialize)]
pub struct ImportarXmlInput {
    pub proveedor_id: i64,
    pub numero_factura: String,
    pub fecha_emision: String,
    pub items_mapeados: Vec<ItemMapeado>,
    pub forma_pago: String, // "EFECTIVO" | "CREDITO" | "TRANSFERENCIA" | "DEBITO" | "CHEQUE"
    pub dias_credito: Option<i64>,
    #[serde(default)]
    pub banco_id: Option<i64>,
    #[serde(default)]
    pub referencia_pago: Option<String>,
    /// v2.5.30: del XML — si vino dentro de <autorizacion><estado>AUTORIZADO</estado>
    /// el frontend lo pasa como true y se registra como FACTURA + estado_sri=AUTORIZADA.
    /// Si false → NOTA_VENTA (sin validez tributaria de soporte).
    #[serde(default)]
    pub autorizada: bool,
    /// Clave de acceso SRI (49 dig) — clave única que evita doble importación
    #[serde(default)]
    pub clave_acceso: Option<String>,
}

#[tauri::command]
pub fn importar_xml_compra(
    db: State<Database>,
    sesion: State<SesionState>,
    input: ImportarXmlInput,
) -> Result<serde_json::Value, String> {
    let mut conn = db.conn.lock().map_err(|e| e.to_string())?;
    let usuario = usuario_actual(&sesion);

    // v2.5.30: determinar tipo_documento por estado de autorización del XML
    let tipo_doc_xml = if input.autorizada { "FACTURA" } else { "NOTA_VENTA" };
    let estado_sri_xml = if input.autorizada { Some("AUTORIZADA".to_string()) } else { None };
    let clave_acceso_norm: Option<String> = input.clave_acceso.as_ref()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());

    // v2.5.30: validar duplicado ANTES de empezar la transacción
    validar_factura_unica(
        &conn, input.proveedor_id,
        Some(input.numero_factura.trim()), tipo_doc_xml,
        clave_acceso_norm.as_deref(), None,
    )?;

    let tx = conn.transaction().map_err(|e| e.to_string())?;

    // (producto_id, cantidad, precio_unitario, iva_porcentaje, subtotal, descripcion)
    let mut items_compra: Vec<(i64, f64, f64, f64, f64, String)> = Vec::new();
    let mut gastos_creados = 0_i64;
    let mut productos_creados = 0_i64;

    for item in &input.items_mapeados {
        match item.accion.as_str() {
            "producto_nuevo" => {
                if let Some(np) = &item.producto_nuevo {
                    let cod = match np.codigo.as_ref().map(|s| s.trim().to_string()) {
                        Some(c) if !c.is_empty() => c,
                        _ => {
                            let n: i64 = tx
                                .query_row(
                                    "SELECT COALESCE(MAX(CAST(REPLACE(codigo, 'P', '') AS INTEGER)), 0) + 1 FROM productos WHERE codigo LIKE 'P%'",
                                    [],
                                    |r| r.get(0),
                                )
                                .unwrap_or(1);
                            format!("P{:04}", n)
                        }
                    };
                    let nombre = if np.nombre.trim().is_empty() {
                        item.descripcion.clone()
                    } else {
                        np.nombre.clone()
                    };
                    tx.execute(
                        "INSERT INTO productos (codigo, nombre, categoria_id, precio_costo, precio_venta, iva_porcentaje, incluye_iva, stock_actual, stock_minimo, unidad_medida, es_servicio, activo) \
                         VALUES (?1, ?2, ?3, ?4, ?5, ?6, 0, 0, 0, 'UND', 0, 1)",
                        rusqlite::params![
                            cod,
                            nombre,
                            np.categoria_id,
                            item.precio_unitario,
                            item.precio_unitario * 1.3,
                            np.iva_porcentaje,
                        ],
                    )
                    .map_err(|e| format!("Error creando producto '{}': {}", nombre, e))?;
                    let pid = tx.last_insert_rowid();
                    items_compra.push((
                        pid,
                        item.cantidad,
                        item.precio_unitario,
                        item.iva_porcentaje,
                        item.subtotal,
                        nombre,
                    ));
                    productos_creados += 1;
                }
            }
            "producto_existente" => {
                if let Some(pid) = item.producto_id {
                    items_compra.push((
                        pid,
                        item.cantidad,
                        item.precio_unitario,
                        item.iva_porcentaje,
                        item.subtotal,
                        item.descripcion.clone(),
                    ));
                }
            }
            "gasto" => {
                let cat = item
                    .gasto_categoria
                    .clone()
                    .unwrap_or_else(|| "Compra proveedor".to_string());
                let monto_gasto =
                    item.subtotal + (item.subtotal * item.iva_porcentaje / 100.0);
                // v2.5.32 BUG FIX: convertir fecha SRI (dd/mm/yyyy) a ISO antes
                // de insertar. Sin esto el listar_gastos_dia filtraba por
                // date(g.fecha) y SQLite no parseaba dd/mm/yyyy → gasto invisible.
                let fecha_gasto = convertir_fecha_sri(&input.fecha_emision)
                    .unwrap_or_else(|| chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string());
                // v2.5.32: grabar clave_acceso + numero_factura_xml + proveedor_id en gastos
                // para que validar_factura_unica detecte reimportación del mismo XML
                // aunque la primera vez haya sido como gasto (sin compra creada).
                tx.execute(
                    "INSERT INTO gastos (descripcion, monto, categoria, observacion, fecha,
                                         clave_acceso, numero_factura_xml, proveedor_id) \
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                    rusqlite::params![
                        item.descripcion,
                        monto_gasto,
                        cat,
                        format!("Importado de XML factura: {}", input.numero_factura),
                        fecha_gasto,
                        clave_acceso_norm,
                        input.numero_factura.trim(),
                        input.proveedor_id,
                    ],
                )
                .map_err(|e| {
                    let msg = e.to_string();
                    if msg.contains("UNIQUE") && msg.contains("clave_acceso") {
                        "Este XML ya fue importado anteriormente como gasto".to_string()
                    } else { msg }
                })?;
                gastos_creados += 1;
            }
            _ => {}
        }
    }

    // Crear compra solo si hay items de producto
    let compra_id: Option<i64> = if !items_compra.is_empty() {
        // v2.5.30: usar el mismo helper (formato COMP-XXXXXXXXX)
        let numero_compra = proximo_numero_compra(&tx);

        let subtotal: f64 = items_compra.iter().map(|i| i.4).sum();
        let iva_total: f64 = items_compra
            .iter()
            .map(|(_, _, _, iva_p, sub, _)| sub * iva_p / 100.0)
            .sum();
        let total = subtotal + iva_total;

        let es_credito = input.forma_pago == "CREDITO";
        let forma_pago_db = if es_credito {
            "EFECTIVO".to_string() // la forma fiscal se pagará al liquidar
        } else {
            input.forma_pago.clone()
        };

        // fecha: si viene en formato dd/mm/yyyy la convertimos a ISO
        let fecha_iso = convertir_fecha_sri(&input.fecha_emision);

        // Validar banco si forma de pago requiere cuenta bancaria
        let req_banco = matches!(forma_pago_db.as_str(), "DEBITO" | "TRANSFERENCIA" | "CHEQUE");
        if req_banco && input.banco_id.is_none() {
            return Err("Debe seleccionar una cuenta bancaria para esta forma de pago".into());
        }

        let observacion_xml = if input.autorizada {
            "Importado desde XML SRI autorizado"
        } else {
            "Importado desde XML (no autorizado por SRI)"
        };
        tx.execute(
            "INSERT INTO compras (numero, proveedor_id, fecha, numero_factura, subtotal, iva, total, estado,
                                  forma_pago, es_credito, observacion, banco_id, referencia_pago,
                                  tipo_documento, estado_sri, clave_acceso, fecha_emision, usuario) \
             VALUES (?1, ?2, COALESCE(?3, datetime('now','localtime')), ?4, ?5, ?6, ?7, 'REGISTRADA',
                     ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17)",
            rusqlite::params![
                numero_compra,
                input.proveedor_id,
                fecha_iso,
                input.numero_factura,
                subtotal,
                iva_total,
                total,
                forma_pago_db,
                es_credito as i64,
                observacion_xml,
                input.banco_id,
                input.referencia_pago,
                tipo_doc_xml,
                estado_sri_xml,
                clave_acceso_norm,
                fecha_iso,
                usuario,
            ],
        )
        .map_err(|e| {
            let msg = e.to_string();
            if msg.contains("UNIQUE") && msg.contains("clave_acceso") {
                "Esta factura ya fue importada (clave de acceso SRI duplicada)".to_string()
            } else if msg.contains("UNIQUE") {
                format!("Ya existe una compra de este proveedor con ese numero de factura: {}", input.numero_factura)
            } else { msg }
        })?;
        let cid = tx.last_insert_rowid();

        // Insertar detalles y actualizar stock/costo
        for (pid, cant, precio, _iva_p, sub, desc) in &items_compra {
            tx.execute(
                "INSERT INTO compra_detalles (compra_id, producto_id, descripcion, cantidad, precio_unitario, subtotal) \
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                rusqlite::params![cid, pid, desc, cant, precio, sub],
            )
            .map_err(|e| e.to_string())?;
            // v2.5.22 PMP: recalcular costo_promedio antes del UPDATE (igual que registrar_compra)
            let (stock_actual_pmp, costo_prom_prev): (f64, f64) = tx
                .query_row(
                    "SELECT stock_actual, COALESCE(costo_promedio, precio_costo) FROM productos WHERE id = ?1",
                    rusqlite::params![pid],
                    |row| Ok((row.get::<_, f64>(0)?, row.get::<_, f64>(1)?)),
                )
                .unwrap_or((0.0, *precio));
            let nuevo_pmp = if stock_actual_pmp <= 0.0 {
                *precio
            } else {
                let total_after = stock_actual_pmp + cant;
                if total_after <= 0.0 { *precio } else {
                    (stock_actual_pmp * costo_prom_prev + cant * precio) / total_after
                }
            };
            tx.execute(
                "UPDATE productos SET stock_actual = stock_actual + ?1, precio_costo = ?2, costo_promedio = ?3, updated_at = datetime('now','localtime') WHERE id = ?4",
                rusqlite::params![cant, precio, nuevo_pmp, pid],
            )
            .ok();
            // v2.5.30: registrar kardex INGRESO_COMPRA tambien para importacion XML
            let motivo_xml = format!("Compra {} - {} (XML SRI {})", numero_compra, desc,
                if input.autorizada { "autorizada" } else { "sin autorizar" });
            let _ = tx.execute(
                "INSERT INTO movimientos_inventario (producto_id, tipo, cantidad, stock_anterior, stock_nuevo, costo_unitario, referencia_id, usuario, motivo)
                 VALUES (?1, 'INGRESO_COMPRA', ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                rusqlite::params![pid, cant, stock_actual_pmp, stock_actual_pmp + cant, precio, cid, usuario, motivo_xml],
            );
        }

        // Cuenta por pagar si crédito
        if es_credito {
            let dias = input.dias_credito.unwrap_or(30);
            tx.execute(
                "INSERT INTO cuentas_por_pagar (proveedor_id, compra_id, monto_total, monto_pagado, saldo, estado, fecha_vencimiento) \
                 VALUES (?1, ?2, ?3, 0, ?3, 'PENDIENTE', datetime('now','localtime','+'||?4||' days'))",
                rusqlite::params![input.proveedor_id, cid, total, dias],
            )
            .ok();
        }
        Some(cid)
    } else {
        None
    };

    tx.commit().map_err(|e| e.to_string())?;

    Ok(serde_json::json!({
        "compra_id": compra_id,
        "productos_creados": productos_creados,
        "gastos_creados": gastos_creados,
        "items_compra": items_compra.len()
    }))
}

/// Convierte fechas SRI (dd/mm/yyyy) a formato ISO (yyyy-mm-dd hh:mm:ss).
/// Si ya viene en ISO o no se puede parsear, la retorna tal cual (o None si vacía).
fn convertir_fecha_sri(fecha: &str) -> Option<String> {
    let f = fecha.trim();
    if f.is_empty() {
        return None;
    }
    // dd/mm/yyyy
    if f.len() == 10 && f.chars().nth(2) == Some('/') && f.chars().nth(5) == Some('/') {
        let parts: Vec<&str> = f.split('/').collect();
        if parts.len() == 3 {
            return Some(format!(
                "{}-{}-{} 00:00:00",
                parts[2], parts[1], parts[0]
            ));
        }
    }
    Some(f.to_string())
}

// ════════════════════════════════════════════════════════════════════
// v2.5.30: Devoluciones de Compra (parcial o total)
// ════════════════════════════════════════════════════════════════════

#[derive(serde::Deserialize)]
pub struct ItemDevolucion {
    pub compra_detalle_id: i64,
    pub cantidad: f64,
}

#[derive(serde::Deserialize)]
pub struct NuevaDevolucionCompra {
    pub compra_id: i64,
    pub items: Vec<ItemDevolucion>,
    #[serde(default)]
    pub motivo: Option<String>,
    #[serde(default)]
    pub observacion: Option<String>,
    /// Si true, ignora items y devuelve TODO lo restante (devolucion total)
    #[serde(default)]
    pub devolver_todo: bool,
    /// v2.5.35: datos del comprobante NC del proveedor (opcionales)
    /// Si el proveedor emitio NC SRI, se ingresan aqui (manualmente o via XML import)
    #[serde(default)]
    pub numero_nc: Option<String>,
    #[serde(default)]
    pub clave_acceso_nc: Option<String>,
    #[serde(default)]
    pub fecha_emision_nc: Option<String>,
    /// Si la NC fue importada desde XML SRI autorizado, set "AUTORIZADA". Sino None.
    #[serde(default)]
    pub estado_sri_nc: Option<String>,
    /// XML firmado original (solo si vino de import_xml_nc_compra)
    #[serde(default)]
    pub xml_nc_firmado: Option<String>,
    /// v2.5.42: tipo de NC del proveedor — "MERCANCIA" (default, revierte stock)
    /// o "AJUSTE_PRECIO" (no toca stock, solo ajusta precio_costo + saldo CXP)
    #[serde(default)]
    pub tipo_nc: Option<String>,
    /// v2.5.42: override para permitir devolver con stock negativo (admin)
    #[serde(default)]
    pub forzar_stock_negativo: bool,
}

#[derive(serde::Serialize)]
pub struct DevolucionCompraInfo {
    pub id: i64,
    pub compra_id: i64,
    pub numero: String,
    pub fecha: String,
    pub motivo: Option<String>,
    pub subtotal: f64,
    pub iva: f64,
    pub total: f64,
    pub es_total: bool,
    pub usuario: Option<String>,
    pub observacion: Option<String>,
    /// v2.5.35: datos del comprobante NC del proveedor (si vinieron)
    pub numero_nc: Option<String>,
    pub clave_acceso_nc: Option<String>,
    pub estado_sri_nc: Option<String>,
    pub fecha_emision_nc: Option<String>,
    /// v2.5.42: MERCANCIA (revierte stock) o AJUSTE_PRECIO (no toca stock)
    pub tipo_nc: Option<String>,
}

#[tauri::command]
pub fn registrar_devolucion_compra(
    db: State<Database>,
    sesion: State<SesionState>,
    input: NuevaDevolucionCompra,
) -> Result<serde_json::Value, String> {
    let mut conn = db.conn.lock().map_err(|e| e.to_string())?;
    let usuario = usuario_actual(&sesion);

    // Validar compra existente y no anulada
    let (compra_numero, compra_estado): (String, String) = conn.query_row(
        "SELECT numero, estado FROM compras WHERE id = ?1",
        rusqlite::params![input.compra_id], |r| Ok((r.get(0)?, r.get(1)?)),
    ).map_err(|_| "Compra no encontrada".to_string())?;
    if compra_estado == "ANULADA" {
        return Err("No se puede devolver una compra anulada".into());
    }

    // Cargar detalles con cantidad_devuelta acumulada para validacion
    let detalles_db: Vec<(i64, Option<i64>, Option<String>, f64, f64, f64, f64)> = {
        let mut stmt = conn.prepare(
            "SELECT cd.id, cd.producto_id, COALESCE(cd.descripcion, p.nombre), cd.cantidad,
                    cd.precio_unitario, cd.subtotal, COALESCE(cd.cantidad_devuelta, 0)
             FROM compra_detalles cd
             LEFT JOIN productos p ON cd.producto_id = p.id
             WHERE cd.compra_id = ?1"
        ).map_err(|e| e.to_string())?;
        let rows = stmt.query_map(rusqlite::params![input.compra_id], |r| {
            Ok((r.get(0)?, r.get(1).ok(), r.get(2).ok(), r.get(3)?, r.get(4)?, r.get(5)?, r.get(6)?))
        }).map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())?;
        rows
    };

    // Construir lista de items efectivos a devolver
    // (cd_id, producto_id, desc, cantidad_devolver, precio_unit)
    let items_efectivos: Vec<(i64, Option<i64>, Option<String>, f64, f64)> = if input.devolver_todo {
        detalles_db.iter()
            .map(|(cd_id, pid, desc, cant_orig, pu, _sub, cant_dev)| {
                let pendiente = (cant_orig - cant_dev).max(0.0);
                (*cd_id, *pid, desc.clone(), pendiente, *pu)
            })
            .filter(|(_, _, _, c, _)| *c > 0.0)
            .collect()
    } else {
        let mut out = Vec::new();
        for it in &input.items {
            let row = detalles_db.iter().find(|(id, _, _, _, _, _, _)| *id == it.compra_detalle_id)
                .ok_or_else(|| format!("Item de devolucion invalido (detalle {})", it.compra_detalle_id))?;
            let pendiente = (row.3 - row.6).max(0.0);
            if it.cantidad <= 0.0 { continue; }
            if it.cantidad > pendiente + 0.0001 {
                return Err(format!(
                    "Cantidad a devolver ({}) excede lo pendiente ({}) en '{}'",
                    it.cantidad, pendiente, row.2.clone().unwrap_or_default()
                ));
            }
            out.push((row.0, row.1, row.2.clone(), it.cantidad, row.4));
        }
        out
    };

    if items_efectivos.is_empty() {
        return Err("No hay items para devolver".into());
    }

    // v2.5.42: determinar tipo de NC. MERCANCIA (default) revierte stock. AJUSTE_PRECIO no toca stock.
    let tipo_nc = input.tipo_nc.as_deref().unwrap_or("MERCANCIA").to_uppercase();
    let es_ajuste_precio = tipo_nc == "AJUSTE_PRECIO";

    // v2.5.42: validar trazabilidad solo si es devolución de MERCANCÍA (afecta stock)
    if !es_ajuste_precio {
        let permitido_global: bool = conn.query_row(
            "SELECT value FROM config WHERE key = 'permitir_anulacion_stock_negativo'",
            [], |r| r.get::<_, String>(0),
        ).map(|v| v == "1").unwrap_or(false);

        let permitir_neg = input.forzar_stock_negativo || permitido_global;
        if !permitir_neg {
            let mut conflictos: Vec<String> = Vec::new();
            for (_, pid_opt, desc, cant, _) in &items_efectivos {
                if let Some(pid) = pid_opt {
                    let stock_actual: f64 = conn.query_row(
                        "SELECT stock_actual FROM productos WHERE id = ?1",
                        rusqlite::params![pid], |r| r.get(0),
                    ).unwrap_or(0.0);
                    if *cant > stock_actual + 0.0001 {
                        let disponible = stock_actual.max(0.0);
                        conflictos.push(format!(
                            "  - {}: pides devolver {}, pero solo hay {} en stock (ya vendiste algunas)",
                            desc.clone().unwrap_or_else(|| format!("Producto #{}", pid)),
                            cant, disponible
                        ));
                    }
                }
            }
            if !conflictos.is_empty() {
                return Err(format!(
                    "TRAZABILIDAD: Las cantidades exceden el stock disponible:\n{}\n\nOpciones:\n  1) Devuelve solo la cantidad disponible (ajusta los items)\n  2) Si el proveedor te emitió NC por AJUSTE DE PRECIO (no devuelves mercancía), cambia el tipo de NC a 'AJUSTE_PRECIO'\n  3) Activa 'Permitir anulación con stock negativo' en Configuración (admin)",
                    conflictos.join("\n")
                ));
            }
        }
    }

    // v2.5.35: validar clave_acceso_nc unica si viene (para evitar re-importar la misma NC)
    let clave_nc_norm: Option<String> = input.clave_acceso_nc.as_ref()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());
    if let Some(ca) = &clave_nc_norm {
        if ca.len() == 49 {
            let exists: Option<i64> = conn.query_row(
                "SELECT id FROM compra_devoluciones WHERE clave_acceso_nc = ?1 LIMIT 1",
                rusqlite::params![ca], |r| r.get(0),
            ).ok();
            if exists.is_some() {
                return Err(format!(
                    "Esta NC ya fue importada anteriormente (clave SRI: {}…)",
                    &ca[..20]
                ));
            }
        }
    }

    let tx = conn.transaction().map_err(|e| e.to_string())?;

    // numero de devolucion: ND-CMP-XXXXXXXXX-N (N = consecutivo por compra)
    let n_prev: i64 = tx.query_row(
        "SELECT COUNT(*) FROM compra_devoluciones WHERE compra_id = ?1",
        rusqlite::params![input.compra_id], |r| r.get(0),
    ).unwrap_or(0);
    let numero_dev = format!("ND-{}-{}", compra_numero, n_prev + 1);

    // Calcular subtotal e IVA proporcional segun cada compra_detalle (usar tarifa original de la compra)
    // Para esto, leemos el IVA implícito de la compra (iva / subtotal) si no podemos saber por linea.
    // Simplificación: subtotal = sum(cant * precio), IVA = ratio_iva_global * subtotal.
    let (compra_sub, compra_iva): (f64, f64) = tx.query_row(
        "SELECT subtotal, iva FROM compras WHERE id = ?1",
        rusqlite::params![input.compra_id], |r| Ok((r.get(0)?, r.get(1)?)),
    ).unwrap_or((0.0, 0.0));
    let ratio_iva = if compra_sub > 0.0 { compra_iva / compra_sub } else { 0.0 };

    let subtotal_dev: f64 = items_efectivos.iter().map(|(_, _, _, c, p)| c * p).sum();
    let iva_dev = subtotal_dev * ratio_iva;
    let total_dev = subtotal_dev + iva_dev;

    // Insertar cabecera de devolucion
    let es_total_calculado: bool = input.devolver_todo || {
        // Si la suma de todos los items devueltos despues de esta operación = total de la compra
        let dev_total_actual: f64 = tx.query_row(
            "SELECT COALESCE(SUM(total),0) FROM compra_devoluciones WHERE compra_id = ?1",
            rusqlite::params![input.compra_id], |r| r.get(0),
        ).unwrap_or(0.0);
        (dev_total_actual + total_dev) >= (compra_sub + compra_iva) - 0.01
    };

    // v2.5.35: incluir datos del comprobante NC del proveedor si vinieron
    let fecha_nc_norm = input.fecha_emision_nc.as_ref()
        .and_then(|f| convertir_fecha_sri(f));
    tx.execute(
        "INSERT INTO compra_devoluciones (compra_id, numero, motivo, subtotal, iva, total, es_total, usuario, observacion,
                                          numero_nc, clave_acceso_nc, estado_sri_nc, fecha_emision_nc, xml_nc_firmado, tipo_nc)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)",
        rusqlite::params![
            input.compra_id, numero_dev, input.motivo, subtotal_dev, iva_dev, total_dev,
            es_total_calculado as i64, usuario, input.observacion,
            input.numero_nc.as_ref().map(|s| s.trim().to_string()).filter(|s| !s.is_empty()),
            clave_nc_norm,
            input.estado_sri_nc,
            fecha_nc_norm,
            input.xml_nc_firmado,
            tipo_nc,
        ],
    ).map_err(|e| {
        let msg = e.to_string();
        if msg.contains("UNIQUE") && msg.contains("clave_acceso_nc") {
            "Esta NC ya fue registrada anteriormente (clave SRI duplicada)".to_string()
        } else { msg }
    })?;
    let dev_id = tx.last_insert_rowid();

    // Insertar detalles, actualizar cantidad_devuelta en compra_detalles, revertir stock, kardex
    let motivo_str = input.motivo.clone().unwrap_or_else(|| "Sin motivo".to_string());
    for (cd_id, pid_opt, desc, cant, precio) in &items_efectivos {
        let sub_item = cant * precio;
        tx.execute(
            "INSERT INTO compra_devolucion_detalles (devolucion_id, compra_detalle_id, producto_id, cantidad, precio_unitario, subtotal)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            rusqlite::params![dev_id, cd_id, pid_opt, cant, precio, sub_item],
        ).map_err(|e| e.to_string())?;
        tx.execute(
            "UPDATE compra_detalles SET cantidad_devuelta = COALESCE(cantidad_devuelta,0) + ?1 WHERE id = ?2",
            rusqlite::params![cant, cd_id],
        ).ok();
        // v2.5.42: comportamiento según tipo_nc
        if let Some(pid) = pid_opt {
            if es_ajuste_precio {
                // AJUSTE_PRECIO: NO toca stock. Solo recalcula precio_costo del producto
                // (PMP inverso: como si "comprara" -cant al precio original. El resultado es
                // que el costo_promedio refleja el descuento del proveedor).
                let (stock_actual_p, costo_prom_actual): (f64, f64) = tx.query_row(
                    "SELECT stock_actual, COALESCE(costo_promedio, precio_costo) FROM productos WHERE id = ?1",
                    rusqlite::params![pid],
                    |row| Ok((row.get::<_, f64>(0)?, row.get::<_, f64>(1)?)),
                ).unwrap_or((0.0, *precio));
                // El ajuste es un crédito: como si revirtiera el precio pagado por cant unidades.
                // nuevo_costo = (stock_actual * costo - cant * precio) / (stock_actual - 0)  → inválido si stock_actual=0
                // Mejor: calcular el descuento como "rebaja proporcional" del costo actual.
                // Simplificado: si el descuento es por cant unidades a $precio, y aún tengo stock,
                // el "valor del descuento" se distribuye: nuevo_costo = costo_actual * (1 - cant*precio / (stock_actual * costo_actual))
                if stock_actual_p > 0.0 && costo_prom_actual > 0.0 {
                    let valor_descuento = cant * precio;
                    let valor_actual_inventario = stock_actual_p * costo_prom_actual;
                    let nuevo_costo = (valor_actual_inventario - valor_descuento).max(0.0) / stock_actual_p;
                    tx.execute(
                        "UPDATE productos SET costo_promedio = ?1, precio_costo = ?1, updated_at = datetime('now','localtime') WHERE id = ?2",
                        rusqlite::params![nuevo_costo, pid],
                    ).ok();
                }
                // Kardex: registrar el ajuste como movimiento informativo (cantidad 0, valor en motivo)
                let motivo_kardex = format!("Ajuste precio compra {} - NC ${:.2} (motivo: {})", numero_dev, sub_item, motivo_str);
                let _ = tx.execute(
                    "INSERT INTO movimientos_inventario (producto_id, tipo, cantidad, stock_anterior, stock_nuevo, costo_unitario, referencia_id, usuario, motivo)
                     VALUES (?1, 'AJUSTE_PRECIO_NC', 0, ?2, ?2, ?3, ?4, ?5, ?6)",
                    rusqlite::params![pid, stock_actual_p, precio, input.compra_id, usuario, motivo_kardex],
                );
            } else {
                // MERCANCIA (default): revertir stock + kardex normal
                let stock_antes: f64 = tx.query_row(
                    "SELECT stock_actual FROM productos WHERE id = ?1",
                    rusqlite::params![pid], |r| r.get(0),
                ).unwrap_or(0.0);
                tx.execute(
                    "UPDATE productos SET stock_actual = stock_actual - ?1, updated_at = datetime('now','localtime') WHERE id = ?2",
                    rusqlite::params![cant, pid],
                ).ok();
                let stock_nuevo = stock_antes - cant;
                let motivo_kardex = if stock_nuevo < 0.0 {
                    format!("Devolucion compra {} - {} ⚠ STOCK NEGATIVO (items ya vendidos)", numero_dev, desc.clone().unwrap_or_default())
                } else {
                    format!("Devolucion compra {} - {} (motivo: {})", numero_dev,
                        desc.clone().unwrap_or_default(), motivo_str)
                };
                let _ = tx.execute(
                    "INSERT INTO movimientos_inventario (producto_id, tipo, cantidad, stock_anterior, stock_nuevo, costo_unitario, referencia_id, usuario, motivo)
                     VALUES (?1, 'DEVOLUCION_COMPRA', ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                    rusqlite::params![pid, -cant, stock_antes, stock_nuevo, precio, input.compra_id, usuario, motivo_kardex],
                );
            }
        }
    }

    // v2.5.42: si es AJUSTE_PRECIO, también ajustar saldo de CXP (la cuenta por pagar baja por el total NC)
    if es_ajuste_precio && total_dev > 0.0 {
        // Buscar CXP activa de esta compra y reducir saldo
        let cxp: Option<(i64, f64)> = tx.query_row(
            "SELECT id, saldo FROM cuentas_por_pagar WHERE compra_id = ?1 AND estado != 'ANULADA' LIMIT 1",
            rusqlite::params![input.compra_id], |r| Ok((r.get(0)?, r.get(1)?)),
        ).ok();
        if let Some((cxp_id, saldo_actual)) = cxp {
            let nuevo_saldo = (saldo_actual - total_dev).max(0.0);
            let nuevo_estado = if nuevo_saldo <= 0.01 { "PAGADA" } else { "PENDIENTE" };
            let _ = tx.execute(
                "UPDATE cuentas_por_pagar SET saldo = ?1, estado = ?2 WHERE id = ?3",
                rusqlite::params![nuevo_saldo, nuevo_estado, cxp_id],
            );
        }
    }

    // Si es devolucion total, marcar compra como DEVUELTA (estado nuevo)
    if es_total_calculado {
        tx.execute(
            "UPDATE compras SET estado = 'DEVUELTA' WHERE id = ?1 AND estado != 'ANULADA'",
            rusqlite::params![input.compra_id],
        ).ok();
    }

    tx.commit().map_err(|e| e.to_string())?;

    Ok(serde_json::json!({
        "devolucion_id": dev_id,
        "numero": numero_dev,
        "subtotal": subtotal_dev,
        "iva": iva_dev,
        "total": total_dev,
        "es_total": es_total_calculado,
        "items": items_efectivos.len(),
    }))
}

#[tauri::command]
pub fn listar_devoluciones_compra(
    db: State<Database>,
    compra_id: i64,
) -> Result<Vec<DevolucionCompraInfo>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let mut stmt = conn.prepare(
        "SELECT id, compra_id, numero, fecha, motivo, subtotal, iva, total, es_total, usuario, observacion,
                numero_nc, clave_acceso_nc, estado_sri_nc, fecha_emision_nc,
                COALESCE(tipo_nc, 'MERCANCIA')
         FROM compra_devoluciones WHERE compra_id = ?1 ORDER BY id DESC"
    ).map_err(|e| e.to_string())?;
    let lista = stmt.query_map(rusqlite::params![compra_id], |r| {
        Ok(DevolucionCompraInfo {
            id: r.get(0)?,
            compra_id: r.get(1)?,
            numero: r.get(2)?,
            fecha: r.get(3)?,
            motivo: r.get(4).ok(),
            subtotal: r.get(5)?,
            iva: r.get(6)?,
            total: r.get(7)?,
            es_total: r.get::<_, i64>(8)? != 0,
            usuario: r.get(9).ok(),
            observacion: r.get(10).ok(),
            numero_nc: r.get(11).ok(),
            clave_acceso_nc: r.get(12).ok(),
            estado_sri_nc: r.get(13).ok(),
            fecha_emision_nc: r.get(14).ok(),
            tipo_nc: r.get(15).ok(),
        })
    }).map_err(|e| e.to_string())?
    .collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())?;
    Ok(lista)
}

// ════════════════════════════════════════════════════════════════════
// v2.5.35: Importar XML de NC del proveedor
// ════════════════════════════════════════════════════════════════════
//
// Parsea un XML SRI de tipo notaCredito (con o sin wrapping <autorizacion>)
// y extrae los datos clave: numero (estab-pto-sec), clave_acceso, fecha emision,
// motivo, total, autorizado o no, y la clave_acceso de la factura referenciada
// (numDocModificado). Esto permite auto-rellenar la sección "Comprobante NC" del
// modal de devolución de compra en frontend.
//
// El detalle (items) se mantiene seleccionable manualmente desde la compra
// original — la NC del proveedor referencia esa compra y no siempre coincide
// 1:1 con sus items.

#[derive(serde::Serialize, Debug)]
pub struct PreviewXmlNcCompra {
    /// Número visible (estab-pto-sec, ej: 001-001-000000123)
    pub numero: String,
    pub clave_acceso: String,
    pub fecha_emision: String,
    /// Clave de la factura referenciada (numDocModificado). Permite ubicar
    /// la compra original en BD automáticamente.
    pub clave_factura_referenciada: Option<String>,
    /// Número de la factura referenciada (estab-pto-sec del codDocModificado)
    pub numero_factura_referenciada: Option<String>,
    pub razon_modificacion: Option<String>,
    pub total: f64,
    pub autorizada: bool,
    pub proveedor_ruc: String,
    pub proveedor_nombre: String,
    /// Si encontramos compra en BD con esa clave de factura referenciada
    pub compra_id_sugerida: Option<i64>,
    pub compra_numero_sugerida: Option<String>,
    /// XML firmado original (para guardar en compra_devoluciones.xml_nc_firmado)
    pub xml_firmado: String,
}

#[tauri::command]
pub fn preview_xml_nc_compra(
    db: State<Database>,
    xml_contenido: String,
) -> Result<PreviewXmlNcCompra, String> {
    use quick_xml::events::Event;
    use quick_xml::reader::Reader;

    // 1. Detectar si XML viene envuelto en <autorizacion><estado>AUTORIZADO</estado>
    let estado_sri_xml: Option<String> = {
        let bytes = xml_contenido.as_bytes();
        let mut reader = Reader::from_reader(bytes);
        reader.config_mut().trim_text(true);
        let mut buf = Vec::new();
        let mut inside_estado = false;
        let mut inside_autorizacion = false;
        let mut estado_text = String::new();
        loop {
            match reader.read_event_into(&mut buf) {
                Ok(Event::Start(e)) => {
                    let name = String::from_utf8_lossy(e.name().as_ref()).to_string();
                    if name == "autorizacion" { inside_autorizacion = true; }
                    if name == "estado" && inside_autorizacion { inside_estado = true; }
                }
                Ok(Event::End(e)) => {
                    let name = String::from_utf8_lossy(e.name().as_ref()).to_string();
                    if name == "estado" { inside_estado = false; }
                    if name == "autorizacion" { inside_autorizacion = false; }
                }
                Ok(Event::Text(t)) => {
                    if inside_estado {
                        if let Ok(s) = t.unescape() { estado_text.push_str(&s); }
                    }
                }
                Ok(Event::Eof) => break,
                Err(_) => break,
                _ => {}
            }
            buf.clear();
        }
        let t = estado_text.trim().to_string();
        if t.is_empty() { None } else { Some(t) }
    };
    let autorizada = estado_sri_xml.as_deref().map(|s| s.eq_ignore_ascii_case("AUTORIZADO")).unwrap_or(false);

    // 2. Desenrollar <comprobante><![CDATA[<notaCredito>...]]></comprobante> si aplica
    let xml_real: String = {
        let bytes = xml_contenido.as_bytes();
        let mut reader = Reader::from_reader(bytes);
        reader.config_mut().trim_text(true);
        let mut buf = Vec::new();
        let mut inside_comprobante = false;
        let mut found: Option<String> = None;
        loop {
            match reader.read_event_into(&mut buf) {
                Ok(Event::Start(e)) => {
                    if e.name().as_ref() == b"comprobante" { inside_comprobante = true; }
                }
                Ok(Event::End(e)) => {
                    if e.name().as_ref() == b"comprobante" { inside_comprobante = false; }
                }
                Ok(Event::CData(c)) => {
                    if inside_comprobante {
                        if let Ok(s) = std::str::from_utf8(c.as_ref()) {
                            found = Some(s.to_string());
                            break;
                        }
                    }
                }
                Ok(Event::Text(t)) => {
                    if inside_comprobante {
                        if let Ok(s) = t.unescape() {
                            let trimmed = s.trim();
                            if trimmed.starts_with("<notaCredito") || trimmed.starts_with("<?xml") {
                                found = Some(trimmed.to_string());
                                break;
                            }
                        }
                    }
                }
                Ok(Event::Eof) => break,
                Err(_) => break,
                _ => {}
            }
            buf.clear();
        }
        found.unwrap_or(xml_contenido.clone())
    };

    // 3. Parsear la <notaCredito>
    let mut reader = Reader::from_str(&xml_real);
    reader.config_mut().trim_text(true);
    let mut buf = Vec::new();
    let mut path_stack: Vec<String> = Vec::new();
    let mut current_text = String::new();

    let mut ruc = String::new();
    let mut razon_social = String::new();
    let mut estab = String::new();
    let mut pto_emi = String::new();
    let mut secuencial = String::new();
    let mut fecha_emision = String::new();
    let mut clave_acceso = String::new();
    let mut cod_doc_mod = String::new();
    let mut num_doc_mod = String::new();
    let mut razon_modificacion = String::new();
    let mut clave_factura_ref: Option<String> = None;
    let mut total_nc = 0.0_f64;

    let mut tipo_doc_es_nc = false;

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) => {
                let name = String::from_utf8_lossy(e.name().as_ref()).to_string();
                path_stack.push(name);
                current_text.clear();
            }
            Ok(Event::Text(t)) => {
                if let Ok(s) = t.unescape() { current_text.push_str(&s); }
            }
            Ok(Event::End(e)) => {
                let name = String::from_utf8_lossy(e.name().as_ref()).to_string();
                let path = path_stack.join("/");
                let v = current_text.trim().to_string();

                match name.as_str() {
                    "ruc" if path.contains("infoTributaria") => ruc = v.clone(),
                    "razonSocial" if path.contains("infoTributaria") => razon_social = v.clone(),
                    "estab" if path.contains("infoTributaria") => estab = v.clone(),
                    "ptoEmi" if path.contains("infoTributaria") => pto_emi = v.clone(),
                    "secuencial" if path.contains("infoTributaria") => secuencial = v.clone(),
                    "claveAcceso" if path.contains("infoTributaria") => clave_acceso = v.clone(),
                    "codDoc" if path.contains("infoTributaria") => {
                        if v == "04" { tipo_doc_es_nc = true; }
                    }
                    "fechaEmision" if path.contains("infoNotaCredito") => fecha_emision = v.clone(),
                    "codDocModificado" if path.contains("infoNotaCredito") => cod_doc_mod = v.clone(),
                    "numDocModificado" if path.contains("infoNotaCredito") => num_doc_mod = v.clone(),
                    "motivo" if path.contains("infoNotaCredito") => razon_modificacion = v.clone(),
                    "valorModificacion" if path.contains("infoNotaCredito") => {
                        total_nc = v.parse::<f64>().unwrap_or(0.0);
                    }
                    _ => {}
                }

                if !path_stack.is_empty() { path_stack.pop(); }
                current_text.clear();
            }
            Ok(Event::Eof) => break,
            Err(_) => break,
            _ => {}
        }
        buf.clear();
    }

    if !tipo_doc_es_nc && clave_acceso.is_empty() {
        return Err("El XML no parece ser una Nota de Crédito SRI (codDoc != 04)".into());
    }

    // numDocModificado en SRI viene formato "001-001-000000123"
    // No tenemos la claveAcceso de la factura modificada en infoNotaCredito directamente,
    // pero a veces viene en un nodo separado <docModificado><claveAcceso>...
    // De cualquier forma, podemos buscar la compra por numero_factura o por la clave de acceso
    // de la factura referenciada si la incluyeron.

    // Buscar la clave_acceso de la factura modificada (algunos XML la traen)
    {
        let mut r2 = Reader::from_str(&xml_real);
        r2.config_mut().trim_text(true);
        let mut buf2 = Vec::new();
        let mut path: Vec<String> = Vec::new();
        let mut text = String::new();
        loop {
            match r2.read_event_into(&mut buf2) {
                Ok(Event::Start(e)) => {
                    path.push(String::from_utf8_lossy(e.name().as_ref()).to_string());
                    text.clear();
                }
                Ok(Event::Text(t)) => {
                    if let Ok(s) = t.unescape() { text.push_str(&s); }
                }
                Ok(Event::End(e)) => {
                    let name = String::from_utf8_lossy(e.name().as_ref()).to_string();
                    let full = path.join("/");
                    if name == "claveAccesoModificado" || (name == "claveAcceso" && full.contains("docModificado")) {
                        let t = text.trim();
                        if t.len() == 49 { clave_factura_ref = Some(t.to_string()); }
                    }
                    if !path.is_empty() { path.pop(); }
                    text.clear();
                }
                Ok(Event::Eof) => break,
                Err(_) => break,
                _ => {}
            }
            buf2.clear();
        }
    }

    // 4. Sugerir compra de la BD: primero por clave de acceso de factura modificada,
    //    si no por (proveedor_ruc + numero_factura = num_doc_mod)
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let (compra_id_sugerida, compra_numero_sugerida): (Option<i64>, Option<String>) = {
        let mut id: Option<i64> = None;
        let mut nro: Option<String> = None;
        if let Some(ca) = &clave_factura_ref {
            if let Ok((cid, cnum)) = conn.query_row(
                "SELECT id, numero FROM compras WHERE clave_acceso = ?1 AND estado != 'ANULADA' LIMIT 1",
                rusqlite::params![ca], |r| Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?)),
            ) {
                id = Some(cid); nro = Some(cnum);
            }
        }
        if id.is_none() && !num_doc_mod.is_empty() && !ruc.is_empty() {
            // buscar por (proveedor.ruc + numero_factura)
            if let Ok((cid, cnum)) = conn.query_row(
                "SELECT c.id, c.numero FROM compras c
                 JOIN proveedores p ON c.proveedor_id = p.id
                 WHERE p.ruc = ?1 AND c.numero_factura = ?2 AND c.estado != 'ANULADA' LIMIT 1",
                rusqlite::params![ruc, num_doc_mod],
                |r| Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?)),
            ) { id = Some(cid); nro = Some(cnum); }
        }
        (id, nro)
    };

    // 5. Verificar que la NC no haya sido importada antes
    if !clave_acceso.is_empty() {
        let existe: Option<i64> = conn.query_row(
            "SELECT id FROM compra_devoluciones WHERE clave_acceso_nc = ?1 LIMIT 1",
            rusqlite::params![&clave_acceso], |r| r.get(0),
        ).ok();
        if existe.is_some() {
            return Err(format!(
                "Esta NC ya fue importada anteriormente (clave SRI: {}…)",
                &clave_acceso[..20.min(clave_acceso.len())]
            ));
        }
    }

    let numero = if !estab.is_empty() {
        format!("{}-{}-{}", estab, pto_emi, secuencial)
    } else { String::new() };

    Ok(PreviewXmlNcCompra {
        numero,
        clave_acceso,
        fecha_emision,
        clave_factura_referenciada: clave_factura_ref,
        numero_factura_referenciada: if num_doc_mod.is_empty() { None } else { Some(num_doc_mod) },
        razon_modificacion: if razon_modificacion.is_empty() { None } else { Some(razon_modificacion) },
        total: total_nc,
        autorizada,
        proveedor_ruc: ruc,
        proveedor_nombre: razon_social,
        compra_id_sugerida,
        compra_numero_sugerida,
        xml_firmado: xml_contenido,
    })
}

// Sufijo: ignorar use unused warning porque cod_doc_mod se setea pero no se lee
// (lo usaremos en futuras versiones para diferenciar tipos de doc modificados)
#[allow(dead_code)]
fn _placeholder_cod_doc_mod() {
    let _ = "01"; // FACTURA modificada
}
