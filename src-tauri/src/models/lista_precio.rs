use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ListaPrecio {
    pub id: Option<i64>,
    pub nombre: String,
    pub descripcion: Option<String>,
    pub es_default: bool,
    pub activo: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PrecioProducto {
    pub lista_precio_id: i64,
    pub producto_id: i64,
    pub precio: f64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PrecioProductoDetalle {
    pub lista_precio_id: i64,
    pub lista_nombre: String,
    pub precio: f64,
}
