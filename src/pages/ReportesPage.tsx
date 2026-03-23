import { useState, useEffect } from "react";
import { reporteUtilidad, reporteBalance, reporteProductosRentabilidad, exportarVentasCsv } from "../services/api";
import { save } from "@tauri-apps/plugin-dialog";
import { useToast } from "../components/Toast";
import { BarChart, Bar, XAxis, YAxis, CartesianGrid, Tooltip, ResponsiveContainer, PieChart, Pie, Cell } from "recharts";
import type { ReporteUtilidad, ReporteBalance, ProductoRentabilidad } from "../types";

const hoy = () => new Date().toISOString().slice(0, 10);
const hace7 = () => { const d = new Date(); d.setDate(d.getDate() - 7); return d.toISOString().slice(0, 10); };
const inicioMes = () => { const d = new Date(); return `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, "0")}-01`; };
const inicioAnio = () => `${new Date().getFullYear()}-01-01`;

const COLORES_PIE = ["var(--color-primary)", "var(--color-success)", "var(--color-warning)", "#8b5cf6", "var(--color-danger)", "#06b6d4"];

export default function ReportesPage() {
  const { toastExito, toastError } = useToast();
  const [tab, setTab] = useState<"utilidad" | "balance" | "productos">("utilidad");
  const [desde, setDesde] = useState(inicioMes());
  const [hasta, setHasta] = useState(hoy());
  const [utilidad, setUtilidad] = useState<ReporteUtilidad | null>(null);
  const [balance, setBalance] = useState<ReporteBalance | null>(null);
  const [productos, setProductos] = useState<ProductoRentabilidad[]>([]);
  const [cargando, setCargando] = useState(false);

  const cargar = async () => {
    setCargando(true);
    try {
      if (tab === "utilidad") setUtilidad(await reporteUtilidad(desde, hasta));
      else if (tab === "balance") setBalance(await reporteBalance(desde, hasta));
      else setProductos(await reporteProductosRentabilidad(desde, hasta, 50));
    } catch (err) {
      toastError("Error: " + err);
    }
    setCargando(false);
  };

  useEffect(() => { cargar(); }, [tab, desde, hasta]);

  const setPeriodo = (d: string, h: string) => { setDesde(d); setHasta(h); };

  const fmt = (n: number) => `$${n.toFixed(2)}`;
  const fmtPct = (n: number) => `${n.toFixed(1)}%`;

  return (
    <>
      <div className="page-header">
        <h2>Reportes</h2>
        <div className="flex gap-2 items-center">
          <button className="btn btn-outline" style={{ fontSize: 11, padding: "4px 10px" }} onClick={() => setPeriodo(hoy(), hoy())}>Hoy</button>
          <button className="btn btn-outline" style={{ fontSize: 11, padding: "4px 10px" }} onClick={() => setPeriodo(hace7(), hoy())}>7 dias</button>
          <button className="btn btn-outline" style={{ fontSize: 11, padding: "4px 10px" }} onClick={() => setPeriodo(inicioMes(), hoy())}>Este mes</button>
          <button className="btn btn-outline" style={{ fontSize: 11, padding: "4px 10px" }} onClick={() => setPeriodo(inicioAnio(), hoy())}>Este ano</button>
          <span style={{ color: "var(--color-text-secondary)", fontSize: 11 }}>|</span>
          <input type="date" className="input" style={{ fontSize: 12, padding: "4px 8px", width: 130 }} value={desde} onChange={e => setDesde(e.target.value)} />
          <span style={{ fontSize: 11, color: "var(--color-text-secondary)" }}>a</span>
          <input type="date" className="input" style={{ fontSize: 12, padding: "4px 8px", width: 130 }} value={hasta} onChange={e => setHasta(e.target.value)} />
          <button className="btn btn-outline" style={{ fontSize: 11, padding: "4px 10px" }}
            onClick={async () => {
              try {
                const ruta = await save({ defaultPath: `reporte-ventas-${desde}-${hasta}.csv`, filters: [{ name: "CSV", extensions: ["csv"] }] });
                if (ruta) { await exportarVentasCsv(desde, hasta, ruta); toastExito("CSV exportado"); }
              } catch (e) { toastError("Error: " + e); }
            }}>Exportar CSV</button>
        </div>
      </div>

      <div className="page-body" style={{ padding: 16 }}>
        {/* Tabs */}
        <div className="flex gap-2 mb-4">
          {([["utilidad", "Estado de Resultados"], ["balance", "Balance"], ["productos", "Rentabilidad por Producto"]] as const).map(([key, label]) => (
            <button key={key} className={`btn ${tab === key ? "btn-primary" : "btn-outline"}`}
              style={{ fontSize: 13, padding: "6px 16px" }} onClick={() => setTab(key)}>
              {label}
            </button>
          ))}
        </div>

        {cargando && <div style={{ textAlign: "center", padding: 40, color: "var(--color-text-secondary)" }}>Cargando...</div>}

        {/* Estado de Resultados */}
        {!cargando && tab === "utilidad" && utilidad && (
          <div style={{ display: "flex", flexDirection: "column", gap: 16 }}>
            {/* KPIs */}
            <div style={{ display: "grid", gridTemplateColumns: "repeat(auto-fit, minmax(180px, 1fr))", gap: 12 }}>
              <KpiCard label="Ventas Brutas" valor={fmt(utilidad.ventas_brutas)} />
              <KpiCard label="Costo de Ventas" valor={fmt(utilidad.costo_ventas)} color="var(--color-danger)" />
              <KpiCard label="Utilidad Bruta" valor={fmt(utilidad.utilidad_bruta)} sub={`Margen: ${fmtPct(utilidad.margen_bruto)}`} color={utilidad.utilidad_bruta >= 0 ? "var(--color-success)" : "var(--color-danger)"} />
              <KpiCard label="Gastos" valor={fmt(utilidad.total_gastos)} color="var(--color-warning)" />
              <KpiCard label="Devoluciones" valor={fmt(utilidad.total_devoluciones)} color="var(--color-danger)" />
              <KpiCard label="Utilidad Neta" valor={fmt(utilidad.utilidad_neta)} sub={`Margen: ${fmtPct(utilidad.margen_neto)}`} color={utilidad.utilidad_neta >= 0 ? "var(--color-success)" : "var(--color-danger)"} destacado />
              <KpiCard label="Transacciones" valor={String(utilidad.num_ventas)} />
              <KpiCard label="Promedio/Venta" valor={fmt(utilidad.promedio_por_venta)} />
            </div>

            {/* Charts */}
            <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: 16 }}>
              {/* Utilidad por categoría */}
              {utilidad.por_categoria.length > 0 && (
                <div className="card">
                  <div className="card-header">Utilidad por Categoria</div>
                  <div className="card-body" style={{ height: 250 }}>
                    <ResponsiveContainer>
                      <BarChart data={utilidad.por_categoria} layout="vertical" margin={{ left: 80 }}>
                        <CartesianGrid strokeDasharray="3 3" stroke="var(--color-border)" />
                        <XAxis type="number" tickFormatter={v => `$${v}`} style={{ fontSize: 11 }} />
                        <YAxis type="category" dataKey="categoria" style={{ fontSize: 11 }} width={75} />
                        <Tooltip formatter={(v: number) => fmt(v)} />
                        <Bar dataKey="utilidad" fill="var(--color-success)" radius={[0, 4, 4, 0]} />
                      </BarChart>
                    </ResponsiveContainer>
                  </div>
                </div>
              )}

              {/* Gastos por categoría */}
              {utilidad.gastos_por_categoria.length > 0 && (
                <div className="card">
                  <div className="card-header">Gastos por Categoria</div>
                  <div className="card-body" style={{ height: 250 }}>
                    <ResponsiveContainer>
                      <PieChart>
                        <Pie data={utilidad.gastos_por_categoria} dataKey="monto" nameKey="categoria" cx="50%" cy="50%" outerRadius={80} label={({ categoria, percent }) => `${categoria} ${(percent * 100).toFixed(0)}%`} labelLine={{ stroke: "var(--color-text-secondary)" }}>
                          {utilidad.gastos_por_categoria.map((_, i) => <Cell key={i} fill={COLORES_PIE[i % COLORES_PIE.length]} />)}
                        </Pie>
                        <Tooltip formatter={(v: number) => fmt(v)} />
                      </PieChart>
                    </ResponsiveContainer>
                  </div>
                </div>
              )}
            </div>
          </div>
        )}

        {/* Balance */}
        {!cargando && tab === "balance" && balance && (
          <div style={{ display: "flex", flexDirection: "column", gap: 16 }}>
            <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr 1fr", gap: 16 }}>
              {/* Ingresos */}
              <div className="card">
                <div className="card-header" style={{ color: "var(--color-success)", fontWeight: 700 }}>INGRESOS</div>
                <div className="card-body">
                  <BalanceRow label="Efectivo" valor={balance.ingresos_efectivo} />
                  <BalanceRow label="Transferencias" valor={balance.ingresos_transferencia} />
                  <BalanceRow label="Creditos cobrados" valor={balance.ingresos_credito_cobrado} />
                  <div style={{ borderTop: "2px solid var(--color-border)", marginTop: 8, paddingTop: 8 }}>
                    <BalanceRow label="TOTAL INGRESOS" valor={balance.total_ingresos} bold color="var(--color-success)" />
                  </div>
                </div>
              </div>

              {/* Egresos */}
              <div className="card">
                <div className="card-header" style={{ color: "var(--color-danger)", fontWeight: 700 }}>EGRESOS</div>
                <div className="card-body">
                  {balance.gastos_por_categoria.map(g => (
                    <BalanceRow key={g.categoria} label={g.categoria} valor={g.monto} />
                  ))}
                  {balance.total_devoluciones > 0 && <BalanceRow label="Devoluciones" valor={balance.total_devoluciones} />}
                  <div style={{ borderTop: "2px solid var(--color-border)", marginTop: 8, paddingTop: 8 }}>
                    <BalanceRow label="TOTAL EGRESOS" valor={balance.total_egresos} bold color="var(--color-danger)" />
                  </div>
                </div>
              </div>

              {/* Resultado */}
              <div className="card">
                <div className="card-header" style={{ fontWeight: 700 }}>RESULTADO</div>
                <div className="card-body">
                  <div style={{ textAlign: "center", padding: "20px 0" }}>
                    <div style={{ fontSize: 32, fontWeight: 800, color: balance.resultado >= 0 ? "var(--color-success)" : "var(--color-danger)" }}>
                      {fmt(balance.resultado)}
                    </div>
                    <div style={{ fontSize: 13, color: "var(--color-text-secondary)", marginTop: 4 }}>
                      {balance.resultado >= 0 ? "Ganancia del periodo" : "Perdida del periodo"}
                    </div>
                  </div>
                  <div style={{ borderTop: "1px solid var(--color-border)", paddingTop: 12, marginTop: 12 }}>
                    <BalanceRow label="Cuentas por cobrar" valor={balance.cuentas_por_cobrar} color="var(--color-warning)" />
                    <BalanceRow label="Valor inventario" valor={balance.valor_inventario} color="var(--color-primary)" />
                  </div>
                </div>
              </div>
            </div>
          </div>
        )}

        {/* Rentabilidad por Producto */}
        {!cargando && tab === "productos" && (
          <div className="card">
            <div className="card-header">
              Rentabilidad por Producto
              <span style={{ float: "right", fontSize: 12, color: "var(--color-text-secondary)" }}>{productos.length} productos</span>
            </div>
            <table className="table">
              <thead>
                <tr>
                  <th>Producto</th>
                  <th>Categoria</th>
                  <th className="text-right">Cant.</th>
                  <th className="text-right">Ingreso</th>
                  <th className="text-right">Costo</th>
                  <th className="text-right">Utilidad</th>
                  <th style={{ width: 120 }}>Margen</th>
                </tr>
              </thead>
              <tbody>
                {productos.length === 0 ? (
                  <tr><td colSpan={7} className="text-center text-secondary" style={{ padding: 40 }}>Sin datos para este periodo</td></tr>
                ) : productos.map((p, i) => (
                  <tr key={i}>
                    <td><strong>{p.nombre}</strong></td>
                    <td style={{ fontSize: 12, color: "var(--color-text-secondary)" }}>{p.categoria}</td>
                    <td className="text-right">{p.cantidad.toFixed(0)}</td>
                    <td className="text-right">{fmt(p.ingreso)}</td>
                    <td className="text-right">{fmt(p.costo)}</td>
                    <td className="text-right" style={{ fontWeight: 600, color: p.utilidad >= 0 ? "var(--color-success)" : "var(--color-danger)" }}>{fmt(p.utilidad)}</td>
                    <td>
                      <div style={{ display: "flex", alignItems: "center", gap: 6 }}>
                        <div style={{ flex: 1, height: 8, background: "var(--color-border)", borderRadius: 4, overflow: "hidden" }}>
                          <div style={{
                            width: `${Math.min(Math.max(p.margen, 0), 100)}%`, height: "100%", borderRadius: 4,
                            background: p.margen < 10 ? "var(--color-danger)" : p.margen < 25 ? "var(--color-warning)" : "var(--color-success)",
                          }} />
                        </div>
                        <span style={{ fontSize: 11, fontWeight: 600, minWidth: 40, textAlign: "right",
                          color: p.margen < 10 ? "var(--color-danger)" : p.margen < 25 ? "var(--color-warning)" : "var(--color-success)" }}>
                          {fmtPct(p.margen)}
                        </span>
                      </div>
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        )}
      </div>
    </>
  );
}

function KpiCard({ label, valor, sub, color, destacado }: { label: string; valor: string; sub?: string; color?: string; destacado?: boolean }) {
  return (
    <div className="card" style={destacado ? { border: "2px solid var(--color-success)" } : {}}>
      <div className="card-body" style={{ padding: "12px 16px" }}>
        <div style={{ fontSize: 11, color: "var(--color-text-secondary)", marginBottom: 4 }}>{label}</div>
        <div style={{ fontSize: 22, fontWeight: 700, color: color || "var(--color-text)" }}>{valor}</div>
        {sub && <div style={{ fontSize: 11, color: "var(--color-text-secondary)", marginTop: 2 }}>{sub}</div>}
      </div>
    </div>
  );
}

function BalanceRow({ label, valor, bold, color }: { label: string; valor: number; bold?: boolean; color?: string }) {
  return (
    <div className="flex justify-between" style={{ padding: "4px 0", fontWeight: bold ? 700 : 400 }}>
      <span>{label}</span>
      <span style={{ color: color || "var(--color-text)" }}>${valor.toFixed(2)}</span>
    </div>
  );
}
