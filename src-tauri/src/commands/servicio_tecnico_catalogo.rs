//! v2.4.9 — ST-2: Catálogo jerárquico tipos_equipo → marcas → modelos.
//!
//! Comandos CRUD para gestionar la estructura de equipos del módulo Servicio
//! Técnico. Soft-delete (activo=0) para preservar referencias históricas en
//! órdenes ya creadas.
//!
//! Endpoints:
//! - `st_listar_arbol_completo()` — devuelve árbol jerárquico para UI configuración
//! - `st_listar_tipos_equipo()` / `st_crear_tipo_equipo()` / `st_actualizar_tipo_equipo()` / `st_eliminar_tipo_equipo()`
//! - `st_listar_marcas(tipo_equipo_id)` / `st_crear_marca()` / `st_actualizar_marca()` / `st_eliminar_marca()`
//! - `st_listar_modelos(marca_id)` / `st_crear_modelo()` / `st_actualizar_modelo()` / `st_eliminar_modelo()`
//! - `st_historial_filtrable(filtros)` — historial de órdenes filtrable por cliente/placa/serie/tipo/marca/modelo/fecha/estado

use crate::db::Database;
use rusqlite::params;
use serde::{Deserialize, Serialize};
use tauri::State;

// ─── Estructuras ─────────────────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TipoEquipo {
    pub id: Option<i64>,
    pub nombre: String,
    #[serde(default = "default_icono")]
    pub icono: String,
    #[serde(default)]
    pub requiere_placa: bool,
    #[serde(default)]
    pub requiere_kilometraje: bool,
    #[serde(default)]
    pub requiere_serie: bool,
    #[serde(default)]
    pub orden: i32,
    #[serde(default = "default_true")]
    pub activo: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Marca {
    pub id: Option<i64>,
    pub tipo_equipo_id: i64,
    pub nombre: String,
    #[serde(default = "default_true")]
    pub activo: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Modelo {
    pub id: Option<i64>,
    pub marca_id: i64,
    pub nombre: String,
    pub anio_desde: Option<i32>,
    pub anio_hasta: Option<i32>,
    #[serde(default = "default_true")]
    pub activo: bool,
}

fn default_true() -> bool { true }
fn default_icono() -> String { "🔧".to_string() }

// ─── Helper común ────────────────────────────────────────────────────────

fn requiere_modulo(db: &Database) -> Result<(), String> {
    crate::commands::servicio_tecnico::requiere_modulo_servicio_tecnico(db)
}

// ─── Tipos de Equipo ─────────────────────────────────────────────────────

#[tauri::command]
pub fn st_listar_tipos_equipo(db: State<'_, Database>) -> Result<Vec<TipoEquipo>, String> {
    requiere_modulo(&db)?;
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let mut stmt = conn.prepare(
        "SELECT id, nombre, icono, requiere_placa, requiere_kilometraje, requiere_serie, orden, activo
         FROM st_tipos_equipo WHERE activo = 1 ORDER BY orden, nombre"
    ).map_err(|e| e.to_string())?;
    let rows: Vec<TipoEquipo> = stmt.query_map([], |r| Ok(TipoEquipo {
        id: Some(r.get(0)?),
        nombre: r.get(1)?,
        icono: r.get(2)?,
        requiere_placa: r.get::<_, i32>(3)? != 0,
        requiere_kilometraje: r.get::<_, i32>(4)? != 0,
        requiere_serie: r.get::<_, i32>(5)? != 0,
        orden: r.get(6)?,
        activo: r.get::<_, i32>(7)? != 0,
    })).map_err(|e| e.to_string())?.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())?;
    Ok(rows)
}

#[tauri::command]
pub fn st_crear_tipo_equipo(db: State<'_, Database>, tipo: TipoEquipo) -> Result<i64, String> {
    requiere_modulo(&db)?;
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    conn.execute(
        "INSERT INTO st_tipos_equipo (nombre, icono, requiere_placa, requiere_kilometraje, requiere_serie, orden, activo)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![
            tipo.nombre.trim(), tipo.icono,
            if tipo.requiere_placa { 1 } else { 0 },
            if tipo.requiere_kilometraje { 1 } else { 0 },
            if tipo.requiere_serie { 1 } else { 0 },
            tipo.orden,
            if tipo.activo { 1 } else { 0 }
        ],
    ).map_err(|e| e.to_string())?;
    Ok(conn.last_insert_rowid())
}

