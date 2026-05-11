use crate::db::{Database, SesionState};
use crate::models::{OrdenServicio, MovimientoOrden};
use tauri::State;

/// v2.4.8 — Verifica que la licencia activa tenga el módulo `servicio_tecnico`.
/// Devuelve `Ok(())` si está activo, `Err` con mensaje listo para propagar al
/// frontend si no. Mismo patrón que `restaurante::requiere_modulo_restaurante`
/// y `app_movil::requiere_modulo_app_movil`.
pub fn requiere_modulo_servicio_tecnico(db: &Database) -> Result<(), String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let modulos_json: String = conn
        .query_row(
            "SELECT value FROM config WHERE key = 'licencia_modulos'",
            [],
            |row| row.get(0),
        )
        .unwrap_or_default();
    if modulos_json.is_empty() {
        return Err("Módulo Servicio Técnico no incluido en su licencia".to_string());
    }
    let modulos: Vec<String> = serde_json::from_str(&modulos_json).unwrap_or_default();
    if !modulos.iter().any(|m| m == "servicio_tecnico") {
        return Err(
            "Módulo Servicio Técnico no incluido en su licencia. Contacte a soporte para activarlo."
                .to_string(),
        );
    }
    Ok(())
}

#[tauri::command]
pub fn crear_orden_servicio(
    db: State<Database>,
    sesion: State<SesionState>,
    orden: OrdenServicio,
) -> Result<i64, String> {
    requiere_modulo_servicio_tecnico(&db)?;
    let usuario = {
        let s = sesion.sesion.lock().map_err(|e| e.to_string())?;
        s.as_ref().map(|s| s.nombre.clone()).unwrap_or_else(|| "Sistema".to_string())
    };
    let conn = db.conn.lock().map_err(|e| e.to_string())?;

    // Generar numero secuencial OS-NNNNNN
    let next: i64 = conn.query_row(
        "SELECT COALESCE(MAX(CAST(SUBSTR(numero, 4) AS INTEGER)), 0) + 1 FROM ordenes_servicio WHERE numero LIKE 'OS-%'",
        [], |r| r.get(0)
    ).unwrap_or(1);
    let numero = format!("OS-{:06}", next);

    conn.execute(
        "INSERT INTO ordenes_servicio (numero, cliente_id, cliente_nombre, cliente_telefono,
         tipo_equipo, equipo_descripcion, equipo_marca, equipo_modelo, equipo_serie, equipo_placa,
         equipo_kilometraje, equipo_kilometraje_proximo, accesorios, problema_reportado,
         diagnostico, trabajo_realizado, observaciones, tecnico_id, tecnico_nombre,
         estado, fecha_promesa, presupuesto, garantia_dias, usuario_creador,
         tipo_equipo_id, marca_id, modelo_id)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21, ?22, ?23, ?24, ?25, ?26, ?27)",
        rusqlite::params![
            numero, orden.cliente_id, orden.cliente_nombre, orden.cliente_telefono,
            orden.tipo_equipo, orden.equipo_descripcion, orden.equipo_marca, orden.equipo_modelo,
            orden.equipo_serie, orden.equipo_placa, orden.equipo_kilometraje, orden.equipo_kilometraje_proximo,
            orden.accesorios, orden.problema_reportado, orden.diagnostico, orden.trabajo_realizado,
            orden.observaciones, orden.tecnico_id, orden.tecnico_nombre,
            orden.estado, orden.fecha_promesa, orden.presupuesto, orden.garantia_dias, usuario.clone(),
            orden.tipo_equipo_id, orden.marca_id, orden.modelo_id,
        ],
    ).map_err(|e| e.to_string())?;

    let id = conn.last_insert_rowid();

    // Movimiento inicial
    conn.execute(
        "INSERT INTO ordenes_servicio_movimientos (orden_id, estado_nuevo, observacion, usuario) VALUES (?1, ?2, ?3, ?4)",
        rusqlite::params![id, &orden.estado, "Orden creada", usuario]
    ).ok();

    Ok(id)
}

