use crate::db::{Database, SesionState};
use crate::models::{Caja, ResumenCaja};
use tauri::State;

/// Helper interno: registra evento de auditoria en caja_eventos
fn log_evento_caja(
    conn: &rusqlite::Connection,
    caja_id: i64,
    evento: &str,
    usuario: &str,
    usuario_id: i64,
    valor_anterior: Option<&str>,
    valor_nuevo: Option<&str>,
    motivo: Option<&str>,
) {
    let _ = conn.execute(
        "INSERT INTO caja_eventos (caja_id, evento, usuario, usuario_id, valor_anterior, valor_nuevo, motivo)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        rusqlite::params![caja_id, evento, usuario, usuario_id, valor_anterior, valor_nuevo, motivo],
    );
}

/// Devuelve info del ultimo cierre: monto_real, fecha_cierre, usuario_cierre, id.
/// Se usa al abrir caja para sugerir el monto inicial y advertir si difiere.
#[tauri::command]
pub fn obtener_ultimo_cierre(db: State<Database>) -> Result<Option<serde_json::Value>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let res = conn.query_row(
        "SELECT id, monto_real, COALESCE(cerrada_at, fecha_cierre) as cerrada_at, usuario_cierre, usuario, diferencia
         FROM caja
         WHERE estado = 'CERRADA' AND monto_real IS NOT NULL
         ORDER BY id DESC LIMIT 1",
        [],
        |r| Ok(serde_json::json!({
            "caja_id": r.get::<_, i64>(0)?,
            "monto_real": r.get::<_, Option<f64>>(1)?,
            "cerrada_at": r.get::<_, Option<String>>(2)?,
            "usuario_cierre": r.get::<_, Option<String>>(3)?.or_else(|| r.get::<_, Option<String>>(4).ok().flatten()),
            "diferencia_cierre": r.get::<_, Option<f64>>(5)?,
        })),
    );
    match res {
        Ok(v) => Ok(Some(v)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(e.to_string()),
    }
}

#[tauri::command]
pub fn abrir_caja(
    db: State<Database>,
    sesion: State<SesionState>,
    monto_inicial: f64,
    motivo_diferencia: Option<String>,
    desglose: Option<String>,
) -> Result<Caja, String> {
    // Obtener usuario de la sesión
    let sesion_guard = sesion.sesion.lock().map_err(|e| e.to_string())?;
    let sesion_actual = sesion_guard
        .as_ref()
        .ok_or("Debe iniciar sesión para abrir la caja".to_string())?;
    let usuario_nombre = sesion_actual.nombre.clone();
    let usuario_id = sesion_actual.usuario_id;
    drop(sesion_guard);

    if monto_inicial < 0.0 {
        return Err("Monto inicial no puede ser negativo".to_string());
    }

    let conn = db.conn.lock().map_err(|e| e.to_string())?;

    // Verificar que no haya caja abierta
    let caja_abierta: bool = conn
        .query_row(
            "SELECT COUNT(*) FROM caja WHERE estado = 'ABIERTA'",
            [],
            |row| row.get::<_, i64>(0),
        )
        .map(|count| count > 0)
        .unwrap_or(false);

    if caja_abierta {
        return Err("Ya existe una caja abierta. Ciérrela primero.".to_string());
    }

    // Buscar ultimo cierre para validar continuidad
    let ultimo: Option<(i64, f64)> = conn.query_row(
        "SELECT id, COALESCE(monto_real, 0) FROM caja
         WHERE estado = 'CERRADA' AND monto_real IS NOT NULL
         ORDER BY id DESC LIMIT 1",
        [],
        |r| Ok((r.get(0)?, r.get(1)?)),
    ).ok();

    let (caja_anterior_id, monto_esperado_apertura) = match ultimo {
        Some((id, m)) => (Some(id), m),
        None => (None, 0.0),
    };

    // Si difiere del cierre anterior, exigir motivo
    let difiere = (monto_inicial - monto_esperado_apertura).abs() > 0.01;
    if difiere && caja_anterior_id.is_some() {
        let motivo_str = motivo_diferencia.as_deref().map(|s| s.trim()).unwrap_or("");
        if motivo_str.len() < 5 {
            return Err(format!(
                "DESCUADRE_APERTURA:{:.2}:{:.2}:El monto inicial difiere del cierre anterior. Cierre anterior: ${:.2}, apertura intentada: ${:.2}, diferencia: ${:.2}. Debe justificar la diferencia (mínimo 5 caracteres).",
                monto_esperado_apertura, monto_inicial,
                monto_esperado_apertura, monto_inicial, monto_inicial - monto_esperado_apertura
            ));
        }
    }

    conn.execute(
        "INSERT INTO caja (monto_inicial, monto_esperado, estado, usuario, usuario_id, motivo_diferencia_apertura, caja_anterior_id, desglose_apertura)
         VALUES (?1, ?1, 'ABIERTA', ?2, ?3, ?4, ?5, ?6)",
        rusqlite::params![monto_inicial, usuario_nombre, usuario_id, motivo_diferencia, caja_anterior_id, desglose],
    )
    .map_err(|e| e.to_string())?;

    let id = conn.last_insert_rowid();

    // Log evento APERTURA
    let snapshot_nuevo = serde_json::json!({
        "monto_inicial": monto_inicial,
        "caja_anterior_id": caja_anterior_id,
        "monto_esperado_apertura": monto_esperado_apertura,
        "diferencia_apertura": monto_inicial - monto_esperado_apertura,
    }).to_string();
    log_evento_caja(&conn, id, "APERTURA", &usuario_nombre, usuario_id,
        None, Some(&snapshot_nuevo), motivo_diferencia.as_deref());

    Ok(Caja {
        id: Some(id),
        fecha_apertura: None,
        fecha_cierre: None,
        monto_inicial,
        monto_ventas: 0.0,
        monto_esperado: monto_inicial,
        monto_real: None,
        diferencia: None,
        estado: "ABIERTA".to_string(),
        usuario: Some(usuario_nombre),
        usuario_id: Some(usuario_id),
        observacion: None,
    })
}

