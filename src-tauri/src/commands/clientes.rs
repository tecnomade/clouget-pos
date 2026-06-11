use crate::db::{Database, SesionState};
use crate::models::Cliente;
use serde::Deserialize;
use tauri::State;

/// Elimina un cliente. Reglas:
/// - Requiere ADMIN o permiso 'eliminar_clientes'.
/// - Consumidor Final (id=1) nunca se elimina.
/// - Si tiene crédito PENDIENTE se bloquea (prefijo BLOCK_DELETE_CREDITO:saldo).
/// - DELETE físico; si tiene referencias (ventas, notas débito, cxc pagadas)
///   hace soft delete liberando la identificación para poder re-crearlo.
#[tauri::command]
pub fn eliminar_cliente(db: State<Database>, sesion: State<SesionState>, id: i64) -> Result<(), String> {
    {
        let sesion_guard = sesion.sesion.lock().map_err(|e| e.to_string())?;
        if let Some(s) = sesion_guard.as_ref() {
            if s.rol != "ADMIN" {
                let tiene = serde_json::from_str::<serde_json::Value>(&s.permisos)
                    .ok()
                    .and_then(|v| v.get("eliminar_clientes")?.as_bool())
                    .unwrap_or(false);
                if !tiene {
                    return Err("No tiene permiso para eliminar clientes.".to_string());
                }
            }
        }
    }

    if id == 1 {
        return Err("No se puede eliminar Consumidor Final.".to_string());
    }

    let conn = db.conn.lock().map_err(|e| e.to_string())?;

    let saldo_pendiente: f64 = conn
        .query_row(
            "SELECT COALESCE(SUM(saldo), 0) FROM cuentas_por_cobrar
             WHERE cliente_id = ?1 AND estado = 'PENDIENTE'",
            rusqlite::params![id],
            |r| r.get(0),
        )
        .unwrap_or(0.0);
    if saldo_pendiente > 0.009 {
        return Err(format!("BLOCK_DELETE_CREDITO:{:.2}", saldo_pendiente));
    }

    if conn
        .execute("DELETE FROM clientes WHERE id = ?1", rusqlite::params![id])
        .is_ok()
    {
        return Ok(());
    }

    // Tiene referencias → soft delete liberando identificacion (UNIQUE)
    conn.execute(
        "UPDATE clientes
         SET activo = 0,
             identificacion = COALESCE(identificacion, '') || '_DEL' || id
         WHERE id = ?1",
        rusqlite::params![id],
    )
    .map_err(|e| format!("No se pudo eliminar cliente: {}", e))?;

    Ok(())
}

#[tauri::command]
pub fn crear_cliente(db: State<Database>, cliente: Cliente) -> Result<i64, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;

    // v2.5.39: si no viene categoria_id, asignar la default. Si no vienen campos override,
    // heredar valores de la categoria (permite_credito, dias_credito, limite_credito, descuento_pct).
    let (cat_id, def_pc, def_dc, def_lc, def_dpct) = resolver_categoria_defaults(&conn, cliente.categoria_id)?;

    conn.execute(
        "INSERT INTO clientes (tipo_identificacion, identificacion, nombre, direccion, telefono, email, activo, lista_precio_id,
                               categoria_id, permite_credito, dias_credito, limite_credito, descuento_pct)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
        rusqlite::params![
            cliente.tipo_identificacion,
            cliente.identificacion,
            cliente.nombre,
            cliente.direccion,
            cliente.telefono,
            cliente.email,
            cliente.activo as i32,
            cliente.lista_precio_id,
            cat_id,
            cliente.permite_credito.map(|b| b as i32).unwrap_or(def_pc),
            cliente.dias_credito.unwrap_or(def_dc),
            cliente.limite_credito.unwrap_or(def_lc),
            cliente.descuento_pct.unwrap_or(def_dpct),
        ],
    )
    .map_err(|e| e.to_string())?;

    Ok(conn.last_insert_rowid())
}

#[tauri::command]
pub fn actualizar_cliente(db: State<Database>, cliente: Cliente) -> Result<(), String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let id = cliente.id.ok_or("ID requerido para actualizar")?;

    // v2.5.39: si cambio la categoria, re-heredar defaults SOLO para los campos no explicitamente overrideados.
    let (cat_id, def_pc, def_dc, def_lc, def_dpct) = resolver_categoria_defaults(&conn, cliente.categoria_id)?;

    conn.execute(
        "UPDATE clientes SET tipo_identificacion=?1, identificacion=?2, nombre=?3,
         direccion=?4, telefono=?5, email=?6, activo=?7, lista_precio_id=?8,
         categoria_id=?9, permite_credito=?10, dias_credito=?11, limite_credito=?12, descuento_pct=?13,
         updated_at=datetime('now','localtime')
         WHERE id=?14",
        rusqlite::params![
            cliente.tipo_identificacion,
            cliente.identificacion,
            cliente.nombre,
            cliente.direccion,
            cliente.telefono,
            cliente.email,
            cliente.activo as i32,
            cliente.lista_precio_id,
            cat_id,
            cliente.permite_credito.map(|b| b as i32).unwrap_or(def_pc),
            cliente.dias_credito.unwrap_or(def_dc),
            cliente.limite_credito.unwrap_or(def_lc),
            cliente.descuento_pct.unwrap_or(def_dpct),
            id,
        ],
    )
    .map_err(|e| e.to_string())?;

    Ok(())
}

