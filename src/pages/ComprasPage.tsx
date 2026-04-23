import React, { useState, useEffect, useRef } from "react";
import {
  listarProveedores,
  buscarProveedores,
  crearProveedor,
  actualizarProveedor,
  eliminarProveedor,
  registrarCompra,
  listarCompras,
  obtenerCompra,
  anularCompra,
  buscarProductos,
  consultarIdentificacion,
  previewXmlCompra,
  importarXmlCompra,
  listarCategorias,
} from "../services/api";
import type { PreviewXmlCompra, ItemMapeadoXml } from "../services/api";
import { useToast } from "../components/Toast";
import Modal from "../components/Modal";
import type { Proveedor, Compra, CompraCompleta, ItemCompra, Categoria } from "../types";
import type { ProductoBusqueda } from "../types";

type AccionItem = "producto_nuevo" | "producto_existente" | "gasto" | "ignorar";
interface ItemUI {
  accion: AccionItem;
  producto_id?: number;
  producto_nombre?: string;
  producto_nuevo_codigo: string;
  producto_nuevo_nombre: string;
  producto_nuevo_categoria?: number;
  gasto_categoria: string;
  // datos originales del XML
  descripcion: string;
  codigo_principal?: string;
  cantidad: number;
  precio_unitario: number;
  iva_porcentaje: number;
  subtotal: number;
}

function fechaHoy(): string {
  const now = new Date();
  const y = now.getFullYear();
  const m = String(now.getMonth() + 1).padStart(2, "0");
  const d = String(now.getDate()).padStart(2, "0");
  return `${y}-${m}-${d}`;
}

function fechaHace(dias: number): string {
  const d = new Date();
  d.setDate(d.getDate() - dias);
  const y = d.getFullYear();
  const m = String(d.getMonth() + 1).padStart(2, "0");
  const dd = String(d.getDate()).padStart(2, "0");
  return `${y}-${m}-${dd}`;
}

