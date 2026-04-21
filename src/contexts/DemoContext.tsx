import { createContext, useContext, useState, useEffect, useCallback } from "react";
import { esDemo as checkDemo, salirDemo as apiSalirDemo } from "../services/api";

interface DemoContextType {
  esDemo: boolean;
  salirDemo: () => Promise<void>;
}

const DemoContext = createContext<DemoContextType>({
  esDemo: false,
  salirDemo: async () => {},
});

export function DemoProvider({ children }: { children: React.ReactNode }) {
  const [demo, setDemo] = useState(false);

  useEffect(() => {
    checkDemo().then(setDemo).catch(() => setDemo(false));
  }, []);

  const salirDemo = useCallback(async () => {
    let backendOk = false;
    try {
      await apiSalirDemo();
      backendOk = true;
    } catch (err) {
      console.error("Error saliendo de demo:", err);
      // Si el backend falló, mostrar mensaje y dejar al usuario decidir
      const reintentar = confirm("Error al salir del modo demo: " + err + "\n\n¿Forzar reinicio de la app de todos modos? (puede requerir cerrar y volver a abrir)");
      if (!reintentar) return;
    }
    setDemo(false);
    // Forzar recarga completa
    if (backendOk) {
      setTimeout(() => {
        window.location.href = window.location.origin;
      }, 100);
    } else {
      // Reload duro
      window.location.reload();
    }
  }, []);

  return (
    <DemoContext.Provider value={{ esDemo: demo, salirDemo }}>
      {children}
    </DemoContext.Provider>
  );
}

export function useDemo() {
  return useContext(DemoContext);
}
