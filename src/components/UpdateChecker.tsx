/**
 * v2.5.9 — UpdateChecker con UX diferenciada startup vs runtime.
 *
 * Comportamiento:
 *
 * 1. AL INICIAR LA APP (startup, 3s después de mount):
 *    - Muestra banner "🔄 Buscando actualización..."
 *    - Si encuentra update → instala AUTOMÁTICAMENTE (sin preguntar).
 *      El cliente recién está abriendo, no está en medio de nada.
 *    - Si no hay update → oculta el banner silenciosamente.
 *
 * 2. CHECK PERIÓDICO (cada 60 min, app ya abierta):
 *    - Si encuentra update → muestra banner con [Actualizar ahora] / [Más tarde].
 *      No instala sin confirmación porque podría perder trabajo en curso.
 *    - Si no hay update → no muestra nada.
 *
 * 3. CHECK MANUAL (botón en Configuración):
 *    - Si encuentra → banner con confirmación (igual que runtime).
 *    - Si no encuentra → banner verde "Estás en la última versión" (4s).
 *
 * 4. DETALLES DEL CAMBIO:
 *    - El banner muestra las notas de la release (campo `body` del update),
 *      o un fallback genérico si no vienen.
 */
import { useState, useEffect, useCallback, useRef } from "react";
import { check, type Update } from "@tauri-apps/plugin-updater";
import { relaunch } from "@tauri-apps/plugin-process";
import { invoke } from "@tauri-apps/api/core";
import { obtenerConfig } from "../services/api";

type Estado = "idle" | "buscando" | "disponible" | "descargando" | "instalado" | "error" | "al-dia";

