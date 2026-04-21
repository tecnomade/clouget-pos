use crate::db::{Database, SesionState};
use crate::models::{Compra, CompraCompleta, CompraDetalle, NuevaCompra};
use tauri::State;

#[tauri::command]
pub fn registrar_compra(db: State<Database>, compra: NuevaCompra) -> Result<CompraCompleta, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;

    // Generar numero secuencial CMP-000001
    let secuencial: i64 = conn
        .query_row(
            "SELECT CAST(value AS INTEGER) FROM config WHERE key = 'secuencial_compra'",
            [],
            |row| row.get(0),
        )
        .unwrap_or(1);

    let numero = format!("CMP-{:06}", secuencial);

    // Incrementar secuencial
    conn.execute(
        "UPDATE config SET value = CAST(?1 AS TEXT) WHERE key = 'secuencial_compra'",
        rusqlite::params![secuencial + 1],
    )
    .map_err(|e| e.to_string())?;

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

    // Insertar compra
    conn.execute(
        "INSERT INTO compras (numero, proveedor_id, numero_factura, subtotal, iva, total, forma_pago, es_credito, observacion)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
        rusqlite::params![
            numero,
            compra.proveedor_id,
            compra.numero_factura,
            subtotal_total,
            iva_total,
            total,
            compra.forma_pago,
            compra.es_credito,
            compra.observacion,
        ],
    )
    .map_err(|e| e.to_string())?;

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

            // Actualizar stock y precio_costo del producto
            conn.execute(
                "UPDATE productos SET stock_actual = stock_actual + ?1, precio_costo = ?2,
                 updated_at = datetime('now','localtime') WHERE id = ?3",
                rusqlite::params![item.cantidad, item.precio_unitario, pid],
            )
            .map_err(|e| e.to_string())?;

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

    Ok(CompraCompleta {
        compra: Compra {
            id: Some(compra_id),
            numero: numero.clone(),
            proveedor_id: compra.proveedor_id,
            fecha,
            numero_factura: compra.numero_factura,
            subtotal: subtotal_total,
            iva: iva_total,
            total,
            estado: "REGISTRADA".to_string(),
            forma_pago: compra.forma_pago,
            es_credito: compra.es_credito,
            observacion: compra.observacion,
            proveedor_nombre,
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
                    c.observacion, p.nombre
             FROM compras c
             JOIN proveedores p ON c.proveedor_id = p.id
             WHERE date(c.fecha) >= date(?1) AND date(c.fecha) <= date(?2)
             ORDER BY c.fecha DESC"
        }
        _ => {
            "SELECT c.id, c.numero, c.proveedor_id, c.fecha, c.numero_factura,
                    c.subtotal, c.iva, c.total, c.estado, c.forma_pago, c.es_credito,
                    c.observacion, p.nombre
             FROM compras c
             JOIN proveedores p ON c.proveedor_id = p.id
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
                    c.observacion, p.nombre
             FROM compras c
             JOIN proveedores p ON c.proveedor_id = p.id
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
                })
            },
        )
        .map_err(|_| "Compra no encontrada".to_string())?;

    let mut stmt = conn
        .prepare(
            "SELECT cd.id, cd.compra_id, cd.producto_id, cd.descripcion,
                    cd.cantidad, cd.precio_unitario, cd.subtotal, p.nombre
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
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    Ok(CompraCompleta { compra, detalles })
}

