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
    // Uses venta_detalles.precio_costo when available (> 0), falls back to productos.precio_costo for old records
    let utilidad_bruta: f64 = conn
        .query_row(
            "SELECT COALESCE(SUM(vd.subtotal - (CASE WHEN COALESCE(vd.precio_costo, 0) > 0 THEN vd.precio_costo ELSE p.precio_costo END * vd.cantidad)), 0)
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
            "SELECT COALESCE(SUM(vd.subtotal - (CASE WHEN COALESCE(vd.precio_costo, 0) > 0 THEN vd.precio_costo ELSE p.precio_costo END * vd.cantidad)), 0)
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

#[derive(Debug, Serialize, Deserialize)]
pub struct VentaDiaria {
    pub fecha: String,
    pub total: f64,
    pub num_ventas: i64,
}

#[tauri::command]
pub fn ventas_por_dia(
    db: State<Database>,
    fecha_inicio: String,
    fecha_fin: String,
) -> Result<Vec<VentaDiaria>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;

    let mut stmt = conn
        .prepare(
            "SELECT date(fecha) as dia, COALESCE(SUM(total), 0), COUNT(*)
             FROM ventas
             WHERE date(fecha) BETWEEN date(?1) AND date(?2) AND anulada = 0
             GROUP BY dia
             ORDER BY dia ASC",
        )
        .map_err(|e| e.to_string())?;

    let ventas = stmt
        .query_map(rusqlite::params![fecha_inicio, fecha_fin], |row| {
            Ok(VentaDiaria {
                fecha: row.get(0)?,
                total: row.get(1)?,
                num_ventas: row.get(2)?,
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    Ok(ventas)
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
            "SELECT v.id, v.numero, v.cliente_id, v.fecha, v.subtotal_sin_iva, v.subtotal_con_iva,
             v.descuento, v.iva, v.total, v.forma_pago, v.monto_recibido, v.cambio, v.estado,
             v.tipo_documento, v.estado_sri, v.autorizacion_sri, v.clave_acceso, v.observacion,
             v.numero_factura, v.establecimiento, v.punto_emision,
             v.banco_id, v.referencia_pago, cb.nombre as banco_nombre,
             COALESCE(v.tipo_estado, 'COMPLETADA') as tipo_estado
             FROM ventas v
             LEFT JOIN cuentas_banco cb ON v.banco_id = cb.id
             WHERE date(v.fecha) BETWEEN date(?1) AND date(?2) AND v.anulada = 0
             ORDER BY v.fecha DESC",
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
                establecimiento: row.get(19).ok(),
                punto_emision: row.get(20).ok(),
                banco_id: row.get(21).ok(),
                referencia_pago: row.get(22).ok(),
                banco_nombre: row.get(23).ok(),
                tipo_estado: row.get(24).ok(),
                guia_placa: None, guia_chofer: None, guia_direccion_destino: None,
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    Ok(ventas)
}

// --- Dashboard: comparativo vs ayer ---

#[tauri::command]
pub fn resumen_diario_ayer(db: State<Database>) -> Result<ResumenDiario, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let ayer = conn
        .query_row("SELECT date('now', '-1 day')", [], |row| row.get::<_, String>(0))
        .map_err(|e| e.to_string())?;
    drop(conn);
    resumen_diario(db, ayer)
}

// --- Dashboard: últimas ventas del día ---

#[derive(Debug, Serialize, Deserialize)]
pub struct UltimaVenta {
    pub id: i64,
    pub numero: String,
    pub hora: String,
    pub cliente_nombre: String,
    pub total: f64,
    pub forma_pago: String,
}

#[tauri::command]
pub fn ultimas_ventas_dia(db: State<Database>, limite: i64) -> Result<Vec<UltimaVenta>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;

    let mut stmt = conn
        .prepare(
            "SELECT v.id, v.numero,
                    COALESCE(strftime('%H:%M', v.fecha), '') as hora,
                    COALESCE(c.nombre, 'Consumidor Final') as cliente_nombre,
                    v.total, v.forma_pago
             FROM ventas v
             LEFT JOIN clientes c ON v.cliente_id = c.id
             WHERE date(v.fecha) = date('now') AND v.anulada = 0
             ORDER BY v.fecha DESC
             LIMIT ?1",
        )
        .map_err(|e| e.to_string())?;

    let ventas = stmt
        .query_map(rusqlite::params![limite], |row| {
            Ok(UltimaVenta {
                id: row.get(0)?,
                numero: row.get(1)?,
                hora: row.get(2)?,
                cliente_nombre: row.get(3)?,
                total: row.get(4)?,
                forma_pago: row.get(5)?,
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    Ok(ventas)
}

// --- Reportes avanzados ---

#[derive(Debug, Serialize, Deserialize)]
pub struct CategoriaUtilidad { pub categoria: String, pub ventas: f64, pub costo: f64, pub utilidad: f64 }
#[derive(Debug, Serialize, Deserialize)]
pub struct GastoCategoria { pub categoria: String, pub monto: f64 }

#[derive(Debug, Serialize, Deserialize)]
pub struct ReporteUtilidad {
    pub ventas_brutas: f64, pub costo_ventas: f64, pub utilidad_bruta: f64, pub margen_bruto: f64,
    pub total_gastos: f64, pub utilidad_neta: f64, pub margen_neto: f64, pub num_ventas: i64,
    pub promedio_por_venta: f64, pub total_devoluciones: f64,
    pub por_categoria: Vec<CategoriaUtilidad>, pub gastos_por_categoria: Vec<GastoCategoria>,
}

#[tauri::command]
pub fn reporte_utilidad(db: State<Database>, fecha_inicio: String, fecha_hasta: String) -> Result<ReporteUtilidad, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let vf = "date(fecha) BETWEEN date(?1) AND date(?2) AND anulada = 0 AND COALESCE(tipo_estado, 'COMPLETADA') IN ('COMPLETADA', 'CONVERTIDA')";

    let ventas_brutas: f64 = conn.query_row(&format!("SELECT COALESCE(SUM(total), 0) FROM ventas WHERE {}", vf), rusqlite::params![fecha_inicio, fecha_hasta], |r| r.get(0)).unwrap_or(0.0);
    let num_ventas: i64 = conn.query_row(&format!("SELECT COUNT(*) FROM ventas WHERE {}", vf), rusqlite::params![fecha_inicio, fecha_hasta], |r| r.get(0)).unwrap_or(0);
    let costo_ventas: f64 = conn.query_row(&format!("SELECT COALESCE(SUM(CASE WHEN COALESCE(vd.precio_costo, 0) > 0 THEN vd.precio_costo ELSE p.precio_costo END * vd.cantidad), 0) FROM venta_detalles vd JOIN ventas v ON vd.venta_id = v.id JOIN productos p ON vd.producto_id = p.id WHERE date(v.fecha) BETWEEN date(?1) AND date(?2) AND v.anulada = 0 AND COALESCE(v.tipo_estado, 'COMPLETADA') IN ('COMPLETADA', 'CONVERTIDA')"), rusqlite::params![fecha_inicio, fecha_hasta], |r| r.get(0)).unwrap_or(0.0);
    let total_gastos: f64 = conn.query_row("SELECT COALESCE(SUM(monto), 0) FROM gastos WHERE date(fecha) BETWEEN date(?1) AND date(?2)", rusqlite::params![fecha_inicio, fecha_hasta], |r| r.get(0)).unwrap_or(0.0);
    let total_devoluciones: f64 = conn.query_row("SELECT COALESCE(SUM(total), 0) FROM notas_credito WHERE date(fecha) BETWEEN date(?1) AND date(?2)", rusqlite::params![fecha_inicio, fecha_hasta], |r| r.get(0)).unwrap_or(0.0);

    let utilidad_bruta = ventas_brutas - costo_ventas;
    let utilidad_neta = utilidad_bruta - total_gastos - total_devoluciones;
    let margen_bruto = if ventas_brutas > 0.0 { (utilidad_bruta / ventas_brutas) * 100.0 } else { 0.0 };
    let margen_neto = if ventas_brutas > 0.0 { (utilidad_neta / ventas_brutas) * 100.0 } else { 0.0 };
    let promedio_por_venta = if num_ventas > 0 { ventas_brutas / num_ventas as f64 } else { 0.0 };

    let mut sc = conn.prepare("SELECT COALESCE(cat.nombre, 'Sin categoría'), COALESCE(SUM(vd.subtotal), 0), COALESCE(SUM(CASE WHEN COALESCE(vd.precio_costo, 0) > 0 THEN vd.precio_costo ELSE p.precio_costo END * vd.cantidad), 0) FROM venta_detalles vd JOIN ventas v ON vd.venta_id = v.id JOIN productos p ON vd.producto_id = p.id LEFT JOIN categorias cat ON p.categoria_id = cat.id WHERE date(v.fecha) BETWEEN date(?1) AND date(?2) AND v.anulada = 0 AND COALESCE(v.tipo_estado, 'COMPLETADA') IN ('COMPLETADA', 'CONVERTIDA') GROUP BY COALESCE(cat.nombre, 'Sin categoría') ORDER BY SUM(vd.subtotal) DESC LIMIT 10").map_err(|e| e.to_string())?;
    let por_categoria = sc.query_map(rusqlite::params![fecha_inicio, fecha_hasta], |r| { let v: f64 = r.get(1)?; let c: f64 = r.get(2)?; Ok(CategoriaUtilidad { categoria: r.get(0)?, ventas: v, costo: c, utilidad: v - c }) }).map_err(|e| e.to_string())?.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())?;

    let mut sg = conn.prepare("SELECT COALESCE(categoria, 'Sin categoría'), SUM(monto) FROM gastos WHERE date(fecha) BETWEEN date(?1) AND date(?2) GROUP BY COALESCE(categoria, 'Sin categoría') ORDER BY SUM(monto) DESC").map_err(|e| e.to_string())?;
    let gastos_por_categoria = sg.query_map(rusqlite::params![fecha_inicio, fecha_hasta], |r| Ok(GastoCategoria { categoria: r.get(0)?, monto: r.get(1)? })).map_err(|e| e.to_string())?.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())?;

    Ok(ReporteUtilidad { ventas_brutas, costo_ventas, utilidad_bruta, margen_bruto, total_gastos, utilidad_neta, margen_neto, num_ventas, promedio_por_venta, total_devoluciones, por_categoria, gastos_por_categoria })
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ReporteBalance {
    pub ingresos_efectivo: f64, pub ingresos_transferencia: f64, pub ingresos_credito_cobrado: f64,
    pub total_ingresos: f64, pub gastos_por_categoria: Vec<GastoCategoria>, pub total_gastos: f64,
    pub total_devoluciones: f64, pub total_egresos: f64, pub resultado: f64,
    pub cuentas_por_cobrar: f64, pub valor_inventario: f64,
}

#[tauri::command]
pub fn reporte_balance(db: State<Database>, fecha_inicio: String, fecha_hasta: String) -> Result<ReporteBalance, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let bf = "date(fecha) BETWEEN date(?1) AND date(?2) AND anulada = 0 AND COALESCE(tipo_estado, 'COMPLETADA') IN ('COMPLETADA', 'CONVERTIDA')";

    let ingresos_efectivo: f64 = conn.query_row(&format!("SELECT COALESCE(SUM(total), 0) FROM ventas WHERE {} AND forma_pago = 'EFECTIVO'", bf), rusqlite::params![fecha_inicio, fecha_hasta], |r| r.get(0)).unwrap_or(0.0);
    let ingresos_transferencia: f64 = conn.query_row(&format!("SELECT COALESCE(SUM(total), 0) FROM ventas WHERE {} AND forma_pago = 'TRANSFER'", bf), rusqlite::params![fecha_inicio, fecha_hasta], |r| r.get(0)).unwrap_or(0.0);
    let ingresos_credito_cobrado: f64 = conn.query_row("SELECT COALESCE(SUM(monto), 0) FROM pagos_cuenta WHERE date(fecha) BETWEEN date(?1) AND date(?2)", rusqlite::params![fecha_inicio, fecha_hasta], |r| r.get(0)).unwrap_or(0.0);
    let total_ingresos = ingresos_efectivo + ingresos_transferencia + ingresos_credito_cobrado;
    let total_gastos: f64 = conn.query_row("SELECT COALESCE(SUM(monto), 0) FROM gastos WHERE date(fecha) BETWEEN date(?1) AND date(?2)", rusqlite::params![fecha_inicio, fecha_hasta], |r| r.get(0)).unwrap_or(0.0);
    let total_devoluciones: f64 = conn.query_row("SELECT COALESCE(SUM(total), 0) FROM notas_credito WHERE date(fecha) BETWEEN date(?1) AND date(?2)", rusqlite::params![fecha_inicio, fecha_hasta], |r| r.get(0)).unwrap_or(0.0);

    let mut sg = conn.prepare("SELECT COALESCE(categoria, 'Sin categoría'), SUM(monto) FROM gastos WHERE date(fecha) BETWEEN date(?1) AND date(?2) GROUP BY COALESCE(categoria, 'Sin categoría') ORDER BY SUM(monto) DESC").map_err(|e| e.to_string())?;
    let gastos_por_categoria = sg.query_map(rusqlite::params![fecha_inicio, fecha_hasta], |r| Ok(GastoCategoria { categoria: r.get(0)?, monto: r.get(1)? })).map_err(|e| e.to_string())?.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())?;

    let total_egresos = total_gastos + total_devoluciones;
    let resultado = total_ingresos - total_egresos;
    let cuentas_por_cobrar: f64 = conn.query_row("SELECT COALESCE(SUM(saldo), 0) FROM cuentas_por_cobrar WHERE estado = 'PENDIENTE'", [], |r| r.get(0)).unwrap_or(0.0);
    let valor_inventario: f64 = conn.query_row("SELECT COALESCE(SUM(stock_actual * precio_costo), 0) FROM productos WHERE activo = 1 AND es_servicio = 0", [], |r| r.get(0)).unwrap_or(0.0);

    Ok(ReporteBalance { ingresos_efectivo, ingresos_transferencia, ingresos_credito_cobrado, total_ingresos, gastos_por_categoria, total_gastos, total_devoluciones, total_egresos, resultado, cuentas_por_cobrar, valor_inventario })
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ProductoRentabilidad { pub nombre: String, pub categoria: String, pub cantidad: f64, pub ingreso: f64, pub costo: f64, pub utilidad: f64, pub margen: f64 }

#[tauri::command]
pub fn reporte_productos_rentabilidad(db: State<Database>, fecha_inicio: String, fecha_hasta: String, limite: i64) -> Result<Vec<ProductoRentabilidad>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let mut stmt = conn.prepare("SELECT p.nombre, COALESCE(cat.nombre, 'Sin categoría'), SUM(vd.cantidad), SUM(vd.subtotal), SUM(CASE WHEN COALESCE(vd.precio_costo, 0) > 0 THEN vd.precio_costo ELSE p.precio_costo END * vd.cantidad) FROM venta_detalles vd JOIN ventas v ON vd.venta_id = v.id JOIN productos p ON vd.producto_id = p.id LEFT JOIN categorias cat ON p.categoria_id = cat.id WHERE date(v.fecha) BETWEEN date(?1) AND date(?2) AND v.anulada = 0 AND COALESCE(v.tipo_estado, 'COMPLETADA') IN ('COMPLETADA', 'CONVERTIDA') GROUP BY p.id ORDER BY SUM(vd.subtotal) - SUM(CASE WHEN COALESCE(vd.precio_costo, 0) > 0 THEN vd.precio_costo ELSE p.precio_costo END * vd.cantidad) DESC LIMIT ?3").map_err(|e| e.to_string())?;
    let productos = stmt.query_map(rusqlite::params![fecha_inicio, fecha_hasta, limite], |r| {
        let i: f64 = r.get(3)?; let c: f64 = r.get(4)?; let u = i - c;
        Ok(ProductoRentabilidad { nombre: r.get(0)?, categoria: r.get(1)?, cantidad: r.get(2)?, ingreso: i, costo: c, utilidad: u, margen: if i > 0.0 { (u / i) * 100.0 } else { 0.0 } })
    }).map_err(|e| e.to_string())?.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())?;
    Ok(productos)
}

#[tauri::command]
pub fn listar_libro_movimientos(
    db: State<Database>,
    fecha_desde: String,
    fecha_hasta: String,
) -> Result<Vec<serde_json::Value>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;

    let sql = "
        SELECT tipo, referencia, descripcion, ingreso, egreso, forma_pago, fecha FROM (
            -- Ventas completadas
            SELECT 'VENTA' as tipo, v.numero as referencia,
                   COALESCE(cl.nombre, 'Consumidor Final') as descripcion,
                   v.total as ingreso, 0.0 as egreso,
                   v.forma_pago, v.fecha
            FROM ventas v
            LEFT JOIN clientes cl ON v.cliente_id = cl.id
            WHERE v.anulada = 0 AND COALESCE(v.tipo_estado, 'COMPLETADA') IN ('COMPLETADA', 'CONVERTIDA')
              AND v.tipo_documento IN ('NOTA_VENTA', 'FACTURA')
              AND date(v.fecha) >= date(?1) AND date(v.fecha) <= date(?2)

            UNION ALL

            -- Cobros de credito
            SELECT 'COBRO_CREDITO' as tipo, COALESCE(pc.numero_comprobante, '') as referencia,
                   COALESCE(cl.nombre, '') as descripcion, pc.monto as ingreso, 0.0 as egreso,
                   pc.forma_pago, pc.fecha
            FROM pagos_cuenta pc
            LEFT JOIN cuentas_por_cobrar cc ON pc.cuenta_id = cc.id
            LEFT JOIN clientes cl ON cc.cliente_id = cl.id
            WHERE pc.estado = 'CONFIRMADO'
              AND date(pc.fecha) >= date(?1) AND date(pc.fecha) <= date(?2)

            UNION ALL

            -- Gastos
            SELECT 'GASTO' as tipo, '' as referencia,
                   g.descripcion, 0.0 as ingreso, g.monto as egreso,
                   'EFECTIVO' as forma_pago, g.fecha
            FROM gastos g
            WHERE date(g.fecha) >= date(?1) AND date(g.fecha) <= date(?2)

            UNION ALL

            -- Retiros de caja
            SELECT 'RETIRO' as tipo, COALESCE(r.referencia, '') as referencia,
                   r.motivo as descripcion, 0.0 as ingreso, r.monto as egreso,
                   CASE WHEN r.banco_id IS NOT NULL THEN 'TRANSFERENCIA' ELSE 'EFECTIVO' END as forma_pago,
                   r.fecha
            FROM retiros_caja r
            WHERE date(r.fecha) >= date(?1) AND date(r.fecha) <= date(?2)

            UNION ALL

            -- Compras
            SELECT 'COMPRA' as tipo, c.numero as referencia,
                   COALESCE(p.nombre, '') as descripcion, 0.0 as ingreso, c.total as egreso,
                   c.forma_pago, c.fecha
            FROM compras c
            LEFT JOIN proveedores p ON c.proveedor_id = p.id
            WHERE c.estado != 'ANULADA'
              AND date(c.fecha) >= date(?1) AND date(c.fecha) <= date(?2)

            UNION ALL

            -- Pagos a proveedores
            SELECT 'PAGO_PROVEEDOR' as tipo, COALESCE(pp.numero_comprobante, '') as referencia,
                   COALESCE(pr.nombre, '') as descripcion, 0.0 as ingreso, pp.monto as egreso,
                   pp.forma_pago, pp.fecha
            FROM pagos_proveedor pp
            LEFT JOIN cuentas_por_pagar cp ON pp.cuenta_id = cp.id
            LEFT JOIN proveedores pr ON cp.proveedor_id = pr.id
            WHERE date(pp.fecha) >= date(?1) AND date(pp.fecha) <= date(?2)

            UNION ALL

            -- Notas de credito (devolucion = egreso)
            SELECT 'NOTA_CREDITO' as tipo, nc.numero as referencia,
                   nc.motivo as descripcion, 0.0 as ingreso, nc.total as egreso,
                   'DEVOLUCION' as forma_pago, nc.fecha
            FROM notas_credito nc
            WHERE date(nc.fecha) >= date(?1) AND date(nc.fecha) <= date(?2)

        ) movimientos ORDER BY fecha DESC, tipo
    ";

    let mut stmt = conn.prepare(sql).map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map(rusqlite::params![fecha_desde, fecha_hasta], |row| {
            Ok(serde_json::json!({
                "tipo": row.get::<_, String>(0)?,
                "referencia": row.get::<_, String>(1)?,
                "descripcion": row.get::<_, String>(2)?,
                "ingreso": row.get::<_, f64>(3)?,
                "egreso": row.get::<_, f64>(4)?,
                "forma_pago": row.get::<_, String>(5)?,
                "fecha": row.get::<_, String>(6)?
            }))
        })
        .map_err(|e| e.to_string())?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())
}