export default function ComprasPage() {
  const { toastExito, toastError } = useToast();
  const [tab, setTab] = useState<"compras" | "proveedores">("compras");

  // ==================== PROVEEDORES TAB ====================
  const [proveedores, setProveedores] = useState<Proveedor[]>([]);
  const [busquedaProv, setBusquedaProv] = useState("");
  const [mostrarFormProv, setMostrarFormProv] = useState(false);
  const [editandoProv, setEditandoProv] = useState<Proveedor | null>(null);
  const [confirmarEliminarProv, setConfirmarEliminarProv] = useState<number | null>(null);
  const [provForm, setProvForm] = useState({
    ruc: "", nombre: "", direccion: "", telefono: "", email: "", contacto: "", dias_credito: "30",
  });

  const cargarProveedores = async () => {
    try {
      if (busquedaProv.trim()) {
        setProveedores(await buscarProveedores(busquedaProv.trim()));
      } else {
        setProveedores(await listarProveedores());
      }
    } catch (err) {
      toastError("Error cargando proveedores: " + err);
    }
  };

  useEffect(() => {
    if (tab === "proveedores") cargarProveedores();
  }, [tab, busquedaProv]);

  const resetProvForm = () => {
    setProvForm({ ruc: "", nombre: "", direccion: "", telefono: "", email: "", contacto: "", dias_credito: "30" });
    setEditandoProv(null);
    setMostrarFormProv(false);
  };

  const handleGuardarProv = async () => {
    if (!provForm.nombre.trim()) {
      toastError("El nombre es requerido");
      return;
    }
    try {
      const prov: Proveedor = {
        id: editandoProv?.id,
        ruc: provForm.ruc.trim() || undefined,
        nombre: provForm.nombre.trim(),
        direccion: provForm.direccion.trim() || undefined,
        telefono: provForm.telefono.trim() || undefined,
        email: provForm.email.trim() || undefined,
        contacto: provForm.contacto.trim() || undefined,
        dias_credito: provForm.dias_credito ? parseInt(provForm.dias_credito) : undefined,
        activo: true,
      };
      if (editandoProv?.id) {
        await actualizarProveedor(prov);
        toastExito("Proveedor actualizado");
      } else {
        await crearProveedor(prov);
        toastExito("Proveedor creado");
      }
      resetProvForm();
      cargarProveedores();
    } catch (err) {
      toastError("Error: " + err);
    }
  };

  const handleEliminarProv = async () => {
    if (confirmarEliminarProv === null) return;
    try {
      await eliminarProveedor(confirmarEliminarProv);
      toastExito("Proveedor eliminado");
      setConfirmarEliminarProv(null);
      cargarProveedores();
    } catch (err) {
      toastError("Error: " + err);
    }
  };

  // ==================== COMPRAS TAB ====================
  const [compras, setCompras] = useState<Compra[]>([]);
  const [fechaDesde, setFechaDesde] = useState(fechaHoy());
  const [fechaHasta, setFechaHasta] = useState(fechaHoy());
  const [mostrarFormCompra, setMostrarFormCompra] = useState(false);
  const [verCompra, setVerCompra] = useState<CompraCompleta | null>(null);
  const [confirmarAnular, setConfirmarAnular] = useState<number | null>(null);

  // Form compra
  const [proveedoresLista, setProveedoresLista] = useState<Proveedor[]>([]);
  const [proveedorId, setProveedorId] = useState<number | "">("");
  const [numeroFactura, setNumeroFactura] = useState("");
  const [formaPago, setFormaPago] = useState("EFECTIVO");
  const [esCredito, setEsCredito] = useState(false);
  const [diasCredito, setDiasCredito] = useState("30");
  const [observacion, setObservacion] = useState("");
  const [items, setItems] = useState<(ItemCompra & { _key: number; nombre_display: string; requiere_caducidad?: boolean })[]>([]);
  const [buscandoProducto, setBuscandoProducto] = useState("");
  const [resultadosBusqueda, setResultadosBusqueda] = useState<ProductoBusqueda[]>([]);
  const [itemBuscaIndex, setItemBuscaIndex] = useState<number | null>(null);
  const keyCounter = useRef(0);
  const searchTimeoutRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  // ==================== IMPORTAR XML ====================
  const xmlFileRef = useRef<HTMLInputElement>(null);
  const [xmlPreview, setXmlPreview] = useState<PreviewXmlCompra | null>(null);
  const [xmlItems, setXmlItems] = useState<ItemUI[]>([]);
  const [xmlProveedorId, setXmlProveedorId] = useState<number | "">("");
  const [xmlFormaPago, setXmlFormaPago] = useState<string>("EFECTIVO");
  const [xmlDiasCredito, setXmlDiasCredito] = useState<string>("30");
  const [xmlProcesando, setXmlProcesando] = useState(false);
  const [categoriasXml, setCategoriasXml] = useState<Categoria[]>([]);
  // Búsqueda producto existente
  const [xmlBusquedaIdx, setXmlBusquedaIdx] = useState<number | null>(null);
  const [xmlBusquedaTexto, setXmlBusquedaTexto] = useState("");
  const [xmlResultadosBusqueda, setXmlResultadosBusqueda] = useState<ProductoBusqueda[]>([]);
  const xmlSearchTimeoutRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  const handleXmlUpload = async (e: React.ChangeEvent<HTMLInputElement>) => {
    const file = e.target.files?.[0];
    if (!file) return;
    try {
      const texto = await file.text();
      const preview = await previewXmlCompra(texto);
      // Auto-crear proveedor si no existe (con datos del XML)
      let provId: number | "" = preview.proveedor_id ?? "";
      if (!preview.proveedor_existe && preview.proveedor_ruc) {
        try {
          const creado: any = await crearProveedor({
            ruc: preview.proveedor_ruc,
            nombre: preview.proveedor_nombre || "Proveedor XML",
            dias_credito: 30,
            activo: true,
          } as Proveedor);
          const newId: number = typeof creado === "number" ? creado : Number(creado?.id ?? 0);
          if (newId) {
            provId = newId;
            preview.proveedor_existe = true;
            preview.proveedor_id = newId;
            const lista = await listarProveedores();
            setProveedoresLista(lista);
            toastExito(`Proveedor "${preview.proveedor_nombre}" creado automáticamente`);
          }
        } catch (errProv) {
          // Si falla, simplemente continuar — el usuario podrá crear manualmente
          console.error("Error auto-creando proveedor:", errProv);
        }
      }
      setXmlPreview(preview);
      setXmlProveedorId(provId);
      setXmlFormaPago("EFECTIVO");
      setXmlDiasCredito("30");
      // Construir items con acción por defecto
      const items: ItemUI[] = preview.items.map((it) => ({
        accion: it.producto_existente_id ? "producto_existente" : "producto_nuevo",
        producto_id: it.producto_existente_id ?? undefined,
        producto_nombre: it.producto_existente_nombre ?? undefined,
        producto_nuevo_codigo: it.codigo_principal ?? "",
        producto_nuevo_nombre: it.descripcion,
        producto_nuevo_categoria: undefined,
        gasto_categoria: "Compra proveedor",
        descripcion: it.descripcion,
        codigo_principal: it.codigo_principal ?? undefined,
        cantidad: it.cantidad,
        precio_unitario: it.precio_unitario,
        iva_porcentaje: it.iva_porcentaje,
        subtotal: it.subtotal,
      }));
      setXmlItems(items);
      // Cargar categorías
      try { setCategoriasXml(await listarCategorias()); } catch { /* noop */ }
    } catch (err) {
      toastError("Error al leer XML: " + err);
    } finally {
      if (xmlFileRef.current) xmlFileRef.current.value = "";
    }
  };

  const crearProveedorDesdeXml = async () => {
    if (!xmlPreview) return;
    try {
      const creado: any = await crearProveedor({
        ruc: xmlPreview.proveedor_ruc || undefined,
        nombre: xmlPreview.proveedor_nombre || "Proveedor XML",
        dias_credito: 30,
        activo: true,
      } as Proveedor);
      // El backend puede retornar el objeto Proveedor o un number según la version
      const newId: number = typeof creado === "number" ? creado : Number(creado?.id ?? 0);
      const lista = await listarProveedores();
      setProveedoresLista(lista);
      // Si no obtuvimos id, intentar buscarlo por ruc
      let finalId = newId;
      if (!finalId && xmlPreview.proveedor_ruc) {
        const match = lista.find((p) => p.ruc === xmlPreview.proveedor_ruc);
        if (match?.id) finalId = match.id;
      }
      if (finalId) {
        setXmlProveedorId(finalId);
        setXmlPreview({ ...xmlPreview, proveedor_existe: true, proveedor_id: finalId });
      }
      toastExito("Proveedor creado");
    } catch (err) {
      toastError("Error creando proveedor: " + err);
    }
  };

  const actualizarItemXml = (idx: number, cambios: Partial<ItemUI>) => {
    setXmlItems((prev) => prev.map((it, i) => (i === idx ? { ...it, ...cambios } : it)));
  };

  const buscarProductoExistenteXml = (texto: string, idx: number) => {
    setXmlBusquedaIdx(idx);
    setXmlBusquedaTexto(texto);
    if (xmlSearchTimeoutRef.current) clearTimeout(xmlSearchTimeoutRef.current);
    if (texto.trim().length < 2) {
      setXmlResultadosBusqueda([]);
      return;
    }
    xmlSearchTimeoutRef.current = setTimeout(async () => {
      try {
        const res = await buscarProductos(texto.trim());
        setXmlResultadosBusqueda(res);
      } catch {
        setXmlResultadosBusqueda([]);
      }
    }, 300);
  };

  const handleProcesarXml = async () => {
    if (!xmlPreview) return;
    if (!xmlProveedorId) {
      toastError("Seleccione un proveedor");
      return;
    }
    // Validar items
    for (const it of xmlItems) {
      if (it.accion === "producto_existente" && !it.producto_id) {
        toastError(`Seleccione un producto existente para: ${it.descripcion}`);
        return;
      }
      if (it.accion === "producto_nuevo" && !it.producto_nuevo_nombre.trim()) {
        toastError(`Ingrese nombre del nuevo producto: ${it.descripcion}`);
        return;
      }
    }
    const mapeados: ItemMapeadoXml[] = xmlItems.map((it) => ({
      accion: it.accion,
      producto_id: it.accion === "producto_existente" ? it.producto_id ?? null : null,
      producto_nuevo:
        it.accion === "producto_nuevo"
          ? {
              codigo: it.producto_nuevo_codigo.trim() || null,
              nombre: it.producto_nuevo_nombre.trim(),
              categoria_id: it.producto_nuevo_categoria ?? null,
              iva_porcentaje: it.iva_porcentaje,
            }
          : null,
      gasto_categoria: it.accion === "gasto" ? it.gasto_categoria : null,
      descripcion: it.descripcion,
      cantidad: it.cantidad,
      precio_unitario: it.precio_unitario,
      iva_porcentaje: it.iva_porcentaje,
      subtotal: it.subtotal,
    }));
    try {
      setXmlProcesando(true);
      const res = await importarXmlCompra({
        proveedor_id: xmlProveedorId as number,
        numero_factura: xmlPreview.numero_factura,
        fecha_emision: xmlPreview.fecha_emision,
        items_mapeados: mapeados,
        forma_pago: xmlFormaPago,
        dias_credito: xmlFormaPago === "CREDITO" ? parseInt(xmlDiasCredito) || 30 : null,
      });
      const partes: string[] = [];
      if (res.productos_creados > 0) partes.push(`${res.productos_creados} producto(s) creado(s)`);
      if (res.gastos_creados > 0) partes.push(`${res.gastos_creados} gasto(s) creado(s)`);
      if (res.compra_id) partes.push(`Compra registrada`);
      toastExito(partes.length ? partes.join(" · ") : "Importación completada");
      setXmlPreview(null);
      setXmlItems([]);
      cargarCompras();
    } catch (err) {
      toastError("Error: " + err);
    } finally {
      setXmlProcesando(false);
    }
  };

  const cargarCompras = async () => {
    try {
      setCompras(await listarCompras(fechaDesde, fechaHasta));
    } catch (err) {
      toastError("Error cargando compras: " + err);
    }
  };

  useEffect(() => {
    if (tab === "compras") {
      cargarCompras();
      listarProveedores().then(setProveedoresLista).catch(() => {});
    }
  }, [tab, fechaDesde, fechaHasta]);

  const setPreset = (dias: number) => {
    setFechaDesde(fechaHace(dias));
    setFechaHasta(fechaHoy());
  };

  const agregarItemVacio = () => {
    setItems([...items, {
      _key: ++keyCounter.current,
      producto_id: undefined,
      descripcion: "",
      cantidad: 1,
      precio_unitario: 0,
      iva_porcentaje: 15,
      nombre_display: "",
    }]);
  };

  const actualizarItem = (index: number, campo: string, valor: string | number | undefined) => {
    const newItems = [...items];
    (newItems[index] as unknown as Record<string, unknown>)[campo] = valor;
    setItems(newItems);
  };

  const eliminarItem = (index: number) => {
    setItems(items.filter((_, i) => i !== index));
  };

  const buscarProductoParaItem = async (termino: string, index: number) => {
    setBuscandoProducto(termino);
    setItemBuscaIndex(index);
    if (searchTimeoutRef.current) clearTimeout(searchTimeoutRef.current);
    if (termino.trim().length < 2) {
      setResultadosBusqueda([]);
      return;
    }
    searchTimeoutRef.current = setTimeout(async () => {
      try {
        const res = await buscarProductos(termino.trim());
        setResultadosBusqueda(res);
      } catch {
        setResultadosBusqueda([]);
      }
    }, 300);
  };

  const seleccionarProducto = async (prod: ProductoBusqueda, index: number) => {
    // Consultar si el producto requiere caducidad (buscamos en productos completo)
    let requiereCaducidad = false;
    try {
      const { obtenerProducto } = await import("../services/api");
      const p: any = await obtenerProducto(prod.id);
      requiereCaducidad = !!p?.requiere_caducidad;
    } catch { /* ignore */ }

    const newItems = [...items];
    newItems[index] = {
      ...newItems[index],
      producto_id: prod.id,
      nombre_display: prod.nombre,
      descripcion: prod.nombre,
      precio_unitario: prod.precio_costo ?? 0,
      iva_porcentaje: prod.iva_porcentaje,
      requiere_caducidad: requiereCaducidad,
    };
    setItems(newItems);
    setBuscandoProducto("");
    setResultadosBusqueda([]);
    setItemBuscaIndex(null);
  };

  const calcularTotales = () => {
    let subtotal = 0;
    let iva = 0;
    for (const it of items) {
      const sub = it.cantidad * it.precio_unitario;
      subtotal += sub;
      iva += sub * (it.iva_porcentaje / 100);
    }
    return { subtotal, iva, total: subtotal + iva };
  };

  const handleRegistrarCompra = async () => {
    if (!proveedorId) {
      toastError("Seleccione un proveedor");
      return;
    }
    if (items.length === 0) {
      toastError("Agregue al menos un item");
      return;
    }
    for (const it of items) {
      if (it.cantidad <= 0 || it.precio_unitario <= 0) {
        toastError("Todos los items deben tener cantidad y precio mayor a 0");
        return;
      }
      if (!it.producto_id && !it.descripcion?.trim()) {
        toastError("Cada item debe tener un producto o descripcion");
        return;
      }
    }
    try {
      await registrarCompra({
        proveedor_id: proveedorId as number,
        numero_factura: numeroFactura.trim() || undefined,
        items: items.map(({ producto_id, descripcion, cantidad, precio_unitario, iva_porcentaje, lote_numero, lote_fecha_caducidad, lote_fecha_elaboracion }) => ({
          producto_id,
          descripcion: descripcion?.trim() || undefined,
          cantidad, precio_unitario, iva_porcentaje,
          lote_numero: lote_numero?.trim() || undefined,
          lote_fecha_caducidad: lote_fecha_caducidad || undefined,
          lote_fecha_elaboracion: lote_fecha_elaboracion || undefined,
        })),
        forma_pago: formaPago,
        es_credito: esCredito,
        observacion: observacion.trim() || undefined,
        dias_credito: esCredito ? parseInt(diasCredito) : undefined,
      });
      toastExito("Compra registrada exitosamente");
      setMostrarFormCompra(false);
      setProveedorId("");
      setNumeroFactura("");
      setFormaPago("EFECTIVO");
      setEsCredito(false);
      setDiasCredito("30");
      setObservacion("");
      setItems([]);
      cargarCompras();
    } catch (err) {
      toastError("Error: " + err);
    }
  };

  const handleVerCompra = async (id: number) => {
    try {
      setVerCompra(await obtenerCompra(id));
    } catch (err) {
      toastError("Error: " + err);
    }
  };

  const handleAnularCompra = async () => {
    if (confirmarAnular === null) return;
    try {
      await anularCompra(confirmarAnular);
      toastExito("Compra anulada");
      setConfirmarAnular(null);
      cargarCompras();
    } catch (err) {
      toastError("Error: " + err);
    }
  };

  const totales = calcularTotales();

  // ==================== RENDER ====================

  return (
    <>
      <div className="page-header">
        <div className="flex gap-2 items-center">
          <h2>Compras / Proveedores</h2>
          <div className="flex gap-1">
            <button
              className={`btn ${tab === "compras" ? "btn-primary" : "btn-outline"}`}
              style={{ fontSize: 12, padding: "4px 14px" }}
              onClick={() => setTab("compras")}
            >
              Compras
            </button>
            <button
              className={`btn ${tab === "proveedores" ? "btn-primary" : "btn-outline"}`}
              style={{ fontSize: 12, padding: "4px 14px" }}
              onClick={() => setTab("proveedores")}
            >
              Proveedores
            </button>
          </div>
        </div>
        {tab === "compras" && (
          <div className="flex gap-2">
            <input
              type="file"
              ref={xmlFileRef}
              accept=".xml"
              style={{ display: "none" }}
              onChange={handleXmlUpload}
            />
            <button
              className="btn btn-outline"
              onClick={() => xmlFileRef.current?.click()}
              title="Importar factura desde XML del SRI"
            >
              Importar XML
            </button>
            <button className="btn btn-primary" onClick={() => { setMostrarFormCompra(!mostrarFormCompra); if (!mostrarFormCompra) agregarItemVacio(); }}>
              + Nueva Compra
            </button>
          </div>
        )}
        {tab === "proveedores" && (
          <button className="btn btn-primary" onClick={() => { setMostrarFormProv(true); setEditandoProv(null); setProvForm({ ruc: "", nombre: "", direccion: "", telefono: "", email: "", contacto: "", dias_credito: "30" }); }}>
            + Nuevo Proveedor
          </button>
        )}
      </div>

      <div className="page-body">
        {/* ==================== TAB COMPRAS ==================== */}
        {tab === "compras" && (
          <>
            {/* Formulario nueva compra */}
            {mostrarFormCompra && (
              <div className="card mb-4">
                <div className="card-header">Registrar Compra</div>
                <div className="card-body">
                  {/* Fila 1: Proveedor + Factura + Forma pago */}
                  <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr 1fr", gap: 12 }}>
                    <div>
                      <label className="text-secondary" style={{ fontSize: 12 }}>Proveedor *</label>
                      <select
                        className="input"
                        value={proveedorId}
                        onChange={(e) => setProveedorId(e.target.value ? Number(e.target.value) : "")}
                      >
                        <option value="">Seleccione proveedor...</option>
                        {proveedoresLista.map((p) => (
                          <option key={p.id} value={p.id}>{p.nombre}{p.ruc ? ` (${p.ruc})` : ""}</option>
                        ))}
                      </select>
                    </div>
                    <div>
                      <label className="text-secondary" style={{ fontSize: 12 }}>N. Factura proveedor</label>
                      <input
                        className="input"
                        placeholder="001-001-000000001"
                        value={numeroFactura}
                        onChange={(e) => setNumeroFactura(e.target.value)}
                      />
                    </div>
                    <div>
                      <label className="text-secondary" style={{ fontSize: 12 }}>Forma de pago</label>
                      <select className="input" value={formaPago} onChange={(e) => setFormaPago(e.target.value)}>
                        <option value="EFECTIVO">Efectivo</option>
                        <option value="TRANSFERENCIA">Transferencia</option>
                        <option value="DEBITO">Débito Bancario</option>
                        <option value="CHEQUE">Cheque</option>
                      </select>
                    </div>
                  </div>

                  {/* Fila 2: Credito + Dias + Observacion */}
                  <div style={{ display: "grid", gridTemplateColumns: "auto 120px 1fr", gap: 12, marginTop: 12, alignItems: "end" }}>
                    <div>
                      <label className="text-secondary" style={{ fontSize: 12, display: "flex", alignItems: "center", gap: 6 }}>
                        <input
                          type="checkbox"
                          checked={esCredito}
                          onChange={(e) => setEsCredito(e.target.checked)}
                        />
                        Compra a credito
                      </label>
                    </div>
                    {esCredito && (
                      <div>
                        <label className="text-secondary" style={{ fontSize: 12 }}>Dias credito</label>
                        <input
                          className="input"
                          type="number"
                          min="1"
                          value={diasCredito}
                          onChange={(e) => setDiasCredito(e.target.value)}
                        />
                      </div>
                    )}
                    <div>
                      <label className="text-secondary" style={{ fontSize: 12 }}>Observacion</label>
                      <input
                        className="input"
                        placeholder="Opcional"
                        value={observacion}
                        onChange={(e) => setObservacion(e.target.value)}
                      />
                    </div>
                  </div>

                  {/* Items */}
                  <div style={{ marginTop: 16 }}>
                    <div className="flex justify-between items-center mb-2">
                      <label className="text-secondary" style={{ fontSize: 12, fontWeight: 600 }}>Items de compra</label>
                      <button className="btn btn-outline" style={{ fontSize: 11, padding: "2px 10px" }} onClick={agregarItemVacio}>
                        + Agregar item
                      </button>
                    </div>
                    <table className="table" style={{ fontSize: 13 }}>
                      <thead>
                        <tr>
                          <th style={{ width: "35%" }}>Producto / Descripcion</th>
                          <th style={{ width: "12%" }}>Cantidad</th>
                          <th style={{ width: "15%" }}>Precio Unit.</th>
                          <th style={{ width: "10%" }}>IVA %</th>
                          <th style={{ width: "15%" }} className="text-right">Subtotal</th>
                          <th style={{ width: "50px" }}></th>
                        </tr>
                      </thead>
                      <tbody>
                        {items.map((item, idx) => (
                          <React.Fragment key={item._key}>
                          <tr>
                            <td style={{ position: "relative" }}>
                              <input
                                className="input"
                                placeholder="Buscar producto o escribir descripcion..."
                                value={itemBuscaIndex === idx ? buscandoProducto : (item.nombre_display || item.descripcion || "")}
                                onChange={(e) => {
                                  buscarProductoParaItem(e.target.value, idx);
                                  actualizarItem(idx, "descripcion", e.target.value);
                                  actualizarItem(idx, "nombre_display", e.target.value);
                                  actualizarItem(idx, "producto_id", undefined);
                                }}
                                onFocus={() => {
                                  setItemBuscaIndex(idx);
                                  setBuscandoProducto(item.nombre_display || item.descripcion || "");
                                }}
                                onBlur={() => {
                                  setTimeout(() => {
                                    if (itemBuscaIndex === idx) {
                                      setItemBuscaIndex(null);
                                      setResultadosBusqueda([]);
                                    }
                                  }, 200);
                                }}
                                style={{ fontSize: 12 }}
                              />
                              {itemBuscaIndex === idx && resultadosBusqueda.length > 0 && (
                                <div style={{
                                  position: "absolute", top: "100%", left: 0, right: 0, zIndex: 50,
                                  background: "var(--color-surface)", border: "1px solid var(--color-border)",
                                  borderRadius: "var(--radius)", maxHeight: 200, overflowY: "auto",
                                  boxShadow: "0 4px 12px rgba(0,0,0,0.3)",
                                }}>
                                  {resultadosBusqueda.map((prod) => (
                                    <div
                                      key={prod.id}
                                      style={{
                                        padding: "8px 12px", cursor: "pointer", fontSize: 12,
                                        borderBottom: "1px solid var(--color-border)",
                                      }}
                                      onMouseDown={(e) => { e.preventDefault(); seleccionarProducto(prod, idx); }}
                                    >
                                      <div style={{ fontWeight: 600 }}>{prod.nombre}</div>
                                      <div className="text-secondary" style={{ fontSize: 11 }}>
                                        {prod.codigo ? `${prod.codigo} | ` : ""}${prod.precio_venta.toFixed(2)} | Stock: {prod.stock_actual}
                                      </div>
                                    </div>
                                  ))}
                                </div>
                              )}
                            </td>
                            <td>
                              <input
                                className="input"
                                type="number"
                                min="0.01"
                                step="0.01"
                                value={item.cantidad}
                                onChange={(e) => actualizarItem(idx, "cantidad", parseFloat(e.target.value) || 0)}
                                style={{ fontSize: 12, textAlign: "center" }}
                              />
                            </td>
                            <td>
                              <input
                                className="input"
                                type="number"
                                min="0"
                                step="0.01"
                                value={item.precio_unitario}
                                onChange={(e) => actualizarItem(idx, "precio_unitario", parseFloat(e.target.value) || 0)}
                                style={{ fontSize: 12, textAlign: "right" }}
                              />
                            </td>
                            <td>
                              <select
                                className="input"
                                value={item.iva_porcentaje}
                                onChange={(e) => actualizarItem(idx, "iva_porcentaje", parseFloat(e.target.value))}
                                style={{ fontSize: 12 }}
                              >
                                <option value={0}>0%</option>
                                <option value={15}>15%</option>
                              </select>
                            </td>
                            <td className="text-right font-bold" style={{ fontSize: 13 }}>
                              ${(item.cantidad * item.precio_unitario).toFixed(2)}
                            </td>
                            <td>
                              <button
                                className="btn btn-danger"
                                style={{ padding: "2px 8px", fontSize: 11 }}
                                onClick={() => eliminarItem(idx)}
                              >
                                x
                              </button>
                            </td>
                          </tr>
                          {/* Sub-fila de LOTE si el producto requiere caducidad */}
                          {item.requiere_caducidad && (
                            <tr>
                              <td colSpan={6} style={{ padding: "4px 8px 8px 30px", background: "rgba(245, 158, 11, 0.04)" }}>
                                <div style={{ display: "grid", gridTemplateColumns: "120px 1fr 1fr 1fr", gap: 8, alignItems: "end" }}>
                                  <span style={{ fontSize: 11, fontWeight: 600, color: "var(--color-warning)" }}>
                                    🕐 Caducidad:
                                  </span>
                                  <div>
                                    <label style={{ fontSize: 10, color: "var(--color-text-secondary)", display: "block" }}>Nro. Lote (auto si vacio)</label>
                                    <input className="input" style={{ fontSize: 12 }} placeholder="LOT-001"
                                      value={item.lote_numero || ""}
                                      onChange={(e) => actualizarItem(idx, "lote_numero", e.target.value)} />
                                  </div>
                                  <div>
                                    <label style={{ fontSize: 10, color: "var(--color-text-secondary)", display: "block" }}>Fecha elaboracion</label>
                                    <input type="date" className="input" style={{ fontSize: 12 }}
                                      value={item.lote_fecha_elaboracion || ""}
                                      onChange={(e) => actualizarItem(idx, "lote_fecha_elaboracion", e.target.value)} />
                                  </div>
                                  <div>
                                    <label style={{ fontSize: 10, color: "var(--color-text-secondary)", display: "block" }}>Fecha caducidad *</label>
                                    <input type="date" className="input" style={{ fontSize: 12 }}
                                      value={item.lote_fecha_caducidad || ""}
                                      onChange={(e) => actualizarItem(idx, "lote_fecha_caducidad", e.target.value)} />
                                  </div>
                                </div>
                              </td>
                            </tr>
                          )}
                          </React.Fragment>
                        ))}
                        {items.length === 0 && (
                          <tr>
                            <td colSpan={6} className="text-center text-secondary" style={{ padding: 20 }}>
                              Agregue items a la compra
                            </td>
                          </tr>
                        )}
                      </tbody>
                    </table>
                  </div>

                  {/* Totales */}
                  <div style={{ display: "flex", justifyContent: "flex-end", marginTop: 12 }}>
                    <div style={{ minWidth: 220, textAlign: "right" }}>
                      <div className="flex justify-between" style={{ padding: "4px 0" }}>
                        <span className="text-secondary">Subtotal:</span>
                        <span>${totales.subtotal.toFixed(2)}</span>
                      </div>
                      <div className="flex justify-between" style={{ padding: "4px 0" }}>
                        <span className="text-secondary">IVA:</span>
                        <span>${totales.iva.toFixed(2)}</span>
                      </div>
                      <div className="flex justify-between" style={{ padding: "4px 0", borderTop: "1px solid var(--color-border)", fontWeight: 700, fontSize: 16 }}>
                        <span>Total:</span>
                        <span>${totales.total.toFixed(2)}</span>
                      </div>
                    </div>
                  </div>

                  {/* Botones */}
                  <div className="flex gap-2 mt-4" style={{ justifyContent: "flex-end" }}>
                    <button className="btn btn-outline" onClick={() => { setMostrarFormCompra(false); setItems([]); }}>
                      Cancelar
                    </button>
                    <button className="btn btn-primary" onClick={handleRegistrarCompra}>
                      Registrar Compra
                    </button>
                  </div>
                </div>
              </div>
            )}

            {/* Filtros de fecha */}
            <div className="flex gap-2 items-center mb-4">
              <button className="btn btn-outline" style={{ fontSize: 11, padding: "4px 10px" }} onClick={() => setPreset(0)}>Hoy</button>
              <button className="btn btn-outline" style={{ fontSize: 11, padding: "4px 10px" }} onClick={() => setPreset(7)}>7 dias</button>
              <button className="btn btn-outline" style={{ fontSize: 11, padding: "4px 10px" }} onClick={() => setPreset(30)}>30 dias</button>
              <input type="date" className="input" value={fechaDesde} onChange={(e) => setFechaDesde(e.target.value)} style={{ width: 150 }} />
              <span className="text-secondary">a</span>
              <input type="date" className="input" value={fechaHasta} onChange={(e) => setFechaHasta(e.target.value)} style={{ width: 150 }} />
            </div>

            {/* Tabla de compras */}
            <div className="card">
              <table className="table">
                <thead>
                  <tr>
                    <th>Numero</th>
                    <th>Fecha</th>
                    <th>Proveedor</th>
                    <th>Factura #</th>
                    <th className="text-right">Total</th>
                    <th>Estado</th>
                    <th style={{ width: 140 }}></th>
                  </tr>
                </thead>
                <tbody>
                  {compras.length === 0 ? (
                    <tr>
                      <td colSpan={7} className="text-center text-secondary" style={{ padding: 40 }}>
                        No hay compras registradas en este periodo
                      </td>
                    </tr>
                  ) : (
                    compras.map((c) => (
                      <tr key={c.id}>
                        <td><strong>{c.numero}</strong></td>
                        <td className="text-secondary" style={{ fontSize: 12 }}>
                          {c.fecha ? new Date(c.fecha).toLocaleDateString("es-EC", { day: "2-digit", month: "2-digit", year: "numeric" }) : "-"}
                        </td>
                        <td>{c.proveedor_nombre || "-"}</td>
                        <td className="text-secondary">{c.numero_factura || "-"}</td>
                        <td className="text-right font-bold">${c.total.toFixed(2)}</td>
                        <td>
                          <span style={{
                            padding: "2px 8px", borderRadius: 4, fontSize: 11, fontWeight: 600,
                            background: c.estado === "COMPLETADA" ? "rgba(34,197,94,0.15)" : c.estado === "ANULADA" ? "rgba(239,68,68,0.15)" : "rgba(250,204,21,0.15)",
                            color: c.estado === "COMPLETADA" ? "var(--color-success)" : c.estado === "ANULADA" ? "var(--color-danger)" : "var(--color-warning)",
                          }}>
                            {c.estado}
                          </span>
                        </td>
                        <td>
                          <div className="flex gap-1">
                            <button className="btn btn-outline" style={{ fontSize: 11, padding: "2px 8px" }}
                              onClick={() => handleVerCompra(c.id!)}>
                              Ver
                            </button>
                            {c.estado === "COMPLETADA" && (
                              <button className="btn btn-outline" style={{ fontSize: 11, padding: "2px 8px", color: "var(--color-danger)" }}
                                onClick={() => setConfirmarAnular(c.id!)}>
                                Anular
                              </button>
                            )}
                          </div>
                        </td>
                      </tr>
                    ))
                  )}
                </tbody>
              </table>
            </div>
          </>
        )}

        {/* ==================== TAB PROVEEDORES ==================== */}
        {tab === "proveedores" && (
          <>
            {/* Barra de busqueda */}
            <div className="flex gap-2 mb-4">
              <input
                className="input"
                placeholder="Buscar proveedor por nombre o RUC..."
                value={busquedaProv}
                onChange={(e) => setBusquedaProv(e.target.value)}
                style={{ maxWidth: 400 }}
              />
            </div>

            {/* Formulario inline */}
            {mostrarFormProv && (
              <div className="card mb-4">
                <div className="card-header">{editandoProv ? "Editar Proveedor" : "Nuevo Proveedor"}</div>
                <div className="card-body">
                  <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr 1fr", gap: 12 }}>
                    <div>
                      <label className="text-secondary" style={{ fontSize: 12 }}>RUC / Identificacion</label>
                      <div style={{ display: "flex", gap: 4 }}>
                        <input className="input" placeholder="1234567890001" value={provForm.ruc}
                          style={{ flex: 1 }}
                          onChange={(e) => setProvForm({ ...provForm, ruc: e.target.value })}
                          onBlur={async () => {
                            const ruc = provForm.ruc.trim();
                            if (/^\d{10}(\d{3})?$/.test(ruc) && !provForm.nombre) {
                              try {
                                const cli = await consultarIdentificacion(ruc);
                                setProvForm(prev => ({
                                  ...prev,
                                  nombre: cli.nombre || prev.nombre,
                                  direccion: cli.direccion || prev.direccion,
                                  telefono: cli.telefono || prev.telefono,
                                  email: cli.email || prev.email,
                                }));
                                toastExito("Datos encontrados en SRI");
                              } catch { /* silencioso */ }
                            }
                          }} />
                        <button className="btn btn-outline" style={{ padding: "4px 8px", fontSize: 11 }}
                          title="Consultar en SRI"
                          onClick={async () => {
                            const ruc = provForm.ruc.trim();
                            if (!/^\d{10}(\d{3})?$/.test(ruc)) { toastError("Ingrese un RUC o cédula válida"); return; }
                            try {
                              const cli = await consultarIdentificacion(ruc);
                              setProvForm(prev => ({
                                ...prev,
                                nombre: cli.nombre || prev.nombre,
                                direccion: cli.direccion || prev.direccion,
                                telefono: cli.telefono || prev.telefono,
                                email: cli.email || prev.email,
                              }));
                              toastExito("Datos encontrados en SRI");
                            } catch (err) { toastError("No se encontró: " + err); }
                          }}>
                          SRI
                        </button>
                      </div>
                    </div>
                    <div>
                      <label className="text-secondary" style={{ fontSize: 12 }}>Nombre *</label>
                      <input className="input" placeholder="Nombre del proveedor" value={provForm.nombre}
                        onChange={(e) => setProvForm({ ...provForm, nombre: e.target.value })} autoFocus />
                    </div>
                    <div>
                      <label className="text-secondary" style={{ fontSize: 12 }}>Contacto</label>
                      <input className="input" placeholder="Persona de contacto" value={provForm.contacto}
                        onChange={(e) => setProvForm({ ...provForm, contacto: e.target.value })} />
                    </div>
                    <div>
                      <label className="text-secondary" style={{ fontSize: 12 }}>Direccion</label>
                      <input className="input" placeholder="Direccion" value={provForm.direccion}
                        onChange={(e) => setProvForm({ ...provForm, direccion: e.target.value })} />
                    </div>
                    <div>
                      <label className="text-secondary" style={{ fontSize: 12 }}>Telefono</label>
                      <input className="input" placeholder="0999999999" value={provForm.telefono}
                        onChange={(e) => setProvForm({ ...provForm, telefono: e.target.value })} />
                    </div>
                    <div>
                      <label className="text-secondary" style={{ fontSize: 12 }}>Email</label>
                      <input className="input" placeholder="email@ejemplo.com" value={provForm.email}
                        onChange={(e) => setProvForm({ ...provForm, email: e.target.value })} />
                    </div>
                    <div>
                      <label className="text-secondary" style={{ fontSize: 12 }}>Dias credito</label>
                      <input className="input" type="number" min="0" value={provForm.dias_credito}
                        onChange={(e) => setProvForm({ ...provForm, dias_credito: e.target.value })} />
                    </div>
                  </div>
                  <div className="flex gap-2 mt-4" style={{ justifyContent: "flex-end" }}>
                    <button className="btn btn-outline" onClick={resetProvForm}>Cancelar</button>
                    <button className="btn btn-primary" onClick={handleGuardarProv}>
                      {editandoProv ? "Actualizar" : "Crear Proveedor"}
                    </button>
                  </div>
                </div>
              </div>
            )}

            {/* Tabla de proveedores */}
            <div className="card">
              <table className="table">
                <thead>
                  <tr>
                    <th>RUC</th>
                    <th>Nombre</th>
                    <th>Contacto</th>
                    <th>Telefono</th>
                    <th>Email</th>
                    <th>Dias Credito</th>
                    <th style={{ width: 120 }}></th>
                  </tr>
                </thead>
                <tbody>
                  {proveedores.length === 0 ? (
                    <tr>
                      <td colSpan={7} className="text-center text-secondary" style={{ padding: 40 }}>
                        No hay proveedores registrados
                      </td>
                    </tr>
                  ) : (
                    proveedores.map((p) => (
                      <tr key={p.id}>
                        <td className="text-secondary">{p.ruc || "-"}</td>
                        <td><strong>{p.nombre}</strong></td>
                        <td className="text-secondary">{p.contacto || "-"}</td>
                        <td className="text-secondary">{p.telefono || "-"}</td>
                        <td className="text-secondary">{p.email || "-"}</td>
                        <td className="text-center">{p.dias_credito ?? "-"}</td>
                        <td>
                          <div className="flex gap-1">
                            <button className="btn btn-outline" style={{ fontSize: 11, padding: "2px 8px" }}
                              onClick={() => {
                                setEditandoProv(p);
                                setProvForm({
                                  ruc: p.ruc || "",
                                  nombre: p.nombre,
                                  direccion: p.direccion || "",
                                  telefono: p.telefono || "",
                                  email: p.email || "",
                                  contacto: p.contacto || "",
                                  dias_credito: String(p.dias_credito ?? 30),
                                });
                                setMostrarFormProv(true);
                              }}>
                              Editar
                            </button>
                            <button className="btn btn-outline" style={{ fontSize: 11, padding: "2px 8px", color: "var(--color-danger)" }}
                              onClick={() => setConfirmarEliminarProv(p.id!)}>
                              x
                            </button>
                          </div>
                        </td>
                      </tr>
                    ))
                  )}
                </tbody>
              </table>
            </div>
          </>
        )}
      </div>

      {/* Modal ver detalle compra */}
      {verCompra && (
        <div style={{
          position: "fixed", inset: 0, background: "rgba(0,0,0,0.5)", zIndex: 1000,
          display: "flex", justifyContent: "center", alignItems: "center",
        }} onClick={() => setVerCompra(null)}>
          <div className="card" style={{ width: 600, maxHeight: "80vh", overflow: "auto" }} onClick={(e) => e.stopPropagation()}>
            <div className="card-header flex justify-between items-center">
              <span>Compra #{verCompra.compra.numero}</span>
              <button className="btn btn-outline" style={{ padding: "2px 8px" }} onClick={() => setVerCompra(null)}>x</button>
            </div>
            <div className="card-body">
              <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: 8, marginBottom: 16 }}>
                <div><span className="text-secondary" style={{ fontSize: 12 }}>Proveedor:</span> <strong>{verCompra.compra.proveedor_nombre}</strong></div>
                <div><span className="text-secondary" style={{ fontSize: 12 }}>Fecha:</span> {verCompra.compra.fecha ? new Date(verCompra.compra.fecha).toLocaleDateString("es-EC") : "-"}</div>
                <div><span className="text-secondary" style={{ fontSize: 12 }}>Factura #:</span> {verCompra.compra.numero_factura || "-"}</div>
                <div><span className="text-secondary" style={{ fontSize: 12 }}>Forma pago:</span> {verCompra.compra.forma_pago}</div>
                <div><span className="text-secondary" style={{ fontSize: 12 }}>Estado:</span> <span style={{ fontWeight: 600, color: verCompra.compra.estado === "ANULADA" ? "var(--color-danger)" : "var(--color-success)" }}>{verCompra.compra.estado}</span></div>
                <div><span className="text-secondary" style={{ fontSize: 12 }}>Credito:</span> {verCompra.compra.es_credito ? "Si" : "No"}</div>
                {verCompra.compra.observacion && (
                  <div style={{ gridColumn: "1/3" }}><span className="text-secondary" style={{ fontSize: 12 }}>Observacion:</span> {verCompra.compra.observacion}</div>
                )}
              </div>

              <table className="table" style={{ fontSize: 13 }}>
                <thead>
                  <tr>
                    <th>Producto</th>
                    <th className="text-center">Cant.</th>
                    <th className="text-right">P. Unit.</th>
                    <th className="text-right">Subtotal</th>
                  </tr>
                </thead>
                <tbody>
                  {verCompra.detalles.map((d, i) => (
                    <tr key={i}>
                      <td>{d.nombre_producto || d.descripcion || "-"}</td>
                      <td className="text-center">{d.cantidad}</td>
                      <td className="text-right">${d.precio_unitario.toFixed(2)}</td>
                      <td className="text-right font-bold">${d.subtotal.toFixed(2)}</td>
                    </tr>
                  ))}
                </tbody>
              </table>

              <div style={{ display: "flex", justifyContent: "flex-end", marginTop: 12 }}>
                <div style={{ minWidth: 200, textAlign: "right" }}>
                  <div className="flex justify-between" style={{ padding: "4px 0" }}>
                    <span className="text-secondary">Subtotal:</span>
                    <span>${verCompra.compra.subtotal.toFixed(2)}</span>
                  </div>
                  <div className="flex justify-between" style={{ padding: "4px 0" }}>
                    <span className="text-secondary">IVA:</span>
                    <span>${verCompra.compra.iva.toFixed(2)}</span>
                  </div>
                  <div className="flex justify-between" style={{ padding: "4px 0", borderTop: "1px solid var(--color-border)", fontWeight: 700, fontSize: 16 }}>
                    <span>Total:</span>
                    <span>${verCompra.compra.total.toFixed(2)}</span>
                  </div>
                </div>
              </div>
            </div>
          </div>
        </div>
      )}

      {/* Modal Importar XML */}
      {xmlPreview && (
        <div style={{
          position: "fixed", inset: 0, background: "rgba(0,0,0,0.6)", zIndex: 1000,
          display: "flex", justifyContent: "center", alignItems: "center",
        }} onClick={() => !xmlProcesando && setXmlPreview(null)}>
          <div className="card" style={{ width: "min(1100px, 95vw)", maxHeight: "90vh", overflow: "auto" }} onClick={(e) => e.stopPropagation()}>
            <div className="card-header flex justify-between items-center">
              <span>Importar factura XML (SRI)</span>
              <button className="btn btn-outline" style={{ padding: "2px 8px" }} onClick={() => setXmlPreview(null)} disabled={xmlProcesando}>x</button>
            </div>
            <div className="card-body">
              {/* Proveedor */}
              <div className="card mb-4" style={{ background: "rgba(255,255,255,0.03)" }}>
                <div className="card-body" style={{ padding: 12 }}>
                  <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: 12 }}>
                    <div>
                      <label className="text-secondary" style={{ fontSize: 12 }}>Proveedor del XML</label>
                      <div style={{ fontWeight: 600 }}>{xmlPreview.proveedor_nombre || "-"}</div>
                      <div className="text-secondary" style={{ fontSize: 12 }}>RUC: {xmlPreview.proveedor_ruc || "-"}</div>
                    </div>
                    <div>
                      <label className="text-secondary" style={{ fontSize: 12 }}>Proveedor a usar *</label>
                      <div style={{ display: "flex", gap: 6 }}>
                        <select
                          className="input"
                          value={xmlProveedorId}
                          style={{ flex: 1 }}
                          onChange={(e) => setXmlProveedorId(e.target.value ? Number(e.target.value) : "")}
                        >
                          <option value="">Seleccione proveedor...</option>
                          {proveedoresLista.map((p) => (
                            <option key={p.id} value={p.id}>{p.nombre}{p.ruc ? ` (${p.ruc})` : ""}</option>
                          ))}
                        </select>
                        {!xmlPreview.proveedor_existe && (
                          <button className="btn btn-outline" style={{ fontSize: 11, padding: "2px 10px" }} onClick={crearProveedorDesdeXml}>
                            + Crear
                          </button>
                        )}
                      </div>
                    </div>
                  </div>
                  <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr 1fr", gap: 12, marginTop: 10 }}>
                    <div>
                      <label className="text-secondary" style={{ fontSize: 12 }}>Numero factura</label>
                      <div>{xmlPreview.numero_factura || "-"}</div>
                    </div>
                    <div>
                      <label className="text-secondary" style={{ fontSize: 12 }}>Fecha emision</label>
                      <div>{xmlPreview.fecha_emision || "-"}</div>
                    </div>
                    <div>
                      <label className="text-secondary" style={{ fontSize: 12 }}>Clave de acceso</label>
                      <div className="text-secondary" style={{ fontSize: 11, wordBreak: "break-all" }}>{xmlPreview.clave_acceso || "-"}</div>
                    </div>
                  </div>
                </div>
              </div>

              {/* Items */}
              <div style={{ marginBottom: 10, fontWeight: 600 }}>Items ({xmlItems.length})</div>
              <div style={{ border: "1px solid var(--color-border)", borderRadius: "var(--radius)" }}>
                <table className="table" style={{ fontSize: 12, margin: 0 }}>
                  <thead>
                    <tr>
                      <th style={{ width: "30%" }}>Descripcion XML</th>
                      <th style={{ width: 80 }}>Cant.</th>
                      <th style={{ width: 80 }}>P. Unit.</th>
                      <th style={{ width: 60 }}>IVA %</th>
                      <th style={{ width: 80 }}>Subtotal</th>
                      <th style={{ width: 130 }}>Accion</th>
                      <th style={{ width: "30%" }}>Destino</th>
                    </tr>
                  </thead>
                  <tbody>
                    {xmlItems.map((it, idx) => (
                      <tr key={idx}>
                        <td>
                          <div style={{ fontWeight: 600 }}>{it.descripcion}</div>
                          {it.codigo_principal && (
                            <div className="text-secondary" style={{ fontSize: 11 }}>Cod: {it.codigo_principal}</div>
                          )}
                        </td>
                        <td className="text-center">{it.cantidad}</td>
                        <td className="text-right">${it.precio_unitario.toFixed(2)}</td>
                        <td className="text-center">{it.iva_porcentaje}%</td>
                        <td className="text-right">${it.subtotal.toFixed(2)}</td>
                        <td>
                          <select
                            className="input"
                            value={it.accion}
                            onChange={(e) => actualizarItemXml(idx, { accion: e.target.value as AccionItem })}
                            style={{ fontSize: 11, padding: "2px 4px" }}
                          >
                            <option value="producto_nuevo">Producto nuevo</option>
                            <option value="producto_existente">Producto existente</option>
                            <option value="gasto">Gasto</option>
                            <option value="ignorar">Ignorar</option>
                          </select>
                        </td>
                        <td style={{ position: "relative" }}>
                          {it.accion === "producto_nuevo" && (
                            <div style={{ display: "flex", flexDirection: "column", gap: 4 }}>
                              <input
                                className="input"
                                placeholder="Nombre producto"
                                value={it.producto_nuevo_nombre}
                                onChange={(e) => actualizarItemXml(idx, { producto_nuevo_nombre: e.target.value })}
                                style={{ fontSize: 11 }}
                              />
                              <div style={{ display: "flex", gap: 4 }}>
                                <input
                                  className="input"
                                  placeholder="Codigo (auto)"
                                  value={it.producto_nuevo_codigo}
                                  onChange={(e) => actualizarItemXml(idx, { producto_nuevo_codigo: e.target.value })}
                                  style={{ fontSize: 11, flex: 1 }}
                                />
                                <select
                                  className="input"
                                  value={it.producto_nuevo_categoria ?? ""}
                                  onChange={(e) => actualizarItemXml(idx, { producto_nuevo_categoria: e.target.value ? Number(e.target.value) : undefined })}
                                  style={{ fontSize: 11, flex: 1 }}
                                >
                                  <option value="">(Sin categoria)</option>
                                  {categoriasXml.map((c) => (
                                    <option key={c.id} value={c.id}>{c.nombre}</option>
                                  ))}
                                </select>
                              </div>
                            </div>
                          )}
                          {it.accion === "producto_existente" && (
                            <>
                              {it.producto_id ? (
                                <div style={{ display: "flex", alignItems: "center", gap: 6 }}>
                                  <span style={{ fontWeight: 600, fontSize: 11 }}>{it.producto_nombre}</span>
                                  <button
                                    className="btn btn-outline"
                                    style={{ fontSize: 10, padding: "0 6px" }}
                                    onClick={() => actualizarItemXml(idx, { producto_id: undefined, producto_nombre: undefined })}
                                  >
                                    x
                                  </button>
                                </div>
                              ) : (
                                <input
                                  className="input"
                                  placeholder="Buscar producto..."
                                  value={xmlBusquedaIdx === idx ? xmlBusquedaTexto : ""}
                                  onChange={(e) => buscarProductoExistenteXml(e.target.value, idx)}
                                  onFocus={() => { setXmlBusquedaIdx(idx); setXmlBusquedaTexto(""); }}
                                  onBlur={() => { setTimeout(() => { setXmlBusquedaIdx(null); setXmlResultadosBusqueda([]); }, 200); }}
                                  style={{ fontSize: 11 }}
                                />
                              )}
                              {xmlBusquedaIdx === idx && xmlResultadosBusqueda.length > 0 && (
                                <div style={{
                                  position: "absolute", top: "100%", left: 0, right: 0, zIndex: 50,
                                  background: "var(--color-surface)", border: "1px solid var(--color-border)",
                                  borderRadius: "var(--radius)", maxHeight: 180, overflowY: "auto",
                                  boxShadow: "0 4px 12px rgba(0,0,0,0.3)",
                                }}>
                                  {xmlResultadosBusqueda.map((p) => (
                                    <div
                                      key={p.id}
                                      style={{ padding: "6px 10px", cursor: "pointer", fontSize: 11, borderBottom: "1px solid var(--color-border)" }}
                                      onMouseDown={(e) => {
                                        e.preventDefault();
                                        actualizarItemXml(idx, { producto_id: p.id, producto_nombre: p.nombre });
                                        setXmlBusquedaIdx(null);
                                        setXmlResultadosBusqueda([]);
                                      }}
                                    >
                                      <div style={{ fontWeight: 600 }}>{p.nombre}</div>
                                      <div className="text-secondary" style={{ fontSize: 10 }}>
                                        {p.codigo ? `${p.codigo} | ` : ""}Stock: {p.stock_actual}
                                      </div>
                                    </div>
                                  ))}
                                </div>
                              )}
                            </>
                          )}
                          {it.accion === "gasto" && (
                            <input
                              className="input"
                              placeholder="Categoria gasto"
                              value={it.gasto_categoria}
                              onChange={(e) => actualizarItemXml(idx, { gasto_categoria: e.target.value })}
                              style={{ fontSize: 11 }}
                            />
                          )}
                          {it.accion === "ignorar" && (
                            <span className="text-secondary" style={{ fontSize: 11 }}>No se importa</span>
                          )}
                        </td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              </div>

              {/* Totales + forma pago */}
              <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: 16, marginTop: 16 }}>
                <div>
                  <div style={{ display: "grid", gridTemplateColumns: "auto 120px", gap: 8, alignItems: "end" }}>
                    <div>
                      <label className="text-secondary" style={{ fontSize: 12 }}>Forma de pago</label>
                      <select className="input" value={xmlFormaPago} onChange={(e) => setXmlFormaPago(e.target.value)}>
                        <option value="EFECTIVO">Efectivo</option>
                        <option value="TRANSFERENCIA">Transferencia</option>
                        <option value="DEBITO">Débito Bancario</option>
                        <option value="CREDITO">Credito</option>
                      </select>
                    </div>
                    {xmlFormaPago === "CREDITO" && (
                      <div>
                        <label className="text-secondary" style={{ fontSize: 12 }}>Dias credito</label>
                        <input className="input" type="number" min="1" value={xmlDiasCredito}
                          onChange={(e) => setXmlDiasCredito(e.target.value)} />
                      </div>
                    )}
                  </div>
                </div>
                <div style={{ minWidth: 220, textAlign: "right", justifySelf: "end" }}>
                  <div className="flex justify-between" style={{ padding: "2px 0" }}>
                    <span className="text-secondary">Subtotal 0%:</span>
                    <span>${xmlPreview.subtotal_0.toFixed(2)}</span>
                  </div>
                  <div className="flex justify-between" style={{ padding: "2px 0" }}>
                    <span className="text-secondary">Subtotal 15%:</span>
                    <span>${xmlPreview.subtotal_15.toFixed(2)}</span>
                  </div>
                  <div className="flex justify-between" style={{ padding: "2px 0" }}>
                    <span className="text-secondary">IVA:</span>
                    <span>${xmlPreview.iva.toFixed(2)}</span>
                  </div>
                  <div className="flex justify-between" style={{ padding: "4px 0", borderTop: "1px solid var(--color-border)", fontWeight: 700, fontSize: 16 }}>
                    <span>Total XML:</span>
                    <span>${xmlPreview.total.toFixed(2)}</span>
                  </div>
                </div>
              </div>

              {/* Botones */}
              <div className="flex gap-2 mt-4" style={{ justifyContent: "flex-end" }}>
                <button className="btn btn-outline" onClick={() => setXmlPreview(null)} disabled={xmlProcesando}>
                  Cancelar
                </button>
                <button className="btn btn-primary" onClick={handleProcesarXml} disabled={xmlProcesando}>
                  {xmlProcesando ? "Procesando..." : "Procesar Importacion"}
                </button>
              </div>
            </div>
          </div>
        </div>
      )}

      {/* Modales de confirmacion */}
      <Modal
        abierto={confirmarAnular !== null}
        titulo="Anular Compra"
        mensaje="¿Esta seguro que desea anular esta compra? Se revertira el stock de los productos."
        tipo="peligro"
        textoConfirmar="Si, anular"
        onConfirmar={handleAnularCompra}
        onCancelar={() => setConfirmarAnular(null)}
      />
      <Modal
        abierto={confirmarEliminarProv !== null}
        titulo="Eliminar Proveedor"
        mensaje="¿Esta seguro que desea eliminar este proveedor?"
        tipo="peligro"
        textoConfirmar="Si, eliminar"
        onConfirmar={handleEliminarProv}
        onCancelar={() => setConfirmarEliminarProv(null)}
      />
    </>
  );
}
