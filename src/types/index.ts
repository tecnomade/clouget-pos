// Producto
export interface Producto {
  id?: number;
  codigo?: string;
  codigo_barras?: string;
  nombre: string;
  descripcion?: string;
  categoria_id?: number;
  precio_costo: number;
  precio_venta: number;
  iva_porcentaje: number;
  incluye_iva: boolean;
  stock_actual: number;
  stock_minimo: number;
  unidad_medida: string;
  es_servicio: boolean;
  activo: boolean;
  imagen?: string;
  requiere_serie?: boolean;
  requiere_caducidad?: boolean;
  no_controla_stock?: boolean;
}

export interface ProductoTactil {
  id: number;
  nombre: string;
  precio_venta: number;
  iva_porcentaje: number;
  incluye_iva?: boolean;
  stock_actual: number;
  categoria_id?: number;
  categoria_nombre?: string;
  imagen?: string;
}

export interface ProductoBusqueda {
  id: number;
  codigo?: string;
  codigo_barras?: string;
  nombre: string;
  precio_venta: number;
  iva_porcentaje: number;
  incluye_iva?: boolean;
  stock_actual: number;
  stock_minimo: number;
  categoria_nombre?: string;
  precio_lista?: number;
}

export interface Categoria {
  id?: number;
  nombre: string;
  descripcion?: string;
  activo: boolean;
}

// Cliente
export interface Cliente {
  id?: number;
  tipo_identificacion: string;
  identificacion?: string;
  nombre: string;
  direccion?: string;
  telefono?: string;
  email?: string;
  activo: boolean;
  lista_precio_id?: number;
  lista_precio_nombre?: string;
}

// Listas de Precios
export interface ListaPrecio {
  id?: number;
  nombre: string;
  descripcion?: string;
  es_default: boolean;
  activo: boolean;
}

export interface PrecioProducto {
  lista_precio_id: number;
  producto_id: number;
  precio: number;
}

export interface PrecioProductoDetalle {
  lista_precio_id: number;
  lista_nombre: string;
  precio: number;
}

// Venta
export interface Venta {
  id?: number;
  numero: string;
  cliente_id?: number;
  fecha?: string;
  subtotal_sin_iva: number;
  subtotal_con_iva: number;
  descuento: number;
  iva: number;
  total: number;
  forma_pago: string;
  monto_recibido: number;
  cambio: number;
  estado: string;
  tipo_documento: string;
  estado_sri: string;
  autorizacion_sri?: string;
  clave_acceso?: string;
  observacion?: string;
  numero_factura?: string;
  email_enviado?: number;
  establecimiento?: string;
  punto_emision?: string;
  banco_id?: number;
  referencia_pago?: string;
  banco_nombre?: string;
  tipo_estado?: string;
  anulada?: number;
  guia_placa?: string;
  guia_chofer?: string;
  guia_direccion_destino?: string;
}

export interface DocumentoReciente {
  id: number;
  numero: string;
  tipo_estado: string;
  tipo_documento: string;
  cliente_nombre?: string;
  total: number;
  fecha: string;
}

export interface ResumenGuias {
  abiertas: number;
  cerradas: number;
  total_pendiente: number;
  total_cerrado: number;
}

// Reportes avanzados
export interface ReporteUtilidad {
  ventas_brutas: number; costo_ventas: number; utilidad_bruta: number; margen_bruto: number;
  total_gastos: number; utilidad_neta: number; margen_neto: number; num_ventas: number;
  promedio_por_venta: number; total_devoluciones: number;
  por_categoria: { categoria: string; ventas: number; costo: number; utilidad: number }[];
  gastos_por_categoria: { categoria: string; monto: number }[];
}
export interface ReporteBalance {
  ingresos_efectivo: number; ingresos_transferencia: number; ingresos_credito_cobrado: number;
  total_ingresos: number; gastos_por_categoria: { categoria: string; monto: number }[];
  total_gastos: number; total_devoluciones: number; total_egresos: number; resultado: number;
  cuentas_por_cobrar: number; valor_inventario: number;
}
export interface ProductoRentabilidad {
  nombre: string; categoria: string; cantidad: number; ingreso: number; costo: number; utilidad: number; margen: number;
}

export interface VentaDetalle {
  id?: number;
  venta_id?: number;
  producto_id: number;
  nombre_producto?: string;
  cantidad: number;
  precio_unitario: number;
  descuento: number;
  iva_porcentaje: number;
  subtotal: number;
  info_adicional?: string | null;
  unidad_id?: number | null;
  unidad_nombre?: string | null;
  factor_unidad?: number | null;
}

export interface PagoMixto {
  forma_pago: string;
  monto: number;
  banco_id?: number | null;
  referencia?: string | null;
  comprobante_imagen?: string | null;
}

