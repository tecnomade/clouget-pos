/**
 * CocinaPage — Vista para la pantalla de cocina (TV o tablet).
 *
 * Muestra todos los items enviados a cocina, ordenados por antigüedad.
 * El cocinero/parrillero hace click para cambiar estado:
 *
 *   PENDIENTE  →  EN_PREPARACION  →  LISTO  →  ENTREGADO (desaparece)
 *
 * Auto-refresh cada 8 segundos para detectar items nuevos sin recargar.
 *
 * Modo kiosko: pensado para tener esta página abierta en una pantalla fija
 * en la cocina (puede ser una TV con un mini PC, o una tablet montada).
 */

import { useState, useEffect, useCallback, useMemo } from "react";
import { useToast } from "../../components/Toast";
import { listarItemsCocinaPendientes, marcarItemCocina } from "../api";
import type { ItemCocina, EstadoCocina } from "../types";
import { usePausableInterval } from "../../hooks/usePausableInterval";

const REFRESH_MS = 8000;

export default function CocinaPage() {
  const { toastExito, toastError } = useToast();
  const [items, setItems] = useState<ItemCocina[]>([]);
  const [filtroEstado, setFiltroEstado] = useState<"TODOS" | EstadoCocina>("TODOS");
  const [cargando, setCargando] = useState(true);
  const [actualizando, setActualizando] = useState(false);

  const cargar = useCallback(
    async (silencioso = false) => {
      if (!silencioso) setActualizando(true);
      try {
        const data = await listarItemsCocinaPendientes();
        setItems(data);
      } catch (err: any) {
        if (!silencioso) toastError("Error: " + (err?.message || err));
      } finally {
        setActualizando(false);
        setCargando(false);
      }
    },
    [toastError],
  );

  useEffect(() => {
    cargar();
  }, [cargar]);

  // v2.5.60: auto-refresh cada 8s, pausa cuando la tab no está activa.
  // En kiosko (TV cocina) la tab siempre está activa → polling sigue normal.
  // En POS multi-tab, pausa al cambiar y refresca inmediato al volver.
  usePausableInterval(() => cargar(true), REFRESH_MS, "/cocina", { runOnReactivate: true });

  const conteos = useMemo(() => {
    const c = { PENDIENTE: 0, EN_PREPARACION: 0, LISTO: 0 };
    for (const i of items) {
      if (i.estado_cocina in c) c[i.estado_cocina as keyof typeof c]++;
    }
    return c;
  }, [items]);

  const filtrados = useMemo(
    () => (filtroEstado === "TODOS" ? items : items.filter((i) => i.estado_cocina === filtroEstado)),
    [items, filtroEstado],
  );

  // Agrupar por mesa para mejor visualización
  const porMesa = useMemo(() => {
    const m = new Map<string, ItemCocina[]>();
    for (const i of filtrados) {
      const key = `${i.zona_nombre || "—"} · ${i.mesa_nombre}`;
      if (!m.has(key)) m.set(key, []);
      m.get(key)!.push(i);
    }
    return Array.from(m.entries()).map(([mesa, its]) => ({
      mesa,
      items: its,
      mesero: its[0]?.mesero_nombre,
      antiguedadMax: Math.max(...its.map((i) => i.minutos_en_cocina ?? 0)),
    }));
  }, [filtrados]);

  const handleCambiarEstado = async (item: ItemCocina) => {
    const siguiente: Record<string, EstadoCocina> = {
      PENDIENTE: "EN_PREPARACION",
      EN_PREPARACION: "LISTO",
      LISTO: "ENTREGADO",
    };
    const nuevo = siguiente[item.estado_cocina];
    if (!nuevo) return;
    try {
      await marcarItemCocina(item.id, nuevo);
      if (nuevo === "ENTREGADO") toastExito("Entregado ✓");
      await cargar(true);
    } catch (err: any) {
      toastError(err?.message || String(err));
    }
  };

  if (cargando) {
    return (
      <div style={{ padding: 32, textAlign: "center", color: "var(--color-text-muted)" }}>
        Cargando items de cocina...
      </div>
    );
  }

  return (
    <div style={{ padding: "16px 20px", display: "flex", flexDirection: "column", gap: 14, minHeight: "100%" }}>
      {/* Header */}
      <header style={{ display: "flex", justifyContent: "space-between", alignItems: "flex-end", flexWrap: "wrap", gap: 8 }}>
        <div>
          <h1 style={{ margin: 0, fontSize: 22, fontWeight: 700 }}>🍳 Cocina</h1>
          <p style={{ margin: "4px 0 0 0", fontSize: 12, color: "var(--color-text-muted)" }}>
            Auto-actualización cada {REFRESH_MS / 1000}s {actualizando && "· refrescando..."}
          </p>
        </div>
        <div style={{ display: "flex", gap: 6 }}>
          <FiltroChip
            label={`Todos (${items.length})`}
            color="var(--color-text-muted)"
            activa={filtroEstado === "TODOS"}
            onClick={() => setFiltroEstado("TODOS")}
          />
          <FiltroChip
            label={`Pendientes (${conteos.PENDIENTE})`}
            color="#dc2626"
            activa={filtroEstado === "PENDIENTE"}
            onClick={() => setFiltroEstado("PENDIENTE")}
          />
          <FiltroChip
            label={`En cocina (${conteos.EN_PREPARACION})`}
            color="#f59e0b"
            activa={filtroEstado === "EN_PREPARACION"}
            onClick={() => setFiltroEstado("EN_PREPARACION")}
          />
          <FiltroChip
            label={`Listos (${conteos.LISTO})`}
            color="#16a34a"
            activa={filtroEstado === "LISTO"}
            onClick={() => setFiltroEstado("LISTO")}
          />
        </div>
      </header>

      {/* Lista por mesa */}
      {porMesa.length === 0 ? (
        <div
          style={{
            padding: 60,
            textAlign: "center",
            color: "var(--color-text-muted)",
            border: "2px dashed var(--color-border)",
            borderRadius: 12,
            fontSize: 16,
          }}
        >
          {items.length === 0
            ? "🌟 Sin pedidos pendientes — ¡cocina al día!"
            : "Sin items en este filtro"}
        </div>
      ) : (
        <div
          style={{
            display: "grid",
            gridTemplateColumns: "repeat(auto-fill, minmax(320px, 1fr))",
            gap: 14,
          }}
        >
          {porMesa.map((g) => (
            <CardMesaCocina key={g.mesa} grupo={g} onCambiarEstado={handleCambiarEstado} />
          ))}
        </div>
      )}
    </div>
  );
}

