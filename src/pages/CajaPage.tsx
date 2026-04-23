import { useState, useEffect, Fragment } from "react";
import { useNavigate } from "react-router-dom";
import { abrirCaja, cerrarCaja, obtenerCajaAbierta, imprimirReporteCaja, imprimirReporteCajaPdf, obtenerConfig, registrarRetiro, listarRetirosCaja, listarCuentasBanco, confirmarDeposito, obtenerUltimoCierre, historialDescuadresCaja } from "../services/api";
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
  // Modal historial descuadres
  const [mostrarHistorial, setMostrarHistorial] = useState(false);
  const [historialData, setHistorialData] = useState<any>(null);
  const [historialDesde, setHistorialDesde] = useState(() => {
    const d = new Date(); d.setDate(d.getDate() - 30);
    return d.toISOString().slice(0, 10);
  });
  const [historialHasta, setHistorialHasta] = useState(() => new Date().toISOString().slice(0, 10));

  const cargar = async () => {
    setCargando(true);
    const caja = await obtenerCajaAbierta();
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
    setCargando(false);
  };

  useEffect(() => { cargar(); }, []);
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

  const handleCerrar = async () => {
    setConfirmarCierre(false);
    const monto = parseFloat(montoReal) || 0;
    // Calcular descuadre esperado para validar motivo localmente
    const totalRetiros = retiros.reduce((s: number, r: any) => s + (Number(r.monto) || 0), 0);
    const esperado = (cajaAbierta?.monto_inicial || 0) + (cajaAbierta?.monto_ventas || 0) - totalRetiros;
    const dif = monto - esperado;
    if (Math.abs(dif) > 0.01 && motivoDescuadre.trim().length < 5) {
      toastError(`Hay un descuadre de $${dif.toFixed(2)}. Debe explicar el motivo (mínimo 5 caracteres).`);
      return;
    }
    try {
      const res = await cerrarCaja(monto, observacion || undefined, motivoDescuadre.trim() || undefined);
      setResumen(res);
      setCajaAbierta(null);
      setMontoReal("");
      setObservacion("");
      setMotivoDescuadre("");
      toastExito("Caja cerrada correctamente");
    } catch (err) {
      const msg = String(err);
      if (msg.includes("DESCUADRE_CIERRE")) {
        toastError(msg.split(":").slice(3).join(":").trim() || msg);
      } else {
        toastError("Error: " + err);
      }
    }
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
              try {
                const d = await historialDescuadresCaja(historialDesde, historialHasta);
                setHistorialData(d);
              } catch (err) { toastError("Error: " + err); }
            }}>
            📊 Historial descuadres
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
                if (!montoReal || Math.abs(dif) <= 0.01) return null;
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

      {/* Modal: Historial de descuadres */}
      {mostrarHistorial && (
        <div className="modal-overlay" onClick={() => setMostrarHistorial(false)}>
          <div className="modal-content" onClick={(e) => e.stopPropagation()} style={{ maxWidth: 900, maxHeight: "90vh", overflowY: "auto" }}>
            <div className="modal-header">
              <h3>📊 Historial de descuadres de caja</h3>
            </div>
            <div className="modal-body">
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
                      const d = await historialDescuadresCaja(historialDesde, historialHasta);
                      setHistorialData(d);
                    } catch (err) { toastError("Error: " + err); }
                  }}>
                  Filtrar
                </button>
              </div>

              {!historialData ? (
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
              )}
            </div>
            <div className="modal-footer">
              <button className="btn btn-outline" onClick={() => setMostrarHistorial(false)}>Cerrar</button>
            </div>
          </div>
        </div>
      )}
    </>
  );
}
