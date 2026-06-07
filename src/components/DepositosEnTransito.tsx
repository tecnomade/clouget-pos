import { useState, useEffect, useCallback, Fragment } from "react";
import { listarDepositosEnTransito, confirmarDeposito } from "../services/api";
import { useToast } from "./Toast";
import { useSesion } from "../contexts/SesionContext";
import { comprimirImagen } from "../utils/imagen";

/**
 * Panel central de "Depósitos en tránsito": lista TODOS los retiros a banco
 * que un cajero hizo pero que aún no se confirman (estado EN_TRANSITO), de
 * cualquier caja (abierta o cerrada). Permite confirmarlos (referencia +
 * comprobante) → DEPOSITADO.
 *
 * Gateado por el permiso `confirmar_depositos` (o ADMIN). Si el usuario no lo
 * tiene, no renderiza nada. Se usa en Bancos, Reportes y Caja.
 */
export default function DepositosEnTransito({ compacto = false }: { compacto?: boolean }) {
  const { esAdmin, tienePermiso } = useSesion();
  const { toastExito, toastError } = useToast();
  const puede = esAdmin || tienePermiso("confirmar_depositos");

  const [items, setItems] = useState<any[] | null>(null);
  const [confirmandoId, setConfirmandoId] = useState<number | null>(null);
  const [ref, setRef] = useState("");
  const [img, setImg] = useState<string | null>(null);
  const [guardando, setGuardando] = useState(false);

  const cargar = useCallback(() => {
    if (!puede) return;
    listarDepositosEnTransito().then(setItems).catch(() => setItems([]));
  }, [puede]);

  useEffect(() => { cargar(); }, [cargar]);

  const onImg = async (e: React.ChangeEvent<HTMLInputElement>) => {
    const file = e.target.files?.[0];
    if (!file) return;
    // Comprime fotos grandes (celular) en vez de rechazarlas
    try { setImg(await comprimirImagen(file)); }
    catch { toastError("No se pudo procesar la imagen"); }
  };

  const confirmar = async () => {
    if (!ref.trim()) { toastError("Ingrese el número de comprobante / referencia"); return; }
    try {
      setGuardando(true);
      await confirmarDeposito(confirmandoId!, ref.trim(), img || undefined);
      toastExito("Depósito confirmado");
      setConfirmandoId(null); setRef(""); setImg(null);
      cargar();
      window.dispatchEvent(new CustomEvent("clouget:caja-cambio", { detail: { evento: "deposito" } }));
    } catch (err) {
      toastError("Error: " + err);
    } finally {
      setGuardando(false);
    }
  };

  if (!puede) return null;

  const total = (items ?? []).reduce((s, r) => s + (r.monto || 0), 0);

  return (
    <div className="card" style={{ marginBottom: compacto ? 12 : 16, borderColor: (items && items.length > 0) ? "var(--color-warning)" : undefined }}>
      <div className="card-header" style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
        <span>🏦 Depósitos en tránsito {items ? `(${items.length})` : ""}</span>
        {items && items.length > 0 && (
          <span style={{ fontWeight: 700, color: "var(--color-warning)" }}>${total.toFixed(2)}</span>
        )}
      </div>
      <div className="card-body" style={{ padding: 0, overflowX: "auto" }}>
        {!items ? (
          <div style={{ padding: 20, textAlign: "center", color: "var(--color-text-secondary)" }}>Cargando...</div>
        ) : items.length === 0 ? (
          <div style={{ padding: 20, textAlign: "center", color: "var(--color-text-secondary)" }}>
            No hay depósitos pendientes de confirmar.
          </div>
        ) : (
          <table className="table" style={{ width: "100%", tableLayout: "fixed" }}>
            <thead><tr>
              <th style={{ width: "16%" }}>Fecha</th>
              <th style={{ width: "12%" }}>Monto</th>
              <th style={{ width: "18%" }}>Banco</th>
              <th style={{ width: "20%" }}>Motivo</th>
              <th style={{ width: "16%", wordBreak: "break-word" }}>Cajero</th>
              <th style={{ width: "18%" }}>Acción</th>
            </tr></thead>
            <tbody>
              {items.map((r) => (
                <Fragment key={r.id}>
                  <tr>
                    <td style={{ fontSize: 11 }}>{(r.fecha || "").slice(0, 16).replace("T", " ")}</td>
                    <td style={{ fontWeight: 600, color: "var(--color-danger)" }}>${(r.monto || 0).toFixed(2)}</td>
                    <td style={{ fontSize: 12 }}>{r.banco_nombre || "-"}</td>
                    <td style={{ fontSize: 12 }}>{r.motivo || "-"}</td>
                    <td style={{ fontSize: 12, wordBreak: "break-word" }}>{r.usuario}</td>
                    <td>
                      <button className="btn btn-primary" style={{ fontSize: 11, padding: "3px 10px" }}
                        onClick={() => { setConfirmandoId(r.id); setRef(r.referencia || ""); setImg(null); }}>
                        Confirmar
                      </button>
                    </td>
                  </tr>
                  {confirmandoId === r.id && (
                    <tr>
                      <td colSpan={6} style={{ padding: "10px 12px", background: "rgba(245,158,11,0.06)" }}>
                        <div style={{ display: "flex", gap: 8, alignItems: "flex-end", flexWrap: "wrap" }}>
                          <div style={{ flex: 1, minWidth: 180 }}>
                            <label className="text-secondary" style={{ fontSize: 11, display: "block", marginBottom: 2 }}>
                              N° de comprobante / referencia bancaria *
                            </label>
                            <input className="input" value={ref} onChange={(e) => setRef(e.target.value)}
                              placeholder="Ej: dep-0012345" autoFocus />
                          </div>
                          <div>
                            <label className="text-secondary" style={{ fontSize: 11, display: "block", marginBottom: 2 }}>
                              Comprobante (opcional)
                            </label>
                            <input type="file" accept="image/*" onChange={onImg} style={{ fontSize: 11 }} />
                          </div>
                          {img && <img src={img} alt="comprobante" style={{ height: 40, borderRadius: 4 }} />}
                          <button className="btn btn-success" style={{ fontSize: 12 }} disabled={guardando} onClick={confirmar}>
                            {guardando ? "..." : "✓ Marcar depositado"}
                          </button>
                          <button className="btn btn-outline" style={{ fontSize: 12 }}
                            onClick={() => { setConfirmandoId(null); setRef(""); setImg(null); }}>
                            Cancelar
                          </button>
                        </div>
                      </td>
                    </tr>
                  )}
                </Fragment>
              ))}
            </tbody>
          </table>
        )}
      </div>
    </div>
  );
}
