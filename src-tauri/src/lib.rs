mod backup;
mod commands;
mod db;
mod models;
mod offline;
mod printing;
mod server;
mod sri;
pub mod utils;

use db::{Database, SesionState};
use std::sync::{Arc, Mutex};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let database = Database::new().expect("Error al inicializar la base de datos");
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

    // Iniciar servidor HTTP si está en modo servidor con token configurado
    if modo_red == "servidor" && !servidor_token.is_empty() {
        server::start_server(
            database.clone(),
            sesion_state.clone(),
            servidor_puerto,
            servidor_token,
        );
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
        .setup(|app| {
            #[cfg(desktop)]
            app.handle().plugin(tauri_plugin_updater::Builder::new().build())?;
            Ok(())
        })
        .manage(database)
        .manage(sesion_state)
        .manage(offline_db)
        .invoke_handler(tauri::generate_handler![
            // Productos
            commands::productos::crear_producto,
            commands::productos::actualizar_producto,
            commands::productos::buscar_productos,
            commands::productos::obtener_producto,
            commands::productos::listar_productos,
            commands::productos::productos_mas_vendidos,
            commands::productos::crear_categoria,
            commands::productos::listar_categorias,
            commands::productos::cargar_imagen_producto,
            commands::productos::eliminar_imagen_producto,
            commands::productos::listar_productos_tactil,
            // Clientes
            commands::clientes::crear_cliente,
            commands::clientes::actualizar_cliente,
            commands::clientes::buscar_clientes,
            commands::clientes::listar_clientes,
            commands::clientes::consultar_identificacion,
            // Ventas
            commands::ventas::registrar_venta,
            commands::ventas::listar_ventas_dia,
            commands::ventas::obtener_venta,
            commands::ventas::registrar_nota_credito,
            commands::ventas::listar_notas_credito_dia,
            commands::ventas::listar_ventas_sesion_caja,
            commands::ventas::resumen_sesion_caja,
            commands::ventas::listar_notas_credito_sesion_caja,
            // Caja
            commands::caja::abrir_caja,
            commands::caja::cerrar_caja,
            commands::caja::obtener_caja_abierta,
            // Configuración
            commands::config::obtener_config,
            commands::config::guardar_config,
            commands::config::cargar_logo_negocio,
            commands::config::eliminar_logo_negocio,
            commands::config::generar_token_servidor,
            commands::config::probar_conexion_servidor,
            // Impresión
            commands::impresion::imprimir_ticket,
            commands::impresion::imprimir_ticket_pdf,
            commands::impresion::imprimir_reporte_caja,
            commands::impresion::imprimir_reporte_caja_pdf,
            commands::impresion::listar_impresoras,
            commands::impresion::listar_impresoras_cached,
            commands::impresion::refrescar_impresoras,
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
            // Exportar CSV
            commands::exportar::exportar_ventas_csv,
            commands::exportar::exportar_gastos_csv,
            commands::exportar::exportar_inventario_csv,
            commands::exportar::guardar_archivo_texto,
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
            // Offline (cola y cache para modo cliente)
            offline::cache::encolar_operacion,
            offline::cache::listar_cola_offline,
            offline::cache::marcar_operacion_enviada,
            offline::cache::marcar_operacion_error,
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
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
