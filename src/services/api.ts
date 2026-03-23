import { invoke } from "@tauri-apps/api/core";

// --- Modo Red: Proxy remoto ---
// Cuando modo_red === 'cliente', las llamadas se redirigen via HTTP al servidor.

let _modoRed: 'local' | 'servidor' | 'cliente' = 'local';
let _servidorUrl = '';
let _servidorToken = '';

/** Configura el modo de red para las llamadas API */
export function configurarModoRed(modo: 'local' | 'servidor' | 'cliente', url?: string, token?: string) {
  _modoRed = modo;
  _servidorUrl = url || '';
  _servidorToken = token || '';
}

/** Invoke inteligente: usa Tauri invoke en modo local/servidor, HTTP en modo cliente.
 *  En modo cliente, si falla la red y el comando es encolable, lo guarda offline. */
async function smartInvoke<T>(command: string, args?: Record<string, unknown>): Promise<T> {
  if (_modoRed === 'cliente' && _servidorUrl) {
    try {
      const response = await fetch(`${_servidorUrl}/api/v1/invoke`, {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
          'Authorization': `Bearer ${_servidorToken}`,
        },
        body: JSON.stringify({ command, args: args || {} }),
        signal: AbortSignal.timeout(10000),
      });

      const data = await response.json();
      if (!data.ok) {
        throw new Error(data.error || 'Error en servidor remoto');
      }
      return data.data as T;
    } catch (err) {
      // Error de red: intentar encolar si es un comando de escritura
      const { esComandoEncolable, encolarOperacion, setOnline } = await import('./offlineSync');
      setOnline(false);

      if (esComandoEncolable(command)) {
        await encolarOperacion(command, args || {});
        // Retornar un resultado "placeholder" para que la UI no crashee
        return { offline: true, encolado: true } as unknown as T;
      }

      // Para lecturas en modo offline, intentar cache local
      if (command === 'buscar_productos') {
        const results = await invoke<T>('buscar_productos_offline', { termino: (args as Record<string, unknown>)?.termino || '' });
        return results;
      }

      throw new Error('Sin conexion al servidor. ' + (err instanceof Error ? err.message : ''));
    }
  }

  return invoke<T>(command, args);
}

import type {
  Producto,
  ProductoBusqueda,
  ProductoTactil,
  Categoria,
  Cliente,
  NuevaVenta,
  VentaCompleta,
  Venta,
  Caja,
  ResumenCaja,
  Gasto,
  CuentaConCliente,
  ResumenCliente,
  CuentaDetalle,
  CuentaPorCobrar,
  PagoCuenta,
  CuentaBanco,
  LicenciaInfo,
  UsuarioInfo,
  SesionActiva,
  NuevoUsuario,
  ResultadoEmision,
  EstadoSri,
  PlanesDisponibles,
  PedidoCreado,
  NuevaNotaCredito,
  NotaCreditoInfo,
  ListaPrecio,
  PrecioProducto,
  PrecioProductoDetalle,
  Establecimiento,
  PuntoEmision,
  TransferenciaStock,
  StockEstablecimiento,
  DocumentoReciente,
  ResumenGuias,
} from "../types";

// --- Productos ---

export async function crearProducto(producto: Producto): Promise<number> {
  return smartInvoke("crear_producto", { producto });
}

export async function actualizarProducto(producto: Producto): Promise<void> {
  return smartInvoke("actualizar_producto", { producto });
}

export async function buscarProductos(termino: string, listaPrecioId?: number): Promise<ProductoBusqueda[]> {
  return smartInvoke("buscar_productos", { termino, listaPrecioId: listaPrecioId ?? null });
}

export async function obtenerProducto(id: number): Promise<Producto> {
  return smartInvoke("obtener_producto", { id });
}

export async function listarProductos(soloActivos: boolean = true, listaPrecioId?: number): Promise<ProductoBusqueda[]> {
  return smartInvoke("listar_productos", { soloActivos, listaPrecioId: listaPrecioId ?? null });
}

