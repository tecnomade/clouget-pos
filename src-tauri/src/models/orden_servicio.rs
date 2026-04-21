use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct OrdenServicio {
    pub id: Option<i64>,
    pub numero: Option<String>,
    pub cliente_id: Option<i64>,
    pub cliente_nombre: Option<String>,
    pub cliente_telefono: Option<String>,
    #[serde(default = "default_tipo_equipo")]
    pub tipo_equipo: String,
    pub equipo_descripcion: String,
    pub equipo_marca: Option<String>,
    pub equipo_modelo: Option<String>,
    pub equipo_serie: Option<String>,
    pub equipo_placa: Option<String>,
    pub equipo_kilometraje: Option<i64>,
    pub equipo_kilometraje_proximo: Option<i64>,
    pub accesorios: Option<String>,
    pub problema_reportado: String,
    pub diagnostico: Option<String>,
    pub trabajo_realizado: Option<String>,
    pub observaciones: Option<String>,
    pub tecnico_id: Option<i64>,
    pub tecnico_nombre: Option<String>,
    #[serde(default = "default_estado")]
    pub estado: String,
    pub fecha_ingreso: Option<String>,
    pub fecha_promesa: Option<String>,
    pub fecha_entrega: Option<String>,
    #[serde(default)]
    pub presupuesto: f64,
    #[serde(default)]
    pub monto_final: f64,
    #[serde(default)]
    pub garantia_dias: i64,
    pub venta_id: Option<i64>,
    pub usuario_creador: Option<String>,
}

fn default_tipo_equipo() -> String { "GENERAL".to_string() }
fn default_estado() -> String { "RECIBIDO".to_string() }

#[derive(Debug, Serialize)]
pub struct MovimientoOrden {
    pub id: i64,
    pub estado_anterior: Option<String>,
    pub estado_nuevo: String,
    pub observacion: Option<String>,
    pub usuario: Option<String>,
    pub fecha: String,
}
