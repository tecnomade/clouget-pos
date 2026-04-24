import { useState, useEffect, Fragment } from "react";
import { useNavigate } from "react-router-dom";
import { abrirCaja, cerrarCaja, obtenerCajaAbierta, imprimirReporteCaja, imprimirReporteCajaPdf, obtenerConfig, registrarRetiro, listarRetirosCaja, listarCuentasBanco, confirmarDeposito, obtenerUltimoCierre, historialDescuadresCaja, listarSesionesCaja, registrarDepositoCierre, listarEventosCaja } from "../services/api";
import { useToast } from "../components/Toast";
import { useSesion } from "../contexts/SesionContext";
import Modal from "../components/Modal";
import type { Caja, ResumenCaja } from "../types";

export default function CajaPage() {
  const navigate = useNavigate();
  const { toastExito, toastError } = useToast();
  const { sesion, cerrarSesion } = useSesion();
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
    const totalRetiros = retiros.reduce((s: number, r: any) => s + (Number(r.monto) || 0), 0);
    const esperado = (cajaAbierta?.monto_inicial || 0) + (cajaAbierta?.monto_ventas || 0) - totalRetiros;
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

  const handleFinalizarTurno = async () => {
    setResumen(null);
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
      if (cajaAbierta?.id) listarRetirosCaja(cajaAbierta.id).then(setRetiros).catch(() => {});
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
              {/* Botones de impresion */}
              <div className="flex gap-2 mt-4">
                <button
                  className="btn btn-outline"
                  style={{ flex: 1 }}
                  disabled={imprimiendo || !resumen.caja.id}
                  onClick={async () => {
                    if (!resumen.caja.id) return;
                    setImprimiendo(true);
                    try {
                      if (ticketUsarPdf) {
                        await imprimirReporteCajaPdf(resumen.caja.id);
                        toastExito("Reporte PDF generado");
                      } else {
                        await imprimirReporteCaja(resumen.caja.id);
                        toastExito("Reporte impreso");
                      }
                    } catch (err) {
                      toastError("Error imprimiendo: " + err);
                    } finally {
                      setImprimiendo(false);
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
              <div className="mb-4">
                <span className="text-secondary">Monto inicial:</span>
                <span className="font-bold" style={{ marginLeft: 8 }}>${cajaAbierta.monto_inicial.toFixed(2)}</span>
              </div>
              <div className="mb-4" style={{
                padding: "10px 14px", background: "rgba(34, 197, 94, 0.1)", borderRadius: 8,
                border: "1px solid rgba(34, 197, 94, 0.3)",
              }}>
                <div style={{ display: "flex", justifyContent: "space-between", fontSize: 12, marginBottom: 4 }}>
                  <span className="text-secondary">Ventas en efectivo:</span>
                  <span className="font-bold">${(cajaAbierta.monto_ventas ?? 0).toFixed(2)}</span>
                </div>
                {retiros.length > 0 && (
                  <div style={{ display: "flex", justifyContent: "space-between", fontSize: 12, marginBottom: 4, color: "var(--color-danger)" }}>
                    <span>(-) Retiros:</span>
                    <span>-${retiros.reduce((s: number, r: any) => s + r.monto, 0).toFixed(2)}</span>
                  </div>
                )}
                <div style={{ borderTop: "1px solid rgba(34, 197, 94, 0.3)", paddingTop: 6, marginTop: 4, display: "flex", justifyContent: "space-between", fontSize: 14 }}>
                  <span style={{ fontWeight: 600, color: "var(--color-success)" }}>Monto esperado en caja:</span>
                  <span style={{ fontWeight: 700, color: "var(--color-success)", fontSize: 16 }}>
                    ${(cajaAbierta.monto_inicial + (cajaAbierta.monto_ventas ?? 0) - retiros.reduce((s: number, r: any) => s + r.monto, 0)).toFixed(2)}
                  </span>
                </div>
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

              {/* Alerta de descuadre + motivo obligatorio */}
              {(() => {
                const monto = parseFloat(montoReal) || 0;
                const totalRetiros = retiros.reduce((s: number, r: any) => s + (Number(r.monto) || 0), 0);
                const esperado = (cajaAbierta.monto_inicial || 0) + (cajaAbierta.monto_ventas || 0) - totalRetiros;
                const dif = monto - esperado;
                // Aparece si hay diferencia significativa (>0.01). Esto incluye el caso
                // de dejar vacio con esperado>0 (descuadre = -esperado).
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
              <div className="flex gap-2 mt-4">
                <button
                  className="btn btn-success"
                  style={{ flex: 1, fontWeight: 600 }}
                  onClick={() => navigate("/pos")}
                >
                  + Nueva Venta
                </button>
                <button
                  className="btn btn-outline"
                  style={{ flex: 1, borderColor: "var(--color-warning)", color: "var(--color-warning)" }}
                  onClick={() => setMostrarRetiro(!mostrarRetiro)}
                >
                  Retiro de Caja
                </button>
                <button className="btn btn-danger" style={{ flex: 1 }} onClick={() => setConfirmarCierre(true)}>
                  Cerrar Caja
                </button>
              </div>
            </div>

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

            {retiros.length > 0 && (
              <div className="card" style={{ marginTop: 12 }}>
                <div className="card-header">
                  Retiros del día ({retiros.length})
                  <span style={{ float: "right", fontWeight: 700, color: "var(--color-danger)" }}>
                    -${retiros.reduce((s: number, r: any) => s + r.monto, 0).toFixed(2)}
                  </span>
                </div>
                <div className="card-body" style={{ padding: 0 }}>
                  <table className="table">
                    <thead><tr><th>Hora</th><th>Monto</th><th>Motivo</th><th>Cuenta</th><th>Estado</th><th>Usuario</th></tr></thead>
                    <tbody>
                      {retiros.map((r: any) => (
                        <Fragment key={r.id}>
                          <tr>
                            <td style={{ fontSize: 12 }}>{r.fecha?.slice(11, 16)}</td>
                            <td style={{ color: "var(--color-danger)", fontWeight: 600 }}>-${r.monto.toFixed(2)}</td>
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
                      ))}
                    </tbody>
                  </table>
                </div>
              </div>
            )}
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
                                        title="Reimprimir reporte de cierre en impresora térmica"
                                        onClick={async (e) => {
                                          e.stopPropagation();
                                          if (!s.id) return;
                                          try {
                                            await imprimirReporteCaja(s.id);
                                            toastExito("Reporte enviado a la impresora");
                                          } catch (err) { toastError("Error: " + err); }
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
