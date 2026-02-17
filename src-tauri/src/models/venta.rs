use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Venta {
    pub id: Option<i64>,
    pub numero: String,
    pub cliente_id: Option<i64>,
    pub fecha: Option<String>,
    pub subtotal_sin_iva: f64,
    pub subtotal_con_iva: f64,
    pub descuento: f64,
    pub iva: f64,
    pub total: f64,
    pub forma_pago: String,
    pub monto_recibido: f64,
    pub cambio: f64,
    pub estado: String,
    pub tipo_documento: String,
    pub estado_sri: String,
    pub autorizacion_sri: Option<String>,
    pub clave_acceso: Option<String>,
    pub observacion: Option<String>,
    pub numero_factura: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct VentaDetalle {
    pub id: Option<i64>,
    pub venta_id: Option<i64>,
    pub producto_id: i64,
    pub nombre_producto: Option<String>,
    pub cantidad: f64,
    pub precio_unitario: f64,
    pub descuento: f64,
    pub iva_porcentaje: f64,
    pub subtotal: f64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct NuevaVenta {
    pub cliente_id: Option<i64>,
    pub items: Vec<VentaDetalle>,
    pub forma_pago: String,
    pub monto_recibido: f64,
    pub descuento: f64,
    pub tipo_documento: String,
    pub observacion: Option<String>,
    pub es_fiado: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct VentaCompleta {
    pub venta: Venta,
    pub detalles: Vec<VentaDetalle>,
    pub cliente_nombre: Option<String>,
}

// --- Notas de Crédito ---

#[derive(Debug, Serialize, Deserialize)]
pub struct NuevaNotaCredito {
    pub venta_id: i64,
    pub motivo: String,
    pub items: Vec<VentaDetalle>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct NotaCreditoInfo {
    pub id: i64,
    pub numero: String,
    pub venta_id: i64,
    pub factura_numero: String,
    pub motivo: String,
    pub total: f64,
    pub fecha: String,
    pub estado_sri: String,
    pub autorizacion_sri: Option<String>,
    pub clave_acceso: Option<String>,
    pub numero_factura_nc: Option<String>,
}
