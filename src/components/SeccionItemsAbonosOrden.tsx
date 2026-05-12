// v2.4.13 ST-5: Items presupuestados + Abonos en holding para una orden de servicio.
// Se embebe dentro del modal de detalle. Maneja su propio estado y se sincroniza
// con el backend en cada cambio.
//
// Uso:
//   <SeccionItemsAbonosOrden
//     ordenId={detalleId}
//     ordenEstado={detalle.estado}
//     onTotalChange={(t) => setTotalOrden(t)}   // opcional: el padre recibe el total
//     onAbonosChange={(t) => setTotalAbonos(t)} // opcional: el padre recibe el total holding
//   />

import { useEffect, useMemo, useState } from "react";
import {
  stListarItemsOrden, stAgregarItemOrden, stActualizarItemOrden,
  stEliminarItemOrden, stTotalOrden,
  stListarAbonos, stRecibirAbono, stEditarAbono, stEliminarAbono,
  buscarProductos, listarCuentasBanco,
} from "../services/api";
import type {
  ItemOrden, TotalOrden, AbonoServicio,
} from "../services/api";
import type { CuentaBanco, ProductoBusqueda } from "../types";
import { useToast } from "./Toast";

interface Props {
  ordenId: number;
  ordenEstado: string; // si ENTREGADO o CANCELADA → solo lectura
  onTotalChange?: (total: TotalOrden) => void;
  onAbonosChange?: (totalHolding: number, abonos: AbonoServicio[]) => void;
}

const FORMAS_PAGO_ABONO = [
  { value: "EFECTIVO", label: "Efectivo" },
  { value: "TRANSFER", label: "Transferencia" },
  { value: "TARJETA", label: "Tarjeta" },
];

