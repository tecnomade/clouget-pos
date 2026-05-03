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
  Proveedor,
  NuevaCompra,
  CompraCompleta,
  Compra,
  ResumenAcreedor,
  CuentaPorPagar,
  PagoProveedor,
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

/**
 * Lee y codifica una imagen en base64 sin tocar la DB.
 * Para usar al crear un producto nuevo (cuando aun no hay id) — la imagen
 * queda en el form y se persiste cuando se llama a crearProducto.
 */
export async function leerImagenArchivo(imagenPath: string): Promise<string> {
  return smartInvoke("leer_imagen_archivo", { imagenPath });
}

export async function eliminarImagenProducto(id: number): Promise<void> {
  return smartInvoke("eliminar_imagen_producto", { id });
}

export async function eliminarProducto(id: number): Promise<void> {
  return smartInvoke("eliminar_producto", { id });
}

export async function listarProductosTactil(): Promise<ProductoTactil[]> {
  return smartInvoke("listar_productos_tactil");
}

// --- Números de Serie ---

export const registrarSeries = (productoId: number, seriales: string[], compraId?: number) =>
  smartInvoke<{ insertados: number; duplicados: number }>("registrar_series", { productoId, seriales, compraId: compraId ?? null });

export const listarSeriesProducto = (productoId: number, estado?: string) =>
  smartInvoke<any[]>("listar_series_producto", { productoId, estado: estado ?? null });

export const seriesDisponibles = (productoId: number) =>
  smartInvoke<{ id: number; serial: string }[]>("series_disponibles", { productoId });

export const marcarSerieVendida = (serieId: number, ventaId: number, ventaDetalleId?: number, clienteId?: number, clienteNombre?: string) =>
  smartInvoke<void>("marcar_serie_vendida", { serieId, ventaId, ventaDetalleId: ventaDetalleId ?? null, clienteId: clienteId ?? null, clienteNombre: clienteNombre ?? null });

export const buscarSerie = (serial: string) =>
  smartInvoke<any[]>("buscar_serie", { serial });

export const devolverSerie = (serieId: number) =>
  smartInvoke<void>("devolver_serie", { serieId });

// --- Caducidad ---
export const registrarLoteCaducidad = (productoId: number, lote: string | null, fechaCaducidad: string, cantidad: number, compraId?: number, observacion?: string, fechaElaboracion?: string) =>
  smartInvoke<number>("registrar_lote_caducidad", { productoId, lote, fechaCaducidad, cantidad, compraId: compraId ?? null, observacion: observacion ?? null, fechaElaboracion: fechaElaboracion ?? null });

export const listarLotesProducto = (productoId: number) =>
  smartInvoke<any[]>("listar_lotes_producto", { productoId });

export const alertasCaducidad = () =>
  smartInvoke<{lotes: any[], vencidos: number, por_vencer: number, dias_alerta: number}>("alertas_caducidad");

// --- Combos / Kits ---
export interface ComboGrupo {
  id?: number;
  producto_padre_id: number;
  nombre: string;
  minimo: number;
  maximo: number;
  orden: number;
}
export interface ComboComponente {
  id?: number;
  producto_padre_id: number;
  producto_hijo_id: number;
  cantidad: number;
  grupo_id?: number | null;
  orden: number;
  hijo_nombre?: string;
  hijo_codigo?: string;
  hijo_precio_venta?: number;
  hijo_precio_costo?: number;
  hijo_stock_actual?: number;
  hijo_unidad_medida?: string;
  hijo_no_controla_stock?: boolean;
  hijo_es_servicio?: boolean;
}
export const listarComboGrupos = (productoPadreId: number) =>
  smartInvoke<ComboGrupo[]>("listar_combo_grupos", { productoPadreId });
export const listarComboComponentes = (productoPadreId: number) =>
  smartInvoke<ComboComponente[]>("listar_combo_componentes", { productoPadreId });
export const guardarComboEstructura = (productoPadreId: number, grupos: ComboGrupo[], componentes: ComboComponente[]) =>
  smartInvoke<void>("guardar_combo_estructura", { productoPadreId, grupos, componentes });
export const stockCombo = (productoPadreId: number) =>
  smartInvoke<number | null>("stock_combo", { productoPadreId });
