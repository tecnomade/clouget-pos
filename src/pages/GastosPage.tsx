import { useState, useEffect, useMemo } from "react";
import { crearGasto, listarGastosRango, resumenGastosRango, eliminarGasto, exportarGastosCsv } from "../services/api";
import type { ResumenGastos } from "../services/api";
import { save } from "@tauri-apps/plugin-dialog";
import { useToast } from "../components/Toast";
import Modal from "../components/Modal";
import type { Gasto } from "../types";

// v2.5.54: presets de rango de fechas
type PresetRango = "hoy" | "ayer" | "7d" | "30d" | "mes" | "mes_anterior" | "anio" | "rango";

function calcularRango(preset: PresetRango): { desde: string; hasta: string } {
  const hoy = new Date();
  const fmt = (d: Date) => {
    const y = d.getFullYear();
    const m = String(d.getMonth() + 1).padStart(2, "0");
    const dd = String(d.getDate()).padStart(2, "0");
    return `${y}-${m}-${dd}`;
  };
  const hoyStr = fmt(hoy);
  switch (preset) {
    case "hoy":
      return { desde: hoyStr, hasta: hoyStr };
    case "ayer": {
      const ay = new Date(hoy); ay.setDate(ay.getDate() - 1);
      const a = fmt(ay);
      return { desde: a, hasta: a };
    }
    case "7d": {
      const d = new Date(hoy); d.setDate(d.getDate() - 6);
      return { desde: fmt(d), hasta: hoyStr };
    }
    case "30d": {
      const d = new Date(hoy); d.setDate(d.getDate() - 29);
      return { desde: fmt(d), hasta: hoyStr };
    }
    case "mes": {
      const primer = new Date(hoy.getFullYear(), hoy.getMonth(), 1);
      return { desde: fmt(primer), hasta: hoyStr };
    }
    case "mes_anterior": {
      const primer = new Date(hoy.getFullYear(), hoy.getMonth() - 1, 1);
      const ultimo = new Date(hoy.getFullYear(), hoy.getMonth(), 0);
      return { desde: fmt(primer), hasta: fmt(ultimo) };
    }
    case "anio": {
      const primer = new Date(hoy.getFullYear(), 0, 1);
      return { desde: fmt(primer), hasta: hoyStr };
    }
    default:
      return { desde: hoyStr, hasta: hoyStr };
  }
}

