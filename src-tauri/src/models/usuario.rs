use serde::{Deserialize, Serialize};

/// Categorías de permisos. Las categorías RESTAURANTE y APP_MOVIL solo se
/// muestran en la UI si la licencia tiene el módulo correspondiente.
///
/// - `CORE`: permisos siempre visibles (POS escritorio base)
/// - `RESTAURANTE`: visibles solo si `licencia.modulos` incluye `"restaurante"`
/// - `APP_MOVIL`: visibles solo si `licencia.modulos` incluye `"app_movil"`
pub const CAT_CORE: &str = "CORE";
pub const CAT_RESTAURANTE: &str = "RESTAURANTE";
pub const CAT_APP_MOVIL: &str = "APP_MOVIL";
pub const CAT_SERVICIO_TECNICO: &str = "SERVICIO_TECNICO";

/// Permisos disponibles en el sistema (clave, descripción, categoría).
///
/// La categoría determina si el permiso aparece en la UI según la licencia
/// activa. Los permisos RESTAURANTE/APP_MOVIL son consumidos por la app
/// móvil al hacer login PIN — el backend devuelve la lista completa en el
/// payload de sesión y la app decide qué pantallas renderizar.
pub const PERMISOS_DISPONIBLES: &[(&str, &str, &str)] = &[
    // ─── CORE (POS escritorio) ───────────────────────────────────────────
    ("editar_precio",               "Editar precio en punto de venta",                          CAT_CORE),
    ("editar_iva",                  "Editar IVA por item",                                      CAT_CORE),
    ("aplicar_descuentos",          "Aplicar descuentos",                                       CAT_CORE),
    ("anular_ventas",               "Anular ventas",                                            CAT_CORE),
    ("ver_reportes",                "Ver reportes",                                             CAT_CORE),
    ("ver_costos",                  "Ver precios de costo",                                     CAT_CORE),
    ("gestionar_clientes",          "Gestionar clientes",                                       CAT_CORE),
    ("gestionar_productos",         "Gestionar productos",                                      CAT_CORE),
    ("eliminar_productos",          "Eliminar productos y categorías (botón borrar)",           CAT_CORE),
    ("gestionar_inventario",        "Gestionar inventario",                                     CAT_CORE),
    ("ver_guias",                   "Ver guias de remision",                                    CAT_CORE),
    ("ver_movimientos_bancarios",   "Ver movimientos bancarios",                                CAT_CORE),
    ("confirmar_depositos",         "Confirmar depósitos en tránsito (retiros a banco)",        CAT_CORE),
    ("crear_nota_credito",          "Crear notas de crédito",                                   CAT_CORE),
    ("gestionar_compras",           "Gestionar compras y proveedores",                          CAT_CORE),
    // (gestionar_servicio_tecnico y ver_servicio_tecnico movidos a CAT_SERVICIO_TECNICO abajo, v2.4.8)
    ("cerrar_caja",                 "Cerrar caja (sin requerir supervisor)",                    CAT_CORE),
    ("aprobar_descuadre",           "Aprobar cierre con descuadre alto",                        CAT_CORE),
    ("cambiar_lista_precio",        "Cambiar la lista de precios en el POS",                    CAT_CORE),
    ("gestionar_gastos",            "Gestionar gastos",                                         CAT_CORE),
    ("ver_pagos_pendientes_admin",  "Confirmar/rechazar pagos a cuentas (transferencias)",      CAT_CORE),

    // ─── RESTAURANTE (módulo restaurante) ────────────────────────────────
    // Estos permisos también determinan qué ve el usuario en la app móvil
    // cuando se loguea con PIN si su rol es mesero/cocinero/etc.
    ("atiende_mesas",               "Atiende mesas (abre/edita pedidos)",                       CAT_RESTAURANTE),
    ("ve_cocina",                   "Ver pantalla de cocina y marcar items listos",             CAT_RESTAURANTE),
    ("imprime_comandas",            "Reimprimir comandas a cocina",                             CAT_RESTAURANTE),
    ("divide_cuenta",               "Dividir cuenta (sub-cuentas)",                             CAT_RESTAURANTE),
    ("une_mesas",                   "Unir mesas (grupos grandes)",                              CAT_RESTAURANTE),
    ("cancela_pedido",              "Cancelar pedido sin cobrar (libera mesa)",                 CAT_RESTAURANTE),
    ("config_mesas",                "Configurar zonas y mesas",                                 CAT_RESTAURANTE),

    // ─── APP_MOVIL (módulo app móvil para POS sin restaurante) ────────────
    // Permisos que SOLO tienen sentido en la app móvil — no afectan al POS
    // escritorio pero el admin los configura desde acá para cada usuario.
    ("vende_piso",                  "Vendedor de piso (toma pedidos en la app y envía a caja)", CAT_APP_MOVIL),
    ("inventaria",                  "Inventarista (conteo físico con la app)",                  CAT_APP_MOVIL),
    ("dueno_dashboard",             "Dueño/Admin (ve dashboard remoto en la app)",              CAT_APP_MOVIL),
    ("cobra_caja",                  "Puede cobrar en la app (vende piso → cobra él mismo)",     CAT_APP_MOVIL),

    // ─── SERVICIO_TECNICO (módulo servicio técnico — v2.4.8) ──────────────
    // Antes estos 2 permisos estaban en CAT_CORE. Movidos a su propia categoría
    // para que solo se muestren si la licencia tiene `servicio_tecnico` activo.
    ("gestionar_servicio_tecnico",  "Gestionar Servicio Técnico (todas las órdenes)",           CAT_SERVICIO_TECNICO),
    ("ver_servicio_tecnico",        "Ver Servicio Técnico (sólo órdenes asignadas)",            CAT_SERVICIO_TECNICO),
    ("config_servicio_tecnico",     "Configurar tipos de equipo, marcas y modelos",             CAT_SERVICIO_TECNICO),
    ("recibir_abonos_st",           "Recibir abonos en órdenes de servicio (holding en caja)",  CAT_SERVICIO_TECNICO),
    ("retirar_holdings_caja",       "Retirar dinero de holdings (anticipos no cobrados)",       CAT_SERVICIO_TECNICO),
    ("cancelar_orden_servicio",     "Cancelar órdenes de servicio (devuelve abonos)",           CAT_SERVICIO_TECNICO),
];

/// Info de usuario para enviar al frontend (sin hash/salt)
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct UsuarioInfo {
    pub id: i64,
    pub nombre: String,
    pub rol: String,
    pub activo: bool,
    pub permisos: String,
}

/// Sesión activa (almacenada en RAM)
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SesionActiva {
    pub usuario_id: i64,
    pub nombre: String,
    pub rol: String,
    pub permisos: String,
}

/// Datos para crear un nuevo usuario
#[derive(Debug, Serialize, Deserialize)]
pub struct NuevoUsuario {
    pub nombre: String,
    pub pin: String,
    pub rol: String,
    pub permisos: Option<String>,
}