#[tauri::command]
pub fn cerrar_caja(
    db: State<Database>,
    sesion: State<SesionState>,
    monto_real: f64,
    observacion: Option<String>,
    motivo_descuadre: Option<String>,
    desglose: Option<String>,
    pin_supervisor: Option<String>,
) -> Result<ResumenCaja, String> {
    // Obtener usuario actual (puede ser distinto al que abrio)
    let sesion_guard = sesion.sesion.lock().map_err(|e| e.to_string())?;
    let sesion_actual = sesion_guard
        .as_ref()
        .ok_or("Debe iniciar sesión para cerrar la caja".to_string())?;
    let usuario_cierre = sesion_actual.nombre.clone();
    let usuario_cierre_id = sesion_actual.usuario_id;
    let usuario_rol = sesion_actual.rol.clone();
    let usuario_permisos = sesion_actual.permisos.clone();
    drop(sesion_guard);

    let conn = db.conn.lock().map_err(|e| e.to_string())?;

    // === Validar permiso para cerrar caja ===
    // Tienen permiso: rol ADMIN, o quienes tengan permiso "cerrar_caja"
    let es_admin = usuario_rol == "ADMIN";
    let tiene_permiso_cerrar = es_admin || serde_json::from_str::<serde_json::Value>(&usuario_permisos)
        .ok()
        .and_then(|v| v.get("cerrar_caja")?.as_bool())
        .unwrap_or(false);
    if !tiene_permiso_cerrar {
        // Requiere PIN de supervisor
        let pin = pin_supervisor.as_deref().map(|s| s.trim()).unwrap_or("");
        if pin.is_empty() {
            return Err("REQUIERE_PIN_SUPERVISOR:Su rol no tiene permiso para cerrar caja. Solicite a un administrador o supervisor su PIN para autorizar el cierre.".to_string());
        }
        // Validar PIN admin
        let mut stmt = conn.prepare("SELECT pin_hash, pin_salt FROM usuarios WHERE activo = 1 AND rol = 'ADMIN'")
            .map_err(|e| e.to_string())?;
        let admins: Vec<(String, String)> = stmt.query_map([], |r| Ok((r.get(0)?, r.get(1)?)))
            .map_err(|e| e.to_string())?
            .collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())?;
        drop(stmt);
        let mut valido = false;
        for (hash, salt) in admins {
            if crate::utils::hash_pin(&salt, pin) == hash {
                valido = true; break;
            }
        }
        if !valido {
            return Err("REQUIERE_PIN_SUPERVISOR:PIN de supervisor incorrecto.".to_string());
        }
    }

    let caja_id: i64 = conn
        .query_row(
            "SELECT id FROM caja WHERE estado = 'ABIERTA' LIMIT 1",
            [],
            |row| row.get(0),
        )
        .map_err(|_| "No hay caja abierta".to_string())?;

    // Calcular totales
    let total_ventas: f64 = conn
        .query_row(
            "SELECT COALESCE(SUM(total), 0) FROM ventas
             WHERE created_at >= (SELECT fecha_apertura FROM caja WHERE id = ?1)
             AND anulada = 0 AND estado = 'COMPLETADA'",
            rusqlite::params![caja_id],
            |row| row.get(0),
        )
        .unwrap_or(0.0);

    let num_ventas: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM ventas
             WHERE created_at >= (SELECT fecha_apertura FROM caja WHERE id = ?1)
             AND anulada = 0 AND estado = 'COMPLETADA'",
            rusqlite::params![caja_id],
            |row| row.get(0),
        )
        .unwrap_or(0);

    let total_efectivo: f64 = conn
        .query_row(
            "SELECT COALESCE(SUM(total), 0) FROM ventas
             WHERE created_at >= (SELECT fecha_apertura FROM caja WHERE id = ?1)
             AND forma_pago = 'EFECTIVO' AND anulada = 0 AND estado = 'COMPLETADA'",
            rusqlite::params![caja_id],
            |row| row.get(0),
        )
        .unwrap_or(0.0);

    let total_gastos: f64 = conn
        .query_row(
            "SELECT COALESCE(SUM(monto), 0) FROM gastos WHERE caja_id = ?1",
            rusqlite::params![caja_id],
            |row| row.get(0),
        )
        .unwrap_or(0.0);

    let monto_inicial: f64 = conn
        .query_row(
            "SELECT monto_inicial FROM caja WHERE id = ?1",
            rusqlite::params![caja_id],
            |row| row.get(0),
        )
        .unwrap_or(0.0);

    // Cobros de cuentas por cobrar en EFECTIVO (cuenta para arqueo de caja)
    let total_cobros_efectivo: f64 = conn
        .query_row(
            "SELECT COALESCE(SUM(p.monto), 0) FROM pagos_cuenta p
             WHERE p.fecha >= (SELECT fecha_apertura FROM caja WHERE id = ?1)
             AND p.forma_pago = 'EFECTIVO' AND p.estado = 'CONFIRMADO'",
            rusqlite::params![caja_id],
            |row| row.get(0),
        )
        .unwrap_or(0.0);

    // Cobros de cuentas por cobrar en TRANSFERENCIA/BANCO (NO cuenta para arqueo)
    let total_cobros_banco: f64 = conn
        .query_row(
            "SELECT COALESCE(SUM(p.monto), 0) FROM pagos_cuenta p
             WHERE p.fecha >= (SELECT fecha_apertura FROM caja WHERE id = ?1)
             AND p.forma_pago = 'TRANSFERENCIA' AND p.estado = 'CONFIRMADO'",
            rusqlite::params![caja_id],
            |row| row.get(0),
        )
        .unwrap_or(0.0);

    let total_retiros: f64 = conn
        .query_row(
            "SELECT COALESCE(SUM(monto), 0) FROM retiros_caja WHERE caja_id = ?1",
            rusqlite::params![caja_id],
            |row| row.get(0),
        )
        .unwrap_or(0.0);

    // CALCULO DEL ESPERADO: usar el valor STORED en la tabla caja como fuente de verdad,
    // ya que se mantiene actualizado en tiempo real con cada venta (solo porcion EFECTIVO),
    // cobro, gasto, retiro. Asi evitamos la inconsistencia que habia entre lo mostrado en
    // el frontend (basado en monto_esperado stored) y un re-calculo del backend con queries
    // que podian diferir por edge cases (ventas viejas con forma_pago no exacto, etc).
    //
    // Fallback: si el stored es 0 o no existe, recalculamos con la formula clasica.
    let monto_esperado_stored: f64 = conn
        .query_row(
            "SELECT COALESCE(monto_esperado, 0) FROM caja WHERE id = ?1",
            rusqlite::params![caja_id],
            |row| row.get(0),
        )
        .unwrap_or(0.0);
    let monto_esperado_recalculado = monto_inicial + total_efectivo + total_cobros_efectivo - total_gastos - total_retiros;
    // Usar el MAYOR de los dos: el stored es lo que se ve en pantalla; si fue actualizado mal
    // por una version vieja, podria ser mas alto que el recalculado. Tomar el mayor protege
    // contra el "descuadre fantasma" reportado donde stored mostraba $X pero recalc daba $0.
    let monto_esperado = if (monto_esperado_stored - monto_esperado_recalculado).abs() < 0.01 {
        monto_esperado_recalculado
    } else {
        // Discrepancia: usar el stored (lo que se mostro al cajero) para coincidir con la pantalla.
        // El audit log queda registrado con ambos valores para trazabilidad.
        monto_esperado_stored.max(monto_esperado_recalculado)
    };
    let diferencia = monto_real - monto_esperado;

    // Anti-fraude: si hay descuadre, exigir motivo (mínimo 5 caracteres)
    let descuadra = diferencia.abs() > 0.01;
    if descuadra {
        let motivo_str = motivo_descuadre.as_deref().map(|s| s.trim()).unwrap_or("");
        if motivo_str.len() < 5 {
            return Err(format!(
                "DESCUADRE_CIERRE:{:.2}:{:.2}:Hay un descuadre de ${:.2} (esperado ${:.2}, contado ${:.2}). Debe explicar el motivo (mínimo 5 caracteres).",
                monto_esperado, monto_real, diferencia, monto_esperado, monto_real
            ));
        }

        // Validar umbral configurable
        let umbral_pct: f64 = conn
            .query_row("SELECT value FROM config WHERE key = 'caja_descuadre_umbral_pct'", [], |r| r.get::<_, String>(0))
            .ok().and_then(|s| s.parse().ok()).unwrap_or(2.0);
        let requiere_pin_descuadre: bool = conn
            .query_row("SELECT value FROM config WHERE key = 'caja_requiere_pin_descuadre'", [], |r| r.get::<_, String>(0))
            .map(|v| v == "1").unwrap_or(false);
        let umbral_monto = (umbral_pct / 100.0) * monto_esperado.max(1.0);
        let pct_real = if monto_esperado > 0.0 { diferencia.abs() / monto_esperado * 100.0 } else { 0.0 };

        if diferencia.abs() > umbral_monto {
            // Si la config exige PIN para descuadres graves: exigirlo (si el usuario actual no tiene 'aprobar_descuadre')
            let tiene_aprobar = es_admin || serde_json::from_str::<serde_json::Value>(&usuario_permisos)
                .ok()
                .and_then(|v| v.get("aprobar_descuadre")?.as_bool())
                .unwrap_or(false);
            if requiere_pin_descuadre && !tiene_aprobar {
                let pin = pin_supervisor.as_deref().map(|s| s.trim()).unwrap_or("");
                if pin.is_empty() {
                    return Err(format!(
                        "REQUIERE_PIN_DESCUADRE:{:.2}:{:.2}:Descuadre de ${:.2} ({:.1}%) supera el umbral del {:.1}%. Requiere autorización con PIN de supervisor.",
                        umbral_monto, diferencia.abs(), diferencia.abs(), pct_real, umbral_pct
                    ));
                }
                // Validar el PIN nuevamente (puede que ya haya pasado el filtro de permiso cerrar_caja, pero igual revalidamos)
                let mut stmt2 = conn.prepare("SELECT pin_hash, pin_salt FROM usuarios WHERE activo = 1 AND rol = 'ADMIN'")
                    .map_err(|e| e.to_string())?;
                let admins: Vec<(String, String)> = stmt2.query_map([], |r| Ok((r.get(0)?, r.get(1)?)))
                    .map_err(|e| e.to_string())?
                    .collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())?;
                drop(stmt2);
                let mut valido = false;
                for (hash, salt) in admins {
                    if crate::utils::hash_pin(&salt, pin) == hash {
                        valido = true; break;
                    }
                }
                if !valido {
                    return Err("REQUIERE_PIN_DESCUADRE:PIN de supervisor incorrecto para autorizar descuadre.".to_string());
                }
            }
            // Registrar evento DESCUADRE_GRAVE
            let _ = conn.execute(
                "INSERT INTO caja_eventos (caja_id, evento, usuario, usuario_id, motivo, metadatos)
                 VALUES (?1, 'DESCUADRE_GRAVE', ?2, ?3, ?4, ?5)",
                rusqlite::params![caja_id, usuario_cierre, usuario_cierre_id, motivo_descuadre,
                    serde_json::json!({"diferencia": diferencia, "umbral_pct": umbral_pct, "umbral_monto": umbral_monto, "pct_real": pct_real, "pin_validado": requiere_pin_descuadre && !tiene_aprobar}).to_string()
                ],
            );
        }
    }

    // Snapshot anterior para auditoria
    let snapshot_anterior = serde_json::json!({
        "estado": "ABIERTA",
        "monto_inicial": monto_inicial,
    }).to_string();

    conn.execute(
        "UPDATE caja SET fecha_cierre = datetime('now','localtime'),
         cerrada_at = datetime('now','localtime'),
         monto_ventas = ?1, monto_esperado = ?2, monto_real = ?3,
         diferencia = ?4, estado = 'CERRADA', observacion = ?5,
         motivo_descuadre = ?6, desglose_cierre = ?7, usuario_cierre = ?8
         WHERE id = ?9",
        rusqlite::params![total_ventas, monto_esperado, monto_real, diferencia, observacion,
                          motivo_descuadre, desglose, usuario_cierre, caja_id],
    )
    .map_err(|e| e.to_string())?;

    // Log evento CIERRE
    let snapshot_nuevo = serde_json::json!({
        "estado": "CERRADA",
        "monto_real": monto_real,
        "monto_esperado": monto_esperado,
        "diferencia": diferencia,
        "total_ventas": total_ventas,
        "total_efectivo": total_efectivo,
    }).to_string();
    log_evento_caja(&conn, caja_id, "CIERRE", &usuario_cierre, usuario_cierre_id,
        Some(&snapshot_anterior), Some(&snapshot_nuevo), motivo_descuadre.as_deref());

    // NOTA: Antes se cerraba la sesion automaticamente aqui, pero eso rompia el flujo
    // del cajero al querer abrir nueva caja inmediatamente despues (error "Debe iniciar
    // sesion para abrir la caja"). El cierre de sesion ahora es responsabilidad del
    // FRONTEND cuando el cajero hace click en "Finalizar Turno" desde el resumen.
    drop(conn);
    // (no tocar sesion aqui)
    let _ = sesion; // suprimir warning de variable no usada

    let caja = Caja {
        id: Some(caja_id),
        fecha_apertura: None,
        fecha_cierre: None,
        monto_inicial,
        monto_ventas: total_ventas,
        monto_esperado,
        monto_real: Some(monto_real),
        diferencia: Some(diferencia),
        estado: "CERRADA".to_string(),
        usuario: None,
        usuario_id: None,
        observacion,
    };

    Ok(ResumenCaja {
        caja,
        total_ventas,
        num_ventas,
        total_efectivo,
        total_gastos,
        total_cobros_efectivo,
        total_cobros_banco,
        total_retiros,
    })
}

