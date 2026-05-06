/**
 * SelectorProductos — Modal con grid táctil de productos para agregar al pedido.
 *
 * Reutiliza `listarProductosTactil` y filtros por categoría existentes.
 * Soporta búsqueda por nombre/código.
 *
 * Al click en producto: emite `onSeleccionar` con el producto y opcional
 * `info_adicional` capturada en mini-prompt si el usuario hace click derecho.
 */

import { useState, useEffect, useMemo } from "react";
import { listarProductosTactil } from "../../services/api";
import type { ProductoTactil } from "../../types";

interface Props {
  onSeleccionar: (producto: { id: number; nombre: string }, infoAdicional: string | null) => void;
  onCerrar: () => void;
}

export default function SelectorProductos({ onSeleccionar, onCerrar }: Props) {
  const [productos, setProductos] = useState<ProductoTactil[]>([]);
  const [busqueda, setBusqueda] = useState("");
  const [categoriaActiva, setCategoriaActiva] = useState<number | "todas">("todas");
  const [cargando, setCargando] = useState(true);
  const [productoConObs, setProductoConObs] = useState<ProductoTactil | null>(null);
  const [obsInput, setObsInput] = useState("");

  useEffect(() => {
    listarProductosTactil()
      .then(setProductos)
      .catch(() => setProductos([]))
      .finally(() => setCargando(false));
  }, []);

  const categorias = useMemo(() => {
    const m = new Map<number, string>();
    for (const p of productos) {
      if (p.categoria_id && p.categoria_nombre) m.set(p.categoria_id, p.categoria_nombre);
    }
    return Array.from(m.entries()).map(([id, nombre]) => ({ id, nombre }));
  }, [productos]);

  const filtrados = useMemo(() => {
    const term = busqueda.trim().toLowerCase();
    return productos.filter((p) => {
      if (categoriaActiva !== "todas" && p.categoria_id !== categoriaActiva) return false;
      if (!term) return true;
      return p.nombre.toLowerCase().includes(term);
    });
  }, [productos, busqueda, categoriaActiva]);

  const handleClickProducto = (p: ProductoTactil) => {
    onSeleccionar({ id: p.id, nombre: p.nombre }, null);
  };

  const handleConfirmarObs = () => {
    if (!productoConObs) return;
    onSeleccionar(
      { id: productoConObs.id, nombre: productoConObs.nombre },
      obsInput.trim() || null,
    );
    setProductoConObs(null);
    setObsInput("");
  };

  return (
    <>
      <div className="modal-overlay" onClick={onCerrar} style={{ alignItems: "center" }}>
        <div
          onClick={(e) => e.stopPropagation()}
          style={{
            background: "var(--color-surface)",
            width: "min(900px, 95vw)",
            height: "min(700px, 90vh)",
            display: "flex",
            flexDirection: "column",
            borderRadius: 12,
            overflow: "hidden",
          }}
        >
          {/* Header */}
          <div
            style={{
              padding: "12px 16px",
              borderBottom: "1px solid var(--color-border)",
              display: "flex",
              gap: 8,
              alignItems: "center",
            }}
          >
            <input
              autoFocus
              placeholder="Buscar producto..."
              value={busqueda}
              onChange={(e) => setBusqueda(e.target.value)}
              className="input"
              style={{ flex: 1, fontSize: 15, padding: "8px 12px" }}
            />
            <button
              onClick={onCerrar}
              style={{
                background: "transparent",
                border: "none",
                fontSize: 22,
                cursor: "pointer",
                color: "var(--color-text-muted)",
                padding: 0,
                width: 30,
                height: 30,
              }}
            >
              ×
            </button>
          </div>

          {/* Categorías */}
          {categorias.length > 0 && (
            <div
              style={{
                display: "flex",
                gap: 6,
                padding: "8px 16px",
                overflowX: "auto",
                borderBottom: "1px solid var(--color-border)",
              }}
            >
              <ChipCat label="Todas" activa={categoriaActiva === "todas"} onClick={() => setCategoriaActiva("todas")} />
              {categorias.map((c) => (
                <ChipCat
                  key={c.id}
                  label={c.nombre}
                  activa={categoriaActiva === c.id}
                  onClick={() => setCategoriaActiva(c.id)}
                />
              ))}
            </div>
          )}

          {/* Grid productos */}
          <div style={{ flex: 1, overflowY: "auto", padding: 12 }}>
            {cargando ? (
              <div style={{ padding: 30, textAlign: "center", color: "var(--color-text-muted)" }}>
                Cargando productos...
              </div>
            ) : filtrados.length === 0 ? (
              <div style={{ padding: 30, textAlign: "center", color: "var(--color-text-muted)" }}>
                Sin resultados
              </div>
            ) : (
              <div
                style={{
                  display: "grid",
                  gridTemplateColumns: "repeat(auto-fill, minmax(140px, 1fr))",
                  gap: 8,
                }}
              >
                {filtrados.map((p) => (
                  <CardProducto
                    key={p.id}
                    producto={p}
                    onClick={() => handleClickProducto(p)}
                    onConObs={() => {
                      setProductoConObs(p);
                      setObsInput("");
                    }}
                  />
                ))}
              </div>
            )}
          </div>

          {/* Footer hint */}
          <div
            style={{
              padding: "8px 16px",
              borderTop: "1px solid var(--color-border)",
              fontSize: 11,
              color: "var(--color-text-muted)",
              display: "flex",
              justifyContent: "space-between",
            }}
          >
            <span>Click = agregar 1 · Click derecho o 📝 = con observación (sin cebolla, etc.)</span>
            <span>{filtrados.length} producto(s)</span>
          </div>
        </div>
      </div>

      {/* Modal observación */}
      {productoConObs && (
        <div className="modal-overlay" onClick={() => setProductoConObs(null)} style={{ zIndex: 200 }}>
          <div
            className="modal-content"
            style={{ maxWidth: 380 }}
            onClick={(e) => e.stopPropagation()}
          >
            <div className="modal-header">
              <h3 style={{ margin: 0, fontSize: 16 }}>Observación para {productoConObs.nombre}</h3>
            </div>
            <div className="modal-body">
              <input
                autoFocus
                placeholder='Ej: sin cebolla, término medio, picante...'
                value={obsInput}
                onChange={(e) => setObsInput(e.target.value)}
                onKeyDown={(e) => e.key === "Enter" && handleConfirmarObs()}
                className="input"
                style={{ width: "100%" }}
              />
            </div>
            <div className="modal-footer" style={{ display: "flex", gap: 8, justifyContent: "flex-end" }}>
              <button className="btn btn-outline" onClick={() => setProductoConObs(null)}>
                Cancelar
              </button>
              <button className="btn btn-primary" onClick={handleConfirmarObs}>
                Agregar
              </button>
            </div>
          </div>
        </div>
      )}
    </>
  );
}

