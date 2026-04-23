use serde::{Deserialize, Serialize};

/// Permisos disponibles en el sistema (clave, descripción)
pub const PERMISOS_DISPONIBLES: &[(&str, &str)] = &[
    ("editar_precio", "Editar precio en punto de venta"),
    ("editar_iva", "Editar IVA por item"),
    ("aplicar_descuentos", "Aplicar descuentos"),
    ("anular_ventas", "Anular ventas"),
    ("ver_reportes", "Ver reportes"),
    ("ver_costos", "Ver precios de costo"),
    ("gestionar_clientes", "Gestionar clientes"),
    ("gestionar_productos", "Gestionar productos"),
    ("gestionar_inventario", "Gestionar inventario"),
    ("ver_guias", "Ver guias de remision"),
    ("ver_movimientos_bancarios", "Ver movimientos bancarios"),
    ("crear_nota_credito", "Crear notas de crédito"),
    ("gestionar_compras", "Gestionar compras y proveedores"),
    ("gestionar_servicio_tecnico", "Gestionar Servicio Técnico"),
    ("ver_servicio_tecnico", "Ver Servicio Técnico (sólo asignadas)"),
    ("cerrar_caja", "Cerrar caja (sin requerir supervisor)"),
    ("aprobar_descuadre", "Aprobar cierre con descuadre alto"),
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
