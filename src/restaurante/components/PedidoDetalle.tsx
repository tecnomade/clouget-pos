/**
 * PedidoDetalle — Drawer lateral con el detalle del pedido activo de una mesa.
 *
 * Acciones:
 *   - Agregar productos (abre SelectorProductos)
 *   - Eliminar item (solo si NO enviado a cocina)
 *   - Enviar a cocina (marca items pendientes + abre ticket cocina)
 *   - Pedir cuenta (notifica caja)
 *   - Cobrar (delega a registrarVenta + cierra pedido + libera mesa)
 *   - Cancelar pedido (libera mesa sin venta)
 */

import { useState, useEffect, useCallback } from "react";
import { useToast } from "../../components/Toast";
import {
  obtenerPedido,
  agregarItem,
  eliminarItem,
  enviarCocina,
  pedirCuenta,
  cancelarPedido,
  cerrarPedido,
  imprimirPreCuenta,
} from "../api";
import { registrarVenta, obtenerCajaAbierta, listarCuentasBanco, obtenerConfig } from "../../services/api";
import type { PedidoDetalle as PedidoDetalleType } from "../types";
import type { NuevaVenta, VentaDetalle, CuentaBanco } from "../../types";
import SelectorProductos from "./SelectorProductos";

interface Props {
  pedidoId: number;
  /** Cerrar drawer. recargar=true para que MesasPage refresque grid. */
  onCerrar: (recargar: boolean) => void;
}

type ModoCobro = null | "elegir-pago";

