import { useState, useEffect } from "react";
import { useNavigate } from "react-router-dom";
import {
  resumenDiario, resumenDiarioAyer, resumenFiadosPendientes,
  alertasStockBajo, obtenerCajaAbierta, ventasPorDia,
  productosMasVendidosReporte, ultimasVentasDia, resumenDeudores,
  alertasPagosVencidos, alertasCaducidad, contarTransferenciasPendientes,
} from "../services/api";
import { useSesion } from "../contexts/SesionContext";
import { useTabActivated } from "../contexts/TabsContext";
import ModalTransferenciasPendientes from "../components/ModalTransferenciasPendientes";
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

/** Saludo según hora del día. Buenos días/tardes/noches. */
function saludoHora(): string {
  const h = new Date().getHours();
  if (h < 12) return "Buenos días";
  if (h < 19) return "Buenas tardes";
  return "Buenas noches";
}

/** Fecha formato natural en español: "martes 5 de mayo de 2026". */
function fechaNatural(): string {
  const d = new Date();
  const dias = ["domingo", "lunes", "martes", "miércoles", "jueves", "viernes", "sábado"];
  const meses = [
    "enero", "febrero", "marzo", "abril", "mayo", "junio",
    "julio", "agosto", "septiembre", "octubre", "noviembre", "diciembre",
  ];
  return `${dias[d.getDay()]} ${d.getDate()} de ${meses[d.getMonth()]}`;
}

/** Hora corta: "8:30 a.m." */
function horaCorta(fechaIso: string | undefined): string {
  if (!fechaIso) return "";
  // fecha viene como "2026-05-05 08:30:00" o ISO
  const partes = fechaIso.split(/[ T]/);
  if (partes.length < 2) return "";
  const [hh, mm] = partes[1].split(":");
  const h = parseInt(hh, 10);
  const ampm = h >= 12 ? "p.m." : "a.m.";
  const h12 = h % 12 || 12;
  return `${h12}:${mm} ${ampm}`;
}