#[tauri::command]
pub fn actualizar_orden_servicio(
    db: State<Database>,
    orden: OrdenServicio,
) -> Result<(), String> {
    requiere_modulo_servicio_tecnico(&db)?;
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let id = orden.id.ok_or("ID requerido")?;

    conn.execute(
        "UPDATE ordenes_servicio SET cliente_id=?1, cliente_nombre=?2, cliente_telefono=?3,
         tipo_equipo=?4, equipo_descripcion=?5, equipo_marca=?6, equipo_modelo=?7, equipo_serie=?8,
         equipo_placa=?9, equipo_kilometraje=?10, equipo_kilometraje_proximo=?11, accesorios=?12,
         problema_reportado=?13, diagnostico=?14, trabajo_realizado=?15, observaciones=?16,
         tecnico_id=?17, tecnico_nombre=?18, fecha_promesa=?19, presupuesto=?20, monto_final=?21,
         garantia_dias=?22, tipo_equipo_id=?23, marca_id=?24, modelo_id=?25 WHERE id=?26",
        rusqlite::params![
            orden.cliente_id, orden.cliente_nombre, orden.cliente_telefono,
            orden.tipo_equipo, orden.equipo_descripcion, orden.equipo_marca, orden.equipo_modelo,
            orden.equipo_serie, orden.equipo_placa, orden.equipo_kilometraje, orden.equipo_kilometraje_proximo,
            orden.accesorios, orden.problema_reportado, orden.diagnostico, orden.trabajo_realizado,
            orden.observaciones, orden.tecnico_id, orden.tecnico_nombre, orden.fecha_promesa,
            orden.presupuesto, orden.monto_final, orden.garantia_dias,
            orden.tipo_equipo_id, orden.marca_id, orden.modelo_id, id,
        ],
    ).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn cambiar_estado_orden(
    db: State<Database>,
    sesion: State<SesionState>,
    orden_id: i64,
    nuevo_estado: String,
    observacion: Option<String>,
) -> Result<(), String> {
    requiere_modulo_servicio_tecnico(&db)?;
    let usuario = {
        let s = sesion.sesion.lock().map_err(|e| e.to_string())?;
        s.as_ref().map(|s| s.nombre.clone()).unwrap_or_else(|| "Sistema".to_string())
    };
    let conn = db.conn.lock().map_err(|e| e.to_string())?;

    let estado_anterior: String = conn.query_row(
        "SELECT estado FROM ordenes_servicio WHERE id = ?1",
        rusqlite::params![orden_id], |r| r.get(0)
    ).unwrap_or_default();

    if nuevo_estado == "ENTREGADO" {
        conn.execute(
            "UPDATE ordenes_servicio SET estado = ?1, fecha_entrega = datetime('now','localtime') WHERE id = ?2",
            rusqlite::params![nuevo_estado, orden_id]
        ).map_err(|e| e.to_string())?;
    } else {
        conn.execute(
            "UPDATE ordenes_servicio SET estado = ?1 WHERE id = ?2",
            rusqlite::params![nuevo_estado, orden_id]
        ).map_err(|e| e.to_string())?;
    }

    conn.execute(
        "INSERT INTO ordenes_servicio_movimientos (orden_id, estado_anterior, estado_nuevo, observacion, usuario) VALUES (?1, ?2, ?3, ?4, ?5)",
        rusqlite::params![orden_id, estado_anterior, nuevo_estado, observacion, usuario]
    ).ok();
    Ok(())
}

#[tauri::command]
pub fn obtener_orden_servicio(db: State<Database>, id: i64) -> Result<OrdenServicio, String> {
    requiere_modulo_servicio_tecnico(&db)?;
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    conn.query_row(
        "SELECT id, numero, cliente_id, cliente_nombre, cliente_telefono, tipo_equipo,
         equipo_descripcion, equipo_marca, equipo_modelo, equipo_serie, equipo_placa,
         equipo_kilometraje, equipo_kilometraje_proximo, accesorios, problema_reportado,
         diagnostico, trabajo_realizado, observaciones, tecnico_id, tecnico_nombre,
         estado, fecha_ingreso, fecha_promesa, fecha_entrega, presupuesto, monto_final,
         garantia_dias, venta_id, usuario_creador, tipo_equipo_id, marca_id, modelo_id FROM ordenes_servicio WHERE id = ?1",
        rusqlite::params![id], |row| Ok(OrdenServicio {
            id: row.get(0)?, numero: row.get(1)?, cliente_id: row.get(2)?,
            cliente_nombre: row.get(3)?, cliente_telefono: row.get(4)?,
            tipo_equipo: row.get(5)?, equipo_descripcion: row.get(6)?,
            equipo_marca: row.get(7)?, equipo_modelo: row.get(8)?,
            equipo_serie: row.get(9)?, equipo_placa: row.get(10)?,
            equipo_kilometraje: row.get(11)?, equipo_kilometraje_proximo: row.get(12)?,
            accesorios: row.get(13)?, problema_reportado: row.get(14)?,
            diagnostico: row.get(15)?, trabajo_realizado: row.get(16)?,
            observaciones: row.get(17)?, tecnico_id: row.get(18)?,
            tecnico_nombre: row.get(19)?, estado: row.get(20)?,
            fecha_ingreso: row.get(21)?, fecha_promesa: row.get(22)?, fecha_entrega: row.get(23)?,
            presupuesto: row.get(24)?, monto_final: row.get(25)?, garantia_dias: row.get(26)?,
            venta_id: row.get(27)?, usuario_creador: row.get(28)?, tipo_equipo_id: row.get(29)?, marca_id: row.get(30)?, modelo_id: row.get(31)?,
        })
    ).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn listar_ordenes_servicio(
    db: State<Database>,
    filtro_estado: Option<String>,
    fecha_desde: Option<String>,
    fecha_hasta: Option<String>,
    tecnico_id: Option<i64>,
) -> Result<Vec<OrdenServicio>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let mut sql = String::from(
        "SELECT id, numero, cliente_id, cliente_nombre, cliente_telefono, tipo_equipo,
         equipo_descripcion, equipo_marca, equipo_modelo, equipo_serie, equipo_placa,
         equipo_kilometraje, equipo_kilometraje_proximo, accesorios, problema_reportado,
         diagnostico, trabajo_realizado, observaciones, tecnico_id, tecnico_nombre,
         estado, fecha_ingreso, fecha_promesa, fecha_entrega, presupuesto, monto_final,
         garantia_dias, venta_id, usuario_creador, tipo_equipo_id, marca_id, modelo_id FROM ordenes_servicio WHERE 1=1"
    );
    if filtro_estado.is_some() { sql.push_str(" AND estado = ?1"); }
    if fecha_desde.is_some() { sql.push_str(" AND date(fecha_ingreso) >= date(?2)"); }
    if fecha_hasta.is_some() { sql.push_str(" AND date(fecha_ingreso) <= date(?3)"); }
    if tecnico_id.is_some() { sql.push_str(" AND tecnico_id = ?4"); }
    sql.push_str(" ORDER BY fecha_ingreso DESC");

    let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;
    let mapper = |row: &rusqlite::Row| -> rusqlite::Result<OrdenServicio> {
        Ok(OrdenServicio {
            id: row.get(0)?, numero: row.get(1)?, cliente_id: row.get(2)?,
            cliente_nombre: row.get(3)?, cliente_telefono: row.get(4)?,
            tipo_equipo: row.get(5)?, equipo_descripcion: row.get(6)?,
            equipo_marca: row.get(7)?, equipo_modelo: row.get(8)?,
            equipo_serie: row.get(9)?, equipo_placa: row.get(10)?,
            equipo_kilometraje: row.get(11)?, equipo_kilometraje_proximo: row.get(12)?,
            accesorios: row.get(13)?, problema_reportado: row.get(14)?,
            diagnostico: row.get(15)?, trabajo_realizado: row.get(16)?,
            observaciones: row.get(17)?, tecnico_id: row.get(18)?,
            tecnico_nombre: row.get(19)?, estado: row.get(20)?,
            fecha_ingreso: row.get(21)?, fecha_promesa: row.get(22)?, fecha_entrega: row.get(23)?,
            presupuesto: row.get(24)?, monto_final: row.get(25)?, garantia_dias: row.get(26)?,
            venta_id: row.get(27)?, usuario_creador: row.get(28)?, tipo_equipo_id: row.get(29)?, marca_id: row.get(30)?, modelo_id: row.get(31)?,
        })
    };
    let rows: Vec<OrdenServicio> = match (filtro_estado, fecha_desde, fecha_hasta, tecnico_id) {
        (None, None, None, None) => stmt.query_map([], mapper),
        (Some(e), None, None, None) => stmt.query_map(rusqlite::params![e], mapper),
        (Some(e), Some(d), Some(h), None) => stmt.query_map(rusqlite::params![e, d, h], mapper),
        (None, Some(d), Some(h), None) => stmt.query_map(rusqlite::params![Option::<String>::None, d, h], mapper),
        _ => stmt.query_map([], mapper),
    }.map_err(|e| e.to_string())?
     .collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())?;
    Ok(rows)
}