/// Helper: dada una categoria_id (o None → usar default), retorna
/// (categoria_id_resuelto, permite_credito, dias_credito, limite_credito, descuento_pct)
fn resolver_categoria_defaults(
    conn: &rusqlite::Connection,
    categoria_id: Option<i64>,
) -> Result<(Option<i64>, i32, i64, f64, f64), String> {
    let cat_id = match categoria_id {
        Some(cid) => Some(cid),
        None => conn.query_row(
            "SELECT id FROM categorias_clientes WHERE es_default = 1 AND activo = 1 LIMIT 1",
            [], |r| r.get::<_, i64>(0),
        ).ok(),
    };
    if let Some(cid) = cat_id {
        let row: Result<(i32, i64, f64, f64), _> = conn.query_row(
            "SELECT permite_credito, dias_credito, limite_credito, descuento_pct
             FROM categorias_clientes WHERE id = ?1",
            rusqlite::params![cid],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
        );
        if let Ok((pc, dc, lc, dpct)) = row {
            return Ok((Some(cid), pc, dc, lc, dpct));
        }
    }
    Ok((cat_id, 0, 0, 0.0, 0.0))
}

#[tauri::command]
pub fn buscar_clientes(db: State<Database>, termino: String) -> Result<Vec<Cliente>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let busqueda = format!("%{}%", termino);

    let mut stmt = conn
        .prepare(
            "SELECT c.id, c.tipo_identificacion, c.identificacion, c.nombre, c.direccion,
                    c.telefono, c.email, c.activo, c.lista_precio_id, lp.nombre,
                    c.categoria_id, COALESCE(c.permite_credito, 0), COALESCE(c.dias_credito, 0),
                    COALESCE(c.limite_credito, 0), COALESCE(c.descuento_pct, 0)
             FROM clientes c
             LEFT JOIN listas_precios lp ON lp.id = c.lista_precio_id
             WHERE c.activo = 1
             AND (c.nombre LIKE ?1 OR c.identificacion LIKE ?1)
             ORDER BY c.nombre LIMIT 30",
        )
        .map_err(|e| e.to_string())?;

    let clientes = stmt
        .query_map(rusqlite::params![busqueda], |row| {
            Ok(Cliente {
                id: Some(row.get(0)?),
                tipo_identificacion: row.get(1)?,
                identificacion: row.get(2)?,
                nombre: row.get(3)?,
                direccion: row.get(4)?,
                telefono: row.get(5)?,
                email: row.get(6)?,
                activo: row.get::<_, i32>(7)? != 0,
                lista_precio_id: row.get(8)?,
                lista_precio_nombre: row.get(9)?,
                categoria_id: row.get(10).ok(),
                permite_credito: Some(row.get::<_, i32>(11)? != 0),
                dias_credito: row.get(12).ok(),
                limite_credito: row.get(13).ok(),
                descuento_pct: row.get(14).ok(),
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    Ok(clientes)
}

#[tauri::command]
pub fn listar_clientes(db: State<Database>) -> Result<Vec<Cliente>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;

    let mut stmt = conn
        .prepare(
            "SELECT c.id, c.tipo_identificacion, c.identificacion, c.nombre, c.direccion,
                    c.telefono, c.email, c.activo, c.lista_precio_id, lp.nombre,
                    c.categoria_id, COALESCE(c.permite_credito, 0), COALESCE(c.dias_credito, 0),
                    COALESCE(c.limite_credito, 0), COALESCE(c.descuento_pct, 0)
             FROM clientes c
             LEFT JOIN listas_precios lp ON lp.id = c.lista_precio_id
             WHERE c.activo = 1 ORDER BY c.nombre",
        )
        .map_err(|e| e.to_string())?;

    let clientes = stmt
        .query_map([], |row| {
            Ok(Cliente {
                id: Some(row.get(0)?),
                tipo_identificacion: row.get(1)?,
                identificacion: row.get(2)?,
                nombre: row.get(3)?,
                direccion: row.get(4)?,
                telefono: row.get(5)?,
                email: row.get(6)?,
                activo: row.get::<_, i32>(7)? != 0,
                lista_precio_id: row.get(8)?,
                lista_precio_nombre: row.get(9)?,
                categoria_id: row.get(10).ok(),
                permite_credito: Some(row.get::<_, i32>(11)? != 0),
                dias_credito: row.get(12).ok(),
                limite_credito: row.get(13).ok(),
                descuento_pct: row.get(14).ok(),
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    Ok(clientes)
}

// ── Structs para deserializar respuestas de APIs externas ──

#[derive(Deserialize, Debug)]
#[allow(non_snake_case)]
struct SriContribuyente {
    razonSocial: Option<String>,
    #[allow(dead_code)]
    nombreComercial: Option<String>,
    #[allow(dead_code)]
    estadoContribuyente: Option<String>,
}

#[derive(Deserialize, Debug)]
#[allow(non_snake_case)]
struct SriEstablecimiento {
    numeroEstablecimiento: Option<String>,
    calle: Option<String>,
    numero: Option<String>,
    interseccion: Option<String>,
    #[allow(dead_code)]
    barrio: Option<String>,
    #[allow(dead_code)]
    ciudadela: Option<String>,
    descripcionCanton: Option<String>,
    descripcionProvincia: Option<String>,
    direccionCompleta: Option<String>,
}

#[derive(Deserialize, Debug)]
#[allow(non_snake_case)]
struct SecapPersona {
    respuesta: Option<i32>,
    nombreCompleto: Option<String>,
    #[allow(dead_code)]
    nombres: Option<String>,
    #[allow(dead_code)]
    apellidos: Option<String>,
}

const SRI_BASE: &str = "https://srienlinea.sri.gob.ec/sri-catastro-sujeto-servicio-internet/rest";

#[tauri::command]
pub async fn consultar_identificacion(
    db: State<'_, Database>,
    identificacion: String,
) -> Result<Cliente, String> {
    let identificacion = identificacion.trim().to_string();

    // Validar formato: solo dígitos, 10 o 13 caracteres
    if !identificacion.chars().all(|c| c.is_ascii_digit())
        || (identificacion.len() != 10 && identificacion.len() != 13)
    {
        return Err("Ingrese una cédula (10 dígitos) o RUC (13 dígitos) válido".to_string());
    }

    // ── 1. Buscar primero en la base local ──
    {
        let conn = db.conn.lock().map_err(|e| e.to_string())?;
        let mut stmt = conn
            .prepare(
                "SELECT c.id, c.tipo_identificacion, c.identificacion, c.nombre, c.direccion,
                        c.telefono, c.email, c.activo, c.lista_precio_id, lp.nombre
                 FROM clientes c
                 LEFT JOIN listas_precios lp ON lp.id = c.lista_precio_id
                 WHERE c.identificacion = ?1 AND c.activo = 1
                 LIMIT 1",
            )
            .map_err(|e| e.to_string())?;

        let resultado: Vec<Cliente> = stmt
            .query_map(rusqlite::params![identificacion], |row| {
                Ok(Cliente {
                    id: Some(row.get(0)?),
                    tipo_identificacion: row.get(1)?,
                    identificacion: row.get(2)?,
                    nombre: row.get(3)?,
                    direccion: row.get(4)?,
                    telefono: row.get(5)?,
                    email: row.get(6)?,
                    activo: row.get::<_, i32>(7)? != 0,
                    lista_precio_id: row.get(8)?,
                    lista_precio_nombre: row.get(9)?,
                    categoria_id: None,
                    permite_credito: None,
                    dias_credito: None,
                    limite_credito: None,
                    descuento_pct: None,
                })
            })
            .map_err(|e| e.to_string())?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())?;

        if let Some(cliente) = resultado.into_iter().next() {
            return Ok(cliente);
        }
    }
    // MutexGuard se libera aquí

    // ── 2. Consultar APIs externas ──
    let es_cedula = identificacion.len() == 10;
    let ruc = if es_cedula {
        format!("{}001", identificacion)
    } else {
        identificacion.clone()
    };

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|e| format!("Error creando cliente HTTP: {}", e))?;

    let mut nombre: Option<String> = None;
    let mut direccion: Option<String> = None;

    // ── Paso 2a: SRI Contribuyente (razón social) ──
    let url_contrib = format!(
        "{}/ConsolidadoContribuyente/obtenerPorNumerosRuc?=&ruc={}",
        SRI_BASE, ruc
    );
    if let Ok(resp) = client.get(&url_contrib).send().await {
        if resp.status().is_success() {
            if let Ok(text) = resp.text().await {
                if let Ok(data) = serde_json::from_str::<SriContribuyente>(&text) {
                    nombre = data.razonSocial;
                } else if let Ok(arr) = serde_json::from_str::<Vec<SriContribuyente>>(&text) {
                    if let Some(first) = arr.into_iter().next() {
                        nombre = first.razonSocial;
                    }
                }
            }
        }
    }

    // ── Paso 2b: SRI Establecimiento (dirección) ──
    let url_estab = format!(
        "{}/Establecimiento/consultarPorNumeroRuc?numeroRuc={}",
        SRI_BASE, ruc
    );
    if let Ok(resp) = client.get(&url_estab).send().await {
        if resp.status().is_success() {
            if let Ok(establecimientos) = resp.json::<Vec<SriEstablecimiento>>().await {
                let matriz = establecimientos
                    .iter()
                    .find(|e| e.numeroEstablecimiento.as_deref() == Some("001"))
                    .or_else(|| establecimientos.first());

                if let Some(est) = matriz {
                    let partes: Vec<&str> = [
                        est.calle.as_deref(),
                        est.numero.as_deref(),
                        est.interseccion.as_deref(),
                        est.descripcionCanton.as_deref(),
                        est.descripcionProvincia.as_deref(),
                    ]
                    .iter()
                    .filter_map(|p| *p)
                    .filter(|p| !p.is_empty())
                    .collect();

                    if !partes.is_empty() {
                        direccion = Some(partes.join(" "));
                    } else {
                        direccion = est.direccionCompleta.clone();
                    }
                }
            }
        }
    }

    // ── Paso 2c: SECAP / Registro Civil (fallback para cédulas sin RUC) ──
    if es_cedula && nombre.is_none() {
        let secap_url =
            "https://si.secap.gob.ec/sisecap/logeo_web/json/busca_persona_registro_civil.php";
        let body = format!("documento={}&tipo=1", identificacion);

        if let Ok(resp) = client
            .post(secap_url)
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body(body)
            .send()
            .await
        {
            if resp.status().is_success() {
                if let Ok(data) = resp.json::<SecapPersona>().await {
                    if data.respuesta == Some(1) {
                        nombre = data.nombreCompleto;
                    }
                }
            }
        }
    }

    // ── 3. Si no se encontró nada, retornar error ──
    let nombre = match nombre {
        Some(n) if !n.trim().is_empty() => n.trim().to_uppercase(),
        _ => {
            return Err("No se encontró información para esta identificación".to_string());
        }
    };

    // ── 4. Crear cliente en la base local ──
    let tipo_id = if es_cedula {
        "CEDULA".to_string()
    } else {
        "RUC".to_string()
    };

    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    conn.execute(
        "INSERT INTO clientes (tipo_identificacion, identificacion, nombre, direccion, activo)
         VALUES (?1, ?2, ?3, ?4, 1)",
        rusqlite::params![tipo_id, identificacion, nombre, direccion],
    )
    .map_err(|e| format!("Error guardando cliente: {}", e))?;

    let new_id = conn.last_insert_rowid();

    Ok(Cliente {
        id: Some(new_id),
        tipo_identificacion: tipo_id,
        identificacion: Some(identificacion),
        nombre,
        direccion,
        telefono: None,
        email: None,
        activo: true,
        lista_precio_id: None,
        lista_precio_nombre: None,
        categoria_id: None,
        permite_credito: None,
        dias_credito: None,
        limite_credito: None,
        descuento_pct: None,
    })
}

// ════════════════════════════════════════════════════════════════════
// v2.5.39: Categorías de clientes
// ════════════════════════════════════════════════════════════════════

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
pub struct CategoriaCliente {
    pub id: Option<i64>,
    pub nombre: String,
    pub descripcion: Option<String>,
    pub permite_credito: bool,
    pub dias_credito: i64,
    pub limite_credito: f64,
    pub descuento_pct: f64,
    pub lista_precio_id: Option<i64>,
    pub requiere_ruc: bool,
    pub es_default: bool,
    pub activo: bool,
}

#[tauri::command]
pub fn listar_categorias_clientes(db: State<Database>) -> Result<Vec<CategoriaCliente>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let mut stmt = conn.prepare(
        "SELECT id, nombre, descripcion, permite_credito, dias_credito, limite_credito,
                descuento_pct, lista_precio_id, requiere_ruc, es_default, activo
         FROM categorias_clientes WHERE activo = 1 ORDER BY es_default DESC, nombre ASC"
    ).map_err(|e| e.to_string())?;
    let rows = stmt.query_map([], |r| Ok(CategoriaCliente {
        id: Some(r.get(0)?),
        nombre: r.get(1)?,
        descripcion: r.get(2)?,
        permite_credito: r.get::<_, i32>(3)? != 0,
        dias_credito: r.get(4)?,
        limite_credito: r.get(5)?,
        descuento_pct: r.get(6)?,
        lista_precio_id: r.get(7)?,
        requiere_ruc: r.get::<_, i32>(8)? != 0,
        es_default: r.get::<_, i32>(9)? != 0,
        activo: r.get::<_, i32>(10)? != 0,
    })).map_err(|e| e.to_string())?;
    let lista: Vec<CategoriaCliente> = rows.filter_map(Result::ok).collect();
    Ok(lista)
}

#[tauri::command]
pub fn crear_categoria_cliente(db: State<Database>, categoria: CategoriaCliente) -> Result<i64, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    // Si es default, quitar default de las demás
    if categoria.es_default {
        let _ = conn.execute("UPDATE categorias_clientes SET es_default = 0", []);
    }
    conn.execute(
        "INSERT INTO categorias_clientes (nombre, descripcion, permite_credito, dias_credito, limite_credito,
                                          descuento_pct, lista_precio_id, requiere_ruc, es_default, activo)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, 1)",
        rusqlite::params![
            categoria.nombre.trim(), categoria.descripcion,
            categoria.permite_credito as i32, categoria.dias_credito, categoria.limite_credito,
            categoria.descuento_pct, categoria.lista_precio_id,
            categoria.requiere_ruc as i32, categoria.es_default as i32,
        ],
    ).map_err(|e| {
        let m = e.to_string();
        if m.contains("UNIQUE") { "Ya existe una categoría con ese nombre".to_string() } else { m }
    })?;
    Ok(conn.last_insert_rowid())
}

