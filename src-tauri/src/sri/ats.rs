//! v2.5.48 — Generador del Anexo Transaccional Simplificado (ATS) del SRI Ecuador.
//!
//! El ATS es un XML mensual que el agente sube al portal del SRI (DIMM Anexos)
//! con todas las compras, ventas, retenciones emitidas y comprobantes anulados
//! del mes reportado.
//!
//! Schema oficial (vigente): `AnexoTransaccionalSimplificado.xsd`
//! Root tag: `<iva>` con los siguientes bloques:
//!   - Cabecera: TipoIDInformante, IdInformante, razonSocial, Anio, Mes,
//!     numEstabRuc, totalVentas, codigoOperativo, tipoIVA
//!   - <compras>  → 0..* <detalleCompras>
//!   - <ventas>   → 0..* <detalleVentas>     (agrupados por cliente)
//!   - <ventasEstablecimiento> → 1..* <ventaEst>
//!   - <anulados> → 0..* <detalleAnulados>
//!
//! Esta implementación cubre el caso típico de un agente de retención:
//! compras locales con sustento + retenciones emitidas + ventas a clientes
//! (consumidor final agrupado, identificados detallados) + anulados.
//! Sin exportaciones, sin reembolsos, sin operaciones internacionales.

use serde::{Deserialize, Serialize};

