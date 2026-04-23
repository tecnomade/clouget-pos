import React, { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listarVentasDia, listarVentasPeriodo, imprimirTicket, imprimirTicketPdf, exportarVentasCsv, emitirFacturaSri, obtenerXmlFirmado, imprimirRide, enviarNotificacionSri, obtenerConfig, procesarEmailsPendientes, listarNotasCreditoDia, listarNotasCredito, emitirNotaCreditoSri, generarRideNcPdf, listarVentasSesionCaja, resumenSesionCaja, listarNotasCreditoSesionCaja, ventasPorDia, obtenerVenta, anularVenta } from "../services/api";
import { resumenDiario, resumenPeriodo, productosMasVendidosReporte, alertasStockBajo } from "../services/api";
import { save } from "@tauri-apps/plugin-dialog";
import { useToast } from "../components/Toast";
import { useSesion } from "../contexts/SesionContext";
import ModalEmailCliente from "../components/ModalEmailCliente";
import ModalNotaCredito from "../components/ModalNotaCredito";
import { LineChart, Line, BarChart, Bar, PieChart, Pie, Cell, XAxis, YAxis, CartesianGrid, Tooltip, ResponsiveContainer } from "recharts";
import type { ResumenDiario, ResumenPeriodo, ProductoMasVendido, AlertaStock, VentaDiaria } from "../services/api";
import type { Venta, VentaCompleta, NotaCreditoInfo } from "../types";

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