export const infoComboResumen = (productoId: number) =>
  smartInvoke<{ tipo_producto: string; es_combo: boolean; stock_calculado: number | null; total_componentes: number }>(
    "info_combo_resumen", { productoId }
  );

export const listarTodosLotes = (filtroEstado?: string, busquedaProducto?: string, incluirAgotados?: boolean) =>
  smartInvoke<{lotes: any[], vencidos: number, por_vencer: number, ok: number, total_unidades: number, dias_alerta: number}>(
    "listar_todos_lotes",
    {
      filtroEstado: filtroEstado ?? null,
      busquedaProducto: busquedaProducto ?? null,
      incluirAgotados: incluirAgotados ?? null,
    }
  );

export const eliminarLoteCaducidad = (loteId: number) =>
  smartInvoke<void>("eliminar_lote_caducidad", { loteId });

export const ajustarCantidadLote = (loteId: number, cantidad: number) =>
  smartInvoke<void>("ajustar_cantidad_lote", { loteId, cantidad });

// --- Categorías ---

export async function crearCategoria(categoria: Categoria): Promise<number> {
  return smartInvoke("crear_categoria", { categoria });
}

export async function actualizarCategoria(id: number, nombre: string): Promise<void> {
  return smartInvoke("actualizar_categoria", { id, nombre });
}

export async function eliminarCategoria(id: number, accion?: string, moverA?: number): Promise<any> {
  return smartInvoke("eliminar_categoria", { id, accion, moverA });
}

export async function listarCategorias(): Promise<Categoria[]> {
  return smartInvoke("listar_categorias");
}

// --- Tipos de Unidad ---

export const listarTiposUnidad = () => smartInvoke<Array<{ id: number; nombre: string; abreviatura: string; factor_default: number; es_agrupada: boolean }>>("listar_tipos_unidad");
export const crearTipoUnidad = (nombre: string, abreviatura: string, factorDefault?: number, esAgrupada?: boolean) =>
  smartInvoke<number>("crear_tipo_unidad", { nombre, abreviatura, factorDefault, esAgrupada });
export const actualizarTipoUnidad = (id: number, nombre: string, abreviatura: string, factorDefault?: number, esAgrupada?: boolean) =>
  smartInvoke<void>("actualizar_tipo_unidad", { id, nombre, abreviatura, factorDefault, esAgrupada });
export const eliminarTipoUnidad = (id: number) => smartInvoke<void>("eliminar_tipo_unidad", { id });

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

export async function listarPagosVenta(ventaId: number): Promise<any[]> {
  return smartInvoke("listar_pagos_venta", { ventaId });
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
  itemsOverride?: { producto_id: number; precio_unitario: number; descuento: number; cantidad?: number }[];
}): Promise<VentaCompleta> {
  return smartInvoke("convertir_guia_a_venta", params);
}

export async function cambiarEstadoGuia(guiaId: number, nuevoEstado: string): Promise<void> {
  return smartInvoke("cambiar_estado_guia", { guiaId, nuevoEstado });
}

export async function listarChoferes(): Promise<[number, string, string | null][]> {
  return smartInvoke("listar_choferes", {});
}

// === Vehiculos guardados (placas para guias) ===
export async function listarVehiculos(): Promise<[number, string, string | null][]> {
  return smartInvoke("listar_vehiculos", {});
}
export async function guardarVehiculo(placa: string, descripcion?: string): Promise<void> {
  return smartInvoke("guardar_vehiculo", { placa, descripcion: descripcion || null });
}

