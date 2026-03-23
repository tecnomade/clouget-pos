import { useState, useEffect } from "react";
import { listarGuiasRemision, resumenGuiasRemision, convertirGuiaAVenta, obtenerVenta, listarCuentasBanco, imprimirGuiaRemisionPdf } from "../services/api";
import { useToast } from "../components/Toast";
import type { VentaCompleta, CuentaBanco, ResumenGuias } from "../types";

function fechaHoy(): string {
  const now = new Date();
  const y = now.getFullYear();
  const m = String(now.getMonth() + 1).padStart(2, "0");
  const d = String(now.getDate()).padStart(2, "0");
  return `${y}-${m}-${d}`;
}

function fechaHace(dias: number): string {
  const now = new Date();
  now.setDate(now.getDate() - dias);
  const y = now.getFullYear();
  const m = String(now.getMonth() + 1).padStart(2, "0");
  const d = String(now.getDate()).padStart(2, "0");
  return `${y}-${m}-${d}`;
}

function primerDiaMes(): string {
  const now = new Date();
  const y = now.getFullYear();
  const m = String(now.getMonth() + 1).padStart(2, "0");
  return `${y}-${m}-01`;
}

interface GuiaItem {
  id: number;
  numero: string;
  fecha: string;
  cliente_nombre?: string;
  cliente_id?: number;
  total: number;
  estado: string;
  tipo_estado?: string;
}

