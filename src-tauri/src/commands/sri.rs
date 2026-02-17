use crate::db::Database;
use crate::sri::{clave_acceso, firma, soap, suscripcion, xml};
use serde::{Deserialize, Serialize};
use tauri::State;
use x509_parser::prelude::FromDer;

/// Resultado de emision para el frontend
#[derive(Debug, Serialize, Deserialize)]
pub struct ResultadoEmision {
    pub exito: bool,
    pub estado_sri: String,
    pub clave_acceso: Option<String>,
    pub numero_autorizacion: Option<String>,
    pub fecha_autorizacion: Option<String>,
    pub mensaje: String,
    pub numero_factura: Option<String>,
}

/// Estado del modulo SRI para el frontend
#[derive(Debug, Serialize, Deserialize)]
pub struct EstadoSri {
    pub modulo_activo: bool,
    pub certificado_cargado: bool,
    pub ambiente: String,
    pub facturas_usadas: i64,
    pub facturas_gratis: i64,
    // Suscripcion online
    pub suscripcion_autorizada: bool,
    pub suscripcion_plan: String,
    pub suscripcion_hasta: String,
    pub suscripcion_docs_restantes: Option<i64>,
    pub suscripcion_es_lifetime: bool,
    pub suscripcion_mensaje: String,
}

/// Carga un certificado P12 para facturacion electronica.
/// Lee el archivo .p12, valida que sea correcto, y lo guarda en la BD.
#[tauri::command]
pub fn cargar_certificado_sri(
    db: State<Database>,
    p12_path: String,
    password: String,
) -> Result<String, String> {
    // Leer archivo P12
    let p12_data = std::fs::read(&p12_path)
        .map_err(|e| format!("Error leyendo archivo P12: {}", e))?;

    // Validar que el P12 sea valido intentando parsearlo con pure Rust
    let keystore = p12_keystore::KeyStore::from_pkcs12(&p12_data, &password)
        .map_err(|e| format!("Password incorrecta o P12 invalido: {}", e))?;

    // Verificar que tiene llave privada y certificado
    let (_alias, chain) = keystore
        .private_key_chain()
        .ok_or("El P12 no contiene llave privada y certificado")?;

    let certs = chain.chain();
    if certs.is_empty() {
        return Err("El P12 no contiene un certificado X509".to_string());
    }

    // Parsear certificado para obtener info
    let cert_der = certs[0].as_der();
    let (_, x509_cert) = x509_parser::certificate::X509Certificate::from_der(cert_der)
        .map_err(|e| format!("Error parseando certificado: {:?}", e))?;

    // Obtener nombre del sujeto
    let subject: String = format!("{}", x509_cert.subject());
    let not_after: String = format!("{}", x509_cert.validity().not_after);

    // Obtener nombre del archivo para referencia
    let nombre_archivo = std::path::Path::new(&p12_path)
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "certificado.p12".to_string());

    // Guardar en la BD (tabla sri_certificado, max 1 fila)
    let conn = db.conn.lock().map_err(|e| e.to_string())?;

    // Usar REPLACE para siempre sobreescribir el unico registro (id=1)
    conn.execute(
        "INSERT OR REPLACE INTO sri_certificado (id, p12_data, password, nombre, fecha_expiracion)
         VALUES (1, ?1, ?2, ?3, ?4)",
        rusqlite::params![p12_data, password, nombre_archivo, not_after],
    )
    .map_err(|e| format!("Error guardando certificado: {}", e))?;

    // Actualizar config
    conn.execute(
        "UPDATE config SET value = '1' WHERE key = 'sri_certificado_cargado'",
        [],
    )
    .map_err(|e| format!("Error actualizando config: {}", e))?;

    Ok(format!("Certificado cargado: {} (expira: {})", subject, not_after))
}

/// Datos de venta para emision SRI (evita tuplas largas)
struct DatosVentaSri {
    numero: String,
    numero_factura: Option<String>,
    cliente_id: i64,
    fecha: String,
    descuento: f64,
    forma_pago: String,
    tipo_documento: String,
    estado_sri: String,
    clave_acceso_previa: Option<String>,
    xml_firmado_previo: Option<String>,
}

/// Datos de detalle para emision SRI
struct DetalleSri {
    codigo: String,
    nombre: String,
    cantidad: f64,
    precio_unitario: f64,
    descuento: f64,
    iva_porcentaje: f64,
}

/// Datos del cliente para emision SRI
struct ClienteSri {
    tipo_identificacion: String,
    identificacion: Option<String>,
    nombre: String,
    direccion: Option<String>,
    email: Option<String>,
}