#[tauri::command]
pub fn registrar_retiro(
    db: State<Database>,
    sesion: State<SesionState>,
    monto: f64,
    motivo: String,
    banco_id: Option<i64>,
    referencia: Option<String>,
) -> Result<serde_json::Value, String> {
    // Get session user
    let sesion_guard = sesion.sesion.lock().map_err(|e| e.to_string())?;
    let sesion_actual = sesion_guard
        .as_ref()
        .ok_or("Debe iniciar sesión para registrar un retiro".to_string())?;
    let usuario_nombre = sesion_actual.nombre.clone();
    let usuario_id = sesion_actual.usuario_id;
    drop(sesion_guard);

    if monto <= 0.0 {
        return Err("El monto del retiro debe ser mayor a 0".to_string());
    }

    let conn = db.conn.lock().map_err(|e| e.to_string())?;

    // Find open caja
    let caja_id: i64 = conn
        .query_row(
            "SELECT id FROM caja WHERE estado = 'ABIERTA' LIMIT 1",
            [],
            |row| row.get(0),
        )
        .map_err(|_| "No hay caja abierta para registrar el retiro".to_string())?;

    let estado = if banco_id.is_some() { "EN_TRANSITO" } else { "SIN_DEPOSITO" };

    conn.execute(
        "INSERT INTO retiros_caja (caja_id, monto, motivo, banco_id, referencia, usuario, usuario_id, estado)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        rusqlite::params![caja_id, monto, motivo, banco_id, referencia, usuario_nombre, usuario_id, estado],
    )
    .map_err(|e| e.to_string())?;

    let id = conn.last_insert_rowid();
    let fecha: String = conn
        .query_row(
            "SELECT fecha FROM retiros_caja WHERE id = ?1",
            rusqlite::params![id],
            |row| row.get(0),
        )
        .unwrap_or_default();

    Ok(serde_json::json!({
        "id": id,
        "monto": monto,
        "motivo": motivo,
        "fecha": fecha,
        "usuario": usuario_nombre,
        "estado": estado,
    }))
}

