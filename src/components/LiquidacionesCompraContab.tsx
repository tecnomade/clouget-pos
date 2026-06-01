/**
 * v2.5.69 — Liquidaciones de Compra (SRI codDoc 03)
 *
 * Vive dentro del módulo Contabilidad (Documentos SRI). La emite el negocio
 * cuando compra a un proveedor que NO puede facturar (agricultor, reciclador,
 * informal). Lista + crear (con buscador de productos y proveedor) + emitir SRI.
 */
import { useState, useEffect } from "react";
import {
  listarLiquidacionesCompra, crearLiquidacionCompra, emitirLiquidacionCompraSri,
  anularLiquidacionCompra, buscarProveedores, buscarProductos,
  generarRideLiquidacionPdf, enviarEmailDocSri,
  type LiquidacionResumen, type ItemLiquidacion,
} from "../services/api";
import { useToast } from "./Toast";
import type { Proveedor, ProductoBusqueda } from "../types";

interface ItemUI {
  codigo?: string;
  descripcion: string;
  cantidad: number;
  precio_unitario: number;
  iva_porcentaje: number;
}

function fechaHace(dias: number): string {
  const d = new Date(); d.setDate(d.getDate() - dias);
  return d.toISOString().slice(0, 10);
}
function fechaHoy(): string { return new Date().toISOString().slice(0, 10); }