// === Direcciones de entrega del cliente ===
export interface DireccionCliente {
  id: number;
  direccion: string;
  etiqueta?: string | null;
  contacto_nombre?: string | null;
  contacto_telefono?: string | null;
  referencia?: string | null;
}
export async function listarDireccionesCliente(clienteId: number): Promise<DireccionCliente[]> {
  return smartInvoke("listar_direcciones_cliente", { clienteId });
}
export async function guardarDireccionCliente(clienteId: number, direccion: string, etiqueta?: string): Promise<number> {
  return smartInvoke("guardar_direccion_cliente", { clienteId, direccion, etiqueta: etiqueta || null });
}
export async function eliminarDireccionCliente(id: number): Promise<void> {
  return smartInvoke("eliminar_direccion_cliente", { id });
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

export async function reporteVentasPorCajero(fechaDesde?: string, fechaHasta?: string): Promise<{
  cajeros: any[];
  total_global: number;
  num_ventas_global: number;
  ticket_promedio_global: number;
  total_cajeros: number;
  descuadre_neto_global: number;
  fecha_desde: string;
  fecha_hasta: string;
}> {
  return smartInvoke("reporte_ventas_por_cajero", {
    fechaDesde: fechaDesde ?? null,
    fechaHasta: fechaHasta ?? null,
  });
}

// --- Caja ---

export async function abrirCaja(montoInicial: number, motivoDiferencia?: string, desglose?: string): Promise<Caja> {
  return smartInvoke("abrir_caja", { montoInicial, motivoDiferencia: motivoDiferencia ?? null, desglose: desglose ?? null });
}

export async function cerrarCaja(montoReal: number, observacion?: string, motivoDescuadre?: string, desglose?: string, pinSupervisor?: string): Promise<ResumenCaja> {
  return smartInvoke("cerrar_caja", { montoReal, observacion: observacion ?? null, motivoDescuadre: motivoDescuadre ?? null, desglose: desglose ?? null, pinSupervisor: pinSupervisor ?? null });
}

export async function listarSesionesCaja(fechaDesde?: string, fechaHasta?: string, usuario?: string, soloDescuadradas?: boolean): Promise<any[]> {
  return smartInvoke("listar_sesiones_caja", {
    fechaDesde: fechaDesde ?? null,
    fechaHasta: fechaHasta ?? null,
    usuario: usuario ?? null,
    soloDescuadradas: soloDescuadradas ?? null,
  });
}

export async function registrarDepositoCierre(cajaId: number, monto: number, bancoId: number, referencia?: string, comprobanteImagen?: string): Promise<{ id: number; estado: string }> {
  return smartInvoke("registrar_deposito_cierre", {
    cajaId, monto, bancoId,
    referencia: referencia ?? null,
    comprobanteImagen: comprobanteImagen ?? null,
  });
}

export async function obtenerCajaAbierta(): Promise<Caja | null> {
  return smartInvoke("obtener_caja_abierta");
}

export async function obtenerUltimoCierre(): Promise<{
  caja_id: number;
  monto_real: number | null;
  cerrada_at: string | null;
  usuario_cierre: string | null;
  diferencia_cierre: number | null;
} | null> {
  return smartInvoke("obtener_ultimo_cierre");
}

export async function listarEventosCaja(cajaId: number): Promise<any[]> {
  return smartInvoke("listar_eventos_caja", { cajaId });
}

export async function historialDescuadresCaja(fechaDesde?: string, fechaHasta?: string): Promise<{
  cierres: any[];
  total_descuadrados: number;
  total_faltantes: number;
  total_sobrantes: number;
  neto: number;
  por_usuario: any[];
}> {
  return smartInvoke("historial_descuadres_caja", { fechaDesde: fechaDesde ?? null, fechaHasta: fechaHasta ?? null });
}

// --- Retiros de Caja ---

export const registrarRetiro = (monto: number, motivo: string, bancoId?: number, referencia?: string) =>
  smartInvoke<any>("registrar_retiro", { monto, motivo, bancoId: bancoId ?? null, referencia: referencia ?? null });

// v2.3.46+: ingreso manual a caja (solo admin)
// Casos: compensar gasto erroneo de caja anterior, aporte del dueno, etc.
export const registrarIngresoCaja = (monto: number, motivo: string) =>
  smartInvoke<any>("registrar_ingreso_caja", { monto, motivo });

export async function listarRetirosCaja(cajaId: number): Promise<any[]> {
  return smartInvoke("listar_retiros_caja", { cajaId });
}

export const confirmarDeposito = (retiroId: number, referencia: string, comprobanteImagen?: string) =>
  smartInvoke<void>("confirmar_deposito", { retiroId, referencia, comprobanteImagen: comprobanteImagen ?? null });

// --- Impresión ---

export async function imprimirTicket(ventaId: number): Promise<string> {
  return invoke("imprimir_ticket", { ventaId });
}

export async function imprimirTicketPdf(ventaId: number): Promise<string> {
  return invoke("imprimir_ticket_pdf", { ventaId });
}

// v2.3.53: detallado=false (default) imprime solo totales — ahorra papel.
// detallado=true incluye lista item por item de ventas, gastos, retiros, cobros.
export async function imprimirReporteCaja(cajaId: number, detallado?: boolean): Promise<string> {
  return invoke("imprimir_reporte_caja", { cajaId, detallado: detallado ?? false });
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

// --- Cotización PDF ---

export async function generarCotizacionPdf(ventaId: number): Promise<string> {
  return invoke("generar_cotizacion_pdf", { ventaId });
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

export async function listarPagosPendientesConfirmacion(): Promise<any[]> {
  return smartInvoke("listar_pagos_pendientes_confirmacion");
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

export async function obtenerSecuenciales(): Promise<Record<string, number>> {
  return smartInvoke("obtener_secuenciales");
}

export async function actualizarSecuencial(establecimiento: string, puntoEmision: string, tipoDocumento: string, secuencial: number): Promise<void> {
  return smartInvoke("actualizar_secuencial", { establecimiento, puntoEmision, tipoDocumento, secuencial });
}

export async function cargarLogoNegocio(logoPath: string): Promise<string> {
  return smartInvoke("cargar_logo_negocio", { logoPath });
}

export async function eliminarLogoNegocio(): Promise<string> {
  return smartInvoke("eliminar_logo_negocio");
}

// --- Usuarios / Sesión ---

export async function iniciarSesion(pin: string, password?: string): Promise<SesionActiva> {
  return smartInvoke("iniciar_sesion", { pin, password: password ?? null });
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
  activo?: boolean,
  permisos?: string
): Promise<UsuarioInfo> {
  return smartInvoke("actualizar_usuario", { id, nombre, pin, rol, activo, permisos });
}

export const obtenerPermisosDisponibles = () => smartInvoke<[string, string][]>("obtener_permisos_disponibles");

export async function eliminarUsuario(id: number): Promise<void> {
  return smartInvoke("eliminar_usuario", { id });
}

export async function cambiarPassword(usuarioId: number, password: string): Promise<void> {
  return smartInvoke("cambiar_password", { usuarioId, password });
}

export async function listarUsuariosLogin(): Promise<[number, string][]> {
  return smartInvoke("listar_usuarios_login");
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

export const exportarPlantillaProductos = () => smartInvoke<number[]>("exportar_plantilla_productos");
export const exportarProductosExcel = () => smartInvoke<number[]>("exportar_productos_excel");
export const importarProductosExcel = (archivoBytes: number[]) => smartInvoke<{creados: number, actualizados: number, errores: number, mensajes: string[], lotes_creados?: number, warnings_caducidad?: string[]}>("importar_productos_excel", { archivoBytes });

// --- SRI - Facturacion Electronica ---

export async function cargarCertificadoSri(p12Path: string, password: string): Promise<string> {
  return invoke("cargar_certificado_sri", { p12Path, password });
}

export async function emitirFacturaSri(ventaId: number, formaPagoCreditoSri?: string): Promise<ResultadoEmision> {
  return smartInvoke("emitir_factura_sri", { ventaId, formaPagoCreditoSri });
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

export async function listarNotasCredito(fechaDesde: string, fechaHasta: string, estado?: string): Promise<any[]> {
  return smartInvoke("listar_notas_credito", { fechaDesde, fechaHasta, estado: estado ?? null });
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

export const anularVenta = (ventaId: number, motivo: string, efectivoDevuelto?: boolean) =>
  smartInvoke<void>("anular_venta", { ventaId, motivo, efectivoDevuelto: efectivoDevuelto ?? null });

export const crearDevolucionInterna = (ventaId: number, motivo: string, items: any[]) =>
  smartInvoke<any>("crear_devolucion_interna", { ventaId, motivo, items });

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

export const exportarKardexCsv = (fechaDesde: string, fechaHasta: string, productoId?: number) =>
  smartInvoke<string>("exportar_kardex_csv", { fechaDesde, fechaHasta, productoId: productoId ?? null });

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

export const resetearBaseDatos = (confirmacion: string) =>
  smartInvoke<string>("resetear_base_datos", { confirmacion });

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

export const reporteIvaMensual = (anio: number, mes: number) =>
  smartInvoke<{
    anio: number; mes: number; fecha_desde: string; fecha_hasta: string;
    ventas_0: number; ventas_15_base: number; iva_ventas: number;
    nc_base: number; nc_iva: number; iva_ventas_neto: number;
    compras_0: number; compras_15_base: number; iva_compras: number;
    iva_a_pagar: number; total_ventas: number; total_compras: number;
  }>("reporte_iva_mensual", { anio, mes });

// --- Export reportes XLSX/PDF (v2.1.0) ---
export const exportarInventarioXlsx = (ruta: string, categoriaNombre?: string, busqueda?: string, estadoFiltro?: string) =>
  smartInvoke<void>("exportar_inventario_xlsx", { ruta, categoriaNombre, busqueda, estadoFiltro });
export const exportarInventarioPdf = (ruta: string, categoriaNombre?: string, busqueda?: string, estadoFiltro?: string) =>
  smartInvoke<void>("exportar_inventario_pdf", { ruta, categoriaNombre, busqueda, estadoFiltro });
export const exportarTablaXlsx = (ruta: string, titulo: string, subtitulo: string | null, encabezados: string[], filas: string[][], columnasNumericas: number[] | null = null) =>
  smartInvoke<void>("exportar_tabla_xlsx", { ruta, titulo, subtitulo, encabezados, filas, columnasNumericas });
export const exportarTablaPdf = (ruta: string, titulo: string, subtitulo: string | null, encabezados: string[], filas: string[][], orientacionHorizontal: boolean | null = true) =>
  smartInvoke<void>("exportar_tabla_pdf", { ruta, titulo, subtitulo, encabezados, filas, orientacionHorizontal });

// --- Multi-unidad (v1.9.7) ---
export const listarUnidadesProducto = (productoId: number) =>
  smartInvoke<Array<{ id: number; nombre: string; abreviatura: string | null; factor: number; precio: number; es_base: boolean; orden: number; activa: boolean }>>(
    "listar_unidades_producto", { productoId });
export const guardarUnidadesProducto = (productoId: number, unidades: any[]) =>
  smartInvoke<void>("guardar_unidades_producto", { productoId, unidades });

// --- Reportes detallados (v1.9.4) ---
export const reporteCxcPorCliente = () => smartInvoke<any[]>("reporte_cxc_por_cliente");
export const reporteCxcDetalleCliente = (clienteId: number) =>
  smartInvoke<any[]>("reporte_cxc_detalle_cliente", { clienteId });
export const reporteCxpPorProveedor = () => smartInvoke<any[]>("reporte_cxp_por_proveedor");
export const reporteCxpDetalleProveedor = (proveedorId: number) =>
  smartInvoke<any[]>("reporte_cxp_detalle_proveedor", { proveedorId });
export const reporteInventarioValorizado = () =>
  smartInvoke<{
    productos: any[]; total_productos: number; total_unidades: number;
    valor_total_costo: number; valor_total_venta: number; utilidad_potencial: number;
    productos_sin_stock: number; productos_stock_bajo: number;
  }>("reporte_inventario_valorizado");
export const reporteKardexProducto = (productoId: number, fechaDesde?: string, fechaHasta?: string) =>
  smartInvoke<{ producto: any; movimientos: any[]; total_movimientos: number }>(
    "reporte_kardex_producto", { productoId, fechaDesde, fechaHasta });
export const reporteKardexMulti = (categorias: number[] | null, productos: number[] | null, fechaDesde?: string, fechaHasta?: string) =>
  smartInvoke<{
    movimientos: any[]; total_movimientos: number;
    total_entradas: number; total_salidas: number;
    valor_entradas: number; valor_salidas: number;
  }>("reporte_kardex_multi", { categorias, productos, fechaDesde, fechaHasta });
export const listarCategoriasSimple = () =>
  smartInvoke<Array<{ id: number; nombre: string }>>("listar_categorias_simple");

// --- Proveedores ---

export const crearProveedor = (proveedor: Proveedor) => smartInvoke<number>("crear_proveedor", { proveedor });
export const actualizarProveedor = (proveedor: Proveedor) => smartInvoke<void>("actualizar_proveedor", { proveedor });
export const listarProveedores = () => smartInvoke<Proveedor[]>("listar_proveedores");
export const buscarProveedores = (termino: string) => smartInvoke<Proveedor[]>("buscar_proveedores", { termino });
export const eliminarProveedor = (id: number) => smartInvoke<void>("eliminar_proveedor", { id });

// --- Compras ---

export const registrarCompra = (compra: NuevaCompra) => smartInvoke<CompraCompleta>("registrar_compra", { compra });
export const listarCompras = (fechaDesde: string, fechaHasta: string) => smartInvoke<Compra[]>("listar_compras", { fechaDesde, fechaHasta });
export const obtenerCompra = (id: number) => smartInvoke<CompraCompleta>("obtener_compra", { id });
export const anularCompra = (id: number) => smartInvoke<void>("anular_compra", { id });

// --- Importacion XML Factura Electronica (SRI) ---

export interface PreviewItemXml {
  codigo_principal?: string | null;
  descripcion: string;
  cantidad: number;
  precio_unitario: number;
  descuento: number;
  iva_porcentaje: number;
  subtotal: number;
  producto_existente_id?: number | null;
  producto_existente_nombre?: string | null;
}

export interface PreviewXmlCompra {
  proveedor_ruc: string;
  proveedor_nombre: string;
  proveedor_existe: boolean;
  proveedor_id?: number | null;
  numero_factura: string;
  fecha_emision: string;
  clave_acceso: string;
  subtotal_0: number;
  subtotal_15: number;
  iva: number;
  total: number;
  items: PreviewItemXml[];
}

export interface NuevoProductoSimple {
  codigo?: string | null;
  nombre: string;
  categoria_id?: number | null;
  iva_porcentaje: number;
}

export interface ItemMapeadoXml {
  accion: "producto_nuevo" | "producto_existente" | "gasto" | "ignorar";
  producto_id?: number | null;
  producto_nuevo?: NuevoProductoSimple | null;
  gasto_categoria?: string | null;
  descripcion: string;
  cantidad: number;
  precio_unitario: number;
  iva_porcentaje: number;
  subtotal: number;
}

export interface ImportarXmlInput {
  proveedor_id: number;
  numero_factura: string;
  fecha_emision: string;
  items_mapeados: ItemMapeadoXml[];
  forma_pago: string;
  dias_credito?: number | null;
  banco_id?: number | null;
  referencia_pago?: string | null;
}

export const previewXmlCompra = (xmlContenido: string) =>
  smartInvoke<PreviewXmlCompra>("preview_xml_compra", { xmlContenido });

export const importarXmlCompra = (input: ImportarXmlInput) =>
  smartInvoke<{ compra_id: number | null; productos_creados: number; gastos_creados: number; items_compra: number }>(
    "importar_xml_compra",
    { input }
  );

// --- Cuentas por Pagar ---

export const alertasPagosVencidos = () => smartInvoke<any[]>("alertas_pagos_vencidos");
export const resumenAcreedores = () => smartInvoke<ResumenAcreedor[]>("resumen_acreedores");
export const listarCuentasPagar = (proveedorId?: number) => smartInvoke<CuentaPorPagar[]>("listar_cuentas_pagar", { proveedorId: proveedorId ?? null });
export const registrarPagoProveedor = (cuentaId: number, monto: number, formaPago: string, comprobante?: string, observacion?: string, bancoId?: number) =>
  smartInvoke<void>("registrar_pago_proveedor", { cuentaId, monto, formaPago, numeroComprobante: comprobante ?? null, observacion: observacion ?? null, bancoId: bancoId ?? null });
export const historialPagosProveedor = (cuentaId: number) => smartInvoke<PagoProveedor[]>("historial_pagos_proveedor", { cuentaId });
export const listarMovimientosBancarios = (bancoId?: number, fechaDesde?: string, fechaHasta?: string) =>
  smartInvoke<any[]>("listar_movimientos_bancarios", { bancoId: bancoId ?? null, fechaDesde: fechaDesde ?? "", fechaHasta: fechaHasta ?? "" });

// --- Nota de Venta PDF ---

export async function generarNotaVentaPdf(ventaId: number): Promise<string> {
  return invoke("generar_nota_venta_pdf", { ventaId });
}

// --- Servicio Técnico ---

export interface OrdenServicio {
  id?: number;
  numero?: string;
  cliente_id?: number | null;
  cliente_nombre?: string;
  cliente_telefono?: string;
  tipo_equipo?: string;
  equipo_descripcion: string;
  equipo_marca?: string;
  equipo_modelo?: string;
  equipo_serie?: string;
  equipo_placa?: string;
  equipo_kilometraje?: number;
  equipo_kilometraje_proximo?: number;
  accesorios?: string;
  problema_reportado: string;
  diagnostico?: string;
  trabajo_realizado?: string;
  observaciones?: string;
  tecnico_id?: number | null;
  tecnico_nombre?: string;
  estado?: string;
  fecha_ingreso?: string;
  fecha_promesa?: string;
  fecha_entrega?: string;
  presupuesto?: number;
  monto_final?: number;
  garantia_dias?: number;
  venta_id?: number | null;
  usuario_creador?: string;
}

export const crearOrdenServicio = (orden: OrdenServicio) =>
  smartInvoke<number>("crear_orden_servicio", { orden });
export const actualizarOrdenServicio = (orden: OrdenServicio) =>
  smartInvoke<void>("actualizar_orden_servicio", { orden });
export const cambiarEstadoOrden = (ordenId: number, nuevoEstado: string, observacion?: string) =>
  smartInvoke<void>("cambiar_estado_orden", { ordenId, nuevoEstado, observacion: observacion ?? null });
export const obtenerOrdenServicio = (id: number) =>
  smartInvoke<OrdenServicio>("obtener_orden_servicio", { id });
export const listarOrdenesServicio = (filtroEstado?: string, fechaDesde?: string, fechaHasta?: string, tecnicoId?: number) =>
  smartInvoke<OrdenServicio[]>("listar_ordenes_servicio", { filtroEstado: filtroEstado ?? null, fechaDesde: fechaDesde ?? null, fechaHasta: fechaHasta ?? null, tecnicoId: tecnicoId ?? null });
export const buscarOrdenesPorEquipo = (query: string) =>
  smartInvoke<OrdenServicio[]>("buscar_ordenes_por_equipo", { query });
export const historialMovimientosOrden = (ordenId: number) =>
  smartInvoke<any[]>("historial_movimientos_orden", { ordenId });
export const eliminarOrdenServicio = (id: number) =>
  smartInvoke<void>("eliminar_orden_servicio", { id });
export const agregarImagenOrden = (ordenId: number, tipo: string, imagenBase64: string, descripcion?: string) =>
  smartInvoke<number>("agregar_imagen_orden", { ordenId, tipo, imagenBase64, descripcion: descripcion ?? null });
export const listarImagenesOrden = (ordenId: number) =>
  smartInvoke<any[]>("listar_imagenes_orden", { ordenId });
export const eliminarImagenOrden = (imagenId: number) =>
  smartInvoke<void>("eliminar_imagen_orden", { imagenId });
export const cobrarOrdenServicio = (ordenId: number, formaPago: string, montoRecibido: number, itemsRepuestos: any[]) =>
  smartInvoke<number>("cobrar_orden_servicio", { ordenId, formaPago, montoRecibido, itemsRepuestos });
export const imprimirOrdenServicioPdf = (ordenId: number) =>
  smartInvoke<string>("imprimir_orden_servicio_pdf", { ordenId });

// === Resumen detallado de caja (abierta o cerrada) ===
// Retorna ResumenCajaReporte con desglose por forma de pago, lista de gastos,
// retiros con motivo, ventas detalladas. Usado en CajaPage para que el cajero
// entienda de donde sale el monto esperado (gastos, retiros restan; ventas
// efectivo suman).
export const obtenerResumenCaja = (cajaId: number) =>
  smartInvoke<any>("obtener_resumen_caja", { cajaId });

// === Verificacion de transferencias (admin) ===
export const listarTransferenciasVerificacion = (soloPendientes?: boolean, fechaDesde?: string, fechaHasta?: string) =>
  smartInvoke<any[]>("listar_transferencias_verificacion", {
    soloPendientes: soloPendientes ?? null,
    fechaDesde: fechaDesde ?? null,
    fechaHasta: fechaHasta ?? null,
  });
export const verificarTransferencia = (origen: string, origenId: number, aprobar: boolean, motivo?: string) =>
  smartInvoke<void>("verificar_transferencia", { origen, origenId, aprobar, motivo: motivo ?? null });
export const contarTransferenciasPendientes = () =>
  smartInvoke<number>("contar_transferencias_pendientes");

// === Detalle expandido de un movimiento bancario ===
export const obtenerDetalleMovimientoBancario = (tipo: string, origenId: number) =>
  smartInvoke<any>("obtener_detalle_movimiento_bancario", { tipo, origenId });

