import { useState, useEffect, useMemo } from "react";
import {
  crearOrdenServicio, actualizarOrdenServicio, cambiarEstadoOrden,
  obtenerOrdenServicio, listarOrdenesServicio, buscarOrdenesPorEquipo,
  historialMovimientosOrden, eliminarOrdenServicio,
  agregarImagenOrden, listarImagenesOrden, eliminarImagenOrden,
  cobrarOrdenServicio, imprimirOrdenServicioPdf,
  buscarClientes, listarUsuarios, buscarProductos,
} from "../services/api";
import type { OrdenServicio } from "../services/api";
import { useToast } from "../components/Toast";
import { useSesion } from "../contexts/SesionContext";

const ESTADOS = ["RECIBIDO", "DIAGNOSTICANDO", "EN_REPARACION", "ESPERANDO_REPUESTOS", "LISTO", "ENTREGADO"];
const ESTADOS_COLORS: Record<string, string> = {
  RECIBIDO: "#94a3b8",
  DIAGNOSTICANDO: "#f59e0b",
  EN_REPARACION: "#facc15",
  ESPERANDO_REPUESTOS: "#3b82f6",
  LISTO: "#86efac",
  ENTREGADO: "#22c55e",
  GARANTIA: "#a855f7",
  CANCELADO: "#ef4444",
};
const TIPOS_EQUIPO = [
  { value: "GENERAL", label: "General", icon: "🔧", color: "#94a3b8" },
  { value: "TECNOLOGIA", label: "Tecnología", icon: "💻", color: "#3b82f6" },
  { value: "AUTOMOTRIZ", label: "Automotriz", icon: "🚗", color: "#f59e0b" },
  { value: "ELECTRODOMESTICO", label: "Electrodoméstico", icon: "🔌", color: "#22c55e" },
];

const formNuevo = (): OrdenServicio => ({
  cliente_id: null,
  cliente_nombre: "",
  cliente_telefono: "",
  tipo_equipo: "GENERAL",
  equipo_descripcion: "",
  equipo_marca: "",
  equipo_modelo: "",
  equipo_serie: "",
  equipo_placa: "",
  equipo_kilometraje: undefined,
  equipo_kilometraje_proximo: undefined,
  accesorios: "",
  problema_reportado: "",
  presupuesto: 0,
  garantia_dias: 0,
  estado: "RECIBIDO",
});

