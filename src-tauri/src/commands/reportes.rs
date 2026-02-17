use crate::db::Database;
use serde::{Deserialize, Serialize};
use tauri::State;

#[derive(Debug, Serialize, Deserialize)]
pub struct ResumenDiario {
    pub total_ventas: f64,
    pub num_ventas: i64,
    pub total_efectivo: f64,
    pub total_transferencia: f64,
    pub total_fiado: f64,
    pub utilidad_bruta: f64,
    pub total_notas_credito: f64,
    pub num_notas_credito: i64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ProductoMasVendido {
    pub nombre: String,
    pub cantidad_total: f64,
    pub total_vendido: f64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AlertaStock {
    pub id: i64,
    pub codigo: Option<String>,
    pub nombre: String,
    pub stock_actual: f64,
    pub stock_minimo: f64,
}

#[tauri::command]
pub fn resumen_diario(db: State<Database>, fecha: String) -> Result<ResumenDiario, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;

    let total_ventas: f64 = conn
        .query_row(
            "SELECT COALESCE(SUM(total), 0) FROM ventas WHERE date(fecha) = date(?1) AND anulada = 0",
            rusqlite::params![fecha],
            |row| row.get(0),
        )
        .unwrap_or(0.0);

    let num_ventas: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM ventas WHERE date(fecha) = date(?1) AND anulada = 0",
            rusqlite::params![fecha],
            |row| row.get(0),
        )
        .unwrap_or(0);

    let total_efectivo: f64 = conn
        .query_row(
            "SELECT COALESCE(SUM(total), 0) FROM ventas WHERE date(fecha) = date(?1) AND forma_pago = 'EFECTIVO' AND anulada = 0",
            rusqlite::params![fecha],
            |row| row.get(0),
        )
        .unwrap_or(0.0);

    let total_transferencia: f64 = conn
        .query_row(
            "SELECT COALESCE(SUM(total), 0) FROM ventas WHERE date(fecha) = date(?1) AND forma_pago = 'TRANSFER' AND anulada = 0",
            rusqlite::params![fecha],
            |row| row.get(0),
        )
        .unwrap_or(0.0);

    let total_fiado: f64 = conn
        .query_row(
            "SELECT COALESCE(SUM(monto_total), 0) FROM cuentas_por_cobrar
             WHERE date(created_at) = date(?1)",
            rusqlite::params![fecha],
            |row| row.get(0),
        )
        .unwrap_or(0.0);

    // Utilidad bruta = sum(subtotal_venta) - sum(precio_costo * cantidad)
    let utilidad_bruta: f64 = conn
        .query_row(
            "SELECT COALESCE(SUM(vd.subtotal - (p.precio_costo * vd.cantidad)), 0)
             FROM venta_detalles vd
             JOIN ventas v ON vd.venta_id = v.id
             JOIN productos p ON vd.producto_id = p.id
             WHERE date(v.fecha) = date(?1) AND v.anulada = 0",
            rusqlite::params![fecha],
            |row| row.get(0),
        )
        .unwrap_or(0.0);

    let total_notas_credito: f64 = conn
        .query_row(
            "SELECT COALESCE(SUM(total), 0) FROM notas_credito WHERE date(fecha) = date(?1)",
            rusqlite::params![fecha],
            |row| row.get(0),
        )
        .unwrap_or(0.0);

    let num_notas_credito: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM notas_credito WHERE date(fecha) = date(?1)",
            rusqlite::params![fecha],
            |row| row.get(0),
        )
        .unwrap_or(0);

    Ok(ResumenDiario {
        total_ventas,
        num_ventas,
        total_efectivo,
        total_transferencia,
        total_fiado,
        utilidad_bruta,
        total_notas_credito,
        num_notas_credito,
    })
}

