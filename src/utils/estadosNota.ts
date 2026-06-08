/**
 * v2.5.68 — Estados DERIVADOS de una Nota de Entrega / Guía.
 *
 * Según la arquitectura objetivo, los estados NO deben depender de botones
 * manuales ni mezclarse en un solo campo monstruoso. Se derivan automáticamente
 * de los datos existentes (estado operativo, estado_sri, tipo_documento) y se
 * separan por contexto. No se guardan físicamente: se calculan al mostrar.
 */

export interface EstadosDerivados {
  operativo: { texto: string; color: string };
  comercial: { texto: string; color: string };
  tributario: { texto: string; color: string };
}

interface FuenteEstado {
  estado?: string;          // PENDIENTE / ENTREGADA / RECHAZADA / FACTURADA / COMPLETADA / ANULADA
  estado_sri?: string;      // NO_APLICA / PENDIENTE / RECHAZADA / AUTORIZADA
  anulada?: boolean | number | null;
  despacho_estado?: string | null; // PREPARANDO / EN_TRANSITO / ENTREGADO / DEVUELTO / PARCIAL
}

const C = {
  warning: "var(--color-warning)",
  success: "var(--color-success)",
  danger: "var(--color-danger)",
  primary: "var(--color-primary)",
  secondary: "var(--color-text-secondary)",
};

/**
 * Deriva los 3 estados (operativo, comercial, tributario) de una nota/guía.
 * Todo se calcula a partir de los datos ya existentes — sin nuevos campos en BD.
 */
export function derivarEstadosNota(v: FuenteEstado): EstadosDerivados {
  const estado = (v.estado || "").toUpperCase();
  const sri = (v.estado_sri || "NO_APLICA").toUpperCase();
  const desp = (v.despacho_estado || "").toUpperCase();
  const anulada = v.anulada === true || v.anulada === 1;

  // ── Estado OPERATIVO (movimiento físico) ──
  // Prioridad: el despacho gestionado (Fase C) es la fuente real; si no hay,
  // se infiere del estado comercial.
  let operativo = { texto: "Despachada", color: C.warning };
  if (anulada || estado === "ANULADA") operativo = { texto: "Anulada", color: C.danger };
  else if (desp === "DEVUELTO" || estado === "RECHAZADA") operativo = { texto: "Devuelta", color: C.danger };
  else if (desp === "ENTREGADO") operativo = { texto: "Entregada", color: C.success };
  else if (desp === "EN_TRANSITO") operativo = { texto: "En tránsito", color: C.primary };
  else if (desp === "PARCIAL") operativo = { texto: "Entrega parcial", color: C.warning };
  else if (desp === "PREPARANDO") operativo = { texto: "Preparando", color: C.warning };
  else if (estado === "ENTREGADA") operativo = { texto: "Entregada", color: C.success };
  else if (estado === "FACTURADA" || estado === "COMPLETADA") operativo = { texto: "Entregada", color: C.success };
  // v2.6.20: una nota PENDIENTE aún no fue recibida → "En tránsito" (antes "Despachada").
  else if (estado === "PENDIENTE") operativo = { texto: "En tránsito", color: C.primary };
  else operativo = { texto: "Despachada", color: C.warning };

  // ── Estado COMERCIAL (operación de venta) ──
  let comercial = { texto: "Pendiente venta", color: C.warning };
  if (estado === "FACTURADA" || estado === "COMPLETADA") comercial = { texto: "Convertida en venta", color: C.success };
  else if (anulada || estado === "ANULADA" || estado === "RECHAZADA") comercial = { texto: "Sin venta", color: C.secondary };

  // ── Estado TRIBUTARIO (sustento legal del traslado) ──
  let tributario = { texto: "Sin guía SRI", color: C.secondary };
  if (sri === "AUTORIZADA") tributario = { texto: "Guía autorizada SRI", color: C.success };
  else if (sri === "PENDIENTE") tributario = { texto: "Guía pendiente SRI", color: C.warning };
  else if (sri === "RECHAZADA") tributario = { texto: "Guía rechazada SRI", color: C.danger };

  return { operativo, comercial, tributario };
}
