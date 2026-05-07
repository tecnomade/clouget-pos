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

fn listar_mesas_con_estado_internal(conn: &Connection) -> Result<Vec<MesaConEstado>, String> {
    // v2.3.63 FIX: usar subquery con MAX(id) para garantizar UN SOLO pedido
    // por mesa. Si por algun bug/race condition hay multiples pedidos abiertos
    // para la misma mesa, agarramos el MÁS RECIENTE (que probablemente es el
    // que tiene los items reales). Esto previene el bug "items desaparecen"
    // que pasa cuando el LEFT JOIN devuelve filas duplicadas y aleatorio cual
    // gana en SQLite.
    //
    // Ademas: AUTO-LIMPIEZA al pasada — cierra pedidos abiertos VACIOS de mas
    // de 24h (sin items, claramente abandonados). Idempotente y safe.
    let _ = conn.execute(
        "UPDATE rest_pedidos_abiertos
         SET estado = 'CANCELADO', fecha_cierre = datetime('now', 'localtime')
         WHERE estado IN ('ABIERTO', 'CUENTA_PEDIDA')
           AND julianday('now') - julianday(fecha_apertura) > 1.0
           AND id NOT IN (SELECT DISTINCT pedido_id FROM rest_pedido_items)",
        [],
    );

    let mut stmt = conn
        .prepare(
            "SELECT
                m.id, m.zona_id, z.nombre, z.color, m.nombre, m.capacidad, m.orden,
                p.id, p.mesero_nombre, p.comensales, p.estado, p.fecha_apertura
             FROM rest_mesas m
             LEFT JOIN rest_zonas z ON m.zona_id = z.id
             LEFT JOIN rest_pedidos_abiertos p
                ON p.id = (
                    -- Pedido MÁS RECIENTE de esta mesa, abierto o con cuenta pedida.
                    -- Si hay multiples (bug), gana el de id mayor (mas reciente).
                    SELECT MAX(p2.id) FROM rest_pedidos_abiertos p2
                    WHERE p2.mesa_id = m.id
                      AND p2.estado IN ('ABIERTO', 'CUENTA_PEDIDA')
                )
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
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    // Para cada mesa con pedido, calcular total y items pendientes en cocina
    for m in &mut mesas {
        if let Some(pid) = m.pedido_id {
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

            // Calcular minutos desde apertura
            if let Some(fecha) = &m.fecha_apertura {
                if let Ok(mins) = conn.query_row(
                    "SELECT CAST((julianday('now', 'localtime') - julianday(?1)) * 24 * 60 AS INTEGER)",
                    params![fecha],
                    |row| row.get::<_, i64>(0),
                ) {
                    m.minutos_abierta = Some(mins);
                }
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
    Ok(())
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
    let impresora_cocina_efectiva = impresora_cocina.clone().or_else(|| impresora_principal.clone());

    // Impresora separada para barra (opcional, si config `comanda_modo_separado=1`)
    let modo_separado = config.get("comanda_modo_separado").map(|s| s.as_str()) == Some("1");
    let impresora_barra = config.get("impresora_barra")
        .map(|s| s.trim().to_string()).filter(|s| !s.is_empty())
        .or_else(|| impresora_cocina_efectiva.clone()); // fallback a cocina si no hay separada

    // Si no hay NINGUNA impresora configurada, error claro
    if impresora_cocina_efectiva.is_none() {
        return Err("No hay impresora configurada (ni 'impresora_cocina' ni 'impresora' principal). Vaya a Configuración.".to_string());
    }

    let mut mensajes: Vec<String> = Vec::new();

    if modo_separado {
        // 2 tickets separados: COCINA + BARRA
        let ticket_cocina = super::printing::generar_comanda_cocina(
            pedido_id,
            &detalle.mesa_nombre,
            detalle.zona_nombre.as_deref(),
            detalle.pedido.mesero_nombre.as_deref(),
            &items_objetivo,
            super::printing::DestinoComanda::Cocina,
            &config,
        );
        let ticket_barra = super::printing::generar_comanda_cocina(
            pedido_id,
            &detalle.mesa_nombre,
            detalle.zona_nombre.as_deref(),
            detalle.pedido.mesero_nombre.as_deref(),
            &items_objetivo,
            super::printing::DestinoComanda::Barra,
            &config,
        );

        if let Some(t) = ticket_cocina {
            if let Some(ref imp) = impresora_cocina_efectiva {
                crate::printing::imprimir_raw_windows(imp, &t)?;
                mensajes.push("🍳 Cocina impresa".to_string());
            }
        }
        if let Some(t) = ticket_barra {
            if let Some(ref imp) = impresora_barra {
                crate::printing::imprimir_raw_windows(imp, &t)?;
                mensajes.push("🍷 Barra impresa".to_string());
            }
        }
    } else {
        // 1 ticket combinado (default)
        let ticket = super::printing::generar_comanda_cocina(
            pedido_id,
            &detalle.mesa_nombre,
            detalle.zona_nombre.as_deref(),
            detalle.pedido.mesero_nombre.as_deref(),
            &items_objetivo,
            super::printing::DestinoComanda::Ambos,
            &config,
        );
        if let Some(t) = ticket {
            if let Some(ref imp) = impresora_cocina_efectiva {
                crate::printing::imprimir_raw_windows(imp, &t)?;
                mensajes.push("🍽 Comanda impresa".to_string());
            }
        }
    }

    if mensajes.is_empty() {
        // No había items COCINA/BARRA (todos eran DIRECTO) — no es error, solo informativo
        Ok("Sin items para cocina (todos despacho directo)".to_string())
    } else {
        Ok(mensajes.join(" + "))
    }
}

// ─── Helpers internos ────────────────────────────────────────────────────

fn obtener_pedido_detalle(conn: &Connection, pedido_id: i64) -> Result<PedidoDetalle, String> {
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

    Ok(PedidoDetalle {
        pedido,
        items,
        mesa_nombre,
        zona_nombre,
        subtotal,
        iva,
        total,
    })
}