#[tauri::command]
pub fn actualizar_categoria_cliente(db: State<Database>, categoria: CategoriaCliente) -> Result<(), String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let id = categoria.id.ok_or("ID requerido")?;
    if categoria.es_default {
        let _ = conn.execute("UPDATE categorias_clientes SET es_default = 0 WHERE id != ?1", rusqlite::params![id]);
    }
    conn.execute(
        "UPDATE categorias_clientes SET nombre=?1, descripcion=?2, permite_credito=?3, dias_credito=?4,
         limite_credito=?5, descuento_pct=?6, lista_precio_id=?7, requiere_ruc=?8, es_default=?9, activo=?10
         WHERE id=?11",
        rusqlite::params![
            categoria.nombre.trim(), categoria.descripcion,
            categoria.permite_credito as i32, categoria.dias_credito, categoria.limite_credito,
            categoria.descuento_pct, categoria.lista_precio_id,
            categoria.requiere_ruc as i32, categoria.es_default as i32, categoria.activo as i32, id
        ],
    ).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn eliminar_categoria_cliente(db: State<Database>, id: i64) -> Result<(), String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    // Validar que no haya clientes vinculados
    let n: i64 = conn.query_row(
        "SELECT COUNT(*) FROM clientes WHERE categoria_id = ?1 AND activo = 1",
        rusqlite::params![id], |r| r.get(0),
    ).unwrap_or(0);
    if n > 0 {
        return Err(format!("No se puede eliminar: tiene {} cliente(s) asignado(s). Cámbialos de categoría primero.", n));
    }
    // No permitir borrar default si es la única
    let total: i64 = conn.query_row(
        "SELECT COUNT(*) FROM categorias_clientes WHERE activo = 1", [], |r| r.get(0),
    ).unwrap_or(0);
    if total <= 1 {
        return Err("Debe existir al menos una categoría activa".to_string());
    }
    conn.execute("UPDATE categorias_clientes SET activo = 0 WHERE id = ?1", rusqlite::params![id])
        .map_err(|e| e.to_string())?;
    Ok(())
}

