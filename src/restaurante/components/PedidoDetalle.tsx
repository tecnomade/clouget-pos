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
  registrarAbono,
  imprimirPreCuenta,
  imprimirComandaCocina,
  unirMesas,
  desunirMesa,
  listarMesasLibresParaUnir,
  dividirCuenta,
  listarSubcuentas,
  cancelarDivision,
  marcarSubcuentaCobrada,
  productoDivisionId,
} from "../api";
import { registrarVenta, obtenerCajaAbierta, listarCuentasBanco, obtenerConfig, emitirFacturaSri, enviarNotificacionSri } from "../../services/api";
// v2.5.36: post-cobro con SRI + retenciones
import ModalRetenciones from "../../components/ModalRetenciones";
// v2.3.64+ pendiente: aplicar descuento por forma de pago al cobrar mesa.
// Ya está implementado en POS normal (v2.3.63). Aquí se agregará en próxima
// iteración para mantener este release manejable.
// import { ... } from "../../utils/descuentoFormaPago";
import type { PedidoDetalle as PedidoDetalleType, MesaResumen, Subcuenta } from "../types";
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
  // v2.3.68 — Unir mesas
  const [mostrarUnirMesas, setMostrarUnirMesas] = useState(false);
  // v2.3.69 — Dividir cuenta
  const [subcuentas, setSubcuentas] = useState<Subcuenta[]>([]);
  const [mostrarDividir, setMostrarDividir] = useState(false);
  /** sub-cuenta que se está cobrando (modal de forma de pago) */
  const [cobrandoSubcuenta, setCobrandoSubcuenta] = useState<Subcuenta | null>(null);
  // v2.5.36: post-cobro — permite emitir Factura SRI y aplicar retenciones a la venta de restaurante
  const [postCobro, setPostCobro] = useState<{ ventaId: number; numero: string; total: number; subtotal: number; iva: number } | null>(null);
  const [postCobroEmitiendo, setPostCobroEmitiendo] = useState(false);
  const [postCobroEstadoSri, setPostCobroEstadoSri] = useState<string | null>(null);
  const [postCobroMostrarRet, setPostCobroMostrarRet] = useState(false);
  const [certificadoSriCargado, setCertificadoSriCargado] = useState(false);
  // v2.5.91 — abonos / pagos parciales sobre la mesa
  const [cuentasAbono, setCuentasAbono] = useState<CuentaBanco[]>([]);
  useEffect(() => { listarCuentasBanco().then(setCuentasAbono).catch(() => setCuentasAbono([])); }, []);
  const [mostrarAbono, setMostrarAbono] = useState(false);
  const [abonoMonto, setAbonoMonto] = useState("");
  const [abonoForma, setAbonoForma] = useState("EFECTIVO");
  const [abonoBancoId, setAbonoBancoId] = useState<number | null>(null);
  const [abonoReferencia, setAbonoReferencia] = useState("");
  const [abonoGuardando, setAbonoGuardando] = useState(false);

  const handleRegistrarAbono = async () => {
    if (!detalle) return;
    const monto = parseFloat(abonoMonto.replace(",", ".")) || 0;
    if (monto <= 0) { toastWarning("Ingresa un monto válido"); return; }
    const saldo = (detalle.total ?? 0) - (detalle.total_abonado ?? 0);
    if (monto > saldo + 0.01) { toastError(`El abono no puede superar el saldo ($${saldo.toFixed(2)})`); return; }
    if (abonoForma === "TRANSFER" && !abonoBancoId) { toastError("Selecciona la cuenta bancaria"); return; }
    try {
      const caja = await obtenerCajaAbierta();
      if (!caja) { toastError("Debes abrir una caja antes de recibir abonos"); return; }
    } catch { toastError("Error verificando caja"); return; }
    setAbonoGuardando(true);
    try {
      const actualizado = await registrarAbono({
        pedidoId, monto, formaPago: abonoForma,
        bancoId: abonoForma === "TRANSFER" ? abonoBancoId : null,
        referenciaPago: abonoReferencia.trim() || null,
      });
      setDetalle(actualizado);
      setMostrarAbono(false);
      setAbonoMonto(""); setAbonoReferencia(""); setAbonoBancoId(null); setAbonoForma("EFECTIVO");
      toastExito(`Abono de $${monto.toFixed(2)} registrado · Saldo $${(actualizado.saldo ?? 0).toFixed(2)}`);
    } catch (err: any) {
      toastError("No se pudo registrar el abono: " + (err?.message || err));
    } finally {
      setAbonoGuardando(false);
    }
  };

  const cargar = useCallback(async () => {
    try {
      const [d, subs] = await Promise.all([
        obtenerPedido(pedidoId),
        listarSubcuentas(pedidoId).catch(() => [] as Subcuenta[]),
      ]);
      setDetalle(d);
      setSubcuentas(subs);
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

  // v2.5.36: detectar si hay certificado SRI cargado para mostrar boton "Emitir Factura SRI"
  useEffect(() => {
    obtenerConfig().then((cfg: any) => {
      setCertificadoSriCargado(cfg.sri_certificado_cargado === "1" && cfg.sri_modulo_activo === "1");
    }).catch(() => {});
  }, []);

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
      // v2.3.67: auto-imprimir comanda con SOLO los items recién enviados
      // (pasamos sus IDs para que el ticket no incluya items viejos).
      // Si falla la impresión, no rompemos el flujo — el estado ya está actualizado.
      const itemIds = items.map(i => i.id).filter((id): id is number => id != null);
      let mensajeImpresion = "";
      try {
        const r = await imprimirComandaCocina(pedidoId, itemIds);
        mensajeImpresion = ` · ${r}`;
      } catch (err: any) {
        // No bloquear: solo warn. Igual los items quedan marcados como enviados.
        toastWarning(
          `${items.length} item(s) enviado(s), pero la comanda no se imprimió: ${err?.message || err}. Verifica impresora en Configuración → Cocina.`,
        );
        await cargar();
        return;
      }
      toastExito(`${items.length} item(s) enviado(s) a cocina${mensajeImpresion}`);
      await cargar();
    } catch (err: any) {
      toastError(err?.message || String(err));
    }
  };

  // v2.3.67: re-imprimir comanda completa de un pedido (todos los items, no solo
  // los recién enviados). Útil si el ticket se perdió o no salió bien.
  const handleReimprimirComanda = async () => {
    try {
      const r = await imprimirComandaCocina(pedidoId);
      toastExito(`Comanda reimpresa · ${r}`);
    } catch (err: any) {
      toastError(`No se pudo reimprimir: ${err?.message || err}`);
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

  // v2.3.68 — Unir mesas
  const handleUnirMesas = async (mesasIds: number[]) => {
    if (mesasIds.length === 0) return;
    try {
      await unirMesas(pedidoId, mesasIds);
      toastExito(`${mesasIds.length} mesa(s) unida(s) al pedido`);
      setMostrarUnirMesas(false);
      await cargar();
    } catch (err: any) {
      toastError("No se pudieron unir mesas: " + (err?.message || err));
    }
  };

  const handleDesunirMesa = async (mesa: MesaResumen) => {
    if (!confirm(`¿Liberar la ${mesa.nombre} del grupo? Sus items quedan en la mesa principal.`)) return;
    try {
      await desunirMesa(pedidoId, mesa.id);
      toastExito(`${mesa.nombre} liberada del grupo`);
      await cargar();
    } catch (err: any) {
      toastError("No se pudo desunir: " + (err?.message || err));
    }
  };

  // v2.3.69 — Dividir cuenta
  const handleDividirCuenta = async (nPartes: number) => {
    if (!detalle || detalle.items.length === 0) {
      toastWarning("El pedido no tiene items para dividir");
      return;
    }
    try {
      const subs = await dividirCuenta(pedidoId, nPartes);
      setSubcuentas(subs);
      setMostrarDividir(false);
      toastExito(`Cuenta dividida en ${nPartes} partes (~$${(detalle.total / nPartes).toFixed(2)} c/u)`);
    } catch (err: any) {
      toastError("No se pudo dividir: " + (err?.message || err));
    }
  };

  const handleCancelarDivision = async () => {
    if (!confirm("¿Deshacer la división? Volverá a una sola cuenta.")) return;
    try {
      await cancelarDivision(pedidoId);
      setSubcuentas([]);
      toastExito("División cancelada");
    } catch (err: any) {
      toastError(err?.message || String(err));
    }
  };

  /** Cobra una sub-cuenta: genera una venta con el producto especial
   *  _DIVISION_CUENTA_ por el monto exacto, luego marca la sub-cuenta como
   *  cobrada. Si todas quedan cobradas → cierra pedido y libera mesa(s). */
  const handleCobrarSubcuenta = async (
    sub: Subcuenta,
    formaPago: string,
    esFiado: boolean,
    extras?: { bancoId?: number | null; referencia?: string | null },
  ) => {
    if (!detalle) return;
    try {
      // Verificar caja abierta (usa la del POS, no hay caja separada para restaurante)
      const caja = await obtenerCajaAbierta();
      if (!caja) {
        toastError("Debes abrir una caja antes de cobrar");
        return;
      }
    } catch {
      toastError("Error verificando caja");
      return;
    }

    let prodId: number;
    try {
      prodId = await productoDivisionId();
    } catch (err: any) {
      toastError("No se pudo obtener producto especial: " + (err?.message || err));
      return;
    }

    const item: VentaDetalle = {
      producto_id: prodId,
      cantidad: 1,
      precio_unitario: sub.total,
      descuento: 0,
      iva_porcentaje: 0,
      subtotal: sub.total,
      info_adicional: `Items consumidos: ${detalle.items
        .map((i) => `${i.cantidad}x ${i.producto_nombre || ""}`.trim())
        .join(", ")}`,
    };

    const payload: NuevaVenta = {
      items: [item],
      forma_pago: formaPago,
      monto_recibido: esFiado ? 0 : sub.total,
      descuento: 0,
      tipo_documento: "NOTA_VENTA",
      es_fiado: esFiado,
      observacion: `Mesa: ${detalle.mesa_nombre}${detalle.zona_nombre ? ` (${detalle.zona_nombre})` : ""} · Pedido #${pedidoId} · Sub-cuenta ${sub.numero}/${subcuentas.length}`,
      banco_id: extras?.bancoId ?? null,
      referencia_pago: extras?.referencia?.trim() || null,
    };

    try {
      const resultado = await registrarVenta(payload);
      const cobro = await marcarSubcuentaCobrada(
        sub.id,
        resultado.venta.id!,
        formaPago,
        extras?.bancoId ?? null,
        extras?.referencia?.trim() || null,
      );

      if (cobro.todas_cobradas) {
        toastExito(`Sub-cuenta ${sub.numero} cobrada · 🎉 Mesa liberada (todas pagadas)`);
        onCerrar(true);
      } else {
        toastExito(`Sub-cuenta ${sub.numero} cobrada · Quedan ${cobro.pendientes} pendiente(s)`);
        await cargar();
      }
    } catch (err: any) {
      toastError("No se pudo cobrar la sub-cuenta: " + (err?.message || err));
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
    const observacion = `Mesa: ${detalle.mesa_nombre}${detalle.zona_nombre ? ` (${detalle.zona_nombre})` : ""} · Pedido #${pedidoId}`;

    // v2.5.91 — si la mesa tiene abonos (pagos parciales), la venta es MIXTO:
    // cada abono con su forma + el SALDO con la forma del cierre.
    const abonosHolding = (detalle.abonos ?? []).filter((a) => a.estado === "HOLDING");
    const totalAbonado = abonosHolding.reduce((s, a) => s + a.monto, 0);
    const saldo = Math.max(0, total - totalAbonado);

    let payload: NuevaVenta;
    if (totalAbonado > 0.01) {
      const pagos = abonosHolding.map((a) => ({
        forma_pago: a.forma_pago,
        monto: a.monto,
        banco_id: a.banco_id ?? null,
        referencia: a.referencia_pago ?? null,
      }));
      if (saldo > 0.01) {
        pagos.push({
          forma_pago: esFiado ? "CREDITO" : formaPago,
          monto: saldo,
          banco_id: extras?.bancoId ?? null,
          referencia: extras?.referencia?.trim() || null,
        });
      }
      payload = {
        items,
        forma_pago: "MIXTO",
        monto_recibido: total,
        descuento: 0,
        tipo_documento: "NOTA_VENTA",
        es_fiado: false,
        observacion,
        pagos,
      };
    } else {
      payload = {
        items,
        forma_pago: formaPago,
        monto_recibido: esFiado ? 0 : total,
        descuento: 0,
        tipo_documento: "NOTA_VENTA",
        es_fiado: esFiado,
        observacion,
        // Transferencia: pasar banco + referencia para que aparezca en
        // /movimientos-bancarios y /verificacion (panel admin) — mismo flujo POS.
        banco_id: extras?.bancoId ?? null,
        referencia_pago: extras?.referencia?.trim() || null,
      };
    }

    try {
      const resultado = await registrarVenta(payload);
      // Vincular venta con pedido y liberar mesa
      await cerrarPedido(pedidoId, resultado.venta.id!);
      toastExito(`Venta ${resultado.venta.numero} registrada · Mesa liberada`);

      // v2.5.36: si hay certificado SRI cargado, abrir post-cobro con opciones SRI/retenciones
      if (resultado.venta.id && certificadoSriCargado) {
        setPostCobro({
          ventaId: resultado.venta.id,
          numero: resultado.venta.numero,
          total: resultado.venta.total,
          subtotal: resultado.venta.subtotal_con_iva + resultado.venta.subtotal_sin_iva,
          iva: resultado.venta.iva,
        });
        setPostCobroEstadoSri(null);
        // NO cerrar aún — el modal post-cobro maneja el cierre
        return;
      }

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
            <div style={{ flex: 1, minWidth: 0 }}>
              <h2 style={{ margin: 0, fontSize: 18, display: "flex", alignItems: "center", gap: 8, flexWrap: "wrap" }}>
                <span>{detalle.mesa_nombre}</span>
                {detalle.mesas_extra.length > 0 && (
                  <span style={{
                    fontSize: 11,
                    fontWeight: 700,
                    padding: "2px 7px",
                    borderRadius: 10,
                    background: "var(--color-primary)",
                    color: "#fff",
                    whiteSpace: "nowrap",
                  }} title="Pedido con mesas unidas">
                    🔗 +{detalle.mesas_extra.length}
                  </span>
                )}
              </h2>
              <div style={{ fontSize: 12, color: "var(--color-text-muted)" }}>
                {detalle.zona_nombre || "—"} · {detalle.pedido.comensales} comensales
                {detalle.capacidad_total > 0 && ` (cap. ${detalle.capacidad_total})`}
                {" · "}
                {detalle.pedido.mesero_nombre || "—"}
              </div>
              {/* v2.3.68 — Lista de mesas unidas con botón desunir cada una */}
              {detalle.mesas_extra.length > 0 && (
                <div style={{
                  marginTop: 6,
                  display: "flex",
                  flexWrap: "wrap",
                  gap: 4,
                }}>
                  {detalle.mesas_extra.map(m => (
                    <span
                      key={m.id}
                      style={{
                        fontSize: 10,
                        padding: "2px 6px 2px 8px",
                        borderRadius: 10,
                        background: "rgba(59, 130, 246, 0.15)",
                        color: "var(--color-primary)",
                        border: "1px solid rgba(59, 130, 246, 0.3)",
                        display: "inline-flex",
                        alignItems: "center",
                        gap: 4,
                      }}
                      title={`Capacidad ${m.capacidad}${m.zona_nombre ? ` · ${m.zona_nombre}` : ""}`}
                    >
                      🔗 {m.nombre}
                      {detalle.pedido.estado !== "COBRADO" && detalle.pedido.estado !== "CANCELADO" && (
                        <button
                          onClick={() => handleDesunirMesa(m)}
                          style={{
                            background: "transparent",
                            border: "none",
                            color: "var(--color-danger)",
                            cursor: "pointer",
                            padding: 0,
                            fontSize: 14,
                            lineHeight: 1,
                            marginLeft: 2,
                          }}
                          title="Desunir esta mesa"
                        >
                          ×
                        </button>
                      )}
                    </span>
                  ))}
                </div>
              )}
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
              <span style={{ fontSize: 14, fontWeight: 600 }}>{(detalle.total_abonado ?? 0) > 0 ? "Consumido" : "Total"}</span>
              <strong style={{ fontSize: 24, color: "var(--color-primary)" }}>
                ${detalle.total.toFixed(2)}
              </strong>
            </div>

            {/* v2.5.91 — Abonos / pagos parciales */}
            {(detalle.total_abonado ?? 0) > 0 && (
              <div style={{ marginTop: 4, padding: "6px 10px", background: "rgba(34,197,94,0.08)", border: "1px solid rgba(34,197,94,0.3)", borderRadius: 8 }}>
                <div style={{ display: "flex", justifyContent: "space-between", fontSize: 13 }}>
                  <span style={{ color: "var(--color-text-secondary)" }}>Abonado (parcial)</span>
                  <strong style={{ color: "var(--color-success)" }}>− ${(detalle.total_abonado ?? 0).toFixed(2)}</strong>
                </div>
                <div style={{ display: "flex", justifyContent: "space-between", fontSize: 15, marginTop: 2 }}>
                  <strong>Saldo a cobrar</strong>
                  <strong style={{ color: "var(--color-danger)" }}>${(detalle.saldo ?? detalle.total).toFixed(2)}</strong>
                </div>
                {(detalle.abonos ?? []).filter(a => a.estado === "HOLDING").map((a) => (
                  <div key={a.id} style={{ display: "flex", justifyContent: "space-between", fontSize: 10, color: "var(--color-text-secondary)", marginTop: 2 }}>
                    <span>· {a.fecha?.slice(11, 16)} {a.forma_pago}{a.banco_nombre ? ` (${a.banco_nombre})` : ""}{a.usuario_nombre ? ` — ${a.usuario_nombre}` : ""}</span>
                    <span>${a.monto.toFixed(2)}</span>
                  </div>
                ))}
              </div>
            )}

            {/* Botón registrar pago parcial (mesa con consumo) */}
            {detalle.items.length > 0 && (detalle.saldo ?? detalle.total) > 0.01 && (
              <button
                className="btn btn-outline"
                style={{ width: "100%", fontSize: 12 }}
                onClick={() => { setAbonoMonto(((detalle.saldo ?? detalle.total)).toFixed(2)); setMostrarAbono(true); }}
              >
                ＋ Registrar pago parcial (abono)
              </button>
            )}

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
              <div>
                <button
                  className="btn btn-outline"
                  onClick={handleEnviarCocina}
                  disabled={itemsNuevos === 0}
                  title={itemsNuevos === 0 ? "No hay items nuevos" : ""}
                  style={{ padding: "8px", width: "100%" }}
                >
                  🔔 Enviar cocina {itemsNuevos > 0 && `(${itemsNuevos})`}
                </button>
                {/* v2.3.67: Si ya hay items enviados, ofrecer re-imprimir comanda */}
                {detalle.items.some(i => i.enviado_cocina && i.destino_preparacion !== "DIRECTO") && (
                  <button
                    onClick={handleReimprimirComanda}
                    style={{
                      width: "100%", marginTop: 4, padding: "4px",
                      background: "transparent", border: "none",
                      color: "var(--color-primary)", fontSize: 10,
                      cursor: "pointer", textDecoration: "underline",
                    }}
                    title="Re-imprimir comanda completa (todos los items del pedido)"
                  >
                    🖨 Reimprimir comanda
                  </button>
                )}
              </div>
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

            {/* v2.3.68 — Unir mesas (solo si el pedido sigue activo) */}
            {detalle.pedido.estado !== "COBRADO" && detalle.pedido.estado !== "CANCELADO" && (
              <button
                onClick={() => setMostrarUnirMesas(true)}
                style={{
                  width: "100%",
                  padding: "8px",
                  background: "transparent",
                  color: "var(--color-primary)",
                  border: "1.5px dashed var(--color-primary)",
                  borderRadius: 6,
                  fontSize: 12,
                  fontWeight: 600,
                  cursor: "pointer",
                }}
                title="Une mesas libres a este pedido (grupos grandes)"
              >
                🔗 Unir mesas{detalle.mesas_extra.length > 0 ? ` (ya unidas: ${detalle.mesas_extra.length})` : ""}
              </button>
            )}

            {/* v2.3.69 — Sub-cuentas (división de cuenta) */}
            {subcuentas.length > 0 && (
              <div style={{
                display: "flex",
                flexDirection: "column",
                gap: 6,
                background: "var(--color-surface)",
                border: "1.5px solid var(--color-primary)",
                borderRadius: 8,
                padding: 8,
              }}>
                <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
                  <strong style={{ fontSize: 12, color: "var(--color-primary)" }}>
                    ✂️ Cuenta dividida en {subcuentas.length} partes
                  </strong>
                  {subcuentas.every(s => s.estado === "PENDIENTE") && (
                    <button
                      onClick={handleCancelarDivision}
                      style={{
                        background: "transparent",
                        border: "none",
                        color: "var(--color-danger)",
                        fontSize: 11,
                        cursor: "pointer",
                        textDecoration: "underline",
                        padding: 0,
                      }}
                      title="Deshacer la división (solo si ninguna está cobrada)"
                    >
                      Cancelar división
                    </button>
                  )}
                </div>
                <div style={{ display: "flex", flexDirection: "column", gap: 4 }}>
                  {subcuentas.map(sub => {
                    const cobrada = sub.estado === "COBRADA";
                    return (
                      <div
                        key={sub.id}
                        style={{
                          display: "flex",
                          justifyContent: "space-between",
                          alignItems: "center",
                          gap: 8,
                          padding: "6px 8px",
                          background: cobrada ? "rgba(34, 197, 94, 0.08)" : "var(--color-surface-hover)",
                          borderRadius: 6,
                          opacity: cobrada ? 0.7 : 1,
                        }}
                      >
                        <div style={{ flex: 1, minWidth: 0 }}>
                          <div style={{ fontSize: 13, fontWeight: 600 }}>
                            Parte {sub.numero}/{subcuentas.length} · ${sub.total.toFixed(2)}
                          </div>
                          {cobrada && (
                            <div style={{ fontSize: 10, color: "var(--color-success)" }}>
                              ✓ {sub.forma_pago}{sub.venta_numero ? ` · ${sub.venta_numero}` : ""}
                            </div>
                          )}
                        </div>
                        {cobrada ? (
                          <span style={{
                            fontSize: 10,
                            fontWeight: 700,
                            padding: "2px 8px",
                            borderRadius: 10,
                            background: "var(--color-success)",
                            color: "#fff",
                          }}>
                            COBRADA
                          </span>
                        ) : (
                          <button
                            onClick={() => setCobrandoSubcuenta(sub)}
                            style={{
                              padding: "6px 14px",
                              background: "var(--color-success)",
                              color: "#fff",
                              border: "none",
                              borderRadius: 6,
                              fontSize: 12,
                              fontWeight: 700,
                              cursor: "pointer",
                              whiteSpace: "nowrap",
                            }}
                          >
                            💰 Cobrar
                          </button>
                        )}
                      </div>
                    );
                  })}
                </div>
              </div>
            )}

            {/* Cobrar y cancelar — solo si NO hay división activa */}
            {subcuentas.length === 0 && (
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
                💰 Cobrar ${((detalle.total_abonado ?? 0) > 0 ? (detalle.saldo ?? detalle.total) : detalle.total).toFixed(2)}
                {(detalle.total_abonado ?? 0) > 0 && <span style={{ fontSize: 10, opacity: 0.85 }}> (saldo)</span>}
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
            )}

            {/* v2.3.69 — Dividir cuenta (solo si NO hay división y hay items) */}
            {subcuentas.length === 0 && detalle.items.length > 0 &&
              detalle.pedido.estado !== "COBRADO" && detalle.pedido.estado !== "CANCELADO" && (
                <button
                  onClick={() => setMostrarDividir(true)}
                  style={{
                    width: "100%",
                    padding: "8px",
                    background: "transparent",
                    color: "var(--color-text-muted)",
                    border: "1px dashed var(--color-border)",
                    borderRadius: 6,
                    fontSize: 11,
                    fontWeight: 500,
                    cursor: "pointer",
                  }}
                  title="Divide el total en partes iguales (cada comensal paga su parte)"
                >
                  ✂️ Dividir cuenta entre varios
                </button>
              )}
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

      {/* v2.3.68 — Modal unir mesas */}
      {mostrarUnirMesas && (
        <ModalUnirMesas
          pedidoId={pedidoId}
          mesaPrincipalNombre={detalle.mesa_nombre}
          onUnir={handleUnirMesas}
          onCerrar={() => setMostrarUnirMesas(false)}
        />
      )}

      {/* v2.3.69 — Modal dividir cuenta */}
      {mostrarDividir && (
        <ModalDividirCuenta
          totalPedido={detalle.total}
          comensales={detalle.pedido.comensales}
          onDividir={handleDividirCuenta}
          onCerrar={() => setMostrarDividir(false)}
        />
      )}

      {/* v2.3.69 — Modal cobrar sub-cuenta (reusa ModalCobro) */}
      {cobrandoSubcuenta && (
        <ModalCobro
          total={cobrandoSubcuenta.total}
          onCobrar={(forma, fiado, extras) => {
            const sub = cobrandoSubcuenta;
            setCobrandoSubcuenta(null);
            handleCobrarSubcuenta(sub, forma, fiado, extras);
          }}
          onCancelar={() => setCobrandoSubcuenta(null)}
        />
      )}

      {/* v2.5.36: Modal post-cobro restaurante — emitir Factura SRI / aplicar retenciones */}
      {postCobro && (
        <div className="modal-overlay" onClick={() => {
          if (postCobroEmitiendo) return;
          setPostCobro(null);
          onCerrar(true);
        }}>
          <div className="modal-content" onClick={(e) => e.stopPropagation()} style={{ maxWidth: 480 }}>
            <div className="modal-header">
              <h3>Mesa cobrada</h3>
            </div>
            <div className="modal-body" style={{ textAlign: "center", padding: 20 }}>
              <div style={{ fontSize: 36, marginBottom: 8 }}>✓</div>
              <h3 style={{ marginBottom: 4 }}>
                {postCobroEstadoSri === "AUTORIZADA"
                  ? "Factura electrónica autorizada"
                  : `Nota de Venta ${postCobro.numero}`}
              </h3>
              <div className="text-secondary" style={{ fontSize: 12, marginBottom: 12 }}>
                Total: ${postCobro.total.toFixed(2)}
              </div>

              {postCobroEstadoSri === "AUTORIZADA" && (
                <div style={{
                  padding: "8px 12px", borderRadius: 6, marginBottom: 12,
                  background: "rgba(34,197,94,0.15)", color: "var(--color-success)", fontSize: 12,
                }}>
                  ✓ Factura electrónica autorizada por el SRI
                </div>
              )}
              {postCobroEmitiendo && (
                <div style={{ color: "var(--color-primary)", fontSize: 13, marginBottom: 12 }}>
                  Enviando al SRI...
                </div>
              )}

              <div style={{ display: "flex", flexDirection: "column", gap: 8, marginTop: 16 }}>
                {certificadoSriCargado && postCobroEstadoSri !== "AUTORIZADA" && !postCobroEmitiendo && (
                  <button className="btn btn-primary"
                    onClick={async () => {
                      if (!confirm(`¿Emitir factura electrónica para ${postCobro.numero}?\n\nSi el SRI autoriza, la nota de venta pasa a ser Factura.`)) return;
                      setPostCobroEmitiendo(true);
                      try {
                        const res = await emitirFacturaSri(postCobro.ventaId);
                        if (res.exito) {
                          toastExito("Factura autorizada por el SRI");
                          setPostCobroEstadoSri("AUTORIZADA");
                          window.dispatchEvent(new CustomEvent("sri-factura-emitida"));
                        } else {
                          toastError(`SRI: ${res.mensaje}`);
                        }
                      } catch (err) {
                        toastError("Error SRI: " + err);
                      } finally {
                        setPostCobroEmitiendo(false);
                      }
                    }}>
                    📄 Emitir Factura SRI
                  </button>
                )}

                {postCobroEstadoSri === "AUTORIZADA" && (
                  <button className="btn btn-outline" style={{ color: "#a855f7", borderColor: "#a855f7" }}
                    onClick={() => setPostCobroMostrarRet(true)}>
                    📋 Aplicar Retenciones SRI
                  </button>
                )}

                {postCobroEstadoSri === "AUTORIZADA" && (
                  <button className="btn btn-outline"
                    onClick={async () => {
                      try {
                        // intentar enviar al email del cliente si tiene
                        await enviarNotificacionSri(postCobro.ventaId, "");
                        toastExito("Email enviado");
                      } catch (err: any) {
                        toastError("No se pudo enviar email: " + (err?.message || err));
                      }
                    }}>
                    ✉ Notificar al cliente
                  </button>
                )}

                <button className="btn btn-outline" onClick={() => {
                  setPostCobro(null);
                  onCerrar(true);
                }}>
                  Cerrar
                </button>
              </div>
            </div>
          </div>
        </div>
      )}

      {/* v2.5.36: Modal de retenciones para la venta de restaurante */}
      {postCobro && postCobroMostrarRet && (
        <ModalRetenciones
          ventaId={postCobro.ventaId}
          numero={postCobro.numero}
          subtotal={postCobro.subtotal}
          iva={postCobro.iva}
          total={postCobro.total}
          totalCobrado={postCobro.total}
          onClose={() => setPostCobroMostrarRet(false)}
        />
      )}

      {/* v2.5.91 — Modal: registrar abono (pago parcial) */}
      {mostrarAbono && detalle && (
        <div className="modal-overlay" onClick={() => !abonoGuardando && setMostrarAbono(false)}>
          <div className="modal-content" style={{ maxWidth: 380 }} onClick={(e) => e.stopPropagation()}>
            <h3 style={{ marginTop: 0 }}>Registrar pago parcial</h3>
            <p className="text-secondary" style={{ fontSize: 12, marginTop: -6 }}>
              {detalle.mesa_nombre} · Saldo actual: <strong>${(detalle.saldo ?? detalle.total).toFixed(2)}</strong>
            </p>
            <label className="text-secondary" style={{ fontSize: 12 }}>Monto del abono</label>
            <input className="input" type="number" step="0.01" value={abonoMonto}
              onChange={(e) => setAbonoMonto(e.target.value)} placeholder="0.00" autoFocus />
            <label className="text-secondary" style={{ fontSize: 12, marginTop: 10, display: "block" }}>Forma de pago</label>
            <div style={{ display: "flex", gap: 6, flexWrap: "wrap", marginTop: 4 }}>
              {[["EFECTIVO", "Efectivo"], ["TRANSFER", "Transferencia"], ["TARJETA", "Tarjeta"]].map(([v, l]) => (
                <button key={v} type="button" className={`btn ${abonoForma === v ? "btn-primary" : "btn-outline"}`}
                  style={{ fontSize: 12, padding: "4px 12px" }} onClick={() => setAbonoForma(v)}>{l}</button>
              ))}
            </div>
            {abonoForma === "TRANSFER" && (
              <div style={{ marginTop: 8 }}>
                <select className="input" value={abonoBancoId ?? ""} onChange={(e) => setAbonoBancoId(e.target.value ? Number(e.target.value) : null)}>
                  <option value="">Selecciona cuenta…</option>
                  {cuentasAbono.map((c) => <option key={c.id} value={c.id}>{c.nombre}</option>)}
                </select>
              </div>
            )}
            {(abonoForma === "TRANSFER" || abonoForma === "TARJETA") && (
              <input className="input" style={{ marginTop: 8 }} value={abonoReferencia}
                onChange={(e) => setAbonoReferencia(e.target.value)}
                placeholder={abonoForma === "TARJETA" ? "Voucher (opcional)" : "Referencia (opcional)"} />
            )}
            <div style={{ display: "flex", gap: 8, marginTop: 16, justifyContent: "flex-end" }}>
              <button className="btn btn-secondary" disabled={abonoGuardando} onClick={() => setMostrarAbono(false)}>Cancelar</button>
              <button className="btn btn-primary" disabled={abonoGuardando} onClick={handleRegistrarAbono}>
                {abonoGuardando ? "Guardando…" : "Registrar abono"}
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

// ─── v2.3.68 — Modal unir mesas ──────────────────────────────────────────

interface PropsModalUnirMesas {
  pedidoId: number;
  mesaPrincipalNombre: string;
  onUnir: (mesasIds: number[]) => void;
  onCerrar: () => void;
}

function ModalUnirMesas({ pedidoId, mesaPrincipalNombre, onUnir, onCerrar }: PropsModalUnirMesas) {
  const { toastError } = useToast();
  const [mesas, setMesas] = useState<MesaResumen[]>([]);
  const [cargando, setCargando] = useState(true);
  const [seleccionadas, setSeleccionadas] = useState<Set<number>>(new Set());

  useEffect(() => {
    listarMesasLibresParaUnir(pedidoId)
      .then(setMesas)
      .catch((err: any) => toastError("Error cargando mesas: " + (err?.message || err)))
      .finally(() => setCargando(false));
  }, [pedidoId, toastError]);

  const toggle = (id: number) => {
    setSeleccionadas(prev => {
      const next = new Set(prev);
      if (next.has(id)) next.delete(id);
      else next.add(id);
      return next;
    });
  };

  // Agrupar mesas por zona para mejor UX
  const porZona = mesas.reduce((acc, m) => {
    const zona = m.zona_nombre || "Sin zona";
    if (!acc[zona]) acc[zona] = [];
    acc[zona].push(m);
    return acc;
  }, {} as Record<string, MesaResumen[]>);

  const capacidadExtra = Array.from(seleccionadas)
    .map(id => mesas.find(m => m.id === id)?.capacidad ?? 0)
    .reduce((a, b) => a + b, 0);

  return (
    <div className="modal-overlay" onClick={onCerrar}>
      <div
        className="modal-content"
        onClick={e => e.stopPropagation()}
        style={{ maxWidth: 560, width: "100%", maxHeight: "85vh", display: "flex", flexDirection: "column" }}
      >
        <div className="modal-header" style={{ borderBottom: "1px solid var(--color-border)", padding: "14px 18px" }}>
          <div>
            <h3 style={{ margin: 0, fontSize: 17 }}>🔗 Unir mesas a {mesaPrincipalNombre}</h3>
            <div style={{ fontSize: 12, color: "var(--color-text-muted)", marginTop: 4 }}>
              Para grupos grandes que ocupan varias mesas. Las mesas seleccionadas se liberan
              automáticamente al cobrar.
            </div>
          </div>
        </div>
        <div className="modal-body" style={{ overflowY: "auto", padding: "12px 18px", flex: 1 }}>
          {cargando ? (
            <div style={{ padding: 24, textAlign: "center", color: "var(--color-text-muted)" }}>
              Cargando mesas libres...
            </div>
          ) : mesas.length === 0 ? (
            <div
              style={{
                padding: 24,
                textAlign: "center",
                color: "var(--color-text-muted)",
                border: "2px dashed var(--color-border)",
                borderRadius: 8,
              }}
            >
              No hay mesas libres disponibles para unir.
            </div>
          ) : (
            <div style={{ display: "flex", flexDirection: "column", gap: 12 }}>
              {Object.entries(porZona).map(([zona, ms]) => (
                <div key={zona}>
                  <div style={{
                    fontSize: 11,
                    fontWeight: 700,
                    textTransform: "uppercase",
                    letterSpacing: 0.5,
                    color: "var(--color-text-muted)",
                    marginBottom: 6,
                  }}>
                    {zona}
                  </div>
                  <div style={{
                    display: "grid",
                    gridTemplateColumns: "repeat(auto-fill, minmax(120px, 1fr))",
                    gap: 8,
                  }}>
                    {ms.map(m => {
                      const sel = seleccionadas.has(m.id);
                      return (
                        <button
                          key={m.id}
                          onClick={() => toggle(m.id)}
                          style={{
                            padding: "12px 8px",
                            background: sel ? "var(--color-primary)" : "var(--color-surface)",
                            color: sel ? "#fff" : "var(--color-text)",
                            border: sel ? "2px solid var(--color-primary)" : "2px solid var(--color-border)",
                            borderRadius: 8,
                            cursor: "pointer",
                            fontWeight: 600,
                            fontSize: 13,
                            transition: "transform 0.05s, background 0.1s",
                          }}
                        >
                          {sel && "✓ "}{m.nombre}
                          <div style={{
                            fontSize: 10,
                            fontWeight: 400,
                            opacity: 0.85,
                            marginTop: 2,
                          }}>
                            cap. {m.capacidad}
                          </div>
                        </button>
                      );
                    })}
                  </div>
                </div>
              ))}
            </div>
          )}
        </div>
        <div className="modal-footer" style={{
          borderTop: "1px solid var(--color-border)",
          padding: "12px 18px",
          display: "flex",
          gap: 8,
          justifyContent: "space-between",
          alignItems: "center",
        }}>
          <div style={{ fontSize: 12, color: "var(--color-text-muted)" }}>
            {seleccionadas.size > 0
              ? `${seleccionadas.size} mesa(s) · +${capacidadExtra} comensales`
              : "Selecciona una o más mesas"}
          </div>
          <div style={{ display: "flex", gap: 8 }}>
            <button className="btn btn-outline" onClick={onCerrar}>
              Cancelar
            </button>
            <button
              onClick={() => onUnir(Array.from(seleccionadas))}
              disabled={seleccionadas.size === 0}
              style={{
                padding: "8px 16px",
                background: seleccionadas.size === 0 ? "var(--color-border)" : "var(--color-primary)",
                color: "#fff",
                border: "none",
                borderRadius: 6,
                fontWeight: 700,
                cursor: seleccionadas.size === 0 ? "not-allowed" : "pointer",
              }}
            >
              🔗 Unir {seleccionadas.size > 0 ? `(${seleccionadas.size})` : ""}
            </button>
          </div>
        </div>
      </div>
    </div>
  );
}

// ─── v2.3.69 — Modal dividir cuenta ──────────────────────────────────────

interface PropsModalDividirCuenta {
  totalPedido: number;
  comensales: number;
  onDividir: (nPartes: number) => void;
  onCerrar: () => void;
}

function ModalDividirCuenta({ totalPedido, comensales, onDividir, onCerrar }: PropsModalDividirCuenta) {
  // Default: dividir entre el número de comensales del pedido
  const [nPartes, setNPartes] = useState<number>(Math.max(2, Math.min(20, comensales || 2)));

  const montoPorPersona = totalPedido / nPartes;

  const incrementar = () => setNPartes(n => Math.min(20, n + 1));
  const decrementar = () => setNPartes(n => Math.max(2, n - 1));

  return (
    <div className="modal-overlay" onClick={onCerrar}>
      <div
        className="modal-content"
        onClick={e => e.stopPropagation()}
        style={{ maxWidth: 420 }}
      >
        <div className="modal-header" style={{ borderBottom: "1px solid var(--color-border)", padding: "14px 18px" }}>
          <h3 style={{ margin: 0, fontSize: 17 }}>✂️ Dividir cuenta</h3>
          <div style={{ fontSize: 12, color: "var(--color-text-muted)", marginTop: 4 }}>
            Cada parte se cobra de forma independiente con su propia forma de pago.
          </div>
        </div>
        <div className="modal-body" style={{ padding: 18, display: "flex", flexDirection: "column", gap: 16 }}>
          <div style={{ textAlign: "center" }}>
            <div style={{ fontSize: 12, color: "var(--color-text-muted)", marginBottom: 4 }}>Total pedido</div>
            <strong style={{ fontSize: 22, color: "var(--color-primary)" }}>
              ${totalPedido.toFixed(2)}
            </strong>
          </div>

          <div>
            <label style={{ fontSize: 13, fontWeight: 600, display: "block", marginBottom: 8 }}>
              Número de partes
            </label>
            <div style={{ display: "flex", alignItems: "center", justifyContent: "center", gap: 10 }}>
              <button
                onClick={decrementar}
                disabled={nPartes <= 2}
                style={{
                  width: 38,
                  height: 38,
                  background: nPartes <= 2 ? "var(--color-border)" : "var(--color-surface)",
                  border: "1.5px solid var(--color-border)",
                  borderRadius: 8,
                  fontSize: 20,
                  fontWeight: 700,
                  cursor: nPartes <= 2 ? "not-allowed" : "pointer",
                  color: "var(--color-text)",
                }}
              >
                −
              </button>
              <input
                type="number"
                min="2"
                max="20"
                value={nPartes}
                onChange={(e) => {
                  const v = parseInt(e.target.value, 10);
                  if (!isNaN(v)) setNPartes(Math.max(2, Math.min(20, v)));
                }}
                className="input"
                style={{
                  width: 80,
                  fontSize: 22,
                  textAlign: "center",
                  fontWeight: 700,
                  padding: "6px 4px",
                }}
              />
              <button
                onClick={incrementar}
                disabled={nPartes >= 20}
                style={{
                  width: 38,
                  height: 38,
                  background: nPartes >= 20 ? "var(--color-border)" : "var(--color-surface)",
                  border: "1.5px solid var(--color-border)",
                  borderRadius: 8,
                  fontSize: 20,
                  fontWeight: 700,
                  cursor: nPartes >= 20 ? "not-allowed" : "pointer",
                  color: "var(--color-text)",
                }}
              >
                +
              </button>
            </div>
          </div>

          <div style={{
            background: "var(--color-surface-hover)",
            padding: "12px 16px",
            borderRadius: 8,
            display: "flex",
            justifyContent: "space-between",
            alignItems: "center",
          }}>
            <span style={{ fontSize: 13, fontWeight: 600 }}>Cada parte paga</span>
            <strong style={{ fontSize: 20, color: "var(--color-success)" }}>
              ${montoPorPersona.toFixed(2)}
            </strong>
          </div>

          <div style={{
            fontSize: 11,
            color: "var(--color-text-muted)",
            background: "rgba(245, 158, 11, 0.08)",
            border: "1px solid rgba(245, 158, 11, 0.3)",
            padding: "8px 10px",
            borderRadius: 6,
            lineHeight: 1.4,
          }}>
            ⚠️ <strong>Nota:</strong> Cada sub-cuenta genera una nota de venta independiente.
            La división puede cancelarse mientras NINGUNA sub-cuenta esté cobrada.
          </div>
        </div>
        <div className="modal-footer" style={{
          borderTop: "1px solid var(--color-border)",
          padding: "12px 18px",
          display: "flex",
          gap: 8,
          justifyContent: "flex-end",
        }}>
          <button className="btn btn-outline" onClick={onCerrar}>
            Cancelar
          </button>
          <button
            onClick={() => onDividir(nPartes)}
            style={{
              padding: "8px 18px",
              background: "var(--color-primary)",
              color: "#fff",
              border: "none",
              borderRadius: 6,
              fontWeight: 700,
              cursor: "pointer",
            }}
          >
            ✂️ Dividir en {nPartes}
          </button>
        </div>
      </div>
    </div>
  );
}
