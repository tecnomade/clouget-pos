mod backup;
mod branding;
mod commands;
mod db;
mod models;
mod offline;
mod printing;
mod restaurante;
mod app_movil;
mod server;
mod sri;
pub mod utils;

use tauri::Manager;
use tauri_plugin_updater::UpdaterExt;

use db::{Database, SesionState};
use std::sync::{Arc, Mutex};

/// Comando custom: verifica e instala update desde endpoint dinamico (segun canal).
/// El plugin oficial solo lee endpoints estaticos del tauri.conf.json.
///
/// v2.5.10: si canal=beta consulta AMBOS endpoints (beta + stable) y toma la version
/// MAS ALTA. Antes solo consultaba beta, asi que un usuario en canal beta no recibia
/// versiones stable nuevas si no habia una beta posterior — quedaba atras del stable.
#[tauri::command]
async fn verificar_update_canal(app: tauri::AppHandle, canal: String) -> Result<Option<String>, String> {
    let endpoint_base = "https://zakquzflkvfqflqnxpxj.supabase.co/functions/v1/update-manifest";

    // Helper: parsear "X.Y.Z" a tupla (mayor, menor, parche) para comparar versiones.
    // Si falla el parseo, devuelve (0,0,0) que pierde contra cualquier version real.
    fn parse_version(v: &str) -> (u32, u32, u32) {
        let parts: Vec<&str> = v.trim_start_matches('v').split(|c| c == '.' || c == '-').collect();
        let major = parts.first().and_then(|s| s.parse().ok()).unwrap_or(0);
        let minor = parts.get(1).and_then(|s| s.parse().ok()).unwrap_or(0);
        let patch = parts.get(2).and_then(|s| s.parse().ok()).unwrap_or(0);
        (major, minor, patch)
    }

    // Lista de endpoints a consultar segun canal:
    // - stable: solo el endpoint stable
    // - beta: stable Y beta (asi el usuario beta nunca pierde versiones estables nuevas)
    let endpoints_a_consultar: Vec<String> = if canal == "beta" {
        vec![
            format!("{}?canal=stable", endpoint_base),
            format!("{}?canal=beta", endpoint_base),
        ]
    } else {
        vec![format!("{}?canal=stable", endpoint_base)]
    };

    // Consultar cada endpoint por separado, buscar el update con version mas alta
    let mut mejor_update: Option<tauri_plugin_updater::Update> = None;
    let mut mejor_version_tuple = (0u32, 0u32, 0u32);

    for ep_url in &endpoints_a_consultar {
        let url = match url::Url::parse(ep_url) {
            Ok(u) => u,
            Err(_) => continue,
        };
        let updater_one = match app.updater_builder()
            .endpoints(vec![url])
            .and_then(|b| b.build())
        {
            Ok(u) => u,
            Err(_) => continue,
        };
        // check() puede devolver Err si el endpoint no responde — ignorar ese endpoint
        // y seguir con los demas (no abortar todo el chequeo por un endpoint caido)
        match updater_one.check().await {
            Ok(Some(upd)) => {
                let v = parse_version(&upd.version);
                if v > mejor_version_tuple {
                    mejor_version_tuple = v;
                    mejor_update = Some(upd);
                }
            }
            Ok(None) => { /* sin update en este endpoint, seguir */ }
            Err(e) => {
                eprintln!("[Updater {}] error consultando {}: {}", canal, ep_url, e);
            }
        }
    }

    match mejor_update {
        Some(update) => {
            let new_version = update.version.clone();
            update.download_and_install(|_, _| {}, || {}).await
                .map_err(|e| e.to_string())?;
            Ok(Some(new_version))
        }
        None => Ok(None),
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let database = Database::new().expect("Error al inicializar la base de datos");

    // Inicializar módulos opcionales según brand.
    // El módulo Restaurante solo se carga en builds Clouget (no DigitalServer).
    if branding::BRAND.tiene_modulo_restaurante() {
        if let Err(e) = restaurante::init(&database) {
            eprintln!("[Restaurante] Error al inicializar módulo: {}", e);
        }
    }

    // v2.4.2 — Sprint 3a: módulo App Móvil. Disponible en ambas marcas
    // (Clouget y DigitalServer). Crea la tabla `app_tokens`.
    if branding::BRAND.tiene_modulo_app_movil() {
        if let Err(e) = app_movil::init(&database) {
            eprintln!("[App Móvil] Error al inicializar módulo: {}", e);
        }
    }

    // v2.4.8 — Auto-migración de licencia para clientes con órdenes preexistentes.
    // Si la base de datos tiene órdenes de servicio Y la licencia local NO incluye
    // `servicio_tecnico` (porque antes de v2.4.8 era parte de la licencia base),
    // agregarlo automáticamente para no romper a clientes existentes. Idempotente:
    // si ya está incluido, no hace nada.
    {
        let conn = database.conn.lock().unwrap();
        let modulos_actuales: String = conn
            .query_row(
                "SELECT value FROM config WHERE key = 'licencia_modulos'",
                [],
                |row| row.get(0),
            )
            .unwrap_or_default();
        let mut modulos: Vec<String> =
            serde_json::from_str(&modulos_actuales).unwrap_or_default();
        if !modulos.iter().any(|m| m == "servicio_tecnico") {
            let count_ordenes: i64 = conn
                .query_row("SELECT COUNT(*) FROM ordenes_servicio", [], |row| row.get(0))
                .unwrap_or(0);
            if count_ordenes > 0 {
                modulos.push("servicio_tecnico".to_string());
                let nuevo_json = serde_json::to_string(&modulos).unwrap_or_else(|_| "[]".to_string());
                let _ = conn.execute(
                    "INSERT OR REPLACE INTO config (key, value) VALUES ('licencia_modulos', ?1)",
                    rusqlite::params![&nuevo_json],
                );
                eprintln!(
                    "[Migration v2.4.8] Modulo 'servicio_tecnico' agregado automaticamente a la licencia local ({} ordenes preexistentes detectadas)",
                    count_ordenes
                );
            }
        }
    }

    let sesion_state = SesionState {
        sesion: Arc::new(Mutex::new(None)),
    };

    // Leer config de red antes de pasar ownership a Tauri
    let (modo_red, servidor_puerto, servidor_token) = {
        let conn = database.conn.lock().unwrap();
        let get = |key: &str, default: &str| -> String {
            conn.query_row(
                "SELECT value FROM config WHERE key = ?1",
                rusqlite::params![key],
                |row| row.get(0),
            )
            .unwrap_or_else(|_| default.to_string())
        };
        (
            get("modo_red", "local"),
            get("servidor_puerto", "8847").parse::<u16>().unwrap_or(8847),
            get("servidor_token", ""),
        )
    };

    // v2.4.4 — Iniciar servidor HTTP si:
    //   (a) modo Multi-POS server con token configurado, O
    //   (b) la licencia tiene el módulo `app_movil`
    // El server hospeda ambas APIs (`/api/v1/invoke` para Multi-POS y
    // `/api/v1/app/*` para la app móvil), pero `/invoke` solo se monta si
    // `servidor_token` tiene valor (sino sería un endpoint sin auth).
    let modulos_actuales: String = {
        let conn = database.conn.lock().unwrap();
        conn.query_row(
            "SELECT value FROM config WHERE key = 'licencia_modulos'",
            [],
            |row| row.get(0),
        )
        .unwrap_or_default()
    };
    let licencia_tiene_app_movil = modulos_actuales.contains("app_movil");
    let server_modo_multipos = modo_red == "servidor" && !servidor_token.is_empty();
    let arrancar_server = server_modo_multipos || licencia_tiene_app_movil;

    if arrancar_server {
        server::start_server(
            database.clone(),
            sesion_state.clone(),
            servidor_puerto,
            servidor_token.clone(),
        );

        // mDNS broadcast: solo si tiene app_movil (no tiene sentido para
        // Multi-POS porque el cliente Tauri ya conoce la IP por config)
        if licencia_tiene_app_movil {
            let nombre_negocio: String = {
                let conn = database.conn.lock().unwrap();
                conn.query_row(
                    "SELECT value FROM config WHERE key = 'nombre_negocio'",
                    [],
                    |row| row.get(0),
                )
                .unwrap_or_else(|_| "Clouget POS".to_string())
            };
            let tiene_restaurante = modulos_actuales.contains("restaurante");
            app_movil::discovery::start_broadcast(
                &nombre_negocio,
                servidor_puerto,
                &nombre_negocio,
                tiene_restaurante,
                true,
            );
        }
    }

    // Inicializar BD offline en modo cliente (para cache y cola)
    let offline_db: Option<offline::OfflineDb> = if modo_red == "cliente" {
        offline::OfflineDb::new().ok()
    } else {
        None
    };

    // Iniciar scheduler de backup automático (solo en modo local o servidor)
    if modo_red != "cliente" {
        backup::scheduler::start_backup_scheduler(database.clone());
    }

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            // Focus the existing window when a second instance is launched
            if let Some(w) = app.get_webview_window("main") {
                let _ = w.set_focus();
                let _ = w.unminimize();
            }
        }))
        .setup(|app| {
            #[cfg(desktop)]
            app.handle().plugin(tauri_plugin_updater::Builder::new().build())?;
            Ok(())
        })
        .manage(database)
        .manage(sesion_state)
        .manage(offline_db)
        .invoke_handler(tauri::generate_handler![
            verificar_update_canal,
            // Productos
            commands::productos::crear_producto,
            commands::productos::actualizar_producto,
            commands::productos::buscar_productos,
            commands::productos::obtener_producto,
            commands::productos::listar_productos,
            commands::productos::productos_mas_vendidos,
            commands::productos::crear_categoria,
            commands::productos::actualizar_categoria,
            commands::productos::eliminar_categoria,
            commands::productos::listar_categorias,
            commands::productos::listar_tipos_unidad,
            commands::productos::crear_tipo_unidad,
            commands::productos::actualizar_tipo_unidad,
            commands::productos::eliminar_tipo_unidad,
            commands::productos::cargar_imagen_producto,
            commands::productos::leer_imagen_archivo,
            commands::productos::guardar_imagen_producto_b64, // v2.4.1 — paste/drag&drop
            commands::productos::eliminar_imagen_producto,
            commands::productos::listar_productos_tactil,
            commands::productos::exportar_plantilla_productos,
            commands::productos::exportar_productos_excel,
            commands::productos::importar_productos_excel,
            commands::productos::eliminar_producto,
            commands::productos::registrar_series,
            commands::productos::listar_series_producto,
            commands::productos::series_disponibles,
            commands::productos::marcar_serie_vendida,
            commands::productos::buscar_serie,
            commands::productos::devolver_serie,
            commands::productos::registrar_lote_caducidad,
            commands::productos::listar_lotes_producto,
            commands::productos::alertas_caducidad,
            commands::productos::listar_todos_lotes,
            commands::combos::listar_combo_grupos,
            commands::combos::listar_combo_componentes,
            commands::combos::guardar_combo_estructura,
            commands::combos::stock_combo,
            commands::combos::info_combo_resumen,
            commands::productos::eliminar_lote_caducidad,
            commands::productos::ajustar_cantidad_lote,
            commands::productos::reparar_fechas_caducidad,
            commands::productos::listar_unidades_producto,
            commands::productos::guardar_unidades_producto,
            // Clientes
            commands::clientes::crear_cliente,
            commands::clientes::actualizar_cliente,
            commands::clientes::buscar_clientes,
            commands::clientes::listar_clientes,
            commands::clientes::consultar_identificacion,
            // Ventas
            commands::ventas::registrar_venta,
            commands::ventas::listar_pagos_venta,
            commands::ventas::listar_ventas_dia,
            commands::ventas::obtener_venta,
            commands::ventas::registrar_nota_credito,
            commands::ventas::obtener_nota_credito,
            commands::ventas::crear_devolucion_interna,
            commands::ventas::anular_venta,
            commands::ventas::listar_notas_credito_dia,
            commands::ventas::listar_notas_credito,
            commands::ventas::listar_ventas_sesion_caja,
            commands::ventas::resumen_sesion_caja,
            commands::ventas::listar_notas_credito_sesion_caja,
            commands::ventas::guardar_borrador,
            commands::ventas::guardar_cotizacion,
            commands::ventas::eliminar_borrador,
            commands::ventas::listar_documentos_recientes,
            commands::ventas::guardar_guia_remision,
            commands::ventas::listar_guias_remision,
            commands::ventas::resumen_guias_remision,
            commands::ventas::convertir_guia_a_venta,
            commands::ventas::listar_choferes,
            commands::ventas::guardar_chofer,
            commands::ventas::listar_vehiculos,
            commands::ventas::guardar_vehiculo,
            commands::ventas::listar_direcciones_cliente,
            commands::ventas::guardar_direccion_cliente,
            commands::ventas::eliminar_direccion_cliente,
            commands::ventas::cambiar_estado_guia,
            // Caja
            commands::caja::abrir_caja,
            commands::caja::cerrar_caja,
            commands::caja::obtener_caja_abierta,
            commands::caja::obtener_ultimo_cierre,
            commands::caja::listar_eventos_caja,
            commands::caja::historial_descuadres_caja,
            commands::caja::listar_sesiones_caja,
            commands::caja::registrar_deposito_cierre,
            commands::caja::registrar_retiro,
            commands::caja::registrar_ingreso_caja,
            commands::caja::listar_retiros_caja,
            commands::caja::confirmar_deposito,
            // Configuración
            commands::config::obtener_config,
            commands::config::guardar_config,
            commands::config::cargar_logo_negocio,
            commands::config::eliminar_logo_negocio,
            commands::config::generar_token_servidor,
            commands::config::obtener_secuenciales,
            commands::config::actualizar_secuencial,
            commands::config::probar_conexion_servidor,
            commands::config::resetear_base_datos,
            // Impresión
            commands::impresion::imprimir_ticket,
            commands::impresion::imprimir_ticket_pdf,
            commands::impresion::imprimir_reporte_caja,
            commands::impresion::imprimir_reporte_caja_pdf,
            commands::impresion::obtener_resumen_caja,
            commands::impresion::listar_impresoras,
            commands::impresion::listar_impresoras_cached,
            commands::impresion::refrescar_impresoras,
            commands::impresion::imprimir_guia_remision_pdf,
            commands::impresion::imprimir_ticket_nc,
            // Reportes
            commands::reportes::resumen_diario,
            commands::reportes::productos_mas_vendidos_reporte,
            commands::reportes::alertas_stock_bajo,
            commands::reportes::resumen_fiados_pendientes,
            commands::reportes::resumen_periodo,
            commands::reportes::listar_ventas_periodo,
            commands::reportes::ventas_por_dia,
            commands::reportes::resumen_diario_ayer,
            commands::reportes::ultimas_ventas_dia,
            commands::reportes::reporte_utilidad,
            commands::reportes::reporte_balance,
            commands::reportes::reporte_productos_rentabilidad,
            commands::reportes::listar_libro_movimientos,
            commands::reportes::reporte_iva_mensual,
            commands::reportes::reporte_cxc_por_cliente,
            commands::reportes::reporte_cxc_detalle_cliente,
            commands::reportes::reporte_cxp_por_proveedor,
            commands::reportes::reporte_cxp_detalle_proveedor,
            commands::reportes::reporte_inventario_valorizado,
            commands::reportes::reporte_kardex_producto,
            commands::reportes::reporte_kardex_multi,
            commands::reportes::reporte_valuacion_inventario,
            commands::reportes::reporte_ventas_por_cajero,
            commands::reportes::reporte_ventas_filtrable,
            commands::reportes::reporte_ventas_filtros_disponibles,
            commands::reportes::listar_categorias_simple,
            // Gastos
            commands::gastos::crear_gasto,
            commands::gastos::listar_gastos_dia,
            commands::gastos::eliminar_gasto,
            // Cuentas por cobrar
            commands::cuentas::resumen_deudores,
            commands::cuentas::listar_cuentas_pendientes,
            commands::cuentas::obtener_cuenta_detalle,
            commands::cuentas::registrar_pago_cuenta,
            commands::cuentas::listar_cuentas_banco,
            commands::cuentas::crear_cuenta_banco,
            commands::cuentas::actualizar_cuenta_banco,
            commands::cuentas::desactivar_cuenta_banco,
            commands::cuentas::confirmar_pago_cuenta,
            commands::cuentas::rechazar_pago_cuenta,
            commands::cuentas::contar_pagos_pendientes,
            commands::cuentas::listar_pagos_pendientes_confirmacion,
            // Respaldo
            commands::respaldo::obtener_ruta_db,
            commands::respaldo::crear_respaldo,
            commands::respaldo::restaurar_respaldo,
            // Licencia
            commands::licencia::obtener_machine_id,
            commands::licencia::verificar_licencia,
            commands::licencia::obtener_estado_licencia,
            // Usuarios / Sesión
            commands::usuarios::iniciar_sesion,
            commands::usuarios::cerrar_sesion,
            commands::usuarios::obtener_sesion_actual,
            commands::usuarios::crear_usuario,
            commands::usuarios::listar_usuarios,
            commands::usuarios::actualizar_usuario,
            commands::usuarios::eliminar_usuario,
            commands::usuarios::verificar_pin_admin,
            commands::usuarios::obtener_permisos_disponibles,
            commands::usuarios::cambiar_password,
            commands::usuarios::listar_usuarios_login,
            // Exportar CSV
            commands::exportar::exportar_ventas_csv,
            commands::exportar::exportar_gastos_csv,
            commands::exportar::exportar_inventario_csv,
            commands::exportar::guardar_archivo_texto,
            commands::exportar::exportar_inventario_xlsx,
            commands::exportar::exportar_inventario_pdf,
            commands::exportar::exportar_tabla_xlsx,
            commands::exportar::exportar_tabla_pdf,
            // SRI - Facturación Electrónica
            commands::sri::cargar_certificado_sri,
            commands::sri::emitir_factura_sri,
            commands::sri::consultar_estado_sri,
            commands::sri::cambiar_ambiente_sri,
            commands::sri::validar_suscripcion_sri,
            commands::sri::obtener_planes_sri,
            commands::sri::crear_pedido_sri,
            commands::sri::obtener_xml_firmado,
            commands::sri::generar_ride_pdf,
            commands::sri::imprimir_ride,
            commands::sri::enviar_notificacion_sri,
            commands::sri::procesar_emails_pendientes,
            commands::sri::obtener_emails_pendientes,
            commands::sri::emitir_nota_credito_sri,
            commands::sri::generar_ride_nc_pdf,
            // Listas de precios
            commands::listas_precios::listar_listas_precios,
            commands::listas_precios::crear_lista_precio,
            commands::listas_precios::actualizar_lista_precio,
            commands::listas_precios::establecer_lista_default,
            commands::listas_precios::guardar_precios_producto,
            commands::listas_precios::obtener_precios_producto,
            commands::listas_precios::resolver_precio_producto,
            // Inventario / Kardex
            commands::inventario::registrar_movimiento,
            commands::inventario::listar_movimientos,
            commands::inventario::resumen_inventario,
            commands::inventario::exportar_kardex_csv,
            // Demo
            commands::demo::activar_demo,
            commands::demo::salir_demo,
            commands::demo::es_demo,
            // Establecimientos y Puntos de Emisión
            commands::establecimientos::listar_establecimientos,
            commands::establecimientos::crear_establecimiento,
            commands::establecimientos::actualizar_establecimiento,
            commands::establecimientos::listar_puntos_emision,
            commands::establecimientos::crear_punto_emision,
            commands::establecimientos::actualizar_punto_emision,
            // Transferencias y Multi-almacén
            commands::transferencias::crear_transferencia,
            commands::transferencias::recibir_transferencia,
            commands::transferencias::listar_transferencias,
            commands::transferencias::stock_por_establecimiento,
            commands::transferencias::actualizar_stock_establecimiento,
            // Etiquetas de productos
            commands::etiquetas::generar_etiquetas_pdf,
            // Cotización PDF
            commands::cotizacion_pdf::generar_cotizacion_pdf,
            // Nota de Venta PDF
            commands::nota_venta_pdf::generar_nota_venta_pdf,
            // Proveedores
            commands::proveedores::crear_proveedor,
            commands::proveedores::actualizar_proveedor,
            commands::proveedores::listar_proveedores,
            commands::proveedores::buscar_proveedores,
            commands::proveedores::eliminar_proveedor,
            // Compras
            commands::compras::registrar_compra,
            commands::compras::listar_compras,
            commands::compras::obtener_compra,
            commands::compras::anular_compra,
            commands::compras::preview_xml_compra,
            commands::compras::importar_xml_compra,
            commands::compras::registrar_devolucion_compra,
            commands::compras::listar_devoluciones_compra,
            // Cuentas por pagar
            commands::cuentas_pagar::alertas_pagos_vencidos,
            commands::cuentas_pagar::resumen_acreedores,
            commands::cuentas_pagar::listar_cuentas_pagar,
            commands::cuentas_pagar::registrar_pago_proveedor,
            commands::cuentas_pagar::historial_pagos_proveedor,
            commands::cuentas_pagar::listar_movimientos_bancarios,
            commands::cuentas_pagar::obtener_detalle_movimiento_bancario,
            // Verificacion de transferencias (admin)
            commands::verificacion::listar_transferencias_verificacion,
            commands::verificacion::verificar_transferencia,
            commands::verificacion::contar_transferencias_pendientes,
            commands::verificacion::detalle_transferencias_pendientes,
            commands::verificacion::forzar_marcar_transferencia_verificada,
            // Servicio Técnico
            commands::servicio_tecnico::crear_orden_servicio,
            commands::servicio_tecnico::actualizar_orden_servicio,
            commands::servicio_tecnico::cambiar_estado_orden,
            commands::servicio_tecnico::obtener_orden_servicio,
            commands::servicio_tecnico::listar_ordenes_servicio,
            commands::servicio_tecnico::buscar_ordenes_por_equipo,
            commands::servicio_tecnico::historial_movimientos_orden,
            commands::servicio_tecnico::eliminar_orden_servicio,
            commands::servicio_tecnico::agregar_imagen_orden,
            commands::servicio_tecnico::listar_imagenes_orden,
            commands::servicio_tecnico::eliminar_imagen_orden,
            commands::servicio_tecnico::cobrar_orden_servicio,
            commands::servicio_tecnico::imprimir_orden_servicio_pdf,
            // ST-2 (v2.4.9): Catálogo jerárquico equipos/marcas/modelos + historial
            commands::servicio_tecnico_catalogo::st_listar_tipos_equipo,
            commands::servicio_tecnico_catalogo::st_crear_tipo_equipo,
            commands::servicio_tecnico_catalogo::st_actualizar_tipo_equipo,
            commands::servicio_tecnico_catalogo::st_eliminar_tipo_equipo,
            commands::servicio_tecnico_catalogo::st_listar_marcas,
            commands::servicio_tecnico_catalogo::st_crear_marca,
            commands::servicio_tecnico_catalogo::st_actualizar_marca,
            commands::servicio_tecnico_catalogo::st_eliminar_marca,
            commands::servicio_tecnico_catalogo::st_listar_modelos,
            commands::servicio_tecnico_catalogo::st_crear_modelo,
            commands::servicio_tecnico_catalogo::st_actualizar_modelo,
            commands::servicio_tecnico_catalogo::st_eliminar_modelo,
            commands::servicio_tecnico_catalogo::st_listar_arbol_completo,
            commands::servicio_tecnico_catalogo::st_historial_filtrable,
            // Servicio Tecnico - Abonos / Holding / Cancelacion (ST-5)
            commands::servicio_tecnico_abonos::st_listar_abonos,
            commands::servicio_tecnico_abonos::st_abonos_por_venta,
            commands::servicio_tecnico_abonos::st_recibir_abono,
            commands::servicio_tecnico_abonos::st_editar_abono,
            commands::servicio_tecnico_abonos::st_eliminar_abono,
            commands::servicio_tecnico_abonos::st_total_abonos_orden,
            commands::servicio_tecnico_abonos::st_cancelar_orden,
            commands::servicio_tecnico_abonos::st_listar_holdings_caja,
            // Servicio Tecnico - Items presupuestados (ST-5)
            commands::servicio_tecnico_items::st_listar_items_orden,
            commands::servicio_tecnico_items::st_agregar_item_orden,
            commands::servicio_tecnico_items::st_actualizar_item_orden,
            commands::servicio_tecnico_items::st_eliminar_item_orden,
            commands::servicio_tecnico_items::st_total_orden,
            // Servicio Tecnico - Reportes (v2.4.14)
            commands::servicio_tecnico_reportes::st_reporte_cancelaciones,
            commands::servicio_tecnico_reportes::st_reporte_garantias_activas,
            // Offline (cola y cache para modo cliente)
            offline::cache::encolar_operacion,
            offline::cache::listar_cola_offline,
            offline::cache::marcar_operacion_enviada,
            offline::cache::marcar_operacion_error,
            // v2.5.4: Retenciones SRI (cruce con factura para llegar a saldo cero)
            commands::retenciones::listar_retenciones_venta,
            commands::retenciones::total_retenciones_venta,
            commands::retenciones::registrar_retencion,
            commands::retenciones::eliminar_retencion,
            offline::cache::contar_cola_offline,
            offline::cache::sincronizar_cache_productos,
            offline::cache::buscar_productos_offline,
            offline::cache::guardar_secuenciales_reservados,
            offline::cache::obtener_secuencial_offline,
            // Backup Cloud
            backup::cloud::ejecutar_backup_cloud,
            backup::cloud::backup_cloud_premium,
            backup::cloud::backup_cloud_gdrive,
            backup::cloud::estado_backup_cloud,
            backup::cloud::guardar_gdrive_tokens,
            backup::cloud::desconectar_gdrive,
            backup::cloud::conectar_gdrive,
            // ─── Módulo Restaurante (solo build Clouget) ──────────────
            // Comandos siempre registrados — cada uno chequea licencia internamente
            // via `requiere_modulo_restaurante()`. Si BRAND es DigitalServer estos
            // comandos no se cargan porque el módulo no se compila.
            restaurante::commands::rest_listar_zonas,
            restaurante::commands::rest_crear_zona,
            restaurante::commands::rest_actualizar_zona,
            restaurante::commands::rest_eliminar_zona,
            restaurante::commands::rest_crear_mesa,
            restaurante::commands::rest_actualizar_mesa,
            restaurante::commands::rest_eliminar_mesa,
            restaurante::commands::rest_listar_mesas_con_estado,
            restaurante::commands::rest_abrir_pedido,
            restaurante::commands::rest_obtener_pedido,
            restaurante::commands::rest_obtener_pedido_mesa,
            restaurante::commands::rest_listar_pedidos_abiertos,
            restaurante::commands::rest_cancelar_pedido,
            restaurante::commands::rest_agregar_item,
            restaurante::commands::rest_actualizar_item_cantidad,
            restaurante::commands::rest_eliminar_item,
            restaurante::commands::rest_enviar_cocina,
            restaurante::commands::rest_listar_items_cocina_pendientes,
            restaurante::commands::rest_marcar_item_cocina,
            restaurante::commands::rest_pedir_cuenta,
            restaurante::commands::rest_cerrar_pedido,
            restaurante::commands::rest_imprimir_pre_cuenta,
            restaurante::commands::rest_imprimir_comanda_cocina,
            // v2.3.68 — Unir mesas
            restaurante::commands::rest_unir_mesas,
            restaurante::commands::rest_desunir_mesa,
            restaurante::commands::rest_listar_mesas_libres_para_unir,
            // v2.3.69 — Dividir cuenta (sub-cuentas)
            restaurante::commands::rest_dividir_cuenta,
            restaurante::commands::rest_listar_subcuentas,
            restaurante::commands::rest_cancelar_division,
            restaurante::commands::rest_marcar_subcuenta_cobrada,
            restaurante::commands::rest_producto_division_id,
            // v2.4.2 — App Móvil: admin de dispositivos emparejados
            app_movil::commands::app_listar_dispositivos,
            app_movil::commands::app_revocar_dispositivo,
            app_movil::commands::app_eliminar_dispositivo,
            // v2.4.4 — Sprint 3c: QR de emparejamiento
            app_movil::commands::app_generar_qr_emparejamiento,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