export default function GuiasRemisionPage() {
  const { toastExito, toastError } = useToast();
  const [guias, setGuias] = useState<GuiaItem[]>([]);
  const [fechaDesde, setFechaDesde] = useState(fechaHoy);
  const [fechaHasta, setFechaHasta] = useState(fechaHoy);
  const [filtroEstado, setFiltroEstado] = useState<string>("TODAS");
  const [resumen, setResumen] = useState<ResumenGuias | null>(null);

  // Detail modal
  const [detalle, setDetalle] = useState<VentaCompleta | null>(null);

  // Conversion modal
  const [convertir, setConvertir] = useState<VentaCompleta | null>(null);
  const [convFormaPago, setConvFormaPago] = useState("EFECTIVO");
  const [convMonto, setConvMonto] = useState("");
  const [convEsFiado, setConvEsFiado] = useState(false);
  const [convBancoId, setConvBancoId] = useState<number | null>(null);
  const [convReferencia, setConvReferencia] = useState("");
  const [convirtiendo, setConvirtiendo] = useState(false);
  const [cuentasBanco, setCuentasBanco] = useState<CuentaBanco[]>([]);

  const cargar = async () => {
    try {
      const estado = filtroEstado === "TODAS" ? undefined : filtroEstado === "PENDIENTES" ? "PENDIENTE" : "COMPLETADA";
      const [g, r] = await Promise.all([
        listarGuiasRemision({ fechaDesde, fechaHasta, estado }),
        resumenGuiasRemision(fechaDesde, fechaHasta),
      ]);
      setGuias(g);
      setResumen(r);
    } catch (err) {
      toastError("Error al cargar guias: " + err);
    }
  };

  useEffect(() => { cargar(); }, [fechaDesde, fechaHasta, filtroEstado]);
  useEffect(() => { listarCuentasBanco().then(setCuentasBanco).catch(() => {}); }, []);

  const abrirDetalle = async (id: number) => {
    try {
      const vc = await obtenerVenta(id);
      setDetalle(vc);
    } catch (err) {
      toastError("Error al cargar detalle: " + err);
    }
  };

  const abrirConvertir = async (id: number) => {
    try {
      const vc = await obtenerVenta(id);
      setConvertir(vc);
      setConvFormaPago("EFECTIVO");
      setConvMonto("");
      setConvEsFiado(false);
      setConvBancoId(null);
      setConvReferencia("");
    } catch (err) {
      toastError("Error al cargar guia: " + err);
    }
  };

  const ejecutarConversion = async () => {
    if (!convertir?.venta.id) return;
    setConvirtiendo(true);
    try {
      const res = await convertirGuiaAVenta({
        guiaId: convertir.venta.id!,
        formaPago: convFormaPago,
        montoRecibido: convFormaPago === "EFECTIVO" ? parseFloat(convMonto) || 0 : convertir.venta.total,
        esFiado: convEsFiado,
        bancoId: convFormaPago === "TRANSFERENCIA" && convBancoId ? convBancoId : undefined,
        referenciaPago: convFormaPago === "TRANSFERENCIA" && convReferencia ? convReferencia : undefined,
      });
      toastExito(`Venta ${res.venta.numero} creada desde guia`);
      setConvertir(null);
      cargar();
    } catch (err) {
      toastError("Error al convertir: " + err);
    } finally {
      setConvirtiendo(false);
    }
  };

  return (
    <>
      <div className="page-header">
        <h2>Guias de Remision</h2>
        <div className="flex gap-2 items-center">
          <button className="btn btn-outline" style={{ fontSize: 11, padding: "4px 8px" }}
            onClick={() => { setFechaDesde(fechaHoy()); setFechaHasta(fechaHoy()); }}>
            Hoy
          </button>
          <button className="btn btn-outline" style={{ fontSize: 11, padding: "4px 8px" }}
            onClick={() => { setFechaDesde(fechaHace(6)); setFechaHasta(fechaHoy()); }}>
            7 dias
          </button>
          <button className="btn btn-outline" style={{ fontSize: 11, padding: "4px 8px" }}
            onClick={() => { setFechaDesde(primerDiaMes()); setFechaHasta(fechaHoy()); }}>
            Este mes
          </button>
          <span className="text-secondary" style={{ fontSize: 12 }}>|</span>
          <input type="date" className="input" style={{ width: 150, fontSize: 12 }} value={fechaDesde}
            onChange={(e) => setFechaDesde(e.target.value)} />
          <span className="text-secondary" style={{ fontSize: 12 }}>a</span>
          <input type="date" className="input" style={{ width: 150, fontSize: 12 }} value={fechaHasta}
            onChange={(e) => setFechaHasta(e.target.value)} />
        </div>
      </div>

      <div className="page-body">
        {/* Status filter */}
        <div style={{ display: "flex", gap: 6, marginBottom: 16 }}>
          {["TODAS", "PENDIENTES", "CERRADAS"].map(f => (
            <button key={f} className="btn"
              style={{
                fontSize: 12, padding: "5px 14px", fontWeight: 600,
                background: filtroEstado === f ? "var(--color-primary)" : "transparent",
                color: filtroEstado === f ? "white" : "var(--color-text-secondary)",
                border: filtroEstado === f ? "none" : "1px solid var(--color-border)",
              }}
              onClick={() => setFiltroEstado(f)}>
              {f === "TODAS" ? "Todas" : f === "PENDIENTES" ? "Pendientes" : "Cerradas"}
            </button>
          ))}
        </div>

        {/* Summary cards */}
        {resumen && (
          <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: 12, marginBottom: 20, maxWidth: 500 }}>
            <div className="card" style={{ padding: 14, borderLeft: "3px solid var(--color-warning)" }}>
              <div className="text-secondary" style={{ fontSize: 11 }}>Pendientes</div>
              <div style={{ display: "flex", justifyContent: "space-between", alignItems: "baseline" }}>
                <span className="text-xl font-bold" style={{ color: "var(--color-warning)" }}>{resumen.abiertas}</span>
                <span style={{ fontSize: 14, fontWeight: 600, color: "var(--color-warning)" }}>${resumen.total_pendiente.toFixed(2)}</span>
              </div>
            </div>
            <div className="card" style={{ padding: 14, borderLeft: "3px solid var(--color-success)" }}>
              <div className="text-secondary" style={{ fontSize: 11 }}>Cerradas</div>
              <div style={{ display: "flex", justifyContent: "space-between", alignItems: "baseline" }}>
                <span className="text-xl font-bold" style={{ color: "var(--color-success)" }}>{resumen.cerradas}</span>
                <span style={{ fontSize: 14, fontWeight: 600, color: "var(--color-success)" }}>${resumen.total_cerrado.toFixed(2)}</span>
              </div>
            </div>
          </div>
        )}

        {/* Table */}
        <div className="card">
          <div className="card-header flex justify-between items-center">
            <span>Guias de Remision</span>
            <span className="text-secondary" style={{ fontSize: 12 }}>{guias.length} registro{guias.length !== 1 ? "s" : ""}</span>
          </div>
          <div style={{ maxHeight: 500, overflow: "auto" }}>
            <table className="table">
              <thead>
                <tr>
                  <th>Numero</th>
                  <th>Fecha</th>
                  <th>Cliente</th>
                  <th className="text-right">Total</th>
                  <th>Estado</th>
                  <th></th>
                </tr>
              </thead>
              <tbody>
                {guias.map((g) => (
                  <tr key={g.id}>
                    <td><strong>{g.numero}</strong></td>
                    <td className="text-secondary" style={{ fontSize: 12 }}>
                      {g.fecha ? new Date(g.fecha).toLocaleString("es-EC", {
                        day: "2-digit", month: "2-digit", hour: "2-digit", minute: "2-digit"
                      }) : "-"}
                    </td>
                    <td style={{ fontSize: 13 }}>{g.cliente_nombre || "Consumidor Final"}</td>
                    <td className="text-right font-bold">${g.total.toFixed(2)}</td>
                    <td>
                      <span style={{
                        fontSize: 10, padding: "2px 8px", borderRadius: 3, fontWeight: 600,
                        background: g.estado === "PENDIENTE" || g.tipo_estado === "GUIA_REMISION"
                          ? "rgba(251, 146, 60, 0.15)" : "rgba(74, 222, 128, 0.15)",
                        color: g.estado === "PENDIENTE" || g.tipo_estado === "GUIA_REMISION"
                          ? "var(--color-warning)" : "var(--color-success)",
                      }}>
                        {g.estado === "PENDIENTE" || g.tipo_estado === "GUIA_REMISION" ? "PENDIENTE" : "COMPLETADA"}
                      </span>
                    </td>
                    <td>
                      <div className="flex gap-1">
                        {(g.estado === "PENDIENTE" || g.tipo_estado === "GUIA_REMISION") && (
                          <button className="btn btn-outline" style={{
                            fontSize: 10, padding: "2px 8px",
                            color: "var(--color-success)", borderColor: "rgba(74, 222, 128, 0.4)",
                          }}
                            onClick={() => abrirConvertir(g.id)}>
                            Convertir
                          </button>
                        )}
                        <button className="btn btn-outline" style={{ fontSize: 10, padding: "2px 8px" }}
                          onClick={() => abrirDetalle(g.id)}>
                          Ver
                        </button>
                        <button className="btn btn-outline" style={{ fontSize: 10, padding: "2px 8px" }}
                          onClick={async () => {
                            try { await imprimirGuiaRemisionPdf(g.id); toastExito("PDF generado"); }
                            catch (e) { toastError("Error: " + e); }
                          }}>
                          PDF
                        </button>
                      </div>
                    </td>
                  </tr>
                ))}
                {guias.length === 0 && (
                  <tr>
                    <td colSpan={6} className="text-center text-secondary" style={{ padding: 30 }}>
                      No hay guias de remision para este periodo
                    </td>
                  </tr>
                )}
              </tbody>
            </table>
          </div>
        </div>
      </div>

      {/* Detail modal */}
      {detalle && (
        <div style={{ position: "fixed", inset: 0, background: "rgba(0,0,0,0.5)", display: "flex", alignItems: "center", justifyContent: "center", zIndex: 100 }}
          onClick={(e) => { if (e.target === e.currentTarget) setDetalle(null); }}>
          <div className="card" style={{ width: 550, maxHeight: "85vh", overflow: "auto" }}>
            <div className="card-header flex justify-between items-center">
              <span>Guia de Remision {detalle.venta.numero}</span>
              <button className="btn btn-outline" style={{ padding: "2px 8px" }} onClick={() => setDetalle(null)}>x</button>
            </div>
            <div className="card-body">
              <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: 8, marginBottom: 16, fontSize: 13 }}>
                <div>
                  <span className="text-secondary">Fecha: </span>
                  {detalle.venta.fecha ? new Date(detalle.venta.fecha).toLocaleString("es-EC") : "-"}
                </div>
                <div>
                  <span className="text-secondary">Cliente: </span>
                  {detalle.cliente_nombre || "Consumidor Final"}
                </div>
                <div>
                  <span className="text-secondary">Estado: </span>
                  <span style={{
                    fontSize: 10, padding: "2px 6px", borderRadius: 3, fontWeight: 600,
                    background: detalle.venta.tipo_estado === "GUIA_REMISION" ? "rgba(251, 146, 60, 0.15)" : "rgba(74, 222, 128, 0.15)",
                    color: detalle.venta.tipo_estado === "GUIA_REMISION" ? "var(--color-warning)" : "var(--color-success)",
                  }}>
                    {detalle.venta.tipo_estado === "GUIA_REMISION" ? "PENDIENTE" : "COMPLETADA"}
                  </span>
                </div>
              </div>

              <div style={{ fontSize: 12, fontWeight: 600, marginBottom: 6, color: "var(--color-text-secondary)" }}>Productos</div>
              <table className="table" style={{ fontSize: 12 }}>
                <thead>
                  <tr>
                    <th>Producto</th>
                    <th className="text-right">Cant.</th>
                    <th className="text-right">P.Unit</th>
                    <th className="text-right">Subtotal</th>
                  </tr>
                </thead>
                <tbody>
                  {detalle.detalles.map((d, i) => (
                    <tr key={i}>
                      <td>{d.nombre_producto || `Producto #${d.producto_id}`}</td>
                      <td className="text-right">{d.cantidad}</td>
                      <td className="text-right">${d.precio_unitario.toFixed(2)}</td>
                      <td className="text-right">${d.subtotal.toFixed(2)}</td>
                    </tr>
                  ))}
                </tbody>
              </table>

              <div style={{ borderTop: "2px solid var(--color-border)", marginTop: 8, paddingTop: 8, fontSize: 13 }}>
                <div className="flex justify-between">
                  <span className="text-secondary">Subtotal sin IVA:</span>
                  <span>${detalle.venta.subtotal_sin_iva.toFixed(2)}</span>
                </div>
                <div className="flex justify-between">
                  <span className="text-secondary">Subtotal con IVA:</span>
                  <span>${detalle.venta.subtotal_con_iva.toFixed(2)}</span>
                </div>
                <div className="flex justify-between">
                  <span className="text-secondary">IVA:</span>
                  <span>${detalle.venta.iva.toFixed(2)}</span>
                </div>
                <div className="flex justify-between" style={{ fontWeight: 700, fontSize: 16, marginTop: 4 }}>
                  <span>TOTAL:</span>
                  <span className="text-success">${detalle.venta.total.toFixed(2)}</span>
                </div>
              </div>
            </div>
          </div>
        </div>
      )}

      {/* Conversion modal */}
      {convertir && (
        <div style={{ position: "fixed", inset: 0, background: "rgba(0,0,0,0.5)", display: "flex", alignItems: "center", justifyContent: "center", zIndex: 100 }}
          onClick={(e) => { if (e.target === e.currentTarget) setConvertir(null); }}>
          <div className="card" style={{ width: 550, maxHeight: "85vh", overflow: "auto" }}>
            <div className="card-header flex justify-between items-center">
              <span>Convertir Guia {convertir.venta.numero} a Venta</span>
              <button className="btn btn-outline" style={{ padding: "2px 8px" }} onClick={() => setConvertir(null)}>x</button>
            </div>
            <div className="card-body">
              {/* Items (readonly) */}
              <div style={{ fontSize: 12, fontWeight: 600, marginBottom: 6, color: "var(--color-text-secondary)" }}>Productos</div>
              <table className="table" style={{ fontSize: 12, marginBottom: 16 }}>
                <thead>
                  <tr>
                    <th>Producto</th>
                    <th className="text-right">Cant.</th>
                    <th className="text-right">P.Unit</th>
                    <th className="text-right">Subtotal</th>
                  </tr>
                </thead>
                <tbody>
                  {convertir.detalles.map((d, i) => (
                    <tr key={i}>
                      <td>{d.nombre_producto || `Producto #${d.producto_id}`}</td>
                      <td className="text-right">{d.cantidad}</td>
                      <td className="text-right">${d.precio_unitario.toFixed(2)}</td>
                      <td className="text-right">${d.subtotal.toFixed(2)}</td>
                    </tr>
                  ))}
                </tbody>
              </table>

              <div style={{ borderTop: "2px solid var(--color-border)", paddingTop: 8, marginBottom: 16 }}>
                <div className="flex justify-between" style={{ fontWeight: 700, fontSize: 16 }}>
                  <span>TOTAL:</span>
                  <span className="text-success">${convertir.venta.total.toFixed(2)}</span>
                </div>
              </div>

              {/* Payment form */}
              <div style={{ background: "var(--color-surface-alt)", borderRadius: 8, padding: 14 }}>
                <div style={{ fontSize: 12, fontWeight: 600, marginBottom: 10, color: "var(--color-text-secondary)" }}>Forma de Pago</div>

                <div style={{ display: "flex", gap: 6, marginBottom: 12 }}>
                  <button className="btn" style={{
                    flex: 1, padding: "8px 0", fontWeight: 600, fontSize: 13,
                    background: convFormaPago === "EFECTIVO" ? "rgba(74, 222, 128, 0.2)" : "transparent",
                    color: convFormaPago === "EFECTIVO" ? "var(--color-success)" : "var(--color-text-secondary)",
                    border: convFormaPago === "EFECTIVO" ? "1px solid rgba(74, 222, 128, 0.4)" : "1px solid var(--color-border)",
                  }} onClick={() => { setConvFormaPago("EFECTIVO"); setConvEsFiado(false); }}>
                    Efectivo
                  </button>
                  <button className="btn" style={{
                    flex: 1, padding: "8px 0", fontWeight: 600, fontSize: 13,
                    background: convFormaPago === "TRANSFERENCIA" ? "rgba(96, 165, 250, 0.2)" : "transparent",
                    color: convFormaPago === "TRANSFERENCIA" ? "var(--color-primary)" : "var(--color-text-secondary)",
                    border: convFormaPago === "TRANSFERENCIA" ? "1px solid rgba(96, 165, 250, 0.4)" : "1px solid var(--color-border)",
                  }} onClick={() => { setConvFormaPago("TRANSFERENCIA"); setConvEsFiado(false); }}>
                    Transfer.
                  </button>
                  <button className="btn" style={{
                    flex: 1, padding: "8px 0", fontWeight: 600, fontSize: 13,
                    background: convEsFiado ? "rgba(251, 191, 36, 0.2)" : "transparent",
                    color: convEsFiado ? "var(--color-warning)" : "var(--color-text-secondary)",
                    border: convEsFiado ? "1px solid rgba(251, 191, 36, 0.4)" : "1px solid var(--color-border)",
                  }} onClick={() => { setConvFormaPago("EFECTIVO"); setConvEsFiado(true); }}>
                    Credito
                  </button>
                </div>

                {convFormaPago === "EFECTIVO" && !convEsFiado && (
                  <div style={{ marginBottom: 10 }}>
                    <label style={{ fontSize: 12, color: "var(--color-text-secondary)", display: "block", marginBottom: 4 }}>Monto recibido</label>
                    <input type="number" className="input" style={{ width: "100%", fontSize: 14 }}
                      placeholder={`$${convertir.venta.total.toFixed(2)}`}
                      value={convMonto} onChange={(e) => setConvMonto(e.target.value)} />
                  </div>
                )}

                {convFormaPago === "TRANSFERENCIA" && (
                  <>
                    <div style={{ marginBottom: 10 }}>
                      <label style={{ fontSize: 12, color: "var(--color-text-secondary)", display: "block", marginBottom: 4 }}>Cuenta bancaria</label>
                      <select className="input" style={{ width: "100%", fontSize: 13 }}
                        value={convBancoId ?? ""} onChange={(e) => setConvBancoId(e.target.value ? Number(e.target.value) : null)}>
                        <option value="">Seleccionar...</option>
                        {cuentasBanco.filter(c => c.activa).map(c => (
                          <option key={c.id} value={c.id}>{c.nombre} {c.numero_cuenta ? `- ${c.numero_cuenta}` : ""}</option>
                        ))}
                      </select>
                    </div>
                    <div style={{ marginBottom: 10 }}>
                      <label style={{ fontSize: 12, color: "var(--color-text-secondary)", display: "block", marginBottom: 4 }}>Referencia</label>
                      <input type="text" className="input" style={{ width: "100%", fontSize: 13 }}
                        placeholder="Nro. comprobante"
                        value={convReferencia} onChange={(e) => setConvReferencia(e.target.value)} />
                    </div>
                  </>
                )}

                <button className="btn" style={{
                  width: "100%", padding: "10px 0", fontWeight: 700, fontSize: 14, marginTop: 8,
                  background: "var(--color-success)", color: "white", border: "none",
                }}
                  disabled={convirtiendo}
                  onClick={ejecutarConversion}>
                  {convirtiendo ? "Convirtiendo..." : "Convertir a Venta"}
                </button>
              </div>
            </div>
          </div>
        </div>
      )}
    </>
  );
}