/// Emite una factura electronica al SRI para una venta existente.
/// Genera XML, firma con XAdES-BES, envia via SOAP, y actualiza la venta.
#[tauri::command]
pub async fn emitir_factura_sri(
    db: State<'_, Database>,
    venta_id: i64,
) -> Result<ResultadoEmision, String> {
    // 0. Enforcement de suscripcion SRI
    {
        let conn = db.conn.lock().map_err(|e| e.to_string())?;
        let get_cfg = |key: &str| -> String {
            conn.query_row(
                "SELECT value FROM config WHERE key = ?1",
                rusqlite::params![key],
                |row| row.get(0),
            )
            .unwrap_or_default()
        };

        let gratis: i64 = get_cfg("sri_facturas_gratis").parse().unwrap_or(10);
        let usadas: i64 = get_cfg("sri_facturas_usadas").parse().unwrap_or(0);

        // Dentro del trial gratis: siempre permitir
        if usadas >= gratis {
            // Trial agotado — verificar suscripcion con cache
            let autorizado = get_cfg("sri_suscripcion_autorizado") == "1";
            let plan = get_cfg("sri_suscripcion_plan");
            let hasta = get_cfg("sri_suscripcion_hasta");
            let es_lifetime = get_cfg("sri_suscripcion_es_lifetime") == "1";
            let cuota_str = get_cfg("sri_suscripcion_docs_restantes");
            let ultima_validacion = get_cfg("sri_suscripcion_ultima_validacion");

            // Verificar que la cache no ha expirado (7 dias de gracia)
            let cache_valida = suscripcion::evaluar_cache_offline(
                &ultima_validacion,
                autorizado,
                &plan,
                &hasta,
                &cuota_str,
                es_lifetime,
                "",
            );

            match cache_valida {
                None => {
                    return Err(
                        "No se puede verificar su suscripcion SRI. Conectese a internet y verifique en Configuracion."
                            .to_string(),
                    );
                }
                Some(estado) => {
                    if !estado.autorizado {
                        return Err(
                            "Su prueba gratuita ha terminado. Adquiera una suscripcion SRI para continuar facturando."
                                .to_string(),
                        );
                    }

                    // Verificar segun tipo de plan
                    let hoy = chrono::Local::now().format("%Y-%m-%d").to_string();
                    match estado.plan.as_str() {
                        "lifetime" => {
                            // Siempre permitido
                        }
                        "mensual" | "semestral" | "anual" => {
                            if let Some(ref fecha_hasta) = estado.fecha_hasta {
                                if fecha_hasta.as_str() < hoy.as_str() {
                                    return Err(format!(
                                        "Su suscripcion SRI ({}) expiro el {}. Renueve su suscripcion para continuar.",
                                        estado.plan, fecha_hasta
                                    ));
                                }
                            }
                        }
                        "paquete" => {
                            if let Some(docs) = estado.docs_restantes {
                                if docs <= 0 {
                                    return Err(
                                        "Ha agotado sus documentos del paquete. Adquiera un nuevo paquete para continuar."
                                            .to_string(),
                                    );
                                }
                            }
                        }
                        _ => {
                            // Plan desconocido — permitir si autorizado
                        }
                    }
                }
            }
        }
    }
    // Fin enforcement — continuar con emision normal

    // 1. Leer datos de la venta, detalles, cliente y config
    let (venta_data, detalles_data, cliente_data, config_data, p12_data, p12_password) = {
        let conn = db.conn.lock().map_err(|e| e.to_string())?;

        // Leer venta
        let venta = conn.query_row(
            "SELECT numero, cliente_id, fecha, descuento, forma_pago, tipo_documento, estado_sri, clave_acceso, xml_firmado, numero_factura
             FROM ventas WHERE id = ?1",
            rusqlite::params![venta_id],
            |row| {
                Ok(DatosVentaSri {
                    numero: row.get(0)?,
                    numero_factura: row.get(9)?,
                    cliente_id: row.get::<_, Option<i64>>(1)?.unwrap_or(1),
                    fecha: row.get(2)?,
                    descuento: row.get(3)?,
                    forma_pago: row.get(4)?,
                    tipo_documento: row.get(5)?,
                    estado_sri: row.get::<_, String>(6).unwrap_or_else(|_| "PENDIENTE".to_string()),
                    clave_acceso_previa: row.get::<_, Option<String>>(7)?,
                    xml_firmado_previo: row.get::<_, Option<String>>(8)?,
                })
            },
        ).map_err(|e| format!("Venta no encontrada: {}", e))?;

        if venta.tipo_documento != "FACTURA" {
            return Err("Solo se pueden emitir facturas electronicas".to_string());
        }

        if venta.estado_sri == "AUTORIZADA" {
            return Err("Esta factura ya fue autorizada por el SRI".to_string());
        }

        // Leer detalles
        let mut stmt = conn.prepare(
            "SELECT p.codigo, p.nombre, d.cantidad,
             d.precio_unitario, d.descuento, d.iva_porcentaje
             FROM venta_detalles d
             JOIN productos p ON d.producto_id = p.id
             WHERE d.venta_id = ?1"
        ).map_err(|e| e.to_string())?;

        let detalles: Vec<DetalleSri> = stmt
            .query_map(rusqlite::params![venta_id], |row| {
                Ok(DetalleSri {
                    codigo: row.get::<_, Option<String>>(0)?.unwrap_or_else(|| "SIN-COD".to_string()),
                    nombre: row.get(1)?,
                    cantidad: row.get(2)?,
                    precio_unitario: row.get(3)?,
                    descuento: row.get(4)?,
                    iva_porcentaje: row.get(5)?,
                })
            })
            .map_err(|e| e.to_string())?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())?;

        // Leer cliente
        let cliente = conn.query_row(
            "SELECT tipo_identificacion, identificacion, nombre, direccion, email
             FROM clientes WHERE id = ?1",
            rusqlite::params![venta.cliente_id],
            |row| {
                Ok(ClienteSri {
                    tipo_identificacion: row.get(0)?,
                    identificacion: row.get(1)?,
                    nombre: row.get(2)?,
                    direccion: row.get(3)?,
                    email: row.get(4)?,
                })
            },
        ).map_err(|e| format!("Cliente no encontrado: {}", e))?;

        // Leer config
        let mut config: std::collections::HashMap<String, String> = std::collections::HashMap::new();
        let mut stmt_cfg = conn.prepare("SELECT key, value FROM config")
            .map_err(|e| e.to_string())?;
        let rows = stmt_cfg.query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        }).map_err(|e| e.to_string())?;
        for row in rows {
            let (k, v) = row.map_err(|e| e.to_string())?;
            config.insert(k, v);
        }

        // Leer certificado P12
        let (p12_blob, p12_pass): (Vec<u8>, String) = conn.query_row(
            "SELECT p12_data, password FROM sri_certificado WHERE id = 1",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        ).map_err(|_| "No hay certificado digital cargado. Cargue un P12 primero.".to_string())?;

        (venta, detalles, cliente, config, p12_blob, p12_pass)
    };

    // 2. Preparar datos para generar XML
    let cfg = |key: &str| -> String {
        config_data.get(key).cloned().unwrap_or_default()
    };

    let ruc = cfg("ruc");
    if ruc.is_empty() || ruc.len() != 13 {
        return Err("Configure el RUC del negocio (13 digitos) antes de emitir facturas".to_string());
    }

    let ambiente_config = cfg("sri_ambiente");
    let ambiente = match ambiente_config.as_str() {
        "produccion" => "2",
        _ => "1", // pruebas por defecto
    };

    let establecimiento = cfg("establecimiento");
    let punto_emision = cfg("punto_emision");
    let regimen = cfg("regimen");

    let config_key_sri = if ambiente == "1" { "secuencial_factura_pruebas" } else { "secuencial_factura" };

    // --- Logica de reenvio SRI ---
    // Si la factura ya tiene clave_acceso (estado PENDIENTE), primero consultar
    // autorizacion con esa clave. Si ya fue autorizada, listo. Si no, reenviar
    // el mismo XML firmado. Solo generar nuevo XML si es primera vez.
    //
    // secuencial_sri y numero_factura: se definen aqui y se asignan solo en primera emision.
    // Para reenvios se reusan los valores previos.
    let mut secuencial_sri: i64 = 0;
    let mut numero_factura = venta_data.numero_factura.clone().unwrap_or_default();
    let mut es_primera_emision = false;

    let (clave, xml_firmado_final, resultado_sri) = if venta_data.estado_sri == "PENDIENTE"
        && venta_data.clave_acceso_previa.is_some()
        && venta_data.xml_firmado_previo.is_some()
    {
        let clave_previa = venta_data.clave_acceso_previa.clone().unwrap();
        let xml_previo = venta_data.xml_firmado_previo.clone().unwrap();
        soap::log_sri(&format!("=== REENVIO: Factura PENDIENTE, consultando clave previa: {} ===", clave_previa));

        // Primero consultar si ya fue autorizada
        let resultado_consulta = soap::consultar_autorizacion(
            &clave_previa,
            ambiente,
        ).await;

        match resultado_consulta {
            Ok(ref res) if res.exito => {
                // Ya fue autorizada! Retornar directo
                (clave_previa, xml_previo, resultado_consulta.unwrap())
            }
            _ => {
                // No autorizada aun — reenviar el mismo XML firmado
                soap::log_sri("Clave previa no autorizada, reenviando XML firmado...");
                let resultado = soap::enviar_comprobante(
                    &xml_previo,
                    &clave_previa,
                    ambiente,
                ).await?;
                (clave_previa, xml_previo, resultado)
            }
        }
    } else {
        // Primera emision: generar nuevo XML, firmar y enviar
        es_primera_emision = true;

        // Obtener secuencial SRI (solo se asigna en primera emision)
        secuencial_sri = {
            let conn = db.conn.lock().map_err(|e| e.to_string())?;
            conn.query_row(
                "SELECT CAST(value AS INTEGER) FROM config WHERE key = ?1",
                rusqlite::params![config_key_sri],
                |row| row.get(0),
            ).map_err(|e| format!("Error obteniendo secuencial SRI: {}", e))?
        };
        let secuencial = format!("{:09}", secuencial_sri);
        numero_factura = format!("{}-{}-{}", establecimiento, punto_emision, secuencial);

        let fecha_emision = formatear_fecha_emision(&venta_data.fecha)?;

        let clave_nueva = clave_acceso::generar_clave_acceso(
            &fecha_emision,
            "01", // factura
            &ruc,
            ambiente,
            &establecimiento,
            &punto_emision,
            &secuencial,
            "1", // tipo emision normal
        );

        let tipo_id_sri = match cliente_data.tipo_identificacion.as_str() {
            "RUC" => "04",
            "CEDULA" => "05",
            "PASAPORTE" => "06",
            _ => "07",
        };

        let identificacion_comprador = cliente_data.identificacion.clone()
            .unwrap_or_else(|| "9999999999999".to_string());

        let mut detalles_factura = Vec::new();
        let mut subtotal_iva_0 = 0.0_f64;
        let mut subtotal_iva_15 = 0.0_f64;
        let mut iva_total = 0.0_f64;

        for det in &detalles_data {
            let precio_total_sin_imp = det.cantidad * det.precio_unitario - det.descuento;
            let codigo_porcentaje_iva = if det.iva_porcentaje > 0.0 { "4" } else { "0" };
            let tarifa = xml::tarifa_iva(codigo_porcentaje_iva);
            let valor_iva_det = precio_total_sin_imp * (tarifa / 100.0);

            if det.iva_porcentaje > 0.0 {
                subtotal_iva_15 += precio_total_sin_imp;
                iva_total += valor_iva_det;
            } else {
                subtotal_iva_0 += precio_total_sin_imp;
            }

            detalles_factura.push(xml::DetalleFactura {
                codigo_principal: det.codigo.clone(),
                descripcion: det.nombre.clone(),
                cantidad: det.cantidad,
                precio_unitario: det.precio_unitario,
                descuento: det.descuento,
                precio_total_sin_impuesto: precio_total_sin_imp,
                codigo_porcentaje_iva: codigo_porcentaje_iva.to_string(),
                tarifa_iva: tarifa,
                base_imponible: precio_total_sin_imp,
                valor_iva: valor_iva_det,
            });
        }

        let mut impuestos_totales = Vec::new();

        if subtotal_iva_0 > 0.0 {
            impuestos_totales.push(xml::ImpuestoTotal {
                codigo: "2".to_string(),
                codigo_porcentaje: "0".to_string(),
                base_imponible: subtotal_iva_0,
                valor: 0.0,
            });
        }

        if subtotal_iva_15 > 0.0 {
            impuestos_totales.push(xml::ImpuestoTotal {
                codigo: "2".to_string(),
                codigo_porcentaje: "4".to_string(),
                base_imponible: subtotal_iva_15,
                valor: iva_total,
            });
        }

        let total_sin_impuestos = subtotal_iva_0 + subtotal_iva_15;
        let importe_total = total_sin_impuestos + iva_total;

        let contribuyente_rimpe = match regimen.as_str() {
            "RIMPE_EMPRENDEDOR" => Some("CONTRIBUYENTE RÉGIMEN RIMPE".to_string()),
            "RIMPE_POPULAR" => Some("CONTRIBUYENTE NEGOCIO POPULAR - RÉGIMEN RIMPE".to_string()),
            _ => None,
        };

        let forma_pago_cod = xml::forma_pago_sri(&venta_data.forma_pago);

        let mut info_adicional = Vec::new();
        if let Some(ref email) = cliente_data.email {
            if !email.is_empty() {
                info_adicional.push(xml::CampoAdicional {
                    nombre: "email".to_string(),
                    valor: email.clone(),
                });
            }
        }
        if let Some(ref dir) = cliente_data.direccion {
            if !dir.is_empty() {
                info_adicional.push(xml::CampoAdicional {
                    nombre: "direccion".to_string(),
                    valor: dir.clone(),
                });
            }
        }

        let datos_factura = xml::DatosFactura {
            ambiente: ambiente.to_string(),
            tipo_emision: "1".to_string(),
            razon_social: cfg("nombre_negocio"),
            nombre_comercial: cfg("nombre_negocio"),
            ruc: ruc.clone(),
            clave_acceso: clave_nueva.clone(),
            cod_doc: "01".to_string(),
            estab: establecimiento.clone(),
            pto_emi: punto_emision.clone(),
            secuencial: secuencial.clone(),
            dir_matriz: cfg("direccion"),
            fecha_emision: fecha_emision.clone(),
            dir_establecimiento: cfg("direccion"),
            obligado_contabilidad: "NO".to_string(),
            contribuyente_rimpe,
            tipo_identificacion_comprador: tipo_id_sri.to_string(),
            razon_social_comprador: cliente_data.nombre.clone(),
            identificacion_comprador,
            direccion_comprador: cliente_data.direccion.clone(),
            total_sin_impuestos,
            total_descuento: venta_data.descuento,
            importe_total,
            impuestos_totales,
            pagos: vec![xml::PagoFactura {
                forma_pago: forma_pago_cod.to_string(),
                total: importe_total,
            }],
            detalles: detalles_factura,
            info_adicional,
        };

        let xml_sin_firma = xml::generar_xml_factura(&datos_factura);

        soap::log_sri(&format!("XML sin firma generado ({} bytes):\n{}", xml_sin_firma.len(), xml_sin_firma));

        let xml_firmado_result = firma::firmar_comprobante(
            &xml_sin_firma,
            &p12_data,
            &p12_password,
            "factura",
        )?;

        let resultado = soap::enviar_comprobante(
            &xml_firmado_result.xml,
            &clave_nueva,
            ambiente,
        ).await?;

        (clave_nueva, xml_firmado_result.xml, resultado)
    };

    // 10. Actualizar la venta en la BD
    {
        let conn = db.conn.lock().map_err(|e| e.to_string())?;

        let nuevo_estado = if resultado_sri.exito {
            "AUTORIZADA"
        } else if resultado_sri.estado == "EN_PROCESO" {
            "PENDIENTE" // Timeout — se puede reintentar
        } else {
            "RECHAZADA"
        };

        // Guardar XML firmado si fue autorizada o pendiente (para reintentar)
        let xml_para_guardar = if resultado_sri.exito || resultado_sri.estado == "EN_PROCESO" {
            Some(xml_firmado_final.clone())
        } else {
            None
        };

        // Guardar numero_factura: en primera emision siempre, en reenvio mantener el previo
        let nf_para_guardar = if !numero_factura.is_empty() {
            Some(numero_factura.clone())
        } else {
            venta_data.numero_factura.clone()
        };

        conn.execute(
            "UPDATE ventas SET estado_sri = ?1, clave_acceso = ?2,
             autorizacion_sri = ?3, xml_firmado = ?4, fecha_autorizacion = ?5,
             numero_factura = ?6
             WHERE id = ?7",
            rusqlite::params![
                nuevo_estado,
                clave.clone(),
                resultado_sri.numero_autorizacion,
                xml_para_guardar,
                if resultado_sri.exito { resultado_sri.fecha_autorizacion.as_deref() } else { None },
                nf_para_guardar,
                venta_id,
            ],
        ).map_err(|e| format!("Error actualizando venta: {}", e))?;

        // Si fue autorizada, incrementar secuencial SRI (solo en primera emision) y contador
        if resultado_sri.exito {
            if es_primera_emision {
                conn.execute(
                    "UPDATE config SET value = CAST(?1 AS TEXT) WHERE key = ?2",
                    rusqlite::params![secuencial_sri + 1, config_key_sri],
                ).ok();
            }

            conn.execute(
                "UPDATE config SET value = CAST(CAST(value AS INTEGER) + 1 AS TEXT)
                 WHERE key = 'sri_facturas_usadas'",
                [],
            ).ok();
        }
    }

    // Si fue autorizada y el plan es paquete, consumir documento en el servidor
    if resultado_sri.exito {
        let plan_actual = {
            let conn = db.conn.lock().map_err(|e| e.to_string())?;
            conn.query_row(
                "SELECT value FROM config WHERE key = 'sri_suscripcion_plan'",
                [],
                |row| row.get::<_, String>(0),
            )
            .unwrap_or_default()
        };

        if plan_actual == "paquete" {
            let machine_id = crate::commands::licencia::obtener_machine_id()
                .unwrap_or_default();
            let (api_url, api_key) = {
                let conn = db.conn.lock().map_err(|e| e.to_string())?;
                let get = |key: &str| -> String {
                    conn.query_row(
                        "SELECT value FROM config WHERE key = ?1",
                        rusqlite::params![key],
                        |row| row.get(0),
                    )
                    .unwrap_or_default()
                };
                (get("sri_suscripcion_url"), get("licencia_api_key"))
            };

            // Consumir documento async (no bloqueante, si falla no afecta la emision)
            if let Ok(docs_rest) = suscripcion::consumir_documento(
                &machine_id,
                &clave,
                &api_url,
                &api_key,
            )
            .await
            {
                let conn = db.conn.lock().map_err(|e| e.to_string())?;
                conn.execute(
                    "UPDATE config SET value = ?1 WHERE key = 'sri_suscripcion_docs_restantes'",
                    rusqlite::params![docs_rest.to_string()],
                )
                .ok();
            }
        }
    }

    Ok(ResultadoEmision {
        exito: resultado_sri.exito,
        estado_sri: resultado_sri.estado.clone(),
        clave_acceso: Some(clave),
        numero_autorizacion: resultado_sri.numero_autorizacion,
        fecha_autorizacion: resultado_sri.fecha_autorizacion,
        mensaje: resultado_sri.mensaje.unwrap_or_else(|| {
            if resultado_sri.exito {
                "Factura autorizada correctamente".to_string()
            } else {
                format!("Estado: {}", resultado_sri.estado)
            }
        }),
        numero_factura: if !numero_factura.is_empty() { Some(numero_factura) } else { None },
    })
}

