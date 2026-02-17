use serde::{Deserialize, Serialize};

/// URL base de Supabase Edge Functions (se sobreescribe desde config)
const SUPABASE_FUNCTIONS_URL: &str = "https://placeholder.supabase.co/functions/v1";

/// Dias de gracia para operar sin conexion al servidor de validacion
const DIAS_GRACIA_OFFLINE: i64 = 7;

/// Respuesta del servidor de validacion de suscripcion
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct EstadoSuscripcion {
    pub autorizado: bool,
    pub plan: String,
    pub fecha_hasta: Option<String>,
    pub docs_restantes: Option<i64>,
    pub es_lifetime: bool,
    pub mensaje: String,
}

/// Respuesta del endpoint consumir-documento
#[derive(Debug, Deserialize)]
struct RespuestaConsumo {
    ok: bool,
    docs_restantes: Option<i64>,
}

/// Respuesta del endpoint validar-suscripcion
#[derive(Debug, Deserialize)]
struct RespuestaValidacion {
    autorizado: bool,
    plan: Option<String>,
    fecha_hasta: Option<String>,
    docs_restantes: Option<i64>,
    es_lifetime: Option<bool>,
    mensaje: Option<String>,
}

/// Valida la suscripcion SRI llamando al servidor online.
/// Si no hay conexion, usa la cache local con gracia de 7 dias.
pub async fn validar_suscripcion(
    machine_id: &str,
    api_url: &str,
    api_key: &str,
) -> Result<EstadoSuscripcion, String> {
    let url = if api_url.is_empty() {
        SUPABASE_FUNCTIONS_URL
    } else {
        api_url
    };

    let endpoint = format!("{}/validar-suscripcion", url);

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|e| format!("Error creando cliente HTTP: {}", e))?;

    let body = serde_json::json!({
        "machine_id": machine_id
    });

    let resp = client
        .post(&endpoint)
        .header("Content-Type", "application/json")
        .header("Authorization", format!("Bearer {}", api_key))
        .header("apikey", api_key)
        .json(&body)
        .send()
        .await;

    match resp {
        Ok(response) => {
            if !response.status().is_success() {
                return Err(format!(
                    "Error del servidor de suscripciones (HTTP {})",
                    response.status()
                ));
            }

            let data: RespuestaValidacion = response
                .json()
                .await
                .map_err(|e| format!("Error parseando respuesta del servidor: {}", e))?;

            Ok(EstadoSuscripcion {
                autorizado: data.autorizado,
                plan: data.plan.unwrap_or_default(),
                fecha_hasta: data.fecha_hasta,
                docs_restantes: data.docs_restantes,
                es_lifetime: data.es_lifetime.unwrap_or(false),
                mensaje: data.mensaje.unwrap_or_else(|| {
                    if data.autorizado {
                        "Suscripcion activa".to_string()
                    } else {
                        "Sin suscripcion activa".to_string()
                    }
                }),
            })
        }
        Err(_e) => {
            // No hay conexion — retornamos error para que el caller use la cache
            Err("SIN_CONEXION".to_string())
        }
    }
}

/// Consume 1 documento del paquete (solo para plan "paquete").
/// Llama al endpoint consumir-documento en el servidor.
pub async fn consumir_documento(
    machine_id: &str,
    clave_acceso: &str,
    api_url: &str,
    api_key: &str,
) -> Result<i64, String> {
    let url = if api_url.is_empty() {
        SUPABASE_FUNCTIONS_URL
    } else {
        api_url
    };

    let endpoint = format!("{}/consumir-documento", url);

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|e| format!("Error creando cliente HTTP: {}", e))?;

    let body = serde_json::json!({
        "machine_id": machine_id,
        "clave_acceso": clave_acceso
    });

    let resp = client
        .post(&endpoint)
        .header("Content-Type", "application/json")
        .header("Authorization", format!("Bearer {}", api_key))
        .header("apikey", api_key)
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("Error conectando al servidor de suscripciones: {}", e))?;

    if !resp.status().is_success() {
        return Err(format!(
            "Error al consumir documento (HTTP {})",
            resp.status()
        ));
    }

    let data: RespuestaConsumo = resp
        .json()
        .await
        .map_err(|e| format!("Error parseando respuesta: {}", e))?;

    if !data.ok {
        return Err("No se pudo consumir el documento. Verifique su suscripcion.".to_string());
    }

    Ok(data.docs_restantes.unwrap_or(0))
}

/// Evalua si la cache local de suscripcion es valida (dentro de los dias de gracia).
/// Retorna Some(EstadoSuscripcion) si la cache es usable, None si expiro.
pub fn evaluar_cache_offline(
    ultima_validacion: &str,
    autorizado_cache: bool,
    plan_cache: &str,
    hasta_cache: &str,
    docs_restantes_cache: &str,
    es_lifetime_cache: bool,
    mensaje_cache: &str,
) -> Option<EstadoSuscripcion> {
    if ultima_validacion.is_empty() {
        return None;
    }

    // Parsear fecha de ultima validacion (YYYY-MM-DD)
    let hoy = chrono::Local::now().format("%Y-%m-%d").to_string();

    // Calcular diferencia en dias (simplificado: comparar strings de fecha)
    let dias_desde = calcular_dias_diferencia(ultima_validacion, &hoy);

    if dias_desde > DIAS_GRACIA_OFFLINE {
        return None; // Cache expiro — necesita reconectarse
    }

    let docs_rest = if docs_restantes_cache.is_empty() {
        None
    } else {
        docs_restantes_cache.parse::<i64>().ok()
    };

    Some(EstadoSuscripcion {
        autorizado: autorizado_cache,
        plan: plan_cache.to_string(),
        fecha_hasta: if hasta_cache.is_empty() {
            None
        } else {
            Some(hasta_cache.to_string())
        },
        docs_restantes: docs_rest,
        es_lifetime: es_lifetime_cache,
        mensaje: if mensaje_cache.is_empty() {
            "Usando cache offline".to_string()
        } else {
            format!("{} (offline)", mensaje_cache)
        },
    })
}

/// Calcula la diferencia aproximada en dias entre dos fechas YYYY-MM-DD.
/// Usa chrono para precision.
fn calcular_dias_diferencia(fecha_desde: &str, fecha_hasta: &str) -> i64 {
    use chrono::NaiveDate;

    let desde = NaiveDate::parse_from_str(fecha_desde, "%Y-%m-%d");
    let hasta = NaiveDate::parse_from_str(fecha_hasta, "%Y-%m-%d");

    match (desde, hasta) {
        (Ok(d), Ok(h)) => (h - d).num_days().max(0),
        _ => 999, // Si no se puede parsear, asumir que expiro
    }
}