export async function productosMasVendidos(limite: number = 12): Promise<ProductoBusqueda[]> {
  return smartInvoke("productos_mas_vendidos", { limite });
}

export async function cargarImagenProducto(id: number, imagenPath: string): Promise<string> {
  return smartInvoke("cargar_imagen_producto", { id, imagenPath });
}

export async function eliminarImagenProducto(id: number): Promise<void> {
  return smartInvoke("eliminar_imagen_producto", { id });
}

export async function listarProductosTactil(): Promise<ProductoTactil[]> {
  return smartInvoke("listar_productos_tactil");
}

// --- Categorías ---

export async function crearCategoria(categoria: Categoria): Promise<number> {
  return smartInvoke("crear_categoria", { categoria });
}

export async function listarCategorias(): Promise<Categoria[]> {
  return smartInvoke("listar_categorias");
}

// --- Clientes ---

export async function crearCliente(cliente: Cliente): Promise<number> {
  return smartInvoke("crear_cliente", { cliente });
}

export async function actualizarCliente(cliente: Cliente): Promise<void> {
  return smartInvoke("actualizar_cliente", { cliente });
}

export async function buscarClientes(termino: string): Promise<Cliente[]> {
  return smartInvoke("buscar_clientes", { termino });
}

export async function listarClientes(): Promise<Cliente[]> {
  return smartInvoke("listar_clientes");
}

export async function consultarIdentificacion(identificacion: string): Promise<Cliente> {
  return smartInvoke("consultar_identificacion", { identificacion });
}

// --- Ventas ---

export async function registrarVenta(venta: NuevaVenta): Promise<VentaCompleta> {
  return smartInvoke("registrar_venta", { venta });
}

export async function guardarBorrador(venta: NuevaVenta): Promise<VentaCompleta> {
  return smartInvoke("guardar_borrador", { venta });
}

export async function guardarCotizacion(venta: NuevaVenta): Promise<VentaCompleta> {
  return smartInvoke("guardar_cotizacion", { venta });
}

export async function eliminarBorrador(id: number): Promise<void> {
  return smartInvoke("eliminar_borrador", { id });
}

export async function listarDocumentosRecientes(limite?: number): Promise<DocumentoReciente[]> {
  return smartInvoke("listar_documentos_recientes", { limite });
}

export async function guardarGuiaRemision(venta: NuevaVenta): Promise<VentaCompleta> {
  return smartInvoke("guardar_guia_remision", { venta });
}

export async function listarGuiasRemision(filtros: {
  fechaDesde?: string; fechaHasta?: string;
  clienteId?: number; estado?: string;
}): Promise<any[]> {
  return smartInvoke("listar_guias_remision", filtros);
}

export async function resumenGuiasRemision(fechaDesde: string, fechaHasta: string): Promise<ResumenGuias> {
  return smartInvoke("resumen_guias_remision", { fechaDesde, fechaHasta });
}

export async function convertirGuiaAVenta(params: {
  guiaId: number; formaPago: string; montoRecibido: number;
  esFiado?: boolean; bancoId?: number; referenciaPago?: string;
}): Promise<VentaCompleta> {
  return smartInvoke("convertir_guia_a_venta", params);
}

export async function listarChoferes(): Promise<[number, string, string | null][]> {
  return smartInvoke("listar_choferes", {});
}

export async function guardarChofer(nombre: string, placa?: string): Promise<void> {
  return smartInvoke("guardar_chofer", { nombre, placa: placa || null });
}

export async function listarVentasDia(fecha: string): Promise<Venta[]> {
  return smartInvoke("listar_ventas_dia", { fecha });
}

export async function obtenerVenta(id: number): Promise<VentaCompleta> {
  return smartInvoke("obtener_venta", { id });
}

// --- Reportes ---

export interface ResumenDiario {
  total_ventas: number;
  num_ventas: number;
  total_efectivo: number;
  total_transferencia: number;
  total_fiado: number;
  utilidad_bruta: number;
  total_notas_credito: number;
  num_notas_credito: number;
}

