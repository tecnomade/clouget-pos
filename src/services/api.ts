import { invoke } from "@tauri-apps/api/core";
import type {
  Producto,
  ProductoBusqueda,
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
} from "../types";

// --- Productos ---

export async function crearProducto(producto: Producto): Promise<number> {
  return invoke("crear_producto", { producto });
}

export async function actualizarProducto(producto: Producto): Promise<void> {
  return invoke("actualizar_producto", { producto });
}

export async function buscarProductos(termino: string, listaPrecioId?: number): Promise<ProductoBusqueda[]> {
  return invoke("buscar_productos", { termino, listaPrecioId: listaPrecioId ?? null });
}

export async function obtenerProducto(id: number): Promise<Producto> {
  return invoke("obtener_producto", { id });
}

export async function listarProductos(soloActivos: boolean = true, listaPrecioId?: number): Promise<ProductoBusqueda[]> {
  return invoke("listar_productos", { soloActivos, listaPrecioId: listaPrecioId ?? null });
}

export async function productosMasVendidos(limite: number = 12): Promise<ProductoBusqueda[]> {
  return invoke("productos_mas_vendidos", { limite });
}

// --- Categorías ---

export async function crearCategoria(categoria: Categoria): Promise<number> {
  return invoke("crear_categoria", { categoria });
}

export async function listarCategorias(): Promise<Categoria[]> {
  return invoke("listar_categorias");
}

// --- Clientes ---

export async function crearCliente(cliente: Cliente): Promise<number> {
  return invoke("crear_cliente", { cliente });
}

export async function actualizarCliente(cliente: Cliente): Promise<void> {
  return invoke("actualizar_cliente", { cliente });
}

export async function buscarClientes(termino: string): Promise<Cliente[]> {
  return invoke("buscar_clientes", { termino });
}

export async function listarClientes(): Promise<Cliente[]> {
  return invoke("listar_clientes");
}

// --- Ventas ---

export async function registrarVenta(venta: NuevaVenta): Promise<VentaCompleta> {
  return invoke("registrar_venta", { venta });
}

export async function listarVentasDia(fecha: string): Promise<Venta[]> {
  return invoke("listar_ventas_dia", { fecha });
}

export async function obtenerVenta(id: number): Promise<VentaCompleta> {
  return invoke("obtener_venta", { id });
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
  return invoke("resumen_diario", { fecha });
}

export async function productosMasVendidosReporte(fechaInicio: string, fechaFin: string, limite: number = 10): Promise<ProductoMasVendido[]> {
  return invoke("productos_mas_vendidos_reporte", { fechaInicio, fechaFin, limite });
}

export async function alertasStockBajo(): Promise<AlertaStock[]> {
  return invoke("alertas_stock_bajo");
}

export async function resumenFiadosPendientes(): Promise<number> {
  return invoke("resumen_fiados_pendientes");
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
  return invoke("resumen_periodo", { fechaInicio, fechaFin });
}

export async function listarVentasPeriodo(fechaInicio: string, fechaFin: string): Promise<Venta[]> {
  return invoke("listar_ventas_periodo", { fechaInicio, fechaFin });
}

// --- Caja ---

export async function abrirCaja(montoInicial: number): Promise<Caja> {
  return invoke("abrir_caja", { montoInicial });
}

export async function cerrarCaja(montoReal: number, observacion?: string): Promise<ResumenCaja> {
  return invoke("cerrar_caja", { montoReal, observacion });
}

export async function obtenerCajaAbierta(): Promise<Caja | null> {
  return invoke("obtener_caja_abierta");
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
  return invoke("crear_gasto", { gasto });
}

export async function listarGastosDia(fecha: string): Promise<Gasto[]> {
  return invoke("listar_gastos_dia", { fecha });
}

export async function eliminarGasto(id: number): Promise<void> {
  return invoke("eliminar_gasto", { id });
}

// --- Cuentas por Cobrar ---

export async function resumenDeudores(): Promise<ResumenCliente[]> {
  return invoke("resumen_deudores");
}

export async function listarCuentasPendientes(clienteId?: number): Promise<CuentaConCliente[]> {
  return invoke("listar_cuentas_pendientes", { clienteId: clienteId ?? null });
}

export async function obtenerCuentaDetalle(id: number): Promise<CuentaDetalle> {
  return invoke("obtener_cuenta_detalle", { id });
}