#[tauri::command]
pub fn buscar_ordenes_por_equipo(db: State<Database>, query: String) -> Result<Vec<OrdenServicio>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let q = format!("%{}%", query.trim());
    let mut stmt = conn.prepare(
        "SELECT os.id, os.numero, os.cliente_id, os.cliente_nombre, os.cliente_telefono, os.tipo_equipo,
         os.equipo_descripcion, os.equipo_marca, os.equipo_modelo, os.equipo_serie, os.equipo_placa,
         os.equipo_kilometraje, os.equipo_kilometraje_proximo, os.accesorios, os.problema_reportado,
         os.diagnostico, os.trabajo_realizado, os.observaciones, os.tecnico_id, os.tecnico_nombre,
         os.estado, os.fecha_ingreso, os.fecha_promesa, os.fecha_entrega, os.presupuesto, os.monto_final,
         os.garantia_dias, os.venta_id, os.usuario_creador, os.tipo_equipo_id, os.marca_id, os.modelo_id
         FROM ordenes_servicio os
         LEFT JOIN clientes c ON os.cliente_id = c.id
         WHERE os.equipo_serie LIKE ?1 OR os.equipo_placa LIKE ?1 OR os.equipo_descripcion LIKE ?1
            OR os.cliente_nombre LIKE ?1 OR c.identificacion LIKE ?1 OR os.numero LIKE ?1
         ORDER BY os.fecha_ingreso DESC LIMIT 50"
    ).map_err(|e| e.to_string())?;
    let rows: Vec<OrdenServicio> = stmt.query_map(rusqlite::params![q], |row| {
        Ok(OrdenServicio {
            id: row.get(0)?, numero: row.get(1)?, cliente_id: row.get(2)?,
            cliente_nombre: row.get(3)?, cliente_telefono: row.get(4)?,
            tipo_equipo: row.get(5)?, equipo_descripcion: row.get(6)?,
            equipo_marca: row.get(7)?, equipo_modelo: row.get(8)?,
            equipo_serie: row.get(9)?, equipo_placa: row.get(10)?,
            equipo_kilometraje: row.get(11)?, equipo_kilometraje_proximo: row.get(12)?,
            accesorios: row.get(13)?, problema_reportado: row.get(14)?,
            diagnostico: row.get(15)?, trabajo_realizado: row.get(16)?,
            observaciones: row.get(17)?, tecnico_id: row.get(18)?,
            tecnico_nombre: row.get(19)?, estado: row.get(20)?,
            fecha_ingreso: row.get(21)?, fecha_promesa: row.get(22)?, fecha_entrega: row.get(23)?,
            presupuesto: row.get(24)?, monto_final: row.get(25)?, garantia_dias: row.get(26)?,
            venta_id: row.get(27)?, usuario_creador: row.get(28)?, tipo_equipo_id: row.get(29)?, marca_id: row.get(30)?, modelo_id: row.get(31)?,
        })
    }).map_err(|e| e.to_string())?
      .collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())?;
    Ok(rows)
}

