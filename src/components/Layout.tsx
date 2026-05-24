import { useState, useEffect, useMemo, type ReactNode } from "react";
import { NavLink, useLocation, useNavigate } from "react-router-dom";
import { useKeyboardShortcuts, SHORTCUTS_LIST } from "../hooks/useKeyboardShortcuts";
import { useSesion } from "../contexts/SesionContext";
import { useDemo } from "../contexts/DemoContext";
import { useTabs } from "../contexts/TabsContext";
import { getTabMetadata } from "../config/tabsRegistry";
import TabBar from "./TabBar";
import SuscripcionBanner from "./SuscripcionBanner";
import UpdateChecker from "./UpdateChecker";
import { FEATURES } from "../config/branding";
import { House, Storefront, Package, Users, Receipt, Truck, Money, Coins, Bank, ShoppingCart, ChartLineUp, Warehouse, Gear, CurrencyDollar, SignOut, Question, Moon, Sun, Wallet, Barcode, Calendar, Wrench, ForkKnife, CookingPot, CaretLeft, CaretRight } from "@phosphor-icons/react";
import type { Icon } from "@phosphor-icons/react";

/** Grupos del sidebar — organizan los items en secciones lógicas.
 *  El orden aquí define el orden visual de los grupos. */
type GroupKey = "principal" | "ventas" | "gestion" | "compras" | "operaciones" | "restaurante" | "tributario" | "analitica";
const GROUPS: Record<GroupKey, { label: string; orden: number }> = {
  principal:   { label: "PRINCIPAL",    orden: 0 },
  ventas:      { label: "VENTAS",       orden: 1 },
  gestion:     { label: "GESTIÓN",      orden: 2 },
  compras:     { label: "COMPRAS",      orden: 3 },
  operaciones: { label: "OPERACIONES",  orden: 4 },
  restaurante: { label: "RESTAURANTE",  orden: 5 },
  // v2.5.43: sección TRIBUTARIO solo visible si licencia tiene módulo sri_avanzado.
  // Encierra Agente de Retención + futura Declaración IVA, ATS, etc.
  tributario:  { label: "TRIBUTARIO",   orden: 6 },
  analitica:   { label: "ANALÍTICA",    orden: 7 },
};

interface NavItem {
  path: string;
  label: string;
  icon: Icon;
  shortcut: string;
  todos: boolean;
  permiso?: string;
  permisoAlt?: string;
  group: GroupKey;
}

