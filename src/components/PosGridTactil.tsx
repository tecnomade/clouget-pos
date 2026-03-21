import { useState, useMemo } from "react";
import type { ProductoTactil, ProductoBusqueda, Categoria, ItemCarrito } from "../types";

interface PosGridTactilProps {
  categorias: Categoria[];
  productosTactil: ProductoTactil[];
  carrito: ItemCarrito[];
  onAgregarProducto: (producto: ProductoBusqueda) => void;
  onActualizarCantidad: (productoId: number, cantidad: number) => void;
  onEliminarItem: (productoId: number) => void;
  busqueda: string;
  onBusquedaChange: (v: string) => void;
  resultados: ProductoBusqueda[];
  inputRef: React.RefObject<HTMLInputElement | null>;
}

export default function PosGridTactil({
  categorias,
  productosTactil,
  carrito,
  onAgregarProducto,
  onActualizarCantidad,
  onEliminarItem,
  busqueda,
  onBusquedaChange,
  resultados,
  inputRef,
}: PosGridTactilProps) {
  const [categoriaActiva, setCategoriaActiva] = useState<number | null>(null);
  const [carritoExpandido, setCarritoExpandido] = useState(false);

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

  const totalCarrito = carrito.reduce((s, i) => s + i.subtotal, 0);
  const itemsCarrito = carrito.reduce((s, i) => s + i.cantidad, 0);

  const handleTap = (p: ProductoTactil) => {
    const busquedaCompatible: ProductoBusqueda = {
      id: p.id,
      nombre: p.nombre,
      precio_venta: p.precio_venta,
      iva_porcentaje: p.iva_porcentaje,
      stock_actual: p.stock_actual,
      stock_minimo: 0,
      categoria_nombre: p.categoria_nombre,
      precio_lista: undefined,
    };
    onAgregarProducto(busquedaCompatible);
  };

  return (
    <div style={{ display: "flex", flexDirection: "column", height: "100%", overflow: "hidden" }}>
      {/* Search bar */}
      <div style={{ padding: "8px 12px", borderBottom: "1px solid var(--color-border)" }}>
        <input
          ref={inputRef}
          className="input"
          placeholder="Buscar producto..."
          value={busqueda}
          onChange={(e) => onBusquedaChange(e.target.value)}
          style={{ fontSize: 14 }}
        />
        {/* Search results dropdown (same as normal mode) */}
        {busqueda.trim() && resultados.length > 0 && (
          <div style={{
            position: "absolute", zIndex: 100, background: "white",
            border: "1px solid var(--color-border)", borderRadius: "var(--radius)",
            maxHeight: 200, overflowY: "auto", width: "calc(100% - 24px)",
            boxShadow: "0 4px 12px rgba(0,0,0,0.15)",
          }}>
            {resultados.map((r) => (
              <div key={r.id}
                onClick={() => { onAgregarProducto(r); onBusquedaChange(""); }}
                style={{
                  padding: "8px 12px", cursor: "pointer", borderBottom: "1px solid #f1f5f9",
                  display: "flex", justifyContent: "space-between", alignItems: "center",
                }}
                onMouseEnter={(e) => (e.currentTarget.style.background = "#f8fafc")}
                onMouseLeave={(e) => (e.currentTarget.style.background = "white")}
              >
                <span style={{ fontWeight: 500 }}>{r.nombre}</span>
                <span style={{ fontWeight: 700, color: "var(--color-primary)" }}>${r.precio_venta.toFixed(2)}</span>
              </div>
            ))}
          </div>
        )}
      </div>

      {/* Category tabs */}
      <div style={{
        display: "flex", gap: 6, padding: "8px 12px",
        overflowX: "auto", borderBottom: "1px solid var(--color-border)",
        flexShrink: 0,
      }}>
        <button
          className={`btn ${categoriaActiva === null ? "btn-primary" : "btn-outline"}`}
          style={{ fontSize: 12, padding: "4px 12px", whiteSpace: "nowrap" }}
          onClick={() => setCategoriaActiva(null)}
        >
          Todos
        </button>
        {categorias.map((c) => (
          <button
            key={c.id}
            className={`btn ${categoriaActiva === c.id ? "btn-primary" : "btn-outline"}`}
            style={{ fontSize: 12, padding: "4px 12px", whiteSpace: "nowrap" }}
            onClick={() => setCategoriaActiva(c.id!)}
          >
            {c.nombre}
          </button>
        ))}
      </div>

      {/* Product grid */}
      <div style={{
        display: "grid",
        gridTemplateColumns: "repeat(auto-fill, minmax(120px, 1fr))",
        gap: 8,
        overflowY: "auto",
        flex: 1,
        padding: 8,
        alignContent: "start",
      }}>
        {productosFiltrados.length === 0 ? (
          <div style={{ gridColumn: "1 / -1", textAlign: "center", color: "#94a3b8", padding: 40 }}>
            No hay productos {categoriaActiva !== null ? "en esta categoria" : ""}
          </div>
        ) : (
          productosFiltrados.map((p) => (
            <button
              key={p.id}
              onClick={() => handleTap(p)}
              style={{
                display: "flex", flexDirection: "column",
                alignItems: "center", justifyContent: "center",
                padding: 8, border: "1px solid var(--color-border)",
                borderRadius: "var(--radius)", background: "white",
                cursor: "pointer", minHeight: 130,
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
                  style={{ width: 64, height: 64, objectFit: "cover", borderRadius: 6 }}
                />
              ) : (
                <div style={{
                  width: 64, height: 64, background: "#f1f5f9", borderRadius: 6,
                  display: "flex", alignItems: "center", justifyContent: "center",
                  color: "#94a3b8", fontSize: 22, fontWeight: 700,
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
              {p.stock_actual <= 0 && (
                <span style={{ fontSize: 9, color: "#ef4444", marginTop: 1 }}>Sin stock</span>
              )}
            </button>
          ))
        )}
      </div>

      {/* Cart bar (collapsible) */}
      <div style={{
        borderTop: "2px solid var(--color-border)",
        background: "white",
      }}>
        {/* Cart header - always visible */}
        <div
          onClick={() => setCarritoExpandido(!carritoExpandido)}
          style={{
            padding: "10px 12px", cursor: "pointer",
            display: "flex", justifyContent: "space-between", alignItems: "center",
            fontWeight: 600, fontSize: 14,
          }}
        >
          <span>
            Carrito ({itemsCarrito} {itemsCarrito === 1 ? "item" : "items"})
            {carritoExpandido ? " ▼" : " ▲"}
          </span>
          <span style={{ color: "var(--color-primary)", fontSize: 16 }}>
            ${totalCarrito.toFixed(2)}
          </span>
        </div>

        {/* Cart items - expandable */}
        {carritoExpandido && (
          <div style={{ maxHeight: 200, overflowY: "auto", borderTop: "1px solid var(--color-border)" }}>
            {carrito.length === 0 ? (
              <div style={{ padding: 16, textAlign: "center", color: "#94a3b8", fontSize: 12 }}>
                Carrito vacio
              </div>
            ) : (
              carrito.map((item) => (
                <div key={item.producto_id} style={{
                  padding: "6px 12px", borderBottom: "1px solid #f1f5f9",
                  display: "flex", alignItems: "center", gap: 8, fontSize: 12,
                }}>
                  <span style={{ flex: 1, fontWeight: 500 }}>{item.nombre}</span>
                  <div style={{ display: "flex", alignItems: "center", gap: 4 }}>
                    <button
                      className="btn btn-outline"
                      style={{ padding: "0 6px", fontSize: 14, lineHeight: 1, minWidth: 24 }}
                      onClick={() => {
                        if (item.cantidad <= 1) {
                          onEliminarItem(item.producto_id);
                        } else {
                          onActualizarCantidad(item.producto_id, item.cantidad - 1);
                        }
                      }}
                    >
                      -
                    </button>
                    <span style={{ minWidth: 20, textAlign: "center", fontWeight: 600 }}>
                      {item.cantidad}
                    </span>
                    <button
                      className="btn btn-outline"
                      style={{ padding: "0 6px", fontSize: 14, lineHeight: 1, minWidth: 24 }}
                      onClick={() => onActualizarCantidad(item.producto_id, item.cantidad + 1)}
                    >
                      +
                    </button>
                  </div>
                  <span style={{ fontWeight: 600, minWidth: 60, textAlign: "right" }}>
                    ${item.subtotal.toFixed(2)}
                  </span>
                  <button
                    style={{ background: "none", border: "none", cursor: "pointer", color: "#ef4444", fontSize: 14, padding: 2 }}
                    onClick={() => onEliminarItem(item.producto_id)}
                  >
                    x
                  </button>
                </div>
              ))
            )}
          </div>
        )}
      </div>
    </div>
  );
}
