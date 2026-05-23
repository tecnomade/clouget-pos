use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Cliente {
    pub id: Option<i64>,
    pub tipo_identificacion: String,
    pub identificacion: Option<String>,
    pub nombre: String,
    pub direccion: Option<String>,
    pub telefono: Option<String>,
    pub email: Option<String>,
    pub activo: bool,
    pub lista_precio_id: Option<i64>,
    pub lista_precio_nombre: Option<String>,
    // v2.5.39: categoria + defaults heredados (overrideables por cliente individual)
    #[serde(default)]
    pub categoria_id: Option<i64>,
    #[serde(default)]
    pub permite_credito: Option<bool>,
    #[serde(default)]
    pub dias_credito: Option<i64>,
    #[serde(default)]
    pub limite_credito: Option<f64>,
    #[serde(default)]
    pub descuento_pct: Option<f64>,
}
