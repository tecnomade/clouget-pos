/**
 * v2.5.0 — Sistema de pestañas internas (multi-vista en una ventana).
 *
 * Diseño:
 * - 1 tab por ruta (no se permiten duplicados)
 * - Si abrís un path que ya tiene tab → activa la existente
 * - Tab "Inicio" (/) está pineada (no se puede cerrar)
 * - Máximo 8 tabs abiertas (safety net contra memoria)
 * - Persiste en sessionStorage (sobrevive reload, no cierre de app)
 * - Toggle en Configuración para desactivar (fallback a navegación clásica)
 *
 * Las pestañas se mantienen montadas pero se ocultan con display:none cuando no
 * están activas → el state se preserva al cambiar de tab (carrito de POS,
 * filtros, scroll, formularios a medio llenar, etc.).
 */
import {
  createContext, useCallback, useContext, useEffect, useMemo, useRef, useState,
  type ReactNode,
} from "react";

export interface TabDef {
  /** path es el ID único — solo una tab por path. */
  path: string;
  title: string;
  icon: string; // emoji o caracter
  /** Si true, no se puede cerrar (ej. Inicio) */
  pinned?: boolean;
}

interface TabsContextValue {
  enabled: boolean;
  tabs: TabDef[];
  activeId: string | null;
  /** Abre nueva tab o activa la existente (si ya hay una con ese path). */
  openOrSwitch: (tab: TabDef) => void;
  /** Cierra tab. Si era la activa, activa la anterior. */
  close: (path: string) => void;
  /** Activa una tab existente (no crea). */
  setActive: (path: string) => void;
  /** Indica si el path dado es la tab actualmente activa. */
  isActive: (path: string) => boolean;
  /** v2.5.3: contador por tab que se incrementa cada vez que la tab se activa.
   *  Se usa con useTabActivated para refrescar data cuando el usuario vuelve a una tab. */
  activationVersion: Record<string, number>;
}

const TabsContext = createContext<TabsContextValue | null>(null);

const MAX_TABS = 8;
const STORAGE_KEY_BASE = "clouget-tabs-state-v1";

const TAB_INICIO: TabDef = { path: "/", title: "Inicio", icon: "🏠", pinned: true };

function storageKey(scope: string | number): string {
  return `${STORAGE_KEY_BASE}-${scope}`;
}

function loadFromStorage(scope: string | number): { tabs: TabDef[]; activeId: string | null } {
  try {
    const raw = sessionStorage.getItem(storageKey(scope));
    if (!raw) return { tabs: [TAB_INICIO], activeId: "/" };
    const parsed = JSON.parse(raw);
    if (!parsed.tabs || !Array.isArray(parsed.tabs) || parsed.tabs.length === 0) {
      return { tabs: [TAB_INICIO], activeId: "/" };
    }
    // Asegurar que Inicio siempre esté presente y pineada
    const hasInicio = parsed.tabs.some((t: TabDef) => t.path === "/");
    const tabs = hasInicio ? parsed.tabs : [TAB_INICIO, ...parsed.tabs];
    return {
      tabs: tabs.map((t: TabDef) => t.path === "/" ? { ...t, pinned: true } : t),
      activeId: parsed.activeId || "/",
    };
  } catch {
    return { tabs: [TAB_INICIO], activeId: "/" };
  }
}

function saveToStorage(scope: string | number, tabs: TabDef[], activeId: string | null) {
  try {
    sessionStorage.setItem(storageKey(scope), JSON.stringify({ tabs, activeId }));
  } catch { /* ignore */ }
}

interface TabsProviderProps {
  enabled: boolean;
  /** v2.5.0: scope opcional para que cada usuario tenga su propio set de tabs.
   *  Si se omite, usa scope "default". */
  scope?: string | number;
  children: ReactNode;
}

