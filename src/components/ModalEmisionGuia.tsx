import { useState, useEffect } from "react";
import { guiaGuardarDatosSri, guiaObtenerDatosSri, emitirGuiaRemisionSri, sugerirPorPlaca, obtenerConfig } from "../services/api";
import type { GuiaDatosSri } from "../services/api";
import { useToast } from "./Toast";

function fechaHoy(): string {
  const now = new Date();
  const y = now.getFullYear();
  const m = String(now.getMonth() + 1).padStart(2, "0");
  const d = String(now.getDate()).padStart(2, "0");
  return `${y}-${m}-${d}`;
}

const EMIT_FORM_VACIO = {
  transportista: "", ruc_transportista: "", tipo_id_transportista: "",
  dir_partida: "", fecha_inicio_transporte: "", fecha_fin_transporte: "",
  motivo_traslado: "", ruta: "", cod_doc_sustento: "01", num_doc_sustento: "",
  num_aut_sustento: "", fecha_emision_sustento: "", placa: "", direccion_destino: "",
};

interface Props {
  guiaId: number;
  numero: string;
  /** Título mostrado en el encabezado del modal. */
  titulo?: string;
  onClose: () => void;
  /** Se llama cuando la guía fue autorizada por el SRI. */
  onEmitida: () => void;
}

/**
 * Modal reutilizable para emitir una Guía de Remisión electrónica (SRI codDoc 06)
 * a partir de una guía ya creada (existe la fila con tipo_estado='GUIA_REMISION').
 * Captura/edita los datos de transporte, los guarda y emite al SRI en un paso.
 */
