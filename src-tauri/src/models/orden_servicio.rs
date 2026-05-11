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
    /// v2.4.25: intervalo recomendado entre mantenimientos (km).
    /// Cuando cambia kilometraje (entrada o salida), proximo = ref + intervalo.
    #[serde(default)]
    pub equipo_kilometraje_intervalo: Option<i64>,
    /// v2.4.25: km del vehiculo al ser entregado (post-trabajo).
    /// Si está presente, sustituye al kilometraje de entrada para el cálculo.
    #[serde(default)]
    pub equipo_kilometraje_salida: Option<i64>,
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
    // v2.4.10 — ST-2.5: FKs opcionales al catálogo jerárquico (st_tipos_equipo,
    // st_marcas, st_modelos). Si el user elige del catálogo, los IDs se llenan;
    // si escribe libre, quedan NULL pero los TEXT (equipo_marca, equipo_modelo,
    // tipo_equipo) se siguen guardando.
    #[serde(default)]
    pub tipo_equipo_id: Option<i64>,
    #[serde(default)]
    pub marca_id: Option<i64>,
    #[serde(default)]
    pub modelo_id: Option<i64>,
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