export default function DashboardPage() {
  const { sesion, esAdmin, tienePermiso } = useSesion();
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
  const [pagosVencidos, setPagosVencidos] = useState<any[]>([]);
  const [caducidadVencidos, setCaducidadVencidos] = useState(0);
  const [caducidadPorVencer, setCaducidadPorVencer] = useState(0);
  const [transferenciasPendientes, setTransferenciasPendientes] = useState(0);
  // v2.3.64: modal de diagnóstico transferencias
  const [verModalTransferencias, setVerModalTransferencias] = useState(false);
  const [cargando, setCargando] = useState(true);
  const [cajaViejaAbierta, setCajaViejaAbierta] = useState(false);
  // v2.5.16: trigger para recargar el dashboard cuando volvamos a la tab o cuando
  // otra tab dispare evento de venta-completada (sino quedaba stale).
  const [refreshKey, setRefreshKey] = useState(0);

  useEffect(() => {
    const hoy = fechaHoy();
    const hace7 = fechaHace7Dias();
    // Fetch pagos vencidos if user has permission
    if (esAdmin || tienePermiso("gestionar_compras")) {
      alertasPagosVencidos().then(setPagosVencidos).catch(() => {});
    }
    // Fetch alertas adicionales para panel "Atención"
    alertasCaducidad()
      .then(r => {
        setCaducidadVencidos(r.vencidos || 0);
        setCaducidadPorVencer(r.por_vencer || 0);
      })
      .catch(() => {});
    contarTransferenciasPendientes()
      .then(setTransferenciasPendientes)
      .catch(() => {});

    Promise.all([
      resumenDiario(hoy).catch(() => null),
      resumenDiarioAyer().catch(() => null),
      resumenFiadosPendientes().catch(() => 0),
      alertasStockBajo().catch(() => []),
      obtenerCajaAbierta().catch(() => null),
      ventasPorDia(hace7, hoy).catch(() => []),
      productosMasVendidosReporte(hoy, hoy, 10).catch(() => []),
      ultimasVentasDia(5).catch(() => []),
      resumenDeudores().catch(() => []),
    ]).then(([r, ra, f, a, c, vs, tp, uv, d]) => {
      setResumen(r);
      setResumenAyer(ra);
      setFiadosPendientes(f);
      setAlertas(a);
      setCajaAbierta(c);
      // Detectar caja vieja (abierta de un día anterior)
      if (c && c.fecha_apertura) {
        const fechaCaja = c.fecha_apertura.slice(0, 10);
        const hoyStr = fechaHoy();
        if (fechaCaja < hoyStr) {
          setCajaViejaAbierta(true);
        }
      }
      setVentasSemana(vs);
      setTopProductos(tp);
      setUltimasVentas(uv);
      setDeudores(d);
      setCargando(false);
    }).catch(() => setCargando(false));
  }, [refreshKey]);

  // v2.5.16: recargar al volver a la tab Inicio + al detectar venta completada
  // en cualquier otra tab (escucha evento global emitido por POS/ST al cobrar).
  useTabActivated("/", () => setRefreshKey(k => k + 1));
  useEffect(() => {
    const h = () => setRefreshKey(k => k + 1);
    window.addEventListener("clouget:venta-completada", h);
    window.addEventListener("clouget:caja-cambio", h);
    return () => {
      window.removeEventListener("clouget:venta-completada", h);
      window.removeEventListener("clouget:caja-cambio", h);
    };
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
                  Ir a Vender <span className="kbd">F1</span>
                </button>
              ) : (
                <button className="btn btn-primary btn-lg" onClick={() => navigate("/caja")}>
                  Abrir Caja <span className="kbd">F5</span>
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
      {/* Modal advertencia caja vieja */}
      {cajaViejaAbierta && (
        <div className="modal-overlay">
          <div className="modal-content" style={{ maxWidth: 420 }}>
            <div className="modal-header">
              <h3>⚠️ Caja sin cerrar</h3>
            </div>
            <div className="modal-body">
              <p>Hay una caja abierta del día <strong>{cajaAbierta?.fecha_apertura?.slice(0, 10)}</strong> que no fue cerrada.</p>
              <p style={{ marginTop: 8, color: "var(--color-text-secondary)", fontSize: 13 }}>
                Es importante cerrar la caja del día anterior antes de continuar vendiendo para mantener un cuadre correcto.
              </p>
            </div>
            <div className="modal-footer">
              <button className="btn btn-outline" onClick={() => setCajaViejaAbierta(false)}>
                Después
              </button>
              <button className="btn btn-primary" onClick={() => { setCajaViejaAbierta(false); navigate("/caja"); }}>
                Ir a cerrar caja
              </button>
            </div>
          </div>
        </div>
      )}
      {/* Header rediseñado: saludo personalizado + contexto del día.
          Reemplaza "Inicio" + fecha YYYY-MM-DD plana por algo más cálido y útil. */}
      <div className="page-header" style={{ alignItems: "flex-start" }}>
        <div>
          <h2 style={{ marginBottom: 2 }}>
            {saludoHora()}, <span style={{ color: "var(--color-primary)" }}>{sesion?.nombre}</span> 👋
          </h2>
          <div style={{ fontSize: 12, color: "var(--color-text-secondary)", textTransform: "capitalize" }}>
            {fechaNatural()}
            {cajaAbierta?.fecha_apertura && (
              <>
                {" · "}
                <span style={{ color: "var(--color-success)", fontWeight: 600 }}>
                  Caja abierta desde {horaCorta(cajaAbierta.fecha_apertura)}
                </span>
              </>
            )}
            {!cajaAbierta && (
              <>
                {" · "}
                <span style={{ color: "var(--color-danger)", fontWeight: 600 }}>
                  Caja cerrada
                </span>
              </>
            )}
          </div>
        </div>
        <div className="flex gap-4 items-center">
          <button className="btn btn-success" style={{ fontSize: 12, padding: "6px 16px" }}
            onClick={() => navigate("/pos")}>
            + Nueva Venta
          </button>
        </div>
      </div>
      <div className="page-body">
        {/* KPI Hero (estilo Stripe): el numero mas importante (Ventas Hoy) prominente,
            con ticket promedio + transacciones como contexto. Cards secundarios abajo
            para los breakdowns por forma de pago. */}
        <div className="kpi-hero anim-fade-up" style={{
          background: "var(--color-surface)",
          border: "1px solid var(--color-border)",
          borderRadius: 14,
          padding: "20px 24px",
          marginBottom: 14,
          boxShadow: "0 1px 3px rgba(0,0,0,0.04), 0 1px 2px rgba(0,0,0,0.06)",
          display: "grid",
          gridTemplateColumns: "1fr auto",
          gap: 20,
          alignItems: "center",
        }}>
          <div>
            <div style={{ fontSize: 11, fontWeight: 600, letterSpacing: 0.8, color: "var(--color-text-secondary)", textTransform: "uppercase", marginBottom: 4 }}>
              Ventas hoy
            </div>
            <div style={{ display: "flex", alignItems: "baseline", gap: 12, flexWrap: "wrap" }}>
              <span style={{ fontSize: 36, fontWeight: 800, color: "var(--color-text)", lineHeight: 1 }}>
                ${(resumen?.total_ventas ?? 0).toFixed(2)}
              </span>
              {resumenAyer && resumenAyer.total_ventas > 0 && (() => {
                const hoy = resumen?.total_ventas ?? 0;
                const ayer = resumenAyer.total_ventas;
                const pct = ayer > 0 ? ((hoy - ayer) / ayer) * 100 : 0;
                const sube = pct >= 0;
                return (
                  <span style={{
                    fontSize: 13, fontWeight: 700,
                    padding: "3px 9px", borderRadius: 999,
                    background: sube ? "rgba(34,197,94,0.12)" : "rgba(239,68,68,0.12)",
                    color: sube ? "var(--color-success)" : "var(--color-danger)",
                  }}>
                    {sube ? "↑" : "↓"} {Math.abs(pct).toFixed(0)}%
                  </span>
                );
              })()}
            </div>
            <div style={{ fontSize: 12, color: "var(--color-text-secondary)", marginTop: 6 }}>
              {resumen?.num_ventas ?? 0} transacc.
              {(resumen?.num_ventas ?? 0) > 0 && (
                <> · ticket promedio <strong style={{ color: "var(--color-text)" }}>${((resumen?.total_ventas ?? 0) / (resumen?.num_ventas ?? 1)).toFixed(2)}</strong></>
              )}
              {(resumen?.utilidad_bruta ?? 0) > 0 && (
                <> · utilidad <strong style={{ color: "var(--color-success)" }}>${(resumen?.utilidad_bruta ?? 0).toFixed(2)}</strong></>
              )}
            </div>
          </div>
          <div style={{ fontSize: 56, opacity: 0.15, lineHeight: 1, display: "flex", alignItems: "center" }}>
            💰
          </div>
        </div>

        {/* KPIs secundarios: forma de pago + cobros pendientes */}
        <div className="kpi-grid anim-fade-up" style={{
          display: "grid",
          gridTemplateColumns: "repeat(auto-fit, minmax(170px, 1fr))",
          gap: 10,
          marginBottom: 18,
          animationDelay: "60ms",
        }}>
          <KpiCard label="Efectivo" valor={resumen?.total_efectivo ?? 0} ayer={resumenAyer?.total_efectivo} prefix="$" color="var(--color-success)" icon="💵" />
          <KpiCard label="Transferencia" valor={resumen?.total_transferencia ?? 0} ayer={resumenAyer?.total_transferencia} prefix="$" color="var(--color-primary)" icon="🏦" />
          <KpiCard label="Por cobrar" valor={fiadosPendientes} prefix="$" color={fiadosPendientes > 0 ? "var(--color-warning)" : undefined} icon="📋" />
        </div>

        {/* Alertas: Pagos vencidos + Stock bajo */}
        {(pagosVencidos.length > 0 || alertas.length > 0) && (
          <div style={{ display: "flex", gap: 12, marginBottom: 16 }}>
            {pagosVencidos.length > 0 && (esAdmin || tienePermiso("gestionar_compras")) && (
              <div className="card" style={{ flex: 1, borderLeft: "4px solid var(--color-danger)" }}>
                <div className="card-body" style={{ padding: 12 }}>
                  <div style={{ fontWeight: 700, color: "var(--color-danger)", marginBottom: 8 }}>
                    Pagos Vencidos ({pagosVencidos.length})
                  </div>
                  {pagosVencidos.slice(0, 3).map((p: any) => (
                    <div key={p.id} style={{ fontSize: 12, display: "flex", justifyContent: "space-between", marginBottom: 4 }}>
                      <span>{p.proveedor_nombre}</span>
                      <span style={{ color: "var(--color-danger)", fontWeight: 600 }}>
                        ${Number(p.saldo).toFixed(2)} — {Math.floor(p.dias_vencido)} dias
                      </span>
                    </div>
                  ))}
                  {pagosVencidos.length > 3 && (
                    <div style={{ fontSize: 11, color: "var(--color-text-secondary)" }}>
                      y {pagosVencidos.length - 3} mas...
                    </div>
                  )}
                </div>
              </div>
            )}
            {alertas.length > 0 && (esAdmin || tienePermiso("gestionar_inventario") || tienePermiso("gestionar_productos")) && (
              <div className="card" style={{ flex: 1, borderLeft: "4px solid var(--color-warning)" }}>
                <div className="card-body" style={{ padding: 12 }}>
                  <div style={{ fontWeight: 700, color: "var(--color-warning)", marginBottom: 8 }}>
                    Stock Bajo ({alertas.length})
                  </div>
                  {alertas.slice(0, 3).map((a) => (
                    <div key={a.id} style={{ fontSize: 12, display: "flex", justifyContent: "space-between", marginBottom: 4 }}>
                      <span>{a.nombre}</span>
                      <span style={{ fontWeight: 600, color: a.stock_actual <= 0 ? "var(--color-danger)" : "var(--color-warning)" }}>
                        {a.stock_actual} / {a.stock_minimo}
                      </span>
                    </div>
                  ))}
                  {alertas.length > 3 && (
                    <div style={{ fontSize: 11, color: "var(--color-text-secondary)" }}>
                      y {alertas.length - 3} mas...
                    </div>
                  )}
                </div>
              </div>
            )}
          </div>
        )}

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
                    <CartesianGrid strokeDasharray="3 3" stroke="var(--color-border)" />
                    <XAxis dataKey="dia" tick={{ fontSize: 11 }} />
                    <YAxis tick={{ fontSize: 11 }} />
                    <Tooltip
                      formatter={(value) => [`$${Number(value).toFixed(2)}`, "Total"]}
                      labelFormatter={(label) => `Fecha: ${label}`}
                    />
                    <Bar dataKey="total" fill="#3b82f6" radius={[4, 4, 0, 0]} />
                  </BarChart>
                </ResponsiveContainer>
              )}
            </div>
          </div>

          {/* Panel "Atención": alertas inteligentes que requieren accion del usuario.
              Reemplaza "Acciones Rapidas" que duplicaba el sidebar. Solo se muestran las
              alertas con valor > 0 para no saturar. Si no hay nada → mensaje positivo. */}
          {(() => {
            // v2.3.64: action puede ser navegar (ruta) o abrir modal (onClick)
            const alertas: { icon: string; texto: string; ruta?: string; onClick?: () => void; color: string; count: number }[] = [];
            if (transferenciasPendientes > 0)
              alertas.push({
                icon: "🏦",
                texto: `${transferenciasPendientes} transferencia${transferenciasPendientes > 1 ? "s" : ""} por verificar`,
                onClick: () => setVerModalTransferencias(true),
                color: "var(--color-primary)",
                count: transferenciasPendientes,
              });
            if (pagosVencidos.length > 0 && (esAdmin || tienePermiso("gestionar_compras")))
              alertas.push({ icon: "⏰", texto: `${pagosVencidos.length} pago${pagosVencidos.length > 1 ? "s" : ""} vencido${pagosVencidos.length > 1 ? "s" : ""} a proveedores`, ruta: "/pagar", color: "var(--color-danger)", count: pagosVencidos.length });
            if (fiadosPendientes > 0)
              alertas.push({ icon: "💵", texto: `$${fiadosPendientes.toFixed(2)} pendiente de cobro a clientes`, ruta: "/cuentas", color: "var(--color-warning)", count: fiadosPendientes });
            if (caducidadVencidos > 0 && (esAdmin || tienePermiso("gestionar_inventario")))
              alertas.push({ icon: "📅", texto: `${caducidadVencidos} lote${caducidadVencidos > 1 ? "s" : ""} de productos vencido${caducidadVencidos > 1 ? "s" : ""}`, ruta: "/caducidad", color: "var(--color-danger)", count: caducidadVencidos });
            if (caducidadPorVencer > 0 && (esAdmin || tienePermiso("gestionar_inventario")))
              alertas.push({ icon: "⚠", texto: `${caducidadPorVencer} lote${caducidadPorVencer > 1 ? "s" : ""} por vencer pronto`, ruta: "/caducidad", color: "var(--color-warning)", count: caducidadPorVencer });
            if (alertas.length === 0 && alertas.length === 0 && (esAdmin || tienePermiso("gestionar_inventario")) && alertas.length === 0)
              {/* placeholder for alert without items */}

            return (
              <div className="card">
                <div className="card-header">
                  <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", width: "100%" }}>
                    <span>🔔 Atención</span>
                    {alertas.length > 0 && (
                      <span style={{
                        fontSize: 11, fontWeight: 700, padding: "2px 8px", borderRadius: 10,
                        background: "rgba(239, 68, 68, 0.15)", color: "var(--color-danger)",
                      }}>
                        {alertas.length} item{alertas.length > 1 ? "s" : ""}
                      </span>
                    )}
                  </div>
                </div>
                <div className="card-body" style={{ display: "flex", flexDirection: "column", gap: 6, padding: 12 }}>
                  {alertas.length === 0 ? (
                    <div style={{ padding: "20px 0", textAlign: "center", color: "var(--color-text-secondary)" }}>
                      <div style={{ fontSize: 32, marginBottom: 4 }}>✨</div>
                      <div style={{ fontSize: 13, fontWeight: 600 }}>Todo al día</div>
                      <div style={{ fontSize: 11, marginTop: 2 }}>Sin alertas pendientes</div>
                    </div>
                  ) : (
                    alertas.map((a, idx) => (
                      <button
                        key={idx}
                        onClick={() => {
                          if (a.onClick) a.onClick();
                          else if (a.ruta) navigate(a.ruta);
                        }}
                        style={{
                          display: "flex", alignItems: "center", gap: 10,
                          padding: "10px 12px", background: "var(--color-surface)",
                          border: "1px solid var(--color-border)",
                          borderLeft: `3px solid ${a.color}`,
                          borderRadius: 6, cursor: "pointer", textAlign: "left",
                          color: "var(--color-text)",
                          transition: "transform 0.1s, background 0.1s",
                        }}
                        onMouseEnter={(e) => { e.currentTarget.style.background = "var(--color-surface-hover)"; e.currentTarget.style.transform = "translateX(2px)"; }}
                        onMouseLeave={(e) => { e.currentTarget.style.background = "var(--color-surface)"; e.currentTarget.style.transform = "translateX(0)"; }}
                      >
                        <span style={{ fontSize: 18, flexShrink: 0 }}>{a.icon}</span>
                        <span style={{ flex: 1, fontSize: 13, fontWeight: 500 }}>{a.texto}</span>
                        <span style={{ fontSize: 14, color: "var(--color-text-secondary)" }}>›</span>
                      </button>
                    ))
                  )}
                  {/* Caja status: indicador discreto al final, no como item de alerta */}
                  <div style={{
                    marginTop: 8, padding: "8px 12px",
                    background: cajaAbierta ? "rgba(34, 197, 94, 0.08)" : "rgba(239, 68, 68, 0.06)",
                    border: `1px solid ${cajaAbierta ? "rgba(34, 197, 94, 0.2)" : "rgba(239, 68, 68, 0.2)"}`,
                    borderRadius: 6, fontSize: 12,
                    display: "flex", justifyContent: "space-between", alignItems: "center", cursor: "pointer",
                  }} onClick={() => navigate("/caja")}>
                    {cajaAbierta ? (
                      <>
                        <span>💰 Caja abierta · {cajaAbierta.usuario ?? ""}</span>
                        <span style={{ fontWeight: 600 }}>+${cajaAbierta.monto_ventas.toFixed(2)}</span>
                      </>
                    ) : (
                      <>
                        <span style={{ color: "var(--color-danger)", fontWeight: 600 }}>⚠ Caja cerrada</span>
                        <span style={{ color: "var(--color-primary)", fontWeight: 600 }}>Abrir →</span>
                      </>
                    )}
                  </div>
                </div>
              </div>
            );
          })()}
        </div>

        {/* Fila 2: Top productos + Últimas ventas */}
        <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: 16, marginBottom: 16 }}>
          {/* Top 10 productos más vendidos */}
          <div className="card">
            <div className="card-header">Top 10 Productos del Dia</div>
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
                        <div style={{ flex: 1, height: 6, background: "var(--color-border)", borderRadius: 3 }}>
                          <div style={{ width: `${pct}%`, height: "100%", background: "var(--color-primary)", borderRadius: 3 }} />
                        </div>
                        <span style={{ fontSize: 10, color: "var(--color-text-secondary)", minWidth: 40, textAlign: "right" }}>
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
                            background: v.forma_pago === "EFECTIVO" ? "rgba(34, 197, 94, 0.15)" : v.forma_pago === "TRANSFER" ? "rgba(59, 130, 246, 0.15)" : "rgba(245, 158, 11, 0.15)",
                            color: v.forma_pago === "EFECTIVO" ? "var(--color-success)" : v.forma_pago === "TRANSFER" ? "var(--color-primary)" : "var(--color-warning)",
                          }}>
                            {v.forma_pago === "EFECTIVO" ? "Efectivo" : v.forma_pago === "TRANSFER" ? "Transfer" : v.forma_pago === "TARJETA" ? "Tarjeta" : v.forma_pago === "CHEQUE" ? "Cheque" : v.forma_pago === "MIXTO" ? "Mixto" : "Credito"}
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

        {/* Fila 3: Stock Bajo + Cuentas por cobrar */}
        <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: 16 }}>
          {/* Stock Bajo con chips de severidad y barras de progreso */}
          {(() => {
            const sinStock = alertas.filter(a => a.stock_actual <= 0).length;
            const criticos = alertas.length - sinStock;
            return (
              <div className="card">
                <div className="card-header" style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
                  <span>📦 Stock Bajo</span>
                  {alertas.length > 0 && (
                    <div style={{ display: "flex", gap: 6 }}>
                      {sinStock > 0 && (
                        <span style={{
                          fontSize: 10, fontWeight: 700, padding: "2px 7px", borderRadius: 999,
                          background: "rgba(239,68,68,0.15)", color: "var(--color-danger)",
                        }}>
                          🔴 {sinStock} sin stock
                        </span>
                      )}
                      {criticos > 0 && (
                        <span style={{
                          fontSize: 10, fontWeight: 700, padding: "2px 7px", borderRadius: 999,
                          background: "rgba(245,158,11,0.15)", color: "var(--color-warning)",
                        }}>
                          🟠 {criticos} crítico{criticos > 1 ? "s" : ""}
                        </span>
                      )}
                    </div>
                  )}
                </div>
                <div className="card-body" style={{ padding: 0 }}>
                  {alertas.length === 0 ? (
                    <div style={{ padding: 24, textAlign: "center" }}>
                      <div style={{ fontSize: 28, marginBottom: 4 }}>✨</div>
                      <div style={{ fontSize: 13, fontWeight: 600, color: "var(--color-success)" }}>
                        Stock OK
                      </div>
                      <div style={{ fontSize: 11, color: "var(--color-text-secondary)", marginTop: 2 }}>
                        Todos los productos con stock suficiente
                      </div>
                    </div>
                  ) : (
                    alertas.slice(0, 8).map((a) => {
                      const pct = a.stock_minimo > 0 ? Math.min((a.stock_actual / a.stock_minimo) * 100, 100) : 0;
                      const esAgotado = a.stock_actual <= 0;
                      const colorBarra = esAgotado ? "var(--color-danger)" : pct < 50 ? "var(--color-warning)" : "var(--color-success)";
                      return (
                        <div key={a.id} style={{
                          padding: "8px 12px", borderBottom: "1px solid var(--color-border)",
                          background: esAgotado ? "rgba(239, 68, 68, 0.05)" : undefined,
                        }}>
                          <div style={{ display: "flex", justifyContent: "space-between", fontSize: 12, marginBottom: 4, alignItems: "center" }}>
                            <span style={{ fontWeight: 500, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap", flex: 1, marginRight: 8 }}>
                              {a.nombre}
                            </span>
                            <span style={{
                              fontSize: 11, fontWeight: 700,
                              padding: "2px 7px", borderRadius: 4,
                              background: esAgotado ? "rgba(239,68,68,0.12)" : "rgba(245,158,11,0.12)",
                              color: esAgotado ? "var(--color-danger)" : "var(--color-warning)",
                              flexShrink: 0,
                            }}>
                              {a.stock_actual} / {a.stock_minimo}
                            </span>
                          </div>
                          <div style={{ height: 5, background: "var(--color-border)", borderRadius: 3, overflow: "hidden" }}>
                            <div style={{
                              width: `${Math.max(pct, esAgotado ? 0 : 4)}%`, height: "100%", borderRadius: 3,
                              background: colorBarra, transition: "width 0.3s ease",
                            }} />
                          </div>
                        </div>
                      );
                    })
                  )}
                  {alertas.length > 8 && (
                    <button onClick={() => navigate("/inventario")} style={{
                      width: "100%", padding: "8px",
                      background: "transparent", border: "none",
                      color: "var(--color-primary)", fontSize: 12, fontWeight: 600, cursor: "pointer",
                      borderTop: "1px solid var(--color-border)",
                    }}>
                      Ver los {alertas.length - 8} restantes →
                    </button>
                  )}
                </div>
              </div>
            );
          })()}

          {/* Resumen de cuentas por cobrar / deudores */}
          <div className="card" style={{ borderColor: deudores.length > 0 ? "var(--color-danger)" : undefined }}>
            <div className="card-header" style={deudores.length > 0 ? { background: "rgba(239, 68, 68, 0.1)", color: "var(--color-danger)" } : {}}>
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
                      <div style={{ fontSize: 10, color: "var(--color-text-secondary)" }}>{d.num_cuentas} cuenta{d.num_cuentas !== 1 ? "s" : ""}</div>
                    </div>
                    <span style={{ fontWeight: 700, color: "var(--color-danger)", fontSize: 13 }}>
                      ${d.total_deuda.toFixed(2)}
                    </span>
                  </div>
                ))
              )}
            </div>
          </div>
        </div>
      </div>

      {/* v2.3.64: modal diagnóstico transferencias pendientes */}
      {verModalTransferencias && (
        <ModalTransferenciasPendientes
          onCerrar={() => setVerModalTransferencias(false)}
          onCambio={() => {
            // Refrescar el contador después de forzar verificación
            import("../services/api").then(({ contarTransferenciasPendientes }) => {
              contarTransferenciasPendientes().then(setTransferenciasPendientes).catch(() => {});
            });
          }}
        />
      )}
    </>
  );
}