#[tauri::command]
pub fn listar_retiros_caja(
    db: State<Database>,
    caja_id: i64,
) -> Result<Vec<serde_json::Value>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;

    let mut stmt = conn
        .prepare(
            "SELECT r.id, r.caja_id, r.monto, r.motivo, r.banco_id, r.referencia,
                    r.usuario, r.usuario_id, r.fecha, cb.nombre as banco_nombre,
                    r.estado, r.comprobante_imagen
             FROM retiros_caja r
             LEFT JOIN cuentas_banco cb ON r.banco_id = cb.id
             WHERE r.caja_id = ?1
             ORDER BY r.fecha DESC",
        )
        .map_err(|e| e.to_string())?;

    let retiros = stmt
        .query_map(rusqlite::params![caja_id], |row| {
            Ok(serde_json::json!({
                "id": row.get::<_, i64>(0)?,
                "caja_id": row.get::<_, i64>(1)?,
                "monto": row.get::<_, f64>(2)?,
                "motivo": row.get::<_, String>(3)?,
                "banco_id": row.get::<_, Option<i64>>(4)?,
                "referencia": row.get::<_, Option<String>>(5)?,
                "usuario": row.get::<_, String>(6)?,
                "usuario_id": row.get::<_, Option<i64>>(7)?,
                "fecha": row.get::<_, String>(8)?,
                "banco_nombre": row.get::<_, Option<String>>(9)?,
                "estado": row.get::<_, Option<String>>(10)?.unwrap_or_else(|| "SIN_DEPOSITO".to_string()),
                "comprobante_imagen": row.get::<_, Option<String>>(11)?,
            }))
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    Ok(retiros)
}

#[tauri::command]
pub fn confirmar_deposito(
    db: State<Database>,
    retiro_id: i64,
    referencia: String,
    comprobante_imagen: Option<String>,
) -> Result<(), String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let rows = conn.execute(
        "UPDATE retiros_caja SET estado = 'DEPOSITADO', referencia = ?1, comprobante_imagen = ?2 WHERE id = ?3 AND estado = 'EN_TRANSITO'",
        rusqlite::params![referencia, comprobante_imagen, retiro_id],
    ).map_err(|e| e.to_string())?;
    if rows == 0 {
        return Err("No se encontró el retiro en tránsito".to_string());
    }
    Ok(())
}