export async function registrarPagoCuenta(pago: PagoCuenta): Promise<CuentaPorCobrar> {
  return invoke("registrar_pago_cuenta", { pago });
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

// --- Configuración ---

export async function obtenerConfig(): Promise<Record<string, string>> {
  return invoke("obtener_config");
}

export async function guardarConfig(configs: Record<string, string>): Promise<void> {
  return invoke("guardar_config", { configs });
}

export async function cargarLogoNegocio(logoPath: string): Promise<string> {
  return invoke("cargar_logo_negocio", { logoPath });
}

export async function eliminarLogoNegocio(): Promise<string> {
  return invoke("eliminar_logo_negocio");
}

// --- Usuarios / Sesión ---

export async function iniciarSesion(pin: string): Promise<SesionActiva> {
  return invoke("iniciar_sesion", { pin });
}

export async function cerrarSesion(): Promise<void> {
  return invoke("cerrar_sesion");
}

export async function obtenerSesionActual(): Promise<SesionActiva | null> {
  return invoke("obtener_sesion_actual");
}

export async function verificarPinAdmin(pin: string): Promise<string> {
  return invoke("verificar_pin_admin", { pin });
}

export async function crearUsuario(usuario: NuevoUsuario): Promise<UsuarioInfo> {
  return invoke("crear_usuario", { usuario });
}

export async function listarUsuarios(): Promise<UsuarioInfo[]> {
  return invoke("listar_usuarios");
}

export async function actualizarUsuario(
  id: number,
  nombre?: string,
  pin?: string,
  rol?: string,
  activo?: boolean
): Promise<UsuarioInfo> {
  return invoke("actualizar_usuario", { id, nombre, pin, rol, activo });
}

export async function eliminarUsuario(id: number): Promise<void> {
  return invoke("eliminar_usuario", { id });
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
  return invoke("emitir_factura_sri", { ventaId });
}

export async function consultarEstadoSri(): Promise<EstadoSri> {
  return invoke("consultar_estado_sri");
}

export async function cambiarAmbienteSri(ambiente: string): Promise<void> {
  return invoke("cambiar_ambiente_sri", { ambiente });
}

export async function validarSuscripcionSri(): Promise<EstadoSri> {
  return invoke("validar_suscripcion_sri");
}

// --- SRI - Contratación de Planes ---

export async function obtenerPlanesSri(): Promise<PlanesDisponibles> {
  return invoke("obtener_planes_sri");
}

export async function crearPedidoSri(
  planClave: string,
  planNombre: string,
  precio: number,
  metodoPago: string
): Promise<PedidoCreado> {
  return invoke("crear_pedido_sri", { planClave, planNombre, precio, metodoPago });
}

export async function obtenerXmlFirmado(ventaId: number): Promise<string> {
  return invoke("obtener_xml_firmado", { ventaId });
}

export async function generarRidePdf(ventaId: number): Promise<string> {
  return invoke("generar_ride_pdf", { ventaId });
}

export async function imprimirRide(ventaId: number): Promise<string> {
  return invoke("imprimir_ride", { ventaId });
}

export async function enviarNotificacionSri(ventaId: number, email: string): Promise<string> {
  return invoke("enviar_notificacion_sri", { ventaId, email });
}

export async function procesarEmailsPendientes(): Promise<{ total: number; enviados: number; fallidos: number }> {
  return invoke("procesar_emails_pendientes");
}

export async function obtenerEmailsPendientes(): Promise<number> {
  return invoke("obtener_emails_pendientes");
}

// --- Notas de Crédito ---

export async function registrarNotaCredito(nota: NuevaNotaCredito): Promise<NotaCreditoInfo> {
  return invoke("registrar_nota_credito", { nota });
}

export async function listarNotasCreditoDia(fecha: string): Promise<NotaCreditoInfo[]> {
  return invoke("listar_notas_credito_dia", { fecha });
}

// --- Ventas por sesión de caja (para cajeros) ---

export async function listarVentasSesionCaja(): Promise<Venta[]> {
  return invoke("listar_ventas_sesion_caja");
}

export async function resumenSesionCaja(): Promise<ResumenDiario> {
  return invoke("resumen_sesion_caja");
}

export async function listarNotasCreditoSesionCaja(): Promise<NotaCreditoInfo[]> {
  return invoke("listar_notas_credito_sesion_caja");
}

export async function emitirNotaCreditoSri(ncId: number): Promise<ResultadoEmision> {
  return invoke("emitir_nota_credito_sri", { ncId });
}

export async function generarRideNcPdf(ncId: number): Promise<string> {
  return invoke("generar_ride_nc_pdf", { ncId });
}

// --- Listas de Precios ---

export async function listarListasPrecios(): Promise<ListaPrecio[]> {
  return invoke("listar_listas_precios");
}

export async function crearListaPrecio(lista: ListaPrecio): Promise<number> {
  return invoke("crear_lista_precio", { lista });
}

export async function actualizarListaPrecio(lista: ListaPrecio): Promise<void> {
  return invoke("actualizar_lista_precio", { lista });
}

export async function establecerListaDefault(id: number): Promise<void> {
  return invoke("establecer_lista_default", { id });
}

export async function guardarPreciosProducto(productoId: number, precios: PrecioProducto[]): Promise<void> {
  return invoke("guardar_precios_producto", { productoId, precios });
}

export async function obtenerPreciosProducto(productoId: number): Promise<PrecioProductoDetalle[]> {
  return invoke("obtener_precios_producto", { productoId });
}

export async function resolverPrecioProducto(productoId: number, clienteId?: number): Promise<number> {
  return invoke("resolver_precio_producto", { productoId, clienteId: clienteId ?? null });
}
