import { useState } from "react";
import { clientesPorLote, type ClientePorLote } from "../services/api";
import { useToast } from "../components/Toast";

export default function RecallPage() {
  const { toastExito, toastError } = useToast();
  const [busqueda, setBusqueda] = useState("");
  const [resultados, setResultados] = useState<ClientePorLote[]>([]);
  const [buscando, setBuscando] = useState(false);
  const [buscado, setBuscado] = useState(false);

  const handleBuscar = async () => {
    const termino = busqueda.trim();
    if (!termino) return;
    setBuscando(true);
    try {
      const res = await clientesPorLote(termino);
      setResultados(res);
      setBuscado(true);
    } catch (err) {
      toastError("Error: " + err);
    } finally {
      setBuscando(false);
    }
  };

  const fecha = (s?: string | null) => (s ? s.slice(0, 10) : "-");

  const exportarCsv = () => {
    if (resultados.length === 0) return;
    try {
      const headers = ["Cliente", "Telefono", "Email", "Producto", "Cantidad", "Lote", "Caducidad", "Fecha venta", "N venta"];
      const rows = resultados.map(r => [
        r.cliente_nombre,
        r.cliente_telefono || "",
        r.cliente_email || "",
        r.producto_nombre || "",
        r.cantidad,
        r.lote || "",
        fecha(r.caducidad),
        fecha(r.fecha),
        r.venta_numero,
      ]);
      const csv = [headers, ...rows].map(row => row.map(c => {
        const s = String(c);
        return s.includes(",") || s.includes('"') || s.includes("\n") ? `"${s.replace(/"/g, '""')}"` : s;
      }).join(",")).join("\n");
      // BOM para que Excel reconozca UTF-8
      const blob = new Blob(["﻿" + csv], { type: "text/csv;charset=utf-8;" });
      const url = URL.createObjectURL(blob);
      const a = document.createElement("a");
      a.href = url;
      a.download = `recall_lote_${(busqueda.trim() || "lote").replace(/[^\w-]/g, "_")}.csv`;
      document.body.appendChild(a);
      a.click();
      document.body.removeChild(a);
      URL.revokeObjectURL(url);
      toastExito(`Exportados ${resultados.length} registros`);
    } catch (err) {
      toastError("Error: " + err);
    }
  };

  return (
    <>
      <div className="page-header">
        <h2>Trazabilidad de Lotes</h2>
      </div>
      <div className="page-body">
        <div className="card">
          <div className="card-body">
            <p className="text-secondary" style={{ fontSize: 13, marginTop: 0, marginBottom: 16 }}>
              Busca un número de lote para ver a qué clientes se les vendió (para reclamos o
              retiros de producto caducado).
            </p>

            {/* Search bar */}
            <div style={{ display: "flex", gap: 8, marginBottom: 16 }}>
              <input
                className="input"
                placeholder="Número de lote..."
                value={busqueda}
                onChange={e => setBusqueda(e.target.value)}
                onKeyDown={e => { if (e.key === "Enter") handleBuscar(); }}
                style={{ flex: 1 }}
              />
              <button className="btn btn-primary" onClick={handleBuscar} disabled={buscando || !busqueda.trim()}>
                {buscando ? "Buscando..." : "Buscar"}
              </button>
              {resultados.length > 0 && (
                <button className="btn btn-outline" onClick={exportarCsv} title="Exportar a CSV">
                  📥 Exportar CSV
                </button>
              )}
            </div>

            <div style={{
              display: "flex", alignItems: "center", gap: 8, marginBottom: 12,
              padding: "6px 12px", background: "rgba(245,158,11,0.1)", borderRadius: 6,
              fontSize: 12, color: "var(--color-text-secondary)",
            }}>
              <span>
                Las ventas a "Consumidor Final" no tienen datos de contacto del cliente.
              </span>
            </div>

            {/* Results table */}
            <table className="table">
              <thead>
                <tr>
                  <th>Cliente</th>
                  <th>Teléfono</th>
                  <th>Email</th>
                  <th>Producto</th>
                  <th className="text-right">Cantidad</th>
                  <th>Lote</th>
                  <th>Caducidad</th>
                  <th>Fecha venta</th>
                  <th>N° venta</th>
                </tr>
              </thead>
              <tbody>
                {resultados.map((r, i) => (
                  <tr key={i}>
                    <td>{r.cliente_nombre || "-"}</td>
                    <td className="text-secondary" style={{ fontSize: 12 }}>{r.cliente_telefono || "-"}</td>
                    <td className="text-secondary" style={{ fontSize: 12 }}>{r.cliente_email || "-"}</td>
                    <td>{r.producto_nombre || "-"}</td>
                    <td className="text-right">{r.cantidad}</td>
                    <td style={{ fontFamily: "monospace", fontWeight: 600, fontSize: 13 }}>{r.lote || "-"}</td>
                    <td className="text-secondary" style={{ fontSize: 12 }}>{fecha(r.caducidad)}</td>
                    <td className="text-secondary" style={{ fontSize: 12 }}>{fecha(r.fecha)}</td>
                    <td className="text-secondary" style={{ fontSize: 12 }}>{r.venta_numero}</td>
                  </tr>
                ))}
                {resultados.length === 0 && (
                  <tr>
                    <td colSpan={9} className="text-center text-secondary" style={{ padding: 40 }}>
                      {buscado
                        ? "Sin resultados para este lote"
                        : "Ingrese un número de lote para buscar a qué clientes se les vendió"}
                    </td>
                  </tr>
                )}
              </tbody>
            </table>
          </div>
        </div>
      </div>
    </>
  );
}
