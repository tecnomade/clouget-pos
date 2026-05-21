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

    // v2.4.27: prefijo OT (Orden de Trabajo) — terminologia local en EC.
    // El secuencial continua desde el max de OS- + OT- (compatibilidad con datos viejos).
    let next: i64 = conn.query_row(
        "SELECT COALESCE(MAX(CAST(SUBSTR(numero, 4) AS INTEGER)), 0) + 1 FROM ordenes_servicio WHERE numero LIKE 'OS-%' OR numero LIKE 'OT-%'",
        [], |r| r.get(0)
    ).unwrap_or(1);
    let numero = format!("OT-{:06}", next);

    // v2.4.25: si vino intervalo y entrada pero no proximo, calcular auto.
    let proximo_calc = match (orden.equipo_kilometraje, orden.equipo_kilometraje_intervalo, orden.equipo_kilometraje_proximo) {
        (Some(entrada), Some(intervalo), None) if intervalo > 0 => Some(entrada + intervalo),
        (_, _, p) => p,
    };

    conn.execute(
        "INSERT INTO ordenes_servicio (numero, cliente_id, cliente_nombre, cliente_telefono,
         tipo_equipo, equipo_descripcion, equipo_marca, equipo_modelo, equipo_serie, equipo_placa,
         equipo_kilometraje, equipo_kilometraje_proximo, accesorios, problema_reportado,
         diagnostico, trabajo_realizado, observaciones, tecnico_id, tecnico_nombre,
         estado, fecha_promesa, presupuesto, garantia_dias, usuario_creador,
         tipo_equipo_id, marca_id, modelo_id,
         equipo_kilometraje_intervalo, equipo_kilometraje_salida)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21, ?22, ?23, ?24, ?25, ?26, ?27, ?28, ?29)",
        rusqlite::params![
            numero, orden.cliente_id, orden.cliente_nombre, orden.cliente_telefono,
            orden.tipo_equipo, orden.equipo_descripcion, orden.equipo_marca, orden.equipo_modelo,
            orden.equipo_serie, orden.equipo_placa, orden.equipo_kilometraje, proximo_calc,
            orden.accesorios, orden.problema_reportado, orden.diagnostico, orden.trabajo_realizado,
            orden.observaciones, orden.tecnico_id, orden.tecnico_nombre,
            orden.estado, orden.fecha_promesa, orden.presupuesto, orden.garantia_dias, usuario.clone(),
            orden.tipo_equipo_id, orden.marca_id, orden.modelo_id,
            orden.equipo_kilometraje_intervalo, orden.equipo_kilometraje_salida,
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

    // v2.4.25: si hay km salida + intervalo → recalcular próximo desde salida.
    // Si solo hay entrada + intervalo → desde entrada. Si vino próximo explícito, ese gana.
    let proximo_calc = match (
        orden.equipo_kilometraje_salida,
        orden.equipo_kilometraje,
        orden.equipo_kilometraje_intervalo,
        orden.equipo_kilometraje_proximo,
    ) {
        (Some(salida), _, Some(intervalo), _) if intervalo > 0 => Some(salida + intervalo),
        (_, Some(entrada), Some(intervalo), None) if intervalo > 0 => Some(entrada + intervalo),
        (_, _, _, p) => p,
    };

    conn.execute(
        "UPDATE ordenes_servicio SET cliente_id=?1, cliente_nombre=?2, cliente_telefono=?3,
         tipo_equipo=?4, equipo_descripcion=?5, equipo_marca=?6, equipo_modelo=?7, equipo_serie=?8,
         equipo_placa=?9, equipo_kilometraje=?10, equipo_kilometraje_proximo=?11, accesorios=?12,
         problema_reportado=?13, diagnostico=?14, trabajo_realizado=?15, observaciones=?16,
         tecnico_id=?17, tecnico_nombre=?18, fecha_promesa=?19, presupuesto=?20, monto_final=?21,
         garantia_dias=?22, tipo_equipo_id=?23, marca_id=?24, modelo_id=?25,
         equipo_kilometraje_intervalo=?26, equipo_kilometraje_salida=?27 WHERE id=?28",
        rusqlite::params![
            orden.cliente_id, orden.cliente_nombre, orden.cliente_telefono,
            orden.tipo_equipo, orden.equipo_descripcion, orden.equipo_marca, orden.equipo_modelo,
            orden.equipo_serie, orden.equipo_placa, orden.equipo_kilometraje, proximo_calc,
            orden.accesorios, orden.problema_reportado, orden.diagnostico, orden.trabajo_realizado,
            orden.observaciones, orden.tecnico_id, orden.tecnico_nombre, orden.fecha_promesa,
            orden.presupuesto, orden.monto_final, orden.garantia_dias,
            orden.tipo_equipo_id, orden.marca_id, orden.modelo_id,
            orden.equipo_kilometraje_intervalo, orden.equipo_kilometraje_salida, id,
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

    // v2.4.22: bloquear cambios desde estados "cerrados" (post-cobro/cancelación).
    // Razón: la orden ENTREGADO/ENTREGADO_PARCIAL ya tiene venta generada y abonos
    // APLICADOS; CANCELADA tiene abonos DEVUELTOS al cliente. Retroceder generaría
    // inconsistencia (la caja no muestra holdings que la orden parecería tener).
    // Si se necesita reabrir, hay que anular la venta primero (manualmente).
    let cerrados = ["ENTREGADO", "ENTREGADO_PARCIAL", "CANCELADA", "CANCELADO"];
    if cerrados.contains(&estado_anterior.as_str()) && estado_anterior != nuevo_estado {
        return Err(format!(
            "La orden está en estado {} y no se puede cambiar. Si necesitas reabrirla, anula la venta vinculada desde Ventas del Día.",
            estado_anterior
        ));
    }
    // También bloquear forzar a estados cerrados desde aquí — el flujo correcto
    // para entregar es "💰 Cobrar" y para cancelar "🚫 Cancelar orden".
    if cerrados.contains(&nuevo_estado.as_str()) {
        return Err(format!(
            "Para llegar al estado {} usa el flujo correspondiente (Cobrar / Cancelar orden), no el cambio manual de estado.",
            nuevo_estado
        ));
    }

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
         garantia_dias, venta_id, usuario_creador, tipo_equipo_id, marca_id, modelo_id,
         equipo_kilometraje_intervalo, equipo_kilometraje_salida
         FROM ordenes_servicio WHERE id = ?1",
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
            equipo_kilometraje_intervalo: row.get(32).ok(),
            equipo_kilometraje_salida: row.get(33).ok(),
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
            equipo_kilometraje_intervalo: None,
            equipo_kilometraje_salida: None,
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
            equipo_kilometraje_intervalo: None,
            equipo_kilometraje_salida: None,
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

    // v2.5.11 BUG FIX: bloquear eliminacion si hay abonos en cualquier estado.
    // Antes solo se chequeaba venta_id, dejando que se elimine una orden con
    // abonos en HOLDING — eso causa que el dinero entre a caja sin contrapartida
    // y rompe la contabilidad. Si la orden tiene abonos, hay que usar
    // "Cancelar orden" (st_cancelar_orden) que devuelve los abonos correctamente.
    let total_abonos: i64 = conn.query_row(
        "SELECT COUNT(*) FROM st_abonos WHERE orden_id = ?1",
        rusqlite::params![id], |r| r.get(0)
    ).unwrap_or(0);
    if total_abonos > 0 {
        return Err(format!(
            "No se puede eliminar esta orden porque tiene {} abono(s) registrado(s) en caja. \
             Si querés anular la orden, usá 'Cancelar orden' — eso devuelve los abonos \
             en holding automáticamente.",
            total_abonos
        ));
    }

    // v2.5.11: tambien bloquear si tiene items presupuestados (actividad documentada).
    // Es razonable que el usuario al menos limpie los items antes de eliminar.
    let total_items: i64 = conn.query_row(
        "SELECT COUNT(*) FROM orden_servicio_items WHERE orden_id = ?1",
        rusqlite::params![id], |r| r.get(0)
    ).unwrap_or(0);
    if total_items > 0 {
        return Err(format!(
            "No se puede eliminar esta orden porque tiene {} item(s) en el detalle. \
             Eliminá los items primero o usá 'Cancelar orden' para anularla preservando la traza.",
            total_items
        ));
    }

    let venta_id: Option<i64> = conn.query_row(
        "SELECT venta_id FROM ordenes_servicio WHERE id = ?1",
        rusqlite::params![id], |r| r.get(0)
    ).unwrap_or(None);
    if venta_id.is_some() {
        // Si tiene venta vinculada, no eliminar fisicamente — marcar cancelada
        // (la venta tiene su propio ciclo de vida, no podemos romper integridad)
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
                // v2.5.25 BUG FIX: aplicar misma lógica que registrar_venta — si el producto
                // es COMBO, descontar componentes; si es simple, descontar del padre.
                // Auto-healing por si tipo_producto está mal en BD: chequea también si
                // existen registros en producto_componentes.
                let (tipo_prod, n_comp): (String, i64) = tx.query_row(
                    "SELECT COALESCE(tipo_producto, 'SIMPLE'),
                            (SELECT COUNT(*) FROM producto_componentes WHERE producto_padre_id = productos.id)
                     FROM productos WHERE id = ?1",
                    rusqlite::params![pid],
                    |row| Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?)),
                ).unwrap_or(("SIMPLE".to_string(), 0));
                let es_combo = tipo_prod == "COMBO_FIJO" || tipo_prod == "COMBO_FLEXIBLE" || n_comp > 0;

                if !es_combo {
                    // Producto simple: descontar del padre y registrar kardex con motivo
                    // (v2.5.27: agregar trazabilidad en kardex — antes solo hacía UPDATE)
                    let (stock_p_antes, costo_p): (f64, f64) = tx.query_row(
                        "SELECT stock_actual, precio_costo FROM productos WHERE id = ?1 AND COALESCE(es_servicio,0) = 0 AND COALESCE(no_controla_stock,0) = 0",
                        rusqlite::params![pid],
                        |row| Ok((row.get::<_, f64>(0)?, row.get::<_, f64>(1)?)),
                    ).unwrap_or((0.0, 0.0));
                    tx.execute(
                        "UPDATE productos SET stock_actual = stock_actual - ?1 WHERE id = ?2 AND es_servicio = 0 AND no_controla_stock = 0",
                        rusqlite::params![it.cantidad, pid],
                    ).ok();
                    let motivo_st = format!("Venta ST {} (orden {})", numero, numero_orden);
                    let _ = tx.execute(
                        "INSERT INTO movimientos_inventario (producto_id, tipo, cantidad, stock_anterior, stock_nuevo, costo_unitario, referencia_id, usuario, establecimiento_id, motivo)
                         VALUES (?1, 'VENTA', ?2, ?3, ?4, ?5, ?6, ?7, NULL, ?8)",
                        rusqlite::params![pid, -it.cantidad, stock_p_antes, stock_p_antes - it.cantidad, costo_p, venta_id, usuario, motivo_st],
                    );
                } else {
                    // Combo: descontar componentes según producto_componentes
                    // Obtener nombre del combo padre para incluirlo en el motivo del kardex
                    let nombre_combo: String = tx.query_row(
                        "SELECT nombre FROM productos WHERE id = ?1",
                        rusqlite::params![pid], |r| r.get(0),
                    ).unwrap_or_else(|_| format!("#{}", pid));
                    let componentes: Vec<(i64, f64)> = {
                        let mut stmt = match tx.prepare(
                            "SELECT producto_hijo_id, cantidad FROM producto_componentes WHERE producto_padre_id = ?1"
                        ) {
                            Ok(s) => s,
                            Err(_) => continue,
                        };
                        let mut comps: Vec<(i64, f64)> = Vec::new();
                        if let Ok(iter) = stmt.query_map(rusqlite::params![pid], |r| {
                            Ok((r.get::<_, i64>(0)?, r.get::<_, f64>(1)?))
                        }) {
                            for r in iter {
                                if let Ok(c) = r { comps.push(c); }
                            }
                        }
                        comps
                    };

                    if componentes.is_empty() {
                        eprintln!("[ST Combo VACIO] Producto {} (orden #{}) vendido como combo pero sin componentes. Stock NO descontado.", pid, orden_id);
                    }
                    let motivo_combo = format!("Venta ST {} (orden {} · combo: {})", numero, numero_orden, nombre_combo);
                    for (hijo_id, cant_componente) in componentes {
                        let cant_total = cant_componente * it.cantidad;
                        // Solo descontar si el hijo controla stock (no servicio, no no_controla_stock)
                        tx.execute(
                            "UPDATE productos SET stock_actual = stock_actual - ?1
                             WHERE id = ?2 AND COALESCE(es_servicio,0) = 0 AND COALESCE(no_controla_stock,0) = 0",
                            rusqlite::params![cant_total, hijo_id],
                        ).ok();
                        // Registrar movimiento de kardex (VENTA_COMBO) con motivo trazable
                        let (stock_h_antes, costo_h): (f64, f64) = tx.query_row(
                            "SELECT stock_actual + ?1, precio_costo FROM productos WHERE id = ?2",
                            rusqlite::params![cant_total, hijo_id],
                            |r| Ok((r.get::<_, f64>(0)?, r.get::<_, f64>(1)?)),
                        ).unwrap_or((0.0, 0.0));
                        let _ = tx.execute(
                            "INSERT INTO movimientos_inventario (producto_id, tipo, cantidad, stock_anterior, stock_nuevo, costo_unitario, referencia_id, usuario, establecimiento_id, motivo)
                             VALUES (?1, 'VENTA_COMBO', ?2, ?3, ?4, ?5, ?6, ?7, NULL, ?8)",
                            rusqlite::params![hijo_id, -cant_total, stock_h_antes, stock_h_antes - cant_total, costo_h, venta_id, usuario, motivo_combo],
                        );
                    }
                }
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
    // v2.4.29: tipo de documento: "ORDEN" (default, comportamiento clasico) | "COTIZACION"
    // Cotizacion = lista items presupuestados + texto de validez, sin abonos/pagos/garantia.
    tipo: Option<String>,
) -> Result<String, String> {
    requiere_modulo_servicio_tecnico(&db)?;
    use genpdf::{elements::*, fonts, style::*, Alignment, Margins, Document, Element, SimplePageDecorator};

    let tipo_doc = tipo.unwrap_or_else(|| "ORDEN".to_string()).to_uppercase();
    let es_cotizacion = tipo_doc == "COTIZACION";

    // v2.4.12 ST-4: formato configurable. Default A4 (compatibilidad con clientes
    // que no actualicen el frontend). Opciones: "A4" | "TICKET_80"
    let formato_efectivo = formato.unwrap_or_else(|| "A4".to_string()).to_uppercase();
    let es_ticket = formato_efectivo == "TICKET_80";

    // v2.4.23: incluir abonos en HOLDING para mostrar en el PDF
    #[derive(Debug, Clone)]
    struct AbonoSimple {
        monto: f64,
        forma_pago: String,
        fecha: String,
        referencia: Option<String>,
    }
    // v2.4.27: pagos hechos AL COBRO (de pagos_venta).
    #[derive(Debug, Clone)]
    struct PagoCobroSimple {
        monto: f64,
        forma_pago: String,
        referencia: Option<String>,
    }

    let (
        nombre_negocio, ruc, direccion, telefono, leyenda_orden,
        numero, cliente_nombre, cliente_telefono, tipo_equipo, equipo_descripcion,
        equipo_marca, equipo_modelo, equipo_serie, equipo_placa, accesorios,
        diagnostico, problema_reportado, trabajo_realizado, observaciones,
        estado, fecha_ingreso, presupuesto, monto_final,
        km_entrada, km_proximo, km_intervalo, km_salida,
        garantia_dias, venta_id_opt, fecha_entrega_opt,
        abonos, total_abonos,
        pagos_cobro, total_pagos_cobro,
    ): (
        String, String, String, String, String,
        String, Option<String>, Option<String>, String, String,
        String, Option<String>, Option<String>, Option<String>, Option<String>,
        String, String, Option<String>, Option<String>,
        String, String, f64, f64,
        Option<i64>, Option<i64>, Option<i64>, Option<i64>,
        i64, Option<i64>, Option<String>,
        Vec<AbonoSimple>, f64,
        Vec<PagoCobroSimple>, f64,
    ) = {
        let conn = db.conn.lock().map_err(|e| e.to_string())?;
        // v2.4.26: incluir kilometraje. v2.4.27: tambien garantia + venta_id + fecha_entrega.
        let orden = conn.query_row(
            "SELECT numero, cliente_nombre, cliente_telefono, tipo_equipo, equipo_descripcion,
             COALESCE(equipo_marca,''), equipo_modelo, equipo_serie, equipo_placa, accesorios,
             COALESCE(diagnostico,''), problema_reportado, trabajo_realizado, observaciones,
             estado, fecha_ingreso, presupuesto, monto_final,
             equipo_kilometraje, equipo_kilometraje_proximo,
             equipo_kilometraje_intervalo, equipo_kilometraje_salida,
             COALESCE(garantia_dias, 0), venta_id, fecha_entrega
             FROM ordenes_servicio WHERE id = ?1",
            rusqlite::params![orden_id],
            |r| Ok((
                r.get::<_, String>(0)?, r.get::<_, Option<String>>(1)?, r.get::<_, Option<String>>(2)?,
                r.get::<_, String>(3)?, r.get::<_, String>(4)?, r.get::<_, String>(5)?,
                r.get::<_, Option<String>>(6)?, r.get::<_, Option<String>>(7)?, r.get::<_, Option<String>>(8)?,
                r.get::<_, Option<String>>(9)?, r.get::<_, String>(10)?, r.get::<_, String>(11)?,
                r.get::<_, Option<String>>(12)?, r.get::<_, Option<String>>(13)?,
                r.get::<_, String>(14)?, r.get::<_, String>(15)?, r.get::<_, f64>(16)?, r.get::<_, f64>(17)?,
                r.get::<_, Option<i64>>(18)?, r.get::<_, Option<i64>>(19)?,
                r.get::<_, Option<i64>>(20).ok().flatten(), r.get::<_, Option<i64>>(21).ok().flatten(),
                r.get::<_, i64>(22)?, r.get::<_, Option<i64>>(23)?, r.get::<_, Option<String>>(24)?,
            )),
        ).map_err(|e| format!("Orden no encontrada: {}", e))?;

        let nombre_negocio: String = conn.query_row("SELECT value FROM config WHERE key = 'nombre_negocio'", [], |r| r.get(0)).unwrap_or_else(|_| "Mi Negocio".to_string());
        let ruc: String = conn.query_row("SELECT value FROM config WHERE key = 'ruc'", [], |r| r.get(0)).unwrap_or_default();
        let direccion: String = conn.query_row("SELECT value FROM config WHERE key = 'direccion'", [], |r| r.get(0)).unwrap_or_default();
        let telefono: String = conn.query_row("SELECT value FROM config WHERE key = 'telefono'", [], |r| r.get(0)).unwrap_or_default();
        let leyenda_orden: String = conn.query_row("SELECT value FROM config WHERE key = 'leyenda_orden_servicio'", [], |r| r.get(0)).unwrap_or_default();

        // v2.4.23: cargar abonos en HOLDING para mostrar en el PDF.
        // Si la orden ya está cobrada, los abonos están en APLICADO — los mostramos
        // igual porque son parte del registro de la orden.
        let abonos: Vec<AbonoSimple> = {
            let mut stmt = conn.prepare(
                "SELECT monto, forma_pago, fecha, referencia_pago
                 FROM st_abonos
                 WHERE orden_id = ?1 AND estado IN ('HOLDING', 'APLICADO')
                 ORDER BY fecha ASC"
            ).map_err(|e| e.to_string())?;
            let rows = stmt.query_map(rusqlite::params![orden_id], |r| Ok(AbonoSimple {
                monto: r.get(0)?,
                forma_pago: r.get(1)?,
                fecha: r.get(2)?,
                referencia: r.get(3).ok(),
            })).map_err(|e| e.to_string())?
              .collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())?;
            rows
        };
        let total_abonos: f64 = abonos.iter().map(|a| a.monto).sum();

        // v2.4.27: pagos hechos al cobrar la orden (de pagos_venta).
        // Si la orden tiene venta_id, traemos el desglose. Si la venta usa pago mixto,
        // habrá varias filas. Si fue legacy (forma_pago única en `ventas`), construimos
        // una entrada sintética desde la tabla `ventas`.
        let (pagos_cobro, total_pagos_cobro): (Vec<PagoCobroSimple>, f64) = {
            if let Some(vid) = orden.23 {
                let mut stmt_res = conn.prepare(
                    "SELECT monto, forma_pago, referencia FROM pagos_venta WHERE venta_id = ?1 ORDER BY id ASC"
                );
                let pagos: Vec<PagoCobroSimple> = if let Ok(mut stmt) = stmt_res.as_mut() {
                    let rows = stmt.query_map(rusqlite::params![vid], |r| Ok(PagoCobroSimple {
                        monto: r.get(0)?,
                        forma_pago: r.get(1)?,
                        referencia: r.get(2).ok(),
                    })).map(|it| it.collect::<Result<Vec<_>, _>>().unwrap_or_default())
                    .unwrap_or_default();
                    rows
                } else { Vec::new() };
                let pagos = if pagos.is_empty() {
                    // Fallback a `ventas.total + forma_pago` (caso legacy o sin pagos_venta).
                    conn.query_row(
                        "SELECT total, forma_pago FROM ventas WHERE id = ?1",
                        rusqlite::params![vid],
                        |r| Ok(PagoCobroSimple {
                            monto: r.get(0)?,
                            forma_pago: r.get(1)?,
                            referencia: None,
                        }),
                    ).ok().map(|p| vec![p]).unwrap_or_default()
                } else { pagos };
                let total: f64 = pagos.iter().map(|p| p.monto).sum();
                (pagos, total)
            } else {
                (Vec::new(), 0.0)
            }
        };

        (
            nombre_negocio, ruc, direccion, telefono, leyenda_orden,
            orden.0, orden.1, orden.2, orden.3, orden.4,
            orden.5, orden.6, orden.7, orden.8, orden.9,
            orden.10, orden.11, orden.12, orden.13,
            orden.14, orden.15, orden.16, orden.17,
            orden.18, orden.19, orden.20, orden.21,
            orden.22, orden.23, orden.24,
            abonos, total_abonos,
            pagos_cobro, total_pagos_cobro,
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

    // v2.4.29: titulo cambia segun tipo (orden vs cotizacion)
    let titulo_doc = if es_cotizacion { "COTIZACIÓN" } else { "ORDEN DE SERVICIO TÉCNICO" };
    doc.push(Paragraph::new(titulo_doc).aligned(Alignment::Center).styled(header_st));
    let numero_doc = if es_cotizacion { format!("COT-{}", numero.trim_start_matches("OT-").trim_start_matches("OS-")) } else { numero.clone() };
    doc.push(Paragraph::new(&format!("No: {}    Fecha: {}", numero_doc, fecha_ingreso)).aligned(Alignment::Center).styled(normal_st));
    if !es_cotizacion {
        doc.push(Paragraph::new(&format!("Estado: {}", estado)).aligned(Alignment::Center).styled(normal_st));
    }
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
    // v2.4.26: kilometraje (entrada / salida / próximo / intervalo)
    if km_entrada.is_some() || km_salida.is_some() || km_proximo.is_some() {
        if let Some(k) = km_entrada { doc.push(Paragraph::new(&format!("Km entrada: {}", k)).styled(normal_st)); }
        if let Some(k) = km_salida { doc.push(Paragraph::new(&format!("Km salida: {}", k)).styled(normal_st)); }
        if let Some(k) = km_proximo {
            let suffix = match km_intervalo {
                Some(i) if i > 0 => format!(" (cada {} km)", i),
                _ => String::new(),
            };
            doc.push(Paragraph::new(&format!("Próximo mantenimiento: {} km{}", k, suffix)).styled(normal_st));
        }
    }
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

    // v2.4.29: en COTIZACION, listamos los items presupuestados con precios.
    // Esto da al cliente el detalle de qué se le va a cobrar antes de aprobar el trabajo.
    if es_cotizacion {
        let items_cot: Vec<(String, f64, f64, f64)> = {
            let conn = db.conn.lock().map_err(|e| e.to_string())?;
            let mut stmt = match conn.prepare(
                "SELECT descripcion, cantidad, precio_unitario, iva_porcentaje
                 FROM orden_servicio_items WHERE orden_id = ?1 ORDER BY id ASC"
            ) {
                Ok(s) => s,
                Err(_) => return Err("Error preparando query items".to_string()),
            };
            let rows = stmt.query_map(rusqlite::params![orden_id], |r| Ok((
                r.get::<_, String>(0)?, r.get::<_, f64>(1)?, r.get::<_, f64>(2)?, r.get::<_, f64>(3)?,
            ))).ok().map(|it| it.collect::<Result<Vec<_>, _>>().unwrap_or_default())
              .unwrap_or_default();
            rows
        };
        if !items_cot.is_empty() {
            doc.push(Break::new(1));
            doc.push(Paragraph::new("DETALLE DE COTIZACIÓN").styled(header_st));
            // v2.5.11: A4 usa tabla por columnas (estilo nota de venta) en vez de
            // viñetas planas. 80mm mantiene formato multi-linea (mejor en angosto).
            let mut subtotal = 0.0_f64;
            let mut iva_total = 0.0_f64;

            if es_ticket {
                // ── 80mm: descripcion + linea de cantidad/precio ──
                for (desc, cant, precio, iva_pct) in &items_cot {
                    let st = cant * precio;
                    let iva_item = st * iva_pct / 100.0;
                    subtotal += st;
                    iva_total += iva_item;
                    doc.push(Paragraph::new(&format!("• {}", desc)).styled(normal_st));
                    doc.push(Paragraph::new(&format!("  {} x ${:.2}  =  ${:.2}", cant, precio, st)).styled(normal_st));
                }
                doc.push(Break::new(0.3));
                doc.push(Paragraph::new("--------------------------------").styled(normal_st));
            } else {
                // ── A4: tabla con columnas # | Descripcion | Cant | P.Unit | Subtotal ──
                let s_small = Style::new().with_font_size(9);
                let s_small_bold = Style::new().with_font_size(9).bold();
                // Helpers locales (mismo patron que nota_venta_pdf.rs)
                let pp = |text: String, style: Style| Paragraph::new(text).styled(style).padded(Margins::trbl(2, 2, 2, 4));
                let pp_right = |text: String, style: Style| Paragraph::new(text).aligned(Alignment::Right).styled(style).padded(Margins::trbl(2, 4, 2, 2));

                // Columnas proporcionales: # (1) | Descripcion (8) | Cant (2) | P.Unit (2) | Subtotal (2)
                let mut tabla = TableLayout::new(vec![1, 8, 2, 2, 2]);
                tabla.set_cell_decorator(FrameCellDecorator::new(true, true, false));

                // Cabecera
                tabla.row()
                    .element(pp("#".to_string(), s_small_bold))
                    .element(pp("Descripción".to_string(), s_small_bold))
                    .element(pp_right("Cant.".to_string(), s_small_bold))
                    .element(pp_right("P.Unit.".to_string(), s_small_bold))
                    .element(pp_right("Subtotal".to_string(), s_small_bold))
                    .push()
                    .map_err(|e| format!("Error tabla cabecera cotizacion: {}", e))?;

                // Filas
                for (i, (desc, cant, precio, iva_pct)) in items_cot.iter().enumerate() {
                    let st = cant * precio;
                    let iva_item = st * iva_pct / 100.0;
                    subtotal += st;
                    iva_total += iva_item;
                    let cant_fmt = if *cant == cant.floor() { format!("{:.0}", cant) } else { format!("{:.2}", cant) };
                    tabla.row()
                        .element(pp(format!("{}", i + 1), s_small))
                        .element(pp(desc.clone(), s_small))
                        .element(pp_right(cant_fmt, s_small))
                        .element(pp_right(format!("${:.2}", precio), s_small))
                        .element(pp_right(format!("${:.2}", st), s_small))
                        .push()
                        .map_err(|e| format!("Error tabla fila cotizacion: {}", e))?;
                }
                doc.push(tabla);
                doc.push(Break::new(0.5));
            }

            doc.push(Paragraph::new(&format!("Subtotal: ${:.2}", subtotal)).styled(label_st));
            if iva_total > 0.001 {
                doc.push(Paragraph::new(&format!("IVA: ${:.2}", iva_total)).styled(label_st));
            }
            doc.push(Paragraph::new(&format!("TOTAL: ${:.2}", subtotal + iva_total)).styled(header_st));
        } else if presupuesto > 0.0 {
            // Fallback al presupuesto si no hay items detallados
            doc.push(Break::new(1));
            doc.push(Paragraph::new(&format!("Presupuesto: ${:.2}", presupuesto)).styled(header_st));
        }
    } else if presupuesto > 0.0 || monto_final > 0.0 {
        doc.push(Break::new(1));
        if presupuesto > 0.0 { doc.push(Paragraph::new(&format!("Presupuesto: ${:.2}", presupuesto)).styled(label_st)); }
        if monto_final > 0.0 { doc.push(Paragraph::new(&format!("Total: ${:.2}", monto_final)).styled(header_st)); }
    }

    // v2.4.23: ABONOS recibidos (HOLDING o APLICADOS al cobro).
    // v2.4.29: en cotizacion no se muestran (la cotizacion es un documento previo al cobro).
    if !es_cotizacion && !abonos.is_empty() {
        doc.push(Break::new(1));
        doc.push(Paragraph::new("ABONOS RECIBIDOS").styled(header_st));
        for a in &abonos {
            let fecha_corta = a.fecha.split(' ').next().unwrap_or(&a.fecha);
            let mut linea = format!("• {} · ${:.2} · {}", fecha_corta, a.monto, a.forma_pago);
            if let Some(ref r) = a.referencia {
                if !r.trim().is_empty() {
                    linea.push_str(&format!(" · ref: {}", r));
                }
            }
            doc.push(Paragraph::new(&linea).styled(normal_st));
        }
        doc.push(Paragraph::new(&format!("Total abonado: ${:.2}", total_abonos)).styled(label_st));
    }

    // v2.4.27: PAGOS HECHOS AL COBRO (de pagos_venta).
    // Antes el PDF solo mostraba abonos y dejaba "saldo pendiente" aún cuando ya
    // se había cobrado el remanente al entregar. Ahora reflejamos ambos.
    // v2.4.29: en cotizacion no se muestran (no hay cobro todavia).
    if !es_cotizacion && !pagos_cobro.is_empty() {
        doc.push(Break::new(1));
        let titulo_pagos = if abonos.is_empty() { "PAGO RECIBIDO" } else { "PAGO AL COBRO" };
        doc.push(Paragraph::new(titulo_pagos).styled(header_st));
        let fecha_pago = fecha_entrega_opt.as_deref().unwrap_or("").split(' ').next().unwrap_or("").to_string();
        for p in &pagos_cobro {
            let mut linea = if !fecha_pago.is_empty() {
                format!("• {} · ${:.2} · {}", fecha_pago, p.monto, p.forma_pago)
            } else {
                format!("• ${:.2} · {}", p.monto, p.forma_pago)
            };
            if let Some(ref r) = p.referencia {
                if !r.trim().is_empty() {
                    linea.push_str(&format!(" · ref: {}", r));
                }
            }
            doc.push(Paragraph::new(&linea).styled(normal_st));
        }
        if pagos_cobro.len() > 1 {
            doc.push(Paragraph::new(&format!("Total pagado al cobro: ${:.2}", total_pagos_cobro)).styled(label_st));
        }
    }

    // v2.4.27: SALDO REAL = max(monto_final, presupuesto) - (abonos + pagos_cobro)
    // v2.4.29: en cotizacion no se calcula saldo (no hay pagos aplicados).
    if !es_cotizacion {
        let referencia = if monto_final > 0.0 { monto_final } else { presupuesto };
        let total_recibido = total_abonos + total_pagos_cobro;
        if referencia > 0.0 && total_recibido > 0.0 {
            let saldo = (referencia - total_recibido).max(0.0);
            doc.push(Break::new(0.5));
            if saldo > 0.001 {
                doc.push(Paragraph::new(&format!("Saldo pendiente: ${:.2}", saldo)).styled(header_st));
            } else if total_recibido >= referencia - 0.001 {
                doc.push(Paragraph::new("CANCELADO TOTALMENTE").styled(header_st));
            }
        }
    }

    // v2.4.27: GARANTÍA del trabajo (si aplica).
    // v2.4.29: en cotizacion no se muestra (la garantia se documenta al entregar).
    if !es_cotizacion && garantia_dias > 0 {
        doc.push(Break::new(0.8));
        doc.push(Paragraph::new(&format!("🛡 Garantía del trabajo: {} día{}",
            garantia_dias, if garantia_dias == 1 { "" } else { "s" })).styled(label_st));
        // Si tenemos fecha de entrega, calcular fecha de vencimiento de la garantía.
        if let Some(ref fe) = fecha_entrega_opt {
            let fecha_part = fe.split(' ').next().unwrap_or("");
            if let Ok(parsed) = chrono::NaiveDate::parse_from_str(fecha_part, "%Y-%m-%d") {
                let venc = parsed + chrono::Duration::days(garantia_dias);
                doc.push(Paragraph::new(&format!("Válida hasta: {}", venc.format("%d/%m/%Y"))).styled(normal_st));
            }
        }
    }
    let _ = venta_id_opt; // por ahora solo lo usamos para query de pagos

    // v2.4.29: linea de validez al final de la COTIZACION.
    if es_cotizacion {
        let validez_dias: i64 = {
            let conn = db.conn.lock().map_err(|e| e.to_string())?;
            conn.query_row(
                "SELECT CAST(value AS INTEGER) FROM config WHERE key = 'st_cotizacion_validez_dias'",
                [], |r| r.get(0),
            ).unwrap_or(30)
        };
        let validez_dias = if validez_dias > 0 { validez_dias } else { 30 };
        doc.push(Break::new(0.8));
        doc.push(Paragraph::new(&format!("📅 Cotización válida por {} día{}.",
            validez_dias, if validez_dias == 1 { "" } else { "s" })).styled(label_st));
        let hoy = chrono::Local::now().date_naive();
        let venc = hoy + chrono::Duration::days(validez_dias);
        doc.push(Paragraph::new(&format!("Vence el: {}", venc.format("%d/%m/%Y"))).styled(normal_st));
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
    doc.push(Paragraph::new(if es_cotizacion { "Aceptación del Cliente" } else { "Firma del Cliente" }).aligned(Alignment::Center).styled(normal_st));

    let temp_dir = std::env::temp_dir();
    let filename = if es_cotizacion {
        format!("Cotizacion-{}.pdf", numero.replace("/", "-"))
    } else {
        format!("OrdenServicio-{}.pdf", numero.replace("/", "-"))
    };
    let pdf_path = temp_dir.join(&filename);
    doc.render_to_file(&pdf_path).map_err(|e| format!("Error generando PDF: {}", e))?;

    #[cfg(target_os = "windows")]
    crate::utils::silent_command("cmd").args(["/C", "start", "", &pdf_path.to_string_lossy()]).spawn().ok();
    #[cfg(not(target_os = "windows"))]
    std::process::Command::new("xdg-open").arg(&pdf_path).spawn().ok();

    Ok(pdf_path.to_string_lossy().to_string())
}
