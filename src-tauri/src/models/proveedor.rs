use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Proveedor {
    pub id: Option<i64>,
    pub ruc: Option<String>,
    pub nombre: String,
    pub direccion: Option<String>,
    pub telefono: Option<String>,
    pub email: Option<String>,
    pub contacto: Option<String>,
    pub dias_credito: i64,
    pub activo: bool,
}
