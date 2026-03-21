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
    await apiSalirDemo();
    setDemo(false);
    // Recargar la app para volver a LicenciaPage
    window.location.reload();
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
