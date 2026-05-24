/**
 * v2.5.43 — Módulo Contabilidad (Agente de Retención + ATS)
 *
 * Página dedicada con tabs internos. Solo accesible si la licencia
 * incluye el módulo `contabilidad` (validado en Layout.tsx).
 *
 * Tabs:
 *   1. Configuración — datos del agente de retención (resolución, tipo, etc.)
 *   2. Comprobantes  — listado de retenciones emitidas (v2.5.44+)
 *   3. Generador ATS — XML mensual del Anexo Transaccional (v2.5.47)
 */
import { useState, useEffect } from "react";
import { useToast } from "../components/Toast";
import { invoke } from "@tauri-apps/api/core";

// ─── Tipos (mirror del backend contabilidad.rs) ─────────────────────────────

interface ContabilidadConfig {
  es_agente_retencion: boolean;
  resolucion_designacion: string | null;
  fecha_designacion: string | null;
  tipo_contribuyente: string | null;
  obligado_contabilidad: boolean;
  codigo_retencion_renta_default: string | null;
  codigo_retencion_iva_default: string | null;
  contador_ruc: string | null;
  contador_nombre: string | null;
  observacion: string | null;
}

interface RetencionEmitidaResumen {
  id: number;
  numero: string;
  fecha_emision: string;
  proveedor_nombre: string;
  proveedor_ruc: string | null;
  numero_documento_referencia: string | null;
  total: number;
  estado_sri: string;
  anulada: boolean;
}

// ─── Catálogo SRI (referencia rápida) ────────────────────────────────────────

const TIPOS_CONTRIBUYENTE = [
  { code: "SOCIEDAD", label: "Sociedad / Persona Jurídica" },
  { code: "PERSONA_NATURAL_OBLIGADA", label: "Persona Natural obligada a llevar contabilidad" },
  { code: "PERSONA_NATURAL_NO_OBLIGADA", label: "Persona Natural NO obligada" },
  { code: "ESPECIAL", label: "Contribuyente Especial" },
  { code: "RIMPE_EMPRENDEDOR", label: "RIMPE Emprendedor" },
  { code: "RIMPE_POPULAR", label: "RIMPE Popular (NO es agente, normalmente)" },
];

// Códigos de Retención RENTA más comunes (Tabla 304 SRI). Lista corta.
const CODIGOS_RENTA_COMUNES = [
  { code: "303", label: "303 — Honorarios profesionales (10%)" },
  { code: "304", label: "304 — Servicios predomina intelecto (8%)" },
  { code: "307", label: "307 — Servicios entre sociedades (2%)" },
  { code: "308", label: "308 — Servicios predomina mano de obra (2%)" },
  { code: "312", label: "312 — Transferencia de bienes muebles (1.75%)" },
  { code: "320", label: "320 — Arrendamiento bienes inmuebles (8%)" },
];

// Códigos de Retención IVA más comunes (Tabla 21 SRI)
const CODIGOS_IVA_COMUNES = [
  { code: "9",  label: "9  — Retención 30% IVA (bienes)" },
  { code: "10", label: "10 — Retención 70% IVA (servicios)" },
  { code: "11", label: "11 — Retención 100% IVA (honorarios, arriendos)" },
  { code: "725", label: "725 — Retención 10% (RIMPE)" },
  { code: "726", label: "726 — Retención 20% (RIMPE)" },
];

// ─── Componente principal ────────────────────────────────────────────────────

type Tab = "config" | "comprobantes" | "ats";

