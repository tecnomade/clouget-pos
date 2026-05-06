/**
 * ModalDetalleNc — vista de detalle de una Nota de Crédito (SRI o devolución interna).
 *
 * Antes de v2.3.62 no había forma de ver qué se devolvió en una NC ya creada.
 * Solo se podían usar los botones SRI/XML/RIDE para NC autorizadas; las
 * devoluciones internas no tenían NINGUNA forma de ver su contenido.
 *
 * Este modal muestra:
 *  - Header con número, motivo, fecha, cliente, factura original, estado SRI
 *  - Lista de items devueltos con cantidades y montos
 *  - Desglose del REEMBOLSO (efectivo / transferencia / crédito) — feature nueva
 *  - Indicación si se creó retiro_caja automático (anti descuadre)
 *  - Botones: Imprimir térmica, Imprimir PDF
 */

import { useEffect, useState } from "react";
import { obtenerNotaCredito, imprimirTicketNc, generarRideNcPdf } from "../services/api";
import { useToast } from "./Toast";

interface Props {
  ncId: number;
  onCerrar: () => void;
}

export default function ModalDetalleNc({ ncId, onCerrar }: Props) {
  const { toastExito, toastError } = useToast();
  const [data, setData] = useState<{ header: any; items: any[] } | null>(null);
  const [cargando, setCargando] = useState(true);
  const [imprimiendo, setImprimiendo] = useState<"ticket" | "pdf" | null>(null);

  useEffect(() => {
    obtenerNotaCredito(ncId)
      .then(setData)
      .catch((e) => toastError("Error cargando NC: " + e))
      .finally(() => setCargando(false));
  }, [ncId]);

  const handleImprimirTicket = async () => {
    setImprimiendo("ticket");
    try {
      const msg = await imprimirTicketNc(ncId);
      toastExito(msg);
    } catch (e: any) {
      toastError("Error imprimiendo ticket: " + (e?.message || e));
    } finally {
      setImprimiendo(null);
    }
  };

  const handleImprimirPdf = async () => {
    setImprimiendo("pdf");
    try {
      await generarRideNcPdf(ncId);
      toastExito("PDF abierto");
    } catch (e: any) {
      toastError("Error generando PDF: " + (e?.message || e));
    } finally {
      setImprimiendo(null);
    }
  };

  if (cargando) {
    return (
      <div className="modal-overlay" onClick={onCerrar}>
        <div className="modal-content" style={{ maxWidth: 480, padding: 32, textAlign: "center" }}>
          Cargando detalle...
        </div>
      </div>
    );
  }

  if (!data) {
    return (
      <div className="modal-overlay" onClick={onCerrar}>
        <div className="modal-content" style={{ maxWidth: 480, padding: 32 }}>
          No se pudo cargar la NC. <button onClick={onCerrar} className="btn btn-outline" style={{ marginLeft: 8 }}>Cerrar</button>
        </div>
      </div>
    );
  }

  const h = data.header;
  const esSri = h.estado_sri === "AUTORIZADA";
  const esDevolucionInterna = h.estado_sri === "NO_APLICA";
  const totalReembolsado = (h.monto_efectivo_devuelto || 0) + (h.monto_transfer_devuelto || 0) + (h.monto_credito_devuelto || 0);

  // Color del badge según estado SRI
  const estadoColor =
    h.estado_sri === "AUTORIZADA" ? "var(--color-success)"
    : h.estado_sri === "RECHAZADA" ? "var(--color-danger)"
    : h.estado_sri === "NO_APLICA" ? "var(--color-text-muted)"
    : "var(--color-warning)";

  const estadoLabel =
    h.estado_sri === "AUTORIZADA" ? "✓ AUTORIZADA SRI"
    : h.estado_sri === "RECHAZADA" ? "✗ RECHAZADA SRI"
    : h.estado_sri === "NO_APLICA" ? "🏠 DEVOLUCIÓN INTERNA"
    : "⏱ PENDIENTE SRI";

  return (
    <div className="modal-overlay" onClick={onCerrar}>
      <div
        className="modal-content"
        style={{
          maxWidth: 720,
          maxHeight: "90vh",
          overflowY: "auto",
          padding: 0,
        }}
        onClick={(e) => e.stopPropagation()}
      >
        {/* Header */}
        <div style={{
          padding: "16px 20px",
          borderBottom: "1px solid var(--color-border)",
          display: "flex", justifyContent: "space-between", alignItems: "flex-start", gap: 12,
        }}>
          <div>
            <h2 style={{ margin: 0, fontSize: 18 }}>
              📄 Nota de Crédito {h.numero_factura_nc || h.numero}
            </h2>
            <div style={{ fontSize: 12, color: "var(--color-text-secondary)", marginTop: 2 }}>
              {h.fecha} · Por {h.usuario || "—"}
            </div>
          </div>
          <div style={{ display: "flex", gap: 8, alignItems: "center" }}>
            <span style={{
              fontSize: 11, fontWeight: 700, padding: "4px 10px", borderRadius: 999,
              background: `color-mix(in srgb, ${estadoColor} 15%, transparent)`,
              color: estadoColor,
            }}>
              {estadoLabel}
            </span>
            <button
              onClick={onCerrar}
              style={{
                background: "transparent", border: "none", fontSize: 22,
                cursor: "pointer", color: "var(--color-text-muted)", padding: 0, width: 30,
              }}
            >×</button>
          </div>
        </div>

        {/* Body */}
        <div style={{ padding: "16px 20px" }}>
          {/* Info general */}
          <div style={{
            display: "grid",
            gridTemplateColumns: "1fr 1fr",
            gap: 12,
            marginBottom: 16,
            padding: 12,
            background: "var(--color-surface-hover)",
            borderRadius: 8,
            fontSize: 12,
          }}>
            <div>
              <div style={{ color: "var(--color-text-secondary)", fontSize: 10, fontWeight: 600, textTransform: "uppercase", marginBottom: 2 }}>Cliente</div>
              <div style={{ fontWeight: 600 }}>{h.cliente_nombre}</div>
              {h.cliente_identificacion && <div style={{ fontSize: 11, color: "var(--color-text-secondary)" }}>{h.cliente_identificacion}</div>}
            </div>
            <div>
              <div style={{ color: "var(--color-text-secondary)", fontSize: 10, fontWeight: 600, textTransform: "uppercase", marginBottom: 2 }}>Venta original</div>
              <div style={{ fontWeight: 600 }}>{h.venta_numero}</div>
              <div style={{ fontSize: 11, color: "var(--color-text-secondary)" }}>
                {h.venta_tipo} · ${(h.venta_total || 0).toFixed(2)} · {h.venta_forma_pago}
              </div>
            </div>
            <div style={{ gridColumn: "1 / -1" }}>
              <div style={{ color: "var(--color-text-secondary)", fontSize: 10, fontWeight: 600, textTransform: "uppercase", marginBottom: 2 }}>Motivo</div>
              <div style={{ fontStyle: "italic" }}>"{h.motivo}"</div>
            </div>
            <div>
              <div style={{ color: "var(--color-text-secondary)", fontSize: 10, fontWeight: 600, textTransform: "uppercase", marginBottom: 2 }}>Tipo</div>
              <div style={{ fontWeight: 600, color: h.tipo_devolucion === "PARCIAL" ? "var(--color-warning)" : "var(--color-primary)" }}>
                {h.tipo_devolucion === "PARCIAL" ? "🔸 Devolución parcial" : "🔹 Devolución total"}
              </div>
            </div>
            {h.autorizacion_sri && (
              <div>
                <div style={{ color: "var(--color-text-secondary)", fontSize: 10, fontWeight: 600, textTransform: "uppercase", marginBottom: 2 }}>Autorización SRI</div>
                <div style={{ fontFamily: "monospace", fontSize: 10 }}>{h.autorizacion_sri}</div>
              </div>
            )}
          </div>

          {/* Items devueltos */}
          <div style={{ marginBottom: 16 }}>
            <h3 style={{ fontSize: 13, marginBottom: 8, fontWeight: 700, color: "var(--color-text-secondary)", textTransform: "uppercase", letterSpacing: 0.5 }}>
              📦 Items devueltos ({data.items.length})
            </h3>
            <table style={{ width: "100%", fontSize: 12, borderCollapse: "collapse" }}>
              <thead>
                <tr style={{ background: "var(--color-surface-hover)" }}>
                  <th style={{ padding: "8px 10px", textAlign: "left", fontSize: 10, fontWeight: 600 }}>Producto</th>
                  <th style={{ padding: "8px 10px", textAlign: "right", fontSize: 10, fontWeight: 600 }}>Cant.</th>
                  <th style={{ padding: "8px 10px", textAlign: "right", fontSize: 10, fontWeight: 600 }}>P. Unit</th>
                  <th style={{ padding: "8px 10px", textAlign: "right", fontSize: 10, fontWeight: 600 }}>Subtotal</th>
                </tr>
              </thead>
              <tbody>
                {data.items.map((it: any) => (
                  <tr key={it.id} style={{ borderTop: "1px solid var(--color-border)" }}>
                    <td style={{ padding: "8px 10px" }}>{it.nombre_producto}</td>
                    <td style={{ padding: "8px 10px", textAlign: "right" }}>{it.cantidad}</td>
                    <td style={{ padding: "8px 10px", textAlign: "right" }}>${it.precio_unitario.toFixed(2)}</td>
                    <td style={{ padding: "8px 10px", textAlign: "right", fontWeight: 600 }}>${it.subtotal.toFixed(2)}</td>
                  </tr>
                ))}
                <tr style={{ borderTop: "2px solid var(--color-border)", background: "var(--color-surface-hover)" }}>
                  <td colSpan={3} style={{ padding: "10px", textAlign: "right", fontWeight: 700, fontSize: 13 }}>
                    TOTAL NC
                  </td>
                  <td style={{ padding: "10px", textAlign: "right", fontWeight: 800, fontSize: 14, color: "var(--color-primary)" }}>
                    ${h.total.toFixed(2)}
                  </td>
                </tr>
              </tbody>
            </table>
          </div>

          {/* Desglose REEMBOLSO — la feature nueva del v2.3.62 */}
          <div style={{
            padding: 14,
            background: "rgba(34, 197, 94, 0.06)",
            border: "1px solid rgba(34, 197, 94, 0.25)",
            borderRadius: 10,
            marginBottom: 16,
          }}>
            <h3 style={{ fontSize: 13, marginBottom: 10, fontWeight: 700, color: "var(--color-success)" }}>
              💵 Reembolso al cliente
            </h3>
            {totalReembolsado < 0.01 ? (
              <div style={{ fontSize: 12, color: "var(--color-text-secondary)" }}>
                Sin información de reembolso registrada (NC creada antes de v2.3.62 o sin reembolso aplicado).
              </div>
            ) : (
              <>
                <div style={{ display: "grid", gridTemplateColumns: "repeat(3, 1fr)", gap: 10 }}>
                  <ReembolsoBox icon="💵" label="Efectivo" monto={h.monto_efectivo_devuelto} color="#16a34a" />
                  <ReembolsoBox icon="🏦" label="Transferencia" monto={h.monto_transfer_devuelto} color="#0ea5e9" />
                  <ReembolsoBox icon="📋" label="A crédito" monto={h.monto_credito_devuelto} color="#f59e0b" />
                </div>
                {h.retiro_caja_id && (h.monto_efectivo_devuelto > 0) && (
                  <div style={{
                    marginTop: 10, padding: "8px 10px", fontSize: 11,
                    background: "rgba(59,130,246,0.08)", borderRadius: 6,
                    color: "var(--color-primary)",
                  }}>
                    ℹ Retiro automático de caja generado (#{h.retiro_caja_id}) — el cierre quedará cuadrado.
                  </div>
                )}
                {h.monto_transfer_devuelto > 0 && (
                  <div style={{
                    marginTop: 8, padding: "8px 10px", fontSize: 11,
                    background: "rgba(245,158,11,0.08)", borderRadius: 6,
                    color: "var(--color-warning)",
                  }}>
                    ⚠ Transferencia: el reembolso por banco lo realiza admin manualmente desde su app bancaria.
                  </div>
                )}
              </>
            )}
          </div>

          {esDevolucionInterna && (
            <div style={{
              padding: 10, fontSize: 11,
              background: "rgba(245,158,11,0.08)", borderRadius: 6,
              color: "var(--color-text-secondary)", marginBottom: 16,
            }}>
              Este es un comprobante <strong>interno</strong> (sin valor fiscal SRI).
              Se usa cuando la venta original era NOTA DE VENTA o factura no autorizada.
            </div>
          )}
        </div>

        {/* Footer con acciones */}
        <div style={{
          padding: "12px 20px",
          borderTop: "1px solid var(--color-border)",
          display: "flex", gap: 8, justifyContent: "space-between",
          background: "var(--color-surface-hover)",
        }}>
          <button className="btn btn-outline" onClick={onCerrar}>
            Cerrar
          </button>
          <div style={{ display: "flex", gap: 8 }}>
            <button
              className="btn btn-outline"
              onClick={handleImprimirTicket}
              disabled={imprimiendo !== null}
              title="Imprimir en impresora térmica configurada"
            >
              🖨 {imprimiendo === "ticket" ? "Imprimiendo..." : "Térmica"}
            </button>
            <button
              className="btn btn-primary"
              onClick={handleImprimirPdf}
              disabled={imprimiendo !== null}
              title={esSri ? "RIDE PDF SRI" : "PDF de devolución interna"}
            >
              📄 {imprimiendo === "pdf" ? "Generando..." : "PDF"}
            </button>
          </div>
        </div>
      </div>
    </div>
  );
}

function ReembolsoBox({ icon, label, monto, color }: {
  icon: string; label: string; monto: number; color: string;
}) {
  const activo = monto > 0.01;
  return (
    <div style={{
      padding: "10px 12px",
      background: activo ? "var(--color-surface)" : "transparent",
      border: `1px solid ${activo ? color : "var(--color-border)"}`,
      borderRadius: 8,
      opacity: activo ? 1 : 0.4,
    }}>
      <div style={{ fontSize: 18, marginBottom: 2 }}>{icon}</div>
      <div style={{ fontSize: 10, color: "var(--color-text-secondary)", fontWeight: 600 }}>
        {label}
      </div>
      <div style={{ fontSize: 16, fontWeight: 700, color: activo ? color : "var(--color-text-secondary)" }}>
        ${monto.toFixed(2)}
      </div>
    </div>
  );
}
