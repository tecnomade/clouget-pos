/**
 * v2.5.68 — Guías de Remisión electrónicas (SRI codDoc 06)
 *
 * Vive DENTRO del módulo Contabilidad (documento 100% tributario). NO satura el POS.
 * Permite:
 *   - Crear una guía desde cero con un buscador de productos (descripción + cantidad,
 *     sin precio obligatorio → soporta "camión, 5.5 ton chatarra ferrosa").
 *   - Emitirla al SRI (firma + autorización) en un paso, reutilizando ModalEmisionGuia.
 *   - Listar las guías recientes con su estado SRI y reimprimir / reintentar.
 */
import { useState, useEffect } from "react";
import {
  listarGuiasRemision, buscarProductos, guardarGuiaRemision,
  imprimirGuiaRemisionPdf, buscarClientes,
} from "../services/api";
import { useToast } from "./Toast";
import ModalEmisionGuia from "./ModalEmisionGuia";
import type { ProductoBusqueda, Cliente, NuevaVenta } from "../types";

interface GuiaRow {
  id: number;
  numero: string;
  fecha: string;
  cliente_nombre?: string;
  total: number;
  estado: string;
  estado_sri?: string;
  numero_factura?: string;
}

interface ItemGuia {
  producto_id: number;
  nombre: string;
  cantidad: number;
}

function fechaHace(dias: number): string {
  const d = new Date();
  d.setDate(d.getDate() - dias);
  return d.toISOString().slice(0, 10);
}
function fechaHoy(): string {
  return new Date().toISOString().slice(0, 10);
}