export interface ProductoMasVendido {
  nombre: string;
  cantidad_total: number;
  total_vendido: number;
}

export interface AlertaStock {
  id: number;
  codigo?: string;
  nombre: string;
  stock_actual: number;
  stock_minimo: number;
}

export async function resumenDiario(fecha: string): Promise<ResumenDiario> {
  return smartInvoke("resumen_diario", { fecha });
}

export async function productosMasVendidosReporte(fechaInicio: string, fechaFin: string, limite: number = 10): Promise<ProductoMasVendido[]> {
  return smartInvoke("productos_mas_vendidos_reporte", { fechaInicio, fechaFin, limite });
}

export async function alertasStockBajo(): Promise<AlertaStock[]> {
  return smartInvoke("alertas_stock_bajo");
}

export async function resumenFiadosPendientes(): Promise<number> {
  return smartInvoke("resumen_fiados_pendientes");
}

export interface ResumenPeriodo {
  total_ventas: number;
  num_ventas: number;
  total_efectivo: number;
  total_transferencia: number;
  total_fiado: number;
  utilidad_bruta: number;
  total_gastos: number;
  promedio_por_venta: number;
  total_notas_credito: number;
  num_notas_credito: number;
}

export async function resumenPeriodo(fechaInicio: string, fechaFin: string): Promise<ResumenPeriodo> {
  return smartInvoke("resumen_periodo", { fechaInicio, fechaFin });
}

export async function listarVentasPeriodo(fechaInicio: string, fechaFin: string): Promise<Venta[]> {
  return smartInvoke("listar_ventas_periodo", { fechaInicio, fechaFin });
}

export interface VentaDiaria {
  fecha: string;
  total: number;
  num_ventas: number;
}

export async function ventasPorDia(fechaInicio: string, fechaFin: string): Promise<VentaDiaria[]> {
  return smartInvoke("ventas_por_dia", { fechaInicio, fechaFin });
}

export async function resumenDiarioAyer(): Promise<ResumenDiario> {
  return smartInvoke("resumen_diario_ayer");
}

export interface UltimaVenta {
  id: number;
  numero: string;
  hora: string;
  cliente_nombre: string;
  total: number;
  forma_pago: string;
}

export async function ultimasVentasDia(limite: number = 5): Promise<UltimaVenta[]> {
  return smartInvoke("ultimas_ventas_dia", { limite });
}

// --- Caja ---

export async function abrirCaja(montoInicial: number): Promise<Caja> {
  return smartInvoke("abrir_caja", { montoInicial });
}

export async function cerrarCaja(montoReal: number, observacion?: string): Promise<ResumenCaja> {
  return smartInvoke("cerrar_caja", { montoReal, observacion });
}

export async function obtenerCajaAbierta(): Promise<Caja | null> {
  return smartInvoke("obtener_caja_abierta");
}

// --- Impresión ---

export async function imprimirTicket(ventaId: number): Promise<string> {
  return invoke("imprimir_ticket", { ventaId });
}

export async function imprimirTicketPdf(ventaId: number): Promise<string> {
  return invoke("imprimir_ticket_pdf", { ventaId });
}

export async function imprimirReporteCaja(cajaId: number): Promise<string> {
  return invoke("imprimir_reporte_caja", { cajaId });
}

export async function imprimirReporteCajaPdf(cajaId: number): Promise<string> {
  return invoke("imprimir_reporte_caja_pdf", { cajaId });
}

export async function imprimirGuiaRemisionPdf(ventaId: number): Promise<string> {
  return invoke("imprimir_guia_remision_pdf", { ventaId });
}

export async function listarImpresoras(): Promise<string[]> {
  return invoke("listar_impresoras");
}

export async function listarImpresorasCached(): Promise<string[]> {
  return invoke("listar_impresoras_cached");
}

export async function refrescarImpresoras(): Promise<string[]> {
  return invoke("refrescar_impresoras");
}

// --- Gastos ---

export async function crearGasto(gasto: Gasto): Promise<Gasto> {
  return smartInvoke("crear_gasto", { gasto });
}