const navItems: NavItem[] = [
  { path: "/", label: "Inicio", icon: House, shortcut: "", todos: true, group: "principal" },
  // VENTAS
  { path: "/pos", label: "Venta", icon: Storefront, shortcut: "F1", todos: true, group: "ventas" },
  { path: "/ventas", label: "Ventas", icon: Receipt, shortcut: "F4", todos: true, group: "ventas" },
  { path: "/cuentas", label: "Cobrar", icon: Coins, shortcut: "F8", todos: true, group: "ventas" },
  { path: "/guias", label: "Guías", icon: Truck, shortcut: "", todos: false, permiso: "ver_guias", group: "ventas" },
  // GESTIÓN (productos, clientes, stock)
  { path: "/productos", label: "Productos", icon: Package, shortcut: "F2", todos: false, permiso: "gestionar_productos", group: "gestion" },
  { path: "/clientes", label: "Clientes", icon: Users, shortcut: "F3", todos: false, permiso: "gestionar_clientes", group: "gestion" },
  { path: "/inventario", label: "Inventario", icon: Warehouse, shortcut: "", todos: false, permiso: "gestionar_inventario", group: "gestion" },
  { path: "/series", label: "Series", icon: Barcode, shortcut: "", todos: false, permiso: "gestionar_inventario", group: "gestion" },
  { path: "/caducidad", label: "Caducidad", icon: Calendar, shortcut: "", todos: false, permiso: "gestionar_inventario", group: "gestion" },
  // COMPRAS
  { path: "/compras", label: "Compras", icon: ShoppingCart, shortcut: "", todos: false, permiso: "gestionar_compras", group: "compras" },
  { path: "/pagar", label: "Pagar", icon: Wallet, shortcut: "", todos: false, permiso: "gestionar_compras", group: "compras" },
  { path: "/movimientos-bancarios", label: "Bancos", icon: Bank, shortcut: "", todos: false, permiso: "ver_movimientos_bancarios", group: "compras" },
  // OPERACIONES (gastos, servicio técnico)
  { path: "/gastos", label: "Gastos", icon: Money, shortcut: "F7", todos: false, permiso: "gestionar_gastos", group: "operaciones" },
  { path: "/servicio-tecnico", label: "Servicio", icon: Wrench, shortcut: "", todos: false, permiso: "gestionar_servicio_tecnico", permisoAlt: "ver_servicio_tecnico", group: "operaciones" },
  // RESTAURANTE (filtrado por modulo)
  { path: "/mesas", label: "Mesas", icon: ForkKnife, shortcut: "", todos: true, group: "restaurante" },
  { path: "/cocina", label: "Cocina", icon: CookingPot, shortcut: "", todos: true, group: "restaurante" },
  // TRIBUTARIO (v2.5.43, filtrado por modulo sri_avanzado en licencia)
  { path: "/sri-avanzado", label: "Agente Retención", icon: Receipt, shortcut: "", todos: false, permiso: "gestionar_compras", group: "tributario" },
  // ANALÍTICA
  { path: "/reportes", label: "Reportes", icon: ChartLineUp, shortcut: "", todos: false, permiso: "ver_reportes", group: "analitica" },
];

const headerNavItems = [
  { path: "/caja", icon: CurrencyDollar, title: "Caja (F5)", label: "Caja", todos: true },
  { path: "/config", icon: Gear, title: "Configuración (F6)", label: "", todos: false },
];