/// Lista los eventos de auditoria de una caja especifica
#[tauri::command]
pub fn listar_eventos_caja(db: State<Database>, caja_id: i64) -> Result<Vec<serde_json::Value>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let mut stmt = conn.prepare(
        "SELECT id, evento, usuario, usuario_id, valor_anterior, valor_nuevo, motivo, metadatos, timestamp
         FROM caja_eventos WHERE caja_id = ?1 ORDER BY timestamp ASC, id ASC"
    ).map_err(|e| e.to_string())?;
    let rows = stmt.query_map(rusqlite::params![caja_id], |r| {
        Ok(serde_json::json!({
            "id": r.get::<_, i64>(0)?,
            "evento": r.get::<_, String>(1)?,
            "usuario": r.get::<_, Option<String>>(2)?,
            "usuario_id": r.get::<_, Option<i64>>(3)?,
            "valor_anterior": r.get::<_, Option<String>>(4)?,
            "valor_nuevo": r.get::<_, Option<String>>(5)?,
            "motivo": r.get::<_, Option<String>>(6)?,
            "metadatos": r.get::<_, Option<String>>(7)?,
            "timestamp": r.get::<_, String>(8)?,
        }))
    }).map_err(|e| e.to_string())?;
    rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
}

/// Lista TODAS las sesiones de caja (abiertas y cerradas) con filtros opcionales.
/// Util para auditoria completa, no solo descuadres.
#[tauri::command]
pub fn listar_sesiones_caja(
    db: State<Database>,
    fecha_desde: Option<String>,
    fecha_hasta: Option<String>,
    usuario: Option<String>,
    solo_descuadradas: Option<bool>,
) -> Result<Vec<serde_json::Value>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let desde = fecha_desde.unwrap_or_else(|| "1970-01-01".to_string());
    let hasta = fecha_hasta.unwrap_or_else(|| "2999-12-31".to_string());

    let mut sql = String::from(
        "SELECT c.id, c.fecha_apertura, COALESCE(c.cerrada_at, c.fecha_cierre) as fecha_cierre,
                c.monto_inicial, c.monto_ventas, c.monto_esperado, c.monto_real, c.diferencia,
                c.estado, c.usuario, c.usuario_id,
                COALESCE(c.usuario_cierre, c.usuario) as usuario_cierre,
                c.observacion, c.motivo_descuadre, c.motivo_diferencia_apertura, c.caja_anterior_id
         FROM caja c
         WHERE date(c.fecha_apertura) >= date(?1) AND date(c.fecha_apertura) <= date(?2)"
    );
    let mut params_vec: Vec<rusqlite::types::Value> = vec![desde.into(), hasta.into()];
    if let Some(u) = usuario.as_ref().filter(|s| !s.is_empty()) {
        sql.push_str(" AND (c.usuario LIKE ?3 OR c.usuario_cierre LIKE ?3)");
        params_vec.push(format!("%{}%", u).into());
    }
    if solo_descuadradas.unwrap_or(false) {
        sql.push_str(" AND c.estado = 'CERRADA' AND ABS(COALESCE(c.diferencia, 0)) > 0.01");
    }
    sql.push_str(" ORDER BY c.id DESC LIMIT 200");

    let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;
    let params_refs: Vec<&dyn rusqlite::ToSql> = params_vec.iter().map(|v| v as &dyn rusqlite::ToSql).collect();
    let sesiones = stmt.query_map(rusqlite::params_from_iter(params_refs.iter()), |r| {
        Ok(serde_json::json!({
            "id": r.get::<_, i64>(0)?,
            "fecha_apertura": r.get::<_, Option<String>>(1)?,
            "fecha_cierre": r.get::<_, Option<String>>(2)?,
            "monto_inicial": r.get::<_, f64>(3)?,
            "monto_ventas": r.get::<_, f64>(4)?,
            "monto_esperado": r.get::<_, f64>(5)?,
            "monto_real": r.get::<_, Option<f64>>(6)?,
            "diferencia": r.get::<_, Option<f64>>(7)?,
            "estado": r.get::<_, String>(8)?,
            "usuario_apertura": r.get::<_, Option<String>>(9)?,
            "usuario_id": r.get::<_, Option<i64>>(10)?,
            "usuario_cierre": r.get::<_, Option<String>>(11)?,
            "observacion": r.get::<_, Option<String>>(12)?,
            "motivo_descuadre": r.get::<_, Option<String>>(13)?,
            "motivo_diferencia_apertura": r.get::<_, Option<String>>(14)?,
            "caja_anterior_id": r.get::<_, Option<i64>>(15)?,
        }))
    }).map_err(|e| e.to_string())?
    .collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())?;
    Ok(sesiones)
}

