import { useState, useEffect, useCallback } from "react";
import { listarDocumentosRecientes, eliminarBorrador, obtenerVenta, imprimirTicket, imprimirTicketPdf, imprimirRide, imprimirGuiaRemisionPdf } from "../services/api";
import { useToast } from "./Toast";
import type { DocumentoReciente, VentaCompleta } from "../types";

interface Props {
  abierto: boolean;
  onCerrar: () => void;
  onCargarDocumento: (venta: VentaCompleta) => void;
  ticketUsarPdf?: boolean;
}

export default function DocumentosRecientes({ abierto, onCerrar, onCargarDocumento, ticketUsarPdf }: Props) {
  const { toastExito, toastError } = useToast();
  const [documentos, setDocumentos] = useState<DocumentoReciente[]>([]);
  const [filtro, setFiltro] = useState<string>("TODOS");
  const [cargando, setCargando] = useState(false);

  const cargar = useCallback(async () => {
    try {
      const docs = await listarDocumentosRecientes(20);
      setDocumentos(docs);
    } catch { /* silencioso */ }
  }, []);

  useEffect(() => {
    if (abierto) cargar();
  }, [abierto, cargar]);

  if (!abierto) return null;

  const filtrados = filtro === "TODOS"
    ? documentos
    : documentos.filter(d => d.tipo_estado === filtro);

  const handleAbrir = async (doc: DocumentoReciente) => {
    setCargando(true);
    try {
      const venta = await obtenerVenta(doc.id);
      onCargarDocumento(venta);
      onCerrar();
      toastExito(`${doc.tipo_estado === "BORRADOR" ? "Borrador" : "Cotizacion"} cargado en carrito`);
    } catch (err) {
      toastError("Error al cargar: " + err);
    } finally {
      setCargando(false);
    }
  };

  const handleEliminar = async (doc: DocumentoReciente) => {
    if (!confirm(`Eliminar ${doc.numero}?`)) return;
    try {
      await eliminarBorrador(doc.id);
      toastExito("Documento eliminado");
      cargar();
    } catch (err) {
      toastError("Error: " + err);
    }
  };

  const handleImprimir = async (doc: DocumentoReciente) => {
    try {
      if (ticketUsarPdf) {
        await imprimirTicketPdf(doc.id);
        toastExito("PDF generado");
      } else {
        await imprimirTicket(doc.id);
        toastExito("Ticket impreso");
      }
    } catch (err) {
      toastError("Error: " + err);
    }
  };

  const handleRide = async (doc: DocumentoReciente) => {
    try {
      await imprimirRide(doc.id);
      toastExito("RIDE generado");
    } catch (err) {
      toastError("Error: " + err);
    }
  };

  const badgeColor = (tipo: string) => {
    switch (tipo) {
      case "BORRADOR": return { bg: "rgba(245, 158, 11, 0.15)", color: "var(--color-warning)" };
      case "COTIZACION": return { bg: "rgba(96, 165, 250, 0.15)", color: "var(--color-primary)" };
      case "CONVERTIDA": return { bg: "rgba(148, 163, 184, 0.15)", color: "var(--color-text-secondary)" };
      case "GUIA_REMISION": return { bg: "rgba(251, 146, 60, 0.15)", color: "var(--color-warning)" };
      default: return { bg: "rgba(74, 222, 128, 0.15)", color: "var(--color-success)" };
    }
  };

  const formatHora = (fecha: string) => {
    try {
      const d = new Date(fecha);
      return d.toLocaleTimeString("es", { hour: "2-digit", minute: "2-digit" });
    } catch { return ""; }
  };

  return (
    <div
      style={{
        position: "fixed", inset: 0, background: "rgba(0,0,0,0.5)",
        display: "flex", justifyContent: "flex-end", zIndex: 50,
      }}
      onClick={onCerrar}
    >
      <div
        style={{
          width: 360, height: "100%", background: "var(--color-surface)",
          borderLeft: "2px solid var(--color-border-strong, var(--color-border))",
          display: "flex", flexDirection: "column",
          animation: "slide-in-right 0.2s ease-out",
        }}
        onClick={(e) => e.stopPropagation()}
      >
        {/* Header */}
        <div style={{
          padding: "12px 16px", borderBottom: "2px solid var(--color-border)",
          display: "flex", justifyContent: "space-between", alignItems: "center",
        }}>
          <span style={{ fontWeight: 700, fontSize: 15 }}>Documentos Recientes</span>
          <button
            onClick={onCerrar}
            style={{ background: "none", border: "none", cursor: "pointer", fontSize: 18, padding: "0 4px" }}
          >x</button>
        </div>

        {/* Filtros */}
        <div style={{ display: "flex", gap: 4, padding: "8px 12px", borderBottom: "1px solid var(--color-border)" }}>
          {["TODOS", "COMPLETADA", "BORRADOR", "COTIZACION", "GUIA_REMISION"].map(f => (
            <button
              key={f}
              className="btn"
              style={{
                fontSize: 10, padding: "3px 8px", fontWeight: 600,
                background: filtro === f ? "#3b82f6" : "transparent",
                color: filtro === f ? "white" : "var(--color-text-secondary)",
                border: filtro === f ? "none" : "1px solid var(--color-border)",
              }}
              onClick={() => setFiltro(f)}
            >
              {f === "TODOS" ? "Todos" : f === "COMPLETADA" ? "Ventas" : f === "BORRADOR" ? "Borrador" : f === "COTIZACION" ? "Cotizacion" : "Guias"}
            </button>
          ))}
        </div>

        {/* Lista */}
        <div style={{ flex: 1, overflowY: "auto", padding: "4px 0" }}>
          {filtrados.length === 0 ? (
            <div style={{ padding: 24, textAlign: "center", color: "var(--color-text-secondary)", fontSize: 12 }}>
              No hay documentos
            </div>
          ) : (
            filtrados.map((doc) => {
              const badge = badgeColor(doc.tipo_estado);
              return (
                <div key={doc.id} style={{
                  padding: "10px 14px", borderBottom: "1px solid var(--color-border)",
                }}>
                  {/* Fila 1: Numero + Badge + Total */}
                  <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: 4 }}>
                    <div style={{ display: "flex", alignItems: "center", gap: 6 }}>
                      <span style={{ fontWeight: 700, fontSize: 13 }}>{doc.numero}</span>
                      <span style={{
                        fontSize: 9, padding: "1px 6px", borderRadius: 3, fontWeight: 600,
                        background: badge.bg, color: badge.color,
                      }}>
                        {doc.tipo_estado}
                      </span>
                    </div>
                    <span style={{ fontWeight: 700, fontSize: 14 }}>${doc.total.toFixed(2)}</span>
                  </div>

                  {/* Fila 2: Cliente + Hora */}
                  <div style={{ display: "flex", justifyContent: "space-between", fontSize: 11, color: "var(--color-text-secondary)", marginBottom: 6 }}>
                    <span>{doc.cliente_nombre || "Consumidor Final"}</span>
                    <span>{formatHora(doc.fecha)}</span>
                  </div>

                  {/* Fila 3: Acciones */}
                  <div style={{ display: "flex", gap: 4, flexWrap: "wrap" }}>
                    {(doc.tipo_estado === "BORRADOR" || doc.tipo_estado === "COTIZACION" || doc.tipo_estado === "GUIA_REMISION") && (
                      <>
                        <button className="btn btn-outline" style={{ fontSize: 10, padding: "2px 8px" }}
                          disabled={cargando}
                          onClick={() => handleAbrir(doc)}>
                          {doc.tipo_estado === "BORRADOR" ? "Abrir" : "Convertir"}
                        </button>
                        {doc.tipo_estado !== "GUIA_REMISION" && (
                          <button className="btn btn-outline" style={{ fontSize: 10, padding: "2px 8px", color: "var(--color-danger)" }}
                            onClick={() => handleEliminar(doc)}>
                            Eliminar
                          </button>
                        )}
                      </>
                    )}
                    {doc.tipo_estado === "COMPLETADA" && (
                      <>
                        <button className="btn btn-outline" style={{ fontSize: 10, padding: "2px 8px" }}
                          onClick={() => handleImprimir(doc)}>
                          Ticket
                        </button>
                        {doc.tipo_documento === "FACTURA" && (
                          <button className="btn btn-outline" style={{ fontSize: 10, padding: "2px 8px" }}
                            onClick={() => handleRide(doc)}>
                            RIDE
                          </button>
                        )}
                      </>
                    )}
                    {doc.tipo_estado === "COTIZACION" && (
                      <button className="btn btn-outline" style={{ fontSize: 10, padding: "2px 8px" }}
                        onClick={() => handleImprimir(doc)}>
                        Imprimir
                      </button>
                    )}
                    {doc.tipo_estado === "GUIA_REMISION" && (
                      <button className="btn btn-outline" style={{ fontSize: 10, padding: "2px 8px" }}
                        onClick={async () => {
                          try { await imprimirGuiaRemisionPdf(doc.id); toastExito("PDF generado"); }
                          catch (e) { toastError("Error: " + e); }
                        }}>
                        PDF
                      </button>
                    )}
                  </div>
                </div>
              );
            })
          )}
        </div>

        {/* Footer */}
        <div style={{ padding: "8px 14px", borderTop: "1px solid var(--color-border)", textAlign: "center" }}>
          <button className="btn btn-outline" style={{ fontSize: 11, width: "100%", justifyContent: "center" }}
            onClick={cargar}>
            Actualizar
          </button>
        </div>
      </div>

      <style>{`
        @keyframes slide-in-right {
          from { transform: translateX(100%); }
          to { transform: translateX(0); }
        }
      `}</style>
    </div>
  );
}