#[tauri::command]
pub fn st_actualizar_tipo_equipo(db: State<'_, Database>, tipo: TipoEquipo) -> Result<(), String> {
    requiere_modulo(&db)?;
    let id = tipo.id.ok_or("Tipo sin id")?;
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    conn.execute(
        "UPDATE st_tipos_equipo
         SET nombre = ?1, icono = ?2, requiere_placa = ?3, requiere_kilometraje = ?4,
             requiere_serie = ?5, orden = ?6, activo = ?7
         WHERE id = ?8",
        params![
            tipo.nombre.trim(), tipo.icono,
            if tipo.requiere_placa { 1 } else { 0 },
            if tipo.requiere_kilometraje { 1 } else { 0 },
            if tipo.requiere_serie { 1 } else { 0 },
            tipo.orden,
            if tipo.activo { 1 } else { 0 },
            id
        ],
    ).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn st_eliminar_tipo_equipo(db: State<'_, Database>, id: i64) -> Result<(), String> {
    requiere_modulo(&db)?;
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    // Soft delete — preserva referencias en órdenes históricas
    conn.execute("UPDATE st_tipos_equipo SET activo = 0 WHERE id = ?1", params![id])
        .map_err(|e| e.to_string())?;
    Ok(())
}

// ─── Marcas ──────────────────────────────────────────────────────────────

#[tauri::command]
pub fn st_listar_marcas(db: State<'_, Database>, tipo_equipo_id: i64) -> Result<Vec<Marca>, String> {
    requiere_modulo(&db)?;
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let mut stmt = conn.prepare(
        "SELECT id, tipo_equipo_id, nombre, activo FROM st_marcas
         WHERE tipo_equipo_id = ?1 AND activo = 1 ORDER BY nombre"
    ).map_err(|e| e.to_string())?;
    let rows: Vec<Marca> = stmt.query_map(params![tipo_equipo_id], |r| Ok(Marca {
        id: Some(r.get(0)?),
        tipo_equipo_id: r.get(1)?,
        nombre: r.get(2)?,
        activo: r.get::<_, i32>(3)? != 0,
    })).map_err(|e| e.to_string())?.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())?;
    Ok(rows)
}

#[tauri::command]
pub fn st_crear_marca(db: State<'_, Database>, marca: Marca) -> Result<i64, String> {
    requiere_modulo(&db)?;
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    conn.execute(
        "INSERT INTO st_marcas (tipo_equipo_id, nombre, activo) VALUES (?1, ?2, ?3)",
        params![marca.tipo_equipo_id, marca.nombre.trim(), if marca.activo { 1 } else { 0 }],
    ).map_err(|e| {
        if e.to_string().contains("UNIQUE") {
            format!("Ya existe una marca '{}' para este tipo de equipo", marca.nombre)
        } else {
            e.to_string()
        }
    })?;
    Ok(conn.last_insert_rowid())
}

#[tauri::command]
pub fn st_actualizar_marca(db: State<'_, Database>, marca: Marca) -> Result<(), String> {
    requiere_modulo(&db)?;
    let id = marca.id.ok_or("Marca sin id")?;
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    conn.execute(
        "UPDATE st_marcas SET tipo_equipo_id = ?1, nombre = ?2, activo = ?3 WHERE id = ?4",
        params![marca.tipo_equipo_id, marca.nombre.trim(), if marca.activo { 1 } else { 0 }, id],
    ).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn st_eliminar_marca(db: State<'_, Database>, id: i64) -> Result<(), String> {
    requiere_modulo(&db)?;
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    conn.execute("UPDATE st_marcas SET activo = 0 WHERE id = ?1", params![id])
        .map_err(|e| e.to_string())?;
    Ok(())
}

// ─── Modelos ─────────────────────────────────────────────────────────────

#[tauri::command]
pub fn st_listar_modelos(db: State<'_, Database>, marca_id: i64) -> Result<Vec<Modelo>, String> {
    requiere_modulo(&db)?;
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let mut stmt = conn.prepare(
        "SELECT id, marca_id, nombre, anio_desde, anio_hasta, activo FROM st_modelos
         WHERE marca_id = ?1 AND activo = 1 ORDER BY nombre"
    ).map_err(|e| e.to_string())?;
    let rows: Vec<Modelo> = stmt.query_map(params![marca_id], |r| Ok(Modelo {
        id: Some(r.get(0)?),
        marca_id: r.get(1)?,
        nombre: r.get(2)?,
        anio_desde: r.get(3)?,
        anio_hasta: r.get(4)?,
        activo: r.get::<_, i32>(5)? != 0,
    })).map_err(|e| e.to_string())?.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())?;
    Ok(rows)
}

