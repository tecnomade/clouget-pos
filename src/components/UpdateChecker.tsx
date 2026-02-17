import { useState, useEffect, useCallback, useRef } from "react";
import { check, type Update } from "@tauri-apps/plugin-updater";
import { relaunch } from "@tauri-apps/plugin-process";

type Estado = "idle" | "disponible" | "descargando" | "instalado" | "error";

export default function UpdateChecker() {
  const [estado, setEstado] = useState<Estado>("idle");
  const [update, setUpdate] = useState<Update | null>(null);
  const [progreso, setProgreso] = useState(0);
  const totalBytesRef = useRef(0);
  const [error, setError] = useState("");
  const [cerrado, setCerrado] = useState(false);

  const verificarActualizacion = useCallback(async () => {
    try {
      const resultado = await check();
      if (resultado) {
        setUpdate(resultado);
        setEstado("disponible");
      }
    } catch (e) {
      // Fallo silencioso - problemas de red no deben molestar al usuario
      console.warn("Error al verificar actualizacion:", e);
    }
  }, []);

  useEffect(() => {
    // Verificar 5 segundos despues de montar para no bloquear el inicio
    const timer = setTimeout(verificarActualizacion, 5000);
    return () => clearTimeout(timer);
  }, [verificarActualizacion]);

  const descargarEInstalar = async () => {
    if (!update) return;
    setEstado("descargando");
    setProgreso(0);

    try {
      let descargado = 0;
      await update.downloadAndInstall((event) => {
        switch (event.event) {
          case "Started":
            if (event.data.contentLength) {
              totalBytesRef.current = event.data.contentLength;
            }
            break;
          case "Progress":
            descargado += event.data.chunkLength;
            if (totalBytesRef.current > 0) {
              setProgreso(Math.round((descargado / totalBytesRef.current) * 100));
            }
            break;
          case "Finished":
            setProgreso(100);
            break;
        }
      });
      setEstado("instalado");
      // Esperar un momento y reiniciar
      setTimeout(async () => {
        await relaunch();
      }, 1500);
    } catch (e) {
      setEstado("error");
      setError(String(e));
    }
  };

  if (estado === "idle" || cerrado) return null;

  if (estado === "disponible") {
    return (
      <div style={{
        padding: "8px 16px",
        background: "#eff6ff",
        borderBottom: "2px solid #93c5fd",
        display: "flex",
        alignItems: "center",
        gap: 10,
        fontSize: 13,
        color: "#1e40af",
      }}>
        <span style={{ fontSize: 16 }}>&#8593;</span>
        <span style={{ flex: 1 }}>
          Nueva version <strong>{update?.version}</strong> disponible.
        </span>
        <button
          onClick={descargarEInstalar}
          style={{
            background: "#2563eb",
            color: "white",
            border: "none",
            borderRadius: 4,
            padding: "4px 12px",
            cursor: "pointer",
            fontSize: 12,
            fontWeight: 600,
          }}
        >
          Actualizar ahora
        </button>
        <button
          onClick={() => setCerrado(true)}
          style={{
            background: "none",
            border: "none",
            cursor: "pointer",
            fontSize: 16,
            color: "#1e40af",
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

  if (estado === "descargando") {
    return (
      <div style={{
        padding: "8px 16px",
        background: "#eff6ff",
        borderBottom: "2px solid #93c5fd",
        fontSize: 13,
        color: "#1e40af",
      }}>
        <div style={{ display: "flex", alignItems: "center", gap: 10 }}>
          <span>Descargando actualizacion... {progreso}%</span>
        </div>
        <div style={{
          marginTop: 4,
          height: 4,
          background: "#dbeafe",
          borderRadius: 2,
          overflow: "hidden",
        }}>
          <div style={{
            height: "100%",
            width: `${progreso}%`,
            background: "#2563eb",
            transition: "width 0.3s",
            borderRadius: 2,
          }} />
        </div>
      </div>
    );
  }

  if (estado === "instalado") {
    return (
      <div style={{
        padding: "8px 16px",
        background: "#f0fdf4",
        borderBottom: "2px solid #86efac",
        display: "flex",
        alignItems: "center",
        gap: 10,
        fontSize: 13,
        color: "#166534",
      }}>
        <span>Actualizacion instalada. Reiniciando...</span>
      </div>
    );
  }

  if (estado === "error") {
    return (
      <div style={{
        padding: "8px 16px",
        background: "#fef2f2",
        borderBottom: "2px solid #fca5a5",
        display: "flex",
        alignItems: "center",
        gap: 10,
        fontSize: 13,
        color: "#991b1b",
      }}>
        <span style={{ flex: 1 }}>Error al actualizar: {error}</span>
        <button
          onClick={() => setCerrado(true)}
          style={{
            background: "none",
            border: "none",
            cursor: "pointer",
            fontSize: 16,
            color: "#991b1b",
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

  return null;
}