#[tauri::command]
pub fn anular_compra(db: State<Database>, id: i64) -> Result<(), String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;

    // Verificar que la compra existe y no está ya anulada
    let estado: String = conn
        .query_row(
            "SELECT estado FROM compras WHERE id = ?1",
            rusqlite::params![id],
            |row| row.get(0),
        )
        .map_err(|_| "Compra no encontrada".to_string())?;

    if estado == "ANULADA" {
        return Err("La compra ya está anulada".to_string());
    }

    // Revertir stock para cada detalle con producto_id
    let mut stmt = conn
        .prepare(
            "SELECT producto_id, cantidad FROM compra_detalles WHERE compra_id = ?1 AND producto_id IS NOT NULL",
        )
        .map_err(|e| e.to_string())?;

    let detalles: Vec<(i64, f64)> = stmt
        .query_map(rusqlite::params![id], |row| {
            Ok((row.get(0)?, row.get(1)?))
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    for (producto_id, cantidad) in detalles {
        conn.execute(
            "UPDATE productos SET stock_actual = stock_actual - ?1,
             updated_at = datetime('now','localtime') WHERE id = ?2",
            rusqlite::params![cantidad, producto_id],
        )
        .map_err(|e| e.to_string())?;
    }

    // Marcar compra como anulada
    conn.execute(
        "UPDATE compras SET estado = 'ANULADA' WHERE id = ?1",
        rusqlite::params![id],
    )
    .map_err(|e| e.to_string())?;

    // Anular cuenta por pagar asociada si existe
    conn.execute(
        "UPDATE cuentas_por_pagar SET estado = 'ANULADA' WHERE compra_id = ?1 AND estado = 'PENDIENTE'",
        rusqlite::params![id],
    )
    .map_err(|e| e.to_string())?;

    Ok(())
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
    pub forma_pago: String, // "EFECTIVO" | "CREDITO" | "TRANSFERENCIA"
    pub dias_credito: Option<i64>,
}

#[tauri::command]
pub fn importar_xml_compra(
    db: State<Database>,
    _sesion: State<SesionState>,
    input: ImportarXmlInput,
) -> Result<serde_json::Value, String> {
    let mut conn = db.conn.lock().map_err(|e| e.to_string())?;
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
                // Usar fecha de emisión del XML (no la fecha actual)
                let fecha_gasto = if !input.fecha_emision.is_empty() {
                    input.fecha_emision.clone()
                } else {
                    chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string()
                };
                tx.execute(
                    "INSERT INTO gastos (descripcion, monto, categoria, observacion, fecha) \
                     VALUES (?1, ?2, ?3, ?4, ?5)",
                    rusqlite::params![
                        item.descripcion,
                        monto_gasto,
                        cat,
                        format!("Importado de XML factura: {}", input.numero_factura),
                        fecha_gasto
                    ],
                )
                .map_err(|e| e.to_string())?;
                gastos_creados += 1;
            }
            _ => {}
        }
    }

    // Crear compra solo si hay items de producto
    let compra_id: Option<i64> = if !items_compra.is_empty() {
        // Usar el mismo mecanismo que registrar_compra: secuencial en config
        let secuencial: i64 = tx
            .query_row(
                "SELECT CAST(value AS INTEGER) FROM config WHERE key = 'secuencial_compra'",
                [],
                |row| row.get(0),
            )
            .unwrap_or(1);
        let numero_compra = format!("CMP-{:06}", secuencial);
        tx.execute(
            "UPDATE config SET value = CAST(?1 AS TEXT) WHERE key = 'secuencial_compra'",
            rusqlite::params![secuencial + 1],
        )
        .map_err(|e| e.to_string())?;

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

        tx.execute(
            "INSERT INTO compras (numero, proveedor_id, fecha, numero_factura, subtotal, iva, total, estado, forma_pago, es_credito, observacion) \
             VALUES (?1, ?2, COALESCE(?3, datetime('now','localtime')), ?4, ?5, ?6, ?7, 'REGISTRADA', ?8, ?9, ?10)",
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
                "Importado desde XML"
            ],
        )
        .map_err(|e| e.to_string())?;
        let cid = tx.last_insert_rowid();

        // Insertar detalles y actualizar stock/costo
        for (pid, cant, precio, _iva_p, sub, desc) in &items_compra {
            tx.execute(
                "INSERT INTO compra_detalles (compra_id, producto_id, descripcion, cantidad, precio_unitario, subtotal) \
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                rusqlite::params![cid, pid, desc, cant, precio, sub],
            )
            .map_err(|e| e.to_string())?;
            tx.execute(
                "UPDATE productos SET stock_actual = stock_actual + ?1, precio_costo = ?2, updated_at = datetime('now','localtime') WHERE id = ?3",
                rusqlite::params![cant, precio, pid],
            )
            .ok();
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
