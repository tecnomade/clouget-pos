import { useState, useEffect, useMemo } from "react";
import {
  crearOrdenServicio, actualizarOrdenServicio, cambiarEstadoOrden,
  obtenerOrdenServicio, listarOrdenesServicio, buscarOrdenesPorEquipo,
  historialMovimientosOrden, eliminarOrdenServicio,
  agregarImagenOrden, listarImagenesOrden, eliminarImagenOrden,
  cobrarOrdenServicio, imprimirOrdenServicioPdf,
  buscarClientes, listarUsuarios, obtenerConfig,
  // v2.4.10 ST-2.5: catálogo
  stListarTiposEquipo, stListarMarcas, stListarModelos,
  stCrearTipoEquipo, stCrearMarca, stCrearModelo,
  // v2.4.11 ST-3: SRI lookup
  consultarIdentificacion,
} from "../services/api";
import type { OrdenServicio, StTipoEquipo, StMarca, StModelo } from "../services/api";
import { useToast } from "../components/Toast";
import { useSesion } from "../contexts/SesionContext";
import ModalConfigServicioTecnico from "../components/ModalConfigServicioTecnico";
import ModalHistorialServicioTecnico from "../components/ModalHistorialServicioTecnico";
import ComboCatalogoEquipo from "../components/ComboCatalogoEquipo";
import SeccionItemsAbonosOrden from "../components/SeccionItemsAbonosOrden";
// v2.4.13 ST-5: cancelar orden + nuevo cobro mixto
import { stCancelarOrden, listarCuentasBanco } from "../services/api";
import type { TotalOrden, AbonoServicio, PagoOrden } from "../services/api";
import type { CuentaBanco } from "../types";

const ESTADOS = ["RECIBIDO", "DIAGNOSTICANDO", "EN_REPARACION", "ESPERANDO_REPUESTOS", "LISTO", "ENTREGADO"];
// v2.4.22: estados "cerrados" — no permiten cambiar a otro estado porque
// arrastran inconsistencias: ENTREGADO/ENTREGADO_PARCIAL ya tienen venta
// generada + abonos APLICADOS; CANCELADA tiene abonos DEVUELTOS al cliente.
// Si se necesita reabrir, hay que anular la venta primero (eso es manual).
const ESTADOS_CERRADOS = ["ENTREGADO", "ENTREGADO_PARCIAL", "CANCELADA", "CANCELADO"];
const ESTADOS_COLORS: Record<string, string> = {
  RECIBIDO: "#94a3b8",
  DIAGNOSTICANDO: "#f59e0b",
  EN_REPARACION: "#facc15",
  ESPERANDO_REPUESTOS: "#3b82f6",
  LISTO: "#86efac",
  ENTREGADO: "#22c55e",
  ENTREGADO_PARCIAL: "#34d399",
  GARANTIA: "#a855f7",
  CANCELADA: "#ef4444",
  CANCELADO: "#ef4444",
};
const TIPOS_EQUIPO = [
  { value: "GENERAL", label: "General", icon: "🔧", color: "#94a3b8" },
  { value: "TECNOLOGIA", label: "Tecnología", icon: "💻", color: "#3b82f6" },
  { value: "AUTOMOTRIZ", label: "Automotriz", icon: "🚗", color: "#f59e0b" },
  { value: "ELECTRODOMESTICO", label: "Electrodoméstico", icon: "🔌", color: "#22c55e" },
];

// Labels adaptativas segun tipo de taller configurado.
// MIXTO mantiene el comportamiento actual (escoger por orden).
type TallerLabels = {
  titulo: string;
  ordenLabel: string;        // "Orden de Servicio" / "Orden de Trabajo" / "Orden de Reparacion"
  nuevaOrden: string;        // texto del boton crear
  equipoSingular: string;    // "Equipo" / "Vehiculo" / "Aparato"
  buscarPh: string;
  defaultTipoEquipo: string;
};
// v2.4.13: defaultTipoEquipo vacío — el user elige del catálogo (st_tipos_equipo)
// al crear nueva orden. Antes esto pre-cargaba strings hardcoded como "TECNOLOGIA"
// que terminaban duplicándose en el catálogo si el user clickeaba "+ Agregar".
const TALLER_LABELS: Record<string, TallerLabels> = {
  MIXTO:           { titulo: "Servicio Técnico",            ordenLabel: "Orden de Servicio",   nuevaOrden: "+ Nueva Orden",        equipoSingular: "Equipo",    buscarPh: "Buscar placa, serie, cliente...", defaultTipoEquipo: "" },
  GENERAL:         { titulo: "Servicio Técnico",            ordenLabel: "Orden de Servicio",   nuevaOrden: "+ Nueva Orden",        equipoSingular: "Equipo",    buscarPh: "Buscar serie, cliente...",        defaultTipoEquipo: "" },
  TECNOLOGIA:      { titulo: "Taller de Tecnología",        ordenLabel: "Orden de Reparación", nuevaOrden: "+ Nueva Reparación",   equipoSingular: "Equipo",    buscarPh: "Buscar modelo, serie, cliente...", defaultTipoEquipo: "" },
  AUTOMOTRIZ:      { titulo: "Taller Mecánico",             ordenLabel: "Orden de Trabajo",    nuevaOrden: "+ Nueva Orden de Trabajo", equipoSingular: "Vehículo", buscarPh: "Buscar placa, marca, cliente...", defaultTipoEquipo: "" },
  ELECTRODOMESTICO:{ titulo: "Servicio de Electrodomésticos", ordenLabel: "Orden de Servicio", nuevaOrden: "+ Nueva Orden",        equipoSingular: "Aparato",   buscarPh: "Buscar marca, serie, cliente...", defaultTipoEquipo: "" },
};

