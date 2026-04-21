import { createContext, useContext, useState, useEffect, useRef, useCallback } from "react";
import { cerrarSesion as apiCerrarSesion, obtenerConfig } from "../services/api";
import type { SesionActiva } from "../types";

interface SesionContextType {
  sesion: SesionActiva | null;
  setSesion: (s: SesionActiva | null) => void;
  cerrarSesion: () => Promise<void>;
  esAdmin: boolean;
  tienePermiso: (permiso: string) => boolean;
}

const SesionContext = createContext<SesionContextType | null>(null);

export function SesionProvider({ children }: { children: React.ReactNode }) {
  const [sesion, setSesion] = useState<SesionActiva | null>(null);
  const lastActivity = useRef<number>(Date.now());
  const timeoutMinutos = useRef<number>(15);

  const cerrarSesion = useCallback(async () => {
    try {
      await apiCerrarSesion();
    } catch {
      // Ignorar error si ya no hay sesión
    }
    setSesion(null);
  }, []);

  // Cargar timeout desde config cuando hay sesion
  useEffect(() => {
    if (!sesion) return;
    obtenerConfig().then((cfg) => {
      const val = parseInt(cfg.timeout_inactividad || "15", 10);
      timeoutMinutos.current = val;
    }).catch(() => {});
  }, [sesion]);

  // Listeners de actividad del usuario
  useEffect(() => {
    if (!sesion) return;

    const resetActivity = () => {
      lastActivity.current = Date.now();
    };

    const events = ["mousemove", "keydown", "click", "touchstart", "scroll"];
    events.forEach((evt) => window.addEventListener(evt, resetActivity, { passive: true }));

    // Verificar inactividad cada 30 segundos
    const interval = setInterval(() => {
      if (timeoutMinutos.current <= 0) return; // 0 = desactivado
      const elapsed = (Date.now() - lastActivity.current) / 1000 / 60; // en minutos
      if (elapsed >= timeoutMinutos.current) {
        cerrarSesion();
      }
    }, 30000);

    return () => {
      events.forEach((evt) => window.removeEventListener(evt, resetActivity));
      clearInterval(interval);
    };
  }, [sesion, cerrarSesion]);

  const esAdmin = sesion?.rol === "ADMIN";

  const tienePermiso = useCallback((permiso: string): boolean => {
    if (!sesion) return false;
    if (sesion.rol === "ADMIN") return true;
    try {
      const permisos = JSON.parse(sesion.permisos || "{}");
      return !!permisos[permiso];
    } catch {
      return false;
    }
  }, [sesion]);

  return (
    <SesionContext.Provider value={{ sesion, setSesion, cerrarSesion, esAdmin, tienePermiso }}>
      {children}
    </SesionContext.Provider>
  );
}

export function useSesion() {
  const ctx = useContext(SesionContext);
  if (!ctx) throw new Error("useSesion debe usarse dentro de SesionProvider");
  return ctx;
}
