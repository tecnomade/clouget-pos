/**
 * Brand flag — controla qué features renderiza este build del frontend.
 *
 * Debe mantenerse sincronizado con `src-tauri/src/branding.rs::BRAND`.
 * El valor real se inyecta en build time desde `vite.config.ts` leyendo
 * la env var `CLOUGET_BRAND` (default: "clouget").
 *
 * Para generar el build de DigitalServer POS:
 *   1. Setear `CLOUGET_BRAND=digitalserver` en el entorno
 *   2. Cambiar `BRAND = Brand::DigitalServer` en `branding.rs`
 *   3. `npm run tauri build`
 *
 * Doble capa de control:
 *   - BRAND (compile-time)        → qué features EXISTEN en este build
 *   - LICENSE.modulos (runtime)   → qué features están ACTIVAS para este cliente
 */

export type Brand = "clouget" | "digitalserver";

export const BRAND: Brand = __BRAND__;

export const BRAND_NAME: string =
  BRAND === "clouget" ? "Clouget POS" : "DigitalServer POS";

/** Slug para URLs, paths, identificadores. */
export const BRAND_SLUG: string = BRAND;

/** Features gateadas por brand (compile-time). */
export const FEATURES = {
  /** Módulo Restaurante: mesas, comandas, app móvil meseros. Solo Clouget. */
  restaurante: BRAND === "clouget",
  /** Endpoints HTTP para app móvil de meseros. Solo Clouget. */
  appMovilMeseros: BRAND === "clouget",
} as const;

export type FeatureName = keyof typeof FEATURES;

/** Helper: ¿está esta feature disponible en este build? */
export function tieneFeature(feature: FeatureName): boolean {
  return FEATURES[feature];
}

/** Helper: ¿el cliente tiene activo este módulo en su licencia? */
export function tieneModuloLicencia(
  licenciaModulos: string[] | undefined | null,
  modulo: string,
): boolean {
  if (!licenciaModulos) return false;
  return licenciaModulos.includes(modulo);
}

/**
 * Helper combinado: feature visible si AMBOS:
 *  - el build la incluye (brand)
 *  - la licencia del cliente la habilita
 */
export function moduloDisponible(
  feature: FeatureName,
  licenciaModulos: string[] | undefined | null,
  moduloLicencia: string,
): boolean {
  return tieneFeature(feature) && tieneModuloLicencia(licenciaModulos, moduloLicencia);
}
