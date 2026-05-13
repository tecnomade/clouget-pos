/**
 * v2.5.4 — Modal para registrar retenciones recibidas (SRI Ecuador).
 *
 * Cuando un cliente (empresa) compra y nos retiene IVA y/o Renta, registramos
 * el comprobante de retención aquí. Esto reduce el saldo pendiente de la
 * factura para que se considere cobrada totalmente.
 *
 * Soporta:
 * - Lista de retenciones ya registradas (eliminar)
 * - Form para agregar nueva retención (Renta o IVA)
 * - Cálculo automático: valor = base × % / 100
 * - Validación: no excede saldo pendiente
 */
import { useEffect, useMemo, useState } from "react";
import {
  listarRetencionesVenta, registrarRetencion, eliminarRetencion,
  type RetencionRecibida,
} from "../services/api";
import { RETENCIONES_RENTA, RETENCIONES_IVA } from "../config/retencionesSri";
import { useToast } from "./Toast";

interface Props {
  ventaId: number;
  numero: string;
  total: number;
  subtotal: number;     // base sin IVA (para retención de Renta)
  iva: number;          // valor del IVA (base para retención de IVA)
  totalCobrado: number; // pagos_venta o monto_recibido
  onClose: () => void;
  onChanged?: () => void;
}

function fechaHoy(): string {
  const now = new Date();
  return `${now.getFullYear()}-${String(now.getMonth()+1).padStart(2,'0')}-${String(now.getDate()).padStart(2,'0')}`;
}

