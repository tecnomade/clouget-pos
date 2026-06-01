import { useState, useEffect } from "react";
import { listarGuiasRemision, resumenGuiasRemision, convertirGuiaAVenta, obtenerVenta, listarCuentasBanco, imprimirGuiaRemisionPdf, cambiarEstadoGuia, obtenerConfig, guiaGuardarDatosSri, guiaObtenerDatosSri, emitirGuiaRemisionSri } from "../services/api";
import type { GuiaDatosSri } from "../services/api";
import { useToast } from "../components/Toast";
import type { VentaCompleta, CuentaBanco, ResumenGuias } from "../types";

const EMIT_FORM_VACIO = {
  transportista: "", ruc_transportista: "", tipo_id_transportista: "",
  dir_partida: "", fecha_inicio_transporte: "", fecha_fin_transporte: "",
  motivo_traslado: "", ruta: "", cod_doc_sustento: "01", num_doc_sustento: "",
  num_aut_sustento: "", fecha_emision_sustento: "", placa: "", direccion_destino: "",
};

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

  // Emisión SRI (codDoc 06) — solo con módulo contabilidad
  const [moduloContab, setModuloContab] = useState(false);
  const [emitirGuia, setEmitirGuia] = useState<GuiaItem | null>(null);
  const [emitForm, setEmitForm] = useState({ ...EMIT_FORM_VACIO });
  const [emitEstado, setEmitEstado] = useState<{ estado_sri: string; numero_sri: string }>({ estado_sri: "", numero_sri: "" });
  const [emitiendo, setEmitiendo] = useState(false);

  const cargar = async () => {
    try {
      const estado = filtroEstado === "TODAS" ? undefined
        : filtroEstado === "PENDIENTES" ? "PENDIENTE"
        : filtroEstado === "ENTREGADAS" ? "ENTREGADA"
        : filtroEstado === "RECHAZADAS" ? "RECHAZADA"
        : filtroEstado === "FACTURADAS" ? "FACTURADA"
        : "COMPLETADA";
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

  useEffect(() => {
    obtenerConfig().then(cfg => {
      const mods = (cfg.licencia_modulos || "").trim();
      const demo = cfg.demo_activo === "1";
      const tiene = mods.includes("contabilidad") || mods.includes("sri_avanzado");
      setModuloContab(demo || tiene);
    }).catch(() => {});
  }, []);

  const abrirEmitir = async (g: GuiaItem) => {
    setEmitirGuia(g);
    setEmitForm({ ...EMIT_FORM_VACIO });
    setEmitEstado({ estado_sri: "", numero_sri: "" });
    try {
      const d = await guiaObtenerDatosSri(g.id);
      setEmitForm({
        transportista: d.transportista || "",
        ruc_transportista: d.ruc_transportista || "",
        tipo_id_transportista: d.tipo_id_transportista || "",
        dir_partida: d.dir_partida || "",
        fecha_inicio_transporte: d.fecha_inicio_transporte || fechaHoy(),
        fecha_fin_transporte: d.fecha_fin_transporte || fechaHoy(),
        motivo_traslado: d.motivo_traslado || "",
        ruta: d.ruta || "",
        cod_doc_sustento: d.cod_doc_sustento || "01",
        num_doc_sustento: d.num_doc_sustento || "",
        num_aut_sustento: d.num_aut_sustento || "",
        fecha_emision_sustento: d.fecha_emision_sustento || "",
        placa: d.placa || "",
        direccion_destino: d.direccion_destino || "",
      });
      setEmitEstado({ estado_sri: d.estado_sri || "", numero_sri: d.numero_sri || "" });
    } catch (e) {
      toastError("Error cargando datos: " + e);
    }
  };

  const guardarYEmitir = async () => {
    if (!emitirGuia) return;
    if (!emitForm.transportista.trim() || !emitForm.ruc_transportista.trim()) {
      toastError("Ingrese el transportista (razon social e identificacion)");
      return;
    }
    if (!emitForm.dir_partida.trim()) { toastError("Ingrese la direccion de partida"); return; }
    if (!emitForm.motivo_traslado.trim()) { toastError("Ingrese el motivo del traslado"); return; }
    setEmitiendo(true);
    try {
      const datos: GuiaDatosSri = { ...emitForm };
      await guiaGuardarDatosSri(emitirGuia.id, datos);
      const res = await emitirGuiaRemisionSri(emitirGuia.id);
      if (res.exito) {
        toastExito(`Guia autorizada por el SRI (${res.numero_factura || res.clave_acceso || ""})`);
        setEmitirGuia(null);
        cargar();
      } else {
        toastError(`SRI: ${res.mensaje || res.estado_sri}`);
        setEmitEstado(s => ({ ...s, estado_sri: res.estado_sri }));
      }
    } catch (e) {
      toastError("Error al emitir: " + e);
    } finally {
      setEmitiendo(false);
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
          {["TODAS", "PENDIENTES", "ENTREGADAS", "FACTURADAS", "RECHAZADAS", "CERRADAS"].map(f => (
            <button key={f} className="btn"
              style={{
                fontSize: 12, padding: "5px 14px", fontWeight: 600,
                background: filtroEstado === f ? "var(--color-primary)" : "transparent",
                color: filtroEstado === f ? "white" : "var(--color-text-secondary)",
                border: filtroEstado === f ? "none" : "1px solid var(--color-border)",
              }}
              onClick={() => setFiltroEstado(f)}>
              {f === "TODAS" ? "Todas" : f === "PENDIENTES" ? "Pendientes" : f === "ENTREGADAS" ? "Entregadas" : f === "FACTURADAS" ? "Facturadas" : f === "RECHAZADAS" ? "Rechazadas" : "Cerradas"}
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
                        background: g.estado === "PENDIENTE" ? "rgba(251, 146, 60, 0.15)"
                          : g.estado === "ENTREGADA" ? "rgba(74, 222, 128, 0.15)"
                          : g.estado === "FACTURADA" ? "rgba(59, 130, 246, 0.15)"
                          : g.estado === "RECHAZADA" ? "rgba(239, 68, 68, 0.15)"
                          : "rgba(96, 165, 250, 0.15)",
                        color: g.estado === "PENDIENTE" ? "var(--color-warning)"
                          : g.estado === "ENTREGADA" ? "var(--color-success)"
                          : g.estado === "FACTURADA" ? "var(--color-primary)"
                          : g.estado === "RECHAZADA" ? "var(--color-danger)"
                          : "var(--color-primary)",
                      }}>
                        {g.estado === "PENDIENTE" ? "PENDIENTE"
                          : g.estado === "ENTREGADA" ? "ENTREGADA"
                          : g.estado === "FACTURADA" ? "FACTURADA"
                          : g.estado === "RECHAZADA" ? "RECHAZADA"
                          : "CERRADA"}
                      </span>
                    </td>
                    <td>
                      <div className="flex gap-1">
                        {/* Convertir: disponible para PENDIENTE y ENTREGADA (ya entregada al cliente).
                            NO disponible para FACTURADA (ya se convirtio) o RECHAZADA. */}
                        {(g.estado === "PENDIENTE" || g.estado === "ENTREGADA") && (
                          <button className="btn btn-outline" style={{
                            fontSize: 10, padding: "2px 8px",
                            color: "var(--color-primary)", borderColor: "var(--color-primary)",
                            fontWeight: 600,
                          }}
                            title="Convertir esta guia en una venta cobrada (no descuenta stock de nuevo, ya se descontó al crear la guia)"
                            onClick={() => abrirConvertir(g.id)}>
                            💰 Facturar
                          </button>
                        )}
                        {g.estado === "PENDIENTE" && (
                          <>
                            <button className="btn btn-outline" style={{
                              fontSize: 10, padding: "2px 8px",
                              color: "var(--color-success)", borderColor: "rgba(74, 222, 128, 0.4)",
                            }}
                              onClick={async () => {
                                try {
                                  await cambiarEstadoGuia(g.id, "ENTREGADA");
                                  toastExito("Guia marcada como entregada");
                                  cargar();
                                } catch (e) { toastError("Error: " + e); }
                              }}>
                              Entregada
                            </button>
                            <button className="btn btn-outline" style={{
                              fontSize: 10, padding: "2px 8px",
                              color: "var(--color-danger)", borderColor: "rgba(239, 68, 68, 0.4)",
                            }}
                              onClick={async () => {
                                if (!confirm("Rechazar guia? Se devolvera el stock de los productos.")) return;
                                try {
                                  await cambiarEstadoGuia(g.id, "RECHAZADA");
                                  toastExito("Guia rechazada, stock devuelto");
                                  cargar();
                                } catch (e) { toastError("Error: " + e); }
                              }}>
                              Rechazada
                            </button>
                          </>
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
                        {moduloContab && (
                          <button className="btn btn-outline" style={{
                            fontSize: 10, padding: "2px 8px", fontWeight: 600,
                            color: "var(--color-primary)", borderColor: "var(--color-primary)",
                          }}
                            title="Emitir esta guia de remision electronicamente al SRI (codDoc 06)"
                            onClick={() => abrirEmitir(g)}>
                            📤 SRI
                          </button>
                        )}
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
                    background: detalle.venta.estado === "PENDIENTE" ? "rgba(251, 146, 60, 0.15)"
                      : detalle.venta.estado === "ENTREGADA" ? "rgba(74, 222, 128, 0.15)"
                      : detalle.venta.estado === "RECHAZADA" ? "rgba(239, 68, 68, 0.15)"
                      : "rgba(96, 165, 250, 0.15)",
                    color: detalle.venta.estado === "PENDIENTE" ? "var(--color-warning)"
                      : detalle.venta.estado === "ENTREGADA" ? "var(--color-success)"
                      : detalle.venta.estado === "RECHAZADA" ? "var(--color-danger)"
                      : "var(--color-primary)",
                  }}>
                    {detalle.venta.estado === "PENDIENTE" ? "PENDIENTE"
                      : detalle.venta.estado === "ENTREGADA" ? "ENTREGADA"
                      : detalle.venta.estado === "RECHAZADA" ? "RECHAZADA"
                      : "CERRADA"}
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

      {/* Emission modal (SRI codDoc 06) */}
      {emitirGuia && (
        <div style={{ position: "fixed", inset: 0, background: "rgba(0,0,0,0.5)", display: "flex", alignItems: "center", justifyContent: "center", zIndex: 100 }}
          onClick={(e) => { if (e.target === e.currentTarget) setEmitirGuia(null); }}>
          <div className="card" style={{ width: 620, maxHeight: "90vh", overflow: "auto" }}>
            <div className="card-header flex justify-between items-center">
              <span>Emitir Guia {emitirGuia.numero} al SRI</span>
              <button className="btn btn-outline" style={{ padding: "2px 8px" }} onClick={() => setEmitirGuia(null)}>x</button>
            </div>
            <div className="card-body">
              {emitEstado.estado_sri === "AUTORIZADA" ? (
                <div style={{ background: "rgba(74,222,128,0.12)", border: "1px solid rgba(74,222,128,0.4)", borderRadius: 8, padding: 14, marginBottom: 12 }}>
                  <div style={{ fontWeight: 700, color: "var(--color-success)", marginBottom: 4 }}>✓ Guia ya autorizada por el SRI</div>
                  <div style={{ fontSize: 12 }} className="text-secondary">Nro SRI: {emitEstado.numero_sri || "-"}</div>
                </div>
              ) : emitEstado.estado_sri && emitEstado.estado_sri !== "NO_APLICA" && (
                <div style={{ background: "rgba(251,191,36,0.12)", border: "1px solid rgba(251,191,36,0.4)", borderRadius: 8, padding: 10, marginBottom: 12, fontSize: 12 }}>
                  Estado SRI actual: <strong>{emitEstado.estado_sri}</strong> — puede reintentar la emision.
                </div>
              )}

              <div style={{ fontSize: 12, fontWeight: 600, marginBottom: 8, color: "var(--color-text-secondary)" }}>Transportista</div>
              <div style={{ display: "grid", gridTemplateColumns: "2fr 1fr", gap: 8, marginBottom: 10 }}>
                <div>
                  <label style={{ fontSize: 11, color: "var(--color-text-secondary)" }}>Razon social *</label>
                  <input className="input" style={{ width: "100%", fontSize: 13 }} value={emitForm.transportista}
                    onChange={(e) => setEmitForm(f => ({ ...f, transportista: e.target.value }))} />
                </div>
                <div>
                  <label style={{ fontSize: 11, color: "var(--color-text-secondary)" }}>RUC / Cedula *</label>
                  <input className="input" style={{ width: "100%", fontSize: 13 }} value={emitForm.ruc_transportista}
                    onChange={(e) => setEmitForm(f => ({ ...f, ruc_transportista: e.target.value }))} />
                </div>
              </div>

              <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: 8, marginBottom: 10 }}>
                <div>
                  <label style={{ fontSize: 11, color: "var(--color-text-secondary)" }}>Placa</label>
                  <input className="input" style={{ width: "100%", fontSize: 13 }} value={emitForm.placa}
                    onChange={(e) => setEmitForm(f => ({ ...f, placa: e.target.value }))} />
                </div>
                <div>
                  <label style={{ fontSize: 11, color: "var(--color-text-secondary)" }}>Motivo del traslado *</label>
                  <input className="input" style={{ width: "100%", fontSize: 13 }} placeholder="Venta, traslado..." value={emitForm.motivo_traslado}
                    onChange={(e) => setEmitForm(f => ({ ...f, motivo_traslado: e.target.value }))} />
                </div>
              </div>

              <div style={{ marginBottom: 10 }}>
                <label style={{ fontSize: 11, color: "var(--color-text-secondary)" }}>Direccion de partida *</label>
                <input className="input" style={{ width: "100%", fontSize: 13 }} value={emitForm.dir_partida}
                  onChange={(e) => setEmitForm(f => ({ ...f, dir_partida: e.target.value }))} />
              </div>
              <div style={{ marginBottom: 10 }}>
                <label style={{ fontSize: 11, color: "var(--color-text-secondary)" }}>Direccion de destino</label>
                <input className="input" style={{ width: "100%", fontSize: 13 }} value={emitForm.direccion_destino}
                  onChange={(e) => setEmitForm(f => ({ ...f, direccion_destino: e.target.value }))} />
              </div>

              <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr 1fr", gap: 8, marginBottom: 10 }}>
                <div>
                  <label style={{ fontSize: 11, color: "var(--color-text-secondary)" }}>Inicio transporte</label>
                  <input type="date" className="input" style={{ width: "100%", fontSize: 13 }} value={emitForm.fecha_inicio_transporte}
                    onChange={(e) => setEmitForm(f => ({ ...f, fecha_inicio_transporte: e.target.value }))} />
                </div>
                <div>
                  <label style={{ fontSize: 11, color: "var(--color-text-secondary)" }}>Fin transporte</label>
                  <input type="date" className="input" style={{ width: "100%", fontSize: 13 }} value={emitForm.fecha_fin_transporte}
                    onChange={(e) => setEmitForm(f => ({ ...f, fecha_fin_transporte: e.target.value }))} />
                </div>
                <div>
                  <label style={{ fontSize: 11, color: "var(--color-text-secondary)" }}>Ruta</label>
                  <input className="input" style={{ width: "100%", fontSize: 13 }} placeholder="Quito - Guayaquil" value={emitForm.ruta}
                    onChange={(e) => setEmitForm(f => ({ ...f, ruta: e.target.value }))} />
                </div>
              </div>

              <details style={{ marginBottom: 12 }}>
                <summary style={{ fontSize: 12, fontWeight: 600, color: "var(--color-text-secondary)", cursor: "pointer" }}>Documento de sustento (opcional)</summary>
                <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: 8, marginTop: 8 }}>
                  <div>
                    <label style={{ fontSize: 11, color: "var(--color-text-secondary)" }}>Tipo</label>
                    <select className="input" style={{ width: "100%", fontSize: 13 }} value={emitForm.cod_doc_sustento}
                      onChange={(e) => setEmitForm(f => ({ ...f, cod_doc_sustento: e.target.value }))}>
                      <option value="01">Factura (01)</option>
                      <option value="03">Liquidacion compra (03)</option>
                      <option value="04">Nota de credito (04)</option>
                    </select>
                  </div>
                  <div>
                    <label style={{ fontSize: 11, color: "var(--color-text-secondary)" }}>Numero (001-001-000000001)</label>
                    <input className="input" style={{ width: "100%", fontSize: 13 }} value={emitForm.num_doc_sustento}
                      onChange={(e) => setEmitForm(f => ({ ...f, num_doc_sustento: e.target.value }))} />
                  </div>
                  <div>
                    <label style={{ fontSize: 11, color: "var(--color-text-secondary)" }}>Autorizacion (clave 49)</label>
                    <input className="input" style={{ width: "100%", fontSize: 13 }} value={emitForm.num_aut_sustento}
                      onChange={(e) => setEmitForm(f => ({ ...f, num_aut_sustento: e.target.value }))} />
                  </div>
                  <div>
                    <label style={{ fontSize: 11, color: "var(--color-text-secondary)" }}>Fecha emision sustento</label>
                    <input type="date" className="input" style={{ width: "100%", fontSize: 13 }} value={emitForm.fecha_emision_sustento}
                      onChange={(e) => setEmitForm(f => ({ ...f, fecha_emision_sustento: e.target.value }))} />
                  </div>
                </div>
              </details>

              <button className="btn" style={{
                width: "100%", padding: "10px 0", fontWeight: 700, fontSize: 14,
                background: "var(--color-primary)", color: "white", border: "none",
              }}
                disabled={emitiendo}
                onClick={guardarYEmitir}>
                {emitiendo ? "Enviando al SRI..." : emitEstado.estado_sri === "AUTORIZADA" ? "Reemitir al SRI" : "Guardar y Emitir al SRI"}
              </button>
              <div style={{ fontSize: 11, color: "var(--color-text-secondary)", marginTop: 8, textAlign: "center" }}>
                Se firma con tu certificado digital y se envia al SRI (ambiente segun configuracion).
              </div>
            </div>
          </div>
        </div>
      )}
    </>
  );
}
