use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Compra {
    pub id: Option<i64>,
    pub numero: String,
    pub proveedor_id: i64,
    pub fecha: Option<String>,
    pub numero_factura: Option<String>,
    pub subtotal: f64,
    pub iva: f64,
    pub total: f64,
    pub estado: String,
    pub forma_pago: String,
    pub es_credito: bool,
    pub observacion: Option<String>,
    pub proveedor_nombre: Option<String>,
    #[serde(default)]
    pub banco_id: Option<i64>,
    #[serde(default)]
    pub referencia_pago: Option<String>,
    #[serde(default)]
    pub banco_nombre: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CompraDetalle {
    pub id: Option<i64>,
    pub compra_id: Option<i64>,
    pub producto_id: Option<i64>,
    pub descripcion: Option<String>,
    pub cantidad: f64,
    pub precio_unitario: f64,
    pub subtotal: f64,
    pub nombre_producto: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CompraCompleta {
    pub compra: Compra,
    pub detalles: Vec<CompraDetalle>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct NuevaCompra {
    pub proveedor_id: i64,
    pub numero_factura: Option<String>,
    pub items: Vec<ItemCompra>,
    pub forma_pago: String,
    pub es_credito: bool,
    pub observacion: Option<String>,
    pub dias_credito: Option<i64>,
    #[serde(default)]
    pub banco_id: Option<i64>,
    #[serde(default)]
    pub referencia_pago: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ItemCompra {
    pub producto_id: Option<i64>,
    pub descripcion: Option<String>,
    pub cantidad: f64,
    pub precio_unitario: f64,
    pub iva_porcentaje: f64,
    /// Info para crear lote automatico (si el producto requiere_caducidad) - v2.2.0
    #[serde(default)]
    pub lote_numero: Option<String>,
    #[serde(default)]
    pub lote_fecha_caducidad: Option<String>,
    #[serde(default)]
    pub lote_fecha_elaboracion: Option<String>,
}
