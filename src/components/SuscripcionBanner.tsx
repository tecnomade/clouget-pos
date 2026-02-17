import { useState, useEffect, useCallback } from "react";
import { consultarEstadoSri, obtenerConfig } from "../services/api";

const UMBRALES = [10, 5, 3, 2, 1];

function calcularDiasRestantes(fechaHasta: string): number {
  if (!fechaHasta) return 999;
  const hoy = new Date();
  hoy.setHours(0, 0, 0, 0);
  const hasta = new Date(fechaHasta + "T00:00:00");
  const diff = hasta.getTime() - hoy.getTime();
  return Math.ceil(diff / (1000 * 60 * 60 * 24));
}

function obtenerMensaje(tipo: string, restante: number): string | null {
  if (tipo === "trial") {
    if (restante <= 0) return "Prueba gratuita agotada. Adquiera una suscripcion para seguir facturando.";
    if (UMBRALES.includes(restante) || restante <= 3) {
      return `Le quedan ${restante} factura${restante === 1 ? "" : "s"} gratis. Considere adquirir un plan.`;
    }
    return null;
  }
  if (tipo === "paquete") {
    if (restante <= 0) return "Se agotaron los documentos de su paquete. Adquiera mas para seguir facturando.";
    if (UMBRALES.includes(restante) || restante <= 3) {
      return `Le quedan ${restante} documento${restante === 1 ? "" : "s"} en su paquete.`;
    }
    return null;
  }
  if (tipo === "tiempo") {
    if (restante <= 0) return "Su suscripcion ha expirado. Renueve para seguir facturando.";
    if (UMBRALES.includes(restante) || restante <= 3) {
      return `Su suscripcion vence en ${restante} dia${restante === 1 ? "" : "s"}.`;
    }
    return null;
  }
  return null;
}

export default function SuscripcionBanner() {
  const [mensaje, setMensaje] = useState<string | null>(null);
  const [cerrado, setCerrado] = useState(false);
  const [urgente, setUrgente] = useState(false);
  const [visible, setVisible] = useState(false);

  const verificar = useCallback(async () => {
    try {
      const cfg = await obtenerConfig();
      if (!cfg.regimen || cfg.regimen === "RIMPE_POPULAR") return;

      const estado = await consultarEstadoSri();

      // Si tiene lifetime, no alertar
      if (estado.suscripcion_es_lifetime) return;

      let msg: string | null = null;
      let esUrgente = false;
      let restante = 999;

      if (estado.suscripcion_autorizada) {
        if (estado.suscripcion_plan === "paquete" && estado.suscripcion_docs_restantes != null) {
          restante = estado.suscripcion_docs_restantes;
          msg = obtenerMensaje("paquete", restante);
        } else if (estado.suscripcion_hasta) {
          restante = calcularDiasRestantes(estado.suscripcion_hasta);
          msg = obtenerMensaje("tiempo", restante);
        }
      } else {
        restante = estado.facturas_gratis - estado.facturas_usadas;
        msg = obtenerMensaje("trial", restante);
      }

      esUrgente = restante <= 3;

      if (msg) {
        const dismissKey = `banner-sri-dismissed-${restante}`;
        if (sessionStorage.getItem(dismissKey)) return;
        setMensaje(msg);
        setUrgente(esUrgente);
        setVisible(true);
        setCerrado(false);
      }
    } catch {
      // silenciar errores
    }
  }, []);

  useEffect(() => {
    verificar();

    const handler = () => verificar();
    window.addEventListener("sri-factura-emitida", handler);
    return () => window.removeEventListener("sri-factura-emitida", handler);
  }, [verificar]);

  const cerrarBanner = () => {
    setCerrado(true);
    setVisible(false);
    // Guardar que se cerro para este nivel
    if (mensaje) {
      const match = mensaje.match(/(\d+)/);
      if (match) {
        sessionStorage.setItem(`banner-sri-dismissed-${match[1]}`, "1");
      }
    }
  };

  if (!visible || cerrado || !mensaje) return null;

  return (
    <div style={{
      padding: "8px 16px",
      background: urgente ? "#fef2f2" : "#fffbeb",
      borderBottom: `2px solid ${urgente ? "#fca5a5" : "#fcd34d"}`,
      display: "flex",
      alignItems: "center",
      gap: 10,
      fontSize: 13,
      color: urgente ? "#991b1b" : "#92400e",
      animation: "banner-in 0.3s ease-out",
    }}>
      <span style={{ fontSize: 16 }}>{urgente ? "!" : "i"}</span>
      <span style={{ flex: 1 }}>{mensaje}</span>
      <button
        onClick={cerrarBanner}
        style={{
          background: "none",
          border: "none",
          cursor: "pointer",
          fontSize: 16,
          color: urgente ? "#991b1b" : "#92400e",
          padding: "0 4px",
          lineHeight: 1,
          opacity: 0.6,
        }}
        title="Cerrar"
      >
        x
      </button>
    </div>
  );
}
