use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CuentaPorCobrar {
    pub id: Option<i64>,
    pub cliente_id: i64,
    pub venta_id: i64,
    pub monto_total: f64,
    pub monto_pagado: f64,
    pub saldo: f64,
    pub estado: String,
    pub fecha_vencimiento: Option<String>,
    pub created_at: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PagoCuenta {
    pub id: Option<i64>,
    pub cuenta_id: i64,
    pub monto: f64,
    pub fecha: Option<String>,
    pub observacion: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CuentaConCliente {
    pub cuenta: CuentaPorCobrar,
    pub cliente_nombre: String,
    pub venta_numero: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ResumenCliente {
    pub cliente_id: i64,
    pub cliente_nombre: String,
    pub total_deuda: f64,
    pub num_cuentas: i64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CuentaDetalle {
    pub cuenta: CuentaPorCobrar,
    pub cliente_nombre: String,
    pub venta_numero: String,
    pub pagos: Vec<PagoCuenta>,
}
