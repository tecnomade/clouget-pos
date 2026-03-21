use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Establecimiento {
    pub id: Option<i64>,
    pub codigo: String,
    pub nombre: String,
    pub direccion: Option<String>,
    pub telefono: Option<String>,
    pub es_propio: bool,
    pub activo: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PuntoEmision {
    pub id: Option<i64>,
    pub establecimiento_id: i64,
    pub codigo: String,
    pub nombre: Option<String>,
    pub activo: bool,
}
