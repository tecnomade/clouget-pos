/**
 * Types del módulo Restaurante (frontend).
 * Mirror de `src-tauri/src/restaurante/models.rs`.
 *
 * Si cambias estos tipos, actualiza también el lado Rust.
 */

export interface Zona {
  id?: number;
  nombre: string;
  color: string;
  orden: number;
  activa: boolean;
}

export interface Mesa {
  id?: number;
  zona_id?: number | null;
  nombre: string;
  capacidad: number;
  orden: number;
  activa: boolean;
}

export interface MesaConEstado {
  id: number;
  zona_id: number | null;
  zona_nombre: string | null;
  zona_color: string | null;
  nombre: string;
  capacidad: number;
  orden: number;
  /** LIBRE | OCUPADA | CUENTA_PEDIDA */
  estado: "LIBRE" | "OCUPADA" | "CUENTA_PEDIDA";
  pedido_id: number | null;
  mesero_nombre: string | null;
  comensales: number | null;
  total_actual: number;
  /** v2.5.92 — abonos en holding sobre la mesa. >0 = "Cuenta parcial" */
  total_abonado?: number;
  items_pendientes_cocina: number;
  fecha_apertura: string | null;
  minutos_abierta: number | null;
  // v2.3.68 — Unir mesas
  /** Si != null, esta mesa es EXTRA del grupo y la principal es esta mesa */
  mesa_principal_id: number | null;
  mesa_principal_nombre: string | null;
  /** Si esta mesa es la principal, cantidad de mesas extra unidas (0 si no hay) */
  mesas_unidas_count: number;
}

export interface PedidoAbierto {
  id?: number;
  mesa_id: number;
  mesero_id?: number | null;
  mesero_nombre?: string | null;
  comensales: number;
  /** ABIERTO | CUENTA_PEDIDA | COBRADO | CANCELADO */
  estado: string;
  observacion?: string | null;
  fecha_apertura?: string | null;
  fecha_cuenta?: string | null;
  fecha_cierre?: string | null;
  venta_id?: number | null;
}

export interface PedidoItem {
  id?: number;
  pedido_id: number;
  producto_id: number;
  producto_nombre?: string | null;
  cantidad: number;
  precio_unit: number;
  info_adicional?: string | null;
  enviado_cocina: boolean;
  /** PENDIENTE | EN_PREPARACION | LISTO | ENTREGADO */
  estado_cocina: string;
  fecha_creacion?: string | null;
  fecha_envio_cocina?: string | null;
  /** COCINA | BARRA | DIRECTO — del producto, JOIN al consultar */
  destino_preparacion?: string;
}

export interface MesaResumen {
  id: number;
  nombre: string;
  capacidad: number;
  zona_nombre: string | null;
}

export interface PedidoDetalle {
  pedido: PedidoAbierto;
  items: PedidoItem[];
  mesa_nombre: string;
  zona_nombre: string | null;
  subtotal: number;
  iva: number;
  total: number;
  /** v2.3.68 — Mesas EXTRA unidas a este pedido (NO incluye la principal) */
  mesas_extra: MesaResumen[];
  /** v2.3.68 — Capacidad total efectiva (principal + extras) */
  capacidad_total: number;
  /** v2.5.91 — Pagos parciales (abonos) ya recibidos sobre la mesa */
  total_abonado?: number;
  /** v2.5.91 — Saldo pendiente = total − total_abonado */
  saldo?: number;
  /** v2.5.91 — Historial de abonos */
  abonos?: AbonoPedido[];
}

export interface AbonoPedido {
  id: number;
  monto: number;
  forma_pago: string;
  banco_id?: number | null;
  banco_nombre?: string | null;
  referencia_pago?: string | null;
  estado: string;
  fecha: string;
  usuario_nombre?: string | null;
}

export interface ItemCocina {
  id: number;
  pedido_id: number;
  mesa_nombre: string;
  zona_nombre: string | null;
  mesero_nombre: string | null;
  producto_nombre: string;
  cantidad: number;
  info_adicional: string | null;
  estado_cocina: string;
  fecha_envio_cocina: string | null;
  minutos_en_cocina: number | null;
}

export type EstadoCocina = "PENDIENTE" | "EN_PREPARACION" | "LISTO" | "ENTREGADO";

// ─── v2.3.69 — Sub-cuentas (división de cuenta) ──────────────────────────

export interface Subcuenta {
  id: number;
  pedido_id: number;
  numero: number;
  total: number;
  /** PENDIENTE | COBRADA */
  estado: "PENDIENTE" | "COBRADA";
  forma_pago: string | null;
  banco_id: number | null;
  banco_nombre: string | null;
  referencia_pago: string | null;
  venta_id: number | null;
  venta_numero: string | null;
  fecha_cobro: string | null;
}

export interface ResultadoCobroSubcuenta {
  todas_cobradas: boolean;
  pendientes: number;
}
