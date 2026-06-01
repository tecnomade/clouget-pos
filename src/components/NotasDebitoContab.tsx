/**
 * v2.5.69 — Notas de Débito (SRI codDoc 05)
 *
 * Vive dentro del módulo Contabilidad (Documentos SRI). La emite el vendedor
 * para cobrar un valor adicional (interés por mora, recargo) sobre una factura
 * ya emitida al cliente. Lista + crear (cliente + factura + motivos) + emitir.
 */
import { useState, useEffect } from "react";
import {
  listarNotasDebito, crearNotaDebito, emitirNotaDebitoSri, anularNotaDebito,
  buscarClientes, generarRideNotaDebitoPdf, enviarEmailDocSri,
  type NotaDebitoResumen, type MotivoNd,
} from "../services/api";
import { useToast } from "./Toast";
import type { Cliente } from "../types";

function fechaHace(dias: number): string {
  const d = new Date(); d.setDate(d.getDate() - dias);
  return d.toISOString().slice(0, 10);
}
function fechaHoy(): string { return new Date().toISOString().slice(0, 10); }

export default function NotasDebitoContab() {
  const { toastExito, toastError } = useToast();
  const [lista, setLista] = useState<NotaDebitoResumen[]>([]);
  const [cargando, setCargando] = useState(false);

  const [creando, setCreando] = useState(false);
  const [cliBusq, setCliBusq] = useState("");
  const [cliRes, setCliRes] = useState<Cliente[]>([]);
  const [cliente, setCliente] = useState<Cliente | null>(null);
  const [numFactura, setNumFactura] = useState("");
  const [fechaFactura, setFechaFactura] = useState("");
  const [aplicaIva, setAplicaIva] = useState(false);
  const [motivos, setMotivos] = useState<MotivoNd[]>([{ razon: "", valor: 0 }]);
  const [guardando, setGuardando] = useState(false);

  const [emitiendo, setEmitiendo] = useState<number | null>(null);

  const cargar = async () => {
    setCargando(true);
    try { setLista(await listarNotasDebito(fechaHace(60), fechaHoy())); }
    catch (e) { toastError("Error cargando notas de débito: " + e); }
    finally { setCargando(false); }
  };
  useEffect(() => { cargar(); }, []);

  useEffect(() => {
    if (!creando) return;
    const t = setTimeout(async () => {
      if (cliBusq.trim().length < 2) { setCliRes([]); return; }
      try { setCliRes(await buscarClientes(cliBusq.trim())); } catch { /* ignore */ }
    }, 300);
    return () => clearTimeout(t);
  }, [cliBusq, creando]);

  const abrirCrear = () => {
    setCliente(null); setCliBusq(""); setCliRes([]);
    setNumFactura(""); setFechaFactura(""); setAplicaIva(false);
    setMotivos([{ razon: "", valor: 0 }]);
    setCreando(true);
  };

  const setMotivo = (i: number, campo: keyof MotivoNd, val: any) =>
    setMotivos(prev => prev.map((m, idx) => idx === i ? { ...m, [campo]: val } : m));
  const addMotivo = () => setMotivos(prev => [...prev, { razon: "", valor: 0 }]);
  const quitarMotivo = (i: number) => setMotivos(prev => prev.filter((_, idx) => idx !== i));

  const base = motivos.reduce((s, m) => s + (m.valor || 0), 0);
  const total = base + (aplicaIva ? base * 0.15 : 0);

  const crear = async () => {
    if (!cliente?.id) { toastError("Seleccione un cliente"); return; }
    if (!numFactura.trim()) { toastError("Indique el número de la factura (001-001-000000001)"); return; }
    if (motivos.length === 0 || motivos.some(m => !m.razon.trim() || m.valor <= 0)) {
      toastError("Cada motivo requiere razón y valor > 0"); return;
    }
    setGuardando(true);
    try {
      const res = await crearNotaDebito({
        cliente_id: cliente.id, num_doc_modificado: numFactura.trim(),
        fecha_doc_modificado: fechaFactura || undefined, aplica_iva: aplicaIva, motivos,
      });
      setCreando(false);
      toastExito("Nota de débito creada. Emitiendo al SRI...");
      await emitir(res.id);
    } catch (e) { toastError("Error al crear: " + e); }
    finally { setGuardando(false); }
  };

  const emitir = async (id: number) => {
    setEmitiendo(id);
    try {
      const res = await emitirNotaDebitoSri(id);
      if (res.exito) toastExito(`Nota de débito autorizada (${res.numero_factura || res.clave_acceso || ""})`);
      else toastError(`SRI: ${res.mensaje || res.estado_sri}`);
      cargar();
    } catch (e) { toastError("Error al emitir: " + e); }
    finally { setEmitiendo(null); }
  };

  const anular = async (id: number) => {
    if (!confirm("¿Anular esta nota de débito?")) return;
    try { await anularNotaDebito(id); toastExito("Nota de débito anulada"); cargar(); }
    catch (e) { toastError("Error: " + e); }
  };

  const descargarPdf = async (id: number, numero: string) => {
    try {
      const bytes = await generarRideNotaDebitoPdf(id);
      const blob = new Blob([new Uint8Array(bytes)], { type: "application/pdf" });
      const url = URL.createObjectURL(blob);
      const a = document.createElement("a");
      a.href = url; a.download = `NotaDebito-${numero}.pdf`;
      document.body.appendChild(a); a.click(); document.body.removeChild(a);
      setTimeout(() => URL.revokeObjectURL(url), 60_000);
      toastExito("PDF generado");
    } catch (e) { toastError("Error generando PDF: " + e); }
  };

  const enviarEmail = async (id: number) => {
    const email = prompt("Email del cliente para enviar la nota de débito:");
    if (!email || !email.trim()) return;
    try {
      const msg = await enviarEmailDocSri("NOTA_DEBITO", id, email.trim());
      toastExito(msg);
    } catch (e) { toastError(String(e)); }
  };

  const badge = (estadoSri: string, anulada: boolean) => {
    if (anulada) return <span style={{ fontSize: 10, padding: "2px 8px", borderRadius: 3, fontWeight: 600, background: "rgba(148,163,184,0.15)", color: "var(--color-text-secondary)" }}>Anulada</span>;
    const map: Record<string, { bg: string; col: string; txt: string }> = {
      AUTORIZADA: { bg: "rgba(74,222,128,0.15)", col: "var(--color-success)", txt: "✓ Autorizada" },
      PENDIENTE: { bg: "rgba(251,191,36,0.15)", col: "var(--color-warning)", txt: "Pendiente" },
      RECHAZADA: { bg: "rgba(239,68,68,0.15)", col: "var(--color-danger)", txt: "Rechazada" },
      NO_APLICA: { bg: "rgba(148,163,184,0.15)", col: "var(--color-text-secondary)", txt: "Sin emitir" },
    };
    const m = map[estadoSri] || map.NO_APLICA;
    return <span style={{ fontSize: 10, padding: "2px 8px", borderRadius: 3, fontWeight: 600, background: m.bg, color: m.col }}>{m.txt}</span>;
  };

  return (
    <div>
      <div className="card">
        <div className="card-header flex justify-between items-center">
          <span>Notas de Débito (SRI codDoc 05)</span>
          <button className="btn btn-primary" style={{ fontSize: 12, padding: "5px 12px" }} onClick={abrirCrear}>
            + Nueva Nota de Débito
          </button>
        </div>
        <div className="card-body" style={{ padding: 0 }}>
          <div style={{ padding: "10px 14px", fontSize: 12, color: "var(--color-text-secondary)", borderBottom: "1px solid var(--color-border)" }}>
            La <strong>Nota de Débito</strong> cobra un valor adicional (interés por mora, recargo) sobre una
            factura ya emitida al cliente. Poco frecuente. Últimos 60 días.
          </div>
          <table className="table" style={{ fontSize: 13 }}>
            <thead>
              <tr>
                <th>Número</th><th>Fecha</th><th>Cliente</th><th>Factura</th>
                <th className="text-right">Total</th><th>SRI</th><th></th>
              </tr>
            </thead>
            <tbody>
              {lista.map(n => (
                <tr key={n.id}>
                  <td><strong>{n.numero || n.numero_factura || `#${n.id}`}</strong></td>
                  <td className="text-secondary" style={{ fontSize: 12 }}>
                    {n.fecha_emision ? new Date(n.fecha_emision).toLocaleDateString("es-EC", { day: "2-digit", month: "2-digit", year: "2-digit" }) : "-"}
                  </td>
                  <td>{n.cliente_nombre}</td>
                  <td className="text-secondary" style={{ fontSize: 12 }}>{n.num_doc_modificado}</td>
                  <td className="text-right font-bold">${n.valor_total.toFixed(2)}</td>
                  <td>{badge(n.estado_sri, n.anulada)}</td>
                  <td>
                    <div className="flex gap-1">
                      {!n.anulada && n.estado_sri !== "AUTORIZADA" && (
                        <button className="btn btn-outline" style={{ fontSize: 10, padding: "2px 8px", fontWeight: 600, color: "var(--color-primary)", borderColor: "var(--color-primary)" }}
                          disabled={emitiendo === n.id} onClick={() => emitir(n.id)}>
                          {emitiendo === n.id ? "Enviando..." : n.estado_sri === "PENDIENTE" || n.estado_sri === "RECHAZADA" ? "↻ Reintentar SRI" : "📤 Emitir SRI"}
                        </button>
                      )}
                      {!n.anulada && n.estado_sri !== "AUTORIZADA" && (
                        <button className="btn btn-outline" style={{ fontSize: 10, padding: "2px 8px", color: "var(--color-danger)" }}
                          onClick={() => anular(n.id)}>Anular</button>
                      )}
                      {n.estado_sri === "AUTORIZADA" && (
                        <>
                          <button className="btn btn-outline" style={{ fontSize: 10, padding: "2px 8px" }}
                            onClick={() => descargarPdf(n.id, n.numero || n.numero_factura || String(n.id))}>PDF</button>
                          <button className="btn btn-outline" style={{ fontSize: 10, padding: "2px 8px" }}
                            onClick={() => enviarEmail(n.id)}>✉ Email</button>
                        </>
                      )}
                    </div>
                  </td>
                </tr>
              ))}
              {lista.length === 0 && !cargando && (
                <tr><td colSpan={7} className="text-center text-secondary" style={{ padding: 24 }}>No hay notas de débito en los últimos 60 días</td></tr>
              )}
            </tbody>
          </table>
        </div>
      </div>

      {/* Modal CREAR */}
      {creando && (
        <div style={{ position: "fixed", inset: 0, background: "rgba(0,0,0,0.5)", display: "flex", alignItems: "center", justifyContent: "center", zIndex: 150 }}
          onClick={(e) => { if (e.target === e.currentTarget) setCreando(false); }}>
          <div className="card" style={{ width: 640, maxHeight: "90vh", overflow: "auto" }}>
            <div className="card-header flex justify-between items-center">
              <span>Nueva Nota de Débito</span>
              <button className="btn btn-outline" style={{ padding: "2px 8px" }} onClick={() => setCreando(false)}>x</button>
            </div>
            <div className="card-body">
              {/* Cliente */}
              <label style={{ fontSize: 11, color: "var(--color-text-secondary)" }}>Cliente *</label>
              {cliente ? (
                <div className="flex justify-between items-center" style={{ padding: "6px 10px", background: "var(--color-surface-alt)", borderRadius: 6, marginBottom: 12 }}>
                  <span style={{ fontSize: 13 }}>{cliente.nombre}{cliente.identificacion ? ` — ${cliente.identificacion}` : ""}</span>
                  <button className="btn btn-outline" style={{ fontSize: 10, padding: "2px 8px" }} onClick={() => setCliente(null)}>Cambiar</button>
                </div>
              ) : (
                <div style={{ marginBottom: 12, position: "relative" }}>
                  <input className="input" style={{ width: "100%", fontSize: 13 }} placeholder="Buscar cliente por nombre o identificación..."
                    value={cliBusq} onChange={(e) => setCliBusq(e.target.value)} />
                  {cliRes.length > 0 && (
                    <div style={{ position: "absolute", zIndex: 5, background: "var(--color-surface)", border: "1px solid var(--color-border)", borderRadius: 6, width: "100%", maxHeight: 180, overflow: "auto" }}>
                      {cliRes.map(c => (
                        <div key={c.id} style={{ padding: "6px 10px", cursor: "pointer", fontSize: 13, borderBottom: "1px solid var(--color-border)" }}
                          onClick={() => { setCliente(c); setCliBusq(""); setCliRes([]); }}>
                          {c.nombre}{c.identificacion ? ` — ${c.identificacion}` : ""}
                        </div>
                      ))}
                    </div>
                  )}
                </div>
              )}

              {/* Factura modificada */}
              <div style={{ display: "grid", gridTemplateColumns: "2fr 1fr", gap: 8, marginBottom: 12 }}>
                <div>
                  <label style={{ fontSize: 11, color: "var(--color-text-secondary)" }}>Factura a cobrar (001-001-000000001) *</label>
                  <input className="input" style={{ width: "100%", fontSize: 13 }} placeholder="001-001-000000001"
                    value={numFactura} onChange={(e) => setNumFactura(e.target.value)} />
                </div>
                <div>
                  <label style={{ fontSize: 11, color: "var(--color-text-secondary)" }}>Fecha factura</label>
                  <input type="date" className="input" style={{ width: "100%", fontSize: 13 }}
                    value={fechaFactura} onChange={(e) => setFechaFactura(e.target.value)} />
                </div>
              </div>

              {/* Motivos */}
              <label style={{ fontSize: 11, color: "var(--color-text-secondary)" }}>Motivos del cargo *</label>
              <table className="table" style={{ fontSize: 12, marginBottom: 8 }}>
                <thead><tr><th>Razón</th><th className="text-right" style={{ width: 110 }}>Valor</th><th style={{ width: 30 }}></th></tr></thead>
                <tbody>
                  {motivos.map((m, i) => (
                    <tr key={i}>
                      <td><input className="input" style={{ width: "100%", fontSize: 12 }} placeholder="Interés por mora, recargo..." value={m.razon} onChange={(e) => setMotivo(i, "razon", e.target.value)} /></td>
                      <td><input type="number" className="input" style={{ width: 100, fontSize: 12, textAlign: "right" }} min={0} step="any" value={m.valor} onChange={(e) => setMotivo(i, "valor", parseFloat(e.target.value) || 0)} /></td>
                      <td>{motivos.length > 1 && <button className="btn btn-outline" style={{ fontSize: 10, padding: "2px 6px", color: "var(--color-danger)" }} onClick={() => quitarMotivo(i)}>x</button>}</td>
                    </tr>
                  ))}
                </tbody>
              </table>
              <button className="btn btn-outline" style={{ fontSize: 11, marginBottom: 12 }} onClick={addMotivo}>+ Agregar motivo</button>

              <label className="flex items-center gap-2" style={{ fontSize: 12, marginBottom: 12, cursor: "pointer" }}>
                <input type="checkbox" checked={aplicaIva} onChange={(e) => setAplicaIva(e.target.checked)} />
                Los cargos llevan IVA 15%
              </label>

              <div className="flex justify-between items-center" style={{ marginBottom: 12 }}>
                <span style={{ fontSize: 11, color: "var(--color-text-secondary)" }}>ℹ Al continuar se crea la nota y se emite al SRI.</span>
                <span style={{ fontWeight: 700, fontSize: 16 }}>Total: ${total.toFixed(2)}</span>
              </div>

              <button className="btn btn-primary" style={{ width: "100%", padding: "10px 0", fontWeight: 700 }}
                disabled={guardando} onClick={crear}>
                {guardando ? "Procesando..." : "Crear y Emitir al SRI"}
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