/// Consulta el estado del modulo SRI (para la UI de configuracion)
#[tauri::command]
pub fn consultar_estado_sri(db: State<Database>) -> Result<EstadoSri, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;

    // Leer toda la config en una sola query
    let mut stmt = conn.prepare("SELECT key, value FROM config")
        .map_err(|e| e.to_string())?;
    let cfg: std::collections::HashMap<String, String> = stmt
        .query_map([], |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)))
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();

    let get = |key: &str| -> String { cfg.get(key).cloned().unwrap_or_default() };

    let docs_rest_str = get("sri_suscripcion_docs_restantes");

    Ok(EstadoSri {
        modulo_activo: get("sri_modulo_activo") == "1",
        certificado_cargado: get("sri_certificado_cargado") == "1",
        ambiente: get("sri_ambiente"),
        facturas_usadas: get("sri_facturas_usadas").parse().unwrap_or(0),
        facturas_gratis: get("sri_facturas_gratis").parse().unwrap_or(10),
        suscripcion_autorizada: get("sri_suscripcion_autorizado") == "1",
        suscripcion_plan: get("sri_suscripcion_plan"),
        suscripcion_hasta: get("sri_suscripcion_hasta"),
        suscripcion_docs_restantes: if docs_rest_str.is_empty() { None } else { docs_rest_str.parse::<i64>().ok() },
        suscripcion_es_lifetime: get("sri_suscripcion_es_lifetime") == "1",
        suscripcion_mensaje: get("sri_suscripcion_mensaje"),
    })
}

/// Cambia el ambiente del SRI (pruebas/produccion)
#[tauri::command]
pub fn cambiar_ambiente_sri(
    db: State<Database>,
    ambiente: String,
) -> Result<(), String> {
    let ambiente_valido = match ambiente.as_str() {
        "pruebas" | "produccion" => &ambiente,
        _ => return Err("Ambiente invalido. Use 'pruebas' o 'produccion'".to_string()),
    };

    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    conn.execute(
        "UPDATE config SET value = ?1 WHERE key = 'sri_ambiente'",
        rusqlite::params![ambiente_valido],
    ).map_err(|e| format!("Error actualizando ambiente: {}", e))?;

    Ok(())
}

