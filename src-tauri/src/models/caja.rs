use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Caja {
    pub id: Option<i64>,
    pub fecha_apertura: Option<String>,
    pub fecha_cierre: Option<String>,
    pub monto_inicial: f64,
    pub monto_ventas: f64,
    pub monto_esperado: f64,
    pub monto_real: Option<f64>,
    pub diferencia: Option<f64>,
    pub estado: String,
    pub usuario: Option<String>,
    pub usuario_id: Option<i64>,
    pub observacion: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ResumenCaja {
    pub caja: Caja,
    pub total_ventas: f64,
    pub num_ventas: i64,
    pub total_efectivo: f64,
    pub total_gastos: f64,
    pub total_cobros_efectivo: f64,
    pub total_cobros_banco: f64,
    pub total_retiros: f64,
}

/// Resumen extendido para reporte de cierre de caja
#[derive(Debug, Serialize, Deserialize)]
pub struct ResumenCajaReporte {
    pub caja: Caja,
    pub total_ventas: f64,
    pub num_ventas: i64,
    pub total_efectivo: f64,
    pub total_transferencia: f64,
    pub total_fiado: f64,
    pub total_gastos: f64,
    pub total_cobros_efectivo: f64,
    pub total_cobros_banco: f64,
    pub total_retiros: f64,
    pub total_notas_credito: f64,
    pub num_notas_credito: i64,
    pub nombre_negocio: String,
    pub ruc: String,
    pub direccion: String,
    pub ventas_por_categoria: Vec<(String, f64)>,
    // Anti-fraude (v2.3.x)
    #[serde(default)]
    pub motivo_diferencia_apertura: Option<String>,
    #[serde(default)]
    pub motivo_descuadre: Option<String>,
    #[serde(default)]
    pub usuario_cierre: Option<String>,
    #[serde(default)]
    pub caja_anterior_id: Option<i64>,
    #[serde(default)]
    pub monto_cierre_anterior: Option<f64>,
    #[serde(default)]
    pub eventos: Vec<EventoCajaReporte>,
    #[serde(default)]
    pub depositos: Vec<DepositoReporte>,
}

#[derive(Debug, Serialize, Deserialize, Default, Clone)]
pub struct EventoCajaReporte {
    pub timestamp: String,
    pub evento: String,
    pub usuario: Option<String>,
    pub motivo: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Default, Clone)]
pub struct DepositoReporte {
    pub monto: f64,
    pub banco_nombre: Option<String>,
    pub referencia: Option<String>,
    pub estado: String,
    pub fecha: String,
}