export async function listarGastosDia(fecha: string): Promise<Gasto[]> {
  return smartInvoke("listar_gastos_dia", { fecha });
}

export async function eliminarGasto(id: number): Promise<void> {
  return smartInvoke("eliminar_gasto", { id });
}

// --- Cuentas por Cobrar ---

export async function resumenDeudores(): Promise<ResumenCliente[]> {
  return smartInvoke("resumen_deudores");
}

export async function listarCuentasPendientes(clienteId?: number): Promise<CuentaConCliente[]> {
  return smartInvoke("listar_cuentas_pendientes", { clienteId: clienteId ?? null });
}

export async function obtenerCuentaDetalle(id: number): Promise<CuentaDetalle> {
  return smartInvoke("obtener_cuenta_detalle", { id });
}

export async function registrarPagoCuenta(pago: PagoCuenta): Promise<CuentaPorCobrar> {
  return smartInvoke("registrar_pago_cuenta", { pago });
}

// --- Cuentas Banco ---

export async function listarCuentasBanco(): Promise<CuentaBanco[]> {
  return smartInvoke("listar_cuentas_banco");
}

export async function crearCuentaBanco(cuenta: CuentaBanco): Promise<CuentaBanco> {
  return smartInvoke("crear_cuenta_banco", { cuenta });
}

export async function actualizarCuentaBanco(id: number, cuenta: CuentaBanco): Promise<void> {
  return smartInvoke("actualizar_cuenta_banco", { id, cuenta });
}

export async function desactivarCuentaBanco(id: number): Promise<void> {
  return smartInvoke("desactivar_cuenta_banco", { id });
}

// --- Etiquetas de productos ---

export interface EtiquetaConfig {
  producto_ids: number[];
  cantidad_por_producto: number;
  columnas: number;
  mostrar_precio: boolean;
  mostrar_codigo: boolean;
  lista_precio_id?: number;
  preset?: string;
  ancho_mm?: number;
  alto_mm?: number;
  margen_top_mm?: number;
  margen_left_mm?: number;
}

export async function generarEtiquetasPdf(config: EtiquetaConfig): Promise<string> {
  return invoke("generar_etiquetas_pdf", { config });
}

// --- Confirmación de pagos ---

export async function confirmarPagoCuenta(pagoId: number): Promise<CuentaDetalle> {
  return smartInvoke("confirmar_pago_cuenta", { pagoId });
}

export async function rechazarPagoCuenta(pagoId: number, motivo?: string): Promise<CuentaDetalle> {
  return smartInvoke("rechazar_pago_cuenta", { pagoId, motivo: motivo ?? null });
}

export async function contarPagosPendientes(): Promise<number> {
  return smartInvoke("contar_pagos_pendientes");
}

// --- Respaldo ---

export async function obtenerRutaDb(): Promise<string> {
  return invoke("obtener_ruta_db");
}

export async function crearRespaldo(destino: string): Promise<string> {
  return invoke("crear_respaldo", { destino });
}

export async function restaurarRespaldo(origen: string): Promise<string> {
  return invoke("restaurar_respaldo", { origen });
}

// --- Licencia ---

export async function obtenerMachineId(): Promise<string> {
  return invoke("obtener_machine_id");
}

export async function verificarLicencia(claveLicencia: string): Promise<LicenciaInfo> {
  return invoke("verificar_licencia", { claveLicencia });
}

export async function obtenerEstadoLicencia(): Promise<LicenciaInfo | null> {
  return invoke("obtener_estado_licencia");
}

// --- Demo ---

export async function activarDemo(): Promise<LicenciaInfo> {
  return smartInvoke("activar_demo");
}

export async function salirDemo(): Promise<void> {
  return smartInvoke("salir_demo");
}

export async function esDemo(): Promise<boolean> {
  return smartInvoke("es_demo");
}

// --- Configuración ---

export async function obtenerConfig(): Promise<Record<string, string>> {
  return smartInvoke("obtener_config");
}

