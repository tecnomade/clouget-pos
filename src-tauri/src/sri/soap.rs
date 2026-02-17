use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use std::io::Write;

/// Escribe un mensaje al archivo de log SRI (en %LOCALAPPDATA%/CloudgetPOS/sri_debug.log)
pub fn log_sri(msg: &str) {
    sri_log(msg);
}

fn sri_log(msg: &str) {
    let timestamp = chrono::Local::now().format("%Y-%m-%d %H:%M:%S%.3f");
    let line = format!("[{}] {}\n", timestamp, msg);

    // Usar %LOCALAPPDATA%/CloudgetPOS/ o fallback a temp
    let log_dir = std::env::var("LOCALAPPDATA")
        .map(|app_data| std::path::PathBuf::from(app_data).join("CloudgetPOS"))
        .unwrap_or_else(|_| std::env::temp_dir().join("CloudgetPOS"));

    let _ = std::fs::create_dir_all(&log_dir);
    let log_path = log_dir.join("sri_debug.log");
    if let Ok(mut file) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_path)
    {
        let _ = file.write_all(line.as_bytes());
    }
}

/// Endpoints del SRI
const RECEPCION_PRUEBAS: &str = "https://celcer.sri.gob.ec/comprobantes-electronicos-ws/RecepcionComprobantesOffline";
const RECEPCION_PRODUCCION: &str = "https://cel.sri.gob.ec/comprobantes-electronicos-ws/RecepcionComprobantesOffline";
const AUTORIZACION_PRUEBAS: &str = "https://celcer.sri.gob.ec/comprobantes-electronicos-ws/AutorizacionComprobantesOffline";
const AUTORIZACION_PRODUCCION: &str = "https://cel.sri.gob.ec/comprobantes-electronicos-ws/AutorizacionComprobantesOffline";

/// Resultado de la emision al SRI
#[derive(Debug)]
pub struct ResultadoSri {
    pub exito: bool,
    pub estado: String,           // "AUTORIZADO", "NO AUTORIZADO", "RECIBIDA", etc
    pub clave_acceso: String,
    pub numero_autorizacion: Option<String>,
    pub fecha_autorizacion: Option<String>,
    pub mensaje: Option<String>,
}

/// Obtiene la URL de recepcion segun el ambiente
fn url_recepcion(ambiente: &str) -> &'static str {
    match ambiente {
        "2" | "produccion" => RECEPCION_PRODUCCION,
        _ => RECEPCION_PRUEBAS,
    }
}

/// Obtiene la URL de autorizacion segun el ambiente
fn url_autorizacion(ambiente: &str) -> &'static str {
    match ambiente {
        "2" | "produccion" => AUTORIZACION_PRODUCCION,
        _ => AUTORIZACION_PRUEBAS,
    }
}