export default function PedidoDetalle({ pedidoId, onCerrar }: Props) {
  const { toastExito, toastError, toastWarning } = useToast();
  const [detalle, setDetalle] = useState<PedidoDetalleType | null>(null);
  const [cargando, setCargando] = useState(true);
  const [mostrarSelector, setMostrarSelector] = useState(false);
  const [modoCobro, setModoCobro] = useState<ModoCobro>(null);
  const [confirmCancelar, setConfirmCancelar] = useState(false);

  const cargar = useCallback(async () => {
    try {
      const d = await obtenerPedido(pedidoId);
      setDetalle(d);
    } catch (err: any) {
      toastError("Error cargando pedido: " + (err?.message || err));
      onCerrar(true);
    } finally {
      setCargando(false);
    }
  }, [pedidoId, toastError, onCerrar]);

  useEffect(() => {
    cargar();
  }, [cargar]);

  // ─── Acciones ──────────────────────────────────────────────────────────

  const handleAgregarItem = async (producto: { id: number; nombre: string }, infoAdicional: string | null) => {
    try {
      await agregarItem({
        pedidoId,
        productoId: producto.id,
        cantidad: 1,
        infoAdicional,
      });
      toastExito(`+ ${producto.nombre}`);
      await cargar();
    } catch (err: any) {
      toastError("No se pudo agregar: " + (err?.message || err));
    }
  };

  const handleEliminarItem = async (itemId: number, nombre: string) => {
    try {
      await eliminarItem(itemId);
      toastExito(`Quitado: ${nombre}`);
      await cargar();
    } catch (err: any) {
      toastError(err?.message || String(err));
    }
  };

  const handleEnviarCocina = async () => {
    try {
      const items = await enviarCocina(pedidoId);
      toastExito(`${items.length} item(s) enviado(s) a cocina`);
      // TODO Fase 3: imprimir ticket cocina con `items`
      await cargar();
    } catch (err: any) {
      toastError(err?.message || String(err));
    }
  };

  const handlePedirCuenta = async () => {
    try {
      await pedirCuenta(pedidoId);
      // Auto-imprimir pre-cuenta. Si no hay impresora configurada, mostrar
      // warning pero NO romper el flujo (el estado ya cambió a CUENTA_PEDIDA).
      try {
        await imprimirPreCuenta(pedidoId);
        toastExito("Cuenta solicitada · Pre-cuenta impresa 🖨");
      } catch (err: any) {
        toastWarning(
          "Cuenta marcada, pero no se pudo imprimir: " +
            (err?.message || err) +
            ". Verifica impresora en Configuración.",
        );
      }
      await cargar();
    } catch (err: any) {
      toastError(err?.message || String(err));
    }
  };

  const handleReimprimirPreCuenta = async () => {
    try {
      await imprimirPreCuenta(pedidoId);
      toastExito("Pre-cuenta reimpresa 🖨");
    } catch (err: any) {
      toastError(err?.message || String(err));
    }
  };

  const handleCancelarPedido = async () => {
    try {
      await cancelarPedido(pedidoId);
      toastExito("Pedido cancelado");
      onCerrar(true);
    } catch (err: any) {
      toastError(err?.message || String(err));
    }
  };

  const handleCobrar = async (
    formaPago: string,
    esFiado: boolean,
    extras?: { bancoId?: number | null; referencia?: string | null },
  ) => {
    if (!detalle) return;
    if (detalle.items.length === 0) {
      toastWarning("No hay items para cobrar");
      return;
    }

    // Validar caja abierta — usa la MISMA caja del POS normal (no hay caja
    // separada para restaurante; el cobro se registra como una venta normal).
    try {
      const caja = await obtenerCajaAbierta();
      if (!caja) {
        toastError("Debes abrir una caja antes de cobrar");
        return;
      }
    } catch {
      toastError("Error verificando caja");
      return;
    }

    // Construir payload de Venta delegando a registrar_venta (que maneja
    // combos, SRI, secuenciales, IVA, kardex, banco/referencia correctamente).
    const items: VentaDetalle[] = detalle.items.map((i) => ({
      producto_id: i.producto_id,
      cantidad: i.cantidad,
      precio_unitario: i.precio_unit,
      descuento: 0,
      iva_porcentaje: 0, // registrar_venta lo calcula del producto si es 0
      subtotal: i.cantidad * i.precio_unit,
      info_adicional: i.info_adicional ?? null,
    }));

    const total = items.reduce((s, i) => s + i.subtotal, 0);

    const payload: NuevaVenta = {
      items,
      forma_pago: formaPago,
      monto_recibido: esFiado ? 0 : total,
      descuento: 0,
      tipo_documento: "NOTA_VENTA",
      es_fiado: esFiado,
      observacion: `Mesa: ${detalle.mesa_nombre}${detalle.zona_nombre ? ` (${detalle.zona_nombre})` : ""} · Pedido #${pedidoId}`,
      // Transferencia: pasar banco + referencia para que aparezca en
      // /movimientos-bancarios y /verificacion (panel admin) — mismo flujo POS.
      banco_id: extras?.bancoId ?? null,
      referencia_pago: extras?.referencia?.trim() || null,
    };

    try {
      const resultado = await registrarVenta(payload);
      // Vincular venta con pedido y liberar mesa
      await cerrarPedido(pedidoId, resultado.venta.id!);
      toastExito(`Venta ${resultado.venta.numero} registrada · Mesa liberada`);
      onCerrar(true);
    } catch (err: any) {
      toastError("No se pudo cobrar: " + (err?.message || err));
    }
  };

  // ─── Render ────────────────────────────────────────────────────────────

  if (cargando || !detalle) {
    return (
      <div className="modal-overlay" onClick={() => onCerrar(false)}>
        <div className="modal-content" style={{ padding: 32 }}>
          Cargando...
        </div>
      </div>
    );
  }

  const itemsAgrupados = agruparItems(detalle.items);
  const itemsNuevos = detalle.items.filter((i) => !i.enviado_cocina).length;

  return (
    <>
      {/* Drawer lateral */}
      <div
        className="modal-overlay"
        onClick={() => onCerrar(false)}
        style={{ alignItems: "stretch", justifyContent: "flex-end" }}
      >
        <div
          onClick={(e) => e.stopPropagation()}
          style={{
            background: "var(--color-surface)",
            width: "min(520px, 100vw)",
            height: "100vh",
            display: "flex",
            flexDirection: "column",
            boxShadow: "-4px 0 20px rgba(0,0,0,0.2)",
          }}
        >
          {/* Header */}
          <div
            style={{
              padding: "14px 18px",
              borderBottom: "1px solid var(--color-border)",
              display: "flex",
              justifyContent: "space-between",
              alignItems: "center",
            }}
          >
            <div>
              <h2 style={{ margin: 0, fontSize: 18 }}>{detalle.mesa_nombre}</h2>
              <div style={{ fontSize: 12, color: "var(--color-text-muted)" }}>
                {detalle.zona_nombre || "—"} · {detalle.pedido.comensales} comensales · {detalle.pedido.mesero_nombre || "—"}
              </div>
            </div>
            <div style={{ display: "flex", gap: 8, alignItems: "center" }}>
              {detalle.pedido.estado === "CUENTA_PEDIDA" && (
                <span
                  style={{
                    fontSize: 10,
                    fontWeight: 700,
                    padding: "3px 8px",
                    borderRadius: 4,
                    background: "var(--color-warning)",
                    color: "#fff",
                  }}
                >
                  CUENTA PEDIDA
                </span>
              )}
              <button
                onClick={() => onCerrar(false)}
                style={{
                  background: "transparent",
                  border: "none",
                  fontSize: 22,
                  cursor: "pointer",
                  color: "var(--color-text-muted)",
                  padding: 0,
                  width: 30,
                  height: 30,
                }}
              >
                ×
              </button>
            </div>
          </div>

          {/* Items del pedido */}
          <div style={{ flex: 1, overflowY: "auto", padding: "12px 18px" }}>
            {itemsAgrupados.length === 0 ? (
              <div
                style={{
                  padding: 30,
                  textAlign: "center",
                  color: "var(--color-text-muted)",
                  border: "2px dashed var(--color-border)",
                  borderRadius: 8,
                }}
              >
                Sin items aún. Click en "+ Agregar productos".
              </div>
            ) : (
              <div style={{ display: "flex", flexDirection: "column", gap: 6 }}>
                {itemsAgrupados.map((grupo) => (
                  <ItemRow
                    key={grupo.firstId}
                    grupo={grupo}
                    onEliminar={(id) => handleEliminarItem(id, grupo.nombre)}
                  />
                ))}
              </div>
            )}
          </div>

          {/* Footer con totales y acciones */}
          <div
            style={{
              borderTop: "1px solid var(--color-border)",
              padding: "12px 18px",
              display: "flex",
              flexDirection: "column",
              gap: 10,
              background: "var(--color-surface-hover)",
            }}
          >
            {/* Total */}
            <div
              style={{
                display: "flex",
                justifyContent: "space-between",
                alignItems: "baseline",
              }}
            >
              <span style={{ fontSize: 14, fontWeight: 600 }}>Total</span>
              <strong style={{ fontSize: 24, color: "var(--color-primary)" }}>
                ${detalle.total.toFixed(2)}
              </strong>
            </div>

            {/* Botón Agregar productos.
                Si la mesa ya pidió cuenta, pedir confirmación antes de agregar
                (porque la pre-cuenta impresa quedará desactualizada). */}
            <button
              className="btn btn-primary"
              onClick={() => {
                if (detalle.pedido.estado === "CUENTA_PEDIDA") {
                  if (!confirm(
                    "Esta mesa ya pidió la cuenta y la pre-cuenta fue impresa.\n\n" +
                    "Si agregas más productos, deberás reimprimir la pre-cuenta.\n\n" +
                    "¿Continuar?"
                  )) return;
                }
                setMostrarSelector(true);
              }}
              style={{
                width: "100%",
                padding: "10px",
                opacity: detalle.pedido.estado === "CUENTA_PEDIDA" ? 0.7 : 1,
              }}
            >
              {detalle.pedido.estado === "CUENTA_PEDIDA"
                ? "+ Agregar productos (mesa pidió cuenta)"
                : "+ Agregar productos"}
            </button>

            {/* Acciones secundarias */}
            <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: 6 }}>
              <button
                className="btn btn-outline"
                onClick={handleEnviarCocina}
                disabled={itemsNuevos === 0}
                title={itemsNuevos === 0 ? "No hay items nuevos" : ""}
                style={{ padding: "8px" }}
              >
                🔔 Enviar cocina {itemsNuevos > 0 && `(${itemsNuevos})`}
              </button>
              {detalle.pedido.estado === "CUENTA_PEDIDA" ? (
                <button
                  className="btn btn-outline"
                  onClick={handleReimprimirPreCuenta}
                  style={{ padding: "8px" }}
                  title="Reimprimir pre-cuenta (ya fue impresa)"
                >
                  🖨 Reimprimir cuenta
                </button>
              ) : (
                <button
                  className="btn btn-outline"
                  onClick={handlePedirCuenta}
                  disabled={detalle.items.length === 0}
                  style={{ padding: "8px" }}
                  title="Marca la mesa como CUENTA_PEDIDA e imprime ticket de cortesía"
                >
                  📄 Pedir cuenta
                </button>
              )}
            </div>

            {/* Cobrar y cancelar */}
            <div style={{ display: "grid", gridTemplateColumns: "2fr 1fr", gap: 6 }}>
              <button
                onClick={() => setModoCobro("elegir-pago")}
                disabled={detalle.items.length === 0}
                style={{
                  padding: "12px",
                  background: detalle.items.length === 0 ? "var(--color-border)" : "var(--color-success)",
                  color: "#fff",
                  border: "none",
                  borderRadius: 8,
                  fontSize: 15,
                  fontWeight: 700,
                  cursor: detalle.items.length === 0 ? "not-allowed" : "pointer",
                }}
              >
                💰 Cobrar ${detalle.total.toFixed(2)}
              </button>
              <button
                onClick={() => setConfirmCancelar(true)}
                style={{
                  padding: "12px 8px",
                  background: "transparent",
                  color: "var(--color-danger)",
                  border: "1.5px solid var(--color-danger)",
                  borderRadius: 8,
                  fontSize: 12,
                  fontWeight: 600,
                  cursor: "pointer",
                }}
              >
                Cancelar
              </button>
            </div>
          </div>
        </div>
      </div>

      {/* Modal selector de productos */}
      {mostrarSelector && (
        <SelectorProductos
          onSeleccionar={(prod, info) => handleAgregarItem(prod, info)}
          onCerrar={() => setMostrarSelector(false)}
        />
      )}

      {/* Modal cobro */}
      {modoCobro === "elegir-pago" && (
        <ModalCobro
          total={detalle.total}
          onCobrar={(forma, fiado, extras) => {
            setModoCobro(null);
            handleCobrar(forma, fiado, extras);
          }}
          onCancelar={() => setModoCobro(null)}
        />
      )}

      {/* Modal confirmar cancelación */}
      {confirmCancelar && (
        <div className="modal-overlay" onClick={() => setConfirmCancelar(false)}>
          <div
            className="modal-content"
            style={{ maxWidth: 380 }}
            onClick={(e) => e.stopPropagation()}
          >
            <div className="modal-header">
              <h3 style={{ margin: 0, color: "var(--color-danger)" }}>¿Cancelar pedido?</h3>
            </div>
            <div className="modal-body">
              Esta acción libera la mesa <strong>SIN registrar venta</strong>.
              Los items consumidos quedan sin cobrar. ¿Continuar?
            </div>
            <div className="modal-footer" style={{ display: "flex", gap: 8, justifyContent: "flex-end" }}>
              <button className="btn btn-outline" onClick={() => setConfirmCancelar(false)}>
                No, volver
              </button>
              <button
                onClick={() => {
                  setConfirmCancelar(false);
                  handleCancelarPedido();
                }}
                style={{
                  padding: "8px 16px",
                  background: "var(--color-danger)",
                  color: "#fff",
                  border: "none",
                  borderRadius: 6,
                  fontWeight: 600,
                  cursor: "pointer",
                }}
              >
                Sí, cancelar pedido
              </button>
            </div>
          </div>
        </div>
      )}
    </>
  );
}

