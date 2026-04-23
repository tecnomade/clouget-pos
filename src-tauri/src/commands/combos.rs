// Comandos para Combos / Kits / Productos compuestos
//
// Tipos de combo:
//   - COMBO_FIJO: componentes fijos con cantidad fija. Al vender se descuenta stock de cada hijo.
//   - COMBO_FLEXIBLE: componentes agrupados; el cajero escoge en el POS cuantos y cuales.
//
// El precio del combo es independiente (productos.precio_venta del padre).
// El stock del padre (productos.stock_actual) se ignora — se calcula dinamicamente como
//   MIN(stock_hijo / cantidad_componente) por cada componente.

use crate::db::Database;
use crate::models::{ComboGrupo, ComboComponente};
use rusqlite::Connection;
use tauri::State;

/// Lista los grupos de un combo (vacio para COMBO_FIJO).
#[tauri::command]
pub fn listar_combo_grupos(db: State<Database>, producto_padre_id: i64) -> Result<Vec<ComboGrupo>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let mut stmt = conn.prepare(
        "SELECT id, producto_padre_id, nombre, minimo, maximo, orden
         FROM producto_componente_grupos
         WHERE producto_padre_id = ?1 ORDER BY orden ASC, id ASC"
    ).map_err(|e| e.to_string())?;
    let rows = stmt.query_map(rusqlite::params![producto_padre_id], |r| Ok(ComboGrupo {
        id: Some(r.get(0)?),
        producto_padre_id: r.get(1)?,
        nombre: r.get(2)?,
        minimo: r.get(3)?,
        maximo: r.get(4)?,
        orden: r.get(5)?,
    })).map_err(|e| e.to_string())?;
    rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
}

/// Lista los componentes (productos hijos) de un combo, enriquecidos con datos del hijo.
#[tauri::command]
pub fn listar_combo_componentes(db: State<Database>, producto_padre_id: i64) -> Result<Vec<ComboComponente>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let mut stmt = conn.prepare(
        "SELECT c.id, c.producto_padre_id, c.producto_hijo_id, c.cantidad, c.grupo_id, c.orden,
                p.nombre, p.codigo, p.precio_venta, p.precio_costo, p.stock_actual, p.unidad_medida,
                p.no_controla_stock, p.es_servicio
         FROM producto_componentes c
         JOIN productos p ON c.producto_hijo_id = p.id
         WHERE c.producto_padre_id = ?1
         ORDER BY c.grupo_id NULLS FIRST, c.orden ASC, c.id ASC"
    ).map_err(|e| e.to_string())?;
    let rows = stmt.query_map(rusqlite::params![producto_padre_id], |r| Ok(ComboComponente {
        id: Some(r.get(0)?),
        producto_padre_id: r.get(1)?,
        producto_hijo_id: r.get(2)?,
        cantidad: r.get(3)?,
        grupo_id: r.get(4)?,
        orden: r.get(5)?,
        hijo_nombre: r.get(6)?,
        hijo_codigo: r.get(7)?,
        hijo_precio_venta: r.get(8)?,
        hijo_precio_costo: r.get(9)?,
        hijo_stock_actual: r.get(10)?,
        hijo_unidad_medida: r.get(11)?,
        hijo_no_controla_stock: r.get::<_, i32>(12).ok().map(|v| v != 0),
        hijo_es_servicio: r.get::<_, i32>(13).ok().map(|v| v != 0),
    })).map_err(|e| e.to_string())?;
    rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
}

