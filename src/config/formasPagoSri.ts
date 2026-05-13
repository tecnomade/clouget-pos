/**
 * v2.5.2 — Catálogo de formas de pago SRI Ecuador (Tabla 24).
 *
 * Cada forma POS interna mapea a un código SRI que se usa al emitir
 * la factura electrónica. El backend hace el mapeo final en
 * `src-tauri/src/sri/xml.rs::forma_pago_sri` — esta tabla es la fuente
 * de verdad del frontend para mostrar al usuario qué se va a reportar.
 */

export interface FormaPagoSri {
  /** código interno usado en la app (ventas.forma_pago, pagos_venta.forma_pago) */
  codigo: string;
  /** etiqueta visible en UI */
  label: string;
  /** código SRI Tabla 24 que se reporta al emitir factura electrónica */
  codigoSri: string;
  /** descripción SRI oficial */
  descripcionSri: string;
  /** emoji / icono opcional */
  icono?: string;
}

// v2.5.5: códigos POS estandarizados por backward-compat con datos existentes.
// El backend Rust (`forma_pago_sri` en xml.rs) acepta tanto los códigos legacy
// (TRANSFERENCIA, DEBITO) como los nuevos (TRANSFER, TARJETA_DEBITO). El catálogo
// usa los códigos legacy para no romper compras / ventas guardadas con esos valores.
export const FORMAS_PAGO_SRI: FormaPagoSri[] = [
  { codigo: "EFECTIVO",          label: "Efectivo",            codigoSri: "01", descripcionSri: "Sin utilización del sistema financiero", icono: "💵" },
  { codigo: "CHEQUE",            label: "Cheque",              codigoSri: "20", descripcionSri: "Otros con utilización del sistema financiero", icono: "🧾" },
  { codigo: "TRANSFERENCIA",     label: "Transferencia",       codigoSri: "20", descripcionSri: "Otros con utilización del sistema financiero", icono: "🏦" },
  { codigo: "DEBITO",            label: "Tarjeta de débito",   codigoSri: "16", descripcionSri: "Tarjeta de débito",                            icono: "💳" },
  { codigo: "TARJETA",           label: "Tarjeta de crédito",  codigoSri: "19", descripcionSri: "Tarjeta de crédito",                           icono: "💳" },
  { codigo: "TARJETA_PREPAGO",   label: "Tarjeta prepago",     codigoSri: "18", descripcionSri: "Tarjeta prepago",                              icono: "💳" },
  { codigo: "DINERO_ELECTRONICO", label: "Dinero electrónico", codigoSri: "17", descripcionSri: "Dinero electrónico (BCE)",                    icono: "📱" },
  { codigo: "COMPENSACION",      label: "Compensación / canje", codigoSri: "15", descripcionSri: "Compensación de deudas",                      icono: "🔄" },
  { codigo: "CREDITO",           label: "Crédito (queda por pagar)", codigoSri: "20", descripcionSri: "Otros con utilización del sistema financiero", icono: "📋" },
];

/** Helper: obtener forma SRI por código interno */
export function getFormaPagoSri(codigo: string): FormaPagoSri | undefined {
  return FORMAS_PAGO_SRI.find(f => f.codigo === codigo.toUpperCase());
}

/** Helper: obtener label visible para un código interno */
export function labelFormaPago(codigo: string): string {
  return getFormaPagoSri(codigo)?.label || codigo;
}

/** Helper: obtener código SRI Tabla 24 para un código interno */
export function codigoSriDeForma(codigo: string): string {
  return getFormaPagoSri(codigo)?.codigoSri || "01";
}
