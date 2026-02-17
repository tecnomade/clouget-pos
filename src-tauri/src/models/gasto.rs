use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Gasto {
    pub id: Option<i64>,
    pub descripcion: String,
    pub monto: f64,
    pub categoria: Option<String>,
    pub fecha: Option<String>,
    pub caja_id: Option<i64>,
    pub observacion: Option<String>,
}