export default function VentasDia() {
  const { toastExito, toastError, toastWarning } = useToast();
  const { esAdmin, tienePermiso } = useSesion();
  const [ventas, setVentas] = useState<Venta[]>([]);
  const [fechaDesde, setFechaDesde] = useState(fechaHoy);
  const [fechaHasta, setFechaHasta] = useState(fechaHoy);
  const [reintentandoSri, setReintentandoSri] = useState<number | null>(null);
  // Modal: forma de pago para SRI cuando la venta es a credito/mixto
  const [sriPagoVenta, setSriPagoVenta] = useState<{ id: number; numero: string } | null>(null);
  const [sriFormaPagoCredito, setSriFormaPagoCredito] = useState<string>("20");
  // Modal: anular venta
  const [anularVentaModal, setAnularVentaModal] = useState<{ id: number; numero: string } | null>(null);
  const [anularMotivo, setAnularMotivo] = useState<string>("");
  const [reintentandoEmail, setReintentandoEmail] = useState<number | null>(null);
  const [emailVenta, setEmailVenta] = useState<Venta | null>(null);
  const [enviandoEmail, setEnviandoEmail] = useState(false);
  const [resumen, setResumen] = useState<ResumenDiario | null>(null);
  const [resumenRango, setResumenRango] = useState<ResumenPeriodo | null>(null);
  const [topProductos, setTopProductos] = useState<ProductoMasVendido[]>([]);
  const [alertas, setAlertas] = useState<AlertaStock[]>([]);
  const [ticketUsarPdf, setTicketUsarPdf] = useState(false);
  const [notasCredito, setNotasCredito] = useState<NotaCreditoInfo[]>([]);
  const [ncVenta, setNcVenta] = useState<{ id: number; numero: string; esDevolucion?: boolean } | null>(null);
  const [reintentandoNcSri, setReintentandoNcSri] = useState<number | null>(null);
  const [tendencia, setTendencia] = useState<VentaDiaria[]>([]);
  const [ventaDetalle, setVentaDetalle] = useState<VentaCompleta | null>(null);
  const [ventaExpandida, setVentaExpandida] = useState<number | null>(null);
  const [filtroTipo, setFiltroTipo] = useState<string>("COMPLETADA");
  const [ncLista, setNcLista] = useState<any[]>([]);
  const [ncFiltroEstado, setNcFiltroEstado] = useState<string>("");

  const abrirDetalle = async (ventaId: number) => {
    try {
      const vc = await obtenerVenta(ventaId);
      setVentaDetalle(vc);
    } catch (err) {
      toastError("Error al cargar detalle: " + err);
    }
  };

  const esRango = fechaDesde !== fechaHasta;
  const COLORES_PIE = ["var(--color-success)", "var(--color-primary)", "var(--color-warning)", "#8b5cf6"];

  const handleExportarCSV = async () => {
    try {
      const destino = await save({
        defaultPath: `ventas-${fechaDesde}${esRango ? `-a-${fechaHasta}` : ""}.csv`,
        filters: [{ name: "CSV", extensions: ["csv"] }],
      });
      if (!destino) return;
      const msg = await exportarVentasCsv(fechaDesde, fechaHasta, destino);
      toastExito(msg);
    } catch (err) {
      toastError("Error al exportar: " + err);
    }
  };

  const cargar = async () => {
    if (!esAdmin) {
      // Cajero: solo ventas de su sesion de caja, sin reportes avanzados
      try {
        const [v, r, ncs] = await Promise.all([
          listarVentasSesionCaja(),
          resumenSesionCaja(),
          listarNotasCreditoSesionCaja().catch(() => [] as NotaCreditoInfo[]),
        ]);
        setVentas(v);
        setResumen(r);
        setResumenRango(null);
        setNotasCredito(ncs);
      } catch {
        // Si no tiene caja abierta, mostrar vacío
        setVentas([]);
        setResumen(null);
        setResumenRango(null);
        setNotasCredito([]);
      }
      return;
    }
    if (esRango) {
      const [v, r, top, a, ncs, tend] = await Promise.all([
        listarVentasPeriodo(fechaDesde, fechaHasta),
        resumenPeriodo(fechaDesde, fechaHasta),
        productosMasVendidosReporte(fechaDesde, fechaHasta, 10),
        alertasStockBajo(),
        listarNotasCreditoDia(fechaDesde).catch(() => [] as NotaCreditoInfo[]),
        ventasPorDia(fechaDesde, fechaHasta).catch(() => [] as VentaDiaria[]),
      ]);
      setVentas(v);
      setResumen(null);
      setResumenRango(r);
      setTopProductos(top);
      setAlertas(a);
      setNotasCredito(ncs);
      setTendencia(tend);
    } else {
      const [v, r, top, a, ncs] = await Promise.all([
        listarVentasDia(fechaDesde),
        resumenDiario(fechaDesde),
        productosMasVendidosReporte(fechaDesde, fechaDesde, 10),
        alertasStockBajo(),
        listarNotasCreditoDia(fechaDesde).catch(() => [] as NotaCreditoInfo[]),
      ]);
      setVentas(v);
      setResumen(r);
      setResumenRango(null);
      setTopProductos(top);
      setAlertas(a);
      setNotasCredito(ncs);
      setTendencia([]);
    }
  };

  useEffect(() => { cargar(); }, [fechaDesde, fechaHasta]);
  useEffect(() => {
    obtenerConfig().then((cfg) => setTicketUsarPdf(cfg.ticket_usar_pdf === "1")).catch(() => {});
  }, []);

  // Cargar notas de crédito cuando se selecciona la pestaña NC
  useEffect(() => {
    if (filtroTipo === "nc" && esAdmin) {
      listarNotasCredito(fechaDesde, fechaHasta, ncFiltroEstado || undefined)
        .then(setNcLista)
        .catch(() => setNcLista([]));
    }
  }, [filtroTipo, fechaDesde, fechaHasta, ncFiltroEstado]);

  // Resumen de NCs
  const ncResumen = filtroTipo === "nc" ? {
    total: ncLista.length,
    totalMonto: ncLista.reduce((sum, nc) => sum + (nc.total || 0), 0),
    autorizadas: ncLista.filter(nc => nc.estado_sri === "AUTORIZADA").length,
    pendientes: ncLista.filter(nc => nc.estado_sri === "PENDIENTE").length,
    rechazadas: ncLista.filter(nc => nc.estado_sri === "RECHAZADA").length,
  } : null;

  // Helper para mostrar datos del resumen (funciona tanto para diario como periodo)
  const r = resumenRango || resumen;

  return (
    <>
      <div className="page-header">
        <h2>{!esAdmin ? "Ventas del Dia" : esRango ? "Reporte de Ventas" : "Ventas del Dia"}</h2>
        {esAdmin && (
          <div className="flex gap-2 items-center">
            {/* Botones rapidos */}
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
            <button className="btn btn-outline" style={{ fontSize: 11, padding: "4px 10px" }}
              onClick={handleExportarCSV}>
              CSV
            </button>
          </div>
        )}
      </div>
      <div className="page-body">
        {/* Tarjetas de resumen */}
        {r && (
          <div style={{ display: "grid", gridTemplateColumns: "repeat(auto-fit, minmax(130px, 1fr))", gap: 12, marginBottom: 20 }}>
            <div className="card" style={{ padding: 14 }}>
              <div className="text-secondary" style={{ fontSize: 11 }}>Total Ventas</div>
              <div className="text-xl font-bold text-success">${r.total_ventas.toFixed(2)}</div>
            </div>
            <div className="card" style={{ padding: 14 }}>
              <div className="text-secondary" style={{ fontSize: 11 }}>Num. Ventas</div>
              <div className="text-xl font-bold">{r.num_ventas}</div>
            </div>
            <div className="card" style={{ padding: 14 }}>
              <div className="text-secondary" style={{ fontSize: 11 }}>Efectivo</div>
              <div className="text-xl font-bold">${r.total_efectivo.toFixed(2)}</div>
            </div>
            <div className="card" style={{ padding: 14 }}>
              <div className="text-secondary" style={{ fontSize: 11 }}>Transferencia</div>
              <div className="text-xl font-bold">${r.total_transferencia.toFixed(2)}</div>
            </div>
            {esAdmin && (
              <>
                <div className="card" style={{ padding: 14 }}>
                  <div className="text-secondary" style={{ fontSize: 11 }}>Credito</div>
                  <div className="text-xl font-bold" style={{ color: r.total_fiado > 0 ? "var(--color-warning)" : undefined }}>
                    ${r.total_fiado.toFixed(2)}
                  </div>
                </div>
                <div className="card" style={{ padding: 14 }}>
                  <div className="text-secondary" style={{ fontSize: 11 }}>Utilidad Bruta</div>
                  <div className="text-xl font-bold text-success">${r.utilidad_bruta.toFixed(2)}</div>
                </div>
              </>
            )}
            {r.total_notas_credito > 0 && (
              <div className="card" style={{ padding: 14 }}>
                <div className="text-secondary" style={{ fontSize: 11 }}>Devoluciones ({r.num_notas_credito})</div>
                <div className="text-xl font-bold" style={{ color: "var(--color-danger)" }}>-${r.total_notas_credito.toFixed(2)}</div>
              </div>
            )}
            {/* Cards adicionales para rango - solo admin */}
            {esAdmin && resumenRango && (
              <>
                <div className="card" style={{ padding: 14 }}>
                  <div className="text-secondary" style={{ fontSize: 11 }}>Promedio/Venta</div>
                  <div className="text-xl font-bold">${resumenRango.promedio_por_venta.toFixed(2)}</div>
                </div>
                <div className="card" style={{ padding: 14 }}>
                  <div className="text-secondary" style={{ fontSize: 11 }}>Total Gastos</div>
                  <div className="text-xl font-bold" style={{ color: resumenRango.total_gastos > 0 ? "var(--color-danger)" : undefined }}>
                    ${resumenRango.total_gastos.toFixed(2)}
                  </div>
                </div>
              </>
            )}
          </div>
        )}

        {/* Gráficas - solo admin con datos */}
        {esAdmin && (tendencia.length > 1 || topProductos.length > 0 || (r && r.total_ventas > 0)) && (
          <div style={{ display: "grid", gridTemplateColumns: tendencia.length > 1 ? "1fr 1fr" : "1fr 1fr", gap: 16, marginBottom: 20 }}>
            {/* Tendencia de ventas diarias */}
            {tendencia.length > 1 && (
              <div className="card" style={{ padding: "16px 12px" }}>
                <div style={{ fontSize: 13, fontWeight: 600, marginBottom: 8, color: "var(--color-text-secondary)" }}>Ventas por Dia</div>
                <ResponsiveContainer width="100%" height={200}>
                  <LineChart data={tendencia.map(d => ({ ...d, dia: d.fecha.slice(5) }))}>
                    <CartesianGrid strokeDasharray="3 3" stroke="var(--color-border)" />
                    <XAxis dataKey="dia" tick={{ fontSize: 11 }} />
                    <YAxis tick={{ fontSize: 11 }} tickFormatter={(v) => `$${v}`} />
                    <Tooltip formatter={(value) => [`$${Number(value).toFixed(2)}`, "Total"]}
                      labelFormatter={(label) => `Fecha: ${label}`} />
                    <Line type="monotone" dataKey="total" stroke="var(--color-primary)" strokeWidth={2}
                      dot={{ fill: "var(--color-primary)", r: 3 }} activeDot={{ r: 5 }} />
                  </LineChart>
                </ResponsiveContainer>
              </div>
            )}

            {/* Top productos */}
            {topProductos.length > 0 && (
              <div className="card" style={{ padding: "16px 12px" }}>
                <div style={{ fontSize: 13, fontWeight: 600, marginBottom: 8, color: "var(--color-text-secondary)" }}>Top Productos por Ingreso</div>
                <ResponsiveContainer width="100%" height={200}>
                  <BarChart data={topProductos.slice(0, 8).map(p => ({
                    nombre: p.nombre.length > 15 ? p.nombre.slice(0, 15) + "…" : p.nombre,
                    total: p.total_vendido,
                    cantidad: p.cantidad_total,
                  }))} layout="vertical">
                    <CartesianGrid strokeDasharray="3 3" stroke="var(--color-border)" />
                    <XAxis type="number" tick={{ fontSize: 10 }} tickFormatter={(v) => `$${v}`} />
                    <YAxis type="category" dataKey="nombre" tick={{ fontSize: 10 }} width={110} />
                    <Tooltip formatter={(value, name) => [
                      name === "total" ? `$${Number(value).toFixed(2)}` : value,
                      name === "total" ? "Ingreso" : "Cantidad"
                    ]} />
                    <Bar dataKey="total" fill="var(--color-primary)" radius={[0, 4, 4, 0]} />
                  </BarChart>
                </ResponsiveContainer>
              </div>
            )}

            {/* Distribución por método de pago */}
            {r && r.total_ventas > 0 && (
              <div className="card" style={{ padding: "16px 12px" }}>
                <div style={{ fontSize: 13, fontWeight: 600, marginBottom: 8, color: "var(--color-text-secondary)" }}>Metodos de Pago</div>
                <ResponsiveContainer width="100%" height={200}>
                  <PieChart>
                    <Pie
                      data={[
                        { name: "Efectivo", value: r.total_efectivo },
                        { name: "Transferencia", value: r.total_transferencia },
                        { name: "Credito", value: r.total_fiado },
                      ].filter(d => d.value > 0)}
                      cx="50%" cy="50%" innerRadius={50} outerRadius={80}
                      paddingAngle={2} dataKey="value"
                      label={({ name, percent }) => `${name} ${((percent ?? 0) * 100).toFixed(0)}%`}
                    >
                      {[
                        { name: "Efectivo", value: r.total_efectivo },
                        { name: "Transferencia", value: r.total_transferencia },
                        { name: "Credito", value: r.total_fiado },
                      ].filter(d => d.value > 0).map((_, i) => (
                        <Cell key={i} fill={COLORES_PIE[i % COLORES_PIE.length]} />
                      ))}
                    </Pie>
                    <Tooltip formatter={(value) => `$${Number(value).toFixed(2)}`} />
                  </PieChart>
                </ResponsiveContainer>
              </div>
            )}
          </div>
        )}

        <div style={{ display: "grid", gridTemplateColumns: "1fr 300px", gap: 16 }}>
          {/* Tabla de ventas */}
          <div className="card">
            <div className="card-header flex justify-between items-center">
              <div className="flex gap-2 items-center">
                {[
                  { key: "COMPLETADA", label: "Ventas" },
                  { key: "BORRADOR", label: "Borradores" },
                  { key: "COTIZACION", label: "Cotizaciones" },
                  { key: "GUIA_REMISION", label: "Guías" },
                  { key: "nc", label: "N. Crédito" },
                  { key: "TODOS", label: "Todos" },
                ].map(f => (
                  <button key={f.key}
                    className={`btn ${filtroTipo === f.key ? "btn-primary" : "btn-outline"}`}
                    style={{ fontSize: 10, padding: "3px 10px" }}
                    onClick={() => setFiltroTipo(f.key)}>
                    {f.label}
                  </button>
                ))}
              </div>
              <span className="text-secondary" style={{ fontSize: 12 }}>
                {filtroTipo === "nc"
                  ? `${ncLista.length} registro${ncLista.length !== 1 ? "s" : ""}`
                  : `${ventas.filter(v => filtroTipo === "TODOS" || (v.tipo_estado || "COMPLETADA") === filtroTipo).length} registro${ventas.filter(v => filtroTipo === "TODOS" || (v.tipo_estado || "COMPLETADA") === filtroTipo).length !== 1 ? "s" : ""}`
                }
              </span>
            </div>
            {filtroTipo === "nc" ? (
              <>
                {/* Resumen NC */}
                {ncResumen && ncResumen.total > 0 && (
                  <div style={{ display: "flex", gap: 12, padding: "10px 14px", borderBottom: "1px solid var(--color-border)", fontSize: 12 }}>
                    <div>
                      <span className="text-secondary">Total NCs: </span>
                      <strong>{ncResumen.total}</strong>
                    </div>
                    <div>
                      <span className="text-secondary">Monto: </span>
                      <strong style={{ color: "var(--color-danger)" }}>-${ncResumen.totalMonto.toFixed(2)}</strong>
                    </div>
                    {ncResumen.autorizadas > 0 && (
                      <div>
                        <span style={{ color: "var(--color-success)", fontWeight: 600 }}>{ncResumen.autorizadas} Autoriz.</span>
                      </div>
                    )}
                    {ncResumen.pendientes > 0 && (
                      <div>
                        <span style={{ color: "var(--color-warning)", fontWeight: 600 }}>{ncResumen.pendientes} Pend.</span>
                      </div>
                    )}
                    {ncResumen.rechazadas > 0 && (
                      <div>
                        <span style={{ color: "var(--color-danger)", fontWeight: 600 }}>{ncResumen.rechazadas} Rechaz.</span>
                      </div>
                    )}
                    <div style={{ marginLeft: "auto" }}>
                      <select className="input" style={{ fontSize: 11, padding: "2px 6px", width: 120 }}
                        value={ncFiltroEstado}
                        onChange={(e) => setNcFiltroEstado(e.target.value)}>
                        <option value="">Todos</option>
                        <option value="AUTORIZADA">Autorizadas</option>
                        <option value="PENDIENTE">Pendientes</option>
                        <option value="RECHAZADA">Rechazadas</option>
                      </select>
                    </div>
                  </div>
                )}
                <div style={{ maxHeight: 400, overflow: "auto" }}>
                  <table className="table">
                    <thead>
                      <tr>
                        <th>Numero</th>
                        <th>Fecha</th>
                        <th>Venta Original</th>
                        <th>Cliente</th>
                        <th>Motivo</th>
                        <th className="text-right">Total</th>
                        <th>Estado</th>
                        <th></th>
                      </tr>
                    </thead>
                    <tbody>
                      {ncLista.map((nc) => (
                        <tr key={nc.id}>
                          <td>
                            <strong>{nc.numero_factura_nc || nc.numero}</strong>
                          </td>
                          <td className="text-secondary" style={{ fontSize: 12 }}>
                            {nc.fecha
                              ? new Date(nc.fecha).toLocaleDateString("es-EC", { day: "2-digit", month: "2-digit" })
                                + " " + new Date(nc.fecha).toLocaleTimeString("es-EC", { hour: "2-digit", minute: "2-digit" })
                              : "-"}
                          </td>
                          <td>
                            <span style={{ fontSize: 11, color: "var(--color-primary)", cursor: "pointer" }}
                              onClick={() => nc.venta_id && abrirDetalle(nc.venta_id)}
                              title="Ver venta original">
                              {nc.venta_numero}
                            </span>
                          </td>
                          <td style={{ fontSize: 12 }}>{nc.cliente_nombre}</td>
                          <td style={{ fontSize: 12, maxWidth: 180, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}
                            title={nc.motivo}>
                            {nc.motivo}
                          </td>
                          <td className="text-right font-bold" style={{ color: "var(--color-danger)" }}>
                            -${nc.total?.toFixed(2)}
                          </td>
                          <td>
                            <span style={{
                              fontSize: 9, padding: "2px 6px", borderRadius: 3, fontWeight: 600,
                              background: nc.estado_sri === "AUTORIZADA" ? "rgba(34, 197, 94, 0.15)"
                                : nc.estado_sri === "PENDIENTE" ? "rgba(245, 158, 11, 0.15)"
                                : "rgba(239, 68, 68, 0.1)",
                              color: nc.estado_sri === "AUTORIZADA" ? "var(--color-success)"
                                : nc.estado_sri === "PENDIENTE" ? "var(--color-warning)"
                                : "var(--color-danger)",
                            }}>
                              {nc.estado_sri}
                            </span>
                          </td>
                          <td>
                            <div className="flex gap-1">
                              {(nc.estado_sri === "PENDIENTE" || nc.estado_sri === "RECHAZADA") && (
                                <button className="btn btn-outline" style={{
                                  padding: "2px 6px", fontSize: 10,
                                  color: "var(--color-primary)", borderColor: "rgba(59, 130, 246, 0.3)",
                                }}
                                  disabled={reintentandoNcSri === nc.id}
                                  onClick={async () => {
                                    setReintentandoNcSri(nc.id);
                                    try {
                                      const res = await emitirNotaCreditoSri(nc.id);
                                      if (res.exito) {
                                        toastExito("NC autorizada por el SRI");
                                      } else {
                                        toastWarning(`SRI NC: ${res.mensaje}`);
                                      }
                                      // Refrescar lista NC
                                      listarNotasCredito(fechaDesde, fechaHasta, ncFiltroEstado || undefined)
                                        .then(setNcLista).catch(() => {});
                                    } catch (err) {
                                      toastError("Error SRI NC: " + err);
                                    } finally {
                                      setReintentandoNcSri(null);
                                    }
                                  }}>
                                  {reintentandoNcSri === nc.id ? "..." : "SRI"}
                                </button>
                              )}
                              {nc.clave_acceso && (
                                <button className="btn btn-outline" style={{ padding: "2px 6px", fontSize: 10 }}
                                  title="Descargar XML firmado"
                                  onClick={async () => {
                                    try {
                                      // Use the venta's XML endpoint via the NC's venta_id
                                      const xml = await obtenerXmlFirmado(nc.venta_id);
                                      const destino = await save({
                                        defaultPath: `nc-${(nc.numero_factura_nc || nc.numero).replace(/[\/\\:]/g, "-")}.xml`,
                                        filters: [{ name: "XML", extensions: ["xml"] }],
                                      });
                                      if (destino) {
                                        await invoke("guardar_archivo_texto", { ruta: destino, contenido: xml });
                                        toastExito("XML guardado");
                                      }
                                    } catch (err) {
                                      toastError("Error XML: " + err);
                                    }
                                  }}>
                                  XML
                                </button>
                              )}
                              {nc.estado_sri === "AUTORIZADA" && (
                                <button className="btn btn-outline" style={{ padding: "2px 6px", fontSize: 10 }}
                                  title="Imprimir RIDE NC (PDF A4)"
                                  onClick={() => generarRideNcPdf(nc.id)
                                    .then(() => toastExito("RIDE NC abierto"))
                                    .catch((e) => toastError("Error RIDE NC: " + e))}>
                                  RIDE
                                </button>
                              )}
                            </div>
                          </td>
                        </tr>
                      ))}
                      {ncLista.length === 0 && (
                        <tr>
                          <td colSpan={8} className="text-center text-secondary" style={{ padding: 30 }}>
                            No hay notas de credito para {esRango ? "este periodo" : "esta fecha"}
                          </td>
                        </tr>
                      )}
                    </tbody>
                  </table>
                </div>
              </>
            ) : (
            <div style={{ maxHeight: 400, overflow: "auto" }}>
              <table className="table">
                <thead>
                  <tr>
                    <th style={{ width: 24 }}></th>
                    <th>Numero</th>
                    <th>{esRango ? "Fecha" : "Hora"}</th>
                    <th>Tipo</th>
                    <th>Pago</th>
                    <th className="text-right">Total</th>
                    <th></th>
                  </tr>
                </thead>
                <tbody>
                  {ventas.filter(v => filtroTipo === "TODOS" || (v.tipo_estado || "COMPLETADA") === filtroTipo).map((v) => (
                    <React.Fragment key={v.id}>
                    <tr onClick={() => v.id && abrirDetalle(v.id)}
                      style={{ cursor: "pointer" }} title="Click para ver detalle completo">
                      <td onClick={(e) => { e.stopPropagation(); setVentaExpandida(ventaExpandida === v.id ? null : (v.id ?? null)); }}
                          style={{ cursor: "pointer", textAlign: "center", color: "var(--color-text-secondary)", fontSize: 11 }}
                          title="Expandir info rápida">
                        {ventaExpandida === v.id ? "▼" : "▶"}
                      </td>
                      <td>
                        <strong>{v.numero}</strong>
                        {v.numero_factura && (
                          <div style={{ fontSize: 10, color: "var(--color-success)" }}>{v.numero_factura}</div>
                        )}
                      </td>
                      <td className="text-secondary" style={{ fontSize: 12 }}>
                        {v.fecha
                          ? esRango
                            ? new Date(v.fecha).toLocaleDateString("es-EC", { day: "2-digit", month: "2-digit" })
                              + " " + new Date(v.fecha).toLocaleTimeString("es-EC", { hour: "2-digit", minute: "2-digit" })
                            : new Date(v.fecha).toLocaleTimeString("es-EC", { hour: "2-digit", minute: "2-digit" })
                          : "-"}
                      </td>
                      <td>
                        <span style={{
                          fontSize: 10,
                          padding: "2px 6px",
                          borderRadius: 3,
                          fontWeight: 600,
                          background: v.tipo_documento === "FACTURA" ? "rgba(59, 130, 246, 0.15)" : "var(--color-surface-alt)",
                          color: v.tipo_documento === "FACTURA" ? "var(--color-primary)" : "var(--color-text-secondary)",
                        }}>
                          {v.tipo_documento === "FACTURA" ? "FAC" : "NV"}
                        </span>
                        {v.tipo_documento === "FACTURA" && (
                          <span title={v.clave_acceso ? `Clave: ${v.clave_acceso}` : undefined}
                            style={{
                            fontSize: 9,
                            padding: "1px 4px",
                            borderRadius: 3,
                            marginLeft: 4,
                            fontWeight: 600,
                            cursor: v.clave_acceso ? "help" : undefined,
                            background: v.estado_sri === "AUTORIZADA" ? "rgba(34, 197, 94, 0.15)"
                              : v.estado_sri === "PENDIENTE" ? "rgba(245, 158, 11, 0.15)"
                              : v.estado_sri === "RECHAZADA" ? "rgba(239, 68, 68, 0.1)"
                              : "var(--color-surface-alt)",
                            color: v.estado_sri === "AUTORIZADA" ? "var(--color-success)"
                              : v.estado_sri === "PENDIENTE" ? "var(--color-warning)"
                              : v.estado_sri === "RECHAZADA" ? "var(--color-danger)"
                              : "var(--color-text-secondary)",
                          }}>
                            {v.estado_sri}
                          </span>
                        )}
                      </td>
                      <td style={{ fontSize: 12 }}>{v.forma_pago}</td>
                      <td className="text-right font-bold">${v.total.toFixed(2)}</td>
                      <td>
                        <div className="flex gap-1">
                          {v.tipo_documento === "FACTURA" && (v.estado_sri === "PENDIENTE" || v.estado_sri === "RECHAZADA") && (
                            <button className="btn btn-outline" style={{
                              padding: "2px 6px", fontSize: 10,
                              color: "var(--color-primary)", borderColor: "rgba(59, 130, 246, 0.3)",
                            }}
                              disabled={reintentandoSri === v.id}
                              onClick={async () => {
                                if (!v.id) return;
                                // Si la venta es a credito o mixto, preguntar forma de pago para SRI/RIDE
                                const esCreditoOMixto = v.forma_pago?.toUpperCase() === "CREDITO"
                                  || v.forma_pago?.toUpperCase() === "MIXTO";
                                if (esCreditoOMixto) {
                                  setSriPagoVenta({ id: v.id, numero: v.numero });
                                  return;
                                }
                                setReintentandoSri(v.id);
                                try {
                                  const res = await emitirFacturaSri(v.id);
                                  if (res.exito) {
                                    toastExito("Factura autorizada por el SRI");
                                    window.dispatchEvent(new CustomEvent("sri-factura-emitida"));
                                  } else {
                                    toastWarning(`SRI: ${res.mensaje}`);
                                  }
                                  await cargar();
                                } catch (err) {
                                  toastError("Error SRI: " + err);
                                } finally {
                                  setReintentandoSri(null);
                                }
                              }}>
                              {reintentandoSri === v.id ? "..." : "SRI"}
                            </button>
                          )}
                          {v.tipo_documento === "FACTURA" && v.estado_sri === "AUTORIZADA" && (
                            <>
                              {v.email_enviado === 1 ? (
                                <span style={{
                                  fontSize: 9, padding: "2px 5px", borderRadius: 3,
                                  background: "rgba(34, 197, 94, 0.15)", color: "var(--color-success)", fontWeight: 600,
                                }} title="Email enviado">
                                  Enviado
                                </span>
                              ) : (
                                <button className="btn btn-outline" style={{
                                  padding: "2px 6px", fontSize: 10,
                                  color: "var(--color-warning)", borderColor: "var(--color-warning)",
                                }}
                                  title="Enviar por email"
                                  disabled={reintentandoEmail === v.id}
                                  onClick={async () => {
                                    if (!v.id) return;
                                    setReintentandoEmail(v.id);
                                    try {
                                      // Intentar procesar pendientes primero (puede que ya esté encolado)
                                      await procesarEmailsPendientes();
                                      // Si no se envió, abrir modal para ingresar email
                                      await cargar();
                                      const ventaActualizada = ventas.find(vv => vv.id === v.id);
                                      if (ventaActualizada?.email_enviado !== 1) {
                                        setEmailVenta(v);
                                      } else {
                                        toastExito("Email enviado correctamente");
                                      }
                                    } catch {
                                      setEmailVenta(v);
                                    } finally {
                                      setReintentandoEmail(null);
                                    }
                                  }}>
                                  {reintentandoEmail === v.id ? "..." : "Email"}
                                </button>
                              )}
                              <button className="btn btn-outline" style={{ padding: "2px 6px", fontSize: 10 }}
                                title="Descargar XML firmado"
                                onClick={async () => {
                                  if (!v.id) return;
                                  try {
                                    const xml = await obtenerXmlFirmado(v.id);
                                    const destino = await save({
                                      defaultPath: `factura-${(v.numero_factura || v.numero).replace(/[\/\\:]/g, "-")}.xml`,
                                      filters: [{ name: "XML", extensions: ["xml"] }],
                                    });
                                    if (destino) {
                                      await invoke("guardar_archivo_texto", { ruta: destino, contenido: xml });
                                      toastExito("XML guardado");
                                    }
                                  } catch (err) {
                                    toastError("Error XML: " + err);
                                  }
                                }}>
                                XML
                              </button>
                              <button className="btn btn-outline" style={{ padding: "2px 6px", fontSize: 10 }}
                                title="Imprimir RIDE (PDF A4)"
                                onClick={() => v.id && imprimirRide(v.id)
                                  .then(() => toastExito("RIDE abierto"))
                                  .catch((e) => toastError("Error RIDE: " + e))}>
                                RIDE
                              </button>
                              {(esAdmin || tienePermiso("crear_nota_credito")) && (
                                !notasCredito.some(nc => nc.venta_id === v.id) ? (
                                  <button className="btn btn-outline" style={{
                                    padding: "2px 6px", fontSize: 10,
                                    color: "var(--color-danger)", borderColor: "rgba(239, 68, 68, 0.4)",
                                  }}
                                    title="Crear Nota de Credito"
                                    onClick={() => v.id && setNcVenta({ id: v.id, numero: v.numero_factura || v.numero })}>
                                    NC
                                  </button>
                                ) : (
                                  <span style={{
                                    fontSize: 9, padding: "2px 5px", borderRadius: 3,
                                    background: "rgba(239, 68, 68, 0.1)", color: "var(--color-danger)", fontWeight: 600,
                                  }} title="Ya tiene nota de credito">
                                    NC
                                  </span>
                                )
                              )}
                            </>
                          )}
                          {/* Anular: para ventas no autorizadas (NOTA_VENTA siempre, FACTURA pendiente/rechazada) */}
                          {v.anulada !== 1 && v.estado_sri !== "AUTORIZADA" && (esAdmin || tienePermiso("crear_nota_credito")) && (
                            <button className="btn btn-outline" style={{
                              padding: "2px 8px", fontSize: 10, fontWeight: 600,
                              color: "var(--color-danger)", borderColor: "var(--color-danger)",
                              background: "rgba(239, 68, 68, 0.08)",
                            }}
                              title="Anular la venta completa y reintegrar stock"
                              onClick={() => v.id && setAnularVentaModal({ id: v.id, numero: v.numero })}>
                              🗑 Anular
                            </button>
                          )}
                          {/* Devolver: para ventas no autorizadas (NOTA_VENTA siempre, FACTURA pendiente/rechazada)
                               Devolucion parcial o total con restitucion de stock */}
                          {v.anulada !== 1 && v.estado_sri !== "AUTORIZADA" && (esAdmin || tienePermiso("crear_nota_credito")) && (
                            !notasCredito.some(nc => nc.venta_id === v.id) ? (
                              <button className="btn btn-outline" style={{
                                padding: "2px 8px", fontSize: 10, fontWeight: 600,
                                color: "var(--color-warning)", borderColor: "var(--color-warning)",
                                background: "rgba(245, 158, 11, 0.08)",
                              }}
                                title="Devolver productos (parcial o total) y reponer al stock"
                                onClick={() => v.id && setNcVenta({ id: v.id, numero: v.numero, esDevolucion: true })}>
                                ↩ Devolver
                              </button>
                            ) : (
                              <span style={{
                                fontSize: 9, padding: "2px 6px", borderRadius: 3,
                                background: "rgba(245, 158, 11, 0.15)", color: "var(--color-warning)", fontWeight: 600,
                              }} title="Esta venta ya tiene una devolucion registrada">
                                ↩ Devuelto
                              </span>
                            )
                          )}
                          {v.anulada === 1 && (
                            <span style={{
                              fontSize: 9, padding: "2px 6px", borderRadius: 3,
                              background: "rgba(239, 68, 68, 0.15)", color: "var(--color-danger)", fontWeight: 700,
                            }} title={v.observacion || "Anulada"}>
                              🗑 Anulada
                            </span>
                          )}
                          <button className="btn btn-outline" style={{ padding: "2px 6px", fontSize: 11 }}
                            onClick={() => {
                              if (!v.id) return;
                              const fn = ticketUsarPdf ? imprimirTicketPdf : imprimirTicket;
                              fn(v.id)
                                .then(() => toastExito(ticketUsarPdf ? "Ticket PDF generado" : "Ticket impreso"))
                                .catch((e) => toastError("Error al imprimir: " + e));
                            }}>
                            {ticketUsarPdf ? "PDF" : "Impr."}
                          </button>
                        </div>
                      </td>
                    </tr>
                    {ventaExpandida === v.id && (
                      <tr>
                        <td colSpan={7} style={{ background: "var(--color-surface-alt)", padding: 12, fontSize: 11 }}>
                          <div style={{ display: "grid", gridTemplateColumns: "repeat(auto-fit, minmax(220px, 1fr))", gap: 12 }}>
                            {(v as any).cliente_nombre && <div><strong>Cliente:</strong> {(v as any).cliente_nombre}</div>}
                            <div><strong>Forma de pago:</strong> {v.forma_pago}{(v as any).banco_nombre ? ` · ${(v as any).banco_nombre}` : ""}</div>
                            {(v as any).referencia_pago && <div><strong>Referencia:</strong> {(v as any).referencia_pago}</div>}
                            {v.tipo_documento === "FACTURA" && (
                              <>
                                <div>
                                  <strong>Estado SRI:</strong>{" "}
                                  <span style={{
                                    fontSize: 10, padding: "1px 6px", borderRadius: 3,
                                    background: v.estado_sri === "AUTORIZADA" ? "rgba(34,197,94,0.15)" : v.estado_sri === "RECHAZADA" ? "rgba(239,68,68,0.15)" : "rgba(245,158,11,0.15)",
                                    color: v.estado_sri === "AUTORIZADA" ? "var(--color-success)" : v.estado_sri === "RECHAZADA" ? "var(--color-danger)" : "var(--color-warning)",
                                  }}>{v.estado_sri || "PENDIENTE"}</span>
                                </div>
                                {v.numero_factura && <div><strong>N° factura SRI:</strong> {v.numero_factura}</div>}
                                {v.autorizacion_sri && (
                                  <div style={{ gridColumn: "1 / -1" }}>
                                    <strong>Autorización SRI:</strong> <span style={{ fontFamily: "monospace", fontSize: 10 }}>{v.autorizacion_sri}</span>
                                  </div>
                                )}
                                {v.clave_acceso && (
                                  <div style={{ gridColumn: "1 / -1" }}>
                                    <strong>Clave de acceso:</strong> <span style={{ fontFamily: "monospace", fontSize: 10, wordBreak: "break-all" }}>{v.clave_acceso}</span>
                                  </div>
                                )}
                              </>
                            )}
                            {v.tipo_documento !== "FACTURA" && (
                              <div style={{ gridColumn: "1 / -1", color: "var(--color-text-secondary)" }}>
                                <em>Nota de venta — sin autorización SRI (no es comprobante tributario oficial).</em>
                              </div>
                            )}
                          </div>
                          <div style={{ marginTop: 8 }}>
                            <button className="btn btn-outline" style={{ fontSize: 10, padding: "2px 10px" }}
                              onClick={(e) => { e.stopPropagation(); v.id && abrirDetalle(v.id); }}>
                              Ver items y totales completos →
                            </button>
                          </div>
                        </td>
                      </tr>
                    )}
                    </React.Fragment>
                  ))}
                  {ventas.length === 0 && (
                    <tr>
                      <td colSpan={7} className="text-center text-secondary" style={{ padding: 30 }}>
                        No hay ventas para {esRango ? "este periodo" : "esta fecha"}
                      </td>
                    </tr>
                  )}
                </tbody>
              </table>
            </div>
            )}
          </div>

          {/* Panel lateral */}
          <div style={{ display: "flex", flexDirection: "column", gap: 16 }}>
            {/* Notas de Crédito */}
            {notasCredito.length > 0 && (
              <div className="card" style={{ borderColor: "rgba(239, 68, 68, 0.4)" }}>
                <div className="card-header" style={{ background: "rgba(239, 68, 68, 0.1)", color: "var(--color-danger)" }}>
                  Notas de Credito ({notasCredito.length})
                </div>
                <div className="card-body" style={{ padding: 0 }}>
                  {notasCredito.map((nc) => (
                    <div key={nc.id} style={{ padding: "8px 12px", borderBottom: "1px solid var(--color-border)", fontSize: 12 }}>
                      <div className="flex justify-between items-center">
                        <div>
                          <strong>{nc.numero_factura_nc || nc.numero}</strong>
                          <div className="text-secondary" style={{ fontSize: 10 }}>Ref: {nc.factura_numero}</div>
                        </div>
                        <div className="text-right">
                          <div className="font-bold" style={{ color: "var(--color-danger)" }}>-${nc.total.toFixed(2)}</div>
                          <span style={{
                            fontSize: 9, padding: "1px 4px", borderRadius: 3, fontWeight: 600,
                            background: nc.estado_sri === "AUTORIZADA" ? "rgba(34, 197, 94, 0.15)" : nc.estado_sri === "PENDIENTE" ? "rgba(245, 158, 11, 0.15)" : "rgba(239, 68, 68, 0.1)",
                            color: nc.estado_sri === "AUTORIZADA" ? "var(--color-success)" : nc.estado_sri === "PENDIENTE" ? "var(--color-warning)" : "var(--color-danger)",
                          }}>
                            {nc.estado_sri}
                          </span>
                        </div>
                      </div>
                      <div className="text-secondary" style={{ fontSize: 10, marginTop: 2 }}>{nc.motivo}</div>
                      <div className="flex gap-1" style={{ marginTop: 4 }}>
                        {(nc.estado_sri === "PENDIENTE" || nc.estado_sri === "RECHAZADA") && (
                          <button className="btn btn-outline" style={{
                            padding: "1px 5px", fontSize: 9,
                            color: "var(--color-primary)", borderColor: "rgba(59, 130, 246, 0.3)",
                          }}
                            disabled={reintentandoNcSri === nc.id}
                            onClick={async () => {
                              setReintentandoNcSri(nc.id);
                              try {
                                const res = await emitirNotaCreditoSri(nc.id);
                                if (res.exito) {
                                  toastExito("NC autorizada por el SRI");
                                } else {
                                  toastWarning(`SRI NC: ${res.mensaje}`);
                                }
                                await cargar();
                              } catch (err) {
                                toastError("Error SRI NC: " + err);
                              } finally {
                                setReintentandoNcSri(null);
                              }
                            }}>
                            {reintentandoNcSri === nc.id ? "..." : "SRI"}
                          </button>
                        )}
                        {nc.estado_sri === "AUTORIZADA" && (
                          <button className="btn btn-outline" style={{ padding: "1px 5px", fontSize: 9 }}
                            onClick={() => generarRideNcPdf(nc.id)
                              .then(() => toastExito("RIDE NC abierto"))
                              .catch((e) => toastError("Error RIDE NC: " + e))}>
                            RIDE
                          </button>
                        )}
                      </div>
                    </div>
                  ))}
                </div>
              </div>
            )}
            {/* Top productos - solo admin */}
            {esAdmin && topProductos.length > 0 && (
              <div className="card">
                <div className="card-header">Mas Vendidos{esRango ? "" : " Hoy"}</div>
                <div className="card-body" style={{ padding: 0 }}>
                  {topProductos.map((p, i) => (
                    <div key={i} className="flex justify-between items-center"
                      style={{ padding: "8px 12px", borderBottom: "1px solid var(--color-border)", fontSize: 13 }}>
                      <span>{p.nombre}</span>
                      <div className="text-right">
                        <div className="font-bold">{p.cantidad_total}</div>
                        <div className="text-secondary" style={{ fontSize: 11 }}>${p.total_vendido.toFixed(2)}</div>
                      </div>
                    </div>
                  ))}
                </div>
              </div>
            )}

            {/* Alertas de stock - solo admin */}
            {esAdmin && alertas.length > 0 && (
              <div className="card" style={{ borderColor: "var(--color-warning)" }}>
                <div className="card-header" style={{ background: "rgba(245, 158, 11, 0.15)", color: "var(--color-warning)" }}>
                  Stock Bajo ({alertas.length})
                </div>
                <div className="card-body" style={{ padding: 0 }}>
                  {alertas.slice(0, 8).map((a) => (
                    <div key={a.id} className="flex justify-between"
                      style={{ padding: "6px 12px", borderBottom: "1px solid var(--color-border)", fontSize: 12 }}>
                      <span>{a.nombre}</span>
                      <span className={a.stock_actual <= 0 ? "text-danger font-bold" : "text-secondary"}>
                        {a.stock_actual} / {a.stock_minimo}
                      </span>
                    </div>
                  ))}
                </div>
              </div>
            )}
          </div>
        </div>
      </div>

      {ncVenta && (
        <ModalNotaCredito
          ventaId={ncVenta.id}
          ventaNumero={ncVenta.numero}
          esDevolucionInterna={ncVenta.esDevolucion}
          onClose={() => setNcVenta(null)}
          onCreada={() => cargar()}
          toastExito={toastExito}
          toastError={toastError}
          toastWarning={toastWarning}
        />
      )}

      {/* Modal: anular venta (solo ventas no autorizadas) */}
      {anularVentaModal && (
        <div style={{ position: "fixed", inset: 0, background: "rgba(0,0,0,0.5)", display: "flex", alignItems: "center", justifyContent: "center", zIndex: 100 }}
          onClick={(e) => { if (e.target === e.currentTarget) { setAnularVentaModal(null); setAnularMotivo(""); } }}>
          <div className="card" style={{ width: 460 }}>
            <div className="card-header" style={{ color: "var(--color-danger)" }}>⚠ Anular Venta {anularVentaModal.numero}</div>
            <div className="card-body">
              <div style={{ padding: 10, background: "rgba(239,68,68,0.1)", borderRadius: 6, marginBottom: 12, fontSize: 12 }}>
                <strong>Esta accion:</strong>
                <ul style={{ margin: "6px 0", paddingLeft: 20, lineHeight: 1.6 }}>
                  <li>Marca la venta como <strong>ANULADA</strong></li>
                  <li>Reintegra el stock de cada producto</li>
                  <li>Elimina la cuenta por cobrar si existiera</li>
                  <li>Elimina los pagos registrados</li>
                  <li>Descuenta el monto del total de ventas de la caja</li>
                </ul>
                <em style={{ fontSize: 11, color: "var(--color-text-secondary)" }}>
                  No se puede deshacer. Si solo desea devolver parte de los productos, use "Devolver" en su lugar.
                </em>
              </div>
              <label className="text-secondary" style={{ fontSize: 12, display: "block", marginBottom: 6 }}>
                Motivo de la anulacion <span style={{ color: "var(--color-danger)" }}>*</span>
              </label>
              <textarea className="input" rows={3} autoFocus
                placeholder="Ej: Venta duplicada, error del cajero, cliente cancelo..."
                value={anularMotivo}
                onChange={(e) => setAnularMotivo(e.target.value)} />
              <div className="flex gap-2" style={{ justifyContent: "flex-end", marginTop: 12 }}>
                <button className="btn btn-outline" onClick={() => { setAnularVentaModal(null); setAnularMotivo(""); }}>Cancelar</button>
                <button className="btn btn-danger"
                  disabled={!anularMotivo.trim()}
                  onClick={async () => {
                    try {
                      await anularVenta(anularVentaModal.id, anularMotivo.trim());
                      toastExito(`Venta ${anularVentaModal.numero} anulada`);
                      setAnularVentaModal(null); setAnularMotivo("");
                      await cargar();
                    } catch (err) { toastError("Error: " + err); }
                  }}>
                  Confirmar anulacion
                </button>
              </div>
            </div>
          </div>
        </div>
      )}

      {/* Modal: forma de pago para SRI cuando venta es a credito o mixto */}
      {sriPagoVenta && (
        <div style={{ position: "fixed", inset: 0, background: "rgba(0,0,0,0.5)", display: "flex", alignItems: "center", justifyContent: "center", zIndex: 100 }}
          onClick={(e) => { if (e.target === e.currentTarget) setSriPagoVenta(null); }}>
          <div className="card" style={{ width: 480 }}>
            <div className="card-header">Forma de pago para el SRI - {sriPagoVenta.numero}</div>
            <div className="card-body">
              <p style={{ fontSize: 12, color: "var(--color-text-secondary)", marginBottom: 12 }}>
                Esta venta tiene un monto a <strong>credito</strong>. El SRI requiere indicar
                con que metodo se cobrara ese monto. Aparecera en el RIDE de la factura.
              </p>
              <label className="text-secondary" style={{ fontSize: 12, display: "block", marginBottom: 6 }}>
                Forma de pago a registrar:
              </label>
              <select className="input" value={sriFormaPagoCredito}
                onChange={(e) => setSriFormaPagoCredito(e.target.value)}>
                <option value="20">20 - Otros con sistema financiero (transferencia, cheque)</option>
                <option value="01">01 - Sin sistema financiero (efectivo)</option>
                <option value="19">19 - Tarjeta de credito</option>
                <option value="16">16 - Tarjeta de debito</option>
                <option value="17">17 - Dinero electronico</option>
                <option value="18">18 - Tarjeta prepago</option>
                <option value="21">21 - Endoso de titulos</option>
              </select>
              <div style={{ marginTop: 8, padding: 8, background: "var(--color-surface-alt)", borderRadius: 4, fontSize: 11, color: "var(--color-text-secondary)" }}>
                💡 Si no esta seguro, deje "20 - Otros con sistema financiero". Es el codigo mas
                generico y aceptado por el SRI para cobros a credito.
              </div>
              <div className="flex gap-2" style={{ justifyContent: "flex-end", marginTop: 16 }}>
                <button className="btn btn-outline" onClick={() => setSriPagoVenta(null)}>Cancelar</button>
                <button className="btn btn-primary"
                  onClick={async () => {
                    const id = sriPagoVenta.id;
                    const codigo = sriFormaPagoCredito;
                    setSriPagoVenta(null);
                    setReintentandoSri(id);
                    try {
                      const res = await emitirFacturaSri(id, codigo);
                      if (res.exito) {
                        toastExito("Factura autorizada por el SRI");
                        window.dispatchEvent(new CustomEvent("sri-factura-emitida"));
                      } else {
                        toastWarning(`SRI: ${res.mensaje}`);
                      }
                      await cargar();
                    } catch (err) {
                      toastError("Error SRI: " + err);
                    } finally {
                      setReintentandoSri(null);
                    }
                  }}>
                  Emitir Factura
                </button>
              </div>
            </div>
          </div>
        </div>
      )}

      {/* Modal detalle de venta */}
      {ventaDetalle && (
        <div style={{ position: "fixed", inset: 0, background: "rgba(0,0,0,0.5)", display: "flex", alignItems: "center", justifyContent: "center", zIndex: 100 }}
          onClick={(e) => { if (e.target === e.currentTarget) setVentaDetalle(null); }}>
          <div className="card" style={{ width: 550, maxHeight: "85vh", overflow: "auto" }}>
            <div className="card-header flex justify-between items-center">
              <span>Detalle de Venta {ventaDetalle.venta.numero}</span>
              <button className="btn btn-outline" style={{ padding: "2px 8px" }} onClick={() => setVentaDetalle(null)}>x</button>
            </div>
            <div className="card-body">
              {/* Info general */}
              <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: 8, marginBottom: 16, fontSize: 13 }}>
                <div>
                  <span className="text-secondary">Fecha: </span>
                  {ventaDetalle.venta.fecha ? new Date(ventaDetalle.venta.fecha).toLocaleString("es-EC") : "-"}
                </div>
                <div>
                  <span className="text-secondary">Cliente: </span>
                  {ventaDetalle.cliente_nombre || "Consumidor Final"}
                </div>
                <div>
                  <span className="text-secondary">Tipo: </span>
                  {ventaDetalle.venta.tipo_documento === "FACTURA" ? "Factura" : "Nota de Venta"}
                </div>
                <div>
                  <span className="text-secondary">Estado: </span>
                  {ventaDetalle.venta.estado}
                </div>
                {ventaDetalle.venta.numero_factura && (
                  <div style={{ gridColumn: "1 / -1" }}>
                    <span className="text-secondary">Nro. Factura: </span>
                    <strong style={{ color: "var(--color-success)" }}>{ventaDetalle.venta.numero_factura}</strong>
                  </div>
                )}
                {ventaDetalle.venta.observacion && (
                  <div style={{ gridColumn: "1 / -1" }}>
                    <span className="text-secondary">Observacion: </span>
                    {ventaDetalle.venta.observacion}
                  </div>
                )}
              </div>

              {/* Pago */}
              <div style={{ background: "var(--color-surface-alt)", borderRadius: 8, padding: 12, marginBottom: 16 }}>
                <div style={{ fontSize: 12, fontWeight: 600, marginBottom: 6, color: "var(--color-text-secondary)" }}>Informacion de Pago</div>
                <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: 6, fontSize: 13 }}>
                  <div><span className="text-secondary">Forma: </span>{ventaDetalle.venta.forma_pago}</div>
                  <div><span className="text-secondary">Recibido: </span>${ventaDetalle.venta.monto_recibido.toFixed(2)}</div>
                  {ventaDetalle.venta.cambio > 0 && (
                    <div><span className="text-secondary">Cambio: </span>${ventaDetalle.venta.cambio.toFixed(2)}</div>
                  )}
                  {ventaDetalle.venta.banco_nombre && (
                    <div><span className="text-secondary">Banco: </span><strong>{ventaDetalle.venta.banco_nombre}</strong></div>
                  )}
                  {ventaDetalle.venta.referencia_pago && (
                    <div><span className="text-secondary">Referencia: </span><strong>{ventaDetalle.venta.referencia_pago}</strong></div>
                  )}
                </div>
              </div>

              {/* Items */}
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
                  {ventaDetalle.detalles.map((d, i) => (
                    <tr key={i}>
                      <td>{d.nombre_producto || `Producto #${d.producto_id}`}</td>
                      <td className="text-right">{d.cantidad}</td>
                      <td className="text-right">${d.precio_unitario.toFixed(2)}</td>
                      <td className="text-right">${d.subtotal.toFixed(2)}</td>
                    </tr>
                  ))}
                </tbody>
              </table>

              {/* Totales */}
              <div style={{ borderTop: "2px solid var(--color-border)", marginTop: 8, paddingTop: 8, fontSize: 13 }}>
                <div className="flex justify-between">
                  <span className="text-secondary">Subtotal sin IVA:</span>
                  <span>${ventaDetalle.venta.subtotal_sin_iva.toFixed(2)}</span>
                </div>
                <div className="flex justify-between">
                  <span className="text-secondary">Subtotal con IVA:</span>
                  <span>${ventaDetalle.venta.subtotal_con_iva.toFixed(2)}</span>
                </div>
                {ventaDetalle.venta.descuento > 0 && (
                  <div className="flex justify-between">
                    <span className="text-secondary">Descuento:</span>
                    <span>-${ventaDetalle.venta.descuento.toFixed(2)}</span>
                  </div>
                )}
                <div className="flex justify-between">
                  <span className="text-secondary">IVA:</span>
                  <span>${ventaDetalle.venta.iva.toFixed(2)}</span>
                </div>
                <div className="flex justify-between" style={{ fontWeight: 700, fontSize: 16, marginTop: 4 }}>
                  <span>TOTAL:</span>
                  <span className="text-success">${ventaDetalle.venta.total.toFixed(2)}</span>
                </div>
              </div>
            </div>
          </div>
        </div>
      )}

      <ModalEmailCliente
        abierto={!!emailVenta}
        clienteNombre=""
        ventaNumero={emailVenta?.numero_factura || emailVenta?.numero || ""}
        onEnviar={async (email) => {
          if (!emailVenta?.id) return;
          setEnviandoEmail(true);
          try {
            await enviarNotificacionSri(emailVenta.id, email);
            toastExito(`Email enviado a ${email}`);
            setEmailVenta(null);
            await cargar();
          } catch (err) {
            const errStr = String(err);
            if (errStr.startsWith("ENCOLADO:")) {
              toastWarning("Email pendiente, se reintentara automaticamente");
              setEmailVenta(null);
            } else {
              toastError("Error enviando email: " + errStr);
            }
          } finally {
            setEnviandoEmail(false);
          }
        }}
        onOmitir={() => setEmailVenta(null)}
        enviando={enviandoEmail}
      />
    </>
  );
}