/// Valida la suscripcion SRI online (o desde cache si no hay internet).
/// Guarda resultado en config como cache local.
#[tauri::command]
pub async fn validar_suscripcion_sri(
    db: State<'_, Database>,
) -> Result<EstadoSri, String> {
    // Obtener machine_id y URL del servidor
    let machine_id = crate::commands::licencia::obtener_machine_id()?;
    let (api_url, api_key) = {
        let conn = db.conn.lock().map_err(|e| e.to_string())?;
        let get = |key: &str| -> String {
            conn.query_row(
                "SELECT value FROM config WHERE key = ?1",
                rusqlite::params![key],
                |row| row.get(0),
            )
            .unwrap_or_default()
        };
        (get("sri_suscripcion_url"), get("licencia_api_key"))
    };

    // Intentar validar online
    let estado = match suscripcion::validar_suscripcion(&machine_id, &api_url, &api_key).await {
        Ok(estado_online) => {
            // Guardar en cache
            let conn = db.conn.lock().map_err(|e| e.to_string())?;
            let hoy = chrono::Local::now().format("%Y-%m-%d").to_string();

            let configs = [
                ("sri_suscripcion_autorizado", if estado_online.autorizado { "1" } else { "0" }),
                ("sri_suscripcion_plan", &estado_online.plan),
                ("sri_suscripcion_hasta", estado_online.fecha_hasta.as_deref().unwrap_or("")),
                (
                    "sri_suscripcion_docs_restantes",
                    &estado_online
                        .docs_restantes
                        .map(|d| d.to_string())
                        .unwrap_or_default(),
                ),
                ("sri_suscripcion_es_lifetime", if estado_online.es_lifetime { "1" } else { "0" }),
                ("sri_suscripcion_ultima_validacion", &hoy),
                ("sri_suscripcion_mensaje", &estado_online.mensaje),
            ];

            for (key, value) in &configs {
                conn.execute(
                    "INSERT OR REPLACE INTO config (key, value) VALUES (?1, ?2)",
                    rusqlite::params![key, value],
                )
                .ok();
            }

            // Activar modulo si hay suscripcion valida
            if estado_online.autorizado {
                conn.execute(
                    "UPDATE config SET value = '1' WHERE key = 'sri_modulo_activo'",
                    [],
                )
                .ok();
            }

            drop(conn);
            estado_online
        }
        Err(e) if e == "SIN_CONEXION" => {
            // Sin conexion — usar cache
            let conn = db.conn.lock().map_err(|e| e.to_string())?;
            let get_cfg = |key: &str| -> String {
                conn.query_row(
                    "SELECT value FROM config WHERE key = ?1",
                    rusqlite::params![key],
                    |row| row.get(0),
                )
                .unwrap_or_default()
            };

            let cache = suscripcion::evaluar_cache_offline(
                &get_cfg("sri_suscripcion_ultima_validacion"),
                get_cfg("sri_suscripcion_autorizado") == "1",
                &get_cfg("sri_suscripcion_plan"),
                &get_cfg("sri_suscripcion_hasta"),
                &get_cfg("sri_suscripcion_docs_restantes"),
                get_cfg("sri_suscripcion_es_lifetime") == "1",
                &get_cfg("sri_suscripcion_mensaje"),
            );

            match cache {
                Some(estado_cache) => estado_cache,
                None => suscripcion::EstadoSuscripcion {
                    autorizado: false,
                    plan: String::new(),
                    fecha_hasta: None,
                    docs_restantes: None,
                    es_lifetime: false,
                    mensaje: "Sin conexion al servidor. Conectese a internet para verificar su suscripcion.".to_string(),
                },
            }
        }
        Err(e) => {
            return Err(e);
        }
    };

    // Retornar EstadoSri completo
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let get_config = |key: &str| -> String {
        conn.query_row(
            "SELECT value FROM config WHERE key = ?1",
            rusqlite::params![key],
            |row| row.get(0),
        )
        .unwrap_or_default()
    };

    let modulo_activo = get_config("sri_modulo_activo") == "1";
    let certificado_cargado = get_config("sri_certificado_cargado") == "1";
    let ambiente = get_config("sri_ambiente");
    let facturas_usadas: i64 = get_config("sri_facturas_usadas").parse().unwrap_or(0);
    let facturas_gratis: i64 = get_config("sri_facturas_gratis").parse().unwrap_or(10);

    Ok(EstadoSri {
        modulo_activo,
        certificado_cargado,
        ambiente,
        facturas_usadas,
        facturas_gratis,
        suscripcion_autorizada: estado.autorizado,
        suscripcion_plan: estado.plan,
        suscripcion_hasta: estado.fecha_hasta.unwrap_or_default(),
        suscripcion_docs_restantes: estado.docs_restantes,
        suscripcion_es_lifetime: estado.es_lifetime,
        suscripcion_mensaje: estado.mensaje,
    })
}

// ─── Contratación de planes SRI ────────────────────────────

/// Plan SRI disponible para contratación
#[derive(Debug, Serialize, Deserialize)]
pub struct PlanSri {
    pub clave: String,
    pub nombre: String,
    pub precio: f64,
    pub descripcion: String,
    pub tipo: String,
    pub duracion_meses: Option<i64>,
    pub docs_cantidad: Option<i64>,
    pub ahorro: Option<String>,
    pub popular: bool,
    pub orden: i64,
}

/// Configuración de contratación (datos bancarios, WhatsApp)
#[derive(Debug, Serialize, Deserialize)]
pub struct ConfigContratacion {
    pub whatsapp_numero: String,
    pub banco_nombre: String,
    pub banco_tipo_cuenta: String,
    pub banco_numero_cuenta: String,
    pub banco_titular: String,
    pub banco_cedula_ruc: String,
    pub mensaje_transferencia: String,
}

/// Respuesta del endpoint obtener-planes
#[derive(Debug, Serialize, Deserialize)]
pub struct PlanesDisponibles {
    pub ok: bool,
    pub planes: Vec<PlanSri>,
    pub config: ConfigContratacion,
}

/// Respuesta del endpoint crear-pedido
#[derive(Debug, Serialize, Deserialize)]
pub struct PedidoCreado {
    pub ok: bool,
    pub pedido_id: Option<String>,
    pub referencia: Option<String>,
    pub mensaje: String,
    #[serde(default)]
    pub ya_existia: bool,
}

/// Obtiene los planes SRI disponibles y la configuración de pago desde Supabase.
#[tauri::command]
pub async fn obtener_planes_sri(
    db: State<'_, Database>,
) -> Result<PlanesDisponibles, String> {
    let machine_id = crate::commands::licencia::obtener_machine_id()?;
    let (api_url, api_key) = {
        let conn = db.conn.lock().map_err(|e| e.to_string())?;
        let get = |key: &str| -> String {
            conn.query_row(
                "SELECT value FROM config WHERE key = ?1",
                rusqlite::params![key],
                |row| row.get(0),
            )
            .unwrap_or_default()
        };
        (get("sri_suscripcion_url"), get("licencia_api_key"))
    };

    if api_url.is_empty() {
        return Err("URL del servidor no configurada".to_string());
    }

    let endpoint = format!("{}/obtener-planes", api_url);

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .build()
        .map_err(|e| format!("Error HTTP: {}", e))?;

    let body = serde_json::json!({ "machine_id": machine_id });

    let resp = client
        .post(&endpoint)
        .header("Content-Type", "application/json")
        .header("Authorization", format!("Bearer {}", api_key))
        .header("apikey", &api_key)
        .json(&body)
        .send()
        .await
        .map_err(|_| "No se pudo conectar al servidor. Verifique su conexion a internet.".to_string())?;

    if !resp.status().is_success() {
        return Err(format!("Error del servidor (HTTP {})", resp.status()));
    }

    let data: PlanesDisponibles = resp
        .json()
        .await
        .map_err(|e| format!("Error parseando respuesta: {}", e))?;

    Ok(data)
}

/// Crea un pedido de suscripción SRI en Supabase.
#[tauri::command]
pub async fn crear_pedido_sri(
    db: State<'_, Database>,
    plan_clave: String,
    plan_nombre: String,
    precio: f64,
    metodo_pago: String,
) -> Result<PedidoCreado, String> {
    let machine_id = crate::commands::licencia::obtener_machine_id()?;
    let (api_url, api_key, negocio, email) = {
        let conn = db.conn.lock().map_err(|e| e.to_string())?;
        let get = |key: &str| -> String {
            conn.query_row(
                "SELECT value FROM config WHERE key = ?1",
                rusqlite::params![key],
                |row| row.get(0),
            )
            .unwrap_or_default()
        };
        (
            get("sri_suscripcion_url"),
            get("licencia_api_key"),
            get("licencia_negocio"),
            get("licencia_email"),
        )
    };

    if api_url.is_empty() {
        return Err("URL del servidor no configurada".to_string());
    }

    let endpoint = format!("{}/crear-pedido", api_url);

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .build()
        .map_err(|e| format!("Error HTTP: {}", e))?;

    let body = serde_json::json!({
        "machine_id": machine_id,
        "negocio": negocio,
        "email": email,
        "telefono": "",
        "plan_clave": plan_clave,
        "plan_nombre": plan_nombre,
        "precio": precio,
        "metodo_pago": metodo_pago,
    });

    let resp = client
        .post(&endpoint)
        .header("Content-Type", "application/json")
        .header("Authorization", format!("Bearer {}", api_key))
        .header("apikey", &api_key)
        .json(&body)
        .send()
        .await
        .map_err(|_| "No se pudo conectar al servidor. Verifique su conexion a internet.".to_string())?;

    if !resp.status().is_success() {
        let body_text = resp.text().await.unwrap_or_default();
        if let Ok(err_data) = serde_json::from_str::<serde_json::Value>(&body_text) {
            if let Some(msg) = err_data.get("mensaje").and_then(|v| v.as_str()) {
                return Err(msg.to_string());
            }
        }
        return Err("Error creando el pedido".to_string());
    }

    let data: PedidoCreado = resp
        .json()
        .await
        .map_err(|e| format!("Error parseando respuesta: {}", e))?;

    Ok(data)
}

/// Obtiene el XML firmado de una venta autorizada.
#[tauri::command]
pub fn obtener_xml_firmado(
    db: State<Database>,
    venta_id: i64,
) -> Result<String, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let xml: String = conn
        .query_row(
            "SELECT xml_firmado FROM ventas WHERE id = ?1 AND xml_firmado IS NOT NULL",
            rusqlite::params![venta_id],
            |row| row.get(0),
        )
        .map_err(|_| "No se encontro XML firmado para esta venta".to_string())?;
    Ok(xml)
}

