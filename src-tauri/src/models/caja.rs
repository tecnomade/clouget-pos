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
    pub total_notas_credito: f64,
    pub num_notas_credito: i64,
    pub nombre_negocio: String,
    pub ruc: String,
    pub direccion: String,
}