// ════════════════════════════════════════════════════════════════════
// v2.5.39: Import / Export de clientes en XLSX
// ════════════════════════════════════════════════════════════════════

#[tauri::command]
pub fn exportar_plantilla_clientes(db: State<Database>) -> Result<Vec<u8>, String> {
    use rust_xlsxwriter::*;
    let mut workbook = Workbook::new();

    // Cargar nombres de categorías existentes para mostrarlos en ejemplos
    let categorias: Vec<String> = {
        let conn = db.conn.lock().map_err(|e| e.to_string())?;
        let mut stmt = conn.prepare("SELECT nombre FROM categorias_clientes WHERE activo = 1 ORDER BY es_default DESC")
            .map_err(|e| e.to_string())?;
        let rows: Vec<String> = stmt.query_map([], |r| r.get::<_, String>(0))
            .map_err(|e| e.to_string())?
            .filter_map(Result::ok)
            .collect();
        rows
    };
    let cat_ejemplo = categorias.first().cloned().unwrap_or_else(|| "General".to_string());

    let sheet = workbook.add_worksheet();
    sheet.set_name("Clientes").map_err(|e| e.to_string())?;
    let header_fmt = Format::new().set_bold().set_background_color(Color::RGB(0x2563EB)).set_font_color(Color::White).set_border(FormatBorder::Thin);

    let headers = ["tipo_identificacion", "identificacion", "nombre", "categoria", "direccion", "telefono", "email"];
    for (i, h) in headers.iter().enumerate() {
        sheet.write_string_with_format(0, i as u16, *h, &header_fmt).map_err(|e| e.to_string())?;
    }

    // Ejemplos
    let ej1 = ["CEDULA", "0912345678", "Juan Pérez", &cat_ejemplo, "Av. 9 de Octubre 123", "0991234567", "juan@example.com"];
    let ej2 = ["RUC", "0993377128001", "Empresa Ejemplo S.A.", &cat_ejemplo, "Andrés Marín y Aguirre", "045555555", "compras@empresa.com"];
    let ej3 = ["CONSUMIDOR_FINAL", "9999999999999", "CONSUMIDOR FINAL", &cat_ejemplo, "", "", ""];
    for (i, ej) in [&ej1, &ej2, &ej3].iter().enumerate() {
        for (j, v) in ej.iter().enumerate() {
            sheet.write_string((i + 1) as u32, j as u16, *v).ok();
        }
    }

    // Auto-fit columns
    for i in 0..7u16 { sheet.set_column_width(i, 20).ok(); }
    sheet.set_column_width(2, 30).ok();
    sheet.set_column_width(4, 30).ok();
    sheet.set_column_width(6, 28).ok();
    let _ = sheet;

    // Hoja de instrucciones
    {
        let inst = workbook.add_worksheet();
        inst.set_name("Instrucciones").map_err(|e| e.to_string())?;
        let bold = Format::new().set_bold();
        inst.write_string_with_format(0, 0, "Como llenar esta plantilla", &bold).ok();
        let texto = vec![
            "",
            "COLUMNAS OBLIGATORIAS: nombre",
            "tipo_identificacion: CEDULA | RUC | PASAPORTE | CONSUMIDOR_FINAL (default: CEDULA)",
            "identificacion: cedula 10 dig, RUC 13 dig, pasaporte alfanumerico. Para CONSUMIDOR_FINAL usar 9999999999999.",
            "categoria: nombre EXACTO de una categoria existente. Si la dejas vacia se asigna la categoria DEFAULT.",
            "          Al asignar categoria, el cliente HEREDA los defaults de esa categoria (dias credito, limite, descuento, etc).",
            "",
            "CATEGORIAS DISPONIBLES EN TU SISTEMA:",
        ];
        let mut row_idx = 1u32;
        for t in &texto {
            inst.write_string(row_idx, 0, *t).ok();
            row_idx += 1;
        }
        for cat in &categorias {
            inst.write_string(row_idx, 0, &format!("  - {}", cat)).ok();
            row_idx += 1;
        }
        inst.set_column_width(0, 90).ok();
    }

    let buf = workbook.save_to_buffer().map_err(|e| e.to_string())?;
    Ok(buf)
}

