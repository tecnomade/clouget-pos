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
    #[serde(default)]
    pub es_recurrente: bool,
    /// v2.3.47: usuario que registro el gasto (para auditoria)
    #[serde(default)]
    pub usuario_id: Option<i64>,
    #[serde(default)]
    pub usuario_nombre: Option<String>,
    /// Estado de la caja al consultar (ABIERTA / CERRADA). Solo lectura desde
    /// listar_gastos_dia, no se persiste — se calcula via JOIN.
    #[serde(default)]
    pub caja_estado: Option<String>,
}
