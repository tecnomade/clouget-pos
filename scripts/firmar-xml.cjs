/**
 * Script para firmar XML de comprobantes electrónicos SRI Ecuador.
 * Usa ec-sri-invoice-signer (battle-tested library).
 *
 * Uso: node firmar-xml.cjs <tipo_doc> <p12_path> <p12_password>
 *   - Lee XML sin firma de stdin
 *   - Escribe XML firmado a stdout
 *   - tipo_doc: "factura", "notaCredito", "notaDebito", "guiaRemision", "retencion"
 *
 * Ejemplo:
 *   echo "<factura ...>...</factura>" | node firmar-xml.cjs factura cert.p12 miPassword
 */

const fs = require('fs');
const path = require('path');

// ec-sri-invoice-signer is installed in the project root node_modules
const pkg = require('ec-sri-invoice-signer');

const FIRMA_POR_TIPO = {
  'factura': pkg.signInvoiceXml,
  '01': pkg.signInvoiceXml,
  'notaCredito': pkg.signCreditNoteXml,
  '04': pkg.signCreditNoteXml,
  'notaDebito': pkg.signDebitNoteXml,
  '05': pkg.signDebitNoteXml,
  'guiaRemision': pkg.signDeliveryGuideXml,
  '06': pkg.signDeliveryGuideXml,
  'retencion': pkg.signWithholdingCertificateXml,
  '07': pkg.signWithholdingCertificateXml,
};

async function main() {
  const args = process.argv.slice(2);
  if (args.length < 3) {
    process.stderr.write('Uso: node firmar-xml.cjs <tipo_doc> <p12_path> <p12_password>\n');
    process.exit(1);
  }

  const [tipoDoc, p12Path, p12Password] = args;

  // Leer XML de stdin
  const chunks = [];
  for await (const chunk of process.stdin) {
    chunks.push(chunk);
  }
  const xmlSinFirma = Buffer.concat(chunks).toString('utf-8');

  if (!xmlSinFirma.trim()) {
    process.stderr.write('ERROR: No se recibió XML en stdin\n');
    process.exit(1);
  }

  // Leer P12
  if (!fs.existsSync(p12Path)) {
    process.stderr.write(`ERROR: Archivo P12 no encontrado: ${p12Path}\n`);
    process.exit(1);
  }
  const p12Data = fs.readFileSync(p12Path);

  // Obtener función de firma
  const funcionFirma = FIRMA_POR_TIPO[tipoDoc];
  if (!funcionFirma) {
    process.stderr.write(`ERROR: Tipo de documento no soportado: ${tipoDoc}\n`);
    process.exit(1);
  }

  try {
    const xmlFirmado = funcionFirma(xmlSinFirma, p12Data, {
      pkcs12Password: p12Password,
    });
    process.stdout.write(xmlFirmado);
  } catch (error) {
    process.stderr.write(`ERROR: ${error.message}\n`);
    process.exit(1);
  }
}

main();
