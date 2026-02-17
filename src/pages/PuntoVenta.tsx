import { useState, useRef, useEffect, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { buscarProductos, productosMasVendidos, registrarVenta, buscarClientes, crearCliente, imprimirTicket, imprimirTicketPdf, obtenerCajaAbierta, alertasStockBajo, obtenerConfig, guardarConfig, emitirFacturaSri, consultarEstadoSri, cambiarAmbienteSri, enviarNotificacionSri, actualizarCliente, imprimirRide, obtenerXmlFirmado, procesarEmailsPendientes, resolverPrecioProducto, obtenerPreciosProducto } from "../services/api";
import type { AlertaStock } from "../services/api";
import { save } from "@tauri-apps/plugin-dialog";
import { useToast } from "../components/Toast";
import { useNavigate } from "react-router-dom";
import ModalEmailCliente from "../components/ModalEmailCliente";
import type { ProductoBusqueda, ItemCarrito, NuevaVenta, VentaCompleta, Cliente, Caja, ResultadoEmision } from "../types";

export default function PuntoVenta() {
  const { toastExito, toastError, toastWarning } = useToast();
  const navigate = useNavigate();
  const [busqueda, setBusqueda] = useState("");
  const [resultados, setResultados] = useState<ProductoBusqueda[]>([]);
  const [favoritos, setFavoritos] = useState<ProductoBusqueda[]>([]);
  const [carrito, setCarrito] = useState<ItemCarrito[]>([]);
  const [montoRecibido, setMontoRecibido] = useState("");
  const [ventaCompletada, setVentaCompletada] = useState<VentaCompleta | null>(null);
  const [formaPago, setFormaPago] = useState("EFECTIVO");
  const [esFiado, setEsFiado] = useState(false);
  const [cajaAbierta, setCajaAbierta] = useState<Caja | null>(null);
  const [verificandoCaja, setVerificandoCaja] = useState(true);
  const [alertas, setAlertas] = useState<AlertaStock[]>([]);
  const [mostrarAlertas, setMostrarAlertas] = useState(false);
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

  // Cliente
  const [clienteSeleccionado, setClienteSeleccionado] = useState<Cliente | null>(null);
  const [busquedaCliente, setBusquedaCliente] = useState("");
  const [clientesResultados, setClientesResultados] = useState<Cliente[]>([]);
  const [mostrarClientes, setMostrarClientes] = useState(false);
  const [mostrarCrearCliente, setMostrarCrearCliente] = useState(false);
  const [nuevoClienteNombre, setNuevoClienteNombre] = useState("");
  const [nuevoClienteId, setNuevoClienteId] = useState("");
  const [creandoCliente, setCreandoCliente] = useState(false);

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
    // Cargar regimen y estado ambiente confirmado
    obtenerConfig().then((cfg) => {
      if (cfg.regimen) setRegimen(cfg.regimen);
      setSriAmbienteConfirmado(cfg.sri_ambiente_confirmado === "1");
      setSriEmisionAutomatica(cfg.sri_emision_automatica === "1");
      setTicketUsarPdf(cfg.ticket_usar_pdf === "1");
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
      setResultados(await buscarProductos(termino, clienteSeleccionado?.lista_precio_id));
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
      toastError("Error al crear cliente: " + err);
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

  const actualizarDescuento = (productoId: number, descuento: number) => {
    setCarrito((prev) =>
      prev.map((i) =>
        i.producto_id === productoId
          ? { ...i, descuento, subtotal: i.cantidad * i.precio_unitario - descuento }
          : i
      )
    );
  };

  const eliminarItem = (productoId: number) => {
    setCarrito((prev) => prev.filter((i) => i.producto_id !== productoId));
  };

  const cambiarListaPrecioItem = (productoId: number, nuevoPrecio: number, listaNombre?: string) => {
    setCarrito((prev) =>
      prev.map((i) =>
        i.producto_id === productoId
          ? {
              ...i,
              precio_unitario: nuevoPrecio,
              subtotal: i.cantidad * nuevoPrecio - i.descuento,
              lista_seleccionada: listaNombre,
            }
          : i
      )
    );
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
    // Validar: FACTURA requiere cliente identificado (no Consumidor Final id=1)
    if (tipoDocumento === "FACTURA" && (!clienteSeleccionado || clienteSeleccionado.id === 1)) {
      toastError("Para emitir FACTURA debe seleccionar un cliente con identificacion");
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
      })),
      forma_pago: formaPago,
      monto_recibido: esFiado ? 0 : parseFloat(montoRecibido || "0"),
      descuento: 0,
      tipo_documento: tipoDocumento,
      es_fiado: esFiado,
    };

    try {
      const resultado = await registrarVenta(nuevaVenta);
      setVentaCompletada(resultado);
      setCarrito([]);
      setMontoRecibido("");
      setFormaPago("EFECTIVO");
      setEsFiado(false);
      setClienteSeleccionado(null);
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
      setTipoDocumento("NOTA_VENTA");
      // Refrescar favoritos y alertas de stock
      productosMasVendidos(12).then(setFavoritos).catch(() => {});
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
  }, [carrito, cajaAbierta, clienteSeleccionado, formaPago, montoRecibido, esFiado, tipoDocumento, sriModuloActivo, sriEmisionAutomatica, toastError, toastExito, toastWarning]);

  const nuevaVentaClick = useCallback(() => {
    setVentaCompletada(null);
    setResultadoSri(null);
    setMostrarModalEmail(false);
    setCarrito([]);
    setMontoRecibido("");
    setFormaPago("EFECTIVO");
    setEsFiado(false);
    setClienteSeleccionado(null);
    setTipoDocumento("NOTA_VENTA");
    inputRef.current?.focus();
  }, []);

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
  useEffect(() => {
    const handleCobrar = () => procesarVenta();
    const handleNuevaVenta = () => nuevaVentaClick();
    window.addEventListener("pos-cobrar", handleCobrar);
    window.addEventListener("pos-nueva-venta", handleNuevaVenta);
    return () => {
      window.removeEventListener("pos-cobrar", handleCobrar);
      window.removeEventListener("pos-nueva-venta", handleNuevaVenta);
    };
  }, [procesarVenta, nuevaVentaClick]);

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
                <div style={{ marginTop: 12, color: "#2563eb", fontSize: 13 }}>
                  Enviando factura al SRI...
                </div>
              )}
              {esFacturaAutorizada && (
                <div style={{
                  marginTop: 12, padding: "8px 12px", borderRadius: "var(--radius)",
                  background: "#dcfce7", color: "#166534", fontSize: 13,
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
      <div className="page-header">
        <h2>Punto de Venta</h2>
        <div className="flex gap-2 items-center">
          {/* Selector de cliente */}
          <div style={{ position: "relative" }}>
            {clienteSeleccionado ? (
              <div className="flex gap-2 items-center">
                <span style={{ fontSize: 13, background: "#e0f2fe", padding: "4px 10px", borderRadius: 4 }}>
                  {clienteSeleccionado.nombre}
                  {clienteSeleccionado.lista_precio_nombre && (
                    <span style={{ fontSize: 10, marginLeft: 6, background: "#dbeafe", padding: "1px 6px", borderRadius: 3, color: "#1e40af" }}>
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
                background: "white", border: "1px solid var(--color-border)",
                borderRadius: "var(--radius)", boxShadow: "var(--shadow-md)", zIndex: 20, padding: 8,
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
                    background: "#f0fdf4", border: "1px solid #bbf7d0", borderRadius: "var(--radius)",
                    padding: 8, marginBottom: 8,
                  }}>
                    <div style={{ fontSize: 11, fontWeight: 600, marginBottom: 6, color: "#166534" }}>Nuevo cliente</div>
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
                    onMouseEnter={(e) => (e.currentTarget.style.background = "#f1f5f9")}
                    onMouseLeave={(e) => (e.currentTarget.style.background = "transparent")}
                  >
                    <strong>{c.nombre}</strong>
                    {c.identificacion && <span className="text-secondary" style={{ marginLeft: 6 }}>{c.identificacion}</span>}
                  </div>
                ))}
                {busquedaCliente.length >= 2 && clientesResultados.length === 0 && !mostrarCrearCliente && (
                  <div style={{ padding: "8px", textAlign: "center", fontSize: 12, color: "#94a3b8" }}>
                    No encontrado. Use <strong>+</strong> para crear.
                  </div>
                )}
              </div>
            )}
          </div>
        </div>
      </div>

      <div style={{ display: "flex", flex: 1, overflow: "hidden" }}>
        {/* Panel izquierdo */}
        <div style={{ flex: 1, padding: 16, display: "flex", flexDirection: "column", gap: 12 }}>
          {/* Barra de busqueda */}
          <div style={{ position: "relative" }}>
            <input
              ref={inputRef}
              className="input input-lg"
              data-action="busqueda"
              placeholder="Buscar producto por nombre o codigo... (Ctrl+B)"
              value={busqueda}
              onChange={(e) => handleBuscar(e.target.value)}
              onKeyDown={(e) => {
                if (e.key === "Enter" && resultados.length > 0) {
                  agregarAlCarrito(resultados[0]);
                }
              }}
            />
            {resultados.length > 0 && (
              <div style={{
                position: "absolute", top: "100%", left: 0, right: 0,
                background: "white", border: "1px solid var(--color-border)",
                borderRadius: "var(--radius)", boxShadow: "var(--shadow-md)",
                zIndex: 10, maxHeight: 300, overflowY: "auto",
              }}>
                {resultados.map((p) => (
                  <div key={p.id} style={{
                    padding: "10px 16px", cursor: "pointer", display: "flex",
                    justifyContent: "space-between", borderBottom: "1px solid var(--color-border)",
                  }}
                    onClick={() => agregarAlCarrito(p)}
                    onMouseEnter={(e) => (e.currentTarget.style.background = "#f1f5f9")}
                    onMouseLeave={(e) => (e.currentTarget.style.background = "transparent")}
                  >
                    <div>
                      <strong>{p.nombre}</strong>
                      {p.codigo && <span className="text-secondary" style={{ marginLeft: 8 }}>({p.codigo})</span>}
                    </div>
                    <div className="flex gap-4">
                      <span style={{
                        color: p.stock_actual <= 0 ? "#dc2626" : p.stock_actual <= p.stock_minimo ? "#d97706" : undefined,
                        fontWeight: p.stock_actual <= p.stock_minimo ? 600 : undefined,
                      }}>
                        Stock: {p.stock_actual}{p.stock_actual <= p.stock_minimo && p.stock_minimo > 0 ? ` (min: ${p.stock_minimo})` : ""}
                      </span>
                      {p.precio_lista != null && p.precio_lista !== p.precio_venta ? (
                        <span>
                          <strong style={{ color: "#1e40af" }}>${p.precio_lista.toFixed(2)}</strong>
                          <span style={{ fontSize: 11, color: "#94a3b8", marginLeft: 4, textDecoration: "line-through" }}>${p.precio_venta.toFixed(2)}</span>
                        </span>
                      ) : (
                        <strong>${p.precio_venta.toFixed(2)}</strong>
                      )}
                    </div>
                  </div>
                ))}
              </div>
            )}
          </div>

          {/* Favoritos - productos mas vendidos */}
          {carrito.length === 0 && favoritos.length > 0 && (
            <div>
              <span className="text-secondary" style={{ fontSize: 12, marginBottom: 6, display: "block" }}>
                Productos frecuentes
              </span>
              <div style={{ display: "flex", flexWrap: "wrap", gap: 8 }}>
                {favoritos.map((p) => (
                  <button key={p.id} className="btn btn-outline"
                    style={{ fontSize: 12, padding: "6px 12px" }}
                    onClick={() => agregarAlCarrito(p)}>
                    {p.nombre} - ${p.precio_venta.toFixed(2)}
                    <span style={{
                      fontSize: 10, marginLeft: 4,
                      color: p.stock_actual <= 0 ? "#dc2626" : p.stock_actual <= p.stock_minimo ? "#d97706" : "#64748b",
                    }}>
                      ({p.stock_actual})
                    </span>
                  </button>
                ))}
              </div>
            </div>
          )}

          {/* Tabla del carrito */}
          <div className="card" style={{ flex: 1, overflow: "auto" }}>
            <table className="table">
              <thead>
                <tr>
                  <th>Producto</th>
                  <th style={{ width: 80 }}>Cant.</th>
                  <th style={{ width: 90 }} className="text-right">P. Unit.</th>
                  <th style={{ width: 80 }} className="text-right">Desc.</th>
                  <th style={{ width: 100 }} className="text-right">Subtotal</th>
                  <th style={{ width: 40 }}></th>
                </tr>
              </thead>
              <tbody>
                {carrito.length === 0 ? (
                  <tr>
                    <td colSpan={6} className="text-center text-secondary" style={{ padding: 40 }}>
                      Busca un producto para comenzar la venta
                    </td>
                  </tr>
                ) : (
                  carrito.map((item) => (
                    <tr key={item.producto_id}>
                      <td>
                        <strong>{item.nombre}</strong>
                        {item.codigo && <span className="text-secondary" style={{ marginLeft: 8, fontSize: 12 }}>{item.codigo}</span>}
                        <span style={{
                          fontSize: 11, marginLeft: 6,
                          color: item.stock_disponible <= 0 ? "#dc2626" : item.stock_disponible <= item.stock_minimo ? "#d97706" : "#94a3b8",
                          fontWeight: item.stock_disponible <= item.stock_minimo ? 600 : undefined,
                        }}>
                          (stock: {item.stock_disponible})
                        </span>
                      </td>
                      <td>
                        <input type="number" className="input" value={item.cantidad} min={1} step={1}
                          style={{ width: 60, textAlign: "center", padding: "4px" }}
                          onChange={(e) => actualizarCantidad(item.producto_id, parseFloat(e.target.value) || 0)} />
                      </td>
                      <td className="text-right">
                        {item.precios_disponibles && item.precios_disponibles.length > 0 ? (
                          <div style={{ position: "relative" }}>
                            <select
                              className="input"
                              style={{ width: 90, fontSize: 11, padding: "3px 2px", textAlign: "right", appearance: "auto" }}
                              value={item.lista_seleccionada ?? "base"}
                              onChange={(e) => {
                                const key = e.target.value;
                                if (key === "base") {
                                  cambiarListaPrecioItem(item.producto_id, item.precio_base, undefined);
                                } else {
                                  const match = item.precios_disponibles?.find(p => p.lista_nombre === key);
                                  if (match) {
                                    cambiarListaPrecioItem(item.producto_id, match.precio, match.lista_nombre);
                                  }
                                }
                              }}
                            >
                              <option value="base">
                                ${item.precio_base.toFixed(2)} (Base)
                              </option>
                              {item.precios_disponibles.map((pp) => (
                                <option key={pp.lista_precio_id} value={pp.lista_nombre}>
                                  ${pp.precio.toFixed(2)} ({pp.lista_nombre})
                                </option>
                              ))}
                            </select>
                            {item.lista_seleccionada && (
                              <div style={{ fontSize: 9, color: "#1e40af", textAlign: "right" }}>{item.lista_seleccionada}</div>
                            )}
                          </div>
                        ) : (
                          <span>${item.precio_unitario.toFixed(2)}</span>
                        )}
                      </td>
                      <td className="text-right">
                        <input type="number" className="input" value={item.descuento} min={0} step={0.1}
                          style={{ width: 65, textAlign: "center", padding: "4px" }}
                          onChange={(e) => actualizarDescuento(item.producto_id, parseFloat(e.target.value) || 0)} />
                      </td>
                      <td className="text-right font-bold">${item.subtotal.toFixed(2)}</td>
                      <td>
                        <button className="btn btn-danger" style={{ padding: "2px 6px", fontSize: 11 }}
                          onClick={() => eliminarItem(item.producto_id)}>x</button>
                      </td>
                    </tr>
                  ))
                )}
              </tbody>
            </table>
          </div>

          {/* Panel de alertas de stock bajo */}
          {alertas.length > 0 && (
            <div style={{
              border: "1px solid #fbbf24",
              borderRadius: "var(--radius)",
              background: "#fffbeb",
              overflow: "hidden",
            }}>
              <button
                onClick={() => setMostrarAlertas(!mostrarAlertas)}
                style={{
                  width: "100%", display: "flex", justifyContent: "space-between", alignItems: "center",
                  padding: "6px 12px", background: "transparent", border: "none", cursor: "pointer",
                  color: "#92400e", fontSize: 12, fontWeight: 600,
                }}
              >
                <span>! Stock Bajo ({alertas.length} producto{alertas.length > 1 ? "s" : ""})</span>
                <span>{mostrarAlertas ? "\u25B2" : "\u25BC"}</span>
              </button>
              {mostrarAlertas && (
                <div style={{ maxHeight: 150, overflowY: "auto" }}>
                  {alertas.map((a) => (
                    <div key={a.id} className="flex justify-between"
                      style={{ padding: "4px 12px", borderTop: "1px solid #fde68a", fontSize: 12 }}>
                      <span style={{ color: "#78350f" }}>{a.nombre}</span>
                      <span style={{
                        fontWeight: 600,
                        color: a.stock_actual <= 0 ? "#dc2626" : "#d97706",
                      }}>
                        {a.stock_actual} / {a.stock_minimo}
                      </span>
                    </div>
                  ))}
                </div>
              )}
            </div>
          )}
        </div>

        {/* Panel derecho - Totales y cobro */}
        <div style={{
          width: 300, background: "var(--color-surface)",
          borderLeft: "1px solid var(--color-border)", padding: 16,
          display: "flex", flexDirection: "column",
        }}>
          <div style={{ flex: 1 }}>
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
                {tipoDocumento === "FACTURA" && (!clienteSeleccionado || clienteSeleccionado.id === 1) && (
                  <div style={{ fontSize: 11, color: "#d97706", marginTop: 4 }}>
                    Factura requiere cliente con identificacion
                  </div>
                )}
                {tipoDocumento === "FACTURA" && sriModuloActivo && sriAmbiente && (
                  <div style={{
                    fontSize: 11, marginTop: 4, display: "flex", alignItems: "center", gap: 6,
                    color: sriAmbiente === "produccion" ? "#dc2626" : "#2563eb",
                  }}>
                    <span style={{
                      width: 8, height: 8, borderRadius: "50%",
                      background: sriAmbiente === "produccion" ? "#dc2626" : "#3b82f6",
                      display: "inline-block",
                    }} />
                    Ambiente: {sriAmbiente.toUpperCase()}
                    {!sriAmbienteConfirmado && <span style={{ color: "#d97706" }}>(sin confirmar)</span>}
                  </div>
                )}
              </div>
            )}

            {/* Forma de pago */}
            <div className="mb-4">
              <label className="text-secondary" style={{ fontSize: 12, display: "block", marginBottom: 4 }}>Forma de pago</label>
              <div className="flex gap-2">
                {["EFECTIVO", "TRANSFER"].map((fp) => (
                  <button key={fp}
                    className={`btn ${formaPago === fp ? "btn-primary" : "btn-outline"}`}
                    style={{ flex: 1, fontSize: 12, justifyContent: "center" }}
                    onClick={() => { setFormaPago(fp); setEsFiado(false); }}>
                    {fp === "EFECTIVO" ? "Efectivo" : "Transfer."}
                  </button>
                ))}
                <button
                  className={`btn ${esFiado ? "btn-primary" : "btn-outline"}`}
                  style={{ flex: 1, fontSize: 12, justifyContent: "center" }}
                  onClick={() => setEsFiado(!esFiado)}>
                  Fiado
                </button>
              </div>
            </div>

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
              <div style={{ background: "#fef3c7", padding: 10, borderRadius: "var(--radius)", fontSize: 13, color: "#92400e" }}>
                Se registrara como cuenta por cobrar
                {clienteSeleccionado ? ` a ${clienteSeleccionado.nombre}` : ". Seleccione un cliente arriba."}
              </div>
            )}
          </div>

          <button className="btn btn-success btn-lg" data-action="cobrar"
            style={{ width: "100%", justifyContent: "center", fontSize: 16 }}
            disabled={carrito.length === 0 || (esFiado && !clienteSeleccionado) || (tipoDocumento === "FACTURA" && (!clienteSeleccionado || clienteSeleccionado.id === 1))}
            onClick={procesarVenta}>
            {esFiado ? `Fiar $${total.toFixed(2)}` : `Cobrar $${total.toFixed(2)}`} (F9)
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
                background: sriAmbiente === "produccion" ? "#fef2f2" : "#eff6ff",
                border: `2px solid ${sriAmbiente === "produccion" ? "#ef4444" : "#3b82f6"}`,
                textAlign: "center",
                marginBottom: 12,
              }}>
                <span style={{
                  fontSize: 18,
                  fontWeight: 700,
                  color: sriAmbiente === "produccion" ? "#dc2626" : "#2563eb",
                }}>
                  {sriAmbiente === "produccion" ? "PRODUCCION" : "PRUEBAS"}
                </span>
                <div style={{ fontSize: 12, marginTop: 4, color: "#64748b" }}>
                  {sriAmbiente === "produccion"
                    ? "Las facturas tendran validez tributaria real"
                    : "Las facturas NO tendran validez tributaria"}
                </div>
              </div>
              <div style={{ display: "flex", gap: 8, alignItems: "center", marginBottom: 8 }}>
                <span style={{ fontSize: 13, color: "#64748b" }}>Cambiar a:</span>
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
    </>
  );
}
