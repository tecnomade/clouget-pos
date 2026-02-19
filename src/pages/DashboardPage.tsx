import { useState, useEffect } from "react";
import { useNavigate } from "react-router-dom";
import {
  resumenDiario, resumenDiarioAyer, resumenFiadosPendientes,
  alertasStockBajo, obtenerCajaAbierta, ventasPorDia,
  productosMasVendidosReporte, ultimasVentasDia, resumenDeudores,
} from "../services/api";
import { useSesion } from "../contexts/SesionContext";
import type { ResumenDiario, AlertaStock, VentaDiaria, ProductoMasVendido, UltimaVenta } from "../services/api";
import type { Caja, ResumenCliente } from "../types";
import { BarChart, Bar, XAxis, YAxis, Tooltip, ResponsiveContainer, CartesianGrid } from "recharts";

function fechaHoy(): string {
  const now = new Date();
  const y = now.getFullYear();
  const m = String(now.getMonth() + 1).padStart(2, "0");
  const d = String(now.getDate()).padStart(2, "0");
  return `${y}-${m}-${d}`;
}

function fechaHace7Dias(): string {
  const d = new Date();
  d.setDate(d.getDate() - 6);
  const y = d.getFullYear();
  const m = String(d.getMonth() + 1).padStart(2, "0");
  const dd = String(d.getDate()).padStart(2, "0");
  return `${y}-${m}-${dd}`;
}

