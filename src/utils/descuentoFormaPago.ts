/**
 * Helper: cálculo de descuento automático según forma de pago.
 * v2.3.63 — política configurable desde admin (Configuración → Descuentos).
 *
 * Casos típicos en Ecuador:
 *   - Efectivo: descuento (sin comisión bancaria, dinero inmediato)
 *   - Tarjeta:  precio normal (banco cobra ~3-5% al comerciante)
 *   - Transfer: descuento menor que efectivo (comisión bancaria menor)
 *   - Crédito:  sin descuento (riesgo de no pago)
 *   - Mixto:    sin descuento (decisión del usuario para evitar gaming)
 *
 * Uso:
 *   const config = await obtenerConfig();
 *   const result = calcularDescuentoFormaPago("EFECTIVO", subtotalSinIva, totalConIva, config);
 *   // → { activo, porcentaje, montoDescuento, etiqueta, totalFinal }
 */

export interface DescuentoConfig {
  activo: boolean;
  /** "SUBTOTAL_SIN_IVA" (default) | "TOTAL_CON_IVA" */
  aplicarSobre: "SUBTOTAL_SIN_IVA" | "TOTAL_CON_IVA";
  porcentajes: {
    efectivo: number;
    tarjeta: number;
    transfer: number;
    credito: number;
  };
  montoMinimo: number;
}

export interface DescuentoResult {
  activo: boolean;
  porcentaje: number;
  /** Monto del descuento en dinero ($X.XX) */
  montoDescuento: number;
  /** Etiqueta visible al cajero, ej: "Descuento -5% por pago en EFECTIVO" */
  etiqueta: string;
  /** Total después del descuento (subtotal o total según `aplicarSobre`) */
  totalFinal: number;
  /** Si NO se aplicó descuento, el motivo (para mostrar tooltip/info) */
  motivoNoAplica?: string;
}

/** Lee la configuración de descuentos desde el objeto config plano (key-value). */
export function leerConfigDescuento(config: Record<string, string>): DescuentoConfig {
  return {
    activo: config.descuento_forma_pago_activo === "1",
    aplicarSobre: (config.descuento_forma_pago_aplicar_sobre || "SUBTOTAL_SIN_IVA") as
      | "SUBTOTAL_SIN_IVA"
      | "TOTAL_CON_IVA",
    porcentajes: {
      efectivo: parseFloat(config.descuento_efectivo_pct || "0") || 0,
      tarjeta: parseFloat(config.descuento_tarjeta_pct || "0") || 0,
      transfer: parseFloat(config.descuento_transfer_pct || "0") || 0,
      credito: parseFloat(config.descuento_credito_pct || "0") || 0,
    },
    montoMinimo: parseFloat(config.descuento_forma_pago_minimo || "0") || 0,
  };
}

/** Normaliza la forma de pago a una key del config */
function normalizarFormaPago(formaPago: string): "efectivo" | "tarjeta" | "transfer" | "credito" | "mixto" | null {
  const f = formaPago.toUpperCase();
  if (f === "EFECTIVO") return "efectivo";
  if (f === "TARJETA") return "tarjeta";
  if (f === "TRANSFER" || f === "TRANSFERENCIA") return "transfer";
  if (f === "CREDITO" || f === "CRÉDITO" || f === "FIADO") return "credito";
  if (f === "MIXTO") return "mixto";
  return null;
}

/** Etiqueta amigable para mostrar al cajero. */
function etiquetaFormaPago(forma: "efectivo" | "tarjeta" | "transfer" | "credito" | "mixto"): string {
  const map = {
    efectivo: "EFECTIVO",
    tarjeta: "TARJETA",
    transfer: "TRANSFERENCIA",
    credito: "CRÉDITO",
    mixto: "MIXTO",
  };
  return map[forma];
}

/**
 * Calcula el descuento automático según la forma de pago elegida.
 *
 * @param formaPago Forma de pago seleccionada (EFECTIVO/TARJETA/TRANSFER/CREDITO/MIXTO)
 * @param subtotalSinIva Subtotal de la venta SIN IVA
 * @param totalConIva Total con IVA aplicado
 * @param config Config global (de obtenerConfig())
 */
export function calcularDescuentoFormaPago(
  formaPago: string,
  subtotalSinIva: number,
  totalConIva: number,
  config: DescuentoConfig,
): DescuentoResult {
  const baseSinDescuento = config.aplicarSobre === "TOTAL_CON_IVA" ? totalConIva : subtotalSinIva;

  // 1. Feature desactivada → sin descuento
  if (!config.activo) {
    return {
      activo: false,
      porcentaje: 0,
      montoDescuento: 0,
      etiqueta: "",
      totalFinal: baseSinDescuento,
    };
  }

  const forma = normalizarFormaPago(formaPago);
  if (!forma) {
    return {
      activo: false,
      porcentaje: 0,
      montoDescuento: 0,
      etiqueta: "",
      totalFinal: baseSinDescuento,
      motivoNoAplica: "Forma de pago no reconocida",
    };
  }

  // 2. Pago MIXTO → sin descuento (decisión del usuario por simplicidad)
  if (forma === "mixto") {
    return {
      activo: false,
      porcentaje: 0,
      montoDescuento: 0,
      etiqueta: "",
      totalFinal: baseSinDescuento,
      motivoNoAplica: "Pago mixto no aplica descuento automático",
    };
  }

  const porcentaje = config.porcentajes[forma];

  // 3. Porcentaje 0 (no configurado para este método) → sin descuento
  if (porcentaje <= 0) {
    return {
      activo: false,
      porcentaje: 0,
      montoDescuento: 0,
      etiqueta: "",
      totalFinal: baseSinDescuento,
      motivoNoAplica: `Sin descuento configurado para ${etiquetaFormaPago(forma)}`,
    };
  }

  // 4. Monto mínimo no alcanzado → sin descuento
  if (config.montoMinimo > 0 && totalConIva < config.montoMinimo) {
    return {
      activo: false,
      porcentaje: 0,
      montoDescuento: 0,
      etiqueta: "",
      totalFinal: baseSinDescuento,
      motivoNoAplica: `Monto mínimo $${config.montoMinimo.toFixed(2)} no alcanzado`,
    };
  }

  // 5. Aplicar descuento
  const montoDescuento = (baseSinDescuento * porcentaje) / 100;
  const totalFinal = baseSinDescuento - montoDescuento;

  return {
    activo: true,
    porcentaje,
    montoDescuento: Math.round(montoDescuento * 100) / 100, // redondear a 2 decimales
    etiqueta: `Descuento -${porcentaje}% por pago en ${etiquetaFormaPago(forma)}`,
    totalFinal: Math.round(totalFinal * 100) / 100,
  };
}
