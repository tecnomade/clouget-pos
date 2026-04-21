import { useState, useEffect, useMemo } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useToast } from "../components/Toast";

type Periodo = "hoy" | "7dias" | "mes" | "custom";

function fechaHoy(): string {
  return new Date().toISOString().slice(0, 10);
}

function fechaHace7Dias(): string {
  const d = new Date();
  d.setDate(d.getDate() - 7);
  return d.toISOString().slice(0, 10);
}

function primerDiaMes(): string {
  const d = new Date();
  return `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, "0")}-01`;
}

interface Movimiento {
  tipo: string;
  referencia: string;
  descripcion: string;
  ingreso: number;
  egreso: number;
  forma_pago: string;
  fecha: string;
}

type FiltroTipo = "TODOS" | "VENTA" | "COBRO_CREDITO" | "GASTO" | "COMPRA" | "RETIRO" | "PAGO_PROVEEDOR" | "NOTA_CREDITO";
type FiltroFormaPago = "TODOS" | "EFECTIVO" | "TRANSFER" | "TRANSFERENCIA" | "CREDITO" | "DEVOLUCION";

export default function LibroMovimientosPage() {
  const { toastError } = useToast();
  const [movimientos, setMovimientos] = useState<Movimiento[]>([]);
  const [periodo, setPeriodo] = useState<Periodo>("mes");
  const [fechaDesde, setFechaDesde] = useState(primerDiaMes());
  const [fechaHasta, setFechaHasta] = useState(fechaHoy());
  const [cargando, setCargando] = useState(false);
  const [filtroTipo, setFiltroTipo] = useState<FiltroTipo>("TODOS");
  const [filtroFormaPago, setFiltroFormaPago] = useState<FiltroFormaPago>("TODOS");

  useEffect(() => {
    if (periodo === "hoy") {
      setFechaDesde(fechaHoy());
      setFechaHasta(fechaHoy());
    } else if (periodo === "7dias") {
      setFechaDesde(fechaHace7Dias());
      setFechaHasta(fechaHoy());
    } else if (periodo === "mes") {
      setFechaDesde(primerDiaMes());
      setFechaHasta(fechaHoy());
    }
  }, [periodo]);

  const cargarMovimientos = async () => {
    setCargando(true);
    try {
      const data = await invoke<Movimiento[]>("listar_libro_movimientos", {
        fechaDesde,
        fechaHasta,
      });
      setMovimientos(data);
    } catch (err) {
      toastError("Error al cargar movimientos: " + err);
    } finally {
      setCargando(false);
    }
  };

  useEffect(() => {
    cargarMovimientos();
  }, [fechaDesde, fechaHasta]);

  const movimientosFiltrados = useMemo(() => {
    return movimientos.filter((m) => {
      if (filtroTipo !== "TODOS" && m.tipo !== filtroTipo) return false;
      if (filtroFormaPago !== "TODOS") {
        // TRANSFER y TRANSFERENCIA are equivalent
        if (filtroFormaPago === "TRANSFER" || filtroFormaPago === "TRANSFERENCIA") {
          if (m.forma_pago !== "TRANSFER" && m.forma_pago !== "TRANSFERENCIA") return false;
        } else if (m.forma_pago !== filtroFormaPago) {
          return false;
        }
      }
      return true;
    });
  }, [movimientos, filtroTipo, filtroFormaPago]);

  const { totalIngresos, totalEgresos, balance } = useMemo(() => {
    let ingresos = 0;
    let egresos = 0;
    for (const m of movimientosFiltrados) {
      ingresos += m.ingreso;
      egresos += m.egreso;
    }
    return { totalIngresos: ingresos, totalEgresos: egresos, balance: ingresos - egresos };
  }, [movimientosFiltrados]);

  // Running balance (from oldest to newest)
  const movimientosConSaldo = useMemo(() => {
    const reversed = [...movimientosFiltrados].reverse();
    let saldo = 0;
    const result = reversed.map((m) => {
      saldo += m.ingreso - m.egreso;
      return { ...m, saldo_acumulado: saldo };
    });
    return result.reverse();
  }, [movimientosFiltrados]);

  const tipoBadge = (tipo: string) => {
    const estilos: Record<string, { bg: string; color: string }> = {
      VENTA: { bg: "rgba(34,197,94,0.15)", color: "var(--color-success)" },
      COBRO_CREDITO: { bg: "rgba(59,130,246,0.15)", color: "var(--color-primary)" },
      GASTO: { bg: "rgba(239,68,68,0.15)", color: "var(--color-danger)" },
      RETIRO: { bg: "rgba(251,146,60,0.15)", color: "#fb923c" },
      COMPRA: { bg: "rgba(99,102,241,0.15)", color: "#818cf8" },
      PAGO_PROVEEDOR: { bg: "rgba(168,85,247,0.15)", color: "#a855f7" },
      NOTA_CREDITO: { bg: "rgba(239,68,68,0.15)", color: "var(--color-danger)" },
    };
    const s = estilos[tipo] || { bg: "rgba(148,163,184,0.15)", color: "var(--color-text-secondary)" };
    const labels: Record<string, string> = {
      VENTA: "Venta",
      COBRO_CREDITO: "Cobro Credito",
      GASTO: "Gasto",
      RETIRO: "Retiro",
      COMPRA: "Compra",
      PAGO_PROVEEDOR: "Pago Proveedor",
      NOTA_CREDITO: "Nota Credito",
    };
    return (
      <span
        style={{
          padding: "2px 8px",
          borderRadius: 4,
          fontSize: 11,
          fontWeight: 600,
          background: s.bg,
          color: s.color,
        }}
      >
        {labels[tipo] || tipo}
      </span>
    );
  };

  const formatMoney = (n: number) => `$${n.toFixed(2)}`;

  const formatFecha = (fecha: string) => {
    if (!fecha) return "";
    // Show date + time if available
    if (fecha.includes(" ")) {
      const [date, time] = fecha.split(" ");
      return `${date} ${time.slice(0, 5)}`;
    }
    return fecha;
  };

  const formatFormaPago = (fp: string) => {
    const labels: Record<string, string> = {
      EFECTIVO: "Efectivo",
      TRANSFER: "Transferencia",
      TRANSFERENCIA: "Transferencia",
      CREDITO: "Credito",
      DEVOLUCION: "Devolucion",
    };
    return labels[fp] || fp;
  };

  const exportarCSV = () => {
    const headers = ["Fecha", "Tipo", "Referencia", "Descripcion", "Forma Pago", "Ingreso", "Egreso", "Saldo"];
    const rows = movimientosConSaldo.map((m) => [
      m.fecha,
      m.tipo,
      m.referencia,
      m.descripcion,
      m.forma_pago,
      m.ingreso.toFixed(2),
      m.egreso.toFixed(2),
      m.saldo_acumulado.toFixed(2),
    ]);
    const csv = [headers.join(","), ...rows.map((r) => r.map((c) => `"${c}"`).join(","))].join("\n");
    const blob = new Blob(["\uFEFF" + csv], { type: "text/csv;charset=utf-8;" });
    const url = URL.createObjectURL(blob);
    const a = document.createElement("a");
    a.href = url;
    a.download = `libro_movimientos_${fechaDesde}_${fechaHasta}.csv`;
    a.click();
    URL.revokeObjectURL(url);
  };

  return (
    <>
      <div className="page-header">
        <h2>Libro de Movimientos</h2>
        <button className="btn btn-outline" onClick={exportarCSV} disabled={movimientosConSaldo.length === 0}>
          Exportar CSV
        </button>
      </div>
      <div className="page-body">
        {/* Filters */}
        <div className="flex gap-3 mb-4 items-end" style={{ flexWrap: "wrap" }}>
          <div>
            <label className="text-secondary" style={{ fontSize: 11, display: "block", marginBottom: 4 }}>
              Periodo
            </label>
            <select
              className="input"
              value={periodo}
              onChange={(e) => setPeriodo(e.target.value as Periodo)}
              style={{ minWidth: 140 }}
            >
              <option value="hoy">Hoy</option>
              <option value="7dias">Ultimos 7 dias</option>
              <option value="mes">Este mes</option>
              <option value="custom">Personalizado</option>
            </select>
          </div>
          {periodo === "custom" && (
            <>
              <div>
                <label className="text-secondary" style={{ fontSize: 11, display: "block", marginBottom: 4 }}>
                  Desde
                </label>
                <input
                  className="input"
                  type="date"
                  value={fechaDesde}
                  onChange={(e) => setFechaDesde(e.target.value)}
                />
              </div>
              <div>
                <label className="text-secondary" style={{ fontSize: 11, display: "block", marginBottom: 4 }}>
                  Hasta
                </label>
                <input
                  className="input"
                  type="date"
                  value={fechaHasta}
                  onChange={(e) => setFechaHasta(e.target.value)}
                />
              </div>
            </>
          )}
          <div>
            <label className="text-secondary" style={{ fontSize: 11, display: "block", marginBottom: 4 }}>
              Tipo
            </label>
            <select
              className="input"
              value={filtroTipo}
              onChange={(e) => setFiltroTipo(e.target.value as FiltroTipo)}
              style={{ minWidth: 140 }}
            >
              <option value="TODOS">Todos</option>
              <option value="VENTA">Ventas</option>
              <option value="COBRO_CREDITO">Cobros Credito</option>
              <option value="GASTO">Gastos</option>
              <option value="COMPRA">Compras</option>
              <option value="RETIRO">Retiros</option>
              <option value="PAGO_PROVEEDOR">Pagos Proveedor</option>
              <option value="NOTA_CREDITO">Notas Credito</option>
            </select>
          </div>
          <div>
            <label className="text-secondary" style={{ fontSize: 11, display: "block", marginBottom: 4 }}>
              Forma Pago
            </label>
            <select
              className="input"
              value={filtroFormaPago}
              onChange={(e) => setFiltroFormaPago(e.target.value as FiltroFormaPago)}
              style={{ minWidth: 140 }}
            >
              <option value="TODOS">Todos</option>
              <option value="EFECTIVO">Efectivo</option>
              <option value="TRANSFER">Transferencia</option>
              <option value="CREDITO">Credito</option>
              <option value="DEVOLUCION">Devolucion</option>
            </select>
          </div>
        </div>

        {/* Summary Cards */}
        <div className="flex gap-3 mb-4" style={{ flexWrap: "wrap" }}>
          <div
            className="card"
            style={{
              flex: "1 1 180px",
              padding: "16px 20px",
              textAlign: "center",
            }}
          >
            <div className="text-secondary" style={{ fontSize: 12, marginBottom: 4 }}>
              Total Ingresos
            </div>
            <div style={{ fontSize: 22, fontWeight: 700, color: "var(--color-success)" }}>
              {formatMoney(totalIngresos)}
            </div>
          </div>
          <div
            className="card"
            style={{
              flex: "1 1 180px",
              padding: "16px 20px",
              textAlign: "center",
            }}
          >
            <div className="text-secondary" style={{ fontSize: 12, marginBottom: 4 }}>
              Total Egresos
            </div>
            <div style={{ fontSize: 22, fontWeight: 700, color: "var(--color-danger)" }}>
              {formatMoney(totalEgresos)}
            </div>
          </div>
          <div
            className="card"
            style={{
              flex: "1 1 180px",
              padding: "16px 20px",
              textAlign: "center",
            }}
          >
            <div className="text-secondary" style={{ fontSize: 12, marginBottom: 4 }}>
              Balance
            </div>
            <div
              style={{
                fontSize: 22,
                fontWeight: 700,
                color: balance >= 0 ? "var(--color-success)" : "var(--color-danger)",
              }}
            >
              {formatMoney(balance)}
            </div>
          </div>
        </div>

        {/* Table */}
        <div className="card">
          <div style={{ overflowX: "auto" }}>
            <table className="table">
              <thead>
                <tr>
                  <th style={{ minWidth: 130 }}>Fecha</th>
                  <th style={{ minWidth: 110 }}>Tipo</th>
                  <th style={{ minWidth: 100 }}>Referencia</th>
                  <th style={{ minWidth: 160 }}>Descripcion</th>
                  <th style={{ minWidth: 100 }}>Forma Pago</th>
                  <th style={{ minWidth: 90, textAlign: "right" }}>Ingreso</th>
                  <th style={{ minWidth: 90, textAlign: "right" }}>Egreso</th>
                  <th style={{ minWidth: 90, textAlign: "right" }}>Saldo</th>
                </tr>
              </thead>
              <tbody>
                {cargando ? (
                  <tr>
                    <td colSpan={8} style={{ textAlign: "center", padding: 32 }}>
                      Cargando...
                    </td>
                  </tr>
                ) : movimientosConSaldo.length === 0 ? (
                  <tr>
                    <td colSpan={8} style={{ textAlign: "center", padding: 32 }} className="text-secondary">
                      No hay movimientos en este periodo
                    </td>
                  </tr>
                ) : (
                  movimientosConSaldo.map((m, i) => (
                    <tr key={i}>
                      <td style={{ fontSize: 12, whiteSpace: "nowrap" }}>{formatFecha(m.fecha)}</td>
                      <td>{tipoBadge(m.tipo)}</td>
                      <td style={{ fontSize: 12 }}>{m.referencia || "-"}</td>
                      <td
                        style={{
                          fontSize: 12,
                          maxWidth: 220,
                          overflow: "hidden",
                          textOverflow: "ellipsis",
                          whiteSpace: "nowrap",
                        }}
                        title={m.descripcion}
                      >
                        {m.descripcion || "-"}
                      </td>
                      <td style={{ fontSize: 12 }}>{formatFormaPago(m.forma_pago)}</td>
                      <td
                        style={{
                          textAlign: "right",
                          fontWeight: 600,
                          color: m.ingreso > 0 ? "var(--color-success)" : "var(--color-text-secondary)",
                          fontSize: 13,
                        }}
                      >
                        {m.ingreso > 0 ? formatMoney(m.ingreso) : "-"}
                      </td>
                      <td
                        style={{
                          textAlign: "right",
                          fontWeight: 600,
                          color: m.egreso > 0 ? "var(--color-danger)" : "var(--color-text-secondary)",
                          fontSize: 13,
                        }}
                      >
                        {m.egreso > 0 ? formatMoney(m.egreso) : "-"}
                      </td>
                      <td
                        style={{
                          textAlign: "right",
                          fontWeight: 600,
                          fontSize: 13,
                          color:
                            m.saldo_acumulado >= 0 ? "var(--color-success)" : "var(--color-danger)",
                        }}
                      >
                        {formatMoney(m.saldo_acumulado)}
                      </td>
                    </tr>
                  ))
                )}
              </tbody>
              {movimientosConSaldo.length > 0 && (
                <tfoot>
                  <tr style={{ fontWeight: 700, borderTop: "2px solid var(--color-border)" }}>
                    <td colSpan={5} style={{ textAlign: "right", paddingRight: 12 }}>
                      Totales ({movimientosConSaldo.length} movimientos)
                    </td>
                    <td style={{ textAlign: "right", color: "var(--color-success)" }}>
                      {formatMoney(totalIngresos)}
                    </td>
                    <td style={{ textAlign: "right", color: "var(--color-danger)" }}>
                      {formatMoney(totalEgresos)}
                    </td>
                    <td
                      style={{
                        textAlign: "right",
                        color: balance >= 0 ? "var(--color-success)" : "var(--color-danger)",
                      }}
                    >
                      {formatMoney(balance)}
                    </td>
                  </tr>
                </tfoot>
              )}
            </table>
          </div>
        </div>
      </div>
    </>
  );
}
