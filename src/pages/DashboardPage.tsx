import { useState, useEffect } from "react";
import { useNavigate } from "react-router-dom";
import { resumenDiario, resumenFiadosPendientes, alertasStockBajo, obtenerCajaAbierta } from "../services/api";
import { useSesion } from "../contexts/SesionContext";
import type { ResumenDiario, AlertaStock } from "../services/api";
import type { Caja } from "../types";

function fechaHoy(): string {
  const now = new Date();
  const y = now.getFullYear();
  const m = String(now.getMonth() + 1).padStart(2, "0");
  const d = String(now.getDate()).padStart(2, "0");
  return `${y}-${m}-${d}`;
}

export default function DashboardPage() {
  const { sesion, esAdmin } = useSesion();
  const navigate = useNavigate();
  const [resumen, setResumen] = useState<ResumenDiario | null>(null);
  const [fiadosPendientes, setFiadosPendientes] = useState(0);
  const [alertas, setAlertas] = useState<AlertaStock[]>([]);
  const [cajaAbierta, setCajaAbierta] = useState<Caja | null>(null);
  const [cargando, setCargando] = useState(true);

  useEffect(() => {
    const hoy = fechaHoy();
    Promise.all([
      resumenDiario(hoy),
      resumenFiadosPendientes(),
      alertasStockBajo(),
      obtenerCajaAbierta(),
    ]).then(([r, f, a, c]) => {
      setResumen(r);
      setFiadosPendientes(f);
      setAlertas(a);
      setCajaAbierta(c);
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
        {/* KPI Cards */}
        <div style={{ display: "grid", gridTemplateColumns: "repeat(auto-fit, minmax(160px, 1fr))", gap: 12, marginBottom: 24 }}>
          <KpiCard label="Ventas Hoy" valor={`$${resumen?.total_ventas.toFixed(2) ?? "0.00"}`} color="var(--color-success)" />
          <KpiCard label="Transacciones" valor={String(resumen?.num_ventas ?? 0)} />
          <KpiCard label="Utilidad Bruta" valor={`$${resumen?.utilidad_bruta.toFixed(2) ?? "0.00"}`} color="var(--color-success)" />
          <KpiCard label="Efectivo" valor={`$${resumen?.total_efectivo.toFixed(2) ?? "0.00"}`} />
          <KpiCard label="Transferencia" valor={`$${resumen?.total_transferencia.toFixed(2) ?? "0.00"}`} />
          <KpiCard label="Fiados Pendientes" valor={`$${fiadosPendientes.toFixed(2)}`} color={fiadosPendientes > 0 ? "var(--color-warning)" : undefined} />
        </div>

        <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: 16 }}>
          {/* Acciones rapidas */}
          <div className="card">
            <div className="card-header">Acciones Rapidas</div>
            <div className="card-body" style={{ display: "flex", flexDirection: "column", gap: 8 }}>
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

          {/* Alertas de stock */}
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
                alertas.slice(0, 10).map((a) => (
                  <div key={a.id} className="flex justify-between"
                    style={{ padding: "6px 12px", borderBottom: "1px solid var(--color-border)", fontSize: 12 }}>
                    <span>{a.nombre}</span>
                    <span style={{
                      fontWeight: 600,
                      color: a.stock_actual <= 0 ? "#dc2626" : "#d97706",
                    }}>
                      {a.stock_actual} / {a.stock_minimo}
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

function KpiCard({ label, valor, color }: { label: string; valor: string; color?: string }) {
  return (
    <div className="card" style={{ padding: 14 }}>
      <div className="text-secondary" style={{ fontSize: 11 }}>{label}</div>
      <div className="text-xl font-bold" style={{ color }}>{valor}</div>
    </div>
  );
}
