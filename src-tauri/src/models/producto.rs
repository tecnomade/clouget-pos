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
    pub imagen: Option<String>,
    #[serde(default)]
    pub requiere_serie: bool,
    #[serde(default)]
    pub requiere_caducidad: bool,
    #[serde(default)]
    pub no_controla_stock: bool,
    /// 'SIMPLE' (default) | 'COMBO_FIJO' | 'COMBO_FLEXIBLE'
    #[serde(default = "default_tipo_producto")]
    pub tipo_producto: String,
}

fn default_tipo_producto() -> String { "SIMPLE".to_string() }

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ComboGrupo {
    pub id: Option<i64>,
    pub producto_padre_id: i64,
    pub nombre: String,
    pub minimo: i64,
    pub maximo: i64,
    #[serde(default)]
    pub orden: i64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ComboComponente {
    pub id: Option<i64>,
    pub producto_padre_id: i64,
    pub producto_hijo_id: i64,
    pub cantidad: f64,
    #[serde(default)]
    pub grupo_id: Option<i64>,
    #[serde(default)]
    pub orden: i64,
    // Campos enriquecidos en lecturas (no se serializan al insert):
    #[serde(default)]
    pub hijo_nombre: Option<String>,
    #[serde(default)]
    pub hijo_codigo: Option<String>,
    #[serde(default)]
    pub hijo_precio_venta: Option<f64>,
    #[serde(default)]
    pub hijo_precio_costo: Option<f64>,
    #[serde(default)]
    pub hijo_stock_actual: Option<f64>,
    #[serde(default)]
    pub hijo_unidad_medida: Option<String>,
    #[serde(default)]
    pub hijo_no_controla_stock: Option<bool>,
    #[serde(default)]
    pub hijo_es_servicio: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ProductoTactil {
    pub id: i64,
    pub nombre: String,
    pub precio_venta: f64,
    pub iva_porcentaje: f64,
    pub incluye_iva: bool,
    pub stock_actual: f64,
    pub categoria_id: Option<i64>,
    pub categoria_nombre: Option<String>,
    pub imagen: Option<String>,
    pub es_servicio: bool,
    pub no_controla_stock: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ProductoBusqueda {
    pub id: i64,
    pub codigo: Option<String>,
    pub codigo_barras: Option<String>,
    pub nombre: String,
    pub precio_venta: f64,
    #[serde(default)]
    pub precio_costo: f64,
    pub iva_porcentaje: f64,
    pub incluye_iva: bool,
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
