use crate::db::{Database, SesionState};
use crate::models::{OrdenServicio, MovimientoOrden};
use tauri::State;

#[tauri::command]
pub fn crear_orden_servicio(
    db: State<Database>,
    sesion: State<SesionState>,
    orden: OrdenServicio,
) -> Result<i64, String> {
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
         estado, fecha_promesa, presupuesto, garantia_dias, usuario_creador)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21, ?22, ?23, ?24)",
        rusqlite::params![
            numero, orden.cliente_id, orden.cliente_nombre, orden.cliente_telefono,
            orden.tipo_equipo, orden.equipo_descripcion, orden.equipo_marca, orden.equipo_modelo,
            orden.equipo_serie, orden.equipo_placa, orden.equipo_kilometraje, orden.equipo_kilometraje_proximo,
            orden.accesorios, orden.problema_reportado, orden.diagnostico, orden.trabajo_realizado,
            orden.observaciones, orden.tecnico_id, orden.tecnico_nombre,
            orden.estado, orden.fecha_promesa, orden.presupuesto, orden.garantia_dias, usuario.clone(),
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
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let id = orden.id.ok_or("ID requerido")?;

    conn.execute(
        "UPDATE ordenes_servicio SET cliente_id=?1, cliente_nombre=?2, cliente_telefono=?3,
         tipo_equipo=?4, equipo_descripcion=?5, equipo_marca=?6, equipo_modelo=?7, equipo_serie=?8,
         equipo_placa=?9, equipo_kilometraje=?10, equipo_kilometraje_proximo=?11, accesorios=?12,
         problema_reportado=?13, diagnostico=?14, trabajo_realizado=?15, observaciones=?16,
         tecnico_id=?17, tecnico_nombre=?18, fecha_promesa=?19, presupuesto=?20, monto_final=?21,
         garantia_dias=?22 WHERE id=?23",
        rusqlite::params![
            orden.cliente_id, orden.cliente_nombre, orden.cliente_telefono,
            orden.tipo_equipo, orden.equipo_descripcion, orden.equipo_marca, orden.equipo_modelo,
            orden.equipo_serie, orden.equipo_placa, orden.equipo_kilometraje, orden.equipo_kilometraje_proximo,
            orden.accesorios, orden.problema_reportado, orden.diagnostico, orden.trabajo_realizado,
            orden.observaciones, orden.tecnico_id, orden.tecnico_nombre, orden.fecha_promesa,
            orden.presupuesto, orden.monto_final, orden.garantia_dias, id,
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
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    conn.query_row(
        "SELECT id, numero, cliente_id, cliente_nombre, cliente_telefono, tipo_equipo,
         equipo_descripcion, equipo_marca, equipo_modelo, equipo_serie, equipo_placa,
         equipo_kilometraje, equipo_kilometraje_proximo, accesorios, problema_reportado,
         diagnostico, trabajo_realizado, observaciones, tecnico_id, tecnico_nombre,
         estado, fecha_ingreso, fecha_promesa, fecha_entrega, presupuesto, monto_final,
         garantia_dias, venta_id, usuario_creador FROM ordenes_servicio WHERE id = ?1",
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
            venta_id: row.get(27)?, usuario_creador: row.get(28)?,
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
         garantia_dias, venta_id, usuario_creador FROM ordenes_servicio WHERE 1=1"
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
            venta_id: row.get(27)?, usuario_creador: row.get(28)?,
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
         os.garantia_dias, os.venta_id, os.usuario_creador
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
            venta_id: row.get(27)?, usuario_creador: row.get(28)?,
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
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    conn.execute("DELETE FROM ordenes_servicio_imagenes WHERE id = ?1", rusqlite::params![imagen_id])
        .map_err(|e| e.to_string())?;
    Ok(())
}

// --- Cobrar orden -> crear venta ---

#[tauri::command]
pub fn cobrar_orden_servicio(
    db: State<Database>,
    sesion: State<SesionState>,
    orden_id: i64,
    forma_pago: String,
    monto_recibido: f64,
    items_repuestos: Vec<serde_json::Value>,
) -> Result<i64, String> {
    let (cliente_id, monto_final, numero_orden, equipo_descripcion): (Option<i64>, f64, String, String) = {
        let conn = db.conn.lock().map_err(|e| e.to_string())?;
        conn.query_row(
            "SELECT cliente_id, monto_final, numero, equipo_descripcion FROM ordenes_servicio WHERE id = ?1",
            rusqlite::params![orden_id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
        ).map_err(|e| e.to_string())?
    };

    if monto_final <= 0.0 && items_repuestos.is_empty() {
        return Err("La orden no tiene monto ni repuestos para cobrar".to_string());
    }

    let (usuario, usuario_id) = {
        let s = sesion.sesion.lock().map_err(|e| e.to_string())?;
        (
            s.as_ref().map(|s| s.nombre.clone()).unwrap_or_else(|| "Sistema".to_string()),
            s.as_ref().map(|s| s.usuario_id).unwrap_or(1),
        )
    };

    let mut conn = db.conn.lock().map_err(|e| e.to_string())?;
    let tx = conn.transaction().map_err(|e| e.to_string())?;

    let next_seq: i64 = tx.query_row(
        "SELECT COALESCE(MAX(CAST(SUBSTR(numero, 4) AS INTEGER)), 0) + 1 FROM ventas WHERE numero LIKE 'NV-%'",
        [], |r| r.get(0),
    ).unwrap_or(1);
    let numero = format!("NV-{:09}", next_seq);

    let mut subtotal_sin_iva = monto_final;
    let mut iva_total = 0.0_f64;

    let mut productos_data: Vec<(i64, f64, f64, f64, f64)> = Vec::new();
    for item in &items_repuestos {
        let pid = item["producto_id"].as_i64().unwrap_or(0);
        let cant = item["cantidad"].as_f64().unwrap_or(0.0);
        let precio = item["precio_unitario"].as_f64().unwrap_or(0.0);
        let iva_porc: f64 = tx.query_row(
            "SELECT iva_porcentaje FROM productos WHERE id = ?1",
            rusqlite::params![pid],
            |r| r.get(0),
        ).unwrap_or(0.0);
        let sub = cant * precio;
        if iva_porc > 0.0 {
            iva_total += sub * (iva_porc / 100.0);
        } else {
            subtotal_sin_iva += sub;
        }
        productos_data.push((pid, cant, precio, iva_porc, sub));
    }

    let total = subtotal_sin_iva + iva_total;
    let cambio = if monto_recibido > total { monto_recibido - total } else { 0.0 };
    let observacion = format!("Servicio Técnico {} - {}", numero_orden, equipo_descripcion);

    tx.execute(
        "INSERT INTO ventas (numero, cliente_id, subtotal_sin_iva, subtotal_con_iva, descuento, iva, total, forma_pago, monto_recibido, cambio, estado, tipo_documento, estado_sri, observacion, usuario, usuario_id, tipo_estado) VALUES (?1, ?2, ?3, 0, 0, ?4, ?5, ?6, ?7, ?8, 'COMPLETADA', 'NOTA_VENTA', 'NO_APLICA', ?9, ?10, ?11, 'COMPLETADA')",
        rusqlite::params![numero, cliente_id, subtotal_sin_iva, iva_total, total, forma_pago, monto_recibido, cambio, observacion, usuario, usuario_id],
    ).map_err(|e| e.to_string())?;
    let venta_id = tx.last_insert_rowid();

    if monto_final > 0.0 {
        tx.execute(
            "INSERT INTO venta_detalles (venta_id, producto_id, cantidad, precio_unitario, descuento, iva_porcentaje, subtotal, info_adicional) VALUES (?1, NULL, 1, ?2, 0, 0, ?2, ?3)",
            rusqlite::params![venta_id, monto_final, format!("Servicio: {} - {}", numero_orden, equipo_descripcion)],
        ).ok();
    }

    for (pid, cant, precio, iva_porc, sub) in &productos_data {
        tx.execute(
            "INSERT INTO venta_detalles (venta_id, producto_id, cantidad, precio_unitario, descuento, iva_porcentaje, subtotal) VALUES (?1, ?2, ?3, ?4, 0, ?5, ?6)",
            rusqlite::params![venta_id, pid, cant, precio, iva_porc, sub],
        ).ok();
        tx.execute(
            "UPDATE productos SET stock_actual = stock_actual - ?1 WHERE id = ?2 AND es_servicio = 0 AND no_controla_stock = 0",
            rusqlite::params![cant, pid],
        ).ok();
    }

    tx.execute(
        "UPDATE ordenes_servicio SET venta_id = ?1, estado = 'ENTREGADO', fecha_entrega = datetime('now','localtime') WHERE id = ?2",
        rusqlite::params![venta_id, orden_id],
    ).ok();

    tx.execute(
        "INSERT INTO ordenes_servicio_movimientos (orden_id, estado_anterior, estado_nuevo, observacion, usuario) VALUES (?1, 'LISTO', 'ENTREGADO', 'Cobrado y entregado', ?2)",
        rusqlite::params![orden_id, usuario],
    ).ok();

    tx.commit().map_err(|e| e.to_string())?;
    Ok(venta_id)
}

// --- PDF de la orden ---

#[tauri::command]
pub fn imprimir_orden_servicio_pdf(db: State<Database>, orden_id: i64) -> Result<String, String> {
    use genpdf::{elements::*, fonts, style::*, Alignment, Margins, Document, Element, SimplePageDecorator};

    let (
        nombre_negocio, ruc, direccion, telefono,
        numero, cliente_nombre, cliente_telefono, tipo_equipo, equipo_descripcion,
        equipo_marca, equipo_modelo, equipo_serie, equipo_placa, accesorios,
        diagnostico, problema_reportado, trabajo_realizado, observaciones,
        estado, fecha_ingreso, presupuesto, monto_final,
    ): (
        String, String, String, String,
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

        (
            nombre_negocio, ruc, direccion, telefono,
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
    decorator.set_margins(Margins::trbl(15, 15, 15, 15));
    doc.set_page_decorator(decorator);

    let title_st = Style::new().bold().with_font_size(16);
    let header_st = Style::new().bold().with_font_size(12);
    let label_st = Style::new().bold().with_font_size(10);
    let normal_st = Style::new().with_font_size(10);

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

    doc.push(Break::new(3));
    doc.push(Paragraph::new("___________________________            ___________________________").aligned(Alignment::Center).styled(normal_st));
    doc.push(Paragraph::new("Firma del Cliente                                       Firma del Técnico").aligned(Alignment::Center).styled(normal_st));

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
