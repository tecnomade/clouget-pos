import { useState, useEffect, useCallback } from "react";
import {
  listarMovimientos,
  registrarMovimiento,
  resumenInventario,
  buscarProductos,
} from "../services/api";
import type { MovimientoInventario, ResumenInventario } from "../services/api";
import type { ProductoBusqueda } from "../types";
import { useToast } from "../components/Toast";
import { useSesion } from "../contexts/SesionContext";

function fechaHoy(): string {
  const now = new Date();
  return `${now.getFullYear()}-${String(now.getMonth() + 1).padStart(2, "0")}-${String(now.getDate()).padStart(2, "0")}`;
}

function fechaHace(dias: number): string {
  const now = new Date();
  now.setDate(now.getDate() - dias);
  return `${now.getFullYear()}-${String(now.getMonth() + 1).padStart(2, "0")}-${String(now.getDate()).padStart(2, "0")}`;
}

const TIPO_COLORES: Record<string, { bg: string; color: string }> = {
  ENTRADA: { bg: "#dcfce7", color: "#166534" },
  SALIDA: { bg: "#fee2e2", color: "#dc2626" },
  VENTA: { bg: "#dbeafe", color: "#1e40af" },
  AJUSTE: { bg: "#fef3c7", color: "#92400e" },
  DEVOLUCION: { bg: "#f3e8ff", color: "#7c3aed" },
};

