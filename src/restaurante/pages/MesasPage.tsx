/**
 * MesasPage — Grid visual de mesas del restaurante.
 *
 * Estados de mesa:
 *   - LIBRE          → click abre modal "Abrir pedido"
 *   - OCUPADA        → click abre PedidoDetalle (componente)
 *   - CUENTA_PEDIDA  → mismo PedidoDetalle, botón Cobrar destacado
 *
 * Auto-refresh cada 15s para actualizar tiempo de mesas abiertas.
 */

import { useState, useEffect, useCallback, useMemo } from "react";
import { useNavigate } from "react-router-dom";
import { useToast } from "../../components/Toast";
import { useSesion } from "../../contexts/SesionContext";
import {
  listarMesasConEstado,
  listarZonas,
  abrirPedido,
} from "../api";
import type { MesaConEstado, Zona } from "../types";
import PedidoDetalle from "../components/PedidoDetalle";

export default function MesasPage() {
  const { toastExito, toastError } = useToast();
  const { sesion, esAdmin } = useSesion();
  const navigate = useNavigate();
  const [mesas, setMesas] = useState<MesaConEstado[]>([]);
  const [zonas, setZonas] = useState<Zona[]>([]);
  const [filtroZona, setFiltroZona] = useState<number | "todas">("todas");
  const [cargando, setCargando] = useState(true);

  // Modal "Abrir pedido"
  const [mesaAbriendo, setMesaAbriendo] = useState<MesaConEstado | null>(null);
  const [comensalesInput, setComensalesInput] = useState("1");

  // Drawer "Detalle pedido"
  const [pedidoActivoId, setPedidoActivoId] = useState<number | null>(null);

  const cargar = useCallback(async () => {
    try {
      const [m, z] = await Promise.all([listarMesasConEstado(), listarZonas()]);
      setMesas(m);
      setZonas(z);
    } catch (err: any) {
      toastError("Error cargando mesas: " + (err?.message || err));
    } finally {
      setCargando(false);
    }
  }, [toastError]);

  useEffect(() => {
    cargar();
    const intervalo = setInterval(cargar, 15000);
    return () => clearInterval(intervalo);
  }, [cargar]);

  const mesasFiltradas = useMemo(
    () =>
      filtroZona === "todas"
        ? mesas
        : mesas.filter((m) => m.zona_id === filtroZona),
    [mesas, filtroZona],
  );

  // Resumen para barra superior
  const resumen = useMemo(() => {
    const libres = mesas.filter((m) => m.estado === "LIBRE").length;
    const ocupadas = mesas.filter((m) => m.estado === "OCUPADA").length;
    const cuenta = mesas.filter((m) => m.estado === "CUENTA_PEDIDA").length;
    const totalActivo = mesas.reduce((s, m) => s + (m.total_actual || 0), 0);
    return { libres, ocupadas, cuenta, totalActivo };
  }, [mesas]);

  const handleClickMesa = (mesa: MesaConEstado) => {
    if (mesa.estado === "LIBRE") {
      setMesaAbriendo(mesa);
      setComensalesInput("1");
    } else if (mesa.pedido_id) {
      setPedidoActivoId(mesa.pedido_id);
    }
  };

  const handleConfirmarAbrir = async () => {
    if (!mesaAbriendo) return;
    try {
      const pedidoId = await abrirPedido({
        mesaId: mesaAbriendo.id,
        meseroId: sesion?.usuario_id ?? null,
        meseroNombre: sesion?.nombre ?? null,
        comensales: Math.max(1, parseInt(comensalesInput, 10) || 1),
      });
      toastExito(`Pedido abierto en ${mesaAbriendo.nombre}`);
      setMesaAbriendo(null);
      await cargar();
      setPedidoActivoId(pedidoId);
    } catch (err: any) {
      toastError("No se pudo abrir el pedido: " + (err?.message || err));
    }
  };

  const handleCerrarDrawer = async (recargar: boolean) => {
    setPedidoActivoId(null);
    if (recargar) await cargar();
  };

  if (cargando) {
    return (
      <div style={{ padding: 32, textAlign: "center", color: "var(--color-text-muted)" }}>
        Cargando mesas...
      </div>
    );
  }

  return (
    <div style={{ padding: "16px 20px", display: "flex", flexDirection: "column", gap: 16 }}>
      {/* Header con resumen */}
      <header style={{ display: "flex", justifyContent: "space-between", alignItems: "center", flexWrap: "wrap", gap: 12 }}>
        <div>
          <h1 style={{ margin: 0, fontSize: 22, fontWeight: 700 }}>Mesas</h1>
          <p style={{ margin: "4px 0 0 0", fontSize: 13, color: "var(--color-text-muted)" }}>
            {mesas.length} mesas · {resumen.libres} libres · {resumen.ocupadas} ocupadas
            {resumen.cuenta > 0 && ` · ${resumen.cuenta} esperando cuenta`}
          </p>
        </div>
        <div style={{ display: "flex", gap: 12, alignItems: "center" }}>
          {esAdmin && (
            <button
              onClick={() => navigate("/config-mesas")}
              className="btn btn-outline"
              style={{ padding: "6px 12px", fontSize: 12 }}
              title="Configurar zonas y mesas"
            >
              ⚙ Configurar
            </button>
          )}
          <ResumenBadge label="Total abierto" valor={`$${resumen.totalActivo.toFixed(2)}`} color="var(--color-primary)" />
        </div>
      </header>

      {/* Filtro por zona */}
      {zonas.length > 1 && (
        <div style={{ display: "flex", gap: 6, flexWrap: "wrap" }}>
          <ZonaChip
            label="Todas"
            color="var(--color-text-muted)"
            activa={filtroZona === "todas"}
            onClick={() => setFiltroZona("todas")}
          />
          {zonas.map((z) => (
            <ZonaChip
              key={z.id}
              label={z.nombre}
              color={z.color}
              activa={filtroZona === z.id}
              onClick={() => setFiltroZona(z.id!)}
            />
          ))}
        </div>
      )}

      {/* Grid de mesas */}
      {mesasFiltradas.length === 0 ? (
        <div
          style={{
            padding: 40,
            textAlign: "center",
            color: "var(--color-text-muted)",
            border: "2px dashed var(--color-border)",
            borderRadius: 12,
          }}
        >
          {mesas.length === 0
            ? "Aún no has configurado mesas. Ve a Config → Mesas para agregar."
            : "No hay mesas en esta zona."}
        </div>
      ) : (
        <div
          style={{
            display: "grid",
            gridTemplateColumns: "repeat(auto-fill, minmax(150px, 1fr))",
            gap: 10,
          }}
        >
          {mesasFiltradas.map((m) => (
            <CardMesa key={m.id} mesa={m} onClick={() => handleClickMesa(m)} />
          ))}
        </div>
      )}

      {/* Modal "Abrir pedido" */}
      {mesaAbriendo && (
        <div className="modal-overlay" onClick={() => setMesaAbriendo(null)}>
          <div
            className="modal-content"
            onClick={(e) => e.stopPropagation()}
            style={{ maxWidth: 380 }}
          >
            <div className="modal-header">
              <h3 style={{ margin: 0 }}>Abrir pedido — {mesaAbriendo.nombre}</h3>
            </div>
            <div className="modal-body" style={{ display: "flex", flexDirection: "column", gap: 12 }}>
              {mesaAbriendo.zona_nombre && (
                <div style={{ fontSize: 13, color: "var(--color-text-muted)" }}>
                  Zona: {mesaAbriendo.zona_nombre}
                </div>
              )}
              <label style={{ fontSize: 13, fontWeight: 600 }}>
                Número de comensales
                <input
                  type="number"
                  min="1"
                  max={mesaAbriendo.capacidad * 2 || 99}
                  value={comensalesInput}
                  onChange={(e) => setComensalesInput(e.target.value)}
                  className="input"
                  style={{ marginTop: 4, width: "100%" }}
                  autoFocus
                  onKeyDown={(e) => e.key === "Enter" && handleConfirmarAbrir()}
                />
              </label>
              <div style={{ fontSize: 12, color: "var(--color-text-muted)" }}>
                Mesero: <strong>{sesion?.nombre || "—"}</strong>
              </div>
            </div>
            <div className="modal-footer" style={{ display: "flex", gap: 8, justifyContent: "flex-end" }}>
              <button className="btn btn-outline" onClick={() => setMesaAbriendo(null)}>
                Cancelar
              </button>
              <button className="btn btn-primary" onClick={handleConfirmarAbrir}>
                Abrir pedido
              </button>
            </div>
          </div>
        </div>
      )}

      {/* Drawer "Detalle pedido" */}
      {pedidoActivoId && (
        <PedidoDetalle
          pedidoId={pedidoActivoId}
          onCerrar={handleCerrarDrawer}
        />
      )}
    </div>
  );
}

