import { useState, useEffect, useCallback } from "react";
import {
  listarMovimientos,
  registrarMovimiento,
  resumenInventario,
  buscarProductos,
  exportarKardexCsv,
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
  ENTRADA: { bg: "rgba(22, 163, 74, 0.15)", color: "var(--color-success, #22c55e)" },
  SALIDA: { bg: "rgba(220, 38, 38, 0.15)", color: "var(--color-danger, #ef4444)" },
  VENTA: { bg: "rgba(59, 130, 246, 0.15)", color: "var(--color-primary, #3b82f6)" },
  AJUSTE: { bg: "rgba(245, 158, 11, 0.15)", color: "var(--color-warning, #f59e0b)" },
  DEVOLUCION: { bg: "rgba(139, 92, 246, 0.15)", color: "#a78bfa" },
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
  // v2.5.25: buscador en kardex (filtra movimientos ya cargados)
  const [busquedaKardex, setBusquedaKardex] = useState("");

  // Modal de movimiento
  const [modalAbierto, setModalAbierto] = useState(false);
  const [modalTipo, setModalTipo] = useState<"ENTRADA" | "AJUSTE">("ENTRADA");
  const [modalProducto, setModalProducto] = useState<ProductoBusqueda | null>(null);
  const [modalBusqueda, setModalBusqueda] = useState("");
  const [modalResultados, setModalResultados] = useState<ProductoBusqueda[]>([]);
  const [modalCantidad, setModalCantidad] = useState("");
  const [modalMotivo, setModalMotivo] = useState("");
  const [modalCosto, setModalCosto] = useState("");
  // v2.6.28 Sprint 4: presentación al ingresar/ajustar inventario
  const [modalPresentaciones, setModalPresentaciones] = useState<{ id?: number; nombre: string; factor: number; activo: boolean }[]>([]);
  const [modalPresentacionId, setModalPresentacionId] = useState<number | null>(null);

  const cargar = useCallback(async () => {
    const [movs, res] = await Promise.all([
      listarMovimientos(filtroProductoId, fechaDesde, fechaHasta, filtroTipo || undefined, 200),
      resumenInventario(),
    ]);
    setMovimientos(movs);
    setResumen(res);
  }, [fechaDesde, fechaHasta, filtroTipo, filtroProductoId]);

  useEffect(() => { cargar(); }, [cargar]);

  // v2.5.32: refrescar kardex automaticamente cuando hay cambios de compras/ventas
  // en otras tabs (anular compra, devolucion, importar XML, NC venta). Antes el
  // usuario tenia que cerrar y abrir la pestaña para ver los nuevos movimientos.
  useEffect(() => {
    const handler = () => { cargar(); };
    window.addEventListener("clouget:compra-cambio", handler);
    window.addEventListener("clouget:venta-completada", handler);
    return () => {
      window.removeEventListener("clouget:compra-cambio", handler);
      window.removeEventListener("clouget:venta-completada", handler);
    };
  }, [cargar]);

  const handleBuscarProductoModal = async (termino: string) => {
    setModalBusqueda(termino);
    if (termino.length >= 1) {
      const res = await buscarProductos(termino);
      setModalResultados(res);
    } else {
      setModalResultados([]);
    }
  };

  const handleExportarKardex = async () => {
    try {
      const csv = await exportarKardexCsv(fechaDesde, fechaHasta, filtroProductoId);
      const blob = new Blob(["\uFEFF" + csv], { type: "text/csv;charset=utf-8" });
      const url = URL.createObjectURL(blob);
      const a = document.createElement("a");
      a.href = url;
      a.download = `kardex_${fechaDesde}_${fechaHasta}.csv`;
      document.body.appendChild(a);
      a.click();
      document.body.removeChild(a);
      URL.revokeObjectURL(url);
      toastExito("Kardex exportado");
    } catch (err) {
      toastError("Error al exportar: " + err);
    }
  };

  const handleRegistrar = async () => {
    if (!modalProducto) return toastError("Seleccione un producto");
    const cantTipeada = parseFloat(modalCantidad);
    if (isNaN(cantTipeada) || cantTipeada <= 0) return toastError("Cantidad invalida");
    // v2.6.28 Sprint 4: si hay presentación, convertir a unidad base.
    const pres = modalPresentacionId != null
      ? modalPresentaciones.find(p => p.id === modalPresentacionId)
      : null;
    const cant = pres ? cantTipeada * pres.factor : cantTipeada;
    const motivoFinal = pres
      ? `${(modalMotivo || "").trim()} [${cantTipeada} × ${pres.nombre}]`.trim()
      : modalMotivo || undefined;

    try {
      await registrarMovimiento(
        modalProducto.id,
        modalTipo,
        cant,
        motivoFinal,
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
      setModalPresentaciones([]);
      setModalPresentacionId(null);
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
              <div className="text-xl font-bold" style={{ color: "var(--color-success, #22c55e)" }}>{resumen.total_entradas_mes}</div>
            </div>
            <div className="card" style={{ padding: 14 }}>
              <div className="text-secondary" style={{ fontSize: 11 }}>Salidas (mes)</div>
              <div className="text-xl font-bold" style={{ color: "var(--color-danger, #ef4444)" }}>{resumen.total_salidas_mes}</div>
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
            {/* v2.5.25: buscador instantaneo sobre movimientos ya cargados */}
            <input className="input" style={{ flex: 1, minWidth: 180, fontSize: 12 }}
              placeholder="🔍 Buscar en movimientos (producto, motivo, usuario)..."
              value={busquedaKardex}
              onChange={(e) => setBusquedaKardex(e.target.value)} />
            {filtroProductoId ? (
              <div className="flex items-center gap-1">
                <span style={{ fontSize: 12, background: "var(--color-surface-alt)", color: "var(--color-text)", padding: "4px 8px", borderRadius: 4 }}>
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
            <button className="btn btn-outline" style={{ fontSize: 11, padding: "6px 12px", marginLeft: "auto" }}
              onClick={handleExportarKardex}>
              Exportar Kardex CSV
            </button>
          </div>
        </div>

        {/* v2.5.25: filtrar movimientos por busqueda (instantaneo, sobre datos ya cargados) */}
        {(() => {
          const q = busquedaKardex.trim().toLowerCase();
          const movimientosFiltrados = !q ? movimientos : movimientos.filter((m: any) =>
            (m.producto_nombre || m.nombre || "").toLowerCase().includes(q) ||
            (m.motivo || "").toLowerCase().includes(q) ||
            (m.usuario || "").toLowerCase().includes(q) ||
            (m.tipo || "").toLowerCase().includes(q)
          );
          return (
        <div className="card">
          <div className="card-header flex justify-between items-center">
            <span>Movimientos de Inventario</span>
            <span className="text-secondary" style={{ fontSize: 12 }}>
              {q
                ? `${movimientosFiltrados.length} de ${movimientos.length} registros`
                : `${movimientos.length} registro${movimientos.length !== 1 ? "s" : ""}`}
            </span>
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
                {movimientosFiltrados.map((m: any) => {
                  const tc = TIPO_COLORES[m.tipo] || { bg: "var(--color-surface-alt)", color: "var(--color-text-secondary)" };
                  return (
                    <tr key={m.id}>
                      <td className="text-secondary" style={{ fontSize: 12, whiteSpace: "nowrap" }}>
                        {m.created_at ? new Date(m.created_at).toLocaleString("es-EC", {
                          day: "2-digit", month: "2-digit", hour: "2-digit", minute: "2-digit"
                        }) : "-"}
                      </td>
                      <td>
                        <span style={{ cursor: "pointer", color: "var(--color-primary, #3b82f6)", fontWeight: 500 }}
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
                          {m.tipo === "GUIA_REMISION" ? "Nota de Entrega" : m.tipo === "AJUSTE_GUIA" ? "Ajuste N. Entrega" : m.tipo}
                        </span>
                      </td>
                      <td className="text-right font-bold" style={{
                        color: m.cantidad >= 0 ? "var(--color-success, #22c55e)" : "var(--color-danger, #ef4444)",
                      }}>
                        {m.cantidad >= 0 ? "+" : ""}{m.cantidad}
                      </td>
                      <td className="text-right text-secondary">{m.stock_anterior}</td>
                      <td className="text-right font-bold">{m.stock_nuevo}</td>
                      <td className="text-secondary" style={{ fontSize: 12, maxWidth: 240, overflow: "hidden", textOverflow: "ellipsis" }} title={m.motivo || ""}>
                        {/* v2.5.28: el backend ahora resuelve el motivo correctamente
                            (Venta NV-XXXX, Compra COMP-XXXX, etc.) via LEFT JOIN.
                            Si aún así está vacío (movimientos manuales sin motivo) mostrar "-" */}
                        {m.motivo || "-"}
                      </td>
                      <td className="text-secondary" style={{ fontSize: 12 }}>{m.usuario || "-"}</td>
                    </tr>
                  );
                })}
                {movimientosFiltrados.length === 0 && (
                  <tr>
                    <td colSpan={8} className="text-center text-secondary" style={{ padding: 40 }}>
                      {q ? `Sin resultados para "${busquedaKardex}"` : "No hay movimientos para este periodo"}
                    </td>
                  </tr>
                )}
              </tbody>
            </table>
          </div>
        </div>
          );
        })()}
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
              <label style={{ fontSize: 12, fontWeight: 600, color: "var(--color-text-secondary)", display: "block", marginBottom: 4 }}>Producto</label>
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
                      background: "var(--color-surface)", border: "1px solid var(--color-border)",
                      borderRadius: 6, maxHeight: 200, overflow: "auto", zIndex: 10,
                      boxShadow: "0 4px 12px rgba(0,0,0,0.3)",
                    }}>
                      {modalResultados.slice(0, 10).map((p) => (
                        <div key={p.id}
                          style={{ padding: "8px 12px", cursor: "pointer", fontSize: 13, borderBottom: "1px solid var(--color-border)" }}
                          onClick={async () => {
                            setModalProducto(p);
                            setModalResultados([]);
                            setModalBusqueda(p.nombre);
                            setModalPresentacionId(null);
                            if (modalTipo === "AJUSTE") {
                              setModalCantidad(String(p.stock_actual));
                            }
                            // v2.6.28/29: cargar unidades_producto Y presentaciones,
                            // combinar para no tener sistemas paralelos. Excluye la
                            // unidad base (factor=1, es_base=1).
                            try {
                              const api = await import("../services/api");
                              const [pres, unis] = await Promise.all([
                                api.listarPresentacionesProducto(p.id).catch(() => []),
                                api.listarUnidadesProducto(p.id).catch(() => []),
                              ]);
                              const unisNorm = (unis as any[])
                                .filter(u => !u.es_base && (u.activa === 1 || u.activa === true))
                                .map((u: any) => ({ id: u.id, nombre: u.nombre, factor: u.factor, activo: true }));
                              const presNorm = (pres as any[])
                                .filter((r: any) => r.activo)
                                .map((r: any) => ({ id: r.id, nombre: r.nombre, factor: r.factor, activo: true }));
                              const seen = new Set(unisNorm.map(u => u.nombre.toLowerCase().trim()));
                              const merged = [
                                ...unisNorm,
                                ...presNorm.filter((q: any) => !seen.has(q.nombre.toLowerCase().trim())),
                              ];
                              setModalPresentaciones(merged);
                            } catch {
                              setModalPresentaciones([]);
                            }
                          }}
                          onMouseEnter={(e) => (e.currentTarget.style.background = "var(--color-surface-hover)")}
                          onMouseLeave={(e) => (e.currentTarget.style.background = "transparent")}>
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

            {/* v2.6.28 Sprint 4: Dropdown de presentacion si el producto tiene */}
            {modalPresentaciones.length > 0 && (
              <div style={{ marginBottom: 10 }}>
                <label style={{ fontSize: 12, fontWeight: 600, color: "var(--color-text-secondary)", display: "block", marginBottom: 4 }}>
                  Expresar en
                </label>
                <select
                  className="input"
                  style={{ width: "100%", fontSize: 13 }}
                  value={modalPresentacionId ?? ""}
                  onChange={(e) => {
                    const v = e.target.value;
                    if (v === "") {
                      setModalPresentacionId(null);
                      if (modalTipo === "AJUSTE" && modalProducto) {
                        setModalCantidad(String(modalProducto.stock_actual));
                      } else {
                        setModalCantidad("");
                      }
                    } else {
                      setModalPresentacionId(parseInt(v, 10));
                      setModalCantidad("");
                    }
                  }}
                >
                  <option value="">Unidad base</option>
                  {modalPresentaciones.map((p) => (
                    <option key={p.id} value={p.id}>
                      {p.nombre} (×{p.factor})
                    </option>
                  ))}
                </select>
              </div>
            )}

            {/* Cantidad */}
            <div style={{ marginBottom: 12 }}>
              <label style={{ fontSize: 12, fontWeight: 600, color: "var(--color-text-secondary)", display: "block", marginBottom: 4 }}>
                {(() => {
                  const pres = modalPresentacionId != null ? modalPresentaciones.find(p => p.id === modalPresentacionId) : null;
                  if (modalTipo === "ENTRADA") {
                    return pres ? `Cantidad a ingresar (en ${pres.nombre})` : "Cantidad a ingresar";
                  }
                  return pres ? `Stock real (en ${pres.nombre})` : "Stock real (conteo fisico)";
                })()}
              </label>
              <input className="input" type="number" style={{ width: "100%", fontSize: 13 }}
                placeholder={modalTipo === "ENTRADA" ? "Ej: 50" : "Ej: 23"}
                value={modalCantidad}
                onChange={(e) => setModalCantidad(e.target.value)} />
              {/* Hint visual: si hay presentacion, mostrar conversion */}
              {modalPresentacionId != null && modalCantidad && (() => {
                const pres = modalPresentaciones.find(p => p.id === modalPresentacionId);
                if (!pres) return null;
                const total = parseFloat(modalCantidad) * pres.factor;
                return (
                  <div style={{ fontSize: 11, marginTop: 4, color: "var(--color-success)" }}>
                    = {total.toFixed(2)} unidades base ({pres.factor} por cada {pres.nombre})
                  </div>
                );
              })()}
              {modalTipo === "AJUSTE" && modalProducto && modalCantidad && (() => {
                const pres = modalPresentacionId != null ? modalPresentaciones.find(p => p.id === modalPresentacionId) : null;
                const tipeada = parseFloat(modalCantidad);
                const stockNuevo = pres ? tipeada * pres.factor : tipeada;
                const diff = stockNuevo - modalProducto.stock_actual;
                return (
                  <div style={{ fontSize: 11, marginTop: 4, color: diff >= 0 ? "var(--color-success)" : "var(--color-danger)" }}>
                    Diferencia: {diff >= 0 ? "+" : ""}{diff.toFixed(1)} unidades base
                  </div>
                );
              })()}
              {modalTipo === "AJUSTE" && modalProducto && !modalCantidad && (
                <div style={{ fontSize: 11, marginTop: 4, color: "var(--color-text-secondary)" }}>
                  Stock actual: {modalProducto.stock_actual} unidades base
                </div>
              )}
            </div>

            {/* Costo (solo para entradas) */}
            {modalTipo === "ENTRADA" && (
              <div style={{ marginBottom: 12 }}>
                <label style={{ fontSize: 12, fontWeight: 600, color: "var(--color-text-secondary)", display: "block", marginBottom: 4 }}>
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
              <label style={{ fontSize: 12, fontWeight: 600, color: "var(--color-text-secondary)", display: "block", marginBottom: 4 }}>
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
