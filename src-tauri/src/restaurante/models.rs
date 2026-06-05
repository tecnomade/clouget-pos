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
    /// v2.5.92 — abonos (pagos parciales) en HOLDING sobre la mesa. >0 = "Cuenta parcial".
    #[serde(default)]
    pub total_abonado: f64,
    pub items_pendientes_cocina: i32,
    pub fecha_apertura: Option<String>,
    pub minutos_abierta: Option<i64>,
    // ─── v2.3.68 — Unir mesas ──────────────────────────────────────
    /// Si esta mesa está unida como EXTRA a un pedido cuyo principal es otra mesa.
    /// Ejemplo: Mesa 5 unida a Mesa 2 → en Mesa 5 estos campos apuntan a Mesa 2.
    /// Si NULL, esta mesa es la principal (o está libre).
    pub mesa_principal_id: Option<i64>,
    pub mesa_principal_nombre: Option<String>,
    /// Cantidad de mesas EXTRA unidas a esta (cuando es la principal de un pedido).
    /// 0 si no tiene mesas unidas o si esta mesa es extra.
    #[serde(default)]
    pub mesas_unidas_count: i32,
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

/// Mesa "ligera" (id + nombre + capacidad) — usado en listados embebidos
/// (mesas extra de un pedido, mesas libres para unir).
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MesaResumen {
    pub id: i64,
    pub nombre: String,
    pub capacidad: i32,
    pub zona_nombre: Option<String>,
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
    /// v2.3.68 — Mesas EXTRA unidas a este pedido (NO incluye la principal).
    /// Si vacío, el pedido ocupa solo la mesa principal.
    #[serde(default)]
    pub mesas_extra: Vec<MesaResumen>,
    /// v2.3.68 — Capacidad efectiva del grupo: mesa principal + extras.
    #[serde(default)]
    pub capacidad_total: i32,
    /// v2.5.91 — Pagos parciales (abonos) ya recibidos sobre esta mesa.
    #[serde(default)]
    pub total_abonado: f64,
    /// v2.5.91 — Saldo pendiente = total − total_abonado (mín. 0).
    #[serde(default)]
    pub saldo: f64,
    /// v2.5.91 — Lista de abonos registrados (historial).
    #[serde(default)]
    pub abonos: Vec<AbonoPedido>,
}

/// v2.5.91 — Un abono (pago parcial) sobre un pedido de mesa.
#[derive(Debug, Serialize, Clone)]
pub struct AbonoPedido {
    pub id: i64,
    pub monto: f64,
    pub forma_pago: String,
    pub banco_id: Option<i64>,
    pub banco_nombre: Option<String>,
    pub referencia_pago: Option<String>,
    pub estado: String,
    pub fecha: String,
    pub usuario_nombre: Option<String>,
}

// ─── v2.3.69 — Sub-cuentas (división de cuenta) ──────────────────────────

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Subcuenta {
    pub id: i64,
    pub pedido_id: i64,
    pub numero: i32,
    pub total: f64,
    /// PENDIENTE | COBRADA
    pub estado: String,
    pub forma_pago: Option<String>,
    pub banco_id: Option<i64>,
    pub banco_nombre: Option<String>,
    pub referencia_pago: Option<String>,
    pub venta_id: Option<i64>,
    /// numero asignado a la venta cuando ya se cobró (ej. NV-001-001-000000042)
    pub venta_numero: Option<String>,
    pub fecha_cobro: Option<String>,
}

/// Resultado de marcar una sub-cuenta como cobrada — el frontend usa
/// `todas_cobradas` para saber si debe mostrar el toast "Mesa liberada".
#[derive(Debug, Serialize, Clone)]
pub struct ResultadoCobroSubcuenta {
    pub todas_cobradas: bool,
    /// Cantidad de sub-cuentas pendientes restantes
    pub pendientes: i32,
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