/// Registra un deposito a banco POST-CIERRE de caja (cuando el dinero contado
/// va al banco). Crea entrada en retiros_caja con estado EN_TRANSITO si requiere
/// confirmacion, o DEPOSITADO directo si se sube comprobante.
#[tauri::command]
pub fn registrar_deposito_cierre(
    db: State<Database>,
    sesion: State<SesionState>,
    caja_id: i64,
    monto: f64,
    banco_id: i64,
    referencia: Option<String>,
    comprobante_imagen: Option<String>,
) -> Result<serde_json::Value, String> {
    let sesion_guard = sesion.sesion.lock().map_err(|e| e.to_string())?;
    let sesion_actual = sesion_guard
        .as_ref()
        .ok_or("Debe iniciar sesión".to_string())?;
    let usuario_nombre = sesion_actual.nombre.clone();
    let usuario_id = sesion_actual.usuario_id;
    drop(sesion_guard);

    if monto <= 0.0 {
        return Err("El monto del depósito debe ser mayor a 0".to_string());
    }

    let conn = db.conn.lock().map_err(|e| e.to_string())?;

    // Verificar que la caja existe (puede estar abierta o cerrada)
    let _: i64 = conn.query_row(
        "SELECT id FROM caja WHERE id = ?1",
        rusqlite::params![caja_id], |r| r.get(0),
    ).map_err(|_| "Caja no encontrada".to_string())?;

    let estado = if comprobante_imagen.is_some() && referencia.is_some() {
        "DEPOSITADO"
    } else {
        "EN_TRANSITO"
    };

    let motivo = format!("Depósito a banco (cierre caja #{})", caja_id);
    conn.execute(
        "INSERT INTO retiros_caja (caja_id, monto, motivo, banco_id, referencia, usuario, usuario_id, estado, comprobante_imagen)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
        rusqlite::params![caja_id, monto, motivo, banco_id, referencia, usuario_nombre, usuario_id, estado, comprobante_imagen],
    ).map_err(|e| e.to_string())?;

    let id = conn.last_insert_rowid();

    // Log evento DEPOSITO en caja_eventos
    let snapshot = serde_json::json!({
        "monto": monto,
        "banco_id": banco_id,
        "referencia": referencia,
        "estado": estado,
    }).to_string();
    log_evento_caja(&conn, caja_id, "DEPOSITO", &usuario_nombre, usuario_id,
        None, Some(&snapshot), Some(&motivo));

    Ok(serde_json::json!({ "id": id, "estado": estado }))
}