export async function guardarConfig(configs: Record<string, string>): Promise<void> {
  return smartInvoke("guardar_config", { configs });
}

export async function cargarLogoNegocio(logoPath: string): Promise<string> {
  return smartInvoke("cargar_logo_negocio", { logoPath });
}

export async function eliminarLogoNegocio(): Promise<string> {
  return smartInvoke("eliminar_logo_negocio");
}

// --- Usuarios / Sesión ---

export async function iniciarSesion(pin: string): Promise<SesionActiva> {
  return smartInvoke("iniciar_sesion", { pin });
}

export async function cerrarSesion(): Promise<void> {
  return smartInvoke("cerrar_sesion");
}

export async function obtenerSesionActual(): Promise<SesionActiva | null> {
  return smartInvoke("obtener_sesion_actual");
}

export async function verificarPinAdmin(pin: string): Promise<string> {
  return smartInvoke("verificar_pin_admin", { pin });
}

export async function crearUsuario(usuario: NuevoUsuario): Promise<UsuarioInfo> {
  return smartInvoke("crear_usuario", { usuario });
}

export async function listarUsuarios(): Promise<UsuarioInfo[]> {
  return smartInvoke("listar_usuarios");
}

export async function actualizarUsuario(
  id: number,
  nombre?: string,
  pin?: string,
  rol?: string,
  activo?: boolean
): Promise<UsuarioInfo> {
  return smartInvoke("actualizar_usuario", { id, nombre, pin, rol, activo });
}

export async function eliminarUsuario(id: number): Promise<void> {
  return smartInvoke("eliminar_usuario", { id });
}

// --- Exportar CSV ---

export async function exportarVentasCsv(fechaInicio: string, fechaFin: string, ruta: string): Promise<string> {
  return invoke("exportar_ventas_csv", { fechaInicio, fechaFin, ruta });
}

export async function exportarGastosCsv(fechaInicio: string, fechaFin: string, ruta: string): Promise<string> {
  return invoke("exportar_gastos_csv", { fechaInicio, fechaFin, ruta });
}

export async function exportarInventarioCsv(ruta: string): Promise<string> {
  return invoke("exportar_inventario_csv", { ruta });
}

// --- SRI - Facturacion Electronica ---

export async function cargarCertificadoSri(p12Path: string, password: string): Promise<string> {
  return invoke("cargar_certificado_sri", { p12Path, password });
}

export async function emitirFacturaSri(ventaId: number): Promise<ResultadoEmision> {
  return smartInvoke("emitir_factura_sri", { ventaId });
}

export async function consultarEstadoSri(): Promise<EstadoSri> {
  return smartInvoke("consultar_estado_sri");
}

export async function cambiarAmbienteSri(ambiente: string): Promise<void> {
  return smartInvoke("cambiar_ambiente_sri", { ambiente });
}

export async function validarSuscripcionSri(): Promise<EstadoSri> {
  return smartInvoke("validar_suscripcion_sri");
}

// --- SRI - Contratación de Planes ---

export async function obtenerPlanesSri(): Promise<PlanesDisponibles> {
  return smartInvoke("obtener_planes_sri");
}

export async function crearPedidoSri(
  planClave: string,
  planNombre: string,
  precio: number,
  metodoPago: string
): Promise<PedidoCreado> {
  return smartInvoke("crear_pedido_sri", { planClave, planNombre, precio, metodoPago });
}

export async function obtenerXmlFirmado(ventaId: number): Promise<string> {
  return smartInvoke("obtener_xml_firmado", { ventaId });
}

export async function generarRidePdf(ventaId: number): Promise<string> {
  return invoke("generar_ride_pdf", { ventaId });
}

export async function imprimirRide(ventaId: number): Promise<string> {
  return invoke("imprimir_ride", { ventaId });
}

export async function enviarNotificacionSri(ventaId: number, email: string): Promise<string> {
  return smartInvoke("enviar_notificacion_sri", { ventaId, email });
}

export async function procesarEmailsPendientes(): Promise<{ total: number; enviados: number; fallidos: number }> {
  return smartInvoke("procesar_emails_pendientes");
}

