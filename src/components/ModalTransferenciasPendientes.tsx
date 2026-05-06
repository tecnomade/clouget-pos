/**
 * ModalTransferenciasPendientes — diagnóstico + acción del badge
 * "X transferencias por verificar" del Dashboard.
 *
 * v2.3.64: cuando el usuario reporta "ya verifiqué todas pero sigue contando",
 * este modal muestra exactamente QUÉ transferencias está contando el backend
 * (id, número de venta, fecha, monto, cliente, origen).
 *
 * Permite "Forzar verificar" como último recurso si el cleanup automático
 * no las pesca (caso edge: venta padre también está REGISTRADO).
 */

import { useEffect, useState } from "react";
import { detalleTransferenciasPendientes, forzarMarcarTransferenciaVerificada } from "../services/api";
import { useToast } from "./Toast";
import { useSesion } from "../contexts/SesionContext";

interface Props {
  onCerrar: () => void;
  /** Se llama después de cualquier acción para refrescar el dashboard. */
  onCambio?: () => void;
}

export default function ModalTransferenciasPendientes({ onCerrar, onCambio }: Props) {
  const { toastExito, toastError } = useToast();
  const { esAdmin } = useSesion();
  const [items, setItems] = useState<any[]>([]);
  const [cargando, setCargando] = useState(true);
  const [forzandoId, setForzandoId] = useState<string | null>(null);

  const cargar = async () => {
    setCargando(true);
    try {
      const data = await detalleTransferenciasPendientes();
      setItems(data);
    } catch (e: any) {
      toastError("Error: " + (e?.message || e));
    } finally {
      setCargando(false);
    }
  };

  useEffect(() => { cargar(); }, []);

  const handleForzar = async (item: any) => {
    if (!confirm(
      `Marcar como VERIFICADA (forzado) la transferencia:\n\n` +
      `${item.numero} — $${item.monto.toFixed(2)}\n${item.cliente}\n${item.fecha}\n\n` +
      `Esto dejará de aparecer en el contador. Solo úsalo si confirmas que ya verificaste manualmente o el banco te confirmó el ingreso.`
    )) return;

    const motivo = prompt("Motivo del forzado (queda registrado):") || "Sin motivo";
    if (!motivo.trim()) return;

    const key = `${item.origen}-${item.id}`;
    setForzandoId(key);
    try {
      await forzarMarcarTransferenciaVerificada(item.origen, item.id, motivo);
      toastExito("Transferencia marcada como verificada");
      await cargar();
      onCambio?.();
    } catch (e: any) {
      toastError("Error: " + (e?.message || e));
    } finally {
      setForzandoId(null);
    }
  };

  return (
    <div className="modal-overlay" onClick={onCerrar}>
      <div
        className="modal-content"
        style={{ maxWidth: 720, maxHeight: "85vh", overflowY: "auto", padding: 0 }}
        onClick={(e) => e.stopPropagation()}
      >
        {/* Header */}
        <div style={{
          padding: "14px 20px", borderBottom: "1px solid var(--color-border)",
          display: "flex", justifyContent: "space-between", alignItems: "center",
        }}>
          <div>
            <h2 style={{ margin: 0, fontSize: 17 }}>🏦 Transferencias pendientes de verificar</h2>
            <div style={{ fontSize: 11, color: "var(--color-text-secondary)", marginTop: 2 }}>
              Detalle exacto de qué está contando el badge del dashboard.
            </div>
          </div>
          <button
            onClick={onCerrar}
            style={{
              background: "transparent", border: "none", fontSize: 22,
              cursor: "pointer", color: "var(--color-text-muted)", padding: 0, width: 30,
            }}
          >×</button>
        </div>

        {/* Body */}
        <div style={{ padding: 16 }}>
          {cargando ? (
            <div style={{ textAlign: "center", padding: 40, color: "var(--color-text-muted)" }}>
              Cargando...
            </div>
          ) : items.length === 0 ? (
            <div style={{ textAlign: "center", padding: 40 }}>
              <div style={{ fontSize: 36, marginBottom: 8 }}>✨</div>
              <div style={{ fontWeight: 600, color: "var(--color-success)" }}>
                Sin transferencias pendientes
              </div>
              <div style={{ fontSize: 12, color: "var(--color-text-secondary)", marginTop: 4 }}>
                El badge debería desaparecer en el próximo refresh.
              </div>
            </div>
          ) : (
            <>
              <div style={{
                padding: "10px 12px", marginBottom: 12, fontSize: 12,
                background: "rgba(245,158,11,0.08)",
                border: "1px solid rgba(245,158,11,0.3)", borderRadius: 6,
                color: "var(--color-text-secondary)",
              }}>
                ℹ Estas son TODAS las transferencias que el sistema cuenta como pendientes
                (sin filtro de fecha). Si verificaste alguna y sigue aquí, usa <strong>"Forzar verificar"</strong>.
              </div>

              <table style={{ width: "100%", fontSize: 12, borderCollapse: "collapse" }}>
                <thead>
                  <tr style={{ background: "var(--color-surface-hover)" }}>
                    <th style={{ padding: "8px", textAlign: "left", fontSize: 10, fontWeight: 600 }}>#</th>
                    <th style={{ padding: "8px", textAlign: "left", fontSize: 10, fontWeight: 600 }}>Venta</th>
                    <th style={{ padding: "8px", textAlign: "left", fontSize: 10, fontWeight: 600 }}>Fecha</th>
                    <th style={{ padding: "8px", textAlign: "left", fontSize: 10, fontWeight: 600 }}>Cliente</th>
                    <th style={{ padding: "8px", textAlign: "right", fontSize: 10, fontWeight: 600 }}>Monto</th>
                    <th style={{ padding: "8px", textAlign: "center", fontSize: 10, fontWeight: 600 }}>Tipo</th>
                    {esAdmin && <th style={{ padding: "8px", textAlign: "center", fontSize: 10, fontWeight: 600 }}>Acción</th>}
                  </tr>
                </thead>
                <tbody>
                  {items.map((it, idx) => {
                    const key = `${it.origen}-${it.id}`;
                    return (
                      <tr key={key} style={{ borderTop: "1px solid var(--color-border)" }}>
                        <td style={{ padding: "8px", color: "var(--color-text-secondary)" }}>{idx + 1}</td>
                        <td style={{ padding: "8px", fontWeight: 600 }}>{it.numero}</td>
                        <td style={{ padding: "8px", fontSize: 11 }}>{it.fecha?.slice(0, 16) || "—"}</td>
                        <td style={{ padding: "8px", maxWidth: 180, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>{it.cliente}</td>
                        <td style={{ padding: "8px", textAlign: "right", fontWeight: 600 }}>${it.monto.toFixed(2)}</td>
                        <td style={{ padding: "8px", textAlign: "center" }}>
                          <span style={{
                            fontSize: 9, padding: "2px 6px", borderRadius: 999, fontWeight: 700,
                            background: it.origen === "VENTA" ? "rgba(59,130,246,0.15)" : "rgba(168,85,247,0.15)",
                            color: it.origen === "VENTA" ? "var(--color-primary)" : "#a855f7",
                          }}>
                            {it.origen === "VENTA" ? "VENTA" : "MIXTO"}
                          </span>
                        </td>
                        {esAdmin && (
                          <td style={{ padding: "8px", textAlign: "center" }}>
                            <button
                              onClick={() => handleForzar(it)}
                              disabled={forzandoId === key}
                              style={{
                                fontSize: 10, padding: "4px 10px", borderRadius: 4,
                                background: "var(--color-warning)", color: "#fff",
                                border: "none", cursor: "pointer", fontWeight: 600,
                              }}
                            >
                              {forzandoId === key ? "..." : "Forzar"}
                            </button>
                          </td>
                        )}
                      </tr>
                    );
                  })}
                </tbody>
              </table>
            </>
          )}
        </div>

        {/* Footer */}
        <div style={{
          padding: "12px 20px", borderTop: "1px solid var(--color-border)",
          display: "flex", justifyContent: "space-between", alignItems: "center",
          background: "var(--color-surface-hover)",
        }}>
          <button className="btn btn-outline" onClick={cargar}>
            🔄 Refrescar
          </button>
          <button className="btn btn-primary" onClick={onCerrar}>
            Cerrar
          </button>
        </div>
      </div>
    </div>
  );
}
