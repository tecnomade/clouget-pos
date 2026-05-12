/**
 * v2.5.0 — Container que renderiza TODAS las tabs abiertas, con solo la activa
 * visible (las demás con display:none). Esto preserva el state de cada página
 * (carrito, formularios, scroll, filtros, etc.) al cambiar de tab.
 *
 * Sincronización con URL:
 * - Cuando cambia activeId, navega al path de esa tab (URL sync)
 * - Cuando cambia URL externamente (deep link, browser back), busca tab con
 *   ese path; si no existe, la abre nueva.
 */
import { useEffect, useRef } from "react";
import { useLocation, useNavigate } from "react-router-dom";
import { useTabs, type TabDef } from "../contexts/TabsContext";
import PageRenderer from "./PageRenderer";

interface Props {
  /** Función que determina si el path es accesible (permisos). */
  canAccessPath: (path: string) => boolean;
  /** Resuelve título + icono para un path. */
  resolveMetadata: (path: string) => Omit<TabDef, "path"> | null;
}

export default function TabsContainer({ canAccessPath, resolveMetadata }: Props) {
  const { enabled, tabs, activeId, openOrSwitch, setActive } = useTabs();
  const location = useLocation();
  const navigate = useNavigate();
  const lastUrlSync = useRef<string | null>(null);

  // === Sincronizar URL → Tabs ===
  // Cuando cambia la URL (deep link, browser back/forward, navegación inicial)
  useEffect(() => {
    if (!enabled) return;
    const path = location.pathname;
    if (path === lastUrlSync.current) return; // ya procesado
    lastUrlSync.current = path;

    // Verificar permiso
    if (!canAccessPath(path)) {
      navigate("/", { replace: true });
      return;
    }

    // Si ya hay una tab con este path, activarla
    const existing = tabs.find(t => t.path === path);
    if (existing) {
      if (activeId !== path) setActive(path);
      return;
    }
    // No existe → abrirla
    const meta = resolveMetadata(path);
    if (meta) {
      openOrSwitch({ path, ...meta });
    } else {
      // Path desconocido → ir a Inicio
      navigate("/", { replace: true });
    }
  }, [location.pathname, enabled]); // eslint-disable-line react-hooks/exhaustive-deps

  // === Sincronizar activeId → URL ===
  useEffect(() => {
    if (!enabled || !activeId) return;
    if (activeId === location.pathname) return;
    if (activeId === lastUrlSync.current) return;
    lastUrlSync.current = activeId;
    navigate(activeId, { replace: true });
  }, [activeId, enabled]); // eslint-disable-line react-hooks/exhaustive-deps

  // Si tabs deshabilitadas → modo clásico (renderiza solo la página de la URL actual)
  if (!enabled) {
    return (
      <div style={{ flex: 1, minHeight: 0, overflow: "auto" }}>
        <PageRenderer path={location.pathname} />
      </div>
    );
  }

  return (
    <div style={{ flex: 1, minHeight: 0, position: "relative" }}>
      {tabs.map((tab) => {
        const isActive = tab.path === activeId;
        return (
          <div
            key={tab.path}
            style={{
              display: isActive ? "block" : "none",
              height: "100%",
              overflow: "auto",
            }}
            // Marca el panel para que useIsActiveTab pueda detectar visibilidad
            data-tab-path={tab.path}
            data-tab-active={isActive ? "true" : "false"}
          >
            <PageRenderer path={tab.path} />
          </div>
        );
      })}
    </div>
  );
}