export async function obtenerEmailsPendientes(): Promise<number> {
  return smartInvoke("obtener_emails_pendientes");
}

// --- Notas de Crédito ---

export async function registrarNotaCredito(nota: NuevaNotaCredito): Promise<NotaCreditoInfo> {
  return smartInvoke("registrar_nota_credito", { nota });
}

export async function listarNotasCreditoDia(fecha: string): Promise<NotaCreditoInfo[]> {
  return smartInvoke("listar_notas_credito_dia", { fecha });
}

// --- Ventas por sesión de caja (para cajeros) ---

export async function listarVentasSesionCaja(): Promise<Venta[]> {
  return smartInvoke("listar_ventas_sesion_caja");
}

export async function resumenSesionCaja(): Promise<ResumenDiario> {
  return smartInvoke("resumen_sesion_caja");
}

export async function listarNotasCreditoSesionCaja(): Promise<NotaCreditoInfo[]> {
  return smartInvoke("listar_notas_credito_sesion_caja");
}

export async function emitirNotaCreditoSri(ncId: number): Promise<ResultadoEmision> {
  return smartInvoke("emitir_nota_credito_sri", { ncId });
}

export async function generarRideNcPdf(ncId: number): Promise<string> {
  return invoke("generar_ride_nc_pdf", { ncId });
}

// --- Inventario / Kardex ---

export interface MovimientoInventario {
  id?: number;
  producto_id: number;
  producto_nombre?: string;
  producto_codigo?: string;
  tipo: string;
  cantidad: number;
  stock_anterior: number;
  stock_nuevo: number;
  costo_unitario?: number;
  referencia_id?: number;
  motivo?: string;
  usuario?: string;
  created_at?: string;
}

export interface ResumenInventario {
  total_productos: number;
  total_entradas_mes: number;
  total_salidas_mes: number;
  total_ajustes_mes: number;
  valor_inventario: number;
}

export async function registrarMovimiento(
  productoId: number,
  tipo: string,
  cantidad: number,
  motivo?: string,
  costoUnitario?: number,
  usuario?: string,
): Promise<MovimientoInventario> {
  return smartInvoke("registrar_movimiento", { productoId, tipo, cantidad, motivo, costoUnitario: costoUnitario ?? null, usuario: usuario ?? null });
}

export async function listarMovimientos(
  productoId?: number,
  fechaInicio?: string,
  fechaFin?: string,
  tipo?: string,
  limite?: number,
): Promise<MovimientoInventario[]> {
  return smartInvoke("listar_movimientos", {
    productoId: productoId ?? null,
    fechaInicio: fechaInicio ?? null,
    fechaFin: fechaFin ?? null,
    tipo: tipo ?? null,
    limite: limite ?? null,
  });
}

export async function resumenInventario(): Promise<ResumenInventario> {
  return smartInvoke("resumen_inventario");
}

// --- Listas de Precios ---

export async function listarListasPrecios(): Promise<ListaPrecio[]> {
  return smartInvoke("listar_listas_precios");
}

export async function crearListaPrecio(lista: ListaPrecio): Promise<number> {
  return smartInvoke("crear_lista_precio", { lista });
}

export async function actualizarListaPrecio(lista: ListaPrecio): Promise<void> {
  return smartInvoke("actualizar_lista_precio", { lista });
}

export async function establecerListaDefault(id: number): Promise<void> {
  return smartInvoke("establecer_lista_default", { id });
}

export async function guardarPreciosProducto(productoId: number, precios: PrecioProducto[]): Promise<void> {
  return smartInvoke("guardar_precios_producto", { productoId, precios });
}

export async function obtenerPreciosProducto(productoId: number): Promise<PrecioProductoDetalle[]> {
  return smartInvoke("obtener_precios_producto", { productoId });
}

export async function resolverPrecioProducto(productoId: number, clienteId?: number): Promise<number> {
  return smartInvoke("resolver_precio_producto", { productoId, clienteId: clienteId ?? null });
}

