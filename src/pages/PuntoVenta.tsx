import { useState, useRef, useEffect, useCallback } from "react";
import { buscarProductos, productosMasVendidos, registrarVenta, buscarClientes, crearCliente, imprimirTicket, imprimirTicketPdf, obtenerCajaAbierta, alertasStockBajo, obtenerConfig, guardarConfig, emitirFacturaSri, consultarEstadoSri, cambiarAmbienteSri, enviarNotificacionSri, actualizarCliente, imprimirRide, procesarEmailsPendientes, resolverPrecioProducto, obtenerPreciosProducto, listarProductosTactil, listarCategorias, consultarIdentificacion, listarCuentasBanco, guardarBorrador, guardarCotizacion, guardarGuiaRemision, listarChoferes, guardarChofer, verificarPinAdmin, obtenerProducto, listarLotesProducto, listarComboGrupos, listarComboComponentes, listarListasPrecios } from "../services/api";
import type { AlertaStock } from "../services/api";
import { useToast } from "../components/Toast";
import { useNavigate } from "react-router-dom";
import ModalEmailCliente from "../components/ModalEmailCliente";
import PosGridTactil from "../components/PosGridTactil";
import { useSesion } from "../contexts/SesionContext";
import DocumentosRecientes from "../components/DocumentosRecientes";
import type { ProductoBusqueda, ProductoTactil, Categoria, ItemCarrito, NuevaVenta, VentaCompleta, Cliente, Caja, ResultadoEmision } from "../types";