export default function ModalRetenciones({
  ventaId, numero, total, subtotal, iva, totalCobrado, onClose, onChanged,
}: Props) {
  const { toastExito, toastError } = useToast();
  const [retenciones, setRetenciones] = useState<RetencionRecibida[]>([]);
  const [cargando, setCargando] = useState(true);

  // Form
  const [tipo, setTipo] = useState<"RENTA" | "IVA">("IVA");
  const [codigoSel, setCodigoSel] = useState<string>("");
  const [baseInput, setBaseInput] = useState("");
  const [porcInput, setPorcInput] = useState("");
  const [numComprobante, setNumComprobante] = useState("");
  const [fecha, setFecha] = useState(fechaHoy());
  const [observacion, setObservacion] = useState("");

  const recargar = async () => {
    setCargando(true);
    try {
      const r = await listarRetencionesVenta(ventaId);
      setRetenciones(r || []);
    } catch (err) {
      toastError("Error cargando retenciones: " + err);
    } finally {
      setCargando(false);
    }
  };

  useEffect(() => { recargar(); }, [ventaId]); // eslint-disable-line react-hooks/exhaustive-deps

  const totalRetenido = useMemo(() => retenciones.reduce((s, r) => s + r.valor, 0), [retenciones]);
  const saldoPendiente = Math.max(total - totalCobrado - totalRetenido, 0);
  const esCancelada = saldoPendiente <= 0.001 && (totalCobrado + totalRetenido) >= total - 0.001;

  // Catalogo segun tipo seleccionado
  const catalogo = tipo === "RENTA" ? RETENCIONES_RENTA : RETENCIONES_IVA;

  // Auto-llenar base segun tipo: Renta usa subtotal, IVA usa el valor del IVA
  useEffect(() => {
    if (tipo === "RENTA" && !baseInput) {
      setBaseInput(subtotal.toFixed(2));
    } else if (tipo === "IVA" && !baseInput) {
      setBaseInput(iva.toFixed(2));
    }
  }, [tipo]); // eslint-disable-line react-hooks/exhaustive-deps

  // Cuando cambia tipo, resetear base + codigo
  const cambiarTipo = (nuevo: "RENTA" | "IVA") => {
    setTipo(nuevo);
    setCodigoSel("");
    setPorcInput("");
    setBaseInput(nuevo === "RENTA" ? subtotal.toFixed(2) : iva.toFixed(2));
  };

  // Cuando se selecciona codigo, autollenar porcentaje
  const cambiarCodigo = (cod: string) => {
    setCodigoSel(cod);
    const def = catalogo.find(c => c.codigo === cod);
    if (def && def.porcentaje > 0) {
      setPorcInput(def.porcentaje.toString());
    }
  };

  const baseNum = parseFloat(baseInput) || 0;
  const porcNum = parseFloat(porcInput) || 0;
  const valorCalc = +(baseNum * porcNum / 100).toFixed(2);

  const handleGuardar = async () => {
    if (!codigoSel) { toastError("Selecciona el código SRI"); return; }
    if (baseNum <= 0) { toastError("La base imponible debe ser mayor a 0"); return; }
    if (porcNum <= 0) { toastError("El porcentaje debe ser mayor a 0"); return; }
    if (valorCalc <= 0) { toastError("El valor debe ser mayor a 0"); return; }
    if (!numComprobante.trim()) { toastError("Ingresa el número del comprobante de retención"); return; }
    if (!fecha) { toastError("Ingresa la fecha de emisión"); return; }

    try {
      await registrarRetencion(
        ventaId, tipo, codigoSel, baseNum, porcNum, valorCalc,
        numComprobante.trim(), fecha, observacion.trim() || null,
      );
      toastExito(`Retención de $${valorCalc.toFixed(2)} registrada`);
      // Reset form
      setCodigoSel("");
      setPorcInput("");
      setBaseInput("");
      setNumComprobante("");
      setObservacion("");
      await recargar();
      onChanged?.();
    } catch (err) {
      toastError("Error: " + err);
    }
  };

  const handleEliminar = async (r: RetencionRecibida) => {
    if (!r.id) return;
    if (!confirm(`¿Eliminar retención ${r.tipo} de $${r.valor.toFixed(2)} (comp. ${r.numero_comprobante})?`)) return;
    try {
      await eliminarRetencion(r.id);
      toastExito("Retención eliminada");
      await recargar();
      onChanged?.();
    } catch (err) {
      toastError("Error: " + err);
    }
  };

  return (
    <div
      onClick={(e) => { if (e.target === e.currentTarget) onClose(); }}
      style={{
        position: "fixed", inset: 0, background: "rgba(0,0,0,0.55)", zIndex: 200,
        display: "flex", alignItems: "center", justifyContent: "center", padding: 20,
      }}
    >
      <div className="card" style={{ width: 720, maxHeight: "90vh", overflow: "auto" }}>
        <div className="card-header flex justify-between items-center">
          <span>📋 Retenciones recibidas — Factura {numero}</span>
          <button className="btn btn-outline" style={{ padding: "2px 8px" }} onClick={onClose}>x</button>
        </div>
        <div className="card-body">
          {/* Resumen factura */}
          <div style={{
            display: "grid", gridTemplateColumns: "repeat(4, 1fr)", gap: 8,
            marginBottom: 16, padding: 12,
            background: "var(--color-surface-alt)", borderRadius: 8, fontSize: 12,
          }}>
            <div>
              <div style={{ color: "var(--color-text-secondary)" }}>Total factura</div>
              <div style={{ fontWeight: 700, fontSize: 14 }}>${total.toFixed(2)}</div>
            </div>
            <div>
              <div style={{ color: "var(--color-text-secondary)" }}>Cobrado</div>
              <div style={{ fontWeight: 700, fontSize: 14, color: "var(--color-success)" }}>${totalCobrado.toFixed(2)}</div>
            </div>
            <div>
              <div style={{ color: "var(--color-text-secondary)" }}>Retenido</div>
              <div style={{ fontWeight: 700, fontSize: 14, color: "var(--color-warning)" }}>${totalRetenido.toFixed(2)}</div>
            </div>
            <div>
              <div style={{ color: "var(--color-text-secondary)" }}>Saldo pendiente</div>
              <div style={{
                fontWeight: 700, fontSize: 14,
                color: esCancelada ? "var(--color-success)" : (saldoPendiente > 0 ? "var(--color-danger)" : "var(--color-text)"),
              }}>
                {esCancelada ? "✓ CANCELADA" : `$${saldoPendiente.toFixed(2)}`}
              </div>
            </div>
          </div>

          {/* Lista de retenciones registradas */}
          {cargando ? (
            <div style={{ padding: 20, textAlign: "center", color: "var(--color-text-secondary)" }}>Cargando...</div>
          ) : retenciones.length > 0 ? (
            <div style={{ marginBottom: 16 }}>
              <div style={{ fontSize: 12, fontWeight: 600, marginBottom: 6, color: "var(--color-text-secondary)" }}>
                Retenciones aplicadas
              </div>
              <table style={{ width: "100%", fontSize: 12, borderCollapse: "collapse" }}>
                <thead>
                  <tr style={{ textAlign: "left", borderBottom: "1px solid var(--color-border)" }}>
                    <th style={{ padding: "4px 6px" }}>Tipo</th>
                    <th style={{ padding: "4px 6px" }}>Código</th>
                    <th style={{ padding: "4px 6px", textAlign: "right" }}>Base</th>
                    <th style={{ padding: "4px 6px", textAlign: "right" }}>%</th>
                    <th style={{ padding: "4px 6px", textAlign: "right" }}>Valor</th>
                    <th style={{ padding: "4px 6px" }}>Comp.</th>
                    <th style={{ padding: "4px 6px" }}>Fecha</th>
                    <th style={{ padding: "4px 6px" }}></th>
                  </tr>
                </thead>
                <tbody>
                  {retenciones.map(r => (
                    <tr key={r.id} style={{ borderBottom: "1px solid var(--color-border)" }}>
                      <td style={{ padding: "4px 6px" }}>
                        <span style={{
                          fontSize: 10, padding: "1px 6px", borderRadius: 4, fontWeight: 600,
                          background: r.tipo === "RENTA" ? "rgba(168,85,247,0.15)" : "rgba(245,158,11,0.15)",
                          color: r.tipo === "RENTA" ? "#a855f7" : "var(--color-warning)",
                        }}>{r.tipo}</span>
                      </td>
                      <td style={{ padding: "4px 6px", fontFamily: "monospace" }}>{r.codigo_sri}</td>
                      <td style={{ padding: "4px 6px", textAlign: "right" }}>${r.base_imponible.toFixed(2)}</td>
                      <td style={{ padding: "4px 6px", textAlign: "right" }}>{r.porcentaje}%</td>
                      <td style={{ padding: "4px 6px", textAlign: "right", fontWeight: 600 }}>${r.valor.toFixed(2)}</td>
                      <td style={{ padding: "4px 6px", fontSize: 11 }}>{r.numero_comprobante}</td>
                      <td style={{ padding: "4px 6px", fontSize: 11 }}>{r.fecha_emision}</td>
                      <td style={{ padding: "4px 6px", textAlign: "right" }}>
                        <button onClick={() => handleEliminar(r)}
                          style={{
                            fontSize: 11, padding: "2px 8px",
                            border: "1px solid var(--color-danger)",
                            background: "rgba(239,68,68,0.1)",
                            color: "var(--color-danger)",
                            borderRadius: 4, cursor: "pointer",
                          }}
                          title="Eliminar retención">🗑</button>
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          ) : (
            <div style={{ padding: 12, textAlign: "center", fontSize: 12, color: "var(--color-text-secondary)", marginBottom: 16 }}>
              Sin retenciones registradas todavía.
            </div>
          )}

          {/* Form para nueva retención */}
          <div style={{ padding: 12, background: "var(--color-surface-alt)", borderRadius: 8 }}>
            <div style={{ fontSize: 13, fontWeight: 700, marginBottom: 10 }}>➕ Registrar nueva retención</div>

            {/* Tipo */}
            <div style={{ display: "flex", gap: 8, marginBottom: 10 }}>
              <button onClick={() => cambiarTipo("IVA")}
                style={{
                  flex: 1, padding: "8px 12px", borderRadius: 6, cursor: "pointer", fontSize: 12, fontWeight: 600,
                  border: tipo === "IVA" ? "2px solid var(--color-warning)" : "1px solid var(--color-border)",
                  background: tipo === "IVA" ? "rgba(245,158,11,0.1)" : "transparent",
                  color: tipo === "IVA" ? "var(--color-warning)" : "var(--color-text)",
                }}>Retención de IVA (Tabla 21)</button>
              <button onClick={() => cambiarTipo("RENTA")}
                style={{
                  flex: 1, padding: "8px 12px", borderRadius: 6, cursor: "pointer", fontSize: 12, fontWeight: 600,
                  border: tipo === "RENTA" ? "2px solid #a855f7" : "1px solid var(--color-border)",
                  background: tipo === "RENTA" ? "rgba(168,85,247,0.1)" : "transparent",
                  color: tipo === "RENTA" ? "#a855f7" : "var(--color-text)",
                }}>Retención de Renta (Tabla 304)</button>
            </div>

            {/* Codigo SRI */}
            <div style={{ marginBottom: 8 }}>
              <label style={{ fontSize: 11, fontWeight: 600, color: "var(--color-text-secondary)" }}>Código SRI *</label>
              <select className="input" value={codigoSel} onChange={(e) => cambiarCodigo(e.target.value)}
                style={{ width: "100%", fontSize: 12 }}>
                <option value="">— Selecciona código —</option>
                {catalogo.map(c => (
                  <option key={c.codigo} value={c.codigo}>
                    {c.codigo} · {c.descripcion} {c.porcentaje > 0 ? `(${c.porcentaje}%)` : "(% variable)"}
                  </option>
                ))}
              </select>
            </div>

            {/* Base + porc + valor */}
            <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr 1fr", gap: 8, marginBottom: 8 }}>
              <div>
                <label style={{ fontSize: 11, fontWeight: 600, color: "var(--color-text-secondary)" }}>
                  Base imponible *
                </label>
                <input className="input" type="number" step="0.01" min="0"
                  value={baseInput} onChange={(e) => setBaseInput(e.target.value)}
                  style={{ width: "100%", fontSize: 12 }} />
                <div style={{ fontSize: 9, color: "var(--color-text-secondary)" }}>
                  {tipo === "RENTA" ? `Subtotal: $${subtotal.toFixed(2)}` : `IVA: $${iva.toFixed(2)}`}
                </div>
              </div>
              <div>
                <label style={{ fontSize: 11, fontWeight: 600, color: "var(--color-text-secondary)" }}>Porcentaje *</label>
                <input className="input" type="number" step="0.01" min="0" max="100"
                  value={porcInput} onChange={(e) => setPorcInput(e.target.value)}
                  style={{ width: "100%", fontSize: 12 }} />
              </div>
              <div>
                <label style={{ fontSize: 11, fontWeight: 600, color: "var(--color-text-secondary)" }}>Valor (auto)</label>
                <input className="input" type="number" value={valorCalc.toFixed(2)} readOnly
                  style={{ width: "100%", fontSize: 12, fontWeight: 700, background: "rgba(34,197,94,0.05)" }} />
              </div>
            </div>

            {/* Comprobante */}
            <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: 8, marginBottom: 8 }}>
              <div>
                <label style={{ fontSize: 11, fontWeight: 600, color: "var(--color-text-secondary)" }}>
                  N° Comprobante *
                </label>
                <input className="input" placeholder="001-001-000000123"
                  value={numComprobante} onChange={(e) => setNumComprobante(e.target.value)}
                  style={{ width: "100%", fontSize: 12 }} />
              </div>
              <div>
                <label style={{ fontSize: 11, fontWeight: 600, color: "var(--color-text-secondary)" }}>
                  Fecha emisión *
                </label>
                <input className="input" type="date"
                  value={fecha} onChange={(e) => setFecha(e.target.value)}
                  style={{ width: "100%", fontSize: 12 }} />
              </div>
            </div>

            {/* Observacion */}
            <div style={{ marginBottom: 10 }}>
              <label style={{ fontSize: 11, fontWeight: 600, color: "var(--color-text-secondary)" }}>
                Observación (opcional)
              </label>
              <input className="input" placeholder="..."
                value={observacion} onChange={(e) => setObservacion(e.target.value)}
                style={{ width: "100%", fontSize: 12 }} />
            </div>

            <button className="btn btn-success" onClick={handleGuardar}
              style={{ width: "100%", fontSize: 13, padding: "8px 0", fontWeight: 700 }}>
              💾 Registrar retención
            </button>
          </div>
        </div>
      </div>
    </div>
  );
}