export default function SeccionItemsAbonosOrden({ ordenId, ordenEstado, onTotalChange, onAbonosChange }: Props) {
  const { toastExito, toastError } = useToast();
  const editable = ordenEstado !== "ENTREGADO" && ordenEstado !== "CANCELADA";

  const [items, setItems] = useState<ItemOrden[]>([]);
  const [total, setTotal] = useState<TotalOrden>({ subtotal_sin_iva: 0, subtotal_con_iva: 0, iva: 0, total: 0, cantidad_items: 0 });
  const [abonos, setAbonos] = useState<AbonoServicio[]>([]);

  // Form para agregar item
  const [busquedaProducto, setBusquedaProducto] = useState("");
  const [resultadosProducto, setResultadosProducto] = useState<ProductoBusqueda[]>([]);
  const [modoServicioManual, setModoServicioManual] = useState(false);
  const [descripcionManual, setDescripcionManual] = useState("");
  const [cantidadManual, setCantidadManual] = useState("1");
  const [precioManual, setPrecioManual] = useState("");
  const [ivaManual, setIvaManual] = useState("0");

  // Form para recibir abono
  const [mostrarFormAbono, setMostrarFormAbono] = useState(false);
  const [abonoMonto, setAbonoMonto] = useState("");
  const [abonoForma, setAbonoForma] = useState("EFECTIVO");
  const [abonoBancoId, setAbonoBancoId] = useState<number | null>(null);
  const [abonoReferencia, setAbonoReferencia] = useState("");
  const [abonoObs, setAbonoObs] = useState("");
  const [bancos, setBancos] = useState<CuentaBanco[]>([]);
  // v2.4.28: editar abono en HOLDING (corregir typo de monto/forma)
  const [editandoAbonoId, setEditandoAbonoId] = useState<number | null>(null);
  const [editAbonoMonto, setEditAbonoMonto] = useState("");
  const [editAbonoForma, setEditAbonoForma] = useState("EFECTIVO");
  const [editAbonoBancoId, setEditAbonoBancoId] = useState<number | null>(null);
  const [editAbonoReferencia, setEditAbonoReferencia] = useState("");
  const [editAbonoObs, setEditAbonoObs] = useState("");

  const totalHolding = useMemo(
    () => abonos.filter(a => a.estado === "HOLDING").reduce((s, a) => s + a.monto, 0),
    [abonos],
  );
  const saldoPendiente = Math.max(total.total - totalHolding, 0);

  const recargar = async () => {
    try {
      const [its, tot, abs] = await Promise.all([
        stListarItemsOrden(ordenId),
        stTotalOrden(ordenId),
        stListarAbonos(ordenId),
      ]);
      setItems(its);
      setTotal(tot);
      setAbonos(abs);
      onTotalChange?.(tot);
      const th = abs.filter(a => a.estado === "HOLDING").reduce((s, a) => s + a.monto, 0);
      onAbonosChange?.(th, abs);
    } catch (err) {
      toastError("Error cargando items: " + err);
    }
  };

  useEffect(() => { recargar(); }, [ordenId]);
  useEffect(() => { listarCuentasBanco().then(setBancos).catch(() => {}); }, []);

  // Buscar productos al teclear
  useEffect(() => {
    if (modoServicioManual) return;
    if (busquedaProducto.trim().length < 2) { setResultadosProducto([]); return; }
    const t = setTimeout(async () => {
      try {
        const r = await buscarProductos(busquedaProducto.trim());
        setResultadosProducto(r.slice(0, 8));
      } catch { setResultadosProducto([]); }
    }, 200);
    return () => clearTimeout(t);
  }, [busquedaProducto, modoServicioManual]);

  const handleAgregarProducto = async (p: ProductoBusqueda) => {
    try {
      await stAgregarItemOrden(ordenId, p.nombre, 1, p.precio_venta, p.id, p.iva_porcentaje, false);
      toastExito("Item agregado");
      setBusquedaProducto("");
      setResultadosProducto([]);
      recargar();
    } catch (err) { toastError("" + err); }
  };

  const handleAgregarServicioManual = async () => {
    const precio = parseFloat(precioManual) || 0;
    const cant = parseFloat(cantidadManual) || 1;
    const iva = parseFloat(ivaManual) || 0;
    if (!descripcionManual.trim()) { toastError("Describe el servicio"); return; }
    if (precio <= 0) { toastError("Indica un precio mayor a 0"); return; }
    if (cant <= 0) { toastError("La cantidad debe ser mayor a 0"); return; }
    try {
      await stAgregarItemOrden(ordenId, descripcionManual.trim(), cant, precio, null, iva, true);
      toastExito("Servicio agregado");
      setDescripcionManual("");
      setCantidadManual("1");
      setPrecioManual("");
      setIvaManual("0");
      setModoServicioManual(false);
      recargar();
    } catch (err) { toastError("" + err); }
  };

  const handleEditarItem = async (it: ItemOrden, campo: "cantidad" | "precio_unitario", valor: number) => {
    if (!it.id) return;
    const nuevo = { ...it, [campo]: valor };
    try {
      await stActualizarItemOrden(it.id, nuevo.descripcion, nuevo.cantidad, nuevo.precio_unitario, nuevo.iva_porcentaje);
      recargar();
    } catch (err) { toastError("" + err); }
  };

  const handleEliminarItem = async (id: number) => {
    if (!confirm("¿Eliminar este item?")) return;
    try {
      await stEliminarItemOrden(id);
      recargar();
    } catch (err) { toastError("" + err); }
  };

  const handleRecibirAbono = async () => {
    const monto = parseFloat(abonoMonto) || 0;
    if (monto <= 0) { toastError("El monto debe ser mayor a 0"); return; }
    try {
      await stRecibirAbono(
        ordenId, monto, abonoForma,
        abonoForma === "TRANSFER" ? abonoBancoId : null,
        abonoReferencia.trim() || null,
        abonoObs.trim() || null,
      );
      toastExito(`Abono de $${monto.toFixed(2)} recibido`);
      setMostrarFormAbono(false);
      setAbonoMonto("");
      setAbonoReferencia("");
      setAbonoObs("");
      recargar();
    } catch (err) { toastError("" + err); }
  };

  const colorTotal = saldoPendiente <= 0.001 ? "var(--color-success)" : "var(--color-text)";

  return (
    <div style={{ marginBottom: 12 }}>
      {/* ─── ITEMS ────────────────────────────────────────────── */}
      <div style={{ background: "var(--color-surface-alt)", padding: 10, borderRadius: 8, marginBottom: 12 }}>
        <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: 8 }}>
          <strong style={{ fontSize: 13 }}>🧾 Items de la orden ({items.length})</strong>
        </div>

        {/* Tabla items */}
        {items.length > 0 ? (
          <table style={{ width: "100%", fontSize: 12, borderCollapse: "collapse" }}>
            <thead>
              <tr style={{ borderBottom: "1px solid var(--color-border)" }}>
                <th style={{ textAlign: "left", padding: "4px 6px" }}>Descripción</th>
                <th style={{ textAlign: "right", padding: "4px 6px", width: 70 }}>Cant.</th>
                <th style={{ textAlign: "right", padding: "4px 6px", width: 90 }}>P.Unit</th>
                <th style={{ textAlign: "right", padding: "4px 6px", width: 50 }}>IVA%</th>
                <th style={{ textAlign: "right", padding: "4px 6px", width: 80 }}>Subtotal</th>
                {editable && <th style={{ width: 30 }}></th>}
              </tr>
            </thead>
            <tbody>
              {items.map(it => (
                <tr key={it.id} style={{ borderBottom: "1px solid var(--color-border)" }}>
                  <td style={{ padding: "4px 6px" }}>
                    {it.descripcion}
                    {!!it.es_servicio && <span style={{ marginLeft: 4, fontSize: 10, color: "var(--color-text-secondary)" }}>(servicio)</span>}
                  </td>
                  <td style={{ textAlign: "right", padding: "2px 6px" }}>
                    {editable ? (
                      <input className="input" type="number" step="0.01" min="0.01"
                        value={it.cantidad}
                        onChange={(e) => setItems(items.map(x => x.id === it.id ? { ...x, cantidad: parseFloat(e.target.value) || 0 } : x))}
                        onBlur={(e) => handleEditarItem(it, "cantidad", parseFloat(e.target.value) || 1)}
                        style={{ width: 60, fontSize: 11, textAlign: "right", padding: "2px 4px" }} />
                    ) : it.cantidad}
                  </td>
                  <td style={{ textAlign: "right", padding: "2px 6px" }}>
                    {editable ? (
                      <input className="input" type="number" step="0.01" min="0"
                        value={it.precio_unitario}
                        onChange={(e) => setItems(items.map(x => x.id === it.id ? { ...x, precio_unitario: parseFloat(e.target.value) || 0 } : x))}
                        onBlur={(e) => handleEditarItem(it, "precio_unitario", parseFloat(e.target.value) || 0)}
                        style={{ width: 80, fontSize: 11, textAlign: "right", padding: "2px 4px" }} />
                    ) : `$${it.precio_unitario.toFixed(2)}`}
                  </td>
                  <td style={{ textAlign: "right", padding: "4px 6px", fontSize: 11 }}>{it.iva_porcentaje.toFixed(0)}%</td>
                  <td style={{ textAlign: "right", padding: "4px 6px", fontWeight: 600 }}>${(it.cantidad * it.precio_unitario).toFixed(2)}</td>
                  {editable && (
                    <td style={{ textAlign: "center" }}>
                      <button onClick={() => it.id && handleEliminarItem(it.id)}
                        style={{ background: "transparent", border: "none", cursor: "pointer", color: "var(--color-danger)", fontSize: 14 }}
                        title="Eliminar item">×</button>
                    </td>
                  )}
                </tr>
              ))}
            </tbody>
          </table>
        ) : (
          <div style={{ padding: "8px 4px", fontSize: 12, color: "var(--color-text-secondary)", textAlign: "center" }}>
            Aún no hay items en esta orden.
          </div>
        )}

        {/* Totales */}
        <div style={{ marginTop: 8, paddingTop: 8, borderTop: "1px solid var(--color-border)", fontSize: 12 }}>
          {total.subtotal_con_iva > 0 && (
            <>
              <div style={{ display: "flex", justifyContent: "space-between" }}>
                <span style={{ color: "var(--color-text-secondary)" }}>Subtotal con IVA:</span>
                <span>${total.subtotal_con_iva.toFixed(2)}</span>
              </div>
              <div style={{ display: "flex", justifyContent: "space-between" }}>
                <span style={{ color: "var(--color-text-secondary)" }}>IVA:</span>
                <span>${total.iva.toFixed(2)}</span>
              </div>
            </>
          )}
          {total.subtotal_sin_iva > 0 && (
            <div style={{ display: "flex", justifyContent: "space-between" }}>
              <span style={{ color: "var(--color-text-secondary)" }}>Subtotal sin IVA:</span>
              <span>${total.subtotal_sin_iva.toFixed(2)}</span>
            </div>
          )}
          <div style={{ display: "flex", justifyContent: "space-between", fontWeight: 700, fontSize: 14, marginTop: 4 }}>
            <span>Total:</span>
            <span>${total.total.toFixed(2)}</span>
          </div>
          {totalHolding > 0 && (
            <>
              <div style={{ display: "flex", justifyContent: "space-between", color: "var(--color-warning)", fontSize: 12 }}>
                <span>− Abonos en holding:</span>
                <span>−${totalHolding.toFixed(2)}</span>
              </div>
              <div style={{ display: "flex", justifyContent: "space-between", fontWeight: 700, fontSize: 13, color: colorTotal }}>
                <span>Saldo a cobrar:</span>
                <span>${saldoPendiente.toFixed(2)}</span>
              </div>
            </>
          )}
        </div>

        {/* Form agregar item */}
        {editable && (
          <div style={{ marginTop: 10, paddingTop: 10, borderTop: "1px dashed var(--color-border)" }}>
            {!modoServicioManual ? (
              <>
                <div style={{ display: "flex", gap: 6, alignItems: "center" }}>
                  <input className="input" placeholder="🔎 Buscar producto del catálogo..."
                    value={busquedaProducto}
                    onChange={(e) => setBusquedaProducto(e.target.value)}
                    style={{ flex: 1, fontSize: 12 }} />
                  <button className="btn btn-outline" style={{ fontSize: 11, whiteSpace: "nowrap" }}
                    onClick={() => { setModoServicioManual(true); setBusquedaProducto(""); setResultadosProducto([]); }}>
                    + Servicio manual
                  </button>
                </div>
                {resultadosProducto.length > 0 && (
                  <div style={{ marginTop: 4, border: "1px solid var(--color-border)", borderRadius: 6, maxHeight: 180, overflowY: "auto", background: "var(--color-surface)" }}>
                    {resultadosProducto.map(p => {
                      // v2.5.1: mostrar stock disponible. Color rojo si <=0, amarillo si bajo
                      // (≤ stock_minimo), verde si OK. Si es servicio (precio_costo===-1 indicador
                      // o stock 0 sin minimo), mostrar "—" en lugar de cantidad.
                      const stock = p.stock_actual ?? 0;
                      const minimo = p.stock_minimo ?? 0;
                      const sinStock = stock <= 0;
                      const stockBajo = !sinStock && minimo > 0 && stock <= minimo;
                      const stockColor = sinStock
                        ? "var(--color-danger)"
                        : stockBajo
                          ? "var(--color-warning)"
                          : "var(--color-success)";
                      return (
                        <div key={p.id}
                          onClick={() => handleAgregarProducto(p)}
                          style={{ padding: "6px 8px", cursor: "pointer", fontSize: 12, borderBottom: "1px solid var(--color-border)", display: "flex", justifyContent: "space-between", alignItems: "center", gap: 8 }}
                          onMouseEnter={(e) => e.currentTarget.style.background = "var(--color-surface-alt)"}
                          onMouseLeave={(e) => e.currentTarget.style.background = "transparent"}>
                          <span style={{ flex: 1, minWidth: 0, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>
                            {p.nombre}
                            {p.codigo ? <span style={{ color: "var(--color-text-secondary)", fontSize: 10 }}> · {p.codigo}</span> : null}
                          </span>
                          <span style={{ fontSize: 10, padding: "1px 6px", borderRadius: 10, background: `${stockColor}20`, color: stockColor, fontWeight: 600, whiteSpace: "nowrap" }}
                            title={sinStock ? "Sin stock" : stockBajo ? `Stock bajo (mínimo: ${minimo})` : "Stock disponible"}>
                            📦 {stock}
                          </span>
                          <span style={{ fontWeight: 600, minWidth: 50, textAlign: "right" }}>${p.precio_venta.toFixed(2)}</span>
                        </div>
                      );
                    })}
                  </div>
                )}
              </>
            ) : (
              <div style={{ display: "grid", gridTemplateColumns: "1fr 70px 90px 70px auto auto", gap: 6, alignItems: "end" }}>
                <div>
                  <label style={{ fontSize: 10, fontWeight: 600, display: "block", marginBottom: 2, color: "var(--color-text-secondary)" }}>Descripción *</label>
                  <input className="input" placeholder="Mano de obra, diagnóstico..."
                    value={descripcionManual}
                    onChange={(e) => setDescripcionManual(e.target.value)}
                    style={{ fontSize: 12 }} autoFocus />
                </div>
                <div>
                  <label style={{ fontSize: 10, fontWeight: 600, display: "block", marginBottom: 2, color: "var(--color-text-secondary)" }}>Cantidad</label>
                  <input className="input" type="number" step="0.01" min="0.01"
                    value={cantidadManual}
                    onChange={(e) => setCantidadManual(e.target.value)}
                    style={{ fontSize: 12 }} />
                </div>
                <div>
                  <label style={{ fontSize: 10, fontWeight: 600, display: "block", marginBottom: 2, color: "var(--color-text-secondary)" }}>Precio unitario *</label>
                  <input className="input" type="number" step="0.01" placeholder="0.00"
                    value={precioManual}
                    onChange={(e) => setPrecioManual(e.target.value)}
                    style={{ fontSize: 12 }} />
                </div>
                <div>
                  <label style={{ fontSize: 10, fontWeight: 600, display: "block", marginBottom: 2, color: "var(--color-text-secondary)" }}>IVA %</label>
                  <input className="input" type="number" step="1" placeholder="0"
                    value={ivaManual}
                    onChange={(e) => setIvaManual(e.target.value)}
                    style={{ fontSize: 12 }} />
                </div>
                <button className="btn btn-success" style={{ fontSize: 11, height: 30 }} onClick={handleAgregarServicioManual}>Agregar</button>
                <button className="btn btn-outline" style={{ fontSize: 11, height: 30 }} onClick={() => setModoServicioManual(false)}>Cancelar</button>
              </div>
            )}
          </div>
        )}
      </div>

      {/* ─── ABONOS ───────────────────────────────────────────── */}
      <div style={{ background: "var(--color-surface-alt)", padding: 10, borderRadius: 8 }}>
        <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: 8 }}>
          <strong style={{ fontSize: 13 }}>💵 Abonos / Anticipos ({abonos.length})</strong>
          {editable && (
            <button className="btn btn-primary" style={{ fontSize: 11 }}
              onClick={() => { setMostrarFormAbono(!mostrarFormAbono); setAbonoMonto(""); }}>
              {mostrarFormAbono ? "Cancelar" : "+ Recibir abono"}
            </button>
          )}
        </div>

        {mostrarFormAbono && editable && (
          <div style={{ background: "var(--color-surface)", padding: 8, borderRadius: 6, marginBottom: 8, border: "1px solid var(--color-primary)" }}>
            <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: 6, marginBottom: 6 }}>
              <div>
                <label style={{ fontSize: 10, fontWeight: 600 }}>Monto *</label>
                <input className="input" type="number" step="0.01" autoFocus
                  value={abonoMonto}
                  onChange={(e) => setAbonoMonto(e.target.value)}
                  placeholder={saldoPendiente > 0 ? `Máx: $${saldoPendiente.toFixed(2)}` : "0.00"}
                  style={{ fontSize: 12 }} />
              </div>
              <div>
                <label style={{ fontSize: 10, fontWeight: 600 }}>Forma de pago</label>
                <select className="input" value={abonoForma}
                  onChange={(e) => setAbonoForma(e.target.value)}
                  style={{ fontSize: 12 }}>
                  {FORMAS_PAGO_ABONO.map(f => <option key={f.value} value={f.value}>{f.label}</option>)}
                </select>
              </div>
            </div>
            {abonoForma === "TRANSFER" && (
              <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: 6, marginBottom: 6 }}>
                <div>
                  <label style={{ fontSize: 10, fontWeight: 600 }}>Cuenta bancaria</label>
                  <select className="input" value={abonoBancoId || ""}
                    onChange={(e) => setAbonoBancoId(parseInt(e.target.value) || null)}
                    style={{ fontSize: 12 }}>
                    <option value="">Selecciona...</option>
                    {bancos.map(b => <option key={b.id} value={b.id}>{b.nombre}</option>)}
                  </select>
                </div>
                <div>
                  <label style={{ fontSize: 10, fontWeight: 600 }}>Referencia</label>
                  <input className="input" value={abonoReferencia}
                    onChange={(e) => setAbonoReferencia(e.target.value)}
                    placeholder="N° de transferencia"
                    style={{ fontSize: 12 }} />
                </div>
              </div>
            )}
            <input className="input" placeholder="Observación (opcional)"
              value={abonoObs}
              onChange={(e) => setAbonoObs(e.target.value)}
              style={{ fontSize: 12, marginBottom: 6 }} />
            <button className="btn btn-success" onClick={handleRecibirAbono} style={{ width: "100%", fontSize: 12 }}>
              Confirmar abono
            </button>
          </div>
        )}

        {abonos.length > 0 ? (
          <div style={{ fontSize: 11 }}>
            {abonos.map(a => {
              const colorEstado =
                a.estado === "HOLDING" ? "var(--color-warning)" :
                a.estado === "APLICADO" ? "var(--color-success)" :
                "var(--color-text-secondary)";
              const editandoEste = editandoAbonoId === a.id;
              return (
                <div key={a.id}
                  style={{ padding: "4px 6px", borderBottom: "1px solid var(--color-border)" }}>
                  <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
                    <div>
                      <strong style={{ color: colorEstado }}>${a.monto.toFixed(2)}</strong>
                      {" · "}
                      <span style={{ color: "var(--color-text-secondary)" }}>{a.forma_pago}</span>
                      {a.banco_nombre && <span style={{ color: "var(--color-text-secondary)" }}> · {a.banco_nombre}</span>}
                      {a.referencia_pago && <span style={{ color: "var(--color-text-secondary)" }}> · ref: {a.referencia_pago}</span>}
                      {a.observacion && <div style={{ fontSize: 10, color: "var(--color-text-secondary)" }}>{a.observacion}</div>}
                    </div>
                    <div style={{ textAlign: "right", display: "flex", gap: 4, alignItems: "center" }}>
                      {/* v2.4.28: editar/eliminar solo para HOLDING. APLICADO/DEVUELTO inmutables */}
                      {a.estado === "HOLDING" && editable && !editandoEste && (
                        <>
                          <button type="button"
                            onClick={() => {
                              setEditandoAbonoId(a.id || null);
                              setEditAbonoMonto(a.monto.toString());
                              setEditAbonoForma(a.forma_pago);
                              setEditAbonoBancoId(a.banco_id ?? null);
                              setEditAbonoReferencia(a.referencia_pago || "");
                              setEditAbonoObs(a.observacion || "");
                            }}
                            style={{ fontSize: 10, padding: "2px 6px", border: "1px solid var(--color-border)", borderRadius: 4, background: "transparent", cursor: "pointer", color: "var(--color-text)" }}
                            title="Editar abono (solo HOLDING)">
                            ✏
                          </button>
                          <button type="button"
                            onClick={async () => {
                              if (!a.id) return;
                              if (!confirm(`¿Eliminar este abono de $${a.monto.toFixed(2)}? Esta acción no se puede deshacer.`)) return;
                              try {
                                await stEliminarAbono(a.id);
                                toastExito("Abono eliminado");
                                await recargar();
                              } catch (err) { toastError("Error: " + err); }
                            }}
                            style={{ fontSize: 10, padding: "2px 6px", border: "1px solid rgba(239,68,68,0.4)", borderRadius: 4, background: "transparent", cursor: "pointer", color: "var(--color-danger)" }}
                            title="Eliminar abono (solo HOLDING)">
                            🗑
                          </button>
                        </>
                      )}
                      <span style={{ fontSize: 10, padding: "2px 6px", borderRadius: 4, background: colorEstado, color: "#fff" }}>
                        {a.estado}
                      </span>
                      <div style={{ fontSize: 10, color: "var(--color-text-secondary)" }}>{a.fecha?.slice(0, 16)}</div>
                    </div>
                  </div>
                  {/* v2.4.28: form de edicion inline */}
                  {editandoEste && (
                    <div style={{ marginTop: 6, padding: 8, background: "rgba(245, 158, 11, 0.06)", borderRadius: 6, border: "1px solid rgba(245, 158, 11, 0.3)" }}>
                      <div style={{ fontSize: 10, fontWeight: 600, marginBottom: 6, color: "var(--color-warning)" }}>
                        Editando abono — solo HOLDING puede modificarse
                      </div>
                      <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: 6, marginBottom: 6 }}>
                        <div>
                          <label style={{ fontSize: 10, color: "var(--color-text-secondary)" }}>Monto</label>
                          <input type="number" step="0.01" className="input"
                            value={editAbonoMonto}
                            onChange={(e) => setEditAbonoMonto(e.target.value)}
                            style={{ fontSize: 11 }} />
                        </div>
                        <div>
                          <label style={{ fontSize: 10, color: "var(--color-text-secondary)" }}>Forma de pago</label>
                          <select className="input"
                            value={editAbonoForma}
                            onChange={(e) => {
                              setEditAbonoForma(e.target.value);
                              if (e.target.value !== "TRANSFER") {
                                setEditAbonoBancoId(null);
                                setEditAbonoReferencia("");
                              }
                            }}
                            style={{ fontSize: 11 }}>
                            {FORMAS_PAGO_ABONO.map(f => <option key={f.value} value={f.value}>{f.label}</option>)}
                          </select>
                        </div>
                      </div>
                      {editAbonoForma === "TRANSFER" && (
                        <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: 6, marginBottom: 6 }}>
                          <select className="input"
                            value={editAbonoBancoId || ""}
                            onChange={(e) => setEditAbonoBancoId(e.target.value ? parseInt(e.target.value) : null)}
                            style={{ fontSize: 11 }}>
                            <option value="">— Banco —</option>
                            {bancos.map(b => <option key={b.id} value={b.id}>{b.nombre}</option>)}
                          </select>
                          <input className="input" placeholder="Referencia"
                            value={editAbonoReferencia}
                            onChange={(e) => setEditAbonoReferencia(e.target.value)}
                            style={{ fontSize: 11 }} />
                        </div>
                      )}
                      <input className="input" placeholder="Observación (opcional)"
                        value={editAbonoObs}
                        onChange={(e) => setEditAbonoObs(e.target.value)}
                        style={{ fontSize: 11, marginBottom: 6 }} />
                      <div style={{ display: "flex", gap: 6 }}>
                        <button type="button" className="btn btn-success" style={{ fontSize: 11, flex: 1 }}
                          onClick={async () => {
                            if (!a.id) return;
                            const m = parseFloat(editAbonoMonto);
                            if (isNaN(m) || m <= 0) { toastError("Monto inválido"); return; }
                            try {
                              await stEditarAbono(a.id, m, editAbonoForma,
                                editAbonoForma === "TRANSFER" ? editAbonoBancoId : null,
                                editAbonoForma === "TRANSFER" ? (editAbonoReferencia || null) : null,
                                editAbonoObs || null);
                              toastExito("Abono actualizado");
                              setEditandoAbonoId(null);
                              await recargar();
                            } catch (err) { toastError("Error: " + err); }
                          }}>
                          Guardar cambios
                        </button>
                        <button type="button" className="btn btn-outline" style={{ fontSize: 11 }}
                          onClick={() => setEditandoAbonoId(null)}>
                          Cancelar
                        </button>
                      </div>
                    </div>
                  )}
                </div>
              );
            })}
          </div>
        ) : (
          <div style={{ padding: "4px", fontSize: 11, color: "var(--color-text-secondary)", textAlign: "center" }}>
            Sin abonos registrados.
          </div>
        )}
      </div>
    </div>
  );
}