/// Envia un comprobante firmado al SRI y espera la autorizacion.
///
/// Flujo de 2 pasos:
/// 1. Enviar XML firmado (base64) al WS de recepcion
/// 2. Consultar autorizacion con la clave de acceso (con reintentos)
pub async fn enviar_comprobante(
    xml_firmado: &str,
    clave_acceso: &str,
    ambiente: &str,
) -> Result<ResultadoSri, String> {
    sri_log("========== INICIO EMISION SRI ==========");
    sri_log(&format!("Clave acceso: {}", clave_acceso));
    sri_log(&format!("Ambiente: {}", ambiente));
    sri_log(&format!("XML firmado longitud: {} bytes", xml_firmado.len()));

    let client = reqwest::Client::builder()
        .danger_accept_invalid_certs(ambiente != "2" && ambiente != "produccion") // Solo en pruebas
        .timeout(std::time::Duration::from_secs(60))
        .build()
        .map_err(|e| format!("Error creando cliente HTTP: {}", e))?;

    // Paso 1: Enviar a recepcion
    let xml_base64 = BASE64.encode(xml_firmado.as_bytes());
    let soap_recepcion = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?><soapenv:Envelope xmlns:soapenv="http://schemas.xmlsoap.org/soap/envelope/" xmlns:ec="http://ec.gob.sri.ws.recepcion"><soapenv:Header/><soapenv:Body><ec:validarComprobante><xml>{}</xml></ec:validarComprobante></soapenv:Body></soapenv:Envelope>"#,
        xml_base64
    );

    let url_rec = url_recepcion(ambiente);
    sri_log(&format!("PASO 1: Enviando a recepcion: {}", url_rec));

    // Reintentos en paso de recepcion (el SRI puede fallar por timeout/red)
    let delays_rec = [0u64, 3, 5];
    let mut body_recepcion = String::new();
    let mut last_err = String::new();

    for (intento, delay) in delays_rec.iter().enumerate() {
        if *delay > 0 {
            sri_log(&format!("Reintentando recepcion (intento {}/3) en {}s...", intento + 1, delay));
            tokio::time::sleep(std::time::Duration::from_secs(*delay)).await;
        }

        match client
            .post(url_rec)
            .header("Content-Type", "text/xml; charset=utf-8")
            .body(soap_recepcion.clone())
            .send()
            .await
        {
            Ok(resp) => {
                let http_status_rec = resp.status();
                sri_log(&format!("Respuesta HTTP recepcion: {}", http_status_rec));
                match resp.text().await {
                    Ok(body) => {
                        body_recepcion = body;
                        last_err.clear();
                        break;
                    }
                    Err(e) => {
                        last_err = format!("Error leyendo respuesta SRI recepcion: {}", e);
                        sri_log(&last_err);
                    }
                }
            }
            Err(e) => {
                last_err = format!("Error enviando al SRI (recepcion): {}", e);
                sri_log(&last_err);
            }
        }
    }

    if !last_err.is_empty() {
        return Err(last_err);
    }

    sri_log(&format!("Body recepcion completo:\n{}", body_recepcion));

    // Verificar que fue recibida
    let estado_recepcion = extraer_tag(&body_recepcion, "estado")
        .unwrap_or_default();
    sri_log(&format!("Estado recepcion: '{}'", estado_recepcion));

    if estado_recepcion != "RECIBIDA" {
        let identificador = extraer_tag(&body_recepcion, "identificador").unwrap_or_default();

        // Error 70 = clave de acceso ya en procesamiento → saltar a consulta de autorizacion
        if identificador == "70" {
            // El SRI ya tiene este comprobante, consultamos directamente la autorizacion
        } else {
            // Cualquier otro error: reportar al usuario
            let mensaje_tag = extraer_tag(&body_recepcion, "mensaje").unwrap_or_default();
            let info_adicional = extraer_tag(&body_recepcion, "informacionAdicional").unwrap_or_default();

            let mut msg_parts = Vec::new();
            if !identificador.is_empty() {
                msg_parts.push(format!("Error {}", identificador));
            }
            if !mensaje_tag.is_empty() {
                msg_parts.push(mensaje_tag);
            }
            if !info_adicional.is_empty() {
                msg_parts.push(info_adicional);
            }
            let mensaje = if msg_parts.is_empty() {
                format!("Estado SRI: {}", estado_recepcion)
            } else {
                msg_parts.join(" - ")
            };

            return Ok(ResultadoSri {
                exito: false,
                estado: estado_recepcion,
                clave_acceso: clave_acceso.to_string(),
                numero_autorizacion: None,
                fecha_autorizacion: None,
                mensaje: Some(mensaje),
            });
        }
    }

    // Paso 2: Consultar autorizacion (con reintentos)
    sri_log("PASO 2: Consultando autorizacion...");
    let soap_autorizacion = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?><soapenv:Envelope xmlns:soapenv="http://schemas.xmlsoap.org/soap/envelope/" xmlns:ec="http://ec.gob.sri.ws.autorizacion"><soapenv:Header/><soapenv:Body><ec:autorizacionComprobante><claveAccesoComprobante>{}</claveAccesoComprobante></ec:autorizacionComprobante></soapenv:Body></soapenv:Envelope>"#,
        clave_acceso
    );

    let url_aut = url_autorizacion(ambiente);
    let max_reintentos = 8;

    for intento in 0..max_reintentos {
        if intento > 0 {
            // Espera progresiva: 3, 5, 8, 12, 15, 20, 25 segundos
            let espera = match intento {
                1 => 3,
                2 => 5,
                3 => 8,
                4 => 12,
                5 => 15,
                6 => 20,
                _ => 25,
            };
            sri_log(&format!("Esperando {} segundos antes de reintento {}...", espera, intento + 1));
            tokio::time::sleep(std::time::Duration::from_secs(espera)).await;
        }

        sri_log(&format!("Intento autorizacion {}/{}: {}", intento + 1, max_reintentos, url_aut));

        let resp_aut = match client
            .post(url_aut)
            .header("Content-Type", "text/xml; charset=utf-8")
            .body(soap_autorizacion.clone())
            .send()
            .await
        {
            Ok(r) => r,
            Err(e) => {
                sri_log(&format!("ERROR HTTP autorizacion intento {}: {}", intento + 1, e));
                if intento < max_reintentos - 1 {
                    continue;
                }
                return Err(format!("Error consultando autorizacion SRI: {}", e));
            }
        };

        let http_status_aut = resp_aut.status();
        sri_log(&format!("HTTP status autorizacion: {}", http_status_aut));

        let body_aut = resp_aut.text().await
            .map_err(|e| format!("Error leyendo respuesta autorizacion: {}", e))?;

        sri_log(&format!("Body autorizacion intento {}:\n{}", intento + 1, body_aut));

        let estado = extraer_tag(&body_aut, "estado")
            .unwrap_or_default();
        sri_log(&format!("Estado autorizacion: '{}'", estado));

        if estado == "AUTORIZADO" {
            let numero_aut = extraer_tag(&body_aut, "numeroAutorizacion");
            let fecha_aut = extraer_tag(&body_aut, "fechaAutorizacion");
            sri_log(&format!("AUTORIZADO! Autorizacion: {:?}, Fecha: {:?}", numero_aut, fecha_aut));

            return Ok(ResultadoSri {
                exito: true,
                estado: "AUTORIZADO".to_string(),
                clave_acceso: clave_acceso.to_string(),
                numero_autorizacion: numero_aut,
                fecha_autorizacion: fecha_aut,
                mensaje: Some("Factura autorizada correctamente".to_string()),
            });
        }

        if estado == "NO AUTORIZADO" || estado == "RECHAZADO" {
            let mensaje = extraer_tag(&body_aut, "mensaje")
                .or_else(|| extraer_tag(&body_aut, "informacionAdicional"))
                .unwrap_or_else(|| "Comprobante no autorizado por el SRI".to_string());
            sri_log(&format!("RECHAZADO/NO AUTORIZADO: {}", mensaje));

            return Ok(ResultadoSri {
                exito: false,
                estado,
                clave_acceso: clave_acceso.to_string(),
                numero_autorizacion: None,
                fecha_autorizacion: None,
                mensaje: Some(mensaje),
            });
        }

        // Si esta en proceso, reintentamos
        sri_log(&format!("Estado '{}' - en proceso, reintentando...", estado));
    }

    // Agotamos reintentos — queda pendiente
    sri_log("TIMEOUT: Agotamos reintentos, queda PENDIENTE");
    let msg_pendiente = if ambiente == "1" || ambiente == "pruebas" {
        "Comprobante en procesamiento en el SRI (ambiente de pruebas). Esto es normal — el formato es correcto pero el SRI de pruebas puede tardar en autorizar. Reintente mas tarde."
    } else {
        "El SRI no respondio a tiempo. El comprobante quedo en procesamiento. Reintente mas tarde."
    };
    Ok(ResultadoSri {
        exito: false,
        estado: "EN_PROCESO".to_string(),
        clave_acceso: clave_acceso.to_string(),
        numero_autorizacion: None,
        fecha_autorizacion: None,
        mensaje: Some(msg_pendiente.to_string()),
    })
}

