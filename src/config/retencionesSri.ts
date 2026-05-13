/**
 * v2.5.4 — Catálogo de retenciones SRI Ecuador.
 *
 * Tabla 21 — Retenciones de IVA
 * Tabla 304 — Retenciones de Renta
 *
 * Solo incluimos los códigos más frecuentes para evitar abrumar al usuario.
 * Si necesita uno específico, puede tipear el código manualmente.
 *
 * IMPORTANTE: Los códigos y porcentajes los puede actualizar el SRI.
 * Verificar contra la última versión vigente del catálogo SRI.
 */

export interface RetencionSriDef {
  /** código de la tabla SRI */
  codigo: string;
  /** descripción legible */
  descripcion: string;
  /** porcentaje a retener (ej. 30 para 30%) */
  porcentaje: number;
}

/**
 * Retenciones de IVA — Tabla 21 SRI
 * Aplican sobre el VALOR DEL IVA de la factura (no sobre el subtotal).
 */
export const RETENCIONES_IVA: RetencionSriDef[] = [
  { codigo: "721", descripcion: "10% Bienes",                                porcentaje: 10 },
  { codigo: "723", descripcion: "20% Servicios prestados",                   porcentaje: 20 },
  { codigo: "725", descripcion: "30% Bienes",                                porcentaje: 30 },
  { codigo: "727", descripcion: "70% Servicios",                             porcentaje: 70 },
  { codigo: "729", descripcion: "100% Honorarios profesionales",             porcentaje: 100 },
  { codigo: "731", descripcion: "100% Arriendo de inmuebles personas naturales", porcentaje: 100 },
];

/**
 * Retenciones de Renta — Tabla 304 SRI (los más comunes)
 * Aplican sobre el SUBTOTAL (sin IVA) de la factura.
 */
export const RETENCIONES_RENTA: RetencionSriDef[] = [
  { codigo: "303", descripcion: "10% Honorarios profesionales",              porcentaje: 10 },
  { codigo: "304", descripcion: "8% Servicios donde predomina el intelecto", porcentaje: 8 },
  { codigo: "307", descripcion: "8% Servicios predomina mano de obra",       porcentaje: 8 },
  { codigo: "308", descripcion: "1.75% Servicios entre sociedades",          porcentaje: 1.75 },
  { codigo: "309", descripcion: "2% Otros servicios",                        porcentaje: 2 },
  { codigo: "312", descripcion: "1% Compra de bienes",                       porcentaje: 1 },
  { codigo: "320", descripcion: "1% Transporte privado de pasajeros",        porcentaje: 1 },
  { codigo: "322", descripcion: "1% Por arrendamiento mercantil",            porcentaje: 1 },
  { codigo: "327", descripcion: "1.75% Compra de bienes inmuebles",          porcentaje: 1.75 },
  { codigo: "332", descripcion: "Otros / Tarifa variable",                   porcentaje: 0 }, // usuario define %
];
