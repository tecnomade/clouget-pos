import { useState, useEffect } from "react";
import { NavLink, Outlet, useLocation } from "react-router-dom";
import { useKeyboardShortcuts, SHORTCUTS_LIST } from "../hooks/useKeyboardShortcuts";
import { useSesion } from "../contexts/SesionContext";
import { useDemo } from "../contexts/DemoContext";
import SuscripcionBanner from "./SuscripcionBanner";
import UpdateChecker from "./UpdateChecker";

// Items principales en bottom bar
const navItems = [
  { path: "/", label: "Inicio", icon: "🏠", shortcut: "", todos: true },
  { path: "/pos", label: "Venta", icon: "💰", shortcut: "F1", todos: true },
  { path: "/productos", label: "Productos", icon: "📦", shortcut: "F2", todos: false },
  { path: "/clientes", label: "Clientes", icon: "👥", shortcut: "F3", todos: false },
  { path: "/ventas", label: "Ventas", icon: "📋", shortcut: "F4", todos: true },
  { path: "/guias", label: "Guías", icon: "🚚", shortcut: "", todos: false },
  { path: "/gastos", label: "Gastos", icon: "💸", shortcut: "F7", todos: false },
  { path: "/cuentas", label: "Cobrar", icon: "💵", shortcut: "F8", todos: true },
  { path: "/inventario", label: "Inventario", icon: "📑", shortcut: "", todos: false },
  { path: "/reportes", label: "Reportes", icon: "📊", shortcut: "", todos: false },
];

// Items en el header superior (iconos)
const headerNavItems = [
  { path: "/caja", label: "$", title: "Caja (F5)", shortcut: "F5", todos: true },
  { path: "/config", label: "⚙", title: "Configuración (F6)", shortcut: "F6", todos: false },
];

export default function Layout() {
  const { sesion, cerrarSesion, esAdmin } = useSesion();
  const { esDemo, salirDemo } = useDemo();
  useKeyboardShortcuts(sesion?.rol);
  const [mostrarAyuda, setMostrarAyuda] = useState(false);
  const [saliendoDemo, setSaliendoDemo] = useState(false);
  const location = useLocation();
  const enPOS = location.pathname === "/pos";

  // Tema claro/oscuro
  const [tema, setTema] = useState(() => localStorage.getItem("clouget-theme") || "light");
  useEffect(() => {
    document.documentElement.setAttribute("data-theme", tema);
    localStorage.setItem("clouget-theme", tema);
  }, [tema]);
  const toggleTema = () => setTema(t => t === "dark" ? "light" : "dark");

  const navFiltrados = esAdmin
    ? navItems
    : navItems.filter((item) => item.todos);

  const headerNavFiltrados = esAdmin
    ? headerNavItems
    : headerNavItems.filter((item) => item.todos);

  return (
    <div className="app-layout">
      {/* Top Header */}
      <header className="top-header">
        <div className="top-header-logo">
          CLOUGET<span>POS</span>
        </div>

        <div className="top-header-right">
          {/* Nav items del header: Caja, Config */}
          <div style={{ display: "flex", gap: 4, marginRight: 8, paddingRight: 12, borderRight: "1px solid rgba(255,255,255,0.1)" }}>
            {headerNavFiltrados.map((item) => (
              <NavLink
                key={item.path}
                to={item.path}
                className={({ isActive }) => (isActive ? "active" : "")}
                style={({ isActive }) => ({
                  padding: "4px 12px", borderRadius: 6, fontSize: 12, fontWeight: 600,
                  textDecoration: "none", transition: "all 0.15s",
                  background: isActive ? "rgba(96, 165, 250, 0.15)" : "transparent",
                  color: isActive ? "var(--color-primary)" : "rgba(255,255,255,0.5)",
                  border: isActive ? "none" : "1px solid rgba(255,255,255,0.08)",
                })}
              title={item.title}
              >
                {item.label}
              </NavLink>
            ))}
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
            {tema === "dark" ? "☀" : "🌙"}
          </button>
          <button
            onClick={() => setMostrarAyuda(!mostrarAyuda)}
            style={{
              background: "rgba(255,255,255,0.06)", border: "1px solid rgba(255,255,255,0.15)",
              borderRadius: 6, cursor: "pointer", color: "rgba(255,255,255,0.6)",
              fontSize: 13, padding: "4px 10px", fontWeight: 600,
            }}
          >
            ?
          </button>
          <button
            onClick={cerrarSesion}
            style={{
              background: "none", border: "none", cursor: "pointer",
              color: "rgba(255,255,255,0.5)", fontSize: 12,
            }}
          >
            Salir
          </button>
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

      {/* Main Content */}
      <main className="main-content">
        <Outlet />
      </main>

      {/* Bottom Navigation Bar */}
      <nav className="bottom-bar" style={enPOS ? { justifyContent: "space-between", paddingRight: 324 } : {}}>
        {enPOS ? (
          <>
            <div style={{ display: "flex", gap: 6 }}>
              <button className="btn btn-outline" style={{ fontSize: 11, padding: "4px 14px" }}
                onClick={() => window.dispatchEvent(new CustomEvent("pos-guardar-borrador"))}>
                Borrador
              </button>
              <button className="btn" style={{ fontSize: 11, padding: "4px 14px", fontWeight: 600,
                background: "rgba(251, 146, 60, 0.2)", color: "#fb923c",
                border: "1px solid rgba(251, 146, 60, 0.4)",
              }}
                onClick={() => window.dispatchEvent(new CustomEvent("pos-guardar-guia"))}>
                Guia R.
              </button>
              <button className="btn" style={{ fontSize: 11, padding: "4px 14px", fontWeight: 600,
                background: "rgba(96, 165, 250, 0.15)", color: "var(--color-primary)",
                border: "1px solid rgba(96, 165, 250, 0.3)",
              }}
                onClick={() => window.dispatchEvent(new CustomEvent("pos-guardar-cotizacion"))}>
                Cotizacion
              </button>
            </div>
          </>
        ) : (
          navFiltrados.map((item) => (
            <NavLink
              key={item.path}
              to={item.path}
              end={item.path === "/"}
              className={({ isActive }) => (isActive ? "active" : "")}
            >
              <span className="nav-icon">{item.icon}</span>
              <span>{item.label}</span>
            </NavLink>
          ))
        )}
      </nav>

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
