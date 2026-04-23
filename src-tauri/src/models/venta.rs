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
    pub establecimiento: Option<String>,
    pub punto_emision: Option<String>,
    #[serde(default)]
    pub banco_id: Option<i64>,
    #[serde(default)]
    pub referencia_pago: Option<String>,
    #[serde(default)]
    pub banco_nombre: Option<String>,
    #[serde(default)]
    pub tipo_estado: Option<String>,
    #[serde(default)]
    pub guia_placa: Option<String>,
    #[serde(default)]
    pub guia_chofer: Option<String>,
    #[serde(default)]
    pub guia_direccion_destino: Option<String>,
    #[serde(default)]
    pub anulada: Option<i64>,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct VentaDetalle {
    #[serde(default)]
    pub id: Option<i64>,
    #[serde(default)]
    pub venta_id: Option<i64>,
    #[serde(default)]
    pub producto_id: i64,
    #[serde(default)]
    pub nombre_producto: Option<String>,
    #[serde(default)]
    pub cantidad: f64,
    #[serde(default)]
    pub precio_unitario: f64,
    #[serde(default)]
    pub descuento: f64,
    #[serde(default)]
    pub iva_porcentaje: f64,
    #[serde(default)]
    pub subtotal: f64,
    #[serde(default)]
    pub info_adicional: Option<String>,
    /// Unidad de medida usada (opcional). Si presente, factor_unidad indica cuantas
    /// unidades base equivale 1 unidad de venta (ej: SIXPACK = 6 unidades base)
    #[serde(default)]
    pub unidad_id: Option<i64>,
    #[serde(default)]
    pub unidad_nombre: Option<String>,
    #[serde(default)]
    pub factor_unidad: Option<f64>,
    /// Lote de caducidad del que se vendio este item (v2.2.0)
    #[serde(default)]
    pub lote_id: Option<i64>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PagoMixto {
    pub forma_pago: String,
    pub monto: f64,
    #[serde(default)]
    pub banco_id: Option<i64>,
    #[serde(default)]
    pub referencia: Option<String>,
    #[serde(default)]
    pub comprobante_imagen: Option<String>,
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
    #[serde(default)]
    pub banco_id: Option<i64>,
    #[serde(default)]
    pub referencia_pago: Option<String>,
    #[serde(default)]
    pub comprobante_imagen: Option<String>,
    #[serde(default)]
    pub tipo_estado: Option<String>,
    #[serde(default)]
    pub guia_placa: Option<String>,
    #[serde(default)]
    pub guia_chofer: Option<String>,
    #[serde(default)]
    pub guia_direccion_destino: Option<String>,
    /// Pagos multiples (opcional). Si presente y no vacio, se usa en lugar de forma_pago/banco_id.
    /// La suma de pagos debe igualar el total de la venta.
    /// Si hay un pago tipo CREDITO, ese monto crea cuenta_por_cobrar.
    #[serde(default)]
    pub pagos: Option<Vec<PagoMixto>>,
}

// --- Documentos Recientes ---

#[derive(Debug, Serialize, Deserialize)]
pub struct DocumentoReciente {
    pub id: i64,
    pub numero: String,
    pub tipo_estado: String,
    pub tipo_documento: String,
    pub cliente_nombre: Option<String>,
    pub total: f64,
    pub fecha: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct VentaCompleta {
    pub venta: Venta,
    pub detalles: Vec<VentaDetalle>,
    pub cliente_nombre: Option<String>,
}

// --- Guías de Remisión ---

#[derive(Debug, Serialize, Deserialize)]
pub struct ResumenGuias {
    pub abiertas: i64,
    pub cerradas: i64,
    pub total_pendiente: f64,
    pub total_cerrado: f64,
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
