import { useState, useEffect, Fragment } from "react";
import { useNavigate } from "react-router-dom";
import { abrirCaja, cerrarCaja, obtenerCajaAbierta, imprimirReporteCaja, imprimirReporteCajaPdf, obtenerConfig, registrarRetiro, registrarIngresoCaja, listarRetirosCaja, listarCuentasBanco, confirmarDeposito, obtenerUltimoCierre, historialDescuadresCaja, listarSesionesCaja, registrarDepositoCierre, listarEventosCaja, obtenerResumenCaja } from "../services/api";
import { useToast } from "../components/Toast";
import { useSesion } from "../contexts/SesionContext";
import Modal from "../components/Modal";
import { ask } from "@tauri-apps/plugin-dialog";
import type { Caja, ResumenCaja } from "../types";

export default function CajaPage() {
  const navigate = useNavigate();
  const { toastExito, toastError } = useToast();
  const { sesion, cerrarSesion, esAdmin } = useSesion();
  const [cajaAbierta, setCajaAbierta] = useState<Caja | null>(null);
  const [montoInicial, setMontoInicial] = useState("");
  const [montoReal, setMontoReal] = useState("");
  const [observacion, setObservacion] = useState("");
  const [resumen, setResumen] = useState<ResumenCaja | null>(null);
  const [cargando, setCargando] = useState(true);
  const [confirmarCierre, setConfirmarCierre] = useState(false);
  const [ticketUsarPdf, setTicketUsarPdf] = useState(false);
  const [imprimiendo, setImprimiendo] = useState(false);
  const [mostrarRetiro, setMostrarRetiro] = useState(false);
  const [retiroMonto, setRetiroMonto] = useState("");
  const [retiroMotivo, setRetiroMotivo] = useState("");
  const [retiroBancoId, setRetiroBancoId] = useState<number | null>(null);
  const [retiroReferencia, setRetiroReferencia] = useState("");
  const [retiros, setRetiros] = useState<any[]>([]);
  // v2.3.46: ingreso manual a caja (solo admin)
  const [mostrarIngreso, setMostrarIngreso] = useState(false);
  const [ingresoMonto, setIngresoMonto] = useState("");
  const [ingresoMotivo, setIngresoMotivo] = useState("");
  const [cuentasBanco, setCuentasBanco] = useState<any[]>([]);
  const [confirmandoRetiroId, setConfirmandoRetiroId] = useState<number | null>(null);
  const [confirmRef, setConfirmRef] = useState("");
  const [confirmImg, setConfirmImg] = useState<string | null>(null);
  // Anti-fraude: info del ultimo cierre + motivos
  const [ultimoCierre, setUltimoCierre] = useState<any>(null);
  const [motivoApertura, setMotivoApertura] = useState("");
  const [motivoDescuadre, setMotivoDescuadre] = useState("");
  // PIN supervisor modal (cuando rol no tiene permiso cerrar_caja o descuadre alto)
  const [pinPrompt, setPinPrompt] = useState<{ tipo: "permiso" | "descuadre"; mensaje: string } | null>(null);
  const [pinSupervisor, setPinSupervisor] = useState("");
  // Deposito post-cierre
  const [mostrarDeposito, setMostrarDeposito] = useState(false);
  const [depositoMonto, setDepositoMonto] = useState("");
  const [depositoBancoId, setDepositoBancoId] = useState<number | null>(null);
  const [depositoReferencia, setDepositoReferencia] = useState("");
  // Lista de depositos / retiros del cierre actual (para mostrar en la card de resumen)
  const [resumenRetiros, setResumenRetiros] = useState<any[]>([]);
  // Desglose detallado de la caja abierta (gastos, retiros, ventas por forma de pago)
  // para mostrar al cajero el por que del monto esperado.
  const [breakdownCaja, setBreakdownCaja] = useState<any>(null);
  const [breakdownExpandido, setBreakdownExpandido] = useState(false);
  // Modal historial descuadres
  const [mostrarHistorial, setMostrarHistorial] = useState(false);
  const [historialTab, setHistorialTab] = useState<"sesiones" | "descuadres">("sesiones");
  const [historialData, setHistorialData] = useState<any>(null);
  const [sesionesData, setSesionesData] = useState<any[]>([]);
  const [sesionExpandida, setSesionExpandida] = useState<number | null>(null);
  const [eventosCache, setEventosCache] = useState<Record<number, any[]>>({});
  const [historialDesde, setHistorialDesde] = useState(() => {
    const d = new Date(); d.setDate(d.getDate() - 30);
    return d.toISOString().slice(0, 10);
  });
  const [historialHasta, setHistorialHasta] = useState(() => new Date().toISOString().slice(0, 10));

  const cargar = async () => {
    setCargando(true);
    try {
      const caja = await obtenerCajaAbierta().catch(() => null);
      setCajaAbierta(caja);
      // Si no hay caja abierta, cargar info del ultimo cierre para sugerencia
      if (!caja) {
        try {
          const uc = await obtenerUltimoCierre();
          setUltimoCierre(uc);
          if (uc?.monto_real != null) {
            setMontoInicial(uc.monto_real.toString());
          }
        } catch { /* ignore */ }
      }
    } catch (err) {
      // Cualquier error al cargar no debe romper la pagina
      console.error("[CajaPage] Error en cargar():", err);
      toastError("Error cargando caja: " + err);
    }
    setCargando(false);
  };

  useEffect(() => { cargar(); }, []);

  // Auto-refresh: recargar caja cuando la ventana recupera el foco o vuelve a ser visible.
  // Evita que el usuario tenga que navegar afuera y volver para ver el monto_esperado
  // actualizado despues de hacer cambios en otra ventana (ej: una venta en POS).
  useEffect(() => {
    const refrescar = () => {
      obtenerCajaAbierta()
        .then((c) => {
          setCajaAbierta(c);
          if (c?.id) {
            listarRetirosCaja(c.id).then(setRetiros).catch(() => {});
            obtenerResumenCaja(c.id).then(setBreakdownCaja).catch(() => {});
          }
        })
        .catch(() => {});
    };
    const onVisibility = () => {
      if (document.visibilityState === "visible") refrescar();
    };
    window.addEventListener("focus", refrescar);
    document.addEventListener("visibilitychange", onVisibility);
    return () => {
      window.removeEventListener("focus", refrescar);
      document.removeEventListener("visibilitychange", onVisibility);
    };
  }, []);

  // Soporte tecla Esc para cerrar drawer historial
  useEffect(() => {
    if (!mostrarHistorial) return;
    const handler = (e: KeyboardEvent) => {
      if (e.key === "Escape") setMostrarHistorial(false);
    };
    window.addEventListener("keydown", handler);
    return () => window.removeEventListener("keydown", handler);
  }, [mostrarHistorial]);
  useEffect(() => {
    obtenerConfig().then((cfg) => setTicketUsarPdf(cfg.ticket_usar_pdf === "1")).catch(() => {});
  }, []);
  useEffect(() => {
    if (cajaAbierta && cajaAbierta.id) {
      listarRetirosCaja(cajaAbierta.id).then(setRetiros).catch(() => {});
      listarCuentasBanco().then(setCuentasBanco).catch(() => {});
      // Cargar desglose detallado para mostrar en el cierre
      obtenerResumenCaja(cajaAbierta.id).then(setBreakdownCaja).catch(() => setBreakdownCaja(null));
    } else {
      setBreakdownCaja(null);
    }
  }, [cajaAbierta]);

  const handleAbrir = async () => {
    const monto = parseFloat(montoInicial) || 0;
    // Validar motivo si difiere del ultimo cierre
    const cierreEsperado = ultimoCierre?.monto_real;
    if (cierreEsperado != null && Math.abs(monto - cierreEsperado) > 0.01 && motivoApertura.trim().length < 5) {
      toastError(`El monto inicial difiere del cierre anterior ($${cierreEsperado.toFixed(2)}). Debe explicar el motivo (mínimo 5 caracteres).`);
      return;
    }
    try {
      const caja = await abrirCaja(monto, motivoApertura.trim() || undefined);
      setCajaAbierta(caja);
      setMontoInicial("");
      setMotivoApertura("");
      setUltimoCierre(null);
      // Limpiar retiros y resumen de cierre anterior para evitar mezclas visuales
      setRetiros([]);
      setResumenRetiros([]);
      setResumen(null);
      toastExito("Caja abierta correctamente");
    } catch (err) {
      const msg = String(err);
      if (msg.includes("DESCUADRE_APERTURA")) {
        // Backend ya devolvio mensaje detallado, mostrar tal cual
        toastError(msg.split(":").slice(3).join(":").trim() || msg);
      } else {
        toastError("Error: " + err);
      }
    }
  };

  const intentarCerrarCaja = async (pinOverride?: string) => {
    const monto = parseFloat(montoReal) || 0;
    // monto_esperado del backend YA es el valor recalculado correcto
    // (= monto_inicial + ventas_efectivo + cobros_efectivo - gastos - retiros).
    // NO restar retiros aqui porque ya estan dentro de monto_esperado.
    const esperado = cajaAbierta?.monto_esperado ?? 0;
    const dif = monto - esperado;
    if (Math.abs(dif) > 0.01 && motivoDescuadre.trim().length < 5) {
      toastError(`Hay un descuadre de $${dif.toFixed(2)}. Debe explicar el motivo (mínimo 5 caracteres).`);
      return false;
    }
    try {
      const res = await cerrarCaja(monto, observacion || undefined, motivoDescuadre.trim() || undefined, undefined, pinOverride);
      setResumen(res);
      setCajaAbierta(null);
      setMontoReal("");
      setObservacion("");
      setMotivoDescuadre("");
      // Cargar lista de depositos/retiros para mostrar en la card del resumen
      // (incluye los que se hicieron antes del cierre + los nuevos post-cierre)
      if (res.caja?.id) {
        listarRetirosCaja(res.caja.id).then(setResumenRetiros).catch(() => setResumenRetiros([]));
      }
      toastExito("Caja cerrada correctamente");
      return true;
    } catch (err) {
      const msg = String(err);
      if (msg.includes("REQUIERE_PIN_SUPERVISOR")) {
        // Pedir PIN supervisor (rol no tiene permiso cerrar_caja)
        setPinPrompt({ tipo: "permiso", mensaje: msg.split(":").slice(1).join(":").trim() });
      } else if (msg.includes("REQUIERE_PIN_DESCUADRE")) {
        // Pedir PIN supervisor (descuadre supera umbral)
        setPinPrompt({ tipo: "descuadre", mensaje: msg.split(":").slice(3).join(":").trim() });
      } else if (msg.includes("DESCUADRE_CIERRE")) {
        toastError(msg.split(":").slice(3).join(":").trim() || msg);
      } else {
        toastError("Error: " + err);
      }
      return false;
    }
  };

  const handleCerrar = async () => {
    setConfirmarCierre(false);
    await intentarCerrarCaja();
  };

  // v2.3.53: helper para imprimir ticket de cierre preguntando al cajero
  // si quiere version resumida (solo totales — ahorra papel) o detallada
  // (incluye lista de cada venta, gasto, retiro, cobro).
  const imprimirTicketCierreCaja = async (cajaId: number) => {
    const detallado = await ask(
      "¿Querés imprimir el reporte DETALLADO (incluye lista de cada venta, gasto y retiro)?\n\n" +
      "• Sí: detallado completo (más papel, mejor para auditoría)\n" +
      "• No: resumido — solo totales (ahorra papel, suficiente para el día a día)",
      { title: "Tipo de reporte de cierre", kind: "info" }
    );
    setImprimiendo(true);
    try {
      await imprimirReporteCaja(cajaId, detallado);
      toastExito(detallado ? "Reporte detallado impreso" : "Reporte resumido impreso");
    } catch (err) {
      toastError("Error imprimiendo: " + err);
    } finally {
      setImprimiendo(false);
    }
  };

  const handleFinalizarTurno = async () => {
    setResumen(null);
    setResumenRetiros([]);
    setRetiros([]);
    // El backend ya cerro la sesion, solo actualizamos el frontend
    await cerrarSesion();
  };

  const handleRetiro = async () => {
    const monto = parseFloat(retiroMonto);
    if (!monto || monto <= 0) { toastError("Monto inválido"); return; }
    try {
      await registrarRetiro(monto, retiroMotivo, retiroBancoId || undefined, retiroReferencia || undefined);
      toastExito(`Retiro de $${monto.toFixed(2)} registrado`);
      setMostrarRetiro(false);
      setRetiroMonto(""); setRetiroMotivo(""); setRetiroBancoId(null); setRetiroReferencia("");
      // Refrescar caja Y retiros para que monto_esperado se actualice inmediatamente
      // sin que el usuario tenga que navegar afuera y volver.
      const c = await obtenerCajaAbierta();
      setCajaAbierta(c);
      if (c?.id) {
        listarRetirosCaja(c.id).then(setRetiros).catch(() => {});
        obtenerResumenCaja(c.id).then(setBreakdownCaja).catch(() => {});
      }
    } catch (err) { toastError("Error: " + err); }
  };

  // v2.3.46: registrar ingreso manual (solo admin)
  const handleIngreso = async () => {
    const monto = parseFloat(ingresoMonto);
    if (!monto || monto <= 0) { toastError("Monto inválido"); return; }
    if (ingresoMotivo.trim().length < 5) { toastError("Motivo: mínimo 5 caracteres"); return; }
    try {
      await registrarIngresoCaja(monto, ingresoMotivo.trim());
      toastExito(`Ingreso de $${monto.toFixed(2)} registrado`);
      setMostrarIngreso(false);
      setIngresoMonto(""); setIngresoMotivo("");
      const c = await obtenerCajaAbierta();
      setCajaAbierta(c);
      if (c?.id) {
        listarRetirosCaja(c.id).then(setRetiros).catch(() => {});
        obtenerResumenCaja(c.id).then(setBreakdownCaja).catch(() => {});
      }
    } catch (err) { toastError("Error: " + err); }
  };

  const handleConfirmarDeposito = async () => {
    if (!confirmRef.trim()) { toastError("Ingrese el número de comprobante"); return; }
    try {
      await confirmarDeposito(confirmandoRetiroId!, confirmRef, confirmImg || undefined);
      toastExito("Depósito confirmado");
      setConfirmandoRetiroId(null); setConfirmRef(""); setConfirmImg(null);
      if (cajaAbierta?.id) listarRetirosCaja(cajaAbierta.id).then(setRetiros).catch(() => {});
    } catch (err) { toastError("Error: " + err); }
  };

  const handleImagenComprobante = (e: React.ChangeEvent<HTMLInputElement>) => {
    const file = e.target.files?.[0];
    if (!file) return;
    if (file.size > 500000) { toastError("Imagen muy grande (max 500KB)"); return; }
    const reader = new FileReader();
    reader.onload = () => setConfirmImg(reader.result as string);
    reader.readAsDataURL(file);
  };

  if (cargando) return <div className="page-body text-center text-secondary">Cargando...</div>;

  return (
    <>
      <div className="page-header">
        <h2>Caja</h2>
        <div className="flex gap-2 items-center">
          {cajaAbierta && (
            <span style={{
              padding: "4px 12px",
              borderRadius: 4,
              background: "rgba(34, 197, 94, 0.15)",
              color: "var(--color-success)",
              fontSize: 13,
              fontWeight: 600,
            }}>
              CAJA ABIERTA
            </span>
          )}
          {cajaAbierta?.usuario && (
            <span className="text-secondary" style={{ fontSize: 12 }}>
              Abierta por: {cajaAbierta.usuario}
            </span>
          )}
          <button className="btn btn-outline" style={{ fontSize: 11 }}
            onClick={async () => {
              setMostrarHistorial(true);
              setHistorialTab("sesiones");
              try {
                const [sesiones, descuadres] = await Promise.all([
                  listarSesionesCaja(historialDesde, historialHasta),
                  historialDescuadresCaja(historialDesde, historialHasta),
                ]);
                setSesionesData(sesiones);
                setHistorialData(descuadres);
              } catch (err) { toastError("Error: " + err); }
            }}>
            📊 Historial caja
          </button>
        </div>
      </div>
      <div className="page-body">
        {resumen && (
          <div className="card mb-4" style={{ maxWidth: 500, margin: "40px auto" }}>
            <div className="card-header">Resumen de Cierre de Caja</div>
            <div className="card-body">
              <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: 16 }}>
                <div>
                  <span className="text-secondary">Total ventas:</span>
                  <div className="text-lg font-bold">${resumen.total_ventas.toFixed(2)}</div>
                </div>
                <div>
                  <span className="text-secondary">Numero de ventas:</span>
                  <div className="text-lg font-bold">{resumen.num_ventas}</div>
                </div>
                <div>
                  <span className="text-secondary">Efectivo ventas:</span>
                  <div className="text-lg font-bold">${resumen.total_efectivo.toFixed(2)}</div>
                </div>
                <div>
                  <span className="text-secondary">Total gastos:</span>
                  <div className="text-lg font-bold">${resumen.total_gastos.toFixed(2)}</div>
                </div>
                {(resumen.total_retiros ?? 0) > 0 && (
                  <div>
                    <span className="text-secondary" style={{ color: "var(--color-danger)" }}>(-) Retiros:</span>
                    <div className="text-lg font-bold" style={{ color: "var(--color-danger)" }}>${resumen.total_retiros.toFixed(2)}</div>
                  </div>
                )}
                {(resumen.total_cobros_efectivo > 0 || resumen.total_cobros_banco > 0) && (
                  <>
                    <div>
                      <span className="text-secondary">Cobros en efectivo:</span>
                      <div className="text-lg font-bold" style={{ color: "var(--color-success)" }}>
                        ${resumen.total_cobros_efectivo.toFixed(2)}
                      </div>
                    </div>
                    <div>
                      <span className="text-secondary">Cobros en banco:</span>
                      <div className="text-lg font-bold" style={{ color: "var(--color-primary)" }}>
                        ${resumen.total_cobros_banco.toFixed(2)}
                      </div>
                    </div>
                  </>
                )}
              </div>
              <div style={{
                borderTop: "1px solid var(--color-border)", marginTop: 16, paddingTop: 16,
                display: "grid", gridTemplateColumns: "1fr 1fr", gap: 16,
              }}>
                <div>
                  <span className="text-secondary">Monto esperado:</span>
                  <div className="text-lg font-bold">${resumen.caja.monto_esperado.toFixed(2)}</div>
                </div>
                <div>
                  <span className="text-secondary">Diferencia:</span>
                  <div className={`text-lg font-bold ${(resumen.caja.diferencia ?? 0) >= 0 ? "text-success" : "text-danger"}`}>
                    ${(resumen.caja.diferencia ?? 0).toFixed(2)}
                  </div>
                </div>
              </div>

              {/* Lista de depositos / retiros del cierre — auto-refresca al registrar
                  un deposito post-cierre, asi el usuario ve inmediatamente el saldo restante. */}
              {resumenRetiros.length > 0 && (() => {
                const totalDep = resumenRetiros.reduce((s: number, r: any) => s + r.monto, 0);
                const efectivoRestante = (resumen.caja.monto_real ?? 0) - totalDep;
                return (
                  <div style={{
                    borderTop: "1px solid var(--color-border)", marginTop: 12, paddingTop: 12,
                  }}>
                    <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: 6 }}>
                      <strong style={{ fontSize: 13 }}>Depósitos / retiros registrados ({resumenRetiros.length})</strong>
                      <span style={{ fontWeight: 700, color: "var(--color-danger)" }}>
                        -${totalDep.toFixed(2)}
                      </span>
                    </div>
                    <div style={{ maxHeight: 160, overflowY: "auto", border: "1px solid var(--color-border)", borderRadius: 6 }}>
                      {resumenRetiros.map((r: any) => (
                        <div key={r.id} style={{
                          display: "grid",
                          gridTemplateColumns: "60px 80px 1fr 80px",
                          gap: 8, padding: "6px 10px", fontSize: 12,
                          borderBottom: "1px solid var(--color-border)",
                        }}>
                          <span>{r.fecha?.slice(11, 16)}</span>
                          <span style={{ color: "var(--color-danger)", fontWeight: 600 }}>-${r.monto.toFixed(2)}</span>
                          <span style={{ overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>
                            {r.banco_nombre || r.motivo || "—"}
                          </span>
                          <span style={{
                            fontSize: 10, padding: "2px 6px", borderRadius: 4, textAlign: "center",
                            background: r.estado === "DEPOSITADO" ? "rgba(34,197,94,0.15)"
                              : r.estado === "EN_TRANSITO" ? "rgba(245,158,11,0.15)"
                              : "rgba(148,148,148,0.15)",
                            color: r.estado === "DEPOSITADO" ? "var(--color-success)"
                              : r.estado === "EN_TRANSITO" ? "var(--color-warning)"
                              : "var(--color-text-secondary)",
                          }}>
                            {r.estado === "DEPOSITADO" ? "OK" : r.estado === "EN_TRANSITO" ? "Pdte" : "Sin dep"}
                          </span>
                        </div>
                      ))}
                    </div>
                    <div style={{ display: "flex", justifyContent: "space-between", marginTop: 8, fontSize: 13 }}>
                      <span className="text-secondary">Efectivo restante en caja:</span>
                      <strong style={{ color: efectivoRestante >= 0 ? "var(--color-success)" : "var(--color-danger)" }}>
                        ${efectivoRestante.toFixed(2)}
                      </strong>
                    </div>
                  </div>
                );
              })()}

              {/* Botones de impresion */}
              <div className="flex gap-2 mt-4">
                <button
                  className="btn btn-outline"
                  style={{ flex: 1 }}
                  disabled={imprimiendo || !resumen.caja.id}
                  onClick={async () => {
                    if (!resumen.caja.id) return;
                    if (ticketUsarPdf) {
                      setImprimiendo(true);
                      try {
                        await imprimirReporteCajaPdf(resumen.caja.id);
                        toastExito("Reporte PDF generado");
                      } catch (err) {
                        toastError("Error imprimiendo: " + err);
                      } finally {
                        setImprimiendo(false);
                      }
                    } else {
                      // Ticket termico: preguntar resumido o detallado
                      await imprimirTicketCierreCaja(resumen.caja.id);
                    }
                  }}
                >
                  {imprimiendo ? "Imprimiendo..." : ticketUsarPdf ? "Imprimir Ticket (PDF)" : "Imprimir Ticket"}
                </button>
                <button
                  className="btn btn-outline"
                  style={{ flex: 1 }}
                  disabled={imprimiendo || !resumen.caja.id}
                  onClick={async () => {
                    if (!resumen.caja.id) return;
                    setImprimiendo(true);
                    try {
                      await imprimirReporteCajaPdf(resumen.caja.id);
                      toastExito("Reporte A4 generado");
                    } catch (err) {
                      toastError("Error generando PDF: " + err);
                    } finally {
                      setImprimiendo(false);
                    }
                  }}
                >
                  {imprimiendo ? "..." : "Reporte A4 (PDF)"}
                </button>
              </div>

              {/* Botón depósito a banco */}
              <button
                className="btn btn-outline mt-3"
                style={{ width: "100%", borderColor: "var(--color-primary)", color: "var(--color-primary)", fontWeight: 600 }}
                onClick={() => {
                  setDepositoMonto((resumen.caja.monto_real || 0).toFixed(2));
                  setMostrarDeposito(true);
                }}
              >
                🏦 Registrar depósito a banco
              </button>

              <button
                className="btn btn-primary btn-lg mt-4"
                style={{ width: "100%" }}
                onClick={handleFinalizarTurno}
              >
                Finalizar Turno
              </button>
            </div>
          </div>
        )}

        {!resumen && !cajaAbierta && (
          <div className="card" style={{ maxWidth: 460, margin: "40px auto" }}>
            <div className="card-header">Abrir Caja</div>
            <div className="card-body">
              {sesion && (
                <div className="mb-4" style={{ fontSize: 13 }}>
                  <span className="text-secondary">Cajero:</span>
                  <span className="font-bold" style={{ marginLeft: 8 }}>{sesion.nombre}</span>
                </div>
              )}

              {/* Banner: info del ultimo cierre */}
              {ultimoCierre && (
                <div style={{
                  padding: "10px 12px", marginBottom: 12, borderRadius: 6,
                  background: "rgba(59,130,246,0.1)", border: "1px solid rgba(59,130,246,0.3)",
                  fontSize: 12,
                }}>
                  <div style={{ fontWeight: 600, marginBottom: 4 }}>📋 Cierre anterior</div>
                  <div>
                    Monto contado: <strong style={{ color: "var(--color-primary)" }}>${ultimoCierre.monto_real?.toFixed(2)}</strong>
                  </div>
                  {ultimoCierre.cerrada_at && (
                    <div style={{ color: "var(--color-text-secondary)" }}>
                      Fecha: {new Date(ultimoCierre.cerrada_at).toLocaleString("es-EC")}
                    </div>
                  )}
                  {ultimoCierre.usuario_cierre && (
                    <div style={{ color: "var(--color-text-secondary)" }}>
                      Cerró: {ultimoCierre.usuario_cierre}
                    </div>
                  )}
                  <div style={{ marginTop: 6, fontSize: 11, color: "var(--color-text-secondary)" }}>
                    El monto inicial debería coincidir con este cierre. Si no, deberá justificar la diferencia.
                  </div>
                </div>
              )}

              <label className="text-secondary" style={{ fontSize: 12 }}>Monto inicial en caja</label>
              <input
                className="input input-lg mt-2"
                type="number"
                step="0.01"
                placeholder="0.00"
                value={montoInicial}
                onChange={(e) => setMontoInicial(e.target.value)}
                onKeyDown={(e) => { if (e.key === "Enter") handleAbrir(); }}
              />

              {/* Alerta + motivo si difiere */}
              {ultimoCierre?.monto_real != null && montoInicial &&
                Math.abs((parseFloat(montoInicial) || 0) - ultimoCierre.monto_real) > 0.01 && (
                <div style={{ marginTop: 10 }}>
                  <div style={{
                    padding: "8px 10px", borderRadius: 6,
                    background: "rgba(239,68,68,0.1)", border: "1px solid rgba(239,68,68,0.4)",
                    color: "var(--color-danger)", fontSize: 12, marginBottom: 6,
                  }}>
                    ⚠ Diferencia: <strong>${((parseFloat(montoInicial) || 0) - ultimoCierre.monto_real).toFixed(2)}</strong>
                    {" "} (cierre: ${ultimoCierre.monto_real.toFixed(2)} → apertura: ${(parseFloat(montoInicial) || 0).toFixed(2)})
                  </div>
                  <label className="text-secondary" style={{ fontSize: 12 }}>Motivo de la diferencia *</label>
                  <textarea
                    className="input mt-2"
                    placeholder="Ej: Faltaron $X tomados por el dueño / sobraron $X de propina / etc."
                    value={motivoApertura}
                    onChange={(e) => setMotivoApertura(e.target.value)}
                    rows={2} />
                </div>
              )}

              <button className="btn btn-success btn-lg mt-4" style={{ width: "100%" }} onClick={handleAbrir}>
                Abrir Caja
              </button>
            </div>
          </div>
        )}

        {!resumen && cajaAbierta && (
          <div className="card" style={{ maxWidth: 500, margin: "40px auto" }}>
            <div className="card-header">Cerrar Caja</div>
            <div className="card-body">
              {/* Banner de apertura: ayuda al cajero a entender que el desglose
                  cubre solo desde esta hora, no todo el dia. Ventas anteriores
                  pertenecen a sesiones cerradas y se ven en VentasDia. */}
              {cajaAbierta.fecha_apertura && (
                <div style={{
                  padding: "8px 12px", marginBottom: 10, borderRadius: 6,
                  background: "rgba(59,130,246,0.08)", border: "1px solid rgba(59,130,246,0.25)",
                  fontSize: 12, color: "var(--color-primary)",
                }}>
                  ⏰ Caja abierta el <strong>{cajaAbierta.fecha_apertura}</strong>
                  <div style={{ fontSize: 11, color: "var(--color-text-secondary)", marginTop: 2 }}>
                    El desglose cuenta solo ventas/gastos/retiros desde esa hora.
                    Para ver TODAS las ventas del día (incluyendo sesiones cerradas), ir a "Ventas".
                  </div>
                </div>
              )}
              <div className="mb-4">
                <span className="text-secondary">Monto inicial:</span>
                <span className="font-bold" style={{ marginLeft: 8 }}>${cajaAbierta.monto_inicial.toFixed(2)}</span>
              </div>
              <div className="mb-4" style={{
                padding: "10px 14px", background: "rgba(34, 197, 94, 0.1)", borderRadius: 8,
                border: "1px solid rgba(34, 197, 94, 0.3)",
              }}>
                {/* Desglose detallado del monto esperado:
                      esperado = inicial + EFECTIVO_ventas + EFECTIVO_cobros - gastos - retiros
                    Las TRANSFER y CREDITO se muestran solo informativamente porque NO
                    afectan el efectivo en caja (van al banco / cuentas por cobrar). */}
                {(() => {
                  const b = breakdownCaja;
                  const totalEfectivoVentas = b?.total_efectivo ?? 0;
                  const totalTransferencia = b?.total_transferencia ?? 0;
                  const totalCredito = b?.total_credito ?? 0;
                  const totalCobrosEfectivo = b?.total_cobros_efectivo ?? 0;
                  const totalCobrosBanco = b?.total_cobros_banco ?? 0;
                  const totalGastos = b?.total_gastos ?? 0;
                  const totalRetiros = b?.total_retiros ?? 0;
                  const numVentas = b?.num_ventas ?? 0;
                  const numTransfer = b?.num_ventas_transfer ?? 0;
                  const numCredito = b?.num_ventas_credito ?? 0;
                  const gastosLista = b?.gastos_lista ?? [];
                  const retirosLista = b?.retiros ?? [];

                  return (
                    <>
                      <div style={{ display: "flex", justifyContent: "space-between", fontSize: 12, marginBottom: 4 }}>
                        <span className="text-secondary">Monto inicial:</span>
                        <span className="font-bold" style={{ color: "var(--color-success)" }}>
                          +${(cajaAbierta.monto_inicial ?? 0).toFixed(2)}
                        </span>
                      </div>

                      <div style={{ display: "flex", justifyContent: "space-between", fontSize: 12, marginBottom: 4 }}>
                        <span className="text-secondary">
                          Ventas EFECTIVO
                          <span style={{ color: "var(--color-text-secondary)", fontSize: 10, marginLeft: 4 }}>
                            ({numVentas} venta{numVentas === 1 ? "" : "s"} en esta sesión)
                          </span>:
                        </span>
                        <span className="font-bold" style={{ color: "var(--color-success)" }}>
                          +${totalEfectivoVentas.toFixed(2)}
                        </span>
                      </div>

                      {totalCobrosEfectivo > 0 && (
                        <div style={{ display: "flex", justifyContent: "space-between", fontSize: 12, marginBottom: 4 }}>
                          <span className="text-secondary">Cobros CXC en efectivo:</span>
                          <span className="font-bold" style={{ color: "var(--color-success)" }}>
                            +${totalCobrosEfectivo.toFixed(2)}
                          </span>
                        </div>
                      )}

                      {totalGastos > 0 && (
                        <>
                          <div style={{ display: "flex", justifyContent: "space-between", fontSize: 12, marginBottom: 4, color: "var(--color-danger)" }}>
                            <span>(−) Gastos ({gastosLista.length}):</span>
                            <span style={{ fontWeight: 600 }}>−${totalGastos.toFixed(2)}</span>
                          </div>
                          {breakdownExpandido && gastosLista.map((g: any, i: number) => (
                            <div key={`g${i}`} style={{ display: "flex", justifyContent: "space-between", fontSize: 11, padding: "2px 12px", color: "var(--color-text-secondary)" }}>
                              <span>· {g.fecha?.slice(11, 16)} {g.categoria}{g.descripcion ? ` — ${g.descripcion}` : ""}</span>
                              <span>−${g.monto.toFixed(2)}</span>
                            </div>
                          ))}
                        </>
                      )}

                      {totalRetiros > 0 && (
                        <>
                          <div style={{ display: "flex", justifyContent: "space-between", fontSize: 12, marginBottom: 4, color: "var(--color-danger)" }}>
                            <span>(−) Retiros ({retirosLista.length}):</span>
                            <span style={{ fontWeight: 600 }}>−${totalRetiros.toFixed(2)}</span>
                          </div>
                          {breakdownExpandido && retirosLista.map((r: any, i: number) => (
                            <div key={`r${i}`} style={{ display: "flex", justifyContent: "space-between", fontSize: 11, padding: "2px 12px", color: "var(--color-text-secondary)" }}>
                              <span>· {r.fecha?.slice(11, 16)} {r.usuario}{r.banco_nombre ? ` → ${r.banco_nombre}` : ""}{r.motivo ? ` — ${r.motivo}` : ""}</span>
                              <span>−${r.monto.toFixed(2)}</span>
                            </div>
                          ))}
                        </>
                      )}

                      <div style={{ borderTop: "1px solid rgba(34, 197, 94, 0.3)", paddingTop: 6, marginTop: 4, display: "flex", justifyContent: "space-between", fontSize: 14 }}>
                        <span style={{ fontWeight: 600, color: "var(--color-success)" }}>= Monto esperado (efectivo):</span>
                        <span style={{ fontWeight: 700, color: "var(--color-success)", fontSize: 16 }}>
                          ${(cajaAbierta.monto_esperado ?? 0).toFixed(2)}
                        </span>
                      </div>

                      {/* Info adicional: ventas que NO afectan efectivo */}
                      {(totalTransferencia > 0 || totalCredito > 0 || totalCobrosBanco > 0) && (
                        <div style={{ marginTop: 8, paddingTop: 6, borderTop: "1px dashed rgba(148,163,184,0.4)" }}>
                          <div style={{ fontSize: 10, color: "var(--color-text-secondary)", marginBottom: 4 }}>
                            ℹ Otras formas de pago (no afectan efectivo en caja):
                          </div>
                          {totalTransferencia > 0 && (
                            <div style={{ display: "flex", justifyContent: "space-between", fontSize: 11, color: "var(--color-text-secondary)" }}>
                              <span>· Transferencia ({numTransfer} ventas):</span>
                              <span>${totalTransferencia.toFixed(2)}</span>
                            </div>
                          )}
                          {totalCredito > 0 && (
                            <div style={{ display: "flex", justifyContent: "space-between", fontSize: 11, color: "var(--color-text-secondary)" }}>
                              <span>· Crédito ({numCredito} ventas):</span>
                              <span>${totalCredito.toFixed(2)}</span>
                            </div>
                          )}
                          {totalCobrosBanco > 0 && (
                            <div style={{ display: "flex", justifyContent: "space-between", fontSize: 11, color: "var(--color-text-secondary)" }}>
                              <span>· Cobros CXC al banco:</span>
                              <span>${totalCobrosBanco.toFixed(2)}</span>
                            </div>
                          )}
                        </div>
                      )}

                      {/* Toggle expandir/contraer detalle */}
                      {(gastosLista.length > 0 || retirosLista.length > 0) && (
                        <button type="button"
                          onClick={() => setBreakdownExpandido(!breakdownExpandido)}
                          style={{
                            background: "none", border: "none", color: "var(--color-primary)",
                            fontSize: 11, cursor: "pointer", padding: "6px 0 0", textDecoration: "underline",
                            width: "100%", textAlign: "left",
                          }}>
                          {breakdownExpandido ? "▲ Ocultar detalle de gastos/retiros" : "▼ Ver detalle de gastos/retiros"}
                        </button>
                      )}
                    </>
                  );
                })()}
              </div>
              <div>
                <label className="text-secondary" style={{ fontSize: 12 }}>Monto real contado en caja</label>
                <input
                  className="input input-lg mt-2"
                  type="number"
                  step="0.01"
                  placeholder="0.00"
                  value={montoReal}
                  onChange={(e) => setMontoReal(e.target.value)}
                />
              </div>

              {/* Alerta de descuadre + motivo obligatorio.
                  Solo se muestra cuando el cajero ya escribio un monto (no aparece
                  por defecto al abrir la pantalla, asi no asusta al usuario nuevo
                  haciendo creer que la caja esta descuadrada antes de contar). */}
              {(() => {
                // Si el campo esta vacio (default), no mostrar nada todavia
                if (montoReal.trim() === "") return null;
                const monto = parseFloat(montoReal) || 0;
                const esperado = cajaAbierta.monto_esperado ?? 0;
                const dif = monto - esperado;
                if (Math.abs(dif) <= 0.01) return null;
                const esFaltante = dif < 0;
                return (
                  <div className="mt-4">
                    <div style={{
                      padding: "8px 10px", borderRadius: 6,
                      background: esFaltante ? "rgba(239,68,68,0.12)" : "rgba(245,158,11,0.12)",
                      border: `1px solid ${esFaltante ? "rgba(239,68,68,0.4)" : "rgba(245,158,11,0.4)"}`,
                      color: esFaltante ? "var(--color-danger)" : "var(--color-warning)",
                      fontSize: 12, marginBottom: 6,
                    }}>
                      ⚠ Descuadre: <strong>${dif.toFixed(2)}</strong> {esFaltante ? "(faltante)" : "(sobrante)"}
                    </div>
                    <label className="text-secondary" style={{ fontSize: 12 }}>Motivo del descuadre *</label>
                    <textarea
                      className="input mt-2"
                      placeholder="Ej: cliente pago de mas en efectivo / falta vuelto entregado / etc."
                      value={motivoDescuadre}
                      onChange={(e) => setMotivoDescuadre(e.target.value)}
                      rows={2} />

                    {/* Boton de ajuste rapido para admin: corrige descuadre arrastrado
                        creando un retiro con motivo. Util cuando el monto_esperado quedo
                        desincronizado por bugs de versiones anteriores. Solo cuando es FALTANTE */}
                    {esAdmin && esFaltante && Math.abs(dif) > 0.01 && (
                      <div style={{ marginTop: 10, padding: 8, background: "rgba(59,130,246,0.08)", border: "1px solid rgba(59,130,246,0.3)", borderRadius: 6, fontSize: 11 }}>
                        <div style={{ marginBottom: 6 }}>
                          💡 <strong>Solo admin</strong>: si no puedes encontrar el origen del descuadre,
                          puedes ajustar el monto esperado a $0 registrando un retiro tipo [AJUSTE]
                          con el motivo que describas arriba. Queda en el historial.
                        </div>
                        <button
                          type="button"
                          className="btn btn-outline"
                          style={{ fontSize: 11, padding: "4px 10px", color: "var(--color-primary)", borderColor: "var(--color-primary)" }}
                          onClick={async () => {
                            const montoAjuste = Math.abs(dif);
                            const motivoAjuste = motivoDescuadre.trim();
                            if (!motivoAjuste) {
                              toastError("Debes escribir un motivo del descuadre antes de ajustar.");
                              return;
                            }
                            const ok = await ask(
                              `Vas a crear un retiro de ajuste por $${montoAjuste.toFixed(2)} para llevar la caja a $0.\n\nMotivo: ${motivoAjuste}\n\nEsto queda registrado en el historial como retiro tipo [AJUSTE]. ¿Confirmar?`,
                              { title: "Ajustar caja a $0", kind: "warning" }
                            );
                            if (!ok) return;
                            try {
                              await registrarRetiro(montoAjuste, `[AJUSTE] ${motivoAjuste}`, undefined, undefined);
                              toastExito("Retiro de ajuste creado. Recarga para ver el nuevo monto esperado.");
                              // Recargar caja para reflejar nuevo monto_esperado
                              const c = await obtenerCajaAbierta();
                              setCajaAbierta(c);
                              const r = c?.id ? await listarRetirosCaja(c.id) : [];
                              setRetiros(r);
                              // Si la caja queda en 0, limpiar montoReal
                              if (c?.monto_esperado != null && Math.abs(c.monto_esperado) < 0.01) {
                                setMontoReal("0");
                              }
                            } catch (err) {
                              toastError("Error: " + err);
                            }
                          }}>
                          🔧 Ajustar caja a $0 (retiro de ajuste)
                        </button>
                      </div>
                    )}
                  </div>
                );
              })()}

              <div className="mt-4">
                <label className="text-secondary" style={{ fontSize: 12 }}>Observación adicional (opcional)</label>
                <input
                  className="input mt-2"
                  value={observacion}
                  onChange={(e) => setObservacion(e.target.value)}
                />
              </div>
              <div className="flex gap-2 mt-4" style={{ flexWrap: "wrap" }}>
                <button
                  className="btn btn-success"
                  style={{ flex: 1, fontWeight: 600, minWidth: 130 }}
                  onClick={() => navigate("/pos")}
                >
                  + Nueva Venta
                </button>
                <button
                  className="btn btn-outline"
                  style={{ flex: 1, borderColor: "var(--color-warning)", color: "var(--color-warning)", minWidth: 130 }}
                  onClick={() => setMostrarRetiro(!mostrarRetiro)}
                >
                  Retiro de Caja
                </button>
                {esAdmin && (
                  <button
                    className="btn btn-outline"
                    style={{ flex: 1, borderColor: "var(--color-success)", color: "var(--color-success)", minWidth: 130 }}
                    title="Registrar ingreso manual a caja (ej: compensar gasto erroneo de caja anterior, aporte del dueño)"
                    onClick={() => setMostrarIngreso(!mostrarIngreso)}
                  >
                    + Ingreso a Caja
                  </button>
                )}
                <button className="btn btn-danger" style={{ flex: 1, minWidth: 130 }} onClick={() => setConfirmarCierre(true)}>
                  Cerrar Caja
                </button>
              </div>
            </div>

            {mostrarIngreso && (
              <div className="card" style={{ marginTop: 12, borderColor: "rgba(34,197,94,0.4)" }}>
                <div className="card-header" style={{ color: "var(--color-success)" }}>+ Ingreso Manual a Caja (solo admin)</div>
                <div className="card-body">
                  <div style={{
                    fontSize: 11, padding: 8, marginBottom: 10, borderRadius: 6,
                    background: "rgba(34,197,94,0.08)", color: "var(--color-success)",
                  }}>
                    💡 Usa esta opción para casos especiales:
                    compensar gastos erróneos de cajas cerradas, aportes del dueño, devolver dinero a caja, etc.
                    Quedará registrado en el historial con tu nombre y motivo.
                  </div>
                  <div style={{ display: "grid", gridTemplateColumns: "1fr 2fr", gap: 8 }}>
                    <div>
                      <label className="text-secondary" style={{ fontSize: 12 }}>Monto *</label>
                      <input className="input" type="number" step="0.01" min="0" placeholder="0.00"
                        value={ingresoMonto} onChange={e => setIngresoMonto(e.target.value)} autoFocus />
                    </div>
                    <div>
                      <label className="text-secondary" style={{ fontSize: 12 }}>Motivo (mín. 5 caracteres) *</label>
                      <input className="input" placeholder="Ej: Compensación gasto erróneo caja #42"
                        value={ingresoMotivo} onChange={e => setIngresoMotivo(e.target.value)} />
                    </div>
                  </div>
                  <div className="flex gap-2 mt-3">
                    <button className="btn btn-outline" onClick={() => {
                      setMostrarIngreso(false); setIngresoMonto(""); setIngresoMotivo("");
                    }}>Cancelar</button>
                    <button className="btn btn-success" onClick={handleIngreso}>Registrar Ingreso</button>
                  </div>
                </div>
              </div>
            )}

            {mostrarRetiro && (
              <div className="card" style={{ marginTop: 12 }}>
                <div className="card-header">Registrar Retiro de Efectivo</div>
                <div className="card-body">
                  <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: 8 }}>
                    <div>
                      <label className="text-secondary" style={{ fontSize: 12 }}>Monto *</label>
                      <input className="input" type="number" step="0.01" min="0" placeholder="0.00"
                        value={retiroMonto} onChange={e => setRetiroMonto(e.target.value)} autoFocus />
                    </div>
                    <div>
                      <label className="text-secondary" style={{ fontSize: 12 }}>Motivo</label>
                      <input className="input" placeholder="Ej: Depósito banco, pago proveedor..."
                        value={retiroMotivo} onChange={e => setRetiroMotivo(e.target.value)} />
                    </div>
                    <div>
                      <label className="text-secondary" style={{ fontSize: 12 }}>Depositar en cuenta (opcional)</label>
                      <select className="input" value={retiroBancoId ?? ""} onChange={e => setRetiroBancoId(e.target.value ? Number(e.target.value) : null)}>
                        <option value="">Sin depósito bancario</option>
                        {cuentasBanco.map(cb => <option key={cb.id} value={cb.id}>{cb.nombre}{cb.numero_cuenta ? ` — ${cb.numero_cuenta}` : ""}</option>)}
                      </select>
                    </div>
                    <div>
                      <label className="text-secondary" style={{ fontSize: 12 }}>Referencia</label>
                      <input className="input" placeholder="Nro. comprobante" value={retiroReferencia} onChange={e => setRetiroReferencia(e.target.value)} />
                    </div>
                  </div>
                  <div className="flex gap-2 mt-3">
                    <button className="btn btn-outline" onClick={() => setMostrarRetiro(false)}>Cancelar</button>
                    <button className="btn btn-primary" onClick={handleRetiro}>Registrar Retiro</button>
                  </div>
                </div>
              </div>
            )}

            {retiros.length > 0 && (() => {
              // Separar retiros de ingresos para totales independientes
              const totalRetiros = retiros.filter((r: any) => (r.tipo || "RETIRO") === "RETIRO").reduce((s: number, r: any) => s + r.monto, 0);
              const totalIngresos = retiros.filter((r: any) => r.tipo === "INGRESO").reduce((s: number, r: any) => s + r.monto, 0);
              return (
              <div className="card" style={{ marginTop: 12 }}>
                <div className="card-header" style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
                  <span>Movimientos de caja ({retiros.length})</span>
                  <span style={{ display: "flex", gap: 12, fontSize: 13 }}>
                    {totalIngresos > 0 && (
                      <span style={{ fontWeight: 700, color: "var(--color-success)" }}>
                        Ingresos: +${totalIngresos.toFixed(2)}
                      </span>
                    )}
                    {totalRetiros > 0 && (
                      <span style={{ fontWeight: 700, color: "var(--color-danger)" }}>
                        Retiros: -${totalRetiros.toFixed(2)}
                      </span>
                    )}
                  </span>
                </div>
                <div className="card-body" style={{ padding: 0 }}>
                  <table className="table">
                    <thead><tr><th>Hora</th><th>Tipo</th><th>Monto</th><th>Motivo</th><th>Cuenta</th><th>Estado</th><th>Usuario</th></tr></thead>
                    <tbody>
                      {retiros.map((r: any) => {
                        const esIngreso = r.tipo === "INGRESO";
                        return (
                        <Fragment key={r.id}>
                          <tr>
                            <td style={{ fontSize: 12 }}>{r.fecha?.slice(11, 16)}</td>
                            <td style={{ fontSize: 11 }}>
                              <span style={{
                                padding: "2px 6px", borderRadius: 4, fontWeight: 600,
                                background: esIngreso ? "rgba(34,197,94,0.15)" : "rgba(239,68,68,0.10)",
                                color: esIngreso ? "var(--color-success)" : "var(--color-danger)",
                              }}>
                                {esIngreso ? "+ Ingreso" : "− Retiro"}
                              </span>
                            </td>
                            <td style={{ color: esIngreso ? "var(--color-success)" : "var(--color-danger)", fontWeight: 600 }}>
                              {esIngreso ? "+" : "-"}${r.monto.toFixed(2)}
                            </td>
                            <td style={{ fontSize: 12 }}>{r.motivo || "-"}</td>
                            <td style={{ fontSize: 12 }}>{r.banco_nombre || "-"}</td>
                            <td style={{ fontSize: 12 }}>
                              {r.estado === "EN_TRANSITO" && (
                                <span style={{ display: "inline-flex", alignItems: "center", gap: 6 }}>
                                  <span style={{ padding: "2px 8px", borderRadius: 4, background: "rgba(245, 158, 11, 0.15)", color: "var(--color-warning)", fontSize: 11, fontWeight: 600 }}>En tránsito</span>
                                  <button className="btn" style={{ padding: "2px 8px", fontSize: 11, minHeight: 0 }}
                                    onClick={() => { setConfirmandoRetiroId(r.id); setConfirmRef(r.referencia || ""); setConfirmImg(null); }}>
                                    Confirmar
                                  </button>
                                </span>
                              )}
                              {r.estado === "DEPOSITADO" && (
                                <span style={{ padding: "2px 8px", borderRadius: 4, background: "rgba(34, 197, 94, 0.15)", color: "var(--color-success)", fontSize: 11, fontWeight: 600 }}>Depositado</span>
                              )}
                              {(r.estado === "SIN_DEPOSITO" || !r.estado) && (
                                <span style={{ padding: "2px 8px", borderRadius: 4, background: "rgba(148, 148, 148, 0.15)", color: "var(--color-text-secondary)", fontSize: 11, fontWeight: 600 }}>Sin depósito</span>
                              )}
                            </td>
                            <td style={{ fontSize: 12 }}>{r.usuario}</td>
                          </tr>
                          {confirmandoRetiroId === r.id && (
                            <tr>
                              <td colSpan={6} style={{ padding: "8px 12px", background: "rgba(245, 158, 11, 0.05)" }}>
                                <div style={{ display: "flex", gap: 8, alignItems: "flex-end", flexWrap: "wrap" }}>
                                  <div style={{ flex: 1, minWidth: 150 }}>
                                    <label className="text-secondary" style={{ fontSize: 11 }}>Nro. Comprobante *</label>
                                    <input className="input" style={{ fontSize: 13 }} placeholder="Referencia del depósito"
                                      value={confirmRef} onChange={e => setConfirmRef(e.target.value)} autoFocus />
                                  </div>
                                  <div style={{ flex: 1, minWidth: 150 }}>
                                    <label className="text-secondary" style={{ fontSize: 11 }}>Imagen comprobante (opcional)</label>
                                    <input type="file" accept="image/*" style={{ fontSize: 12 }}
                                      onChange={handleImagenComprobante} />
                                    {confirmImg && <span style={{ fontSize: 11, color: "var(--color-success)" }}>Imagen cargada</span>}
                                  </div>
                                  <button className="btn btn-success" style={{ fontSize: 12, padding: "6px 12px" }}
                                    onClick={handleConfirmarDeposito}>
                                    Confirmar Depósito
                                  </button>
                                  <button className="btn btn-outline" style={{ fontSize: 12, padding: "6px 12px" }}
                                    onClick={() => { setConfirmandoRetiroId(null); setConfirmRef(""); setConfirmImg(null); }}>
                                    Cancelar
                                  </button>
                                </div>
                              </td>
                            </tr>
                          )}
                        </Fragment>
                        );
                      })}
                    </tbody>
                  </table>
                </div>
              </div>
              );
            })()}
          </div>
        )}
      </div>

      <Modal
        abierto={confirmarCierre}
        titulo="Cerrar Caja"
        mensaje="Esta seguro que desea cerrar la caja? Se cerrara su sesion automaticamente."
        tipo="peligro"
        textoConfirmar="Si, cerrar caja"
        onConfirmar={handleCerrar}
        onCancelar={() => setConfirmarCierre(false)}
      />

      {/* Drawer lateral derecho: Historial completo de caja (sesiones + descuadres) */}
      {mostrarHistorial && (
        <>
          {/* Backdrop */}
          <div
            onClick={() => setMostrarHistorial(false)}
            style={{
              position: "fixed", inset: 0,
              background: "rgba(0,0,0,0.4)",
              zIndex: 999,
              animation: "fadeIn 0.15s ease-out",
            }}
          />
          {/* Drawer */}
          <div
            onClick={(e) => e.stopPropagation()}
            style={{
              position: "fixed", top: 0, right: 0, bottom: 0,
              width: "min(96vw, 1280px)",
              background: "var(--color-bg)",
              boxShadow: "-4px 0 20px rgba(0,0,0,0.25)",
              zIndex: 1000,
              display: "flex", flexDirection: "column",
              animation: "slideInRight 0.2s ease-out",
            }}>
            <div className="modal-header" style={{
              display: "flex", justifyContent: "space-between", alignItems: "center",
              padding: "12px 20px", borderBottom: "1px solid var(--color-border)",
              flexShrink: 0,
            }}>
              <h3 style={{ margin: 0 }}>📊 Historial de caja</h3>
              <button
                onClick={() => setMostrarHistorial(false)}
                style={{
                  background: "none", border: "none", cursor: "pointer",
                  fontSize: 24, color: "var(--color-text-secondary)", padding: "0 8px",
                  lineHeight: 1,
                }}
                title="Cerrar (Esc)">×</button>
            </div>
            <style>{`
              @keyframes slideInRight { from { transform: translateX(100%); } to { transform: translateX(0); } }
              @keyframes fadeIn { from { opacity: 0; } to { opacity: 1; } }
            `}</style>
            <div style={{ flex: 1, overflowY: "auto", padding: "16px 20px" }}>
              {/* Tabs */}
              <div style={{ display: "flex", gap: 4, marginBottom: 12, borderBottom: "1px solid var(--color-border)" }}>
                <button
                  className={`btn ${historialTab === "sesiones" ? "btn-primary" : "btn-outline"}`}
                  style={{ fontSize: 12, padding: "6px 14px", borderRadius: "6px 6px 0 0" }}
                  onClick={() => setHistorialTab("sesiones")}>
                  📋 Todas las sesiones ({sesionesData.length})
                </button>
                <button
                  className={`btn ${historialTab === "descuadres" ? "btn-primary" : "btn-outline"}`}
                  style={{ fontSize: 12, padding: "6px 14px", borderRadius: "6px 6px 0 0" }}
                  onClick={() => setHistorialTab("descuadres")}>
                  ⚠ Descuadres ({historialData?.total_descuadrados || 0})
                </button>
              </div>

              <div style={{ display: "flex", gap: 8, alignItems: "end", marginBottom: 14, flexWrap: "wrap" }}>
                <div>
                  <label className="text-secondary" style={{ fontSize: 11 }}>Desde</label>
                  <input type="date" className="input" style={{ fontSize: 12 }}
                    value={historialDesde}
                    onChange={(e) => setHistorialDesde(e.target.value)} />
                </div>
                <div>
                  <label className="text-secondary" style={{ fontSize: 11 }}>Hasta</label>
                  <input type="date" className="input" style={{ fontSize: 12 }}
                    value={historialHasta}
                    onChange={(e) => setHistorialHasta(e.target.value)} />
                </div>
                <button className="btn btn-primary" style={{ fontSize: 12 }}
                  onClick={async () => {
                    try {
                      const [sesiones, d] = await Promise.all([
                        listarSesionesCaja(historialDesde, historialHasta),
                        historialDescuadresCaja(historialDesde, historialHasta),
                      ]);
                      setSesionesData(sesiones);
                      setHistorialData(d);
                    } catch (err) { toastError("Error: " + err); }
                  }}>
                  Filtrar
                </button>
              </div>

              {/* TAB: Todas las sesiones */}
              {historialTab === "sesiones" && (
                sesionesData.length === 0 ? (
                  <div className="text-center text-secondary" style={{ padding: 30 }}>Sin sesiones en este período</div>
                ) : (
                  <table className="table" style={{ width: "100%" }}>
                    <thead>
                      <tr>
                        <th style={{ width: 24 }}></th>
                        <th>Apertura</th>
                        <th>Cierre</th>
                        <th>Cajero apertura</th>
                        <th>Cajero cierre</th>
                        <th className="text-right">Inicial</th>
                        <th className="text-right">Ventas</th>
                        <th className="text-right">Esperado</th>
                        <th className="text-right">Contado</th>
                        <th className="text-right">Dif.</th>
                        <th>Estado</th>
                      </tr>
                    </thead>
                    <tbody>
                      {sesionesData.map((s: any) => {
                        const dif = Number(s.diferencia ?? 0);
                        const descuadrada = s.estado === "CERRADA" && Math.abs(dif) > 0.01;
                        const expandida = sesionExpandida === s.id;
                        const eventos = eventosCache[s.id] || [];
                        return (
                          <Fragment key={s.id}>
                            <tr style={{ background: descuadrada ? "rgba(245,158,11,0.06)" : undefined, cursor: "pointer" }}
                              onClick={async () => {
                                if (expandida) {
                                  setSesionExpandida(null);
                                } else {
                                  setSesionExpandida(s.id);
                                  // Lazy load eventos si no estan cacheados
                                  if (!eventosCache[s.id]) {
                                    try {
                                      const evs = await listarEventosCaja(s.id);
                                      setEventosCache({ ...eventosCache, [s.id]: evs });
                                    } catch (err) { toastError("Error eventos: " + err); }
                                  }
                                }
                              }}>
                              <td style={{ textAlign: "center", fontSize: 11, color: "var(--color-text-secondary)" }}>
                                {expandida ? "▼" : "▶"}
                              </td>
                              <td style={{ fontSize: 11 }}>{s.fecha_apertura?.slice(0, 16).replace("T", " ")}</td>
                              <td style={{ fontSize: 11 }}>{s.fecha_cierre?.slice(0, 16).replace("T", " ") || "-"}</td>
                              <td>{s.usuario_apertura || "-"}</td>
                              <td>{s.usuario_cierre || "-"}</td>
                              <td className="text-right">${s.monto_inicial.toFixed(2)}</td>
                              <td className="text-right">${s.monto_ventas.toFixed(2)}</td>
                              <td className="text-right">${s.monto_esperado.toFixed(2)}</td>
                              <td className="text-right font-bold">{s.monto_real != null ? `$${s.monto_real.toFixed(2)}` : "-"}</td>
                              <td className="text-right" style={{
                                color: dif < 0 ? "var(--color-danger)" : dif > 0 ? "var(--color-warning)" : "var(--color-text-secondary)",
                                fontWeight: descuadrada ? 700 : undefined,
                              }}>
                                {s.diferencia != null ? `$${dif.toFixed(2)}` : "-"}
                              </td>
                              <td>
                                <span style={{
                                  fontSize: 10, fontWeight: 700, padding: "2px 6px", borderRadius: 3,
                                  background: s.estado === "ABIERTA" ? "rgba(34,197,94,0.15)" : descuadrada ? "rgba(245,158,11,0.15)" : "rgba(148,163,184,0.15)",
                                  color: s.estado === "ABIERTA" ? "var(--color-success)" : descuadrada ? "var(--color-warning)" : "var(--color-text-secondary)",
                                }}>
                                  {s.estado}{descuadrada ? " ⚠" : ""}
                                </span>
                              </td>
                            </tr>
                            {expandida && (
                              <tr>
                                <td colSpan={11} style={{ background: "var(--color-surface-alt)", padding: 12, fontSize: 11 }}>
                                  {/* Botones de reimpresion del reporte de cierre */}
                                  {s.estado === "CERRADA" && (
                                    <div style={{ display: "flex", gap: 6, marginBottom: 10, justifyContent: "flex-end" }}>
                                      <button className="btn btn-outline" style={{ fontSize: 11, padding: "4px 12px" }}
                                        title="Reimprimir reporte de cierre en impresora térmica (te preguntará si quieres resumido o detallado)"
                                        onClick={async (e) => {
                                          e.stopPropagation();
                                          if (!s.id) return;
                                          await imprimirTicketCierreCaja(s.id);
                                        }}>
                                        🖨 Reimprimir ticket
                                      </button>
                                      <button className="btn btn-outline" style={{ fontSize: 11, padding: "4px 12px" }}
                                        title="Generar PDF A4 del reporte"
                                        onClick={async (e) => {
                                          e.stopPropagation();
                                          if (!s.id) return;
                                          try {
                                            await imprimirReporteCajaPdf(s.id);
                                            toastExito("PDF generado");
                                          } catch (err) { toastError("Error: " + err); }
                                        }}>
                                        📄 Reporte PDF A4
                                      </button>
                                    </div>
                                  )}

                                  {/* Datos resumen + motivos */}
                                  <div style={{ display: "grid", gridTemplateColumns: "repeat(auto-fit, minmax(220px, 1fr))", gap: 10, marginBottom: 10 }}>
                                    {s.caja_anterior_id && (
                                      <div><strong>Cierre anterior:</strong> #{s.caja_anterior_id}</div>
                                    )}
                                    {s.motivo_diferencia_apertura && (
                                      <div style={{ gridColumn: "1 / -1", padding: "6px 8px", background: "rgba(59,130,246,0.08)", borderRadius: 4 }}>
                                        <strong>📋 Motivo dif. apertura:</strong> {s.motivo_diferencia_apertura}
                                      </div>
                                    )}
                                    {s.motivo_descuadre && (
                                      <div style={{ gridColumn: "1 / -1", padding: "6px 8px", background: "rgba(245,158,11,0.08)", borderRadius: 4, color: "var(--color-warning)" }}>
                                        <strong>⚠ Motivo descuadre:</strong> {s.motivo_descuadre}
                                      </div>
                                    )}
                                    {s.observacion && (
                                      <div style={{ gridColumn: "1 / -1" }}>
                                        <strong>Observación:</strong> {s.observacion}
                                      </div>
                                    )}
                                  </div>
                                  {/* Audit log */}
                                  <div style={{ fontWeight: 700, marginBottom: 4, fontSize: 12 }}>🔍 Audit log</div>
                                  {eventos.length === 0 ? (
                                    <div style={{ color: "var(--color-text-secondary)", fontStyle: "italic" }}>
                                      {eventosCache[s.id] ? "Sin eventos registrados (sesión anterior a la auditoría)" : "Cargando..."}
                                    </div>
                                  ) : (
                                    <table style={{ width: "100%", fontSize: 10, borderCollapse: "collapse" }}>
                                      <thead>
                                        <tr style={{ borderBottom: "1px solid var(--color-border)" }}>
                                          <th style={{ textAlign: "left", padding: "3px 6px" }}>Fecha/hora</th>
                                          <th style={{ textAlign: "left", padding: "3px 6px" }}>Evento</th>
                                          <th style={{ textAlign: "left", padding: "3px 6px" }}>Usuario</th>
                                          <th style={{ textAlign: "left", padding: "3px 6px" }}>Motivo</th>
                                        </tr>
                                      </thead>
                                      <tbody>
                                        {eventos.map((e: any) => {
                                          const evColor = e.evento === "DESCUADRE_GRAVE" ? "var(--color-danger)" :
                                                          e.evento === "DEPOSITO" ? "var(--color-primary)" :
                                                          e.evento === "CIERRE" ? "var(--color-warning)" :
                                                          "var(--color-text)";
                                          return (
                                            <tr key={e.id}>
                                              <td style={{ padding: "3px 6px" }}>{e.timestamp?.slice(0, 16).replace("T", " ")}</td>
                                              <td style={{ padding: "3px 6px", fontWeight: 600, color: evColor }}>{e.evento}</td>
                                              <td style={{ padding: "3px 6px" }}>{e.usuario || "-"}</td>
                                              <td style={{ padding: "3px 6px" }}>{e.motivo || <span style={{ color: "var(--color-text-secondary)" }}>-</span>}</td>
                                            </tr>
                                          );
                                        })}
                                      </tbody>
                                    </table>
                                  )}
                                </td>
                              </tr>
                            )}
                          </Fragment>
                        );
                      })}
                    </tbody>
                  </table>
                )
              )}

              {/* TAB: Descuadres (contenido original) */}
              {historialTab === "descuadres" && (!historialData ? (
                <div className="text-center text-secondary" style={{ padding: 30 }}>Cargando...</div>
              ) : (
                <>
                  {/* KPIs */}
                  <div style={{ display: "grid", gridTemplateColumns: "repeat(4, 1fr)", gap: 10, marginBottom: 16 }}>
                    <div className="card" style={{ borderLeft: "4px solid var(--color-text-secondary)" }}>
                      <div className="card-body" style={{ padding: 12 }}>
                        <div className="text-secondary" style={{ fontSize: 11 }}>Cierres con descuadre</div>
                        <div style={{ fontSize: 22, fontWeight: 700 }}>{historialData.total_descuadrados}</div>
                      </div>
                    </div>
                    <div className="card" style={{ borderLeft: "4px solid var(--color-danger)" }}>
                      <div className="card-body" style={{ padding: 12 }}>
                        <div className="text-secondary" style={{ fontSize: 11 }}>Total faltantes</div>
                        <div style={{ fontSize: 22, fontWeight: 700, color: "var(--color-danger)" }}>${historialData.total_faltantes.toFixed(2)}</div>
                      </div>
                    </div>
                    <div className="card" style={{ borderLeft: "4px solid var(--color-warning)" }}>
                      <div className="card-body" style={{ padding: 12 }}>
                        <div className="text-secondary" style={{ fontSize: 11 }}>Total sobrantes</div>
                        <div style={{ fontSize: 22, fontWeight: 700, color: "var(--color-warning)" }}>${historialData.total_sobrantes.toFixed(2)}</div>
                      </div>
                    </div>
                    <div className="card" style={{ borderLeft: `4px solid ${historialData.neto < 0 ? "var(--color-danger)" : "var(--color-success)"}` }}>
                      <div className="card-body" style={{ padding: 12 }}>
                        <div className="text-secondary" style={{ fontSize: 11 }}>Neto</div>
                        <div style={{ fontSize: 22, fontWeight: 700, color: historialData.neto < 0 ? "var(--color-danger)" : "var(--color-success)" }}>
                          ${historialData.neto.toFixed(2)}
                        </div>
                      </div>
                    </div>
                  </div>

                  {/* Resumen por usuario */}
                  {historialData.por_usuario.length > 0 && (
                    <div style={{ marginBottom: 16 }}>
                      <div style={{ fontWeight: 700, fontSize: 13, marginBottom: 6 }}>Por cajero</div>
                      <table className="table" style={{ width: "100%" }}>
                        <thead>
                          <tr>
                            <th>Cajero</th>
                            <th className="text-right">Cierres con descuadre</th>
                            <th className="text-right">Faltantes</th>
                            <th className="text-right">Sobrantes</th>
                            <th className="text-right">Neto</th>
                          </tr>
                        </thead>
                        <tbody>
                          {historialData.por_usuario
                            .sort((a: any, b: any) => b.total_faltantes - a.total_faltantes)
                            .map((u: any) => (
                            <tr key={u.usuario}>
                              <td><strong>{u.usuario}</strong></td>
                              <td className="text-right">{u.total_cierres_descuadrados}</td>
                              <td className="text-right" style={{ color: "var(--color-danger)" }}>${u.total_faltantes.toFixed(2)}</td>
                              <td className="text-right" style={{ color: "var(--color-warning)" }}>${u.total_sobrantes.toFixed(2)}</td>
                              <td className="text-right" style={{ color: u.diferencia_neta < 0 ? "var(--color-danger)" : "var(--color-success)", fontWeight: 700 }}>
                                ${u.diferencia_neta.toFixed(2)}
                              </td>
                            </tr>
                          ))}
                        </tbody>
                      </table>
                    </div>
                  )}

                  {/* Lista de cierres */}
                  <div style={{ fontWeight: 700, fontSize: 13, marginBottom: 6 }}>Detalle de cierres</div>
                  {historialData.cierres.length === 0 ? (
                    <div className="text-center text-secondary" style={{ padding: 30 }}>Sin cierres con descuadre en este período 🎉</div>
                  ) : (
                    <table className="table" style={{ width: "100%" }}>
                      <thead>
                        <tr>
                          <th>Fecha cierre</th>
                          <th>Cajero</th>
                          <th className="text-right">Inicial</th>
                          <th className="text-right">Esperado</th>
                          <th className="text-right">Contado</th>
                          <th className="text-right">Diferencia</th>
                          <th>Motivo</th>
                          <th style={{ width: 180 }}>Acciones</th>
                        </tr>
                      </thead>
                      <tbody>
                        {historialData.cierres.map((c: any) => (
                          <tr key={c.caja_id}>
                            <td style={{ fontSize: 11 }}>{c.fecha_cierre?.slice(0, 16).replace("T", " ")}</td>
                            <td>{c.usuario || "-"}</td>
                            <td className="text-right">${c.monto_inicial.toFixed(2)}</td>
                            <td className="text-right">${c.monto_esperado.toFixed(2)}</td>
                            <td className="text-right font-bold">${(c.monto_real ?? 0).toFixed(2)}</td>
                            <td className="text-right" style={{ color: c.diferencia < 0 ? "var(--color-danger)" : "var(--color-warning)", fontWeight: 700 }}>
                              ${c.diferencia.toFixed(2)}
                            </td>
                            <td style={{ fontSize: 11, maxWidth: 220 }}>{c.motivo_descuadre || <span className="text-secondary">(sin motivo)</span>}</td>
                            <td>
                              <div style={{ display: "flex", gap: 4 }}>
                                <button className="btn btn-outline" style={{ fontSize: 10, padding: "3px 8px" }}
                                  title="Reimprimir reporte de cierre en impresora térmica (te preguntará si quieres resumido o detallado)"
                                  onClick={async () => {
                                    if (!c.caja_id) return;
                                    await imprimirTicketCierreCaja(c.caja_id);
                                  }}>🖨</button>
                                <button className="btn btn-outline" style={{ fontSize: 10, padding: "3px 8px" }}
                                  title="Generar PDF A4 del reporte de cierre"
                                  onClick={async () => {
                                    if (!c.caja_id) return;
                                    try {
                                      await imprimirReporteCajaPdf(c.caja_id);
                                      toastExito("PDF generado");
                                    } catch (err) { toastError("Error: " + err); }
                                  }}>📄 PDF</button>
                              </div>
                            </td>
                          </tr>
                        ))}
                      </tbody>
                    </table>
                  )}
                </>
              ))}
            </div>
            <div style={{
              padding: "10px 20px", borderTop: "1px solid var(--color-border)",
              display: "flex", justifyContent: "flex-end", flexShrink: 0,
            }}>
              <button className="btn btn-outline" onClick={() => setMostrarHistorial(false)}>Cerrar (Esc)</button>
            </div>
          </div>
        </>
      )}

      {/* Modal PIN supervisor (cuando rol no tiene permiso o descuadre alto) */}
      {pinPrompt && (
        <div className="modal-overlay" onClick={() => { setPinPrompt(null); setPinSupervisor(""); }}>
          <div className="modal-content" onClick={(e) => e.stopPropagation()} style={{ maxWidth: 420 }}>
            <div className="modal-header">
              <h3>🔐 Autorización requerida</h3>
            </div>
            <div className="modal-body">
              <div style={{
                padding: "10px 12px", marginBottom: 12, borderRadius: 6,
                background: pinPrompt.tipo === "descuadre" ? "rgba(245,158,11,0.12)" : "rgba(59,130,246,0.1)",
                border: `1px solid ${pinPrompt.tipo === "descuadre" ? "rgba(245,158,11,0.4)" : "rgba(59,130,246,0.3)"}`,
                fontSize: 12, color: pinPrompt.tipo === "descuadre" ? "var(--color-warning)" : "var(--color-text)",
              }}>
                {pinPrompt.mensaje}
              </div>
              <label className="text-secondary" style={{ fontSize: 12 }}>PIN de supervisor (administrador)</label>
              <input
                type="password"
                className="input mt-2"
                placeholder="••••"
                value={pinSupervisor}
                onChange={(e) => setPinSupervisor(e.target.value)}
                autoFocus
                onKeyDown={async (e) => {
                  if (e.key === "Enter" && pinSupervisor.length >= 4) {
                    const ok = await intentarCerrarCaja(pinSupervisor);
                    if (ok) { setPinPrompt(null); setPinSupervisor(""); }
                  }
                }} />
            </div>
            <div className="modal-footer" style={{ display: "flex", justifyContent: "space-between" }}>
              <button className="btn btn-outline" onClick={() => { setPinPrompt(null); setPinSupervisor(""); }}>Cancelar</button>
              <button className="btn btn-primary"
                disabled={pinSupervisor.length < 4}
                onClick={async () => {
                  const ok = await intentarCerrarCaja(pinSupervisor);
                  if (ok) { setPinPrompt(null); setPinSupervisor(""); }
                }}>
                Autorizar y cerrar
              </button>
            </div>
          </div>
        </div>
      )}

      {/* Modal: registrar deposito a banco post-cierre */}
      {mostrarDeposito && resumen && (
        <div className="modal-overlay" onClick={() => setMostrarDeposito(false)}>
          <div className="modal-content" onClick={(e) => e.stopPropagation()} style={{ maxWidth: 480 }}>
            <div className="modal-header">
              <h3>🏦 Depósito a banco</h3>
            </div>
            <div className="modal-body">
              <p style={{ fontSize: 12, color: "var(--color-text-secondary)", marginBottom: 12 }}>
                Registra el depósito del efectivo contado al banco. Queda atado a esta caja para auditoría.
              </p>
              <label className="text-secondary" style={{ fontSize: 12 }}>Monto a depositar</label>
              <input className="input mt-2" type="number" step="0.01"
                placeholder={resumen.caja.monto_real?.toFixed(2) || "0.00"}
                value={depositoMonto}
                onChange={(e) => setDepositoMonto(e.target.value)} />
              <div className="mt-3">
                <label className="text-secondary" style={{ fontSize: 12 }}>Cuenta bancaria *</label>
                <select className="input mt-2"
                  value={depositoBancoId === null ? "" : String(depositoBancoId)}
                  onChange={(e) => setDepositoBancoId(e.target.value ? parseInt(e.target.value) : null)}>
                  <option value="">— Seleccione cuenta —</option>
                  {cuentasBanco.map((b: any) => (
                    <option key={b.id} value={b.id}>
                      {b.nombre}{b.numero_cuenta ? ` · ${b.numero_cuenta}` : ""}
                    </option>
                  ))}
                </select>
              </div>
              <div className="mt-3">
                <label className="text-secondary" style={{ fontSize: 12 }}>Referencia / N° transacción (opcional)</label>
                <input className="input mt-2"
                  placeholder="Ej: 0012345"
                  value={depositoReferencia}
                  onChange={(e) => setDepositoReferencia(e.target.value)} />
              </div>
              <div style={{ fontSize: 11, color: "var(--color-text-secondary)", marginTop: 8 }}>
                💡 Si tienes la referencia, el depósito se marca como CONFIRMADO. Si no, queda EN_TRANSITO hasta confirmar.
              </div>
            </div>
            <div className="modal-footer" style={{ display: "flex", justifyContent: "space-between" }}>
              <button className="btn btn-outline" onClick={() => setMostrarDeposito(false)}>Cancelar</button>
              <button className="btn btn-primary"
                disabled={!depositoBancoId || !depositoMonto || parseFloat(depositoMonto) <= 0}
                onClick={async () => {
                  if (!resumen.caja.id || !depositoBancoId) return;
                  try {
                    await registrarDepositoCierre(
                      resumen.caja.id,
                      parseFloat(depositoMonto),
                      depositoBancoId,
                      depositoReferencia.trim() || undefined,
                    );
                    toastExito("Depósito registrado");
                    setMostrarDeposito(false);
                    setDepositoMonto(""); setDepositoBancoId(null); setDepositoReferencia("");
                    // Refrescar la lista de depositos en la card del resumen para que se vea
                    // inmediatamente sin tener que finalizar turno y volver a mirar.
                    listarRetirosCaja(resumen.caja.id).then(setResumenRetiros).catch(() => {});
                  } catch (err) { toastError("Error: " + err); }
                }}>
                Registrar depósito
              </button>
            </div>
          </div>
        </div>
      )}
    </>
  );
}
