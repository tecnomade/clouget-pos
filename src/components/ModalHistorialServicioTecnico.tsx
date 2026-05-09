/**
 * v2.4.9 — ST-2: Modal de historial filtrable de órdenes de servicio.
 *
 * Filtros: cliente, placa/serie, tipo/marca/modelo, estado, rango de fecha.
 * Tabla scrolleable con resultados + total monto sumado.
 */

import { useEffect, useState } from "react";
import { useToast } from "./Toast";
import {
  stHistorialFiltrable, stListarTiposEquipo, stListarMarcas, stListarModelos,
  type StTipoEquipo, type StMarca, type StModelo, type StFiltrosHistorial,
} from "../services/api";

interface Props {
  onCerrar: () => void;
  onAbrirOrden: (id: number) => void;
}

const ESTADOS = ["", "RECIBIDO", "DIAGNOSTICO", "EN_REPARACION", "LISTO", "ENTREGADO", "CANCELADA"];

export default function ModalHistorialServicioTecnico({ onCerrar, onAbrirOrden }: Props) {
  const { toastError } = useToast();
  const [filtros, setFiltros] = useState<StFiltrosHistorial>({
    busqueda_cliente: "",
    placa: "",
    serie: "",
    estado: "",
    fecha_desde: "",
    fecha_hasta: "",
    limite: 200,
  });
  const [tipos, setTipos] = useState<StTipoEquipo[]>([]);
  const [marcas, setMarcas] = useState<StMarca[]>([]);
  const [modelos, setModelos] = useState<StModelo[]>([]);
  const [resultado, setResultado] = useState<{ ordenes: any[]; total: number; total_monto: number } | null>(null);
  const [cargando, setCargando] = useState(false);

  useEffect(() => {
    stListarTiposEquipo().then(setTipos).catch(() => {});
    aplicar();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  // Cuando cambia tipo, cargar marcas
  useEffect(() => {
    if (filtros.tipo_equipo_id) {
      stListarMarcas(filtros.tipo_equipo_id).then(setMarcas).catch(() => setMarcas([]));
    } else {
      setMarcas([]); setModelos([]);
    }
    setFiltros(f => ({ ...f, marca_id: null, modelo_id: null }));
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [filtros.tipo_equipo_id]);

  useEffect(() => {
    if (filtros.marca_id) {
      stListarModelos(filtros.marca_id).then(setModelos).catch(() => setModelos([]));
    } else {
      setModelos([]);
    }
    setFiltros(f => ({ ...f, modelo_id: null }));
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [filtros.marca_id]);

  const aplicar = async () => {
    setCargando(true);
    try {
      const r = await stHistorialFiltrable(filtros);
      setResultado(r);
    } catch (err: any) {
      toastError("Error: " + (err?.message || err));
    }
    setCargando(false);
  };

  const limpiar = () => {
    setFiltros({ busqueda_cliente: "", placa: "", serie: "", estado: "", fecha_desde: "", fecha_hasta: "", limite: 200 });
    setTimeout(aplicar, 0);
  };

  return (
    <div className="modal-overlay" onClick={onCerrar} style={{ zIndex: 100 }}>
      <div
        className="modal-content"
        onClick={e => e.stopPropagation()}
        style={{ maxWidth: 1100, width: "100%", maxHeight: "92vh", display: "flex", flexDirection: "column" }}
      >
        <div className="modal-header" style={{ display: "flex", justifyContent: "space-between", alignItems: "center", padding: "14px 18px", borderBottom: "1px solid var(--color-border)" }}>
          <div>
            <h2 style={{ margin: 0, fontSize: 18 }}>📜 Historial de órdenes</h2>
            <div style={{ fontSize: 12, color: "var(--color-text-secondary)", marginTop: 2 }}>
              Búsqueda y filtrado completo de órdenes de servicio (incluye canceladas)
            </div>
          </div>
          <button onClick={onCerrar} style={{ background: "transparent", border: "none", fontSize: 24, cursor: "pointer", color: "var(--color-text-muted)" }}>×</button>
        </div>

        <div className="modal-body" style={{ flex: 1, overflowY: "auto", padding: 16 }}>
          {/* Filtros */}
          <div style={{
            background: "var(--color-surface)", border: "1px solid var(--color-border)",
            borderRadius: 8, padding: 12, marginBottom: 12,
            display: "grid", gridTemplateColumns: "repeat(auto-fit, minmax(160px, 1fr))", gap: 10,
          }}>
            <div>
              <label style={{ fontSize: 10, fontWeight: 700, textTransform: "uppercase", color: "var(--color-text-muted)" }}>Cliente</label>
              <input className="input" placeholder="Nombre o cédula"
                value={filtros.busqueda_cliente || ""}
                onChange={e => setFiltros({ ...filtros, busqueda_cliente: e.target.value })}
                style={{ fontSize: 12 }} />
            </div>
            <div>
              <label style={{ fontSize: 10, fontWeight: 700, textTransform: "uppercase", color: "var(--color-text-muted)" }}>Placa</label>
              <input className="input" value={filtros.placa || ""}
                onChange={e => setFiltros({ ...filtros, placa: e.target.value })} style={{ fontSize: 12 }} />
            </div>
            <div>
              <label style={{ fontSize: 10, fontWeight: 700, textTransform: "uppercase", color: "var(--color-text-muted)" }}>Serie</label>
              <input className="input" value={filtros.serie || ""}
                onChange={e => setFiltros({ ...filtros, serie: e.target.value })} style={{ fontSize: 12 }} />
            </div>
            <div>
              <label style={{ fontSize: 10, fontWeight: 700, textTransform: "uppercase", color: "var(--color-text-muted)" }}>Tipo</label>
              <select className="input" value={filtros.tipo_equipo_id || ""}
                onChange={e => setFiltros({ ...filtros, tipo_equipo_id: e.target.value ? parseInt(e.target.value) : null })}
                style={{ fontSize: 12 }}>
                <option value="">— Todos —</option>
                {tipos.map(t => <option key={t.id} value={t.id}>{t.icono} {t.nombre}</option>)}
              </select>
            </div>
            <div>
              <label style={{ fontSize: 10, fontWeight: 700, textTransform: "uppercase", color: "var(--color-text-muted)" }}>Marca</label>
              <select className="input" value={filtros.marca_id || ""}
                onChange={e => setFiltros({ ...filtros, marca_id: e.target.value ? parseInt(e.target.value) : null })}
                disabled={!filtros.tipo_equipo_id}
                style={{ fontSize: 12 }}>
                <option value="">— Todas —</option>
                {marcas.map(m => <option key={m.id} value={m.id}>{m.nombre}</option>)}
              </select>
            </div>
            <div>
              <label style={{ fontSize: 10, fontWeight: 700, textTransform: "uppercase", color: "var(--color-text-muted)" }}>Modelo</label>
              <select className="input" value={filtros.modelo_id || ""}
                onChange={e => setFiltros({ ...filtros, modelo_id: e.target.value ? parseInt(e.target.value) : null })}
                disabled={!filtros.marca_id}
                style={{ fontSize: 12 }}>
                <option value="">— Todos —</option>
                {modelos.map(m => <option key={m.id} value={m.id}>{m.nombre}</option>)}
              </select>
            </div>
            <div>
              <label style={{ fontSize: 10, fontWeight: 700, textTransform: "uppercase", color: "var(--color-text-muted)" }}>Estado</label>
              <select className="input" value={filtros.estado || ""}
                onChange={e => setFiltros({ ...filtros, estado: e.target.value })}
                style={{ fontSize: 12 }}>
                {ESTADOS.map(e => <option key={e} value={e}>{e || "— Todos —"}</option>)}
              </select>
            </div>
            <div>
              <label style={{ fontSize: 10, fontWeight: 700, textTransform: "uppercase", color: "var(--color-text-muted)" }}>Desde</label>
              <input className="input" type="date" value={filtros.fecha_desde || ""}
                onChange={e => setFiltros({ ...filtros, fecha_desde: e.target.value })} style={{ fontSize: 12 }} />
            </div>
            <div>
              <label style={{ fontSize: 10, fontWeight: 700, textTransform: "uppercase", color: "var(--color-text-muted)" }}>Hasta</label>
              <input className="input" type="date" value={filtros.fecha_hasta || ""}
                onChange={e => setFiltros({ ...filtros, fecha_hasta: e.target.value })} style={{ fontSize: 12 }} />
            </div>
            <div style={{ display: "flex", gap: 6, alignItems: "flex-end" }}>
              <button className="btn btn-primary" onClick={aplicar} disabled={cargando} style={{ flex: 1, fontSize: 12 }}>
                {cargando ? "..." : "🔍 Filtrar"}
              </button>
              <button className="btn btn-outline" onClick={limpiar} style={{ fontSize: 12 }}>Limpiar</button>
            </div>
          </div>

          {/* Resumen */}
          {resultado && (
            <div style={{ display: "flex", gap: 12, marginBottom: 8, fontSize: 13 }}>
              <div><strong>{resultado.total}</strong> órdenes</div>
              <div style={{ color: "var(--color-success)" }}>Total: <strong>${resultado.total_monto.toFixed(2)}</strong></div>
            </div>
          )}

          {/* Tabla */}
          <div style={{ overflowX: "auto", border: "1px solid var(--color-border)", borderRadius: 8 }}>
            <table style={{ width: "100%", borderCollapse: "collapse", fontSize: 12 }}>
              <thead>
                <tr style={{ background: "var(--color-surface-alt)" }}>
                  <th style={th}>Número</th>
                  <th style={th}>Fecha</th>
                  <th style={th}>Cliente</th>
                  <th style={th}>Equipo</th>
                  <th style={th}>Placa/Serie</th>
                  <th style={th}>Estado</th>
                  <th style={{ ...th, textAlign: "right" }}>Monto</th>
                </tr>
              </thead>
              <tbody>
                {!resultado ? (
                  <tr><td colSpan={7} style={{ padding: 30, textAlign: "center", color: "var(--color-text-muted)" }}>Aplicá filtros para buscar</td></tr>
                ) : resultado.ordenes.length === 0 ? (
                  <tr><td colSpan={7} style={{ padding: 30, textAlign: "center", color: "var(--color-text-muted)" }}>Sin resultados</td></tr>
                ) : (
                  resultado.ordenes.map(o => (
                    <tr key={o.id} onClick={() => onAbrirOrden(o.id)}
                        style={{ borderTop: "1px solid var(--color-border)", cursor: "pointer" }}>
                      <td style={td}><strong>{o.numero}</strong></td>
                      <td style={td}>{(o.fecha_ingreso || "").slice(0, 10)}</td>
                      <td style={td}>
                        {o.cliente_nombre || "—"}
                        {o.cliente_identificacion && <div style={{ fontSize: 10, color: "var(--color-text-muted)" }}>{o.cliente_identificacion}</div>}
                      </td>
                      <td style={td}>
                        {o.equipo_descripcion}
                        {(o.equipo_marca || o.equipo_modelo) && (
                          <div style={{ fontSize: 10, color: "var(--color-text-muted)" }}>
                            {[o.equipo_marca, o.equipo_modelo].filter(Boolean).join(" / ")}
                          </div>
                        )}
                      </td>
                      <td style={td}>
                        {o.equipo_placa && <div>🚗 {o.equipo_placa}</div>}
                        {o.equipo_serie && <div style={{ fontSize: 10 }}>S/N: {o.equipo_serie}</div>}
                      </td>
                      <td style={td}>
                        <span style={{
                          padding: "2px 8px", borderRadius: 10, fontSize: 10, fontWeight: 700,
                          background: badgeColor(o.estado), color: "#fff",
                        }}>{o.estado}</span>
                      </td>
                      <td style={{ ...td, textAlign: "right", fontWeight: 600 }}>${(o.monto_final || 0).toFixed(2)}</td>
                    </tr>
                  ))
                )}
              </tbody>
            </table>
          </div>
        </div>
      </div>
    </div>
  );
}

const th: React.CSSProperties = { padding: "8px 10px", textAlign: "left", fontWeight: 700, fontSize: 11, textTransform: "uppercase", letterSpacing: 0.4 };
const td: React.CSSProperties = { padding: "8px 10px", verticalAlign: "top" };

function badgeColor(estado: string): string {
  switch (estado) {
    case "RECIBIDO": return "#64748b";
    case "DIAGNOSTICO": return "#3b82f6";
    case "EN_REPARACION": return "#f59e0b";
    case "LISTO": return "#10b981";
    case "ENTREGADO": return "#16a34a";
    case "CANCELADA": return "#ef4444";
    default: return "#94a3b8";
  }
}
