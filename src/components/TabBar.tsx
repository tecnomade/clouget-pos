/**
 * v2.5.0 — Barra de pestañas (estilo navegador).
 * Renderiza la lista de tabs abiertas. Click activa, X cierra.
 * La tab activa se resalta. Solo se renderiza si tabs están habilitadas.
 */
import { useTabs } from "../contexts/TabsContext";
import { useNavigate } from "react-router-dom";

export default function TabBar() {
  const { enabled, tabs, activeId, setActive, close } = useTabs();
  const navigate = useNavigate();

  if (!enabled) return null;
  if (tabs.length <= 1) {
    // Si solo hay Inicio (pineada), no mostrar la barra — ahorra espacio vertical
    return null;
  }

  return (
    <div
      style={{
        display: "flex",
        alignItems: "stretch",
        background: "var(--color-surface-alt)",
        borderBottom: "1px solid var(--color-border)",
        overflowX: "auto",
        overflowY: "hidden",
        flexShrink: 0,
        height: 36,
      }}
    >
      {tabs.map((tab) => {
        const activa = tab.path === activeId;
        return (
          <div
            key={tab.path}
            onClick={() => {
              setActive(tab.path);
              navigate(tab.path, { replace: true });
            }}
            onMouseDown={(e) => {
              // Middle-click cierra (estilo navegador), salvo pineada
              if (e.button === 1 && !tab.pinned) {
                e.preventDefault();
                close(tab.path);
                if (activa) {
                  // Navegar a la nueva activa después del cierre
                  setTimeout(() => {
                    const next = tabs.find(t => t.path !== tab.path);
                    if (next) navigate(next.path, { replace: true });
                  }, 0);
                }
              }
            }}
            style={{
              display: "flex",
              alignItems: "center",
              gap: 6,
              padding: "0 12px",
              cursor: "pointer",
              borderRight: "1px solid var(--color-border)",
              background: activa ? "var(--color-bg)" : "transparent",
              color: activa ? "var(--color-text)" : "var(--color-text-secondary)",
              fontSize: 12,
              fontWeight: activa ? 600 : 500,
              borderTop: activa ? "2px solid var(--color-primary)" : "2px solid transparent",
              minWidth: 100,
              maxWidth: 220,
              userSelect: "none",
              whiteSpace: "nowrap",
              transition: "background 0.1s",
            }}
            onMouseEnter={(e) => {
              if (!activa) e.currentTarget.style.background = "var(--color-surface-hover, rgba(255,255,255,0.04))";
            }}
            onMouseLeave={(e) => {
              if (!activa) e.currentTarget.style.background = "transparent";
            }}
            title={tab.title}
          >
            <span style={{ fontSize: 13 }}>{tab.icon}</span>
            <span style={{ overflow: "hidden", textOverflow: "ellipsis", flex: 1 }}>
              {tab.title}
            </span>
            {!tab.pinned && (
              <button
                type="button"
                onClick={(e) => {
                  e.stopPropagation();
                  close(tab.path);
                  if (activa) {
                    // Navegar a la nueva activa después del cierre
                    setTimeout(() => {
                      const next = tabs.find(t => t.path !== tab.path);
                      if (next) navigate(next.path, { replace: true });
                    }, 0);
                  }
                }}
                style={{
                  background: "transparent",
                  border: "none",
                  color: "inherit",
                  cursor: "pointer",
                  padding: "2px 4px",
                  borderRadius: 4,
                  fontSize: 12,
                  opacity: 0.6,
                  lineHeight: 1,
                }}
                onMouseEnter={(e) => {
                  e.currentTarget.style.background = "rgba(239,68,68,0.2)";
                  e.currentTarget.style.opacity = "1";
                }}
                onMouseLeave={(e) => {
                  e.currentTarget.style.background = "transparent";
                  e.currentTarget.style.opacity = "0.6";
                }}
                title="Cerrar pestaña"
              >
                ✕
              </button>
            )}
          </div>
        );
      })}
    </div>
  );
}