/// Consulta solo la autorizacion de un comprobante por clave de acceso (sin reenviar).
/// Util para verificar el estado de un comprobante PENDIENTE.
pub async fn consultar_autorizacion(
    clave_acceso: &str,
    ambiente: &str,
) -> Result<ResultadoSri, String> {
    sri_log(&format!("=== CONSULTA AUTORIZACION: clave={} ===", clave_acceso));

    let client = reqwest::Client::builder()
        .danger_accept_invalid_certs(ambiente != "2" && ambiente != "produccion")
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .map_err(|e| format!("Error creando cliente HTTP: {}", e))?;

    let soap_autorizacion = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?><soapenv:Envelope xmlns:soapenv="http://schemas.xmlsoap.org/soap/envelope/" xmlns:ec="http://ec.gob.sri.ws.autorizacion"><soapenv:Header/><soapenv:Body><ec:autorizacionComprobante><claveAccesoComprobante>{}</claveAccesoComprobante></ec:autorizacionComprobante></soapenv:Body></soapenv:Envelope>"#,
        clave_acceso
    );

    let url_aut = url_autorizacion(ambiente);
    sri_log(&format!("Consultando: {}", url_aut));

    let resp = client
        .post(url_aut)
        .header("Content-Type", "text/xml; charset=utf-8")
        .body(soap_autorizacion)
        .send()
        .await
        .map_err(|e| {
            sri_log(&format!("ERROR consultando autorizacion: {}", e));
            format!("Error consultando autorizacion SRI: {}", e)
        })?;

    let body = resp.text().await
        .map_err(|e| format!("Error leyendo respuesta: {}", e))?;

    sri_log(&format!("Respuesta consulta:\n{}", body));

    let estado = extraer_tag(&body, "estado").unwrap_or_default();

    if estado == "AUTORIZADO" {
        let numero_aut = extraer_tag(&body, "numeroAutorizacion");
        let fecha_aut = extraer_tag(&body, "fechaAutorizacion");
        sri_log(&format!("CONSULTA: AUTORIZADO! Aut: {:?}", numero_aut));

        return Ok(ResultadoSri {
            exito: true,
            estado: "AUTORIZADO".to_string(),
            clave_acceso: clave_acceso.to_string(),
            numero_autorizacion: numero_aut,
            fecha_autorizacion: fecha_aut,
            mensaje: Some("Factura autorizada correctamente".to_string()),
        });
    }

    if estado == "NO AUTORIZADO" || estado == "RECHAZADO" {
        let mensaje = extraer_tag(&body, "mensaje")
            .or_else(|| extraer_tag(&body, "informacionAdicional"))
            .unwrap_or_else(|| "Comprobante no autorizado".to_string());
        sri_log(&format!("CONSULTA: RECHAZADO - {}", mensaje));

        return Ok(ResultadoSri {
            exito: false,
            estado,
            clave_acceso: clave_acceso.to_string(),
            numero_autorizacion: None,
            fecha_autorizacion: None,
            mensaje: Some(mensaje),
        });
    }

    // No encontrada o en proceso
    sri_log(&format!("CONSULTA: Estado '{}' - no autorizada aun", estado));
    Ok(ResultadoSri {
        exito: false,
        estado: "EN_PROCESO".to_string(),
        clave_acceso: clave_acceso.to_string(),
        numero_autorizacion: None,
        fecha_autorizacion: None,
        mensaje: Some("Comprobante aun en procesamiento".to_string()),
    })
}