export default function GuiasRemisionContab() {
  const { toastExito, toastError } = useToast();
  const [guias, setGuias] = useState<GuiaRow[]>([]);
  const [cargando, setCargando] = useState(false);

  // Modal de creación
  const [creando, setCreando] = useState(false);
  const [items, setItems] = useState<ItemGuia[]>([]);
  const [busqueda, setBusqueda] = useState("");
  const [resultados, setResultados] = useState<ProductoBusqueda[]>([]);
  const [clienteBusq, setClienteBusq] = useState("");
  const [clientesRes, setClientesRes] = useState<Cliente[]>([]);
  const [cliente, setCliente] = useState<Cliente | null>(null);
  const [guardando, setGuardando] = useState(false);

  // Emisión SRI (reutiliza modal)
  const [emitir, setEmitir] = useState<{ id: number; numero: string } | null>(null);

  const cargar = async () => {
    setCargando(true);
    try {
      const g = await listarGuiasRemision({ fechaDesde: fechaHace(30), fechaHasta: fechaHoy() });
      setGuias(g);
    } catch (e) {
      toastError("Error cargando guías: " + e);
    } finally {
      setCargando(false);
    }
  };
  useEffect(() => { cargar(); }, []);

  // Buscador de productos (debounce simple)
  useEffect(() => {
    if (!creando) return;
    const t = setTimeout(async () => {
      if (busqueda.trim().length < 2) { setResultados([]); return; }
      try { setResultados(await buscarProductos(busqueda.trim())); } catch { /* ignore */ }
    }, 300);
    return () => clearTimeout(t);
  }, [busqueda, creando]);

  // Buscador de cliente
  useEffect(() => {
    if (!creando) return;
    const t = setTimeout(async () => {
      if (clienteBusq.trim().length < 2) { setClientesRes([]); return; }
      try { setClientesRes(await buscarClientes(clienteBusq.trim())); } catch { /* ignore */ }
    }, 300);
    return () => clearTimeout(t);
  }, [clienteBusq, creando]);

  const abrirCrear = () => {
    setItems([]); setBusqueda(""); setResultados([]);
    setClienteBusq(""); setClientesRes([]); setCliente(null);
    setCreando(true);
  };

  const agregarItem = (p: ProductoBusqueda) => {
    setItems(prev => {
      const ya = prev.find(i => i.producto_id === p.id);
      if (ya) return prev.map(i => i.producto_id === p.id ? { ...i, cantidad: i.cantidad + 1 } : i);
      return [...prev, { producto_id: p.id, nombre: p.nombre, cantidad: 1 }];
    });
    setBusqueda(""); setResultados([]);
  };

  const setCantidad = (pid: number, cant: number) => {
    setItems(prev => prev.map(i => i.producto_id === pid ? { ...i, cantidad: cant } : i));
  };
  const quitarItem = (pid: number) => setItems(prev => prev.filter(i => i.producto_id !== pid));

  const crearYContinuar = async () => {
    if (items.length === 0) { toastError("Agregue al menos un producto"); return; }
    if (items.some(i => !i.cantidad || i.cantidad <= 0)) { toastError("Las cantidades deben ser mayores a 0"); return; }
    setGuardando(true);
    const nueva: NuevaVenta = {
      cliente_id: cliente?.id ?? 1,
      // Precio 0: la guía de remisión no transporta valores comerciales (caso chatarra).
      items: items.map(i => ({
        producto_id: i.producto_id,
        cantidad: i.cantidad,
        precio_unitario: 0,
        descuento: 0,
        iva_porcentaje: 0,
        subtotal: 0,
        info_adicional: null,
      } as any)),
      forma_pago: "EFECTIVO",
      monto_recibido: 0,
      descuento: 0,
      tipo_documento: "NOTA_VENTA",
      es_fiado: false,
      guia_direccion_destino: cliente?.direccion || null,
    };
    try {
      const res = await guardarGuiaRemision(nueva);
      setCreando(false);
      // Abrir modal de emisión SRI con la guía recién creada
      setEmitir({ id: res.venta.id!, numero: res.venta.numero });
    } catch (e) {
      toastError("Error al crear la guía: " + e);
    } finally {
      setGuardando(false);
    }
  };

  const badge = (estadoSri?: string) => {
    const s = estadoSri || "NO_APLICA";
    const map: Record<string, { bg: string; col: string; txt: string }> = {
      AUTORIZADA: { bg: "rgba(74,222,128,0.15)", col: "var(--color-success)", txt: "✓ Autorizada" },
      PENDIENTE: { bg: "rgba(251,191,36,0.15)", col: "var(--color-warning)", txt: "Pendiente" },
      RECHAZADA: { bg: "rgba(239,68,68,0.15)", col: "var(--color-danger)", txt: "Rechazada" },
      NO_APLICA: { bg: "rgba(148,163,184,0.15)", col: "var(--color-text-secondary)", txt: "Sin emitir" },
    };
    const m = map[s] || map.NO_APLICA;
    return <span style={{ fontSize: 10, padding: "2px 8px", borderRadius: 3, fontWeight: 600, background: m.bg, color: m.col }}>{m.txt}</span>;
  };

  return (
    <div>
      <div className="card">
        <div className="card-header flex justify-between items-center">
          <span>Guías de Remisión electrónicas (SRI codDoc 06)</span>
          <button className="btn btn-primary" style={{ fontSize: 12, padding: "5px 12px" }} onClick={abrirCrear}>
            + Nueva Guía de Remisión
          </button>
        </div>
        <div className="card-body" style={{ padding: 0 }}>
          <div style={{ padding: "10px 14px", fontSize: 12, color: "var(--color-text-secondary)", borderBottom: "1px solid var(--color-border)" }}>
            La Guía de Remisión sustenta legalmente el <strong>traslado de mercadería</strong> (no la venta).
            No requiere factura previa ni valores comerciales — sirve para transportar incluso por peso/volumen
            (ej. "camión, 5.5 ton chatarra ferrosa"). Últimos 30 días.
          </div>
          <table className="table" style={{ fontSize: 13 }}>
            <thead>
              <tr>
                <th>Número</th>
                <th>Fecha</th>
                <th>Cliente / Destinatario</th>
                <th>SRI</th>
                <th>Nro Autorización</th>
                <th></th>
              </tr>
            </thead>
            <tbody>
              {guias.map(g => (
                <tr key={g.id}>
                  <td><strong>{g.numero}</strong></td>
                  <td className="text-secondary" style={{ fontSize: 12 }}>
                    {g.fecha ? new Date(g.fecha).toLocaleDateString("es-EC", { day: "2-digit", month: "2-digit", year: "2-digit" }) : "-"}
                  </td>
                  <td>{g.cliente_nombre || "Consumidor Final"}</td>
                  <td>{badge(g.estado_sri)}</td>
                  <td className="text-secondary" style={{ fontSize: 11 }}>{g.numero_factura || "-"}</td>
                  <td>
                    <div className="flex gap-1">
                      <button className="btn btn-outline" style={{ fontSize: 10, padding: "2px 8px" }}
                        onClick={async () => { try { await imprimirGuiaRemisionPdf(g.id); toastExito("PDF generado"); } catch (e) { toastError("Error: " + e); } }}>
                        PDF
                      </button>
                      {(g.estado_sri === "PENDIENTE" || g.estado_sri === "RECHAZADA" || !g.estado_sri || g.estado_sri === "NO_APLICA") && (
                        <button className="btn btn-outline" style={{ fontSize: 10, padding: "2px 8px", fontWeight: 600, color: "var(--color-primary)", borderColor: "var(--color-primary)" }}
                          onClick={() => setEmitir({ id: g.id, numero: g.numero })}>
                          {g.estado_sri === "PENDIENTE" || g.estado_sri === "RECHAZADA" ? "↻ Reintentar SRI" : "📤 Emitir SRI"}
                        </button>
                      )}
                    </div>
                  </td>
                </tr>
              ))}
              {guias.length === 0 && !cargando && (
                <tr><td colSpan={6} className="text-center text-secondary" style={{ padding: 24 }}>No hay guías de remisión en los últimos 30 días</td></tr>
              )}
            </tbody>
          </table>
        </div>
      </div>

      {/* Modal CREAR guía */}
      {creando && (
        <div style={{ position: "fixed", inset: 0, background: "rgba(0,0,0,0.5)", display: "flex", alignItems: "center", justifyContent: "center", zIndex: 150 }}
          onClick={(e) => { if (e.target === e.currentTarget) setCreando(false); }}>
          <div className="card" style={{ width: 640, maxHeight: "90vh", overflow: "auto" }}>
            <div className="card-header flex justify-between items-center">
              <span>Nueva Guía de Remisión</span>
              <button className="btn btn-outline" style={{ padding: "2px 8px" }} onClick={() => setCreando(false)}>x</button>
            </div>
            <div className="card-body">
              {/* Cliente / destinatario */}
              <label style={{ fontSize: 11, color: "var(--color-text-secondary)" }}>Cliente / Destinatario</label>
              {cliente ? (
                <div className="flex justify-between items-center" style={{ padding: "6px 10px", background: "var(--color-surface-alt)", borderRadius: 6, marginBottom: 12 }}>
                  <span style={{ fontSize: 13 }}>{cliente.nombre}{cliente.identificacion ? ` — ${cliente.identificacion}` : ""}</span>
                  <button className="btn btn-outline" style={{ fontSize: 10, padding: "2px 8px" }} onClick={() => setCliente(null)}>Cambiar</button>
                </div>
              ) : (
                <div style={{ marginBottom: 12, position: "relative" }}>
                  <input className="input" style={{ width: "100%", fontSize: 13 }} placeholder="Buscar cliente (opcional, por defecto Consumidor Final)"
                    value={clienteBusq} onChange={(e) => setClienteBusq(e.target.value)} />
                  {clientesRes.length > 0 && (
                    <div style={{ position: "absolute", zIndex: 5, background: "var(--color-surface)", border: "1px solid var(--color-border)", borderRadius: 6, width: "100%", maxHeight: 180, overflow: "auto" }}>
                      {clientesRes.map(c => (
                        <div key={c.id} style={{ padding: "6px 10px", cursor: "pointer", fontSize: 13, borderBottom: "1px solid var(--color-border)" }}
                          onClick={() => { setCliente(c); setClienteBusq(""); setClientesRes([]); }}>
                          {c.nombre}{c.identificacion ? ` — ${c.identificacion}` : ""}
                        </div>
                      ))}
                    </div>
                  )}
                </div>
              )}

              {/* Buscador de productos */}
              <label style={{ fontSize: 11, color: "var(--color-text-secondary)" }}>Mercadería a transportar</label>
              <div style={{ position: "relative", marginBottom: 10 }}>
                <input className="input" style={{ width: "100%", fontSize: 13 }} placeholder="Buscar producto por nombre o código..."
                  value={busqueda} onChange={(e) => setBusqueda(e.target.value)} />
                {resultados.length > 0 && (
                  <div style={{ position: "absolute", zIndex: 5, background: "var(--color-surface)", border: "1px solid var(--color-border)", borderRadius: 6, width: "100%", maxHeight: 200, overflow: "auto" }}>
                    {resultados.map(p => (
                      <div key={p.id} style={{ padding: "6px 10px", cursor: "pointer", fontSize: 13, borderBottom: "1px solid var(--color-border)" }}
                        onClick={() => agregarItem(p)}>
                        {p.nombre}{p.codigo ? ` (${p.codigo})` : ""}
                      </div>
                    ))}
                  </div>
                )}
              </div>

              {items.length > 0 && (
                <table className="table" style={{ fontSize: 13, marginBottom: 12 }}>
                  <thead>
                    <tr><th>Producto</th><th className="text-right" style={{ width: 120 }}>Cantidad</th><th style={{ width: 40 }}></th></tr>
                  </thead>
                  <tbody>
                    {items.map(i => (
                      <tr key={i.producto_id}>
                        <td>{i.nombre}</td>
                        <td className="text-right">
                          <input type="number" className="input" style={{ width: 100, fontSize: 13, textAlign: "right" }}
                            min={0} step="any" value={i.cantidad}
                            onChange={(e) => setCantidad(i.producto_id, parseFloat(e.target.value) || 0)} />
                        </td>
                        <td><button className="btn btn-outline" style={{ fontSize: 10, padding: "2px 6px", color: "var(--color-danger)" }} onClick={() => quitarItem(i.producto_id)}>x</button></td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              )}

              <div style={{ fontSize: 11, color: "var(--color-text-secondary)", marginBottom: 12 }}>
                ℹ La guía descuenta stock al crearse. En el siguiente paso se capturan los datos de
                transporte (transportista, placa, motivo) y se emite al SRI.
              </div>

              <button className="btn btn-primary" style={{ width: "100%", padding: "10px 0", fontWeight: 700 }}
                disabled={guardando} onClick={crearYContinuar}>
                {guardando ? "Creando..." : "Continuar a datos de transporte →"}
              </button>
            </div>
          </div>
        </div>
      )}

      {/* Modal EMITIR (reutilizable) */}
      {emitir && (
        <ModalEmisionGuia
          guiaId={emitir.id}
          numero={emitir.numero}
          titulo={`Guía de Remisión SRI (${emitir.numero})`}
          onClose={() => { setEmitir(null); cargar(); }}
          onEmitida={() => { setEmitir(null); cargar(); }}
        />
      )}
    </div>
  );
}
