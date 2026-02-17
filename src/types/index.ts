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
}

export interface ProductoBusqueda {
  id: number;
  codigo?: string;
  nombre: string;
  precio_venta: number;
  iva_porcentaje: number;
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
}

// Usuarios / Sesión
export interface UsuarioInfo {
  id: number;
  nombre: string;
  rol: string;
  activo: boolean;
}

export interface SesionActiva {
  usuario_id: number;
  nombre: string;
  rol: string;
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

// Item del carrito (para la pantalla de venta)
export interface ItemCarrito {
  producto_id: number;
  codigo?: string;
  nombre: string;
  cantidad: number;
  precio_unitario: number;
  descuento: number;
  iva_porcentaje: number;
  subtotal: number;
  stock_disponible: number;
  stock_minimo: number;
  precio_base: number;
  precios_disponibles?: PrecioProductoDetalle[];
  lista_seleccionada?: string;
}
