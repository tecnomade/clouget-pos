import { useState, useEffect } from "react";
import { obtenerVenta, registrarNotaCredito, emitirNotaCreditoSri, verificarPinAdmin, crearDevolucionInterna } from "../services/api";
import { useSesion } from "../contexts/SesionContext";
import type { VentaCompleta, VentaDetalle } from "../types";

interface ItemNC {
  incluido: boolean;
  producto_id: number;
  nombre_producto: string;
  cantidad_original: number;
  cantidad: number;
  precio_unitario: number;
  descuento: number;
  iva_porcentaje: number;
}

interface ModalNotaCreditoProps {
  ventaId: number;
  ventaNumero: string;
  esDevolucionInterna?: boolean; // true for NOTA_VENTA returns
  onClose: () => void;
  onCreada: () => void;
  toastExito: (msg: string) => void;
  toastError: (msg: string) => void;
  toastWarning: (msg: string) => void;
}

export default function ModalNotaCredito({
  ventaId,
  ventaNumero,
  esDevolucionInterna = false,
  onClose,
  onCreada,
  toastExito,
  toastError,
  toastWarning,
}: ModalNotaCreditoProps) {
  const { esAdmin, tienePermiso } = useSesion();
  const [pinAdmin, setPinAdmin] = useState("");
  const [pinError, setPinError] = useState("");
  const [pinVerificado, setPinVerificado] = useState(false);
  const [verificandoPin, setVerificandoPin] = useState(false);

  const [cargando, setCargando] = useState(true);
  const [items, setItems] = useState<ItemNC[]>([]);
  const [motivo, setMotivo] = useState("");
  const [procesando, setProcesando] = useState(false);
  const [error, setError] = useState("");

  // If user is admin or has crear_nota_credito permission, skip PIN
  const tienePermisoNC = esAdmin || tienePermiso("crear_nota_credito");

  const handleVerificarPin = async () => {
    if (!pinAdmin.trim()) {
      setPinError("Ingrese el PIN");
      return;
    }
    setVerificandoPin(true);
    setPinError("");
    try {
      await verificarPinAdmin(pinAdmin);
      setPinVerificado(true);
    } catch {
      setPinError("PIN de administrador incorrecto");
    } finally {
      setVerificandoPin(false);
    }
  };

  // If user has permission, auto-verify
  useEffect(() => {
    if (tienePermisoNC) {
      setPinVerificado(true);
    }
  }, [tienePermisoNC]);

  useEffect(() => {
    if (!pinVerificado) return;
    obtenerVenta(ventaId)
      .then((vc: VentaCompleta) => {
        setItems(
          vc.detalles.map((d) => ({
            incluido: true,
            producto_id: d.producto_id,
            nombre_producto: d.nombre_producto || `Producto #${d.producto_id}`,
            cantidad_original: d.cantidad,
            cantidad: d.cantidad,
            precio_unitario: d.precio_unitario,
            descuento: d.descuento,
            iva_porcentaje: d.iva_porcentaje,
          }))
        );
        setCargando(false);
      })
      .catch((err: unknown) => {
        toastError("Error cargando venta: " + err);
        onClose();
      });
  }, [ventaId, pinVerificado]);

  const toggleItem = (idx: number) => {
    setItems((prev) =>
      prev.map((it, i) => (i === idx ? { ...it, incluido: !it.incluido } : it))
    );
  };

  const setCantidad = (idx: number, val: number) => {
    setItems((prev) =>
      prev.map((it, i) =>
        i === idx
          ? { ...it, cantidad: Math.max(0.01, Math.min(val, it.cantidad_original)) }
          : it
      )
    );
  };

  const itemsIncluidos = items.filter((it) => it.incluido);

  const calcularSubtotal = (it: ItemNC) => {
    const base = it.cantidad * it.precio_unitario;
    return base - it.descuento;
  };

  const totalSinIva = itemsIncluidos
    .filter((it) => it.iva_porcentaje === 0)
    .reduce((sum, it) => sum + calcularSubtotal(it), 0);

  const totalConIva = itemsIncluidos
    .filter((it) => it.iva_porcentaje > 0)
    .reduce((sum, it) => sum + calcularSubtotal(it), 0);

  const totalIva = itemsIncluidos
    .filter((it) => it.iva_porcentaje > 0)
    .reduce((sum, it) => sum + calcularSubtotal(it) * (it.iva_porcentaje / 100), 0);

  const total = totalSinIva + totalConIva + totalIva;

  const titulo = esDevolucionInterna ? "Devolucion" : "Nota de Credito";
  const tituloRef = esDevolucionInterna ? "Nota de Venta" : "Factura";

  const handleCrear = async () => {
    if (!motivo.trim()) {
      setError(`Ingrese el motivo de la ${titulo.toLowerCase()}`);
      return;
    }
    if (itemsIncluidos.length === 0) {
      setError("Seleccione al menos un item");
      return;
    }
    setError("");
    setProcesando(true);

    try {
      if (esDevolucionInterna) {
        // Internal return for NOTA_VENTA
        const itemsData = itemsIncluidos.map((it) => ({
          producto_id: it.producto_id,
          cantidad: it.cantidad,
          precio_unitario: it.precio_unitario,
          descuento: it.descuento,
          iva_porcentaje: it.iva_porcentaje,
        }));

        const nc = await crearDevolucionInterna(ventaId, motivo.trim(), itemsData);

        // Mensaje claro al usuario segun como se devolvio el dinero.
        // Backend retorna monto_efectivo_devuelto / monto_transfer_devuelto / monto_credito_devuelto
        // y retiro_caja_creado_id si auto-genero el retiro de efectivo.
        const efectivo = (nc as any).monto_efectivo_devuelto ?? 0;
        const transfer = (nc as any).monto_transfer_devuelto ?? 0;
        const credito = (nc as any).monto_credito_devuelto ?? 0;
        const retiroId = (nc as any).retiro_caja_creado_id;

        let mensajePartes: string[] = [`✓ Devolución ${nc.numero} creada por $${(nc as any).total?.toFixed(2)}`];
        if (efectivo > 0.01) {
          mensajePartes.push(retiroId
            ? `💵 Se descontó $${efectivo.toFixed(2)} de la caja automáticamente (entregaste el efectivo al cliente).`
            : `⚠ $${efectivo.toFixed(2)} en efectivo a devolver — pero no hay caja abierta para descontarlo.`
          );
        }
        if (transfer > 0.01) {
          mensajePartes.push(`🏦 $${transfer.toFixed(2)} fue por transferencia — debes hacer la devolución al cliente desde la app del banco. La caja no se modifica.`);
        }
        if (credito > 0.01) {
          mensajePartes.push(`📋 $${credito.toFixed(2)} era a crédito — el saldo del cliente se reduce solo, no se devuelve dinero.`);
        }

        // Mostrar como confirm para que el cajero lea el mensaje completo
        if (mensajePartes.length > 1) {
          alert(mensajePartes.join("\n\n"));
        } else {
          toastExito(mensajePartes[0]);
        }
        onCreada();
        onClose();
      } else {
        // SRI NC flow for FACTURA
        const detalles: VentaDetalle[] = itemsIncluidos.map((it) => ({
          producto_id: it.producto_id,
          nombre_producto: it.nombre_producto,
          cantidad: it.cantidad,
          precio_unitario: it.precio_unitario,
          descuento: it.descuento,
          iva_porcentaje: it.iva_porcentaje,
          subtotal: calcularSubtotal(it),
        }));

        const nc = await registrarNotaCredito({
          venta_id: ventaId,
          motivo: motivo.trim(),
          items: detalles,
        });

        toastExito(`Nota de credito ${nc.numero} creada`);

        // Intentar emitir al SRI
        try {
          const res = await emitirNotaCreditoSri(nc.id);
          if (res.exito) {
            toastExito("NC autorizada por el SRI");
          } else {
            toastWarning(`SRI NC: ${res.mensaje}`);
          }
        } catch (err) {
          toastWarning("NC creada pero error al emitir al SRI: " + err);
        }

        onCreada();
        onClose();
      }
    } catch (err) {
      toastError(`Error creando ${titulo.toLowerCase()}: ${err}`);
    } finally {
      setProcesando(false);
    }
  };

  // Paso 1: Verificar PIN admin (only if user doesn't have permission)
  if (!pinVerificado) {
    return (
      <div className="modal-overlay" onClick={onClose}>
        <div className="modal-content" onClick={(e) => e.stopPropagation()} style={{ maxWidth: 380 }}>
          <div className="modal-header">
            <h3>Autorizacion Requerida</h3>
          </div>
          <div className="modal-body">
            <p style={{ fontSize: 13, color: "var(--color-text-secondary)", marginBottom: 12 }}>
              Para crear una {titulo.toLowerCase()} de la {tituloRef.toLowerCase()} <strong>{ventaNumero}</strong>,
              ingrese el PIN de administrador.
            </p>
            <input
              className="input"
              type="password"
              placeholder="PIN administrador"
              value={pinAdmin}
              onChange={(e) => { setPinAdmin(e.target.value); setPinError(""); }}
              onKeyDown={(e) => { if (e.key === "Enter") handleVerificarPin(); }}
              autoFocus
              disabled={verificandoPin}
              style={{ textAlign: "center", fontSize: 18, letterSpacing: 8 }}
            />
            {pinError && <div style={{ color: "var(--color-danger)", fontSize: 12, marginTop: 6 }}>{pinError}</div>}
          </div>
          <div className="modal-footer">
            <button className="btn btn-outline" onClick={onClose} disabled={verificandoPin}>
              Cancelar
            </button>
            <button className="btn btn-primary" onClick={handleVerificarPin} disabled={verificandoPin || !pinAdmin.trim()}>
              {verificandoPin ? "Verificando..." : "Verificar"}
            </button>
          </div>
        </div>
      </div>
    );
  }

  // Paso 2: Cargando datos
  if (cargando) {
    return (
      <div className="modal-overlay" onClick={onClose}>
        <div className="modal-content" onClick={(e) => e.stopPropagation()} style={{ maxWidth: 600 }}>
          <div className="modal-body" style={{ padding: 40, textAlign: "center" }}>
            Cargando datos de la venta...
          </div>
        </div>
      </div>
    );
  }

  // Paso 3: Formulario NC / Devolucion
  return (
    <div className="modal-overlay" onClick={onClose}>
      <div className="modal-content" onClick={(e) => e.stopPropagation()} style={{ maxWidth: 650 }}>
        <div className="modal-header">
          <h3>{titulo}</h3>
          <span className="text-secondary" style={{ fontSize: 12 }}>
            {tituloRef}: {ventaNumero}
          </span>
        </div>
        <div className="modal-body">
          {/* Motivo */}
          <div style={{ marginBottom: 12 }}>
            <label style={{ fontSize: 12, fontWeight: 600, marginBottom: 4, display: "block" }}>
              Motivo *
            </label>
            <input
              className="input"
              placeholder={esDevolucionInterna
                ? "Ej: Devolucion de producto, Producto defectuoso..."
                : "Ej: Devolucion de producto, Error en facturacion..."}
              value={motivo}
              onChange={(e) => { setMotivo(e.target.value); setError(""); }}
              autoFocus
              disabled={procesando}
            />
          </div>

          {/* Items */}
          <div style={{ marginBottom: 12 }}>
            <label style={{ fontSize: 12, fontWeight: 600, marginBottom: 4, display: "block" }}>
              Items a {esDevolucionInterna ? "devolver" : "incluir en la NC"}
            </label>
            <div style={{ maxHeight: 250, overflow: "auto", border: "1px solid var(--color-border)", borderRadius: 6 }}>
              <table className="table" style={{ fontSize: 12 }}>
                <thead>
                  <tr>
                    <th style={{ width: 30 }}></th>
                    <th>Producto</th>
                    <th style={{ width: 80 }}>Cantidad</th>
                    <th className="text-right" style={{ width: 70 }}>P. Unit</th>
                    <th className="text-right" style={{ width: 80 }}>Subtotal</th>
                  </tr>
                </thead>
                <tbody>
                  {items.map((it, idx) => (
                    <tr key={idx} style={{ opacity: it.incluido ? 1 : 0.4 }}>
                      <td>
                        <input
                          type="checkbox"
                          checked={it.incluido}
                          onChange={() => toggleItem(idx)}
                          disabled={procesando}
                        />
                      </td>
                      <td>{it.nombre_producto}</td>
                      <td>
                        <input
                          className="input"
                          type="number"
                          min={0.01}
                          max={it.cantidad_original}
                          step={1}
                          value={it.cantidad}
                          onChange={(e) => setCantidad(idx, parseFloat(e.target.value) || 0)}
                          disabled={!it.incluido || procesando}
                          style={{ width: 65, fontSize: 12, padding: "2px 4px", textAlign: "center" }}
                        />
                        <span className="text-secondary" style={{ fontSize: 10 }}>/{it.cantidad_original}</span>
                      </td>
                      <td className="text-right">${it.precio_unitario.toFixed(2)}</td>
                      <td className="text-right font-bold">
                        {it.incluido ? `$${calcularSubtotal(it).toFixed(2)}` : "-"}
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          </div>

          {/* Totales */}
          <div style={{ background: "var(--color-surface-alt)", padding: 12, borderRadius: 6, fontSize: 13 }}>
            <div className="flex justify-between">
              <span>Subtotal 0%:</span>
              <span>${totalSinIva.toFixed(2)}</span>
            </div>
            <div className="flex justify-between">
              <span>Subtotal IVA:</span>
              <span>${totalConIva.toFixed(2)}</span>
            </div>
            <div className="flex justify-between">
              <span>IVA:</span>
              <span>${totalIva.toFixed(2)}</span>
            </div>
            <div className="flex justify-between font-bold" style={{ borderTop: "1px solid var(--color-border)", paddingTop: 6, marginTop: 6 }}>
              <span>TOTAL {esDevolucionInterna ? "DEVOLUCION" : "NC"}:</span>
              <span style={{ color: "var(--color-danger)" }}>${total.toFixed(2)}</span>
            </div>
          </div>

          {esDevolucionInterna && (
            <div style={{ fontSize: 11, color: "var(--color-text-secondary)", marginTop: 8, fontStyle: "italic" }}>
              Esta devolucion es interna (no se envia al SRI).
            </div>
          )}

          {error && <div style={{ color: "var(--color-danger)", fontSize: 12, marginTop: 8 }}>{error}</div>}
        </div>
        <div className="modal-footer">
          <button className="btn btn-outline" onClick={onClose} disabled={procesando}>
            Cancelar
          </button>
          <button
            className="btn btn-primary"
            onClick={handleCrear}
            disabled={procesando || itemsIncluidos.length === 0}
            style={{ background: "var(--color-danger)", borderColor: "var(--color-danger)" }}
          >
            {procesando ? "Procesando..." : esDevolucionInterna ? "Crear Devolucion" : "Crear Nota de Credito"}
          </button>
        </div>
      </div>
    </div>
  );
}
