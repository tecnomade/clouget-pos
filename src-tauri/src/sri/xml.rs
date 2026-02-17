use serde::{Deserialize, Serialize};

/// Datos necesarios para generar el XML de una factura electronica SRI v2.0.0
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DatosFactura {
    // Info tributaria
    pub ambiente: String,       // "1" pruebas, "2" produccion
    pub tipo_emision: String,   // "1" normal
    pub razon_social: String,
    pub nombre_comercial: String,
    pub ruc: String,
    pub clave_acceso: String,
    pub cod_doc: String,        // "01" factura
    pub estab: String,          // "001"
    pub pto_emi: String,        // "001"
    pub secuencial: String,     // "000000001"
    pub dir_matriz: String,

    // Info factura
    pub fecha_emision: String,  // dd/mm/yyyy
    pub dir_establecimiento: String,
    pub obligado_contabilidad: String, // "SI" o "NO"
    pub contribuyente_rimpe: Option<String>,
    pub tipo_identificacion_comprador: String, // "04"=RUC, "05"=cedula, "07"=CF
    pub razon_social_comprador: String,
    pub identificacion_comprador: String,
    pub direccion_comprador: Option<String>,

    // Totales
    pub total_sin_impuestos: f64,
    pub total_descuento: f64,
    pub importe_total: f64,

    // Impuestos totales agrupados
    pub impuestos_totales: Vec<ImpuestoTotal>,

    // Pagos
    pub pagos: Vec<PagoFactura>,

    // Detalles
    pub detalles: Vec<DetalleFactura>,

    // Info adicional (opcional)
    pub info_adicional: Vec<CampoAdicional>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ImpuestoTotal {
    pub codigo: String,              // "2" = IVA
    pub codigo_porcentaje: String,   // "0"=0%, "4"=15%, etc
    pub base_imponible: f64,
    pub valor: f64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PagoFactura {
    pub forma_pago: String, // "01"=efectivo, "20"=otros
    pub total: f64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DetalleFactura {
    pub codigo_principal: String,
    pub descripcion: String,
    pub cantidad: f64,
    pub precio_unitario: f64,
    pub descuento: f64,
    pub precio_total_sin_impuesto: f64,
    pub codigo_porcentaje_iva: String,
    pub tarifa_iva: f64,
    pub base_imponible: f64,
    pub valor_iva: f64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CampoAdicional {
    pub nombre: String,
    pub valor: String,
}

/// Codigos de porcentaje IVA del SRI
pub fn tarifa_iva(codigo: &str) -> f64 {
    match codigo {
        "0" => 0.0,
        "2" => 12.0,
        "3" => 14.0,
        "4" => 15.0,
        "5" => 5.0,
        "6" => 0.0, // no objeto de impuesto
        "7" => 0.0, // exento
        _ => 15.0,  // default 15%
    }
}

/// Mapea forma de pago POS a codigo SRI
pub fn forma_pago_sri(forma_pos: &str) -> &str {
    match forma_pos {
        "EFECTIVO" => "01",
        "TRANSFERENCIA" => "20",
        "TARJETA" => "19",
        _ => "01",
    }
}

/// Genera el XML de factura electronica SRI v2.0.0
///
/// IMPORTANTE: No usa self-closing tags (<tag/>) porque el SRI los rechaza.
/// Todos los tags usan formato <tag></tag>.
pub fn generar_xml_factura(datos: &DatosFactura) -> String {
    let mut xml = String::with_capacity(8192);

    xml.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
    xml.push_str("<factura id=\"comprobante\" version=\"2.0.0\">\n");

    // === infoTributaria ===
    xml.push_str("  <infoTributaria>\n");
    xml_tag(&mut xml, 4, "ambiente", &datos.ambiente);
    xml_tag(&mut xml, 4, "tipoEmision", &datos.tipo_emision);
    xml_tag(&mut xml, 4, "razonSocial", &xml_escape(&datos.razon_social));
    xml_tag(&mut xml, 4, "nombreComercial", &xml_escape(&datos.nombre_comercial));
    xml_tag(&mut xml, 4, "ruc", &datos.ruc);
    xml_tag(&mut xml, 4, "claveAcceso", &datos.clave_acceso);
    xml_tag(&mut xml, 4, "codDoc", &datos.cod_doc);
    xml_tag(&mut xml, 4, "estab", &datos.estab);
    xml_tag(&mut xml, 4, "ptoEmi", &datos.pto_emi);
    xml_tag(&mut xml, 4, "secuencial", &datos.secuencial);
    xml_tag(&mut xml, 4, "dirMatriz", &xml_escape(&datos.dir_matriz));

    if let Some(ref rimpe) = datos.contribuyente_rimpe {
        xml_tag(&mut xml, 4, "contribuyenteRimpe", rimpe);
    }

    xml.push_str("  </infoTributaria>\n");

    // === infoFactura ===
    xml.push_str("  <infoFactura>\n");
    xml_tag(&mut xml, 4, "fechaEmision", &datos.fecha_emision);
    xml_tag(&mut xml, 4, "dirEstablecimiento", &xml_escape(&datos.dir_establecimiento));
    xml_tag(&mut xml, 4, "obligadoContabilidad", &datos.obligado_contabilidad);
    xml_tag(&mut xml, 4, "tipoIdentificacionComprador", &datos.tipo_identificacion_comprador);
    xml_tag(&mut xml, 4, "razonSocialComprador", &xml_escape(&datos.razon_social_comprador));
    xml_tag(&mut xml, 4, "identificacionComprador", &datos.identificacion_comprador);

    if let Some(ref dir) = datos.direccion_comprador {
        if !dir.is_empty() {
            xml_tag(&mut xml, 4, "direccionComprador", &xml_escape(dir));
        }
    }

    xml_tag(&mut xml, 4, "totalSinImpuestos", &format!("{:.2}", datos.total_sin_impuestos));
    xml_tag(&mut xml, 4, "totalDescuento", &format!("{:.2}", datos.total_descuento));

    // totalConImpuestos
    xml.push_str("    <totalConImpuestos>\n");
    for imp in &datos.impuestos_totales {
        xml.push_str("      <totalImpuesto>\n");
        xml_tag(&mut xml, 8, "codigo", &imp.codigo);
        xml_tag(&mut xml, 8, "codigoPorcentaje", &imp.codigo_porcentaje);
        xml_tag(&mut xml, 8, "baseImponible", &format!("{:.2}", imp.base_imponible));
        xml_tag(&mut xml, 8, "valor", &format!("{:.2}", imp.valor));
        xml.push_str("      </totalImpuesto>\n");
    }
    xml.push_str("    </totalConImpuestos>\n");

    xml_tag(&mut xml, 4, "propina", "0.00");
    xml_tag(&mut xml, 4, "importeTotal", &format!("{:.2}", datos.importe_total));
    xml_tag(&mut xml, 4, "moneda", "DOLAR");

    // pagos
    xml.push_str("    <pagos>\n");
    for pago in &datos.pagos {
        xml.push_str("      <pago>\n");
        xml_tag(&mut xml, 8, "formaPago", &pago.forma_pago);
        xml_tag(&mut xml, 8, "total", &format!("{:.2}", pago.total));
        xml.push_str("      </pago>\n");
    }
    xml.push_str("    </pagos>\n");

    xml.push_str("  </infoFactura>\n");

    // === detalles ===
    xml.push_str("  <detalles>\n");
    for det in &datos.detalles {
        xml.push_str("    <detalle>\n");
        xml_tag(&mut xml, 6, "codigoPrincipal", &det.codigo_principal);
        xml_tag(&mut xml, 6, "descripcion", &xml_escape(&det.descripcion));
        xml_tag(&mut xml, 6, "cantidad", &format!("{:.6}", det.cantidad));
        xml_tag(&mut xml, 6, "precioUnitario", &format!("{:.6}", det.precio_unitario));
        xml_tag(&mut xml, 6, "descuento", &format!("{:.2}", det.descuento));
        xml_tag(&mut xml, 6, "precioTotalSinImpuesto", &format!("{:.2}", det.precio_total_sin_impuesto));

        xml.push_str("      <impuestos>\n");
        xml.push_str("        <impuesto>\n");
        xml_tag(&mut xml, 10, "codigo", "2"); // IVA
        xml_tag(&mut xml, 10, "codigoPorcentaje", &det.codigo_porcentaje_iva);
        xml_tag(&mut xml, 10, "tarifa", &format!("{:.2}", det.tarifa_iva));
        xml_tag(&mut xml, 10, "baseImponible", &format!("{:.2}", det.base_imponible));
        xml_tag(&mut xml, 10, "valor", &format!("{:.2}", det.valor_iva));
        xml.push_str("        </impuesto>\n");
        xml.push_str("      </impuestos>\n");

        xml.push_str("    </detalle>\n");
    }
    xml.push_str("  </detalles>\n");

    // === infoAdicional (opcional) ===
    if !datos.info_adicional.is_empty() {
        xml.push_str("  <infoAdicional>\n");
        for campo in &datos.info_adicional {
            xml.push_str(&format!(
                "    <campoAdicional nombre=\"{}\">{}</campoAdicional>\n",
                xml_escape(&campo.nombre),
                xml_escape(&campo.valor)
            ));
        }
        xml.push_str("  </infoAdicional>\n");
    }

    xml.push_str("</factura>");
    xml
}

/// Escribe un tag XML sin self-closing: <tag>value</tag>
fn xml_tag(xml: &mut String, indent: usize, tag: &str, value: &str) {
    let spaces = " ".repeat(indent);
    xml.push_str(&format!("{}<{}>{}</{}>\n", spaces, tag, value, tag));
}

/// Normaliza texto para XML del SRI Ecuador.
/// Remueve caracteres de control, normaliza Unicode problematico y espacios.
fn normalize_text(s: &str) -> String {
    let mut result = String::with_capacity(s.len());

    for ch in s.chars() {
        match ch {
            // Remover caracteres de control (excepto tab, newline, carriage return)
            '\x00'..='\x08' | '\x0B' | '\x0C' | '\x0E'..='\x1F' | '\x7F' => {}
            // Comillas simples curvas -> recta
            '\u{2018}' | '\u{2019}' | '\u{201A}' | '\u{201B}' => result.push('\''),
            // Comillas dobles curvas -> recta
            '\u{201C}' | '\u{201D}' | '\u{201E}' | '\u{201F}' => result.push('"'),
            // Guiones largos -> guion normal
            '\u{2013}' | '\u{2014}' | '\u{2015}' => result.push('-'),
            // Elipsis -> tres puntos
            '\u{2026}' => result.push_str("..."),
            // Non-breaking space -> espacio normal
            '\u{00A0}' => result.push(' '),
            // Soft hyphen -> remover
            '\u{00AD}' => {}
            // Cualquier otro caracter: mantener
            _ => result.push(ch),
        }
    }

    // Colapsar multiples espacios en uno y trim
    let mut collapsed = String::with_capacity(result.len());
    let mut prev_space = false;
    for ch in result.trim().chars() {
        if ch.is_whitespace() {
            if !prev_space {
                collapsed.push(' ');
            }
            prev_space = true;
        } else {
            collapsed.push(ch);
            prev_space = false;
        }
    }

    collapsed
}

/// Escapa caracteres especiales XML (con normalizacion previa)
fn xml_escape(s: &str) -> String {
    let normalized = normalize_text(s);
    normalized
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

// === NOTA DE CRÉDITO ===

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DatosNotaCredito {
    // Info tributaria (mismos campos que factura)
    pub ambiente: String,
    pub tipo_emision: String,
    pub razon_social: String,
    pub nombre_comercial: String,
    pub ruc: String,
    pub clave_acceso: String,
    pub cod_doc: String,        // "04" nota de crédito
    pub estab: String,
    pub pto_emi: String,
    pub secuencial: String,
    pub dir_matriz: String,
    pub contribuyente_rimpe: Option<String>,

    // Info nota de crédito
    pub fecha_emision: String,
    pub dir_establecimiento: String,
    pub obligado_contabilidad: String,
    pub tipo_identificacion_comprador: String,
    pub razon_social_comprador: String,
    pub identificacion_comprador: String,

    // Referencia al documento modificado
    pub cod_doc_modificado: String,       // "01" factura
    pub num_doc_modificado: String,       // "001-001-000000001"
    pub fecha_emision_doc_sustento: String, // dd/mm/yyyy

    pub rise: Option<String>,
    pub motivo: String,

    // Totales
    pub total_sin_impuestos: f64,
    pub importe_total: f64,
    pub impuestos_totales: Vec<ImpuestoTotal>,

    // Detalles (reutiliza DetalleFactura)
    pub detalles: Vec<DetalleFactura>,

    pub info_adicional: Vec<CampoAdicional>,
}

/// Genera XML de nota de crédito electrónica SRI v1.1.0
pub fn generar_xml_nota_credito(datos: &DatosNotaCredito) -> String {
    let mut xml = String::with_capacity(8192);

    xml.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
    xml.push_str("<notaCredito id=\"comprobante\" version=\"1.1.0\">\n");

    // === infoTributaria ===
    xml.push_str("  <infoTributaria>\n");
    xml_tag(&mut xml, 4, "ambiente", &datos.ambiente);
    xml_tag(&mut xml, 4, "tipoEmision", &datos.tipo_emision);
    xml_tag(&mut xml, 4, "razonSocial", &xml_escape(&datos.razon_social));
    xml_tag(&mut xml, 4, "nombreComercial", &xml_escape(&datos.nombre_comercial));
    xml_tag(&mut xml, 4, "ruc", &datos.ruc);
    xml_tag(&mut xml, 4, "claveAcceso", &datos.clave_acceso);
    xml_tag(&mut xml, 4, "codDoc", &datos.cod_doc);
    xml_tag(&mut xml, 4, "estab", &datos.estab);
    xml_tag(&mut xml, 4, "ptoEmi", &datos.pto_emi);
    xml_tag(&mut xml, 4, "secuencial", &datos.secuencial);
    xml_tag(&mut xml, 4, "dirMatriz", &xml_escape(&datos.dir_matriz));

    if let Some(ref rimpe) = datos.contribuyente_rimpe {
        xml_tag(&mut xml, 4, "contribuyenteRimpe", rimpe);
    }

    xml.push_str("  </infoTributaria>\n");

    // === infoNotaCredito ===
    xml.push_str("  <infoNotaCredito>\n");
    xml_tag(&mut xml, 4, "fechaEmision", &datos.fecha_emision);
    xml_tag(&mut xml, 4, "dirEstablecimiento", &xml_escape(&datos.dir_establecimiento));
    xml_tag(&mut xml, 4, "tipoIdentificacionComprador", &datos.tipo_identificacion_comprador);
    xml_tag(&mut xml, 4, "razonSocialComprador", &xml_escape(&datos.razon_social_comprador));
    xml_tag(&mut xml, 4, "identificacionComprador", &datos.identificacion_comprador);
    xml_tag(&mut xml, 4, "obligadoContabilidad", &datos.obligado_contabilidad);

    // Referencia al documento modificado
    xml_tag(&mut xml, 4, "codDocModificado", &datos.cod_doc_modificado);
    xml_tag(&mut xml, 4, "numDocModificado", &datos.num_doc_modificado);
    xml_tag(&mut xml, 4, "fechaEmisionDocSustento", &datos.fecha_emision_doc_sustento);
    xml_tag(&mut xml, 4, "totalSinImpuestos", &format!("{:.2}", datos.total_sin_impuestos));
    xml_tag(&mut xml, 4, "valorModificacion", &format!("{:.2}", datos.importe_total));
    xml_tag(&mut xml, 4, "moneda", "DOLAR");

    // totalConImpuestos
    xml.push_str("    <totalConImpuestos>\n");
    for imp in &datos.impuestos_totales {
        xml.push_str("      <totalImpuesto>\n");
        xml_tag(&mut xml, 8, "codigo", &imp.codigo);
        xml_tag(&mut xml, 8, "codigoPorcentaje", &imp.codigo_porcentaje);
        xml_tag(&mut xml, 8, "baseImponible", &format!("{:.2}", imp.base_imponible));
        xml_tag(&mut xml, 8, "valor", &format!("{:.2}", imp.valor));
        xml.push_str("      </totalImpuesto>\n");
    }
    xml.push_str("    </totalConImpuestos>\n");

    xml_tag(&mut xml, 4, "motivo", &xml_escape(&datos.motivo));

    xml.push_str("  </infoNotaCredito>\n");

    // === detalles ===
    xml.push_str("  <detalles>\n");
    for det in &datos.detalles {
        xml.push_str("    <detalle>\n");
        xml_tag(&mut xml, 6, "codigoInterno", &det.codigo_principal);
        xml_tag(&mut xml, 6, "descripcion", &xml_escape(&det.descripcion));
        xml_tag(&mut xml, 6, "cantidad", &format!("{:.6}", det.cantidad));
        xml_tag(&mut xml, 6, "precioUnitario", &format!("{:.6}", det.precio_unitario));
        xml_tag(&mut xml, 6, "descuento", &format!("{:.2}", det.descuento));
        xml_tag(&mut xml, 6, "precioTotalSinImpuesto", &format!("{:.2}", det.precio_total_sin_impuesto));

        xml.push_str("      <impuestos>\n");
        xml.push_str("        <impuesto>\n");
        xml_tag(&mut xml, 10, "codigo", "2");
        xml_tag(&mut xml, 10, "codigoPorcentaje", &det.codigo_porcentaje_iva);
        xml_tag(&mut xml, 10, "tarifa", &format!("{:.2}", det.tarifa_iva));
        xml_tag(&mut xml, 10, "baseImponible", &format!("{:.2}", det.base_imponible));
        xml_tag(&mut xml, 10, "valor", &format!("{:.2}", det.valor_iva));
        xml.push_str("        </impuesto>\n");
        xml.push_str("      </impuestos>\n");

        xml.push_str("    </detalle>\n");
    }
    xml.push_str("  </detalles>\n");

    // === infoAdicional (opcional) ===
    if !datos.info_adicional.is_empty() {
        xml.push_str("  <infoAdicional>\n");
        for campo in &datos.info_adicional {
            xml.push_str(&format!(
                "    <campoAdicional nombre=\"{}\">{}</campoAdicional>\n",
                xml_escape(&campo.nombre),
                xml_escape(&campo.valor)
            ));
        }
        xml.push_str("  </infoAdicional>\n");
    }

    xml.push_str("</notaCredito>");
    xml
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generar_xml_basico() {
        let datos = DatosFactura {
            ambiente: "1".to_string(),
            tipo_emision: "1".to_string(),
            razon_social: "NEGOCIO TEST".to_string(),
            nombre_comercial: "NEGOCIO TEST".to_string(),
            ruc: "0912345678001".to_string(),
            clave_acceso: "1".repeat(49),
            cod_doc: "01".to_string(),
            estab: "001".to_string(),
            pto_emi: "001".to_string(),
            secuencial: "000000001".to_string(),
            dir_matriz: "Guayaquil".to_string(),
            fecha_emision: "11/02/2026".to_string(),
            dir_establecimiento: "Guayaquil".to_string(),
            obligado_contabilidad: "NO".to_string(),
            contribuyente_rimpe: Some("CONTRIBUYENTE RÉGIMEN RIMPE".to_string()),
            tipo_identificacion_comprador: "07".to_string(),
            razon_social_comprador: "CONSUMIDOR FINAL".to_string(),
            identificacion_comprador: "9999999999999".to_string(),
            direccion_comprador: None,
            total_sin_impuestos: 10.0,
            total_descuento: 0.0,
            importe_total: 11.50,
            impuestos_totales: vec![ImpuestoTotal {
                codigo: "2".to_string(),
                codigo_porcentaje: "4".to_string(),
                base_imponible: 10.0,
                valor: 1.50,
            }],
            pagos: vec![PagoFactura {
                forma_pago: "01".to_string(),
                total: 11.50,
            }],
            detalles: vec![DetalleFactura {
                codigo_principal: "PROD001".to_string(),
                descripcion: "Producto Test".to_string(),
                cantidad: 1.0,
                precio_unitario: 10.0,
                descuento: 0.0,
                precio_total_sin_impuesto: 10.0,
                codigo_porcentaje_iva: "4".to_string(),
                tarifa_iva: 15.0,
                base_imponible: 10.0,
                valor_iva: 1.50,
            }],
            info_adicional: vec![],
        };

        let xml = generar_xml_factura(&datos);
        assert!(xml.contains("<factura id=\"comprobante\" version=\"2.0.0\">"));
        assert!(xml.contains("<ruc>0912345678001</ruc>"));
        assert!(xml.contains("<contribuyenteRimpe>"));
        assert!(xml.contains("</factura>"));
        // No debe tener self-closing tags
        assert!(!xml.contains("/>"));
    }
}