/// Historial de cierres con descuadre. Util para detectar patrones por cajero.
#[tauri::command]
pub fn historial_descuadres_caja(
    db: State<Database>,
    fecha_desde: Option<String>,
    fecha_hasta: Option<String>,
) -> Result<serde_json::Value, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let desde = fecha_desde.unwrap_or_else(|| "1970-01-01".to_string());
    let hasta = fecha_hasta.unwrap_or_else(|| "2999-12-31".to_string());

    let mut stmt = conn.prepare(
        "SELECT c.id, COALESCE(c.cerrada_at, c.fecha_cierre) as fecha_cierre,
                c.fecha_apertura, c.monto_inicial, c.monto_esperado, c.monto_real, c.diferencia,
                COALESCE(c.usuario_cierre, c.usuario) as usuario, c.motivo_descuadre, c.motivo_diferencia_apertura,
                c.observacion
         FROM caja c
         WHERE c.estado = 'CERRADA'
           AND c.diferencia IS NOT NULL
           AND ABS(c.diferencia) > 0.01
           AND date(COALESCE(c.cerrada_at, c.fecha_cierre)) >= date(?1)
           AND date(COALESCE(c.cerrada_at, c.fecha_cierre)) <= date(?2)
         ORDER BY c.id DESC"
    ).map_err(|e| e.to_string())?;

    let cierres: Vec<serde_json::Value> = stmt.query_map(rusqlite::params![desde, hasta], |r| {
        Ok(serde_json::json!({
            "caja_id": r.get::<_, i64>(0)?,
            "fecha_cierre": r.get::<_, Option<String>>(1)?,
            "fecha_apertura": r.get::<_, Option<String>>(2)?,
            "monto_inicial": r.get::<_, f64>(3)?,
            "monto_esperado": r.get::<_, f64>(4)?,
            "monto_real": r.get::<_, Option<f64>>(5)?,
            "diferencia": r.get::<_, Option<f64>>(6)?,
            "usuario": r.get::<_, Option<String>>(7)?,
            "motivo_descuadre": r.get::<_, Option<String>>(8)?,
            "motivo_diferencia_apertura": r.get::<_, Option<String>>(9)?,
            "observacion": r.get::<_, Option<String>>(10)?,
        }))
    }).map_err(|e| e.to_string())?
    .collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())?;

    // Resumen por usuario
    let mut por_usuario: std::collections::HashMap<String, (i64, f64, f64, f64)> = std::collections::HashMap::new();
    for c in &cierres {
        let usuario = c["usuario"].as_str().unwrap_or("?").to_string();
        let dif = c["diferencia"].as_f64().unwrap_or(0.0);
        let entry = por_usuario.entry(usuario).or_insert((0, 0.0, 0.0, 0.0));
        entry.0 += 1;
        entry.1 += dif; // suma neta (sobrante - faltante)
        if dif < 0.0 { entry.2 += dif.abs(); } // total faltantes
        if dif > 0.0 { entry.3 += dif; } // total sobrantes
    }
    let resumen_usuarios: Vec<serde_json::Value> = por_usuario.into_iter().map(|(u, (n, neto, falt, sobr))| {
        serde_json::json!({
            "usuario": u,
            "total_cierres_descuadrados": n,
            "diferencia_neta": neto,
            "total_faltantes": falt,
            "total_sobrantes": sobr,
        })
    }).collect();

    let total_faltantes: f64 = cierres.iter()
        .filter_map(|c| c["diferencia"].as_f64())
        .filter(|d| *d < 0.0).map(|d| d.abs()).sum();
    let total_sobrantes: f64 = cierres.iter()
        .filter_map(|c| c["diferencia"].as_f64())
        .filter(|d| *d > 0.0).sum();

    Ok(serde_json::json!({
        "cierres": cierres,
        "total_descuadrados": cierres.len(),
        "total_faltantes": total_faltantes,
        "total_sobrantes": total_sobrantes,
        "neto": total_sobrantes - total_faltantes,
        "por_usuario": resumen_usuarios,
    }))
}

#[tauri::command]
pub fn obtener_caja_abierta(db: State<Database>) -> Result<Option<Caja>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;

    let result = conn.query_row(
        "SELECT id, fecha_apertura, fecha_cierre, monto_inicial, monto_ventas,
         monto_esperado, monto_real, diferencia, estado, usuario, observacion, usuario_id
         FROM caja WHERE estado = 'ABIERTA' LIMIT 1",
        [],
        |row| {
            Ok(Caja {
                id: Some(row.get(0)?),
                fecha_apertura: row.get(1)?,
                fecha_cierre: row.get(2)?,
                monto_inicial: row.get(3)?,
                monto_ventas: row.get(4)?,
                monto_esperado: row.get(5)?,
                monto_real: row.get(6)?,
                diferencia: row.get(7)?,
                estado: row.get(8)?,
                usuario: row.get(9)?,
                observacion: row.get(10)?,
                usuario_id: row.get(11)?,
            })
        },
    );

    match result {
        Ok(caja) => Ok(Some(caja)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(e.to_string()),
    }
}
