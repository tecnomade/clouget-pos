import { useState, useEffect } from "react";
import { onConnectionStatusChange } from "../services/offlineSync";

/**
 * Indicador de estado de conexión para terminales en modo cliente.
 * Muestra: online/offline + operaciones pendientes en cola.
 */
export default function ConnectionStatus() {
  const [online, setOnline] = useState(true);
  const [pendientes, setPendientes] = useState(0);

  useEffect(() => {
    onConnectionStatusChange((isOnline, count) => {
      setOnline(isOnline);
      setPendientes(count);
    });
  }, []);

  // Solo mostrar si hay algo que informar
  if (online && pendientes === 0) return null;

  return (
    <div
      style={{
        position: "fixed",
        bottom: 12,
        right: 12,
        padding: "8px 16px",
        borderRadius: 8,
        fontSize: 13,
        fontWeight: 600,
        zIndex: 9999,
        display: "flex",
        alignItems: "center",
        gap: 8,
        background: online ? "rgba(34,197,94,0.15)" : "rgba(239,68,68,0.15)",
        color: online ? "#4ade80" : "#fca5a5",
        border: `1px solid ${online ? "rgba(34,197,94,0.3)" : "rgba(239,68,68,0.3)"}`,
        boxShadow: "0 2px 8px rgba(0,0,0,0.4)",
      }}
    >
      <span
        style={{
          width: 8,
          height: 8,
          borderRadius: "50%",
          background: online ? "#22c55e" : "#ef4444",
          display: "inline-block",
        }}
      />
      {!online && "MODO OFFLINE"}
      {online && pendientes > 0 && "Sincronizando..."}
      {pendientes > 0 && (
        <span style={{ fontWeight: 400 }}>
          ({pendientes} pendiente{pendientes !== 1 ? "s" : ""})
        </span>
      )}
    </div>
  );
}
