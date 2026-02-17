import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listarVentasDia, listarVentasPeriodo, imprimirTicket, imprimirTicketPdf, exportarVentasCsv, emitirFacturaSri, obtenerXmlFirmado, imprimirRide, enviarNotificacionSri, obtenerConfig, procesarEmailsPendientes, listarNotasCreditoDia, emitirNotaCreditoSri, generarRideNcPdf, listarVentasSesionCaja, resumenSesionCaja, listarNotasCreditoSesionCaja } from "../services/api";
import { resumenDiario, resumenPeriodo, productosMasVendidosReporte, alertasStockBajo } from "../services/api";
import { save } from "@tauri-apps/plugin-dialog";
import { useToast } from "../components/Toast";
import { useSesion } from "../contexts/SesionContext";
import ModalEmailCliente from "../components/ModalEmailCliente";
import ModalNotaCredito from "../components/ModalNotaCredito";
import type { ResumenDiario, ResumenPeriodo, ProductoMasVendido, AlertaStock } from "../services/api";
import type { Venta, NotaCreditoInfo } from "../types";

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
  const { esAdmin } = useSesion();
  const [ventas, setVentas] = useState<Venta[]>([]);
  const [fechaDesde, setFechaDesde] = useState(fechaHoy);
  const [fechaHasta, setFechaHasta] = useState(fechaHoy);
  const [reintentandoSri, setReintentandoSri] = useState<number | null>(null);
  const [reintentandoEmail, setReintentandoEmail] = useState<number | null>(null);
  const [emailVenta, setEmailVenta] = useState<Venta | null>(null);
  const [enviandoEmail, setEnviandoEmail] = useState(false);
  const [resumen, setResumen] = useState<ResumenDiario | null>(null);
  const [resumenRango, setResumenRango] = useState<ResumenPeriodo | null>(null);
  const [topProductos, setTopProductos] = useState<ProductoMasVendido[]>([]);
  const [alertas, setAlertas] = useState<AlertaStock[]>([]);
  const [ticketUsarPdf, setTicketUsarPdf] = useState(false);
  const [notasCredito, setNotasCredito] = useState<NotaCreditoInfo[]>([]);
  const [ncVenta, setNcVenta] = useState<{ id: number; numero: string } | null>(null);
  const [reintentandoNcSri, setReintentandoNcSri] = useState<number | null>(null);

  const esRango = fechaDesde !== fechaHasta;

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
      const [v, r, top, a, ncs] = await Promise.all([
        listarVentasPeriodo(fechaDesde, fechaHasta),
        resumenPeriodo(fechaDesde, fechaHasta),
        productosMasVendidosReporte(fechaDesde, fechaHasta, 5),
        alertasStockBajo(),
        listarNotasCreditoDia(fechaDesde).catch(() => [] as NotaCreditoInfo[]),
      ]);
      setVentas(v);
      setResumen(null);
      setResumenRango(r);
      setTopProductos(top);
      setAlertas(a);
      setNotasCredito(ncs);
    } else {
      const [v, r, top, a, ncs] = await Promise.all([
        listarVentasDia(fechaDesde),
        resumenDiario(fechaDesde),
        productosMasVendidosReporte(fechaDesde, fechaDesde, 5),
        alertasStockBajo(),
        listarNotasCreditoDia(fechaDesde).catch(() => [] as NotaCreditoInfo[]),
      ]);
      setVentas(v);
      setResumen(r);
      setResumenRango(null);
      setTopProductos(top);
      setAlertas(a);
      setNotasCredito(ncs);
    }
  };

  useEffect(() => { cargar(); }, [fechaDesde, fechaHasta]);
  useEffect(() => {
    obtenerConfig().then((cfg) => setTicketUsarPdf(cfg.ticket_usar_pdf === "1")).catch(() => {});
  }, []);

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
                  <div className="text-secondary" style={{ fontSize: 11 }}>Fiado</div>
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
                <div className="text-xl font-bold" style={{ color: "#dc2626" }}>-${r.total_notas_credito.toFixed(2)}</div>
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
                  <div className="text-xl font-bold" style={{ color: resumenRango.total_gastos > 0 ? "#ef4444" : undefined }}>
                    ${resumenRango.total_gastos.toFixed(2)}
                  </div>
                </div>
              </>
            )}
          </div>
        )}

        <div style={{ display: "grid", gridTemplateColumns: "1fr 300px", gap: 16 }}>
          {/* Tabla de ventas */}
          <div className="card">
            <div className="card-header flex justify-between items-center">
              <span>Detalle de Ventas</span>
              <span className="text-secondary" style={{ fontSize: 12 }}>{ventas.length} registro{ventas.length !== 1 ? "s" : ""}</span>
            </div>
            <div style={{ maxHeight: 400, overflow: "auto" }}>
              <table className="table">
                <thead>
                  <tr>
                    <th>Numero</th>
                    <th>{esRango ? "Fecha" : "Hora"}</th>
                    <th>Tipo</th>
                    <th>Pago</th>
                    <th className="text-right">Total</th>
                    <th></th>
                  </tr>
                </thead>
                <tbody>
                  {ventas.map((v) => (
                    <tr key={v.id}>
                      <td>
                        <strong>{v.numero}</strong>
                        {v.numero_factura && (
                          <div style={{ fontSize: 10, color: "#166534" }}>{v.numero_factura}</div>
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
                          background: v.tipo_documento === "FACTURA" ? "#dbeafe" : "#f1f5f9",
                          color: v.tipo_documento === "FACTURA" ? "#1e40af" : "#64748b",
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
                            background: v.estado_sri === "AUTORIZADA" ? "#dcfce7"
                              : v.estado_sri === "PENDIENTE" ? "#fef3c7"
                              : v.estado_sri === "RECHAZADA" ? "#fee2e2"
                              : "#f1f5f9",
                            color: v.estado_sri === "AUTORIZADA" ? "#166534"
                              : v.estado_sri === "PENDIENTE" ? "#92400e"
                              : v.estado_sri === "RECHAZADA" ? "#dc2626"
                              : "#94a3b8",
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
                              color: "#2563eb", borderColor: "#93c5fd",
                            }}
                              disabled={reintentandoSri === v.id}
                              onClick={async () => {
                                if (!v.id) return;
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
                                  background: "#dcfce7", color: "#166534", fontWeight: 600,
                                }} title="Email enviado">
                                  Enviado
                                </span>
                              ) : (
                                <button className="btn btn-outline" style={{
                                  padding: "2px 6px", fontSize: 10,
                                  color: "#d97706", borderColor: "#fbbf24",
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
                              {!notasCredito.some(nc => nc.venta_id === v.id) ? (
                                <button className="btn btn-outline" style={{
                                  padding: "2px 6px", fontSize: 10,
                                  color: "#dc2626", borderColor: "#fca5a5",
                                }}
                                  title="Crear Nota de Credito"
                                  onClick={() => v.id && setNcVenta({ id: v.id, numero: v.numero_factura || v.numero })}>
                                  NC
                                </button>
                              ) : (
                                <span style={{
                                  fontSize: 9, padding: "2px 5px", borderRadius: 3,
                                  background: "#fef2f2", color: "#dc2626", fontWeight: 600,
                                }} title="Ya tiene nota de credito">
                                  NC
                                </span>
                              )}
                            </>
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
                  ))}
                  {ventas.length === 0 && (
                    <tr>
                      <td colSpan={6} className="text-center text-secondary" style={{ padding: 30 }}>
                        No hay ventas para {esRango ? "este periodo" : "esta fecha"}
                      </td>
                    </tr>
                  )}
                </tbody>
              </table>
            </div>
          </div>

          {/* Panel lateral */}
          <div style={{ display: "flex", flexDirection: "column", gap: 16 }}>
            {/* Notas de Crédito */}
            {notasCredito.length > 0 && (
              <div className="card" style={{ borderColor: "#fca5a5" }}>
                <div className="card-header" style={{ background: "#fef2f2", color: "#dc2626" }}>
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
                          <div className="font-bold" style={{ color: "#dc2626" }}>-${nc.total.toFixed(2)}</div>
                          <span style={{
                            fontSize: 9, padding: "1px 4px", borderRadius: 3, fontWeight: 600,
                            background: nc.estado_sri === "AUTORIZADA" ? "#dcfce7" : nc.estado_sri === "PENDIENTE" ? "#fef3c7" : "#fee2e2",
                            color: nc.estado_sri === "AUTORIZADA" ? "#166534" : nc.estado_sri === "PENDIENTE" ? "#92400e" : "#dc2626",
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
                            color: "#2563eb", borderColor: "#93c5fd",
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
              <div className="card" style={{ borderColor: "#fbbf24" }}>
                <div className="card-header" style={{ background: "#fffbeb", color: "#92400e" }}>
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
          onClose={() => setNcVenta(null)}
          onCreada={() => cargar()}
          toastExito={toastExito}
          toastError={toastError}
          toastWarning={toastWarning}
        />
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
