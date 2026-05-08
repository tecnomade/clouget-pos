//! Discovery automático del servidor POS para la app móvil.
//!
//! Usa **mDNS** (Multicast DNS) para que el servidor se anuncie en la red local
//! como `_clouget-pos._tcp.local.`. La app móvil escanea con `mdns-sd` (o
//! `react-native-zeroconf`) y encuentra automáticamente todos los POS de Clouget
//! en la LAN sin pedirle al usuario que escriba IP/puerto.
//!
//! El servicio publica las propiedades:
//! - `negocio` — nombre del negocio (`config.nombre_negocio`)
//! - `version` — versión del POS (`CARGO_PKG_VERSION`)
//! - `restaurante` — `1` si el módulo está activo, `0` si no
//! - `app_movil` — `1` si el módulo está activo, `0` si no
//!
//! Hostname publicado: `clouget-pos-<8chars>.local.` (estable por instancia para
//! que la app pueda recordar y reconectar).

use mdns_sd::{ServiceDaemon, ServiceInfo};
use std::collections::HashMap;

const SERVICE_TYPE: &str = "_clouget-pos._tcp.local.";

/// Inicia el broadcast mDNS en background. Se llama una vez al arrancar el server.
///
/// Ignora errores si la red no soporta mDNS (ej. interfaz sin multicast). El
/// flujo nunca falla — la app puede caer al método manual de IP/puerto.
pub fn start_broadcast(
    instance_name: &str,
    port: u16,
    nombre_negocio: &str,
    tiene_restaurante: bool,
    tiene_app_movil: bool,
) {
    let instance = instance_name.to_string();
    let negocio = nombre_negocio.to_string();
    std::thread::spawn(move || {
        let daemon = match ServiceDaemon::new() {
            Ok(d) => d,
            Err(e) => {
                eprintln!("[mDNS] No se pudo iniciar daemon: {}", e);
                return;
            }
        };

        // IP local — la app la usa para conectarse vía HTTP
        let ip_str = match local_ip_address::local_ip() {
            Ok(ip) => ip.to_string(),
            Err(e) => {
                eprintln!("[mDNS] No se pudo obtener IP local: {}", e);
                return;
            }
        };

        // Hostname mDNS — debe terminar en `.local.`
        let host_short = instance.replace(' ', "-").to_lowercase();
        let host_short = host_short.chars().take(20).collect::<String>();
        let hostname = format!("clouget-pos-{}.local.", host_short);

        // Propiedades TXT
        let mut props: HashMap<String, String> = HashMap::new();
        props.insert("negocio".to_string(), negocio);
        props.insert("version".to_string(), env!("CARGO_PKG_VERSION").to_string());
        props.insert(
            "restaurante".to_string(),
            (if tiene_restaurante { "1" } else { "0" }).to_string(),
        );
        props.insert(
            "app_movil".to_string(),
            (if tiene_app_movil { "1" } else { "0" }).to_string(),
        );
        props.insert("api".to_string(), "/api/v1/app".to_string());

        let service = match ServiceInfo::new(
            SERVICE_TYPE,
            &instance,
            &hostname,
            ip_str.as_str(),
            port,
            Some(props),
        ) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("[mDNS] Error creando ServiceInfo: {}", e);
                return;
            }
        };

        match daemon.register(service) {
            Ok(_) => {
                eprintln!(
                    "[mDNS] Servicio registrado: {} en {}:{} ({})",
                    instance, ip_str, port, hostname
                );
            }
            Err(e) => {
                eprintln!("[mDNS] Error al registrar servicio: {}", e);
                return;
            }
        }

        // El daemon necesita seguir vivo. Bloqueamos este hilo indefinidamente
        // (es un thread spawned dedicado, no impacta el resto).
        std::thread::park();
    });
}

/// Devuelve la IP local IPv4 — la app la necesita para construir URLs.
/// Si no se puede determinar (ej. PC sin red), devuelve `None`.
pub fn obtener_ip_local() -> Option<String> {
    local_ip_address::local_ip().ok().map(|ip| ip.to_string())
}
