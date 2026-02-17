import { useState, useEffect } from "react";
import { abrirCaja, cerrarCaja, obtenerCajaAbierta, imprimirReporteCaja, imprimirReporteCajaPdf, obtenerConfig } from "../services/api";
import { useToast } from "../components/Toast";
import { useSesion } from "../contexts/SesionContext";
import Modal from "../components/Modal";
import type { Caja, ResumenCaja } from "../types";

export default function CajaPage() {
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

  const cargar = async () => {
    setCargando(true);
    const caja = await obtenerCajaAbierta();
    setCajaAbierta(caja);
    setCargando(false);
  };

  useEffect(() => { cargar(); }, []);
  useEffect(() => {
    obtenerConfig().then((cfg) => setTicketUsarPdf(cfg.ticket_usar_pdf === "1")).catch(() => {});
  }, []);

  const handleAbrir = async () => {
    try {
      const caja = await abrirCaja(parseFloat(montoInicial) || 0);
      setCajaAbierta(caja);
      setMontoInicial("");
      toastExito("Caja abierta correctamente");
    } catch (err) {
      toastError("Error: " + err);
    }
  };

  const handleCerrar = async () => {
    setConfirmarCierre(false);
    try {
      const res = await cerrarCaja(parseFloat(montoReal) || 0, observacion || undefined);
      setResumen(res);
      setCajaAbierta(null);
      setMontoReal("");
      setObservacion("");
      toastExito("Caja cerrada correctamente");
      // Nota: el backend ya cerro la sesion. Mostramos el resumen primero.
    } catch (err) {
      toastError("Error: " + err);
    }
  };

  const handleFinalizarTurno = async () => {
    setResumen(null);
    // El backend ya cerro la sesion, solo actualizamos el frontend
    await cerrarSesion();
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
              background: "#dcfce7",
              color: "#166534",
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
                  <span className="text-secondary">Total efectivo:</span>
                  <div className="text-lg font-bold">${resumen.total_efectivo.toFixed(2)}</div>
                </div>
                <div>
                  <span className="text-secondary">Total gastos:</span>
                  <div className="text-lg font-bold">${resumen.total_gastos.toFixed(2)}</div>
                </div>
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
          <div className="card" style={{ maxWidth: 400, margin: "40px auto" }}>
            <div className="card-header">Abrir Caja</div>
            <div className="card-body">
              {sesion && (
                <div className="mb-4" style={{ fontSize: 13 }}>
                  <span className="text-secondary">Cajero:</span>
                  <span className="font-bold" style={{ marginLeft: 8 }}>{sesion.nombre}</span>
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
              <button className="btn btn-success btn-lg mt-4" style={{ width: "100%" }} onClick={handleAbrir}>
                Abrir Caja
              </button>
            </div>
          </div>
        )}

        {!resumen && cajaAbierta && (
          <div className="card" style={{ maxWidth: 400, margin: "40px auto" }}>
            <div className="card-header">Cerrar Caja</div>
            <div className="card-body">
              <div className="mb-4">
                <span className="text-secondary">Monto inicial:</span>
                <span className="font-bold" style={{ marginLeft: 8 }}>${cajaAbierta.monto_inicial.toFixed(2)}</span>
              </div>
              <div>
                <label className="text-secondary" style={{ fontSize: 12 }}>Monto real en caja</label>
                <input
                  className="input input-lg mt-2"
                  type="number"
                  step="0.01"
                  placeholder="0.00"
                  value={montoReal}
                  onChange={(e) => setMontoReal(e.target.value)}
                />
              </div>
              <div className="mt-4">
                <label className="text-secondary" style={{ fontSize: 12 }}>Observacion (opcional)</label>
                <input
                  className="input mt-2"
                  value={observacion}
                  onChange={(e) => setObservacion(e.target.value)}
                />
              </div>
              <button className="btn btn-danger btn-lg mt-4" style={{ width: "100%" }} onClick={() => setConfirmarCierre(true)}>
                Cerrar Caja
              </button>
            </div>
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
    </>
  );
}
