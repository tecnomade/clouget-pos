import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { reporteUtilidad, reporteBalance, reporteProductosRentabilidad, exportarVentasCsv, reporteIvaMensual } from "../services/api";
import { save } from "@tauri-apps/plugin-dialog";
import { useToast } from "../components/Toast";
import { BarChart, Bar, XAxis, YAxis, CartesianGrid, Tooltip, ResponsiveContainer, PieChart, Pie, Cell } from "recharts";
import type { ReporteUtilidad, ReporteBalance, ProductoRentabilidad } from "../types";

type ReporteIva = {
  anio: number; mes: number; fecha_desde: string; fecha_hasta: string;
  ventas_0: number; ventas_15_base: number; iva_ventas: number;
  nc_base: number; nc_iva: number; iva_ventas_neto: number;
  compras_0: number; compras_15_base: number; iva_compras: number;
  iva_a_pagar: number; total_ventas: number; total_compras: number;
};

const MESES = [
  "Enero", "Febrero", "Marzo", "Abril", "Mayo", "Junio",
  "Julio", "Agosto", "Septiembre", "Octubre", "Noviembre", "Diciembre",
];

const hoy = () => new Date().toISOString().slice(0, 10);
const hace7 = () => { const d = new Date(); d.setDate(d.getDate() - 7); return d.toISOString().slice(0, 10); };
const inicioMes = () => { const d = new Date(); return `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, "0")}-01`; };
const inicioAnio = () => `${new Date().getFullYear()}-01-01`;

const COLORES_PIE = ["var(--color-primary)", "var(--color-success)", "var(--color-warning)", "#8b5cf6", "var(--color-danger)", "#06b6d4"];

