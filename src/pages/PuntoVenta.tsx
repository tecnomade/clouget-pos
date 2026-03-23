import { useState, useRef, useEffect, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { buscarProductos, productosMasVendidos, registrarVenta, buscarClientes, crearCliente, imprimirTicket, imprimirTicketPdf, obtenerCajaAbierta, alertasStockBajo, obtenerConfig, guardarConfig, emitirFacturaSri, consultarEstadoSri, cambiarAmbienteSri, enviarNotificacionSri, actualizarCliente, imprimirRide, obtenerXmlFirmado, procesarEmailsPendientes, resolverPrecioProducto, obtenerPreciosProducto, listarProductosTactil, listarCategorias, consultarIdentificacion, listarCuentasBanco, guardarBorrador, guardarCotizacion, guardarGuiaRemision, listarChoferes, guardarChofer } from "../services/api";
import type { AlertaStock } from "../services/api";
import { save } from "@tauri-apps/plugin-dialog";
import { useToast } from "../components/Toast";
import { useNavigate } from "react-router-dom";
import ModalEmailCliente from "../components/ModalEmailCliente";
import PosGridTactil from "../components/PosGridTactil";
import DocumentosRecientes from "../components/DocumentosRecientes";
import type { ProductoBusqueda, ProductoTactil, Categoria, ItemCarrito, NuevaVenta, VentaCompleta, Cliente, Caja, ResultadoEmision } from "../types";

export default function PuntoVenta() {
  const { toastExito, toastError, toastWarning } = useToast();
  const navigate = useNavigate();
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
  const [categoriasTactil, setCategoriasTactil] = useState<Categoria[]>([]);

  // Cliente
  const [clienteSeleccionado, setClienteSeleccionado] = useState<Cliente | null>(null);
  const [busquedaCliente, setBusquedaCliente] = useState("");
  const [clientesResultados, setClientesResultados] = useState<Cliente[]>([]);
  const [mostrarClientes, setMostrarClientes] = useState(false);
  const [mostrarCrearCliente, setMostrarCrearCliente] = useState(false);
  const [nuevoClienteNombre, setNuevoClienteNombre] = useState("");
  const [nuevoClienteId, setNuevoClienteId] = useState("");
  const [creandoCliente, setCreandoCliente] = useState(false);
  const [consultandoSri, setConsultandoSri] = useState(false);

  // Transferencia bancaria
  const [cuentasBanco, setCuentasBanco] = useState<{ id?: number; nombre: string; numero_cuenta?: string }[]>([]);
  const [bancoSeleccionado, setBancoSeleccionado] = useState<number | null>(null);
  const [referenciaPago, setReferenciaPago] = useState("");
  const [requiereReferencia, setRequiereReferencia] = useState(false);
  const [_requiereComprobante, setRequiereComprobante] = useState(false);

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
    listarCuentasBanco().then(setCuentasBanco).catch(() => {});
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
      // Cargar productos y categorias para grid
      listarProductosTactil().then(setProductosTactil).catch(() => {});
      listarCategorias().then(setCategoriasTactil).catch(() => {});
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
      // Si hay exactamente 1 resultado y el término parece código exacto (código de barras numérico o código de producto), agregar directamente
      if (res.length === 1 && (res[0].codigo === termino || /^\d{4,}$/.test(termino))) {
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

      const id = await crearCliente({
        tipo_identificacion: tipoId,
        identificacion: ident || undefined,
        nombre: nuevoClienteNombre.trim().toUpperCase(),
        activo: true,
      });
      const nuevoCliente: Cliente = {
        id,
        tipo_identificacion: tipoId,
        identificacion: ident || undefined,
        nombre: nuevoClienteNombre.trim().toUpperCase(),
        activo: true,
      };
      setClienteSeleccionado(nuevoCliente);
      setMostrarClientes(false);
      setMostrarCrearCliente(false);
      setNuevoClienteNombre("");
      setNuevoClienteId("");
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

  const agregarAlCarrito = async (producto: ProductoBusqueda) => {
    const precioEfectivo = producto.precio_lista ?? producto.precio_venta;

    // Check if already in cart
    const existente = carrito.find((i) => i.producto_id === producto.id);
    if (existente) {
      setCarrito((prev) =>
        prev.map((i) =>
          i.producto_id === producto.id
            ? { ...i, cantidad: i.cantidad + 1, subtotal: (i.cantidad + 1) * i.precio_unitario - i.descuento }
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

      setCarrito((prev) => [
        ...prev,
        {
          producto_id: producto.id,
          codigo: producto.codigo ?? undefined,
          nombre: producto.nombre,
          cantidad: 1,
          precio_unitario: precioEfectivo,
          descuento: 0,
          iva_porcentaje: producto.iva_porcentaje,
          subtotal: precioEfectivo,
          stock_disponible: producto.stock_actual,
          stock_minimo: producto.stock_minimo,
          precio_base: producto.precio_venta,
          precios_disponibles: preciosDisponibles,
          lista_seleccionada: listaSel,
        },
      ]);
    }
    setBusqueda("");
    setResultados([]);
    inputRef.current?.focus();
  };

  const actualizarCantidad = (productoId: number, cantidad: number) => {
    if (cantidad <= 0) {
      setCarrito((prev) => prev.filter((i) => i.producto_id !== productoId));
      return;
    }
    setCarrito((prev) =>
      prev.map((i) =>
        i.producto_id === productoId
          ? { ...i, cantidad, subtotal: cantidad * i.precio_unitario - i.descuento }
          : i
      )
    );
  };

  const eliminarItem = (productoId: number) => {
    setCarrito((prev) => prev.filter((i) => i.producto_id !== productoId));
  };

  const subtotal = carrito.reduce((sum, i) => sum + i.subtotal, 0);
  const iva = carrito.reduce((sum, i) => sum + i.subtotal * (i.iva_porcentaje / 100), 0);
  const total = subtotal + iva;
  const cambio = parseFloat(montoRecibido || "0") - total;

  const procesarVenta = useCallback(async () => {
    if (carrito.length === 0) return;
    if (!cajaAbierta) {
      toastError("Debe abrir la caja antes de realizar ventas");
      return;
    }
    // Validar referencia obligatoria en transferencia
    if (formaPago === "TRANSFER" && requiereReferencia && !referenciaPago.trim()) {
      toastError("El numero de referencia es obligatorio para transferencias");
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

    const nuevaVenta: NuevaVenta = {
      cliente_id: clienteSeleccionado?.id ?? 1,
      items: carrito.map((i) => ({
        producto_id: i.producto_id,
        cantidad: i.cantidad,
        precio_unitario: i.precio_unitario,
        descuento: i.descuento,
        iva_porcentaje: i.iva_porcentaje,
        subtotal: i.subtotal,
        info_adicional: i.info_adicional || null,
      })),
      forma_pago: formaPago,
      monto_recibido: esFiado ? 0 : parseFloat(montoRecibido || "0"),
      descuento: 0,
      tipo_documento: tipoDocumento,
      es_fiado: esFiado,
      banco_id: formaPago === "TRANSFER" ? bancoSeleccionado : null,
      referencia_pago: formaPago === "TRANSFER" ? (referenciaPago.trim() || null) : null,
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
      // Si fue FACTURA, modulo SRI activo y emision automatica activada, emitir al SRI
      if (tipoDocumento === "FACTURA" && sriModuloActivo && sriEmisionAutomatica && resultado.venta.id) {
        setEmitiendo(true);
        try {
          const res = await emitirFacturaSri(resultado.venta.id);
          setResultadoSri(res);
          if (res.exito) {
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
  }, [carrito, cajaAbierta, clienteSeleccionado, formaPago, montoRecibido, esFiado, tipoDocumento, sriModuloActivo, sriEmisionAutomatica, regimen, toastError, toastExito, toastWarning]);

  const nuevaVentaClick = useCallback(() => {
    setVentaCompletada(null);
    setResultadoSri(null);
    setMostrarModalEmail(false);
    setCarrito([]);
    setMontoRecibido("");
    setFormaPago("EFECTIVO");
    setEsFiado(false);
    setClienteSeleccionado(null);
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
    const nueva: NuevaVenta = {
      cliente_id: clienteSeleccionado?.id ?? 1,
      items: carrito.map(i => ({ producto_id: i.producto_id, cantidad: i.cantidad, precio_unitario: i.precio_unitario, descuento: i.descuento, iva_porcentaje: i.iva_porcentaje, subtotal: i.subtotal, info_adicional: i.info_adicional || null } as any)),
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
    const nueva: NuevaVenta = {
      cliente_id: clienteSeleccionado?.id ?? 1,
      items: carrito.map((i) => ({
        producto_id: i.producto_id,
        cantidad: i.cantidad,
        precio_unitario: i.precio_unitario,
        descuento: i.descuento,
        iva_porcentaje: i.iva_porcentaje,
        subtotal: i.subtotal,
        info_adicional: i.info_adicional || null,
      } as any)),
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
      toastExito(`Guía ${res.venta.numero} creada - stock descontado`);
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
    window.addEventListener("pos-cobrar", handleCobrar);
    window.addEventListener("pos-nueva-venta", handleNuevaVenta);
    window.addEventListener("pos-guardar-borrador", handleBorrador);
    window.addEventListener("pos-guardar-cotizacion", handleCotizacion);
    window.addEventListener("pos-guardar-guia", handleGuia);
    return () => {
      window.removeEventListener("pos-cobrar", handleCobrar);
      window.removeEventListener("pos-nueva-venta", handleNuevaVenta);
      window.removeEventListener("pos-guardar-borrador", handleBorrador);
      window.removeEventListener("pos-guardar-cotizacion", handleCotizacion);
      window.removeEventListener("pos-guardar-guia", handleGuia);
    };
  }, [procesarVenta, nuevaVentaClick, guardarComoDocumento, handleGuiaRemision]);

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
      // Guardar email en el cliente si tiene id
      if (clienteSeleccionado?.id && clienteSeleccionado.id !== 1) {
        await actualizarCliente({ ...clienteSeleccionado, email: emailInput });
      }
      await enviarNotificacionSri(ventaCompletada.venta.id, emailInput);
      toastExito(`Email enviado a ${emailInput}`);
      setMostrarModalEmail(false);
    } catch (err) {
      toastError("Error enviando email: " + err);
    } finally {
      setEnviandoEmail(false);
    }
  };

  const handleDescargarXml = async (ventaId: number, ventaNumero: string) => {
    try {
      const xml = await obtenerXmlFirmado(ventaId);
      const destino = await save({
        defaultPath: `factura-${ventaNumero.replace(/[\/\\:]/g, "-")}.xml`,
        filters: [{ name: "XML", extensions: ["xml"] }],
      });
      if (destino) {
        await invoke("guardar_archivo_texto", { ruta: destino, contenido: xml });
        toastExito("XML guardado");
      }
    } catch (err) {
      toastError("Error descargando XML: " + err);
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
                  {ticketUsarPdf ? "Ver Ticket PDF" : "Imprimir Ticket"}
                </button>
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
                      Imprimir RIDE
                    </button>
                    <button className="btn btn-outline btn-lg" onClick={() => handleDescargarXml(ventaCompletada.venta.id!, ventaCompletada.venta.numero)}>
                      Descargar XML
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
    <>
      <div className="page-header" style={{ paddingRight: 340 }}>
        <h2>Punto de Venta</h2>
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
                    <input className="input mb-2" placeholder="Nombre completo" value={nuevoClienteNombre}
                      onChange={(e) => setNuevoClienteNombre(e.target.value)}
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
                            toastError(err?.toString() || "No se encontró información");
                          } finally {
                            setConsultandoSri(false);
                          }
                        }}
                      >
                        {consultandoSri ? "Consultando..." : "🔍 Consultar en SRI"}
                      </button>
                      <div style={{ fontSize: 11, color: "var(--color-text-secondary)", marginTop: 4 }}>
                        Buscar datos por cédula/RUC en el SRI
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

      <div style={{ display: "flex", flex: 1, overflow: "hidden", position: "relative" }}>
        {/* Panel izquierdo - Grid de productos (con margen para columna fixed) */}
        <div style={{ flex: 1, position: "relative", marginRight: 324 }}>
          <PosGridTactil
            categorias={categoriasTactil}
            productosTactil={productosTactil}
            carrito={carrito}
            onAgregarProducto={agregarAlCarrito}
            onActualizarCantidad={actualizarCantidad}
            onEliminarItem={eliminarItem}
            onEditarInfoAdicional={(productoId, info) => {
              setCarrito(prev => prev.map(i =>
                i.producto_id === productoId ? { ...i, info_adicional: info } : i
              ));
            }}
            busqueda={busqueda}
            onBusquedaChange={handleBuscar}
            resultados={resultados}
            inputRef={inputRef}
          />
        </div>
        {/* Código de modo normal eliminado - ahora siempre usa grid */}

        {/* Botón Recientes - solo zona superior */}
        <button
          onClick={() => setMostrarRecientes(true)}
          style={{
            position: "fixed", right: 300, top: "50%", transform: "translateY(-50%)", height: 120,
            writingMode: "vertical-rl", textOrientation: "mixed",
            padding: "0 5px", background: "rgba(96, 165, 250, 0.15)",
            border: "none", borderLeft: "2px solid var(--color-primary)",
            borderRadius: "8px 0 0 8px",
            cursor: "pointer", fontSize: 11, fontWeight: 700, letterSpacing: 1,
            color: "var(--color-primary)", zIndex: 5, width: 24,
          }}
        >
          RECIENTES
        </button>

        {/* Panel derecho - Totales y cobro - fixed hasta el fondo */}
        <div style={{
          position: "fixed", right: 0, top: 44, bottom: 0, width: 300,
          background: "var(--color-surface)",
          borderLeft: "2px solid var(--color-border-strong, var(--color-border))",
          display: "flex", flexDirection: "column", zIndex: 5,
        }}>
          <div style={{ flex: 1, padding: "12px 16px" }}>
            <div className="flex justify-between mb-2">
              <span className="text-secondary">Items:</span>
              <span>{carrito.reduce((s, i) => s + i.cantidad, 0)}</span>
            </div>
            <div className="flex justify-between mb-2">
              <span className="text-secondary">Subtotal:</span>
              <span>${subtotal.toFixed(2)}</span>
            </div>
            {iva > 0 && (
              <div className="flex justify-between mb-2">
                <span className="text-secondary">IVA:</span>
                <span>${iva.toFixed(2)}</span>
              </div>
            )}
            <div className="flex justify-between" style={{
              fontSize: 26, fontWeight: 700, padding: "12px 0",
              borderTop: "2px solid var(--color-border)",
              borderBottom: "2px solid var(--color-border)", margin: "8px 0",
            }}>
              <span>TOTAL:</span>
              <span>${total.toFixed(2)}</span>
            </div>

            {/* Tipo de documento - solo visible si no es RIMPE_POPULAR */}
            {regimen !== "RIMPE_POPULAR" && (
              <div className="mb-4">
                <label className="text-secondary" style={{ fontSize: 12, display: "block", marginBottom: 4 }}>Tipo de documento</label>
                <div className="flex gap-2">
                  {(["NOTA_VENTA", "FACTURA"] as const).map((tipo) => (
                    <button key={tipo}
                      className={`btn ${tipoDocumento === tipo ? "btn-primary" : "btn-outline"}`}
                      style={{ flex: 1, fontSize: 12, justifyContent: "center" }}
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

            {/* Forma de pago - como la referencia: todos coloridos, mismo tamaño */}
            <div className="mb-4">
              <label className="text-secondary" style={{ fontSize: 12, display: "block", marginBottom: 4 }}>Forma de pago</label>
              <div style={{ display: "flex", gap: 6 }}>
                <button
                  className="btn"
                  style={{
                    flex: 1, fontSize: 13, padding: "10px 0", justifyContent: "center", fontWeight: 700,
                    background: formaPago === "EFECTIVO" && !esFiado ? "#16a34a" : "#22c55e",
                    color: "white", border: "none", borderRadius: 8,
                    opacity: formaPago === "EFECTIVO" && !esFiado ? 1 : 0.6,
                  }}
                  onClick={() => { setFormaPago("EFECTIVO"); setEsFiado(false); }}>
                  Efectivo
                </button>
                <button
                  className="btn"
                  style={{
                    flex: 1, fontSize: 13, padding: "10px 0", justifyContent: "center", fontWeight: 700,
                    background: formaPago === "TRANSFER" && !esFiado ? "#1d4ed8" : "#3b82f6",
                    color: "white", border: "none", borderRadius: 8,
                    opacity: formaPago === "TRANSFER" && !esFiado ? 1 : 0.6,
                  }}
                  onClick={() => { setFormaPago("TRANSFER"); setEsFiado(false); }}>
                  Transferencia
                </button>
                <button
                  className="btn"
                  style={{
                    flex: 1, fontSize: 13, padding: "10px 0", justifyContent: "center", fontWeight: 700,
                    background: esFiado ? "#b45309" : "#f59e0b",
                    color: "white", border: "none", borderRadius: 8,
                    opacity: esFiado ? 1 : 0.6,
                  }}
                  onClick={() => setEsFiado(!esFiado)}>
                  Credito
                </button>
              </div>
            </div>

            {/* Transferencia: cuenta bancaria + referencia */}
            {!esFiado && formaPago === "TRANSFER" && (
              <div className="mb-4" style={{ display: "flex", flexDirection: "column", gap: 8 }}>
                {cuentasBanco.length > 0 && (
                  <div>
                    <label className="text-secondary" style={{ fontSize: 12, display: "block", marginBottom: 4 }}>Cuenta destino</label>
                    <select
                      className="input"
                      value={bancoSeleccionado ?? ""}
                      onChange={(e) => setBancoSeleccionado(e.target.value ? Number(e.target.value) : null)}
                    >
                      <option value="">Seleccionar cuenta...</option>
                      {cuentasBanco.map((cb) => (
                        <option key={cb.id} value={cb.id}>
                          {cb.nombre}{cb.numero_cuenta ? ` — ${cb.numero_cuenta}` : ""}
                        </option>
                      ))}
                    </select>
                  </div>
                )}
                <div>
                  <label className="text-secondary" style={{ fontSize: 12, display: "block", marginBottom: 4 }}>
                    Nro. referencia {requiereReferencia && <span style={{ color: "var(--color-danger)" }}>*</span>}
                  </label>
                  <input className="input" placeholder="Ej: 123456789"
                    value={referenciaPago}
                    onChange={(e) => setReferenciaPago(e.target.value)} />
                </div>
              </div>
            )}

            {/* Monto recibido - solo si no es fiado */}
            {!esFiado && formaPago === "EFECTIVO" && (
              <div className="mb-4">
                <label className="text-secondary" style={{ fontSize: 12, display: "block", marginBottom: 4 }}>Monto recibido</label>
                <input className="input input-lg text-right" type="number" step="0.01" placeholder="0.00"
                  value={montoRecibido}
                  onChange={(e) => setMontoRecibido(e.target.value)}
                  onKeyDown={(e) => { if (e.key === "Enter") procesarVenta(); }} />
              </div>
            )}

            {!esFiado && formaPago === "EFECTIVO" && cambio >= 0 && montoRecibido && (
              <div className="flex justify-between text-lg">
                <span>Cambio:</span>
                <span className="font-bold text-success">${cambio.toFixed(2)}</span>
              </div>
            )}

            {esFiado && (
              <div style={{ background: "rgba(245, 158, 11, 0.15)", padding: 10, borderRadius: "var(--radius)", fontSize: 13, color: "var(--color-warning)" }}>
                Se registrara como cuenta por cobrar
                {clienteSeleccionado ? ` a ${clienteSeleccionado.nombre}` : ". Seleccione un cliente arriba."}
              </div>
            )}
          </div>

          <button className="btn btn-success btn-lg" data-action="cobrar"
            style={{ width: "100%", justifyContent: "center", fontSize: 16, marginBottom: 6, borderRadius: 10 }}
            disabled={carrito.length === 0 || (esFiado && !clienteSeleccionado)}
            onClick={procesarVenta}>
            {esFiado ? `Credito $${total.toFixed(2)}` : `Cobrar $${total.toFixed(2)}`} (F9)
          </button>
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
                Las facturas se enviarán al ambiente:
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
              <h3>Guía de Remisión</h3>
            </div>
            <div className="modal-body" style={{ display: "flex", flexDirection: "column", gap: 12 }}>
              <p className="text-secondary" style={{ fontSize: 12, margin: 0 }}>
                Se descontará stock al crear la guía. Todos los campos son opcionales.
              </p>
              <div>
                <label className="text-secondary" style={{ fontSize: 12, display: "block", marginBottom: 4 }}>Placa del vehículo</label>
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
                <label className="text-secondary" style={{ fontSize: 12, display: "block", marginBottom: 4 }}>Dirección de destino</label>
                <input className="input" placeholder="Dirección de entrega" value={guiaDireccion}
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
                {guardandoGuia ? "Guardando..." : "Crear Guía de Remisión"}
              </button>
            </div>
          </div>
        </div>
      )}
    </>
  );
}
