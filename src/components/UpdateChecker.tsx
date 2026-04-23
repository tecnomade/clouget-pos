import { useState, useEffect, useCallback, useRef } from "react";
import { check, type Update } from "@tauri-apps/plugin-updater";
import { relaunch } from "@tauri-apps/plugin-process";
import { invoke } from "@tauri-apps/api/core";
import { obtenerConfig } from "../services/api";

type Estado = "idle" | "disponible" | "descargando" | "instalado" | "error";

export default function UpdateChecker() {
  const [estado, setEstado] = useState<Estado>("idle");
  const [update, setUpdate] = useState<Update | null>(null);
  const [progreso, setProgreso] = useState(0);
  const totalBytesRef = useRef(0);
  const [error, setError] = useState("");
  const [cerrado, setCerrado] = useState(false);

  const descargarEInstalar = useCallback(async (upd: Update) => {
    setEstado("descargando");
    setProgreso(0);

    try {
      let descargado = 0;
      await upd.downloadAndInstall((event) => {
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
  }, []);

  const verificarActualizacion = useCallback(async () => {
    try {
      // Leer canal de actualizaciones de config
      let canal = "stable";
      try {
        const cfg = await obtenerConfig();
        canal = cfg.update_canal === "beta" ? "beta" : "stable";
      } catch { /* fallback a stable */ }

      if (canal === "beta") {
        // Canal BETA: usa el comando Rust 'verificar_update_canal' que construye un
        // updater con endpoint dinamico (Supabase con ?canal=beta) y descarga+instala
        try {
          console.log("[Updater] Canal BETA: verificando via comando Rust...");
          setEstado("descargando");
          setProgreso(0);
          const nuevaVersion = await invoke<string | null>("verificar_update_canal", { canal: "beta" });
          if (nuevaVersion) {
            console.log(`[Updater] Beta v${nuevaVersion} instalada, reiniciando...`);
            setUpdate({ version: nuevaVersion } as any);
            setEstado("instalado");
            setTimeout(async () => { await relaunch(); }, 1500);
          } else {
            console.log("[Updater] Canal BETA: ya en ultima version");
            setEstado("idle");
          }
        } catch (e) {
          console.warn("[Updater] Error canal beta:", e);
          setEstado("idle");
        }
        return;
      }

      // Canal STABLE: usa el plugin estandar con el endpoint configurado en tauri.conf.json
      // (apunta primero a Supabase edge function con canal=stable, fallback a GitHub)
      console.log("[Updater] Canal: stable, verificando via endpoint configurado...");
      const resultado = await check();
      console.log("[Updater] Resultado:", resultado);
      if (resultado) {
        setUpdate(resultado);
        setEstado("disponible");
        descargarEInstalar(resultado);
      } else {
        console.log("[Updater] No hay actualizaciones disponibles");
      }
    } catch (e) {
      console.error("[Updater] Error al verificar actualizacion:", e);
    }
  }, [descargarEInstalar]);

  useEffect(() => {
    // Verificar 5 segundos despues de montar para no bloquear el inicio
    const timer = setTimeout(verificarActualizacion, 5000);
    return () => clearTimeout(timer);
  }, [verificarActualizacion]);

  if (estado === "idle" || cerrado) return null;

  if (estado === "disponible") {
    return (
      <div style={{
        padding: "8px 16px",
        background: "rgba(59,130,246,0.15)",
        borderBottom: "2px solid rgba(59,130,246,0.3)",
        display: "flex",
        alignItems: "center",
        gap: 10,
        fontSize: 13,
        color: "var(--color-primary)",
      }}>
        <span style={{ fontSize: 16 }}>&#8593;</span>
        <span style={{ flex: 1 }}>
          Nueva version <strong>{update?.version}</strong> disponible. Descargando...
        </span>
      </div>
    );
  }

  if (estado === "descargando") {
    return (
      <div style={{
        padding: "8px 16px",
        background: "rgba(59,130,246,0.15)",
        borderBottom: "2px solid rgba(59,130,246,0.3)",
        fontSize: 13,
        color: "var(--color-primary)",
      }}>
        <div style={{ display: "flex", alignItems: "center", gap: 10 }}>
          <span>Descargando v{update?.version}... {progreso}%</span>
        </div>
        <div style={{
          marginTop: 4,
          height: 4,
          background: "rgba(59,130,246,0.2)",
          borderRadius: 2,
          overflow: "hidden",
        }}>
          <div style={{
            height: "100%",
            width: `${progreso}%`,
            background: "var(--color-primary)",
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
        background: "rgba(34,197,94,0.15)",
        borderBottom: "2px solid rgba(34,197,94,0.3)",
        display: "flex",
        alignItems: "center",
        gap: 10,
        fontSize: 13,
        color: "var(--color-success)",
      }}>
        <span>Actualizacion instalada. Reiniciando...</span>
      </div>
    );
  }

  if (estado === "error") {
    return (
      <div style={{
        padding: "8px 16px",
        background: "rgba(239,68,68,0.15)",
        borderBottom: "2px solid rgba(239,68,68,0.3)",
        display: "flex",
        alignItems: "center",
        gap: 10,
        fontSize: 13,
        color: "var(--color-danger)",
      }}>
        <span style={{ flex: 1 }}>Error al actualizar: {error}</span>
        <button
          onClick={() => setCerrado(true)}
          style={{
            background: "none",
            border: "none",
            cursor: "pointer",
            fontSize: 16,
            color: "var(--color-danger)",
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