export interface NuevaVenta {
  cliente_id?: number;
  items: VentaDetalle[];
  forma_pago: string;
  monto_recibido: number;
  descuento: number;
  tipo_documento: string;
  observacion?: string;
  es_fiado: boolean;
  banco_id?: number | null;
  referencia_pago?: string | null;
  comprobante_imagen?: string | null;
  guia_placa?: string | null;
  guia_chofer?: string | null;
  guia_direccion_destino?: string | null;
  /** Si presente y no vacio: pagos multiples (la suma debe igualar el total) */
  pagos?: PagoMixto[];
}

export interface VentaCompleta {
  venta: Venta;
  detalles: VentaDetalle[];
  cliente_nombre?: string;
}

// Caja
export interface Caja {
  id?: number;
  fecha_apertura?: string;
  fecha_cierre?: string;
  monto_inicial: number;
  monto_ventas: number;
  monto_esperado: number;
  monto_real?: number;
  diferencia?: number;
  estado: string;
  usuario?: string;
  usuario_id?: number;
  observacion?: string;
}

export interface ResumenCaja {
  caja: Caja;
  total_ventas: number;
  num_ventas: number;
  total_efectivo: number;
  total_gastos: number;
  total_cobros_efectivo: number;
  total_cobros_banco: number;
  total_retiros: number;
}

// Gasto
export interface Gasto {
  id?: number;
  descripcion: string;
  monto: number;
  categoria?: string;
  fecha?: string;
  caja_id?: number;
  observacion?: string;
  es_recurrente?: boolean;
}

// Cuentas por Cobrar
export interface CuentaPorCobrar {
  id?: number;
  cliente_id: number;
  venta_id: number;
  monto_total: number;
  monto_pagado: number;
  saldo: number;
  estado: string;
  fecha_vencimiento?: string;
  created_at?: string;
}

export interface PagoCuenta {
  id?: number;
  cuenta_id: number;
  monto: number;
  fecha?: string;
  observacion?: string;
  forma_pago: string;
  banco_id?: number;
  numero_comprobante?: string;
  comprobante_imagen?: string;
  banco_nombre?: string;
  estado?: string;
  confirmado_por?: number;
  fecha_confirmacion?: string;
}

export interface CuentaBanco {
  id?: number;
  nombre: string;
  tipo_cuenta?: string;
  numero_cuenta?: string;
  titular?: string;
  activa: boolean;
}

export interface CuentaConCliente {
  cuenta: CuentaPorCobrar;
  cliente_nombre: string;
  venta_numero: string;
}

export interface ResumenCliente {
  cliente_id: number;
  cliente_nombre: string;
  total_deuda: number;
  num_cuentas: number;
}

export interface CuentaDetalle {
  cuenta: CuentaPorCobrar;
  cliente_nombre: string;
  venta_numero: string;
  pagos: PagoCuenta[];
}

// Licencia (validación online via Supabase)
export interface LicenciaInfo {
  negocio: string;
  email: string;
  tipo: string;       // "perpetua", "anual"
  emitida: string;    // fecha ISO
  machine_id: string;
  activa: boolean;
  modulos: string[];  // ["multi_pos", "multi_almacen", "backup_cloud"]
}

// Usuarios / Sesión
export interface UsuarioInfo {
  id: number;
  nombre: string;
  rol: string;
  activo: boolean;
  permisos: string;
}

export interface SesionActiva {
  usuario_id: number;
  nombre: string;
  rol: string;
  permisos: string;
}

export interface NuevoUsuario {
  nombre: string;
  pin: string;
  rol: string;
}

// SRI - Facturacion Electronica
export interface ResultadoEmision {
  exito: boolean;
  estado_sri: string;
  clave_acceso?: string;
  numero_autorizacion?: string;
  fecha_autorizacion?: string;
  mensaje: string;
  numero_factura?: string;
}

export interface EstadoSri {
  modulo_activo: boolean;
  certificado_cargado: boolean;
  ambiente: string;
  facturas_usadas: number;
  facturas_gratis: number;
  // Suscripcion online
  suscripcion_autorizada: boolean;
  suscripcion_plan: string;
  suscripcion_hasta: string;
  suscripcion_docs_restantes: number | null;
  suscripcion_es_lifetime: boolean;
  suscripcion_mensaje: string;
}

// Planes SRI - Contratación
export interface PlanSri {
  clave: string;
  nombre: string;
  precio: number;
  descripcion: string;
  tipo: string;           // "tiempo", "paquete", "lifetime"
  duracion_meses: number | null;
  docs_cantidad: number | null;
  ahorro: string | null;
  popular: boolean;
  orden: number;
}