// ─── Sub-componentes ────────────────────────────────────────────────────

function ResumenBadge({ label, valor, color }: { label: string; valor: string; color: string }) {
  return (
    <div
      style={{
        background: "var(--color-surface)",
        border: "1px solid var(--color-border)",
        borderRadius: 8,
        padding: "6px 12px",
        display: "flex",
        flexDirection: "column",
        alignItems: "flex-end",
      }}
    >
      <span style={{ fontSize: 11, color: "var(--color-text-muted)", lineHeight: 1 }}>{label}</span>
      <strong style={{ fontSize: 16, color, lineHeight: 1.2 }}>{valor}</strong>
    </div>
  );
}

function ZonaChip({
  label,
  color,
  activa,
  onClick,
}: {
  label: string;
  color: string;
  activa: boolean;
  onClick: () => void;
}) {
  return (
    <button
      onClick={onClick}
      style={{
        padding: "6px 14px",
        borderRadius: 999,
        border: `1.5px solid ${activa ? color : "var(--color-border)"}`,
        background: activa ? color : "transparent",
        color: activa ? "#fff" : "var(--color-text)",
        fontSize: 13,
        fontWeight: 600,
        cursor: "pointer",
        transition: "all 0.15s",
      }}
    >
      {label}
    </button>
  );
}

