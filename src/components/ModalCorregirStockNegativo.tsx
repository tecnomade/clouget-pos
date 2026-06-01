/**
 * v2.5.71 — Corrección en lote de productos con stock negativo.
 *
 * Se abre desde el aviso rojo "X productos con stock NEGATIVO". Carga los
 * productos con stock < 0 pre-seleccionados, permite escribir el stock real
 * (contado físicamente) de cada uno y aplicar el ajuste en lote con una
 * explicación. Cada ajuste queda registrado en el kardex (auditable).
 */
import { useState, useEffect } from "react";
import { listarProductosStockNegativo, ajustarStockLote, type ProductoStockNegativo } from "../services/api";
import { useToast } from "./Toast";

interface Props {
  onClose: () => void;
  onAplicado: () => void;
}

interface Fila extends ProductoStockNegativo {
  stock_real: number;     // valor a fijar (por defecto 0)
  incluir: boolean;
}

export default function ModalCorregirStockNegativo({ onClose, onAplicado }: Props) {
  const { toastExito, toastError } = useToast();
  const [filas, setFilas] = useState<Fila[]>([]);
  const [cargando, setCargando] = useState(true);
  const [motivo, setMotivo] = useState("Corrección de stock negativo (ventas sin stock registrado)");
  const [aplicando, setAplicando] = useState(false);

  useEffect(() => {
    listarProductosStockNegativo()
      .then((ps) => setFilas(ps.map((p) => ({ ...p, stock_real: 0, incluir: true }))))
      .catch((e) => toastError("Error cargando productos: " + e))
      .finally(() => setCargando(false));
  }, []);

  const setReal = (id: number, val: number) =>
    setFilas((f) => f.map((x) => (x.id === id ? { ...x, stock_real: val } : x)));
  const toggle = (id: number) =>
    setFilas((f) => f.map((x) => (x.id === id ? { ...x, incluir: !x.incluir } : x)));

  const aplicar = async () => {
    const seleccionados = filas.filter((f) => f.incluir);
    if (seleccionados.length === 0) { toastError("Selecciona al menos un producto"); return; }
    if (!motivo.trim()) { toastError("Escribe una explicación del ajuste"); return; }
    setAplicando(true);
    try {
      const n = await ajustarStockLote(
        seleccionados.map((f) => ({ producto_id: f.id, stock_real: f.stock_real })),
        motivo.trim()
      );
      toastExito(`${n} producto${n !== 1 ? "s" : ""} corregido${n !== 1 ? "s" : ""}`);
      onAplicado();
    } catch (e) {
      toastError("Error al ajustar: " + e);
    } finally {
      setAplicando(false);
    }
  };

  return (
    <div style={{ position: "fixed", inset: 0, background: "rgba(0,0,0,0.5)", display: "flex", alignItems: "center", justifyContent: "center", zIndex: 200 }}
      onClick={(e) => { if (e.target === e.currentTarget) onClose(); }}>
      <div className="card" style={{ width: 680, maxHeight: "90vh", overflow: "auto" }}>
        <div className="card-header flex justify-between items-center">
          <span>⚠ Corregir stock negativo</span>
          <button className="btn btn-outline" style={{ padding: "2px 8px" }} onClick={onClose}>x</button>
        </div>
        <div className="card-body">
          <div style={{ fontSize: 12, color: "var(--color-text-secondary)", background: "rgba(59,130,246,0.08)", border: "1px solid rgba(59,130,246,0.25)", borderRadius: 6, padding: 10, marginBottom: 14 }}>
            Estos productos tienen <strong>stock negativo</strong> (se vendieron más unidades de las que había registradas).
            Cuenta las unidades que <strong>realmente tienes</strong> en bodega y escríbelas en <strong>"Stock real"</strong>.
            Si no tienes ninguna, deja <strong>0</strong>. Se registrará un ajuste auditable en el kardex.
          </div>

          {cargando ? (
            <div style={{ padding: 24, textAlign: "center", color: "var(--color-text-secondary)" }}>Cargando...</div>
          ) : filas.length === 0 ? (
            <div style={{ padding: 24, textAlign: "center", color: "var(--color-success)", fontWeight: 600 }}>
              ✓ No hay productos con stock negativo. Todo en orden.
            </div>
          ) : (
            <>
              <table className="table" style={{ fontSize: 13, marginBottom: 14 }}>
                <thead>
                  <tr>
                    <th style={{ width: 30 }}></th>
                    <th>Producto</th>
                    <th className="text-right" style={{ width: 90 }}>Stock actual</th>
                    <th className="text-right" style={{ width: 120 }}>Stock real</th>
                  </tr>
                </thead>
                <tbody>
                  {filas.map((f) => (
                    <tr key={f.id} style={{ opacity: f.incluir ? 1 : 0.45 }}>
                      <td><input type="checkbox" checked={f.incluir} onChange={() => toggle(f.id)} /></td>
                      <td>
                        {f.nombre}
                        {f.codigo ? <span className="text-secondary" style={{ fontSize: 11 }}> ({f.codigo})</span> : null}
                      </td>
                      <td className="text-right" style={{ color: "var(--color-danger)", fontWeight: 700 }}>{f.stock_actual}</td>
                      <td className="text-right">
                        <input type="number" className="input" style={{ width: 100, fontSize: 13, textAlign: "right" }}
                          min={0} step="any" value={f.stock_real} disabled={!f.incluir}
                          onChange={(e) => setReal(f.id, parseFloat(e.target.value) || 0)} />
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>

              <label style={{ fontSize: 11, color: "var(--color-text-secondary)", display: "block", marginBottom: 4 }}>Explicación del ajuste (queda en el historial)</label>
              <textarea className="input" style={{ width: "100%", fontSize: 13, minHeight: 54, marginBottom: 14, resize: "vertical" }}
                value={motivo} onChange={(e) => setMotivo(e.target.value)} />

              <button className="btn btn-primary" style={{ width: "100%", padding: "10px 0", fontWeight: 700 }}
                disabled={aplicando} onClick={aplicar}>
                {aplicando ? "Aplicando..." : `Aplicar ajuste a ${filas.filter((f) => f.incluir).length} producto(s)`}
              </button>
            </>
          )}
        </div>
      </div>
    </div>
  );
}
