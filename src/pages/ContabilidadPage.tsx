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

        {/* TAB: Comprobantes (placeholder funcional) */}
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
                  <button className="btn btn-primary" disabled
                    title="Disponible en v2.5.44 (próxima release)"
                    style={{ marginLeft: "auto", marginTop: 16, opacity: 0.5 }}>
                    + Nueva retención (próximamente)
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
                  </tr>
                </thead>
                <tbody>
                  {retenciones.length === 0 ? (
                    <tr><td colSpan={7} className="text-center text-secondary" style={{ padding: 40 }}>
                      No hay retenciones emitidas en este período.
                      <div style={{ fontSize: 11, marginTop: 8 }}>
                        La captura de retenciones se habilita en <strong>v2.5.44</strong> (próxima release).
                      </div>
                    </td></tr>
                  ) : retenciones.map(r => (
                    <tr key={r.id} style={{ opacity: r.anulada ? 0.5 : 1 }}>
                      <td><strong>{r.numero}</strong></td>
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
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
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