/// Reporte IVA mensual (Ecuador SRI - Formulario 104)
/// Calcula ventas gravadas 0%, 15%, IVA cobrado, notas de crédito, y compras.
/// NOTA: La tabla `compra_detalles` no tiene `iva_porcentaje`, así que usamos los
/// campos agregados `compras.subtotal` y `compras.iva` para calcular las compras.
#[tauri::command]
pub fn reporte_iva_mensual(
    db: State<Database>,
    anio: i32,
    mes: u32,
) -> Result<serde_json::Value, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;

    let mes_str = format!("{:02}", mes);
    let fecha_desde = format!("{}-{}-01", anio, mes_str);
    let ultimo_dia = match mes {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 => {
            if (anio % 4 == 0 && anio % 100 != 0) || anio % 400 == 0 {
                29
            } else {
                28
            }
        }
        _ => 30,
    };
    let fecha_hasta = format!("{}-{}-{:02}", anio, mes_str, ultimo_dia);

    // Ventas gravadas 0% (sin IVA)
    let ventas_0: f64 = conn
        .query_row(
            "SELECT COALESCE(SUM(vd.subtotal), 0) FROM venta_detalles vd
             JOIN ventas v ON vd.venta_id = v.id
             WHERE vd.iva_porcentaje = 0 AND v.anulada = 0 AND v.tipo_estado = 'COMPLETADA'
               AND v.tipo_documento IN ('NOTA_VENTA', 'FACTURA')
               AND date(v.fecha) BETWEEN date(?1) AND date(?2)",
            rusqlite::params![fecha_desde, fecha_hasta],
            |r| r.get(0),
        )
        .unwrap_or(0.0);

    // Ventas gravadas 15% (base imponible)
    let ventas_15_base: f64 = conn
        .query_row(
            "SELECT COALESCE(SUM(vd.subtotal), 0) FROM venta_detalles vd
             JOIN ventas v ON vd.venta_id = v.id
             WHERE vd.iva_porcentaje > 0 AND v.anulada = 0 AND v.tipo_estado = 'COMPLETADA'
               AND v.tipo_documento IN ('NOTA_VENTA', 'FACTURA')
               AND date(v.fecha) BETWEEN date(?1) AND date(?2)",
            rusqlite::params![fecha_desde, fecha_hasta],
            |r| r.get(0),
        )
        .unwrap_or(0.0);

    // IVA cobrado en ventas (débito fiscal)
    let iva_ventas: f64 = conn
        .query_row(
            "SELECT COALESCE(SUM(vd.subtotal * vd.iva_porcentaje / 100), 0) FROM venta_detalles vd
             JOIN ventas v ON vd.venta_id = v.id
             WHERE vd.iva_porcentaje > 0 AND v.anulada = 0 AND v.tipo_estado = 'COMPLETADA'
               AND v.tipo_documento IN ('NOTA_VENTA', 'FACTURA')
               AND date(v.fecha) BETWEEN date(?1) AND date(?2)",
            rusqlite::params![fecha_desde, fecha_hasta],
            |r| r.get(0),
        )
        .unwrap_or(0.0);

    // Notas de crédito (reducen base e IVA de ventas)
    let nc_base: f64 = conn
        .query_row(
            "SELECT COALESCE(SUM(subtotal_con_iva), 0) FROM notas_credito
             WHERE date(fecha) BETWEEN date(?1) AND date(?2)",
            rusqlite::params![fecha_desde, fecha_hasta],
            |r| r.get(0),
        )
        .unwrap_or(0.0);
    let nc_iva: f64 = conn
        .query_row(
            "SELECT COALESCE(SUM(iva), 0) FROM notas_credito
             WHERE date(fecha) BETWEEN date(?1) AND date(?2)",
            rusqlite::params![fecha_desde, fecha_hasta],
            |r| r.get(0),
        )
        .unwrap_or(0.0);

    // Compras: `compra_detalles` no tiene iva_porcentaje, así que agregamos desde
    // la cabecera `compras`. Consideramos "tarifa 15%" las compras que cobraron IVA
    // (iva > 0) y "tarifa 0%" las que no cobraron IVA (iva = 0).
    let compras_0: f64 = conn
        .query_row(
            "SELECT COALESCE(SUM(subtotal), 0) FROM compras
             WHERE iva = 0 AND estado = 'REGISTRADA'
               AND date(fecha) BETWEEN date(?1) AND date(?2)",
            rusqlite::params![fecha_desde, fecha_hasta],
            |r| r.get(0),
        )
        .unwrap_or(0.0);

    let compras_15_base: f64 = conn
        .query_row(
            "SELECT COALESCE(SUM(subtotal), 0) FROM compras
             WHERE iva > 0 AND estado = 'REGISTRADA'
               AND date(fecha) BETWEEN date(?1) AND date(?2)",
            rusqlite::params![fecha_desde, fecha_hasta],
            |r| r.get(0),
        )
        .unwrap_or(0.0);

    // IVA pagado en compras (crédito tributario)
    let iva_compras: f64 = conn
        .query_row(
            "SELECT COALESCE(SUM(iva), 0) FROM compras
             WHERE estado = 'REGISTRADA'
               AND date(fecha) BETWEEN date(?1) AND date(?2)",
            rusqlite::params![fecha_desde, fecha_hasta],
            |r| r.get(0),
        )
        .unwrap_or(0.0);

    // Cálculos finales
    let iva_ventas_neto = iva_ventas - nc_iva;
    let iva_a_pagar = iva_ventas_neto - iva_compras;

    Ok(serde_json::json!({
        "anio": anio,
        "mes": mes,
        "fecha_desde": fecha_desde,
        "fecha_hasta": fecha_hasta,
        "ventas_0": ventas_0,
        "ventas_15_base": ventas_15_base,
        "iva_ventas": iva_ventas,
        "nc_base": nc_base,
        "nc_iva": nc_iva,
        "iva_ventas_neto": iva_ventas_neto,
        "compras_0": compras_0,
        "compras_15_base": compras_15_base,
        "iva_compras": iva_compras,
        "iva_a_pagar": iva_a_pagar,
        "total_ventas": ventas_0 + ventas_15_base,
        "total_compras": compras_0 + compras_15_base,
    }))
}