// --- Establecimientos y Puntos de Emisión ---

export async function listarEstablecimientos(): Promise<Establecimiento[]> {
  return smartInvoke("listar_establecimientos");
}

export async function crearEstablecimiento(establecimiento: Establecimiento): Promise<Establecimiento> {
  return smartInvoke("crear_establecimiento", { establecimiento });
}

export async function actualizarEstablecimiento(establecimiento: Establecimiento): Promise<void> {
  return smartInvoke("actualizar_establecimiento", { establecimiento });
}

export async function listarPuntosEmision(establecimientoId: number): Promise<PuntoEmision[]> {
  return smartInvoke("listar_puntos_emision", { establecimientoId });
}

export async function crearPuntoEmision(punto: PuntoEmision): Promise<PuntoEmision> {
  return smartInvoke("crear_punto_emision", { punto });
}

export async function actualizarPuntoEmision(punto: PuntoEmision): Promise<void> {
  return smartInvoke("actualizar_punto_emision", { punto });
}

// --- Red ---

export async function generarTokenServidor(): Promise<string> {
  return invoke("generar_token_servidor");
}

export async function probarConexionServidor(url: string, token: string): Promise<string> {
  return invoke("probar_conexion_servidor", { url, token });
}

// --- Transferencias y Multi-almacén ---

export async function crearTransferencia(
  productoId: number, origenEstablecimientoId: number, destinoEstablecimientoId: number,
  cantidad: number, usuario?: string
): Promise<TransferenciaStock> {
  return smartInvoke("crear_transferencia", { productoId, origenEstablecimientoId, destinoEstablecimientoId, cantidad, usuario: usuario ?? null });
}

export async function recibirTransferencia(id: number): Promise<void> {
  return smartInvoke("recibir_transferencia", { id });
}

export async function listarTransferencias(establecimientoId?: number, estado?: string): Promise<TransferenciaStock[]> {
  return smartInvoke("listar_transferencias", { establecimientoId: establecimientoId ?? null, estado: estado ?? null });
}

export async function stockPorEstablecimiento(productoId: number): Promise<StockEstablecimiento[]> {
  return smartInvoke("stock_por_establecimiento", { productoId });
}

export async function actualizarStockEstablecimiento(
  productoId: number, establecimientoId: number, stockActual: number, stockMinimo: number
): Promise<void> {
  return smartInvoke("actualizar_stock_establecimiento", { productoId, establecimientoId, stockActual, stockMinimo });
}

// --- Backup Cloud ---

export async function ejecutarBackupCloud(): Promise<string> {
  return invoke("ejecutar_backup_cloud");
}

export async function estadoBackupCloud(): Promise<{
  activo: boolean;
  tipo: string;
  frecuencia_horas: number;
  ultima: string;
  gdrive_conectado: boolean;
}> {
  return invoke("estado_backup_cloud");
}

export async function guardarGdriveTokens(accessToken: string, refreshToken: string): Promise<void> {
  return invoke("guardar_gdrive_tokens", { accessToken, refreshToken });
}

export async function desconectarGdrive(): Promise<void> {
  return invoke("desconectar_gdrive");
}

export async function conectarGdrive(): Promise<string> {
  return invoke("conectar_gdrive");
}

// --- Reportes avanzados ---
import type { ReporteUtilidad, ReporteBalance, ProductoRentabilidad } from "../types";
export async function reporteUtilidad(fechaInicio: string, fechaHasta: string): Promise<ReporteUtilidad> {
  return smartInvoke("reporte_utilidad", { fechaInicio, fechaHasta });
}
export async function reporteBalance(fechaInicio: string, fechaHasta: string): Promise<ReporteBalance> {
  return smartInvoke("reporte_balance", { fechaInicio, fechaHasta });
}
export async function reporteProductosRentabilidad(fechaInicio: string, fechaHasta: string, limite: number = 50): Promise<ProductoRentabilidad[]> {
  return smartInvoke("reporte_productos_rentabilidad", { fechaInicio, fechaHasta, limite });
}
