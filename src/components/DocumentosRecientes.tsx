import { useState, useEffect, useCallback } from "react";
import { listarDocumentosRecientes, eliminarBorrador, obtenerVenta, imprimirTicket, imprimirTicketPdf, imprimirRide, imprimirGuiaRemisionPdf, generarCotizacionPdf, generarNotaVentaPdf, convertirGuiaAVenta, listarCuentasBanco } from "../services/api";
import { useToast } from "./Toast";
import type { DocumentoReciente, VentaCompleta, CuentaBanco } from "../types";

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

  // Estados del modal de conversion guia -> venta (igual que en GuiasRemisionPage)
  const [convertir, setConvertir] = useState<VentaCompleta | null>(null);
  const [convFormaPago, setConvFormaPago] = useState<string>("EFECTIVO");
  const [convMonto, setConvMonto] = useState<string>("");
  const [convBancoId, setConvBancoId] = useState<number | null>(null);
  const [convReferencia, setConvReferencia] = useState<string>("");
  const [convEsFiado, setConvEsFiado] = useState<boolean>(false);
  const [convirtiendo, setConvirtiendo] = useState<boolean>(false);
  const [cuentasBanco, setCuentasBanco] = useState<CuentaBanco[]>([]);
  // Items editables — array de { producto_id, cantidad, precio_unitario, descuento, iva_porcentaje, nombre }
  const [convItems, setConvItems] = useState<{
    producto_id: number; cantidad: number; precio_unitario: number;
    descuento: number; iva_porcentaje: number; nombre: string;
  }[]>([]);
  // Estado de la guia: si PENDIENTE, permite editar cantidad. Si ENTREGADA, no.
  const [convGuiaEstado, setConvGuiaEstado] = useState<string>("PENDIENTE");

  const cargar = useCallback(async () => {
    try {
      const docs = await listarDocumentosRecientes(20);
      setDocumentos(docs);
    } catch { /* silencioso */ }
  }, []);

  useEffect(() => {
    if (abierto) {
      cargar();
      listarCuentasBanco().then(setCuentasBanco).catch(() => {});
    }
  }, [abierto, cargar]);

  if (!abierto) return null;

  const filtrados = filtro === "TODOS"
    ? documentos
    : documentos.filter(d => d.tipo_estado === filtro);

  const handleAbrir = async (doc: DocumentoReciente) => {
    // Hard-block: NUNCA permitir cargar una GUIA_REMISION al carrito.
    // Si se carga, el usuario al cobrar dispara crear_venta que descuenta
    // stock de nuevo (el stock ya se descontó al crear la guía).
    // Para facturar guías hay que usar el botón "💰 Facturar".
    if (doc.tipo_estado === "GUIA_REMISION") {
      toastError("⚠ Las guías de remisión deben facturarse con el botón 💰 Facturar (no cargar al carrito — duplicaría el descuento de stock).");
      return;
    }
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

  // Abrir el modal de conversion de guia (igual que en GuiasRemisionPage).
  // Trae los detalles completos para mostrar items y total al cajero.
  const abrirConvertir = async (doc: DocumentoReciente) => {
    setCargando(true);
    try {
      const vc = await obtenerVenta(doc.id);
      setConvertir(vc);
      setConvGuiaEstado(vc.venta.estado || "PENDIENTE");
      // Inicializar items editables con datos de la guia
      setConvItems(vc.detalles.map(d => ({
        producto_id: d.producto_id,
        cantidad: d.cantidad,
        precio_unitario: d.precio_unitario,
        descuento: d.descuento,
        iva_porcentaje: d.iva_porcentaje,
        nombre: d.nombre_producto || `#${d.producto_id}`,
      })));
      setConvFormaPago("EFECTIVO");
      setConvMonto("");
      setConvBancoId(null);
      setConvReferencia("");
      setConvEsFiado(false);
    } catch (err) {
      toastError("Error al cargar guia: " + err);
    } finally {
      setCargando(false);
    }
  };

  // Calcular total en vivo desde items editados
  const calcularTotalConv = () => {
    let sub = 0, iva = 0;
    for (const it of convItems) {
      const subItem = it.cantidad * it.precio_unitario - it.descuento;
      sub += subItem;
      if (it.iva_porcentaje > 0) iva += subItem * (it.iva_porcentaje / 100);
    }
    return Math.round((sub + iva) * 100) / 100;
  };

  // Ejecutar la conversion. Usa convertir_guia_a_venta del backend que NO
  // descuenta stock de nuevo (cantidad fija si guia ENTREGADA).
  const ejecutarConversion = async () => {
    if (!convertir || !convertir.venta.id) return;
    if (convFormaPago === "TRANSFERENCIA" && !convBancoId) {
      toastError("Seleccione cuenta bancaria"); return;
    }
    setConvirtiendo(true);
    try {
      const totalCalc = calcularTotalConv();
      const monto = convFormaPago === "EFECTIVO" && !convEsFiado
        ? (parseFloat(convMonto) || totalCalc)
        : totalCalc;
      // Detectar si hay cambios respecto a los items originales
      const huboCambios = convertir.detalles.some((d, i) => {
        const it = convItems[i];
        return !it
          || Math.abs(it.precio_unitario - d.precio_unitario) > 0.001
          || Math.abs(it.descuento - d.descuento) > 0.001
          || (convGuiaEstado === "PENDIENTE" && Math.abs(it.cantidad - d.cantidad) > 0.0001);
      });
      const itemsOverride = huboCambios
        ? convItems.map(it => ({
            producto_id: it.producto_id,
            precio_unitario: it.precio_unitario,
            descuento: it.descuento,
            cantidad: convGuiaEstado === "PENDIENTE" ? it.cantidad : undefined,
          }))
        : undefined;
      const res = await convertirGuiaAVenta({
        guiaId: convertir.venta.id,
        formaPago: convFormaPago === "TRANSFERENCIA" ? "TRANSFER" : convFormaPago,
        montoRecibido: monto,
        esFiado: convEsFiado,
        bancoId: convBancoId ?? undefined,
        referenciaPago: convReferencia.trim() || undefined,
        itemsOverride,
      });
      toastExito(`Guía convertida a venta ${res.venta.numero}`);
      setConvertir(null);
      cargar();
    } catch (err) {
      toastError("Error al convertir: " + err);
    } finally {
      setConvirtiendo(false);
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
                    {(doc.tipo_estado === "BORRADOR" || doc.tipo_estado === "COTIZACION") && (
                      <>
                        <button className="btn btn-outline" style={{ fontSize: 10, padding: "2px 8px" }}
                          disabled={cargando}
                          onClick={() => handleAbrir(doc)}>
                          {doc.tipo_estado === "BORRADOR" ? "Abrir" : "Convertir"}
                        </button>
                        <button className="btn btn-outline" style={{ fontSize: 10, padding: "2px 8px", color: "var(--color-danger)" }}
                          onClick={() => handleEliminar(doc)}>
                          Eliminar
                        </button>
                      </>
                    )}
                    {doc.tipo_estado === "GUIA_REMISION" && (
                      <button className="btn" style={{
                        fontSize: 10, padding: "2px 8px", fontWeight: 600,
                        background: "var(--color-primary)", color: "white", border: "none",
                      }}
                        disabled={cargando}
                        title="Convertir a venta cobrada (no descuenta stock de nuevo, ya se descontó al crear la guía)"
                        onClick={() => abrirConvertir(doc)}>
                        💰 Facturar
                      </button>
                    )}
                    {doc.tipo_estado === "COMPLETADA" && (
                      <>
                        <button className="btn btn-outline" style={{ fontSize: 10, padding: "2px 8px" }}
                          onClick={() => handleImprimir(doc)}>
                          Ticket
                        </button>
                        <button className="btn btn-outline" style={{ fontSize: 10, padding: "2px 8px" }}
                          onClick={async () => {
                            try { await generarNotaVentaPdf(doc.id); toastExito("PDF A4 generado"); }
                            catch (e) { toastError("Error: " + e); }
                          }}>
                          A4
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
                      <>
                        <button className="btn btn-outline" style={{ fontSize: 10, padding: "2px 8px" }}
                          onClick={() => handleImprimir(doc)}>
                          Ticket
                        </button>
                        <button className="btn btn-outline" style={{ fontSize: 10, padding: "2px 8px" }}
                          onClick={async () => {
                            try { await generarCotizacionPdf(doc.id); toastExito("PDF A4 generado"); }
                            catch (e) { toastError("Error: " + e); }
                          }}>
                          A4
                        </button>
                      </>
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

      {/* Modal de conversion guia -> venta (mismo flujo que GuiasRemisionPage).
          Usa convertir_guia_a_venta del backend que crea la venta SIN tocar
          inventario (ya descontado al crear la guia). Asi no hay doble descuento. */}
      {convertir && (
        <div
          style={{
            position: "fixed", inset: 0, background: "rgba(0,0,0,0.6)",
            display: "flex", alignItems: "center", justifyContent: "center", zIndex: 200,
          }}
          onClick={(e) => { if (e.target === e.currentTarget) setConvertir(null); }}
        >
          <div className="card" style={{ width: 550, maxHeight: "85vh", overflow: "auto" }}
               onClick={(e) => e.stopPropagation()}>
            <div className="card-header flex justify-between items-center">
              <span>Facturar Guía {convertir.venta.numero}</span>
              <button className="btn btn-outline" style={{ padding: "2px 8px" }}
                onClick={() => setConvertir(null)}>x</button>
            </div>
            <div className="card-body">
              <div style={{
                fontSize: 11, padding: 8, marginBottom: 12,
                background: "rgba(96, 165, 250, 0.08)", border: "1px solid rgba(96, 165, 250, 0.25)",
                borderRadius: 6, color: "var(--color-primary)",
              }}>
                ℹ El stock NO se descuenta de nuevo (ya se descontó al crear la guía).
                Esto solo registra el cobro y marca la guía como FACTURADA.
              </div>

              <div style={{ fontSize: 12, fontWeight: 600, marginBottom: 6, color: "var(--color-text-secondary)" }}>
                Cliente: {convertir.cliente_nombre || "Consumidor Final"}
              </div>
              <div style={{
                fontSize: 11, padding: 6, marginBottom: 8, borderRadius: 4,
                background: convGuiaEstado === "PENDIENTE" ? "rgba(245,158,11,0.08)" : "rgba(74,222,128,0.08)",
                color: convGuiaEstado === "PENDIENTE" ? "var(--color-warning)" : "var(--color-success)",
              }}>
                {convGuiaEstado === "PENDIENTE"
                  ? "🟡 Guía PENDIENTE — puedes editar cantidad, precio y descuento. El stock se ajustará."
                  : "🟢 Guía ya ENTREGADA — solo puedes editar precio y descuento (cantidad fija porque ya fue entregada al cliente)."
                }
              </div>
              <div style={{ fontSize: 12, fontWeight: 600, marginBottom: 6, color: "var(--color-text-secondary)" }}>Productos (click en valores para editar)</div>
              <table className="table" style={{ fontSize: 12, marginBottom: 16 }}>
                <thead>
                  <tr>
                    <th>Producto</th>
                    <th style={{ width: 70 }} className="text-right">Cant.</th>
                    <th style={{ width: 90 }} className="text-right">P.Unit</th>
                    <th style={{ width: 80 }} className="text-right">Desc.</th>
                    <th style={{ width: 90 }} className="text-right">Subtotal</th>
                  </tr>
                </thead>
                <tbody>
                  {convItems.map((it, i) => {
                    const subtotal = it.cantidad * it.precio_unitario - it.descuento;
                    return (
                      <tr key={i}>
                        <td>{it.nombre}</td>
                        <td className="text-right">
                          <input type="number" step="0.01" min="0.01"
                            disabled={convGuiaEstado !== "PENDIENTE"}
                            style={{
                              width: "100%", textAlign: "right", padding: "2px 4px", fontSize: 12,
                              border: convGuiaEstado === "PENDIENTE" ? "1px solid var(--color-border)" : "none",
                              borderRadius: 3, background: convGuiaEstado === "PENDIENTE" ? "var(--color-surface)" : "transparent",
                            }}
                            value={it.cantidad}
                            onChange={(e) => {
                              const v = parseFloat(e.target.value) || 0;
                              setConvItems(prev => prev.map((p, j) => j === i ? { ...p, cantidad: v } : p));
                            }} />
                        </td>
                        <td className="text-right">
                          <input type="number" step="0.01" min="0"
                            style={{
                              width: "100%", textAlign: "right", padding: "2px 4px", fontSize: 12,
                              border: "1px solid var(--color-border)", borderRadius: 3,
                              background: "var(--color-surface)",
                            }}
                            value={it.precio_unitario}
                            onChange={(e) => {
                              const v = parseFloat(e.target.value) || 0;
                              setConvItems(prev => prev.map((p, j) => j === i ? { ...p, precio_unitario: v } : p));
                            }} />
                        </td>
                        <td className="text-right">
                          <input type="number" step="0.01" min="0"
                            style={{
                              width: "100%", textAlign: "right", padding: "2px 4px", fontSize: 12,
                              border: "1px solid var(--color-border)", borderRadius: 3,
                              background: "var(--color-surface)",
                            }}
                            value={it.descuento}
                            onChange={(e) => {
                              const v = parseFloat(e.target.value) || 0;
                              setConvItems(prev => prev.map((p, j) => j === i ? { ...p, descuento: v } : p));
                            }} />
                        </td>
                        <td className="text-right" style={{ fontWeight: 600 }}>${subtotal.toFixed(2)}</td>
                      </tr>
                    );
                  })}
                </tbody>
              </table>

              <div style={{ borderTop: "2px solid var(--color-border)", paddingTop: 8, marginBottom: 16 }}>
                <div className="flex justify-between" style={{ fontWeight: 700, fontSize: 16 }}>
                  <span>TOTAL:</span>
                  <span className="text-success">${calcularTotalConv().toFixed(2)}</span>
                </div>
              </div>

              {/* Forma de pago */}
              <div style={{ background: "var(--color-surface-alt)", borderRadius: 8, padding: 14 }}>
                <div style={{ fontSize: 12, fontWeight: 600, marginBottom: 10, color: "var(--color-text-secondary)" }}>
                  Forma de Pago
                </div>

                <div style={{ display: "flex", gap: 6, marginBottom: 12 }}>
                  <button className="btn" style={{
                    flex: 1, padding: "8px 0", fontWeight: 600, fontSize: 13,
                    background: convFormaPago === "EFECTIVO" && !convEsFiado ? "rgba(74, 222, 128, 0.2)" : "transparent",
                    color: convFormaPago === "EFECTIVO" && !convEsFiado ? "var(--color-success)" : "var(--color-text-secondary)",
                    border: convFormaPago === "EFECTIVO" && !convEsFiado ? "1px solid rgba(74, 222, 128, 0.4)" : "1px solid var(--color-border)",
                  }} onClick={() => { setConvFormaPago("EFECTIVO"); setConvEsFiado(false); }}>
                    Efectivo
                  </button>
                  <button className="btn" style={{
                    flex: 1, padding: "8px 0", fontWeight: 600, fontSize: 13,
                    background: convFormaPago === "TRANSFERENCIA" ? "rgba(96, 165, 250, 0.2)" : "transparent",
                    color: convFormaPago === "TRANSFERENCIA" ? "var(--color-primary)" : "var(--color-text-secondary)",
                    border: convFormaPago === "TRANSFERENCIA" ? "1px solid rgba(96, 165, 250, 0.4)" : "1px solid var(--color-border)",
                  }} onClick={() => { setConvFormaPago("TRANSFERENCIA"); setConvEsFiado(false); }}>
                    Transfer.
                  </button>
                  <button className="btn" style={{
                    flex: 1, padding: "8px 0", fontWeight: 600, fontSize: 13,
                    background: convEsFiado ? "rgba(251, 191, 36, 0.2)" : "transparent",
                    color: convEsFiado ? "var(--color-warning)" : "var(--color-text-secondary)",
                    border: convEsFiado ? "1px solid rgba(251, 191, 36, 0.4)" : "1px solid var(--color-border)",
                  }} onClick={() => { setConvFormaPago("EFECTIVO"); setConvEsFiado(true); }}>
                    Crédito
                  </button>
                </div>

                {convFormaPago === "EFECTIVO" && !convEsFiado && (
                  <div style={{ marginBottom: 10 }}>
                    <label style={{ fontSize: 12, color: "var(--color-text-secondary)", display: "block", marginBottom: 4 }}>
                      Monto recibido (vacío = exacto)
                    </label>
                    <input type="number" className="input" style={{ width: "100%", fontSize: 14 }}
                      placeholder={`$${convertir.venta.total.toFixed(2)}`}
                      value={convMonto} onChange={(e) => setConvMonto(e.target.value)} />
                  </div>
                )}

                {convFormaPago === "TRANSFERENCIA" && (
                  <>
                    <div style={{ marginBottom: 10 }}>
                      <label style={{ fontSize: 12, color: "var(--color-text-secondary)", display: "block", marginBottom: 4 }}>
                        Cuenta bancaria *
                      </label>
                      <select className="input" style={{ width: "100%", fontSize: 13 }}
                        value={convBancoId ?? ""}
                        onChange={(e) => setConvBancoId(e.target.value ? Number(e.target.value) : null)}>
                        <option value="">Seleccionar...</option>
                        {cuentasBanco.filter(c => c.activa).map(c => (
                          <option key={c.id} value={c.id}>
                            {c.nombre} {c.numero_cuenta ? `- ${c.numero_cuenta}` : ""}
                          </option>
                        ))}
                      </select>
                    </div>
                    <div style={{ marginBottom: 10 }}>
                      <label style={{ fontSize: 12, color: "var(--color-text-secondary)", display: "block", marginBottom: 4 }}>
                        Referencia
                      </label>
                      <input type="text" className="input" style={{ width: "100%", fontSize: 13 }}
                        placeholder="Nro. comprobante"
                        value={convReferencia} onChange={(e) => setConvReferencia(e.target.value)} />
                    </div>
                  </>
                )}

                {convEsFiado && (
                  <div style={{
                    padding: 8, background: "rgba(251, 191, 36, 0.1)", borderRadius: 6,
                    fontSize: 11, color: "var(--color-warning)", marginBottom: 8,
                  }}>
                    Se creará cuenta por cobrar al cliente por este monto.
                  </div>
                )}

                <button className="btn" style={{
                  width: "100%", padding: "10px 0", fontWeight: 700, fontSize: 14, marginTop: 8,
                  background: "var(--color-success)", color: "white", border: "none",
                }}
                  disabled={convirtiendo}
                  onClick={ejecutarConversion}>
                  {convirtiendo ? "Convirtiendo..." : "💰 Confirmar y Facturar"}
                </button>
              </div>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