/// Reemplaza atomicamente la lista de grupos y componentes de un combo.
/// Espera la lista completa: cualquier grupo/componente no incluido se elimina.
#[tauri::command]
pub fn guardar_combo_estructura(
    db: State<Database>,
    producto_padre_id: i64,
    grupos: Vec<ComboGrupo>,
    componentes: Vec<ComboComponente>,
) -> Result<(), String> {
    let mut conn = db.conn.lock().map_err(|e| e.to_string())?;

    // Validacion: ningun componente puede ser el mismo producto padre (recursion)
    for c in &componentes {
        if c.producto_hijo_id == producto_padre_id {
            return Err("Un combo no puede contener a si mismo como componente".into());
        }
        if c.cantidad <= 0.0 {
            return Err(format!("Componente con cantidad invalida: {}", c.cantidad));
        }
    }

    let tx = conn.transaction().map_err(|e| e.to_string())?;

    // Limpiar todo lo previo
    tx.execute("DELETE FROM producto_componentes WHERE producto_padre_id = ?1", rusqlite::params![producto_padre_id])
        .map_err(|e| e.to_string())?;
    tx.execute("DELETE FROM producto_componente_grupos WHERE producto_padre_id = ?1", rusqlite::params![producto_padre_id])
        .map_err(|e| e.to_string())?;

    // Insertar grupos y mapear IDs temporales -> IDs reales (los grupos pueden venir con id None
    // si son nuevos, o un id "temporal" negativo del cliente si quiere referenciarlos en componentes)
    let mut id_map: std::collections::HashMap<i64, i64> = std::collections::HashMap::new();
    for g in &grupos {
        tx.execute(
            "INSERT INTO producto_componente_grupos (producto_padre_id, nombre, minimo, maximo, orden)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            rusqlite::params![producto_padre_id, g.nombre, g.minimo, g.maximo, g.orden],
        ).map_err(|e| e.to_string())?;
        let nuevo_id = tx.last_insert_rowid();
        if let Some(old_id) = g.id {
            id_map.insert(old_id, nuevo_id);
        }
    }

    // Insertar componentes (resolviendo grupo_id via id_map si aplica)
    for c in &componentes {
        let grupo_id_real = c.grupo_id.and_then(|gid| id_map.get(&gid).copied().or(Some(gid)));
        tx.execute(
            "INSERT INTO producto_componentes (producto_padre_id, producto_hijo_id, cantidad, grupo_id, orden)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            rusqlite::params![producto_padre_id, c.producto_hijo_id, c.cantidad, grupo_id_real, c.orden],
        ).map_err(|e| e.to_string())?;
    }

    tx.commit().map_err(|e| e.to_string())
}

/// Calcula stock disponible de un combo (cuantas unidades del combo pueden formarse
/// con el stock actual de sus componentes). Solo aplica a COMBO_FIJO.
/// Para COMBO_FLEXIBLE devuelve None (depende de la seleccion).
pub fn calcular_stock_combo(conn: &Connection, producto_padre_id: i64) -> Result<Option<f64>, String> {
    let tipo: String = conn.query_row(
        "SELECT COALESCE(tipo_producto, 'SIMPLE') FROM productos WHERE id = ?1",
        rusqlite::params![producto_padre_id],
        |r| r.get(0),
    ).map_err(|e| e.to_string())?;

    if tipo != "COMBO_FIJO" {
        return Ok(None);
    }

    let mut stmt = conn.prepare(
        "SELECT c.cantidad, p.stock_actual, COALESCE(p.no_controla_stock, 0), COALESCE(p.es_servicio, 0)
         FROM producto_componentes c
         JOIN productos p ON c.producto_hijo_id = p.id
         WHERE c.producto_padre_id = ?1"
    ).map_err(|e| e.to_string())?;

    let rows = stmt.query_map(rusqlite::params![producto_padre_id], |r| {
        Ok((r.get::<_, f64>(0)?, r.get::<_, f64>(1)?, r.get::<_, i32>(2)? != 0, r.get::<_, i32>(3)? != 0))
    }).map_err(|e| e.to_string())?;

    let mut min_combos: Option<f64> = None;
    let mut tiene_componentes = false;
    for r in rows {
        let (cant_componente, stock_hijo, no_stock, es_serv) = r.map_err(|e| e.to_string())?;
        tiene_componentes = true;
        // Servicios y productos sin control de stock no limitan
        if no_stock || es_serv { continue; }
        if cant_componente <= 0.0 { continue; }
        let posibles = (stock_hijo / cant_componente).floor();
        min_combos = Some(match min_combos {
            None => posibles,
            Some(actual) => actual.min(posibles),
        });
    }

    if !tiene_componentes {
        return Ok(Some(0.0));
    }
    Ok(min_combos.or(Some(0.0)))
}

#[tauri::command]
pub fn stock_combo(db: State<Database>, producto_padre_id: i64) -> Result<Option<f64>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    calcular_stock_combo(&conn, producto_padre_id)
}

/// Devuelve resumen "es combo + tipo + stock_calculado" para usar en UI rapidamente
#[tauri::command]
pub fn info_combo_resumen(db: State<Database>, producto_id: i64) -> Result<serde_json::Value, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let tipo: String = conn.query_row(
        "SELECT COALESCE(tipo_producto, 'SIMPLE') FROM productos WHERE id = ?1",
        rusqlite::params![producto_id],
        |r| r.get(0),
    ).map_err(|e| e.to_string())?;

    let stock = calcular_stock_combo(&conn, producto_id)?;
    let total_componentes: i64 = conn.query_row(
        "SELECT COUNT(*) FROM producto_componentes WHERE producto_padre_id = ?1",
        rusqlite::params![producto_id], |r| r.get(0)
    ).unwrap_or(0);

    Ok(serde_json::json!({
        "tipo_producto": tipo,
        "es_combo": tipo == "COMBO_FIJO" || tipo == "COMBO_FLEXIBLE",
        "stock_calculado": stock,
        "total_componentes": total_componentes,
    }))
}