/* --- KPI Card con comparativo vs ayer --- */
function KpiCard({ label, valor, ayer, prefix, color, icon }: {
  label: string; valor: number; ayer?: number | null; prefix?: string; color?: string; icon?: string;
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
    <div className="card kpi-card" style={{
      padding: 14,
      position: "relative",
      overflow: "hidden",
    }}>
      <div style={{ display: "flex", justifyContent: "space-between", alignItems: "flex-start" }}>
        <div className="text-secondary" style={{ fontSize: 11, fontWeight: 600, letterSpacing: 0.4, textTransform: "uppercase" }}>{label}</div>
        {icon && (
          <span style={{ fontSize: 14, opacity: 0.4 }}>{icon}</span>
        )}
      </div>
      <div style={{ fontSize: 22, fontWeight: 700, marginTop: 4, color }}>{formatted}</div>
      {diff !== null && diffPct !== null && (
        <div style={{
          fontSize: 10, marginTop: 4,
          color: diff > 0 ? "var(--color-success)" : diff < 0 ? "var(--color-danger)" : "var(--color-text-secondary)",
        }}>
          {diff > 0 ? "↑" : diff < 0 ? "↓" : "="}{" "}
          {Math.abs(diffPct).toFixed(0)}% vs ayer
        </div>
      )}
    </div>
  );
}