/// Genera el RIDE (PDF A4) para una factura autorizada y lo guarda en archivo temporal.
/// Retorna la ruta del archivo PDF.
#[tauri::command]
pub fn generar_ride_pdf(
    db: State<Database>,
    venta_id: i64,
) -> Result<String, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;

    // Obtener venta (incluye fecha_autorizacion)
    let (venta, fecha_autorizacion) = conn
        .query_row(
            "SELECT id, numero, cliente_id, fecha, subtotal_sin_iva, subtotal_con_iva,
             descuento, iva, total, forma_pago, monto_recibido, cambio, estado,
             tipo_documento, estado_sri, autorizacion_sri, clave_acceso, observacion,
             fecha_autorizacion, numero_factura
             FROM ventas WHERE id = ?1",
            rusqlite::params![venta_id],
            |row| {
                let v = crate::models::Venta {
                    id: Some(row.get(0)?),
                    numero: row.get(1)?,
                    cliente_id: row.get(2)?,
                    fecha: row.get(3)?,
                    subtotal_sin_iva: row.get(4)?,
                    subtotal_con_iva: row.get(5)?,
                    descuento: row.get(6)?,
                    iva: row.get(7)?,
                    total: row.get(8)?,
                    forma_pago: row.get(9)?,
                    monto_recibido: row.get(10)?,
                    cambio: row.get(11)?,
                    estado: row.get(12)?,
                    tipo_documento: row.get(13)?,
                    estado_sri: row.get::<_, String>(14).unwrap_or_else(|_| "NO_APLICA".to_string()),
                    autorizacion_sri: row.get(15)?,
                    clave_acceso: row.get(16)?,
                    observacion: row.get(17)?,
                    numero_factura: row.get(19)?,
                };
                let fecha_aut: Option<String> = row.get(18).unwrap_or(None);
                Ok((v, fecha_aut))
            },
        )
        .map_err(|e| format!("Venta no encontrada: {}", e))?;

    // Obtener detalles con codigo de producto
    let mut stmt = conn
        .prepare(
            "SELECT d.id, d.venta_id, d.producto_id, p.nombre, d.cantidad,
             d.precio_unitario, d.descuento, d.iva_porcentaje, d.subtotal,
             COALESCE(p.codigo, CAST(d.producto_id AS TEXT)) as codigo
             FROM venta_detalles d
             JOIN productos p ON d.producto_id = p.id
             WHERE d.venta_id = ?1",
        )
        .map_err(|e| e.to_string())?;

    let rows: Vec<(crate::models::VentaDetalle, String)> = stmt
        .query_map(rusqlite::params![venta_id], |row| {
            let det = crate::models::VentaDetalle {
                id: Some(row.get(0)?),
                venta_id: Some(row.get(1)?),
                producto_id: row.get(2)?,
                nombre_producto: Some(row.get(3)?),
                cantidad: row.get(4)?,
                precio_unitario: row.get(5)?,
                descuento: row.get(6)?,
                iva_porcentaje: row.get(7)?,
                subtotal: row.get(8)?,
            };
            let codigo: String = row.get(9)?;
            Ok((det, codigo))
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    let detalles: Vec<crate::models::VentaDetalle> = rows.iter().map(|(d, _)| d.clone()).collect();

    // Construir DetalleRide con codigo de producto
    let detalles_ride: Vec<crate::sri::ride::DetalleRide> = rows
        .iter()
        .map(|(det, codigo)| {
            let precio_total = det.cantidad * det.precio_unitario - det.descuento;
            crate::sri::ride::DetalleRide {
                codigo: codigo.clone(),
                nombre: det.nombre_producto.clone().unwrap_or_else(|| "?".to_string()),
                cantidad: det.cantidad,
                precio_unitario: det.precio_unitario,
                descuento: det.descuento,
                iva_porcentaje: det.iva_porcentaje,
                precio_total_sin_impuesto: precio_total,
            }
        })
        .collect();

    let cliente_nombre: String = venta.cliente_id
        .and_then(|cid| {
            conn.query_row("SELECT nombre FROM clientes WHERE id = ?1", rusqlite::params![cid], |row| row.get(0)).ok()
        })
        .unwrap_or_else(|| "CONSUMIDOR FINAL".to_string());

    let (cli_ident, cli_dir, cli_email, cli_tel) = venta.cliente_id
        .and_then(|cid| {
            conn.query_row(
                "SELECT COALESCE(identificacion,''), COALESCE(direccion,''), COALESCE(email,''), COALESCE(telefono,'') FROM clientes WHERE id = ?1",
                rusqlite::params![cid],
                |row| Ok((row.get::<_,String>(0)?, row.get::<_,String>(1)?, row.get::<_,String>(2)?, row.get::<_,String>(3)?)),
            ).ok()
        })
        .unwrap_or_default();

    // Config
    let mut cfg_stmt = conn.prepare("SELECT key, value FROM config").map_err(|e| e.to_string())?;
    let config_map: std::collections::HashMap<String, String> = cfg_stmt
        .query_map([], |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)))
        .map_err(|e| e.to_string())?
        .collect::<Result<std::collections::HashMap<_, _>, _>>()
        .map_err(|e| e.to_string())?;

    let observacion = venta.observacion.clone().unwrap_or_default();

    drop(stmt);
    drop(cfg_stmt);
    drop(conn);

    let venta_completa = crate::models::VentaCompleta {
        venta,
        detalles,
        cliente_nombre: Some(cliente_nombre.clone()),
    };

    let cliente_ride = crate::sri::ride::ClienteRide {
        nombre: cliente_nombre,
        identificacion: cli_ident,
        direccion: cli_dir,
        email: cli_email,
        telefono: cli_tel,
        observacion,
    };

    let pdf_bytes = crate::sri::ride::generar_ride_pdf(
        &venta_completa,
        &detalles_ride,
        &cliente_ride,
        &config_map,
        fecha_autorizacion.as_deref(),
        "FACTURA",
        None,
    )?;

    // Guardar en directorio temporal
    let temp_dir = std::env::temp_dir();
    let num_for_file = venta_completa.venta.numero_factura.as_deref()
        .unwrap_or(&venta_completa.venta.numero);
    let filename = format!("RIDE-{}.pdf", num_for_file.replace(['/', '\\', ':'], "-"));
    let pdf_path = temp_dir.join(&filename);
    std::fs::write(&pdf_path, &pdf_bytes)
        .map_err(|e| format!("Error guardando PDF: {}", e))?;

    Ok(pdf_path.to_string_lossy().to_string())
}

/// Abre el RIDE PDF en el visor del sistema para imprimir.
#[tauri::command]
pub fn imprimir_ride(
    db: State<Database>,
    venta_id: i64,
) -> Result<String, String> {
    let pdf_path = generar_ride_pdf(db, venta_id)?;

    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("cmd")
            .args(["/C", "start", "", &pdf_path])
            .spawn()
            .map_err(|e| format!("Error abriendo PDF: {}", e))?;
    }

    #[cfg(not(target_os = "windows"))]
    {
        std::process::Command::new("xdg-open")
            .arg(&pdf_path)
            .spawn()
            .map_err(|e| format!("Error abriendo PDF: {}", e))?;
    }

    Ok(pdf_path)
}

/// Lógica interna para enviar email (reutilizable por enviar y procesar cola).
async fn enviar_email_interno(
    db: &State<'_, Database>,
    venta_id: i64,
    email: &str,
) -> Result<String, String> {
    let (email_url, email_api_key, nombre_negocio) = {
        let conn = db.conn.lock().map_err(|e| e.to_string())?;
        let get = |key: &str| -> String {
            conn.query_row("SELECT value FROM config WHERE key = ?1", rusqlite::params![key], |row| row.get(0))
                .unwrap_or_default()
        };
        (get("email_service_url"), get("email_service_api_key"), get("nombre_negocio"))
    };

    if email_url.is_empty() || email_api_key.is_empty() {
        return Err("Servicio de email no configurado.".to_string());
    }

    let (numero, xml_firmado) = {
        let conn = db.conn.lock().map_err(|e| e.to_string())?;
        let row: (String, Option<String>, Option<String>) = conn.query_row(
            "SELECT numero, xml_firmado, numero_factura FROM ventas WHERE id = ?1",
            rusqlite::params![venta_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        ).map_err(|e| format!("Venta no encontrada: {}", e))?;
        let num = row.2.unwrap_or(row.0);
        (num, row.1)
    };

    let pdf_path = generar_ride_pdf(db.clone(), venta_id)?;
    let pdf_bytes = std::fs::read(&pdf_path)
        .map_err(|e| format!("Error leyendo PDF: {}", e))?;
    let pdf_b64 = base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &pdf_bytes);

    let mut adjuntos = vec![
        serde_json::json!({
            "nombre": format!("RIDE-{}.pdf", numero.replace(['/', '\\', ':'], "-")),
            "contenido_base64": pdf_b64,
            "tipo": "application/pdf"
        }),
    ];

    if let Some(ref xml) = xml_firmado {
        let xml_b64 = base64::Engine::encode(&base64::engine::general_purpose::STANDARD, xml.as_bytes());
        adjuntos.push(serde_json::json!({
            "nombre": format!("factura-{}.xml", numero.replace(['/', '\\', ':'], "-")),
            "contenido_base64": xml_b64,
            "tipo": "application/xml"
        }));
    }

    let cuerpo_html = format!(
        r#"<div style="font-family:Arial,sans-serif;max-width:600px;margin:0 auto">
        <h2 style="color:#1e40af">{}</h2>
        <p>Estimado cliente,</p>
        <p>Adjunto encontrara su factura electronica <strong>{}</strong>.</p>
        <p>Se incluyen los siguientes archivos:</p>
        <ul>
            <li>RIDE (PDF) - Representacion Impresa del Documento Electronico</li>
            <li>XML - Archivo electronico firmado</li>
        </ul>
        <p style="color:#64748b;font-size:12px">Este es un mensaje automatico generado por Clouget POS.</p>
        </div>"#,
        nombre_negocio, numero
    );

    let body = serde_json::json!({
        "destinatario": email,
        "asunto": format!("Factura Electronica {} - {}", numero, nombre_negocio),
        "cuerpo_html": cuerpo_html,
        "adjuntos": adjuntos,
    });

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .map_err(|e| format!("Error HTTP: {}", e))?;

    let resp = client
        .post(format!("{}/enviar-email", email_url))
        .header("Authorization", format!("Bearer {}", email_api_key))
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("No se pudo conectar al servicio de email: {}", e))?;

    if !resp.status().is_success() {
        let err_body = resp.text().await.unwrap_or_default();
        return Err(format!("Error enviando email: {}", err_body));
    }

    // Marcar email_enviado = 1 en la venta
    {
        let conn = db.conn.lock().map_err(|e| e.to_string())?;
        conn.execute(
            "UPDATE ventas SET email_enviado = 1 WHERE id = ?1",
            rusqlite::params![venta_id],
        ).ok();
    }

    Ok("Email enviado correctamente".to_string())
}

