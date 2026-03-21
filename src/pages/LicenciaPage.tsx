import { useState, useEffect } from "react";
import { verificarLicencia, obtenerMachineId, activarDemo } from "../services/api";
import type { LicenciaInfo } from "../types";

interface Props {
  onActivada: (licencia: LicenciaInfo) => void;
}

export default function LicenciaPage({ onActivada }: Props) {
  const [codigo, setCodigo] = useState("");
  const [error, setError] = useState("");
  const [verificando, setVerificando] = useState(false);
  const [activandoDemo, setActivandoDemo] = useState(false);
  const [machineId, setMachineId] = useState("");
  const [copiado, setCopiado] = useState(false);

  useEffect(() => {
    obtenerMachineId()
      .then(setMachineId)
      .catch(() => setMachineId("ERROR"));
  }, []);

  const handleActivar = async () => {
    const trimmed = codigo.trim();
    if (!trimmed) {
      setError("Ingrese el código de activación");
      return;
    }

    setVerificando(true);
    setError("");

    try {
      const licencia = await verificarLicencia(trimmed);
      onActivada(licencia);
    } catch (err) {
      setError(String(err));
    } finally {
      setVerificando(false);
    }
  };

  const handleDemo = async () => {
    setActivandoDemo(true);
    setError("");

    try {
      const licencia = await activarDemo();
      onActivada(licencia);
    } catch (err) {
      setError(String(err));
    } finally {
      setActivandoDemo(false);
    }
  };

  const handleCopiar = async () => {
    try {
      await navigator.clipboard.writeText(machineId);
      setCopiado(true);
      setTimeout(() => setCopiado(false), 2000);
    } catch {
      const el = document.createElement("textarea");
      el.value = machineId;
      document.body.appendChild(el);
      el.select();
      document.execCommand("copy");
      document.body.removeChild(el);
      setCopiado(true);
      setTimeout(() => setCopiado(false), 2000);
    }
  };

  return (
    <div
      style={{
        minHeight: "100vh",
        display: "flex",
        alignItems: "center",
        justifyContent: "center",
        background: "linear-gradient(135deg, #1e293b 0%, #0f172a 100%)",
        padding: 24,
      }}
    >
      <div
        style={{
          background: "white",
          borderRadius: 16,
          padding: 40,
          maxWidth: 480,
          width: "100%",
          boxShadow: "0 25px 50px rgba(0,0,0,0.25)",
        }}
      >
        <div style={{ textAlign: "center", marginBottom: 32 }}>
          <h1 style={{ fontSize: 28, fontWeight: 800, color: "#1e293b", margin: 0 }}>
            CLOUGET
          </h1>
          <p style={{ color: "#64748b", margin: "4px 0 0 0", fontSize: 14 }}>
            Punto de Venta
          </p>
        </div>

        {/* Código de máquina */}
        <div
          style={{
            background: "#f0f9ff",
            border: "2px solid #bae6fd",
            borderRadius: 12,
            padding: 20,
            marginBottom: 20,
            textAlign: "center",
          }}
        >
          <p style={{ margin: "0 0 8px 0", fontSize: 12, color: "#0369a1", fontWeight: 600, textTransform: "uppercase", letterSpacing: 1 }}>
            Codigo de Maquina
          </p>
          <div style={{ display: "flex", alignItems: "center", justifyContent: "center", gap: 12 }}>
            <span
              style={{
                fontSize: 28,
                fontWeight: 800,
                fontFamily: "monospace",
                color: "#0c4a6e",
                letterSpacing: 4,
              }}
            >
              {machineId || "..."}
            </span>
            <button
              onClick={handleCopiar}
              style={{
                padding: "6px 12px",
                border: "1px solid #7dd3fc",
                borderRadius: 6,
                background: copiado ? "#dcfce7" : "white",
                color: copiado ? "#166534" : "#0369a1",
                fontSize: 12,
                fontWeight: 600,
                cursor: "pointer",
                transition: "all 0.2s",
              }}
            >
              {copiado ? "Copiado!" : "Copiar"}
            </button>
          </div>
          <p style={{ margin: "10px 0 0 0", fontSize: 12, color: "#64748b" }}>
            Envie este codigo junto con su pago para recibir su licencia
          </p>
        </div>

        {/* Activar licencia */}
        <div
          style={{
            background: "#f8fafc",
            border: "1px solid #e2e8f0",
            borderRadius: 12,
            padding: 24,
            marginBottom: 16,
          }}
        >
          <h3 style={{ margin: "0 0 8px 0", fontSize: 16, color: "#334155" }}>
            Activar Licencia
          </h3>
          <p style={{ margin: "0 0 16px 0", fontSize: 13, color: "#64748b" }}>
            Ingrese el codigo de activacion que recibio despues de su compra.
          </p>

          <input
            value={codigo}
            onChange={(e) => {
              setCodigo(e.target.value.toUpperCase());
              setError("");
            }}
            placeholder="Ej: CLG-A7F3-B21E-X9K2"
            maxLength={20}
            style={{
              width: "100%",
              padding: 14,
              border: error ? "2px solid #ef4444" : "1px solid #cbd5e1",
              borderRadius: 8,
              fontSize: 18,
              fontFamily: "monospace",
              fontWeight: 700,
              textAlign: "center",
              letterSpacing: 3,
              outline: "none",
              boxSizing: "border-box",
              textTransform: "uppercase",
            }}
            onKeyDown={(e) => {
              if (e.key === "Enter") handleActivar();
            }}
            autoFocus
          />

          {error && (
            <p style={{ color: "#ef4444", fontSize: 13, margin: "8px 0 0 0" }}>
              {error}
            </p>
          )}

          <button
            onClick={handleActivar}
            disabled={verificando || activandoDemo}
            style={{
              width: "100%",
              marginTop: 16,
              padding: "12px 24px",
              background: verificando ? "#94a3b8" : "#2563eb",
              color: "white",
              border: "none",
              borderRadius: 8,
              fontSize: 15,
              fontWeight: 600,
              cursor: verificando ? "not-allowed" : "pointer",
            }}
          >
            {verificando ? "Verificando..." : "Activar Licencia"}
          </button>

          {verificando && (
            <p style={{ textAlign: "center", fontSize: 12, color: "#64748b", margin: "8px 0 0 0" }}>
              Conectando al servidor...
            </p>
          )}
        </div>

        {/* Separador */}
        <div style={{ display: "flex", alignItems: "center", gap: 12, margin: "16px 0" }}>
          <div style={{ flex: 1, height: 1, background: "#e2e8f0" }} />
          <span style={{ fontSize: 13, color: "#94a3b8", fontWeight: 500 }}>o</span>
          <div style={{ flex: 1, height: 1, background: "#e2e8f0" }} />
        </div>

        {/* Demo */}
        <div
          style={{
            background: "#f0fdf4",
            border: "2px solid #bbf7d0",
            borderRadius: 12,
            padding: 20,
            marginBottom: 24,
            textAlign: "center",
          }}
        >
          <h3 style={{ margin: "0 0 6px 0", fontSize: 15, color: "#166534" }}>
            Probar Demo
          </h3>
          <p style={{ margin: "0 0 14px 0", fontSize: 13, color: "#4ade80", lineHeight: 1.4 }}>
            Explore todas las funcionalidades con datos de ejemplo.
            Sin compromiso, sin limite de tiempo.
          </p>
          <button
            onClick={handleDemo}
            disabled={activandoDemo || verificando}
            style={{
              width: "100%",
              padding: "12px 24px",
              background: activandoDemo ? "#94a3b8" : "#16a34a",
              color: "white",
              border: "none",
              borderRadius: 8,
              fontSize: 15,
              fontWeight: 600,
              cursor: activandoDemo ? "not-allowed" : "pointer",
              transition: "background 0.2s",
            }}
          >
            {activandoDemo ? "Preparando demo..." : "Probar Demo Gratis"}
          </button>
        </div>

        <div
          style={{
            textAlign: "center",
            padding: "16px 0 0 0",
            borderTop: "1px solid #e2e8f0",
          }}
        >
          <p style={{ fontSize: 13, color: "#64748b", margin: "0 0 8px 0" }}>
            No tiene licencia? Contactenos para adquirir una:
          </p>
          <p style={{ fontSize: 14, margin: 0 }}>
            <span style={{ color: "#22c55e", fontWeight: 600 }}>WhatsApp:</span>{" "}
            <span style={{ color: "#334155" }}>+593 98 128 5671</span>
          </p>
        </div>
      </div>
    </div>
  );
}
