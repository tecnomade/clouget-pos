//! Comandos Tauri del módulo Restaurante.
//!
//! Todos los comandos chequean primero `requiere_modulo_restaurante()` para
//! asegurar que la licencia activa tenga el módulo habilitado. Esto permite
//! que el módulo viva en el binario pero esté inactivo si el cliente no
//! pagó por él.
//!
//! Comandos expuestos (registrar en `lib.rs::invoke_handler`):
//!
//! Zonas:
//!   - rest_listar_zonas, rest_crear_zona, rest_actualizar_zona, rest_eliminar_zona
//!
//! Mesas (config):
//!   - rest_crear_mesa, rest_actualizar_mesa, rest_eliminar_mesa
//!
//! Mesas (operación):
//!   - rest_listar_mesas_con_estado  (grid principal)
//!
//! Pedidos:
//!   - rest_abrir_pedido (mesa_id, mesero_id, comensales)
//!   - rest_obtener_pedido (id)
//!   - rest_obtener_pedido_mesa (mesa_id) — el activo
//!   - rest_listar_pedidos_abiertos
//!   - rest_cancelar_pedido (id)
//!
//! Items:
//!   - rest_agregar_item, rest_actualizar_item_cantidad, rest_eliminar_item
//!
//! Cocina:
//!   - rest_enviar_cocina (pedido_id) — marca items pendientes y retorna lista para imprimir
//!   - rest_listar_items_cocina_pendientes — vista cocina/TV
//!   - rest_marcar_item_listo, rest_marcar_item_entregado
//!
//! Cuenta y cobro:
//!   - rest_pedir_cuenta (pedido_id)
//!   - rest_cobrar_pedido (pedido_id, forma_pago) — vincula con venta y libera mesa

use super::models::*;
use super::requiere_modulo_restaurante;
use crate::db::Database;
use rusqlite::{params, Connection};
use tauri::State;

// ─── Zonas ───────────────────────────────────────────────────────────────