/// Envia notificacion por email. Si falla, encola para reintento automatico.
#[tauri::command]
pub async fn enviar_notificacion_sri(
    db: State<'_, Database>,
    venta_id: i64,
    email: String,
) -> Result<String, String> {
    match enviar_email_interno(&db, venta_id, &email).await {
        Ok(msg) => {
            // Limpiar pendientes anteriores si los hay
            let conn = db.conn.lock().map_err(|e| e.to_string())?;
            conn.execute(
                "UPDATE email_log SET estado = 'ENVIADO', enviado_at = datetime('now','localtime') WHERE venta_id = ?1 AND estado = 'PENDIENTE'",
                rusqlite::params![venta_id],
            ).ok();
            Ok(msg)
        }
        Err(err) => {
            // Encolar para reintento
            let conn = db.conn.lock().map_err(|e| e.to_string())?;
            // Solo encolar si no hay ya un pendiente para esta venta
            let ya_encolado: i64 = conn.query_row(
                "SELECT COUNT(*) FROM email_log WHERE venta_id = ?1 AND estado = 'PENDIENTE'",
                rusqlite::params![venta_id],
                |row| row.get(0),
            ).unwrap_or(0);

            if ya_encolado == 0 {
                conn.execute(
                    "INSERT INTO email_log (venta_id, email, estado, intentos, ultimo_error) VALUES (?1, ?2, 'PENDIENTE', 1, ?3)",
                    rusqlite::params![venta_id, email, err],
                ).ok();
            } else {
                conn.execute(
                    "UPDATE email_log SET intentos = intentos + 1, ultimo_error = ?1 WHERE venta_id = ?2 AND estado = 'PENDIENTE'",
                    rusqlite::params![err, venta_id],
                ).ok();
            }

            Err(format!("ENCOLADO:{}", err))
        }
    }
}

/// Procesa emails pendientes en la cola (llamado periodicamente desde frontend).
#[tauri::command]
pub async fn procesar_emails_pendientes(
    db: State<'_, Database>,
) -> Result<serde_json::Value, String> {
    let pendientes: Vec<(i64, i64, String, i64)> = {
        let conn = db.conn.lock().map_err(|e| e.to_string())?;
        let mut stmt = conn.prepare(
            "SELECT id, venta_id, email, intentos FROM email_log WHERE estado = 'PENDIENTE' AND intentos < 4 ORDER BY created_at ASC LIMIT 5"
        ).map_err(|e| e.to_string())?;
        let rows = stmt.query_map([], |row| {
            Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?))
        }).map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect::<Vec<_>>();
        rows
    };

    let total = pendientes.len();
    let mut enviados = 0;
    let mut fallidos = 0;

    for (log_id, venta_id, email, intentos) in pendientes {
        match enviar_email_interno(&db, venta_id, &email).await {
            Ok(_) => {
                let conn = db.conn.lock().map_err(|e| e.to_string())?;
                conn.execute(
                    "UPDATE email_log SET estado = 'ENVIADO', enviado_at = datetime('now','localtime') WHERE id = ?1",
                    rusqlite::params![log_id],
                ).ok();
                enviados += 1;
            }
            Err(err) => {
                let conn = db.conn.lock().map_err(|e| e.to_string())?;
                let nuevo_estado = if intentos + 1 >= 4 { "ERROR" } else { "PENDIENTE" };
                conn.execute(
                    "UPDATE email_log SET estado = ?1, intentos = intentos + 1, ultimo_error = ?2 WHERE id = ?3",
                    rusqlite::params![nuevo_estado, err, log_id],
                ).ok();
                fallidos += 1;
            }
        }
    }

    Ok(serde_json::json!({
        "total": total,
        "enviados": enviados,
        "fallidos": fallidos,
    }))
}

/// Obtiene conteo de emails pendientes.
#[tauri::command]
pub fn obtener_emails_pendientes(db: State<'_, Database>) -> Result<i64, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM email_log WHERE estado = 'PENDIENTE' AND intentos < 4",
        [],
        |row| row.get(0),
    ).unwrap_or(0);
    Ok(count)
}

