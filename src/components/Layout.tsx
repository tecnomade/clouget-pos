import { useState } from "react";
import { NavLink, Outlet } from "react-router-dom";
import { useKeyboardShortcuts, SHORTCUTS_LIST } from "../hooks/useKeyboardShortcuts";
import { useSesion } from "../contexts/SesionContext";
import SuscripcionBanner from "./SuscripcionBanner";
import UpdateChecker from "./UpdateChecker";

const navItems = [
  { path: "/", label: "Inicio", icon: "I", shortcut: "", todos: true },
  { path: "/pos", label: "Venta", icon: "V", shortcut: "F1", todos: true },
  { path: "/productos", label: "Productos", icon: "P", shortcut: "F2", todos: false },
  { path: "/clientes", label: "Clientes", icon: "C", shortcut: "F3", todos: false },
  { path: "/ventas", label: "Ventas del dia", icon: "D", shortcut: "F4", todos: true },
  { path: "/caja", label: "Caja", icon: "$", shortcut: "F5", todos: true },
  { path: "/gastos", label: "Gastos", icon: "G", shortcut: "F7", todos: false },
  { path: "/cuentas", label: "Fiados", icon: "F", shortcut: "F8", todos: false },
  { path: "/inventario", label: "Inventario", icon: "K", shortcut: "", todos: false },
  { path: "/config", label: "Configuracion", icon: "*", shortcut: "F6", todos: false },
];

export default function Layout() {
  const { sesion, cerrarSesion, esAdmin } = useSesion();
  useKeyboardShortcuts(sesion?.rol);
  const [mostrarAyuda, setMostrarAyuda] = useState(false);

  const navFiltrados = esAdmin
    ? navItems
    : navItems.filter((item) => item.todos);

  return (
    <div className="app-layout">
      <aside className="sidebar">
        <div className="sidebar-header">
          <h1>CLOUGET</h1>
          <span>Punto de Venta</span>
        </div>
        <nav className="sidebar-nav">
          {navFiltrados.map((item) => (
            <NavLink
              key={item.path}
              to={item.path}
              end={item.path === "/"}
              className={({ isActive }) => (isActive ? "active" : "")}
            >
              <span className="nav-icon">{item.icon}</span>
              <span style={{ flex: 1 }}>{item.label}</span>
              <span style={{ fontSize: 10, opacity: 0.5 }}>{item.shortcut}</span>
            </NavLink>
          ))}
        </nav>

        {/* Info de sesion */}
        <div style={{ padding: 8, borderTop: "1px solid rgba(255,255,255,0.08)" }}>
          {sesion && (
            <div style={{ padding: "8px 4px", marginBottom: 8 }}>
              <div style={{ fontSize: 13, fontWeight: 600, color: "rgba(255,255,255,0.85)" }}>
                {sesion.nombre}
              </div>
              <div style={{ display: "flex", alignItems: "center", gap: 6, marginTop: 4 }}>
                <span style={{
                  fontSize: 10,
                  padding: "1px 6px",
                  borderRadius: 3,
                  background: sesion.rol === "ADMIN" ? "#3b82f6" : "#64748b",
                  color: "white",
                  fontWeight: 600,
                }}>
                  {sesion.rol}
                </span>
              </div>
            </div>
          )}
          <button
            className="btn btn-outline"
            style={{ width: "100%", color: "rgba(255,255,255,0.5)", borderColor: "rgba(255,255,255,0.1)", fontSize: 12, marginBottom: 4 }}
            onClick={() => setMostrarAyuda(!mostrarAyuda)}
          >
            ? Atajos de teclado
          </button>
          <button
            className="btn btn-outline"
            style={{ width: "100%", color: "rgba(255,255,255,0.4)", borderColor: "rgba(255,255,255,0.08)", fontSize: 11 }}
            onClick={cerrarSesion}
          >
            Cerrar Sesion
          </button>
        </div>
      </aside>
      <main className="main-content">
        <UpdateChecker />
        <SuscripcionBanner />
        <Outlet />
      </main>

      {mostrarAyuda && (
        <div
          style={{
            position: "fixed",
            inset: 0,
            background: "rgba(0,0,0,0.5)",
            display: "flex",
            alignItems: "center",
            justifyContent: "center",
            zIndex: 100,
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
                      background: "#f1f5f9",
                      border: "1px solid #cbd5e1",
                      borderRadius: 4,
                      padding: "2px 8px",
                      fontSize: 12,
                      fontFamily: "monospace",
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