/// Extrae el contenido de un tag XML por nombre (busqueda simple sin parser completo).
/// Soporta tags con namespace (ej: <ns2:estado>) buscando variantes.
fn extraer_tag(xml: &str, tag: &str) -> Option<String> {
    // Intentar sin namespace primero
    let open = format!("<{}>", tag);
    let close = format!("</{}>", tag);

    if let Some(start) = xml.find(&open) {
        let content_start = start + open.len();
        if let Some(end) = xml[content_start..].find(&close) {
            let content = &xml[content_start..content_start + end];
            return Some(content.trim().to_string());
        }
    }

    // Intentar con cualquier namespace prefix (ej: <ns2:estado>)
    let pattern_open = format!(":{}>" , tag);
    if let Some(colon_pos) = xml.find(&pattern_open) {
        // Buscar el '<' antes del namespace
        let search_back = &xml[..colon_pos];
        if let Some(lt_pos) = search_back.rfind('<') {
            let full_open_end = colon_pos + pattern_open.len();
            // Extraer el prefix (ej: "ns2")
            let prefix = &xml[lt_pos + 1..colon_pos];
            let full_close = format!("</{}:{}>", prefix, tag);
            if let Some(end) = xml[full_open_end..].find(&full_close) {
                let content = &xml[full_open_end..full_open_end + end];
                return Some(content.trim().to_string());
            }
        }
    }

    None
}