#[tauri::command]
pub fn rest_listar_zonas(db: State<'_, Database>) -> Result<Vec<Zona>, String> {
    requiere_modulo_restaurante(&db)?;
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let mut stmt = conn
        .prepare("SELECT id, nombre, color, orden, activa FROM rest_zonas WHERE activa = 1 ORDER BY orden, nombre")
        .map_err(|e| e.to_string())?;
    let zonas: Vec<Zona> = stmt
        .query_map([], |row| {
            Ok(Zona {
                id: Some(row.get(0)?),
                nombre: row.get(1)?,
                color: row.get(2)?,
                orden: row.get(3)?,
                activa: row.get::<_, i32>(4)? != 0,
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;
    Ok(zonas)
}

#[tauri::command]
pub fn rest_crear_zona(db: State<'_, Database>, zona: Zona) -> Result<i64, String> {
    requiere_modulo_restaurante(&db)?;
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    conn.execute(
        "INSERT INTO rest_zonas (nombre, color, orden, activa) VALUES (?1, ?2, ?3, ?4)",
        params![zona.nombre, zona.color, zona.orden, if zona.activa { 1 } else { 0 }],
    )
    .map_err(|e| e.to_string())?;
    Ok(conn.last_insert_rowid())
}

#[tauri::command]
pub fn rest_actualizar_zona(db: State<'_, Database>, zona: Zona) -> Result<(), String> {
    requiere_modulo_restaurante(&db)?;
    let id = zona.id.ok_or("Zona sin id")?;
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    conn.execute(
        "UPDATE rest_zonas SET nombre = ?1, color = ?2, orden = ?3, activa = ?4 WHERE id = ?5",
        params![zona.nombre, zona.color, zona.orden, if zona.activa { 1 } else { 0 }, id],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn rest_eliminar_zona(db: State<'_, Database>, id: i64) -> Result<(), String> {
    requiere_modulo_restaurante(&db)?;
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    // Soft delete — preserva integridad con mesas existentes
    conn.execute("UPDATE rest_zonas SET activa = 0 WHERE id = ?1", params![id])
        .map_err(|e| e.to_string())?;
    Ok(())
}

// ─── Mesas (configuración) ───────────────────────────────────────────────

#[tauri::command]
pub fn rest_crear_mesa(db: State<'_, Database>, mesa: Mesa) -> Result<i64, String> {
    requiere_modulo_restaurante(&db)?;
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    conn.execute(
        "INSERT INTO rest_mesas (zona_id, nombre, capacidad, orden, activa) VALUES (?1, ?2, ?3, ?4, ?5)",
        params![mesa.zona_id, mesa.nombre, mesa.capacidad, mesa.orden, if mesa.activa { 1 } else { 0 }],
    )
    .map_err(|e| e.to_string())?;
    Ok(conn.last_insert_rowid())
}

#[tauri::command]
pub fn rest_actualizar_mesa(db: State<'_, Database>, mesa: Mesa) -> Result<(), String> {
    requiere_modulo_restaurante(&db)?;
    let id = mesa.id.ok_or("Mesa sin id")?;
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    conn.execute(
        "UPDATE rest_mesas SET zona_id = ?1, nombre = ?2, capacidad = ?3, orden = ?4, activa = ?5 WHERE id = ?6",
        params![mesa.zona_id, mesa.nombre, mesa.capacidad, mesa.orden, if mesa.activa { 1 } else { 0 }, id],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn rest_eliminar_mesa(db: State<'_, Database>, id: i64) -> Result<(), String> {
    requiere_modulo_restaurante(&db)?;
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    // Validar que no haya pedido abierto
    let abierto: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM rest_pedidos_abiertos WHERE mesa_id = ?1 AND estado IN ('ABIERTO', 'CUENTA_PEDIDA')",
            params![id],
            |row| row.get(0),
        )
        .unwrap_or(0);
    if abierto > 0 {
        return Err("No se puede eliminar una mesa con pedido abierto".to_string());
    }
    conn.execute("UPDATE rest_mesas SET activa = 0 WHERE id = ?1", params![id])
        .map_err(|e| e.to_string())?;
    Ok(())
}

// ─── Mesas (operación: grid principal) ───────────────────────────────────

#[tauri::command]
pub fn rest_listar_mesas_con_estado(
    db: State<'_, Database>,
) -> Result<Vec<MesaConEstado>, String> {
    requiere_modulo_restaurante(&db)?;
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    listar_mesas_con_estado_internal(&conn)
}

pub(crate) fn listar_mesas_con_estado_internal(conn: &Connection) -> Result<Vec<MesaConEstado>, String> {
    // v2.3.63 FIX: usar subquery con MAX(id) para garantizar UN SOLO pedido
    // por mesa. Si por algun bug/race condition hay multiples pedidos abiertos
    // para la misma mesa, agarramos el MÁS RECIENTE (que probablemente es el
    // que tiene los items reales). Esto previene el bug "items desaparecen"
    // que pasa cuando el LEFT JOIN devuelve filas duplicadas y aleatorio cual
    // gana en SQLite.
    //
    // Ademas: AUTO-LIMPIEZA al pasada — cierra pedidos abiertos VACIOS de mas
    // de 24h (sin items, claramente abandonados). Idempotente y safe.
    // FIX v2.4.1: usar 'localtime' en julianday('now') para que coincida con
    // fecha_apertura que se guarda con datetime('now','localtime'). Sin esto
    // hay un desfase de la zona horaria (en Ecuador UTC-5 son 5h menos), lo
    // que vuelve el threshold de 24h efectivamente más permisivo.
    let _ = conn.execute(
        "UPDATE rest_pedidos_abiertos
         SET estado = 'CANCELADO', fecha_cierre = datetime('now', 'localtime')
         WHERE estado IN ('ABIERTO', 'CUENTA_PEDIDA')
           AND julianday('now', 'localtime') - julianday(fecha_apertura) > 1.0
           AND id NOT IN (SELECT DISTINCT pedido_id FROM rest_pedido_items)",
        [],
    );

    // v2.3.68: COALESCE entre pedido propio y pedido al que esta mesa fue unida como EXTRA.
    // Una mesa "extra" muestra el estado del pedido principal y debe ser clickeable
    // (abre el mismo pedido). El campo mesa_principal_id != NULL indica que esta mesa
    // es secundaria del grupo.
    let mut stmt = conn
        .prepare(
            "SELECT
                m.id, m.zona_id, z.nombre, z.color, m.nombre, m.capacidad, m.orden,
                COALESCE(p_propio.id, p_extra.id)              AS pedido_id,
                COALESCE(p_propio.mesero_nombre, p_extra.mesero_nombre) AS mesero,
                COALESCE(p_propio.comensales, p_extra.comensales)       AS comensales,
                COALESCE(p_propio.estado, p_extra.estado)               AS estado,
                COALESCE(p_propio.fecha_apertura, p_extra.fecha_apertura) AS fecha_apertura,
                p_extra.id                                              AS pedido_extra_id,
                p_extra.mesa_id                                         AS mesa_principal_id,
                m_principal.nombre                                      AS mesa_principal_nombre
             FROM rest_mesas m
             LEFT JOIN rest_zonas z ON m.zona_id = z.id
             LEFT JOIN rest_pedidos_abiertos p_propio
                ON p_propio.id = (
                    SELECT MAX(p2.id) FROM rest_pedidos_abiertos p2
                    WHERE p2.mesa_id = m.id
                      AND p2.estado IN ('ABIERTO', 'CUENTA_PEDIDA')
                )
             LEFT JOIN rest_pedido_mesas_extra pe ON pe.mesa_id = m.id
             LEFT JOIN rest_pedidos_abiertos p_extra
                ON p_extra.id = pe.pedido_id
                AND p_extra.estado IN ('ABIERTO', 'CUENTA_PEDIDA')
             LEFT JOIN rest_mesas m_principal ON m_principal.id = p_extra.mesa_id
             WHERE m.activa = 1
             ORDER BY z.orden NULLS LAST, m.orden, m.nombre",
        )
        .map_err(|e| e.to_string())?;

    let mut mesas: Vec<MesaConEstado> = stmt
        .query_map([], |row| {
            let pedido_id: Option<i64> = row.get(7)?;
            let estado_pedido: Option<String> = row.get(10)?;
            let estado = match estado_pedido.as_deref() {
                Some("CUENTA_PEDIDA") => "CUENTA_PEDIDA".to_string(),
                Some("ABIERTO") => "OCUPADA".to_string(),
                _ => "LIBRE".to_string(),
            };
            // pedido_extra_id != NULL → esta mesa es EXTRA del grupo
            let es_extra: bool = row.get::<_, Option<i64>>(12)?.is_some();
            Ok(MesaConEstado {
                id: row.get(0)?,
                zona_id: row.get(1)?,
                zona_nombre: row.get(2)?,
                zona_color: row.get(3)?,
                nombre: row.get(4)?,
                capacidad: row.get(5)?,
                orden: row.get(6)?,
                estado,
                pedido_id,
                mesero_nombre: row.get(8)?,
                comensales: row.get(9)?,
                total_actual: 0.0,
                items_pendientes_cocina: 0,
                fecha_apertura: row.get(11)?,
                minutos_abierta: None,
                mesa_principal_id: if es_extra { row.get(13)? } else { None },
                mesa_principal_nombre: if es_extra { row.get(14)? } else { None },
                mesas_unidas_count: 0,
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    // Para cada mesa con pedido, calcular total y items pendientes en cocina.
    // v2.3.68: Solo calculamos esto para la mesa PRINCIPAL del pedido. Las mesas
    // EXTRA muestran el indicador de "unida a X" pero NO el total ni los items
    // (eso ya lo refleja la principal — duplicar en extras inflaría el "Total abierto").
    for m in &mut mesas {
        if let Some(pid) = m.pedido_id {
            // Calcular minutos desde apertura — siempre (también en mesas extra,
            // para que muestren cuánto tiempo llevan ocupadas)
            if let Some(fecha) = &m.fecha_apertura {
                if let Ok(mins) = conn.query_row(
                    "SELECT CAST((julianday('now', 'localtime') - julianday(?1)) * 24 * 60 AS INTEGER)",
                    params![fecha],
                    |row| row.get::<_, i64>(0),
                ) {
                    m.minutos_abierta = Some(mins);
                }
            }

            // Solo la principal acumula total/items pendientes/count de mesas unidas
            if m.mesa_principal_id.is_none() {
                let total: f64 = conn
                    .query_row(
                        "SELECT COALESCE(SUM(cantidad * precio_unit), 0) FROM rest_pedido_items WHERE pedido_id = ?1",
                        params![pid],
                        |row| row.get(0),
                    )
                    .unwrap_or(0.0);
                m.total_actual = total;

                let pendientes: i32 = conn
                    .query_row(
                        "SELECT COUNT(*) FROM rest_pedido_items WHERE pedido_id = ?1 AND estado_cocina IN ('PENDIENTE', 'EN_PREPARACION')",
                        params![pid],
                        |row| row.get(0),
                    )
                    .unwrap_or(0);
                m.items_pendientes_cocina = pendientes;

                let count: i32 = conn
                    .query_row(
                        "SELECT COUNT(*) FROM rest_pedido_mesas_extra WHERE pedido_id = ?1",
                        params![pid],
                        |row| row.get(0),
                    )
                    .unwrap_or(0);
                m.mesas_unidas_count = count;
            }
        }
    }

    Ok(mesas)
}

// ─── Pedidos ─────────────────────────────────────────────────────────────

#[tauri::command]
pub fn rest_abrir_pedido(
    db: State<'_, Database>,
    mesa_id: i64,
    mesero_id: Option<i64>,
    mesero_nombre: Option<String>,
    comensales: Option<i32>,
) -> Result<i64, String> {
    requiere_modulo_restaurante(&db)?;
    let conn = db.conn.lock().map_err(|e| e.to_string())?;

    // Validar que la mesa exista y esté activa
    let mesa_existe: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM rest_mesas WHERE id = ?1 AND activa = 1",
            params![mesa_id],
            |row| row.get(0),
        )
        .unwrap_or(0);
    if mesa_existe == 0 {
        return Err("Mesa no encontrada o inactiva".to_string());
    }

    // Validar que no haya ya un pedido abierto para esta mesa
    let abierto_existe: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM rest_pedidos_abiertos WHERE mesa_id = ?1 AND estado IN ('ABIERTO', 'CUENTA_PEDIDA')",
            params![mesa_id],
            |row| row.get(0),
        )
        .unwrap_or(0);
    if abierto_existe > 0 {
        return Err("Ya existe un pedido abierto en esta mesa".to_string());
    }

    conn.execute(
        "INSERT INTO rest_pedidos_abiertos (mesa_id, mesero_id, mesero_nombre, comensales, estado)
         VALUES (?1, ?2, ?3, ?4, 'ABIERTO')",
        params![mesa_id, mesero_id, mesero_nombre, comensales.unwrap_or(1)],
    )
    .map_err(|e| e.to_string())?;
    Ok(conn.last_insert_rowid())
}

#[tauri::command]
pub fn rest_obtener_pedido(db: State<'_, Database>, id: i64) -> Result<PedidoDetalle, String> {
    requiere_modulo_restaurante(&db)?;
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    obtener_pedido_detalle(&conn, id)
}

#[tauri::command]
pub fn rest_obtener_pedido_mesa(
    db: State<'_, Database>,
    mesa_id: i64,
) -> Result<Option<PedidoDetalle>, String> {
    requiere_modulo_restaurante(&db)?;
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let pedido_id: Option<i64> = conn
        .query_row(
            "SELECT id FROM rest_pedidos_abiertos WHERE mesa_id = ?1 AND estado IN ('ABIERTO', 'CUENTA_PEDIDA') ORDER BY id DESC LIMIT 1",
            params![mesa_id],
            |row| row.get(0),
        )
        .ok();
    match pedido_id {
        Some(pid) => Ok(Some(obtener_pedido_detalle(&conn, pid)?)),
        None => Ok(None),
    }
}

#[tauri::command]
pub fn rest_listar_pedidos_abiertos(
    db: State<'_, Database>,
) -> Result<Vec<PedidoAbierto>, String> {
    requiere_modulo_restaurante(&db)?;
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let mut stmt = conn
        .prepare(
            "SELECT id, mesa_id, mesero_id, mesero_nombre, comensales, estado, observacion,
                    fecha_apertura, fecha_cuenta, fecha_cierre, venta_id
             FROM rest_pedidos_abiertos
             WHERE estado IN ('ABIERTO', 'CUENTA_PEDIDA')
             ORDER BY fecha_apertura DESC",
        )
        .map_err(|e| e.to_string())?;
    let pedidos: Vec<PedidoAbierto> = stmt
        .query_map([], |row| {
            Ok(PedidoAbierto {
                id: Some(row.get(0)?),
                mesa_id: row.get(1)?,
                mesero_id: row.get(2)?,
                mesero_nombre: row.get(3)?,
                comensales: row.get(4)?,
                estado: row.get(5)?,
                observacion: row.get(6)?,
                fecha_apertura: row.get(7)?,
                fecha_cuenta: row.get(8)?,
                fecha_cierre: row.get(9)?,
                venta_id: row.get(10)?,
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;
    Ok(pedidos)
}

#[tauri::command]
pub fn rest_cancelar_pedido(db: State<'_, Database>, id: i64) -> Result<(), String> {
    requiere_modulo_restaurante(&db)?;
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    conn.execute(
        "UPDATE rest_pedidos_abiertos SET estado = 'CANCELADO', fecha_cierre = datetime('now', 'localtime') WHERE id = ?1",
        params![id],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

// ─── Items ───────────────────────────────────────────────────────────────

#[tauri::command]
pub fn rest_agregar_item(
    db: State<'_, Database>,
    pedido_id: i64,
    producto_id: i64,
    cantidad: f64,
    info_adicional: Option<String>,
) -> Result<i64, String> {
    requiere_modulo_restaurante(&db)?;
    let conn = db.conn.lock().map_err(|e| e.to_string())?;

    // Validar pedido abierto
    let estado: String = conn
        .query_row(
            "SELECT estado FROM rest_pedidos_abiertos WHERE id = ?1",
            params![pedido_id],
            |row| row.get(0),
        )
        .map_err(|_| "Pedido no encontrado".to_string())?;
    if estado != "ABIERTO" && estado != "CUENTA_PEDIDA" {
        return Err(format!("No se pueden agregar items a un pedido {}", estado));
    }

    // Obtener precio + destino del producto
    let (precio, destino): (f64, String) = conn
        .query_row(
            "SELECT precio_venta, COALESCE(destino_preparacion, 'COCINA')
             FROM productos WHERE id = ?1 AND activo = 1",
            params![producto_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .map_err(|_| "Producto no encontrado o inactivo".to_string())?;

    // Si el destino es DIRECTO (bebidas embotelladas, snacks, etc.):
    // marcar el item como YA enviado a cocina y YA entregado, para que NO
    // aparezca en /cocina. El mesero lo despacha del mostrador.
    let es_directo = destino == "DIRECTO";
    if es_directo {
        conn.execute(
            "INSERT INTO rest_pedido_items
             (pedido_id, producto_id, cantidad, precio_unit, info_adicional,
              enviado_cocina, estado_cocina, fecha_envio_cocina)
             VALUES (?1, ?2, ?3, ?4, ?5, 1, 'ENTREGADO', datetime('now', 'localtime'))",
            params![pedido_id, producto_id, cantidad, precio, info_adicional],
        )
        .map_err(|e| e.to_string())?;
    } else {
        // COCINA o BARRA: flujo normal — mesero lo enviará a cocina cuando esté listo
        conn.execute(
            "INSERT INTO rest_pedido_items (pedido_id, producto_id, cantidad, precio_unit, info_adicional)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![pedido_id, producto_id, cantidad, precio, info_adicional],
        )
        .map_err(|e| e.to_string())?;
    }
    Ok(conn.last_insert_rowid())
}

#[tauri::command]
pub fn rest_actualizar_item_cantidad(
    db: State<'_, Database>,
    item_id: i64,
    cantidad: f64,
) -> Result<(), String> {
    requiere_modulo_restaurante(&db)?;
    if cantidad <= 0.0 {
        return Err("La cantidad debe ser mayor a 0".to_string());
    }
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    conn.execute(
        "UPDATE rest_pedido_items SET cantidad = ?1 WHERE id = ?2",
        params![cantidad, item_id],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn rest_eliminar_item(db: State<'_, Database>, item_id: i64) -> Result<(), String> {
    requiere_modulo_restaurante(&db)?;
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    // Permitir eliminar:
    //   1. Items NO enviados a cocina (mesero se equivocó al agregar) — OK
    //   2. Items DIRECTO (bebidas/snacks que el mesero toma del mostrador):
    //      aunque están marcados con enviado_cocina=1 al insertarse, NUNCA
    //      pasaron por cocina, así que el mesero puede deshacerlos sin problema.
    // BLOQUEAR:
    //   3. Items COCINA/BARRA ya enviados a cocina (porque el cocinero ya los está preparando o ya están listos).
    let (enviado, destino): (i32, String) = conn
        .query_row(
            "SELECT i.enviado_cocina, COALESCE(p.destino_preparacion, 'COCINA')
             FROM rest_pedido_items i
             JOIN productos p ON i.producto_id = p.id
             WHERE i.id = ?1",
            params![item_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .map_err(|_| "Item no encontrado".to_string())?;
    if enviado != 0 && destino != "DIRECTO" {
        return Err("No se puede eliminar un item ya enviado a cocina. Use anulación.".to_string());
    }
    conn.execute("DELETE FROM rest_pedido_items WHERE id = ?1", params![item_id])
        .map_err(|e| e.to_string())?;
    Ok(())
}

// ─── Cocina ──────────────────────────────────────────────────────────────

/// Marca todos los items pendientes (no enviados) del pedido como enviados a cocina.
/// Retorna la lista de items recién enviados para que el frontend los imprima
/// en el ticket de cocina.
#[tauri::command]
pub fn rest_enviar_cocina(
    db: State<'_, Database>,
    pedido_id: i64,
) -> Result<Vec<PedidoItem>, String> {
    requiere_modulo_restaurante(&db)?;
    let conn = db.conn.lock().map_err(|e| e.to_string())?;

    // Obtener items pendientes ANTES de marcarlos.
    // Nota: items con destino DIRECTO se insertaron ya con enviado_cocina=1, asi que
    // el filtro `enviado_cocina = 0` los excluye automáticamente. Solo COCINA + BARRA aqui.
    let mut stmt = conn
        .prepare(
            "SELECT i.id, i.pedido_id, i.producto_id, p.nombre, i.cantidad, i.precio_unit,
                    i.info_adicional, i.enviado_cocina, i.estado_cocina,
                    i.fecha_creacion, i.fecha_envio_cocina,
                    COALESCE(p.destino_preparacion, 'COCINA') as destino
             FROM rest_pedido_items i
             JOIN productos p ON i.producto_id = p.id
             WHERE i.pedido_id = ?1 AND i.enviado_cocina = 0
             ORDER BY i.id",
        )
        .map_err(|e| e.to_string())?;
    let items: Vec<PedidoItem> = stmt
        .query_map(params![pedido_id], |row| {
            Ok(PedidoItem {
                id: Some(row.get(0)?),
                pedido_id: row.get(1)?,
                producto_id: row.get(2)?,
                producto_nombre: Some(row.get(3)?),
                cantidad: row.get(4)?,
                precio_unit: row.get(5)?,
                info_adicional: row.get(6)?,
                enviado_cocina: row.get::<_, i32>(7)? != 0,
                estado_cocina: row.get(8)?,
                fecha_creacion: row.get(9)?,
                fecha_envio_cocina: row.get(10)?,
                destino_preparacion: row.get(11)?,
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    if items.is_empty() {
        return Err("No hay items nuevos para enviar a cocina".to_string());
    }

    // Marcar como enviados
    conn.execute(
        "UPDATE rest_pedido_items
         SET enviado_cocina = 1, fecha_envio_cocina = datetime('now', 'localtime')
         WHERE pedido_id = ?1 AND enviado_cocina = 0",
        params![pedido_id],
    )
    .map_err(|e| e.to_string())?;

    Ok(items)
}

#[tauri::command]
pub fn rest_listar_items_cocina_pendientes(
    db: State<'_, Database>,
) -> Result<Vec<ItemCocina>, String> {
    requiere_modulo_restaurante(&db)?;
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let mut stmt = conn
        .prepare(
            "SELECT i.id, i.pedido_id, m.nombre, z.nombre, p.mesero_nombre,
                    pr.nombre, i.cantidad, i.info_adicional, i.estado_cocina, i.fecha_envio_cocina,
                    CAST((julianday('now', 'localtime') - julianday(i.fecha_envio_cocina)) * 24 * 60 AS INTEGER) AS mins
             FROM rest_pedido_items i
             JOIN rest_pedidos_abiertos p ON i.pedido_id = p.id
             JOIN rest_mesas m ON p.mesa_id = m.id
             LEFT JOIN rest_zonas z ON m.zona_id = z.id
             JOIN productos pr ON i.producto_id = pr.id
             WHERE i.enviado_cocina = 1
               AND i.estado_cocina IN ('PENDIENTE', 'EN_PREPARACION', 'LISTO')
             ORDER BY i.fecha_envio_cocina ASC",
        )
        .map_err(|e| e.to_string())?;
    let items: Vec<ItemCocina> = stmt
        .query_map([], |row| {
            Ok(ItemCocina {
                id: row.get(0)?,
                pedido_id: row.get(1)?,
                mesa_nombre: row.get(2)?,
                zona_nombre: row.get(3)?,
                mesero_nombre: row.get(4)?,
                producto_nombre: row.get(5)?,
                cantidad: row.get(6)?,
                info_adicional: row.get(7)?,
                estado_cocina: row.get(8)?,
                fecha_envio_cocina: row.get(9)?,
                minutos_en_cocina: row.get(10).ok(),
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;
    Ok(items)
}

#[tauri::command]
pub fn rest_marcar_item_cocina(
    db: State<'_, Database>,
    item_id: i64,
    estado: String,
) -> Result<(), String> {
    requiere_modulo_restaurante(&db)?;
    let estado_valido = matches!(
        estado.as_str(),
        "PENDIENTE" | "EN_PREPARACION" | "LISTO" | "ENTREGADO"
    );
    if !estado_valido {
        return Err(format!("Estado de cocina inválido: {}", estado));
    }
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    conn.execute(
        "UPDATE rest_pedido_items SET estado_cocina = ?1 WHERE id = ?2",
        params![estado, item_id],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

// ─── Cuenta y cobro ──────────────────────────────────────────────────────

#[tauri::command]
pub fn rest_pedir_cuenta(db: State<'_, Database>, pedido_id: i64) -> Result<(), String> {
    requiere_modulo_restaurante(&db)?;
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    conn.execute(
        "UPDATE rest_pedidos_abiertos
         SET estado = 'CUENTA_PEDIDA', fecha_cuenta = datetime('now', 'localtime')
         WHERE id = ?1 AND estado = 'ABIERTO'",
        params![pedido_id],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

/// Cierra el pedido vinculándolo con una venta ya generada (desde el POS).
/// El POS es responsable de generar la venta real (con SRI, secuencial, etc.)
/// y luego llama a este comando con el venta_id resultante.
///
/// En Fase 2 del módulo agregaremos un comando combo `rest_cobrar_pedido_completo`
/// que delega al comando de ventas existente.
#[tauri::command]
pub fn rest_cerrar_pedido(
    db: State<'_, Database>,
    pedido_id: i64,
    venta_id: i64,
) -> Result<(), String> {
    requiere_modulo_restaurante(&db)?;
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    conn.execute(
        "UPDATE rest_pedidos_abiertos
         SET estado = 'COBRADO', venta_id = ?1, fecha_cierre = datetime('now', 'localtime')
         WHERE id = ?2",
        params![venta_id, pedido_id],
    )
    .map_err(|e| e.to_string())?;
    // v2.5.91 — los abonos en HOLDING pasan a APLICADO (ya forman parte de la venta).
    conn.execute(
        "UPDATE rest_pedido_abonos
         SET estado = 'APLICADO', venta_id_aplicado = ?1, fecha_aplicado = datetime('now', 'localtime')
         WHERE pedido_id = ?2 AND estado = 'HOLDING'",
        params![venta_id, pedido_id],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

/// v2.5.91 — Registra un abono (pago parcial) sobre una mesa desde el escritorio.
/// Devuelve el detalle actualizado del pedido (con saldo/abonado).
#[tauri::command]
pub fn rest_registrar_abono(
    db: State<'_, Database>,
    sesion: State<'_, crate::db::SesionState>,
    pedido_id: i64,
    monto: f64,
    forma_pago: String,
    banco_id: Option<i64>,
    referencia_pago: Option<String>,
) -> Result<crate::restaurante::models::PedidoDetalle, String> {
    requiere_modulo_restaurante(&db)?;
    let (usuario_id, usuario_nombre) = {
        let g = sesion.sesion.lock().map_err(|e| e.to_string())?;
        match g.as_ref() {
            Some(s) => (Some(s.usuario_id), Some(s.nombre.clone())),
            None => (None, None),
        }
    };
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    registrar_abono_pedido(
        &conn, pedido_id, monto, &forma_pago, banco_id,
        referencia_pago.as_deref(), usuario_id, usuario_nombre.as_deref(),
    )?;
    obtener_pedido_detalle(&conn, pedido_id)
}

// ─── Impresión ──────────────────────────────────────────────────────────

/// Detecta si el nombre de impresora corresponde a una "impresora virtual"
/// (Microsoft Print to PDF, OneNote, XPS, Fax). Esas no entienden ESC/POS y
/// generan basura si les enviamos los bytes crudos — para esos casos hay que
/// generar PDF nativo y abrirlo con el visor del sistema.
fn impresora_es_virtual(nombre: &str) -> bool {
    let lower = nombre.to_lowercase();
    lower.is_empty()
        || lower.contains("pdf")
        || lower.contains("onenote")
        || lower.contains("xps")
        || lower.contains("fax")
        || lower.contains("microsoft print")
}

/// Imprime/genera el ticket de pre-cuenta.
///
/// **Auto-detecta** el tipo de impresora configurada:
/// - **Impresora térmica real** (POS-58, Epson TM, etc.) → envía bytes ESC/POS directos.
/// - **Impresora virtual** (Microsoft Print to PDF, OneNote, XPS, Fax) → genera
///   un PDF nativo legible y lo abre con el visor del sistema. Antes este caso
///   producía un PDF con bytes ESC/POS basura ilegible.
/// - **Sin impresora configurada** → genera PDF nativo y lo abre.
///
/// La pre-cuenta NO es comprobante fiscal — solo informa al cliente. La
/// factura/nota de venta real se genera al cobrar vía `registrar_venta`.
/// Se puede llamar múltiples veces (botón "Reimprimir") sin efecto secundario.
///
/// Retorna mensaje descriptivo del método usado para que el frontend muestre
/// el toast apropiado ("impresa" vs "PDF abierto").
#[tauri::command]
pub fn rest_imprimir_pre_cuenta(
    db: State<'_, Database>,
    pedido_id: i64,
) -> Result<String, String> {
    requiere_modulo_restaurante(&db)?;
    let conn = db.conn.lock().map_err(|e| e.to_string())?;

    let detalle = obtener_pedido_detalle(&conn, pedido_id)?;

    if detalle.items.is_empty() {
        return Err("El pedido no tiene items — no hay nada que imprimir".to_string());
    }

    // Cargar config (logo, nombre negocio, dirección, etc.)
    let mut cfg_stmt = conn.prepare("SELECT key, value FROM config").map_err(|e| e.to_string())?;
    let config: std::collections::HashMap<String, String> = cfg_stmt
        .query_map([], |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)))
        .map_err(|e| e.to_string())?
        .collect::<Result<std::collections::HashMap<_, _>, _>>()
        .map_err(|e| e.to_string())?;

    drop(cfg_stmt);
    drop(conn);

    let impresora = config
        .get("impresora")
        .map(|s| s.to_string())
        .unwrap_or_default();

    // Caso 1: impresora virtual o vacía → PDF nativo
    if impresora_es_virtual(&impresora) {
        let pdf_bytes = super::printing::generar_pre_cuenta_pdf(&detalle, &config)?;

        // Guardar en directorio temporal
        let temp_dir = std::env::temp_dir();
        let filename = format!(
            "PreCuenta-Mesa{}-Ped{}.pdf",
            detalle.mesa_nombre.replace(['/', '\\', ':', ' '], "_"),
            detalle.pedido.id.unwrap_or(0)
        );
        let pdf_path = temp_dir.join(&filename);
        std::fs::write(&pdf_path, &pdf_bytes)
            .map_err(|e| format!("Error guardando pre-cuenta PDF: {}", e))?;

        // Abrir con visor del sistema
        #[cfg(target_os = "windows")]
        {
            crate::utils::silent_command("cmd")
                .args(["/C", "start", "", &pdf_path.to_string_lossy()])
                .spawn()
                .map_err(|e| format!("Error abriendo PDF: {}", e))?;
        }
        #[cfg(not(target_os = "windows"))]
        {
            std::process::Command::new("xdg-open")
                .arg(&pdf_path.to_string_lossy().to_string())
                .spawn()
                .map_err(|e| format!("Error abriendo PDF: {}", e))?;
        }

        return Ok(format!("PDF generado y abierto: {}", filename));
    }

    // Caso 2: impresora térmica real → ESC/POS
    let ticket = super::printing::generar_pre_cuenta(&detalle, &config);
    crate::printing::imprimir_raw_windows(&impresora, &ticket)?;

    Ok("Pre-cuenta impresa".to_string())
}

/// v2.3.67: Imprime la comanda de cocina (lista de items a preparar).
/// Se llama automáticamente desde el frontend después de "Enviar cocina"
/// y opcionalmente puede usar una impresora SEPARADA (config `impresora_cocina`).
///
/// Modo de impresión configurable (`comanda_modo_separado`):
/// - "0" (default): UN ticket combinado COCINA + BARRA (con [BARRA] tag en items)
/// - "1": DOS tickets separados (cocina y barra) — útil si cocina y barra
///   son áreas físicas distintas con impresoras propias.
///
/// Si `items_ids` viene con valores, solo imprime esos items (recién enviados).
/// Si es None/vacío, imprime TODOS los items pendientes del pedido (re-imprimir).
///
/// Items DIRECTO (despacho directo, ej. bebidas embotelladas) NUNCA se imprimen.
#[tauri::command]
pub fn rest_imprimir_comanda_cocina(
    db: State<'_, Database>,
    pedido_id: i64,
    items_ids: Option<Vec<i64>>,
) -> Result<String, String> {
    requiere_modulo_restaurante(&db)?;
    let conn = db.conn.lock().map_err(|e| e.to_string())?;

    let detalle = obtener_pedido_detalle(&conn, pedido_id)?;

    // Filtrar items según items_ids (si vino) o usar todos los items COCINA/BARRA
    let items_objetivo: Vec<super::models::PedidoItem> = if let Some(ids) = items_ids.as_ref() {
        if ids.is_empty() {
            return Err("No se especificaron items para imprimir".to_string());
        }
        detalle.items.iter()
            .filter(|i| i.id.map(|id| ids.contains(&id)).unwrap_or(false))
            .cloned()
            .collect()
    } else {
        // Sin filtro = todos los items del pedido (re-imprimir)
        detalle.items.clone()
    };

    if items_objetivo.is_empty() {
        return Err("No hay items para imprimir comanda".to_string());
    }

    // Cargar config
    let mut cfg_stmt = conn.prepare("SELECT key, value FROM config").map_err(|e| e.to_string())?;
    let config: std::collections::HashMap<String, String> = cfg_stmt
        .query_map([], |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)))
        .map_err(|e| e.to_string())?
        .collect::<Result<std::collections::HashMap<_, _>, _>>()
        .map_err(|e| e.to_string())?;

    drop(cfg_stmt);
    drop(conn);

    // Resolver impresora: usar `impresora_cocina` si está configurada, sino `impresora` principal.
    let impresora_cocina = config.get("impresora_cocina")
        .map(|s| s.trim().to_string()).filter(|s| !s.is_empty());
    let impresora_principal = config.get("impresora")
        .map(|s| s.trim().to_string()).filter(|s| !s.is_empty());
    let impresora_cocina_efectiva = impresora_cocina.clone().or_else(|| impresora_principal.clone())
        .unwrap_or_default(); // vacío = caerá a PDF

    // Impresora separada para barra (opcional, si config `comanda_modo_separado=1`)
    let modo_separado = config.get("comanda_modo_separado").map(|s| s.as_str()) == Some("1");
    let impresora_barra = config.get("impresora_barra")
        .map(|s| s.trim().to_string()).filter(|s| !s.is_empty())
        .unwrap_or_else(|| impresora_cocina_efectiva.clone()); // fallback a cocina si no hay separada

    let mut mensajes: Vec<String> = Vec::new();

    // v2.4.5 — Helper: decide ESC/POS vs PDF según si la impresora es térmica
    // o virtual/vacía. Sigue el mismo patrón que `rest_imprimir_pre_cuenta`:
    //   - Térmica real → bytes ESC/POS directos
    //   - Virtual (Microsoft Print to PDF, OneNote, XPS, Fax) o vacía → PDF
    //     nativo legible y se abre con visor del sistema
    let imprimir_o_pdf = |impresora: &str, destino: super::printing::DestinoComanda, etiqueta: &str| -> Result<Option<String>, String> {
        if impresora_es_virtual(impresora) {
            // Generar PDF
            match super::printing::generar_comanda_cocina_pdf(
                pedido_id, &detalle.mesa_nombre, detalle.zona_nombre.as_deref(),
                detalle.pedido.mesero_nombre.as_deref(), &items_objetivo, destino, &config,
            )? {
                None => Ok(None), // sin items para este destino → silencio
                Some(pdf_bytes) => {
                    let temp_dir = std::env::temp_dir();
                    let filename = format!(
                        "Comanda-{}-Mesa{}-Ped{}.pdf",
                        etiqueta,
                        detalle.mesa_nombre.replace(['/', '\\', ':', ' '], "_"),
                        pedido_id
                    );
                    let pdf_path = temp_dir.join(&filename);
                    std::fs::write(&pdf_path, &pdf_bytes)
                        .map_err(|e| format!("Error guardando comanda PDF: {}", e))?;
                    #[cfg(target_os = "windows")]
                    {
                        crate::utils::silent_command("cmd")
                            .args(["/C", "start", "", &pdf_path.to_string_lossy()])
                            .spawn()
                            .map_err(|e| format!("Error abriendo PDF: {}", e))?;
                    }
                    #[cfg(not(target_os = "windows"))]
                    {
                        std::process::Command::new("xdg-open")
                            .arg(&pdf_path.to_string_lossy().to_string())
                            .spawn()
                            .map_err(|e| format!("Error abriendo PDF: {}", e))?;
                    }
                    Ok(Some(format!("📄 {} PDF abierto", etiqueta)))
                }
            }
        } else {
            // Impresora térmica real → ESC/POS
            match super::printing::generar_comanda_cocina(
                pedido_id, &detalle.mesa_nombre, detalle.zona_nombre.as_deref(),
                detalle.pedido.mesero_nombre.as_deref(), &items_objetivo, destino, &config,
            ) {
                None => Ok(None),
                Some(t) => {
                    crate::printing::imprimir_raw_windows(impresora, &t)?;
                    Ok(Some(format!("{} impresa", etiqueta)))
                }
            }
        }
    };

    if modo_separado {
        if let Some(m) = imprimir_o_pdf(&impresora_cocina_efectiva, super::printing::DestinoComanda::Cocina, "🍳 Cocina")? {
            mensajes.push(m);
        }
        if let Some(m) = imprimir_o_pdf(&impresora_barra, super::printing::DestinoComanda::Barra, "🍷 Barra")? {
            mensajes.push(m);
        }
    } else {
        if let Some(m) = imprimir_o_pdf(&impresora_cocina_efectiva, super::printing::DestinoComanda::Ambos, "🍽 Comanda")? {
            mensajes.push(m);
        }
    }

    if mensajes.is_empty() {
        Ok("Sin items para cocina (todos despacho directo)".to_string())
    } else {
        Ok(mensajes.join(" + "))
    }
}

// ─── v2.3.68 — Unir mesas ────────────────────────────────────────────────

/// Une una o varias mesas LIBRES al pedido `pedido_id`. La mesa principal
/// (la del pedido) NO cambia — solo se agregan mesas extra al grupo.
///
/// Reglas:
/// - El pedido debe estar ABIERTO o CUENTA_PEDIDA
/// - Cada mesa target debe estar LIBRE: sin pedido propio activo y sin
///   estar ya unida a OTRO pedido
/// - No se puede unir la misma mesa principal del pedido
///
/// Si alguna mesa falla la validación, retorna error y NO une ninguna
/// (transacción atómica).
#[tauri::command]
pub fn rest_unir_mesas(
    db: State<'_, Database>,
    pedido_id: i64,
    mesas_ids: Vec<i64>,
) -> Result<(), String> {
    requiere_modulo_restaurante(&db)?;
    if mesas_ids.is_empty() {
        return Err("Debe seleccionar al menos una mesa".to_string());
    }
    let mut conn = db.conn.lock().map_err(|e| e.to_string())?;

    // Validar pedido
    let (mesa_principal, estado): (i64, String) = conn
        .query_row(
            "SELECT mesa_id, estado FROM rest_pedidos_abiertos WHERE id = ?1",
            params![pedido_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .map_err(|_| "Pedido no encontrado".to_string())?;
    if estado != "ABIERTO" && estado != "CUENTA_PEDIDA" {
        return Err(format!("No se pueden unir mesas a un pedido {}", estado));
    }

    // Transacción: validar TODAS las mesas y luego insertar TODAS
    let tx = conn.transaction().map_err(|e| e.to_string())?;
    for &mesa_id in &mesas_ids {
        if mesa_id == mesa_principal {
            return Err("No se puede unir la mesa principal a sí misma".to_string());
        }

        // Validar que la mesa exista y esté activa
        let activa: i64 = tx
            .query_row(
                "SELECT COUNT(*) FROM rest_mesas WHERE id = ?1 AND activa = 1",
                params![mesa_id],
                |row| row.get(0),
            )
            .unwrap_or(0);
        if activa == 0 {
            return Err(format!("Mesa {} no encontrada o inactiva", mesa_id));
        }

        // Validar que NO tenga pedido propio activo
        let pedido_propio: i64 = tx
            .query_row(
                "SELECT COUNT(*) FROM rest_pedidos_abiertos
                 WHERE mesa_id = ?1 AND estado IN ('ABIERTO', 'CUENTA_PEDIDA')",
                params![mesa_id],
                |row| row.get(0),
            )
            .unwrap_or(0);
        if pedido_propio > 0 {
            let nombre: String = tx
                .query_row(
                    "SELECT nombre FROM rest_mesas WHERE id = ?1",
                    params![mesa_id],
                    |row| row.get(0),
                )
                .unwrap_or_else(|_| format!("#{}", mesa_id));
            return Err(format!("La mesa {} ya tiene un pedido propio abierto", nombre));
        }

        // Validar que NO esté unida a otro pedido activo
        let unida_a_otro: i64 = tx
            .query_row(
                "SELECT COUNT(*) FROM rest_pedido_mesas_extra pe
                 JOIN rest_pedidos_abiertos p ON pe.pedido_id = p.id
                 WHERE pe.mesa_id = ?1 AND pe.pedido_id != ?2
                   AND p.estado IN ('ABIERTO', 'CUENTA_PEDIDA')",
                params![mesa_id, pedido_id],
                |row| row.get(0),
            )
            .unwrap_or(0);
        if unida_a_otro > 0 {
            let nombre: String = tx
                .query_row(
                    "SELECT nombre FROM rest_mesas WHERE id = ?1",
                    params![mesa_id],
                    |row| row.get(0),
                )
                .unwrap_or_else(|_| format!("#{}", mesa_id));
            return Err(format!("La mesa {} ya está unida a otro pedido", nombre));
        }

        // Insertar (idempotente — INSERT OR IGNORE por si se reintenta)
        tx.execute(
            "INSERT OR IGNORE INTO rest_pedido_mesas_extra (pedido_id, mesa_id) VALUES (?1, ?2)",
            params![pedido_id, mesa_id],
        )
        .map_err(|e| e.to_string())?;
    }
    tx.commit().map_err(|e| e.to_string())?;
    Ok(())
}

/// Desune una mesa EXTRA del pedido (la libera). NO se puede usar sobre la
/// mesa principal del pedido (esa solo se libera al cobrar/cancelar).
#[tauri::command]
pub fn rest_desunir_mesa(
    db: State<'_, Database>,
    pedido_id: i64,
    mesa_id: i64,
) -> Result<(), String> {
    requiere_modulo_restaurante(&db)?;
    let conn = db.conn.lock().map_err(|e| e.to_string())?;

    // El pedido debe estar abierto/cuenta pedida
    let estado: String = conn
        .query_row(
            "SELECT estado FROM rest_pedidos_abiertos WHERE id = ?1",
            params![pedido_id],
            |row| row.get(0),
        )
        .map_err(|_| "Pedido no encontrado".to_string())?;
    if estado != "ABIERTO" && estado != "CUENTA_PEDIDA" {
        return Err(format!("No se pueden desunir mesas de un pedido {}", estado));
    }

    let filas = conn
        .execute(
            "DELETE FROM rest_pedido_mesas_extra WHERE pedido_id = ?1 AND mesa_id = ?2",
            params![pedido_id, mesa_id],
        )
        .map_err(|e| e.to_string())?;
    if filas == 0 {
        return Err("Esa mesa no estaba unida a este pedido".to_string());
    }
    Ok(())
}

/// Lista las mesas LIBRES disponibles para unir a un pedido. Excluye:
/// - La mesa principal del pedido
/// - Mesas con pedido propio activo
/// - Mesas ya unidas a este o cualquier otro pedido activo
/// - Mesas inactivas
#[tauri::command]
pub fn rest_listar_mesas_libres_para_unir(
    db: State<'_, Database>,
    pedido_id: i64,
) -> Result<Vec<MesaResumen>, String> {
    requiere_modulo_restaurante(&db)?;
    let conn = db.conn.lock().map_err(|e| e.to_string())?;

    let mesa_principal: i64 = conn
        .query_row(
            "SELECT mesa_id FROM rest_pedidos_abiertos WHERE id = ?1",
            params![pedido_id],
            |row| row.get(0),
        )
        .map_err(|_| "Pedido no encontrado".to_string())?;

    let mut stmt = conn
        .prepare(
            "SELECT m.id, m.nombre, m.capacidad, z.nombre
             FROM rest_mesas m
             LEFT JOIN rest_zonas z ON m.zona_id = z.id
             WHERE m.activa = 1
               AND m.id != ?1
               AND NOT EXISTS (
                    SELECT 1 FROM rest_pedidos_abiertos p
                    WHERE p.mesa_id = m.id AND p.estado IN ('ABIERTO', 'CUENTA_PEDIDA')
               )
               AND NOT EXISTS (
                    SELECT 1 FROM rest_pedido_mesas_extra pe
                    JOIN rest_pedidos_abiertos p2 ON pe.pedido_id = p2.id
                    WHERE pe.mesa_id = m.id AND p2.estado IN ('ABIERTO', 'CUENTA_PEDIDA')
               )
             ORDER BY z.orden NULLS LAST, m.orden, m.nombre",
        )
        .map_err(|e| e.to_string())?;
    let mesas: Vec<MesaResumen> = stmt
        .query_map(params![mesa_principal], |row| {
            Ok(MesaResumen {
                id: row.get(0)?,
                nombre: row.get(1)?,
                capacidad: row.get(2)?,
                zona_nombre: row.get(3)?,
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;
    Ok(mesas)
}

// ─── v2.3.69 — Dividir cuenta (sub-cuentas) ──────────────────────────────

/// Divide el total del pedido en N partes iguales y crea N sub-cuentas
/// PENDIENTES de cobro. Si ya existen sub-cuentas para este pedido, falla
/// (hay que cancelar primero la división).
///
/// El reparto: cada sub-cuenta lleva floor(total*100/N)/100 centavos. La
/// ÚLTIMA sub-cuenta absorbe el residuo de redondeo para que la suma cuadre
/// exactamente con el total. Ejemplo: total $100 / 3 → $33.33, $33.33, $33.34.
#[tauri::command]
pub fn rest_dividir_cuenta(
    db: State<'_, Database>,
    pedido_id: i64,
    n_partes: i32,
) -> Result<Vec<Subcuenta>, String> {
    requiere_modulo_restaurante(&db)?;
    if !(2..=20).contains(&n_partes) {
        return Err("El número de partes debe ser entre 2 y 20".to_string());
    }
    let mut conn = db.conn.lock().map_err(|e| e.to_string())?;

    // Validar que el pedido esté activo
    let estado: String = conn
        .query_row(
            "SELECT estado FROM rest_pedidos_abiertos WHERE id = ?1",
            params![pedido_id],
            |row| row.get(0),
        )
        .map_err(|_| "Pedido no encontrado".to_string())?;
    if estado != "ABIERTO" && estado != "CUENTA_PEDIDA" {
        return Err(format!("No se puede dividir un pedido {}", estado));
    }

    // No re-dividir si ya hay sub-cuentas
    let existentes: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM rest_subcuentas WHERE pedido_id = ?1",
            params![pedido_id],
            |row| row.get(0),
        )
        .unwrap_or(0);
    if existentes > 0 {
        return Err("Este pedido ya está dividido. Cancele la división actual primero.".to_string());
    }

    // Calcular total del pedido (suma de items)
    let total: f64 = conn
        .query_row(
            "SELECT COALESCE(SUM(cantidad * precio_unit), 0)
             FROM rest_pedido_items WHERE pedido_id = ?1",
            params![pedido_id],
            |row| row.get(0),
        )
        .unwrap_or(0.0);
    if total <= 0.0 {
        return Err("El pedido no tiene total para dividir (sin items)".to_string());
    }

    // Reparto en centavos para evitar imprecisión float
    let total_centavos: i64 = (total * 100.0).round() as i64;
    let parte_centavos: i64 = total_centavos / (n_partes as i64);
    let residuo: i64 = total_centavos - parte_centavos * (n_partes as i64);

    let tx = conn.transaction().map_err(|e| e.to_string())?;
    for i in 1..=n_partes {
        let centavos = if i == n_partes {
            parte_centavos + residuo
        } else {
            parte_centavos
        };
        let monto = (centavos as f64) / 100.0;
        tx.execute(
            "INSERT INTO rest_subcuentas (pedido_id, numero, total) VALUES (?1, ?2, ?3)",
            params![pedido_id, i, monto],
        )
        .map_err(|e| e.to_string())?;
    }
    tx.commit().map_err(|e| e.to_string())?;

    // Devolver las sub-cuentas creadas
    listar_subcuentas_internal(&conn, pedido_id)
}

/// Lista las sub-cuentas asociadas al pedido (con datos de banco y venta JOIN).
/// Vacío si el pedido no está dividido.
#[tauri::command]
pub fn rest_listar_subcuentas(
    db: State<'_, Database>,
    pedido_id: i64,
) -> Result<Vec<Subcuenta>, String> {
    requiere_modulo_restaurante(&db)?;
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    listar_subcuentas_internal(&conn, pedido_id)
}

/// Cancela la división del pedido (borra todas las sub-cuentas). Solo permitido
/// si NINGUNA sub-cuenta fue cobrada todavía.
#[tauri::command]
pub fn rest_cancelar_division(
    db: State<'_, Database>,
    pedido_id: i64,
) -> Result<(), String> {
    requiere_modulo_restaurante(&db)?;
    let conn = db.conn.lock().map_err(|e| e.to_string())?;

    let cobradas: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM rest_subcuentas WHERE pedido_id = ?1 AND estado = 'COBRADA'",
            params![pedido_id],
            |row| row.get(0),
        )
        .unwrap_or(0);
    if cobradas > 0 {
        return Err(format!(
            "No se puede cancelar la división: ya hay {} sub-cuenta(s) cobrada(s).",
            cobradas
        ));
    }

    conn.execute(
        "DELETE FROM rest_subcuentas WHERE pedido_id = ?1",
        params![pedido_id],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

/// Marca una sub-cuenta como COBRADA, vinculándola con la venta ya generada
/// en el frontend (vía `registrar_venta`). El frontend genera la venta usando
/// el producto especial `_DIVISION_CUENTA_` con precio = monto de la sub-cuenta.
///
/// Cuando TODAS las sub-cuentas del pedido quedan COBRADAS, automáticamente
/// cierra el pedido (`estado=COBRADO`) vinculándolo con la venta de la primera
/// sub-cuenta cobrada — esto libera la mesa principal y todas las extras.
#[tauri::command]
pub fn rest_marcar_subcuenta_cobrada(
    db: State<'_, Database>,
    subcuenta_id: i64,
    venta_id: i64,
    forma_pago: String,
    banco_id: Option<i64>,
    referencia_pago: Option<String>,
) -> Result<ResultadoCobroSubcuenta, String> {
    requiere_modulo_restaurante(&db)?;
    let conn = db.conn.lock().map_err(|e| e.to_string())?;

    // Validar que exista y esté pendiente
    let (pedido_id, estado): (i64, String) = conn
        .query_row(
            "SELECT pedido_id, estado FROM rest_subcuentas WHERE id = ?1",
            params![subcuenta_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .map_err(|_| "Sub-cuenta no encontrada".to_string())?;
    if estado == "COBRADA" {
        return Err("Esta sub-cuenta ya fue cobrada".to_string());
    }

    conn.execute(
        "UPDATE rest_subcuentas
         SET estado = 'COBRADA', forma_pago = ?1, banco_id = ?2, referencia_pago = ?3,
             venta_id = ?4, fecha_cobro = datetime('now', 'localtime')
         WHERE id = ?5",
        params![forma_pago, banco_id, referencia_pago, venta_id, subcuenta_id],
    )
    .map_err(|e| e.to_string())?;

    // ¿Quedan pendientes?
    let pendientes: i32 = conn
        .query_row(
            "SELECT COUNT(*) FROM rest_subcuentas WHERE pedido_id = ?1 AND estado = 'PENDIENTE'",
            params![pedido_id],
            |row| row.get(0),
        )
        .unwrap_or(0);

    let todas_cobradas = pendientes == 0;

    if todas_cobradas {
        // Vincular el pedido con la venta de la PRIMERA sub-cuenta cobrada
        // (cualquiera sirve como ancla — VentasDia mostrará todas independientes).
        let primera_venta_id: i64 = conn
            .query_row(
                "SELECT venta_id FROM rest_subcuentas
                 WHERE pedido_id = ?1 AND venta_id IS NOT NULL
                 ORDER BY numero LIMIT 1",
                params![pedido_id],
                |row| row.get(0),
            )
            .unwrap_or(venta_id);

        conn.execute(
            "UPDATE rest_pedidos_abiertos
             SET estado = 'COBRADO', venta_id = ?1, fecha_cierre = datetime('now', 'localtime')
             WHERE id = ?2",
            params![primera_venta_id, pedido_id],
        )
        .map_err(|e| e.to_string())?;
    }

    Ok(ResultadoCobroSubcuenta {
        todas_cobradas,
        pendientes,
    })
}

/// ID del producto especial usado para sub-cuentas. El frontend lo necesita
/// para construir la venta al cobrar cada sub-cuenta.
#[tauri::command]
pub fn rest_producto_division_id(db: State<'_, Database>) -> Result<i64, String> {
    requiere_modulo_restaurante(&db)?;
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    conn.query_row(
        "SELECT id FROM productos WHERE codigo = '_DIVISION_CUENTA_'",
        params![],
        |row| row.get(0),
    )
    .map_err(|_| "Producto especial _DIVISION_CUENTA_ no encontrado. Reinicia la app.".to_string())
}

// ─── Helpers internos ────────────────────────────────────────────────────

pub(crate) fn obtener_pedido_detalle(conn: &Connection, pedido_id: i64) -> Result<PedidoDetalle, String> {
    let pedido: PedidoAbierto = conn
        .query_row(
            "SELECT id, mesa_id, mesero_id, mesero_nombre, comensales, estado, observacion,
                    fecha_apertura, fecha_cuenta, fecha_cierre, venta_id
             FROM rest_pedidos_abiertos WHERE id = ?1",
            params![pedido_id],
            |row| {
                Ok(PedidoAbierto {
                    id: Some(row.get(0)?),
                    mesa_id: row.get(1)?,
                    mesero_id: row.get(2)?,
                    mesero_nombre: row.get(3)?,
                    comensales: row.get(4)?,
                    estado: row.get(5)?,
                    observacion: row.get(6)?,
                    fecha_apertura: row.get(7)?,
                    fecha_cuenta: row.get(8)?,
                    fecha_cierre: row.get(9)?,
                    venta_id: row.get(10)?,
                })
            },
        )
        .map_err(|_| "Pedido no encontrado".to_string())?;

    let (mesa_nombre, zona_nombre): (String, Option<String>) = conn
        .query_row(
            "SELECT m.nombre, z.nombre FROM rest_mesas m
             LEFT JOIN rest_zonas z ON m.zona_id = z.id
             WHERE m.id = ?1",
            params![pedido.mesa_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .unwrap_or(("?".to_string(), None));

    let mut stmt = conn
        .prepare(
            "SELECT i.id, i.pedido_id, i.producto_id, p.nombre, i.cantidad, i.precio_unit,
                    i.info_adicional, i.enviado_cocina, i.estado_cocina,
                    i.fecha_creacion, i.fecha_envio_cocina,
                    COALESCE(p.destino_preparacion, 'COCINA') as destino
             FROM rest_pedido_items i
             JOIN productos p ON i.producto_id = p.id
             WHERE i.pedido_id = ?1
             ORDER BY i.id",
        )
        .map_err(|e| e.to_string())?;
    let items: Vec<PedidoItem> = stmt
        .query_map(params![pedido_id], |row| {
            Ok(PedidoItem {
                id: Some(row.get(0)?),
                pedido_id: row.get(1)?,
                producto_id: row.get(2)?,
                producto_nombre: Some(row.get(3)?),
                cantidad: row.get(4)?,
                precio_unit: row.get(5)?,
                info_adicional: row.get(6)?,
                enviado_cocina: row.get::<_, i32>(7)? != 0,
                estado_cocina: row.get(8)?,
                fecha_creacion: row.get(9)?,
                fecha_envio_cocina: row.get(10)?,
                destino_preparacion: row.get(11)?,
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    // Total simple — sin IVA por ahora (la venta real lo recalcula con sus reglas)
    let subtotal: f64 = items.iter().map(|i| i.cantidad * i.precio_unit).sum();
    let iva = 0.0;
    let total = subtotal + iva;

    // v2.3.68: Mesas EXTRA unidas a este pedido + capacidad efectiva del grupo.
    let mut mesas_extra_stmt = conn
        .prepare(
            "SELECT m.id, m.nombre, m.capacidad, z.nombre
             FROM rest_pedido_mesas_extra pe
             JOIN rest_mesas m ON pe.mesa_id = m.id
             LEFT JOIN rest_zonas z ON m.zona_id = z.id
             WHERE pe.pedido_id = ?1
             ORDER BY z.orden NULLS LAST, m.orden, m.nombre",
        )
        .map_err(|e| e.to_string())?;
    let mesas_extra: Vec<MesaResumen> = mesas_extra_stmt
        .query_map(params![pedido_id], |row| {
            Ok(MesaResumen {
                id: row.get(0)?,
                nombre: row.get(1)?,
                capacidad: row.get(2)?,
                zona_nombre: row.get(3)?,
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    // Capacidad de la mesa principal (para sumar luego)
    let capacidad_principal: i32 = conn
        .query_row(
            "SELECT capacidad FROM rest_mesas WHERE id = ?1",
            params![pedido.mesa_id],
            |row| row.get(0),
        )
        .unwrap_or(0);
    let capacidad_total: i32 = capacidad_principal + mesas_extra.iter().map(|m| m.capacidad).sum::<i32>();

    // v2.5.91 — abonos (pagos parciales) en HOLDING sobre este pedido.
    let abonos = listar_abonos_pedido(conn, pedido_id).unwrap_or_default();
    let total_abonado: f64 = abonos.iter()
        .filter(|a| a.estado == "HOLDING")
        .map(|a| a.monto)
        .sum();
    let saldo = (total - total_abonado).max(0.0);

    Ok(PedidoDetalle {
        pedido,
        items,
        mesa_nombre,
        zona_nombre,
        subtotal,
        iva,
        total,
        mesas_extra,
        capacidad_total,
        total_abonado: r2(total_abonado),
        saldo: r2(saldo),
        abonos,
    })
}

/// v2.5.91 — Lista los abonos de un pedido (con nombre del banco).
pub fn listar_abonos_pedido(conn: &rusqlite::Connection, pedido_id: i64) -> Result<Vec<crate::restaurante::models::AbonoPedido>, String> {
    let mut stmt = conn.prepare(
        "SELECT a.id, a.monto, a.forma_pago, a.banco_id, cb.nombre, a.referencia_pago,
                a.estado, a.fecha, a.usuario_nombre
         FROM rest_pedido_abonos a
         LEFT JOIN cuentas_banco cb ON a.banco_id = cb.id
         WHERE a.pedido_id = ?1
         ORDER BY a.id ASC",
    ).map_err(|e| e.to_string())?;
    let rows = stmt.query_map(rusqlite::params![pedido_id], |r| {
        Ok(crate::restaurante::models::AbonoPedido {
            id: r.get(0)?,
            monto: r.get(1)?,
            forma_pago: r.get(2)?,
            banco_id: r.get(3)?,
            banco_nombre: r.get(4)?,
            referencia_pago: r.get(5)?,
            estado: r.get(6)?,
            fecha: r.get(7)?,
            usuario_nombre: r.get(8)?,
        })
    }).map_err(|e| e.to_string())?;
    rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
}

/// v2.5.91 — Registra un abono (pago parcial) sobre un pedido. El dinero entra
/// a la caja activa como HOLDING (anticipo), igual que los abonos de ST.
/// Devuelve el id del abono. Valida que el abono no supere el saldo pendiente.
pub fn registrar_abono_pedido(
    conn: &rusqlite::Connection,
    pedido_id: i64,
    monto: f64,
    forma_pago: &str,
    banco_id: Option<i64>,
    referencia_pago: Option<&str>,
    usuario_id: Option<i64>,
    usuario_nombre: Option<&str>,
) -> Result<i64, String> {
    if monto <= 0.0 {
        return Err("El monto del abono debe ser mayor a 0".to_string());
    }
    // Estado del pedido debe estar abierto.
    let estado: String = conn.query_row(
        "SELECT estado FROM rest_pedidos_abiertos WHERE id = ?1",
        rusqlite::params![pedido_id], |r| r.get(0),
    ).map_err(|_| "Pedido no encontrado".to_string())?;
    if estado != "ABIERTO" && estado != "CUENTA_PEDIDA" {
        return Err("El pedido ya no está abierto".to_string());
    }
    // Saldo actual = total consumido − abonos HOLDING.
    let total: f64 = conn.query_row(
        "SELECT COALESCE(SUM(cantidad * precio_unit), 0) FROM rest_pedido_items WHERE pedido_id = ?1",
        rusqlite::params![pedido_id], |r| r.get(0),
    ).unwrap_or(0.0);
    let abonado: f64 = conn.query_row(
        "SELECT COALESCE(SUM(monto), 0) FROM rest_pedido_abonos WHERE pedido_id = ?1 AND estado = 'HOLDING'",
        rusqlite::params![pedido_id], |r| r.get(0),
    ).unwrap_or(0.0);
    let saldo = r2(total - abonado);
    if monto > saldo + 0.01 {
        return Err(format!("El abono (${:.2}) supera el saldo pendiente (${:.2}).", monto, saldo));
    }
    // Caja activa (a la que entra el dinero).
    let caja_id: Option<i64> = conn.query_row(
        "SELECT id FROM caja WHERE estado = 'ABIERTA' LIMIT 1", [], |r| r.get(0),
    ).ok();

    conn.execute(
        "INSERT INTO rest_pedido_abonos
            (pedido_id, monto, forma_pago, banco_id, referencia_pago, caja_id, estado, usuario_id, usuario_nombre)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, 'HOLDING', ?7, ?8)",
        rusqlite::params![pedido_id, r2(monto), forma_pago, banco_id, referencia_pago, caja_id, usuario_id, usuario_nombre],
    ).map_err(|e| e.to_string())?;
    Ok(conn.last_insert_rowid())
}

#[inline]
fn r2(n: f64) -> f64 { (n * 100.0).round() / 100.0 }

/// v2.3.69 — Lista las sub-cuentas del pedido con datos enriquecidos
/// (nombre del banco, número de venta) — usado por `rest_listar_subcuentas`
/// y por `rest_dividir_cuenta` (devuelve las recién creadas).
pub(crate) fn listar_subcuentas_internal(conn: &Connection, pedido_id: i64) -> Result<Vec<Subcuenta>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT s.id, s.pedido_id, s.numero, s.total, s.estado,
                    s.forma_pago, s.banco_id, b.nombre AS banco_nombre,
                    s.referencia_pago, s.venta_id, v.numero AS venta_numero,
                    s.fecha_cobro
             FROM rest_subcuentas s
             LEFT JOIN cuentas_banco b ON s.banco_id = b.id
             LEFT JOIN ventas v ON s.venta_id = v.id
             WHERE s.pedido_id = ?1
             ORDER BY s.numero",
        )
        .map_err(|e| e.to_string())?;
    let subs: Vec<Subcuenta> = stmt
        .query_map(params![pedido_id], |row| {
            Ok(Subcuenta {
                id: row.get(0)?,
                pedido_id: row.get(1)?,
                numero: row.get(2)?,
                total: row.get(3)?,
                estado: row.get(4)?,
                forma_pago: row.get(5)?,
                banco_id: row.get(6)?,
                banco_nombre: row.get(7)?,
                referencia_pago: row.get(8)?,
                venta_id: row.get(9)?,
                venta_numero: row.get(10)?,
                fecha_cobro: row.get(11)?,
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;
    Ok(subs)
}
