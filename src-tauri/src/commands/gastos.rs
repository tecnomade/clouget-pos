use crate::commands::caja::calcular_monto_esperado_actual;
use crate::db::{Database, SesionState};
use crate::models::Gasto;
use tauri::State;

#[tauri::command]
pub fn crear_gasto(db: State<Database>, sesion: State<SesionState>, gasto: Gasto) -> Result<Gasto, String> {
    // v2.3.47: capturar usuario de sesion para trazabilidad
    let (usuario_id, usuario_nombre): (Option<i64>, Option<String>) = {
        let sesion_guard = sesion.sesion.lock().map_err(|e| e.to_string())?;
        match sesion_guard.as_ref() {
            Some(s) => (Some(s.usuario_id), Some(s.nombre.clone())),
            None => (None, None),
        }
    };
    let conn = db.conn.lock().map_err(|e| e.to_string())?;

    // Obtener caja abierta si existe
    let caja_id: Option<i64> = conn
        .query_row(
            "SELECT id FROM caja WHERE estado = 'ABIERTA' LIMIT 1",
            [],
            |row| row.get(0),
        )
        .ok();

    // VALIDACION: si hay caja abierta, no permitir gastos que la dejen
    // en negativo (igual que la validacion en registrar_retiro).
    if let Some(cid) = caja_id {
        let disponible = calcular_monto_esperado_actual(&conn, cid);
        if gasto.monto > disponible + 0.01 {
            return Err(format!(
                "No hay efectivo suficiente en caja. Disponible: ${:.2}. No puede registrar un gasto de ${:.2}.",
                disponible, gasto.monto
            ));
        }
    }

    conn.execute(
        "INSERT INTO gastos (descripcion, monto, categoria, caja_id, observacion, es_recurrente, usuario_id, usuario_nombre)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        rusqlite::params![
            gasto.descripcion,
            gasto.monto,
            gasto.categoria,
            caja_id,
            gasto.observacion,
            gasto.es_recurrente as i32,
            usuario_id,
            usuario_nombre,
        ],
    )
    .map_err(|e| e.to_string())?;

    let id = conn.last_insert_rowid();

    // v2.3.44 FIX: actualizar monto_esperado stored al crear gasto, igual que se hace
    // en registrar_retiro y al crear venta. Antes esto faltaba → descuadre fantasma
    // entre lo mostrado en pantalla (recalculado) y lo guardado (sin restar gastos).
    if let Some(cid) = caja_id {
        let _ = conn.execute(
            "UPDATE caja SET monto_esperado = monto_esperado - ?1 WHERE id = ?2",
            rusqlite::params![gasto.monto, cid],
        );
    }

    // Obtener el gasto insertado con su fecha + JOINs para retornar info completa
    let resultado = conn
        .query_row(
            "SELECT g.id, g.descripcion, g.monto, g.categoria, g.fecha, g.caja_id, g.observacion,
                    COALESCE(g.es_recurrente, 0), g.usuario_id,
                    COALESCE(g.usuario_nombre, u.nombre) as usuario_nombre,
                    c.estado as caja_estado
             FROM gastos g
             LEFT JOIN usuarios u ON g.usuario_id = u.id
             LEFT JOIN caja c ON g.caja_id = c.id
             WHERE g.id = ?1",
            rusqlite::params![id],
            |row| {
                Ok(Gasto {
                    id: Some(row.get(0)?),
                    descripcion: row.get(1)?,
                    monto: row.get(2)?,
                    categoria: row.get(3)?,
                    fecha: row.get(4)?,
                    caja_id: row.get(5)?,
                    observacion: row.get(6)?,
                    es_recurrente: row.get::<_, i32>(7)? != 0,
                    usuario_id: row.get(8).ok(),
                    usuario_nombre: row.get(9).ok(),
                    caja_estado: row.get(10).ok(),
                })
            },
        )
        .map_err(|e| e.to_string())?;

    Ok(resultado)
}

