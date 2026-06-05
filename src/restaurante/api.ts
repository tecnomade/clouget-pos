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
  MesaResumen,
  PedidoAbierto,
  PedidoDetalle,
  ResultadoCobroSubcuenta,
  Subcuenta,
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

/** v2.5.91 — Registra un abono (pago parcial) sobre una mesa. Devuelve el detalle actualizado. */
export const registrarAbono = (args: {
  pedidoId: number; monto: number; formaPago: string; bancoId?: number | null; referenciaPago?: string | null;
}) =>
  invoke<PedidoDetalle>("rest_registrar_abono", {
    pedidoId: args.pedidoId,
    monto: args.monto,
    formaPago: args.formaPago,
    bancoId: args.bancoId ?? null,
    referenciaPago: args.referenciaPago ?? null,
  });

// ─── Impresión ───────────────────────────────────────────────────────────

/** Imprime el ticket de pre-cuenta (cortesía, no fiscal) en la térmica configurada. */
export const imprimirPreCuenta = (pedidoId: number) =>
  invoke<string>("rest_imprimir_pre_cuenta", { pedidoId });

/** v2.3.67: Imprime la comanda de cocina al enviar items.
 *  itemsIds opcional: si vienen, solo esos items; si null, todos los del pedido (re-imprimir).
 *  Usa `impresora_cocina` o fallback a `impresora` principal. Soporta 1 o 2 tickets
 *  separados (cocina/barra) según `comanda_modo_separado`. */
export const imprimirComandaCocina = (pedidoId: number, itemsIds?: number[]) =>
  invoke<string>("rest_imprimir_comanda_cocina", { pedidoId, itemsIds: itemsIds ?? null });

// ─── Unir mesas (v2.3.68) ────────────────────────────────────────────────

/** Une una o varias mesas LIBRES al pedido (grupos grandes que ocupan varias mesas) */
export const unirMesas = (pedidoId: number, mesasIds: number[]) =>
  invoke<void>("rest_unir_mesas", { pedidoId, mesasIds });

/** Desune una mesa EXTRA del pedido (la libera). NO sirve para la mesa principal. */
export const desunirMesa = (pedidoId: number, mesaId: number) =>
  invoke<void>("rest_desunir_mesa", { pedidoId, mesaId });

/** Lista mesas LIBRES disponibles para unir al pedido. */
export const listarMesasLibresParaUnir = (pedidoId: number) =>
  invoke<MesaResumen[]>("rest_listar_mesas_libres_para_unir", { pedidoId });

// ─── Dividir cuenta (v2.3.69) ────────────────────────────────────────────

/** Divide el pedido en N partes iguales. Falla si ya está dividido. */
export const dividirCuenta = (pedidoId: number, nPartes: number) =>
  invoke<Subcuenta[]>("rest_dividir_cuenta", { pedidoId, nPartes });

/** Lista las sub-cuentas del pedido (vacío si no está dividido). */
export const listarSubcuentas = (pedidoId: number) =>
  invoke<Subcuenta[]>("rest_listar_subcuentas", { pedidoId });

/** Cancela la división (solo si NINGUNA sub-cuenta fue cobrada). */
export const cancelarDivision = (pedidoId: number) =>
  invoke<void>("rest_cancelar_division", { pedidoId });

/** Marca una sub-cuenta como cobrada — vincula con la venta ya generada
 *  por el frontend mediante registrarVenta(). Si todas quedaron cobradas,
 *  cierra el pedido y libera mesas automáticamente. */
export const marcarSubcuentaCobrada = (
  subcuentaId: number,
  ventaId: number,
  formaPago: string,
  bancoId?: number | null,
  referenciaPago?: string | null,
) =>
  invoke<ResultadoCobroSubcuenta>("rest_marcar_subcuenta_cobrada", {
    subcuentaId,
    ventaId,
    formaPago,
    bancoId: bancoId ?? null,
    referenciaPago: referenciaPago ?? null,
  });

/** ID del producto especial _DIVISION_CUENTA_ usado al cobrar sub-cuentas. */
export const productoDivisionId = () =>
  invoke<number>("rest_producto_division_id");
