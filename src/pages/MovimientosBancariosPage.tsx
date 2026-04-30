import { useState, useEffect, useMemo, Fragment } from "react";
import { listarMovimientosBancarios, listarCuentasBanco, obtenerDetalleMovimientoBancario, verificarTransferencia } from "../services/api";
import { useToast } from "../components/Toast";
import { useSesion } from "../contexts/SesionContext";
import type { CuentaBanco } from "../types";

type Periodo = "hoy" | "7dias" | "mes" | "custom";

function fechaHoy(): string {
  return new Date().toISOString().slice(0, 10);
}

function fechaHace7Dias(): string {
  const d = new Date();
  d.setDate(d.getDate() - 7);
  return d.toISOString().slice(0, 10);
}

function primerDiaMes(): string {
  const d = new Date();
  return `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, "0")}-01`;
}

interface MovimientoBancario {
  tipo: string;
  referencia: string;
  monto: number;
  fecha: string;
  banco_nombre: string | null;
  detalle: string | null;
  origen_id: number;
  pago_estado: string;
  tiene_comprobante: boolean;
}

export default function MovimientosBancariosPage() {
  const { toastError, toastExito } = useToast();
  const { esAdmin } = useSesion();
  const [movimientos, setMovimientos] = useState<MovimientoBancario[]>([]);
  const [cuentasBanco, setCuentasBanco] = useState<CuentaBanco[]>([]);
  const [bancoFiltro, setBancoFiltro] = useState<number | undefined>(undefined);
  const [periodo, setPeriodo] = useState<Periodo>("mes");
  const [fechaDesde, setFechaDesde] = useState(primerDiaMes());
  const [fechaHasta, setFechaHasta] = useState(fechaHoy());
  const [cargando, setCargando] = useState(false);
  const [filtroTipo, setFiltroTipo] = useState<string>(""); // "" = todos
  const [expandido, setExpandido] = useState<string | null>(null); // tipo:origen_id
  const [detalle, setDetalle] = useState<Record<string, any>>({});
  const [comprobanteFs, setComprobanteFs] = useState<string | null>(null);

  useEffect(() => {
    listarCuentasBanco().then(setCuentasBanco).catch(() => {});
  }, []);

  // Update dates when periodo changes
  useEffect(() => {
    if (periodo === "hoy") {
      setFechaDesde(fechaHoy());
      setFechaHasta(fechaHoy());
    } else if (periodo === "7dias") {
      setFechaDesde(fechaHace7Dias());
      setFechaHasta(fechaHoy());
    } else if (periodo === "mes") {
      setFechaDesde(primerDiaMes());
      setFechaHasta(fechaHoy());
    }
  }, [periodo]);

  const cargarMovimientos = async () => {
    setCargando(true);
    try {
      const data = await listarMovimientosBancarios(bancoFiltro, fechaDesde, fechaHasta);
      setMovimientos(data);
    } catch (err) {
      toastError("Error al cargar movimientos: " + err);
    } finally {
      setCargando(false);
    }
  };

  useEffect(() => {
    cargarMovimientos();
  }, [bancoFiltro, fechaDesde, fechaHasta]);

  const { totalIngresos, totalEgresos, saldoNeto } = useMemo(() => {
    let ingresos = 0;
    let egresos = 0;
    for (const m of movimientos) {
      if (m.monto >= 0) ingresos += m.monto;
      else egresos += Math.abs(m.monto);
    }
    return { totalIngresos: ingresos, totalEgresos: egresos, saldoNeto: ingresos - egresos };
  }, [movimientos]);

  const tipoBadge = (tipo: string) => {
    const estilos: Record<string, { bg: string; color: string }> = {
      VENTA: { bg: "rgba(34,197,94,0.15)", color: "var(--color-success)" },
      PAGO_VENTA: { bg: "rgba(34,197,94,0.15)", color: "var(--color-success)" },
      RETIRO_CAJA: { bg: "rgba(251,146,60,0.15)", color: "#fb923c" },
      PAGO_PROVEEDOR: { bg: "rgba(239,68,68,0.15)", color: "var(--color-danger)" },
      COBRO_CREDITO: { bg: "rgba(59,130,246,0.15)", color: "var(--color-primary)" },
    };
    const s = estilos[tipo] || { bg: "rgba(148,163,184,0.15)", color: "var(--color-text-secondary)" };
    const labels: Record<string, string> = {
      VENTA: "Venta",
      PAGO_VENTA: "Venta (mixto)",
      RETIRO_CAJA: "Retiro Caja",
      PAGO_PROVEEDOR: "Pago Proveedor",
      COBRO_CREDITO: "Cobro Credito",
    };
    return (
      <span style={{
        padding: "2px 8px", borderRadius: 4, fontSize: 11, fontWeight: 600,
        background: s.bg, color: s.color,
      }}>
        {labels[tipo] || tipo}
      </span>
    );
  };

  const estadoBadge = (estado: string) => {
    if (!estado || estado === "NO_APLICA") return null;
    const map: Record<string, { bg: string; color: string; label: string; icon: string }> = {
      REGISTRADO: { bg: "rgba(245,158,11,0.15)", color: "var(--color-warning)", label: "Por verificar", icon: "⏱" },
      VERIFICADO: { bg: "rgba(34,197,94,0.15)", color: "var(--color-success)", label: "Verificada", icon: "✓" },
      RECHAZADO: { bg: "rgba(239,68,68,0.15)", color: "var(--color-danger)", label: "Rechazada", icon: "✗" },
    };
    const s = map[estado];
    if (!s) return null;
    return (
      <span style={{
        padding: "1px 6px", borderRadius: 4, fontSize: 10, fontWeight: 600,
        background: s.bg, color: s.color, marginLeft: 4,
      }}>
        {s.icon} {s.label}
      </span>
    );
  };

  const toggleExpandir = async (m: MovimientoBancario) => {
    const key = `${m.tipo}:${m.origen_id}`;
    if (expandido === key) {
      setExpandido(null);
      return;
    }
    setExpandido(key);
    if (!detalle[key]) {
      try {
        const d = await obtenerDetalleMovimientoBancario(m.tipo, m.origen_id);
        setDetalle(prev => ({ ...prev, [key]: d }));
      } catch (err) {
        toastError("Error cargando detalle: " + err);
      }
    }
  };

  const handleVerificar = async (m: MovimientoBancario, aprobar: boolean) => {
    const motivo = aprobar ? undefined : prompt("Motivo del rechazo:") || undefined;
    if (!aprobar && !motivo) return;
    try {
      const origen = m.tipo === "VENTA" ? "VENTA" : "PAGO_MIXTO";
      await verificarTransferencia(origen, m.origen_id, aprobar, motivo);
      toastExito(aprobar ? "Transferencia verificada" : "Transferencia rechazada");
      // Refrescar lista
      cargarMovimientos();
      // Limpiar detalle cacheado para que se refresque al expandir de nuevo
      const key = `${m.tipo}:${m.origen_id}`;
      setDetalle(prev => { const c = { ...prev }; delete c[key]; return c; });
      setExpandido(null);
    } catch (err) {
      toastError("Error: " + err);
    }
  };

  const movimientosFiltrados = filtroTipo
    ? movimientos.filter(m => filtroTipo === "VENTA"
        ? (m.tipo === "VENTA" || m.tipo === "PAGO_VENTA")
        : m.tipo === filtroTipo)
    : movimientos;

  return (
    <>
      <div className="page-header">
        <h2>Movimientos Bancarios</h2>
      </div>
      <div className="page-body">
        {/* Filtros */}
        <div className="flex gap-3 mb-4 items-end" style={{ flexWrap: "wrap" }}>
          <div>
            <label className="text-secondary" style={{ fontSize: 11, display: "block", marginBottom: 4 }}>Periodo</label>
            <select className="input" value={periodo} onChange={(e) => setPeriodo(e.target.value as Periodo)} style={{ minWidth: 140 }}>
              <option value="hoy">Hoy</option>
              <option value="7dias">Ultimos 7 dias</option>
              <option value="mes">Este mes</option>
              <option value="custom">Personalizado</option>
            </select>
          </div>
          {periodo === "custom" && (
            <>
              <div>
                <label className="text-secondary" style={{ fontSize: 11, display: "block", marginBottom: 4 }}>Desde</label>
                <input className="input" type="date" value={fechaDesde} onChange={(e) => setFechaDesde(e.target.value)} />
              </div>
              <div>
                <label className="text-secondary" style={{ fontSize: 11, display: "block", marginBottom: 4 }}>Hasta</label>
                <input className="input" type="date" value={fechaHasta} onChange={(e) => setFechaHasta(e.target.value)} />
              </div>
            </>
          )}
          <div>
            <label className="text-secondary" style={{ fontSize: 11, display: "block", marginBottom: 4 }}>Cuenta</label>
            <select
              className="input"
              value={bancoFiltro ?? ""}
              onChange={(e) => setBancoFiltro(e.target.value ? Number(e.target.value) : undefined)}
              style={{ minWidth: 180 }}
            >
              <option value="">Todas las cuentas</option>
              {cuentasBanco.filter(b => b.activa).map((b) => (
                <option key={b.id} value={b.id}>{b.nombre}</option>
              ))}
            </select>
          </div>
        </div>

        {/* Summary cards */}
        <div className="flex gap-4 mb-4">
          <div className="card" style={{ flex: 1, maxWidth: 220 }}>
            <div className="card-body text-center">
              <span className="text-secondary" style={{ fontSize: 12 }}>Ingresos</span>
              <div className="text-xl font-bold" style={{ color: "var(--color-success)" }}>
                ${totalIngresos.toFixed(2)}
              </div>
            </div>
          </div>
          <div className="card" style={{ flex: 1, maxWidth: 220 }}>
            <div className="card-body text-center">
              <span className="text-secondary" style={{ fontSize: 12 }}>Egresos</span>
              <div className="text-xl font-bold" style={{ color: "var(--color-danger)" }}>
                ${totalEgresos.toFixed(2)}
              </div>
            </div>
          </div>
          <div className="card" style={{ flex: 1, maxWidth: 220 }}>
            <div className="card-body text-center">
              <span className="text-secondary" style={{ fontSize: 12 }}>Saldo neto</span>
              <div className="text-xl font-bold" style={{ color: saldoNeto >= 0 ? "var(--color-success)" : "var(--color-danger)" }}>
                ${saldoNeto.toFixed(2)}
              </div>
            </div>
          </div>
          <div className="card" style={{ flex: 1, maxWidth: 220 }}>
            <div className="card-body text-center">
              <span className="text-secondary" style={{ fontSize: 12 }}>Movimientos</span>
              <div className="text-xl font-bold">{movimientos.length}</div>
            </div>
          </div>
        </div>

        {/* Filtro por tipo */}
        <div style={{ display: "flex", gap: 6, marginBottom: 8, flexWrap: "wrap" }}>
          {[
            { v: "", l: "Todos" },
            { v: "VENTA", l: "Ventas" },
            { v: "RETIRO_CAJA", l: "Retiros caja" },
            { v: "PAGO_PROVEEDOR", l: "Pagos proveedor" },
            { v: "COBRO_CREDITO", l: "Cobros crédito" },
          ].map(opt => (
            <button key={opt.v}
              className={filtroTipo === opt.v ? "btn btn-primary" : "btn btn-outline"}
              style={{ fontSize: 11, padding: "4px 10px" }}
              onClick={() => setFiltroTipo(opt.v)}>
              {opt.l}
            </button>
          ))}
        </div>

        {/* Table */}
        <div className="card">
          <table className="table">
            <thead>
              <tr>
                <th style={{ width: 30 }}></th>
                <th>Fecha</th>
                <th>Tipo</th>
                <th>Referencia</th>
                <th>Detalle</th>
                <th className="text-right">Monto</th>
                <th>Banco</th>
              </tr>
            </thead>
            <tbody>
              {cargando ? (
                <tr>
                  <td colSpan={7} className="text-center text-secondary" style={{ padding: 40 }}>
                    Cargando...
                  </td>
                </tr>
              ) : movimientosFiltrados.length === 0 ? (
                <tr>
                  <td colSpan={7} className="text-center text-secondary" style={{ padding: 40 }}>
                    No hay movimientos bancarios en este periodo
                  </td>
                </tr>
              ) : (
                movimientosFiltrados.map((m, i) => {
                  const key = `${m.tipo}:${m.origen_id}`;
                  const isOpen = expandido === key;
                  const det = detalle[key];
                  return (
                    <Fragment key={i}>
                      <tr style={{ cursor: "pointer" }} onClick={() => toggleExpandir(m)}>
                        <td style={{ textAlign: "center", color: "var(--color-text-secondary)" }}>
                          {isOpen ? "▼" : "▶"}
                        </td>
                        <td style={{ whiteSpace: "nowrap" }}>
                          {m.fecha ? new Date(m.fecha).toLocaleDateString("es-EC", {
                            day: "2-digit", month: "2-digit", year: "numeric",
                            hour: "2-digit", minute: "2-digit",
                          }) : "-"}
                        </td>
                        <td>
                          {tipoBadge(m.tipo)}
                          {estadoBadge(m.pago_estado)}
                          {m.tiene_comprobante && <span title="Tiene comprobante" style={{ marginLeft: 4 }}>📎</span>}
                        </td>
                        <td className="text-secondary">{m.referencia || "-"}</td>
                        <td>{m.detalle || "-"}</td>
                        <td className="text-right font-bold" style={{
                          color: m.monto >= 0 ? "var(--color-success)" : "var(--color-danger)",
                        }}>
                          {m.monto >= 0 ? "+" : ""}${m.monto.toFixed(2)}
                        </td>
                        <td className="text-secondary">{m.banco_nombre || "-"}</td>
                      </tr>
                      {isOpen && (
                        <tr>
                          <td colSpan={7} style={{ background: "var(--color-surface-alt)", padding: "12px 20px" }}>
                            {!det ? (
                              <div className="text-secondary" style={{ fontSize: 12 }}>Cargando detalle...</div>
                            ) : (
                              <DetalleMovimiento
                                tipo={m.tipo}
                                det={det}
                                onVerComprobante={(img) => setComprobanteFs(img)}
                                onVerificar={esAdmin && (m.tipo === "VENTA" || m.tipo === "PAGO_VENTA") && m.pago_estado === "REGISTRADO"
                                  ? (aprobar) => handleVerificar(m, aprobar)
                                  : undefined}
                              />
                            )}
                          </td>
                        </tr>
                      )}
                    </Fragment>
                  );
                })
              )}
            </tbody>
          </table>
        </div>

        {/* Visor fullscreen del comprobante */}
        {comprobanteFs && (
          <div onClick={() => setComprobanteFs(null)} style={{
            position: "fixed", inset: 0, background: "rgba(0,0,0,0.92)",
            zIndex: 250, display: "flex", alignItems: "center", justifyContent: "center",
            cursor: "zoom-out", padding: 20,
          }}>
            <img src={comprobanteFs} alt="Comprobante"
              style={{ maxWidth: "100%", maxHeight: "100%", objectFit: "contain" }} />
            <button onClick={(e) => { e.stopPropagation(); setComprobanteFs(null); }}
              style={{
                position: "fixed", top: 16, right: 16,
                background: "rgba(0,0,0,0.6)", color: "white", border: "1px solid rgba(255,255,255,0.3)",
                borderRadius: 8, padding: "6px 14px", fontSize: 16, cursor: "pointer",
              }}>× Cerrar</button>
          </div>
        )}
      </div>
    </>
  );
}

