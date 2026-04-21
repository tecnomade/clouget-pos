import { useState, useEffect, useMemo } from "react";
import { listarMovimientosBancarios, listarCuentasBanco } from "../services/api";
import { useToast } from "../components/Toast";
import type { CuentaBanco } from "../types";

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

interface MovimientoBancario {
  tipo: string;
  referencia: string;
  monto: number;
  fecha: string;
  banco_nombre: string | null;
  detalle: string | null;
}

export default function MovimientosBancariosPage() {
  const { toastError } = useToast();
  const [movimientos, setMovimientos] = useState<MovimientoBancario[]>([]);
  const [cuentasBanco, setCuentasBanco] = useState<CuentaBanco[]>([]);
  const [bancoFiltro, setBancoFiltro] = useState<number | undefined>(undefined);
  const [periodo, setPeriodo] = useState<Periodo>("mes");
  const [fechaDesde, setFechaDesde] = useState(primerDiaMes());
  const [fechaHasta, setFechaHasta] = useState(fechaHoy());
  const [cargando, setCargando] = useState(false);

  useEffect(() => {
    listarCuentasBanco().then(setCuentasBanco).catch(() => {});
  }, []);

  // Update dates when periodo changes
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
      const data = await listarMovimientosBancarios(bancoFiltro, fechaDesde, fechaHasta);
      setMovimientos(data);
    } catch (err) {
      toastError("Error al cargar movimientos: " + err);
    } finally {
      setCargando(false);
    }
  };

  useEffect(() => {
    cargarMovimientos();
  }, [bancoFiltro, fechaDesde, fechaHasta]);

  const { totalIngresos, totalEgresos, saldoNeto } = useMemo(() => {
    let ingresos = 0;
    let egresos = 0;
    for (const m of movimientos) {
      if (m.monto >= 0) ingresos += m.monto;
      else egresos += Math.abs(m.monto);
    }
    return { totalIngresos: ingresos, totalEgresos: egresos, saldoNeto: ingresos - egresos };
  }, [movimientos]);

  const tipoBadge = (tipo: string) => {
    const estilos: Record<string, { bg: string; color: string }> = {
      VENTA: { bg: "rgba(34,197,94,0.15)", color: "var(--color-success)" },
      RETIRO_CAJA: { bg: "rgba(251,146,60,0.15)", color: "#fb923c" },
      PAGO_PROVEEDOR: { bg: "rgba(239,68,68,0.15)", color: "var(--color-danger)" },
      COBRO_CREDITO: { bg: "rgba(59,130,246,0.15)", color: "var(--color-primary)" },
    };
    const s = estilos[tipo] || { bg: "rgba(148,163,184,0.15)", color: "var(--color-text-secondary)" };
    const labels: Record<string, string> = {
      VENTA: "Venta",
      RETIRO_CAJA: "Retiro Caja",
      PAGO_PROVEEDOR: "Pago Proveedor",
      COBRO_CREDITO: "Cobro Credito",
    };
    return (
      <span style={{
        padding: "2px 8px", borderRadius: 4, fontSize: 11, fontWeight: 600,
        background: s.bg, color: s.color,
      }}>
        {labels[tipo] || tipo}
      </span>
    );
  };

  return (
    <>
      <div className="page-header">
        <h2>Movimientos Bancarios</h2>
      </div>
      <div className="page-body">
        {/* Filtros */}
        <div className="flex gap-3 mb-4 items-end" style={{ flexWrap: "wrap" }}>
          <div>
            <label className="text-secondary" style={{ fontSize: 11, display: "block", marginBottom: 4 }}>Periodo</label>
            <select className="input" value={periodo} onChange={(e) => setPeriodo(e.target.value as Periodo)} style={{ minWidth: 140 }}>
              <option value="hoy">Hoy</option>
              <option value="7dias">Ultimos 7 dias</option>
              <option value="mes">Este mes</option>
              <option value="custom">Personalizado</option>
            </select>
          </div>
          {periodo === "custom" && (
            <>
              <div>
                <label className="text-secondary" style={{ fontSize: 11, display: "block", marginBottom: 4 }}>Desde</label>
                <input className="input" type="date" value={fechaDesde} onChange={(e) => setFechaDesde(e.target.value)} />
              </div>
              <div>
                <label className="text-secondary" style={{ fontSize: 11, display: "block", marginBottom: 4 }}>Hasta</label>
                <input className="input" type="date" value={fechaHasta} onChange={(e) => setFechaHasta(e.target.value)} />
              </div>
            </>
          )}
          <div>
            <label className="text-secondary" style={{ fontSize: 11, display: "block", marginBottom: 4 }}>Cuenta</label>
            <select
              className="input"
              value={bancoFiltro ?? ""}
              onChange={(e) => setBancoFiltro(e.target.value ? Number(e.target.value) : undefined)}
              style={{ minWidth: 180 }}
            >
              <option value="">Todas las cuentas</option>
              {cuentasBanco.filter(b => b.activa).map((b) => (
                <option key={b.id} value={b.id}>{b.nombre}</option>
              ))}
            </select>
          </div>
        </div>

        {/* Summary cards */}
        <div className="flex gap-4 mb-4">
          <div className="card" style={{ flex: 1, maxWidth: 220 }}>
            <div className="card-body text-center">
              <span className="text-secondary" style={{ fontSize: 12 }}>Ingresos</span>
              <div className="text-xl font-bold" style={{ color: "var(--color-success)" }}>
                ${totalIngresos.toFixed(2)}
              </div>
            </div>
          </div>
          <div className="card" style={{ flex: 1, maxWidth: 220 }}>
            <div className="card-body text-center">
              <span className="text-secondary" style={{ fontSize: 12 }}>Egresos</span>
              <div className="text-xl font-bold" style={{ color: "var(--color-danger)" }}>
                ${totalEgresos.toFixed(2)}
              </div>
            </div>
          </div>
          <div className="card" style={{ flex: 1, maxWidth: 220 }}>
            <div className="card-body text-center">
              <span className="text-secondary" style={{ fontSize: 12 }}>Saldo neto</span>
              <div className="text-xl font-bold" style={{ color: saldoNeto >= 0 ? "var(--color-success)" : "var(--color-danger)" }}>
                ${saldoNeto.toFixed(2)}
              </div>
            </div>
          </div>
          <div className="card" style={{ flex: 1, maxWidth: 220 }}>
            <div className="card-body text-center">
              <span className="text-secondary" style={{ fontSize: 12 }}>Movimientos</span>
              <div className="text-xl font-bold">{movimientos.length}</div>
            </div>
          </div>
        </div>

        {/* Table */}
        <div className="card">
          <table className="table">
            <thead>
              <tr>
                <th>Fecha</th>
                <th>Tipo</th>
                <th>Referencia</th>
                <th>Detalle</th>
                <th className="text-right">Monto</th>
                <th>Banco</th>
              </tr>
            </thead>
            <tbody>
              {cargando ? (
                <tr>
                  <td colSpan={6} className="text-center text-secondary" style={{ padding: 40 }}>
                    Cargando...
                  </td>
                </tr>
              ) : movimientos.length === 0 ? (
                <tr>
                  <td colSpan={6} className="text-center text-secondary" style={{ padding: 40 }}>
                    No hay movimientos bancarios en este periodo
                  </td>
                </tr>
              ) : (
                movimientos.map((m, i) => (
                  <tr key={i}>
                    <td style={{ whiteSpace: "nowrap" }}>
                      {m.fecha ? new Date(m.fecha).toLocaleDateString("es-EC", {
                        day: "2-digit", month: "2-digit", year: "numeric",
                        hour: "2-digit", minute: "2-digit",
                      }) : "-"}
                    </td>
                    <td>{tipoBadge(m.tipo)}</td>
                    <td className="text-secondary">{m.referencia || "-"}</td>
                    <td>{m.detalle || "-"}</td>
                    <td className="text-right font-bold" style={{
                      color: m.monto >= 0 ? "var(--color-success)" : "var(--color-danger)",
                    }}>
                      {m.monto >= 0 ? "+" : ""}${m.monto.toFixed(2)}
                    </td>
                    <td className="text-secondary">{m.banco_nombre || "-"}</td>
                  </tr>
                ))
              )}
            </tbody>
          </table>
        </div>
      </div>
    </>
  );
}