#[tauri::command]
pub fn st_crear_modelo(db: State<'_, Database>, modelo: Modelo) -> Result<i64, String> {
    requiere_modulo(&db)?;
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    conn.execute(
        "INSERT INTO st_modelos (marca_id, nombre, anio_desde, anio_hasta, activo)
         VALUES (?1, ?2, ?3, ?4, ?5)",
        params![modelo.marca_id, modelo.nombre.trim(), modelo.anio_desde, modelo.anio_hasta, if modelo.activo { 1 } else { 0 }],
    ).map_err(|e| {
        if e.to_string().contains("UNIQUE") {
            format!("Ya existe un modelo '{}' para esta marca", modelo.nombre)
        } else {
            e.to_string()
        }
    })?;
    Ok(conn.last_insert_rowid())
}

#[tauri::command]
pub fn st_actualizar_modelo(db: State<'_, Database>, modelo: Modelo) -> Result<(), String> {
    requiere_modulo(&db)?;
    let id = modelo.id.ok_or("Modelo sin id")?;
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    conn.execute(
        "UPDATE st_modelos SET marca_id = ?1, nombre = ?2, anio_desde = ?3, anio_hasta = ?4, activo = ?5 WHERE id = ?6",
        params![modelo.marca_id, modelo.nombre.trim(), modelo.anio_desde, modelo.anio_hasta, if modelo.activo { 1 } else { 0 }, id],
    ).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn st_eliminar_modelo(db: State<'_, Database>, id: i64) -> Result<(), String> {
    requiere_modulo(&db)?;
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    conn.execute("UPDATE st_modelos SET activo = 0 WHERE id = ?1", params![id])
        .map_err(|e| e.to_string())?;
    Ok(())
}

// ─── Árbol completo (para vista de configuración) ────────────────────────

/// Devuelve el árbol jerárquico completo en una sola llamada.
/// Estructura: `[{ tipo, marcas: [{ marca, modelos: [{ modelo, ordenes_count }] }] }]`
#[tauri::command]
pub fn st_listar_arbol_completo(db: State<'_, Database>) -> Result<Vec<serde_json::Value>, String> {
    requiere_modulo(&db)?;
    let conn = db.conn.lock().map_err(|e| e.to_string())?;

    let mut stmt_tipos = conn.prepare(
        "SELECT id, nombre, icono, requiere_placa, requiere_kilometraje, requiere_serie, orden
         FROM st_tipos_equipo WHERE activo = 1 ORDER BY orden, nombre"
    ).map_err(|e| e.to_string())?;

    let tipos: Vec<(i64, String, String, bool, bool, bool, i32)> = stmt_tipos.query_map([], |r| {
        Ok((
            r.get(0)?, r.get(1)?, r.get(2)?,
            r.get::<_, i32>(3)? != 0,
            r.get::<_, i32>(4)? != 0,
            r.get::<_, i32>(5)? != 0,
            r.get(6)?,
        ))
    }).map_err(|e| e.to_string())?.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())?;

    let mut arbol: Vec<serde_json::Value> = Vec::new();
    for (tipo_id, tipo_nombre, icono, req_placa, req_km, req_serie, _orden) in tipos {
        let count_ordenes_tipo: i64 = conn.query_row(
            "SELECT COUNT(*) FROM ordenes_servicio WHERE tipo_equipo_id = ?1",
            params![tipo_id], |r| r.get(0)
        ).unwrap_or(0);

        let mut marcas_list: Vec<serde_json::Value> = Vec::new();
        let mut stmt_marcas = conn.prepare(
            "SELECT id, nombre FROM st_marcas WHERE tipo_equipo_id = ?1 AND activo = 1 ORDER BY nombre"
        ).map_err(|e| e.to_string())?;
        let marcas: Vec<(i64, String)> = stmt_marcas.query_map(params![tipo_id], |r| {
            Ok((r.get(0)?, r.get(1)?))
        }).map_err(|e| e.to_string())?.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())?;

        for (marca_id, marca_nombre) in marcas {
            let count_ordenes_marca: i64 = conn.query_row(
                "SELECT COUNT(*) FROM ordenes_servicio WHERE marca_id = ?1",
                params![marca_id], |r| r.get(0)
            ).unwrap_or(0);

            let mut stmt_modelos = conn.prepare(
                "SELECT id, nombre, anio_desde, anio_hasta FROM st_modelos
                 WHERE marca_id = ?1 AND activo = 1 ORDER BY nombre"
            ).map_err(|e| e.to_string())?;
            let modelos: Vec<serde_json::Value> = stmt_modelos.query_map(params![marca_id], |r| {
                let id: i64 = r.get(0)?;
                Ok(serde_json::json!({
                    "id": id,
                    "nombre": r.get::<_, String>(1)?,
                    "anio_desde": r.get::<_, Option<i32>>(2)?,
                    "anio_hasta": r.get::<_, Option<i32>>(3)?,
                }))
            }).map_err(|e| e.to_string())?
              .filter_map(|r| r.ok())
              .map(|mut m| {
                  let mid = m["id"].as_i64().unwrap_or(0);
                  let count: i64 = conn.query_row(
                      "SELECT COUNT(*) FROM ordenes_servicio WHERE modelo_id = ?1",
                      params![mid], |r| r.get(0)
                  ).unwrap_or(0);
                  m["ordenes_count"] = serde_json::json!(count);
                  m
              }).collect();

            marcas_list.push(serde_json::json!({
                "id": marca_id,
                "nombre": marca_nombre,
                "ordenes_count": count_ordenes_marca,
                "modelos": modelos,
            }));
        }

        arbol.push(serde_json::json!({
            "id": tipo_id,
            "nombre": tipo_nombre,
            "icono": icono,
            "requiere_placa": req_placa,
            "requiere_kilometraje": req_km,
            "requiere_serie": req_serie,
            "ordenes_count": count_ordenes_tipo,
            "marcas": marcas_list,
        }));
    }

    Ok(arbol)
}