// ─── Estructuras ─────────────────────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DatosAts {
    // Cabecera
    pub razon_social: String,
    pub ruc: String,                    // 13 dígitos
    pub anio: String,                   // "2026"
    pub mes: String,                    // "05" (2 dígitos)
    pub num_estab_ruc: String,          // total estab activos (ej "001" = 1)
    pub total_ventas: f64,
    pub codigo_operativo: String,       // "IVA" por defecto

    pub compras: Vec<DetalleCompra>,
    pub ventas: Vec<DetalleVenta>,
    pub ventas_establecimiento: Vec<VentaEstablecimiento>,
    pub anulados: Vec<DetalleAnulado>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DetalleCompra {
    pub cod_sustento: String,         // Tabla 6 SRI (ej "01"=Crédito Tributario IVA)
    pub tp_id_prov: String,           // "01"=RUC, "02"=cédula, "03"=pasaporte
    pub id_prov: String,
    pub tipo_comprobante: String,     // "01"=factura, "04"=NC, "11"=pasajes, "12"=NV
    pub parte_rel: String,            // "SI" o "NO"
    pub fecha_registro: String,       // dd/mm/yyyy
    pub establecimiento: String,
    pub punto_emision: String,
    pub secuencial: String,
    pub fecha_emision: String,        // dd/mm/yyyy
    pub autorizacion: Option<String>, // 10/37/49 dígitos
    pub base_no_gra_iva: f64,
    pub base_imponible: f64,          // tarifa 0%
    pub base_imp_grav: f64,           // tarifa > 0% (12/15%)
    pub base_imp_exe: f64,
    pub monto_ice: f64,
    pub monto_iva: f64,
    pub val_ret_bien_10: f64,
    pub val_ret_serv_20: f64,
    pub valor_ret_bienes: f64,        // retención IVA 30% bienes
    pub val_ret_serv_50: f64,
    pub valor_ret_servicios: f64,     // retención IVA 70% servicios
    pub val_ret_serv_100: f64,        // retención IVA 100%
    pub totbases_imp_reemb: f64,
    pub pago_loc_ext: String,         // "01"=Local, "02"=Exterior
    pub forma_pago: String,           // "01"/"15"/"16"/"17"/"18"/"19"/"20"/"21"
    pub air: Vec<DetalleAir>,         // retenciones renta emitidas en esta compra
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DetalleAir {
    pub cod_ret_air: String,          // código retención renta (Tabla 304)
    pub base_imp_air: f64,
    pub porcentaje_air: f64,
    pub val_ret_air: f64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DetalleVenta {
    pub tp_id_cliente: String,        // "04"=RUC, "05"=cédula, "06"=pasaporte, "07"=CF
    pub id_cliente: String,
    pub parte_rel_vtas: String,
    pub tipo_cliente: String,         // "01"=PN, "02"=Sociedad — opcional
    pub deno_cli: Option<String>,     // razón social del cliente (si idCliente != 9999...)
    pub tipo_comprobante: String,
    pub tipo_emision: String,         // "F"=Física, "E"=Electrónica
    pub numero_comprobantes: i64,
    pub base_no_gra_iva: f64,
    pub base_imponible: f64,
    pub base_imp_grav: f64,
    pub monto_iva: f64,
    pub monto_ice: f64,
    pub valor_ret_iva: f64,
    pub valor_ret_renta: f64,
    pub forma_pago: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct VentaEstablecimiento {
    pub cod_estab: String,
    pub ventas_estab: f64,
    pub iva_comp: f64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DetalleAnulado {
    pub tipo_comprobante: String,
    pub establecimiento: String,
    pub punto_emision: String,
    pub secuencial_inicio: String,
    pub secuencial_fin: String,
    pub autorizacion: Option<String>,
}

// ─── Helpers ─────────────────────────────────────────────────────────────────

fn tag(xml: &mut String, indent: usize, name: &str, value: &str) {
    let spaces = " ".repeat(indent);
    xml.push_str(&format!("{}<{}>{}</{}>\n", spaces, name, value, name));
}

fn tag_money(xml: &mut String, indent: usize, name: &str, value: f64) {
    tag(xml, indent, name, &format!("{:.2}", value));
}

fn escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

// ─── Generador XML ───────────────────────────────────────────────────────────

/// Genera el XML completo del ATS conforme schema SRI Ecuador.
/// El XML resultante se puede subir directamente al DIMM Anexos del SRI.
pub fn generar_xml_ats(datos: &DatosAts) -> String {
    let mut xml = String::with_capacity(8192);
    xml.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
    xml.push_str("<iva>\n");

    // Cabecera
    tag(&mut xml, 2, "TipoIDInformante", "R");
    tag(&mut xml, 2, "IdInformante", &datos.ruc);
    tag(&mut xml, 2, "razonSocial", &escape(&datos.razon_social));
    tag(&mut xml, 2, "Anio", &datos.anio);
    tag(&mut xml, 2, "Mes", &datos.mes);
    tag(&mut xml, 2, "numEstabRuc", &datos.num_estab_ruc);
    tag_money(&mut xml, 2, "totalVentas", datos.total_ventas);
    tag(&mut xml, 2, "codigoOperativo", &datos.codigo_operativo);

    // Compras
    if !datos.compras.is_empty() {
        xml.push_str("  <compras>\n");
        for c in &datos.compras {
            xml.push_str("    <detalleCompras>\n");
            tag(&mut xml, 6, "codSustento", &c.cod_sustento);
            tag(&mut xml, 6, "tpIdProv", &c.tp_id_prov);
            tag(&mut xml, 6, "idProv", &c.id_prov);
            tag(&mut xml, 6, "tipoComprobante", &c.tipo_comprobante);
            tag(&mut xml, 6, "parteRel", &c.parte_rel);
            tag(&mut xml, 6, "fechaRegistro", &c.fecha_registro);
            tag(&mut xml, 6, "establecimiento", &c.establecimiento);
            tag(&mut xml, 6, "puntoEmision", &c.punto_emision);
            tag(&mut xml, 6, "secuencial", &c.secuencial);
            tag(&mut xml, 6, "fechaEmision", &c.fecha_emision);
            if let Some(ref a) = c.autorizacion {
                if !a.is_empty() {
                    tag(&mut xml, 6, "autorizacion", a);
                }
            }
            tag_money(&mut xml, 6, "baseNoGraIva", c.base_no_gra_iva);
            tag_money(&mut xml, 6, "baseImponible", c.base_imponible);
            tag_money(&mut xml, 6, "baseImpGrav", c.base_imp_grav);
            tag_money(&mut xml, 6, "baseImpExe", c.base_imp_exe);
            tag_money(&mut xml, 6, "montoIce", c.monto_ice);
            tag_money(&mut xml, 6, "montoIva", c.monto_iva);
            tag_money(&mut xml, 6, "valRetBien10", c.val_ret_bien_10);
            tag_money(&mut xml, 6, "valRetServ20", c.val_ret_serv_20);
            tag_money(&mut xml, 6, "valorRetBienes", c.valor_ret_bienes);
            tag_money(&mut xml, 6, "valRetServ50", c.val_ret_serv_50);
            tag_money(&mut xml, 6, "valorRetServicios", c.valor_ret_servicios);
            tag_money(&mut xml, 6, "valRetServ100", c.val_ret_serv_100);
            tag_money(&mut xml, 6, "totbasesImpReemb", c.totbases_imp_reemb);
            tag(&mut xml, 6, "pagoLocExt", &c.pago_loc_ext);
            xml.push_str("      <formasDePago>\n");
            tag(&mut xml, 8, "formaPago", &c.forma_pago);
            xml.push_str("      </formasDePago>\n");
            if !c.air.is_empty() {
                xml.push_str("      <air>\n");
                for a in &c.air {
                    xml.push_str("        <detalleAir>\n");
                    tag(&mut xml, 10, "codRetAir", &a.cod_ret_air);
                    tag_money(&mut xml, 10, "baseImpAir", a.base_imp_air);
                    tag_money(&mut xml, 10, "porcentajeAir", a.porcentaje_air);
                    tag_money(&mut xml, 10, "valRetAir", a.val_ret_air);
                    xml.push_str("        </detalleAir>\n");
                }
                xml.push_str("      </air>\n");
            }
            xml.push_str("    </detalleCompras>\n");
        }
        xml.push_str("  </compras>\n");
    }

    // Ventas
    if !datos.ventas.is_empty() {
        xml.push_str("  <ventas>\n");
        for v in &datos.ventas {
            xml.push_str("    <detalleVentas>\n");
            tag(&mut xml, 6, "tpIdCliente", &v.tp_id_cliente);
            tag(&mut xml, 6, "idCliente", &v.id_cliente);
            tag(&mut xml, 6, "parteRelVtas", &v.parte_rel_vtas);
            tag(&mut xml, 6, "tipoCliente", &v.tipo_cliente);
            if let Some(ref deno) = v.deno_cli {
                if !deno.is_empty() {
                    tag(&mut xml, 6, "denoCli", &escape(deno));
                }
            }
            tag(&mut xml, 6, "tipoComprobante", &v.tipo_comprobante);
            tag(&mut xml, 6, "tipoEmision", &v.tipo_emision);
            tag(&mut xml, 6, "numeroComprobantes", &v.numero_comprobantes.to_string());
            tag_money(&mut xml, 6, "baseNoGraIva", v.base_no_gra_iva);
            tag_money(&mut xml, 6, "baseImponible", v.base_imponible);
            tag_money(&mut xml, 6, "baseImpGrav", v.base_imp_grav);
            tag_money(&mut xml, 6, "montoIva", v.monto_iva);
            tag_money(&mut xml, 6, "montoIce", v.monto_ice);
            tag_money(&mut xml, 6, "valorRetIva", v.valor_ret_iva);
            tag_money(&mut xml, 6, "valorRetRenta", v.valor_ret_renta);
            xml.push_str("      <formasDePago>\n");
            tag(&mut xml, 8, "formaPago", &v.forma_pago);
            xml.push_str("      </formasDePago>\n");
            xml.push_str("    </detalleVentas>\n");
        }
        xml.push_str("  </ventas>\n");
    }

    // Ventas por establecimiento (siempre obligatorio)
    if !datos.ventas_establecimiento.is_empty() {
        xml.push_str("  <ventasEstablecimiento>\n");
        for ve in &datos.ventas_establecimiento {
            xml.push_str("    <ventaEst>\n");
            tag(&mut xml, 6, "codEstab", &ve.cod_estab);
            tag_money(&mut xml, 6, "ventasEstab", ve.ventas_estab);
            tag_money(&mut xml, 6, "ivaComp", ve.iva_comp);
            xml.push_str("    </ventaEst>\n");
        }
        xml.push_str("  </ventasEstablecimiento>\n");
    }

    // Anulados
    if !datos.anulados.is_empty() {
        xml.push_str("  <anulados>\n");
        for a in &datos.anulados {
            xml.push_str("    <detalleAnulados>\n");
            tag(&mut xml, 6, "tipoComprobante", &a.tipo_comprobante);
            tag(&mut xml, 6, "establecimiento", &a.establecimiento);
            tag(&mut xml, 6, "puntoEmision", &a.punto_emision);
            tag(&mut xml, 6, "secuencialInicio", &a.secuencial_inicio);
            tag(&mut xml, 6, "secuencialFin", &a.secuencial_fin);
            if let Some(ref aut) = a.autorizacion {
                if !aut.is_empty() {
                    tag(&mut xml, 6, "autorizacion", aut);
                }
            }
            xml.push_str("    </detalleAnulados>\n");
        }
        xml.push_str("  </anulados>\n");
    }

    xml.push_str("</iva>\n");
    xml
}

// ─── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_xml_basico() {
        let d = DatosAts {
            razon_social: "MI NEGOCIO SA".into(),
            ruc: "1234567890001".into(),
            anio: "2026".into(),
            mes: "05".into(),
            num_estab_ruc: "001".into(),
            total_ventas: 1500.0,
            codigo_operativo: "IVA".into(),
            compras: vec![],
            ventas: vec![],
            ventas_establecimiento: vec![VentaEstablecimiento {
                cod_estab: "001".into(),
                ventas_estab: 1500.0,
                iva_comp: 0.0,
            }],
            anulados: vec![],
        };
        let xml = generar_xml_ats(&d);
        assert!(xml.contains("<iva>"));
        assert!(xml.contains("<IdInformante>1234567890001</IdInformante>"));
        assert!(xml.contains("<Anio>2026</Anio>"));
        assert!(xml.contains("<Mes>05</Mes>"));
        assert!(xml.contains("<ventasEstablecimiento>"));
        assert!(xml.contains("</iva>"));
    }
}