export default function UpdateChecker() {
  const [estado, setEstado] = useState<Estado>("idle");
  const [update, setUpdate] = useState<Update | null>(null);
  const [progreso, setProgreso] = useState(0);
  const totalBytesRef = useRef(0);
  const [error, setError] = useState("");
  const [cerrado, setCerrado] = useState(false);
  // v2.5.8: indica si el usuario pidió explícitamente el chequeo
  const manualRef = useRef(false);
  // v2.5.9: indica si es el check inicial al abrir la app (auto-instala)
  const startupRef = useRef(true);

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

  const verificarActualizacion = useCallback(async (esStartup = false) => {
    try {
      // Si es startup, mostrar "Buscando..." al usuario
      if (esStartup) {
        setEstado("buscando");
        setCerrado(false);
      }

      // Leer canal
      let canal = "stable";
      try {
        const cfg = await obtenerConfig();
        canal = cfg.update_canal === "beta" ? "beta" : "stable";
      } catch { /* fallback stable */ }

      if (canal === "beta") {
        try {
          console.log("[Updater] Canal BETA: verificando via comando Rust...");
          if (esStartup || !manualRef.current) {
            setEstado("descargando");
            setProgreso(0);
          }
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
      console.log("[Updater] Canal: stable, esStartup:", esStartup);
      const resultado = await check();
      console.log("[Updater] Resultado:", resultado);
      if (resultado) {
        setUpdate(resultado);
        if (esStartup) {
          // Startup: instalar automáticamente (cliente acaba de abrir, no pierde nada)
          console.log("[Updater] Startup: instalando automáticamente...");
          descargarEInstalar(resultado);
        } else {
          // Runtime: mostrar banner y dejar que el usuario decida
          setEstado("disponible");
          setCerrado(false);
          manualRef.current = false;
        }
      } else {
        console.log("[Updater] No hay actualizaciones disponibles");
        if (manualRef.current) {
          setEstado("al-dia");
          setTimeout(() => { setEstado("idle"); manualRef.current = false; }, 4000);
        } else if (esStartup) {
          // Startup sin update: ocultar el "Buscando..."
          setEstado("idle");
        }
      }
    } catch (e) {
      console.error("[Updater] Error al verificar actualizacion:", e);
      if (manualRef.current) {
        setError("No se pudo verificar actualizaciones: " + String(e));
        setEstado("error");
        manualRef.current = false;
      } else if (esStartup) {
        setEstado("idle"); // no molestar con error de red en arranque
      }
    }
  }, [descargarEInstalar]);

  useEffect(() => {
    // CHECK INICIAL (startup): 3s después del mount → auto-instala si hay update
    const timerInicial = setTimeout(() => {
      startupRef.current = false;
      verificarActualizacion(true);
    }, 3000);
    // CHECK RECURRENTE: cada 60 min → pregunta antes de instalar
    const intervalRecurrente = setInterval(() => {
      verificarActualizacion(false);
    }, 60 * 60 * 1000);
    // CHECK MANUAL: vía evento global desde Configuración
    const handlerManual = () => {
      manualRef.current = true;
      verificarActualizacion(false);
    };
    window.addEventListener("clouget:verificar-update", handlerManual);
    return () => {
      clearTimeout(timerInicial);
      clearInterval(intervalRecurrente);
      window.removeEventListener("clouget:verificar-update", handlerManual);
    };
  }, [verificarActualizacion]);

  if (estado === "idle" || cerrado) return null;

  // v2.5.9: "Buscando..." al iniciar la app (no bloqueante, deja usar la app igual)
  if (estado === "buscando") {
    return (
      <div style={{
        padding: "6px 16px",
        background: "rgba(59,130,246,0.1)",
        borderBottom: "1px solid rgba(59,130,246,0.25)",
        display: "flex",
        alignItems: "center",
        gap: 8,
        fontSize: 12,
        color: "var(--color-primary)",
      }}>
        <span style={{ display: "inline-block", animation: "spin 1s linear infinite" }}>🔄</span>
        <span>Buscando actualización...</span>
        <style>{`@keyframes spin { from { transform: rotate(0deg); } to { transform: rotate(360deg); } }`}</style>
      </div>
    );
  }

  // ─── Banner "Nueva versión disponible" con confirmación ──────────────
  if (estado === "disponible") {
    const notas = (update as any)?.body || `Esta nueva versión incluye correcciones y mejoras. Revisá el detalle completo en GitHub.`;
    return (
      <div style={{
        padding: "10px 16px",
        background: "linear-gradient(90deg, rgba(59,130,246,0.2) 0%, rgba(34,197,94,0.18) 100%)",
        borderBottom: "2px solid rgba(59,130,246,0.5)",
        fontSize: 13,
        color: "var(--color-primary)",
      }}>
        <div style={{ display: "flex", alignItems: "flex-start", gap: 12, flexWrap: "wrap" }}>
          <span style={{ fontSize: 20 }}>🎉</span>
          <div style={{ flex: 1, minWidth: 220 }}>
            <div>
              <strong style={{ fontSize: 14 }}>Nueva versión {update?.version} disponible</strong>
            </div>
            <div style={{ fontSize: 11, color: "var(--color-text-secondary)", marginTop: 2 }}>
              Aplica el cambio cuando termines lo que estás haciendo — se cerrará y reiniciará la app.
            </div>
            {/* v2.5.9: detalle de la actualización (notas de la release) */}
            <details style={{ marginTop: 6, fontSize: 11, color: "var(--color-text)" }}>
              <summary style={{ cursor: "pointer", color: "var(--color-primary)", fontWeight: 600 }}>
                Ver detalles de la actualización
              </summary>
              <div style={{
                marginTop: 6, padding: 8, maxHeight: 200, overflowY: "auto",
                background: "rgba(0,0,0,0.04)", borderRadius: 4, whiteSpace: "pre-wrap",
                lineHeight: 1.4, fontFamily: "inherit",
              }}>{notas}</div>
            </details>
          </div>
          <div style={{ display: "flex", gap: 8, flexShrink: 0 }}>
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
        </div>
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