export default function Layout({ children }: { children?: ReactNode }) {
  const { sesion, cerrarSesion, esAdmin, tienePermiso } = useSesion();
  const { esDemo, salirDemo } = useDemo();
  const { enabled: tabsEnabled, openOrSwitch } = useTabs();
  const navigate = useNavigate();
  useKeyboardShortcuts(sesion?.rol);

  /** v2.5.0: handler que reemplaza la navegación normal con openOrSwitch.
   *  Si tabs deshabilitadas, hace navigate normal (sin abrir tab). */
  const handleNavClick = (e: React.MouseEvent, path: string) => {
    if (!tabsEnabled) return; // dejar que NavLink navegue normalmente
    e.preventDefault();
    const meta = getTabMetadata(path);
    if (meta) {
      openOrSwitch({ path, ...meta });
    } else {
      navigate(path);
    }
  };
  const [mostrarAyuda, setMostrarAyuda] = useState(false);
  const [saliendoDemo, setSaliendoDemo] = useState(false);
  const [moduloSeriesActivo, setModuloSeriesActivo] = useState(false);
  const [moduloCaducidadActivo, setModuloCaducidadActivo] = useState(false);
  const [moduloServicioTecnicoActivo, setModuloServicioTecnicoActivo] = useState(false);
  const [moduloRestauranteActivo, setModuloRestauranteActivo] = useState(false);
  // v2.5.43: módulo SRI Avanzado (agente de retención + ATS) - solo si licencia lo incluye
  const [moduloSriAvanzadoActivo, setModuloSriAvanzadoActivo] = useState(false);
  const [nombreNegocio, setNombreNegocio] = useState<string>("");
  const [tooltip, setTooltip] = useState<{ label: string; top: number } | null>(null);
  // Sidebar expandido (mostrar labels + group headers). Persistente.
  const [sidebarExpandido, setSidebarExpandido] = useState<boolean>(
    () => localStorage.getItem("clouget-sidebar-expandido") === "1",
  );
  // v2.5.25: sidebar colapsado un poco mas ancho (60→64) para iconos size=24 mas comodos
  const sidebarWidth = sidebarExpandido ? 210 : 64;
  useEffect(() => {
    localStorage.setItem("clouget-sidebar-expandido", sidebarExpandido ? "1" : "0");
    // Setear CSS variable para que .main-content ajuste su margin-left
    document.documentElement.style.setProperty("--sidebar-width", `${sidebarWidth}px`);
  }, [sidebarExpandido, sidebarWidth]);
  const location = useLocation();
  const enPOS = location.pathname === "/pos";

  // Cargar config de módulos + nombre negocio (para header limpio sin logo redundante)
  useEffect(() => {
    import("../services/api").then(({ obtenerConfig }) => {
      obtenerConfig().then(cfg => {
        setModuloSeriesActivo(cfg.modulo_series_activo === "1");
        setModuloCaducidadActivo(cfg.modulo_caducidad === "1");
        setNombreNegocio((cfg.nombre_negocio || "").trim());
        // v2.4.8 / v2.4.17: Servicio Técnico es módulo de licencia. La licencia es la
        // FUENTE DE VERDAD — si admin la desactiva desde el panel admin, el módulo
        // desaparece del POS sin importar el flag local. El flag legacy
        // `modulo_servicio_tecnico` solo se usa como fallback si NO hay licencia
        // cargada todavía (instalación antes de v2.4.8).
        const licStr = (cfg.licencia_modulos || "").trim();
        const tieneLicenciaCargada = licStr !== "" && licStr !== "[]";
        try {
          const mods: string[] = tieneLicenciaCargada ? JSON.parse(licStr) : [];
          setModuloRestauranteActivo(FEATURES.restaurante && mods.includes("restaurante"));
          if (tieneLicenciaCargada) {
            // Fuente de verdad: licencia
            setModuloServicioTecnicoActivo(mods.includes("servicio_tecnico"));
            // v2.5.43: sri_avanzado SOLO si está explicitamente en la licencia
            setModuloSriAvanzadoActivo(mods.includes("sri_avanzado"));
          } else {
            // Sin licencia cargada: caer al flag legacy (instalación pre-v2.4.8)
            setModuloServicioTecnicoActivo(cfg.modulo_servicio_tecnico === "1");
            // sri_avanzado nunca cae a legacy — es modulo nuevo, requiere licencia
            setModuloSriAvanzadoActivo(false);
          }
        } catch {
          setModuloRestauranteActivo(false);
          setModuloServicioTecnicoActivo(false);
          setModuloSriAvanzadoActivo(false);
        }
      }).catch(() => {});
    });
  }, []);

  // Mapear ruta actual → nombre de página para mostrar en el header (estilo Notion/Linear).
  // Usa navItems.label cuando existe, agrega rutas que no están en sidebar.
  const tituloPagina = useMemo(() => {
    const titulosExtra: Record<string, string> = {
      "/caja": "Caja",
      "/config": "Configuración",
      "/config-mesas": "Configuración de Mesas",
    };
    if (titulosExtra[location.pathname]) return titulosExtra[location.pathname];
    const item = navItems.find(i => i.path === location.pathname);
    if (item) return item.label;
    // Fallback: capitalize última parte de la ruta
    const last = location.pathname.split("/").filter(Boolean).pop() || "";
    return last ? last.charAt(0).toUpperCase() + last.slice(1) : "Inicio";
  }, [location.pathname]);

  // Tema claro/oscuro
  const [tema, setTema] = useState(() => localStorage.getItem("clouget-theme") || "light");
  useEffect(() => {
    document.documentElement.setAttribute("data-theme", tema);
    localStorage.setItem("clouget-theme", tema);
  }, [tema]);
  const toggleTema = () => setTema(t => t === "dark" ? "light" : "dark");

  const navFiltrados = useMemo(() => {
    let items = esAdmin
      ? navItems
      : navItems.filter((item) => {
          if (item.todos) return true;
          // Acepta permiso principal o alterno (permisoAlt) — para casos como
          // /servicio-tecnico que se ve con gestionar_servicio_tecnico O ver_servicio_tecnico
          if (item.permiso && tienePermiso(item.permiso)) return true;
          const permisoAlt = (item as any).permisoAlt;
          if (permisoAlt && tienePermiso(permisoAlt)) return true;
          return false;
        });
    // Ocultar Series si módulo no está activo
    if (!moduloSeriesActivo) items = items.filter(i => i.path !== "/series");
    if (!moduloCaducidadActivo) items = items.filter(i => i.path !== "/caducidad");
    if (!moduloServicioTecnicoActivo) items = items.filter(i => i.path !== "/servicio-tecnico");
    if (!moduloSriAvanzadoActivo) items = items.filter(i => i.path !== "/sri-avanzado");
    // Mesas/Cocina: solo si modulo Restaurante activo (build Clouget + licencia con 'restaurante')
    if (!moduloRestauranteActivo) items = items.filter(i => i.path !== "/mesas" && i.path !== "/cocina");
    return items;
  }, [esAdmin, tienePermiso, moduloSeriesActivo, moduloCaducidadActivo, moduloServicioTecnicoActivo, moduloRestauranteActivo, moduloSriAvanzadoActivo]);

  const headerNavFiltrados = esAdmin
    ? headerNavItems
    : headerNavItems.filter((item) => item.todos);

  return (
    <div className="app-layout">
      {/* Top Header — estilo Notion/Linear: NO duplicamos el logo (ya está en barra
          de Windows). Mostramos nombre del negocio + página actual como breadcrumb.
          Esto da contexto útil al usuario en lugar de branding redundante. */}
      <header className="top-header">
        <NavLink
          to="/"
          onClick={(e) => handleNavClick(e, "/")}
          className="top-header-logo"
          style={{
            textDecoration: "none",
            color: "inherit",
            display: "flex",
            alignItems: "center",
            gap: 8,
            minWidth: 0,
            overflow: "hidden",
            paddingLeft: 4, // v2.5.23: alinea bien al borde izquierdo
          }}
          title="Ir a Inicio"
        >
          {/* v2.5.23: logo Clouget eliminado del header — ya está visible en la barra
              de Windows. Quedó solo el nombre del negocio del usuario, alineado a la izquierda. */}
          {nombreNegocio && (
            <span
              style={{
                fontSize: 15,
                color: "rgba(255,255,255,0.95)",
                fontWeight: 700,
                letterSpacing: 0.2,
                whiteSpace: "nowrap",
                overflow: "hidden",
                textOverflow: "ellipsis",
                maxWidth: 320,
              }}
            >
              {nombreNegocio}
            </span>
          )}
          <span style={{ fontSize: 12, color: "rgba(255,255,255,0.3)", fontWeight: 400 }}>·</span>
          <span
            style={{
              fontSize: 13,
              color: "rgba(255,255,255,0.6)",
              fontWeight: 500,
              whiteSpace: "nowrap",
              overflow: "hidden",
              textOverflow: "ellipsis",
            }}
          >
            {tituloPagina}
          </span>
        </NavLink>

        <div className="top-header-right">
          {/* Nav items del header: Caja, Config */}
          <div style={{ display: "flex", gap: 4, marginRight: 8, paddingRight: 12, borderRight: "1px solid rgba(255,255,255,0.1)" }}>
            {enPOS && (
              <NavLink to="/" onClick={(e) => handleNavClick(e, "/")} style={{ padding: "4px 12px", borderRadius: 6, textDecoration: "none", color: "rgba(255,255,255,0.7)", display: "flex", alignItems: "center", gap: 4, border: "1px solid rgba(255,255,255,0.08)" }}
                title="Inicio">
                <House size={18} />
              </NavLink>
            )}
            {headerNavFiltrados.map((item) => {
              const HIcon = item.icon;
              return (
                <NavLink
                  key={item.path}
                  to={item.path}
                  onClick={(e) => handleNavClick(e, item.path)}
                  className={({ isActive }) => (isActive ? "active" : "")}
                  style={({ isActive }) => ({
                    padding: "4px 12px", borderRadius: 6, fontSize: 12, fontWeight: 600,
                    textDecoration: "none", transition: "all 0.15s", display: "flex", alignItems: "center",
                    background: isActive ? "rgba(96, 165, 250, 0.15)" : "transparent",
                    color: isActive ? "var(--color-primary)" : "rgba(255,255,255,0.5)",
                    border: isActive ? "none" : "1px solid rgba(255,255,255,0.08)",
                  })}
                  title={item.title}
                >
                  <HIcon size={18} />
                  {item.label && <span style={{ marginLeft: 4, fontSize: 11 }}>{item.label}</span>}
                </NavLink>
              );
            })}
          </div>
          {sesion && (
            <>
              <span style={{ fontSize: 13, color: "rgba(255,255,255,0.9)", fontWeight: 500 }}>
                {sesion.nombre}
              </span>
              <span style={{
                fontSize: 10, padding: "2px 8px", borderRadius: 4,
                background: sesion.rol === "ADMIN" ? "#3b82f6" : "rgba(255,255,255,0.15)",
                color: "white", fontWeight: 600,
              }}>
                {sesion.rol}
              </span>
            </>
          )}
          <button
            onClick={toggleTema}
            title={tema === "dark" ? "Cambiar a tema claro" : "Cambiar a tema oscuro"}
            style={{
              background: "rgba(255,255,255,0.06)", border: "1px solid rgba(255,255,255,0.15)",
              borderRadius: 6, cursor: "pointer", color: "rgba(255,255,255,0.6)",
              fontSize: 14, padding: "4px 8px", lineHeight: 1,
            }}
          >
            {tema === "dark" ? <Sun size={16} /> : <Moon size={16} />}
          </button>
          <button
            onClick={() => setMostrarAyuda(!mostrarAyuda)}
            style={{
              background: "rgba(255,255,255,0.06)", border: "1px solid rgba(255,255,255,0.15)",
              borderRadius: 6, cursor: "pointer", color: "rgba(255,255,255,0.6)",
              fontSize: 13, padding: "4px 10px", fontWeight: 600,
            }}
          >
            <Question size={16} />
          </button>
          {!enPOS && (
            <button
              onClick={cerrarSesion}
              style={{
                background: "none", border: "none", cursor: "pointer",
                color: "rgba(255,255,255,0.5)", fontSize: 12, display: "flex", alignItems: "center", gap: 4,
              }}
            >
              <SignOut size={14} /> Salir
            </button>
          )}
          <span style={{ fontSize: 9, opacity: 0.35, color: "white" }}>v{__APP_VERSION__}</span>
        </div>
      </header>

      {/* Banners */}
      <UpdateChecker />
      <SuscripcionBanner />
      {esDemo && (
        <div
          style={{
            display: "flex", alignItems: "center", justifyContent: "space-between",
            padding: "6px 16px",
            background: "rgba(245, 158, 11, 0.15)",
            borderBottom: "1px solid rgba(245, 158, 11, 0.3)",
            fontSize: 12, color: "var(--color-warning)",
          }}
        >
          <span style={{ fontWeight: 600 }}>MODO DEMO</span>
          <button
            onClick={async () => {
              setSaliendoDemo(true);
              try { await salirDemo(); } catch { setSaliendoDemo(false); }
            }}
            disabled={saliendoDemo}
            style={{
              padding: "2px 10px", background: "rgba(245, 158, 11, 0.2)",
              border: "1px solid rgba(245, 158, 11, 0.4)", borderRadius: 4,
              color: "var(--color-warning)", fontSize: 11, fontWeight: 600,
              cursor: saliendoDemo ? "not-allowed" : "pointer",
            }}
          >
            {saliendoDemo ? "..." : "Salir Demo"}
          </button>
        </div>
      )}

      {/* Sidebar agrupado — colapsado (íconos) o expandido (íconos + labels + group headers).
          Estado persistente en localStorage. Atajos F1-F10 funcionan independiente del estado. */}
      <nav
        className={`sidebar-compact ${sidebarExpandido ? "sidebar-expandido" : ""}`}
        style={{
          width: sidebarWidth,
          transition: "width 0.18s ease",
          // SIEMPRE permitir scroll vertical para items que no caben en pantalla.
          // overflowX:hidden evita que tooltips/indicadores se escapen lateralmente.
          overflowY: "auto",
          overflowX: "hidden",
        }}
      >
        {/* Botón toggle expandir/colapsar */}
        <button
          onClick={() => setSidebarExpandido(v => !v)}
          title={sidebarExpandido ? "Colapsar menú" : "Expandir menú"}
          style={{
            background: "transparent",
            border: "none",
            cursor: "pointer",
            color: "var(--color-text-muted, rgba(255,255,255,0.4))",
            padding: sidebarExpandido ? "8px 14px" : "8px 0",
            display: "flex",
            alignItems: "center",
            justifyContent: sidebarExpandido ? "flex-end" : "center",
            width: "100%",
            fontSize: 11,
            gap: 6,
          }}
        >
          {sidebarExpandido ? <CaretLeft size={14} /> : <CaretRight size={14} />}
        </button>

        {/* Items agrupados */}
        {(Object.keys(GROUPS) as GroupKey[])
          .sort((a, b) => GROUPS[a].orden - GROUPS[b].orden)
          .map((gk) => {
            const itemsGrupo = navFiltrados.filter(i => i.group === gk);
            if (itemsGrupo.length === 0) return null;
            return (
              <div key={gk} style={{ display: "flex", flexDirection: "column", marginBottom: 4 }}>
                {/* Header del grupo (solo en modo expandido O separador en modo colapsado) */}
                {sidebarExpandido ? (
                  <div
                    style={{
                      padding: "6px 14px 2px",
                      fontSize: 9,
                      fontWeight: 700,
                      letterSpacing: 1,
                      color: "var(--color-text-muted, rgba(255,255,255,0.35))",
                      textTransform: "uppercase",
                    }}
                  >
                    {GROUPS[gk].label}
                  </div>
                ) : (
                  // Modo colapsado: línea sutil entre grupos para dar agrupación visual
                  gk !== "principal" && (
                    <div
                      style={{
                        height: 1,
                        background: "rgba(255,255,255,0.08)",
                        margin: "6px 14px 4px",
                      }}
                    />
                  )
                )}
                {/* Items del grupo */}
                {itemsGrupo.map((item) => {
                  const IconComp = item.icon;
                  const labelCompleto = `${item.label}${item.shortcut ? ` (${item.shortcut})` : ""}`;
                  return (
                    <NavLink
                      key={item.path}
                      to={item.path}
                      end={item.path === "/"}
                      onClick={(e) => handleNavClick(e, item.path)}
                      title={labelCompleto}
                      onMouseEnter={(e) => {
                        if (sidebarExpandido) return; // sin tooltip si ya hay label visible
                        const r = e.currentTarget.getBoundingClientRect();
                        setTooltip({ label: labelCompleto, top: r.top + r.height / 2 });
                      }}
                      onMouseLeave={() => setTooltip(null)}
                      className={({ isActive }) => `nav-item ${isActive ? "active" : ""}`}
                      style={{
                        display: "flex",
                        alignItems: "center",
                        gap: sidebarExpandido ? 12 : 0,
                        justifyContent: sidebarExpandido ? "flex-start" : "center",
                        padding: sidebarExpandido ? "8px 14px" : undefined,
                      }}
                    >
                      <IconComp size={24} weight="regular" />
                      {sidebarExpandido && (
                        <span
                          style={{
                            fontSize: 13,
                            fontWeight: 500,
                            whiteSpace: "nowrap",
                            overflow: "hidden",
                            textOverflow: "ellipsis",
                            flex: 1,
                          }}
                        >
                          {item.label}
                        </span>
                      )}
                      {sidebarExpandido && item.shortcut && (
                        <span className="kbd" style={{ marginLeft: 0 }}>
                          {item.shortcut}
                        </span>
                      )}
                    </NavLink>
                  );
                })}
              </div>
            );
          })}

        <div className="nav-spacer" />

        {/* Cerrar sesión - siempre al final */}
        <div
          className="nav-item"
          onClick={cerrarSesion}
          title="Cerrar Sesión"
          onMouseEnter={(e) => {
            if (sidebarExpandido) return;
            const r = e.currentTarget.getBoundingClientRect();
            setTooltip({ label: "Cerrar Sesión", top: r.top + r.height / 2 });
          }}
          onMouseLeave={() => setTooltip(null)}
          style={{
            cursor: "pointer",
            display: "flex",
            alignItems: "center",
            gap: sidebarExpandido ? 12 : 0,
            justifyContent: sidebarExpandido ? "flex-start" : "center",
            padding: sidebarExpandido ? "8px 14px" : undefined,
          }}
        >
          <SignOut size={22} />
          {sidebarExpandido && (
            <span style={{ fontSize: 13, fontWeight: 500 }}>Cerrar sesión</span>
          )}
        </div>
      </nav>

      {/* Tooltip flotante (solo en modo colapsado, fuera del overflow del sidebar) */}
      {tooltip && !sidebarExpandido && (
        <div
          style={{
            position: "fixed",
            left: 60,
            top: tooltip.top,
            transform: "translateY(-50%)",
            background: "#1e293b",
            color: "#fff",
            padding: "6px 14px",
            borderRadius: 6,
            fontSize: 13,
            fontWeight: 600,
            whiteSpace: "nowrap",
            zIndex: 9999,
            boxShadow: "0 4px 12px rgba(0,0,0,0.4)",
            border: "1px solid rgba(255,255,255,0.1)",
            pointerEvents: "none",
          }}
        >
          <span
            style={{
              position: "absolute",
              left: -5,
              top: "50%",
              transform: "translateY(-50%)",
              width: 0,
              height: 0,
              borderTop: "5px solid transparent",
              borderBottom: "5px solid transparent",
              borderRight: "5px solid #1e293b",
            }}
          />
          {tooltip.label}
        </div>
      )}

      {/* Main Content — v2.5.0: TabBar arriba (solo si hay >1 tab abierta) + children
          (que ahora es TabsContainer con TODAS las tabs abiertas, una visible). */}
      <main className="main-content" style={{ display: "flex", flexDirection: "column" }}>
        <TabBar />
        <div style={{ flex: 1, minHeight: 0, display: "flex", flexDirection: "column" }}>
          {children}
        </div>
      </main>

      {/* Modal de atajos de teclado */}
      {mostrarAyuda && (
        <div
          style={{
            position: "fixed", inset: 0, background: "rgba(0,0,0,0.5)",
            display: "flex", alignItems: "center", justifyContent: "center", zIndex: 100,
          }}
          onClick={() => setMostrarAyuda(false)}
        >
          <div
            className="card"
            style={{ width: 400, maxHeight: "80vh", overflow: "auto" }}
            onClick={(e) => e.stopPropagation()}
          >
            <div className="card-header flex justify-between items-center">
              <span>Atajos de Teclado</span>
              <button className="btn btn-outline" style={{ padding: "2px 8px" }} onClick={() => setMostrarAyuda(false)}>
                x
              </button>
            </div>
            <div className="card-body">
              {SHORTCUTS_LIST.map((s) => (
                <div
                  key={s.keys}
                  className="flex justify-between items-center"
                  style={{ padding: "8px 0", borderBottom: "1px solid var(--color-border)" }}
                >
                  <span>{s.description}</span>
                  <kbd
                    style={{
                      background: "var(--color-surface-hover)",
                      border: "1px solid var(--color-border)",
                      borderRadius: 4, padding: "2px 8px",
                      fontSize: 12, fontFamily: "monospace",
                    }}
                  >
                    {s.keys}
                  </kbd>
                </div>
              ))}
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