export interface ConfigContratacion {
  whatsapp_numero: string;
  banco_nombre: string;
  banco_tipo_cuenta: string;
  banco_numero_cuenta: string;
  banco_titular: string;
  banco_cedula_ruc: string;
  mensaje_transferencia: string;
}

export interface PlanesDisponibles {
  ok: boolean;
  planes: PlanSri[];
  config: ConfigContratacion;
}

export interface PedidoCreado {
  ok: boolean;
  pedido_id: string | null;
  referencia: string | null;
  mensaje: string;
  ya_existia: boolean;
}

// Notas de Crédito
export interface NuevaNotaCredito {
  venta_id: number;
  motivo: string;
  items: VentaDetalle[];
}

export interface NotaCreditoInfo {
  id: number;
  numero: string;
  venta_id: number;
  factura_numero: string;
  motivo: string;
  total: number;
  fecha: string;
  estado_sri: string;
  autorizacion_sri?: string;
  clave_acceso?: string;
  numero_factura_nc?: string;
}

// Establecimientos y Puntos de Emisión
export interface Establecimiento {
  id?: number;
  codigo: string;
  nombre: string;
  direccion?: string;
  telefono?: string;
  es_propio: boolean;
  activo: boolean;
}

export interface PuntoEmision {
  id?: number;
  establecimiento_id: number;
  codigo: string;
  nombre?: string;
  activo: boolean;
}

// Transferencias y Stock Multi-almacén
export interface TransferenciaStock {
  id?: number;
  producto_id: number;
  producto_nombre?: string;
  origen_establecimiento_id: number;
  origen_nombre?: string;
  destino_establecimiento_id: number;
  destino_nombre?: string;
  cantidad: number;
  estado: string;
  usuario?: string;
  created_at?: string;
  recibida_at?: string;
}

export interface StockEstablecimiento {
  establecimiento_id: number;
  establecimiento_nombre: string;
  establecimiento_codigo: string;
  stock_actual: number;
  stock_minimo: number;
}

// Proveedor
export interface Proveedor {
  id?: number;
  ruc?: string;
  nombre: string;
  direccion?: string;
  telefono?: string;
  email?: string;
  contacto?: string;
  dias_credito?: number;
  activo: boolean;
}

// Compras
export interface Compra {
  id?: number;
  numero: string;
  proveedor_id: number;
  fecha?: string;
  numero_factura?: string;
  subtotal: number;
  iva: number;
  total: number;
  estado: string;
  forma_pago: string;
  es_credito: boolean;
  observacion?: string;
  proveedor_nombre?: string;
}

export interface CompraDetalle {
  id?: number;
  compra_id?: number;
  producto_id?: number;
  descripcion?: string;
  cantidad: number;
  precio_unitario: number;
  subtotal: number;
  nombre_producto?: string;
}

export interface CompraCompleta {
  compra: Compra;
  detalles: CompraDetalle[];
}

export interface ItemCompra {
  producto_id?: number;
  descripcion?: string;
  cantidad: number;
  precio_unitario: number;
  iva_porcentaje: number;
}

export interface NuevaCompra {
  proveedor_id: number;
  numero_factura?: string;
  items: ItemCompra[];
  forma_pago: string;
  es_credito: boolean;
  observacion?: string;
  dias_credito?: number;
}

// Cuentas por Pagar
export interface CuentaPorPagar {
  id?: number;
  proveedor_id: number;
  compra_id?: number;
  monto_total: number;
  monto_pagado: number;
  saldo: number;
  estado: string;
  fecha_vencimiento?: string;
  observacion?: string;
  created_at?: string;
  proveedor_nombre?: string;
  compra_numero?: string;
}

export interface PagoProveedor {
  id?: number;
  cuenta_id: number;
  monto: number;
  fecha?: string;
  forma_pago: string;
  numero_comprobante?: string;
  observacion?: string;
  banco_id?: number;
  banco_nombre?: string;
}

export interface ResumenAcreedor {
  proveedor_id: number;
  proveedor_nombre: string;
  total_deuda: number;
  num_cuentas: number;
}

// Item del carrito (para la pantalla de venta)
export interface ItemCarrito {
  producto_id: number;
  codigo?: string;
  nombre: string;
  cantidad: number;
  precio_unitario: number;
  descuento: number;
  iva_porcentaje: number;
  incluye_iva?: boolean;
  subtotal: number;
  stock_disponible: number;
  stock_minimo: number;
  precio_base: number;
  precios_disponibles?: PrecioProductoDetalle[];
  lista_seleccionada?: string;
  info_adicional?: string;
  // Multi-unidad: presentacion seleccionada
  unidad_id?: number;
  unidad_nombre?: string;
  factor_unidad?: number;
}