export default function DashboardPage() {
  const { sesion, esAdmin } = useSesion();
  const navigate = useNavigate();
  const [resumen, setResumen] = useState<ResumenDiario | null>(null);
  const [resumenAyer, setResumenAyer] = useState<ResumenDiario | null>(null);
  const [fiadosPendientes, setFiadosPendientes] = useState(0);
  const [alertas, setAlertas] = useState<AlertaStock[]>([]);
  const [cajaAbierta, setCajaAbierta] = useState<Caja | null>(null);
  const [ventasSemana, setVentasSemana] = useState<VentaDiaria[]>([]);
  const [topProductos, setTopProductos] = useState<ProductoMasVendido[]>([]);
  const [ultimasVentas, setUltimasVentas] = useState<UltimaVenta[]>([]);
  const [deudores, setDeudores] = useState<ResumenCliente[]>([]);
  const [cargando, setCargando] = useState(true);

  useEffect(() => {
    const hoy = fechaHoy();
    const hace7 = fechaHace7Dias();
    Promise.all([
      resumenDiario(hoy),
      resumenDiarioAyer().catch(() => null),
      resumenFiadosPendientes(),
      alertasStockBajo(),
      obtenerCajaAbierta(),
      ventasPorDia(hace7, hoy).catch(() => []),
      productosMasVendidosReporte(hoy, hoy, 5).catch(() => []),
      ultimasVentasDia(5).catch(() => []),
      resumenDeudores().catch(() => []),
    ]).then(([r, ra, f, a, c, vs, tp, uv, d]) => {
      setResumen(r);
      setResumenAyer(ra);
      setFiadosPendientes(f);
      setAlertas(a);
      setCajaAbierta(c);
      setVentasSemana(vs);
      setTopProductos(tp);
      setUltimasVentas(uv);
      setDeudores(d);
      setCargando(false);
    }).catch(() => setCargando(false));
  }, []);

  if (cargando) {
    return (
      <>
        <div className="page-header"><h2>Inicio</h2></div>
        <div className="page-body">
          <p className="text-secondary">Cargando...</p>
        </div>
      </>
    );
  }

  // === Vista CAJERO ===
  if (!esAdmin) {
    return (
      <>
        <div className="page-header"><h2>Inicio</h2></div>
        <div className="page-body">
          <div className="card" style={{ maxWidth: 450, margin: "40px auto", textAlign: "center" }}>
            <div className="card-body" style={{ padding: 32 }}>
              <h3 style={{ marginBottom: 8 }}>Bienvenido, {sesion?.nombre}</h3>
              <p className="text-secondary" style={{ marginBottom: 24 }}>
                {cajaAbierta
                  ? "Tu caja esta abierta. Puedes comenzar a vender."
                  : "Abre la caja para iniciar tu turno."
                }
              </p>
              {cajaAbierta ? (
                <button className="btn btn-primary btn-lg" onClick={() => navigate("/pos")}>
                  Ir a Vender (F1)
                </button>
              ) : (
                <button className="btn btn-primary btn-lg" onClick={() => navigate("/caja")}>
                  Abrir Caja (F5)
                </button>
              )}
            </div>
          </div>
        </div>
      </>
    );
  }

  // === Vista ADMIN ===
  return (
    <>
      <div className="page-header">
        <h2>Inicio</h2>
        <span className="text-secondary" style={{ fontSize: 13 }}>{fechaHoy()}</span>
      </div>
      <div className="page-body">
        {/* KPI Cards con comparativo */}
        <div style={{ display: "grid", gridTemplateColumns: "repeat(auto-fit, minmax(160px, 1fr))", gap: 12, marginBottom: 20 }}>
          <KpiCard label="Ventas Hoy" valor={resumen?.total_ventas ?? 0} ayer={resumenAyer?.total_ventas} prefix="$" color="var(--color-success)" />
          <KpiCard label="Transacciones" valor={resumen?.num_ventas ?? 0} ayer={resumenAyer?.num_ventas} />
          <KpiCard label="Utilidad Bruta" valor={resumen?.utilidad_bruta ?? 0} ayer={resumenAyer?.utilidad_bruta} prefix="$" color="var(--color-success)" />
          <KpiCard label="Efectivo" valor={resumen?.total_efectivo ?? 0} ayer={resumenAyer?.total_efectivo} prefix="$" />
          <KpiCard label="Transferencia" valor={resumen?.total_transferencia ?? 0} ayer={resumenAyer?.total_transferencia} prefix="$" />
          <KpiCard label="Fiados Pendientes" valor={fiadosPendientes} prefix="$" color={fiadosPendientes > 0 ? "var(--color-warning)" : undefined} />
        </div>

        {/* Fila 1: Gráfica + Caja/Acciones */}
        <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: 16, marginBottom: 16 }}>
          {/* Gráfica de ventas 7 días */}
          <div className="card">
            <div className="card-header">Ventas - Ultimos 7 dias</div>
            <div className="card-body" style={{ padding: "8px 8px 0 0" }}>
              {ventasSemana.length === 0 ? (
                <div className="text-center text-secondary" style={{ padding: 32, fontSize: 13 }}>Sin datos de ventas</div>
              ) : (
                <ResponsiveContainer width="100%" height={200}>
                  <BarChart data={ventasSemana.map(v => ({
                    dia: v.fecha.slice(5), // MM-DD
                    total: Number(v.total.toFixed(2)),
                    ventas: v.num_ventas,
                  }))}>
                    <CartesianGrid strokeDasharray="3 3" stroke="#e5e7eb" />
                    <XAxis dataKey="dia" tick={{ fontSize: 11 }} />
                    <YAxis tick={{ fontSize: 11 }} />
                    <Tooltip
                      formatter={(value) => [`$${Number(value).toFixed(2)}`, "Total"]}
                      labelFormatter={(label) => `Fecha: ${label}`}
                    />
                    <Bar dataKey="total" fill="#2563eb" radius={[4, 4, 0, 0]} />
                  </BarChart>
                </ResponsiveContainer>
              )}
            </div>
          </div>

          {/* Indicador de caja + Acciones rápidas */}
          <div className="card">
            <div className="card-header">
              <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", width: "100%" }}>
                <span>Acciones Rapidas</span>
                <span style={{
                  fontSize: 11,
                  fontWeight: 600,
                  padding: "2px 10px",
                  borderRadius: 12,
                  background: cajaAbierta ? "#dcfce7" : "#fee2e2",
                  color: cajaAbierta ? "#166534" : "#991b1b",
                }}>
                  {cajaAbierta ? `CAJA ABIERTA — ${cajaAbierta.usuario ?? ""}` : "CAJA CERRADA"}
                </span>
              </div>
            </div>
            <div className="card-body" style={{ display: "flex", flexDirection: "column", gap: 8 }}>
              {cajaAbierta && (
                <div style={{
                  display: "flex", justifyContent: "space-between", padding: "6px 12px",
                  background: "#f0fdf4", borderRadius: 6, fontSize: 12, marginBottom: 4,
                }}>
                  <span>Monto inicial: <b>${cajaAbierta.monto_inicial.toFixed(2)}</b></span>
                  <span>Ventas: <b>${cajaAbierta.monto_ventas.toFixed(2)}</b></span>
                </div>
              )}
              <button className="btn btn-primary" onClick={() => navigate("/pos")} style={{ justifyContent: "center" }}>
                Punto de Venta (F1)
              </button>
              <button className="btn btn-outline" onClick={() => navigate("/ventas")} style={{ justifyContent: "center" }}>
                Ventas del Dia (F4)
              </button>
              <button className="btn btn-outline" onClick={() => navigate("/caja")} style={{ justifyContent: "center" }}>
                {cajaAbierta ? "Ver Caja (F5)" : "Abrir Caja (F5)"}
              </button>
              <button className="btn btn-outline" onClick={() => navigate("/productos")} style={{ justifyContent: "center" }}>
                Productos (F2)
              </button>
            </div>
          </div>
        </div>

        {/* Fila 2: Top productos + Últimas ventas */}
        <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: 16, marginBottom: 16 }}>
          {/* Top 5 productos más vendidos */}
          <div className="card">
            <div className="card-header">Top 5 Productos del Dia</div>
            <div className="card-body" style={{ padding: 0 }}>
              {topProductos.length === 0 ? (
                <div className="text-center text-secondary" style={{ padding: 24, fontSize: 13 }}>Sin ventas hoy</div>
              ) : (
                topProductos.map((p, i) => {
                  const maxVendido = topProductos[0]?.total_vendido || 1;
                  const pct = (p.total_vendido / maxVendido) * 100;
                  return (
                    <div key={i} style={{ padding: "8px 12px", borderBottom: "1px solid var(--color-border)" }}>
                      <div style={{ display: "flex", justifyContent: "space-between", fontSize: 12, marginBottom: 4 }}>
                        <span style={{ fontWeight: 500 }}>{p.nombre}</span>
                        <span style={{ color: "var(--color-success)", fontWeight: 600 }}>${p.total_vendido.toFixed(2)}</span>
                      </div>
                      <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
                        <div style={{ flex: 1, height: 6, background: "#e5e7eb", borderRadius: 3 }}>
                          <div style={{ width: `${pct}%`, height: "100%", background: "#2563eb", borderRadius: 3 }} />
                        </div>
                        <span style={{ fontSize: 10, color: "#6b7280", minWidth: 40, textAlign: "right" }}>
                          {p.cantidad_total} uds
                        </span>
                      </div>
                    </div>
                  );
                })
              )}
            </div>
          </div>

          {/* Últimas 5 ventas */}
          <div className="card">
            <div className="card-header">Ultimas Ventas del Dia</div>
            <div className="card-body" style={{ padding: 0 }}>
              {ultimasVentas.length === 0 ? (
                <div className="text-center text-secondary" style={{ padding: 24, fontSize: 13 }}>Sin ventas hoy</div>
              ) : (
                <table style={{ width: "100%", fontSize: 12, borderCollapse: "collapse" }}>
                  <thead>
                    <tr style={{ borderBottom: "2px solid var(--color-border)" }}>
                      <th style={{ padding: "6px 12px", textAlign: "left" }}>Hora</th>
                      <th style={{ padding: "6px 12px", textAlign: "left" }}>Cliente</th>
                      <th style={{ padding: "6px 12px", textAlign: "right" }}>Total</th>
                      <th style={{ padding: "6px 12px", textAlign: "center" }}>Pago</th>
                    </tr>
                  </thead>
                  <tbody>
                    {ultimasVentas.map((v) => (
                      <tr key={v.id} style={{ borderBottom: "1px solid var(--color-border)" }}>
                        <td style={{ padding: "6px 12px", fontWeight: 500 }}>{v.hora}</td>
                        <td style={{ padding: "6px 12px", maxWidth: 140, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>
                          {v.cliente_nombre}
                        </td>
                        <td style={{ padding: "6px 12px", textAlign: "right", fontWeight: 600 }}>
                          ${v.total.toFixed(2)}
                        </td>
                        <td style={{ padding: "6px 12px", textAlign: "center" }}>
                          <span style={{
                            fontSize: 10, padding: "1px 6px", borderRadius: 4,
                            background: v.forma_pago === "EFECTIVO" ? "#dcfce7" : v.forma_pago === "TRANSFER" ? "#dbeafe" : "#fef3c7",
                            color: v.forma_pago === "EFECTIVO" ? "#166534" : v.forma_pago === "TRANSFER" ? "#1e40af" : "#92400e",
                          }}>
                            {v.forma_pago === "EFECTIVO" ? "Efectivo" : v.forma_pago === "TRANSFER" ? "Transfer" : "Fiado"}
                          </span>
                        </td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              )}
            </div>
          </div>
        </div>

        {/* Fila 3: Stock Bajo + Fiados */}
        <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: 16 }}>
          {/* Stock Bajo con colores */}
          <div className="card" style={{ borderColor: alertas.length > 0 ? "#fbbf24" : undefined }}>
            <div className="card-header" style={alertas.length > 0 ? { background: "#fffbeb", color: "#92400e" } : {}}>
              Stock Bajo ({alertas.length})
            </div>
            <div className="card-body" style={{ padding: 0 }}>
              {alertas.length === 0 ? (
                <div className="text-center text-secondary" style={{ padding: 24, fontSize: 13 }}>
                  Todos los productos tienen stock suficiente
                </div>
              ) : (
                alertas.slice(0, 8).map((a) => {
                  const pct = a.stock_minimo > 0 ? Math.min((a.stock_actual / a.stock_minimo) * 100, 100) : 0;
                  const esAgotado = a.stock_actual <= 0;
                  return (
                    <div key={a.id} style={{
                      padding: "6px 12px", borderBottom: "1px solid var(--color-border)",
                      background: esAgotado ? "#fef2f2" : undefined,
                    }}>
                      <div style={{ display: "flex", justifyContent: "space-between", fontSize: 12, marginBottom: 3 }}>
                        <span>{a.nombre}</span>
                        <span style={{ fontWeight: 600, color: esAgotado ? "#dc2626" : "#d97706" }}>
                          {a.stock_actual} / {a.stock_minimo}
                        </span>
                      </div>
                      <div style={{ height: 4, background: "#e5e7eb", borderRadius: 2 }}>
                        <div style={{
                          width: `${pct}%`, height: "100%", borderRadius: 2,
                          background: esAgotado ? "#dc2626" : pct < 50 ? "#f59e0b" : "#22c55e",
                        }} />
                      </div>
                    </div>
                  );
                })
              )}
            </div>
          </div>

          {/* Resumen de fiados / deudores */}
          <div className="card" style={{ borderColor: deudores.length > 0 ? "#f87171" : undefined }}>
            <div className="card-header" style={deudores.length > 0 ? { background: "#fef2f2", color: "#991b1b" } : {}}>
              Cuentas por Cobrar ({deudores.length})
            </div>
            <div className="card-body" style={{ padding: 0 }}>
              {deudores.length === 0 ? (
                <div className="text-center text-secondary" style={{ padding: 24, fontSize: 13 }}>
                  No hay cuentas pendientes
                </div>
              ) : (
                deudores.slice(0, 7).map((d) => (
                  <div key={d.cliente_id} style={{
                    padding: "8px 12px", borderBottom: "1px solid var(--color-border)",
                    display: "flex", justifyContent: "space-between", alignItems: "center", fontSize: 12,
                  }}>
                    <div>
                      <div style={{ fontWeight: 500 }}>{d.cliente_nombre}</div>
                      <div style={{ fontSize: 10, color: "#6b7280" }}>{d.num_cuentas} cuenta{d.num_cuentas !== 1 ? "s" : ""}</div>
                    </div>
                    <span style={{ fontWeight: 700, color: "#dc2626", fontSize: 13 }}>
                      ${d.total_deuda.toFixed(2)}
                    </span>
                  </div>
                ))
              )}
            </div>
          </div>
        </div>
      </div>
    </>
  );
}

/* --- KPI Card con comparativo vs ayer --- */
function KpiCard({ label, valor, ayer, prefix, color }: {
  label: string; valor: number; ayer?: number | null; prefix?: string; color?: string;
}) {
  const formatted = prefix
    ? `${prefix}${valor.toFixed(2)}`
    : String(Math.round(valor));

  let diff: number | null = null;
  let diffPct: number | null = null;
  if (ayer != null && ayer !== undefined) {
    diff = valor - ayer;
    diffPct = ayer > 0 ? ((diff / ayer) * 100) : (valor > 0 ? 100 : 0);
  }

  return (
    <div className="card" style={{ padding: 14 }}>
      <div className="text-secondary" style={{ fontSize: 11 }}>{label}</div>
      <div className="text-xl font-bold" style={{ color }}>{formatted}</div>
      {diff !== null && diffPct !== null && (
        <div style={{
          fontSize: 10, marginTop: 2,
          color: diff > 0 ? "#16a34a" : diff < 0 ? "#dc2626" : "#6b7280",
        }}>
          {diff > 0 ? "▲" : diff < 0 ? "▼" : "="}{" "}
          {Math.abs(diffPct).toFixed(0)}% vs ayer
        </div>
      )}
    </div>
  );
}
