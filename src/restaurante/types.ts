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
  items_pendientes_cocina: number;
  fecha_apertura: string | null;
  minutos_abierta: number | null;
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

export interface PedidoDetalle {
  pedido: PedidoAbierto;
  items: PedidoItem[];
  mesa_nombre: string;
  zona_nombre: string | null;
  subtotal: number;
  iva: number;
  total: number;
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
