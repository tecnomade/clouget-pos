use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CuentaPorPagar {
    pub id: Option<i64>,
    pub proveedor_id: i64,
    pub compra_id: Option<i64>,
    pub monto_total: f64,
    pub monto_pagado: f64,
    pub saldo: f64,
    pub estado: String,
    pub fecha_vencimiento: Option<String>,
    pub observacion: Option<String>,
    pub created_at: Option<String>,
    pub proveedor_nombre: Option<String>,
    pub compra_numero: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PagoProveedor {
    pub id: Option<i64>,
    pub cuenta_id: i64,
    pub monto: f64,
    pub fecha: Option<String>,
    pub forma_pago: String,
    pub numero_comprobante: Option<String>,
    pub observacion: Option<String>,
    pub banco_id: Option<i64>,
    pub banco_nombre: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ResumenAcreedor {
    pub proveedor_id: i64,
    pub proveedor_nombre: String,
    pub total_deuda: f64,
    pub num_cuentas: i64,
}
