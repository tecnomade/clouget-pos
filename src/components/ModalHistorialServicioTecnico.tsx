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
  obtenerConfig,
  type StTipoEquipo, type StMarca, type StModelo, type StFiltrosHistorial,
} from "../services/api";

interface Props {
  onCerrar: () => void;
  onAbrirOrden: (id: number) => void;
}

// v2.4.12: label adaptable según tipo de negocio
function labelIdentificador(tipoTaller: string): { titulo: string; placeholder: string } {
  const t = (tipoTaller || "").toUpperCase();
  if (t === "AUTOMOTRIZ") return { titulo: "Placa / Chasis", placeholder: "Buscar por placa o chasis" };
  if (t === "ELECTRODOMESTICO" || t === "ELECTRONICO" || t === "COMPUTADORAS")
    return { titulo: "Serie / IMEI", placeholder: "Buscar por número de serie o IMEI" };
  return { titulo: "Placa / Serie", placeholder: "Buscar por placa, serie o descripción" };
}

const ESTADOS = ["", "RECIBIDO", "DIAGNOSTICO", "EN_REPARACION", "LISTO", "ENTREGADO", "CANCELADA"];

export default function ModalHistorialServicioTecnico({ onCerrar, onAbrirOrden }: Props) {
  const { toastError } = useToast();
  const [filtros, setFiltros] = useState<StFiltrosHistorial>({
    busqueda_cliente: "",
    identificador_equipo: "",
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
  // v2.4.12: tipo de taller para adaptar labels
  const [tipoTaller, setTipoTaller] = useState("MIXTO");
  // v2.4.12: filas expandidas (set de ids)
  const [expandidas, setExpandidas] = useState<Set<number>>(new Set());

  const toggleExpandida = (id: number) => {
    setExpandidas(prev => {
      const next = new Set(prev);
      if (next.has(id)) next.delete(id); else next.add(id);
      return next;
    });
  };

  useEffect(() => {
    stListarTiposEquipo().then(setTipos).catch(() => {});
    obtenerConfig().then((cfg: any) => setTipoTaller((cfg.tipo_taller || "MIXTO").toUpperCase())).catch(() => {});
    aplicar();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  const labelId = labelIdentificador(tipoTaller);

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
    setFiltros({ busqueda_cliente: "", identificador_equipo: "", estado: "", fecha_desde: "", fecha_hasta: "", limite: 200 });
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
            {/* v2.4.12: campo unificado Placa/Serie con label adaptable según tipo de taller */}
            <div style={{ gridColumn: "span 2" }}>
              <label style={{ fontSize: 10, fontWeight: 700, textTransform: "uppercase", color: "var(--color-text-muted)" }}>{labelId.titulo}</label>
              <input className="input" value={filtros.identificador_equipo || ""}
                placeholder={labelId.placeholder}
                onChange={e => setFiltros({ ...filtros, identificador_equipo: e.target.value })}
                onKeyDown={e => { if (e.key === "Enter") aplicar(); }}
                style={{ fontSize: 12 }} />
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

          {/* Tabla — v2.4.12: filas expandibles + columna Venta relacionada */}
          <div style={{ overflowX: "auto", border: "1px solid var(--color-border)", borderRadius: 8 }}>
            <table style={{ width: "100%", borderCollapse: "collapse", fontSize: 12 }}>
              <thead>
                <tr style={{ background: "var(--color-surface-alt)" }}>
                  <th style={{ ...th, width: 30 }}></th>
                  <th style={th}>Número</th>
                  <th style={th}>Fecha</th>
                  <th style={th}>Cliente</th>
                  <th style={th}>Equipo</th>
                  <th style={th}>{labelId.titulo}</th>
                  <th style={th}>Estado</th>
                  <th style={th}>Venta</th>
                  <th style={{ ...th, textAlign: "right" }}>Monto</th>
                </tr>
              </thead>
              <tbody>
                {!resultado ? (
                  <tr><td colSpan={9} style={{ padding: 30, textAlign: "center", color: "var(--color-text-muted)" }}>Aplicá filtros para buscar</td></tr>
                ) : resultado.ordenes.length === 0 ? (
                  <tr><td colSpan={9} style={{ padding: 30, textAlign: "center", color: "var(--color-text-muted)" }}>Sin resultados</td></tr>
                ) : (
                  resultado.ordenes.map(o => {
                    const expandido = expandidas.has(o.id);
                    return [
                      <tr key={o.id}
                          style={{ borderTop: "1px solid var(--color-border)", cursor: "pointer" }}>
                        <td style={{ ...td, textAlign: "center" }}
                            onClick={(e) => { e.stopPropagation(); toggleExpandida(o.id); }}>
                          <span style={{
                            display: "inline-block", width: 22, height: 22, borderRadius: 4,
                            background: "var(--color-surface-alt)", border: "1px solid var(--color-border)",
                            fontWeight: 700, fontSize: 11,
                          }}>{expandido ? "▼" : "▶"}</span>
                        </td>
                        <td style={td} onClick={() => onAbrirOrden(o.id)}><strong>{o.numero}</strong></td>
                        <td style={td} onClick={() => onAbrirOrden(o.id)}>{(o.fecha_ingreso || "").slice(0, 10)}</td>
                        <td style={td} onClick={() => onAbrirOrden(o.id)}>
                          {o.cliente_nombre || "—"}
                          {o.cliente_identificacion && <div style={{ fontSize: 10, color: "var(--color-text-muted)" }}>{o.cliente_identificacion}</div>}
                        </td>
                        <td style={td} onClick={() => onAbrirOrden(o.id)}>
                          {o.equipo_descripcion}
                          {(o.equipo_marca || o.equipo_modelo) && (
                            <div style={{ fontSize: 10, color: "var(--color-text-muted)" }}>
                              {[o.equipo_marca, o.equipo_modelo].filter(Boolean).join(" / ")}
                            </div>
                          )}
                        </td>
                        <td style={td} onClick={() => onAbrirOrden(o.id)}>
                          {o.equipo_placa && <div>🚗 {o.equipo_placa}</div>}
                          {o.equipo_serie && <div style={{ fontSize: 10 }}>S/N: {o.equipo_serie}</div>}
                        </td>
                        <td style={td} onClick={() => onAbrirOrden(o.id)}>
                          <span style={{
                            padding: "2px 8px", borderRadius: 10, fontSize: 10, fontWeight: 700,
                            background: badgeColor(o.estado), color: "#fff",
                          }}>{o.estado}</span>
                        </td>
                        <td style={td} onClick={() => onAbrirOrden(o.id)}>
                          {o.venta_id ? (
                            <span style={{
                              padding: "2px 8px", borderRadius: 6, fontSize: 10, fontWeight: 700,
                              background: "var(--color-success)", color: "#fff",
                            }} title={`Venta vinculada #${o.venta_id}`}>📄 #{o.venta_id}</span>
                          ) : (
                            <span style={{ fontSize: 10, color: "var(--color-text-muted)" }}>—</span>
                          )}
                        </td>
                        <td style={{ ...td, textAlign: "right", fontWeight: 600 }} onClick={() => onAbrirOrden(o.id)}>
                          ${(o.monto_final || 0).toFixed(2)}
                        </td>
                      </tr>,
                      expandido ? (
                        <tr key={`${o.id}-exp`} style={{ background: "var(--color-surface-alt)" }}>
                          <td colSpan={9} style={{ padding: "12px 16px" }}>
                            <FilaExpandida orden={o} onAbrir={() => onAbrirOrden(o.id)} />
                          </td>
                        </tr>
                      ) : null,
                    ].filter(Boolean);
                  }).flat()
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

// v2.4.12: vista compacta dentro de fila expandida — muestra problema, diagnóstico,
// trabajo y botón "abrir orden completa". Si tiene venta vinculada, también un
// link visible.
function FilaExpandida({ orden, onAbrir }: { orden: any; onAbrir: () => void }) {
  return (
    <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: 12, fontSize: 12 }}>
      {orden.problema_reportado && (
        <div>
          <div style={{ fontSize: 10, fontWeight: 700, textTransform: "uppercase", color: "var(--color-text-muted)", marginBottom: 4 }}>
            Problema reportado
          </div>
          <div>{orden.problema_reportado}</div>
        </div>
      )}
      {orden.diagnostico && (
        <div>
          <div style={{ fontSize: 10, fontWeight: 700, textTransform: "uppercase", color: "var(--color-text-muted)", marginBottom: 4 }}>
            Diagnóstico
          </div>
          <div>{orden.diagnostico}</div>
        </div>
      )}
      {orden.trabajo_realizado && (
        <div>
          <div style={{ fontSize: 10, fontWeight: 700, textTransform: "uppercase", color: "var(--color-text-muted)", marginBottom: 4 }}>
            Trabajo realizado
          </div>
          <div>{orden.trabajo_realizado}</div>
        </div>
      )}
      {/* v2.4.25: kilometraje (entrada / salida / próximo) si la orden lo registra */}
      {(orden.equipo_kilometraje != null || orden.equipo_kilometraje_salida != null || orden.equipo_kilometraje_proximo != null) && (
        <div style={{ gridColumn: "span 2" }}>
          <div style={{ fontSize: 10, fontWeight: 700, textTransform: "uppercase", color: "var(--color-text-muted)", marginBottom: 4 }}>
            🚗 Kilometraje
          </div>
          <div style={{ display: "flex", gap: 16, flexWrap: "wrap" }}>
            {orden.equipo_kilometraje != null && (
              <span>Entrada: <strong>{orden.equipo_kilometraje} km</strong></span>
            )}
            {orden.equipo_kilometraje_salida != null && (
              <span style={{ color: "var(--color-success)" }}>Salida: <strong>{orden.equipo_kilometraje_salida} km</strong></span>
            )}
            {orden.equipo_kilometraje_proximo != null && (
              <span style={{ color: "var(--color-warning)" }}>Próximo mant.: <strong>{orden.equipo_kilometraje_proximo} km</strong></span>
            )}
            {orden.equipo_kilometraje_intervalo != null && (
              <span style={{ color: "var(--color-text-muted)" }}>(cada {orden.equipo_kilometraje_intervalo} km)</span>
            )}
          </div>
        </div>
      )}
      <div style={{ display: "flex", flexDirection: "column", gap: 6, alignItems: "flex-start", gridColumn: "span 2", borderTop: "1px solid var(--color-border)", paddingTop: 8, marginTop: 4 }}>
        <div style={{ fontSize: 11, color: "var(--color-text-muted)" }}>
          {orden.fecha_entrega && <>Entregado: {orden.fecha_entrega.slice(0, 16).replace("T", " ")}</>}
        </div>
        <div style={{ display: "flex", gap: 6, flexWrap: "wrap" }}>
          <button
            onClick={(e) => { e.stopPropagation(); onAbrir(); }}
            style={{
              padding: "6px 14px", fontSize: 12, fontWeight: 600,
              background: "var(--color-primary)", color: "#fff",
              border: "none", borderRadius: 6, cursor: "pointer",
            }}
          >
            📋 Abrir orden completa
          </button>
          {/* v2.4.25: imprimir directo desde el historial */}
          <button
            onClick={async (e) => {
              e.stopPropagation();
              try {
                const { imprimirOrdenServicioPdf } = await import("../services/api");
                await imprimirOrdenServicioPdf(orden.id, "A4");
              } catch (err) { console.error(err); }
            }}
            style={{
              padding: "6px 12px", fontSize: 11, fontWeight: 600,
              background: "var(--color-surface-alt)", color: "var(--color-text)",
              border: "1px solid var(--color-border)", borderRadius: 6, cursor: "pointer",
            }}
            title="Imprimir comprobante en formato A4"
          >
            🖨 A4
          </button>
          <button
            onClick={async (e) => {
              e.stopPropagation();
              try {
                const { imprimirOrdenServicioPdf } = await import("../services/api");
                await imprimirOrdenServicioPdf(orden.id, "TICKET_80");
              } catch (err) { console.error(err); }
            }}
            style={{
              padding: "6px 12px", fontSize: 11, fontWeight: 600,
              background: "var(--color-surface-alt)", color: "var(--color-text)",
              border: "1px solid var(--color-border)", borderRadius: 6, cursor: "pointer",
            }}
            title="Imprimir comprobante en formato 80mm (térmica)"
          >
            🧾 80mm
          </button>
        </div>
        {orden.venta_id && (
          <div style={{ fontSize: 11, color: "var(--color-success)" }}>
            ✓ Esta orden generó la venta <strong>#{orden.venta_id}</strong>
            <br/>
            <span style={{ color: "var(--color-text-muted)", fontSize: 10 }}>
              Ve a Ventas → busca ese número para ver detalles, imprimir o anular.
            </span>
          </div>
        )}
      </div>
    </div>
  );
}