// ─── Sub-componentes ────────────────────────────────────────────────────

function ChipCat({ label, activa, onClick }: { label: string; activa: boolean; onClick: () => void }) {
  return (
    <button
      onClick={onClick}
      style={{
        padding: "6px 12px",
        borderRadius: 999,
        border: `1.5px solid ${activa ? "var(--color-primary)" : "var(--color-border)"}`,
        background: activa ? "var(--color-primary)" : "transparent",
        color: activa ? "#fff" : "var(--color-text)",
        fontSize: 12,
        fontWeight: 600,
        cursor: "pointer",
        whiteSpace: "nowrap",
        flexShrink: 0,
      }}
    >
      {label}
    </button>
  );
}

function CardProducto({
  producto,
  onClick,
  onConObs,
}: {
  producto: ProductoTactil;
  onClick: () => void;
  onConObs: () => void;
}) {
  const sinStock =
    !producto.es_servicio &&
    !producto.no_controla_stock &&
    producto.stock_actual <= 0 &&
    producto.tipo_producto !== "COMBO_FLEXIBLE";

  return (
    <div
      onContextMenu={(e) => {
        e.preventDefault();
        onConObs();
      }}
      style={{
        border: "1px solid var(--color-border)",
        borderRadius: 10,
        background: "var(--color-surface)",
        cursor: sinStock ? "not-allowed" : "pointer",
        opacity: sinStock ? 0.5 : 1,
        display: "flex",
        flexDirection: "column",
        overflow: "hidden",
        position: "relative",
        transition: "transform 0.1s",
      }}
      onMouseEnter={(e) => !sinStock && (e.currentTarget.style.transform = "translateY(-2px)")}
      onMouseLeave={(e) => (e.currentTarget.style.transform = "translateY(0)")}
      onClick={() => !sinStock && onClick()}
    >
      <div
        style={{
          height: 80,
          background: "linear-gradient(135deg, rgba(59,130,246,0.15) 0%, rgba(59,130,246,0.05) 100%)",
          display: "flex",
          alignItems: "center",
          justifyContent: "center",
          overflow: "hidden",
        }}
      >
        {producto.imagen ? (
          <img
            src={`data:image/png;base64,${producto.imagen}`}
            alt={producto.nombre}
            style={{ width: "100%", height: "100%", objectFit: "cover", display: "block" }}
          />
        ) : (
          // Fallback estilo POS normal: inicial del nombre en grande con gradient
          <span style={{
            fontSize: 32, fontWeight: 800,
            color: "var(--color-primary)",
            opacity: 0.6,
          }}>
            {producto.nombre.charAt(0).toUpperCase()}
          </span>
        )}
      </div>
      <div style={{ padding: "6px 8px", display: "flex", flexDirection: "column", gap: 2 }}>
        <span
          style={{
            fontSize: 12,
            fontWeight: 600,
            lineHeight: 1.25,
            display: "-webkit-box",
            WebkitLineClamp: 2,
            WebkitBoxOrient: "vertical",
            overflow: "hidden",
          }}
        >
          {producto.nombre}
        </span>
        <strong style={{ fontSize: 13, color: "var(--color-primary)" }}>
          ${producto.precio_venta.toFixed(2)}
        </strong>
      </div>
      <button
        onClick={(e) => {
          e.stopPropagation();
          onConObs();
        }}
        title="Agregar con observación"
        style={{
          position: "absolute",
          top: 4,
          right: 4,
          background: "rgba(0,0,0,0.5)",
          color: "#fff",
          border: "none",
          borderRadius: 4,
          width: 22,
          height: 22,
          fontSize: 12,
          cursor: "pointer",
          display: "flex",
          alignItems: "center",
          justifyContent: "center",
        }}
      >
        📝
      </button>
      {sinStock && (
        <div
          style={{
            position: "absolute",
            inset: 0,
            background: "rgba(239, 68, 68, 0.1)",
            display: "flex",
            alignItems: "center",
            justifyContent: "center",
            color: "var(--color-danger)",
            fontWeight: 700,
            fontSize: 11,
          }}
        >
          SIN STOCK
        </div>
      )}
    </div>
  );
}