const formNuevo = (defaultTipoEquipo: string = "GENERAL"): OrdenServicio => ({
  cliente_id: null,
  cliente_nombre: "",
  cliente_telefono: "",
  tipo_equipo: defaultTipoEquipo,
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
  const [tipoTaller, setTipoTaller] = useState<string>("MIXTO");
  const labels = TALLER_LABELS[tipoTaller] || TALLER_LABELS.MIXTO;
  const tallerEsMixto = tipoTaller === "MIXTO";
  const [form, setForm] = useState<OrdenServicio>(formNuevo(labels.defaultTipoEquipo));
  const [detalleId, setDetalleId] = useState<number | null>(null);
  const [detalle, setDetalle] = useState<OrdenServicio | null>(null);
  const [movimientos, setMovimientos] = useState<any[]>([]);
  const [imagenes, setImagenes] = useState<any[]>([]);
  const [tecnicos, setTecnicos] = useState<any[]>([]);
  const [busquedaCliente, setBusquedaCliente] = useState("");
  const [clientesResultados, setClientesResultados] = useState<any[]>([]);
  const [obsCambioEstado, setObsCambioEstado] = useState("");
  const [tipoImagen, setTipoImagen] = useState<"ANTES" | "DESPUES" | "GENERAL">("GENERAL");
  // Cobrar (legacy: cobroFormaPago/cobroMontoRecibido/cobroRepuestos siguen existiendo
  // como fallback si la orden no tiene items en la nueva tabla orden_servicio_items)
  const [mostrarCobrar, setMostrarCobrar] = useState(false);
  const [cobroFormaPago] = useState("EFECTIVO");
  const [cobroMontoRecibido, setCobroMontoRecibido] = useState("");
  const [cobroRepuestos, setCobroRepuestos] = useState<any[]>([]);
  // v2.4.9 — ST-2: modales de configuración + historial
  const [mostrarConfig, setMostrarConfig] = useState(false);
  const [mostrarHistorial, setMostrarHistorial] = useState(false);
  // v2.4.10 — ST-2.5: catálogo cargado dinámicamente
  const [stTipos, setStTipos] = useState<StTipoEquipo[]>([]);
  const [stMarcas, setStMarcas] = useState<StMarca[]>([]);
  const [stModelos, setStModelos] = useState<StModelo[]>([]);
  // v2.4.11 — ST-3: búsqueda cliente por ced/RUC + SRI lookup
  const [busquedaIdentif, setBusquedaIdentif] = useState("");
  const [consultandoSri, setConsultandoSri] = useState(false);
  // v2.4.12 — ST-4/garantía: días de garantía a aplicar al cobrar + formato impresión
  const [cobroGarantiaDias, setCobroGarantiaDias] = useState("0");
  // v2.4.25: km de salida del vehículo (al entregar). Si se llena, recalcula próximo mantenimiento.
  const [cobroKmSalida, setCobroKmSalida] = useState("");
  const [formatoImpresion, setFormatoImpresion] = useState<"A4" | "TICKET_80">("A4");
  // v2.4.13 — ST-5: total de items + abonos sincronizados desde el componente embebido,
  // para usarlos en el modal de cobrar y validaciones.
  const [totalOrdenItems, setTotalOrdenItems] = useState<TotalOrden>({ subtotal_sin_iva: 0, subtotal_con_iva: 0, iva: 0, total: 0, cantidad_items: 0 });
  const [, setAbonosOrden] = useState<AbonoServicio[]>([]);
  const [totalHoldingOrden, setTotalHoldingOrden] = useState(0);
  const [bancosCobro, setBancosCobro] = useState<CuentaBanco[]>([]);
  // Pago mixto en cobro: lista de pagos (forma + monto + banco/ref opcionales)
  const [cobroPagos, setCobroPagos] = useState<PagoOrden[]>([{ forma_pago: "EFECTIVO", monto: 0 }]);
  // v2.4.14: cobranza parcial — permitir entregar el equipo aunque no se cubra todo el saldo
  const [permitirSaldoPendiente, setPermitirSaldoPendiente] = useState(false);

  // Tecla Esc cierra los drawers (form / detalle)
  useEffect(() => {
    if (!mostrarForm && !detalle) return;
    const handler = (e: KeyboardEvent) => {
      if (e.key !== "Escape") return;
      if (mostrarCobrar) return; // dejar que el modal cobrar maneje su propio cierre
      if (mostrarForm) setMostrarForm(false);
      else if (detalle) { setDetalle(null); setDetalleId(null); }
    };
    window.addEventListener("keydown", handler);
    return () => window.removeEventListener("keydown", handler);
  }, [mostrarForm, detalle, mostrarCobrar]);

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
    obtenerConfig().then((cfg: any) => {
      const t = (cfg.tipo_taller || "MIXTO").toUpperCase();
      setTipoTaller(TALLER_LABELS[t] ? t : "MIXTO");
    }).catch(() => {});
    // v2.4.10 ST-2.5: cargar catálogo de tipos al montar
    stListarTiposEquipo().then(setStTipos).catch(() => {});
    // v2.4.13 ST-5: bancos para cobro mixto y abonos por transferencia
    listarCuentasBanco().then(setBancosCobro).catch(() => {});
  }, []);

  // Cuando cambia tipo_equipo_id en form → cargar marcas
  useEffect(() => {
    if (form.tipo_equipo_id) {
      stListarMarcas(form.tipo_equipo_id).then(setStMarcas).catch(() => setStMarcas([]));
    } else {
      setStMarcas([]);
    }
  }, [form.tipo_equipo_id]);

  // Cuando cambia marca_id → cargar modelos
  useEffect(() => {
    if (form.marca_id) {
      stListarModelos(form.marca_id).then(setStModelos).catch(() => setStModelos([]));
    } else {
      setStModelos([]);
    }
  }, [form.marca_id]);

  // Tipo seleccionado del catálogo (para flags requiere_*)
  const tipoSeleccionado = useMemo(() => stTipos.find(t => t.id === form.tipo_equipo_id), [stTipos, form.tipo_equipo_id]);

  const ordenesPorEstado = useMemo(() => {
    const grupos: Record<string, OrdenServicio[]> = {};
    ESTADOS.forEach(e => { grupos[e] = []; });
    ordenes.forEach(o => {
      const e = o.estado || "RECIBIDO";
      if (grupos[e]) grupos[e].push(o);
    });
    return grupos;
  }, [ordenes]);

  /** v2.4.11 ST-3: Consulta cédula/RUC en el SRI y crea/vincula cliente al form. */
  const consultarSriHandler = async () => {
    if (busquedaIdentif.length < 8) {
      toastError("Ingresa una cédula (10 dígitos) o RUC (13 dígitos) válida");
      return;
    }
    setConsultandoSri(true);
    try {
      const cliente = await consultarIdentificacion(busquedaIdentif);
      setForm({
        ...form,
        cliente_id: cliente.id ?? null,
        cliente_nombre: cliente.nombre,
        cliente_telefono: cliente.telefono || "",
      });
      setClientesResultados([]);
      toastExito(`Cliente cargado del SRI: ${cliente.nombre}`);
    } catch (err: any) {
      toastError(err?.toString() || "No se encontró información en el SRI");
    } finally {
      setConsultandoSri(false);
    }
  };

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
      setForm(formNuevo(labels.defaultTipoEquipo));
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
    const garantia = parseInt(cobroGarantiaDias) || 0;
    // v2.4.13: pago mixto. Si no hay pagos cargados, usar legacy (forma + monto único).
    const pagosFiltrados = cobroPagos.filter(p => p.monto > 0);
    const saldoPend = Math.max(totalOrdenItems.total - totalHoldingOrden - cobroPagos.reduce((s, p) => s + p.monto, 0), 0);
    try {
      // v2.4.25: si se ingresó km de salida, actualizar la orden ANTES del cobro.
      // El backend recalcula próximo = salida + intervalo automáticamente.
      const kmSalida = parseInt(cobroKmSalida) || undefined;
      if (kmSalida && detalle) {
        await actualizarOrdenServicio({ ...detalle, equipo_kilometraje_salida: kmSalida });
      }
      if (pagosFiltrados.length > 0) {
        await cobrarOrdenServicio(detalleId, {
          pagos: pagosFiltrados,
          garantiaDias: garantia,
          permitirSaldoPendiente: permitirSaldoPendiente,
        });
      } else {
        const monto = parseFloat(cobroMontoRecibido) || 0;
        await cobrarOrdenServicio(detalleId, {
          formaPago: cobroFormaPago,
          montoRecibido: monto,
          itemsRepuestos: cobroRepuestos,
          garantiaDias: garantia,
          permitirSaldoPendiente: permitirSaldoPendiente,
        });
      }
      const msgGarantia = garantia > 0 ? ` · 🛡 Garantía ${garantia} días` : "";
      const msgSaldo = saldoPend > 0.001 && permitirSaldoPendiente ? ` · 💰 Saldo pendiente $${saldoPend.toFixed(2)}` : "";
      toastExito(`Entregado${msgGarantia}${msgSaldo}`);
      setMostrarCobrar(false);
      setDetalleId(null);
      setDetalle(null);
      cargar();
    } catch (err) { toastError("Error: " + err); }
  };

  // v2.4.13: total a cobrar = total de items − abonos en HOLDING
  const saldoCobro = useMemo(() => {
    return Math.max(totalOrdenItems.total - totalHoldingOrden, 0);
  }, [totalOrdenItems, totalHoldingOrden]);

  const totalPagosCobro = useMemo(() => cobroPagos.reduce((s, p) => s + (p.monto || 0), 0), [cobroPagos]);


  const handleCancelarOrden = async () => {
    if (!detalleId) return;
    const obs = prompt("¿Por qué se cancela la orden? (opcional)");
    // prompt regresa "" si user pulsa OK sin texto, null si cancela
    if (obs === null) return;
    if (!confirm("¿Cancelar esta orden? Los abonos en holding se devolverán automáticamente al cliente.")) return;
    try {
      const r = await stCancelarOrden(detalleId, obs.trim() || null);
      if (r.abonos_devueltos > 0) {
        toastExito(`Orden cancelada · ${r.abonos_devueltos} abono(s) devuelto(s) ($${r.monto_devuelto.toFixed(2)})`);
      } else {
        toastExito("Orden cancelada");
      }
      setDetalleId(null);
      setDetalle(null);
      cargar();
    } catch (err) { toastError("" + err); }
  };

  return (
    <>
      <div className="page-header">
        <h2>{labels.titulo}</h2>
        <div className="flex gap-2 items-center">
          {/* v2.4.14: input de busqueda con boton X para limpiar (sin tener que borrar a mano) */}
          <div style={{ position: "relative", display: "inline-block" }}>
            <input className="input" style={{ width: 280, fontSize: 12, paddingRight: busqueda ? 26 : 8 }}
              placeholder={labels.buscarPh}
              value={busqueda}
              onChange={(e) => setBusqueda(e.target.value)}
              onKeyDown={(e) => { if (e.key === "Enter") cargar(); }} />
            {busqueda && (
              <button type="button"
                onClick={() => { setBusqueda(""); setTimeout(cargar, 0); }}
                title="Limpiar búsqueda"
                style={{
                  position: "absolute", right: 4, top: "50%", transform: "translateY(-50%)",
                  background: "transparent", border: "none", cursor: "pointer",
                  fontSize: 14, color: "var(--color-text-secondary)", padding: "0 6px",
                }}>×</button>
            )}
          </div>
          <button className="btn btn-outline" style={{ fontSize: 11 }} onClick={cargar}>Buscar</button>
          {/* v2.4.9 ST-2: nuevos botones */}
          <button className="btn btn-outline" style={{ fontSize: 12 }} onClick={() => setMostrarHistorial(true)}>📜 Historial</button>
          <button className="btn btn-outline" style={{ fontSize: 12 }} onClick={() => setMostrarConfig(true)}>⚙ Configuración</button>
          <button className="btn btn-primary" onClick={() => { setForm(formNuevo(labels.defaultTipoEquipo)); setMostrarForm(true); }}>{labels.nuevaOrden}</button>
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

      {/* Drawer lateral derecho: Form Crear/Editar */}
      {mostrarForm && (
        <>
          <div onClick={() => setMostrarForm(false)} style={{
            position: "fixed", inset: 0, background: "rgba(0,0,0,0.4)",
            zIndex: 999, animation: "stFadeIn 0.15s ease-out",
          }} />
          <div onClick={(e) => e.stopPropagation()} style={{
            position: "fixed", top: 0, right: 0, bottom: 0,
            width: "min(96vw, 760px)",
            background: "var(--color-bg)",
            boxShadow: "-4px 0 20px rgba(0,0,0,0.25)",
            zIndex: 1000, display: "flex", flexDirection: "column",
            animation: "stSlideInRight 0.2s ease-out",
          }}>
            <style>{`
              @keyframes stSlideInRight { from { transform: translateX(100%); } to { transform: translateX(0); } }
              @keyframes stFadeIn { from { opacity: 0; } to { opacity: 1; } }
            `}</style>
            <div style={{
              display: "flex", justifyContent: "space-between", alignItems: "center",
              padding: "12px 20px", borderBottom: "1px solid var(--color-border)",
              flexShrink: 0,
            }}>
              <h3 style={{ margin: 0 }}>{form.id ? `Editar ${labels.ordenLabel}` : `Nueva ${labels.ordenLabel}`}</h3>
              <button
                onClick={() => setMostrarForm(false)}
                style={{ background: "none", border: "none", cursor: "pointer", fontSize: 24, color: "var(--color-text-secondary)", padding: "0 8px", lineHeight: 1 }}
                title="Cerrar (Esc)">×</button>
            </div>
            <div style={{ flex: 1, overflowY: "auto", padding: "16px 20px" }}>
              {/* Cliente — v2.4.11 ST-3: búsqueda por ced/RUC + lookup SRI */}
              <div style={{ marginBottom: 12 }}>
                <label style={{ fontSize: 12, fontWeight: 600 }}>
                  Cliente
                  {form.cliente_id && (
                    <span style={{ marginLeft: 8, fontSize: 10, padding: "1px 6px", borderRadius: 4, background: "var(--color-success)", color: "#fff", fontWeight: 700 }}>
                      ✓ vinculado al cliente #{form.cliente_id}
                    </span>
                  )}
                </label>
                <div style={{ display: "grid", gridTemplateColumns: "1.5fr 1fr 1fr auto", gap: 6 }}>
                  <input className="input" placeholder="Nombre del cliente"
                    value={form.cliente_nombre || ""}
                    onChange={(e) => {
                      setForm({ ...form, cliente_nombre: e.target.value, cliente_id: null });
                      setBusquedaCliente(e.target.value);
                      if (e.target.value.length >= 2) buscarClientes(e.target.value).then(setClientesResultados).catch(() => {});
                    }} />
                  <input className="input" placeholder="Cédula / RUC"
                    value={busquedaIdentif}
                    onChange={async (e) => {
                      const v = e.target.value.replace(/[^0-9]/g, "");
                      setBusquedaIdentif(v);
                      // Auto-lookup local cuando llega a 10 (ced) o 13 (RUC)
                      if (v.length === 10 || v.length === 13) {
                        try {
                          const r = await buscarClientes(v);
                          const exacto = r.find((c: any) => c.identificacion === v);
                          if (exacto) {
                            setForm({ ...form, cliente_id: exacto.id, cliente_nombre: exacto.nombre, cliente_telefono: exacto.telefono || "" });
                            setClientesResultados([]);
                            toastExito(`Cliente cargado: ${exacto.nombre}`);
                          }
                        } catch {}
                      }
                    }}
                    onKeyDown={async (e) => {
                      if (e.key === "Enter" && busquedaIdentif.length >= 8) {
                        await consultarSriHandler();
                      }
                    }} />
                  <input className="input" placeholder="Teléfono"
                    value={form.cliente_telefono || ""}
                    onChange={(e) => setForm({ ...form, cliente_telefono: e.target.value })} />
                  <button type="button"
                    className="btn btn-outline"
                    style={{ fontSize: 11, padding: "0 12px", whiteSpace: "nowrap" }}
                    onClick={consultarSriHandler}
                    disabled={consultandoSri || busquedaIdentif.length < 8}
                    title="Consulta los datos del contribuyente en el SRI (Ecuador) y crea/vincula el cliente automáticamente">
                    {consultandoSri ? "⏳" : "🔍 SRI"}
                  </button>
                </div>
                {clientesResultados.length > 0 && busquedaCliente.length >= 2 && form.cliente_id == null && (
                  <div style={{ background: "var(--color-surface)", border: "1px solid var(--color-border)", borderRadius: 4, marginTop: 4, maxHeight: 120, overflow: "auto" }}>
                    {clientesResultados.slice(0, 5).map(c => (
                      <div key={c.id} style={{ padding: "4px 8px", cursor: "pointer", fontSize: 12 }}
                        onClick={() => {
                          setForm({ ...form, cliente_id: c.id, cliente_nombre: c.nombre, cliente_telefono: c.telefono || "" });
                          setBusquedaIdentif(c.identificacion || "");
                          setClientesResultados([]);
                        }}>
                        {c.nombre} {c.identificacion && <span style={{ color: "var(--color-text-muted)" }}>({c.identificacion})</span>}
                      </div>
                    ))}
                  </div>
                )}
              </div>

              {/* Tipo de Equipo (botones legacy) - solo si NO hay catálogo (fallback)
                  v2.4.10 ST-2.5: cuando el catálogo tiene tipos, se usa ComboCatalogoEquipo abajo */}
              {tallerEsMixto && stTipos.length === 0 && (
                <div style={{ marginBottom: 12 }}>
                  <label style={{ fontSize: 12, fontWeight: 600 }}>Tipo de {labels.equipoSingular}</label>
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
              )}

              {/* v2.4.10 ST-2.5: si hay catálogo, usar selector tipo del catálogo en lugar de botones hardcoded */}
              {stTipos.length > 0 && (
                <div style={{ marginBottom: 12 }}>
                  <ComboCatalogoEquipo
                    label={`Tipo de ${labels.equipoSingular.toLowerCase()}`}
                    valorTexto={form.tipo_equipo || ""}
                    valorId={form.tipo_equipo_id || null}
                    opciones={stTipos.filter(t => t.id != null).map(t => ({ id: t.id!, nombre: `${t.icono} ${t.nombre}` }))}
                    onChange={(id, nombre) => {
                      // Limpiamos marca/modelo al cambiar tipo
                      const tipoFromId = stTipos.find(t => t.id === id);
                      setForm({
                        ...form,
                        tipo_equipo_id: id,
                        tipo_equipo: tipoFromId?.nombre || nombre.replace(/^[^\s]+\s/, ''), // sin emoji
                        marca_id: null, equipo_marca: "",
                        modelo_id: null, equipo_modelo: "",
                      });
                    }}
                    onCrearNuevo={async (nombre) => {
                      const id = await stCrearTipoEquipo({
                        nombre, icono: "🔧", requiere_placa: false, requiere_kilometraje: false,
                        requiere_serie: false, orden: 99, activo: true,
                      });
                      const fresh = await stListarTiposEquipo();
                      setStTipos(fresh);
                      return id;
                    }}
                    placeholder="Elige un tipo o escribe uno nuevo (Vehículo, Computadora...)"
                    required
                  />
                </div>
              )}

              <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: 8, marginBottom: 12 }}>
                <div>
                  <label style={{ fontSize: 12, fontWeight: 600 }}>Descripción del {labels.equipoSingular.toLowerCase()} *</label>
                  <input className="input" placeholder={
                      tipoSeleccionado?.nombre.toLowerCase().includes("veh") ? "Ej: Toyota Hilux 2020" :
                      tipoSeleccionado?.nombre.toLowerCase().includes("electr") ? "Ej: Refrigeradora LG 250L" :
                      "Ej: Laptop HP Pavilion 15"
                    }
                    value={form.equipo_descripcion}
                    onChange={(e) => setForm({ ...form, equipo_descripcion: e.target.value })} />
                </div>
                <div>
                  {/* v2.4.10 ST-2.5 / v2.4.13: Marca con autocomplete del catálogo.
                      Jerarquía estricta: requiere tipo seleccionado antes (sino se mezclarían
                      modelos de marcas que no corresponden, ej: Latitude bajo Lenovo). */}
                  <ComboCatalogoEquipo
                    label="Marca"
                    valorTexto={form.equipo_marca || ""}
                    valorId={form.marca_id || null}
                    opciones={stMarcas.filter(m => m.id != null).map(m => ({ id: m.id!, nombre: m.nombre }))}
                    onChange={(id, nombre) => {
                      setForm({ ...form, marca_id: id, equipo_marca: nombre, modelo_id: null, equipo_modelo: "" });
                    }}
                    onCrearNuevo={form.tipo_equipo_id ? async (nombre) => {
                      const id = await stCrearMarca({ tipo_equipo_id: form.tipo_equipo_id!, nombre, activo: true });
                      const fresh = await stListarMarcas(form.tipo_equipo_id!);
                      setStMarcas(fresh);
                      return id;
                    } : undefined}
                    disabled={!form.tipo_equipo_id}
                    placeholder={form.tipo_equipo_id
                      ? (stMarcas.length > 0 ? "Elige una marca o escribe una nueva" : "Escribe una marca y agrégala")
                      : "Elige primero un tipo de equipo"}
                  />
                </div>
                <div>
                  {/* v2.4.10 ST-2.5 / v2.4.13: Modelo, requiere marca seleccionada del catálogo. */}
                  <ComboCatalogoEquipo
                    label="Modelo"
                    valorTexto={form.equipo_modelo || ""}
                    valorId={form.modelo_id || null}
                    opciones={stModelos.filter(m => m.id != null).map(m => ({
                      id: m.id!,
                      nombre: m.nombre + (m.anio_desde ? ` (${m.anio_desde}${m.anio_hasta ? `–${m.anio_hasta}` : ''})` : ''),
                    }))}
                    onChange={(id, nombre) => {
                      setForm({ ...form, modelo_id: id, equipo_modelo: nombre.replace(/\s*\(\d{4}.*\)$/, '') });
                    }}
                    onCrearNuevo={form.marca_id ? async (nombre) => {
                      const id = await stCrearModelo({ marca_id: form.marca_id!, nombre, anio_desde: null, anio_hasta: null, activo: true });
                      const fresh = await stListarModelos(form.marca_id!);
                      setStModelos(fresh);
                      return id;
                    } : undefined}
                    disabled={!form.marca_id}
                    placeholder={form.marca_id
                      ? (stModelos.length > 0 ? "Elige un modelo o escribe uno nuevo" : "Escribe un modelo y agrégalo")
                      : "Elige primero una marca"}
                  />
                </div>
                <div>
                  <label style={{ fontSize: 12, fontWeight: 600 }}>
                    {tipoSeleccionado?.nombre.toLowerCase().includes("veh") ? "Chasis / VIN" : "Serie"}
                    {tipoSeleccionado?.requiere_serie && <span style={{ color: "var(--color-danger)" }}> *</span>}
                  </label>
                  <input className="input" value={form.equipo_serie || ""}
                    onChange={(e) => setForm({ ...form, equipo_serie: e.target.value })} />
                </div>
              </div>

              {/* v2.4.10 ST-2.5: campos placa/km solo si el tipo del catálogo los requiere */}
              {(tipoSeleccionado?.requiere_placa || tipoSeleccionado?.requiere_kilometraje) && (
                <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr 1fr", gap: 8, marginBottom: 12, padding: 8, background: "rgba(245, 158, 11, 0.1)", borderRadius: 6 }}>
                  {tipoSeleccionado.requiere_placa && (
                    <div>
                      <label style={{ fontSize: 12, fontWeight: 600 }}>Placa <span style={{ color: "var(--color-danger)" }}>*</span></label>
                      <input className="input" value={form.equipo_placa || ""}
                        onChange={(e) => setForm({ ...form, equipo_placa: e.target.value.toUpperCase() })} />
                    </div>
                  )}
                  {tipoSeleccionado.requiere_kilometraje && (
                    <>
                      <div>
                        <label style={{ fontSize: 12, fontWeight: 600 }}>Kilometraje actual</label>
                        <input className="input" type="number" value={form.equipo_kilometraje || ""}
                          onChange={(e) => {
                            const km = parseInt(e.target.value) || undefined;
                            const intervalo = form.equipo_kilometraje_intervalo;
                            // Auto-calcular próximo si hay intervalo
                            const proximo = (km && intervalo) ? km + intervalo : form.equipo_kilometraje_proximo;
                            setForm({ ...form, equipo_kilometraje: km, equipo_kilometraje_proximo: proximo });
                          }} />
                      </div>
                      <div>
                        <label style={{ fontSize: 12, fontWeight: 600 }} title="Cada cuántos km se recomienda mantenimiento">
                          Cada (km)
                        </label>
                        <input className="input" type="number" value={form.equipo_kilometraje_intervalo || ""}
                          placeholder="Ej: 5000"
                          onChange={(e) => {
                            const intervalo = parseInt(e.target.value) || undefined;
                            const km = form.equipo_kilometraje;
                            const proximo = (km && intervalo) ? km + intervalo : form.equipo_kilometraje_proximo;
                            setForm({ ...form, equipo_kilometraje_intervalo: intervalo, equipo_kilometraje_proximo: proximo });
                          }} />
                      </div>
                      <div>
                        <label style={{ fontSize: 12, fontWeight: 600 }}>Próximo (auto)</label>
                        <input className="input" type="number" value={form.equipo_kilometraje_proximo || ""}
                          onChange={(e) => setForm({ ...form, equipo_kilometraje_proximo: parseInt(e.target.value) || undefined })}
                          style={{ background: "var(--color-surface-alt)" }}
                          title="Calculado automáticamente. Editable si necesitas un valor distinto." />
                      </div>
                    </>
                  )}
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

              {/* v2.4.24: campo Garantía en el form de creación, así al cobrar
                  ya viene precargado el valor que el usuario quiere por defecto. */}
              <div style={{ marginTop: 8 }}>
                <label style={{ fontSize: 12, fontWeight: 600 }}>🛡 Garantía del trabajo (días)</label>
                <div style={{ display: "flex", gap: 6, alignItems: "center", marginTop: 4, flexWrap: "wrap" }}>
                  <input className="input" type="number" min="0" max="365"
                    style={{ width: 90 }}
                    value={form.garantia_dias || 0}
                    onChange={(e) => setForm({ ...form, garantia_dias: parseInt(e.target.value) || 0 })} />
                  <span style={{ fontSize: 11, color: "var(--color-text-muted)" }}>días</span>
                  {[0, 7, 15, 30, 60, 90, 180].map(d => (
                    <button key={d} type="button"
                      onClick={() => setForm({ ...form, garantia_dias: d })}
                      style={{
                        padding: "2px 8px", fontSize: 10, cursor: "pointer",
                        background: (form.garantia_dias || 0) === d ? "var(--color-primary)" : "transparent",
                        color: (form.garantia_dias || 0) === d ? "#fff" : "var(--color-text)",
                        border: "1px solid var(--color-border)", borderRadius: 4,
                      }}>
                      {d === 0 ? "Sin" : `${d}d`}
                    </button>
                  ))}
                </div>
                <div style={{ fontSize: 10, color: "var(--color-text-muted)", marginTop: 2 }}>
                  Este valor se precarga al cobrar la orden y se imprime en el comprobante.
                </div>
              </div>
            </div>
            <div style={{
              padding: "10px 20px", borderTop: "1px solid var(--color-border)",
              display: "flex", justifyContent: "flex-end", gap: 8, flexShrink: 0,
            }}>
              <button className="btn btn-outline" onClick={() => setMostrarForm(false)}>Cancelar</button>
              <button className="btn btn-primary" onClick={handleSubmit}>{form.id ? "Actualizar" : `Crear ${labels.ordenLabel}`}</button>
            </div>
          </div>
        </>
      )}

      {/* Drawer lateral derecho: Detalle de la orden */}
      {detalle && !mostrarCobrar && (
        <>
          <div onClick={() => { setDetalle(null); setDetalleId(null); }} style={{
            position: "fixed", inset: 0, background: "rgba(0,0,0,0.4)",
            zIndex: 999, animation: "stFadeIn 0.15s ease-out",
          }} />
          <div onClick={(e) => e.stopPropagation()} style={{
            position: "fixed", top: 0, right: 0, bottom: 0,
            width: "min(96vw, 900px)",
            background: "var(--color-bg)",
            boxShadow: "-4px 0 20px rgba(0,0,0,0.25)",
            zIndex: 1000, display: "flex", flexDirection: "column",
            animation: "stSlideInRight 0.2s ease-out",
          }}>
            <div style={{
              display: "flex", justifyContent: "space-between", alignItems: "center",
              padding: "12px 20px", borderBottom: "1px solid var(--color-border)",
              flexShrink: 0,
            }}>
              <h3 style={{ margin: 0 }}>{detalle.numero} <span style={{ background: ESTADOS_COLORS[detalle.estado || "RECIBIDO"], color: "#fff", padding: "2px 10px", borderRadius: 4, fontSize: 11, marginLeft: 8 }}>{detalle.estado}</span></h3>
              <button onClick={() => { setDetalle(null); setDetalleId(null); }} style={{ background: "none", border: "none", fontSize: 24, cursor: "pointer", color: "var(--color-text-secondary)", padding: "0 8px", lineHeight: 1 }} title="Cerrar (Esc)">×</button>
            </div>
            <div style={{ flex: 1, overflowY: "auto", padding: "16px 20px" }}>
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

              {/* Cambiar estado — v2.4.22: bloqueado si orden está cerrada
                  (entregada/cancelada) porque arrastraría inconsistencias con
                  la venta generada y los abonos APLICADOS/DEVUELTOS. */}
              {(() => {
                const cerrada = ESTADOS_CERRADOS.includes(detalle.estado || "");
                if (cerrada) {
                  return (
                    <div style={{ marginBottom: 16, padding: 10, background: "var(--color-surface-alt)", borderRadius: 6, border: "1px solid var(--color-border)" }}>
                      <div style={{ fontSize: 12, fontWeight: 600, color: "var(--color-text)" }}>
                        🔒 Estado: <span style={{ color: ESTADOS_COLORS[detalle.estado || ""] || "var(--color-text)" }}>{(detalle.estado || "").replace(/_/g, " ")}</span>
                      </div>
                      <div style={{ fontSize: 11, color: "var(--color-text-secondary)", marginTop: 4, lineHeight: 1.4 }}>
                        Esta orden ya está cerrada. {detalle.estado === "CANCELADA" || detalle.estado === "CANCELADO"
                          ? "Los abonos en holding (si había) se devolvieron al cliente."
                          : `La venta vinculada (#${detalle.venta_id || "?"}) está registrada y los abonos pasaron a APLICADOS.`}
                        {" "}No se puede cambiar de estado para evitar inconsistencias contables. Si necesitas reabrirla, anula la venta primero desde Ventas del Día.
                      </div>
                    </div>
                  );
                }
                return (
                  <div style={{ marginBottom: 16 }}>
                    <label style={{ fontSize: 12, fontWeight: 600 }}>Cambiar estado</label>
                    <div style={{ display: "flex", gap: 4, flexWrap: "wrap", marginTop: 4 }}>
                      {ESTADOS.filter(e => e !== "ENTREGADO").map(e => (
                        <button key={e} className="btn"
                          style={{ fontSize: 10, padding: "3px 8px", background: detalle.estado === e ? ESTADOS_COLORS[e] : "var(--color-surface)", color: detalle.estado === e ? "#fff" : "var(--color-text)", border: "1px solid var(--color-border)" }}
                          onClick={() => handleCambiarEstado(e)}>
                          {e.replace(/_/g, " ")}
                        </button>
                      ))}
                      <button className="btn"
                        style={{ fontSize: 10, padding: "3px 8px", background: detalle.estado === "GARANTIA" ? ESTADOS_COLORS.GARANTIA : "var(--color-surface)", color: detalle.estado === "GARANTIA" ? "#fff" : "var(--color-text)", border: "1px solid var(--color-border)" }}
                        onClick={() => handleCambiarEstado("GARANTIA")}>GARANTIA</button>
                    </div>
                    <input className="input" placeholder="Observación del cambio (opcional)" style={{ marginTop: 4, fontSize: 12 }}
                      value={obsCambioEstado} onChange={(e) => setObsCambioEstado(e.target.value)} />
                    <div style={{ fontSize: 10, color: "var(--color-text-muted)", marginTop: 4 }}>
                      Para entregar la orden usa "💰 Cobrar" abajo. Para cancelar, "🚫 Cancelar orden".
                    </div>
                  </div>
                );
              })()}

              {/* v2.4.20: cambiar técnico asignado en cualquier momento (post-creación) */}
              <div style={{ marginBottom: 12 }}>
                <label style={{ fontSize: 12, fontWeight: 600 }}>👤 Técnico asignado</label>
                <select className="input" style={{ fontSize: 12 }}
                  value={detalle.tecnico_id || ""}
                  onChange={(e) => {
                    const tid = e.target.value ? parseInt(e.target.value) : null;
                    const t = tecnicos.find(x => x.id === tid);
                    const nuevoDetalle = { ...detalle, tecnico_id: tid, tecnico_nombre: t?.nombre || null };
                    setDetalle(nuevoDetalle);
                    actualizarOrdenServicio(nuevoDetalle)
                      .then(() => toastExito(tid ? `Asignada a ${t?.nombre}` : "Sin asignar"))
                      .catch(err => toastError("" + err));
                  }}>
                  <option value="">Sin asignar</option>
                  {tecnicos.map(t => <option key={t.id} value={t.id}>{t.nombre}</option>)}
                </select>
                {detalle.tecnico_nombre && (
                  <div style={{ fontSize: 10, color: "var(--color-text-secondary)", marginTop: 2 }}>
                    Actualmente: <strong>{detalle.tecnico_nombre}</strong>
                  </div>
                )}
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
              {/* v2.4.13 ST-5: Items presupuestados + abonos. El total de la orden se calcula
                  desde aquí; reemplaza al campo "Monto final" libre que estaba antes. */}
              {detalleId && (
                <SeccionItemsAbonosOrden
                  ordenId={detalleId}
                  ordenEstado={detalle.estado || "RECIBIDO"}
                  onTotalChange={setTotalOrdenItems}
                  onAbonosChange={(t, abs) => { setTotalHoldingOrden(t); setAbonosOrden(abs); }}
                />
              )}
              <div style={{ marginBottom: 12 }}>
                <label style={{ fontSize: 12, fontWeight: 600 }}>Garantía (días)</label>
                <input className="input" type="number"
                  value={detalle.garantia_dias || 0}
                  onChange={(e) => setDetalle({ ...detalle, garantia_dias: parseInt(e.target.value) || 0 })}
                  onBlur={() => actualizarOrdenServicio(detalle).then(() => toastExito("Guardado")).catch(err => toastError("" + err))} />
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
            <div style={{
              padding: "10px 20px", borderTop: "1px solid var(--color-border)",
              display: "flex", gap: 6, justifyContent: "flex-end", flexWrap: "wrap", flexShrink: 0,
            }}>
              {esAdmin && (
                <button className="btn btn-danger" style={{ fontSize: 11 }}
                  onClick={async () => {
                    if (!confirm("¿Eliminar esta orden?")) return;
                    try { await eliminarOrdenServicio(detalleId!); toastExito("Eliminada"); setDetalle(null); setDetalleId(null); cargar(); }
                    catch (err) { toastError("" + err); }
                  }}>Eliminar</button>
              )}
              {/* v2.4.13 ST-5: cancelar orden (devuelve abonos automáticamente). Cualquier cajero puede,
                  no requiere admin (los abonos se devuelven sin autorización). */}
              {detalle.estado !== "ENTREGADO" && detalle.estado !== "CANCELADA" && (
                <button className="btn btn-outline" style={{ fontSize: 11, color: "var(--color-warning)" }}
                  onClick={handleCancelarOrden}
                  title="Cancela la orden y devuelve abonos en holding al cliente">
                  🚫 Cancelar orden
                </button>
              )}
              {/* v2.4.14: avisar al cliente por WhatsApp con el estado de la orden */}
              {detalle.cliente_telefono && (
                <button className="btn btn-outline" style={{ fontSize: 11, color: "#25D366" }}
                  onClick={() => {
                    const tel = (detalle.cliente_telefono || "").replace(/\D/g, "");
                    if (!tel) { toastError("El cliente no tiene teléfono"); return; }
                    // Si el número no empieza con código de país, asumir Ecuador (593)
                    const telFmt = tel.startsWith("593") ? tel : tel.startsWith("0") ? "593" + tel.slice(1) : "593" + tel;
                    const equipo = detalle.equipo_descripcion || "su equipo";
                    const num = detalle.numero || "";
                    let mensaje = "";
                    switch (detalle.estado) {
                      case "LISTO":
                        mensaje = `Hola ${detalle.cliente_nombre || ""}, su ${equipo} (orden ${num}) está listo para retirar. ¡Lo esperamos!`; break;
                      case "ENTREGADO_PARCIAL":
                        mensaje = `Hola ${detalle.cliente_nombre || ""}, le recordamos que tiene un saldo pendiente sobre la orden ${num} (${equipo}). Saludos.`; break;
                      case "ESPERANDO_REPUESTOS":
                        mensaje = `Hola ${detalle.cliente_nombre || ""}, su ${equipo} (orden ${num}) está en espera de repuestos. Le avisaremos apenas llegue.`; break;
                      case "DIAGNOSTICANDO":
                      case "EN_REPARACION":
                        mensaje = `Hola ${detalle.cliente_nombre || ""}, le informamos que su ${equipo} (orden ${num}) está actualmente en proceso. Le avisaremos cuando esté listo.`; break;
                      default:
                        mensaje = `Hola ${detalle.cliente_nombre || ""}, le escribimos sobre la orden ${num} (${equipo}).`;
                    }
                    const url = `https://wa.me/${telFmt}?text=${encodeURIComponent(mensaje)}`;
                    window.open(url, "_blank");
                  }}
                  title={`Avisar a ${detalle.cliente_nombre || "cliente"} por WhatsApp`}>
                  📱 Avisar al cliente
                </button>
              )}
              {/* v2.4.12 ST-4: selector de formato (A4 vs Ticket 80mm) */}
              <div style={{ display: "inline-flex", border: "1px solid var(--color-border)", borderRadius: 6, overflow: "hidden" }}>
                <button className="btn"
                  style={{
                    fontSize: 12, padding: "6px 12px", borderRadius: 0, border: "none",
                    background: formatoImpresion === "A4" ? "var(--color-primary)" : "transparent",
                    color: formatoImpresion === "A4" ? "#fff" : "var(--color-text)",
                  }}
                  onClick={() => setFormatoImpresion("A4")}
                  title="Hoja A4 (impresora normal)">A4</button>
                <button className="btn"
                  style={{
                    fontSize: 12, padding: "6px 12px", borderRadius: 0, border: "none",
                    borderLeft: "1px solid var(--color-border)",
                    background: formatoImpresion === "TICKET_80" ? "var(--color-primary)" : "transparent",
                    color: formatoImpresion === "TICKET_80" ? "#fff" : "var(--color-text)",
                  }}
                  onClick={() => setFormatoImpresion("TICKET_80")}
                  title="Térmica 80mm">80mm</button>
                <button className="btn"
                  style={{ fontSize: 12, padding: "6px 14px", borderRadius: 0, border: "none", borderLeft: "1px solid var(--color-border)" }}
                  onClick={() => imprimirOrdenServicioPdf(detalleId!, formatoImpresion).catch(err => toastError("" + err))}>
                  📄 Imprimir
                </button>
              </div>
              {detalle.estado !== "ENTREGADO" && detalle.estado !== "CANCELADA" && detalle.estado !== "CANCELADO" && (
                <button className="btn btn-success" onClick={() => {
                  setMostrarCobrar(true);
                  setCobroMontoRecibido("");
                  setCobroRepuestos([]);
                  // v2.4.12: precargar garantía con el valor actual de la orden
                  setCobroGarantiaDias(String(detalle?.garantia_dias ?? 0));
                  // v2.4.13: precargar primer pago con el saldo (= total − abonos)
                  const saldo = Math.max(totalOrdenItems.total - totalHoldingOrden, 0);
                  setCobroPagos([{ forma_pago: "EFECTIVO", monto: saldo }]);
                  // v2.4.14: reset del flag de saldo pendiente
                  setPermitirSaldoPendiente(false);
                  // v2.4.25: precargar km salida con km de entrada (sugerencia)
                  setCobroKmSalida(detalle?.equipo_kilometraje ? String(detalle.equipo_kilometraje) : "");
                }}>💰 Cobrar</button>
              )}
            </div>
          </div>
        </>
      )}

      {/* Modal Cobrar */}
      {mostrarCobrar && detalle && (
        <div className="modal-overlay" onClick={() => setMostrarCobrar(false)}>
          <div className="modal-content" onClick={(e) => e.stopPropagation()} style={{ maxWidth: 640 }}>
            <div className="modal-header"><h3>Cobrar orden {detalle.numero}</h3></div>
            <div className="modal-body">
              {/* v2.4.13 ST-5: resumen de items + abonos descontados */}
              <div style={{ background: "var(--color-surface-alt)", padding: 12, borderRadius: 6, marginBottom: 12 }}>
                <div style={{ display: "flex", justifyContent: "space-between", fontSize: 12 }}>
                  <span>Total de items:</span>
                  <strong>${totalOrdenItems.total.toFixed(2)}</strong>
                </div>
                {totalHoldingOrden > 0 && (
                  <div style={{ display: "flex", justifyContent: "space-between", fontSize: 12, color: "var(--color-warning)" }}>
                    <span>− Abonos en holding:</span>
                    <strong>−${totalHoldingOrden.toFixed(2)}</strong>
                  </div>
                )}
                <div style={{ display: "flex", justifyContent: "space-between", fontSize: 16, fontWeight: 700, marginTop: 4, paddingTop: 4, borderTop: "1px solid var(--color-border)" }}>
                  <span>Saldo a cobrar:</span>
                  <span style={{ color: "var(--color-success)" }}>${saldoCobro.toFixed(2)}</span>
                </div>
                {totalOrdenItems.cantidad_items === 0 && (
                  <div style={{ fontSize: 11, color: "var(--color-danger)", marginTop: 6 }}>
                    ⚠ Esta orden no tiene items. Cierra este modal y agrega items antes de cobrar.
                  </div>
                )}
              </div>

              {/* v2.4.13 ST-5: Pago mixto. Usa la misma lógica que POS: lista de pagos. */}
              <div style={{ marginBottom: 12 }}>
                <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: 6 }}>
                  <label style={{ fontSize: 12, fontWeight: 600 }}>Pagos</label>
                  <button type="button" className="btn btn-outline" style={{ fontSize: 11, padding: "2px 8px" }}
                    onClick={() => setCobroPagos([...cobroPagos, { forma_pago: "EFECTIVO", monto: 0 }])}>
                    + Agregar pago
                  </button>
                </div>
                {cobroPagos.map((p, i) => (
                  <div key={i} style={{ display: "grid", gridTemplateColumns: "130px 1fr 30px", gap: 6, alignItems: "center", marginBottom: 6 }}>
                    <select className="input" value={p.forma_pago}
                      onChange={(e) => {
                        const nu = [...cobroPagos];
                        nu[i] = { ...nu[i], forma_pago: e.target.value, banco_id: null, referencia: null };
                        setCobroPagos(nu);
                      }}
                      style={{ fontSize: 12 }}>
                      <option value="EFECTIVO">Efectivo</option>
                      <option value="TRANSFER">Transferencia</option>
                      <option value="TARJETA">Tarjeta</option>
                      <option value="CREDITO">Crédito</option>
                    </select>
                    <div>
                      <input className="input" type="number" step="0.01" placeholder="Monto"
                        value={p.monto || ""}
                        onChange={(e) => {
                          const nu = [...cobroPagos];
                          nu[i] = { ...nu[i], monto: parseFloat(e.target.value) || 0 };
                          setCobroPagos(nu);
                        }}
                        style={{ fontSize: 12 }} />
                      {p.forma_pago === "TRANSFER" && (
                        <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: 4, marginTop: 4 }}>
                          <select className="input" value={p.banco_id || ""}
                            onChange={(e) => {
                              const nu = [...cobroPagos];
                              nu[i] = { ...nu[i], banco_id: parseInt(e.target.value) || null };
                              setCobroPagos(nu);
                            }}
                            style={{ fontSize: 11 }}>
                            <option value="">Cuenta...</option>
                            {bancosCobro.map(b => <option key={b.id} value={b.id}>{b.nombre}</option>)}
                          </select>
                          <input className="input" placeholder="Referencia"
                            value={p.referencia || ""}
                            onChange={(e) => {
                              const nu = [...cobroPagos];
                              nu[i] = { ...nu[i], referencia: e.target.value };
                              setCobroPagos(nu);
                            }}
                            style={{ fontSize: 11 }} />
                        </div>
                      )}
                    </div>
                    {cobroPagos.length > 1 && (
                      <button type="button"
                        onClick={() => setCobroPagos(cobroPagos.filter((_, j) => j !== i))}
                        style={{ background: "transparent", border: "none", cursor: "pointer", color: "var(--color-danger)", fontSize: 16 }}>×</button>
                    )}
                  </div>
                ))}
                {/* Atajos rápidos: completa el primer pago al saldo exacto */}
                {saldoCobro > 0 && (
                  <div style={{ display: "flex", gap: 6, marginTop: 4 }}>
                    <button type="button" className="btn btn-outline" style={{ fontSize: 10, padding: "2px 8px" }}
                      onClick={() => {
                        const nu = [...cobroPagos];
                        nu[0] = { ...nu[0], monto: saldoCobro };
                        setCobroPagos(nu);
                      }}>= Saldo (${saldoCobro.toFixed(2)})</button>
                  </div>
                )}
              </div>

              {/* v2.4.25: km de salida si la orden es de vehículo. Al ingresar,
                  el sistema recalcula el próximo mantenimiento (= salida + intervalo). */}
              {detalle?.equipo_kilometraje != null && (
                <div style={{ marginBottom: 12, padding: 10, background: "rgba(168, 85, 247, 0.08)", borderRadius: 6, border: "1px solid rgba(168, 85, 247, 0.3)" }}>
                  <label style={{ fontSize: 12, fontWeight: 600 }}>🚗 Kilometraje de salida</label>
                  <div style={{ display: "flex", gap: 6, alignItems: "center", marginTop: 4 }}>
                    <input className="input" type="number"
                      style={{ width: 140 }}
                      placeholder={`Km al entregar`}
                      value={cobroKmSalida}
                      onChange={(e) => setCobroKmSalida(e.target.value)} />
                    <span style={{ fontSize: 11, color: "var(--color-text-muted)" }}>
                      Entrada: {detalle.equipo_kilometraje} km
                      {detalle.equipo_kilometraje_intervalo ? ` · Cada ${detalle.equipo_kilometraje_intervalo} km` : ""}
                    </span>
                  </div>
                  {cobroKmSalida && detalle.equipo_kilometraje_intervalo && (
                    <div style={{ fontSize: 11, color: "var(--color-success)", marginTop: 4 }}>
                      ✓ Próximo mantenimiento: <strong>{(parseInt(cobroKmSalida) || 0) + detalle.equipo_kilometraje_intervalo} km</strong>
                    </div>
                  )}
                </div>
              )}

              {/* v2.4.12: garantía aplicada al cobrar (queda registrada en la orden) */}
              <div style={{ marginBottom: 12, padding: 10, background: "rgba(59, 130, 246, 0.08)", borderRadius: 6, border: "1px solid rgba(59, 130, 246, 0.3)" }}>
                <label style={{ fontSize: 12, fontWeight: 600 }}>
                  🛡 Garantía del trabajo (días)
                </label>
                <div style={{ display: "flex", gap: 6, alignItems: "center", marginTop: 4 }}>
                  <input className="input" type="number" min="0" max="365"
                    style={{ width: 100 }}
                    value={cobroGarantiaDias}
                    onChange={(e) => setCobroGarantiaDias(e.target.value)} />
                  <span style={{ fontSize: 11, color: "var(--color-text-muted)" }}>días</span>
                  {/* Atajos rápidos */}
                  <div style={{ display: "flex", gap: 4, marginLeft: 12 }}>
                    {[0, 7, 15, 30, 60, 90, 180].map(d => (
                      <button key={d} type="button"
                        onClick={() => setCobroGarantiaDias(String(d))}
                        style={{
                          padding: "2px 8px", fontSize: 10, cursor: "pointer",
                          background: cobroGarantiaDias === String(d) ? "var(--color-primary)" : "transparent",
                          color: cobroGarantiaDias === String(d) ? "#fff" : "var(--color-text)",
                          border: "1px solid var(--color-border)", borderRadius: 4,
                        }}>
                        {d === 0 ? "Sin" : `${d}d`}
                      </button>
                    ))}
                  </div>
                </div>
                <div style={{ fontSize: 10, color: "var(--color-text-muted)", marginTop: 4 }}>
                  Quedará registrada en la orden y aparecerá en el comprobante.
                </div>
              </div>

              <div style={{ padding: 12, background: "rgba(34, 197, 94, 0.1)", borderRadius: 6, fontSize: 13 }}>
                <div style={{ display: "flex", justifyContent: "space-between" }}>
                  <span>Total pagado:</span>
                  <strong>${totalPagosCobro.toFixed(2)}</strong>
                </div>
                <div style={{ display: "flex", justifyContent: "space-between" }}>
                  <span>Saldo:</span>
                  <strong>${saldoCobro.toFixed(2)}</strong>
                </div>
                {totalPagosCobro > saldoCobro && cobroPagos.some(p => p.forma_pago === "EFECTIVO") && (
                  <div style={{ display: "flex", justifyContent: "space-between", color: "var(--color-success)", fontWeight: 700 }}>
                    <span>Cambio:</span>
                    <span>${(totalPagosCobro - saldoCobro).toFixed(2)}</span>
                  </div>
                )}
                {totalPagosCobro < saldoCobro && (
                  <>
                    <div style={{ color: "var(--color-danger)", fontSize: 11, marginTop: 4 }}>
                      Falta ${(saldoCobro - totalPagosCobro).toFixed(2)} para cubrir el saldo
                    </div>
                    {/* v2.4.14: cobranza parcial */}
                    <label style={{
                      display: "flex", alignItems: "flex-start", gap: 6,
                      marginTop: 8, padding: 8, borderRadius: 4,
                      background: "rgba(245,158,11,0.1)", border: "1px solid rgba(245,158,11,0.3)",
                      cursor: "pointer",
                    }}>
                      <input type="checkbox" checked={permitirSaldoPendiente}
                        onChange={(e) => setPermitirSaldoPendiente(e.target.checked)}
                        style={{ marginTop: 2 }} />
                      <div style={{ fontSize: 11 }}>
                        <strong style={{ color: "var(--color-warning)" }}>Permitir saldo pendiente</strong>
                        <div style={{ color: "var(--color-text-secondary)", marginTop: 2 }}>
                          Entregar el equipo dejando ${(saldoCobro - totalPagosCobro).toFixed(2)} pendiente.
                          Se marca como ENTREGADO_PARCIAL y queda registrado el saldo.
                        </div>
                      </div>
                    </label>
                  </>
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

      {/* v2.4.9 ST-2: modales nuevos */}
      {mostrarConfig && (
        <ModalConfigServicioTecnico onCerrar={() => setMostrarConfig(false)} />
      )}
      {mostrarHistorial && (
        <ModalHistorialServicioTecnico
          onCerrar={() => setMostrarHistorial(false)}
          // v2.4.25: cargar el detalle completo + imágenes + movimientos al abrir
          onAbrirOrden={(id) => { setMostrarHistorial(false); abrirDetalle(id); }}
        />
      )}
    </>
  );
}
