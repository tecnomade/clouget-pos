/**
 * API wrappers del módulo Restaurante.
 * Pasa por `smartInvoke` para que en modo cliente (Multi-POS) las llamadas
 * vayan al servidor remoto vía HTTP en lugar de Tauri local.
 */

import { smartInvoke as invoke } from "../services/api";
import type {
  EstadoCocina,
  ItemCocina,
  Mesa,
  MesaConEstado,
  PedidoAbierto,
  PedidoDetalle,
  Zona,
} from "./types";

// ─── Zonas ───────────────────────────────────────────────────────────────

export const listarZonas = () => invoke<Zona[]>("rest_listar_zonas");

export const crearZona = (zona: Omit<Zona, "id">) =>
  invoke<number>("rest_crear_zona", { zona });

export const actualizarZona = (zona: Zona) =>
  invoke<void>("rest_actualizar_zona", { zona });

export const eliminarZona = (id: number) =>
  invoke<void>("rest_eliminar_zona", { id });

// ─── Mesas (configuración) ───────────────────────────────────────────────

export const crearMesa = (mesa: Omit<Mesa, "id">) =>
  invoke<number>("rest_crear_mesa", { mesa });

export const actualizarMesa = (mesa: Mesa) =>
  invoke<void>("rest_actualizar_mesa", { mesa });

export const eliminarMesa = (id: number) =>
  invoke<void>("rest_eliminar_mesa", { id });

// ─── Mesas (operación: grid principal) ───────────────────────────────────

export const listarMesasConEstado = () =>
  invoke<MesaConEstado[]>("rest_listar_mesas_con_estado");

// ─── Pedidos ─────────────────────────────────────────────────────────────

export const abrirPedido = (args: {
  mesaId: number;
  meseroId?: number | null;
  meseroNombre?: string | null;
  comensales?: number;
}) =>
  invoke<number>("rest_abrir_pedido", {
    mesaId: args.mesaId,
    meseroId: args.meseroId ?? null,
    meseroNombre: args.meseroNombre ?? null,
    comensales: args.comensales ?? 1,
  });

export const obtenerPedido = (id: number) =>
  invoke<PedidoDetalle>("rest_obtener_pedido", { id });

export const obtenerPedidoMesa = (mesaId: number) =>
  invoke<PedidoDetalle | null>("rest_obtener_pedido_mesa", { mesaId });

export const listarPedidosAbiertos = () =>
  invoke<PedidoAbierto[]>("rest_listar_pedidos_abiertos");

export const cancelarPedido = (id: number) =>
  invoke<void>("rest_cancelar_pedido", { id });

// ─── Items ───────────────────────────────────────────────────────────────

export const agregarItem = (args: {
  pedidoId: number;
  productoId: number;
  cantidad: number;
  infoAdicional?: string | null;
}) =>
  invoke<number>("rest_agregar_item", {
    pedidoId: args.pedidoId,
    productoId: args.productoId,
    cantidad: args.cantidad,
    infoAdicional: args.infoAdicional ?? null,
  });

export const actualizarItemCantidad = (itemId: number, cantidad: number) =>
  invoke<void>("rest_actualizar_item_cantidad", { itemId, cantidad });

export const eliminarItem = (itemId: number) =>
  invoke<void>("rest_eliminar_item", { itemId });

// ─── Cocina ──────────────────────────────────────────────────────────────

export const enviarCocina = (pedidoId: number) =>
  invoke<import("./types").PedidoItem[]>("rest_enviar_cocina", { pedidoId });

export const listarItemsCocinaPendientes = () =>
  invoke<ItemCocina[]>("rest_listar_items_cocina_pendientes");

export const marcarItemCocina = (itemId: number, estado: EstadoCocina) =>
  invoke<void>("rest_marcar_item_cocina", { itemId, estado });

// ─── Cuenta y cobro ──────────────────────────────────────────────────────

export const pedirCuenta = (pedidoId: number) =>
  invoke<void>("rest_pedir_cuenta", { pedidoId });

export const cerrarPedido = (pedidoId: number, ventaId: number) =>
  invoke<void>("rest_cerrar_pedido", { pedidoId, ventaId });

// ─── Impresión ───────────────────────────────────────────────────────────

/** Imprime el ticket de pre-cuenta (cortesía, no fiscal) en la térmica configurada. */
export const imprimirPreCuenta = (pedidoId: number) =>
  invoke<string>("rest_imprimir_pre_cuenta", { pedidoId });
