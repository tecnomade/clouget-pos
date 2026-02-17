use rand::Rng;

/// Genera la clave de acceso de 49 digitos para comprobantes electronicos SRI.
///
/// Estructura (48 digitos + 1 digito verificador):
/// - Posiciones 1-8:   fecha emision (ddmmyyyy)
/// - Posiciones 9-10:  codigo documento (01=factura, 04=nota credito)
/// - Posiciones 11-23: RUC emisor (13 digitos)
/// - Posicion 24:      ambiente (1=pruebas, 2=produccion)
/// - Posiciones 25-27: establecimiento (3 digitos)
/// - Posiciones 28-30: punto de emision (3 digitos)
/// - Posiciones 31-39: secuencial (9 digitos)
/// - Posiciones 40-47: codigo numerico aleatorio (8 digitos)
/// - Posicion 48:      tipo emision (1=normal)
/// - Posicion 49:      digito verificador (modulo 11)
pub fn generar_clave_acceso(
    fecha_emision: &str, // formato dd/mm/yyyy
    cod_doc: &str,       // "01" factura, "04" nota credito
    ruc: &str,           // 13 digitos
    ambiente: &str,      // "1" o "2"
    establecimiento: &str,
    punto_emision: &str,
    secuencial: &str,
    tipo_emision: &str,  // normalmente "1"
) -> String {
    let mut rng = rand::thread_rng();
    let codigo_numerico: u32 = rng.gen_range(10000000..99999999);

    let base = format!(
        "{}{}{}{}{}{}{}{}{}",
        fecha_emision.replace('/', ""),
        cod_doc,
        ruc,
        ambiente,
        establecimiento,
        punto_emision,
        secuencial,
        format!("{:08}", codigo_numerico),
        tipo_emision,
    );

    let dv = digito_verificador_modulo11(&base);
    format!("{}{}", base, dv)
}

/// Calcula el digito verificador usando modulo 11 con pesos [2,3,4,5,6,7]
/// ciclicos desde derecha a izquierda.
fn digito_verificador_modulo11(cadena: &str) -> u32 {
    let pesos = [2, 3, 4, 5, 6, 7];
    let mut suma: u32 = 0;

    for (i, ch) in cadena.chars().rev().enumerate() {
        let digito = ch.to_digit(10).unwrap_or(0);
        let peso = pesos[i % pesos.len()];
        suma += digito * peso;
    }

    let residuo = suma % 11;
    match 11 - residuo {
        11 => 0,
        10 => 1,
        dv => dv,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_digito_verificador() {
        // Verificar que el DV siempre es 0-9
        let result = digito_verificador_modulo11("123456789012345678901234567890123456789012345678");
        assert!(result <= 9);
    }

    #[test]
    fn test_clave_acceso_longitud() {
        let clave = generar_clave_acceso(
            "11/02/2026",
            "01",
            "1234567890001",
            "1",
            "001",
            "001",
            "000000001",
            "1",
        );
        assert_eq!(clave.len(), 49);
        // Todos deben ser digitos
        assert!(clave.chars().all(|c| c.is_ascii_digit()));
    }

    #[test]
    fn test_clave_acceso_estructura() {
        let clave = generar_clave_acceso(
            "15/03/2026",
            "01",
            "0912345678001",
            "2",
            "001",
            "002",
            "000000123",
            "1",
        );
        // fecha ddmmyyyy
        assert_eq!(&clave[0..8], "15032026");
        // cod doc
        assert_eq!(&clave[8..10], "01");
        // ruc
        assert_eq!(&clave[10..23], "0912345678001");
        // ambiente
        assert_eq!(&clave[23..24], "2");
        // establecimiento
        assert_eq!(&clave[24..27], "001");
        // punto emision
        assert_eq!(&clave[27..30], "002");
        // secuencial
        assert_eq!(&clave[30..39], "000000123");
        // tipo emision
        assert_eq!(&clave[47..48], "1");
    }
}
