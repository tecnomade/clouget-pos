import { useState, useEffect } from "react";
import { obtenerVenta, registrarNotaCredito, emitirNotaCreditoSri, verificarPinAdmin } from "../services/api";
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
  onClose: () => void;
  onCreada: () => void;
  toastExito: (msg: string) => void;
  toastError: (msg: string) => void;
  toastWarning: (msg: string) => void;
}

export default function ModalNotaCredito({
  ventaId,
  ventaNumero,
  onClose,
  onCreada,
  toastExito,
  toastError,
  toastWarning,
}: ModalNotaCreditoProps) {
  const [pinAdmin, setPinAdmin] = useState("");
  const [pinError, setPinError] = useState("");
  const [pinVerificado, setPinVerificado] = useState(false);
  const [verificandoPin, setVerificandoPin] = useState(false);

  const [cargando, setCargando] = useState(true);
  const [items, setItems] = useState<ItemNC[]>([]);
  const [motivo, setMotivo] = useState("");
  const [procesando, setProcesando] = useState(false);
  const [error, setError] = useState("");

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

  const handleCrear = async () => {
    if (!motivo.trim()) {
      setError("Ingrese el motivo de la nota de credito");
      return;
    }
    if (itemsIncluidos.length === 0) {
      setError("Seleccione al menos un item");
      return;
    }
    setError("");
    setProcesando(true);

    try {
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
    } catch (err) {
      toastError("Error creando NC: " + err);
    } finally {
      setProcesando(false);
    }
  };

  // Paso 1: Verificar PIN admin
  if (!pinVerificado) {
    return (
      <div className="modal-overlay" onClick={onClose}>
        <div className="modal-content" onClick={(e) => e.stopPropagation()} style={{ maxWidth: 380 }}>
          <div className="modal-header">
            <h3>Autorizacion Requerida</h3>
          </div>
          <div className="modal-body">
            <p style={{ fontSize: 13, color: "#64748b", marginBottom: 12 }}>
              Para crear una nota de credito de la factura <strong>{ventaNumero}</strong>,
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
            {pinError && <div style={{ color: "#dc2626", fontSize: 12, marginTop: 6 }}>{pinError}</div>}
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

  // Paso 3: Formulario NC
  return (
    <div className="modal-overlay" onClick={onClose}>
      <div className="modal-content" onClick={(e) => e.stopPropagation()} style={{ maxWidth: 650 }}>
        <div className="modal-header">
          <h3>Nota de Credito</h3>
          <span className="text-secondary" style={{ fontSize: 12 }}>
            Factura: {ventaNumero}
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
              placeholder="Ej: Devolucion de producto, Error en facturacion..."
              value={motivo}
              onChange={(e) => { setMotivo(e.target.value); setError(""); }}
              autoFocus
              disabled={procesando}
            />
          </div>

          {/* Items */}
          <div style={{ marginBottom: 12 }}>
            <label style={{ fontSize: 12, fontWeight: 600, marginBottom: 4, display: "block" }}>
              Items a incluir en la NC
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
          <div style={{ background: "#f8fafc", padding: 12, borderRadius: 6, fontSize: 13 }}>
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
              <span>TOTAL NC:</span>
              <span style={{ color: "#dc2626" }}>${total.toFixed(2)}</span>
            </div>
          </div>

          {error && <div style={{ color: "#dc2626", fontSize: 12, marginTop: 8 }}>{error}</div>}
        </div>
        <div className="modal-footer">
          <button className="btn btn-outline" onClick={onClose} disabled={procesando}>
            Cancelar
          </button>
          <button
            className="btn btn-primary"
            onClick={handleCrear}
            disabled={procesando || itemsIncluidos.length === 0}
            style={{ background: "#dc2626", borderColor: "#dc2626" }}
          >
            {procesando ? "Procesando..." : "Crear Nota de Credito"}
          </button>
        </div>
      </div>
    </div>
  );
}
