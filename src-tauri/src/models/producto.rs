use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Producto {
    pub id: Option<i64>,
    pub codigo: Option<String>,
    pub codigo_barras: Option<String>,
    pub nombre: String,
    pub descripcion: Option<String>,
    pub categoria_id: Option<i64>,
    pub precio_costo: f64,
    pub precio_venta: f64,
    pub iva_porcentaje: f64,
    pub incluye_iva: bool,
    pub stock_actual: f64,
    pub stock_minimo: f64,
    pub unidad_medida: String,
    pub es_servicio: bool,
    pub activo: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ProductoBusqueda {
    pub id: i64,
    pub codigo: Option<String>,
    pub nombre: String,
    pub precio_venta: f64,
    pub iva_porcentaje: f64,
    pub stock_actual: f64,
    pub stock_minimo: f64,
    pub categoria_nombre: Option<String>,
    pub precio_lista: Option<f64>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Categoria {
    pub id: Option<i64>,
    pub nombre: String,
    pub descripcion: Option<String>,
    pub activo: bool,
}