#[tauri::command]
pub fn historial_movimientos_orden(db: State<Database>, orden_id: i64) -> Result<Vec<MovimientoOrden>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let mut stmt = conn.prepare(
        "SELECT id, estado_anterior, estado_nuevo, observacion, usuario, fecha
         FROM ordenes_servicio_movimientos WHERE orden_id = ?1 ORDER BY fecha DESC"
    ).map_err(|e| e.to_string())?;
    let rows: Vec<MovimientoOrden> = stmt.query_map(rusqlite::params![orden_id], |row| {
        Ok(MovimientoOrden {
            id: row.get(0)?, estado_anterior: row.get(1)?, estado_nuevo: row.get(2)?,
            observacion: row.get(3)?, usuario: row.get(4)?, fecha: row.get(5)?,
        })
    }).map_err(|e| e.to_string())?
      .collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())?;
    Ok(rows)
}

#[tauri::command]
pub fn eliminar_orden_servicio(db: State<Database>, id: i64) -> Result<(), String> {
    requiere_modulo_servicio_tecnico(&db)?;
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let venta_id: Option<i64> = conn.query_row(
        "SELECT venta_id FROM ordenes_servicio WHERE id = ?1",
        rusqlite::params![id], |r| r.get(0)
    ).unwrap_or(None);
    if venta_id.is_some() {
        conn.execute("UPDATE ordenes_servicio SET estado = 'CANCELADO' WHERE id = ?1", rusqlite::params![id])
            .map_err(|e| e.to_string())?;
    } else {
        conn.execute("DELETE FROM ordenes_servicio WHERE id = ?1", rusqlite::params![id])
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}

// --- Imágenes ---

#[tauri::command]
pub fn agregar_imagen_orden(
    db: State<Database>,
    orden_id: i64,
    tipo: String,
    imagen_base64: String,
    descripcion: Option<String>,
) -> Result<i64, String> {
    requiere_modulo_servicio_tecnico(&db)?;
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    conn.execute(
        "INSERT INTO ordenes_servicio_imagenes (orden_id, tipo, imagen_base64, descripcion) VALUES (?1, ?2, ?3, ?4)",
        rusqlite::params![orden_id, tipo, imagen_base64, descripcion],
    ).map_err(|e| e.to_string())?;
    Ok(conn.last_insert_rowid())
}

#[tauri::command]
pub fn listar_imagenes_orden(db: State<Database>, orden_id: i64) -> Result<Vec<serde_json::Value>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let mut stmt = conn.prepare(
        "SELECT id, tipo, imagen_base64, descripcion, fecha FROM ordenes_servicio_imagenes WHERE orden_id = ?1 ORDER BY fecha DESC"
    ).map_err(|e| e.to_string())?;
    let rows = stmt.query_map(rusqlite::params![orden_id], |r| {
        Ok(serde_json::json!({
            "id": r.get::<_, i64>(0)?,
            "tipo": r.get::<_, String>(1)?,
            "imagen_base64": r.get::<_, String>(2)?,
            "descripcion": r.get::<_, Option<String>>(3)?,
            "fecha": r.get::<_, String>(4)?
        }))
    }).map_err(|e| e.to_string())?;
    rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn eliminar_imagen_orden(db: State<Database>, imagen_id: i64) -> Result<(), String> {
    requiere_modulo_servicio_tecnico(&db)?;
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    conn.execute("DELETE FROM ordenes_servicio_imagenes WHERE id = ?1", rusqlite::params![imagen_id])
        .map_err(|e| e.to_string())?;
    Ok(())
}

// --- Cobrar orden -> crear venta ---

#[tauri::command]
/// v2.4.13: refactor — los items se leen de `orden_servicio_items` (no del frontend),
/// soporta pago mixto (`pagos: Vec<{forma, monto, banco_id?, referencia?}>`) y aplica
/// abonos HOLDING como descuento. Mantiene compat: si vienen los parametros viejos
/// (`forma_pago` + `items_repuestos`), funciona como antes para no romper clientes.
///
/// `garantia_dias` (opcional): actualiza la garantia antes de generar la venta.
pub fn cobrar_orden_servicio(
    db: State<Database>,
    sesion: State<SesionState>,
    orden_id: i64,
    // Compat con firma vieja
    forma_pago: Option<String>,
    monto_recibido: Option<f64>,
    items_repuestos: Option<Vec<serde_json::Value>>,
    // Nuevo: pago mixto
    pagos: Option<Vec<serde_json::Value>>,
    garantia_dias: Option<i64>,
    // v2.4.14: cobranza parcial — si el cliente paga menos que el total,
    // permitir entregar el equipo igual y dejar el saldo pendiente registrado.
    // Estado pasa a ENTREGADO_PARCIAL en lugar de ENTREGADO.
    permitir_saldo_pendiente: Option<bool>,
) -> Result<i64, String> {
    requiere_modulo_servicio_tecnico(&db)?;

    // Si vino garantia_dias, actualizar antes de leer datos
    if let Some(g) = garantia_dias {
        if g >= 0 {
            let conn = db.conn.lock().map_err(|e| e.to_string())?;
            let _ = conn.execute(
                "UPDATE ordenes_servicio SET garantia_dias = ?1 WHERE id = ?2",
                rusqlite::params![g, orden_id],
            );
        }
    }

    let (cliente_id, monto_final_legacy, numero_orden, equipo_descripcion): (Option<i64>, f64, String, String) = {
        let conn = db.conn.lock().map_err(|e| e.to_string())?;
        conn.query_row(
            "SELECT cliente_id, monto_final, numero, equipo_descripcion FROM ordenes_servicio WHERE id = ?1",
            rusqlite::params![orden_id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
        ).map_err(|e| e.to_string())?
    };

    let (usuario, usuario_id) = {
        let s = sesion.sesion.lock().map_err(|e| e.to_string())?;
        (
            s.as_ref().map(|s| s.nombre.clone()).unwrap_or_else(|| "Sistema".to_string()),
            s.as_ref().map(|s| s.usuario_id).unwrap_or(1),
        )
    };

    let mut conn = db.conn.lock().map_err(|e| e.to_string())?;

    // ─── Cargar items: nueva tabla > legacy (monto_final + items_repuestos) ───
    #[derive(Clone)]
    struct ItemCobro {
        producto_id: Option<i64>,
        descripcion: String,
        cantidad: f64,
        precio: f64,
        iva_porc: f64,
        es_servicio: bool,
    }

    let mut items: Vec<ItemCobro> = {
        let mut stmt = conn.prepare(
            "SELECT producto_id, descripcion, cantidad, precio_unitario, iva_porcentaje, es_servicio
             FROM orden_servicio_items WHERE orden_id = ?1 ORDER BY id ASC"
        ).map_err(|e| e.to_string())?;
        let rows = stmt.query_map(rusqlite::params![orden_id], |r| Ok(ItemCobro {
            producto_id: r.get(0)?,
            descripcion: r.get(1)?,
            cantidad: r.get(2)?,
            precio: r.get(3)?,
            iva_porc: r.get(4)?,
            es_servicio: r.get::<_, i64>(5)? != 0,
        })).map_err(|e| e.to_string())?
          .collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())?;
        rows
    };

    // Si la orden no tiene items en la nueva tabla, usar legacy: monto_final + items_repuestos
    if items.is_empty() {
        if monto_final_legacy > 0.0 {
            items.push(ItemCobro {
                producto_id: None,
                descripcion: format!("Servicio: {} - {}", numero_orden, equipo_descripcion),
                cantidad: 1.0,
                precio: monto_final_legacy,
                iva_porc: 0.0,
                es_servicio: true,
            });
        }
        if let Some(reps) = items_repuestos.as_ref() {
            for item in reps {
                let pid = item["producto_id"].as_i64().unwrap_or(0);
                let cant = item["cantidad"].as_f64().unwrap_or(0.0);
                let precio = item["precio_unitario"].as_f64().unwrap_or(0.0);
                let iva_porc: f64 = conn.query_row(
                    "SELECT iva_porcentaje FROM productos WHERE id = ?1",
                    rusqlite::params![pid], |r| r.get(0),
                ).unwrap_or(0.0);
                let descripcion: String = conn.query_row(
                    "SELECT nombre FROM productos WHERE id = ?1",
                    rusqlite::params![pid], |r| r.get(0),
                ).unwrap_or_else(|_| "Repuesto".to_string());
                items.push(ItemCobro {
                    producto_id: Some(pid),
                    descripcion,
                    cantidad: cant,
                    precio,
                    iva_porc,
                    es_servicio: false,
                });
            }
        }
    }

    if items.is_empty() {
        return Err("La orden no tiene items para cobrar. Agrega productos o servicios primero.".to_string());
    }

    // ─── Calcular totales ────────────────────────────────────────────────
    let mut subtotal_sin_iva: f64 = 0.0;
    let mut subtotal_con_iva: f64 = 0.0;
    let mut iva_total: f64 = 0.0;
    for it in &items {
        let sub = it.cantidad * it.precio;
        if it.iva_porc > 0.0 {
            subtotal_con_iva += sub;
            iva_total += sub * (it.iva_porc / 100.0);
        } else {
            subtotal_sin_iva += sub;
        }
    }
    let total = subtotal_sin_iva + subtotal_con_iva + iva_total;

    // ─── Abonos HOLDING (descuento del total) ────────────────────────────
    let total_holdings: f64 = conn.query_row(
        "SELECT COALESCE(SUM(monto), 0) FROM st_abonos WHERE orden_id = ?1 AND estado = 'HOLDING'",
        rusqlite::params![orden_id], |r| r.get(0),
    ).unwrap_or(0.0);
    let saldo = (total - total_holdings).max(0.0);

    // ─── Pagos: nuevo (mixto) o legacy (forma_pago + monto_recibido) ─────
    #[derive(Clone)]
    struct PagoCobro {
        forma: String,
        monto: f64,
        banco_id: Option<i64>,
        referencia: Option<String>,
    }
    let pagos_norm: Vec<PagoCobro> = if let Some(ps) = pagos {
        ps.into_iter().map(|p| PagoCobro {
            forma: p["forma_pago"].as_str().or_else(|| p["forma"].as_str()).unwrap_or("EFECTIVO").to_string(),
            monto: p["monto"].as_f64().unwrap_or(0.0),
            banco_id: p["banco_id"].as_i64(),
            referencia: p["referencia"].as_str().or_else(|| p["referencia_pago"].as_str()).map(|s| s.to_string()),
        }).filter(|p| p.monto > 0.0).collect()
    } else {
        let f = forma_pago.unwrap_or_else(|| "EFECTIVO".to_string());
        vec![PagoCobro {
            forma: f,
            monto: monto_recibido.unwrap_or(saldo),
            banco_id: None,
            referencia: None,
        }]
    };

    let total_pagado_efectivo: f64 = pagos_norm.iter().filter(|p| p.forma == "EFECTIVO").map(|p| p.monto).sum();
    let total_pagado_no_efectivo: f64 = pagos_norm.iter().filter(|p| p.forma != "EFECTIVO").map(|p| p.monto).sum();
    let monto_recibido_total = total_pagado_efectivo + total_pagado_no_efectivo;

    // El cambio solo se da si pagaron MAS efectivo del que faltaba
    let cambio = if total_pagado_efectivo > 0.0 && monto_recibido_total > saldo {
        (monto_recibido_total - saldo).max(0.0).min(total_pagado_efectivo)
    } else {
        0.0
    };

    // v2.4.14: cobranza parcial. Si el cliente paga menos que el saldo y el caller
    // permite saldo pendiente, registramos la diferencia y marcamos ENTREGADO_PARCIAL.
    let permitir_parcial = permitir_saldo_pendiente.unwrap_or(false);
    let saldo_no_cubierto = (saldo - monto_recibido_total).max(0.0);
    if saldo > 0.001 && monto_recibido_total + 0.001 < saldo && !permitir_parcial {
        return Err(format!(
            "El monto pagado (${:.2}) no cubre el saldo (${:.2}) despues de abonos (${:.2}). Marca 'Permitir saldo pendiente' si quieres entregar con saldo.",
            monto_recibido_total, saldo, total_holdings
        ));
    }

    // forma_pago_principal en ventas: la de mayor monto (o MIXTO si hay varias)
    let forma_pago_principal = if pagos_norm.len() > 1 {
        "MIXTO".to_string()
    } else {
        pagos_norm.first().map(|p| p.forma.clone()).unwrap_or_else(|| "EFECTIVO".to_string())
    };

    // ─── Insertar venta ──────────────────────────────────────────────────
    let tx = conn.transaction().map_err(|e| e.to_string())?;

    let next_seq: i64 = tx.query_row(
        "SELECT COALESCE(MAX(CAST(SUBSTR(numero, 4) AS INTEGER)), 0) + 1 FROM ventas WHERE numero LIKE 'NV-%'",
        [], |r| r.get(0),
    ).unwrap_or(1);
    let numero = format!("NV-{:09}", next_seq);

    let observacion = if total_holdings > 0.0 {
        format!("Servicio Tecnico {} - {} (Abonos aplicados: ${:.2})", numero_orden, equipo_descripcion, total_holdings)
    } else {
        format!("Servicio Tecnico {} - {}", numero_orden, equipo_descripcion)
    };

    tx.execute(
        "INSERT INTO ventas (numero, cliente_id, subtotal_sin_iva, subtotal_con_iva, descuento, iva, total, forma_pago, monto_recibido, cambio, estado, tipo_documento, estado_sri, observacion, usuario, usuario_id, tipo_estado) VALUES (?1, ?2, ?3, ?4, 0, ?5, ?6, ?7, ?8, ?9, 'COMPLETADA', 'NOTA_VENTA', 'NO_APLICA', ?10, ?11, ?12, 'COMPLETADA')",
        rusqlite::params![numero, cliente_id, subtotal_sin_iva, subtotal_con_iva, iva_total, total, forma_pago_principal, monto_recibido_total, cambio, observacion, usuario, usuario_id],
    ).map_err(|e| e.to_string())?;
    let venta_id = tx.last_insert_rowid();

    // ─── Insertar detalles ───────────────────────────────────────────────
    for it in &items {
        let sub = it.cantidad * it.precio;
        let info_adicional = if it.producto_id.is_none() {
            Some(it.descripcion.clone())
        } else { None };
        tx.execute(
            "INSERT INTO venta_detalles (venta_id, producto_id, cantidad, precio_unitario, descuento, iva_porcentaje, subtotal, info_adicional) VALUES (?1, ?2, ?3, ?4, 0, ?5, ?6, ?7)",
            rusqlite::params![venta_id, it.producto_id, it.cantidad, it.precio, it.iva_porc, sub, info_adicional],
        ).ok();
        if let Some(pid) = it.producto_id {
            if !it.es_servicio {
                tx.execute(
                    "UPDATE productos SET stock_actual = stock_actual - ?1 WHERE id = ?2 AND es_servicio = 0 AND no_controla_stock = 0",
                    rusqlite::params![it.cantidad, pid],
                ).ok();
            }
        }
    }

    // ─── Insertar pagos en pagos_venta ───────────────────────────────────
    for p in &pagos_norm {
        tx.execute(
            "INSERT INTO pagos_venta (venta_id, forma_pago, monto, banco_id, referencia) VALUES (?1, ?2, ?3, ?4, ?5)",
            rusqlite::params![venta_id, p.forma, p.monto, p.banco_id, p.referencia],
        ).ok();
    }

    // ─── Aplicar abonos HOLDING → APLICADO ──────────────────────────────
    if total_holdings > 0.0 {
        tx.execute(
            "UPDATE st_abonos
             SET estado = 'APLICADO', venta_id_aplicado = ?1,
                 fecha_aplicado = datetime('now','localtime')
             WHERE orden_id = ?2 AND estado = 'HOLDING'",
            rusqlite::params![venta_id, orden_id],
        ).ok();
    }

    let estado_final = if saldo_no_cubierto > 0.001 { "ENTREGADO_PARCIAL" } else { "ENTREGADO" };
    tx.execute(
        "UPDATE ordenes_servicio SET venta_id = ?1, monto_final = ?2, saldo_pendiente = ?3,
         estado = ?4, fecha_entrega = datetime('now','localtime') WHERE id = ?5",
        rusqlite::params![venta_id, total, saldo_no_cubierto, estado_final, orden_id],
    ).ok();

    let obs_mov = if saldo_no_cubierto > 0.001 {
        format!("Cobrado parcial · saldo pendiente ${:.2}", saldo_no_cubierto)
    } else { "Cobrado y entregado".to_string() };
    tx.execute(
        "INSERT INTO ordenes_servicio_movimientos (orden_id, estado_anterior, estado_nuevo, observacion, usuario) VALUES (?1, 'LISTO', ?2, ?3, ?4)",
        rusqlite::params![orden_id, estado_final, obs_mov, usuario],
    ).ok();

    tx.commit().map_err(|e| e.to_string())?;
    Ok(venta_id)
}

// --- PDF de la orden ---

#[tauri::command]
pub fn imprimir_orden_servicio_pdf(
    db: State<Database>,
    orden_id: i64,
    formato: Option<String>,
) -> Result<String, String> {
    requiere_modulo_servicio_tecnico(&db)?;
    use genpdf::{elements::*, fonts, style::*, Alignment, Margins, Document, Element, SimplePageDecorator};

    // v2.4.12 ST-4: formato configurable. Default A4 (compatibilidad con clientes
    // que no actualicen el frontend). Opciones: "A4" | "TICKET_80"
    let formato_efectivo = formato.unwrap_or_else(|| "A4".to_string()).to_uppercase();
    let es_ticket = formato_efectivo == "TICKET_80";

    let (
        nombre_negocio, ruc, direccion, telefono, leyenda_orden,
        numero, cliente_nombre, cliente_telefono, tipo_equipo, equipo_descripcion,
        equipo_marca, equipo_modelo, equipo_serie, equipo_placa, accesorios,
        diagnostico, problema_reportado, trabajo_realizado, observaciones,
        estado, fecha_ingreso, presupuesto, monto_final,
    ): (
        String, String, String, String, String,
        String, Option<String>, Option<String>, String, String,
        String, Option<String>, Option<String>, Option<String>, Option<String>,
        String, String, Option<String>, Option<String>,
        String, String, f64, f64,
    ) = {
        let conn = db.conn.lock().map_err(|e| e.to_string())?;
        let orden = conn.query_row(
            "SELECT numero, cliente_nombre, cliente_telefono, tipo_equipo, equipo_descripcion,
             COALESCE(equipo_marca,''), equipo_modelo, equipo_serie, equipo_placa, accesorios,
             COALESCE(diagnostico,''), problema_reportado, trabajo_realizado, observaciones,
             estado, fecha_ingreso, presupuesto, monto_final
             FROM ordenes_servicio WHERE id = ?1",
            rusqlite::params![orden_id],
            |r| Ok((
                r.get::<_, String>(0)?, r.get::<_, Option<String>>(1)?, r.get::<_, Option<String>>(2)?,
                r.get::<_, String>(3)?, r.get::<_, String>(4)?, r.get::<_, String>(5)?,
                r.get::<_, Option<String>>(6)?, r.get::<_, Option<String>>(7)?, r.get::<_, Option<String>>(8)?,
                r.get::<_, Option<String>>(9)?, r.get::<_, String>(10)?, r.get::<_, String>(11)?,
                r.get::<_, Option<String>>(12)?, r.get::<_, Option<String>>(13)?,
                r.get::<_, String>(14)?, r.get::<_, String>(15)?, r.get::<_, f64>(16)?, r.get::<_, f64>(17)?,
            )),
        ).map_err(|e| format!("Orden no encontrada: {}", e))?;

        let nombre_negocio: String = conn.query_row("SELECT value FROM config WHERE key = 'nombre_negocio'", [], |r| r.get(0)).unwrap_or_else(|_| "Mi Negocio".to_string());
        let ruc: String = conn.query_row("SELECT value FROM config WHERE key = 'ruc'", [], |r| r.get(0)).unwrap_or_default();
        let direccion: String = conn.query_row("SELECT value FROM config WHERE key = 'direccion'", [], |r| r.get(0)).unwrap_or_default();
        let telefono: String = conn.query_row("SELECT value FROM config WHERE key = 'telefono'", [], |r| r.get(0)).unwrap_or_default();
        let leyenda_orden: String = conn.query_row("SELECT value FROM config WHERE key = 'leyenda_orden_servicio'", [], |r| r.get(0)).unwrap_or_default();

        (
            nombre_negocio, ruc, direccion, telefono, leyenda_orden,
            orden.0, orden.1, orden.2, orden.3, orden.4,
            orden.5, orden.6, orden.7, orden.8, orden.9,
            orden.10, orden.11, orden.12, orden.13,
            orden.14, orden.15, orden.16, orden.17,
        )
    };

    let fonts_dir = crate::utils::obtener_ruta_fuentes();
    let font_family = fonts::from_files(&fonts_dir, "LiberationSans", None)
        .map_err(|e| format!("Error fuentes: {}", e))?;
    let mut doc = Document::new(font_family);
    doc.set_title("Orden de Servicio");
    let mut decorator = SimplePageDecorator::new();

    // v2.4.12 ST-4: layout distinto según formato
    let (font_titulo, font_header, font_label, font_normal) = if es_ticket {
        // 80mm — todo más chico, márgenes mínimos
        doc.set_paper_size(genpdf::Size::new(80, 297));
        decorator.set_margins(Margins::trbl(3, 3, 3, 3));
        (12u8, 9u8, 8u8, 8u8)
    } else {
        // A4 — default 210×297
        decorator.set_margins(Margins::trbl(15, 15, 15, 15));
        (16u8, 12u8, 10u8, 10u8)
    };
    doc.set_page_decorator(decorator);

    let title_st = Style::new().bold().with_font_size(font_titulo);
    let header_st = Style::new().bold().with_font_size(font_header);
    let label_st = Style::new().bold().with_font_size(font_label);
    let normal_st = Style::new().with_font_size(font_normal);

    doc.push(Paragraph::new(&nombre_negocio).aligned(Alignment::Center).styled(title_st));
    if !ruc.is_empty() { doc.push(Paragraph::new(&format!("RUC: {}", ruc)).aligned(Alignment::Center).styled(normal_st)); }
    if !direccion.is_empty() { doc.push(Paragraph::new(&direccion).aligned(Alignment::Center).styled(normal_st)); }
    if !telefono.is_empty() { doc.push(Paragraph::new(&format!("Tel: {}", telefono)).aligned(Alignment::Center).styled(normal_st)); }
    doc.push(Break::new(1));

    doc.push(Paragraph::new("ORDEN DE SERVICIO TÉCNICO").aligned(Alignment::Center).styled(header_st));
    doc.push(Paragraph::new(&format!("No: {}    Fecha: {}", numero, fecha_ingreso)).aligned(Alignment::Center).styled(normal_st));
    doc.push(Paragraph::new(&format!("Estado: {}", estado)).aligned(Alignment::Center).styled(normal_st));
    doc.push(Break::new(1));

    doc.push(Paragraph::new("CLIENTE").styled(header_st));
    if let Some(n) = &cliente_nombre { doc.push(Paragraph::new(&format!("Nombre: {}", n)).styled(normal_st)); }
    if let Some(t) = &cliente_telefono { doc.push(Paragraph::new(&format!("Teléfono: {}", t)).styled(normal_st)); }
    doc.push(Break::new(1));

    doc.push(Paragraph::new("EQUIPO").styled(header_st));
    doc.push(Paragraph::new(&format!("Tipo: {}", tipo_equipo)).styled(normal_st));
    doc.push(Paragraph::new(&format!("Descripción: {}", equipo_descripcion)).styled(normal_st));
    if !equipo_marca.is_empty() { doc.push(Paragraph::new(&format!("Marca: {}", equipo_marca)).styled(normal_st)); }
    if let Some(m) = &equipo_modelo { doc.push(Paragraph::new(&format!("Modelo: {}", m)).styled(normal_st)); }
    if let Some(s) = &equipo_serie { doc.push(Paragraph::new(&format!("Serie: {}", s)).styled(normal_st)); }
    if let Some(p) = &equipo_placa { doc.push(Paragraph::new(&format!("Placa: {}", p)).styled(normal_st)); }
    if let Some(a) = &accesorios { doc.push(Paragraph::new(&format!("Accesorios: {}", a)).styled(normal_st)); }
    doc.push(Break::new(1));

    doc.push(Paragraph::new("PROBLEMA REPORTADO").styled(header_st));
    doc.push(Paragraph::new(&problema_reportado).styled(normal_st));
    doc.push(Break::new(0.5));

    if !diagnostico.is_empty() {
        doc.push(Paragraph::new("DIAGNÓSTICO").styled(header_st));
        doc.push(Paragraph::new(&diagnostico).styled(normal_st));
        doc.push(Break::new(0.5));
    }
    if let Some(t) = &trabajo_realizado {
        doc.push(Paragraph::new("TRABAJO REALIZADO").styled(header_st));
        doc.push(Paragraph::new(t).styled(normal_st));
        doc.push(Break::new(0.5));
    }
    if let Some(o) = &observaciones {
        doc.push(Paragraph::new("OBSERVACIONES").styled(header_st));
        doc.push(Paragraph::new(o).styled(normal_st));
        doc.push(Break::new(0.5));
    }

    if presupuesto > 0.0 || monto_final > 0.0 {
        doc.push(Break::new(1));
        if presupuesto > 0.0 { doc.push(Paragraph::new(&format!("Presupuesto: ${:.2}", presupuesto)).styled(label_st)); }
        if monto_final > 0.0 { doc.push(Paragraph::new(&format!("Total: ${:.2}", monto_final)).styled(header_st)); }
    }

    // Leyenda / términos configurables (Configuración → Servicio Técnico)
    if !leyenda_orden.trim().is_empty() {
        doc.push(Break::new(1.5));
        doc.push(Paragraph::new("TÉRMINOS Y CONDICIONES").styled(label_st));
        for linea in leyenda_orden.split('\n') {
            let linea = linea.trim_end();
            if linea.is_empty() {
                doc.push(Break::new(0.5));
            } else {
                doc.push(Paragraph::new(linea).styled(normal_st));
            }
        }
    }

    doc.push(Break::new(3));
    doc.push(Paragraph::new("___________________________").aligned(Alignment::Center).styled(normal_st));
    doc.push(Paragraph::new("Firma del Cliente").aligned(Alignment::Center).styled(normal_st));

    let temp_dir = std::env::temp_dir();
    let filename = format!("OrdenServicio-{}.pdf", numero.replace("/", "-"));
    let pdf_path = temp_dir.join(&filename);
    doc.render_to_file(&pdf_path).map_err(|e| format!("Error generando PDF: {}", e))?;

    #[cfg(target_os = "windows")]
    crate::utils::silent_command("cmd").args(["/C", "start", "", &pdf_path.to_string_lossy()]).spawn().ok();
    #[cfg(not(target_os = "windows"))]
    std::process::Command::new("xdg-open").arg(&pdf_path).spawn().ok();

    Ok(pdf_path.to_string_lossy().to_string())
}
