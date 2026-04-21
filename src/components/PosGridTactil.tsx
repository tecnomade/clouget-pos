import { useState, useMemo, useRef } from "react";
import type { ProductoTactil, ProductoBusqueda, Categoria } from "../types";

interface PosGridTactilProps {
  categorias: Categoria[];
  productosTactil: ProductoTactil[];
  onAgregarProducto: (producto: ProductoBusqueda) => void;
  onVerDetalle?: (productoId: number) => void;
  puedeVerDetalle?: boolean;
  busqueda: string;
  onBusquedaChange: (v: string) => void;
  resultados: ProductoBusqueda[];
  inputRef: React.RefObject<HTMLInputElement | null>;
}

export default function PosGridTactil({
  categorias,
  productosTactil,
  onAgregarProducto,
  onVerDetalle,
  puedeVerDetalle,
  busqueda,
  onBusquedaChange,
  resultados,
  inputRef,
}: PosGridTactilProps) {
  const [categoriaActiva, setCategoriaActiva] = useState<number | null>(null);
  const lastAddRef = useRef<{id: number, time: number}>({id: 0, time: 0});
  const categoriasRef = useRef<HTMLDivElement>(null);

  const scrollCategorias = (dir: "left" | "right") => {
    if (categoriasRef.current) {
      categoriasRef.current.scrollBy({ left: dir === "right" ? 200 : -200, behavior: "smooth" });
    }
  };

  const productosFiltrados = useMemo(() => {
    let lista = productosTactil;
    if (categoriaActiva !== null) {
      lista = lista.filter((p) => p.categoria_id === categoriaActiva);
    }
    if (busqueda.trim()) {
      const term = busqueda.toLowerCase();
      lista = lista.filter((p) => p.nombre.toLowerCase().includes(term));
    }
    return lista;
  }, [productosTactil, categoriaActiva, busqueda]);

  const handleTap = (p: ProductoTactil) => {
    const busquedaCompatible: ProductoBusqueda = {
      id: p.id,
      nombre: p.nombre,
      precio_venta: p.precio_venta,
      iva_porcentaje: p.iva_porcentaje,
      incluye_iva: p.incluye_iva ?? false,
      stock_actual: p.stock_actual,
      stock_minimo: 0,
      categoria_nombre: p.categoria_nombre,
      precio_lista: undefined,
    };
    const now = Date.now();
    if (lastAddRef.current.id === busquedaCompatible.id && now - lastAddRef.current.time < 500) return;
    lastAddRef.current = { id: busquedaCompatible.id, time: now };
    onAgregarProducto(busquedaCompatible);
  };

  return (
    <div style={{ display: "flex", flexDirection: "column", height: "100%", width: "100%", overflow: "hidden", minWidth: 0 }}>
      {/* Search bar */}
      <div style={{ padding: "8px 12px", borderBottom: "1px solid var(--color-border)", flexShrink: 0, position: "relative", minWidth: 0, overflow: "hidden" }}>
        <input
          ref={inputRef}
          className="input"
          data-action="busqueda"
          placeholder="Buscar producto... (Ctrl+B)"
          value={busqueda}
          onChange={(e) => onBusquedaChange(e.target.value)}
          style={{ fontSize: 14 }}
        />
        {/* Search results dropdown (same as normal mode) */}
        {busqueda.trim() && resultados.length > 0 && (
          <div style={{
            position: "absolute", zIndex: 100, background: "var(--color-surface)",
            border: "1px solid var(--color-border)", borderRadius: "var(--radius)",
            maxHeight: 200, overflowY: "auto", width: "calc(100% - 24px)",
            boxShadow: "0 4px 12px rgba(0,0,0,0.4)",
          }}>
            {resultados.map((r) => (
              <div key={r.id}
                onClick={() => { const now = Date.now(); if (lastAddRef.current.id === r.id && now - lastAddRef.current.time < 500) return; lastAddRef.current = { id: r.id, time: now }; onAgregarProducto(r); onBusquedaChange(""); }}
                style={{
                  padding: "8px 12px", cursor: "pointer", borderBottom: "1px solid var(--color-border)",
                  display: "flex", justifyContent: "space-between", alignItems: "center",
                }}
                onMouseEnter={(e) => (e.currentTarget.style.background = "var(--color-surface-hover)")}
                onMouseLeave={(e) => (e.currentTarget.style.background = "transparent")}
              >
                <span style={{ fontWeight: 500 }}>{r.nombre}</span>
                <span style={{ fontWeight: 700, color: "var(--color-primary)" }}>${r.precio_venta.toFixed(2)}</span>
              </div>
            ))}
          </div>
        )}
      </div>

      {/* Category tabs - scrollable with arrows */}
      <div style={{
        display: "flex", alignItems: "center", borderBottom: "1px solid var(--color-border)",
        flexShrink: 0,
      }}>
        <button onClick={() => scrollCategorias("left")}
          style={{ background: "none", border: "none", cursor: "pointer", padding: "4px 6px", fontSize: 16, color: "var(--color-text-secondary)", flexShrink: 0 }}>
          ◀
        </button>
        <div ref={categoriasRef} style={{
          display: "flex", gap: 6, padding: "6px 4px",
          overflowX: "auto", flexWrap: "nowrap", flex: 1, minWidth: 0,
          scrollbarWidth: "none",
        }}>
          <button
            className={`btn ${categoriaActiva === null ? "btn-primary" : "btn-outline"}`}
            style={{ fontSize: 12, padding: "4px 12px", whiteSpace: "nowrap", flexShrink: 0 }}
            onClick={() => setCategoriaActiva(null)}
          >
            Todos
          </button>
          {categorias.map((c) => (
            <button
              key={c.id}
              className={`btn ${categoriaActiva === c.id ? "btn-primary" : "btn-outline"}`}
              style={{ fontSize: 12, padding: "4px 12px", whiteSpace: "nowrap", flexShrink: 0 }}
              onClick={() => setCategoriaActiva(c.id!)}
            >
              {c.nombre}
            </button>
          ))}
        </div>
        <button onClick={() => scrollCategorias("right")}
          style={{ background: "none", border: "none", cursor: "pointer", padding: "4px 6px", fontSize: 16, color: "var(--color-text-secondary)", flexShrink: 0 }}>
          ▶
        </button>
      </div>

      {/* Product grid - scrollable */}
      <div style={{
        display: "grid",
        gridTemplateColumns: "repeat(auto-fill, minmax(130px, 1fr))",
        gap: 8,
        overflowY: "auto",
        overflowX: "hidden",
        flex: 1,
        padding: 8,
        alignContent: "start",
        minHeight: 0,
      }}>
        {productosFiltrados.length === 0 ? (
          <div style={{ gridColumn: "1 / -1", textAlign: "center", color: "var(--color-text-secondary)", padding: 40 }}>
            No hay productos {categoriaActiva !== null ? "en esta categoria" : ""}
          </div>
        ) : (
          productosFiltrados.map((p) => (
            <div key={p.id} style={{ position: "relative" }}>
              {puedeVerDetalle && onVerDetalle && (
                <button
                  onClick={(e) => { e.stopPropagation(); onVerDetalle(p.id); }}
                  title="Ver detalles"
                  style={{
                    position: "absolute", top: 4, right: 4, zIndex: 2,
                    background: "rgba(59, 130, 246, 0.15)",
                    border: "1px solid var(--color-primary)",
                    borderRadius: 4, cursor: "pointer", padding: "2px 6px",
                    fontSize: 12, color: "var(--color-primary)",
                  }}
                >
                  👁
                </button>
              )}
              <button
                onClick={() => handleTap(p)}
                style={{
                  display: "flex", flexDirection: "column",
                  alignItems: "center", justifyContent: "center",
                  padding: 10, border: "1px solid var(--color-border)",
                  borderRadius: 12, background: "var(--color-surface)",
                  color: "var(--color-text)",
                  cursor: "pointer", minHeight: 140, width: "100%",
                  boxShadow: "0 2px 8px rgba(0,0,0,0.2)",
                  opacity: p.stock_actual <= 0 ? 0.4 : 1,
                  transition: "transform 0.1s, box-shadow 0.1s",
                }}
                onMouseDown={(e) => (e.currentTarget.style.transform = "scale(0.95)")}
                onMouseUp={(e) => (e.currentTarget.style.transform = "scale(1)")}
                onMouseLeave={(e) => (e.currentTarget.style.transform = "scale(1)")}
              >
              {p.imagen ? (
                <img
                  src={`data:image/png;base64,${p.imagen}`}
                  alt={p.nombre}
                  style={{ width: 80, height: 80, objectFit: "contain", borderRadius: 6 }}
                />
              ) : (
                <div style={{
                  width: 80, height: 80, background: "var(--color-surface-alt)", borderRadius: 6,
                  display: "flex", alignItems: "center", justifyContent: "center",
                  color: "var(--color-text-secondary)", fontSize: 22, fontWeight: 700,
                }}>
                  {p.nombre.charAt(0).toUpperCase()}
                </div>
              )}
              <span style={{
                fontSize: 11, fontWeight: 600, marginTop: 4,
                textAlign: "center", lineHeight: 1.2,
                overflow: "hidden", textOverflow: "ellipsis",
                display: "-webkit-box", WebkitLineClamp: 2, WebkitBoxOrient: "vertical",
                width: "100%",
              }}>
                {p.nombre}
              </span>
              <span style={{ fontSize: 12, fontWeight: 700, color: "var(--color-primary)", marginTop: 2 }}>
                ${p.precio_venta.toFixed(2)}
              </span>
              {p.stock_actual <= 0 ? (
                <span style={{ fontSize: 12, color: "var(--color-danger)", marginTop: 2, fontWeight: 700 }}>Sin stock</span>
              ) : (
                <span style={{ fontSize: 12, color: "var(--color-text-secondary)", marginTop: 2, fontWeight: 600 }}>Stock: <strong style={{ color: "var(--color-text)" }}>{p.stock_actual}</strong></span>
              )}
              </button>
            </div>
          ))
        )}
      </div>
    </div>
  );
}