export default function LiquidacionesCompraContab() {
  const { toastExito, toastError } = useToast();
  const [lista, setLista] = useState<LiquidacionResumen[]>([]);
  const [cargando, setCargando] = useState(false);

  const [creando, setCreando] = useState(false);
  const [provBusq, setProvBusq] = useState("");
  const [provRes, setProvRes] = useState<Proveedor[]>([]);
  const [proveedor, setProveedor] = useState<Proveedor | null>(null);
  const [items, setItems] = useState<ItemUI[]>([]);
  const [busqProd, setBusqProd] = useState("");
  const [prodRes, setProdRes] = useState<ProductoBusqueda[]>([]);
  const [guardando, setGuardando] = useState(false);

  const [emitiendo, setEmitiendo] = useState<number | null>(null);

  const cargar = async () => {
    setCargando(true);
    try {
      setLista(await listarLiquidacionesCompra(fechaHace(60), fechaHoy()));
    } catch (e) { toastError("Error cargando liquidaciones: " + e); }
    finally { setCargando(false); }
  };
  useEffect(() => { cargar(); }, []);

  useEffect(() => {
    if (!creando) return;
    const t = setTimeout(async () => {
      if (provBusq.trim().length < 2) { setProvRes([]); return; }
      try { setProvRes(await buscarProveedores(provBusq.trim())); } catch { /* ignore */ }
    }, 300);
    return () => clearTimeout(t);
  }, [provBusq, creando]);

  useEffect(() => {
    if (!creando) return;
    const t = setTimeout(async () => {
      if (busqProd.trim().length < 2) { setProdRes([]); return; }
      try { setProdRes(await buscarProductos(busqProd.trim())); } catch { /* ignore */ }
    }, 300);
    return () => clearTimeout(t);
  }, [busqProd, creando]);

  const abrirCrear = () => {
    setProveedor(null); setProvBusq(""); setProvRes([]);
    setItems([]); setBusqProd(""); setProdRes([]);
    setCreando(true);
  };

  const agregarProducto = (p: ProductoBusqueda) => {
    setItems(prev => [...prev, {
      codigo: p.codigo, descripcion: p.nombre,
      cantidad: 1, precio_unitario: p.precio_costo || 0, iva_porcentaje: p.iva_porcentaje || 0,
    }]);
    setBusqProd(""); setProdRes([]);
  };
  const agregarLibre = () => {
    setItems(prev => [...prev, { descripcion: "", cantidad: 1, precio_unitario: 0, iva_porcentaje: 0 }]);
  };
  const setItem = (i: number, campo: keyof ItemUI, val: any) => {
    setItems(prev => prev.map((it, idx) => idx === i ? { ...it, [campo]: val } : it));
  };
  const quitarItem = (i: number) => setItems(prev => prev.filter((_, idx) => idx !== i));

  const totalCalc = items.reduce((s, it) => {
    const base = it.cantidad * it.precio_unitario;
    return s + base + base * (it.iva_porcentaje / 100);
  }, 0);

  const crear = async () => {
    if (!proveedor?.id) { toastError("Seleccione un proveedor"); return; }
    if (!proveedor.ruc) { toastError("El proveedor no tiene RUC/cédula configurada (requerido para el SRI)"); return; }
    if (items.length === 0) { toastError("Agregue al menos un producto"); return; }
    if (items.some(it => !it.descripcion.trim() || it.cantidad <= 0)) { toastError("Cada línea requiere descripción y cantidad > 0"); return; }
    setGuardando(true);
    try {
      const payloadItems: ItemLiquidacion[] = items.map(it => ({
        codigo: it.codigo, descripcion: it.descripcion, cantidad: it.cantidad,
        precio_unitario: it.precio_unitario, descuento: 0, iva_porcentaje: it.iva_porcentaje,
      }));
      const res = await crearLiquidacionCompra({ proveedor_id: proveedor.id, items: payloadItems });
      setCreando(false);
      toastExito("Liquidación creada. Emitiendo al SRI...");
      await emitir(res.id);
    } catch (e) {
      toastError("Error al crear: " + e);
    } finally {
      setGuardando(false);
    }
  };

  const emitir = async (id: number) => {
    setEmitiendo(id);
    try {
      const res = await emitirLiquidacionCompraSri(id);
      if (res.exito) toastExito(`Liquidación autorizada por el SRI (${res.numero_factura || res.clave_acceso || ""})`);
      else toastError(`SRI: ${res.mensaje || res.estado_sri}`);
      cargar();
    } catch (e) {
      toastError("Error al emitir: " + e);
    } finally {
      setEmitiendo(null);
    }
  };

  const anular = async (id: number) => {
    if (!confirm("¿Anular esta liquidación de compra?")) return;
    try { await anularLiquidacionCompra(id); toastExito("Liquidación anulada"); cargar(); }
    catch (e) { toastError("Error: " + e); }
  };

  const descargarPdf = async (id: number, numero: string) => {
    try {
      const bytes = await generarRideLiquidacionPdf(id);
      const blob = new Blob([new Uint8Array(bytes)], { type: "application/pdf" });
      const url = URL.createObjectURL(blob);
      const a = document.createElement("a");
      a.href = url; a.download = `Liquidacion-${numero}.pdf`;
      document.body.appendChild(a); a.click(); document.body.removeChild(a);
      setTimeout(() => URL.revokeObjectURL(url), 60_000);
      toastExito("PDF generado");
    } catch (e) { toastError("Error generando PDF: " + e); }
  };

  const enviarEmail = async (id: number) => {
    const email = prompt("Email del proveedor para enviar la liquidación:");
    if (!email || !email.trim()) return;
    try {
      const msg = await enviarEmailDocSri("LIQUIDACION", id, email.trim());
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
          <span>Liquidaciones de Compra (SRI codDoc 03)</span>
          <button className="btn btn-primary" style={{ fontSize: 12, padding: "5px 12px" }} onClick={abrirCrear}>
            + Nueva Liquidación
          </button>
        </div>
        <div className="card-body" style={{ padding: 0 }}>
          <div style={{ padding: "10px 14px", fontSize: 12, color: "var(--color-text-secondary)", borderBottom: "1px solid var(--color-border)" }}>
            La <strong>Liquidación de Compra</strong> la emites tú cuando compras a un proveedor que <strong>no puede facturar</strong>
            (agricultor, reciclador, informal). Sustituye a su factura ante el SRI. Últimos 60 días.
          </div>
          <table className="table" style={{ fontSize: 13 }}>
            <thead>
              <tr>
                <th>Número</th><th>Fecha</th><th>Proveedor</th>
                <th className="text-right">Total</th><th>SRI</th><th></th>
              </tr>
            </thead>
            <tbody>
              {lista.map(l => (
                <tr key={l.id}>
                  <td><strong>{l.numero || l.numero_factura || `#${l.id}`}</strong></td>
                  <td className="text-secondary" style={{ fontSize: 12 }}>
                    {l.fecha_emision ? new Date(l.fecha_emision).toLocaleDateString("es-EC", { day: "2-digit", month: "2-digit", year: "2-digit" }) : "-"}
                  </td>
                  <td>{l.proveedor_nombre}{l.proveedor_ruc ? ` — ${l.proveedor_ruc}` : ""}</td>
                  <td className="text-right font-bold">${l.total.toFixed(2)}</td>
                  <td>{badge(l.estado_sri, l.anulada)}</td>
                  <td>
                    <div className="flex gap-1">
                      {!l.anulada && l.estado_sri !== "AUTORIZADA" && (
                        <button className="btn btn-outline" style={{ fontSize: 10, padding: "2px 8px", fontWeight: 600, color: "var(--color-primary)", borderColor: "var(--color-primary)" }}
                          disabled={emitiendo === l.id}
                          onClick={() => emitir(l.id)}>
                          {emitiendo === l.id ? "Enviando..." : l.estado_sri === "PENDIENTE" || l.estado_sri === "RECHAZADA" ? "↻ Reintentar SRI" : "📤 Emitir SRI"}
                        </button>
                      )}
                      {!l.anulada && l.estado_sri !== "AUTORIZADA" && (
                        <button className="btn btn-outline" style={{ fontSize: 10, padding: "2px 8px", color: "var(--color-danger)" }}
                          onClick={() => anular(l.id)}>Anular</button>
                      )}
                      {l.estado_sri === "AUTORIZADA" && (
                        <>
                          <button className="btn btn-outline" style={{ fontSize: 10, padding: "2px 8px" }}
                            onClick={() => descargarPdf(l.id, l.numero || l.numero_factura || String(l.id))}>PDF</button>
                          <button className="btn btn-outline" style={{ fontSize: 10, padding: "2px 8px" }}
                            onClick={() => enviarEmail(l.id)}>✉ Email</button>
                        </>
                      )}
                    </div>
                  </td>
                </tr>
              ))}
              {lista.length === 0 && !cargando && (
                <tr><td colSpan={6} className="text-center text-secondary" style={{ padding: 24 }}>No hay liquidaciones en los últimos 60 días</td></tr>
              )}
            </tbody>
          </table>
        </div>
      </div>

      {/* Modal CREAR */}
      {creando && (
        <div style={{ position: "fixed", inset: 0, background: "rgba(0,0,0,0.5)", display: "flex", alignItems: "center", justifyContent: "center", zIndex: 150 }}
          onClick={(e) => { if (e.target === e.currentTarget) setCreando(false); }}>
          <div className="card" style={{ width: 720, maxHeight: "90vh", overflow: "auto" }}>
            <div className="card-header flex justify-between items-center">
              <span>Nueva Liquidación de Compra</span>
              <button className="btn btn-outline" style={{ padding: "2px 8px" }} onClick={() => setCreando(false)}>x</button>
            </div>
            <div className="card-body">
              {/* Proveedor */}
              <label style={{ fontSize: 11, color: "var(--color-text-secondary)" }}>Proveedor *</label>
              {proveedor ? (
                <div className="flex justify-between items-center" style={{ padding: "6px 10px", background: "var(--color-surface-alt)", borderRadius: 6, marginBottom: 12 }}>
                  <span style={{ fontSize: 13 }}>{proveedor.nombre}{proveedor.ruc ? ` — ${proveedor.ruc}` : " (sin RUC)"}</span>
                  <button className="btn btn-outline" style={{ fontSize: 10, padding: "2px 8px" }} onClick={() => setProveedor(null)}>Cambiar</button>
                </div>
              ) : (
                <div style={{ marginBottom: 12, position: "relative" }}>
                  <input className="input" style={{ width: "100%", fontSize: 13 }} placeholder="Buscar proveedor por nombre o RUC..."
                    value={provBusq} onChange={(e) => setProvBusq(e.target.value)} />
                  {provRes.length > 0 && (
                    <div style={{ position: "absolute", zIndex: 5, background: "var(--color-surface)", border: "1px solid var(--color-border)", borderRadius: 6, width: "100%", maxHeight: 180, overflow: "auto" }}>
                      {provRes.map(p => (
                        <div key={p.id} style={{ padding: "6px 10px", cursor: "pointer", fontSize: 13, borderBottom: "1px solid var(--color-border)" }}
                          onClick={() => { setProveedor(p); setProvBusq(""); setProvRes([]); }}>
                          {p.nombre}{p.ruc ? ` — ${p.ruc}` : " (sin RUC)"}
                        </div>
                      ))}
                    </div>
                  )}
                </div>
              )}

              {/* Productos */}
              <label style={{ fontSize: 11, color: "var(--color-text-secondary)" }}>Productos / servicios comprados</label>
              <div style={{ position: "relative", marginBottom: 8, display: "flex", gap: 6 }}>
                <input className="input" style={{ flex: 1, fontSize: 13 }} placeholder="Buscar producto..."
                  value={busqProd} onChange={(e) => setBusqProd(e.target.value)} />
                <button className="btn btn-outline" style={{ fontSize: 11, whiteSpace: "nowrap" }} onClick={agregarLibre}>+ Línea libre</button>
                {prodRes.length > 0 && (
                  <div style={{ position: "absolute", top: 38, zIndex: 5, background: "var(--color-surface)", border: "1px solid var(--color-border)", borderRadius: 6, width: "60%", maxHeight: 200, overflow: "auto" }}>
                    {prodRes.map(p => (
                      <div key={p.id} style={{ padding: "6px 10px", cursor: "pointer", fontSize: 13, borderBottom: "1px solid var(--color-border)" }}
                        onClick={() => agregarProducto(p)}>
                        {p.nombre}{p.codigo ? ` (${p.codigo})` : ""}
                      </div>
                    ))}
                  </div>
                )}
              </div>

              {items.length > 0 && (
                <table className="table" style={{ fontSize: 12, marginBottom: 12 }}>
                  <thead>
                    <tr>
                      <th>Descripción</th>
                      <th className="text-right" style={{ width: 70 }}>Cant.</th>
                      <th className="text-right" style={{ width: 90 }}>P.Unit</th>
                      <th className="text-right" style={{ width: 70 }}>IVA%</th>
                      <th style={{ width: 30 }}></th>
                    </tr>
                  </thead>
                  <tbody>
                    {items.map((it, i) => (
                      <tr key={i}>
                        <td><input className="input" style={{ width: "100%", fontSize: 12 }} value={it.descripcion} onChange={(e) => setItem(i, "descripcion", e.target.value)} /></td>
                        <td><input type="number" className="input" style={{ width: 64, fontSize: 12, textAlign: "right" }} min={0} step="any" value={it.cantidad} onChange={(e) => setItem(i, "cantidad", parseFloat(e.target.value) || 0)} /></td>
                        <td><input type="number" className="input" style={{ width: 84, fontSize: 12, textAlign: "right" }} min={0} step="any" value={it.precio_unitario} onChange={(e) => setItem(i, "precio_unitario", parseFloat(e.target.value) || 0)} /></td>
                        <td>
                          <select className="input" style={{ width: 64, fontSize: 12 }} value={it.iva_porcentaje} onChange={(e) => setItem(i, "iva_porcentaje", parseFloat(e.target.value))}>
                            <option value={0}>0%</option>
                            <option value={15}>15%</option>
                          </select>
                        </td>
                        <td><button className="btn btn-outline" style={{ fontSize: 10, padding: "2px 6px", color: "var(--color-danger)" }} onClick={() => quitarItem(i)}>x</button></td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              )}

              <div className="flex justify-between items-center" style={{ marginBottom: 12 }}>
                <span style={{ fontSize: 11, color: "var(--color-text-secondary)" }}>
                  ℹ Al continuar se crea la liquidación y se emite al SRI (firma con tu certificado).
                </span>
                <span style={{ fontWeight: 700, fontSize: 16 }}>Total: ${totalCalc.toFixed(2)}</span>
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
