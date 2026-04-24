import { useState, useEffect, useMemo } from "react";
import { NavLink, Outlet, useLocation } from "react-router-dom";
import { useKeyboardShortcuts, SHORTCUTS_LIST } from "../hooks/useKeyboardShortcuts";
import { useSesion } from "../contexts/SesionContext";
import { useDemo } from "../contexts/DemoContext";
import SuscripcionBanner from "./SuscripcionBanner";
import UpdateChecker from "./UpdateChecker";
import logoClouget from "../assets/logo-clouget.png";
import { House, Storefront, Package, Users, Receipt, Truck, Money, Coins, Bank, ShoppingCart, ChartLineUp, Warehouse, Gear, CurrencyDollar, SignOut, Question, Moon, Sun, Wallet, Barcode, Calendar, Wrench } from "@phosphor-icons/react";
import type { Icon } from "@phosphor-icons/react";

interface NavItem { path: string; label: string; icon: Icon; shortcut: string; todos: boolean; permiso?: string; }

const navItems: NavItem[] = [
  { path: "/", label: "Inicio", icon: House, shortcut: "", todos: true },
  { path: "/pos", label: "Venta", icon: Storefront, shortcut: "F1", todos: true },
  { path: "/productos", label: "Productos", icon: Package, shortcut: "F2", todos: false, permiso: "gestionar_productos" },
  { path: "/clientes", label: "Clientes", icon: Users, shortcut: "F3", todos: false, permiso: "gestionar_clientes" },
  { path: "/ventas", label: "Ventas", icon: Receipt, shortcut: "F4", todos: true },
  { path: "/guias", label: "Guías", icon: Truck, shortcut: "", todos: false, permiso: "ver_guias" },
  { path: "/gastos", label: "Gastos", icon: Money, shortcut: "F7", todos: false, permiso: "gestionar_gastos" },
  { path: "/cuentas", label: "Cobrar", icon: Coins, shortcut: "F8", todos: true },
  { path: "/compras", label: "Compras", icon: ShoppingCart, shortcut: "", todos: false, permiso: "gestionar_compras" },
  { path: "/pagar", label: "Pagar", icon: Wallet, shortcut: "", todos: false, permiso: "gestionar_compras" },
  { path: "/movimientos-bancarios", label: "Bancos", icon: Bank, shortcut: "", todos: false, permiso: "ver_movimientos_bancarios" },
  { path: "/inventario", label: "Inventario", icon: Warehouse, shortcut: "", todos: false, permiso: "gestionar_inventario" },
  { path: "/series", label: "Series", icon: Barcode, shortcut: "", todos: false, permiso: "gestionar_inventario" },
  { path: "/caducidad", label: "Caducidad", icon: Calendar, shortcut: "", todos: false, permiso: "gestionar_inventario" },
  // Servicio Tecnico: visible si admin, gestionar o solo ver. El filtro luego acepta cualquiera de los dos.
  { path: "/servicio-tecnico", label: "Servicio", icon: Wrench, shortcut: "", todos: false, permiso: "gestionar_servicio_tecnico", permisoAlt: "ver_servicio_tecnico" } as any,
  { path: "/reportes", label: "Reportes", icon: ChartLineUp, shortcut: "", todos: false, permiso: "ver_reportes" },
];

const headerNavItems = [
  { path: "/caja", icon: CurrencyDollar, title: "Caja (F5)", label: "Caja", todos: true },
  { path: "/config", icon: Gear, title: "Configuración (F6)", label: "", todos: false },
];

export default function Layout() {
  const { sesion, cerrarSesion, esAdmin, tienePermiso } = useSesion();
  const { esDemo, salirDemo } = useDemo();
  useKeyboardShortcuts(sesion?.rol);
  const [mostrarAyuda, setMostrarAyuda] = useState(false);
  const [saliendoDemo, setSaliendoDemo] = useState(false);
  const [moduloSeriesActivo, setModuloSeriesActivo] = useState(false);
  const [moduloCaducidadActivo, setModuloCaducidadActivo] = useState(false);
  const [moduloServicioTecnicoActivo, setModuloServicioTecnicoActivo] = useState(false);
  const [tooltip, setTooltip] = useState<{ label: string; top: number } | null>(null);
  const location = useLocation();
  const enPOS = location.pathname === "/pos";

  // Cargar config de módulos
  useEffect(() => {
    import("../services/api").then(({ obtenerConfig }) => {
      obtenerConfig().then(cfg => {
        setModuloSeriesActivo(cfg.modulo_series_activo === "1");
        setModuloCaducidadActivo(cfg.modulo_caducidad === "1");
        setModuloServicioTecnicoActivo(cfg.modulo_servicio_tecnico === "1");
      }).catch(() => {});
    });
  }, []);

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
    return items;
  }, [esAdmin, tienePermiso, moduloSeriesActivo, moduloCaducidadActivo, moduloServicioTecnicoActivo]);

  const headerNavFiltrados = esAdmin
    ? headerNavItems
    : headerNavItems.filter((item) => item.todos);

  return (
    <div className="app-layout">
      {/* Top Header */}
      <header className="top-header">
        <NavLink to="/" className="top-header-logo" style={{ textDecoration: "none", color: "inherit", display: "flex", alignItems: "center", gap: 8 }}>
          <img src={logoClouget} alt="Clouget"
            style={{ height: 22, width: "auto", filter: "brightness(0) invert(1)", objectFit: "contain" }} />
          <span style={{ fontSize: 11, color: "rgba(255,255,255,0.5)", fontWeight: 400, letterSpacing: 0 }}>Punto de Venta</span>
        </NavLink>

        <div className="top-header-right">
          {/* Nav items del header: Caja, Config */}
          <div style={{ display: "flex", gap: 4, marginRight: 8, paddingRight: 12, borderRight: "1px solid rgba(255,255,255,0.1)" }}>
            {enPOS && (
              <NavLink to="/" style={{ padding: "4px 12px", borderRadius: 6, textDecoration: "none", color: "rgba(255,255,255,0.7)", display: "flex", alignItems: "center", gap: 4, border: "1px solid rgba(255,255,255,0.08)" }}
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

      {/* Sidebar Compacto (siempre visible) */}
      <nav className="sidebar-compact">
        {navFiltrados.map((item) => {
          const IconComp = item.icon;
          const labelCompleto = `${item.label}${item.shortcut ? ` (${item.shortcut})` : ""}`;
          return (
            <NavLink key={item.path} to={item.path} end={item.path === "/"}
              title={labelCompleto}
              onMouseEnter={(e) => {
                const r = e.currentTarget.getBoundingClientRect();
                setTooltip({ label: labelCompleto, top: r.top + r.height / 2 });
              }}
              onMouseLeave={() => setTooltip(null)}
              className={({ isActive }) => `nav-item ${isActive ? "active" : ""}`}>
              <IconComp size={22} weight="regular" />
            </NavLink>
          );
        })}
        <div className="nav-spacer" />
        <div className="nav-item"
          onClick={cerrarSesion}
          title="Cerrar Sesión"
          onMouseEnter={(e) => {
            const r = e.currentTarget.getBoundingClientRect();
            setTooltip({ label: "Cerrar Sesión", top: r.top + r.height / 2 });
          }}
          onMouseLeave={() => setTooltip(null)}
          style={{ cursor: "pointer" }}>
          <SignOut size={20} />
        </div>
      </nav>

      {/* Tooltip flotante (fuera del overflow del sidebar) */}
      {tooltip && (
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

      {/* Main Content */}
      <main className="main-content">
        <Outlet />
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