export function TabsProvider({ enabled, scope = "default", children }: TabsProviderProps) {
  const initial = useRef(loadFromStorage(scope));
  const [tabs, setTabs] = useState<TabDef[]>(initial.current.tabs);
  const [activeId, setActiveId] = useState<string | null>(initial.current.activeId);
  // v2.5.3: contador de activaciones por path. Cada vez que una tab se vuelve
  // activa (después de no serlo), su counter se incrementa. Páginas pueden
  // observar este counter via useTabActivated para refrescar data.
  const [activationVersion, setActivationVersion] = useState<Record<string, number>>(
    initial.current.activeId ? { [initial.current.activeId]: 1 } : {},
  );
  // Track previous activeId para detectar cambios de activación
  const prevActiveIdRef = useRef<string | null>(initial.current.activeId);

  useEffect(() => {
    // Si el activeId cambió, incrementar el counter del nuevo activo (no del anterior)
    if (activeId && activeId !== prevActiveIdRef.current) {
      setActivationVersion(prev => ({ ...prev, [activeId]: (prev[activeId] || 0) + 1 }));
      prevActiveIdRef.current = activeId;
    }
  }, [activeId]);

  useEffect(() => { saveToStorage(scope, tabs, activeId); }, [scope, tabs, activeId]);

  const openOrSwitch = useCallback((tab: TabDef) => {
    setTabs(prev => {
      const existing = prev.find(t => t.path === tab.path);
      if (existing) {
        // Ya existe → solo activar
        return prev;
      }
      if (prev.length >= MAX_TABS) {
        // Sustituir la última no pineada que no sea la activa
        const idxAReemplazar = prev
          .map((t, i) => ({ t, i }))
          .filter(({ t, i }) => !t.pinned && t.path !== activeId && i > 0)
          .map(({ i }) => i)
          .pop();
        if (idxAReemplazar !== undefined) {
          const next = [...prev];
          next[idxAReemplazar] = tab;
          return next;
        }
        // No hay ninguna que reemplazar — agregar al final igual (con warning)
        console.warn(`[Tabs] Excedido el máximo de ${MAX_TABS} tabs.`);
        return [...prev, tab];
      }
      return [...prev, tab];
    });
    setActiveId(tab.path);
  }, [activeId]);

  const close = useCallback((path: string) => {
    setTabs(prev => {
      const idx = prev.findIndex(t => t.path === path);
      if (idx === -1) return prev;
      if (prev[idx].pinned) return prev; // no se puede cerrar
      const next = prev.filter(t => t.path !== path);
      // Si era la activa, activar la anterior (o Inicio)
      if (path === activeId) {
        const nuevaActiva = next[Math.max(0, idx - 1)] || next[0] || null;
        setActiveId(nuevaActiva ? nuevaActiva.path : null);
      }
      return next;
    });
  }, [activeId]);

  const setActive = useCallback((path: string) => {
    setTabs(prev => {
      if (!prev.find(t => t.path === path)) return prev;
      setActiveId(path);
      return prev;
    });
  }, []);

  const isActive = useCallback((path: string) => activeId === path, [activeId]);

  const value = useMemo<TabsContextValue>(() => ({
    enabled, tabs, activeId, openOrSwitch, close, setActive, isActive, activationVersion,
  }), [enabled, tabs, activeId, openOrSwitch, close, setActive, isActive, activationVersion]);

  return <TabsContext.Provider value={value}>{children}</TabsContext.Provider>;
}

export function useTabs(): TabsContextValue {
  const ctx = useContext(TabsContext);
  if (!ctx) {
    // Modo de compatibilidad: si no hay TabsProvider (toggle off), devolvemos
    // un stub que actúa como si tabs estuvieran deshabilitados.
    return {
      enabled: false,
      tabs: [],
      activeId: null,
      openOrSwitch: () => {},
      close: () => {},
      setActive: () => {},
      isActive: () => true, // siempre "activa" en modo no-tabs (single page)
      activationVersion: {},
    };
  }
  return ctx;
}

/**
 * v2.5.3 — Hook que ejecuta el callback cada vez que la tab del path dado
 * pasa a estar ACTIVA (incluyendo el primer mount).
 *
 * Útil para refrescar datos que pudieron cambiar mientras la tab estuvo oculta:
 * por ejemplo, refrescar la lista de productos en POS si el usuario editó
 * un producto en otra tab.
 *
 * @param myPath path de esta tab (ej. "/pos")
 * @param callback función a ejecutar cuando la tab se activa
 *
 * @example
 *   useTabActivated("/pos", () => {
 *     listarProductosTactil().then(setProductosTactil);
 *   });
 *
 * Si tabs están deshabilitadas (modo clásico), el callback NO se ejecuta
 * automáticamente — el flujo single-page ya hace el remount tradicional.
 */
export function useTabActivated(myPath: string, callback: () => void) {
  const { enabled, activationVersion } = useTabs();
  const v = activationVersion[myPath] || 0;
  const cbRef = useRef(callback);
  cbRef.current = callback;
  useEffect(() => {
    if (!enabled) return; // sin tabs, el remount clásico ya carga data
    if (v === 0) return;  // primer mount sin activación todavía — el page hará su carga normal en su useEffect inicial
    cbRef.current();
  }, [enabled, v]);
}

/** Hook para que las páginas sepan si su tab está activa (ej. para pausar polling). */
export function useIsActiveTab(myPath?: string): boolean {
  const { enabled, isActive, activeId } = useTabs();
  if (!enabled) return true; // sin tabs → siempre activa
  if (!myPath) {
    // Si no se pasa path, asumimos que el componente se monta dentro de la tab actual
    // (esto siempre será true porque el render es eager, pero state preservation requiere
    // que la página REACCIONE a cambios de active aunque siga montada)
    return activeId !== null;
  }
  return isActive(myPath);
}