#[tauri::command]
pub fn productos_mas_vendidos_reporte(
    db: State<Database>,
    fecha_inicio: String,
    fecha_fin: String,
    limite: i64,
) -> Result<Vec<ProductoMasVendido>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;

    let mut stmt = conn
        .prepare(
            "SELECT p.nombre, SUM(vd.cantidad) as cant, SUM(vd.subtotal) as tot
             FROM venta_detalles vd
             JOIN ventas v ON vd.venta_id = v.id
             JOIN productos p ON vd.producto_id = p.id
             WHERE date(v.fecha) BETWEEN date(?1) AND date(?2) AND v.anulada = 0
             GROUP BY p.id
             ORDER BY cant DESC
             LIMIT ?3",
        )
        .map_err(|e| e.to_string())?;

    let productos = stmt
        .query_map(rusqlite::params![fecha_inicio, fecha_fin, limite], |row| {
            Ok(ProductoMasVendido {
                nombre: row.get(0)?,
                cantidad_total: row.get(1)?,
                total_vendido: row.get(2)?,
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    Ok(productos)
}

#[tauri::command]
pub fn alertas_stock_bajo(db: State<Database>) -> Result<Vec<AlertaStock>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;

    let mut stmt = conn
        .prepare(
            "SELECT id, codigo, nombre, stock_actual, stock_minimo
             FROM productos
             WHERE activo = 1 AND es_servicio = 0
             AND stock_actual <= stock_minimo
             ORDER BY (stock_actual - stock_minimo) ASC",
        )
        .map_err(|e| e.to_string())?;

    let alertas = stmt
        .query_map([], |row| {
            Ok(AlertaStock {
                id: row.get(0)?,
                codigo: row.get(1)?,
                nombre: row.get(2)?,
                stock_actual: row.get(3)?,
                stock_minimo: row.get(4)?,
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    Ok(alertas)
}

#[tauri::command]
pub fn resumen_fiados_pendientes(db: State<Database>) -> Result<f64, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let total: f64 = conn
        .query_row(
            "SELECT COALESCE(SUM(saldo), 0) FROM cuentas_por_cobrar WHERE estado = 'PENDIENTE'",
            [],
            |row| row.get(0),
        )
        .unwrap_or(0.0);
    Ok(total)
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ResumenPeriodo {
    pub total_ventas: f64,
    pub num_ventas: i64,
    pub total_efectivo: f64,
    pub total_transferencia: f64,
    pub total_fiado: f64,
    pub utilidad_bruta: f64,
    pub total_gastos: f64,
    pub promedio_por_venta: f64,
    pub total_notas_credito: f64,
    pub num_notas_credito: i64,
}

#[tauri::command]
pub fn resumen_periodo(
    db: State<Database>,
    fecha_inicio: String,
    fecha_fin: String,
) -> Result<ResumenPeriodo, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;

    let total_ventas: f64 = conn
        .query_row(
            "SELECT COALESCE(SUM(total), 0) FROM ventas
             WHERE date(fecha) BETWEEN date(?1) AND date(?2) AND anulada = 0",
            rusqlite::params![fecha_inicio, fecha_fin],
            |row| row.get(0),
        )
        .unwrap_or(0.0);

    let num_ventas: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM ventas
             WHERE date(fecha) BETWEEN date(?1) AND date(?2) AND anulada = 0",
            rusqlite::params![fecha_inicio, fecha_fin],
            |row| row.get(0),
        )
        .unwrap_or(0);

    let total_efectivo: f64 = conn
        .query_row(
            "SELECT COALESCE(SUM(total), 0) FROM ventas
             WHERE date(fecha) BETWEEN date(?1) AND date(?2) AND forma_pago = 'EFECTIVO' AND anulada = 0",
            rusqlite::params![fecha_inicio, fecha_fin],
            |row| row.get(0),
        )
        .unwrap_or(0.0);

    let total_transferencia: f64 = conn
        .query_row(
            "SELECT COALESCE(SUM(total), 0) FROM ventas
             WHERE date(fecha) BETWEEN date(?1) AND date(?2) AND forma_pago = 'TRANSFER' AND anulada = 0",
            rusqlite::params![fecha_inicio, fecha_fin],
            |row| row.get(0),
        )
        .unwrap_or(0.0);

    let total_fiado: f64 = conn
        .query_row(
            "SELECT COALESCE(SUM(monto_total), 0) FROM cuentas_por_cobrar
             WHERE date(created_at) BETWEEN date(?1) AND date(?2)",
            rusqlite::params![fecha_inicio, fecha_fin],
            |row| row.get(0),
        )
        .unwrap_or(0.0);

    let utilidad_bruta: f64 = conn
        .query_row(
            "SELECT COALESCE(SUM(vd.subtotal - (p.precio_costo * vd.cantidad)), 0)
             FROM venta_detalles vd
             JOIN ventas v ON vd.venta_id = v.id
             JOIN productos p ON vd.producto_id = p.id
             WHERE date(v.fecha) BETWEEN date(?1) AND date(?2) AND v.anulada = 0",
            rusqlite::params![fecha_inicio, fecha_fin],
            |row| row.get(0),
        )
        .unwrap_or(0.0);

    let total_gastos: f64 = conn
        .query_row(
            "SELECT COALESCE(SUM(monto), 0) FROM gastos
             WHERE date(fecha) BETWEEN date(?1) AND date(?2)",
            rusqlite::params![fecha_inicio, fecha_fin],
            |row| row.get(0),
        )
        .unwrap_or(0.0);

    let promedio_por_venta = if num_ventas > 0 {
        total_ventas / num_ventas as f64
    } else {
        0.0
    };

    let total_notas_credito: f64 = conn
        .query_row(
            "SELECT COALESCE(SUM(total), 0) FROM notas_credito
             WHERE date(fecha) BETWEEN date(?1) AND date(?2)",
            rusqlite::params![fecha_inicio, fecha_fin],
            |row| row.get(0),
        )
        .unwrap_or(0.0);

    let num_notas_credito: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM notas_credito
             WHERE date(fecha) BETWEEN date(?1) AND date(?2)",
            rusqlite::params![fecha_inicio, fecha_fin],
            |row| row.get(0),
        )
        .unwrap_or(0);

    Ok(ResumenPeriodo {
        total_ventas,
        num_ventas,
        total_efectivo,
        total_transferencia,
        total_fiado,
        utilidad_bruta,
        total_gastos,
        promedio_por_venta,
        total_notas_credito,
        num_notas_credito,
    })
}

#[tauri::command]
pub fn listar_ventas_periodo(
    db: State<Database>,
    fecha_inicio: String,
    fecha_fin: String,
) -> Result<Vec<crate::models::Venta>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;

    let mut stmt = conn
        .prepare(
            "SELECT id, numero, cliente_id, fecha, subtotal_sin_iva, subtotal_con_iva,
             descuento, iva, total, forma_pago, monto_recibido, cambio, estado,
             tipo_documento, estado_sri, autorizacion_sri, clave_acceso, observacion,
             numero_factura
             FROM ventas
             WHERE date(fecha) BETWEEN date(?1) AND date(?2) AND anulada = 0
             ORDER BY fecha DESC",
        )
        .map_err(|e| e.to_string())?;

    let ventas = stmt
        .query_map(rusqlite::params![fecha_inicio, fecha_fin], |row| {
            Ok(crate::models::Venta {
                id: Some(row.get(0)?),
                numero: row.get(1)?,
                cliente_id: row.get(2)?,
                fecha: row.get(3)?,
                subtotal_sin_iva: row.get(4)?,
                subtotal_con_iva: row.get(5)?,
                descuento: row.get(6)?,
                iva: row.get(7)?,
                total: row.get(8)?,
                forma_pago: row.get(9)?,
                monto_recibido: row.get(10)?,
                cambio: row.get(11)?,
                estado: row.get(12)?,
                tipo_documento: row.get(13)?,
                estado_sri: row.get::<_, String>(14).unwrap_or_else(|_| "NO_APLICA".to_string()),
                autorizacion_sri: row.get(15)?,
                clave_acceso: row.get(16)?,
                observacion: row.get(17)?,
                numero_factura: row.get(18)?,
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    Ok(ventas)
}