// ─── Helpers ──────────────────────────────────────────────────────────────

interface ItemAgrupado {
  firstId: number;
  productoId: number;
  nombre: string;
  cantidadTotal: number;
  precioUnit: number;
  subtotal: number;
  enviadoCocina: boolean;
  estadoCocina: string;
  infoAdicional: string | null;
  /** COCINA | BARRA | DIRECTO */
  destinoPreparacion: string;
  /** ids individuales para borrar */
  ids: number[];
}

/** Agrupa items idénticos (mismo producto, mismo info_adicional, mismo estado) para mejor UX */
function agruparItems(items: PedidoDetalleType["items"]): ItemAgrupado[] {
  const mapa = new Map<string, ItemAgrupado>();
  for (const i of items) {
    const key = `${i.producto_id}|${i.info_adicional || ""}|${i.enviado_cocina ? 1 : 0}`;
    const existing = mapa.get(key);
    if (existing) {
      existing.cantidadTotal += i.cantidad;
      existing.subtotal += i.cantidad * i.precio_unit;
      existing.ids.push(i.id!);
    } else {
      mapa.set(key, {
        firstId: i.id!,
        productoId: i.producto_id,
        nombre: i.producto_nombre || `Producto #${i.producto_id}`,
        cantidadTotal: i.cantidad,
        precioUnit: i.precio_unit,
        subtotal: i.cantidad * i.precio_unit,
        enviadoCocina: i.enviado_cocina,
        estadoCocina: i.estado_cocina,
        infoAdicional: i.info_adicional ?? null,
        destinoPreparacion: i.destino_preparacion || "COCINA",
        ids: [i.id!],
      });
    }
  }
  return Array.from(mapa.values());
}