#[tauri::command]
pub fn exportar_clientes_excel(db: State<Database>) -> Result<Vec<u8>, String> {
    use rust_xlsxwriter::*;
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let mut stmt = conn.prepare(
        "SELECT c.tipo_identificacion, COALESCE(c.identificacion, ''), c.nombre,
                COALESCE(cc.nombre, '') as categoria,
                COALESCE(c.direccion, ''), COALESCE(c.telefono, ''), COALESCE(c.email, ''),
                COALESCE(c.permite_credito, 0), COALESCE(c.dias_credito, 0),
                COALESCE(c.limite_credito, 0), COALESCE(c.descuento_pct, 0)
         FROM clientes c
         LEFT JOIN categorias_clientes cc ON cc.id = c.categoria_id
         WHERE c.activo = 1 ORDER BY c.nombre"
    ).map_err(|e| e.to_string())?;

    let mut workbook = Workbook::new();
    let sheet = workbook.add_worksheet();
    sheet.set_name("Clientes").map_err(|e| e.to_string())?;
    let header_fmt = Format::new().set_bold().set_background_color(Color::RGB(0x2563EB)).set_font_color(Color::White).set_border(FormatBorder::Thin);

    let headers = ["tipo_identificacion", "identificacion", "nombre", "categoria", "direccion", "telefono", "email",
                   "permite_credito", "dias_credito", "limite_credito", "descuento_pct"];
    for (i, h) in headers.iter().enumerate() {
        sheet.write_string_with_format(0, i as u16, *h, &header_fmt).map_err(|e| e.to_string())?;
    }

    let rows = stmt.query_map([], |r| {
        Ok((
            r.get::<_, String>(0)?, r.get::<_, String>(1)?, r.get::<_, String>(2)?,
            r.get::<_, String>(3)?, r.get::<_, String>(4)?, r.get::<_, String>(5)?, r.get::<_, String>(6)?,
            r.get::<_, i32>(7)?, r.get::<_, i64>(8)?, r.get::<_, f64>(9)?, r.get::<_, f64>(10)?,
        ))
    }).map_err(|e| e.to_string())?;

    let mut row = 1u32;
    for r in rows {
        let (tipo, id, nom, cat, dir, tel, em, pc, dc, lc, dpct) = r.map_err(|e| e.to_string())?;
        sheet.write_string(row, 0, &tipo).ok();
        sheet.write_string(row, 1, &id).ok();
        sheet.write_string(row, 2, &nom).ok();
        sheet.write_string(row, 3, &cat).ok();
        sheet.write_string(row, 4, &dir).ok();
        sheet.write_string(row, 5, &tel).ok();
        sheet.write_string(row, 6, &em).ok();
        sheet.write_number(row, 7, pc as f64).ok();
        sheet.write_number(row, 8, dc as f64).ok();
        sheet.write_number(row, 9, lc).ok();
        sheet.write_number(row, 10, dpct).ok();
        row += 1;
    }

    for i in 0..11u16 { sheet.set_column_width(i, 18).ok(); }
    sheet.set_column_width(2, 32).ok();
    sheet.set_column_width(4, 30).ok();

    let buf = workbook.save_to_buffer().map_err(|e| e.to_string())?;
    Ok(buf)
}