export default function ContabilidadPage() {
  const { toastExito, toastError } = useToast();
  const [tab, setTab] = useState<Tab>("config");
  const [config, setConfig] = useState<ContabilidadConfig | null>(null);
  const [guardando, setGuardando] = useState(false);

  // Comprobantes
  const [retenciones, setRetenciones] = useState<RetencionEmitidaResumen[]>([]);
  const [fechaDesde, setFechaDesde] = useState(() => {
    const d = new Date();
    return new Date(d.getFullYear(), d.getMonth(), 1).toISOString().slice(0, 10);
  });
  const [fechaHasta, setFechaHasta] = useState(() => new Date().toISOString().slice(0, 10));
  // v2.5.45: modal "Nueva retención"
  const [modalNuevaAbierto, setModalNuevaAbierto] = useState(false);

  const anularRetencion = async (id: number, numero: string) => {
    const motivo = prompt(`Motivo de anulación de retención ${numero}:`);
    if (motivo === null) return;
    try {
      await invoke<void>("contabilidad_anular_retencion", { id, motivo: motivo.trim() || null });
      toastExito("Retención anulada");
      invoke<RetencionEmitidaResumen[]>("contabilidad_listar_retenciones", {
        fechaDesde, fechaHasta,
      }).then(setRetenciones).catch(() => setRetenciones([]));
    } catch (err) {
      toastError("Error: " + err);
    }
  };

  useEffect(() => {
    invoke<ContabilidadConfig>("contabilidad_obtener_config")
      .then(setConfig)
      .catch((e) => toastError("Error cargando configuración: " + e));
  }, []);

  useEffect(() => {
    if (tab !== "comprobantes") return;
    invoke<RetencionEmitidaResumen[]>("contabilidad_listar_retenciones", {
      fechaDesde, fechaHasta,
    }).then(setRetenciones).catch(() => setRetenciones([]));
  }, [tab, fechaDesde, fechaHasta]);

  const guardarConfig = async () => {
    if (!config) return;
    try {
      setGuardando(true);
      await invoke<void>("contabilidad_guardar_config", { config });
      toastExito("Configuración guardada");
    } catch (err) {
      toastError("Error: " + err);
    } finally {
      setGuardando(false);
    }
  };

  return (
    <>
      <div className="page-header">
        <h2>Contabilidad · Agente de Retención + ATS</h2>
      </div>
      <div className="page-body">
        {/* Tabs */}
        <div className="flex gap-2 mb-4">
          {[
            { k: "config" as Tab, label: "⚙ Configuración" },
            { k: "comprobantes" as Tab, label: "📋 Comprobantes emitidos" },
            { k: "ats" as Tab, label: "📊 Generador ATS" },
          ].map(t => (
            <button key={t.k}
              className={`btn ${tab === t.k ? "btn-primary" : "btn-outline"}`}
              style={{ fontSize: 12, padding: "5px 12px" }}
              onClick={() => setTab(t.k)}>
              {t.label}
            </button>
          ))}
        </div>

        {/* TAB: Configuración */}
        {tab === "config" && config && (
          <div className="card" style={{ maxWidth: 800 }}>
            <div className="card-header">Datos del Agente de Retención</div>
            <div className="card-body">
              <div style={{ marginBottom: 16, padding: 12, background: "rgba(59,130,246,0.08)", border: "1px solid rgba(59,130,246,0.25)", borderRadius: 6, fontSize: 12 }}>
                <strong>ℹ Configuración independiente del SRI principal.</strong> Aquí solo
                los datos que te identifican como <strong>agente de retención</strong> ante el SRI.
                El RUC, certificado y configuración base SRI siguen en{" "}
                <strong>Configuración → SRI / Facturación electrónica</strong>.
              </div>

              <label style={{ display: "flex", alignItems: "center", gap: 8, marginBottom: 16, padding: 10, background: "var(--color-surface-alt)", borderRadius: 6, cursor: "pointer" }}>
                <input type="checkbox" checked={config.es_agente_retencion}
                  onChange={(e) => setConfig({ ...config, es_agente_retencion: e.target.checked })} />
                <div>
                  <div style={{ fontWeight: 600, fontSize: 14 }}>Soy agente de retención designado por el SRI</div>
                  <div style={{ fontSize: 11, color: "var(--color-text-secondary)" }}>
                    Si está marcado, podrás registrar retenciones a tus proveedores y generar ATS.
                  </div>
                </div>
              </label>

              {config.es_agente_retencion && (
                <>
                  <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: 14 }}>
                    <div>
                      <label className="text-secondary" style={{ fontSize: 12 }}>Resolución de designación</label>
                      <input className="input" placeholder="Ej: NAC-DGECCGC23-00000001"
                        value={config.resolucion_designacion ?? ""}
                        onChange={(e) => setConfig({ ...config, resolucion_designacion: e.target.value || null })} />
                    </div>
                    <div>
                      <label className="text-secondary" style={{ fontSize: 12 }}>Fecha de designación</label>
                      <input className="input" type="date"
                        value={config.fecha_designacion ?? ""}
                        onChange={(e) => setConfig({ ...config, fecha_designacion: e.target.value || null })} />
                    </div>
                    <div style={{ gridColumn: "1 / -1" }}>
                      <label className="text-secondary" style={{ fontSize: 12 }}>Tipo de contribuyente</label>
                      <select className="input" value={config.tipo_contribuyente ?? ""}
                        onChange={(e) => setConfig({ ...config, tipo_contribuyente: e.target.value || null })}>
                        <option value="">— Seleccionar —</option>
                        {TIPOS_CONTRIBUYENTE.map(t => (
                          <option key={t.code} value={t.code}>{t.label}</option>
                        ))}
                      </select>
                    </div>
                  </div>

                  <label style={{ display: "flex", alignItems: "center", gap: 8, marginTop: 14, fontSize: 13 }}>
                    <input type="checkbox" checked={config.obligado_contabilidad}
                      onChange={(e) => setConfig({ ...config, obligado_contabilidad: e.target.checked })} />
                    Obligado a llevar contabilidad
                  </label>

                  <div style={{ marginTop: 20, padding: 12, background: "var(--color-surface-alt)", borderRadius: 6 }}>
                    <div style={{ fontSize: 13, fontWeight: 600, marginBottom: 10 }}>Códigos SRI por defecto</div>
                    <div style={{ fontSize: 11, color: "var(--color-text-secondary)", marginBottom: 10 }}>
                      Estos códigos se sugieren al registrar una retención (puedes cambiarlos por cada caso).
                    </div>
                    <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: 14 }}>
                      <div>
                        <label className="text-secondary" style={{ fontSize: 11 }}>Código RENTA default (Tabla 304)</label>
                        <select className="input" value={config.codigo_retencion_renta_default ?? ""}
                          onChange={(e) => setConfig({ ...config, codigo_retencion_renta_default: e.target.value || null })}>
                          <option value="">— Ninguno —</option>
                          {CODIGOS_RENTA_COMUNES.map(c => (
                            <option key={c.code} value={c.code}>{c.label}</option>
                          ))}
                        </select>
                      </div>
                      <div>
                        <label className="text-secondary" style={{ fontSize: 11 }}>Código IVA default (Tabla 21)</label>
                        <select className="input" value={config.codigo_retencion_iva_default ?? ""}
                          onChange={(e) => setConfig({ ...config, codigo_retencion_iva_default: e.target.value || null })}>
                          <option value="">— Ninguno —</option>
                          {CODIGOS_IVA_COMUNES.map(c => (
                            <option key={c.code} value={c.code}>{c.label}</option>
                          ))}
                        </select>
                      </div>
                    </div>
                  </div>

                  <div style={{ marginTop: 20, padding: 12, background: "var(--color-surface-alt)", borderRadius: 6 }}>
                    <div style={{ fontSize: 13, fontWeight: 600, marginBottom: 10 }}>Datos del Contador (para ATS)</div>
                    <div style={{ display: "grid", gridTemplateColumns: "1fr 2fr", gap: 14 }}>
                      <div>
                        <label className="text-secondary" style={{ fontSize: 11 }}>RUC contador</label>
                        <input className="input" placeholder="13 dígitos"
                          value={config.contador_ruc ?? ""}
                          onChange={(e) => setConfig({ ...config, contador_ruc: e.target.value || null })} />
                      </div>
                      <div>
                        <label className="text-secondary" style={{ fontSize: 11 }}>Nombre completo del contador</label>
                        <input className="input"
                          value={config.contador_nombre ?? ""}
                          onChange={(e) => setConfig({ ...config, contador_nombre: e.target.value || null })} />
                      </div>
                    </div>
                  </div>

                  <div style={{ marginTop: 14 }}>
                    <label className="text-secondary" style={{ fontSize: 12 }}>Observaciones</label>
                    <textarea className="input" rows={2}
                      value={config.observacion ?? ""}
                      onChange={(e) => setConfig({ ...config, observacion: e.target.value || null })} />
                  </div>
                </>
              )}

              <div className="flex gap-2" style={{ justifyContent: "flex-end", marginTop: 20 }}>
                <button className="btn btn-primary" onClick={guardarConfig} disabled={guardando}>
                  {guardando ? "Guardando..." : "Guardar configuración"}
                </button>
              </div>
            </div>
          </div>
        )}

        {/* TAB: Comprobantes — v2.5.45 funcional */}
        {tab === "comprobantes" && (
          <div>
            <div className="card mb-4">
              <div className="card-body" style={{ padding: 12 }}>
                <div className="flex gap-3 items-center" style={{ flexWrap: "wrap" }}>
                  <div>
                    <label className="text-secondary" style={{ fontSize: 11 }}>Desde</label>
                    <input type="date" className="input" value={fechaDesde}
                      onChange={(e) => setFechaDesde(e.target.value)} />
                  </div>
                  <div>
                    <label className="text-secondary" style={{ fontSize: 11 }}>Hasta</label>
                    <input type="date" className="input" value={fechaHasta}
                      onChange={(e) => setFechaHasta(e.target.value)} />
                  </div>
                  <button className="btn btn-primary"
                    style={{ marginLeft: "auto", marginTop: 16 }}
                    onClick={() => setModalNuevaAbierto(true)}>
                    + Nueva retención
                  </button>
                </div>
              </div>
            </div>

            <div className="card">
              <table className="table">
                <thead>
                  <tr>
                    <th>Número</th>
                    <th>Fecha</th>
                    <th>Proveedor</th>
                    <th>RUC</th>
                    <th>Doc. referencia</th>
                    <th className="text-right">Total</th>
                    <th>Estado SRI</th>
                    <th></th>
                  </tr>
                </thead>
                <tbody>
                  {retenciones.length === 0 ? (
                    <tr><td colSpan={8} className="text-center text-secondary" style={{ padding: 40 }}>
                      No hay retenciones emitidas en este período.
                      <div style={{ fontSize: 11, marginTop: 8 }}>
                        Click <strong>+ Nueva retención</strong> para crear el primer comprobante.
                      </div>
                    </td></tr>
                  ) : retenciones.map(r => (
                    <tr key={r.id} style={{ opacity: r.anulada ? 0.5 : 1 }}>
                      <td><strong>{r.numero}</strong>{r.anulada && <span style={{ marginLeft: 6, fontSize: 9, padding: "1px 5px", borderRadius: 3, background: "rgba(239,68,68,0.15)", color: "var(--color-danger)", fontWeight: 600 }}>ANULADA</span>}</td>
                      <td>{r.fecha_emision.slice(0, 10)}</td>
                      <td>{r.proveedor_nombre}</td>
                      <td className="text-secondary">{r.proveedor_ruc ?? "—"}</td>
                      <td className="text-secondary">{r.numero_documento_referencia ?? "—"}</td>
                      <td className="text-right font-bold">${r.total.toFixed(2)}</td>
                      <td>
                        <span style={{
                          fontSize: 10, padding: "2px 6px", borderRadius: 3, fontWeight: 600,
                          background: r.estado_sri === "AUTORIZADA" ? "rgba(34,197,94,0.15)"
                            : r.estado_sri === "RECHAZADA" ? "rgba(239,68,68,0.15)"
                            : "var(--color-surface-alt)",
                          color: r.estado_sri === "AUTORIZADA" ? "var(--color-success)"
                            : r.estado_sri === "RECHAZADA" ? "var(--color-danger)"
                            : "var(--color-text-secondary)",
                        }}>{r.estado_sri}</span>
                      </td>
                      <td style={{ whiteSpace: "nowrap" }}>
                        {!r.anulada && r.estado_sri !== "AUTORIZADA" && (
                          <button className="btn btn-outline" style={{ fontSize: 10, padding: "2px 8px", color: "var(--color-danger)", borderColor: "var(--color-danger)" }}
                            onClick={() => anularRetencion(r.id, r.numero)}>
                            Anular
                          </button>
                        )}
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>

            {/* Modal Nueva Retención */}
            {modalNuevaAbierto && (
              <ModalNuevaRetencion
                config={config}
                onClose={() => setModalNuevaAbierto(false)}
                onCreada={() => {
                  setModalNuevaAbierto(false);
                  // Recargar lista
                  invoke<RetencionEmitidaResumen[]>("contabilidad_listar_retenciones", {
                    fechaDesde, fechaHasta,
                  }).then(setRetenciones).catch(() => setRetenciones([]));
                  toastExito("Retención creada");
                }}
              />
            )}
          </div>
        )}

        {/* TAB: ATS (placeholder) */}
        {tab === "ats" && (
          <div className="card" style={{ maxWidth: 600, textAlign: "center" }}>
            <div className="card-body" style={{ padding: 40 }}>
              <div style={{ fontSize: 48, marginBottom: 12 }}>📊</div>
              <h3 style={{ marginBottom: 8 }}>Generador ATS Mensual</h3>
              <p style={{ color: "var(--color-text-secondary)", marginBottom: 16 }}>
                Genera el Anexo Transaccional Simplificado en formato XML SRI listo para subir al portal.
              </p>
              <div style={{ padding: 12, background: "rgba(245,158,11,0.1)", borderRadius: 6, fontSize: 13, color: "var(--color-warning)" }}>
                Disponible en <strong>v2.5.47</strong>. Primero implementamos captura de retenciones (v2.5.44) y el XML/RIDE del comprobante de retención (v2.5.45-46).
              </div>
            </div>
          </div>
        )}
      </div>
    </>
  );
}

// ─── ModalNuevaRetencion (v2.5.45) ───────────────────────────────────────────

interface CompraOpcion {
  id: number;
  numero: string;
  numero_factura: string | null;
  fecha: string;
  proveedor_nombre: string;
  subtotal: number;
  iva: number;
  total: number;
}

interface LineaForm {
  tipo: "RENTA" | "IVA";
  codigo_sri: string;
  base_imponible: number;
  porcentaje: number;
  valor: number;
}

function ModalNuevaRetencion({ config, onClose, onCreada }: {
  config: ContabilidadConfig | null;
  onClose: () => void;
  onCreada: () => void;
}) {
  const { toastError } = useToast();
  const [compras, setCompras] = useState<CompraOpcion[]>([]);
  const [compraId, setCompraId] = useState<number | "">("");
  const [compraSel, setCompraSel] = useState<CompraOpcion | null>(null);
  const [busquedaCompra, setBusquedaCompra] = useState("");
  const [lineas, setLineas] = useState<LineaForm[]>([]);
  const [observacion, setObservacion] = useState("");
  const [estab, setEstab] = useState("");
  const [pto, setPto] = useState("");
  const [sec, setSec] = useState("");
  const [guardando, setGuardando] = useState(false);

  // Cargar compras del último año (no anuladas, no devueltas)
  useEffect(() => {
    const hoy = new Date().toISOString().slice(0, 10);
    const haceUnAno = new Date();
    haceUnAno.setFullYear(haceUnAno.getFullYear() - 1);
    invoke<any[]>("listar_compras", {
      fechaDesde: haceUnAno.toISOString().slice(0, 10),
      fechaHasta: hoy,
    }).then((rows) => {
      setCompras(rows.filter(c => c.estado !== "ANULADA").map(c => ({
        id: c.id,
        numero: c.numero,
        numero_factura: c.numero_factura,
        fecha: c.fecha,
        proveedor_nombre: c.proveedor_nombre,
        subtotal: c.subtotal,
        iva: c.iva,
        total: c.total,
      })));
    }).catch(() => setCompras([]));
  }, []);

  // Cuando se selecciona compra, sugerir líneas automáticamente con códigos default
  useEffect(() => {
    if (!compraId) { setCompraSel(null); return; }
    const c = compras.find(x => x.id === compraId);
    if (!c) return;
    setCompraSel(c);
    const sugeridas: LineaForm[] = [];
    // RENTA: sobre el subtotal (base imponible 0% se descuenta del subtotal con IVA, simplificamos al subtotal sin IVA)
    if (config?.codigo_retencion_renta_default && c.subtotal > 0) {
      const pct = inferPorcentajeRenta(config.codigo_retencion_renta_default);
      sugeridas.push({
        tipo: "RENTA",
        codigo_sri: config.codigo_retencion_renta_default,
        base_imponible: c.subtotal,
        porcentaje: pct,
        valor: +(c.subtotal * pct / 100).toFixed(2),
      });
    }
    // IVA: sobre el monto del IVA
    if (config?.codigo_retencion_iva_default && c.iva > 0) {
      const pct = inferPorcentajeIva(config.codigo_retencion_iva_default);
      sugeridas.push({
        tipo: "IVA",
        codigo_sri: config.codigo_retencion_iva_default,
        base_imponible: c.iva,
        porcentaje: pct,
        valor: +(c.iva * pct / 100).toFixed(2),
      });
    }
    setLineas(sugeridas);
  }, [compraId, compras, config]);

  const comprasFiltradas = compras.filter(c => {
    if (!busquedaCompra.trim()) return true;
    const q = busquedaCompra.toLowerCase();
    return c.numero.toLowerCase().includes(q) ||
           (c.numero_factura ?? "").toLowerCase().includes(q) ||
           c.proveedor_nombre.toLowerCase().includes(q);
  });

  const agregarLinea = (tipo: "RENTA" | "IVA") => {
    setLineas([...lineas, { tipo, codigo_sri: "", base_imponible: 0, porcentaje: 0, valor: 0 }]);
  };

  const actualizarLinea = (idx: number, patch: Partial<LineaForm>) => {
    setLineas(lineas.map((l, i) => {
      if (i !== idx) return l;
      const upd = { ...l, ...patch };
      // Auto-calcular valor si cambia base o porcentaje
      if (patch.base_imponible !== undefined || patch.porcentaje !== undefined) {
        upd.valor = +(upd.base_imponible * upd.porcentaje / 100).toFixed(2);
      }
      return upd;
    }));
  };

  const eliminarLinea = (idx: number) => {
    setLineas(lineas.filter((_, i) => i !== idx));
  };

  const total = lineas.reduce((s, l) => s + l.valor, 0);

  const guardar = async () => {
    if (!compraId) { toastError("Selecciona una compra"); return; }
    if (lineas.length === 0) { toastError("Agrega al menos una línea de retención"); return; }
    for (const l of lineas) {
      if (!l.codigo_sri.trim()) { toastError("Cada línea requiere código SRI"); return; }
      if (l.valor <= 0) { toastError("Cada línea debe tener valor > 0"); return; }
    }
    try {
      setGuardando(true);
      await invoke("contabilidad_crear_retencion", {
        input: {
          compra_id: compraId,
          numero_documento_referencia: compraSel?.numero_factura ?? null,
          fecha_documento_referencia: compraSel?.fecha ?? null,
          items: lineas,
          observacion: observacion.trim() || null,
          establecimiento: estab.trim() || null,
          punto_emision: pto.trim() || null,
          secuencial: sec.trim() || null,
        },
      });
      onCreada();
    } catch (err) {
      toastError("Error: " + err);
    } finally {
      setGuardando(false);
    }
  };

  return (
    <div className="modal-overlay" onClick={onClose}>
      <div className="modal-content" onClick={(e) => e.stopPropagation()} style={{ maxWidth: 800, maxHeight: "90vh", overflow: "auto" }}>
        <div className="modal-header">
          <h3>Nueva retención emitida</h3>
        </div>
        <div className="modal-body">
          {/* Selector compra */}
          <div style={{ marginBottom: 16 }}>
            <label className="text-secondary" style={{ fontSize: 12, display: "block", marginBottom: 4 }}>
              Compra sobre la cual emites la retención *
            </label>
            <input className="input" placeholder="🔍 Buscar por número compra, factura o proveedor..."
              value={busquedaCompra} onChange={(e) => setBusquedaCompra(e.target.value)}
              style={{ marginBottom: 6 }} />
            <select className="input" value={compraId} onChange={(e) => setCompraId(Number(e.target.value) || "")}>
              <option value="">— Selecciona compra —</option>
              {comprasFiltradas.slice(0, 100).map(c => (
                <option key={c.id} value={c.id}>
                  {c.numero}{c.numero_factura ? ` · F:${c.numero_factura}` : ""} · {c.proveedor_nombre} · ${c.total.toFixed(2)}
                </option>
              ))}
            </select>
            {compraSel && (
              <div style={{ marginTop: 6, padding: 8, background: "var(--color-surface-alt)", borderRadius: 4, fontSize: 11 }}>
                <strong>{compraSel.proveedor_nombre}</strong> · Subtotal: ${compraSel.subtotal.toFixed(2)} · IVA: ${compraSel.iva.toFixed(2)} · Total: ${compraSel.total.toFixed(2)}
              </div>
            )}
          </div>

          {/* Número SRI (opcional ahora, requerido si se va a enviar al SRI) */}
          <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr 1fr", gap: 10, marginBottom: 16 }}>
            <div>
              <label className="text-secondary" style={{ fontSize: 11 }}>Establecimiento</label>
              <input className="input" placeholder="001" maxLength={3}
                value={estab} onChange={(e) => setEstab(e.target.value.replace(/\D/g, ""))} />
            </div>
            <div>
              <label className="text-secondary" style={{ fontSize: 11 }}>Pto. emisión</label>
              <input className="input" placeholder="001" maxLength={3}
                value={pto} onChange={(e) => setPto(e.target.value.replace(/\D/g, ""))} />
            </div>
            <div>
              <label className="text-secondary" style={{ fontSize: 11 }}>Secuencial</label>
              <input className="input" placeholder="000000001" maxLength={9}
                value={sec} onChange={(e) => setSec(e.target.value.replace(/\D/g, ""))} />
            </div>
          </div>

          {/* Líneas de retención */}
          <div style={{ marginBottom: 12 }}>
            <div className="flex justify-between items-center" style={{ marginBottom: 8 }}>
              <div style={{ fontWeight: 600, fontSize: 13 }}>Líneas de retención</div>
              <div className="flex gap-2">
                <button type="button" className="btn btn-outline" style={{ fontSize: 11, padding: "3px 8px" }}
                  onClick={() => agregarLinea("RENTA")}>+ RENTA</button>
                <button type="button" className="btn btn-outline" style={{ fontSize: 11, padding: "3px 8px" }}
                  onClick={() => agregarLinea("IVA")}>+ IVA</button>
              </div>
            </div>
            {lineas.length === 0 && (
              <div className="text-center text-secondary" style={{ padding: 20, fontSize: 12 }}>
                Sin líneas. Selecciona una compra (las líneas se sugieren automáticamente con los códigos default) o agrégalas manualmente.
              </div>
            )}
            {lineas.map((l, idx) => (
              <div key={idx} style={{ display: "grid", gridTemplateColumns: "60px 100px 1fr 80px 80px 80px 30px", gap: 6, marginBottom: 6, alignItems: "center" }}>
                <select className="input" style={{ fontSize: 11, padding: "2px 4px" }}
                  value={l.tipo} onChange={(e) => actualizarLinea(idx, { tipo: e.target.value as "RENTA" | "IVA" })}>
                  <option value="RENTA">RENTA</option>
                  <option value="IVA">IVA</option>
                </select>
                <input className="input" placeholder="Cód SRI" style={{ fontSize: 11, padding: "2px 4px" }}
                  value={l.codigo_sri} onChange={(e) => actualizarLinea(idx, { codigo_sri: e.target.value })} />
                <select className="input" style={{ fontSize: 11, padding: "2px 4px" }}
                  value={l.codigo_sri}
                  onChange={(e) => {
                    const pct = l.tipo === "RENTA" ? inferPorcentajeRenta(e.target.value) : inferPorcentajeIva(e.target.value);
                    actualizarLinea(idx, { codigo_sri: e.target.value, porcentaje: pct,
                      valor: +(l.base_imponible * pct / 100).toFixed(2) });
                  }}>
                  <option value="">— Buscar código —</option>
                  {(l.tipo === "RENTA" ? CODIGOS_RENTA_COMUNES : CODIGOS_IVA_COMUNES).map(c => (
                    <option key={c.code} value={c.code}>{c.label}</option>
                  ))}
                </select>
                <input className="input" type="number" min="0" step="0.01"
                  title="Base imponible"
                  style={{ fontSize: 11, padding: "2px 4px", textAlign: "right" }}
                  value={l.base_imponible}
                  onChange={(e) => actualizarLinea(idx, { base_imponible: parseFloat(e.target.value) || 0 })} />
                <input className="input" type="number" min="0" max="100" step="0.01"
                  title="Porcentaje %"
                  style={{ fontSize: 11, padding: "2px 4px", textAlign: "right" }}
                  value={l.porcentaje}
                  onChange={(e) => actualizarLinea(idx, { porcentaje: parseFloat(e.target.value) || 0 })} />
                <input className="input" type="number" min="0" step="0.01"
                  title="Valor a retener"
                  style={{ fontSize: 11, padding: "2px 4px", textAlign: "right", fontWeight: 600 }}
                  value={l.valor}
                  onChange={(e) => actualizarLinea(idx, { valor: parseFloat(e.target.value) || 0 })} />
                <button type="button" className="btn btn-outline" style={{ fontSize: 10, padding: "2px 4px", color: "var(--color-danger)" }}
                  onClick={() => eliminarLinea(idx)}>×</button>
              </div>
            ))}
          </div>

          <div style={{ padding: 10, background: "var(--color-surface-alt)", borderRadius: 6, marginBottom: 12, textAlign: "right" }}>
            <strong>Total a retener: ${total.toFixed(2)}</strong>
            <div style={{ fontSize: 10, color: "var(--color-text-secondary)", marginTop: 2 }}>
              Este monto se descontará automáticamente del saldo de cuenta por pagar al proveedor.
            </div>
          </div>

          <div>
            <label className="text-secondary" style={{ fontSize: 11 }}>Observación</label>
            <textarea className="input" rows={2}
              value={observacion} onChange={(e) => setObservacion(e.target.value)} />
          </div>
        </div>
        <div className="modal-footer" style={{ borderTop: "1px solid var(--color-border)", padding: 12 }}>
          <button className="btn btn-outline" onClick={onClose} disabled={guardando}>Cancelar</button>
          <button className="btn btn-primary" onClick={guardar} disabled={guardando || !compraId || lineas.length === 0}>
            {guardando ? "Guardando..." : "Crear retención"}
          </button>
        </div>
      </div>
    </div>
  );
}

// Helper para inferir porcentaje desde código SRI conocido
function inferPorcentajeRenta(codigo: string): number {
  const map: Record<string, number> = {
    "303": 10, "304": 8, "307": 2, "308": 2, "312": 1.75, "320": 8,
  };
  return map[codigo] ?? 0;
}
function inferPorcentajeIva(codigo: string): number {
  const map: Record<string, number> = {
    "9": 30, "10": 70, "11": 100, "725": 10, "726": 20,
  };
  return map[codigo] ?? 0;
}