const CATEGORIAS_GASTO_DEFAULT = [
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
  const [resumen, setResumen] = useState<ResumenGastos | null>(null);
  // v2.5.54: rango de fechas con presets
  const [preset, setPreset] = useState<PresetRango>("hoy");
  const [{ desde, hasta }, setRango] = useState(() => calcularRango("hoy"));
  // Filtros adicionales
  const [filtroCategoria, setFiltroCategoria] = useState("");
  const [busqueda, setBusqueda] = useState("");
  const [soloRecurrentes, setSoloRecurrentes] = useState(false);
  const [mostrarFiltrosAvanzados, setMostrarFiltrosAvanzados] = useState(false);

  const [mostrarForm, setMostrarForm] = useState(false);
  const [confirmarEliminar, setConfirmarEliminar] = useState<number | null>(null);

  // Form state
  const [descripcion, setDescripcion] = useState("");
  const [monto, setMonto] = useState("");
  const [categoria, setCategoria] = useState("Otro");
  const [categoriasGasto, setCategoriasGasto] = useState<string[]>(CATEGORIAS_GASTO_DEFAULT);
  const [mostrarNuevaCategoria, setMostrarNuevaCategoria] = useState(false);
  const [nuevaCategoriaGasto, setNuevaCategoriaGasto] = useState("");
  const [observacion, setObservacion] = useState("");
  const [esRecurrente, setEsRecurrente] = useState(false);

  // Cuando cambia el preset (excepto "rango"), recalcular fechas
  useEffect(() => {
    if (preset !== "rango") {
      setRango(calcularRango(preset));
    }
  }, [preset]);

  const cargar = async () => {
    try {
      const [data, res] = await Promise.all([
        listarGastosRango(desde, hasta, {
          categoria: filtroCategoria || undefined,
          solo_recurrentes: soloRecurrentes || undefined,
          busqueda: busqueda.trim() || undefined,
        }),
        resumenGastosRango(desde, hasta),
      ]);
      setGastos(data);
      setResumen(res);
    } catch (err) {
      toastError("Error cargando gastos: " + err);
    }
  };

  useEffect(() => { cargar(); /* eslint-disable-next-line react-hooks/exhaustive-deps */ },
    [desde, hasta, filtroCategoria, soloRecurrentes, busqueda]);

  // Cargar categorías únicas de gastos existentes (para el dropdown del form Y filtro)
  useEffect(() => {
    const catExistentes = gastos.map(g => g.categoria).filter((c): c is string => !!c);
    const todas = [...new Set([...CATEGORIAS_GASTO_DEFAULT, ...catExistentes])];
    setCategoriasGasto(todas);
  }, [gastos]);

  const totalRango = useMemo(() => gastos.reduce((s, g) => s + g.monto, 0), [gastos]);

  // Etiqueta amigable del rango
  const etiquetaRango = useMemo(() => {
    const labels: Record<PresetRango, string> = {
      hoy: "Hoy",
      ayer: "Ayer",
      "7d": "Últimos 7 días",
      "30d": "Últimos 30 días",
      mes: "Este mes",
      mes_anterior: "Mes anterior",
      anio: "Este año",
      rango: `${desde} → ${hasta}`,
    };
    return labels[preset];
  }, [preset, desde, hasta]);

  const handleExportarCSV = async () => {
    try {
      const destino = await save({
        defaultPath: `gastos-${desde}_${hasta}.csv`,
        filters: [{ name: "CSV", extensions: ["csv"] }],
      });
      if (!destino) return;
      const msg = await exportarGastosCsv(desde, hasta, destino);
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
        es_recurrente: esRecurrente,
      });
      toastExito("Gasto registrado");
      setDescripcion("");
      setMonto("");
      setCategoria("Otro");
      setObservacion("");
      setEsRecurrente(false);
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
          <button className="btn btn-outline" style={{ fontSize: 11, padding: "4px 10px" }}
            onClick={handleExportarCSV} title="Exportar el rango actual a CSV">
            ⬇ CSV
          </button>
          <button className="btn btn-primary" onClick={() => setMostrarForm(!mostrarForm)}>
            + Nuevo Gasto
          </button>
        </div>
      </div>
      <div className="page-body">
        {/* v2.5.54: Filtros inteligentes con presets de rango */}
        <div className="card mb-4">
          <div className="card-body" style={{ padding: 12 }}>
            <div style={{ display: "flex", flexWrap: "wrap", gap: 6, alignItems: "center", marginBottom: 8 }}>
              {([
                ["hoy", "Hoy"],
                ["ayer", "Ayer"],
                ["7d", "7 días"],
                ["30d", "30 días"],
                ["mes", "Este mes"],
                ["mes_anterior", "Mes anterior"],
                ["anio", "Este año"],
                ["rango", "📅 Rango"],
              ] as [PresetRango, string][]).map(([p, label]) => (
                <button key={p}
                  className={`btn ${preset === p ? "btn-primary" : "btn-outline"}`}
                  style={{ fontSize: 11, padding: "4px 12px" }}
                  onClick={() => setPreset(p)}>
                  {label}
                </button>
              ))}
              <div style={{ flex: 1 }} />
              <span className="text-secondary" style={{ fontSize: 11 }}>
                Período: <strong>{etiquetaRango}</strong>
              </span>
            </div>

            {preset === "rango" && (
              <div style={{ display: "flex", gap: 8, alignItems: "center", marginBottom: 8 }}>
                <label className="text-secondary" style={{ fontSize: 11 }}>Desde:</label>
                <input type="date" className="input" style={{ width: 160 }}
                  value={desde}
                  onChange={(e) => setRango(r => ({ ...r, desde: e.target.value }))} />
                <label className="text-secondary" style={{ fontSize: 11 }}>Hasta:</label>
                <input type="date" className="input" style={{ width: 160 }}
                  value={hasta}
                  onChange={(e) => setRango(r => ({ ...r, hasta: e.target.value }))} />
              </div>
            )}

            {/* Búsqueda + toggle filtros avanzados */}
            <div style={{ display: "flex", gap: 8, alignItems: "center" }}>
              <input className="input" style={{ flex: 1, fontSize: 12 }}
                placeholder="🔍 Buscar en descripción u observación..."
                value={busqueda}
                onChange={(e) => setBusqueda(e.target.value)} />
              <button className="btn btn-outline" style={{ fontSize: 11, padding: "4px 10px" }}
                onClick={() => setMostrarFiltrosAvanzados(!mostrarFiltrosAvanzados)}>
                {mostrarFiltrosAvanzados ? "▲ Menos filtros" : "▼ Más filtros"}
              </button>
            </div>

            {mostrarFiltrosAvanzados && (
              <div style={{ display: "flex", gap: 12, marginTop: 8, alignItems: "center", flexWrap: "wrap" }}>
                <div style={{ display: "flex", alignItems: "center", gap: 6 }}>
                  <label className="text-secondary" style={{ fontSize: 11 }}>Categoría:</label>
                  <select className="input" style={{ width: 180, fontSize: 12 }}
                    value={filtroCategoria}
                    onChange={(e) => setFiltroCategoria(e.target.value)}>
                    <option value="">Todas</option>
                    {categoriasGasto.map((c) => (
                      <option key={c} value={c}>{c}</option>
                    ))}
                  </select>
                </div>
                <label style={{ display: "flex", alignItems: "center", gap: 6, fontSize: 12, cursor: "pointer" }}>
                  <input type="checkbox" checked={soloRecurrentes}
                    onChange={(e) => setSoloRecurrentes(e.target.checked)} />
                  Solo recurrentes 🔁
                </label>
                {(filtroCategoria || soloRecurrentes || busqueda) && (
                  <button className="btn btn-outline" style={{ fontSize: 11, padding: "4px 10px", color: "var(--color-danger)" }}
                    onClick={() => { setFiltroCategoria(""); setSoloRecurrentes(false); setBusqueda(""); }}>
                    ✕ Limpiar filtros
                  </button>
                )}
              </div>
            )}
          </div>
        </div>

        {/* v2.5.54: KPIs visuales del rango */}
        <div style={{ display: "grid", gridTemplateColumns: "repeat(auto-fit, minmax(180px, 1fr))", gap: 12, marginBottom: 16 }}>
          <div className="card" style={{ borderLeft: "4px solid var(--color-danger)" }}>
            <div className="card-body" style={{ padding: 12 }}>
              <div className="text-secondary" style={{ fontSize: 11 }}>Total {etiquetaRango.toLowerCase()}</div>
              <div className="text-xl font-bold" style={{ color: "var(--color-danger)" }}>
                ${totalRango.toFixed(2)}
              </div>
              <div className="text-secondary" style={{ fontSize: 10, marginTop: 2 }}>
                {gastos.length} gasto{gastos.length === 1 ? "" : "s"}
              </div>
            </div>
          </div>
          {resumen && resumen.count > 0 && (
            <div className="card">
              <div className="card-body" style={{ padding: 12 }}>
                <div className="text-secondary" style={{ fontSize: 11 }}>Promedio por gasto</div>
                <div className="text-xl font-bold">${resumen.promedio.toFixed(2)}</div>
                <div className="text-secondary" style={{ fontSize: 10, marginTop: 2 }}>
                  basado en {resumen.count} registros
                </div>
              </div>
            </div>
          )}
          {resumen && resumen.por_categoria.length > 0 && (
            <div className="card">
              <div className="card-body" style={{ padding: 12 }}>
                <div className="text-secondary" style={{ fontSize: 11 }}>Top categoría</div>
                <div className="font-bold" style={{ fontSize: 14 }}>
                  {resumen.por_categoria[0].categoria}
                </div>
                <div className="text-secondary" style={{ fontSize: 10, marginTop: 2 }}>
                  ${resumen.por_categoria[0].total.toFixed(2)} ({resumen.por_categoria[0].count} gastos)
                </div>
              </div>
            </div>
          )}
          {resumen && resumen.por_dia.length > 1 && (
            <div className="card">
              <div className="card-body" style={{ padding: 12 }}>
                <div className="text-secondary" style={{ fontSize: 11 }}>Promedio diario</div>
                <div className="text-xl font-bold">
                  ${(resumen.total / resumen.por_dia.length).toFixed(2)}
                </div>
                <div className="text-secondary" style={{ fontSize: 10, marginTop: 2 }}>
                  en {resumen.por_dia.length} días con gastos
                </div>
              </div>
            </div>
          )}
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
                  <div style={{ display: "flex", gap: 4 }}>
                    <select
                      className="input"
                      style={{ flex: 1 }}
                      value={categoria}
                      onChange={(e) => setCategoria(e.target.value)}
                    >
                      {categoriasGasto.map((c) => (
                        <option key={c} value={c}>{c}</option>
                      ))}
                    </select>
                    <button className="btn btn-outline" style={{ padding: "4px 10px", fontSize: 14, fontWeight: 700 }}
                      title="Agregar nueva categoría"
                      onClick={() => setMostrarNuevaCategoria(true)}>
                      +
                    </button>
                  </div>
                  {mostrarNuevaCategoria && (
                    <div style={{ display: "flex", gap: 4, marginTop: 4 }}>
                      <input className="input" placeholder="Nueva categoría..." value={nuevaCategoriaGasto}
                        style={{ flex: 1, fontSize: 12 }} autoFocus
                        onChange={(e) => setNuevaCategoriaGasto(e.target.value)}
                        onKeyDown={(e) => {
                          if (e.key === "Enter" && nuevaCategoriaGasto.trim()) {
                            setCategoriasGasto(prev => [...prev.filter(c => c !== "Otro"), nuevaCategoriaGasto.trim(), "Otro"]);
                            setCategoria(nuevaCategoriaGasto.trim());
                            setNuevaCategoriaGasto("");
                            setMostrarNuevaCategoria(false);
                          }
                        }} />
                      <button className="btn btn-primary" style={{ fontSize: 11, padding: "4px 10px" }}
                        onClick={() => {
                          if (nuevaCategoriaGasto.trim()) {
                            setCategoriasGasto(prev => [...prev.filter(c => c !== "Otro"), nuevaCategoriaGasto.trim(), "Otro"]);
                            setCategoria(nuevaCategoriaGasto.trim());
                            setNuevaCategoriaGasto("");
                            setMostrarNuevaCategoria(false);
                          }
                        }}>OK</button>
                      <button className="btn btn-outline" style={{ fontSize: 11, padding: "4px 8px" }}
                        onClick={() => { setMostrarNuevaCategoria(false); setNuevaCategoriaGasto(""); }}>x</button>
                    </div>
                  )}
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
              <label style={{ display: "flex", alignItems: "center", gap: 6, marginTop: 8 }}>
                <input type="checkbox" checked={esRecurrente} onChange={(e) => setEsRecurrente(e.target.checked)} />
                <span style={{ fontSize: 13 }}>Es un gasto recurrente (mensual, trimestral, etc.)</span>
              </label>
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
                <th>Sesión</th>
                <th>Usuario</th>
                <th>Observacion</th>
                <th>Recurrente</th>
                <th className="text-right">Monto</th>
                <th style={{ width: 60 }}></th>
              </tr>
            </thead>
            <tbody>
              {gastos.length === 0 ? (
                <tr>
                  <td colSpan={9} className="text-center text-secondary" style={{ padding: 40 }}>
                    No hay gastos registrados para esta fecha
                  </td>
                </tr>
              ) : (
                gastos.map((g) => {
                  const cajaCerrada = g.caja_estado === "CERRADA";
                  return (
                  <tr key={g.id}>
                    <td className="text-secondary" style={{ fontSize: 12 }}>
                      {g.fecha ? new Date(g.fecha).toLocaleTimeString("es-EC", { hour: "2-digit", minute: "2-digit" }) : "-"}
                    </td>
                    <td><strong>{g.descripcion}</strong></td>
                    <td className="text-secondary">{g.categoria ?? "-"}</td>
                    <td style={{ fontSize: 11 }}>
                      {g.caja_id ? (
                        <span style={{
                          padding: "2px 6px", borderRadius: 3, fontWeight: 600,
                          background: cajaCerrada ? "rgba(148,163,184,0.15)" : "rgba(34,197,94,0.15)",
                          color: cajaCerrada ? "var(--color-text-secondary)" : "var(--color-success)",
                        }} title={cajaCerrada ? "Caja cerrada — gasto bloqueado" : "Caja abierta"}>
                          #{g.caja_id} {cajaCerrada ? "🔒" : "🟢"}
                        </span>
                      ) : (
                        <span className="text-secondary" style={{ fontSize: 11 }}>—</span>
                      )}
                    </td>
                    <td style={{ fontSize: 12 }}>{g.usuario_nombre ?? <span className="text-secondary">—</span>}</td>
                    <td className="text-secondary" style={{ fontSize: 12 }}>{g.observacion ?? "-"}</td>
                    <td>
                      {g.es_recurrente ? (
                        <span style={{
                          fontSize: 10, padding: "2px 6px", borderRadius: 3, fontWeight: 600,
                          background: "rgba(59, 130, 246, 0.15)", color: "var(--color-primary, #3b82f6)",
                        }}>
                          Recurrente
                        </span>
                      ) : (
                        <span className="text-secondary" style={{ fontSize: 11 }}>-</span>
                      )}
                    </td>
                    <td className="text-right font-bold" style={{ color: "var(--color-danger)" }}>
                      ${g.monto.toFixed(2)}
                    </td>
                    <td>
                      <button
                        className="btn btn-danger"
                        style={{ padding: "2px 8px", fontSize: 11, opacity: cajaCerrada ? 0.4 : 1, cursor: cajaCerrada ? "not-allowed" : "pointer" }}
                        disabled={cajaCerrada}
                        title={cajaCerrada ? "No se puede eliminar — pertenece a caja cerrada. Usa + Ingreso a Caja para compensar." : "Eliminar gasto"}
                        onClick={() => !cajaCerrada && setConfirmarEliminar(g.id!)}
                      >
                        x
                      </button>
                    </td>
                  </tr>
                  );
                })
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
