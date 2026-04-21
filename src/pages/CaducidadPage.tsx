import { useState, useEffect } from "react";
import { alertasCaducidad, eliminarLoteCaducidad, ajustarCantidadLote } from "../services/api";
import { useToast } from "../components/Toast";

interface Lote {
  id: number;
  lote: string | null;
  fecha_caducidad: string;
  fecha_elaboracion?: string | null;
  cantidad: number;
  producto_id: number;
  producto_nombre: string;
  producto_codigo: string | null;
  estado: "VENCIDO" | "POR_VENCER" | "OK";
  dias_restantes: number;
}

export default function CaducidadPage() {
  const { toastExito, toastError } = useToast();
  const [lotes, setLotes] = useState<Lote[]>([]);
  const [vencidos, setVencidos] = useState(0);
  const [porVencer, setPorVencer] = useState(0);
  const [filtro, setFiltro] = useState<"TODOS" | "VENCIDO" | "POR_VENCER">("TODOS");
  const [editandoCantidad, setEditandoCantidad] = useState<number | null>(null);
  const [nuevaCantidad, setNuevaCantidad] = useState("");

  const cargar = async () => {
    try {
      const r = await alertasCaducidad();
      setLotes(r.lotes);
      setVencidos(r.vencidos);
      setPorVencer(r.por_vencer);
    } catch (err) { toastError("Error: " + err); }
  };

  useEffect(() => { cargar(); }, []);

  const lotesFiltrados = filtro === "TODOS" ? lotes : lotes.filter(l => l.estado === filtro);

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

  return (
    <>
      <div className="page-header">
        <h2>Control de Caducidad</h2>
        <button className="btn btn-outline" onClick={cargar} style={{ fontSize: 12 }}>Actualizar</button>
      </div>
      <div className="page-body">
        {/* Resumen */}
        <div style={{ display: "grid", gridTemplateColumns: "repeat(3, 1fr)", gap: 12, marginBottom: 16, maxWidth: 700 }}>
          <div className="card" style={{ borderLeft: "4px solid var(--color-danger)" }}>
            <div className="card-body" style={{ padding: 16 }}>
              <div className="text-secondary" style={{ fontSize: 12 }}>Vencidos</div>
              <div style={{ fontSize: 28, fontWeight: 700, color: "var(--color-danger)" }}>{vencidos}</div>
            </div>
          </div>
          <div className="card" style={{ borderLeft: "4px solid var(--color-warning)" }}>
            <div className="card-body" style={{ padding: 16 }}>
              <div className="text-secondary" style={{ fontSize: 12 }}>Por vencer</div>
              <div style={{ fontSize: 28, fontWeight: 700, color: "var(--color-warning)" }}>{porVencer}</div>
            </div>
          </div>
          <div className="card">
            <div className="card-body" style={{ padding: 16 }}>
              <div className="text-secondary" style={{ fontSize: 12 }}>Total alertas</div>
              <div style={{ fontSize: 28, fontWeight: 700 }}>{lotes.length}</div>
            </div>
          </div>
        </div>

        {/* Filtros */}
        <div style={{ display: "flex", gap: 6, marginBottom: 12 }}>
          {(["TODOS", "VENCIDO", "POR_VENCER"] as const).map(f => (
            <button key={f}
              className={`btn ${filtro === f ? "btn-primary" : "btn-outline"}`}
              style={{ fontSize: 12, padding: "4px 12px" }}
              onClick={() => setFiltro(f)}>
              {f === "TODOS" ? "Todos" : f === "VENCIDO" ? "Vencidos" : "Por vencer"}
            </button>
          ))}
        </div>

        {/* Tabla de lotes */}
        <div className="card">
          <table className="table">
            <thead>
              <tr>
                <th>Producto</th>
                <th>Lote</th>
                <th className="text-right">Cantidad</th>
                <th>Elaboracion</th>
                <th>Caducidad</th>
                <th className="text-right">Días restantes</th>
                <th>Estado</th>
                <th></th>
              </tr>
            </thead>
            <tbody>
              {lotesFiltrados.length === 0 ? (
                <tr><td colSpan={8} className="text-center text-secondary" style={{ padding: 30 }}>No hay lotes en alerta</td></tr>
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
                      <span style={{ cursor: "pointer" }} onClick={() => { setEditandoCantidad(l.id); setNuevaCantidad(l.cantidad.toString()); }}>
                        {l.cantidad}
                      </span>
                    )}
                  </td>
                  <td style={{ fontSize: 12, color: "var(--color-text-secondary)" }}>{l.fecha_elaboracion || "—"}</td>
                  <td>{l.fecha_caducidad}</td>
                  <td className="text-right" style={{ color: l.dias_restantes < 0 ? "var(--color-danger)" : "var(--color-warning)", fontWeight: 600 }}>
                    {l.dias_restantes < 0 ? `-${Math.abs(l.dias_restantes)}` : l.dias_restantes} días
                  </td>
                  <td>
                    <span style={{
                      fontSize: 11, fontWeight: 700, padding: "2px 8px", borderRadius: 4,
                      background: l.estado === "VENCIDO" ? "rgba(239, 68, 68, 0.15)" : "rgba(245, 158, 11, 0.15)",
                      color: l.estado === "VENCIDO" ? "var(--color-danger)" : "var(--color-warning)",
                    }}>
                      {l.estado === "VENCIDO" ? "Vencido" : "Por vencer"}
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
