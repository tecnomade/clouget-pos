use crate::db::Database;
use crate::models::Cliente;
use serde::Deserialize;
use tauri::State;

#[tauri::command]
pub fn crear_cliente(db: State<Database>, cliente: Cliente) -> Result<i64, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;

    conn.execute(
        "INSERT INTO clientes (tipo_identificacion, identificacion, nombre, direccion, telefono, email, activo, lista_precio_id)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        rusqlite::params![
            cliente.tipo_identificacion,
            cliente.identificacion,
            cliente.nombre,
            cliente.direccion,
            cliente.telefono,
            cliente.email,
            cliente.activo as i32,
            cliente.lista_precio_id,
        ],
    )
    .map_err(|e| e.to_string())?;

    Ok(conn.last_insert_rowid())
}

#[tauri::command]
pub fn actualizar_cliente(db: State<Database>, cliente: Cliente) -> Result<(), String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let id = cliente.id.ok_or("ID requerido para actualizar")?;

    conn.execute(
        "UPDATE clientes SET tipo_identificacion=?1, identificacion=?2, nombre=?3,
         direccion=?4, telefono=?5, email=?6, activo=?7, lista_precio_id=?8,
         updated_at=datetime('now','localtime')
         WHERE id=?9",
        rusqlite::params![
            cliente.tipo_identificacion,
            cliente.identificacion,
            cliente.nombre,
            cliente.direccion,
            cliente.telefono,
            cliente.email,
            cliente.activo as i32,
            cliente.lista_precio_id,
            id,
        ],
    )
    .map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
pub fn buscar_clientes(db: State<Database>, termino: String) -> Result<Vec<Cliente>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let busqueda = format!("%{}%", termino);

    let mut stmt = conn
        .prepare(
            "SELECT c.id, c.tipo_identificacion, c.identificacion, c.nombre, c.direccion,
                    c.telefono, c.email, c.activo, c.lista_precio_id, lp.nombre
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
                    c.telefono, c.email, c.activo, c.lista_precio_id, lp.nombre
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
    })
}