export default function PuntoVenta() {
  const { toastExito, toastError, toastWarning } = useToast();
  const navigate = useNavigate();
  const { tienePermiso, esAdmin } = useSesion();
  const [busqueda, setBusqueda] = useState("");
  const [resultados, setResultados] = useState<ProductoBusqueda[]>([]);
  const [_favoritos, setFavoritos] = useState<ProductoBusqueda[]>([]);
  const [carrito, setCarrito] = useState<ItemCarrito[]>([]);
  const [montoRecibido, setMontoRecibido] = useState("");
  const [ventaCompletada, setVentaCompletada] = useState<VentaCompleta | null>(null);
  const [formaPago, setFormaPago] = useState("EFECTIVO");
  const [esFiado, setEsFiado] = useState(false);
  const [cajaAbierta, setCajaAbierta] = useState<Caja | null>(null);
  const [verificandoCaja, setVerificandoCaja] = useState(true);
  const [_alertas, setAlertas] = useState<AlertaStock[]>([]);
  const [_mostrarAlertas, _setMostrarAlertas] = useState(false);
  const [tipoDocumento, setTipoDocumento] = useState("NOTA_VENTA");
  const [regimen, setRegimen] = useState("RIMPE_POPULAR");
  const [sriModuloActivo, setSriModuloActivo] = useState(false);
  const [emitiendo, setEmitiendo] = useState(false);
  const [resultadoSri, setResultadoSri] = useState<ResultadoEmision | null>(null);
  const [mostrarModalEmail, setMostrarModalEmail] = useState(false);
  const [enviandoEmail, setEnviandoEmail] = useState(false);
  // Ambiente SRI
  const [sriAmbiente, setSriAmbiente] = useState("");
  const [sriAmbienteConfirmado, setSriAmbienteConfirmado] = useState(true);
  const [mostrarModalAmbiente, setMostrarModalAmbiente] = useState(false);
  const [cambiandoAmbiente, setCambiandoAmbiente] = useState(false);
  const [sriEmisionAutomatica, setSriEmisionAutomatica] = useState(false);
  const [ticketUsarPdf, setTicketUsarPdf] = useState(false);

  // Productos grid
  const [productosTactil, setProductosTactil] = useState<ProductoTactil[]>([]);
  // Permiso para cambiar lista de precios POR ITEM (modal al click en nombre/precio del carrito).
  // El cajero NO ve un selector global — la tarifa se cambia por cada item.
  const puedeCambiarListaPrecio = esAdmin || tienePermiso("cambiar_lista_precio");
  // Lista de TODAS las listas de precios activas (para los modales del item).
  // Aunque el producto no tenga precio especifico en una lista, se muestra y al
  // aplicar usa el precio_venta base del producto.
  const [todasListasPrecios, setTodasListasPrecios] = useState<Array<{ id: number; nombre: string; es_default?: boolean }>>([]);
  // Modal de cambiar precio/lista por item del carrito
  const [editarPrecioItemModal, setEditarPrecioItemModal] = useState<{
    idx: number;
    nombre: string;
    precioActual: number;
    preciosDisponibles: Array<{ lista_precio_id: number; lista_nombre: string; precio: number }>;
  } | null>(null);
  const [precioManualInput, setPrecioManualInput] = useState("");
  // Modal de detalles transferencia (cuenta + referencia + comprobante)
  const [mostrarDetallesTransfer, setMostrarDetallesTransfer] = useState(false);
  const [categoriasTactil, setCategoriasTactil] = useState<Categoria[]>([]);

  // Cliente
  const [clienteSeleccionado, setClienteSeleccionado] = useState<Cliente | null>(null);
  const [busquedaCliente, setBusquedaCliente] = useState("");
  const [clientesResultados, setClientesResultados] = useState<Cliente[]>([]);
  const [mostrarClientes, setMostrarClientes] = useState(false);
  const [mostrarCrearCliente, setMostrarCrearCliente] = useState(false);
  const [nuevoClienteNombre, setNuevoClienteNombre] = useState("");
  const [nuevoClienteId, setNuevoClienteId] = useState("");
  const [nuevoClienteTelefono, setNuevoClienteTelefono] = useState("");
  const [nuevoClienteEmail, setNuevoClienteEmail] = useState("");
  const [nuevoClienteDireccion, setNuevoClienteDireccion] = useState("");
  const [creandoCliente, setCreandoCliente] = useState(false);
  const [consultandoSri, setConsultandoSri] = useState(false);

  // Transferencia bancaria
  const [cuentasBanco, setCuentasBanco] = useState<{ id?: number; nombre: string; numero_cuenta?: string; tipo_cuenta?: string; activa?: boolean }[]>([]);
  const [bancoSeleccionado, setBancoSeleccionado] = useState<number | null>(null);
  const [referenciaPago, setReferenciaPago] = useState("");
  const [requiereReferencia, setRequiereReferencia] = useState(false);
  const [requiereComprobante, setRequiereComprobante] = useState(false);
  const [comprobanteImagen, setComprobanteImagen] = useState<string | null>(null);
  const [autoImprimirTicket, setAutoImprimirTicket] = useState(false);
  const [autoImprimirSri, setAutoImprimirSri] = useState(false);
  // Control de stock negativo (config 'stock_negativo_modo'):
  //   PERMITIR | BLOQUEAR | BLOQUEAR_OCULTAR
  const [stockModo, setStockModo] = useState<"PERMITIR" | "BLOQUEAR" | "BLOQUEAR_OCULTAR">("PERMITIR");

  // Panel documentos recientes
  const [mostrarRecientes, setMostrarRecientes] = useState(false);

  // Modal guía de remisión
  const [mostrarModalGuia, setMostrarModalGuia] = useState(false);
  const [guiaPlaca, setGuiaPlaca] = useState("");
  const [guiaChofer, setGuiaChofer] = useState("");
  const [guiaDireccion, setGuiaDireccion] = useState("");
  const [guardandoGuia, setGuardandoGuia] = useState(false);
  const [choferesGuardados, setChoferesGuardados] = useState<[number, string, string | null][]>([]);

  const inputRef = useRef<HTMLInputElement>(null);
  const lastAddRef = useRef<{id: number, time: number}>({id: 0, time: 0});

  // Auto-focus al campo de busqueda al cargar/montar el POS
  // (al entrar desde cualquier parte: sidebar, F1, redirect, etc.)
  useEffect(() => {
    const t = setTimeout(() => inputRef.current?.focus(), 100);
    return () => clearTimeout(t);
  }, []);

  // Modal detalle producto
  const [productoDetalle, setProductoDetalle] = useState<any | null>(null);

  // Admin PIN modal for price editing
  const [mostrarPinAdmin, setMostrarPinAdmin] = useState(false);
  const [pinAdminValor, setPinAdminValor] = useState("");
  const [pinAdminError, setPinAdminError] = useState("");
  const pinResolveRef = useRef<((ok: boolean) => void) | null>(null);

  // Modal info adicional
  const [infoAdicionalProductoId, setInfoAdicionalProductoId] = useState<number | null>(null);
  const [infoSerie, setInfoSerie] = useState("");
  const [infoLote, setInfoLote] = useState("");
  const [infoObservacion, setInfoObservacion] = useState("");

  // Modal descuento por item
  const [descuentoItemId, setDescuentoItemId] = useState<number | null>(null);
  const [descuentoTipo, setDescuentoTipo] = useState<"monto" | "porcentaje">("porcentaje");
  const [descuentoValor, setDescuentoValor] = useState("");

  // Modal seleccion de unidad (multi-unidad)
  const [seleccionUnidad, setSeleccionUnidad] = useState<{ producto: ProductoBusqueda; unidades: any[] } | null>(null);

  // Modal seleccion de lote (caducidad)
  const [seleccionLote, setSeleccionLote] = useState<{
    producto: ProductoBusqueda;
    unidadElegida?: any;
    lotes: any[];
  } | null>(null);
  // Modal cambiar lote de item ya en carrito
  const [cambiarLoteItem, setCambiarLoteItem] = useState<{ idx: number; lotes: any[] } | null>(null);
  // Cantidad a vender en el modal de seleccion de lote (default 1)
  const [seleccionLoteCantidad, setSeleccionLoteCantidad] = useState<string>("1");
  // Modal de seleccion de componentes para COMBO_FLEXIBLE
  const [seleccionCombo, setSeleccionCombo] = useState<{
    producto: ProductoBusqueda;
    unidadElegida?: any;
    grupos: any[];
    componentes: any[];
  } | null>(null);
  // Selecciones del combo flexible: { grupoId: { hijoId: cantidad } }
  const [comboSel, setComboSel] = useState<Record<string, Record<number, number>>>({});

  // Pago mixto: lista de pagos y modal para agregar
  const [pagosMixtos, setPagosMixtos] = useState<{ forma_pago: string; monto: number; banco_id?: number | null; referencia?: string | null }[]>([]);
  const [modoPagoMixto, setModoPagoMixto] = useState(false);
  const [mostrarAddPago, setMostrarAddPago] = useState(false);
  const [addPagoForma, setAddPagoForma] = useState<"EFECTIVO" | "TRANSFER" | "CREDITO" | "TARJETA">("EFECTIVO");
  const [addPagoMonto, setAddPagoMonto] = useState("");
  const [addPagoBancoId, setAddPagoBancoId] = useState<number | null>(null);
  const [addPagoReferencia, setAddPagoReferencia] = useState("");

  // Cart slide-in panel
  const [carritoAbierto, setCarritoAbierto] = useState(false);
  const [carritoManualCerrado, setCarritoManualCerrado] = useState(false);

  // Auto-open cart when items are added
  useEffect(() => {
    if (carrito.length > 0 && !carritoManualCerrado) {
      setCarritoAbierto(true);
    }
    if (carrito.length === 0) {
      setCarritoAbierto(false);
      setCarritoManualCerrado(false);
    }
  }, [carrito.length, carritoManualCerrado]);

  const solicitarPinAdmin = (): Promise<boolean> => {
    return new Promise((resolve) => {
      pinResolveRef.current = resolve;
      setPinAdminValor("");
      setPinAdminError("");
      setMostrarPinAdmin(true);
    });
  };

  // Las funciones reciben INDEX del array para soportar multiples items del mismo producto
  // con distintas unidades (ej. Cerveza UND y Cerveza SIXPACK como items separados)
  const editarPrecioItem = (idx: number, nuevoPrecio: number) => {
    if (nuevoPrecio < 0) return;
    setCarrito(prev => prev.map((i, k) =>
      k === idx
        ? { ...i, precio_unitario: nuevoPrecio, subtotal: i.cantidad * nuevoPrecio - i.descuento, lista_seleccionada: undefined }
        : i
    ));
  };

  const editarIvaItem = (idx: number, nuevoIva: number) => {
    setCarrito(prev => prev.map((i, k) =>
      k === idx
        ? { ...i, iva_porcentaje: nuevoIva, subtotal: i.cantidad * i.precio_unitario - i.descuento }
        : i
    ));
  };

  // Descuento por item: monto fijo o porcentaje sobre cantidad * precio_unitario
  const aplicarDescuentoItem = (idx: number, descuento: number) => {
    if (descuento < 0) return;
    setCarrito(prev => prev.map((i, k) =>
      k === idx
        ? { ...i, descuento, subtotal: i.cantidad * i.precio_unitario - descuento }
        : i
    ));
  };

  const cargarAlertas = useCallback(() => {
    alertasStockBajo().then(setAlertas).catch(() => {});
  }, []);

  useEffect(() => {
    inputRef.current?.focus();
    productosMasVendidos(12).then(setFavoritos).catch(() => {});
    obtenerCajaAbierta().then((c) => {
      setCajaAbierta(c);
      setVerificandoCaja(false);
    }).catch(() => setVerificandoCaja(false));
    cargarAlertas();
    listarCuentasBanco().then((cbs) => {
      setCuentasBanco(cbs);
      // Auto-seleccionar primera cuenta si no hay ninguna seleccionada
      if (cbs.length > 0 && !bancoSeleccionado) {
        setBancoSeleccionado(cbs[0].id ?? null);
      }
    }).catch(() => {});
    // Cargar regimen y estado ambiente confirmado
    obtenerConfig().then((cfg) => {
      if (cfg.regimen) {
        setRegimen(cfg.regimen);
        if (cfg.regimen !== "RIMPE_POPULAR") {
          setTipoDocumento("FACTURA");
        }
      }
      setSriAmbienteConfirmado(cfg.sri_ambiente_confirmado === "1");
      setSriEmisionAutomatica(cfg.sri_emision_automatica === "1");
      setTicketUsarPdf(cfg.ticket_usar_pdf === "1");
      setRequiereReferencia(cfg.transferencia_requiere_referencia === "1");
      setRequiereComprobante(cfg.transferencia_requiere_comprobante === "1");
      setAutoImprimirTicket(cfg.auto_imprimir === "1");
      setAutoImprimirSri(cfg.auto_imprimir_sri === "1");
      const modo = (cfg.stock_negativo_modo || "PERMITIR") as any;
      setStockModo(modo === "BLOQUEAR" || modo === "BLOQUEAR_OCULTAR" ? modo : "PERMITIR");
      // Cargar productos y categorias para grid
      listarProductosTactil().then(setProductosTactil).catch(() => {});
      listarCategorias().then(setCategoriasTactil).catch(() => {});
      listarListasPrecios().then((ls: any[]) => setTodasListasPrecios(ls.filter((l: any) => l.activo))).catch(() => {});
    }).catch(() => {});
    // Cargar estado SRI (incluyendo suscripcion y ambiente)
    consultarEstadoSri().then((estado) => {
      const tieneAcceso = estado.suscripcion_autorizada || estado.facturas_usadas < estado.facturas_gratis;
      setSriModuloActivo(estado.modulo_activo && estado.certificado_cargado && tieneAcceso);
      setSriAmbiente(estado.ambiente);
    }).catch(() => {});
  }, [cargarAlertas]);

  const handleBuscar = async (termino: string) => {
    setBusqueda(termino);
    if (termino.length >= 1) {
      const res = await buscarProductos(termino, clienteSeleccionado?.lista_precio_id);
      // Si hay exactamente 1 resultado y el término coincide EXACTAMENTE con el código o código de barras, agregar directamente
      if (res.length === 1 && (res[0].codigo === termino || res[0].codigo_barras === termino)) {
        const now = Date.now();
        if (lastAddRef.current.id === res[0].id && now - lastAddRef.current.time < 1000) return;
        agregarAlCarrito(res[0]);
        return;
      }
      setResultados(res);
    } else {
      setResultados([]);
    }
  };

  const handleBuscarCliente = async (termino: string) => {
    setBusquedaCliente(termino);
    if (termino.length >= 2) {
      setClientesResultados(await buscarClientes(termino));
    } else {
      setClientesResultados([]);
    }
  };

  const handleCrearClienteRapido = async () => {
    if (!nuevoClienteNombre.trim()) {
      toastError("Ingrese el nombre del cliente");
      return;
    }
    setCreandoCliente(true);
    try {
      const ident = nuevoClienteId.trim();
      let tipoId = "CONSUMIDOR_FINAL";
      if (ident.length === 13) tipoId = "RUC";
      else if (ident.length === 10) tipoId = "CEDULA";
      else if (ident.length > 0) tipoId = "PASAPORTE";

      const telefono = nuevoClienteTelefono.trim() || undefined;
      const email = nuevoClienteEmail.trim() || undefined;
      const direccion = nuevoClienteDireccion.trim() || undefined;

      const id = await crearCliente({
        tipo_identificacion: tipoId,
        identificacion: ident || undefined,
        nombre: nuevoClienteNombre.trim().toUpperCase(),
        telefono,
        email,
        direccion,
        activo: true,
      });
      const nuevoCliente: Cliente = {
        id,
        tipo_identificacion: tipoId,
        identificacion: ident || undefined,
        nombre: nuevoClienteNombre.trim().toUpperCase(),
        telefono,
        email,
        direccion,
        activo: true,
      };
      setClienteSeleccionado(nuevoCliente);
      setMostrarClientes(false);
      setMostrarCrearCliente(false);
      setNuevoClienteNombre("");
      setNuevoClienteId("");
      setNuevoClienteTelefono("");
      setNuevoClienteEmail("");
      setNuevoClienteDireccion("");
      setBusquedaCliente("");
      setClientesResultados([]);
      toastExito(`Cliente ${nuevoCliente.nombre} creado`);
    } catch (err) {
      const errorStr = String(err);
      if (errorStr.includes("UNIQUE") && nuevoClienteId.trim()) {
        try {
          const existentes = await buscarClientes(nuevoClienteId.trim());
          if (existentes.length >= 1) {
            setClienteSeleccionado(existentes[0]);
            setMostrarClientes(false);
            setMostrarCrearCliente(false);
            setNuevoClienteNombre("");
            setNuevoClienteId("");
            setNuevoClienteTelefono("");
            setNuevoClienteEmail("");
            setNuevoClienteDireccion("");
            setBusquedaCliente("");
            setClientesResultados([]);
            toastWarning(`Cliente ya existe: ${existentes[0].nombre}`);
          } else {
            toastError("Cliente ya existe con esa identificacion");
          }
        } catch {
          toastError("Cliente ya existe con esa identificacion");
        }
      } else {
        toastError("Error al crear cliente: " + err);
      }
    } finally {
      setCreandoCliente(false);
    }
  };

  const agregarAlCarrito = async (producto: ProductoBusqueda, unidadElegida?: any, loteElegido?: any, comboSeleccion?: Array<{ producto_hijo_id: number; cantidad: number; grupo_id?: number | null; nombre?: string }>) => {
    // Debounce para scanner de código de barras
    const now = Date.now();
    if (lastAddRef.current.id === producto.id && now - lastAddRef.current.time < 500) {
      setBusqueda(""); setResultados([]); inputRef.current?.focus();
      return;
    }
    lastAddRef.current = { id: producto.id, time: now };

    // Bloqueo por stock cuando config = BLOQUEAR | BLOQUEAR_OCULTAR
    // Excepciones: servicios y productos sin control de stock siempre se permiten.
    if (stockModo !== "PERMITIR") {
      try {
        const prodFull = await obtenerProducto(producto.id);
        const omiteStock = prodFull && (prodFull.es_servicio || (prodFull as any).no_controla_stock);
        if (!omiteStock) {
          const stockActual = Number(producto.stock_actual ?? prodFull?.stock_actual ?? 0);
          // Calcular cuanto ya esta en el carrito de este producto (todas las lineas)
          const factor = unidadElegida?.factor ?? 1;
          const cantNueva = factor; // 1 unidad de la presentacion seleccionada
          const yaEnCarrito = carrito
            .filter(it => it.producto_id === producto.id)
            .reduce((s, it) => s + (Number(it.cantidad) || 0) * (Number(it.factor_unidad) || 1), 0);
          if (yaEnCarrito + cantNueva > stockActual + 1e-9) {
            const disponible = Math.max(0, stockActual - yaEnCarrito);
            toastError(`Sin stock: ${producto.nombre}. Disponible: ${disponible.toFixed(2)}, ya en carrito: ${yaEnCarrito.toFixed(2)}.`);
            setBusqueda(""); setResultados([]); inputRef.current?.focus();
            return;
          }
        }
      } catch { /* si falla la validacion seguimos (fail-open para no romper) */ }
    }

    // Multi-unidad: si el producto tiene presentaciones y no se eligio una, mostrar selector
    if (!unidadElegida) {
      try {
        const { listarUnidadesProducto } = await import("../services/api");
        const unidades = await listarUnidadesProducto(producto.id);
        if (unidades.length > 0) {
          setSeleccionUnidad({ producto, unidades });
          setBusqueda(""); setResultados([]);
          return;
        }
      } catch { /* ignore - producto sin unidades */ }
    }

    // Combos: verificar tipo_producto y procesar
    try {
      const prodFull = await obtenerProducto(producto.id);
      const tp = (prodFull as any)?.tipo_producto;
      if (tp === "COMBO_FLEXIBLE") {
        // Abrir modal de seleccion de componentes
        const [grupos, componentes] = await Promise.all([
          listarComboGrupos(producto.id),
          listarComboComponentes(producto.id),
        ]);
        if (grupos.length === 0) {
          toastError("Combo flexible sin grupos configurados. Edite el producto para agregar grupos.");
          return;
        }
        // Inicializar seleccion vacia
        const initSel: Record<string, Record<number, number>> = {};
        grupos.forEach((g: any) => { initSel[String(g.id)] = {}; });
        setComboSel(initSel);
        setSeleccionCombo({ producto, unidadElegida, grupos, componentes });
        setBusqueda(""); setResultados([]);
        return;
      }
      // COMBO_FIJO: continua normal, el backend descuenta componentes.
      // Opcionalmente podriamos precargar componentes para mostrar en carrito.
    } catch { /* producto sin info combo, seguir */ }

    // Caducidad: si el producto requiere_caducidad y no se especifico lote, abrir selector
    if (!loteElegido) {
      try {
        const prodFull = await obtenerProducto(producto.id);
        if (prodFull && prodFull.requiere_caducidad) {
          const lotes = await listarLotesProducto(producto.id);
          const lotesConStock = lotes.filter((l: any) => l.cantidad > 0);
          if (lotesConStock.length > 0) {
            // Abrir modal para que escoja (con FEFO pre-seleccionado)
            setSeleccionLote({ producto, unidadElegida, lotes: lotesConStock });
            setBusqueda(""); setResultados([]);
            return;
          } else {
            toastWarning(`${producto.nombre}: sin lotes registrados. Agregue en Productos.`);
            // Continuar agregando sin lote (fallback a venta sin control de caducidad)
          }
        }
      } catch { /* producto sin caducidad, seguir */ }
    }

    // Calcular precio efectivo.
    // Prioridad:
    //   1) Unidad elegida (precio explicito de la presentacion)
    //   2) precio_lista del producto (ya viene resuelto si la busqueda recibio lista_precio_id)
    //   3) Resolver via cliente.lista_precio_id si tiene
    //   4) precio_venta default
    // El cambio de tarifa POR ITEM se hace despues, en el modal del item del carrito.
    let precioEfectivo: number;
    if (unidadElegida?.precio != null) {
      precioEfectivo = unidadElegida.precio;
    } else if (producto.precio_lista != null) {
      precioEfectivo = producto.precio_lista;
    } else if (clienteSeleccionado?.lista_precio_id) {
      try {
        precioEfectivo = await resolverPrecioProducto(producto.id, clienteSeleccionado.id ?? undefined);
      } catch {
        precioEfectivo = producto.precio_venta;
      }
    } else {
      precioEfectivo = producto.precio_venta;
    }

    // Check if already in cart MISMA unidad + MISMO lote
    const unidadId = unidadElegida?.id ?? null;
    const loteId = loteElegido?.id ?? null;
    // Cantidad a agregar: si vino del modal de lote con _cantidadVenta, usarla; sino 1
    const cantidadAAgregar = (loteElegido as any)?._cantidadVenta ?? 1;
    const existente = carrito.find((i) =>
      i.producto_id === producto.id
      && (i.unidad_id ?? null) === unidadId
      && (i.lote_id ?? null) === loteId
    );
    if (existente) {
      // Validar contra stock del lote antes de incrementar
      const loteCantDisp = (existente as any).lote_cantidad_disponible;
      if (existente.lote_id && typeof loteCantDisp === "number" && loteCantDisp > 0) {
        const factor = existente.factor_unidad ?? 1;
        const nuevaCantBase = (existente.cantidad + cantidadAAgregar) * factor;
        if (nuevaCantBase > loteCantDisp + 1e-9) {
          toastError(`El lote ${existente.lote_numero || "#" + existente.lote_id} solo tiene ${loteCantDisp} unidades disponibles. Ya tienes ${existente.cantidad * factor} en carrito.`);
          setBusqueda(""); setResultados([]); inputRef.current?.focus();
          return;
        }
      }
      setCarrito((prev) =>
        prev.map((i) =>
          (i.producto_id === producto.id
            && (i.unidad_id ?? null) === unidadId
            && (i.lote_id ?? null) === loteId)
            ? { ...i, cantidad: i.cantidad + cantidadAAgregar, subtotal: (i.cantidad + cantidadAAgregar) * i.precio_unitario - i.descuento }
            : i
        )
      );
    } else {
      // Fetch available prices for this product
      let preciosDisponibles: { lista_precio_id: number; lista_nombre: string; precio: number }[] = [];
      try {
        preciosDisponibles = await obtenerPreciosProducto(producto.id);
      } catch { /* ignore */ }

      // Determine which list is currently selected
      let listaSel: string | undefined;
      if (producto.precio_lista != null && preciosDisponibles.length > 0) {
        const match = preciosDisponibles.find(p => Math.abs(p.precio - precioEfectivo) < 0.001);
        listaSel = match?.lista_nombre;
      }

      // Calcular dias restantes del lote (si aplica)
      const diasRestantes = loteElegido?.fecha_caducidad
        ? Math.floor((new Date(loteElegido.fecha_caducidad).getTime() - new Date().getTime()) / (1000 * 60 * 60 * 24))
        : undefined;

      setCarrito((prev) => [
        ...prev,
        {
          producto_id: producto.id,
          codigo: producto.codigo ?? undefined,
          nombre: unidadElegida ? `${producto.nombre} (${unidadElegida.abreviatura || unidadElegida.nombre})` : producto.nombre,
          cantidad: cantidadAAgregar,
          precio_unitario: precioEfectivo,
          descuento: 0,
          iva_porcentaje: producto.iva_porcentaje,
          incluye_iva: producto.incluye_iva ?? false,
          subtotal: precioEfectivo * cantidadAAgregar,
          stock_disponible: producto.stock_actual,
          stock_minimo: producto.stock_minimo,
          precio_base: producto.precio_venta,
          precios_disponibles: preciosDisponibles,
          lista_seleccionada: listaSel,
          unidad_id: unidadElegida?.id,
          unidad_nombre: unidadElegida?.nombre,
          factor_unidad: unidadElegida?.factor,
          lote_id: loteElegido?.id,
          lote_numero: loteElegido?.lote,
          lote_fecha_caducidad: loteElegido?.fecha_caducidad,
          lote_dias_restantes: diasRestantes,
          // Cantidad disponible del lote al momento de agregar (para validar incrementos en carrito)
          lote_cantidad_disponible: loteElegido?.cantidad,
          // Combo: selección de componentes (solo COMBO_FLEXIBLE)
          combo_seleccion: comboSeleccion && comboSeleccion.length > 0 ? comboSeleccion : undefined,
        } as any,
      ]);
    }
    setBusqueda("");
    setResultados([]);
    inputRef.current?.focus();

    // Aviso si el lote elegido esta por vencer
    if (loteElegido?.fecha_caducidad) {
      const dias = Math.floor((new Date(loteElegido.fecha_caducidad).getTime() - new Date().getTime()) / (1000 * 60 * 60 * 24));
      if (dias < 0) {
        toastWarning(`⚠ Lote ${loteElegido.lote || ""} esta VENCIDO (hace ${Math.abs(dias)} dias)`);
      } else if (dias <= 7) {
        toastWarning(`🕐 Lote ${loteElegido.lote || ""} vence en ${dias} dia(s)`);
      }
    }
  };

  const actualizarCantidad = (idx: number, cantidad: number) => {
    if (cantidad <= 0) {
      setCarrito((prev) => prev.filter((_, k) => k !== idx));
      return;
    }
    // Validar contra stock del lote si el item tiene lote asignado
    const item = carrito[idx] as any;
    const loteCantDisp = item?.lote_cantidad_disponible;
    if (item?.lote_id && typeof loteCantDisp === "number" && loteCantDisp > 0) {
      const factor = item.factor_unidad ?? 1;
      const cantidadBase = cantidad * factor;
      if (cantidadBase > loteCantDisp + 1e-9) {
        const maxPosibleEnUnidades = Math.floor(loteCantDisp / factor);
        toastError(`El lote ${item.lote_numero || "#" + item.lote_id} solo tiene ${loteCantDisp} unidades disponibles${factor > 1 ? ` (máx ${maxPosibleEnUnidades} en esta presentación)` : ""}. Para vender más, agregue otra línea con lote diferente o "Sin lote".`);
        return;
      }
    }
    // Validar contra stock_disponible (general) si stockModo bloqueante
    if (stockModo !== "PERMITIR" && item) {
      const omiteStock = (item as any).combo_seleccion || (item.stock_disponible == null);
      if (!omiteStock) {
        const factor = item.factor_unidad ?? 1;
        const cantidadBase = cantidad * factor;
        // Sumar lo que ya esta en el carrito de OTROS items del mismo producto
        const otroEnCarrito = carrito.reduce((s, it, k) => k === idx ? s : (it.producto_id === item.producto_id ? s + (Number(it.cantidad) || 0) * (Number(it.factor_unidad) || 1) : s), 0);
        if (otroEnCarrito + cantidadBase > (item.stock_disponible || 0) + 1e-9) {
          toastError(`Sin stock: ${item.nombre}. Disponible: ${item.stock_disponible}, ya en otras líneas: ${otroEnCarrito.toFixed(2)}.`);
          return;
        }
      }
    }
    setCarrito((prev) =>
      prev.map((i, k) =>
        k === idx
          ? { ...i, cantidad, subtotal: cantidad * i.precio_unitario - i.descuento }
          : i
      )
    );
  };

  const eliminarItem = (idx: number) => {
    setCarrito((prev) => prev.filter((_, k) => k !== idx));
  };

  // Cálculo correcto considerando si el precio del producto YA incluye IVA o no
  const subtotal = carrito.reduce((sum, i) => {
    if (i.incluye_iva && i.iva_porcentaje > 0) {
      // Desglosar: el subtotal ya incluye IVA, restamos para obtener base
      return sum + i.subtotal / (1 + i.iva_porcentaje / 100);
    }
    return sum + i.subtotal;
  }, 0);
  const iva = carrito.reduce((sum, i) => {
    if (i.iva_porcentaje === 0) return sum;
    if (i.incluye_iva) {
      const base = i.subtotal / (1 + i.iva_porcentaje / 100);
      return sum + (i.subtotal - base);
    }
    return sum + i.subtotal * (i.iva_porcentaje / 100);
  }, 0);
  const total = subtotal + iva;
  const cambio = parseFloat(montoRecibido || "0") - total;

  const procesarVenta = useCallback(async () => {
    if (carrito.length === 0) return;
    if (!cajaAbierta) {
      toastError("Debe abrir la caja antes de realizar ventas");
      return;
    }
    // Validar cuenta bancaria seleccionada en transferencia
    if (!esFiado && formaPago === "TRANSFER" && !bancoSeleccionado && cuentasBanco.length > 0) {
      toastError("Seleccione una cuenta bancaria para la transferencia");
      return;
    }
    // Validar referencia obligatoria en transferencia (no aplica si es crédito)
    if (!esFiado && formaPago === "TRANSFER" && requiereReferencia && !referenciaPago.trim()) {
      toastError("El numero de referencia es obligatorio para transferencias");
      return;
    }
    // Validar comprobante obligatorio en transferencia
    if (!esFiado && formaPago === "TRANSFER" && requiereComprobante && !comprobanteImagen) {
      toastError("El comprobante de transferencia es obligatorio");
      return;
    }
    // Verificar que el usuario ha confirmado el ambiente SRI
    if (tipoDocumento === "FACTURA" && sriModuloActivo && !sriAmbienteConfirmado) {
      setMostrarModalAmbiente(true);
      return;
    }
    // Validar suscripcion SRI antes de emitir FACTURA
    if (tipoDocumento === "FACTURA") {
      try {
        const estado = await consultarEstadoSri();
        const gratis = estado.facturas_gratis;
        const usadas = estado.facturas_usadas;

        // Dentro del trial gratis: siempre permitir
        if (usadas >= gratis) {
          // Trial agotado — verificar suscripcion
          if (!estado.suscripcion_autorizada) {
            toastError(estado.suscripcion_mensaje || `Ha alcanzado el limite de ${gratis} facturas gratis. Adquiera una suscripcion en Configuracion.`);
            return;
          }
          // Verificar segun plan
          if (estado.suscripcion_plan === "paquete" && estado.suscripcion_docs_restantes != null && estado.suscripcion_docs_restantes <= 0) {
            toastError("Ha agotado los documentos de su paquete. Adquiera un nuevo paquete.");
            return;
          }
          const hoy = new Date().toISOString().slice(0, 10);
          if (["mensual", "semestral", "anual"].includes(estado.suscripcion_plan) && estado.suscripcion_hasta && estado.suscripcion_hasta < hoy) {
            toastError(`Su suscripcion SRI (${estado.suscripcion_plan}) expiro el ${estado.suscripcion_hasta}. Renueve en Configuracion.`);
            return;
          }
        }
      } catch {
        // Si no se puede verificar, el backend enforcera de todas formas
      }
    }

    // Helper: si el item tiene incluye_iva=true, desglosa precio Y descuento antes de enviar al backend
    // IMPORTANTE: redondear precio_unitario a 2 decimales (limite del XSD del SRI)
    // y recalcular subtotal con el precio redondeado para mantener consistencia.
    const r2 = (n: number) => Math.round(n * 100) / 100;
    const desglosar = (i: typeof carrito[0]) => {
      if (i.incluye_iva && i.iva_porcentaje > 0) {
        const factor = 1 + i.iva_porcentaje / 100;
        const precioBase = r2(i.precio_unitario / factor);
        const descBase = r2(i.descuento / factor);
        const subtotalBase = r2(i.cantidad * precioBase - descBase);
        return { precio_unitario: precioBase, descuento: descBase, subtotal: subtotalBase };
      }
      return {
        precio_unitario: r2(i.precio_unitario),
        descuento: r2(i.descuento),
        subtotal: r2(i.cantidad * r2(i.precio_unitario) - r2(i.descuento)),
      };
    };

    // Validacion de pago mixto antes de enviar
    if (modoPagoMixto && pagosMixtos.length > 0) {
      const sumaPagos = pagosMixtos.reduce((s, p) => s + p.monto, 0);
      if (Math.abs(sumaPagos - total) > 0.02) {
        toastError(`La suma de pagos ($${sumaPagos.toFixed(2)}) no coincide con el total ($${total.toFixed(2)})`);
        return;
      }
      // Si hay pago tipo CREDITO pero no hay cliente, error
      const tieneCredito = pagosMixtos.some(p => p.forma_pago === "CREDITO");
      if (tieneCredito && (!clienteSeleccionado || clienteSeleccionado.id === 1)) {
        toastError("Para pago mixto con CREDITO seleccione un cliente identificado");
        return;
      }
    }

    const usarMixto = modoPagoMixto && pagosMixtos.length > 0;

    const nuevaVenta: NuevaVenta = {
      cliente_id: clienteSeleccionado?.id ?? 1,
      items: carrito.map((i) => {
        const d = desglosar(i);
        return {
          producto_id: i.producto_id,
          cantidad: i.cantidad,
          precio_unitario: d.precio_unitario,
          descuento: d.descuento,
          iva_porcentaje: i.iva_porcentaje,
          subtotal: d.subtotal,
          info_adicional: i.info_adicional || null,
          unidad_id: i.unidad_id ?? null,
          unidad_nombre: i.unidad_nombre ?? null,
          factor_unidad: i.factor_unidad ?? null,
          lote_id: i.lote_id ?? null,
          combo_seleccion: (i as any).combo_seleccion ?? null,
        } as any;
      }),
      forma_pago: usarMixto ? "MIXTO" : formaPago,
      monto_recibido: usarMixto ? total : (esFiado ? 0 : parseFloat(montoRecibido || "0")),
      descuento: 0,
      tipo_documento: tipoDocumento,
      // es_fiado solo cuando NO es mixto y es CREDITO; en mixto, el credito se maneja por pagos[]
      es_fiado: usarMixto ? false : esFiado,
      banco_id: usarMixto ? null : (formaPago === "TRANSFER" ? bancoSeleccionado : null),
      referencia_pago: usarMixto ? null : (formaPago === "TRANSFER" ? (referenciaPago.trim() || null) : null),
      comprobante_imagen: usarMixto ? null : (formaPago === "TRANSFER" ? (comprobanteImagen || null) : null),
      pagos: usarMixto ? pagosMixtos.map(p => ({
        forma_pago: p.forma_pago,
        monto: p.monto,
        banco_id: p.banco_id ?? null,
        referencia: p.referencia ?? null,
        comprobante_imagen: null,
      })) : undefined,
    };

    try {
      const resultado = await registrarVenta(nuevaVenta);
      setVentaCompletada(resultado);
      setCarrito([]);
      setMontoRecibido("");
      setFormaPago("EFECTIVO");
      setEsFiado(false);
      setClienteSeleccionado(null);
      setBancoSeleccionado(null);
      setReferenciaPago("");
      setComprobanteImagen(null);
      setPagosMixtos([]);
      setModoPagoMixto(false);
      // IMPRESION AUTOMATICA AL REGISTRAR: si esta activa, imprime ANTES de enviar al SRI
      // Esto asegura que el cliente se lleve su ticket sin esperar al SRI
      if (autoImprimirTicket && resultado.venta.id) {
        const fn = ticketUsarPdf ? imprimirTicketPdf : imprimirTicket;
        fn(resultado.venta.id).catch(() => {});
      }

      // Si fue FACTURA, modulo SRI activo y emision automatica activada, emitir al SRI
      let ventaAutorizada = false;
      if (tipoDocumento === "FACTURA" && sriModuloActivo && sriEmisionAutomatica && resultado.venta.id) {
        setEmitiendo(true);
        try {
          const res = await emitirFacturaSri(resultado.venta.id);
          setResultadoSri(res);
          if (res.exito) {
            ventaAutorizada = true;
            toastExito("Factura autorizada por el SRI");
            // Actualizar ventaCompletada con numero_factura y estado SRI
            setVentaCompletada(prev => prev ? {
              ...prev,
              venta: {
                ...prev.venta,
                estado_sri: "AUTORIZADA",
                numero_factura: res.numero_factura,
                clave_acceso: res.clave_acceso,
                autorizacion_sri: res.numero_autorizacion,
              }
            } : prev);
            // Disparar evento para refrescar banner de suscripcion
            window.dispatchEvent(new CustomEvent("sri-factura-emitida"));
            // Auto-enviar email si el cliente tiene email
            const cli = clienteSeleccionado;
            if (cli?.email && cli.email.trim()) {
              enviarNotificacionSri(resultado.venta.id, cli.email)
                .then(() => toastExito(`Email enviado a ${cli.email}`))
                .catch((err) => {
                  const errStr = String(err);
                  if (errStr.startsWith("ENCOLADO:")) {
                    toastWarning("Email pendiente, se reintentara automaticamente");
                  } else {
                    toastWarning("No se pudo enviar email: " + errStr);
                  }
                });
            } else if (cli && cli.id !== 1) {
              // Cliente sin email: mostrar modal para ingresar
              setMostrarModalEmail(true);
            }
          } else {
            toastWarning(`SRI: ${res.mensaje}`);
          }
        } catch (err) {
          toastWarning("Error enviando al SRI: " + err);
        } finally {
          setEmitiendo(false);
        }
      }

      // IMPRESION AUTOMATICA AL AUTORIZAR SRI: solo si se autorizo con exito
      if (autoImprimirSri && ventaAutorizada && resultado.venta.id) {
        const fn = ticketUsarPdf ? imprimirTicketPdf : imprimirTicket;
        fn(resultado.venta.id).catch(() => {});
      }
      setTipoDocumento(regimen !== "RIMPE_POPULAR" ? "FACTURA" : "NOTA_VENTA");
      // Refrescar favoritos, alertas de stock y productos grid
      productosMasVendidos(12).then(setFavoritos).catch(() => {});
      listarProductosTactil().then(setProductosTactil).catch(() => {});
      alertasStockBajo().then((a) => {
        setAlertas(a);
        if (a.length > 0) {
          const items = a.slice(0, 3).map((p) => `${p.nombre} (${p.stock_actual})`).join(", ");
          toastWarning(`Stock bajo: ${items}${a.length > 3 ? ` y ${a.length - 3} mas...` : ""}`);
        }
      }).catch(() => {});
    } catch (err) {
      toastError("Error al registrar venta: " + err);
    }
  }, [carrito, cajaAbierta, clienteSeleccionado, formaPago, montoRecibido, esFiado, tipoDocumento, sriModuloActivo, sriEmisionAutomatica, regimen, autoImprimirTicket, autoImprimirSri, ticketUsarPdf, requiereComprobante, comprobanteImagen, toastError, toastExito, toastWarning]);

  const nuevaVentaClick = useCallback(() => {
    setVentaCompletada(null);
    setResultadoSri(null);
    setMostrarModalEmail(false);
    setCarrito([]);
    setMontoRecibido("");
    setFormaPago("EFECTIVO");
    setEsFiado(false);
    setClienteSeleccionado(null);
    setPagosMixtos([]);
    setModoPagoMixto(false);
    setTipoDocumento(regimen !== "RIMPE_POPULAR" ? "FACTURA" : "NOTA_VENTA");
    inputRef.current?.focus();
  }, [regimen]);

  // Recalcular precios del carrito al cambiar de cliente
  const recalcularPreciosCarrito = useCallback(async (clienteId: number | null) => {
    if (carrito.length === 0) return;
    const nuevoCarrito = await Promise.all(
      carrito.map(async (item) => {
        try {
          const nuevoPrecio = await resolverPrecioProducto(item.producto_id, clienteId ?? undefined);
          // Determine which list matches the new price
          let listaSel: string | undefined;
          if (item.precios_disponibles) {
            const match = item.precios_disponibles.find(p => Math.abs(p.precio - nuevoPrecio) < 0.001);
            listaSel = match?.lista_nombre;
          }
          return {
            ...item,
            precio_unitario: nuevoPrecio,
            subtotal: item.cantidad * nuevoPrecio - item.descuento,
            lista_seleccionada: listaSel,
          };
        } catch {
          return item;
        }
      })
    );
    setCarrito(nuevoCarrito);
  }, [carrito]);

  // Escuchar F9 (cobrar) y F10 (nueva venta) via CustomEvent
  const guardarComoDocumento = useCallback(async (tipo: "borrador" | "cotizacion") => {
    if (carrito.length === 0) return;
    const r2 = (n: number) => Math.round(n * 100) / 100;
    const desglosar = (i: typeof carrito[0]) => {
      if (i.incluye_iva && i.iva_porcentaje > 0) {
        const factor = 1 + i.iva_porcentaje / 100;
        const pBase = r2(i.precio_unitario / factor);
        const dBase = r2(i.descuento / factor);
        return { precio_unitario: pBase, descuento: dBase, subtotal: r2(i.cantidad * pBase - dBase) };
      }
      return { precio_unitario: r2(i.precio_unitario), descuento: r2(i.descuento), subtotal: r2(i.cantidad * r2(i.precio_unitario) - r2(i.descuento)) };
    };
    const nueva: NuevaVenta = {
      cliente_id: clienteSeleccionado?.id ?? 1,
      items: carrito.map(i => {
        const d = desglosar(i);
        return { producto_id: i.producto_id, cantidad: i.cantidad, precio_unitario: d.precio_unitario, descuento: d.descuento, iva_porcentaje: i.iva_porcentaje, subtotal: d.subtotal, info_adicional: i.info_adicional || null } as any;
      }),
      forma_pago: formaPago, monto_recibido: 0, descuento: 0,
      tipo_documento: tipoDocumento, es_fiado: false,
    };
    try {
      if (tipo === "borrador") {
        await guardarBorrador(nueva);
        toastExito("Borrador guardado");
      } else {
        const res = await guardarCotizacion(nueva);
        toastExito(`Cotizacion ${res.venta.numero} creada`);
      }
      setCarrito([]); setMontoRecibido("");
    } catch (err) { toastError("Error: " + err); }
  }, [carrito, clienteSeleccionado, formaPago, tipoDocumento, toastExito, toastError]);

  const handleGuiaRemision = useCallback(() => {
    if (carrito.length === 0) return;
    // Prellenar dirección del cliente
    setGuiaDireccion(clienteSeleccionado?.direccion || "");
    // Cargar choferes guardados
    listarChoferes().then(setChoferesGuardados).catch(() => {});
    setMostrarModalGuia(true);
  }, [carrito, clienteSeleccionado]);

  const confirmarGuiaRemision = useCallback(async () => {
    if (carrito.length === 0) return;
    setGuardandoGuia(true);
    const r2g = (n: number) => Math.round(n * 100) / 100;
    const desglosar2 = (i: typeof carrito[0]) => {
      if (i.incluye_iva && i.iva_porcentaje > 0) {
        const factor = 1 + i.iva_porcentaje / 100;
        const pBase = r2g(i.precio_unitario / factor);
        const dBase = r2g(i.descuento / factor);
        return { precio_unitario: pBase, descuento: dBase, subtotal: r2g(i.cantidad * pBase - dBase) };
      }
      return { precio_unitario: r2g(i.precio_unitario), descuento: r2g(i.descuento), subtotal: r2g(i.cantidad * r2g(i.precio_unitario) - r2g(i.descuento)) };
    };
    const nueva: NuevaVenta = {
      cliente_id: clienteSeleccionado?.id ?? 1,
      items: carrito.map((i) => {
        const d = desglosar2(i);
        return {
          producto_id: i.producto_id,
          cantidad: i.cantidad,
          precio_unitario: d.precio_unitario,
          descuento: d.descuento,
          iva_porcentaje: i.iva_porcentaje,
          subtotal: d.subtotal,
          info_adicional: i.info_adicional || null,
        } as any;
      }),
      forma_pago: formaPago,
      monto_recibido: 0,
      descuento: 0,
      tipo_documento: tipoDocumento,
      es_fiado: false,
      guia_placa: guiaPlaca.trim() || null,
      guia_chofer: guiaChofer.trim() || null,
      guia_direccion_destino: guiaDireccion.trim() || null,
    };
    try {
      const res = await guardarGuiaRemision(nueva);
      toastExito(`Guia ${res.venta.numero} creada - stock descontado`);
      // Guardar chofer para autocompletar futuro
      if (guiaChofer.trim()) {
        guardarChofer(guiaChofer.trim(), guiaPlaca.trim() || undefined).catch(() => {});
      }
      setCarrito([]);
      setMontoRecibido("");
      setFormaPago("EFECTIVO");
      setEsFiado(false);
      setClienteSeleccionado(null);
      setMostrarModalGuia(false);
      setGuiaPlaca(""); setGuiaChofer(""); setGuiaDireccion("");
    } catch (err) {
      toastError("Error: " + err);
    } finally {
      setGuardandoGuia(false);
    }
  }, [carrito, clienteSeleccionado, formaPago, tipoDocumento, guiaPlaca, guiaChofer, guiaDireccion, toastExito, toastError]);

  useEffect(() => {
    const handleCobrar = () => procesarVenta();
    const handleNuevaVenta = () => nuevaVentaClick();
    const handleBorrador = () => guardarComoDocumento("borrador");
    const handleCotizacion = () => guardarComoDocumento("cotizacion");
    const handleGuia = () => handleGuiaRemision();
    const handleMontoExacto = () => {
      // Solo si forma de pago es EFECTIVO y carrito tiene items
      if (carrito.length === 0 || esFiado || formaPago !== "EFECTIVO") return;
      setMontoRecibido(total.toFixed(2));
      setTimeout(() => procesarVenta(), 100);
    };
    window.addEventListener("pos-cobrar", handleCobrar);
    window.addEventListener("pos-nueva-venta", handleNuevaVenta);
    window.addEventListener("pos-guardar-borrador", handleBorrador);
    window.addEventListener("pos-guardar-cotizacion", handleCotizacion);
    window.addEventListener("pos-guardar-guia", handleGuia);
    window.addEventListener("pos-monto-exacto", handleMontoExacto);
    const handleRecientes = () => setMostrarRecientes(true);
    const handleGuiaRemisionEvt = () => setMostrarModalGuia(true);
    window.addEventListener("pos-recientes", handleRecientes);
    window.addEventListener("pos-guia-remision", handleGuiaRemisionEvt);
    return () => {
      window.removeEventListener("pos-cobrar", handleCobrar);
      window.removeEventListener("pos-nueva-venta", handleNuevaVenta);
      window.removeEventListener("pos-guardar-borrador", handleBorrador);
      window.removeEventListener("pos-guardar-cotizacion", handleCotizacion);
      window.removeEventListener("pos-guardar-guia", handleGuia);
      window.removeEventListener("pos-monto-exacto", handleMontoExacto);
      window.removeEventListener("pos-recientes", handleRecientes);
      window.removeEventListener("pos-guia-remision", handleGuiaRemisionEvt);
    };
  }, [procesarVenta, nuevaVentaClick, guardarComoDocumento, handleGuiaRemision, carrito.length, esFiado, formaPago, total]);

  // Background: procesar emails pendientes cada 60 segundos
  useEffect(() => {
    const intervalo = setInterval(() => {
      procesarEmailsPendientes()
        .then((res) => {
          if (res.enviados > 0) {
            toastExito(`${res.enviados} email(s) pendiente(s) enviado(s)`);
          }
        })
        .catch(() => {}); // silencioso si falla
    }, 60_000);
    return () => clearInterval(intervalo);
  }, [toastExito]);

  const handleEnviarEmailModal = async (emailInput: string) => {
    if (!ventaCompletada?.venta.id) return;
    setEnviandoEmail(true);
    try {
      // Guardar email en el cliente
      if (clienteSeleccionado?.id && clienteSeleccionado.id !== 1) {
        await actualizarCliente({ ...clienteSeleccionado, email: emailInput });
        setClienteSeleccionado(prev => prev ? { ...prev, email: emailInput } : prev);
      }

      // Si la factura aún no está autorizada, autorizar primero
      if (ventaCompletada.venta.tipo_documento === "FACTURA" && ventaCompletada.venta.estado_sri !== "AUTORIZADA") {
        setMostrarModalEmail(false);
        setEmitiendo(true);
        try {
          const res = await emitirFacturaSri(ventaCompletada.venta.id);
          setResultadoSri(res);
          if (res.exito) {
            toastExito("Factura autorizada por el SRI");
            setVentaCompletada(prev => prev ? {
              ...prev,
              venta: { ...prev.venta, estado_sri: "AUTORIZADA", numero_factura: res.numero_factura, clave_acceso: res.clave_acceso, autorizacion_sri: res.numero_autorizacion }
            } : prev);
            window.dispatchEvent(new CustomEvent("sri-factura-emitida"));
            // Enviar notificación con el email recién guardado
            await enviarNotificacionSri(ventaCompletada.venta.id, emailInput);
            toastExito(`Email enviado a ${emailInput}`);
          } else {
            toastWarning(`SRI: ${res.mensaje}`);
          }
        } catch (err) {
          toastWarning("Error enviando al SRI: " + err);
        } finally {
          setEmitiendo(false);
        }
      } else {
        // Ya autorizada, solo enviar notificación
        await enviarNotificacionSri(ventaCompletada.venta.id, emailInput);
        toastExito(`Email enviado a ${emailInput}`);
        setMostrarModalEmail(false);
      }
    } catch (err) {
      toastError("Error: " + err);
    } finally {
      setEnviandoEmail(false);
    }
  };

  // Vista de venta completada
  if (ventaCompletada) {
    const esFacturaAutorizada = resultadoSri?.exito && ventaCompletada.venta.tipo_documento === "FACTURA";

    return (
      <>
        <div className="page-header">
          <h2>Venta Completada</h2>
        </div>
        <div className="page-body">
          <div className="card" style={{ maxWidth: 500, margin: "0 auto", textAlign: "center" }}>
            <div className="card-body">
              <div style={{ fontSize: 48, marginBottom: 16 }}>OK</div>
              <h3>Venta #{ventaCompletada.venta.numero}{ventaCompletada.venta.numero_factura && ` | Factura ${ventaCompletada.venta.numero_factura}`}</h3>
              <p className="text-secondary mt-2">
                {ventaCompletada.detalles.length} producto(s)
                {ventaCompletada.cliente_nombre && ` - ${ventaCompletada.cliente_nombre}`}
              </p>
              <div className="text-xl font-bold mt-4" style={{ color: "var(--color-success)" }}>
                Total: ${ventaCompletada.venta.total.toFixed(2)}
              </div>
              {ventaCompletada.venta.cambio > 0 && (
                <p className="mt-2">
                  Cambio: <strong>${ventaCompletada.venta.cambio.toFixed(2)}</strong>
                </p>
              )}
              {emitiendo && (
                <div style={{ marginTop: 12, color: "var(--color-primary)", fontSize: 13 }}>
                  Enviando factura al SRI...
                </div>
              )}
              {esFacturaAutorizada && (
                <div style={{
                  marginTop: 12, padding: "8px 12px", borderRadius: "var(--radius)",
                  background: "rgba(34, 197, 94, 0.15)", color: "var(--color-success)", fontSize: 13,
                }}>
                  Factura autorizada por el SRI
                </div>
              )}
              <div className="flex gap-2 mt-4" style={{ justifyContent: "center", flexWrap: "wrap" }}>
                <button className="btn btn-outline btn-lg"
                  onClick={() => {
                    if (ventaCompletada.venta.id) {
                      const fn = ticketUsarPdf ? imprimirTicketPdf : imprimirTicket;
                      fn(ventaCompletada.venta.id)
                        .then(() => toastExito(ticketUsarPdf ? "Ticket PDF generado" : "Ticket impreso"))
                        .catch((e) => toastError("Error al imprimir: " + e));
                    }
                  }}>
                  {ticketUsarPdf ? "Ver Ticket" : "Imprimir Ticket"}
                </button>

                {/* Botón Autorizar SRI - solo si es factura, SRI activo, y aún no autorizada */}
                {ventaCompletada.venta.tipo_documento === "FACTURA" && sriModuloActivo && !esFacturaAutorizada && !emitiendo && (
                  <button className="btn btn-outline btn-lg" style={{ color: "var(--color-primary)", borderColor: "var(--color-primary)" }}
                    onClick={async () => {
                      if (!ventaCompletada.venta.id) return;
                      // Si no tiene email y no es consumidor final, pedir email primero
                      if (clienteSeleccionado && clienteSeleccionado.id !== 1 && !clienteSeleccionado.email?.trim()) {
                        setMostrarModalEmail(true);
                        return;
                      }
                      setEmitiendo(true);
                      try {
                        const res = await emitirFacturaSri(ventaCompletada.venta.id);
                        setResultadoSri(res);
                        if (res.exito) {
                          toastExito("Factura autorizada por el SRI");
                          setVentaCompletada(prev => prev ? {
                            ...prev,
                            venta: { ...prev.venta, estado_sri: "AUTORIZADA", numero_factura: res.numero_factura, clave_acceso: res.clave_acceso, autorizacion_sri: res.numero_autorizacion }
                          } : prev);
                          window.dispatchEvent(new CustomEvent("sri-factura-emitida"));
                          // Auto-enviar email
                          if (clienteSeleccionado?.email?.trim()) {
                            enviarNotificacionSri(ventaCompletada.venta.id!, clienteSeleccionado.email)
                              .then(() => toastExito(`Email enviado a ${clienteSeleccionado!.email}`))
                              .catch(() => toastWarning("Email pendiente, se reintentara"));
                          }
                        } else {
                          toastWarning(`SRI: ${res.mensaje}`);
                        }
                      } catch (err) {
                        toastWarning("Error enviando al SRI: " + err);
                      } finally {
                        setEmitiendo(false);
                      }
                    }}>
                    Autorizar SRI
                  </button>
                )}

                {/* Botones post-autorización */}
                {esFacturaAutorizada && (
                  <>
                    <button className="btn btn-outline btn-lg"
                      onClick={() => {
                        if (ventaCompletada.venta.id) {
                          imprimirRide(ventaCompletada.venta.id)
                            .then(() => toastExito("RIDE abierto"))
                            .catch((e) => toastError("Error RIDE: " + e));
                        }
                      }}>
                      RIDE
                    </button>
                    <button className="btn btn-outline btn-lg"
                      onClick={async () => {
                        if (!ventaCompletada.venta.id) return;
                        // Si cliente tiene email, enviar directamente
                        if (clienteSeleccionado?.email) {
                          try {
                            await enviarNotificacionSri(ventaCompletada.venta.id, clienteSeleccionado.email);
                            toastExito(`Email enviado a ${clienteSeleccionado.email}`);
                          } catch (err: any) {
                            const errStr = String(err);
                            if (errStr.startsWith("ENCOLADO:")) {
                              toastWarning("Email pendiente, se reintentara");
                            } else {
                              toastWarning("Error: " + errStr);
                            }
                          }
                        } else {
                          // Sin email, abrir modal para ingresar
                          setMostrarModalEmail(true);
                        }
                      }}>
                      Notificar
                    </button>
                    <button className="btn btn-outline btn-lg"
                      onClick={() => setMostrarModalEmail(true)}>
                      Notificar a...
                    </button>
                  </>
                )}
                <button className="btn btn-primary btn-lg" data-action="nueva-venta" onClick={nuevaVentaClick}>
                  Nueva Venta (F10)
                </button>
              </div>
            </div>
          </div>
        </div>

        <ModalEmailCliente
          abierto={mostrarModalEmail}
          clienteNombre={ventaCompletada.cliente_nombre || ""}
          ventaNumero={ventaCompletada.venta.numero_factura || ventaCompletada.venta.numero}
          onEnviar={handleEnviarEmailModal}
          onOmitir={() => setMostrarModalEmail(false)}
          enviando={enviandoEmail}
        />
      </>
    );
  }

  // Banner de caja cerrada
  if (!verificandoCaja && !cajaAbierta && !ventaCompletada) {
    return (
      <>
        <div className="page-header">
          <h2>Punto de Venta</h2>
        </div>
        <div className="page-body">
          <div className="card" style={{ maxWidth: 450, margin: "60px auto", textAlign: "center" }}>
            <div className="card-body" style={{ padding: 32 }}>
              <div style={{ fontSize: 48, marginBottom: 16 }}>$</div>
              <h3>Caja Cerrada</h3>
              <p className="text-secondary mt-2">
                Debe abrir la caja antes de comenzar a vender.
              </p>
              <button
                className="btn btn-primary btn-lg mt-4"
                onClick={() => navigate("/caja")}
              >
                Abrir Caja (F5)
              </button>
            </div>
          </div>
        </div>
      </>
    );
  }

  return (
    <div style={{ display: "flex", flexDirection: "column", height: "100%", width: "100%", maxWidth: "100%", overflow: "hidden", minWidth: 0 }}>
      <div className="page-header">
        <div className="flex gap-2 items-center">
          <h2>Punto de Venta</h2>
          {/* La tarifa/lista de precios se cambia POR ITEM al hacer click en el
              nombre o precio del item del carrito (modal). NO hay selector global aqui. */}
        </div>
        <div className="flex gap-2 items-center">
          {/* Selector de cliente */}
          <div style={{ position: "relative" }}>
            {clienteSeleccionado ? (
              <div className="flex gap-2 items-center">
                <span style={{ fontSize: 13, background: "rgba(59, 130, 246, 0.2)", padding: "4px 10px", borderRadius: 4 }}>
                  {clienteSeleccionado.nombre}
                  {clienteSeleccionado.lista_precio_nombre && (
                    <span style={{ fontSize: 10, marginLeft: 6, background: "rgba(59, 130, 246, 0.15)", padding: "1px 6px", borderRadius: 3, color: "var(--color-primary)" }}>
                      {clienteSeleccionado.lista_precio_nombre}
                    </span>
                  )}
                </span>
                <button className="btn btn-outline" style={{ padding: "2px 6px", fontSize: 11 }}
                  onClick={() => { setClienteSeleccionado(null); setMostrarClientes(false); recalcularPreciosCarrito(null); }}>
                  x
                </button>
              </div>
            ) : (
              <button className="btn btn-outline" onClick={() => setMostrarClientes(!mostrarClientes)}>
                Consumidor Final
              </button>
            )}
            {mostrarClientes && !clienteSeleccionado && (
              <div style={{
                position: "absolute", top: "100%", right: 0, width: 320,
                background: "var(--color-surface)", border: "1px solid var(--color-border)",
                borderRadius: "var(--radius)", boxShadow: "var(--shadow-lg)", zIndex: 20, padding: 8,
              }}>
                <div className="flex gap-1 mb-2">
                  <input
                    className="input"
                    style={{ flex: 1 }}
                    placeholder="Buscar por nombre o cedula..."
                    value={busquedaCliente}
                    onChange={(e) => handleBuscarCliente(e.target.value)}
                    autoFocus
                  />
                  <button className="btn btn-primary" style={{ padding: "4px 10px", fontSize: 14, fontWeight: 700, minWidth: 34 }}
                    title="Crear cliente nuevo"
                    onClick={() => {
                      const txt = busquedaCliente.trim();
                      const esNumero = /^\d+$/.test(txt);
                      setNuevoClienteId(esNumero ? txt : "");
                      setNuevoClienteNombre(esNumero ? "" : txt);
                      setMostrarCrearCliente(true);
                    }}>
                    +
                  </button>
                </div>
                {mostrarCrearCliente && (
                  <div style={{
                    background: "rgba(34, 197, 94, 0.1)", border: "1px solid rgba(34, 197, 94, 0.3)", borderRadius: "var(--radius)",
                    padding: 8, marginBottom: 8,
                  }}>
                    <div style={{ fontSize: 11, fontWeight: 600, marginBottom: 6, color: "var(--color-success)" }}>Nuevo cliente</div>
                    <input className="input mb-1" placeholder="Cedula / RUC" value={nuevoClienteId}
                      onChange={(e) => setNuevoClienteId(e.target.value)}
                      style={{ fontSize: 13 }}
                      autoFocus />
                    <input className="input mb-1" placeholder="Nombre completo *" value={nuevoClienteNombre}
                      onChange={(e) => setNuevoClienteNombre(e.target.value)}
                      style={{ fontSize: 13 }} />
                    <input className="input mb-1" placeholder="Telefono" value={nuevoClienteTelefono}
                      onChange={(e) => setNuevoClienteTelefono(e.target.value)}
                      style={{ fontSize: 13 }}
                      type="tel"
                      inputMode="tel" />
                    <input className="input mb-1" placeholder="Email" value={nuevoClienteEmail}
                      onChange={(e) => setNuevoClienteEmail(e.target.value)}
                      style={{ fontSize: 13 }}
                      type="email"
                      inputMode="email" />
                    <input className="input mb-2" placeholder="Direccion" value={nuevoClienteDireccion}
                      onChange={(e) => setNuevoClienteDireccion(e.target.value)}
                      style={{ fontSize: 13 }}
                      onKeyDown={(e) => { if (e.key === "Enter") handleCrearClienteRapido(); }} />
                    <div className="flex gap-1">
                      <button className="btn btn-outline" style={{ flex: 1, fontSize: 11, padding: "4px 0", justifyContent: "center" }}
                        onClick={() => setMostrarCrearCliente(false)}>
                        Cancelar
                      </button>
                      <button className="btn btn-primary" style={{ flex: 1, fontSize: 11, padding: "4px 0", justifyContent: "center" }}
                        disabled={creandoCliente || !nuevoClienteNombre.trim()}
                        onClick={handleCrearClienteRapido}>
                        {creandoCliente ? "..." : "Crear"}
                      </button>
                    </div>
                  </div>
                )}
                {clientesResultados.map((c) => (
                  <div key={c.id} style={{ padding: "6px 8px", cursor: "pointer", borderRadius: 4, fontSize: 13 }}
                    onClick={() => { setClienteSeleccionado(c); setMostrarClientes(false); setBusquedaCliente(""); setClientesResultados([]); setMostrarCrearCliente(false); recalcularPreciosCarrito(c.id ?? null); }}
                    onMouseEnter={(e) => (e.currentTarget.style.background = "var(--color-surface-hover)")}
                    onMouseLeave={(e) => (e.currentTarget.style.background = "transparent")}
                  >
                    <strong>{c.nombre}</strong>
                    {c.identificacion && <span className="text-secondary" style={{ marginLeft: 6 }}>{c.identificacion}</span>}
                  </div>
                ))}
                {busquedaCliente.length >= 2 && clientesResultados.length === 0 && !mostrarCrearCliente && (
                  /^\d{10}(\d{3})?$/.test(busquedaCliente.trim()) ? (
                    <div style={{ padding: "8px", textAlign: "center" }}>
                      <button
                        className="btn btn-outline"
                        style={{ fontSize: 12, padding: "6px 16px", width: "100%", justifyContent: "center" }}
                        disabled={consultandoSri}
                        onClick={async () => {
                          setConsultandoSri(true);
                          try {
                            const cliente = await consultarIdentificacion(busquedaCliente.trim());
                            setClienteSeleccionado(cliente);
                            setMostrarClientes(false);
                            setBusquedaCliente("");
                            setClientesResultados([]);
                            recalcularPreciosCarrito(cliente.id ?? null);
                            toastExito(`Cliente registrado: ${cliente.nombre}`);
                          } catch (err: any) {
                            toastError(err?.toString() || "No se encontro informacion");
                          } finally {
                            setConsultandoSri(false);
                          }
                        }}
                      >
                        {consultandoSri ? "Consultando..." : "Consultar en SRI"}
                      </button>
                      <div style={{ fontSize: 11, color: "var(--color-text-secondary)", marginTop: 4 }}>
                        Buscar datos por cedula/RUC en el SRI
                      </div>
                    </div>
                  ) : (
                    <div style={{ padding: "8px", textAlign: "center", fontSize: 12, color: "var(--color-text-secondary)" }}>
                      No encontrado. Use <strong>+</strong> para crear.
                    </div>
                  )
                )}
              </div>
            )}
          </div>
        </div>
      </div>

      <div style={{ display: "flex", flex: 1, overflow: "hidden", position: "relative", minWidth: 0 }}>
        {/* Main content - product grid */}
        <div style={{ flex: 1, position: "relative", marginRight: carritoAbierto ? 420 : 0, transition: "margin-right 0.3s", display: "flex", flexDirection: "column", minHeight: 0, minWidth: 0, overflow: "hidden" }}>
          <div style={{ flex: 1, overflow: "hidden", position: "relative", minHeight: 0, minWidth: 0 }}>
            <PosGridTactil
              categorias={categoriasTactil}
              productosTactil={
                stockModo === "BLOQUEAR_OCULTAR"
                  ? productosTactil.filter(p => (p as any).es_servicio || (p as any).no_controla_stock || p.stock_actual > 0)
                  : productosTactil
              }
              onAgregarProducto={agregarAlCarrito}
              puedeVerDetalle={esAdmin}
              onVerDetalle={async (pid) => {
                try {
                  const p = await obtenerProducto(pid);
                  setProductoDetalle(p);
                } catch (err) { toastError("Error: " + err); }
              }}
              busqueda={busqueda}
              onBusquedaChange={handleBuscar}
              resultados={resultados}
              inputRef={inputRef}
            />
          </div>
          {/* Footer acciones rápidas */}
          <div style={{
            display: "flex", justifyContent: "flex-end", gap: 6, padding: "6px 12px",
            borderTop: "1px solid var(--color-border)", background: "var(--color-surface)",
            flexShrink: 0, position: "relative", zIndex: 2,
          }}>
            <button className="btn btn-outline" style={{ fontSize: 11, padding: "5px 14px" }}
              onClick={() => guardarComoDocumento("borrador")}>
              Borrador
            </button>
            <button className="btn" style={{ fontSize: 11, padding: "5px 14px", background: "#ea580c", color: "#fff", border: "none" }}
              onClick={() => setMostrarModalGuia(true)}>
              Guía R.
            </button>
            <button className="btn" style={{ fontSize: 11, padding: "5px 14px", background: "#2563eb", color: "#fff", border: "none" }}
              onClick={() => guardarComoDocumento("cotizacion")}>
              Cotización
            </button>
            <button className="btn btn-outline" style={{ fontSize: 11, padding: "5px 14px" }}
              onClick={() => setMostrarRecientes(true)}>
              Recientes
            </button>
          </div>
        </div>

        {/* Cart toggle button */}
        {carrito.length > 0 && (
          <button
            className={`cart-panel-toggle ${carritoAbierto ? "open" : ""}`}
            onClick={() => {
              setCarritoAbierto(!carritoAbierto);
              if (carritoAbierto) setCarritoManualCerrado(true);
              else setCarritoManualCerrado(false);
            }}
          >
            {carritoAbierto ? "\u25B6" : "\u25C0"}
          </button>
        )}

        {/* Cart panel */}
        <div className={`cart-panel ${carritoAbierto ? "open" : ""}`}>
          {/* minHeight:0 + height:100% asegura que el flex column con items scrollables
              funcione bien y nunca se desborde el botón Cobrar */}
          <div style={{ padding: 16, flex: 1, display: "flex", flexDirection: "column", minHeight: 0, height: "100%" }}>
            {/* Cliente + Items count en una fila */}
            <div className="flex justify-between items-center mb-2">
              <div style={{ display: "flex", alignItems: "center", gap: 6 }}>
                {clienteSeleccionado ? (
                  <div style={{ display: "flex", alignItems: "center", gap: 4 }}>
                    <span style={{ fontSize: 12, fontWeight: 600, color: "var(--color-primary)" }}>{clienteSeleccionado.nombre}</span>
                    <button style={{ background: "none", border: "none", cursor: "pointer", color: "var(--color-text-secondary)", fontSize: 11, padding: "0 2px" }}
                      onClick={() => { setClienteSeleccionado(null); recalcularPreciosCarrito(null); }}>×</button>
                  </div>
                ) : (
                  <button className="btn btn-outline" style={{ fontSize: 11, padding: "2px 8px" }}
                    onClick={() => setMostrarClientes(!mostrarClientes)}>
                    Consumidor Final
                  </button>
                )}
              </div>
              <span className="text-secondary" style={{ fontSize: 12 }}>Items: {carrito.reduce((s, i) => s + i.cantidad, 0)}</span>
            </div>

            {/* Cart items list - scrollable. flex:1 + min-height:0 permite que se reduzca
                cuando hay muchos items y deje espacio para totales+pago+Cobrar. */}
            <div style={{ flex: "1 1 auto", overflowY: "auto", marginBottom: 12, minHeight: 0 }}>
              {carrito.map((item, idx) => (
                <div key={`${item.producto_id}-${item.unidad_id ?? 0}-${idx}`} style={{
                  padding: "6px 0", borderBottom: "1px solid var(--color-border)",
                }}>
                  {/* Una sola fila compacta */}
                  <div style={{ display: "flex", alignItems: "center", gap: 6 }}>
                    <div style={{ flex: 1, minWidth: 0, cursor: "pointer" }}
                      title={`${item.nombre}${item.unidad_nombre ? ` — ${item.unidad_nombre} (×${item.factor_unidad})` : ""}\nClick para agregar informacion adicional`}
                      onClick={() => {
                        setInfoAdicionalProductoId(idx as any);
                        // Parsear info existente
                        const info = item.info_adicional || "";
                        const serieMatch = info.match(/Serie:\s*([^|]*)/i);
                        const loteMatch = info.match(/Lote:\s*([^|]*)/i);
                        const obsMatch = info.match(/Obs:\s*([^|]*)/i);
                        setInfoSerie(serieMatch ? serieMatch[1].trim() : "");
                        setInfoLote(loteMatch ? loteMatch[1].trim() : "");
                        setInfoObservacion(obsMatch ? obsMatch[1].trim() : "");
                        // Si no tiene formato estructurado, poner todo en observación
                        if (!serieMatch && !loteMatch && !obsMatch && info) {
                          setInfoObservacion(info);
                        }
                      }}>
                      <div style={{ display: "flex", alignItems: "center", gap: 4 }}>
                        <span style={{ fontWeight: 600, fontSize: 12, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap", flex: 1, minWidth: 0 }}>
                          {item.nombre}
                        </span>
                        {item.unidad_nombre && (
                          <span style={{
                            fontSize: 9, fontWeight: 700, padding: "1px 5px", borderRadius: 3,
                            background: "rgba(59,130,246,0.15)", color: "var(--color-primary)",
                            border: "1px solid rgba(59,130,246,0.3)", flexShrink: 0,
                            whiteSpace: "nowrap",
                          }} title={`Presentacion: ${item.unidad_nombre} = ${item.factor_unidad} unidades base`}>
                            ×{item.factor_unidad}
                          </span>
                        )}
                      </div>
                      {/* Badge del lote (caducidad) */}
                      {item.lote_id && (
                        <div
                          style={{ fontSize: 10, cursor: "pointer", marginTop: 2 }}
                          title="Click para cambiar de lote"
                          onClick={async (e) => {
                            e.stopPropagation();
                            try {
                              const lotes = await listarLotesProducto(item.producto_id);
                              setCambiarLoteItem({ idx, lotes: lotes.filter((l: any) => l.cantidad > 0 || l.id === item.lote_id) });
                            } catch { /* ignore */ }
                          }}>
                          <span style={{
                            padding: "1px 6px", borderRadius: 3,
                            background: (item.lote_dias_restantes ?? 99) < 0 ? "rgba(239,68,68,0.2)"
                              : (item.lote_dias_restantes ?? 99) <= 7 ? "rgba(245,158,11,0.2)"
                              : "rgba(34,197,94,0.15)",
                            color: (item.lote_dias_restantes ?? 99) < 0 ? "var(--color-danger)"
                              : (item.lote_dias_restantes ?? 99) <= 7 ? "var(--color-warning)"
                              : "var(--color-success)",
                            fontWeight: 600,
                          }}>
                            🕐 Lote {item.lote_numero || "#" + item.lote_id}
                            {" · "}
                            {item.lote_dias_restantes != null && (
                              item.lote_dias_restantes < 0
                                ? `Vencido (${Math.abs(item.lote_dias_restantes)}d)`
                                : `Vence en ${item.lote_dias_restantes}d`
                            )}
                            <span style={{ marginLeft: 4, fontSize: 9, textDecoration: "underline" }}>cambiar</span>
                          </span>
                        </div>
                      )}
                      {item.info_adicional && <div style={{ fontSize: 10, color: "var(--color-primary)", overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>{item.info_adicional}</div>}
                      {(item as any).combo_seleccion && (item as any).combo_seleccion.length > 0 && (
                        <div style={{ fontSize: 10, color: "var(--color-text-secondary)", marginTop: 2, paddingLeft: 4, borderLeft: "2px solid rgba(168,85,247,0.5)" }}>
                          {(item as any).combo_seleccion.map((c: any, ix: number) => (
                            <div key={ix}>🍽 {c.nombre || `Producto #${c.producto_hijo_id}`} × {c.cantidad}</div>
                          ))}
                        </div>
                      )}
                    </div>
                    <span style={{ color: "var(--color-primary)", cursor: (tienePermiso("editar_precio") || puedeCambiarListaPrecio) ? "pointer" : "default", fontSize: 11, flexShrink: 0, textDecoration: "underline dotted", textUnderlineOffset: 2 }}
                      title="Click para cambiar precio o lista de precios"
                      onClick={async () => {
                        const abrirModal = async () => {
                          let precios = item.precios_disponibles || [];
                          if (precios.length === 0) {
                            try { precios = await obtenerPreciosProducto(item.producto_id); } catch { /* */ }
                          }
                          setEditarPrecioItemModal({
                            idx,
                            nombre: item.nombre,
                            precioActual: item.precio_unitario,
                            preciosDisponibles: precios,
                          });
                          setPrecioManualInput(item.precio_unitario.toFixed(2));
                        };
                        if (tienePermiso("editar_precio") || puedeCambiarListaPrecio) {
                          await abrirModal();
                        } else {
                          solicitarPinAdmin().then(async ok => { if (ok) await abrirModal(); });
                        }
                      }}>
                      ${item.precio_unitario.toFixed(2)}
                    </span>
                    <select style={{ width: 50, fontSize: 10, padding: "2px 1px", border: "1px solid var(--color-border)", borderRadius: 4, background: "var(--color-surface)", color: "var(--color-text)", flexShrink: 0 }}
                      value={item.iva_porcentaje}
                      onChange={(e) => {
                        if (!tienePermiso("editar_precio")) { toastError("Sin permiso"); return; }
                        editarIvaItem(idx, parseFloat(e.target.value));
                      }}>
                      <option value="0">0%</option>
                      <option value="15">15%</option>
                    </select>
                    {/* Boton de descuento */}
                    <button
                      title={item.descuento > 0 ? `Descuento aplicado: $${item.descuento.toFixed(2)}` : "Aplicar descuento"}
                      style={{
                        width: 26, height: 26,
                        border: `1px solid ${item.descuento > 0 ? "var(--color-warning)" : "var(--color-border)"}`,
                        borderRadius: 4,
                        background: item.descuento > 0 ? "rgba(245, 158, 11, 0.15)" : "var(--color-surface)",
                        cursor: "pointer",
                        color: item.descuento > 0 ? "var(--color-warning)" : "var(--color-text-secondary)",
                        flexShrink: 0, fontSize: 12, fontWeight: 700,
                      }}
                      onClick={() => {
                        setDescuentoItemId(idx as any);
                        if (item.descuento > 0) {
                          setDescuentoTipo("monto");
                          setDescuentoValor(item.descuento.toFixed(2));
                        } else {
                          setDescuentoTipo("porcentaje");
                          setDescuentoValor("");
                        }
                      }}>%</button>
                    <button style={{ width: 26, height: 26, border: "1px solid var(--color-border)", borderRadius: 4, background: "var(--color-surface)", cursor: "pointer", color: "var(--color-text)", flexShrink: 0, fontSize: 14 }}
                      onClick={() => actualizarCantidad(idx, item.cantidad - 1)}>-</button>
                    <span style={{ minWidth: 18, textAlign: "center", fontSize: 13, flexShrink: 0 }}>{item.cantidad}</span>
                    <button style={{ width: 26, height: 26, border: "1px solid var(--color-border)", borderRadius: 4, background: "var(--color-surface)", cursor: "pointer", color: "var(--color-text)", flexShrink: 0, fontSize: 14 }}
                      onClick={() => actualizarCantidad(idx, item.cantidad + 1)}>+</button>
                    <span style={{ minWidth: 50, textAlign: "right", fontWeight: 600, fontSize: 12, flexShrink: 0 }}>${item.subtotal.toFixed(2)}</span>
                    <button title="Eliminar" style={{ color: "var(--color-danger)", cursor: "pointer", background: "none", border: "none", fontSize: 15, padding: "0 2px", flexShrink: 0 }}
                      onClick={() => eliminarItem(idx)}>×</button>
                  </div>
                </div>
              ))}
            </div>

            {/* Totals + pago + Cobrar - SIEMPRE visibles (no scroll, flex-shrink:0).
                Items list de arriba scrollea cuando hay muchos. */}
            <div style={{ borderTop: "2px solid var(--color-border)", paddingTop: 8, flexShrink: 0 }}>
              <div className="flex justify-between" style={{ fontSize: 13 }}>
                <span className="text-secondary">Subtotal:</span>
                <span>${subtotal.toFixed(2)}</span>
              </div>
              {iva > 0 && (
                <div className="flex justify-between" style={{ fontSize: 13 }}>
                  <span className="text-secondary">IVA:</span>
                  <span>${iva.toFixed(2)}</span>
                </div>
              )}
              <div className="flex justify-between" style={{ fontSize: 22, fontWeight: 700, margin: "6px 0" }}>
                <span>TOTAL:</span>
                <span>${total.toFixed(2)}</span>
              </div>
            </div>

            {/* Tipo documento - solo si no RIMPE_POPULAR */}
            {regimen !== "RIMPE_POPULAR" && (
              <div style={{ marginBottom: 8 }}>
                <label className="text-secondary" style={{ fontSize: 11 }}>Tipo documento</label>
                <div className="flex gap-2" style={{ marginTop: 4 }}>
                  {(["NOTA_VENTA", "FACTURA"] as const).map((tipo) => (
                    <button key={tipo}
                      className={`btn ${tipoDocumento === tipo ? "btn-primary" : "btn-outline"}`}
                      style={{ flex: 1, fontSize: 11, justifyContent: "center", padding: "4px 0" }}
                      onClick={() => setTipoDocumento(tipo)}>
                      {tipo === "NOTA_VENTA" ? "Nota Venta" : "Factura"}
                    </button>
                  ))}
                </div>
                {tipoDocumento === "FACTURA" && sriModuloActivo && sriAmbiente && (
                  <div style={{
                    fontSize: 11, marginTop: 4, display: "flex", alignItems: "center", gap: 6,
                    color: sriAmbiente === "produccion" ? "var(--color-danger)" : "var(--color-primary)",
                  }}>
                    <span style={{
                      width: 8, height: 8, borderRadius: "50%",
                      background: sriAmbiente === "produccion" ? "var(--color-danger)" : "var(--color-primary)",
                      display: "inline-block",
                    }} />
                    Ambiente: {sriAmbiente.toUpperCase()}
                    {!sriAmbienteConfirmado && <span style={{ color: "var(--color-warning)" }}>(sin confirmar)</span>}
                  </div>
                )}
              </div>
            )}

            {/* Forma de pago */}
            <div style={{ marginBottom: 8 }}>
              <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
                <label className="text-secondary" style={{ fontSize: 11 }}>
                  {modoPagoMixto ? "Pagos mixtos" : "Forma de pago"}
                </label>
                <button type="button"
                  title={modoPagoMixto ? "Volver a un solo metodo de pago" : "Combinar varios metodos (ej: efectivo + transferencia + credito)"}
                  style={{
                    background: modoPagoMixto ? "var(--color-primary)" : "transparent",
                    color: modoPagoMixto ? "#fff" : "var(--color-primary)",
                    border: "1px solid var(--color-primary)",
                    borderRadius: 4, fontSize: 10, padding: "2px 8px", cursor: "pointer", fontWeight: 600,
                  }}
                  onClick={() => {
                    setModoPagoMixto(!modoPagoMixto);
                    if (!modoPagoMixto) { setEsFiado(false); }
                    else { setPagosMixtos([]); }
                  }}>
                  {modoPagoMixto ? "← Pago simple" : "+ Pago mixto"}
                </button>
              </div>

              {!modoPagoMixto ? (
                <div style={{ marginTop: 4 }}>
                  <button className="btn" style={{ width: "100%", justifyContent: "center", fontSize: 13, padding: "8px 0", marginBottom: 4, background: formaPago === "EFECTIVO" && !esFiado ? "#16a34a" : "var(--color-surface)", color: formaPago === "EFECTIVO" && !esFiado ? "#fff" : "var(--color-text)", border: "1px solid var(--color-border)" }}
                    onClick={() => { setFormaPago("EFECTIVO"); setEsFiado(false); }}>
                    Efectivo
                  </button>
                  <div className="flex gap-2">
                    <button className="btn" style={{ flex: 1, justifyContent: "center", fontSize: 11, padding: "6px 0", background: formaPago === "TRANSFER" && !esFiado ? "#2563eb" : "var(--color-surface)", color: formaPago === "TRANSFER" && !esFiado ? "#fff" : "var(--color-text)", border: "1px solid var(--color-border)" }}
                      onClick={() => { setFormaPago("TRANSFER"); setEsFiado(false); }}>
                      Transferencia
                    </button>
                    <button className="btn" style={{ flex: 1, justifyContent: "center", fontSize: 11, padding: "6px 0", background: esFiado ? "#d97706" : "var(--color-surface)", color: esFiado ? "#fff" : "var(--color-text)", border: "1px solid var(--color-border)" }}
                      onClick={() => setEsFiado(!esFiado)}>
                      Credito
                    </button>
                  </div>
                </div>
              ) : (() => {
                const sumaPagos = pagosMixtos.reduce((s, p) => s + p.monto, 0);
                const falta = total - sumaPagos;
                return (
                  <div style={{ marginTop: 4 }}>
                    {/* Lista de pagos agregados */}
                    {pagosMixtos.length === 0 ? (
                      <div style={{ padding: 8, fontSize: 11, color: "var(--color-text-secondary)", textAlign: "center", fontStyle: "italic" }}>
                        Sin pagos. Agregue al menos uno.
                      </div>
                    ) : (
                      <div style={{ display: "flex", flexDirection: "column", gap: 4, marginBottom: 6 }}>
                        {pagosMixtos.map((p, idx) => {
                          const colorBg = p.forma_pago === "EFECTIVO" ? "rgba(22,163,74,0.12)"
                            : p.forma_pago === "TRANSFER" ? "rgba(37,99,235,0.12)"
                            : p.forma_pago === "CREDITO" ? "rgba(217,119,6,0.12)"
                            : "rgba(148,163,184,0.12)";
                          const colorTxt = p.forma_pago === "EFECTIVO" ? "#16a34a"
                            : p.forma_pago === "TRANSFER" ? "#2563eb"
                            : p.forma_pago === "CREDITO" ? "#d97706"
                            : "var(--color-text-secondary)";
                          return (
                            <div key={idx} style={{
                              display: "flex", alignItems: "center", gap: 6,
                              padding: "4px 8px", background: colorBg, borderRadius: 4,
                              border: `1px solid ${colorTxt}33`,
                            }}>
                              <span style={{ fontSize: 11, fontWeight: 700, color: colorTxt, minWidth: 65 }}>{p.forma_pago}</span>
                              {p.referencia && <span style={{ fontSize: 10, color: "var(--color-text-secondary)" }}>{p.referencia}</span>}
                              <span style={{ flex: 1, textAlign: "right", fontSize: 12, fontWeight: 700 }}>${p.monto.toFixed(2)}</span>
                              <button type="button" title="Quitar pago"
                                style={{ background: "none", border: "none", cursor: "pointer", color: "var(--color-danger)", fontSize: 14, padding: 0 }}
                                onClick={() => setPagosMixtos(prev => prev.filter((_, i) => i !== idx))}>×</button>
                            </div>
                          );
                        })}
                      </div>
                    )}

                    {/* Resumen */}
                    <div style={{ padding: "6px 8px", borderRadius: 4, background: "var(--color-surface-alt)", marginBottom: 6 }}>
                      <div className="flex justify-between" style={{ fontSize: 11 }}>
                        <span>Pagado:</span>
                        <span style={{ fontWeight: 600 }}>${sumaPagos.toFixed(2)} de ${total.toFixed(2)}</span>
                      </div>
                      {Math.abs(falta) > 0.01 ? (
                        <div className="flex justify-between" style={{ fontSize: 11, marginTop: 2,
                          color: falta > 0 ? "var(--color-warning)" : "var(--color-danger)",
                          fontWeight: 700 }}>
                          <span>{falta > 0 ? "Falta:" : "Excede por:"}</span>
                          <span>${Math.abs(falta).toFixed(2)}</span>
                        </div>
                      ) : (
                        <div style={{ fontSize: 11, marginTop: 2, color: "var(--color-success)", fontWeight: 700, textAlign: "center" }}>
                          ✓ Pago completo
                        </div>
                      )}
                    </div>

                    {/* Botones de agregar pago */}
                    <div style={{ display: "flex", gap: 4 }}>
                      <button type="button" className="btn"
                        style={{ flex: 1, fontSize: 11, padding: "5px 0", background: "rgba(22,163,74,0.15)", color: "#16a34a", border: "1px solid rgba(22,163,74,0.4)", fontWeight: 600 }}
                        onClick={() => {
                          setAddPagoForma("EFECTIVO");
                          setAddPagoMonto(falta > 0 ? falta.toFixed(2) : "");
                          setAddPagoBancoId(null); setAddPagoReferencia("");
                          setMostrarAddPago(true);
                        }}>+ Efectivo</button>
                      <button type="button" className="btn"
                        style={{ flex: 1, fontSize: 11, padding: "5px 0", background: "rgba(37,99,235,0.15)", color: "#2563eb", border: "1px solid rgba(37,99,235,0.4)", fontWeight: 600 }}
                        onClick={() => {
                          setAddPagoForma("TRANSFER");
                          setAddPagoMonto(falta > 0 ? falta.toFixed(2) : "");
                          setAddPagoBancoId(cuentasBanco[0]?.id || null);
                          setAddPagoReferencia("");
                          setMostrarAddPago(true);
                        }}>+ Transfer</button>
                      <button type="button" className="btn"
                        style={{ flex: 1, fontSize: 11, padding: "5px 0", background: "rgba(217,119,6,0.15)", color: "#d97706", border: "1px solid rgba(217,119,6,0.4)", fontWeight: 600 }}
                        onClick={() => {
                          setAddPagoForma("CREDITO");
                          setAddPagoMonto(falta > 0 ? falta.toFixed(2) : "");
                          setAddPagoBancoId(null); setAddPagoReferencia("");
                          setMostrarAddPago(true);
                        }}>+ Credito</button>
                    </div>
                  </div>
                );
              })()}
            </div>

            {/* Transferencia: chip resumen + boton para abrir modal de detalles */}
            {!modoPagoMixto && !esFiado && formaPago === "TRANSFER" && (() => {
              const bancoNombre = cuentasBanco.find((cb: any) => cb.id === bancoSeleccionado)?.nombre;
              const detallesCompletos = bancoSeleccionado && (!requiereReferencia || referenciaPago.trim()) && (!requiereComprobante || comprobanteImagen);
              return (
                <div style={{ marginBottom: 8 }}>
                  <button
                    type="button"
                    onClick={() => setMostrarDetallesTransfer(true)}
                    style={{
                      width: "100%", padding: "8px 10px", borderRadius: 6, cursor: "pointer",
                      background: detallesCompletos ? "rgba(34,197,94,0.12)" : "rgba(245,158,11,0.12)",
                      border: `1px solid ${detallesCompletos ? "rgba(34,197,94,0.5)" : "rgba(245,158,11,0.5)"}`,
                      display: "flex", justifyContent: "space-between", alignItems: "center", gap: 6,
                      textAlign: "left",
                    }}>
                    <div style={{ flex: 1, minWidth: 0 }}>
                      <div style={{ fontSize: 11, fontWeight: 700, color: detallesCompletos ? "var(--color-success)" : "var(--color-warning)" }}>
                        {detallesCompletos ? "✓ Detalles transferencia" : "⚠ Faltan detalles transfer"}
                      </div>
                      <div style={{ fontSize: 10, color: "var(--color-text-secondary)", marginTop: 2, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>
                        {bancoNombre || "Sin cuenta"}{referenciaPago && ` · ref: ${referenciaPago}`}{comprobanteImagen && " · 📎"}
                      </div>
                    </div>
                    <span style={{ fontSize: 11, color: "var(--color-primary)", fontWeight: 600 }}>Editar →</span>
                  </button>
                </div>
              );
            })()}

            {/* Monto recibido - solo si no es fiado y es efectivo */}
            {!modoPagoMixto && !esFiado && formaPago === "EFECTIVO" && (
              <div style={{ marginBottom: 8 }}>
                <label className="text-secondary" style={{ fontSize: 11, display: "block", marginBottom: 2 }}>Monto recibido</label>
                <div style={{ display: "flex", gap: 4 }}>
                  <div style={{ flex: 1, position: "relative" }}>
                    <input className="input text-right" type="number" step="0.01" placeholder="0.00"
                      style={{ fontSize: 14, width: "100%", paddingLeft: 130 }}
                      value={montoRecibido}
                      onChange={(e) => setMontoRecibido(e.target.value)}
                      onKeyDown={(e) => { if (e.key === "Enter") procesarVenta(); }} />
                    {/* Denominaciones rapidas FLOTANDO dentro del input alineadas a la izquierda */}
                    {total > 0 && (() => {
                      const base = Math.ceil((total + 0.01) / 5) * 5;
                      const opciones = [base, base + 5, base + 15];
                      return (
                        <div style={{
                          position: "absolute", left: 4, top: "50%", transform: "translateY(-50%)",
                          display: "flex", gap: 3, pointerEvents: "none",
                        }}>
                          {opciones.map((monto) => (
                            <button
                              key={monto}
                              type="button"
                              title={`Cliente paga $${monto.toFixed(2)} - cambio $${(monto - total).toFixed(2)}`}
                              style={{
                                pointerEvents: "auto",
                                background: "rgba(59, 130, 246, 0.12)",
                                color: "var(--color-primary)",
                                border: "1px solid rgba(59, 130, 246, 0.35)",
                                borderRadius: 4,
                                fontSize: 10, padding: "2px 6px", fontWeight: 700,
                                cursor: "pointer", lineHeight: 1.2,
                              }}
                              onClick={() => setMontoRecibido(monto.toFixed(2))}>
                              ${monto}
                            </button>
                          ))}
                        </div>
                      );
                    })()}
                  </div>
                  <button
                    className="btn"
                    title="Monto exacto - presione F8"
                    style={{
                      background: "var(--color-primary)", color: "#fff",
                      fontSize: 11, padding: "0 10px", fontWeight: 700,
                      border: "none", borderRadius: "var(--radius)", cursor: "pointer",
                      whiteSpace: "nowrap",
                    }}
                    onClick={() => {
                      setMontoRecibido(total.toFixed(2));
                      setTimeout(() => procesarVenta(), 100);
                    }}
                    disabled={carrito.length === 0}
                  >
                    Exacto <span style={{ fontSize: 9, fontWeight: 600, opacity: 0.85, marginLeft: 2 }}>F8</span>
                  </button>
                </div>
              </div>
            )}

            {!modoPagoMixto && !esFiado && formaPago === "EFECTIVO" && cambio >= 0 && montoRecibido && (
              <div className="flex justify-between" style={{ fontSize: 14, marginBottom: 8 }}>
                <span>Cambio:</span>
                <span className="font-bold" style={{ color: "var(--color-success)" }}>${cambio.toFixed(2)}</span>
              </div>
            )}

            {!modoPagoMixto && esFiado && (
              <div style={{ background: "rgba(245, 158, 11, 0.15)", padding: 8, borderRadius: "var(--radius)", fontSize: 12, color: "var(--color-warning)", marginBottom: 8 }}>
                Se registrara como cuenta por cobrar
                {clienteSeleccionado ? ` a ${clienteSeleccionado.nombre}` : ". Seleccione un cliente arriba."}
              </div>
            )}

            {/* Cobrar button - flex-shrink:0 para que NUNCA se aplaste */}
            <div style={{ flexShrink: 0, marginTop: 8 }}>
              <button className="btn btn-success" data-action="cobrar"
                style={{ width: "100%", justifyContent: "center", fontSize: 15, padding: "12px 0" }}
                disabled={
                  carrito.length === 0
                  || (esFiado && !clienteSeleccionado)
                  || (modoPagoMixto && (pagosMixtos.length === 0 || Math.abs(pagosMixtos.reduce((s, p) => s + p.monto, 0) - total) > 0.02))
                }
                onClick={procesarVenta}>
                {modoPagoMixto
                  ? `Cobrar mixto $${total.toFixed(2)}`
                  : (esFiado ? `Credito $${total.toFixed(2)}` : `Cobrar $${total.toFixed(2)}`)} (F9)
              </button>
            </div>

            {/* Botones movidos al footer del área central */}
          </div>
        </div>
      </div>

      {/* Modal confirmacion de ambiente SRI */}
      {mostrarModalAmbiente && (
        <div className="modal-overlay" onClick={() => setMostrarModalAmbiente(false)}>
          <div className="modal-content" onClick={(e) => e.stopPropagation()} style={{ maxWidth: 420 }}>
            <div className="modal-header">
              <h3>Confirmar Ambiente SRI</h3>
            </div>
            <div className="modal-body">
              <p style={{ marginBottom: 12 }}>
                Las facturas se enviaran al ambiente:
              </p>
              <div style={{
                padding: "12px 16px",
                borderRadius: "var(--radius)",
                background: sriAmbiente === "produccion" ? "rgba(239, 68, 68, 0.1)" : "rgba(59, 130, 246, 0.1)",
                border: `2px solid ${sriAmbiente === "produccion" ? "var(--color-danger)" : "var(--color-primary)"}`,
                textAlign: "center",
                marginBottom: 12,
              }}>
                <span style={{
                  fontSize: 18,
                  fontWeight: 700,
                  color: sriAmbiente === "produccion" ? "var(--color-danger)" : "var(--color-primary)",
                }}>
                  {sriAmbiente === "produccion" ? "PRODUCCION" : "PRUEBAS"}
                </span>
                <div style={{ fontSize: 12, marginTop: 4, color: "var(--color-text-secondary)" }}>
                  {sriAmbiente === "produccion"
                    ? "Las facturas tendran validez tributaria real"
                    : "Las facturas NO tendran validez tributaria"}
                </div>
              </div>
              <div style={{ display: "flex", gap: 8, alignItems: "center", marginBottom: 8 }}>
                <span style={{ fontSize: 13, color: "var(--color-text-secondary)" }}>Cambiar a:</span>
                <button className="btn btn-outline" style={{ fontSize: 12, padding: "4px 12px" }}
                  disabled={cambiandoAmbiente}
                  onClick={async () => {
                    const nuevo = sriAmbiente === "produccion" ? "pruebas" : "produccion";
                    setCambiandoAmbiente(true);
                    try {
                      await cambiarAmbienteSri(nuevo);
                      setSriAmbiente(nuevo);
                      toastExito(`Ambiente cambiado a ${nuevo.toUpperCase()}`);
                    } catch (err) { toastError("Error: " + err); }
                    finally { setCambiandoAmbiente(false); }
                  }}>
                  {cambiandoAmbiente ? "Cambiando..." : sriAmbiente === "produccion" ? "Pruebas" : "Produccion"}
                </button>
              </div>
            </div>
            <div className="modal-footer">
              <button className="btn btn-outline" onClick={() => setMostrarModalAmbiente(false)}>
                Cancelar
              </button>
              <button className="btn btn-primary" onClick={async () => {
                await guardarConfig({ sri_ambiente_confirmado: "1" });
                setSriAmbienteConfirmado(true);
                setMostrarModalAmbiente(false);
                toastExito(`Ambiente confirmado: ${sriAmbiente.toUpperCase()}`);
              }}>
                Confirmar y continuar
              </button>
            </div>
          </div>
        </div>
      )}

      {/* Panel de documentos recientes */}
      <DocumentosRecientes
        abierto={mostrarRecientes}
        onCerrar={() => setMostrarRecientes(false)}
        ticketUsarPdf={ticketUsarPdf}
        onCargarDocumento={(ventaCompleta) => {
          // Restaurar carrito desde borrador/cotización
          setCarrito(ventaCompleta.detalles.map(d => ({
            producto_id: d.producto_id,
            codigo: undefined,
            nombre: d.nombre_producto || "",
            cantidad: d.cantidad,
            precio_unitario: d.precio_unitario,
            descuento: d.descuento,
            iva_porcentaje: d.iva_porcentaje,
            subtotal: d.subtotal,
            stock_disponible: 999,
            stock_minimo: 0,
            precio_base: d.precio_unitario,
            info_adicional: d.info_adicional || undefined,
          })));
          // Restaurar cliente si no es consumidor final
          if (ventaCompleta.venta.cliente_id && ventaCompleta.venta.cliente_id !== 1) {
            setClienteSeleccionado({
              id: ventaCompleta.venta.cliente_id,
              nombre: ventaCompleta.cliente_nombre || "",
              tipo_identificacion: "CONSUMIDOR_FINAL",
              activo: true,
            });
          }
          setFormaPago(ventaCompleta.venta.forma_pago);
          setTipoDocumento(ventaCompleta.venta.tipo_documento);
          setMontoRecibido("");
        }}
      />

      {/* Modal datos de Guía de Remisión */}
      {mostrarModalGuia && (
        <div className="modal-overlay" onClick={() => setMostrarModalGuia(false)}>
          <div className="modal-content" onClick={(e) => e.stopPropagation()} style={{ maxWidth: 420 }}>
            <div className="modal-header">
              <h3>Guia de Remision</h3>
            </div>
            <div className="modal-body" style={{ display: "flex", flexDirection: "column", gap: 12 }}>
              <p className="text-secondary" style={{ fontSize: 12, margin: 0 }}>
                Se descontara stock al crear la guia. Todos los campos son opcionales.
              </p>
              <div>
                <label className="text-secondary" style={{ fontSize: 12, display: "block", marginBottom: 4 }}>Placa del vehiculo</label>
                <input className="input" placeholder="Ej: ABC-1234" value={guiaPlaca}
                  onChange={(e) => setGuiaPlaca(e.target.value.toUpperCase())} autoFocus />
              </div>
              <div>
                <label className="text-secondary" style={{ fontSize: 12, display: "block", marginBottom: 4 }}>Chofer / Transportista</label>
                <input className="input" placeholder="Nombre del chofer" value={guiaChofer}
                  list="choferes-list"
                  onChange={(e) => {
                    setGuiaChofer(e.target.value);
                    // Si selecciona un chofer guardado, prellenar placa
                    const match = choferesGuardados.find(c => c[1] === e.target.value);
                    if (match && match[2] && !guiaPlaca) setGuiaPlaca(match[2]);
                  }} />
                <datalist id="choferes-list">
                  {choferesGuardados.map(c => (
                    <option key={c[0]} value={c[1]}>{c[2] ? `Placa: ${c[2]}` : ""}</option>
                  ))}
                </datalist>
              </div>
              <div>
                <label className="text-secondary" style={{ fontSize: 12, display: "block", marginBottom: 4 }}>Direccion de destino</label>
                <input className="input" placeholder="Direccion de entrega" value={guiaDireccion}
                  onChange={(e) => setGuiaDireccion(e.target.value)} />
              </div>
              <div style={{ fontSize: 12, padding: 8, borderRadius: "var(--radius)", background: "rgba(251, 146, 60, 0.1)", color: "var(--color-warning)" }}>
                {carrito.length} producto(s) — Total: ${total.toFixed(2)}
                {clienteSeleccionado && ` — ${clienteSeleccionado.nombre}`}
              </div>
            </div>
            <div className="modal-footer">
              <button className="btn btn-outline" onClick={() => setMostrarModalGuia(false)}>
                Cancelar
              </button>
              <button className="btn" disabled={guardandoGuia}
                style={{ background: "rgba(251, 146, 60, 0.2)", color: "#fb923c", border: "1px solid rgba(251, 146, 60, 0.4)", fontWeight: 600 }}
                onClick={confirmarGuiaRemision}>
                {guardandoGuia ? "Guardando..." : "Crear Guia de Remision"}
              </button>
            </div>
          </div>
        </div>
      )}
      {/* Modal PIN Admin para editar precio */}
      {productoDetalle && (
        <div className="modal-overlay" onClick={() => setProductoDetalle(null)}>
          <div className="modal-content" onClick={(e) => e.stopPropagation()} style={{ maxWidth: 480 }}>
            <div className="modal-header" style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
              <h3 style={{ margin: 0 }}>Detalle del Producto</h3>
              <button onClick={() => setProductoDetalle(null)} style={{ background: "none", border: "none", fontSize: 20, cursor: "pointer", color: "var(--color-text)" }}>×</button>
            </div>
            <div className="modal-body">
              {productoDetalle.imagen && (
                <img src={`data:image/png;base64,${productoDetalle.imagen}`} alt={productoDetalle.nombre}
                  style={{ width: 120, height: 120, objectFit: "contain", display: "block", margin: "0 auto 12px", borderRadius: 8 }} />
              )}
              <div style={{ fontSize: 18, fontWeight: 700, marginBottom: 8 }}>{productoDetalle.nombre}</div>
              {productoDetalle.descripcion && (
                <p style={{ fontSize: 13, color: "var(--color-text-secondary)", marginBottom: 12 }}>
                  {productoDetalle.descripcion}
                </p>
              )}
              <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: 8, fontSize: 13 }}>
                <div><strong>Código:</strong> {productoDetalle.codigo || "-"}</div>
                <div><strong>Código barras:</strong> {productoDetalle.codigo_barras || "-"}</div>
                <div><strong>Precio venta:</strong> <span style={{ color: "var(--color-primary)", fontWeight: 600 }}>${productoDetalle.precio_venta?.toFixed(2)}</span></div>
                <div><strong>Precio costo:</strong> ${productoDetalle.precio_costo?.toFixed(2)}</div>
                <div><strong>Stock actual:</strong> <span style={{ fontWeight: 600, color: productoDetalle.stock_actual <= 0 ? "var(--color-danger)" : undefined }}>{productoDetalle.stock_actual}</span></div>
                <div><strong>Stock mínimo:</strong> {productoDetalle.stock_minimo}</div>
                <div><strong>IVA:</strong> {productoDetalle.iva_porcentaje}%</div>
                <div><strong>Unidad:</strong> {productoDetalle.unidad_medida}</div>
              </div>
            </div>
            <div className="modal-footer" style={{ display: "flex", gap: 8, justifyContent: "flex-end" }}>
              <button className="btn btn-outline" onClick={() => setProductoDetalle(null)}>Cerrar</button>
              <button className="btn btn-primary" onClick={() => {
                const pid = productoDetalle.id;
                setProductoDetalle(null);
                navigate(`/productos?edit=${pid}`);
              }}>Editar Producto</button>
            </div>
          </div>
        </div>
      )}
      {mostrarPinAdmin && (
        <div className="modal-overlay" onClick={() => {
          setMostrarPinAdmin(false);
          pinResolveRef.current?.(false);
          pinResolveRef.current = null;
        }}>
          <div className="modal-content" onClick={(e) => e.stopPropagation()} style={{ maxWidth: 320 }}>
            <div className="modal-header">
              <h3>PIN de Administrador</h3>
            </div>
            <div className="modal-body" style={{ display: "flex", flexDirection: "column", gap: 12 }}>
              <p style={{ fontSize: 13, color: "var(--color-text-secondary)", margin: 0 }}>
                Ingrese el PIN de administrador para editar el precio.
              </p>
              <input
                className="input"
                type="password"
                inputMode="numeric"
                maxLength={6}
                placeholder="PIN (4-6 digitos)"
                value={pinAdminValor}
                onChange={(e) => {
                  setPinAdminValor(e.target.value.replace(/\D/g, ""));
                  setPinAdminError("");
                }}
                onKeyDown={async (e) => {
                  if (e.key === "Enter" && pinAdminValor.length >= 4) {
                    try {
                      await verificarPinAdmin(pinAdminValor);
                      setMostrarPinAdmin(false);
                      pinResolveRef.current?.(true);
                      pinResolveRef.current = null;
                    } catch {
                      setPinAdminError("PIN incorrecto");
                    }
                  }
                }}
                autoFocus
                style={{ fontSize: 18, textAlign: "center", letterSpacing: 8 }}
              />
              {pinAdminError && (
                <span style={{ color: "var(--color-danger)", fontSize: 12, textAlign: "center" }}>{pinAdminError}</span>
              )}
            </div>
            <div className="modal-footer">
              <button className="btn btn-outline" onClick={() => {
                setMostrarPinAdmin(false);
                pinResolveRef.current?.(false);
                pinResolveRef.current = null;
              }}>
                Cancelar
              </button>
              <button className="btn btn-primary" disabled={pinAdminValor.length < 4} onClick={async () => {
                try {
                  await verificarPinAdmin(pinAdminValor);
                  setMostrarPinAdmin(false);
                  pinResolveRef.current?.(true);
                  pinResolveRef.current = null;
                } catch {
                  setPinAdminError("PIN incorrecto");
                }
              }}>
                Confirmar
              </button>
            </div>
          </div>
        </div>
      )}

      {/* Modal: Seleccionar componentes de un COMBO_FLEXIBLE */}
      {seleccionCombo && (() => {
        const { producto, unidadElegida, grupos, componentes } = seleccionCombo;
        // Validacion mín/máx por grupo
        const validaciones = grupos.map((g: any) => {
          const sels = comboSel[String(g.id)] || {};
          const totalSel = Object.values(sels).reduce((a, b) => a + (Number(b) || 0), 0);
          const ok = totalSel >= g.minimo && totalSel <= g.maximo;
          return { grupo: g, total: totalSel, ok };
        });
        const todoOk = validaciones.every(v => v.ok);
        return (
          <div className="modal-overlay" onClick={() => { setSeleccionCombo(null); setComboSel({}); }}>
            <div className="modal-content" onClick={(e) => e.stopPropagation()} style={{ maxWidth: 620, maxHeight: "85vh", overflowY: "auto" }}>
              <div className="modal-header">
                <h3>🍽 Personalizar combo - {producto.nombre}</h3>
              </div>
              <div className="modal-body">
                <p style={{ fontSize: 12, color: "var(--color-text-secondary)", marginBottom: 12 }}>
                  Escoge los componentes para este combo según las reglas de cada grupo.
                </p>
                {grupos.map((g: any) => {
                  const opciones = componentes.filter((c: any) => c.grupo_id === g.id);
                  const sels = comboSel[String(g.id)] || {};
                  const totalSel = Object.values(sels).reduce((a, b) => a + (Number(b) || 0), 0);
                  const ok = totalSel >= g.minimo && totalSel <= g.maximo;
                  return (
                    <div key={g.id} style={{ marginBottom: 14, padding: 10, background: "var(--color-surface-alt)", borderRadius: 6, border: `1px solid ${ok ? "var(--color-success)" : "var(--color-warning)"}` }}>
                      <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: 6 }}>
                        <div style={{ fontWeight: 700, fontSize: 13 }}>{g.nombre}</div>
                        <div style={{ fontSize: 11, color: ok ? "var(--color-success)" : "var(--color-warning)" }}>
                          {g.minimo === g.maximo
                            ? `Selecciona exactamente ${g.minimo} (tienes ${totalSel})`
                            : `Selecciona entre ${g.minimo} y ${g.maximo} (tienes ${totalSel})`}
                        </div>
                      </div>
                      {opciones.length === 0 ? (
                        <div style={{ fontSize: 11, color: "var(--color-text-secondary)" }}>Sin opciones configuradas en este grupo.</div>
                      ) : opciones.map((c: any) => {
                        const cantSel = sels[c.producto_hijo_id] || 0;
                        const stockHijo = c.hijo_stock_actual ?? 0;
                        return (
                          <div key={c.id ?? c.producto_hijo_id} style={{ display: "flex", alignItems: "center", gap: 8, padding: "5px 0", borderBottom: "1px dashed var(--color-border)" }}>
                            <div style={{ flex: 1 }}>
                              <div style={{ fontSize: 12, fontWeight: 600 }}>{c.hijo_nombre}</div>
                              <div style={{ fontSize: 10, color: "var(--color-text-secondary)" }}>
                                Cantidad por combo: {c.cantidad} · Stock: {stockHijo} {c.hijo_unidad_medida || ""}
                              </div>
                            </div>
                            <div style={{ display: "flex", alignItems: "center", gap: 4 }}>
                              <button type="button" className="btn btn-outline" style={{ fontSize: 11, padding: "2px 8px" }}
                                onClick={() => {
                                  if (cantSel <= 0) return;
                                  setComboSel({ ...comboSel, [String(g.id)]: { ...sels, [c.producto_hijo_id]: cantSel - 1 } });
                                }}>−</button>
                              <span style={{ minWidth: 24, textAlign: "center", fontWeight: 700, fontSize: 13 }}>{cantSel}</span>
                              <button type="button" className="btn btn-outline" style={{ fontSize: 11, padding: "2px 8px" }}
                                disabled={totalSel >= g.maximo}
                                title={totalSel >= g.maximo ? "Máximo alcanzado en este grupo" : ""}
                                onClick={() => {
                                  if (totalSel >= g.maximo) return;
                                  setComboSel({ ...comboSel, [String(g.id)]: { ...sels, [c.producto_hijo_id]: cantSel + 1 } });
                                }}>+</button>
                            </div>
                          </div>
                        );
                      })}
                    </div>
                  );
                })}
              </div>
              <div className="modal-footer" style={{ display: "flex", justifyContent: "space-between", gap: 8 }}>
                <button className="btn btn-outline" onClick={() => { setSeleccionCombo(null); setComboSel({}); }}>Cancelar</button>
                <button className="btn btn-primary"
                  disabled={!todoOk}
                  title={todoOk ? "" : "Completa los grupos según mín/máx"}
                  onClick={() => {
                    // Construir la seleccion final como array
                    const seleccion: Array<{ producto_hijo_id: number; cantidad: number; grupo_id?: number; nombre?: string }> = [];
                    grupos.forEach((g: any) => {
                      const sels = comboSel[String(g.id)] || {};
                      Object.entries(sels).forEach(([hijoIdStr, cant]) => {
                        if (Number(cant) > 0) {
                          const compRef = componentes.find((c: any) => c.producto_hijo_id === Number(hijoIdStr) && c.grupo_id === g.id);
                          // cantidad por unidad de combo = c.cantidad * (cant escogido)
                          // Pero como cantidad de combos vendido es 1 aqui (se multiplica en backend con item.cantidad),
                          // mandamos: cantidad_total_componente_por_combo = compRef.cantidad * cant
                          const cantPorCombo = (compRef?.cantidad || 1) * Number(cant);
                          seleccion.push({
                            producto_hijo_id: Number(hijoIdStr),
                            cantidad: cantPorCombo,
                            grupo_id: g.id,
                            nombre: compRef?.hijo_nombre,
                          });
                        }
                      });
                    });
                    const prod = producto;
                    const uni = unidadElegida;
                    setSeleccionCombo(null);
                    setComboSel({});
                    agregarAlCarrito(prod, uni, undefined, seleccion);
                  }}>
                  Agregar al carrito
                </button>
              </div>
            </div>
          </div>
        );
      })()}

      {/* Modal: detalles de transferencia (cuenta + referencia + comprobante) */}
      {mostrarDetallesTransfer && (
        <div className="modal-overlay" onClick={() => setMostrarDetallesTransfer(false)}>
          <div className="modal-content" onClick={(e) => e.stopPropagation()} style={{ maxWidth: 460 }}>
            <div className="modal-header">
              <h3>🏦 Detalles de transferencia</h3>
            </div>
            <div className="modal-body">
              {cuentasBanco.length === 0 ? (
                <div style={{ padding: 12, background: "rgba(245,158,11,0.1)", border: "1px solid rgba(245,158,11,0.4)", borderRadius: 6, fontSize: 12, color: "var(--color-warning)" }}>
                  ⚠ No hay cuentas bancarias registradas. Vaya a Configuración → Cuentas Bancarias para crear una.
                </div>
              ) : (
                <>
                  <div style={{ marginBottom: 12 }}>
                    <label className="text-secondary" style={{ fontSize: 12, fontWeight: 600 }}>Cuenta destino *</label>
                    <select
                      className="input mt-1"
                      value={bancoSeleccionado ?? ""}
                      onChange={(e) => setBancoSeleccionado(e.target.value ? Number(e.target.value) : null)}
                    >
                      <option value="">Seleccionar cuenta...</option>
                      {cuentasBanco.map((cb: any) => (
                        <option key={cb.id} value={cb.id}>
                          {cb.nombre}{cb.numero_cuenta ? ` - ${cb.numero_cuenta}` : ""}
                        </option>
                      ))}
                    </select>
                  </div>

                  <div style={{ marginBottom: 12 }}>
                    <label className="text-secondary" style={{ fontSize: 12, fontWeight: 600 }}>
                      Nro. referencia {requiereReferencia && <span style={{ color: "var(--color-danger)" }}>*</span>}
                    </label>
                    <input className="input mt-1" placeholder="Ej: 123456789"
                      value={referenciaPago}
                      onChange={(e) => setReferenciaPago(e.target.value)} />
                  </div>

                  <div>
                    <label className="text-secondary" style={{ fontSize: 12, fontWeight: 600 }}>
                      Comprobante {requiereComprobante && <span style={{ color: "var(--color-danger)" }}>*</span>}
                    </label>
                    <div style={{ display: "flex", gap: 8, alignItems: "center", marginTop: 4 }}>
                      <input type="file" accept="image/*" style={{ flex: 1 }}
                        onChange={(e) => {
                          const file = e.target.files?.[0];
                          if (!file) return;
                          if (file.size > 500000) { toastError("Imagen muy grande (max 500KB)"); return; }
                          const reader = new FileReader();
                          reader.onload = () => setComprobanteImagen(reader.result as string);
                          reader.readAsDataURL(file);
                        }} />
                      {comprobanteImagen && (
                        <>
                          <span style={{ fontSize: 11, color: "var(--color-success)", fontWeight: 600 }}>✓ Cargado</span>
                          <button type="button"
                            onClick={() => setComprobanteImagen(null)}
                            style={{ background: "none", border: "none", cursor: "pointer", color: "var(--color-danger)", fontSize: 16 }}>×</button>
                        </>
                      )}
                    </div>
                    {comprobanteImagen && (
                      <img src={comprobanteImagen} alt="Comprobante" style={{ maxWidth: "100%", marginTop: 8, borderRadius: 4, border: "1px solid var(--color-border)" }} />
                    )}
                  </div>
                </>
              )}
            </div>
            <div className="modal-footer">
              <button className="btn btn-primary" style={{ width: "100%" }}
                onClick={() => setMostrarDetallesTransfer(false)}>
                Listo
              </button>
            </div>
          </div>
        </div>
      )}

      {/* Modal: cambiar precio / lista del item del carrito */}
      {editarPrecioItemModal && (() => {
        const m = editarPrecioItemModal;
        const cerrar = () => { setEditarPrecioItemModal(null); setPrecioManualInput(""); };
        const aplicarPrecio = (precio: number) => {
          if (isNaN(precio) || precio < 0) { toastError("Precio inválido"); return; }
          editarPrecioItem(m.idx, precio);
          cerrar();
        };
        return (
          <div className="modal-overlay" onClick={cerrar}>
            <div className="modal-content" onClick={(e) => e.stopPropagation()} style={{ maxWidth: 460 }}>
              <div className="modal-header">
                <h3>💰 Cambiar precio · {m.nombre}</h3>
              </div>
              <div className="modal-body">
                <div style={{ fontSize: 11, color: "var(--color-text-secondary)", marginBottom: 8 }}>
                  Precio actual: <strong style={{ color: "var(--color-primary)" }}>${m.precioActual.toFixed(2)}</strong>
                </div>

                {/* Listas disponibles — solo si admin/permiso cambiar_lista_precio */}
                {puedeCambiarListaPrecio && todasListasPrecios.length > 0 ? (
                  <>
                    <label className="text-secondary" style={{ fontSize: 12, fontWeight: 600 }}>Aplicar lista de precios</label>
                    <div style={{ display: "flex", flexDirection: "column", gap: 4, marginTop: 6, marginBottom: 12 }}>
                      {todasListasPrecios.map(lp => {
                        const especifico = m.preciosDisponibles.find(p => p.lista_precio_id === lp.id);
                        // Precio base: usa precio_base del item si existe, sino el precio actual como fallback
                        const itemEnCarrito = carrito[m.idx];
                        const precioBase = (itemEnCarrito as any)?.precio_base ?? m.precioActual;
                        const precioAplicable = especifico ? especifico.precio : precioBase;
                        const esActual = Math.abs(precioAplicable - m.precioActual) < 0.001;
                        return (
                          <button key={lp.id} type="button"
                            onClick={() => aplicarPrecio(precioAplicable)}
                            style={{
                              display: "flex", justifyContent: "space-between", alignItems: "center",
                              padding: "8px 12px", borderRadius: 6, cursor: "pointer",
                              background: esActual ? "rgba(34,197,94,0.1)" : "var(--color-surface-alt)",
                              border: `1px solid ${esActual ? "rgba(34,197,94,0.4)" : "var(--color-border)"}`,
                              fontSize: 13,
                            }}>
                            <span style={{ fontWeight: 600 }}>
                              {esActual && "✓ "}{lp.nombre}{lp.es_default ? " ⭐" : ""}
                              {!especifico && (
                                <span style={{ fontSize: 10, color: "var(--color-text-secondary)", marginLeft: 6, fontWeight: 400 }}>(precio base)</span>
                              )}
                            </span>
                            <span style={{ fontWeight: 700, color: "var(--color-primary)" }}>${precioAplicable.toFixed(2)}</span>
                          </button>
                        );
                      })}
                    </div>
                  </>
                ) : puedeCambiarListaPrecio && todasListasPrecios.length === 0 ? (
                  <div style={{ fontSize: 11, color: "var(--color-text-secondary)", marginBottom: 12, padding: 8, background: "rgba(245,158,11,0.08)", borderRadius: 4 }}>
                    No hay listas de precios definidas. Crea una en Configuración → Listas de Precios.
                  </div>
                ) : null}

                {/* Precio manual */}
                {tienePermiso("editar_precio") || esAdmin ? (
                  <>
                    <label className="text-secondary" style={{ fontSize: 12, fontWeight: 600 }}>O ingrese un precio manual</label>
                    <div style={{ display: "flex", gap: 6, marginTop: 6 }}>
                      <input
                        className="input"
                        type="number" step="0.01" min="0"
                        value={precioManualInput}
                        autoFocus
                        onChange={(e) => setPrecioManualInput(e.target.value)}
                        onKeyDown={(e) => { if (e.key === "Enter") aplicarPrecio(parseFloat(precioManualInput)); }}
                        style={{ flex: 1, fontSize: 14, fontWeight: 600 }}
                      />
                      <button className="btn btn-primary"
                        onClick={() => aplicarPrecio(parseFloat(precioManualInput))}>
                        Aplicar
                      </button>
                    </div>
                  </>
                ) : (
                  <div style={{ fontSize: 11, color: "var(--color-text-secondary)", fontStyle: "italic" }}>
                    No tienes permiso para precio manual. Selecciona una lista arriba.
                  </div>
                )}
              </div>
              <div className="modal-footer">
                <button className="btn btn-outline" onClick={cerrar}>Cancelar</button>
              </div>
            </div>
          </div>
        );
      })()}

      {/* Modal: Seleccionar lote de caducidad (FEFO) */}
      {seleccionLote && (() => {
        // Lotes ordenados por fecha caducidad ascendente (FEFO)
        const sorted = [...seleccionLote.lotes].sort((a, b) =>
          new Date(a.fecha_caducidad).getTime() - new Date(b.fecha_caducidad).getTime()
        );
        const sumaLotes = sorted.reduce((a, l) => a + (Number(l.cantidad) || 0), 0);
        const stockProd = Number(seleccionLote.producto.stock_actual ?? 0);
        const stockLibre = Math.max(0, stockProd - sumaLotes);
        const cantPedida = Math.max(1, parseFloat(seleccionLoteCantidad) || 1);
        return (
          <div className="modal-overlay" onClick={() => { setSeleccionLote(null); setSeleccionLoteCantidad("1"); }}>
            <div className="modal-content" onClick={(e) => e.stopPropagation()} style={{ maxWidth: 560 }}>
              <div className="modal-header">
                <h3>🕐 Seleccionar lote - {seleccionLote.producto.nombre}</h3>
              </div>
              <div className="modal-body">
                <p style={{ fontSize: 12, color: "var(--color-text-secondary)", marginBottom: 12 }}>
                  El sistema sugiere el lote con fecha mas proxima a vencer (FEFO).
                  Click para seleccionar otro si es necesario.
                </p>
                <div style={{ fontSize: 11, color: "var(--color-text-secondary)", marginBottom: 10, display: "flex", gap: 14, flexWrap: "wrap" }}>
                  <span>Stock total: <strong style={{ color: "var(--color-text)" }}>{stockProd}</strong></span>
                  <span>En lotes: <strong style={{ color: "var(--color-text)" }}>{sumaLotes}</strong></span>
                  <span style={{ color: stockLibre > 0 ? "var(--color-warning)" : "var(--color-text-secondary)" }}>
                    Stock libre (sin lote): <strong>{stockLibre}</strong>
                  </span>
                </div>

                {/* Input: cantidad a vender */}
                <div style={{ display: "flex", alignItems: "center", gap: 10, padding: "8px 12px", marginBottom: 12, background: "var(--color-surface-alt)", borderRadius: 6, border: "1px solid var(--color-border)" }}>
                  <label style={{ fontSize: 12, fontWeight: 600 }}>Cantidad a vender:</label>
                  <input className="input" type="number" min="0.01" step="any"
                    style={{ width: 90, fontSize: 13, textAlign: "right" }}
                    value={seleccionLoteCantidad}
                    onChange={(e) => setSeleccionLoteCantidad(e.target.value)}
                    autoFocus />
                  <span style={{ fontSize: 11, color: "var(--color-text-secondary)" }}>
                    {seleccionLote.unidadElegida?.nombre || ""}
                  </span>
                </div>

                <div style={{ display: "flex", flexDirection: "column", gap: 6 }}>
                  {sorted.map((l, idx) => {
                    const dias = Math.floor((new Date(l.fecha_caducidad).getTime() - new Date().getTime()) / (1000 * 60 * 60 * 24));
                    const esVencido = dias < 0;
                    const esPorVencer = !esVencido && dias <= 7;
                    const esFEFO = idx === 0;
                    const cantLote = Number(l.cantidad) || 0;
                    const cubreCompleto = cantLote >= cantPedida;
                    const cubreParcial = cantLote > 0 && cantLote < cantPedida;
                    const faltante = Math.max(0, cantPedida - cantLote);
                    // Color por disponibilidad: VENCIDO rojo, POR_VENCER ambar, OK con suficiente verde, parcial amarillo
                    const colorEstado = esVencido
                      ? { bg: "rgba(239,68,68,0.10)", border: "rgba(239,68,68,0.4)" }
                      : cubreParcial
                        ? { bg: "rgba(245,158,11,0.12)", border: "rgba(245,158,11,0.5)" }
                        : esPorVencer
                          ? { bg: "rgba(245,158,11,0.08)", border: "rgba(245,158,11,0.35)" }
                          : cubreCompleto
                            ? { bg: "rgba(34,197,94,0.10)", border: "rgba(34,197,94,0.4)" }
                            : { bg: "var(--color-surface-alt)", border: "var(--color-border)" };
                    const disabled = !cubreCompleto && !cubreParcial; // sin nada
                    return (
                      <button key={l.id} type="button"
                        disabled={disabled}
                        style={{
                          padding: "10px 14px", borderRadius: 6,
                          cursor: disabled ? "not-allowed" : "pointer",
                          opacity: disabled ? 0.5 : 1,
                          textAlign: "left",
                          background: colorEstado.bg,
                          border: `1px solid ${colorEstado.border}`,
                          display: "flex", justifyContent: "space-between", alignItems: "center", gap: 8,
                        }}
                        title={cubreParcial ? `Solo cubre ${cantLote} de ${cantPedida}. Faltarian ${faltante} (puedes complementar con stock libre o otro lote despues).` : ""}
                        onClick={() => {
                          if (disabled) return;
                          const prod = seleccionLote.producto;
                          const uni = seleccionLote.unidadElegida;
                          // Si solo cubre parcial, agregamos lo que tiene el lote (cantLote);
                          // el cajero puede luego agregar otro lote para completar.
                          const cantFinal = cubreCompleto ? cantPedida : cantLote;
                          setSeleccionLote(null);
                          setSeleccionLoteCantidad("1");
                          agregarAlCarrito(prod, uni, { ...l, _cantidadVenta: cantFinal });
                        }}>
                        <div style={{ flex: 1 }}>
                          <div style={{ fontWeight: 700, fontSize: 13, color: esVencido ? "var(--color-danger)" : undefined }}>
                            {esFEFO && cubreCompleto && <span style={{ fontSize: 10, marginRight: 6, padding: "1px 5px", borderRadius: 3, background: "var(--color-success)", color: "#fff" }}>FEFO</span>}
                            {cubreParcial && <span style={{ fontSize: 10, marginRight: 6, padding: "1px 5px", borderRadius: 3, background: "var(--color-warning)", color: "#fff" }}>PARCIAL</span>}
                            Lote {l.lote || `#${l.id}`}
                            {esVencido && <span style={{ marginLeft: 6, fontSize: 11 }}>⚠ VENCIDO</span>}
                          </div>
                          <div style={{ fontSize: 11, color: "var(--color-text-secondary)", marginTop: 2 }}>
                            {l.fecha_elaboracion && <>Elab: {l.fecha_elaboracion} · </>}
                            Vence: <strong>{l.fecha_caducidad}</strong>
                            {" "}
                            {esVencido ? `(hace ${Math.abs(dias)}d)` : `(en ${dias}d)`}
                            {cubreParcial && <span style={{ color: "var(--color-warning)" }}> · faltarian {faltante}</span>}
                          </div>
                        </div>
                        <div style={{ textAlign: "right", fontSize: 12 }}>
                          <div style={{ fontWeight: 700, color: cubreCompleto ? "var(--color-success)" : cubreParcial ? "var(--color-warning)" : undefined }}>
                            {cantLote}
                          </div>
                          <div style={{ fontSize: 10, color: "var(--color-text-secondary)" }}>disponibles</div>
                        </div>
                      </button>
                    );
                  })}
                </div>
              </div>
              <div className="modal-footer" style={{ display: "flex", justifyContent: "space-between", gap: 8 }}>
                {stockLibre > 0 ? (
                  <button className="btn btn-outline"
                    style={{ fontSize: 11, opacity: stockLibre >= cantPedida ? 1 : 0.6 }}
                    disabled={stockLibre < cantPedida}
                    title={stockLibre >= cantPedida
                      ? `Vende ${cantPedida} del stock libre (no asignado a lote). Libre actual: ${stockLibre}`
                      : `No hay stock libre suficiente: pedido ${cantPedida}, libre ${stockLibre}`}
                    onClick={() => {
                      const prod = seleccionLote.producto;
                      const uni = seleccionLote.unidadElegida;
                      setSeleccionLote(null);
                      setSeleccionLoteCantidad("1");
                      agregarAlCarrito(prod, uni, { id: null, _cantidadVenta: cantPedida });
                    }}>
                    {stockLibre >= cantPedida
                      ? `Vender ${cantPedida} sin lote (libre: ${stockLibre})`
                      : `Sin lote insuficiente (${stockLibre} disp.)`}
                  </button>
                ) : <span />}
                <button className="btn btn-outline" onClick={() => { setSeleccionLote(null); setSeleccionLoteCantidad("1"); }}>Cancelar</button>
              </div>
            </div>
          </div>
        );
      })()}

      {/* Modal: Cambiar lote de item ya en carrito */}
      {cambiarLoteItem && (() => {
        const sorted = [...cambiarLoteItem.lotes].sort((a, b) =>
          new Date(a.fecha_caducidad).getTime() - new Date(b.fecha_caducidad).getTime()
        );
        const itemActual = carrito[cambiarLoteItem.idx];
        return (
          <div className="modal-overlay" onClick={() => setCambiarLoteItem(null)}>
            <div className="modal-content" onClick={(e) => e.stopPropagation()} style={{ maxWidth: 560 }}>
              <div className="modal-header"><h3>🕐 Cambiar lote - {itemActual?.nombre}</h3></div>
              <div className="modal-body">
                <div style={{ display: "flex", flexDirection: "column", gap: 6 }}>
                  {sorted.map((l) => {
                    const dias = Math.floor((new Date(l.fecha_caducidad).getTime() - new Date().getTime()) / (1000 * 60 * 60 * 24));
                    const esActual = l.id === itemActual?.lote_id;
                    const esVencido = dias < 0;
                    return (
                      <button key={l.id} type="button"
                        style={{
                          padding: "10px 14px", borderRadius: 6, cursor: "pointer",
                          textAlign: "left",
                          background: esActual ? "rgba(59,130,246,0.15)" : esVencido ? "rgba(239,68,68,0.08)" : "var(--color-surface-alt)",
                          border: `2px solid ${esActual ? "var(--color-primary)" : "var(--color-border)"}`,
                          display: "flex", justifyContent: "space-between", alignItems: "center", gap: 8,
                        }}
                        onClick={() => {
                          const idx = cambiarLoteItem.idx;
                          setCarrito(prev => prev.map((it, k) => k === idx ? {
                            ...it,
                            lote_id: l.id,
                            lote_numero: l.lote,
                            lote_fecha_caducidad: l.fecha_caducidad,
                            lote_dias_restantes: dias,
                          } : it));
                          setCambiarLoteItem(null);
                        }}>
                        <div style={{ flex: 1 }}>
                          <div style={{ fontWeight: 700, fontSize: 13 }}>
                            {esActual && <span style={{ fontSize: 10, marginRight: 6, padding: "1px 5px", borderRadius: 3, background: "var(--color-primary)", color: "#fff" }}>ACTUAL</span>}
                            Lote {l.lote || `#${l.id}`}
                            {esVencido && <span style={{ marginLeft: 6, fontSize: 11, color: "var(--color-danger)" }}>⚠ VENCIDO</span>}
                          </div>
                          <div style={{ fontSize: 11, color: "var(--color-text-secondary)", marginTop: 2 }}>
                            Vence: <strong>{l.fecha_caducidad}</strong> {esVencido ? `(hace ${Math.abs(dias)}d)` : `(en ${dias}d)`}
                          </div>
                        </div>
                        <div style={{ textAlign: "right", fontSize: 12 }}>
                          <div style={{ fontWeight: 700 }}>{l.cantidad}</div>
                          <div style={{ fontSize: 10, color: "var(--color-text-secondary)" }}>disp.</div>
                        </div>
                      </button>
                    );
                  })}
                </div>
              </div>
              <div className="modal-footer" style={{ display: "flex", justifyContent: "flex-end" }}>
                <button className="btn btn-outline" onClick={() => setCambiarLoteItem(null)}>Cancelar</button>
              </div>
            </div>
          </div>
        );
      })()}

      {/* Modal: Seleccionar unidad de venta (multi-unidad) */}
      {seleccionUnidad && (
        <div className="modal-overlay" onClick={() => setSeleccionUnidad(null)}>
          <div className="modal-content" onClick={(e) => e.stopPropagation()} style={{ maxWidth: 480 }}>
            <div className="modal-header"><h3>Seleccionar presentacion - {seleccionUnidad.producto.nombre}</h3></div>
            <div className="modal-body">
              <p style={{ fontSize: 12, color: "var(--color-text-secondary)", marginBottom: 12 }}>
                Este producto tiene varias presentaciones. Elija la que va a vender:
              </p>
              <div style={{ display: "flex", flexDirection: "column", gap: 6 }}>
                {/* Opcion: unidad base (sin presentacion) */}
                <button type="button"
                  style={{
                    padding: "10px 14px", borderRadius: 6, cursor: "pointer",
                    background: "var(--color-surface-alt)",
                    border: "1px solid var(--color-border)",
                    color: "var(--color-text)",
                    display: "flex", justifyContent: "space-between", alignItems: "center",
                    fontWeight: 500,
                  }}
                  onClick={() => {
                    const p = seleccionUnidad.producto;
                    setSeleccionUnidad(null);
                    agregarAlCarrito(p, { id: null, nombre: seleccionUnidad.producto.precio_venta != null ? "Unidad base" : "UND", abreviatura: "UND", factor: 1, precio: p.precio_lista ?? p.precio_venta });
                  }}>
                  <span>📦 Unidad individual (UND)</span>
                  <span style={{ fontWeight: 700, color: "var(--color-primary)" }}>${(seleccionUnidad.producto.precio_lista ?? seleccionUnidad.producto.precio_venta).toFixed(2)}</span>
                </button>
                {seleccionUnidad.unidades.map((u) => (
                  <button key={u.id} type="button"
                    style={{
                      padding: "10px 14px", borderRadius: 6, cursor: "pointer",
                      background: "rgba(59,130,246,0.1)",
                      border: "1px solid rgba(59,130,246,0.4)",
                      color: "var(--color-text)",
                      display: "flex", justifyContent: "space-between", alignItems: "center",
                      fontWeight: 500,
                    }}
                    onClick={() => {
                      const p = seleccionUnidad.producto;
                      setSeleccionUnidad(null);
                      agregarAlCarrito(p, u);
                    }}>
                    <span>
                      <strong>{u.nombre}</strong>
                      {u.abreviatura && u.abreviatura !== u.nombre && <span style={{ marginLeft: 6, fontSize: 11, color: "var(--color-text-secondary)" }}>({u.abreviatura})</span>}
                      <span style={{ marginLeft: 8, fontSize: 11, color: "var(--color-text-secondary)" }}>= {u.factor} und base</span>
                    </span>
                    <span style={{ fontWeight: 700, color: "var(--color-primary)" }}>${u.precio.toFixed(2)}</span>
                  </button>
                ))}
              </div>
            </div>
            <div className="modal-footer" style={{ display: "flex", justifyContent: "flex-end" }}>
              <button className="btn btn-outline" onClick={() => setSeleccionUnidad(null)}>Cancelar</button>
            </div>
          </div>
        </div>
      )}

      {/* Modal: Agregar pago a la lista de pago mixto */}
      {mostrarAddPago && (() => {
        const monto = parseFloat(addPagoMonto || "0");
        const aplicar = () => {
          if (monto <= 0) { toastError("Monto debe ser mayor a 0"); return; }
          if (addPagoForma === "TRANSFER" && !addPagoBancoId) { toastError("Seleccione cuenta bancaria"); return; }
          setPagosMixtos(prev => [...prev, {
            forma_pago: addPagoForma,
            monto,
            banco_id: addPagoForma === "TRANSFER" ? addPagoBancoId : null,
            referencia: addPagoForma === "TRANSFER" ? (addPagoReferencia.trim() || null) : null,
          }]);
          setMostrarAddPago(false);
          setAddPagoMonto(""); setAddPagoReferencia(""); setAddPagoBancoId(null);
        };
        const sumaActual = pagosMixtos.reduce((s, p) => s + p.monto, 0);
        const faltaActual = total - sumaActual;
        return (
          <div className="modal-overlay" onClick={() => setMostrarAddPago(false)}>
            <div className="modal-content" onClick={(e) => e.stopPropagation()} style={{ maxWidth: 380 }}>
              <div className="modal-header"><h3>Agregar pago: {addPagoForma}</h3></div>
              <div className="modal-body">
                <div style={{ padding: 8, background: "var(--color-surface-alt)", borderRadius: 4, marginBottom: 10, fontSize: 11 }}>
                  Total venta: <strong>${total.toFixed(2)}</strong> · Ya pagado: <strong>${sumaActual.toFixed(2)}</strong> · Falta: <strong style={{ color: "var(--color-warning)" }}>${faltaActual.toFixed(2)}</strong>
                </div>
                <label className="text-secondary" style={{ fontSize: 12, display: "block", marginBottom: 4 }}>Monto</label>
                <input className="input" type="number" step="0.01" min="0" autoFocus
                  value={addPagoMonto}
                  onChange={(e) => setAddPagoMonto(e.target.value)}
                  onKeyDown={(e) => { if (e.key === "Enter") aplicar(); }} />
                {addPagoForma === "TRANSFER" && (
                  <>
                    <label className="text-secondary" style={{ fontSize: 12, display: "block", marginTop: 10, marginBottom: 4 }}>Cuenta bancaria</label>
                    <select className="input" value={addPagoBancoId ?? ""}
                      onChange={(e) => setAddPagoBancoId(e.target.value ? Number(e.target.value) : null)}>
                      <option value="">Seleccione...</option>
                      {cuentasBanco.filter(b => b.activa !== false).map(b => (
                        <option key={b.id} value={b.id ?? ""}>{b.nombre}{b.tipo_cuenta ? ` (${b.tipo_cuenta})` : ""}</option>
                      ))}
                    </select>
                    <label className="text-secondary" style={{ fontSize: 12, display: "block", marginTop: 10, marginBottom: 4 }}>Referencia (opcional)</label>
                    <input className="input" placeholder="Nro. comprobante / referencia"
                      value={addPagoReferencia}
                      onChange={(e) => setAddPagoReferencia(e.target.value)} />
                  </>
                )}
                {addPagoForma === "CREDITO" && (
                  <div style={{ marginTop: 10, padding: 8, background: "rgba(217,119,6,0.1)", borderRadius: 4, fontSize: 11, color: "var(--color-warning)" }}>
                    Se creara cuenta por cobrar a {clienteSeleccionado?.nombre || "..."} por este monto.
                    {(!clienteSeleccionado || clienteSeleccionado.id === 1) && (
                      <div style={{ marginTop: 4, fontWeight: 700 }}>
                        ⚠ Debe seleccionar un cliente identificado primero
                      </div>
                    )}
                  </div>
                )}
              </div>
              <div className="modal-footer" style={{ display: "flex", gap: 8, justifyContent: "flex-end" }}>
                <button className="btn btn-outline" onClick={() => setMostrarAddPago(false)}>Cancelar</button>
                <button className="btn btn-primary" onClick={aplicar}>Agregar pago</button>
              </div>
            </div>
          </div>
        );
      })()}

      {/* Modal Descuento por item */}
      {descuentoItemId !== null && (() => {
        const item = carrito[descuentoItemId];
        if (!item) return null;
        const baseItem = item.cantidad * item.precio_unitario;
        const descCalc = descuentoTipo === "porcentaje"
          ? (baseItem * (parseFloat(descuentoValor || "0") / 100))
          : parseFloat(descuentoValor || "0");
        const descClampeado = Math.max(0, Math.min(baseItem, descCalc));
        const finalSubtotal = baseItem - descClampeado;
        const aplicar = () => {
          aplicarDescuentoItem(descuentoItemId, descClampeado);
          setDescuentoItemId(null); setDescuentoValor("");
        };
        const quitar = () => {
          aplicarDescuentoItem(descuentoItemId, 0);
          setDescuentoItemId(null); setDescuentoValor("");
        };
        return (
          <div className="modal-overlay" onClick={() => setDescuentoItemId(null)}>
            <div className="modal-content" onClick={(e) => e.stopPropagation()} style={{ maxWidth: 420 }}>
              <div className="modal-header"><h3>Descuento - {item.nombre}</h3></div>
              <div className="modal-body">
                <div style={{ display: "flex", gap: 8, marginBottom: 12 }}>
                  <button className={descuentoTipo === "porcentaje" ? "btn btn-primary" : "btn btn-outline"} style={{ flex: 1 }}
                    onClick={() => { setDescuentoTipo("porcentaje"); setDescuentoValor(""); }}>
                    Porcentaje (%)
                  </button>
                  <button className={descuentoTipo === "monto" ? "btn btn-primary" : "btn btn-outline"} style={{ flex: 1 }}
                    onClick={() => { setDescuentoTipo("monto"); setDescuentoValor(""); }}>
                    Monto fijo ($)
                  </button>
                </div>
                <label className="text-secondary" style={{ fontSize: 12, display: "block", marginBottom: 4 }}>
                  {descuentoTipo === "porcentaje" ? "Porcentaje de descuento" : "Monto a descontar"}
                </label>
                <input className="input" type="number" step="0.01" min="0"
                  placeholder={descuentoTipo === "porcentaje" ? "Ej: 10 (= 10%)" : "Ej: 0.50"}
                  value={descuentoValor}
                  onChange={(e) => setDescuentoValor(e.target.value)}
                  onKeyDown={(e) => { if (e.key === "Enter") aplicar(); }}
                  autoFocus />
                <div style={{ marginTop: 12, padding: 10, background: "var(--color-surface-alt)", borderRadius: 6, fontSize: 12 }}>
                  <div className="flex justify-between"><span>Base ({item.cantidad} x ${item.precio_unitario.toFixed(2)})</span><span>${baseItem.toFixed(2)}</span></div>
                  <div className="flex justify-between" style={{ color: "var(--color-warning)" }}><span>Descuento</span><span>-${descClampeado.toFixed(2)}</span></div>
                  <div className="flex justify-between" style={{ borderTop: "1px solid var(--color-border)", marginTop: 6, paddingTop: 6, fontWeight: 700 }}>
                    <span>Subtotal del item</span><span>${finalSubtotal.toFixed(2)}</span>
                  </div>
                </div>
              </div>
              <div className="modal-footer" style={{ display: "flex", gap: 8, justifyContent: "space-between" }}>
                {item.descuento > 0 && (
                  <button className="btn btn-outline" onClick={quitar} style={{ color: "var(--color-danger)" }}>
                    Quitar descuento
                  </button>
                )}
                <div style={{ marginLeft: "auto", display: "flex", gap: 8 }}>
                  <button className="btn btn-outline" onClick={() => setDescuentoItemId(null)}>Cancelar</button>
                  <button className="btn btn-primary" onClick={aplicar}>Aplicar</button>
                </div>
              </div>
            </div>
          </div>
        );
      })()}

      {/* Modal Información Adicional */}
      {infoAdicionalProductoId !== null && (() => {
        const guardarInfo = () => {
          const partes: string[] = [];
          if (infoSerie.trim()) partes.push(`Serie: ${infoSerie.trim()}`);
          if (infoLote.trim()) partes.push(`Lote: ${infoLote.trim()}`);
          if (infoObservacion.trim()) partes.push(`Obs: ${infoObservacion.trim()}`);
          const valor = partes.join(" | ") || undefined;
          setCarrito(prev => prev.map((i, k) => k === infoAdicionalProductoId ? { ...i, info_adicional: valor } : i));
          setInfoAdicionalProductoId(null);
        };
        const itemActual = carrito[infoAdicionalProductoId as number];
        const idxActual = infoAdicionalProductoId as number;
        const preciosDisp = itemActual?.precios_disponibles || [];
        return (
          <div className="modal-overlay" onClick={() => setInfoAdicionalProductoId(null)}>
            <div className="modal-content" onClick={(e) => e.stopPropagation()} style={{ maxWidth: 500 }}>
              <div className="modal-header" style={{ display: "flex", flexDirection: "column", alignItems: "flex-start", gap: 4 }}>
                <h3 style={{ margin: 0 }}>Editar item del carrito</h3>
                {itemActual && (
                  <div style={{ fontSize: 12, color: "var(--color-text-secondary)", fontWeight: 400 }}>
                    📦 <strong style={{ color: "var(--color-text)" }}>{itemActual.nombre}</strong>
                    {itemActual.unidad_nombre && (
                      <span style={{ marginLeft: 6, padding: "1px 6px", borderRadius: 3,
                        background: "rgba(59,130,246,0.15)", color: "var(--color-primary)",
                        fontWeight: 700, fontSize: 11,
                      }}>
                        {itemActual.unidad_nombre} ×{itemActual.factor_unidad}
                      </span>
                    )}
                    <span style={{ marginLeft: 8, fontSize: 11 }}>
                      Cant: {itemActual.cantidad} · <strong style={{ color: "var(--color-primary)" }}>${itemActual.precio_unitario.toFixed(2)}</strong>
                    </span>
                  </div>
                )}
              </div>
              <div className="modal-body">
                {/* === Cambiar lista de precios / precio manual === */}
                {(puedeCambiarListaPrecio || tienePermiso("editar_precio") || esAdmin) && (
                  <div style={{ padding: 10, background: "rgba(168,85,247,0.06)", border: "1px solid rgba(168,85,247,0.25)", borderRadius: 6, marginBottom: 12 }}>
                    <div style={{ fontSize: 12, fontWeight: 700, marginBottom: 6, color: "var(--color-text)" }}>💰 Tarifa / Precio</div>
                    {puedeCambiarListaPrecio && todasListasPrecios.length > 0 ? (
                      <div style={{ display: "flex", flexDirection: "column", gap: 4, marginBottom: 8 }}>
                        {todasListasPrecios.map(lp => {
                          const especifico = preciosDisp.find(p => p.lista_precio_id === lp.id);
                          const precioBase = (itemActual as any)?.precio_base ?? itemActual?.precio_unitario ?? 0;
                          const precioAplicable = especifico ? especifico.precio : precioBase;
                          const esActual = Math.abs(precioAplicable - (itemActual?.precio_unitario ?? 0)) < 0.001;
                          return (
                            <button key={lp.id} type="button"
                              onClick={() => {
                                editarPrecioItem(idxActual, precioAplicable);
                              }}
                              style={{
                                display: "flex", justifyContent: "space-between", alignItems: "center",
                                padding: "6px 10px", borderRadius: 4, cursor: "pointer",
                                background: esActual ? "rgba(34,197,94,0.12)" : "var(--color-surface-alt)",
                                border: `1px solid ${esActual ? "rgba(34,197,94,0.5)" : "var(--color-border)"}`,
                                fontSize: 12,
                              }}>
                              <span style={{ fontWeight: 600 }}>
                                {esActual && "✓ "}{lp.nombre}{lp.es_default ? " ⭐" : ""}
                                {!especifico && (
                                  <span style={{ fontSize: 10, color: "var(--color-text-secondary)", marginLeft: 6, fontWeight: 400 }}>(precio base)</span>
                                )}
                              </span>
                              <span style={{ fontWeight: 700, color: "var(--color-primary)" }}>${precioAplicable.toFixed(2)}</span>
                            </button>
                          );
                        })}
                      </div>
                    ) : puedeCambiarListaPrecio && todasListasPrecios.length === 0 ? (
                      <div style={{ fontSize: 11, color: "var(--color-text-secondary)", marginBottom: 8 }}>
                        No hay listas de precios definidas. Crea una en Configuración → Listas de Precios.
                      </div>
                    ) : null}
                    {(tienePermiso("editar_precio") || esAdmin) && (
                      <div style={{ display: "flex", gap: 6, alignItems: "center" }}>
                        <label style={{ fontSize: 11, color: "var(--color-text-secondary)" }}>Manual:</label>
                        <input
                          className="input"
                          type="number" step="0.01" min="0"
                          defaultValue={itemActual?.precio_unitario.toFixed(2)}
                          onKeyDown={(e) => {
                            if (e.key === "Enter") {
                              const v = parseFloat((e.target as HTMLInputElement).value);
                              if (!isNaN(v) && v >= 0) {
                                editarPrecioItem(idxActual, v);
                                (e.target as HTMLInputElement).blur();
                              }
                            }
                          }}
                          onBlur={(e) => {
                            const v = parseFloat(e.target.value);
                            if (!isNaN(v) && v >= 0 && Math.abs(v - (itemActual?.precio_unitario ?? 0)) > 0.001) {
                              editarPrecioItem(idxActual, v);
                            }
                          }}
                          style={{ flex: 1, fontSize: 13, fontWeight: 600 }}
                        />
                        <span style={{ fontSize: 10, color: "var(--color-text-secondary)" }}>(Enter o salir = aplicar)</span>
                      </div>
                    )}
                  </div>
                )}

                {/* === Información adicional === */}
                <div style={{ fontSize: 12, fontWeight: 700, marginBottom: 6, color: "var(--color-text)" }}>📝 Información adicional</div>
                <div style={{ display: "flex", flexDirection: "column", gap: 10 }}>
                  <div>
                    <label className="text-secondary" style={{ fontSize: 12, display: "block", marginBottom: 4 }}>Número de serie</label>
                    <input className="input" placeholder="Ej: SN-12345678"
                      value={infoSerie} onChange={(e) => setInfoSerie(e.target.value)}
                      onKeyDown={(e) => { if (e.key === "Enter") guardarInfo(); }} />
                  </div>
                  <div>
                    <label className="text-secondary" style={{ fontSize: 12, display: "block", marginBottom: 4 }}>Lote</label>
                    <input className="input" placeholder="Ej: LOTE-A001"
                      value={infoLote} onChange={(e) => setInfoLote(e.target.value)}
                      onKeyDown={(e) => { if (e.key === "Enter") guardarInfo(); }} />
                  </div>
                  <div>
                    <label className="text-secondary" style={{ fontSize: 12, display: "block", marginBottom: 4 }}>Observación</label>
                    <input className="input" placeholder="Ej: Color rojo, talla M..."
                      value={infoObservacion} onChange={(e) => setInfoObservacion(e.target.value)}
                      onKeyDown={(e) => { if (e.key === "Enter") guardarInfo(); }} />
                  </div>
                </div>
              </div>
              <div className="modal-footer">
                <button className="btn btn-outline" onClick={() => {
                  setInfoSerie(""); setInfoLote(""); setInfoObservacion("");
                  setCarrito(prev => prev.map((i, k) => k === infoAdicionalProductoId ? { ...i, info_adicional: undefined } : i));
                  setInfoAdicionalProductoId(null);
                }}>Limpiar info</button>
                <button className="btn btn-primary" onClick={guardarInfo}>Guardar y cerrar</button>
              </div>
            </div>
          </div>
        );
      })()}
    </div>
  );
}
