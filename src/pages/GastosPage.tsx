import { useState, useEffect } from "react";
import { crearGasto, listarGastosDia, eliminarGasto, exportarGastosCsv } from "../services/api";
import { save } from "@tauri-apps/plugin-dialog";
import { useToast } from "../components/Toast";
import Modal from "../components/Modal";
import type { Gasto } from "../types";

const CATEGORIAS_GASTO = [
  "Compra mercaderia",
  "Servicios basicos",
  "Alquiler",
  "Transporte",
  "Sueldos",
  "Otro",
];

export default function GastosPage() {
  const { toastExito, toastError } = useToast();
  const [gastos, setGastos] = useState<Gasto[]>([]);
  const [fecha, setFecha] = useState(() => {
    // Usar fecha LOCAL (Ecuador UTC-5), no UTC
    const now = new Date();
    const y = now.getFullYear();
    const m = String(now.getMonth() + 1).padStart(2, "0");
    const d = String(now.getDate()).padStart(2, "0");
    return `${y}-${m}-${d}`;
  });
  const [mostrarForm, setMostrarForm] = useState(false);
  const [confirmarEliminar, setConfirmarEliminar] = useState<number | null>(null);

  // Form state
  const [descripcion, setDescripcion] = useState("");
  const [monto, setMonto] = useState("");
  const [categoria, setCategoria] = useState("Otro");
  const [observacion, setObservacion] = useState("");

  const cargar = async () => {
    try {
      const data = await listarGastosDia(fecha);
      setGastos(data);
    } catch (err) {
      toastError("Error cargando gastos: " + err);
    }
  };

  useEffect(() => { cargar(); }, [fecha]);

  const totalDia = gastos.reduce((sum, g) => sum + g.monto, 0);

  const handleExportarCSV = async () => {
    try {
      const destino = await save({
        defaultPath: `gastos-${fecha}.csv`,
        filters: [{ name: "CSV", extensions: ["csv"] }],
      });
      if (!destino) return;
      const msg = await exportarGastosCsv(fecha, fecha, destino);
      toastExito(msg);
    } catch (err) {
      toastError("Error al exportar: " + err);
    }
  };

  const handleCrear = async () => {
    if (!descripcion.trim() || !monto) {
      toastError("Descripcion y monto son requeridos");
      return;
    }

    try {
      await crearGasto({
        descripcion: descripcion.trim(),
        monto: parseFloat(monto),
        categoria,
        observacion: observacion.trim() || undefined,
      });
      toastExito("Gasto registrado");
      setDescripcion("");
      setMonto("");
      setCategoria("Otro");
      setObservacion("");
      setMostrarForm(false);
      cargar();
    } catch (err) {
      toastError("Error: " + err);
    }
  };

  const handleEliminar = async () => {
    if (confirmarEliminar === null) return;
    try {
      await eliminarGasto(confirmarEliminar);
      toastExito("Gasto eliminado");
      setConfirmarEliminar(null);
      cargar();
    } catch (err) {
      toastError("Error: " + err);
    }
  };

  return (
    <>
      <div className="page-header">
        <h2>Gastos</h2>
        <div className="flex gap-2 items-center">
          <input
            type="date"
            className="input"
            value={fecha}
            onChange={(e) => setFecha(e.target.value)}
            style={{ width: 160 }}
          />
          <button className="btn btn-outline" style={{ fontSize: 11, padding: "4px 10px" }}
            onClick={handleExportarCSV}>
            CSV
          </button>
          <button className="btn btn-primary" onClick={() => setMostrarForm(!mostrarForm)}>
            + Nuevo Gasto
          </button>
        </div>
      </div>
      <div className="page-body">
        {/* Resumen */}
        <div className="card mb-4" style={{ maxWidth: 250 }}>
          <div className="card-body" style={{ textAlign: "center" }}>
            <span className="text-secondary" style={{ fontSize: 12 }}>Total gastos del dia</span>
            <div className="text-xl font-bold" style={{ color: "var(--color-danger)" }}>
              ${totalDia.toFixed(2)}
            </div>
          </div>
        </div>

        {/* Formulario inline */}
        {mostrarForm && (
          <div className="card mb-4">
            <div className="card-header">Registrar Gasto</div>
            <div className="card-body">
              <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: 16 }}>
                <div>
                  <label className="text-secondary" style={{ fontSize: 12 }}>Descripcion *</label>
                  <input
                    className="input"
                    placeholder="Ej: Compra de arroz"
                    value={descripcion}
                    onChange={(e) => setDescripcion(e.target.value)}
                    autoFocus
                  />
                </div>
                <div>
                  <label className="text-secondary" style={{ fontSize: 12 }}>Monto *</label>
                  <input
                    className="input"
                    type="number"
                    step="0.01"
                    min="0.01"
                    placeholder="0.00"
                    value={monto}
                    onChange={(e) => setMonto(e.target.value)}
                    onKeyDown={(e) => { if (e.key === "Enter") handleCrear(); }}
                  />
                </div>
                <div>
                  <label className="text-secondary" style={{ fontSize: 12 }}>Categoria</label>
                  <select
                    className="input"
                    value={categoria}
                    onChange={(e) => setCategoria(e.target.value)}
                  >
                    {CATEGORIAS_GASTO.map((c) => (
                      <option key={c} value={c}>{c}</option>
                    ))}
                  </select>
                </div>
                <div>
                  <label className="text-secondary" style={{ fontSize: 12 }}>Observacion</label>
                  <input
                    className="input"
                    placeholder="Opcional"
                    value={observacion}
                    onChange={(e) => setObservacion(e.target.value)}
                  />
                </div>
              </div>
              <div className="flex gap-2 mt-4" style={{ justifyContent: "flex-end" }}>
                <button className="btn btn-outline" onClick={() => setMostrarForm(false)}>
                  Cancelar
                </button>
                <button className="btn btn-primary" onClick={handleCrear}>
                  Registrar Gasto
                </button>
              </div>
            </div>
          </div>
        )}

        {/* Tabla de gastos */}
        <div className="card">
          <table className="table">
            <thead>
              <tr>
                <th>Hora</th>
                <th>Descripcion</th>
                <th>Categoria</th>
                <th>Observacion</th>
                <th className="text-right">Monto</th>
                <th style={{ width: 60 }}></th>
              </tr>
            </thead>
            <tbody>
              {gastos.length === 0 ? (
                <tr>
                  <td colSpan={6} className="text-center text-secondary" style={{ padding: 40 }}>
                    No hay gastos registrados para esta fecha
                  </td>
                </tr>
              ) : (
                gastos.map((g) => (
                  <tr key={g.id}>
                    <td className="text-secondary" style={{ fontSize: 12 }}>
                      {g.fecha ? new Date(g.fecha).toLocaleTimeString("es-EC", { hour: "2-digit", minute: "2-digit" }) : "-"}
                    </td>
                    <td><strong>{g.descripcion}</strong></td>
                    <td className="text-secondary">{g.categoria ?? "-"}</td>
                    <td className="text-secondary" style={{ fontSize: 12 }}>{g.observacion ?? "-"}</td>
                    <td className="text-right font-bold" style={{ color: "var(--color-danger)" }}>
                      ${g.monto.toFixed(2)}
                    </td>
                    <td>
                      <button
                        className="btn btn-danger"
                        style={{ padding: "2px 8px", fontSize: 11 }}
                        onClick={() => setConfirmarEliminar(g.id!)}
                      >
                        x
                      </button>
                    </td>
                  </tr>
                ))
              )}
            </tbody>
          </table>
        </div>
      </div>

      <Modal
        abierto={confirmarEliminar !== null}
        titulo="Eliminar Gasto"
        mensaje="¿Está seguro que desea eliminar este gasto?"
        tipo="peligro"
        textoConfirmar="Sí, eliminar"
        onConfirmar={handleEliminar}
        onCancelar={() => setConfirmarEliminar(null)}
      />
    </>
  );
}