export default function ModalEmisionGuia({ guiaId, numero, titulo, onClose, onEmitida }: Props) {
  const { toastExito, toastError } = useToast();
  const [form, setForm] = useState({ ...EMIT_FORM_VACIO });
  const [estado, setEstado] = useState<{ estado_sri: string; numero_sri: string }>({ estado_sri: "", numero_sri: "" });
  const [emitiendo, setEmitiendo] = useState(false);
  // Datos del negocio (para "Soy yo" como transportista y prellenar dir. de partida)
  const [negocio, setNegocio] = useState<{ nombre: string; ruc: string; direccion: string }>({ nombre: "", ruc: "", direccion: "" });

  useEffect(() => {
    obtenerConfig().then((cfg) => {
      setNegocio({
        nombre: cfg.nombre_negocio || "",
        ruc: cfg.ruc || "",
        direccion: cfg.direccion || "",
      });
    }).catch(() => {});
  }, []);

  const usarMiNegocioComoTransportista = () => {
    setForm((f) => ({
      ...f,
      transportista: negocio.nombre,
      ruc_transportista: negocio.ruc,
      tipo_id_transportista: negocio.ruc.length === 13 ? "04" : "05",
    }));
  };

  useEffect(() => {
    let cancelado = false;
    setForm({ ...EMIT_FORM_VACIO });
    setEstado({ estado_sri: "", numero_sri: "" });
    guiaObtenerDatosSri(guiaId).then((d) => {
      if (cancelado) return;
      setForm({
        transportista: d.transportista || "",
        ruc_transportista: d.ruc_transportista || "",
        tipo_id_transportista: d.tipo_id_transportista || "",
        // Si no hay dirección de partida guardada, se prellenará con la del
        // negocio en cuanto cargue la config (ver efecto de abajo).
        dir_partida: d.dir_partida || "",
        fecha_inicio_transporte: d.fecha_inicio_transporte || fechaHoy(),
        fecha_fin_transporte: d.fecha_fin_transporte || fechaHoy(),
        motivo_traslado: d.motivo_traslado || "",
        ruta: d.ruta || "",
        cod_doc_sustento: d.cod_doc_sustento || "01",
        num_doc_sustento: d.num_doc_sustento || "",
        num_aut_sustento: d.num_aut_sustento || "",
        fecha_emision_sustento: d.fecha_emision_sustento || "",
        placa: d.placa || "",
        direccion_destino: d.direccion_destino || "",
      });
      setEstado({ estado_sri: d.estado_sri || "", numero_sri: d.numero_sri || "" });
    }).catch((e) => { if (!cancelado) toastError("Error cargando datos: " + e); });
    return () => { cancelado = true; };
  }, [guiaId]);

  // Prellenar dirección de PARTIDA con la del negocio si quedó vacía (origen = mi bodega).
  useEffect(() => {
    if (negocio.direccion) {
      setForm((f) => f.dir_partida.trim() ? f : { ...f, dir_partida: negocio.direccion });
    }
  }, [negocio.direccion]);

  const guardarYEmitir = async () => {
    if (!form.transportista.trim() || !form.ruc_transportista.trim()) {
      toastError("Ingrese el transportista (razon social e identificacion)");
      return;
    }
    if (!form.dir_partida.trim()) { toastError("Ingrese la direccion de partida"); return; }
    if (!form.motivo_traslado.trim()) { toastError("Ingrese el motivo del traslado"); return; }
    setEmitiendo(true);
    try {
      const datos: GuiaDatosSri = { ...form };
      await guiaGuardarDatosSri(guiaId, datos);
      const res = await emitirGuiaRemisionSri(guiaId);
      if (res.exito) {
        toastExito(`Guia de Remision autorizada por el SRI (${res.numero_factura || res.clave_acceso || ""})`);
        onEmitida();
      } else {
        toastError(`SRI: ${res.mensaje || res.estado_sri}`);
        setEstado((s) => ({ ...s, estado_sri: res.estado_sri }));
      }
    } catch (e) {
      toastError("Error al emitir: " + e);
    } finally {
      setEmitiendo(false);
    }
  };

  return (
    <div style={{ position: "fixed", inset: 0, background: "rgba(0,0,0,0.5)", display: "flex", alignItems: "center", justifyContent: "center", zIndex: 200 }}
      onClick={(e) => { if (e.target === e.currentTarget) onClose(); }}>
      <div className="card" style={{ width: 620, maxHeight: "90vh", overflow: "auto" }}>
        <div className="card-header flex justify-between items-center">
          <span>{titulo || `Guía de Remisión SRI (${numero})`}</span>
          <button className="btn btn-outline" style={{ padding: "2px 8px" }} onClick={onClose}>x</button>
        </div>
        <div className="card-body">
          {estado.estado_sri === "AUTORIZADA" ? (
            <div style={{ background: "rgba(74,222,128,0.12)", border: "1px solid rgba(74,222,128,0.4)", borderRadius: 8, padding: 14, marginBottom: 12 }}>
              <div style={{ fontWeight: 700, color: "var(--color-success)", marginBottom: 4 }}>✓ Guía de Remisión ya autorizada por el SRI</div>
              <div style={{ fontSize: 12 }} className="text-secondary">Nro SRI: {estado.numero_sri || "-"}</div>
            </div>
          ) : estado.estado_sri && estado.estado_sri !== "NO_APLICA" && (
            <div style={{ background: "rgba(251,191,36,0.12)", border: "1px solid rgba(251,191,36,0.4)", borderRadius: 8, padding: 10, marginBottom: 12, fontSize: 12 }}>
              Estado SRI actual: <strong>{estado.estado_sri}</strong> — puede reintentar la emision.
            </div>
          )}

          <div className="flex items-center justify-between" style={{ marginBottom: 8 }}>
            <span style={{ fontSize: 12, fontWeight: 600, color: "var(--color-text-secondary)" }}>🚚 Transportista (quién lleva la carga)</span>
            {negocio.nombre && (
              <button type="button" className="btn btn-outline" style={{ fontSize: 10, padding: "2px 8px" }}
                title="Usar mi negocio como transportista (yo mismo transporto)"
                onClick={usarMiNegocioComoTransportista}>
                Soy yo
              </button>
            )}
          </div>
          <div style={{ display: "grid", gridTemplateColumns: "2fr 1fr", gap: 8, marginBottom: 10 }}>
            <div>
              <label style={{ fontSize: 11, color: "var(--color-text-secondary)" }}>Razon social *</label>
              <input className="input" style={{ width: "100%", fontSize: 13 }} value={form.transportista}
                onChange={(e) => setForm((f) => ({ ...f, transportista: e.target.value }))} />
            </div>
            <div>
              <label style={{ fontSize: 11, color: "var(--color-text-secondary)" }}>RUC / Cedula *</label>
              <input className="input" style={{ width: "100%", fontSize: 13 }} value={form.ruc_transportista}
                onChange={(e) => setForm((f) => ({ ...f, ruc_transportista: e.target.value }))} />
            </div>
          </div>

          <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: 8, marginBottom: 10 }}>
            <div>
              <label style={{ fontSize: 11, color: "var(--color-text-secondary)" }}>Placa</label>
              <input className="input" style={{ width: "100%", fontSize: 13 }} value={form.placa}
                onChange={async (e) => {
                  const val = e.target.value.toUpperCase();
                  setForm((f) => ({ ...f, placa: val }));
                  if (val.trim().length >= 2) {
                    try {
                      const sugs = await sugerirPorPlaca(val.trim());
                      const exact = sugs.find((s) => s.placa === val.trim() && s.transportista_nombre);
                      const withT = exact || sugs.find((s) => s.transportista_nombre);
                      if (withT) {
                        setForm((f) => ({
                          ...f,
                          transportista: f.transportista.trim() ? f.transportista : (withT.transportista_nombre || ""),
                          ruc_transportista: f.ruc_transportista.trim() ? f.ruc_transportista : (withT.transportista_ruc || ""),
                        }));
                      }
                    } catch { /* ignore */ }
                  }
                }} />
            </div>
            <div>
              <label style={{ fontSize: 11, color: "var(--color-text-secondary)" }}>Motivo del traslado *</label>
              <input className="input" style={{ width: "100%", fontSize: 13 }} placeholder="Venta, traslado..." value={form.motivo_traslado}
                onChange={(e) => setForm((f) => ({ ...f, motivo_traslado: e.target.value }))} />
            </div>
          </div>

          <div style={{ fontSize: 12, fontWeight: 600, margin: "4px 0 8px", color: "var(--color-text-secondary)" }}>📍 Ruta del traslado (de dónde sale → a dónde llega)</div>
          <div style={{ marginBottom: 10 }}>
            <label style={{ fontSize: 11, color: "var(--color-text-secondary)" }}>Origen — Dirección de partida * <span style={{ color: "var(--color-text-secondary)", fontWeight: 400 }}>(de dónde sale la mercadería, normalmente tu bodega)</span></label>
            <input className="input" style={{ width: "100%", fontSize: 13 }} placeholder="Dirección de tu negocio/bodega" value={form.dir_partida}
              onChange={(e) => setForm((f) => ({ ...f, dir_partida: e.target.value }))} />
          </div>
          <div style={{ marginBottom: 10 }}>
            <label style={{ fontSize: 11, color: "var(--color-text-secondary)" }}>Destino — Dirección de entrega <span style={{ color: "var(--color-text-secondary)", fontWeight: 400 }}>(a dónde llega: el destinatario/cliente)</span></label>
            <input className="input" style={{ width: "100%", fontSize: 13 }} placeholder="Dirección donde se entrega la carga" value={form.direccion_destino}
              onChange={(e) => setForm((f) => ({ ...f, direccion_destino: e.target.value }))} />
          </div>

          <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr 1fr", gap: 8, marginBottom: 10 }}>
            <div>
              <label style={{ fontSize: 11, color: "var(--color-text-secondary)" }}>Inicio transporte</label>
              <input type="date" className="input" style={{ width: "100%", fontSize: 13 }} value={form.fecha_inicio_transporte}
                onChange={(e) => setForm((f) => ({ ...f, fecha_inicio_transporte: e.target.value }))} />
            </div>
            <div>
              <label style={{ fontSize: 11, color: "var(--color-text-secondary)" }}>Fin transporte</label>
              <input type="date" className="input" style={{ width: "100%", fontSize: 13 }} value={form.fecha_fin_transporte}
                onChange={(e) => setForm((f) => ({ ...f, fecha_fin_transporte: e.target.value }))} />
            </div>
            <div>
              <label style={{ fontSize: 11, color: "var(--color-text-secondary)" }}>Ruta</label>
              <input className="input" style={{ width: "100%", fontSize: 13 }} placeholder="Quito - Guayaquil" value={form.ruta}
                onChange={(e) => setForm((f) => ({ ...f, ruta: e.target.value }))} />
            </div>
          </div>

          <details style={{ marginBottom: 12 }}>
            <summary style={{ fontSize: 12, fontWeight: 600, color: "var(--color-text-secondary)", cursor: "pointer" }}>Documento de sustento (opcional)</summary>
            <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: 8, marginTop: 8 }}>
              <div>
                <label style={{ fontSize: 11, color: "var(--color-text-secondary)" }}>Tipo</label>
                <select className="input" style={{ width: "100%", fontSize: 13 }} value={form.cod_doc_sustento}
                  onChange={(e) => setForm((f) => ({ ...f, cod_doc_sustento: e.target.value }))}>
                  <option value="01">Factura (01)</option>
                  <option value="03">Liquidacion compra (03)</option>
                  <option value="04">Nota de credito (04)</option>
                </select>
              </div>
              <div>
                <label style={{ fontSize: 11, color: "var(--color-text-secondary)" }}>Numero (001-001-000000001)</label>
                <input className="input" style={{ width: "100%", fontSize: 13 }} value={form.num_doc_sustento}
                  onChange={(e) => setForm((f) => ({ ...f, num_doc_sustento: e.target.value }))} />
              </div>
              <div>
                <label style={{ fontSize: 11, color: "var(--color-text-secondary)" }}>Autorizacion (clave 49)</label>
                <input className="input" style={{ width: "100%", fontSize: 13 }} value={form.num_aut_sustento}
                  onChange={(e) => setForm((f) => ({ ...f, num_aut_sustento: e.target.value }))} />
              </div>
              <div>
                <label style={{ fontSize: 11, color: "var(--color-text-secondary)" }}>Fecha emision sustento</label>
                <input type="date" className="input" style={{ width: "100%", fontSize: 13 }} value={form.fecha_emision_sustento}
                  onChange={(e) => setForm((f) => ({ ...f, fecha_emision_sustento: e.target.value }))} />
              </div>
            </div>
          </details>

          <button className="btn" style={{
            width: "100%", padding: "10px 0", fontWeight: 700, fontSize: 14,
            background: "var(--color-primary)", color: "white", border: "none",
          }}
            disabled={emitiendo}
            onClick={guardarYEmitir}>
            {emitiendo ? "Enviando al SRI..." : estado.estado_sri === "AUTORIZADA" ? "Reemitir al SRI" : "Emitir Guía al SRI"}
          </button>
          <div style={{ fontSize: 11, color: "var(--color-text-secondary)", marginTop: 8, textAlign: "center" }}>
            Se firma con tu certificado digital y se envia al SRI (ambiente segun configuracion).
          </div>
        </div>
      </div>
    </div>
  );
}
