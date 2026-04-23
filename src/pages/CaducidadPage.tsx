import { useState, useEffect, useMemo } from "react";
import { alertasCaducidad, listarTodosLotes, eliminarLoteCaducidad, ajustarCantidadLote } from "../services/api";
import { useToast } from "../components/Toast";

interface Lote {
  id: number;
  lote: string | null;
  fecha_caducidad: string;
  fecha_elaboracion?: string | null;
  fecha_ingreso?: string;
  cantidad: number;
  cantidad_inicial?: number;
  producto_id: number;
  producto_nombre: string;
  producto_codigo: string | null;
  producto_unidad?: string | null;
  estado: "VENCIDO" | "POR_VENCER" | "OK";
  dias_restantes: number;
  observacion?: string | null;
  compra_id?: number | null;
}

type Vista = "alertas" | "todos";
type Filtro = "TODOS" | "VENCIDO" | "POR_VENCER" | "OK";

export default function CaducidadPage() {
  const { toastExito, toastError } = useToast();
  const [vista, setVista] = useState<Vista>("alertas");
  const [lotes, setLotes] = useState<Lote[]>([]);
  const [resumen, setResumen] = useState({ vencidos: 0, por_vencer: 0, ok: 0, total_unidades: 0, dias_alerta: 7 });
  const [filtro, setFiltro] = useState<Filtro>("TODOS");
  const [busqueda, setBusqueda] = useState("");
  const [editandoCantidad, setEditandoCantidad] = useState<number | null>(null);
  const [nuevaCantidad, setNuevaCantidad] = useState("");

  const cargar = async () => {
    try {
      if (vista === "alertas") {
        const r: any = await alertasCaducidad();
        setLotes(r.lotes);
        setResumen({ vencidos: r.vencidos, por_vencer: r.por_vencer, ok: 0, total_unidades: 0, dias_alerta: r.dias_alerta });
      } else {
        const r: any = await listarTodosLotes(filtro, busqueda || undefined, false);
        setLotes(r.lotes);
        setResumen({ vencidos: r.vencidos, por_vencer: r.por_vencer, ok: r.ok, total_unidades: r.total_unidades, dias_alerta: r.dias_alerta });
      }
    } catch (err) { toastError("Error: " + err); }
  };

  // Recarga cuando cambia vista o filtros (en vista "todos")
  useEffect(() => { cargar(); /* eslint-disable-next-line react-hooks/exhaustive-deps */ }, [vista, filtro, busqueda]);

  const lotesFiltrados = useMemo(() => {
    // En vista "alertas" el backend ya filtra; aplicamos filtro local solo si vista=alertas
    if (vista !== "alertas") return lotes;
    return filtro === "TODOS" ? lotes : lotes.filter(l => l.estado === filtro);
  }, [lotes, filtro, vista]);

  const handleEliminar = async (loteId: number, nombre: string) => {
    if (!confirm(`¿Eliminar este lote de "${nombre}"?`)) return;
    try {
      await eliminarLoteCaducidad(loteId);
      toastExito("Lote eliminado");
      cargar();
    } catch (err) { toastError("Error: " + err); }
  };

  const handleAjustar = async (loteId: number) => {
    const cant = parseFloat(nuevaCantidad);
    if (isNaN(cant) || cant < 0) { toastError("Cantidad inválida"); return; }
    try {
      await ajustarCantidadLote(loteId, cant);
      toastExito("Cantidad ajustada");
      setEditandoCantidad(null);
      setNuevaCantidad("");
      cargar();
    } catch (err) { toastError("Error: " + err); }
  };

  const exportarCsv = () => {
    try {
      const headers = ["Producto", "Codigo", "Lote", "Cantidad", "Unidad", "Elaboracion", "Caducidad", "Dias restantes", "Estado", "Ingreso", "Observacion"];
      const rows = lotesFiltrados.map(l => [
        l.producto_nombre,
        l.producto_codigo || "",
        l.lote || "",
        l.cantidad,
        l.producto_unidad || "",
        l.fecha_elaboracion || "",
        l.fecha_caducidad,
        l.dias_restantes,
        l.estado,
        l.fecha_ingreso || "",
        (l.observacion || "").replace(/[\n\r,]/g, " "),
      ]);
      const csv = [headers, ...rows].map(r => r.map(c => {
        const s = String(c);
        return s.includes(",") || s.includes('"') || s.includes("\n") ? `"${s.replace(/"/g, '""')}"` : s;
      }).join(",")).join("\n");
      // BOM para que Excel reconozca UTF-8
      const blob = new Blob(["\uFEFF" + csv], { type: "text/csv;charset=utf-8;" });
      const url = URL.createObjectURL(blob);
      const a = document.createElement("a");
      a.href = url;
      a.download = `caducidad_${new Date().toISOString().slice(0, 10)}.csv`;
      document.body.appendChild(a);
      a.click();
      document.body.removeChild(a);
      URL.revokeObjectURL(url);
      toastExito(`Exportados ${lotesFiltrados.length} lotes`);
    } catch (err) { toastError("Error: " + err); }
  };

  const sinResultados = lotesFiltrados.length === 0;
  const totalUnidades = useMemo(() => lotesFiltrados.reduce((s, l) => s + l.cantidad, 0), [lotesFiltrados]);

  return (
    <>
      <div className="page-header">
        <h2>Control de Caducidad</h2>
        <div style={{ display: "flex", gap: 8 }}>
          <button className="btn btn-outline" onClick={exportarCsv} style={{ fontSize: 12 }}
            disabled={sinResultados}
            title={sinResultados ? "Sin lotes para exportar" : "Exportar a CSV"}>
            📥 Exportar CSV
          </button>
          <button className="btn btn-outline" onClick={cargar} style={{ fontSize: 12 }}>Actualizar</button>
        </div>
      </div>
      <div className="page-body">
        {/* Tabs Alertas / Todos */}
        <div style={{ display: "flex", gap: 6, marginBottom: 12 }}>
          <button className={`btn ${vista === "alertas" ? "btn-primary" : "btn-outline"}`}
            style={{ fontSize: 12, padding: "4px 12px" }}
            onClick={() => { setVista("alertas"); setFiltro("TODOS"); }}>
            🔔 Alertas (próximos {resumen.dias_alerta} días + vencidos)
          </button>
          <button className={`btn ${vista === "todos" ? "btn-primary" : "btn-outline"}`}
            style={{ fontSize: 12, padding: "4px 12px" }}
            onClick={() => { setVista("todos"); setFiltro("TODOS"); }}>
            📋 Todos los lotes
          </button>
        </div>

        {/* Resumen */}
        <div style={{ display: "grid", gridTemplateColumns: vista === "todos" ? "repeat(5, 1fr)" : "repeat(3, 1fr)", gap: 12, marginBottom: 16, maxWidth: vista === "todos" ? 1100 : 700 }}>
          <div className="card" style={{ borderLeft: "4px solid var(--color-danger)" }}>
            <div className="card-body" style={{ padding: 16 }}>
              <div className="text-secondary" style={{ fontSize: 12 }}>Vencidos</div>
              <div style={{ fontSize: 28, fontWeight: 700, color: "var(--color-danger)" }}>{resumen.vencidos}</div>
            </div>
          </div>
          <div className="card" style={{ borderLeft: "4px solid var(--color-warning)" }}>
            <div className="card-body" style={{ padding: 16 }}>
              <div className="text-secondary" style={{ fontSize: 12 }}>Por vencer</div>
              <div style={{ fontSize: 28, fontWeight: 700, color: "var(--color-warning)" }}>{resumen.por_vencer}</div>
            </div>
          </div>
          {vista === "todos" && (
            <div className="card" style={{ borderLeft: "4px solid var(--color-success)" }}>
              <div className="card-body" style={{ padding: 16 }}>
                <div className="text-secondary" style={{ fontSize: 12 }}>OK</div>
                <div style={{ fontSize: 28, fontWeight: 700, color: "var(--color-success)" }}>{resumen.ok}</div>
              </div>
            </div>
          )}
          <div className="card">
            <div className="card-body" style={{ padding: 16 }}>
              <div className="text-secondary" style={{ fontSize: 12 }}>Lotes mostrados</div>
              <div style={{ fontSize: 28, fontWeight: 700 }}>{lotesFiltrados.length}</div>
            </div>
          </div>
          {vista === "todos" && (
            <div className="card">
              <div className="card-body" style={{ padding: 16 }}>
                <div className="text-secondary" style={{ fontSize: 12 }}>Unidades en lotes</div>
                <div style={{ fontSize: 28, fontWeight: 700 }}>{totalUnidades.toFixed(0)}</div>
              </div>
            </div>
          )}
        </div>

        {/* Filtros */}
        <div style={{ display: "flex", gap: 8, marginBottom: 12, alignItems: "center", flexWrap: "wrap" }}>
          <div style={{ display: "flex", gap: 6 }}>
            {(vista === "todos"
              ? ["TODOS", "OK", "POR_VENCER", "VENCIDO"] as const
              : ["TODOS", "VENCIDO", "POR_VENCER"] as const
            ).map(f => (
              <button key={f}
                className={`btn ${filtro === f ? "btn-primary" : "btn-outline"}`}
                style={{ fontSize: 12, padding: "4px 12px" }}
                onClick={() => setFiltro(f)}>
                {f === "TODOS" ? "Todos" : f === "VENCIDO" ? "Vencidos" : f === "POR_VENCER" ? "Por vencer" : "OK"}
              </button>
            ))}
          </div>
          {vista === "todos" && (
            <input
              className="input"
              style={{ flex: 1, minWidth: 200, maxWidth: 360, fontSize: 12 }}
              placeholder="Buscar producto o código..."
              value={busqueda}
              onChange={(e) => setBusqueda(e.target.value)} />
          )}
          {(busqueda || filtro !== "TODOS") && (
            <button className="btn btn-outline" style={{ fontSize: 11 }}
              onClick={() => { setBusqueda(""); setFiltro("TODOS"); }}>
              Limpiar
            </button>
          )}
        </div>

        {/* Tabla de lotes */}
        <div className="card">
          <table className="table">
            <thead>
              <tr>
                <th>Producto</th>
                <th>Lote</th>
                <th className="text-right">Cantidad</th>
                <th>Elaboración</th>
                <th>Caducidad</th>
                <th className="text-right">Días restantes</th>
                <th>Estado</th>
                <th></th>
              </tr>
            </thead>
            <tbody>
              {sinResultados ? (
                <tr><td colSpan={8} className="text-center text-secondary" style={{ padding: 30 }}>
                  {vista === "alertas" ? "No hay lotes en alerta" : (busqueda || filtro !== "TODOS" ? "Sin coincidencias con los filtros" : "No hay lotes registrados")}
                </td></tr>
              ) : lotesFiltrados.map(l => (
                <tr key={l.id}>
                  <td>
                    <strong>{l.producto_nombre}</strong>
                    {l.producto_codigo && <div style={{ fontSize: 11, color: "var(--color-text-secondary)" }}>{l.producto_codigo}</div>}
                  </td>
                  <td>{l.lote || <span className="text-secondary">-</span>}</td>
                  <td className="text-right">
                    {editandoCantidad === l.id ? (
                      <div style={{ display: "flex", gap: 4, justifyContent: "flex-end" }}>
                        <input type="number" className="input" style={{ width: 70 }}
                          value={nuevaCantidad}
                          onChange={(e) => setNuevaCantidad(e.target.value)}
                          autoFocus />
                        <button className="btn btn-primary" style={{ padding: "2px 8px", fontSize: 11 }}
                          onClick={() => handleAjustar(l.id)}>OK</button>
                        <button className="btn btn-outline" style={{ padding: "2px 8px", fontSize: 11 }}
                          onClick={() => setEditandoCantidad(null)}>X</button>
                      </div>
                    ) : (
                      <span style={{ cursor: "pointer" }} title="Click para ajustar"
                        onClick={() => { setEditandoCantidad(l.id); setNuevaCantidad(l.cantidad.toString()); }}>
                        {l.cantidad}{l.producto_unidad ? ` ${l.producto_unidad}` : ""}
                      </span>
                    )}
                  </td>
                  <td style={{ fontSize: 12, color: "var(--color-text-secondary)" }}>{l.fecha_elaboracion || "—"}</td>
                  <td>{l.fecha_caducidad}</td>
                  <td className="text-right" style={{
                    color: l.estado === "VENCIDO" ? "var(--color-danger)" : l.estado === "POR_VENCER" ? "var(--color-warning)" : "var(--color-text-secondary)",
                    fontWeight: 600
                  }}>
                    {l.dias_restantes < 0 ? `-${Math.abs(l.dias_restantes)}` : l.dias_restantes} días
                  </td>
                  <td>
                    <span style={{
                      fontSize: 11, fontWeight: 700, padding: "2px 8px", borderRadius: 4,
                      background: l.estado === "VENCIDO" ? "rgba(239, 68, 68, 0.15)" :
                                  l.estado === "POR_VENCER" ? "rgba(245, 158, 11, 0.15)" :
                                  "rgba(34, 197, 94, 0.15)",
                      color: l.estado === "VENCIDO" ? "var(--color-danger)" :
                             l.estado === "POR_VENCER" ? "var(--color-warning)" :
                             "var(--color-success)",
                    }}>
                      {l.estado === "VENCIDO" ? "Vencido" : l.estado === "POR_VENCER" ? "Por vencer" : "OK"}
                    </span>
                  </td>
                  <td>
                    <button className="btn btn-danger" style={{ fontSize: 11, padding: "2px 8px" }}
                      onClick={() => handleEliminar(l.id, l.producto_nombre)}>
                      Eliminar
                    </button>
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      </div>
    </>
  );
}
