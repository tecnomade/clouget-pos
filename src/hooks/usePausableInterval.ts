/**
 * v2.5.60: hook helper para pausar polling cuando la tab interna no está activa.
 *
 * Problema: el POS mantiene TODAS las páginas montadas con `display:none` cuando
 * el usuario alterna entre tabs (multi-vista). Si una página tiene un
 * `setInterval(fn, 60_000)`, sigue corriendo aunque la tab esté oculta —
 * consume CPU + RAM + hace queries SQL/HTTP innecesarias.
 *
 * Solución: este hook detiene el interval cuando la tab no está activa y lo
 * re-arma al volver. Opcionalmente, ejecuta `callback()` UNA VEZ inmediatamente
 * al re-activarse (útil para refrescar data que pudo cambiar mientras estaba
 * oculta).
 *
 * Si `activePath` es undefined, asume que está siempre activo (modo clásico
 * sin tabs).
 *
 * Uso típico:
 *   usePausableInterval(() => cargar(), 15_000, "/mesas", { runOnReactivate: true });
 */
import { useEffect, useRef } from "react";
import { useIsActiveTab } from "../contexts/TabsContext";

export function usePausableInterval(
  callback: () => void,
  delayMs: number,
  activePath?: string,
  opts?: { runOnReactivate?: boolean },
) {
  const isActive = useIsActiveTab(activePath);
  const callbackRef = useRef(callback);
  // Mantener siempre la última versión del callback (evita stale closures)
  callbackRef.current = callback;

  // Track de si la tab estaba inactiva antes (para detectar reactivación)
  const prevActiveRef = useRef(isActive);

  useEffect(() => {
    if (!isActive) {
      prevActiveRef.current = false;
      return;
    }

    // Si pasamos de inactivo → activo y el flag está, ejecutar 1 vez ahora
    if (!prevActiveRef.current && opts?.runOnReactivate) {
      try { callbackRef.current(); } catch { /* ignore */ }
    }
    prevActiveRef.current = true;

    const id = setInterval(() => {
      callbackRef.current();
    }, delayMs);
    return () => clearInterval(id);
  }, [isActive, delayMs, opts?.runOnReactivate]);
}