// ─── Historial filtrable ─────────────────────────────────────────────────

#[derive(Debug, Deserialize, Default)]
pub struct FiltrosHistorial {
    pub cliente_id: Option<i64>,
    pub busqueda_cliente: Option<String>,        // busca por nombre/identificación parcial
    pub placa: Option<String>,
    pub serie: Option<String>,
    pub tipo_equipo_id: Option<i64>,
    pub marca_id: Option<i64>,
    pub modelo_id: Option<i64>,
    pub estado: Option<String>,
    pub fecha_desde: Option<String>,
    pub fecha_hasta: Option<String>,
    pub limite: Option<i64>,
}

/// Historial filtrable de órdenes. Usado en pestaña "Historial" del módulo.
#[tauri::command]
pub fn st_historial_filtrable(
    db: State<'_, Database>,
    filtros: FiltrosHistorial,
) -> Result<serde_json::Value, String> {
    requiere_modulo(&db)?;
    let conn = db.conn.lock().map_err(|e| e.to_string())?;

    let mut wheres: Vec<String> = Vec::new();
    let mut params_dyn: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();

    if let Some(cid) = filtros.cliente_id {
        params_dyn.push(Box::new(cid));
        wheres.push(format!("o.cliente_id = ?{}", params_dyn.len()));
    }
    if let Some(b) = filtros.busqueda_cliente.as_ref().filter(|s| !s.trim().is_empty()) {
        let pat = format!("%{}%", b.trim());
        params_dyn.push(Box::new(pat.clone()));
        let p1 = params_dyn.len();
        params_dyn.push(Box::new(pat));
        let p2 = params_dyn.len();
        wheres.push(format!("(o.cliente_nombre LIKE ?{} OR c.identificacion LIKE ?{})", p1, p2));
    }
    if let Some(p) = filtros.placa.as_ref().filter(|s| !s.trim().is_empty()) {
        params_dyn.push(Box::new(format!("%{}%", p.trim())));
        wheres.push(format!("o.equipo_placa LIKE ?{}", params_dyn.len()));
    }
    if let Some(s) = filtros.serie.as_ref().filter(|s| !s.trim().is_empty()) {
        params_dyn.push(Box::new(format!("%{}%", s.trim())));
        wheres.push(format!("o.equipo_serie LIKE ?{}", params_dyn.len()));
    }
    if let Some(t) = filtros.tipo_equipo_id {
        params_dyn.push(Box::new(t));
        wheres.push(format!("o.tipo_equipo_id = ?{}", params_dyn.len()));
    }
    if let Some(m) = filtros.marca_id {
        params_dyn.push(Box::new(m));
        wheres.push(format!("o.marca_id = ?{}", params_dyn.len()));
    }
    if let Some(m) = filtros.modelo_id {
        params_dyn.push(Box::new(m));
        wheres.push(format!("o.modelo_id = ?{}", params_dyn.len()));
    }
    if let Some(e) = filtros.estado.as_ref().filter(|s| !s.is_empty()) {
        params_dyn.push(Box::new(e.clone()));
        wheres.push(format!("o.estado = ?{}", params_dyn.len()));
    }
    if let Some(d) = filtros.fecha_desde.as_ref().filter(|s| !s.is_empty()) {
        params_dyn.push(Box::new(d.clone()));
        wheres.push(format!("date(o.fecha_ingreso) >= date(?{})", params_dyn.len()));
    }
    if let Some(h) = filtros.fecha_hasta.as_ref().filter(|s| !s.is_empty()) {
        params_dyn.push(Box::new(h.clone()));
        wheres.push(format!("date(o.fecha_ingreso) <= date(?{})", params_dyn.len()));
    }

    let where_sql = if wheres.is_empty() { "1=1".to_string() } else { wheres.join(" AND ") };
    let limite = filtros.limite.unwrap_or(200).clamp(1, 1000);

    let sql = format!(
        "SELECT o.id, o.numero, o.cliente_id, o.cliente_nombre, c.identificacion,
                o.tipo_equipo, o.equipo_descripcion, o.equipo_marca, o.equipo_modelo,
                o.equipo_serie, o.equipo_placa,
                o.problema_reportado, o.estado, o.fecha_ingreso, o.fecha_entrega,
                o.presupuesto, o.monto_final, o.venta_id
         FROM ordenes_servicio o
         LEFT JOIN clientes c ON o.cliente_id = c.id
         WHERE {}
         ORDER BY o.fecha_ingreso DESC
         LIMIT {}",
        where_sql, limite
    );

    let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;
    let params_refs: Vec<&dyn rusqlite::ToSql> = params_dyn.iter().map(|b| b.as_ref()).collect();

    let ordenes: Vec<serde_json::Value> = stmt.query_map(rusqlite::params_from_iter(params_refs.iter()), |r| {
        Ok(serde_json::json!({
            "id": r.get::<_, i64>(0)?,
            "numero": r.get::<_, String>(1)?,
            "cliente_id": r.get::<_, Option<i64>>(2)?,
            "cliente_nombre": r.get::<_, Option<String>>(3)?,
            "cliente_identificacion": r.get::<_, Option<String>>(4)?,
            "tipo_equipo": r.get::<_, String>(5)?,
            "equipo_descripcion": r.get::<_, String>(6)?,
            "equipo_marca": r.get::<_, Option<String>>(7)?,
            "equipo_modelo": r.get::<_, Option<String>>(8)?,
            "equipo_serie": r.get::<_, Option<String>>(9)?,
            "equipo_placa": r.get::<_, Option<String>>(10)?,
            "problema_reportado": r.get::<_, String>(11)?,
            "estado": r.get::<_, String>(12)?,
            "fecha_ingreso": r.get::<_, String>(13)?,
            "fecha_entrega": r.get::<_, Option<String>>(14)?,
            "presupuesto": r.get::<_, f64>(15)?,
            "monto_final": r.get::<_, f64>(16)?,
            "venta_id": r.get::<_, Option<i64>>(17)?,
        }))
    }).map_err(|e| e.to_string())?
      .collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())?;

    let total_monto: f64 = ordenes.iter()
        .filter_map(|o| o["monto_final"].as_f64())
        .sum();

    Ok(serde_json::json!({
        "ok": true,
        "ordenes": ordenes,
        "total": ordenes.len(),
        "total_monto": total_monto,
    }))
}
