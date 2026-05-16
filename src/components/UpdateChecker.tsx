/**
 * v2.5.8 — UpdateChecker rediseñado.
 *
 * Cambios clave:
 * - Verifica al inicio + cada 60 minutos (antes solo al inicio).
 * - NO descarga automáticamente. Pregunta al usuario primero (evita perder trabajo
 *   en medio de una venta).
 * - Banner llamativo con botones [Actualizar ahora] [Más tarde].
 * - Si el usuario pulsa "Más tarde", oculta el banner pero la verificación volverá
 *   a disparar el aviso en el siguiente check (60 min) o al reiniciar la app.
 * - Disparable manualmente vía evento `clouget:verificar-update`.
 */
import { useState, useEffect, useCallback, useRef } from "react";
import { check, type Update } from "@tauri-apps/plugin-updater";
import { relaunch } from "@tauri-apps/plugin-process";
import { invoke } from "@tauri-apps/api/core";
import { obtenerConfig } from "../services/api";

type Estado = "idle" | "disponible" | "descargando" | "instalado" | "error" | "al-dia";

export default function UpdateChecker() {
  const [estado, setEstado] = useState<Estado>("idle");
  const [update, setUpdate] = useState<Update | null>(null);
  const [progreso, setProgreso] = useState(0);
  const totalBytesRef = useRef(0);
  const [error, setError] = useState("");
  const [cerrado, setCerrado] = useState(false);
  // v2.5.8: indica si el usuario pidió explícitamente el chequeo (para mostrar
  // mensaje "estás en la última versión" cuando no hay update)
  const manualRef = useRef(false);

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
      // Leer canal de actualizaciones
      let canal = "stable";
      try {
        const cfg = await obtenerConfig();
        canal = cfg.update_canal === "beta" ? "beta" : "stable";
      } catch { /* fallback stable */ }

      if (canal === "beta") {
        // Canal BETA: el comando Rust hace check + descarga + install todo junto.
        // Por compatibilidad lo mantenemos así por ahora (no pregunta al usuario).
        // Si el cliente está en beta, asume que sabe lo que hace.
        try {
          console.log("[Updater] Canal BETA: verificando via comando Rust...");
          const tieneUpdate = await invoke<boolean>("verificar_update_canal_disponible", { canal: "beta" }).catch(() => null);
          if (tieneUpdate === null) {
            // Fallback al comportamiento viejo si el comando nuevo no existe
            setEstado("descargando");
            setProgreso(0);
            const nuevaVersion = await invoke<string | null>("verificar_update_canal", { canal: "beta" });
            if (nuevaVersion) {
              setUpdate({ version: nuevaVersion } as any);
              setEstado("instalado");
              setTimeout(async () => { await relaunch(); }, 1500);
            } else {
              if (manualRef.current) {
                setEstado("al-dia");
                setTimeout(() => { setEstado("idle"); manualRef.current = false; }, 4000);
              } else {
                setEstado("idle");
              }
            }
          }
        } catch (e) {
          console.warn("[Updater] Error canal beta:", e);
          if (manualRef.current) {
            setError("No se pudo verificar (canal beta): " + String(e));
            setEstado("error");
            manualRef.current = false;
          } else {
            setEstado("idle");
          }
        }
        return;
      }

      // Canal STABLE
      console.log("[Updater] Canal: stable, verificando via endpoint configurado...");
      const resultado = await check();
      console.log("[Updater] Resultado:", resultado);
      if (resultado) {
        setUpdate(resultado);
        setEstado("disponible");
        setCerrado(false); // re-abrir banner si estaba cerrado
        manualRef.current = false;
      } else {
        console.log("[Updater] No hay actualizaciones disponibles");
        if (manualRef.current) {
          setEstado("al-dia");
          // Auto-cerrar en 4 segundos
          setTimeout(() => { setEstado("idle"); manualRef.current = false; }, 4000);
        }
      }
    } catch (e) {
      console.error("[Updater] Error al verificar actualizacion:", e);
      if (manualRef.current) {
        setError("No se pudo verificar actualizaciones: " + String(e));
        setEstado("error");
        manualRef.current = false;
      }
    }
  }, []);

  useEffect(() => {
    // v2.5.8: verificación INICIAL 5s después de montar + RECURRENTE cada 60 min.
    const timerInicial = setTimeout(verificarActualizacion, 5000);
    const intervalRecurrente = setInterval(verificarActualizacion, 60 * 60 * 1000);
    // Disparable manualmente desde otro lado (ej. botón en Configuración)
    const handlerManual = () => {
      manualRef.current = true;
      verificarActualizacion();
    };
    window.addEventListener("clouget:verificar-update", handlerManual);
    return () => {
      clearTimeout(timerInicial);
      clearInterval(intervalRecurrente);
      window.removeEventListener("clouget:verificar-update", handlerManual);
    };
  }, [verificarActualizacion]);

  if (estado === "idle" || cerrado) return null;

  // ─── Banner "Nueva versión disponible" con confirmación ──────────────
  if (estado === "disponible") {
    return (
      <div style={{
        padding: "10px 16px",
        background: "linear-gradient(90deg, rgba(59,130,246,0.2) 0%, rgba(34,197,94,0.18) 100%)",
        borderBottom: "2px solid rgba(59,130,246,0.5)",
        display: "flex",
        alignItems: "center",
        gap: 12,
        fontSize: 13,
        color: "var(--color-primary)",
        flexWrap: "wrap",
      }}>
        <span style={{ fontSize: 20 }}>🎉</span>
        <span style={{ flex: 1, minWidth: 220 }}>
          <strong>Nueva versión {update?.version} disponible.</strong>{" "}
          <span style={{ opacity: 0.85 }}>Aplica el cambio cuando termines lo que estás haciendo — se cerrará y reiniciará la app.</span>
        </span>
        <button
          onClick={() => update && descargarEInstalar(update)}
          style={{
            background: "var(--color-primary)",
            color: "#fff",
            border: "none",
            borderRadius: 6,
            padding: "8px 16px",
            fontSize: 13,
            fontWeight: 700,
            cursor: "pointer",
          }}
        >
          ⬆ Actualizar ahora
        </button>
        <button
          onClick={() => setCerrado(true)}
          style={{
            background: "transparent",
            color: "var(--color-text)",
            border: "1px solid var(--color-border)",
            borderRadius: 6,
            padding: "8px 16px",
            fontSize: 13,
            cursor: "pointer",
          }}
          title="Te recordaremos en la próxima verificación (60 min) o al reiniciar la app"
        >
          Más tarde
        </button>
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
          <span>⬆ Descargando v{update?.version}... {progreso}%</span>
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
        <span>✓ Actualización instalada. Reiniciando...</span>
      </div>
    );
  }

  // v2.5.8: nuevo estado "al-dia" (cuando manual check y no hay update)
  if (estado === "al-dia") {
    return (
      <div style={{
        padding: "8px 16px",
        background: "rgba(34,197,94,0.12)",
        borderBottom: "2px solid rgba(34,197,94,0.3)",
        display: "flex",
        alignItems: "center",
        gap: 10,
        fontSize: 13,
        color: "var(--color-success)",
      }}>
        <span>✓ Estás en la última versión.</span>
        <button
          onClick={() => { setEstado("idle"); manualRef.current = false; }}
          style={{
            background: "transparent",
            border: "none",
            cursor: "pointer",
            color: "inherit",
            fontSize: 16,
            opacity: 0.6,
            marginLeft: "auto",
            padding: "0 4px",
            lineHeight: 1,
          }}
          title="Cerrar"
        >×</button>
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
        <span style={{ flex: 1 }}>⚠ Error al actualizar: {error}</span>
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
        >x</button>
      </div>
    );
  }

  return null;
}