function CardMesa({ mesa, onClick }: { mesa: MesaConEstado; onClick: () => void }) {
  const colorBorde =
    mesa.estado === "LIBRE"
      ? "var(--color-border)"
      : mesa.estado === "CUENTA_PEDIDA"
        ? "var(--color-warning)"
        : "var(--color-success)";

  const colorFondo =
    mesa.estado === "LIBRE"
      ? "var(--color-surface)"
      : mesa.estado === "CUENTA_PEDIDA"
        ? "rgba(245, 158, 11, 0.1)"
        : "rgba(34, 197, 94, 0.1)";

  const labelEstado =
    mesa.estado === "LIBRE"
      ? "LIBRE"
      : mesa.estado === "CUENTA_PEDIDA"
        ? "CUENTA"
        : "OCUPADA";

  return (
    <button
      onClick={onClick}
      style={{
        position: "relative",
        padding: "12px 10px",
        border: `2px solid ${colorBorde}`,
        borderRadius: 12,
        background: colorFondo,
        cursor: "pointer",
        textAlign: "left",
        display: "flex",
        flexDirection: "column",
        gap: 6,
        minHeight: 110,
        transition: "transform 0.1s, box-shadow 0.1s",
        color: "var(--color-text)",
      }}
      onMouseEnter={(e) => (e.currentTarget.style.transform = "translateY(-2px)")}
      onMouseLeave={(e) => (e.currentTarget.style.transform = "translateY(0)")}
    >
      <div style={{ display: "flex", justifyContent: "space-between", alignItems: "flex-start" }}>
        <strong style={{ fontSize: 16 }}>{mesa.nombre}</strong>
        <span
          style={{
            fontSize: 9,
            fontWeight: 700,
            padding: "2px 6px",
            borderRadius: 4,
            background: colorBorde,
            color: "#fff",
            letterSpacing: 0.5,
          }}
        >
          {labelEstado}
        </span>
      </div>

      {mesa.zona_nombre && (
        <span style={{ fontSize: 10, color: "var(--color-text-muted)" }}>
          {mesa.zona_nombre} · {mesa.capacidad} pax
        </span>
      )}

      {mesa.estado !== "LIBRE" && (
        <>
          {mesa.mesero_nombre && (
            <div style={{ fontSize: 11, color: "var(--color-text-muted)", marginTop: 2 }}>
              👤 {mesa.mesero_nombre}
            </div>
          )}
          <div style={{ display: "flex", justifyContent: "space-between", alignItems: "baseline", marginTop: "auto" }}>
            <strong style={{ fontSize: 18, color: "var(--color-primary)" }}>
              ${mesa.total_actual.toFixed(2)}
            </strong>
            {mesa.minutos_abierta != null && (
              <span style={{ fontSize: 11, color: "var(--color-text-muted)" }}>
                {mesa.minutos_abierta < 60
                  ? `${mesa.minutos_abierta}m`
                  : `${Math.floor(mesa.minutos_abierta / 60)}h ${mesa.minutos_abierta % 60}m`}
              </span>
            )}
          </div>
          {mesa.items_pendientes_cocina > 0 && (
            <div
              style={{
                position: "absolute",
                top: 6,
                right: 6,
                background: "var(--color-danger)",
                color: "#fff",
                fontSize: 10,
                fontWeight: 700,
                padding: "2px 6px",
                borderRadius: 999,
                lineHeight: 1.2,
              }}
              title="Items pendientes en cocina"
            >
              🔔 {mesa.items_pendientes_cocina}
            </div>
          )}
        </>
      )}
    </button>
  );
}