export default function ReportesPage() {
  const { toastExito, toastError } = useToast();
  const [tab, setTab] = useState<"utilidad" | "balance" | "productos" | "iva">("utilidad");
  const [desde, setDesde] = useState(inicioMes());
  const [hasta, setHasta] = useState(hoy());
  const [utilidad, setUtilidad] = useState<ReporteUtilidad | null>(null);
  const [balance, setBalance] = useState<ReporteBalance | null>(null);
  const [productos, setProductos] = useState<ProductoRentabilidad[]>([]);
  const [cargando, setCargando] = useState(false);

  // Estado para reporte IVA mensual
  const anioActual = new Date().getFullYear();
  const mesActual = new Date().getMonth() + 1;
  const [ivaAnio, setIvaAnio] = useState<number>(anioActual);
  const [ivaMes, setIvaMes] = useState<number>(mesActual);
  const [iva, setIva] = useState<ReporteIva | null>(null);
  const [cargandoIva, setCargandoIva] = useState(false);

  const cargar = async () => {
    if (tab === "iva") return; // IVA se carga manualmente con botón
    setCargando(true);
    try {
      if (tab === "utilidad") setUtilidad(await reporteUtilidad(desde, hasta));
      else if (tab === "balance") setBalance(await reporteBalance(desde, hasta));
      else if (tab === "productos") setProductos(await reporteProductosRentabilidad(desde, hasta, 50));
    } catch (err) {
      toastError("Error: " + err);
    }
    setCargando(false);
  };

  useEffect(() => { cargar(); }, [tab, desde, hasta]);

  const calcularIva = async () => {
    setCargandoIva(true);
    try {
      const r = await reporteIvaMensual(ivaAnio, ivaMes);
      setIva(r);
    } catch (err) {
      toastError("Error: " + err);
    }
    setCargandoIva(false);
  };

  const exportarIvaCsv = async () => {
    if (!iva) return;
    try {
      const nombreMes = MESES[iva.mes - 1] || String(iva.mes);
      const ruta = await save({
        defaultPath: `declaracion-iva-${iva.anio}-${String(iva.mes).padStart(2, "0")}.csv`,
        filters: [{ name: "CSV", extensions: ["csv"] }],
      });
      if (!ruta) return;
      const lineas = [
        `Declaracion IVA - ${nombreMes} ${iva.anio}`,
        `Periodo:,${iva.fecha_desde} a ${iva.fecha_hasta}`,
        ``,
        `Concepto,Monto USD`,
        `Ventas tarifa 0%,${iva.ventas_0.toFixed(2)}`,
        `Ventas tarifa 15% (base imponible),${iva.ventas_15_base.toFixed(2)}`,
        `IVA cobrado en ventas (debito fiscal),${iva.iva_ventas.toFixed(2)}`,
        `(-) Notas de credito base,${iva.nc_base.toFixed(2)}`,
        `(-) Notas de credito IVA,${iva.nc_iva.toFixed(2)}`,
        `IVA ventas neto,${iva.iva_ventas_neto.toFixed(2)}`,
        ``,
        `Compras tarifa 0%,${iva.compras_0.toFixed(2)}`,
        `Compras tarifa 15% (base),${iva.compras_15_base.toFixed(2)}`,
        `IVA pagado en compras (credito tributario),${iva.iva_compras.toFixed(2)}`,
        ``,
        `Total ventas,${iva.total_ventas.toFixed(2)}`,
        `Total compras,${iva.total_compras.toFixed(2)}`,
        `IVA A PAGAR AL SRI,${iva.iva_a_pagar.toFixed(2)}`,
      ];
      const contenido = lineas.join("\n");
      await invoke("guardar_archivo_texto", { ruta, contenido });
      toastExito("CSV exportado");
    } catch (e) {
      toastError("Error: " + e);
    }
  };

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
          {([["utilidad", "Estado de Resultados"], ["balance", "Balance"], ["productos", "Rentabilidad por Producto"], ["iva", "Declaracion IVA"]] as const).map(([key, label]) => (
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
                        <Tooltip formatter={(v: unknown) => fmt(Number(v))} />
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
                        <Pie data={utilidad.gastos_por_categoria} dataKey="monto" nameKey="categoria" cx="50%" cy="50%" outerRadius={80} label={({ name, percent }: { name?: string; percent?: number }) => `${name || ""} ${((percent || 0) * 100).toFixed(0)}%`} labelLine={{ stroke: "var(--color-text-secondary)" }}>
                          {utilidad.gastos_por_categoria.map((_, i) => <Cell key={i} fill={COLORES_PIE[i % COLORES_PIE.length]} />)}
                        </Pie>
                        <Tooltip formatter={(v: unknown) => fmt(Number(v))} />
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

        {/* Declaracion IVA Mensual (Formulario 104 SRI) */}
        {tab === "iva" && (
          <div style={{ display: "flex", flexDirection: "column", gap: 16 }}>
            {/* Selectores de periodo */}
            <div className="card">
              <div className="card-body" style={{ display: "flex", gap: 12, alignItems: "center", flexWrap: "wrap" }}>
                <label style={{ fontSize: 13, fontWeight: 600 }}>Periodo:</label>
                <select
                  className="input"
                  style={{ fontSize: 13, padding: "6px 10px", width: 120 }}
                  value={ivaMes}
                  onChange={e => setIvaMes(Number(e.target.value))}
                >
                  {MESES.map((m, i) => (
                    <option key={i + 1} value={i + 1}>{m}</option>
                  ))}
                </select>
                <select
                  className="input"
                  style={{ fontSize: 13, padding: "6px 10px", width: 100 }}
                  value={ivaAnio}
                  onChange={e => setIvaAnio(Number(e.target.value))}
                >
                  {[2024, 2025, 2026, 2027].map(a => (
                    <option key={a} value={a}>{a}</option>
                  ))}
                </select>
                <button
                  className="btn btn-primary"
                  style={{ fontSize: 13, padding: "6px 16px" }}
                  onClick={calcularIva}
                  disabled={cargandoIva}
                >
                  {cargandoIva ? "Calculando..." : "Calcular"}
                </button>
                {iva && (
                  <button
                    className="btn btn-outline"
                    style={{ fontSize: 13, padding: "6px 16px" }}
                    onClick={exportarIvaCsv}
                  >
                    Exportar CSV
                  </button>
                )}
              </div>
            </div>

            {cargandoIva && (
              <div style={{ textAlign: "center", padding: 40, color: "var(--color-text-secondary)" }}>Calculando...</div>
            )}

            {!cargandoIva && iva && (
              <div className="card">
                <div className="card-header" style={{ fontWeight: 700 }}>
                  Declaracion de IVA - {MESES[iva.mes - 1]} {iva.anio}
                  <span style={{ float: "right", fontSize: 12, color: "var(--color-text-secondary)", fontWeight: 400 }}>
                    Periodo: {iva.fecha_desde} al {iva.fecha_hasta}
                  </span>
                </div>
                <div className="card-body" style={{ padding: "16px 20px" }}>
                  {/* Seccion Ventas */}
                  <div style={{ borderBottom: "2px solid var(--color-border)", paddingBottom: 6, marginBottom: 10, marginTop: 4 }}>
                    <span style={{ fontSize: 13, fontWeight: 700, color: "var(--color-success)" }}>VENTAS</span>
                  </div>
                  <IvaRow label="Ventas tarifa 0% (gravadas 0%)" valor={iva.ventas_0} />
                  <IvaRow label="Ventas tarifa 15% (base imponible)" valor={iva.ventas_15_base} />
                  <IvaRow label="IVA cobrado en ventas (debito fiscal)" valor={iva.iva_ventas} bold />
                  <IvaRow label="(-) Notas de credito base" valor={iva.nc_base} indent color="var(--color-warning)" />
                  <IvaRow label="(-) Notas de credito IVA" valor={iva.nc_iva} indent color="var(--color-warning)" />
                  <IvaRow label="IVA ventas neto" valor={iva.iva_ventas_neto} bold color="var(--color-success)" />

                  {/* Seccion Compras */}
                  <div style={{ borderBottom: "2px solid var(--color-border)", paddingBottom: 6, marginBottom: 10, marginTop: 20 }}>
                    <span style={{ fontSize: 13, fontWeight: 700, color: "var(--color-primary)" }}>COMPRAS</span>
                  </div>
                  <IvaRow label="Compras tarifa 0%" valor={iva.compras_0} />
                  <IvaRow label="Compras tarifa 15% (base)" valor={iva.compras_15_base} />
                  <IvaRow label="IVA pagado en compras (credito tributario)" valor={iva.iva_compras} bold color="var(--color-primary)" />

                  {/* Seccion Resumen */}
                  <div style={{ borderBottom: "2px solid var(--color-border)", paddingBottom: 6, marginBottom: 10, marginTop: 20 }}>
                    <span style={{ fontSize: 13, fontWeight: 700 }}>RESUMEN</span>
                  </div>
                  <IvaRow label="Total ventas" valor={iva.total_ventas} />
                  <IvaRow label="Total compras" valor={iva.total_compras} />

                  {/* IVA A PAGAR - destacado */}
                  <div
                    style={{
                      marginTop: 16,
                      padding: "14px 16px",
                      borderRadius: 8,
                      background: iva.iva_a_pagar > 0
                        ? "rgba(239, 68, 68, 0.15)"
                        : "rgba(34, 197, 94, 0.15)",
                      border: `2px solid ${iva.iva_a_pagar > 0 ? "var(--color-danger)" : "var(--color-success)"}`,
                    }}
                  >
                    <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
                      <div>
                        <div style={{
                          fontSize: 14,
                          fontWeight: 700,
                          color: iva.iva_a_pagar > 0 ? "var(--color-danger)" : "var(--color-success)",
                        }}>
                          {iva.iva_a_pagar > 0 ? "IVA A PAGAR AL SRI" : "CREDITO FISCAL A FAVOR"}
                        </div>
                        <div style={{ fontSize: 11, color: "var(--color-text-secondary)", marginTop: 2 }}>
                          (IVA ventas neto - IVA compras)
                        </div>
                      </div>
                      <div style={{
                        fontSize: 24,
                        fontWeight: 800,
                        color: iva.iva_a_pagar > 0 ? "var(--color-danger)" : "var(--color-success)",
                      }}>
                        {fmt(Math.abs(iva.iva_a_pagar))}
                      </div>
                    </div>
                  </div>

                  {/* Helper text */}
                  <div
                    style={{
                      marginTop: 20,
                      padding: 12,
                      borderRadius: 6,
                      background: "rgba(59, 130, 246, 0.1)",
                      fontSize: 12,
                      color: "var(--color-text-secondary)",
                      lineHeight: 1.5,
                    }}
                  >
                    <strong>Nota:</strong> Este reporte es una ayuda para preparar su declaracion mensual
                    (Formulario 104 del SRI). Consulte con su contador antes de presentar al SRI.
                  </div>
                </div>
              </div>
            )}

            {!cargandoIva && !iva && (
              <div className="card">
                <div className="card-body" style={{ textAlign: "center", padding: 40, color: "var(--color-text-secondary)" }}>
                  Seleccione un periodo y presione <strong>Calcular</strong> para generar el reporte de IVA mensual.
                </div>
              </div>
            )}
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

function IvaRow({ label, valor, bold, indent, color }: { label: string; valor: number; bold?: boolean; indent?: boolean; color?: string }) {
  return (
    <div
      className="flex justify-between"
      style={{
        padding: "6px 0",
        paddingLeft: indent ? 16 : 0,
        fontWeight: bold ? 700 : 400,
        fontSize: bold ? 14 : 13,
      }}
    >
      <span style={{ color: color || "var(--color-text)" }}>{label}</span>
      <span style={{ color: color || "var(--color-text)", fontFamily: "monospace" }}>${valor.toFixed(2)}</span>
    </div>
  );
}