function ItemRow({ grupo, onEliminar }: { grupo: ItemAgrupado; onEliminar: (id: number) => void }) {
  const esDirecto = grupo.destinoPreparacion === "DIRECTO";
  const esBarra = grupo.destinoPreparacion === "BARRA";

  const colorEstado = esDirecto
    ? null // Items DIRECTO no muestran estados de cocina (van directo)
    : grupo.estadoCocina === "LISTO"
      ? "var(--color-success)"
      : grupo.estadoCocina === "EN_PREPARACION"
        ? "var(--color-warning)"
        : grupo.estadoCocina === "ENTREGADO"
          ? "var(--color-text-muted)"
          : null;

  // Color de fondo según destino y estado
  const bgColor = esDirecto
    ? "rgba(34, 197, 94, 0.08)" // verde claro = ya disponible para entregar
    : grupo.enviadoCocina
      ? "var(--color-surface)"
      : "rgba(59, 130, 246, 0.06)"; // azul claro = nuevo, no enviado a cocina aún

  const borderColor = esDirecto
    ? "rgba(34, 197, 94, 0.3)"
    : grupo.enviadoCocina
      ? "var(--color-border)"
      : "rgba(59, 130, 246, 0.3)";

  return (
    <div
      style={{
        display: "flex",
        alignItems: "center",
        gap: 10,
        padding: "8px 10px",
        background: bgColor,
        border: `1px solid ${borderColor}`,
        borderRadius: 8,
      }}
    >
      <span
        style={{
          minWidth: 32,
          height: 32,
          background: "var(--color-primary)",
          color: "#fff",
          borderRadius: 6,
          display: "flex",
          alignItems: "center",
          justifyContent: "center",
          fontWeight: 700,
          fontSize: 14,
          flexShrink: 0,
        }}
      >
        {grupo.cantidadTotal}
      </span>
      <div style={{ flex: 1, minWidth: 0 }}>
        <div
          style={{
            fontSize: 14,
            fontWeight: 600,
            display: "flex",
            alignItems: "center",
            gap: 6,
          }}
        >
          <span style={{ overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>
            {grupo.nombre}
          </span>
          {/* Badge DIRECTO/BARRA para distinguir destino. Solo muestra UNO según prioridad:
              - Si es DIRECTO → 📦 DIRECTO (verde, ya disponible)
              - Si es BARRA y aún no enviado → 🍷 BARRA NUEVO
              - Si es COCINA y aún no enviado → NUEVO
              - Si está en algún estado de cocina → badge del estado */}
          {esDirecto ? (
            <span
              style={{
                fontSize: 9,
                fontWeight: 700,
                padding: "1px 5px",
                borderRadius: 3,
                background: "var(--color-success)",
                color: "#fff",
                flexShrink: 0,
              }}
              title="Despacho directo: el mesero lo toma del mostrador"
            >
              📦 DIRECTO
            </span>
          ) : !grupo.enviadoCocina ? (
            <span
              style={{
                fontSize: 9,
                fontWeight: 700,
                padding: "1px 5px",
                borderRadius: 3,
                background: esBarra ? "#7c3aed" : "var(--color-primary)",
                color: "#fff",
                flexShrink: 0,
              }}
            >
              {esBarra ? "🍷 BARRA NUEVO" : "NUEVO"}
            </span>
          ) : null}
          {colorEstado && !esDirecto && (
            <span
              style={{
                fontSize: 9,
                fontWeight: 700,
                padding: "1px 5px",
                borderRadius: 3,
                background: colorEstado,
                color: "#fff",
                flexShrink: 0,
              }}
            >
              {esBarra && grupo.estadoCocina === "EN_PREPARACION"
                ? "🍷 EN BARRA"
                : grupo.estadoCocina.replace("_", " ")}
            </span>
          )}
        </div>
        {grupo.infoAdicional && (
          <div style={{ fontSize: 11, color: "var(--color-text-muted)", fontStyle: "italic" }}>
            ↳ {grupo.infoAdicional}
          </div>
        )}
        <div style={{ fontSize: 11, color: "var(--color-text-muted)" }}>
          ${grupo.precioUnit.toFixed(2)} c/u
        </div>
      </div>
      <strong style={{ fontSize: 14, minWidth: 60, textAlign: "right" }}>
        ${grupo.subtotal.toFixed(2)}
      </strong>
      {/* Permitir eliminar items NO enviados a cocina, O items DIRECTO (porque
          aunque estén marcados como entregados, no pasaron por la cocina y
          el mesero podría haberlos agregado por error). */}
      {(!grupo.enviadoCocina || esDirecto) && (
        <button
          onClick={() => onEliminar(grupo.ids[grupo.ids.length - 1])}
          title="Quitar uno"
          style={{
            background: "transparent",
            border: "none",
            fontSize: 18,
            cursor: "pointer",
            color: "var(--color-danger)",
            padding: 0,
            width: 24,
          }}
        >
          ×
        </button>
      )}
    </div>
  );
}

function ModalCobro({
  total,
  onCobrar,
  onCancelar,
}: {
  total: number;
  onCobrar: (
    forma: string,
    esFiado: boolean,
    extras?: { bancoId?: number | null; referencia?: string | null },
  ) => void;
  onCancelar: () => void;
}) {
  // Sub-paso para transferencia: pide banco + referencia ANTES de confirmar.
  // Mismo flujo que el POS normal cuando se cobra con transferencia.
  const [modoTransfer, setModoTransfer] = useState(false);
  const [cuentas, setCuentas] = useState<CuentaBanco[]>([]);
  const [cuentaSel, setCuentaSel] = useState<number | null>(null);
  const [referencia, setReferencia] = useState("");
  const [requiereRef, setRequiereRef] = useState(false);

  // Cargar cuentas bancarias + config "transferencia_requiere_referencia"
  useEffect(() => {
    if (!modoTransfer) return;
    Promise.all([
      listarCuentasBanco().catch(() => []),
      obtenerConfig().catch(() => ({} as Record<string, string>)),
    ]).then(([cs, cfg]) => {
      const activas = cs.filter((c) => c.activa);
      setCuentas(activas);
      if (activas.length > 0 && cuentaSel === null) {
        setCuentaSel(activas[0].id ?? null);
      }
      setRequiereRef(cfg.transferencia_requiere_referencia === "1");
    });
  }, [modoTransfer]);

  const handleConfirmarTransfer = () => {
    if (cuentas.length > 0 && cuentaSel === null) {
      alert("Selecciona la cuenta bancaria");
      return;
    }
    if (requiereRef && !referencia.trim()) {
      alert("La referencia es obligatoria para transferencias (configurado en Cuentas Bancarias)");
      return;
    }
    onCobrar("TRANSFER", false, {
      bancoId: cuentaSel,
      referencia: referencia.trim() || null,
    });
  };

  // Vista 1: elegir forma de pago
  if (!modoTransfer) {
    return (
      <div className="modal-overlay" onClick={onCancelar}>
        <div className="modal-content" style={{ maxWidth: 420 }} onClick={(e) => e.stopPropagation()}>
          <div className="modal-header">
            <h3 style={{ margin: 0 }}>Cobrar ${total.toFixed(2)}</h3>
          </div>
          <div className="modal-body" style={{ display: "flex", flexDirection: "column", gap: 10 }}>
            <p style={{ margin: 0, fontSize: 13, color: "var(--color-text-muted)" }}>
              Selecciona la forma de pago. Se generará una nota de venta y la mesa quedará libre.
            </p>
            <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: 8 }}>
              <BotonPago label="💵 Efectivo" color="#16a34a" onClick={() => onCobrar("EFECTIVO", false)} />
              <BotonPago label="💳 Tarjeta" color="#2563eb" onClick={() => onCobrar("TARJETA", false)} />
              <BotonPago label="🏦 Transfer." color="#0ea5e9" onClick={() => setModoTransfer(true)} />
              <BotonPago label="📋 Crédito" color="#f59e0b" onClick={() => onCobrar("CREDITO", true)} />
            </div>
          </div>
          <div className="modal-footer" style={{ display: "flex", justifyContent: "flex-end" }}>
            <button className="btn btn-outline" onClick={onCancelar}>
              Cancelar
            </button>
          </div>
        </div>
      </div>
    );
  }

  // Vista 2: datos de transferencia
  return (
    <div className="modal-overlay" onClick={onCancelar}>
      <div className="modal-content" style={{ maxWidth: 460 }} onClick={(e) => e.stopPropagation()}>
        <div className="modal-header">
          <h3 style={{ margin: 0 }}>🏦 Cobrar por transferencia ${total.toFixed(2)}</h3>
        </div>
        <div className="modal-body" style={{ display: "flex", flexDirection: "column", gap: 12 }}>
          {cuentas.length === 0 ? (
            <div style={{
              padding: 12, background: "rgba(245,158,11,0.1)",
              border: "1px solid rgba(245,158,11,0.3)", borderRadius: 6,
              fontSize: 12, color: "var(--color-warning)",
            }}>
              ⚠ No hay cuentas bancarias configuradas. Igual puedes registrar la transferencia
              ingresando solo la referencia. Para mejor control, ve a Configuración → Cuentas Bancarias.
            </div>
          ) : (
            <label style={{ fontSize: 13, fontWeight: 600, display: "block" }}>
              Cuenta bancaria de destino
              <select
                value={cuentaSel ?? ""}
                onChange={(e) => setCuentaSel(e.target.value ? parseInt(e.target.value, 10) : null)}
                className="input"
                style={{ width: "100%", marginTop: 4 }}
                autoFocus
              >
                {cuentas.map((c) => (
                  <option key={c.id} value={c.id}>
                    {c.nombre}
                    {c.numero_cuenta ? ` — ${c.numero_cuenta}` : ""}
                  </option>
                ))}
              </select>
            </label>
          )}

          <label style={{ fontSize: 13, fontWeight: 600, display: "block" }}>
            Referencia / N° comprobante {requiereRef && <span style={{ color: "var(--color-danger)" }}>*</span>}
            <input
              value={referencia}
              onChange={(e) => setReferencia(e.target.value)}
              className="input"
              placeholder="Ej: TX-12345 o número del comprobante"
              style={{ width: "100%", marginTop: 4 }}
              onKeyDown={(e) => e.key === "Enter" && handleConfirmarTransfer()}
            />
            {!requiereRef && (
              <span style={{ fontSize: 11, color: "var(--color-text-muted)", marginTop: 2, display: "block" }}>
                Opcional, pero recomendado para verificación posterior.
              </span>
            )}
          </label>

          <div style={{
            padding: 10, background: "rgba(14,165,233,0.08)",
            border: "1px solid rgba(14,165,233,0.2)", borderRadius: 6,
            fontSize: 11, color: "var(--color-text-muted)",
          }}>
            ℹ La transferencia quedará registrada en <strong>Movimientos Bancarios</strong>.
            Si tu admin tiene activa la verificación, la venta queda en estado pendiente
            hasta que admin confirme el ingreso (Cuentas → Verificación).
          </div>
        </div>
        <div className="modal-footer" style={{ display: "flex", gap: 8, justifyContent: "space-between" }}>
          <button
            className="btn btn-outline"
            onClick={() => setModoTransfer(false)}
            style={{ fontSize: 12 }}
          >
            ← Volver
          </button>
          <div style={{ display: "flex", gap: 8 }}>
            <button className="btn btn-outline" onClick={onCancelar}>
              Cancelar
            </button>
            <button
              onClick={handleConfirmarTransfer}
              style={{
                padding: "8px 18px", background: "#0ea5e9", color: "#fff",
                border: "none", borderRadius: 6, fontWeight: 700, cursor: "pointer",
              }}
            >
              Confirmar transferencia
            </button>
          </div>
        </div>
      </div>
    </div>
  );
}

function BotonPago({ label, color, onClick }: { label: string; color: string; onClick: () => void }) {
  return (
    <button
      onClick={onClick}
      style={{
        padding: "16px 12px",
        background: color,
        color: "#fff",
        border: "none",
        borderRadius: 8,
        fontSize: 14,
        fontWeight: 700,
        cursor: "pointer",
        transition: "transform 0.1s",
      }}
      onMouseEnter={(e) => (e.currentTarget.style.transform = "translateY(-1px)")}
      onMouseLeave={(e) => (e.currentTarget.style.transform = "translateY(0)")}
    >
      {label}
    </button>
  );
}
