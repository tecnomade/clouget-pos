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
    /// v2.5.30: FACTURA (autorizada o no) / NOTA_VENTA / INFORMAL
    #[serde(default)]
    pub tipo_documento: Option<String>,
    /// AUTORIZADA (factura validada en SRI) / NULL (no autorizada o no aplica)
    #[serde(default)]
    pub estado_sri: Option<String>,
    /// Clave de acceso de 49 dígitos del SRI (solo facturas autorizadas)
    #[serde(default)]
    pub clave_acceso: Option<String>,
    /// Fecha que aparece en el documento del proveedor (puede diferir de created_at)
    #[serde(default)]
    pub fecha_emision: Option<String>,
    /// Total devuelto al proveedor (suma de notas de débito/devoluciones)
    #[serde(default)]
    pub total_devuelto: f64,
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
    /// v2.5.30: cantidad ya devuelta al proveedor en notas de débito acumuladas
    #[serde(default)]
    pub cantidad_devuelta: f64,
    /// v2.6.25/26: snapshot de la presentación con la que se cargó el item
    /// (ej: "Jaba x12" con factor=12 y cantidad_presentacion=2 → 24 unidades).
    #[serde(default)]
    pub presentacion_id: Option<i64>,
    #[serde(default)]
    pub presentacion_nombre: Option<String>,
    #[serde(default)]
    pub presentacion_factor: Option<f64>,
    #[serde(default)]
    pub cantidad_presentacion: Option<f64>,
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
    /// v2.5.30: FACTURA | NOTA_VENTA | INFORMAL. Default INFORMAL si no se especifica.
    #[serde(default)]
    pub tipo_documento: Option<String>,
    /// Fecha de emisión del documento (si difiere de la fecha de registro)
    #[serde(default)]
    pub fecha_emision: Option<String>,
    /// Clave de acceso (49 dig) si proviene de XML SRI autorizado
    #[serde(default)]
    pub clave_acceso: Option<String>,
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
    /// v2.6.25: Presentacion con la que se carga (jaba, six-pack). Si viene, el
    /// backend calcula `cantidad` real = cantidad_presentacion * factor y persiste
    /// snapshot (nombre + factor) en compra_detalles para auditoria historica.
    /// Si NO viene, se ignora y se carga en unidad base como siempre.
    #[serde(default)]
    pub presentacion_id: Option<i64>,
    #[serde(default)]
    pub cantidad_presentacion: Option<f64>,
}
