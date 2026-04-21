import { useState } from "react";
import { buscarSerie, listarSeriesProducto, devolverSerie } from "../services/api";
import { useToast } from "../components/Toast";

interface SerieResult {
  id: number;
  serial: string;
  estado: string;
  fecha_ingreso: string;
  fecha_venta?: string;
  cliente_nombre?: string;
  observacion?: string;
  producto_nombre: string;
  producto_id: number;
}

export default function SeriesPage() {
  const { toastExito, toastError } = useToast();
  const [busqueda, setBusqueda] = useState("");
  const [resultados, setResultados] = useState<SerieResult[]>([]);
  const [buscando, setBuscando] = useState(false);
  const [filtroProducto, setFiltroProducto] = useState<{ id: number; nombre: string } | null>(null);

  const handleBuscar = async (term?: string) => {
    const termino = term ?? busqueda;
    if (!termino.trim()) return;
    setBuscando(true);
    try {
      const res = await buscarSerie(termino);
      setResultados(res as SerieResult[]);
      setFiltroProducto(null);
    } catch (err) {
      toastError("Error: " + err);
    } finally {
      setBuscando(false);
    }
  };

  const handleFiltrarProducto = async (productoId: number, productoNombre: string) => {
    setBuscando(true);
    try {
      const series = await listarSeriesProducto(productoId);
      setResultados(series.map((s: any) => ({
        ...s,
        producto_nombre: productoNombre,
        producto_id: productoId,
      })));
      setFiltroProducto({ id: productoId, nombre: productoNombre });
    } catch (err) {
      toastError("Error: " + err);
    } finally {
      setBuscando(false);
    }
  };

  const handleDevolver = async (serieId: number) => {
    if (!confirm("Devolver esta serie a estado DISPONIBLE?")) return;
    try {
      await devolverSerie(serieId);
      toastExito("Serie devuelta");
      // Recargar
      if (filtroProducto) {
        handleFiltrarProducto(filtroProducto.id, filtroProducto.nombre);
      } else if (busqueda) {
        handleBuscar();
      }
    } catch (err) {
      toastError("Error: " + err);
    }
  };

  const estadoBadge = (estado: string) => {
    const colors: Record<string, { bg: string; color: string }> = {
      DISPONIBLE: { bg: "rgba(34,197,94,0.15)", color: "var(--color-success)" },
      VENDIDO: { bg: "rgba(59,130,246,0.15)", color: "var(--color-primary)" },
      DEVUELTO: { bg: "rgba(245,158,11,0.15)", color: "var(--color-warning)" },
    };
    const c = colors[estado] || { bg: "rgba(255,255,255,0.1)", color: "inherit" };
    return (
      <span style={{
        padding: "2px 8px", borderRadius: 4, fontSize: 11, fontWeight: 600,
        background: c.bg, color: c.color,
      }}>
        {estado}
      </span>
    );
  };

  return (
    <>
      <div className="page-header">
        <h2>Numeros de Serie</h2>
      </div>
      <div className="page-body">
        <div className="card">
          <div className="card-body">
            {/* Search bar */}
            <div style={{ display: "flex", gap: 8, marginBottom: 16 }}>
              <input
                className="input"
                placeholder="Buscar por numero de serie..."
                value={busqueda}
                onChange={e => setBusqueda(e.target.value)}
                onKeyDown={e => { if (e.key === "Enter") handleBuscar(); }}
                style={{ flex: 1 }}
              />
              <button className="btn btn-primary" onClick={() => handleBuscar()} disabled={buscando || !busqueda.trim()}>
                {buscando ? "Buscando..." : "Buscar"}
              </button>
            </div>

            {/* Active filter indicator */}
            {filtroProducto && (
              <div style={{
                display: "flex", alignItems: "center", gap: 8, marginBottom: 12,
                padding: "6px 12px", background: "rgba(59,130,246,0.1)", borderRadius: 6,
                fontSize: 12,
              }}>
                <span>Filtrando por producto: <strong>{filtroProducto.nombre}</strong></span>
                <button className="btn btn-outline" style={{ fontSize: 10, padding: "2px 8px" }}
                  onClick={() => { setFiltroProducto(null); setResultados([]); }}>
                  Quitar filtro
                </button>
              </div>
            )}

            {/* Results table */}
            <table className="table">
              <thead>
                <tr>
                  <th>Serial</th>
                  <th>Producto</th>
                  <th>Estado</th>
                  <th>Cliente</th>
                  <th>F. Ingreso</th>
                  <th>F. Venta</th>
                  <th style={{ width: 80 }}></th>
                </tr>
              </thead>
              <tbody>
                {resultados.map(s => (
                  <tr key={s.id}>
                    <td style={{ fontFamily: "monospace", fontWeight: 600, fontSize: 13 }}>{s.serial}</td>
                    <td>
                      <button
                        style={{
                          background: "none", border: "none", cursor: "pointer",
                          color: "var(--color-primary)", textDecoration: "underline",
                          fontSize: 13, padding: 0,
                        }}
                        onClick={() => handleFiltrarProducto(s.producto_id, s.producto_nombre)}
                      >
                        {s.producto_nombre}
                      </button>
                    </td>
                    <td>{estadoBadge(s.estado)}</td>
                    <td className="text-secondary" style={{ fontSize: 12 }}>{s.cliente_nombre || "-"}</td>
                    <td className="text-secondary" style={{ fontSize: 12 }}>{s.fecha_ingreso?.slice(0, 10) || "-"}</td>
                    <td className="text-secondary" style={{ fontSize: 12 }}>{s.fecha_venta?.slice(0, 10) || "-"}</td>
                    <td>
                      {s.estado === "VENDIDO" && (
                        <button className="btn btn-outline" style={{ fontSize: 10, padding: "2px 8px" }}
                          onClick={() => handleDevolver(s.id)}>
                          Devolver
                        </button>
                      )}
                    </td>
                  </tr>
                ))}
                {resultados.length === 0 && (
                  <tr>
                    <td colSpan={7} className="text-center text-secondary" style={{ padding: 40 }}>
                      {busqueda || filtroProducto ? "No se encontraron resultados" : "Busque un numero de serie para ver resultados"}
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
