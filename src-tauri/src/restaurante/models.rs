//! Structs serializables del módulo Restaurante.
//! Patrón: igual que `crate::models::*` — `Serialize + Deserialize + Clone`.

use serde::{Deserialize, Serialize};

// ─── Zona ────────────────────────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Zona {
    pub id: Option<i64>,
    pub nombre: String,
    #[serde(default = "default_color_zona")]
    pub color: String,
    #[serde(default)]
    pub orden: i32,
    #[serde(default = "default_true")]
    pub activa: bool,
}

// ─── Mesa ────────────────────────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Mesa {
    pub id: Option<i64>,
    pub zona_id: Option<i64>,
    pub nombre: String,
    #[serde(default = "default_capacidad")]
    pub capacidad: i32,
    #[serde(default)]
    pub orden: i32,
    #[serde(default = "default_true")]
    pub activa: bool,
}

/// Mesa enriquecida con su estado actual (libre/ocupada/cuenta pedida)
/// y el resumen del pedido si tiene uno abierto.
/// Es el shape que consume el grid de mesas en la UI.
#[derive(Debug, Serialize, Clone)]
pub struct MesaConEstado {
    pub id: i64,
    pub zona_id: Option<i64>,
    pub zona_nombre: Option<String>,
    pub zona_color: Option<String>,
    pub nombre: String,
    pub capacidad: i32,
    pub orden: i32,
    /// LIBRE | OCUPADA | CUENTA_PEDIDA
    pub estado: String,
    pub pedido_id: Option<i64>,
    pub mesero_nombre: Option<String>,
    pub comensales: Option<i32>,
    pub total_actual: f64,
    pub items_pendientes_cocina: i32,
    pub fecha_apertura: Option<String>,
    pub minutos_abierta: Option<i64>,
}

// ─── Pedido ──────────────────────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PedidoAbierto {
    pub id: Option<i64>,
    pub mesa_id: i64,
    pub mesero_id: Option<i64>,
    pub mesero_nombre: Option<String>,
    #[serde(default = "default_comensales")]
    pub comensales: i32,
    #[serde(default = "default_estado_pedido")]
    pub estado: String,
    pub observacion: Option<String>,
    pub fecha_apertura: Option<String>,
    pub fecha_cuenta: Option<String>,
    pub fecha_cierre: Option<String>,
    pub venta_id: Option<i64>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PedidoItem {
    pub id: Option<i64>,
    pub pedido_id: i64,
    pub producto_id: i64,
    /// JOIN con productos.nombre — solo lectura, ignorado en INSERT
    #[serde(default)]
    pub producto_nombre: Option<String>,
    pub cantidad: f64,
    pub precio_unit: f64,
    pub info_adicional: Option<String>,
    #[serde(default)]
    pub enviado_cocina: bool,
    #[serde(default = "default_estado_cocina")]
    pub estado_cocina: String,
    pub fecha_creacion: Option<String>,
    pub fecha_envio_cocina: Option<String>,
    /// JOIN con productos.destino_preparacion — solo lectura.
    /// 'COCINA' | 'BARRA' | 'DIRECTO'. Determina si el item va a /cocina
    /// o se despacha directo (no aparece en cocina).
    #[serde(default = "default_destino_preparacion")]
    pub destino_preparacion: String,
}

/// Pedido + items + datos de mesa + totales calculados.
/// Es el shape completo que consume la pantalla "Detalle de pedido".
#[derive(Debug, Serialize, Clone)]
pub struct PedidoDetalle {
    pub pedido: PedidoAbierto,
    pub items: Vec<PedidoItem>,
    pub mesa_nombre: String,
    pub zona_nombre: Option<String>,
    pub subtotal: f64,
    pub iva: f64,
    pub total: f64,
}

/// Item enriquecido para la vista de cocina (incluye mesa para context).
#[derive(Debug, Serialize, Clone)]
pub struct ItemCocina {
    pub id: i64,
    pub pedido_id: i64,
    pub mesa_nombre: String,
    pub zona_nombre: Option<String>,
    pub mesero_nombre: Option<String>,
    pub producto_nombre: String,
    pub cantidad: f64,
    pub info_adicional: Option<String>,
    pub estado_cocina: String,
    pub fecha_envio_cocina: Option<String>,
    pub minutos_en_cocina: Option<i64>,
}

// ─── Defaults ────────────────────────────────────────────────────────────

fn default_color_zona() -> String {
    "#3b82f6".to_string()
}
fn default_true() -> bool {
    true
}
fn default_capacidad() -> i32 {
    4
}
fn default_comensales() -> i32 {
    1
}
fn default_estado_pedido() -> String {
    "ABIERTO".to_string()
}
fn default_estado_cocina() -> String {
    "PENDIENTE".to_string()
}
fn default_destino_preparacion() -> String {
    "COCINA".to_string()
}