export default function InventarioPage() {
  const { toastExito, toastError } = useToast();
  const { sesion } = useSesion();
  const [movimientos, setMovimientos] = useState<MovimientoInventario[]>([]);
  const [resumen, setResumen] = useState<ResumenInventario | null>(null);
  const [fechaDesde, setFechaDesde] = useState(fechaHace(30));
  const [fechaHasta, setFechaHasta] = useState(fechaHoy);
  const [filtroTipo, setFiltroTipo] = useState<string>("");
  const [filtroProductoId, setFiltroProductoId] = useState<number | undefined>(undefined);
  const [filtroProductoNombre, setFiltroProductoNombre] = useState("");

  // Modal de movimiento
  const [modalAbierto, setModalAbierto] = useState(false);
  const [modalTipo, setModalTipo] = useState<"ENTRADA" | "AJUSTE">("ENTRADA");
  const [modalProducto, setModalProducto] = useState<ProductoBusqueda | null>(null);
  const [modalBusqueda, setModalBusqueda] = useState("");
  const [modalResultados, setModalResultados] = useState<ProductoBusqueda[]>([]);
  const [modalCantidad, setModalCantidad] = useState("");
  const [modalMotivo, setModalMotivo] = useState("");
  const [modalCosto, setModalCosto] = useState("");

  const cargar = useCallback(async () => {
    const [movs, res] = await Promise.all([
      listarMovimientos(filtroProductoId, fechaDesde, fechaHasta, filtroTipo || undefined, 200),
      resumenInventario(),
    ]);
    setMovimientos(movs);
    setResumen(res);
  }, [fechaDesde, fechaHasta, filtroTipo, filtroProductoId]);

  useEffect(() => { cargar(); }, [cargar]);

  const handleBuscarProductoModal = async (termino: string) => {
    setModalBusqueda(termino);
    if (termino.length >= 1) {
      const res = await buscarProductos(termino);
      setModalResultados(res);
    } else {
      setModalResultados([]);
    }
  };

  const handleRegistrar = async () => {
    if (!modalProducto) return toastError("Seleccione un producto");
    const cant = parseFloat(modalCantidad);
    if (isNaN(cant) || cant <= 0) return toastError("Cantidad invalida");

    try {
      await registrarMovimiento(
        modalProducto.id,
        modalTipo,
        modalTipo === "AJUSTE" ? cant : cant,
        modalMotivo || undefined,
        modalCosto ? parseFloat(modalCosto) : undefined,
        sesion?.nombre,
      );
      toastExito(`${modalTipo === "ENTRADA" ? "Entrada" : "Ajuste"} registrado`);
      setModalAbierto(false);
      setModalProducto(null);
      setModalBusqueda("");
      setModalCantidad("");
      setModalMotivo("");
      setModalCosto("");
      await cargar();
    } catch (err) {
      toastError("Error: " + err);
    }
  };

  return (
    <>
      <div className="page-header">
        <h2>Inventario / Kardex</h2>
        <div className="flex gap-2 items-center">
          <button className="btn btn-primary" style={{ fontSize: 12, padding: "6px 14px" }}
            onClick={() => { setModalTipo("ENTRADA"); setModalAbierto(true); }}>
            + Entrada
          </button>
          <button className="btn btn-outline" style={{ fontSize: 12, padding: "6px 14px" }}
            onClick={() => { setModalTipo("AJUSTE"); setModalAbierto(true); }}>
            Ajuste
          </button>
        </div>
      </div>

      <div className="page-body">
        {/* KPIs */}
        {resumen && (
          <div style={{ display: "grid", gridTemplateColumns: "repeat(auto-fit, minmax(150px, 1fr))", gap: 12, marginBottom: 20 }}>
            <div className="card" style={{ padding: 14 }}>
              <div className="text-secondary" style={{ fontSize: 11 }}>Productos</div>
              <div className="text-xl font-bold">{resumen.total_productos}</div>
            </div>
            <div className="card" style={{ padding: 14 }}>
              <div className="text-secondary" style={{ fontSize: 11 }}>Valor Inventario</div>
              <div className="text-xl font-bold text-success">${resumen.valor_inventario.toFixed(2)}</div>
            </div>
            <div className="card" style={{ padding: 14 }}>
              <div className="text-secondary" style={{ fontSize: 11 }}>Entradas (mes)</div>
              <div className="text-xl font-bold" style={{ color: "#166534" }}>{resumen.total_entradas_mes}</div>
            </div>
            <div className="card" style={{ padding: 14 }}>
              <div className="text-secondary" style={{ fontSize: 11 }}>Salidas (mes)</div>
              <div className="text-xl font-bold" style={{ color: "#dc2626" }}>{resumen.total_salidas_mes}</div>
            </div>
            <div className="card" style={{ padding: 14 }}>
              <div className="text-secondary" style={{ fontSize: 11 }}>Ajustes (mes)</div>
              <div className="text-xl font-bold">{resumen.total_ajustes_mes}</div>
            </div>
          </div>
        )}

        {/* Filtros */}
        <div className="card" style={{ padding: 12, marginBottom: 16 }}>
          <div className="flex gap-2 items-center" style={{ flexWrap: "wrap" }}>
            <input type="date" className="input" style={{ width: 140, fontSize: 12 }}
              value={fechaDesde} onChange={(e) => setFechaDesde(e.target.value)} />
            <span className="text-secondary" style={{ fontSize: 12 }}>a</span>
            <input type="date" className="input" style={{ width: 140, fontSize: 12 }}
              value={fechaHasta} onChange={(e) => setFechaHasta(e.target.value)} />
            <select className="input" style={{ width: 130, fontSize: 12 }}
              value={filtroTipo} onChange={(e) => setFiltroTipo(e.target.value)}>
              <option value="">Todos los tipos</option>
              <option value="ENTRADA">Entrada</option>
              <option value="SALIDA">Salida</option>
              <option value="VENTA">Venta</option>
              <option value="AJUSTE">Ajuste</option>
              <option value="DEVOLUCION">Devolucion</option>
            </select>
            {filtroProductoId ? (
              <div className="flex items-center gap-1">
                <span style={{ fontSize: 12, background: "#eff6ff", padding: "4px 8px", borderRadius: 4 }}>
                  {filtroProductoNombre}
                </span>
                <button className="btn btn-outline" style={{ padding: "2px 6px", fontSize: 10 }}
                  onClick={() => { setFiltroProductoId(undefined); setFiltroProductoNombre(""); }}>
                  X
                </button>
              </div>
            ) : (
              <span className="text-secondary" style={{ fontSize: 11 }}>Click en un producto para filtrar</span>
            )}
          </div>
        </div>

        {/* Tabla de movimientos */}
        <div className="card">
          <div className="card-header flex justify-between items-center">
            <span>Movimientos de Inventario</span>
            <span className="text-secondary" style={{ fontSize: 12 }}>{movimientos.length} registro{movimientos.length !== 1 ? "s" : ""}</span>
          </div>
          <div style={{ maxHeight: 500, overflow: "auto" }}>
            <table className="table">
              <thead>
                <tr>
                  <th>Fecha</th>
                  <th>Producto</th>
                  <th>Tipo</th>
                  <th className="text-right">Cantidad</th>
                  <th className="text-right">Stock Ant.</th>
                  <th className="text-right">Stock Nuevo</th>
                  <th>Motivo</th>
                  <th>Usuario</th>
                </tr>
              </thead>
              <tbody>
                {movimientos.map((m) => {
                  const tc = TIPO_COLORES[m.tipo] || { bg: "#f1f5f9", color: "#64748b" };
                  return (
                    <tr key={m.id}>
                      <td className="text-secondary" style={{ fontSize: 12, whiteSpace: "nowrap" }}>
                        {m.created_at ? new Date(m.created_at).toLocaleString("es-EC", {
                          day: "2-digit", month: "2-digit", hour: "2-digit", minute: "2-digit"
                        }) : "-"}
                      </td>
                      <td>
                        <span style={{ cursor: "pointer", color: "#2563eb", fontWeight: 500 }}
                          onClick={() => {
                            setFiltroProductoId(m.producto_id);
                            setFiltroProductoNombre(m.producto_nombre || "");
                          }}>
                          {m.producto_nombre || m.producto_id}
                        </span>
                        {m.producto_codigo && (
                          <div className="text-secondary" style={{ fontSize: 10 }}>{m.producto_codigo}</div>
                        )}
                      </td>
                      <td>
                        <span style={{
                          fontSize: 10, padding: "2px 6px", borderRadius: 3, fontWeight: 600,
                          background: tc.bg, color: tc.color,
                        }}>
                          {m.tipo}
                        </span>
                      </td>
                      <td className="text-right font-bold" style={{
                        color: m.cantidad >= 0 ? "#166534" : "#dc2626",
                      }}>
                        {m.cantidad >= 0 ? "+" : ""}{m.cantidad}
                      </td>
                      <td className="text-right text-secondary">{m.stock_anterior}</td>
                      <td className="text-right font-bold">{m.stock_nuevo}</td>
                      <td className="text-secondary" style={{ fontSize: 12, maxWidth: 200, overflow: "hidden", textOverflow: "ellipsis" }}>
                        {m.motivo || (m.tipo === "VENTA" ? `Venta #${m.referencia_id}` : "-")}
                      </td>
                      <td className="text-secondary" style={{ fontSize: 12 }}>{m.usuario || "-"}</td>
                    </tr>
                  );
                })}
                {movimientos.length === 0 && (
                  <tr>
                    <td colSpan={8} className="text-center text-secondary" style={{ padding: 40 }}>
                      No hay movimientos para este periodo
                    </td>
                  </tr>
                )}
              </tbody>
            </table>
          </div>
        </div>
      </div>

      {/* Modal de Entrada / Ajuste */}
      {modalAbierto && (
        <div style={{
          position: "fixed", inset: 0, background: "rgba(0,0,0,0.4)", display: "flex",
          alignItems: "center", justifyContent: "center", zIndex: 1000,
        }} onClick={() => setModalAbierto(false)}>
          <div className="card" style={{ width: 440, padding: 24 }} onClick={(e) => e.stopPropagation()}>
            <h3 style={{ marginBottom: 16 }}>
              {modalTipo === "ENTRADA" ? "Entrada de Mercaderia" : "Ajuste de Inventario"}
            </h3>

            {/* Buscar producto */}
            <div style={{ marginBottom: 12 }}>
              <label style={{ fontSize: 12, fontWeight: 600, color: "#475569", display: "block", marginBottom: 4 }}>Producto</label>
              {modalProducto ? (
                <div className="flex items-center gap-2">
                  <span style={{ fontSize: 13, fontWeight: 500 }}>{modalProducto.nombre}</span>
                  <span className="text-secondary" style={{ fontSize: 11 }}>
                    (Stock: {modalProducto.stock_actual})
                  </span>
                  <button className="btn btn-outline" style={{ padding: "1px 6px", fontSize: 10 }}
                    onClick={() => { setModalProducto(null); setModalBusqueda(""); }}>Cambiar</button>
                </div>
              ) : (
                <div style={{ position: "relative" }}>
                  <input className="input" style={{ width: "100%", fontSize: 13 }}
                    placeholder="Buscar por nombre o codigo..."
                    value={modalBusqueda}
                    onChange={(e) => handleBuscarProductoModal(e.target.value)}
                    autoFocus />
                  {modalResultados.length > 0 && (
                    <div style={{
                      position: "absolute", top: "100%", left: 0, right: 0,
                      background: "white", border: "1px solid var(--color-border)",
                      borderRadius: 6, maxHeight: 200, overflow: "auto", zIndex: 10,
                      boxShadow: "0 4px 12px rgba(0,0,0,0.1)",
                    }}>
                      {modalResultados.slice(0, 10).map((p) => (
                        <div key={p.id}
                          style={{ padding: "8px 12px", cursor: "pointer", fontSize: 13, borderBottom: "1px solid var(--color-border)" }}
                          onClick={() => {
                            setModalProducto(p);
                            setModalResultados([]);
                            setModalBusqueda(p.nombre);
                            if (modalTipo === "AJUSTE") {
                              setModalCantidad(String(p.stock_actual));
                            }
                          }}
                          onMouseEnter={(e) => (e.currentTarget.style.background = "#f8fafc")}
                          onMouseLeave={(e) => (e.currentTarget.style.background = "white")}>
                          <strong>{p.nombre}</strong>
                          <span className="text-secondary" style={{ marginLeft: 8, fontSize: 11 }}>
                            Stock: {p.stock_actual}
                          </span>
                        </div>
                      ))}
                    </div>
                  )}
                </div>
              )}
            </div>

            {/* Cantidad */}
            <div style={{ marginBottom: 12 }}>
              <label style={{ fontSize: 12, fontWeight: 600, color: "#475569", display: "block", marginBottom: 4 }}>
                {modalTipo === "ENTRADA" ? "Cantidad a ingresar" : "Stock real (conteo fisico)"}
              </label>
              <input className="input" type="number" style={{ width: "100%", fontSize: 13 }}
                placeholder={modalTipo === "ENTRADA" ? "Ej: 50" : "Ej: 23"}
                value={modalCantidad}
                onChange={(e) => setModalCantidad(e.target.value)} />
              {modalTipo === "AJUSTE" && modalProducto && modalCantidad && (
                <div style={{ fontSize: 11, marginTop: 4, color: "#64748b" }}>
                  Diferencia: {(parseFloat(modalCantidad) - modalProducto.stock_actual) >= 0 ? "+" : ""}
                  {(parseFloat(modalCantidad) - modalProducto.stock_actual).toFixed(1)} unidades
                </div>
              )}
            </div>

            {/* Costo (solo para entradas) */}
            {modalTipo === "ENTRADA" && (
              <div style={{ marginBottom: 12 }}>
                <label style={{ fontSize: 12, fontWeight: 600, color: "#475569", display: "block", marginBottom: 4 }}>
                  Costo unitario (opcional)
                </label>
                <input className="input" type="number" step="0.01" style={{ width: "100%", fontSize: 13 }}
                  placeholder="Ej: 1.50"
                  value={modalCosto}
                  onChange={(e) => setModalCosto(e.target.value)} />
              </div>
            )}

            {/* Motivo */}
            <div style={{ marginBottom: 16 }}>
              <label style={{ fontSize: 12, fontWeight: 600, color: "#475569", display: "block", marginBottom: 4 }}>
                Motivo / Observacion
              </label>
              <input className="input" style={{ width: "100%", fontSize: 13 }}
                placeholder={modalTipo === "ENTRADA" ? "Ej: Compra a proveedor" : "Ej: Toma fisica mensual"}
                value={modalMotivo}
                onChange={(e) => setModalMotivo(e.target.value)} />
            </div>

            <div className="flex gap-2 justify-end">
              <button className="btn btn-outline" onClick={() => setModalAbierto(false)}>Cancelar</button>
              <button className="btn btn-primary" onClick={handleRegistrar}>
                {modalTipo === "ENTRADA" ? "Registrar Entrada" : "Aplicar Ajuste"}
              </button>
            </div>
          </div>
        </div>
      )}
    </>
  );
}
