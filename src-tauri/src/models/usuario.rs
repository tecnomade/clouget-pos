use serde::{Deserialize, Serialize};

/// Info de usuario para enviar al frontend (sin hash/salt)
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct UsuarioInfo {
    pub id: i64,
    pub nombre: String,
    pub rol: String,
    pub activo: bool,
}

/// Sesión activa (almacenada en RAM)
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SesionActiva {
    pub usuario_id: i64,
    pub nombre: String,
    pub rol: String,
}

/// Datos para crear un nuevo usuario
#[derive(Debug, Serialize, Deserialize)]
pub struct NuevoUsuario {
    pub nombre: String,
    pub pin: String,
    pub rol: String,
}