// === Componente: detalle expandido segun tipo de movimiento ===
function DetalleMovimiento({
  tipo, det, onVerComprobante, onVerificar,
}: {
  tipo: string;
  det: any;
  onVerComprobante: (img: string) => void;
  onVerificar?: (aprobar: boolean) => void;
}) {
  const F = ({ label, value }: { label: string; value: any }) => (
    value === null || value === undefined || value === "" ? null : (
      <div style={{ display: "flex", gap: 8, fontSize: 12, marginBottom: 2 }}>
        <span className="text-secondary" style={{ minWidth: 110 }}>{label}:</span>
        <span style={{ fontWeight: 500 }}>{value}</span>
      </div>
    )
  );

  if (tipo === "VENTA") {
    return (
      <div>
        <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: 12 }}>
          <div>
            <div style={{ fontSize: 11, fontWeight: 700, color: "var(--color-text-secondary)", marginBottom: 4 }}>VENTA</div>
            <F label="Número" value={det.numero} />
            <F label="Fecha" value={det.fecha} />
            <F label="Documento" value={det.tipo_documento} />
            <F label="Cajero" value={det.usuario} />
            <F label="Total" value={`$${det.total?.toFixed(2)}`} />
            <F label="Forma pago" value={det.forma_pago} />
            <F label="Banco" value={det.banco_nombre} />
            <F label="Referencia" value={det.referencia_pago} />
            <F label="Estado SRI" value={det.estado_sri} />
          </div>
          <div>
            <div style={{ fontSize: 11, fontWeight: 700, color: "var(--color-text-secondary)", marginBottom: 4 }}>CLIENTE</div>
            <F label="Nombre" value={det.cliente_nombre} />
            <F label="Cédula/RUC" value={det.cliente_cedula} />
            <F label="Teléfono" value={det.cliente_telefono} />
            <F label="Email" value={det.cliente_email} />
            {det.observacion && (
              <>
                <div style={{ fontSize: 11, fontWeight: 700, color: "var(--color-text-secondary)", marginTop: 8, marginBottom: 2 }}>OBSERVACIÓN</div>
                <div style={{ fontSize: 12 }}>{det.observacion}</div>
              </>
            )}
          </div>
        </div>

        {det.items && det.items.length > 0 && (
          <div style={{ marginTop: 10 }}>
            <div style={{ fontSize: 11, fontWeight: 700, color: "var(--color-text-secondary)", marginBottom: 4 }}>
              ITEMS ({det.items.length})
            </div>
            <table style={{ width: "100%", fontSize: 11 }}>
              <thead>
                <tr style={{ borderBottom: "1px solid var(--color-border)" }}>
                  <th style={{ textAlign: "left", padding: "4px 6px" }}>Producto</th>
                  <th style={{ textAlign: "right", padding: "4px 6px" }}>Cant.</th>
                  <th style={{ textAlign: "right", padding: "4px 6px" }}>P.Unit.</th>
                  <th style={{ textAlign: "right", padding: "4px 6px" }}>Subtotal</th>
                </tr>
              </thead>
              <tbody>
                {det.items.map((it: any, i: number) => (
                  <tr key={i}>
                    <td style={{ padding: "4px 6px" }}>{it.nombre}</td>
                    <td style={{ textAlign: "right", padding: "4px 6px" }}>{it.cantidad}</td>
                    <td style={{ textAlign: "right", padding: "4px 6px" }}>${it.precio_unitario?.toFixed(2)}</td>
                    <td style={{ textAlign: "right", padding: "4px 6px" }}>${it.subtotal?.toFixed(2)}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        )}

        {/* Verificacion */}
        <BloqueVerificacion det={det} onVerificar={onVerificar} />

        {det.comprobante_imagen && (
          <BloqueComprobante src={det.comprobante_imagen} onAmpliar={onVerComprobante} />
        )}
      </div>
    );
  }

  if (tipo === "PAGO_VENTA") {
    return (
      <div>
        <div style={{ fontSize: 11, fontWeight: 700, color: "var(--color-text-secondary)", marginBottom: 4 }}>
          COMPONENTE TRANSFER DE VENTA MIXTA
        </div>
        <F label="Venta Nº" value={det.venta_numero} />
        <F label="Total venta" value={`$${det.venta_total?.toFixed(2)}`} />
        <F label="Monto este pago" value={`$${det.monto?.toFixed(2)}`} />
        <F label="Cliente" value={det.cliente_nombre} />
        <F label="Cajero" value={det.cajero} />
        <F label="Banco" value={det.banco_nombre} />
        <F label="Referencia" value={det.referencia} />

        <BloqueVerificacion det={det} onVerificar={onVerificar} />

        {det.comprobante_imagen && (
          <BloqueComprobante src={det.comprobante_imagen} onAmpliar={onVerComprobante} />
        )}
      </div>
    );
  }

  if (tipo === "RETIRO_CAJA") {
    return (
      <div>
        <div style={{ fontSize: 11, fontWeight: 700, color: "var(--color-text-secondary)", marginBottom: 4 }}>RETIRO DE CAJA</div>
        <F label="Caja Nº" value={`#${det.caja_id}`} />
        <F label="Fecha" value={det.fecha} />
        <F label="Monto" value={`$${det.monto?.toFixed(2)}`} />
        <F label="Usuario" value={det.usuario} />
        <F label="Banco destino" value={det.banco_nombre} />
        <F label="Referencia" value={det.referencia} />
        <F label="Estado" value={det.estado} />
        {det.motivo && (
          <div style={{ marginTop: 8 }}>
            <div className="text-secondary" style={{ fontSize: 11 }}>Motivo:</div>
            <div style={{ fontSize: 12 }}>{det.motivo}</div>
          </div>
        )}
        {det.comprobante_imagen && (
          <BloqueComprobante src={det.comprobante_imagen} onAmpliar={onVerComprobante} />
        )}
      </div>
    );
  }

  if (tipo === "PAGO_PROVEEDOR") {
    return (
      <div>
        <div style={{ fontSize: 11, fontWeight: 700, color: "var(--color-text-secondary)", marginBottom: 4 }}>PAGO A PROVEEDOR</div>
        <F label="Proveedor" value={det.proveedor_nombre} />
        <F label="RUC" value={det.proveedor_ruc} />
        <F label="Teléfono" value={det.proveedor_telefono} />
        <F label="Factura Nº" value={det.factura_numero} />
        <F label="Fecha factura" value={det.fecha_factura} />
        <F label="Total factura" value={det.factura_total ? `$${det.factura_total.toFixed(2)}` : null} />
        <F label="Monto pagado" value={`$${det.monto?.toFixed(2)}`} />
        <F label="Fecha pago" value={det.fecha} />
        <F label="Forma pago" value={det.forma_pago} />
        <F label="Banco origen" value={det.banco_nombre} />
        <F label="Nº comprobante" value={det.numero_comprobante} />
      </div>
    );
  }

  if (tipo === "COBRO_CREDITO") {
    return (
      <div>
        <div style={{ fontSize: 11, fontWeight: 700, color: "var(--color-text-secondary)", marginBottom: 4 }}>COBRO DE CRÉDITO</div>
        <F label="Cliente" value={det.cliente_nombre} />
        <F label="Cédula/RUC" value={det.cliente_cedula} />
        <F label="Teléfono" value={det.cliente_telefono} />
        <F label="Venta original" value={det.venta_numero} />
        <F label="Crédito total" value={det.credito_total ? `$${det.credito_total.toFixed(2)}` : null} />
        <F label="Saldo restante" value={det.credito_saldo != null ? `$${det.credito_saldo.toFixed(2)}` : null} />
        <F label="Monto cobrado" value={`$${det.monto?.toFixed(2)}`} />
        <F label="Fecha cobro" value={det.fecha} />
        <F label="Forma pago" value={det.forma_pago} />
        <F label="Banco" value={det.banco_nombre} />
        <F label="Nº comprobante" value={det.numero_comprobante} />
        {det.observacion && <F label="Observación" value={det.observacion} />}
        {det.comprobante_imagen && (
          <BloqueComprobante src={det.comprobante_imagen} onAmpliar={onVerComprobante} />
        )}
      </div>
    );
  }

  return <div className="text-secondary" style={{ fontSize: 12 }}>Tipo no soportado: {tipo}</div>;
}

function BloqueComprobante({ src, onAmpliar }: { src: string; onAmpliar: (s: string) => void }) {
  return (
    <div style={{ marginTop: 10, padding: 8, background: "var(--color-surface)", borderRadius: 6, border: "1px solid var(--color-border)" }}>
      <div className="text-secondary" style={{ fontSize: 11, marginBottom: 4 }}>📎 Comprobante:</div>
      <img src={src} alt="comprobante"
        onClick={() => onAmpliar(src)}
        style={{ maxWidth: 200, maxHeight: 150, objectFit: "contain", cursor: "zoom-in", borderRadius: 4, border: "1px solid var(--color-border)" }} />
      <div style={{ display: "flex", gap: 6, marginTop: 4 }}>
        <button className="btn btn-outline" style={{ fontSize: 11, padding: "3px 8px" }}
          onClick={() => onAmpliar(src)}>🔍 Ver completo</button>
        <a href={src} download="comprobante.png"
          className="btn btn-outline" style={{ fontSize: 11, padding: "3px 8px", textDecoration: "none" }}>⬇ Descargar</a>
      </div>
    </div>
  );
}

function BloqueVerificacion({ det, onVerificar }: { det: any; onVerificar?: (aprobar: boolean) => void }) {
  const estado = det.pago_estado;
  if (!estado || estado === "NO_APLICA") return null;

  return (
    <div style={{ marginTop: 10, padding: 8, background: "var(--color-surface)", borderRadius: 6, border: "1px solid var(--color-border)" }}>
      <div className="text-secondary" style={{ fontSize: 11, marginBottom: 4, fontWeight: 700 }}>VERIFICACIÓN DE TRANSFERENCIA</div>
      <div style={{ fontSize: 12 }}>
        Estado: <strong style={{
          color: estado === "VERIFICADO" ? "var(--color-success)"
            : estado === "RECHAZADO" ? "var(--color-danger)"
            : "var(--color-warning)",
        }}>
          {estado === "REGISTRADO" ? "⏱ Por verificar"
            : estado === "VERIFICADO" ? "✓ Verificada"
            : "✗ Rechazada"}
        </strong>
      </div>
      {det.verificador_nombre && (
        <div style={{ fontSize: 11, color: "var(--color-text-secondary)", marginTop: 2 }}>
          {estado === "VERIFICADO" ? "Verificada" : "Rechazada"} por {det.verificador_nombre} el {det.fecha_verificacion}
        </div>
      )}
      {det.motivo_verificacion && (
        <div style={{ fontSize: 11, marginTop: 2 }}>Motivo: {det.motivo_verificacion}</div>
      )}
      {onVerificar && (
        <div style={{ display: "flex", gap: 6, marginTop: 8 }}>
          <button className="btn btn-success" style={{ fontSize: 11, padding: "4px 12px" }}
            onClick={() => onVerificar(true)}>✓ Verificar</button>
          <button className="btn btn-outline" style={{ fontSize: 11, padding: "4px 12px", borderColor: "var(--color-danger)", color: "var(--color-danger)" }}
            onClick={() => onVerificar(false)}>✗ Rechazar</button>
        </div>
      )}
    </div>
  );
}