/// Emite una nota de crédito electrónica al SRI.
#[tauri::command]
pub async fn emitir_nota_credito_sri(
    db: State<'_, Database>,
    nc_id: i64,
) -> Result<ResultadoEmision, String> {
    // 1. Leer NC, factura original, cliente, config, certificado
    let (nc_numero, nc_venta_id, nc_motivo, nc_fecha, factura_numero, factura_fecha,
         cliente_data, detalles_nc, config_data, p12_data, p12_password,
         nc_clave_previa, nc_xml_previo, nc_estado_sri, nc_numero_factura_nc) = {
        let conn = db.conn.lock().map_err(|e| e.to_string())?;

        // Leer nota de crédito
        let (nc_numero, nc_venta_id, nc_motivo, nc_fecha, nc_cliente_id,
             nc_estado, nc_clave, nc_xml, nc_num_factura_nc): (String, i64, String, String, i64,
             String, Option<String>, Option<String>, Option<String>) = conn.query_row(
            "SELECT numero, venta_id, motivo, fecha, cliente_id, estado_sri, clave_acceso, xml_firmado, numero_factura_nc
             FROM notas_credito WHERE id = ?1",
            rusqlite::params![nc_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?,
                       row.get::<_, String>(5).unwrap_or_else(|_| "PENDIENTE".to_string()),
                       row.get(6)?, row.get(7)?,
                       row.get::<_, Option<String>>(8).unwrap_or(None))),
        ).map_err(|e| format!("Nota de credito no encontrada: {}", e))?;

        if nc_estado == "AUTORIZADA" {
            return Err("Esta nota de credito ya fue autorizada por el SRI".to_string());
        }

        // Leer factura original
        let (fac_numero, fac_fecha): (String, String) = conn.query_row(
            "SELECT COALESCE(numero_factura, numero), fecha FROM ventas WHERE id = ?1",
            rusqlite::params![nc_venta_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        ).map_err(|e| format!("Factura original no encontrada: {}", e))?;

        // Leer cliente
        let cliente = conn.query_row(
            "SELECT tipo_identificacion, identificacion, nombre, direccion, email
             FROM clientes WHERE id = ?1",
            rusqlite::params![nc_cliente_id],
            |row| Ok(ClienteSri {
                tipo_identificacion: row.get(0)?,
                identificacion: row.get(1)?,
                nombre: row.get(2)?,
                direccion: row.get(3)?,
                email: row.get(4)?,
            }),
        ).map_err(|e| format!("Cliente no encontrado: {}", e))?;

        // Leer detalles NC
        let mut stmt_det = conn.prepare(
            "SELECT p.codigo, p.nombre, d.cantidad, d.precio_unitario, d.descuento, d.iva_porcentaje
             FROM nota_credito_detalles d
             JOIN productos p ON d.producto_id = p.id
             WHERE d.nota_credito_id = ?1"
        ).map_err(|e| e.to_string())?;

        let detalles: Vec<DetalleSri> = stmt_det.query_map(rusqlite::params![nc_id], |row| {
            Ok(DetalleSri {
                codigo: row.get::<_, Option<String>>(0)?.unwrap_or_else(|| "SIN-COD".to_string()),
                nombre: row.get(1)?,
                cantidad: row.get(2)?,
                precio_unitario: row.get(3)?,
                descuento: row.get(4)?,
                iva_porcentaje: row.get(5)?,
            })
        }).map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

        // Leer config
        let mut config: std::collections::HashMap<String, String> = std::collections::HashMap::new();
        let mut stmt_cfg = conn.prepare("SELECT key, value FROM config").map_err(|e| e.to_string())?;
        let rows = stmt_cfg.query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        }).map_err(|e| e.to_string())?;
        for row in rows {
            let (k, v) = row.map_err(|e| e.to_string())?;
            config.insert(k, v);
        }

        // Leer certificado P12
        let (p12_blob, p12_pass): (Vec<u8>, String) = conn.query_row(
            "SELECT p12_data, password FROM sri_certificado WHERE id = 1",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        ).map_err(|_| "No hay certificado digital cargado.".to_string())?;

        (nc_numero, nc_venta_id, nc_motivo, nc_fecha, fac_numero, fac_fecha,
         cliente, detalles, config, p12_blob, p12_pass,
         nc_clave, nc_xml, nc_estado, nc_num_factura_nc)
    };

    // 2. Preparar datos
    let cfg = |key: &str| -> String {
        config_data.get(key).cloned().unwrap_or_default()
    };

    let ruc = cfg("ruc");
    if ruc.is_empty() || ruc.len() != 13 {
        return Err("Configure el RUC del negocio antes de emitir notas de credito".to_string());
    }

    let ambiente_config = cfg("sri_ambiente");
    let ambiente = match ambiente_config.as_str() {
        "produccion" => "2",
        _ => "1",
    };
    let establecimiento = cfg("establecimiento");
    let punto_emision = cfg("punto_emision");
    let regimen = cfg("regimen");

    let config_key_nc = if ambiente == "1" { "secuencial_nota_credito_pruebas" } else { "secuencial_nota_credito" };

    // --- Lógica reenvío (similar a factura) ---
    let mut secuencial_nc: i64 = 0;
    let mut numero_nc_sri = nc_numero_factura_nc.clone().unwrap_or_default();
    let mut es_primera_emision = false;

    let (clave, xml_firmado_final, resultado_sri) = if nc_estado_sri == "PENDIENTE"
        && nc_clave_previa.is_some()
        && nc_xml_previo.is_some()
    {
        let clave_previa = nc_clave_previa.clone().unwrap();
        let xml_previo = nc_xml_previo.clone().unwrap();
        soap::log_sri(&format!("=== REENVIO NC: Consultando clave previa: {} ===", clave_previa));

        let resultado_consulta = soap::consultar_autorizacion(&clave_previa, ambiente).await;
        match resultado_consulta {
            Ok(ref res) if res.exito => (clave_previa, xml_previo, resultado_consulta.unwrap()),
            _ => {
                soap::log_sri("NC clave previa no autorizada, reenviando...");
                let resultado = soap::enviar_comprobante(&xml_previo, &clave_previa, ambiente).await?;
                (clave_previa, xml_previo, resultado)
            }
        }
    } else {
        // Primera emisión NC
        es_primera_emision = true;

        secuencial_nc = {
            let conn = db.conn.lock().map_err(|e| e.to_string())?;
            conn.query_row(
                "SELECT CAST(value AS INTEGER) FROM config WHERE key = ?1",
                rusqlite::params![config_key_nc],
                |row| row.get(0),
            ).map_err(|e| format!("Error obteniendo secuencial NC: {}", e))?
        };
        let secuencial = format!("{:09}", secuencial_nc);
        numero_nc_sri = format!("{}-{}-{}", establecimiento, punto_emision, secuencial);

        let fecha_emision_nc = formatear_fecha_emision(&nc_fecha)?;
        let fecha_emision_fac = formatear_fecha_emision(&factura_fecha)?;

        let clave_nueva = clave_acceso::generar_clave_acceso(
            &fecha_emision_nc,
            "04", // nota de crédito
            &ruc,
            ambiente,
            &establecimiento,
            &punto_emision,
            &secuencial,
            "1",
        );

        let tipo_id_sri = match cliente_data.tipo_identificacion.as_str() {
            "RUC" => "04",
            "CEDULA" => "05",
            "PASAPORTE" => "06",
            _ => "07",
        };
        let identificacion_comprador = cliente_data.identificacion.clone()
            .unwrap_or_else(|| "9999999999999".to_string());

        // Calcular totales NC
        let mut detalles_xml = Vec::new();
        let mut subtotal_iva_0 = 0.0_f64;
        let mut subtotal_iva_15 = 0.0_f64;
        let mut iva_total = 0.0_f64;

        for det in &detalles_nc {
            let precio_total_sin_imp = det.cantidad * det.precio_unitario - det.descuento;
            let codigo_porcentaje_iva = if det.iva_porcentaje > 0.0 { "4" } else { "0" };
            let tarifa = xml::tarifa_iva(codigo_porcentaje_iva);
            let valor_iva_det = precio_total_sin_imp * (tarifa / 100.0);

            if det.iva_porcentaje > 0.0 {
                subtotal_iva_15 += precio_total_sin_imp;
                iva_total += valor_iva_det;
            } else {
                subtotal_iva_0 += precio_total_sin_imp;
            }

            detalles_xml.push(xml::DetalleFactura {
                codigo_principal: det.codigo.clone(),
                descripcion: det.nombre.clone(),
                cantidad: det.cantidad,
                precio_unitario: det.precio_unitario,
                descuento: det.descuento,
                precio_total_sin_impuesto: precio_total_sin_imp,
                codigo_porcentaje_iva: codigo_porcentaje_iva.to_string(),
                tarifa_iva: tarifa,
                base_imponible: precio_total_sin_imp,
                valor_iva: valor_iva_det,
            });
        }

        let mut impuestos_totales = Vec::new();
        if subtotal_iva_0 > 0.0 {
            impuestos_totales.push(xml::ImpuestoTotal {
                codigo: "2".to_string(),
                codigo_porcentaje: "0".to_string(),
                base_imponible: subtotal_iva_0,
                valor: 0.0,
            });
        }
        if subtotal_iva_15 > 0.0 {
            impuestos_totales.push(xml::ImpuestoTotal {
                codigo: "2".to_string(),
                codigo_porcentaje: "4".to_string(),
                base_imponible: subtotal_iva_15,
                valor: iva_total,
            });
        }

        let total_sin_impuestos = subtotal_iva_0 + subtotal_iva_15;
        let importe_total = total_sin_impuestos + iva_total;

        let contribuyente_rimpe = match regimen.as_str() {
            "RIMPE_EMPRENDEDOR" => Some("CONTRIBUYENTE RÉGIMEN RIMPE".to_string()),
            "RIMPE_POPULAR" => Some("CONTRIBUYENTE NEGOCIO POPULAR - RÉGIMEN RIMPE".to_string()),
            _ => None,
        };

        let mut info_adicional = Vec::new();
        if let Some(ref email) = cliente_data.email {
            if !email.is_empty() {
                info_adicional.push(xml::CampoAdicional { nombre: "email".to_string(), valor: email.clone() });
            }
        }
        if let Some(ref dir) = cliente_data.direccion {
            if !dir.is_empty() {
                info_adicional.push(xml::CampoAdicional { nombre: "direccion".to_string(), valor: dir.clone() });
            }
        }

        let datos_nc = xml::DatosNotaCredito {
            ambiente: ambiente.to_string(),
            tipo_emision: "1".to_string(),
            razon_social: cfg("nombre_negocio"),
            nombre_comercial: cfg("nombre_negocio"),
            ruc: ruc.clone(),
            clave_acceso: clave_nueva.clone(),
            cod_doc: "04".to_string(),
            estab: establecimiento.clone(),
            pto_emi: punto_emision.clone(),
            secuencial: secuencial.clone(),
            dir_matriz: cfg("direccion"),
            contribuyente_rimpe,
            fecha_emision: fecha_emision_nc,
            dir_establecimiento: cfg("direccion"),
            obligado_contabilidad: "NO".to_string(),
            tipo_identificacion_comprador: tipo_id_sri.to_string(),
            razon_social_comprador: cliente_data.nombre.clone(),
            identificacion_comprador,
            cod_doc_modificado: "01".to_string(),
            num_doc_modificado: factura_numero.clone(),
            fecha_emision_doc_sustento: fecha_emision_fac,
            rise: None,
            motivo: nc_motivo.clone(),
            total_sin_impuestos,
            importe_total,
            impuestos_totales,
            detalles: detalles_xml,
            info_adicional,
        };

        let xml_sin_firma = xml::generar_xml_nota_credito(&datos_nc);
        soap::log_sri(&format!("XML NC sin firma ({} bytes)", xml_sin_firma.len()));

        let xml_firmado_result = firma::firmar_comprobante(
            &xml_sin_firma,
            &p12_data,
            &p12_password,
            "notaCredito",
        )?;

        let resultado = soap::enviar_comprobante(
            &xml_firmado_result.xml,
            &clave_nueva,
            ambiente,
        ).await?;

        (clave_nueva, xml_firmado_result.xml, resultado)
    };

    // 3. Actualizar NC en BD
    {
        let conn = db.conn.lock().map_err(|e| e.to_string())?;

        let nuevo_estado = if resultado_sri.exito {
            "AUTORIZADA"
        } else if resultado_sri.estado == "EN_PROCESO" {
            "PENDIENTE"
        } else {
            "RECHAZADA"
        };

        let xml_para_guardar = if resultado_sri.exito || resultado_sri.estado == "EN_PROCESO" {
            Some(xml_firmado_final.clone())
        } else {
            None
        };

        let nf_para_guardar = if !numero_nc_sri.is_empty() {
            Some(numero_nc_sri.clone())
        } else {
            nc_numero_factura_nc.clone()
        };

        conn.execute(
            "UPDATE notas_credito SET estado_sri = ?1, clave_acceso = ?2,
             autorizacion_sri = ?3, xml_firmado = ?4, numero_factura_nc = ?5
             WHERE id = ?6",
            rusqlite::params![
                nuevo_estado,
                clave.clone(),
                resultado_sri.numero_autorizacion,
                xml_para_guardar,
                nf_para_guardar,
                nc_id,
            ],
        ).map_err(|e| format!("Error actualizando NC: {}", e))?;

        if resultado_sri.exito && es_primera_emision {
            conn.execute(
                "UPDATE config SET value = CAST(?1 AS TEXT) WHERE key = ?2",
                rusqlite::params![secuencial_nc + 1, config_key_nc],
            ).ok();
        }
    }

    Ok(ResultadoEmision {
        exito: resultado_sri.exito,
        estado_sri: resultado_sri.estado.clone(),
        clave_acceso: Some(clave),
        numero_autorizacion: resultado_sri.numero_autorizacion,
        fecha_autorizacion: resultado_sri.fecha_autorizacion,
        mensaje: resultado_sri.mensaje.unwrap_or_else(|| {
            if resultado_sri.exito {
                "Nota de credito autorizada correctamente".to_string()
            } else {
                format!("Estado: {}", resultado_sri.estado)
            }
        }),
        numero_factura: if !numero_nc_sri.is_empty() { Some(numero_nc_sri) } else { None },
    })
}

