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
    let mut stmt = conn
        .prepare(
            "SELECT
                m.id, m.zona_id, z.nombre, z.color, m.nombre, m.capacidad, m.orden,
                p.id, p.mesero_nombre, p.comensales, p.estado, p.fecha_apertura
             FROM rest_mesas m
             LEFT JOIN rest_zonas z ON m.zona_id = z.id
             LEFT JOIN rest_pedidos_abiertos p
                ON p.mesa_id = m.id AND p.estado IN ('ABIERTO', 'CUENTA_PEDIDA')
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

    // Obtener precio del producto
    let precio: f64 = conn
        .query_row(
            "SELECT precio_venta FROM productos WHERE id = ?1 AND activo = 1",
            params![producto_id],
            |row| row.get(0),
        )
        .map_err(|_| "Producto no encontrado o inactivo".to_string())?;

    conn.execute(
        "INSERT INTO rest_pedido_items (pedido_id, producto_id, cantidad, precio_unit, info_adicional)
         VALUES (?1, ?2, ?3, ?4, ?5)",
        params![pedido_id, producto_id, cantidad, precio, info_adicional],
    )
    .map_err(|e| e.to_string())?;
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
    // Solo permitir eliminar items que NO han sido enviados a cocina
    let enviado: i32 = conn
        .query_row(
            "SELECT enviado_cocina FROM rest_pedido_items WHERE id = ?1",
            params![item_id],
            |row| row.get(0),
        )
        .map_err(|_| "Item no encontrado".to_string())?;
    if enviado != 0 {
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

    // Obtener items pendientes ANTES de marcarlos
    let mut stmt = conn
        .prepare(
            "SELECT i.id, i.pedido_id, i.producto_id, p.nombre, i.cantidad, i.precio_unit,
                    i.info_adicional, i.enviado_cocina, i.estado_cocina,
                    i.fecha_creacion, i.fecha_envio_cocina
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
                    i.fecha_creacion, i.fecha_envio_cocina
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