#[tauri::command]
pub fn listar_gastos_dia(db: State<Database>, fecha: String) -> Result<Vec<Gasto>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;

    let mut stmt = conn
        .prepare(
            "SELECT g.id, g.descripcion, g.monto, g.categoria, g.fecha, g.caja_id, g.observacion,
                    COALESCE(g.es_recurrente, 0), g.usuario_id,
                    COALESCE(g.usuario_nombre, u.nombre) as usuario_nombre,
                    c.estado as caja_estado
             FROM gastos g
             LEFT JOIN usuarios u ON g.usuario_id = u.id
             LEFT JOIN caja c ON g.caja_id = c.id
             WHERE date(g.fecha) = date(?1)
             ORDER BY g.fecha DESC",
        )
        .map_err(|e| e.to_string())?;

    let gastos = stmt
        .query_map(rusqlite::params![fecha], |row| {
            Ok(Gasto {
                id: Some(row.get(0)?),
                descripcion: row.get(1)?,
                monto: row.get(2)?,
                categoria: row.get(3)?,
                fecha: row.get(4)?,
                caja_id: row.get(5)?,
                observacion: row.get(6)?,
                es_recurrente: row.get::<_, i32>(7)? != 0,
                usuario_id: row.get(8).ok(),
                usuario_nombre: row.get(9).ok(),
                caja_estado: row.get(10).ok(),
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    Ok(gastos)
}

/// v2.5.54: lista gastos con rango de fechas + filtros opcionales.
/// Reemplaza la limitación de `listar_gastos_dia` (que solo aceptaba un día).
#[tauri::command]
pub fn listar_gastos_rango(
    db: State<Database>,
    fecha_desde: String,
    fecha_hasta: String,
    categoria: Option<String>,
    usuario_id: Option<i64>,
    solo_recurrentes: Option<bool>,
    busqueda: Option<String>,
) -> Result<Vec<Gasto>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;

    let mut sql = String::from(
        "SELECT g.id, g.descripcion, g.monto, g.categoria, g.fecha, g.caja_id, g.observacion,
                COALESCE(g.es_recurrente, 0), g.usuario_id,
                COALESCE(g.usuario_nombre, u.nombre) as usuario_nombre,
                c.estado as caja_estado
         FROM gastos g
         LEFT JOIN usuarios u ON g.usuario_id = u.id
         LEFT JOIN caja c ON g.caja_id = c.id
         WHERE date(g.fecha) >= date(?1) AND date(g.fecha) <= date(?2)"
    );
    let mut params_dyn: Vec<Box<dyn rusqlite::ToSql>> =
        vec![Box::new(fecha_desde), Box::new(fecha_hasta)];

    if let Some(cat) = categoria.filter(|s| !s.trim().is_empty()) {
        sql.push_str(&format!(" AND g.categoria = ?{}", params_dyn.len() + 1));
        params_dyn.push(Box::new(cat));
    }
    if let Some(uid) = usuario_id {
        sql.push_str(&format!(" AND g.usuario_id = ?{}", params_dyn.len() + 1));
        params_dyn.push(Box::new(uid));
    }
    if let Some(true) = solo_recurrentes {
        sql.push_str(" AND COALESCE(g.es_recurrente, 0) = 1");
    }
    if let Some(q) = busqueda.filter(|s| !s.trim().is_empty()) {
        sql.push_str(&format!(
            " AND (g.descripcion LIKE ?{idx} OR g.observacion LIKE ?{idx})",
            idx = params_dyn.len() + 1
        ));
        params_dyn.push(Box::new(format!("%{}%", q)));
    }
    sql.push_str(" ORDER BY g.fecha DESC");

    let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;
    let params_refs: Vec<&dyn rusqlite::ToSql> =
        params_dyn.iter().map(|b| b.as_ref()).collect();
    let gastos = stmt
        .query_map(rusqlite::params_from_iter(params_refs.iter()), |row| {
            Ok(Gasto {
                id: Some(row.get(0)?),
                descripcion: row.get(1)?,
                monto: row.get(2)?,
                categoria: row.get(3)?,
                fecha: row.get(4)?,
                caja_id: row.get(5)?,
                observacion: row.get(6)?,
                es_recurrente: row.get::<_, i32>(7)? != 0,
                usuario_id: row.get(8).ok(),
                usuario_nombre: row.get(9).ok(),
                caja_estado: row.get(10).ok(),
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    Ok(gastos)
}

#[derive(serde::Serialize)]
pub struct ResumenGastos {
    pub total: f64,
    pub count: i64,
    pub promedio: f64,
    pub por_categoria: Vec<CategoriaResumen>,
    pub por_dia: Vec<DiaResumen>,
    pub por_usuario: Vec<UsuarioResumen>,
}

#[derive(serde::Serialize)]
pub struct CategoriaResumen {
    pub categoria: String,
    pub total: f64,
    pub count: i64,
}

#[derive(serde::Serialize)]
pub struct DiaResumen {
    pub dia: String,   // YYYY-MM-DD
    pub total: f64,
    pub count: i64,
}

#[derive(serde::Serialize)]
pub struct UsuarioResumen {
    pub usuario: String,
    pub total: f64,
    pub count: i64,
}

/// v2.5.54: resumen de gastos agrupado por categoría, día y usuario.
/// Usado por Reportes y por la cabecera de Gastos para mostrar KPIs.
#[tauri::command]
pub fn resumen_gastos_rango(
    db: State<Database>,
    fecha_desde: String,
    fecha_hasta: String,
) -> Result<ResumenGastos, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;

    let (total, count): (f64, i64) = conn
        .query_row(
            "SELECT COALESCE(SUM(monto), 0), COUNT(*)
             FROM gastos
             WHERE date(fecha) >= date(?1) AND date(fecha) <= date(?2)",
            rusqlite::params![fecha_desde, fecha_hasta],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .unwrap_or((0.0, 0));
    let promedio = if count > 0 { total / count as f64 } else { 0.0 };

    // Por categoría
    let mut stmt_cat = conn
        .prepare(
            "SELECT COALESCE(categoria, 'Sin categoria') as cat,
                    COALESCE(SUM(monto), 0), COUNT(*)
             FROM gastos
             WHERE date(fecha) >= date(?1) AND date(fecha) <= date(?2)
             GROUP BY cat ORDER BY 2 DESC",
        )
        .map_err(|e| e.to_string())?;
    let por_categoria: Vec<CategoriaResumen> = stmt_cat
        .query_map(rusqlite::params![fecha_desde, fecha_hasta], |r| {
            Ok(CategoriaResumen {
                categoria: r.get(0)?,
                total: r.get(1)?,
                count: r.get(2)?,
            })
        })
        .map_err(|e| e.to_string())?
        .filter_map(Result::ok)
        .collect();
    drop(stmt_cat);

    // Por día
    let mut stmt_dia = conn
        .prepare(
            "SELECT date(fecha) as dia, COALESCE(SUM(monto), 0), COUNT(*)
             FROM gastos
             WHERE date(fecha) >= date(?1) AND date(fecha) <= date(?2)
             GROUP BY dia ORDER BY dia",
        )
        .map_err(|e| e.to_string())?;
    let por_dia: Vec<DiaResumen> = stmt_dia
        .query_map(rusqlite::params![fecha_desde, fecha_hasta], |r| {
            Ok(DiaResumen {
                dia: r.get(0)?,
                total: r.get(1)?,
                count: r.get(2)?,
            })
        })
        .map_err(|e| e.to_string())?
        .filter_map(Result::ok)
        .collect();
    drop(stmt_dia);

    // Por usuario
    let mut stmt_usr = conn
        .prepare(
            "SELECT COALESCE(g.usuario_nombre, u.nombre, 'Sin usuario') as usuario,
                    COALESCE(SUM(g.monto), 0), COUNT(*)
             FROM gastos g
             LEFT JOIN usuarios u ON g.usuario_id = u.id
             WHERE date(g.fecha) >= date(?1) AND date(g.fecha) <= date(?2)
             GROUP BY usuario ORDER BY 2 DESC",
        )
        .map_err(|e| e.to_string())?;
    let por_usuario: Vec<UsuarioResumen> = stmt_usr
        .query_map(rusqlite::params![fecha_desde, fecha_hasta], |r| {
            Ok(UsuarioResumen {
                usuario: r.get(0)?,
                total: r.get(1)?,
                count: r.get(2)?,
            })
        })
        .map_err(|e| e.to_string())?
        .filter_map(Result::ok)
        .collect();

    Ok(ResumenGastos {
        total,
        count,
        promedio,
        por_categoria,
        por_dia,
        por_usuario,
    })
}

#[tauri::command]
pub fn eliminar_gasto(db: State<Database>, id: i64) -> Result<(), String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;

    // Leer monto y caja del gasto antes de borrarlo
    let (monto, caja_id_opt): (f64, Option<i64>) = conn.query_row(
        "SELECT monto, caja_id FROM gastos WHERE id = ?1",
        rusqlite::params![id],
        |row| Ok((row.get(0)?, row.get(1)?)),
    ).map_err(|_| "Gasto no encontrado".to_string())?;

    // v2.3.45 ANTI-FRAUDE: no permitir eliminar gastos cuya caja ya fue cerrada.
    // Esto preserva la integridad del historial — una caja cerrada es un cierre
    // firmado, no debe modificarse despues. Si el gasto fue un error, la unica
    // opcion es registrar otro movimiento de compensacion en la caja actual.
    if let Some(cid) = caja_id_opt {
        let estado: String = conn.query_row(
            "SELECT estado FROM caja WHERE id = ?1",
            rusqlite::params![cid],
            |r| r.get(0),
        ).unwrap_or_else(|_| "DESCONOCIDA".to_string());
        if estado != "ABIERTA" {
            return Err(format!(
                "No se puede eliminar este gasto: pertenece a la caja #{} que ya fue cerrada. Para corregir un gasto incorrecto en una caja cerrada, registra un nuevo gasto/ingreso de compensacion en la caja actual.",
                cid
            ));
        }
    }

    conn.execute("DELETE FROM gastos WHERE id = ?1", rusqlite::params![id])
        .map_err(|e| e.to_string())?;

    // Devolver el monto al monto_esperado stored (caja abierta confirmada arriba)
    if let Some(cid) = caja_id_opt {
        let _ = conn.execute(
            "UPDATE caja SET monto_esperado = monto_esperado + ?1 WHERE id = ?2",
            rusqlite::params![monto, cid],
        );
    }

    Ok(())
}