#[tauri::command]
pub fn importar_clientes_excel(db: State<Database>, archivo_bytes: Vec<u8>) -> Result<serde_json::Value, String> {
    use calamine::{Reader, Xlsx, open_workbook_from_rs};
    use std::io::Cursor;

    let conn = db.conn.lock().map_err(|e| e.to_string())?;

    let cursor = Cursor::new(&archivo_bytes);
    let mut workbook: Xlsx<_> = open_workbook_from_rs(cursor)
        .map_err(|e| format!("Error abriendo Excel: {}", e))?;
    let sheet_names = workbook.sheet_names().to_vec();
    let sheet_name = sheet_names.first().ok_or("Archivo sin hojas")?;
    let range = workbook.worksheet_range(sheet_name).map_err(|e| format!("Error leyendo: {}", e))?;

    let mut rows_iter = range.rows();
    let header_row = rows_iter.next().ok_or("Archivo vacío")?;
    let headers: Vec<String> = header_row.iter().map(|c| c.to_string().trim().to_lowercase()).collect();
    let find_col = |n: &str| headers.iter().position(|h| h == n);

    let col_nombre = find_col("nombre").ok_or("Columna 'nombre' es obligatoria")?;
    let col_tipo = find_col("tipo_identificacion");
    let col_id = find_col("identificacion");
    let col_cat = find_col("categoria");
    let col_dir = find_col("direccion");
    let col_tel = find_col("telefono");
    let col_email = find_col("email");
    let col_pc = find_col("permite_credito");
    let col_dc = find_col("dias_credito");
    let col_lc = find_col("limite_credito");
    let col_dpct = find_col("descuento_pct");

    // Cache de categorias por nombre (lowercase)
    let cats: std::collections::HashMap<String, (i64, bool, i64, f64, f64, Option<i64>)> = {
        let mut stmt = conn.prepare(
            "SELECT id, nombre, permite_credito, dias_credito, limite_credito, descuento_pct, lista_precio_id
             FROM categorias_clientes WHERE activo = 1"
        ).map_err(|e| e.to_string())?;
        let mut map = std::collections::HashMap::new();
        let it = stmt.query_map([], |r| Ok((
            r.get::<_, i64>(0)?, r.get::<_, String>(1)?,
            r.get::<_, i32>(2)? != 0, r.get::<_, i64>(3)?,
            r.get::<_, f64>(4)?, r.get::<_, f64>(5)?, r.get::<_, Option<i64>>(6)?,
        ))).map_err(|e| e.to_string())?;
        for x in it.flatten() {
            map.insert(x.1.to_lowercase(), (x.0, x.2, x.3, x.4, x.5, x.6));
        }
        map
    };
    let cat_default_id: Option<i64> = conn.query_row(
        "SELECT id FROM categorias_clientes WHERE es_default = 1 AND activo = 1 LIMIT 1",
        [], |r| r.get(0),
    ).ok();

    let mut creados = 0i64;
    let mut actualizados = 0i64;
    let mut errores = 0i64;
    let mut msgs: Vec<String> = Vec::new();

    for (line_idx, row) in rows_iter.enumerate() {
        let get_s = |idx: Option<usize>| -> String {
            idx.and_then(|i| row.get(i)).map(|c| c.to_string().trim().to_string()).unwrap_or_default()
        };
        let get_f = |idx: Option<usize>| -> f64 {
            idx.and_then(|i| row.get(i)).and_then(|c| match c {
                calamine::Data::Float(f) => Some(*f),
                calamine::Data::Int(i) => Some(*i as f64),
                calamine::Data::String(s) => s.trim().parse::<f64>().ok(),
                _ => None,
            }).unwrap_or(0.0)
        };

        let nombre = get_s(Some(col_nombre));
        if nombre.is_empty() { continue; }
        let tipo_raw = get_s(col_tipo).to_uppercase();
        let tipo = if tipo_raw.is_empty() { "CEDULA".to_string() } else { tipo_raw };
        let ident_raw = get_s(col_id);
        let ident: Option<String> = if ident_raw.is_empty() { None } else { Some(ident_raw) };
        let cat_nombre = get_s(col_cat);
        let dir = get_s(col_dir);
        let tel = get_s(col_tel);
        let email = get_s(col_email);

        // Resolver categoria — si viene por nombre buscar; si no, usar default
        let (cat_id, cat_defaults) = if !cat_nombre.is_empty() {
            match cats.get(&cat_nombre.to_lowercase()) {
                Some(c) => (Some(c.0), Some((c.1, c.2, c.3, c.4))),
                None => {
                    errores += 1;
                    msgs.push(format!("Fila {}: categoria '{}' no existe", line_idx + 2, cat_nombre));
                    continue;
                }
            }
        } else {
            (cat_default_id, None)
        };

        // Si el Excel trae estos campos explícitos, prevalecen sobre los defaults de categoría
        let pc_default = cat_defaults.map(|d| d.0).unwrap_or(false);
        let dc_default = cat_defaults.map(|d| d.1).unwrap_or(0);
        let lc_default = cat_defaults.map(|d| d.2).unwrap_or(0.0);
        let dpct_default = cat_defaults.map(|d| d.3).unwrap_or(0.0);

        let permite_credito: i32 = if col_pc.is_some() {
            let s = get_s(col_pc);
            if !s.is_empty() && (s == "1" || s.to_lowercase() == "si" || s.to_lowercase() == "true") { 1 } else { 0 }
        } else { pc_default as i32 };
        let dias_credito: i64 = if col_dc.is_some() && !get_s(col_dc).is_empty() { get_f(col_dc) as i64 } else { dc_default };
        let limite_credito: f64 = if col_lc.is_some() && !get_s(col_lc).is_empty() { get_f(col_lc) } else { lc_default };
        let descuento_pct: f64 = if col_dpct.is_some() && !get_s(col_dpct).is_empty() { get_f(col_dpct) } else { dpct_default };

        // Existe por identificacion?
        let existing: Option<i64> = ident.as_ref().and_then(|i|
            conn.query_row("SELECT id FROM clientes WHERE identificacion = ?1",
                rusqlite::params![i], |r| r.get(0)).ok()
        );

        let res = if let Some(id) = existing {
            conn.execute(
                "UPDATE clientes SET tipo_identificacion=?1, nombre=?2, direccion=?3, telefono=?4, email=?5,
                 categoria_id=?6, permite_credito=?7, dias_credito=?8, limite_credito=?9, descuento_pct=?10,
                 updated_at=datetime('now','localtime')
                 WHERE id=?11",
                rusqlite::params![tipo, nombre, dir, tel, email, cat_id, permite_credito, dias_credito, limite_credito, descuento_pct, id]
            ).map(|_| (false, id))
        } else {
            conn.execute(
                "INSERT INTO clientes (tipo_identificacion, identificacion, nombre, direccion, telefono, email,
                 activo, categoria_id, permite_credito, dias_credito, limite_credito, descuento_pct)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, 1, ?7, ?8, ?9, ?10, ?11)",
                rusqlite::params![tipo, ident, nombre, dir, tel, email, cat_id, permite_credito, dias_credito, limite_credito, descuento_pct]
            ).map(|_| (true, conn.last_insert_rowid()))
        };

        match res {
            Ok((true, _)) => creados += 1,
            Ok((false, _)) => actualizados += 1,
            Err(e) => {
                errores += 1;
                msgs.push(format!("Fila {}: {}", line_idx + 2, e));
            }
        }
    }

    Ok(serde_json::json!({
        "creados": creados,
        "actualizados": actualizados,
        "errores": errores,
        "mensajes": msgs,
    }))
}