export default function ServicioTecnicoPage() {
  const { toastExito, toastError } = useToast();
  const { esAdmin } = useSesion();
  const [vista, setVista] = useState<"kanban" | "lista">("kanban");
  const [ordenes, setOrdenes] = useState<OrdenServicio[]>([]);
  const [busqueda, setBusqueda] = useState("");
  const [filtroEstado, setFiltroEstado] = useState("");
  const [mostrarForm, setMostrarForm] = useState(false);
  const [form, setForm] = useState<OrdenServicio>(formNuevo());
  const [detalleId, setDetalleId] = useState<number | null>(null);
  const [detalle, setDetalle] = useState<OrdenServicio | null>(null);
  const [movimientos, setMovimientos] = useState<any[]>([]);
  const [imagenes, setImagenes] = useState<any[]>([]);
  const [tecnicos, setTecnicos] = useState<any[]>([]);
  const [busquedaCliente, setBusquedaCliente] = useState("");
  const [clientesResultados, setClientesResultados] = useState<any[]>([]);
  const [obsCambioEstado, setObsCambioEstado] = useState("");
  const [tipoImagen, setTipoImagen] = useState<"ANTES" | "DESPUES" | "GENERAL">("GENERAL");
  // Cobrar
  const [mostrarCobrar, setMostrarCobrar] = useState(false);
  const [cobroFormaPago, setCobroFormaPago] = useState("EFECTIVO");
  const [cobroMontoRecibido, setCobroMontoRecibido] = useState("");
  const [cobroRepuestos, setCobroRepuestos] = useState<any[]>([]);
  const [busquedaProducto, setBusquedaProducto] = useState("");
  const [productosResultados, setProductosResultados] = useState<any[]>([]);

  const cargar = async () => {
    try {
      if (busqueda.trim()) {
        const r = await buscarOrdenesPorEquipo(busqueda.trim());
        setOrdenes(r);
      } else {
        const r = await listarOrdenesServicio(filtroEstado || undefined);
        setOrdenes(r);
      }
    } catch (err) { toastError("Error: " + err); }
  };

  useEffect(() => { cargar(); }, [filtroEstado]);
  useEffect(() => {
    listarUsuarios().then((us: any[]) => setTecnicos(us.filter((u: any) => u.rol === "TECNICO" || u.rol === "ADMIN"))).catch(() => {});
  }, []);

  const ordenesPorEstado = useMemo(() => {
    const grupos: Record<string, OrdenServicio[]> = {};
    ESTADOS.forEach(e => { grupos[e] = []; });
    ordenes.forEach(o => {
      const e = o.estado || "RECIBIDO";
      if (grupos[e]) grupos[e].push(o);
    });
    return grupos;
  }, [ordenes]);

  const handleSubmit = async () => {
    if (!form.equipo_descripcion || !form.problema_reportado) {
      toastError("Equipo y problema son obligatorios");
      return;
    }
    try {
      if (form.id) {
        await actualizarOrdenServicio(form);
        toastExito("Orden actualizada");
      } else {
        await crearOrdenServicio(form);
        toastExito("Orden creada");
      }
      setMostrarForm(false);
      setForm(formNuevo());
      cargar();
    } catch (err) { toastError("Error: " + err); }
  };

  const abrirDetalle = async (id: number) => {
    try {
      const o = await obtenerOrdenServicio(id);
      setDetalleId(id);
      setDetalle(o);
      const movs = await historialMovimientosOrden(id);
      setMovimientos(movs);
      const imgs = await listarImagenesOrden(id);
      setImagenes(imgs);
    } catch (err) { toastError("Error: " + err); }
  };

  const handleCambiarEstado = async (nuevoEstado: string) => {
    if (!detalleId) return;
    try {
      await cambiarEstadoOrden(detalleId, nuevoEstado, obsCambioEstado || undefined);
      toastExito("Estado actualizado");
      setObsCambioEstado("");
      const o = await obtenerOrdenServicio(detalleId);
      setDetalle(o);
      const movs = await historialMovimientosOrden(detalleId);
      setMovimientos(movs);
      cargar();
    } catch (err) { toastError("Error: " + err); }
  };

  const handleSubirImagen = (e: React.ChangeEvent<HTMLInputElement>) => {
    if (!detalleId) return;
    const file = e.target.files?.[0];
    if (!file) return;
    if (file.size > 500000) { toastError("Imagen muy grande (max 500KB)"); return; }
    const reader = new FileReader();
    reader.onload = async () => {
      const b64 = (reader.result as string).split(",")[1];
      try {
        await agregarImagenOrden(detalleId, tipoImagen, b64);
        toastExito("Imagen subida");
        const imgs = await listarImagenesOrden(detalleId);
        setImagenes(imgs);
      } catch (err) { toastError("Error: " + err); }
    };
    reader.readAsDataURL(file);
  };

  const handleEliminarImagen = async (imagenId: number) => {
    try {
      await eliminarImagenOrden(imagenId);
      const imgs = await listarImagenesOrden(detalleId!);
      setImagenes(imgs);
    } catch (err) { toastError("Error: " + err); }
  };

  const handleCobrar = async () => {
    if (!detalleId) return;
    const monto = parseFloat(cobroMontoRecibido) || 0;
    try {
      await cobrarOrdenServicio(detalleId, cobroFormaPago, monto, cobroRepuestos);
      toastExito("Cobrado y entregado");
      setMostrarCobrar(false);
      setDetalleId(null);
      setDetalle(null);
      cargar();
    } catch (err) { toastError("Error: " + err); }
  };

  const totalCobro = useMemo(() => {
    const repuestos = cobroRepuestos.reduce((s, r) => s + r.cantidad * r.precio_unitario, 0);
    return (detalle?.monto_final || 0) + repuestos;
  }, [cobroRepuestos, detalle]);

  return (
    <>
      <div className="page-header">
        <h2>Servicio Técnico</h2>
        <div className="flex gap-2 items-center">
          <input className="input" style={{ width: 280, fontSize: 12 }}
            placeholder="Buscar placa, serie, cliente..."
            value={busqueda}
            onChange={(e) => setBusqueda(e.target.value)}
            onKeyDown={(e) => { if (e.key === "Enter") cargar(); }} />
          <button className="btn btn-outline" style={{ fontSize: 11 }} onClick={cargar}>Buscar</button>
          <button className="btn btn-primary" onClick={() => { setForm(formNuevo()); setMostrarForm(true); }}>+ Nueva Orden</button>
        </div>
      </div>
      <div className="page-body">
        {/* Tabs y filtros */}
        <div style={{ display: "flex", gap: 8, marginBottom: 12, alignItems: "center" }}>
          <button className={`btn ${vista === "kanban" ? "btn-primary" : "btn-outline"}`}
            style={{ fontSize: 12, padding: "4px 12px" }}
            onClick={() => setVista("kanban")}>📋 Kanban</button>
          <button className={`btn ${vista === "lista" ? "btn-primary" : "btn-outline"}`}
            style={{ fontSize: 12, padding: "4px 12px" }}
            onClick={() => setVista("lista")}>📃 Lista</button>
          <select className="input" style={{ width: 180, fontSize: 12 }}
            value={filtroEstado} onChange={(e) => setFiltroEstado(e.target.value)}>
            <option value="">Todos los estados</option>
            {ESTADOS.map(e => <option key={e} value={e}>{e}</option>)}
            <option value="GARANTIA">GARANTIA</option>
            <option value="CANCELADO">CANCELADO</option>
          </select>
        </div>

        {/* Kanban */}
        {vista === "kanban" && (
          <div style={{ display: "grid", gridTemplateColumns: `repeat(${ESTADOS.length}, 1fr)`, gap: 8, overflowX: "auto" }}>
            {ESTADOS.map(estado => (
              <div key={estado} style={{ minWidth: 200 }}>
                <div style={{ background: ESTADOS_COLORS[estado], color: "#fff", padding: "6px 10px", borderRadius: 6, fontSize: 11, fontWeight: 700, textAlign: "center", marginBottom: 6 }}>
                  {estado.replace(/_/g, " ")} ({ordenesPorEstado[estado]?.length || 0})
                </div>
                <div style={{ display: "flex", flexDirection: "column", gap: 6 }}>
                  {(ordenesPorEstado[estado] || []).map(o => {
                    const tipo = TIPOS_EQUIPO.find(t => t.value === o.tipo_equipo) || TIPOS_EQUIPO[0];
                    return (
                      <div key={o.id} className="card" style={{ padding: 8, cursor: "pointer", fontSize: 11 }}
                        onClick={() => abrirDetalle(o.id!)}>
                        <div style={{ fontWeight: 700, color: "var(--color-primary)" }}>{o.numero}</div>
                        <div style={{ fontSize: 11, marginTop: 2 }}>
                          <span style={{ background: tipo.color, color: "#fff", padding: "1px 5px", borderRadius: 3, fontSize: 9, marginRight: 4 }}>
                            {tipo.icon} {tipo.label}
                          </span>
                        </div>
                        <div style={{ fontWeight: 600, marginTop: 4 }}>{o.equipo_descripcion}</div>
                        <div style={{ color: "var(--color-text-secondary)" }}>{o.cliente_nombre || "-"}</div>
                        {o.tecnico_nombre && <div style={{ fontSize: 10, color: "var(--color-text-secondary)" }}>👤 {o.tecnico_nombre}</div>}
                        {o.fecha_promesa && <div style={{ fontSize: 10, color: "var(--color-warning)" }}>⏰ {o.fecha_promesa}</div>}
                      </div>
                    );
                  })}
                </div>
              </div>
            ))}
          </div>
        )}

        {/* Lista */}
        {vista === "lista" && (
          <div className="card">
            <table className="table">
              <thead>
                <tr>
                  <th>Número</th>
                  <th>Cliente</th>
                  <th>Equipo</th>
                  <th>Estado</th>
                  <th>Técnico</th>
                  <th>Ingreso</th>
                  <th>Total</th>
                  <th></th>
                </tr>
              </thead>
              <tbody>
                {ordenes.length === 0 ? (
                  <tr><td colSpan={8} className="text-center text-secondary" style={{ padding: 30 }}>Sin órdenes</td></tr>
                ) : ordenes.map(o => (
                  <tr key={o.id} style={{ cursor: "pointer" }} onClick={() => abrirDetalle(o.id!)}>
                    <td><strong>{o.numero}</strong></td>
                    <td>{o.cliente_nombre || "-"}</td>
                    <td>{o.equipo_descripcion}</td>
                    <td><span style={{ background: ESTADOS_COLORS[o.estado || "RECIBIDO"], color: "#fff", padding: "2px 8px", borderRadius: 4, fontSize: 10, fontWeight: 600 }}>{o.estado}</span></td>
                    <td>{o.tecnico_nombre || "-"}</td>
                    <td style={{ fontSize: 11 }}>{o.fecha_ingreso?.slice(0, 16) || "-"}</td>
                    <td className="text-right">${(o.monto_final || 0).toFixed(2)}</td>
                    <td><button className="btn btn-outline" style={{ fontSize: 10, padding: "2px 8px" }}>Ver</button></td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        )}
      </div>

      {/* Modal Form Crear/Editar */}
      {mostrarForm && (
        <div className="modal-overlay" onClick={() => setMostrarForm(false)}>
          <div className="modal-content" onClick={(e) => e.stopPropagation()} style={{ maxWidth: 700, maxHeight: "90vh", overflowY: "auto" }}>
            <div className="modal-header">
              <h3>{form.id ? "Editar Orden" : "Nueva Orden de Servicio"}</h3>
            </div>
            <div className="modal-body">
              {/* Cliente */}
              <div style={{ marginBottom: 12 }}>
                <label style={{ fontSize: 12, fontWeight: 600 }}>Cliente</label>
                <div style={{ display: "flex", gap: 6 }}>
                  <input className="input" placeholder="Nombre del cliente" style={{ flex: 1 }}
                    value={form.cliente_nombre || ""}
                    onChange={(e) => {
                      setForm({ ...form, cliente_nombre: e.target.value });
                      setBusquedaCliente(e.target.value);
                      if (e.target.value.length >= 2) buscarClientes(e.target.value).then(setClientesResultados).catch(() => {});
                    }} />
                  <input className="input" placeholder="Teléfono" style={{ width: 140 }}
                    value={form.cliente_telefono || ""}
                    onChange={(e) => setForm({ ...form, cliente_telefono: e.target.value })} />
                </div>
                {clientesResultados.length > 0 && busquedaCliente.length >= 2 && form.cliente_id == null && (
                  <div style={{ background: "var(--color-surface)", border: "1px solid var(--color-border)", borderRadius: 4, marginTop: 4, maxHeight: 100, overflow: "auto" }}>
                    {clientesResultados.slice(0, 5).map(c => (
                      <div key={c.id} style={{ padding: "4px 8px", cursor: "pointer", fontSize: 12 }}
                        onClick={() => {
                          setForm({ ...form, cliente_id: c.id, cliente_nombre: c.nombre, cliente_telefono: c.telefono || "" });
                          setClientesResultados([]);
                        }}>
                        {c.nombre} {c.identificacion && `(${c.identificacion})`}
                      </div>
                    ))}
                  </div>
                )}
              </div>

              {/* Equipo */}
              <div style={{ marginBottom: 12 }}>
                <label style={{ fontSize: 12, fontWeight: 600 }}>Tipo de Equipo</label>
                <div style={{ display: "flex", gap: 6, flexWrap: "wrap" }}>
                  {TIPOS_EQUIPO.map(t => (
                    <button key={t.value} type="button"
                      className={`btn ${form.tipo_equipo === t.value ? "btn-primary" : "btn-outline"}`}
                      style={{ fontSize: 11, padding: "4px 10px" }}
                      onClick={() => setForm({ ...form, tipo_equipo: t.value })}>
                      {t.icon} {t.label}
                    </button>
                  ))}
                </div>
              </div>

              <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: 8, marginBottom: 12 }}>
                <div>
                  <label style={{ fontSize: 12, fontWeight: 600 }}>Descripción del equipo *</label>
                  <input className="input" placeholder="Ej: Laptop HP Pavilion 15"
                    value={form.equipo_descripcion}
                    onChange={(e) => setForm({ ...form, equipo_descripcion: e.target.value })} />
                </div>
                <div>
                  <label style={{ fontSize: 12, fontWeight: 600 }}>Marca</label>
                  <input className="input" value={form.equipo_marca || ""}
                    onChange={(e) => setForm({ ...form, equipo_marca: e.target.value })} />
                </div>
                <div>
                  <label style={{ fontSize: 12, fontWeight: 600 }}>Modelo</label>
                  <input className="input" value={form.equipo_modelo || ""}
                    onChange={(e) => setForm({ ...form, equipo_modelo: e.target.value })} />
                </div>
                <div>
                  <label style={{ fontSize: 12, fontWeight: 600 }}>Serie</label>
                  <input className="input" value={form.equipo_serie || ""}
                    onChange={(e) => setForm({ ...form, equipo_serie: e.target.value })} />
                </div>
              </div>

              {form.tipo_equipo === "AUTOMOTRIZ" && (
                <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr 1fr", gap: 8, marginBottom: 12, padding: 8, background: "rgba(245, 158, 11, 0.1)", borderRadius: 6 }}>
                  <div>
                    <label style={{ fontSize: 12, fontWeight: 600 }}>Placa</label>
                    <input className="input" value={form.equipo_placa || ""}
                      onChange={(e) => setForm({ ...form, equipo_placa: e.target.value.toUpperCase() })} />
                  </div>
                  <div>
                    <label style={{ fontSize: 12, fontWeight: 600 }}>Kilometraje</label>
                    <input className="input" type="number" value={form.equipo_kilometraje || ""}
                      onChange={(e) => setForm({ ...form, equipo_kilometraje: parseInt(e.target.value) || undefined })} />
                  </div>
                  <div>
                    <label style={{ fontSize: 12, fontWeight: 600 }}>Próximo recomendado</label>
                    <input className="input" type="number" value={form.equipo_kilometraje_proximo || ""}
                      onChange={(e) => setForm({ ...form, equipo_kilometraje_proximo: parseInt(e.target.value) || undefined })} />
                  </div>
                </div>
              )}

              <div style={{ marginBottom: 12 }}>
                <label style={{ fontSize: 12, fontWeight: 600 }}>Accesorios incluidos</label>
                <input className="input" placeholder="Cargador, mochila, mouse..."
                  value={form.accesorios || ""}
                  onChange={(e) => setForm({ ...form, accesorios: e.target.value })} />
              </div>

              <div style={{ marginBottom: 12 }}>
                <label style={{ fontSize: 12, fontWeight: 600 }}>Problema reportado *</label>
                <textarea className="input" rows={3}
                  value={form.problema_reportado}
                  onChange={(e) => setForm({ ...form, problema_reportado: e.target.value })} />
              </div>

              <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr 1fr", gap: 8 }}>
                <div>
                  <label style={{ fontSize: 12, fontWeight: 600 }}>Técnico asignado</label>
                  <select className="input" value={form.tecnico_id || ""}
                    onChange={(e) => {
                      const tid = e.target.value ? parseInt(e.target.value) : null;
                      const t = tecnicos.find(x => x.id === tid);
                      setForm({ ...form, tecnico_id: tid, tecnico_nombre: t?.nombre || "" });
                    }}>
                    <option value="">Sin asignar</option>
                    {tecnicos.map(t => <option key={t.id} value={t.id}>{t.nombre}</option>)}
                  </select>
                </div>
                <div>
                  <label style={{ fontSize: 12, fontWeight: 600 }}>Presupuesto inicial</label>
                  <input className="input" type="number" step="0.01"
                    value={form.presupuesto || 0}
                    onChange={(e) => setForm({ ...form, presupuesto: parseFloat(e.target.value) || 0 })} />
                </div>
                <div>
                  <label style={{ fontSize: 12, fontWeight: 600 }}>Fecha promesa</label>
                  <input className="input" type="datetime-local" value={form.fecha_promesa || ""}
                    onChange={(e) => setForm({ ...form, fecha_promesa: e.target.value })} />
                </div>
              </div>
            </div>
            <div className="modal-footer">
              <button className="btn btn-outline" onClick={() => setMostrarForm(false)}>Cancelar</button>
              <button className="btn btn-primary" onClick={handleSubmit}>{form.id ? "Actualizar" : "Crear Orden"}</button>
            </div>
          </div>
        </div>
      )}

      {/* Modal Detalle */}
      {detalle && !mostrarCobrar && (
        <div className="modal-overlay" onClick={() => { setDetalle(null); setDetalleId(null); }}>
          <div className="modal-content" onClick={(e) => e.stopPropagation()} style={{ maxWidth: 800, maxHeight: "90vh", overflowY: "auto" }}>
            <div className="modal-header" style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
              <h3 style={{ margin: 0 }}>{detalle.numero} <span style={{ background: ESTADOS_COLORS[detalle.estado || "RECIBIDO"], color: "#fff", padding: "2px 10px", borderRadius: 4, fontSize: 11, marginLeft: 8 }}>{detalle.estado}</span></h3>
              <button onClick={() => { setDetalle(null); setDetalleId(null); }} style={{ background: "none", border: "none", fontSize: 20, cursor: "pointer", color: "var(--color-text)" }}>×</button>
            </div>
            <div className="modal-body">
              {/* Cliente y Equipo */}
              <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: 12, marginBottom: 16 }}>
                <div className="card" style={{ padding: 12 }}>
                  <div style={{ fontSize: 11, color: "var(--color-text-secondary)" }}>Cliente</div>
                  <div style={{ fontWeight: 700 }}>{detalle.cliente_nombre || "Sin cliente"}</div>
                  {detalle.cliente_telefono && <div style={{ fontSize: 12 }}>📞 {detalle.cliente_telefono}</div>}
                </div>
                <div className="card" style={{ padding: 12 }}>
                  <div style={{ fontSize: 11, color: "var(--color-text-secondary)" }}>Equipo</div>
                  <div style={{ fontWeight: 700 }}>{detalle.equipo_descripcion}</div>
                  <div style={{ fontSize: 11 }}>
                    {detalle.equipo_marca && `${detalle.equipo_marca} `}
                    {detalle.equipo_modelo}
                    {detalle.equipo_serie && ` · S/N: ${detalle.equipo_serie}`}
                    {detalle.equipo_placa && ` · Placa: ${detalle.equipo_placa}`}
                  </div>
                  {detalle.tipo_equipo === "AUTOMOTRIZ" && detalle.equipo_kilometraje && (
                    <div style={{ fontSize: 11, marginTop: 4 }}>
                      KM: {detalle.equipo_kilometraje}
                      {detalle.equipo_kilometraje_proximo && ` · Próximo: ${detalle.equipo_kilometraje_proximo}`}
                    </div>
                  )}
                </div>
              </div>

              {/* Cambiar estado */}
              <div style={{ marginBottom: 16 }}>
                <label style={{ fontSize: 12, fontWeight: 600 }}>Cambiar estado</label>
                <div style={{ display: "flex", gap: 4, flexWrap: "wrap", marginTop: 4 }}>
                  {[...ESTADOS, "GARANTIA", "CANCELADO"].map(e => (
                    <button key={e} className="btn"
                      style={{ fontSize: 10, padding: "3px 8px", background: detalle.estado === e ? ESTADOS_COLORS[e] : "var(--color-surface)", color: detalle.estado === e ? "#fff" : "var(--color-text)", border: "1px solid var(--color-border)" }}
                      onClick={() => handleCambiarEstado(e)}>
                      {e.replace(/_/g, " ")}
                    </button>
                  ))}
                </div>
                <input className="input" placeholder="Observación del cambio (opcional)" style={{ marginTop: 4, fontSize: 12 }}
                  value={obsCambioEstado} onChange={(e) => setObsCambioEstado(e.target.value)} />
              </div>

              {/* Diagnóstico, trabajo, observaciones (editables) */}
              <div style={{ marginBottom: 12 }}>
                <label style={{ fontSize: 12, fontWeight: 600 }}>Diagnóstico</label>
                <textarea className="input" rows={2}
                  value={detalle.diagnostico || ""}
                  onChange={(e) => setDetalle({ ...detalle, diagnostico: e.target.value })}
                  onBlur={() => actualizarOrdenServicio(detalle).then(() => toastExito("Guardado")).catch(err => toastError("" + err))} />
              </div>
              <div style={{ marginBottom: 12 }}>
                <label style={{ fontSize: 12, fontWeight: 600 }}>Trabajo realizado</label>
                <textarea className="input" rows={2}
                  value={detalle.trabajo_realizado || ""}
                  onChange={(e) => setDetalle({ ...detalle, trabajo_realizado: e.target.value })}
                  onBlur={() => actualizarOrdenServicio(detalle).then(() => toastExito("Guardado")).catch(err => toastError("" + err))} />
              </div>
              <div style={{ marginBottom: 12, display: "grid", gridTemplateColumns: "1fr 1fr", gap: 8 }}>
                <div>
                  <label style={{ fontSize: 12, fontWeight: 600 }}>Monto final</label>
                  <input className="input" type="number" step="0.01"
                    value={detalle.monto_final || 0}
                    onChange={(e) => setDetalle({ ...detalle, monto_final: parseFloat(e.target.value) || 0 })}
                    onBlur={() => actualizarOrdenServicio(detalle).then(() => toastExito("Guardado")).catch(err => toastError("" + err))} />
                </div>
                <div>
                  <label style={{ fontSize: 12, fontWeight: 600 }}>Garantía (días)</label>
                  <input className="input" type="number"
                    value={detalle.garantia_dias || 0}
                    onChange={(e) => setDetalle({ ...detalle, garantia_dias: parseInt(e.target.value) || 0 })}
                    onBlur={() => actualizarOrdenServicio(detalle).then(() => toastExito("Guardado")).catch(err => toastError("" + err))} />
                </div>
              </div>

              {/* Imágenes */}
              <div style={{ marginBottom: 16 }}>
                <label style={{ fontSize: 12, fontWeight: 600 }}>📷 Imágenes ({imagenes.length})</label>
                <div style={{ display: "flex", gap: 6, marginTop: 4 }}>
                  <select className="input" style={{ width: 120, fontSize: 11 }}
                    value={tipoImagen} onChange={(e) => setTipoImagen(e.target.value as any)}>
                    <option value="GENERAL">General</option>
                    <option value="ANTES">Antes</option>
                    <option value="DESPUES">Después</option>
                  </select>
                  <input type="file" accept="image/*" onChange={handleSubirImagen} style={{ fontSize: 11 }} />
                </div>
                {imagenes.length > 0 && (
                  <div style={{ display: "grid", gridTemplateColumns: "repeat(auto-fill, minmax(100px, 1fr))", gap: 6, marginTop: 8 }}>
                    {imagenes.map(img => (
                      <div key={img.id} style={{ position: "relative" }}>
                        <img src={`data:image/png;base64,${img.imagen_base64}`} style={{ width: "100%", height: 100, objectFit: "cover", borderRadius: 4 }} />
                        <span style={{ position: "absolute", top: 2, left: 2, background: "rgba(0,0,0,0.7)", color: "#fff", fontSize: 9, padding: "1px 4px", borderRadius: 3 }}>{img.tipo}</span>
                        <button onClick={() => handleEliminarImagen(img.id)} style={{ position: "absolute", top: 2, right: 2, background: "var(--color-danger)", color: "#fff", border: "none", borderRadius: "50%", width: 20, height: 20, cursor: "pointer", fontSize: 11 }}>×</button>
                      </div>
                    ))}
                  </div>
                )}
              </div>

              {/* Timeline */}
              {movimientos.length > 0 && (
                <div style={{ marginBottom: 16 }}>
                  <label style={{ fontSize: 12, fontWeight: 600 }}>Historial</label>
                  <div style={{ marginTop: 4, fontSize: 11 }}>
                    {movimientos.map(m => (
                      <div key={m.id} style={{ padding: "4px 8px", borderLeft: "2px solid var(--color-border)", marginLeft: 4 }}>
                        <span style={{ color: "var(--color-text-secondary)" }}>{m.fecha?.slice(0, 16)}</span>
                        {" · "}
                        {m.estado_anterior && <span>{m.estado_anterior} → </span>}
                        <strong>{m.estado_nuevo}</strong>
                        {m.observacion && <div style={{ fontSize: 10 }}>{m.observacion}</div>}
                        {m.usuario && <div style={{ fontSize: 10, color: "var(--color-text-secondary)" }}>👤 {m.usuario}</div>}
                      </div>
                    ))}
                  </div>
                </div>
              )}
            </div>
            <div className="modal-footer" style={{ display: "flex", gap: 6, justifyContent: "flex-end", flexWrap: "wrap" }}>
              {esAdmin && (
                <button className="btn btn-danger" style={{ fontSize: 11 }}
                  onClick={async () => {
                    if (!confirm("¿Eliminar esta orden?")) return;
                    try { await eliminarOrdenServicio(detalleId!); toastExito("Eliminada"); setDetalle(null); setDetalleId(null); cargar(); }
                    catch (err) { toastError("" + err); }
                  }}>Eliminar</button>
              )}
              <button className="btn btn-outline"
                onClick={() => imprimirOrdenServicioPdf(detalleId!).catch(err => toastError("" + err))}>📄 Imprimir PDF</button>
              {detalle.estado !== "ENTREGADO" && detalle.estado !== "CANCELADO" && (
                <button className="btn btn-success" onClick={() => { setMostrarCobrar(true); setCobroMontoRecibido(""); setCobroRepuestos([]); }}>💰 Cobrar</button>
              )}
            </div>
          </div>
        </div>
      )}

      {/* Modal Cobrar */}
      {mostrarCobrar && detalle && (
        <div className="modal-overlay" onClick={() => setMostrarCobrar(false)}>
          <div className="modal-content" onClick={(e) => e.stopPropagation()} style={{ maxWidth: 600 }}>
            <div className="modal-header"><h3>Cobrar orden {detalle.numero}</h3></div>
            <div className="modal-body">
              <div style={{ marginBottom: 12 }}>
                <strong>Servicio: ${(detalle.monto_final || 0).toFixed(2)}</strong>
              </div>

              <div style={{ marginBottom: 12 }}>
                <label style={{ fontSize: 12, fontWeight: 600 }}>Repuestos (opcional)</label>
                <div style={{ display: "flex", gap: 4 }}>
                  <input className="input" placeholder="Buscar producto..." value={busquedaProducto}
                    onChange={(e) => {
                      setBusquedaProducto(e.target.value);
                      if (e.target.value.length >= 2) buscarProductos(e.target.value).then(setProductosResultados).catch(() => {});
                    }} />
                </div>
                {productosResultados.length > 0 && busquedaProducto.length >= 2 && (
                  <div style={{ background: "var(--color-surface)", border: "1px solid var(--color-border)", borderRadius: 4, marginTop: 4, maxHeight: 100, overflow: "auto" }}>
                    {productosResultados.slice(0, 5).map((p: any) => (
                      <div key={p.id} style={{ padding: "4px 8px", cursor: "pointer", fontSize: 11 }}
                        onClick={() => {
                          setCobroRepuestos([...cobroRepuestos, { producto_id: p.id, nombre: p.nombre, cantidad: 1, precio_unitario: p.precio_venta }]);
                          setBusquedaProducto(""); setProductosResultados([]);
                        }}>
                        {p.nombre} - ${p.precio_venta}
                      </div>
                    ))}
                  </div>
                )}
                {cobroRepuestos.map((r, i) => (
                  <div key={i} style={{ display: "flex", gap: 4, alignItems: "center", marginTop: 4, fontSize: 11 }}>
                    <span style={{ flex: 1 }}>{r.nombre}</span>
                    <input className="input" type="number" style={{ width: 60 }} value={r.cantidad}
                      onChange={(e) => {
                        const nu = [...cobroRepuestos];
                        nu[i].cantidad = parseFloat(e.target.value) || 0;
                        setCobroRepuestos(nu);
                      }} />
                    <input className="input" type="number" step="0.01" style={{ width: 80 }} value={r.precio_unitario}
                      onChange={(e) => {
                        const nu = [...cobroRepuestos];
                        nu[i].precio_unitario = parseFloat(e.target.value) || 0;
                        setCobroRepuestos(nu);
                      }} />
                    <span style={{ width: 60, textAlign: "right" }}>${(r.cantidad * r.precio_unitario).toFixed(2)}</span>
                    <button className="btn btn-danger" style={{ fontSize: 10, padding: "2px 6px" }}
                      onClick={() => setCobroRepuestos(cobroRepuestos.filter((_, j) => j !== i))}>x</button>
                  </div>
                ))}
              </div>

              <div style={{ marginBottom: 12 }}>
                <label style={{ fontSize: 12, fontWeight: 600 }}>Forma de pago</label>
                <select className="input" value={cobroFormaPago} onChange={(e) => setCobroFormaPago(e.target.value)}>
                  <option value="EFECTIVO">Efectivo</option>
                  <option value="TRANSFER">Transferencia</option>
                  <option value="CREDITO">Crédito</option>
                </select>
              </div>

              <div style={{ marginBottom: 12 }}>
                <label style={{ fontSize: 12, fontWeight: 600 }}>Monto recibido</label>
                <input className="input" type="number" step="0.01" value={cobroMontoRecibido}
                  onChange={(e) => setCobroMontoRecibido(e.target.value)} />
              </div>

              <div style={{ padding: 12, background: "rgba(34, 197, 94, 0.1)", borderRadius: 6, fontSize: 16, fontWeight: 700 }}>
                TOTAL: ${totalCobro.toFixed(2)}
                {parseFloat(cobroMontoRecibido) > totalCobro && (
                  <div style={{ fontSize: 12, fontWeight: 400 }}>Cambio: ${(parseFloat(cobroMontoRecibido) - totalCobro).toFixed(2)}</div>
                )}
              </div>
            </div>
            <div className="modal-footer">
              <button className="btn btn-outline" onClick={() => setMostrarCobrar(false)}>Cancelar</button>
              <button className="btn btn-success" onClick={handleCobrar}>Confirmar Cobro</button>
            </div>
          </div>
        </div>
      )}
    </>
  );
}