/// Genera RIDE PDF para una nota de crédito autorizada.
#[tauri::command]
pub fn generar_ride_nc_pdf(
    db: State<Database>,
    nc_id: i64,
) -> Result<String, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;

    // Leer NC
    let (nc_numero, nc_venta_id, nc_motivo, nc_fecha, nc_cliente_id,
         nc_estado_sri, nc_autorizacion, nc_clave, nc_total,
         nc_subtotal_sin_iva, nc_subtotal_con_iva, nc_iva, nc_numero_factura_nc) = conn.query_row(
        "SELECT numero, venta_id, motivo, fecha, cliente_id, estado_sri,
         autorizacion_sri, clave_acceso, total, subtotal_sin_iva, subtotal_con_iva, iva, numero_factura_nc
         FROM notas_credito WHERE id = ?1",
        rusqlite::params![nc_id],
        |row| Ok((
            row.get::<_, String>(0)?,
            row.get::<_, i64>(1)?,
            row.get::<_, String>(2)?,
            row.get::<_, String>(3)?,
            row.get::<_, i64>(4)?,
            row.get::<_, String>(5).unwrap_or_else(|_| "PENDIENTE".to_string()),
            row.get::<_, Option<String>>(6)?,
            row.get::<_, Option<String>>(7)?,
            row.get::<_, f64>(8)?,
            row.get::<_, f64>(9)?,
            row.get::<_, f64>(10)?,
            row.get::<_, f64>(11)?,
            row.get::<_, Option<String>>(12)?,
        )),
    ).map_err(|e| format!("Nota de credito no encontrada: {}", e))?;

    // Leer factura original (número y fecha)
    let (fac_numero, fac_fecha): (String, String) = conn.query_row(
        "SELECT COALESCE(numero_factura, numero), fecha FROM ventas WHERE id = ?1",
        rusqlite::params![nc_venta_id],
        |row| Ok((row.get(0)?, row.get(1)?)),
    ).map_err(|e| format!("Factura original no encontrada: {}", e))?;

    let fecha_fac_sri = formatear_fecha_emision(&fac_fecha).unwrap_or(fac_fecha.clone());

    // Construir VentaCompleta "fake" para reutilizar generar_ride_pdf
    let venta_fake = crate::models::Venta {
        id: Some(nc_id),
        numero: nc_numero.clone(),
        cliente_id: Some(nc_cliente_id),
        fecha: Some(nc_fecha.clone()),
        subtotal_sin_iva: nc_subtotal_sin_iva,
        subtotal_con_iva: nc_subtotal_con_iva,
        descuento: 0.0,
        iva: nc_iva,
        total: nc_total,
        forma_pago: String::new(),
        monto_recibido: 0.0,
        cambio: 0.0,
        estado: "COMPLETADA".to_string(),
        tipo_documento: "NOTA_CREDITO".to_string(),
        estado_sri: nc_estado_sri,
        autorizacion_sri: nc_autorizacion,
        clave_acceso: nc_clave,
        observacion: Some(nc_motivo.clone()),
        numero_factura: nc_numero_factura_nc,
    };

    // Detalles
    let mut stmt = conn.prepare(
        "SELECT p.codigo, p.nombre, d.cantidad, d.precio_unitario, d.descuento, d.iva_porcentaje, d.subtotal, d.producto_id
         FROM nota_credito_detalles d
         JOIN productos p ON d.producto_id = p.id
         WHERE d.nota_credito_id = ?1"
    ).map_err(|e| e.to_string())?;

    let detalles_rows: Vec<(crate::models::VentaDetalle, String)> = stmt.query_map(
        rusqlite::params![nc_id],
        |row| {
            let det = crate::models::VentaDetalle {
                id: None,
                venta_id: Some(nc_id),
                producto_id: row.get(7)?,
                nombre_producto: Some(row.get(1)?),
                cantidad: row.get(2)?,
                precio_unitario: row.get(3)?,
                descuento: row.get(4)?,
                iva_porcentaje: row.get(5)?,
                subtotal: row.get(6)?,
            };
            let codigo: String = row.get::<_, Option<String>>(0)?.unwrap_or_else(|| "SIN-COD".to_string());
            Ok((det, codigo))
        },
    ).map_err(|e| e.to_string())?
    .collect::<Result<Vec<_>, _>>()
    .map_err(|e| e.to_string())?;

    let detalles: Vec<crate::models::VentaDetalle> = detalles_rows.iter().map(|(d, _)| d.clone()).collect();
    let detalles_ride: Vec<crate::sri::ride::DetalleRide> = detalles_rows.iter().map(|(det, codigo)| {
        let precio_total = det.cantidad * det.precio_unitario - det.descuento;
        crate::sri::ride::DetalleRide {
            codigo: codigo.clone(),
            nombre: det.nombre_producto.clone().unwrap_or_else(|| "?".to_string()),
            cantidad: det.cantidad,
            precio_unitario: det.precio_unitario,
            descuento: det.descuento,
            iva_porcentaje: det.iva_porcentaje,
            precio_total_sin_impuesto: precio_total,
        }
    }).collect();

    // Cliente
    let cliente_nombre: String = conn.query_row(
        "SELECT nombre FROM clientes WHERE id = ?1",
        rusqlite::params![nc_cliente_id],
        |row| row.get(0),
    ).unwrap_or_else(|_| "CONSUMIDOR FINAL".to_string());

    let (cli_ident, cli_dir, cli_email, cli_tel) = conn.query_row(
        "SELECT COALESCE(identificacion,''), COALESCE(direccion,''), COALESCE(email,''), COALESCE(telefono,'') FROM clientes WHERE id = ?1",
        rusqlite::params![nc_cliente_id],
        |row| Ok((row.get::<_,String>(0)?, row.get::<_,String>(1)?, row.get::<_,String>(2)?, row.get::<_,String>(3)?)),
    ).unwrap_or_default();

    // Config
    let mut cfg_stmt = conn.prepare("SELECT key, value FROM config").map_err(|e| e.to_string())?;
    let config_map: std::collections::HashMap<String, String> = cfg_stmt
        .query_map([], |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)))
        .map_err(|e| e.to_string())?
        .collect::<Result<std::collections::HashMap<_, _>, _>>()
        .map_err(|e| e.to_string())?;

    drop(stmt);
    drop(cfg_stmt);
    drop(conn);

    let venta_completa = crate::models::VentaCompleta {
        venta: venta_fake,
        detalles,
        cliente_nombre: Some(cliente_nombre.clone()),
    };

    let cliente_ride = crate::sri::ride::ClienteRide {
        nombre: cliente_nombre,
        identificacion: cli_ident,
        direccion: cli_dir,
        email: cli_email,
        telefono: cli_tel,
        observacion: nc_motivo.clone(),
    };

    let doc_mod = crate::sri::ride::DocModificado {
        tipo: "01".to_string(),
        numero: fac_numero,
        fecha_emision: fecha_fac_sri,
        motivo: nc_motivo,
    };

    let pdf_bytes = crate::sri::ride::generar_ride_pdf(
        &venta_completa,
        &detalles_ride,
        &cliente_ride,
        &config_map,
        None, // fecha_autorizacion se toma de la venta
        "NOTA DE CREDITO",
        Some(&doc_mod),
    )?;

    let temp_dir = std::env::temp_dir();
    let num_for_file = venta_completa.venta.numero_factura.as_deref()
        .unwrap_or(&venta_completa.venta.numero);
    let filename = format!("RIDE-NC-{}.pdf", num_for_file.replace(['/', '\\', ':'], "-"));
    let pdf_path = temp_dir.join(&filename);
    std::fs::write(&pdf_path, &pdf_bytes)
        .map_err(|e| format!("Error guardando PDF: {}", e))?;

    // Abrir en visor del sistema
    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("cmd")
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

    Ok(pdf_path.to_string_lossy().to_string())
}

/// Formatea fecha de BD (yyyy-mm-dd HH:MM:SS) a formato SRI (dd/mm/yyyy)
fn formatear_fecha_emision(fecha_bd: &str) -> Result<String, String> {
    // La fecha viene de SQLite como "2026-02-11 15:30:00" o "2026-02-11"
    let fecha_parte = fecha_bd.split(' ').next().unwrap_or(fecha_bd);
    let partes: Vec<&str> = fecha_parte.split('-').collect();

    if partes.len() != 3 {
        return Err(format!("Formato de fecha invalido: {}", fecha_bd));
    }

    Ok(format!("{}/{}/{}", partes[2], partes[1], partes[0]))
}