// ─── Sub-componentes ────────────────────────────────────────────────────

function FiltroChip({
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
        padding: "6px 12px",
        borderRadius: 999,
        border: `1.5px solid ${activa ? color : "var(--color-border)"}`,
        background: activa ? color : "transparent",
        color: activa ? "#fff" : "var(--color-text)",
        fontSize: 12,
        fontWeight: 600,
        cursor: "pointer",
        whiteSpace: "nowrap",
      }}
    >
      {label}
    </button>
  );
}

function CardMesaCocina({
  grupo,
  onCambiarEstado,
}: {
  grupo: { mesa: string; items: ItemCocina[]; mesero?: string | null; antiguedadMax: number };
  onCambiarEstado: (item: ItemCocina) => void;
}) {
  // Color de borde según antigüedad (urgencia)
  const colorUrgencia =
    grupo.antiguedadMax < 5
      ? "var(--color-border)"
      : grupo.antiguedadMax < 15
        ? "var(--color-warning)"
        : "var(--color-danger)";

  return (
    <div
      style={{
        background: "var(--color-surface)",
        border: `2px solid ${colorUrgencia}`,
        borderRadius: 10,
        overflow: "hidden",
        display: "flex",
        flexDirection: "column",
      }}
    >
      <div
        style={{
          padding: "10px 14px",
          background: colorUrgencia,
          color: "#fff",
          display: "flex",
          justifyContent: "space-between",
          alignItems: "center",
        }}
      >
        <strong style={{ fontSize: 14 }}>{grupo.mesa}</strong>
        <span style={{ fontSize: 13, fontWeight: 700 }}>
          ⏱ {grupo.antiguedadMax}m
        </span>
      </div>
      {grupo.mesero && (
        <div
          style={{
            padding: "4px 14px",
            fontSize: 11,
            color: "var(--color-text-muted)",
            background: "var(--color-surface-hover)",
            borderBottom: "1px solid var(--color-border)",
          }}
        >
          👤 {grupo.mesero}
        </div>
      )}
      <div style={{ display: "flex", flexDirection: "column" }}>
        {grupo.items.map((item) => (
          <ItemCocinaRow key={item.id} item={item} onClick={() => onCambiarEstado(item)} />
        ))}
      </div>
    </div>
  );
}

const ESTADO_CONFIG: Record<string, { color: string; label: string; siguiente: string }> = {
  PENDIENTE: { color: "#dc2626", label: "PENDIENTE", siguiente: "▶ Empezar" },
  EN_PREPARACION: { color: "#f59e0b", label: "EN COCINA", siguiente: "✓ Listo" },
  LISTO: { color: "#16a34a", label: "LISTO", siguiente: "📦 Entregar" },
};

function ItemCocinaRow({ item, onClick }: { item: ItemCocina; onClick: () => void }) {
  const config = ESTADO_CONFIG[item.estado_cocina] ?? {
    color: "var(--color-text-muted)",
    label: item.estado_cocina,
    siguiente: "",
  };

  return (
    <button
      onClick={onClick}
      style={{
        textAlign: "left",
        padding: "10px 14px",
        background: "transparent",
        border: "none",
        borderTop: "1px solid var(--color-border)",
        cursor: "pointer",
        display: "flex",
        alignItems: "center",
        gap: 10,
        transition: "background 0.1s",
        color: "var(--color-text)",
      }}
      onMouseEnter={(e) => (e.currentTarget.style.background = "var(--color-surface-hover)")}
      onMouseLeave={(e) => (e.currentTarget.style.background = "transparent")}
    >
      <span
        style={{
          minWidth: 36,
          height: 36,
          background: config.color,
          color: "#fff",
          borderRadius: 6,
          display: "flex",
          alignItems: "center",
          justifyContent: "center",
          fontWeight: 700,
          fontSize: 16,
          flexShrink: 0,
        }}
      >
        {item.cantidad}
      </span>
      <div style={{ flex: 1, minWidth: 0 }}>
        <div style={{ fontSize: 14, fontWeight: 600 }}>{item.producto_nombre}</div>
        {item.info_adicional && (
          <div style={{ fontSize: 11, color: "var(--color-text-muted)", fontStyle: "italic" }}>
            ↳ {item.info_adicional}
          </div>
        )}
        <div style={{ fontSize: 10, color: config.color, fontWeight: 700, marginTop: 2 }}>
          {config.label}
        </div>
      </div>
      {config.siguiente && (
        <span
          style={{
            fontSize: 11,
            fontWeight: 700,
            padding: "4px 10px",
            background: config.color,
            color: "#fff",
            borderRadius: 6,
            whiteSpace: "nowrap",
          }}
        >
          {config.siguiente}
        </span>
      )}
    </button>
  );
}

