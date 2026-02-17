import { useState, useEffect, useCallback, createContext, useContext } from "react";

interface ToastMsg {
  id: number;
  tipo: "success" | "error" | "info" | "warning";
  mensaje: string;
}

interface ToastContextType {
  toast: (mensaje: string, tipo?: ToastMsg["tipo"]) => void;
  toastExito: (mensaje: string) => void;
  toastError: (mensaje: string) => void;
  toastWarning: (mensaje: string) => void;
}

const ToastContext = createContext<ToastContextType | null>(null);

let nextId = 0;

export function ToastProvider({ children }: { children: React.ReactNode }) {
  const [toasts, setToasts] = useState<ToastMsg[]>([]);

  const agregar = useCallback((mensaje: string, tipo: ToastMsg["tipo"] = "info") => {
    const id = ++nextId;
    setToasts((prev) => [...prev, { id, tipo, mensaje }]);
    setTimeout(() => {
      setToasts((prev) => prev.filter((t) => t.id !== id));
    }, 4000);
  }, []);

  const ctx: ToastContextType = {
    toast: agregar,
    toastExito: (m) => agregar(m, "success"),
    toastError: (m) => agregar(m, "error"),
    toastWarning: (m) => agregar(m, "warning"),
  };

  return (
    <ToastContext.Provider value={ctx}>
      {children}
      <div className="toast-container">
        {toasts.map((t) => (
          <ToastItem key={t.id} toast={t} onClose={() => setToasts((prev) => prev.filter((x) => x.id !== t.id))} />
        ))}
      </div>
    </ToastContext.Provider>
  );
}

function ToastItem({ toast, onClose }: { toast: ToastMsg; onClose: () => void }) {
  const [saliendo, setSaliendo] = useState(false);

  useEffect(() => {
    const timer = setTimeout(() => setSaliendo(true), 3500);
    return () => clearTimeout(timer);
  }, []);

  const iconos = {
    success: "\u2713",
    error: "\u2717",
    warning: "!",
    info: "i",
  };

  return (
    <div className={`toast toast-${toast.tipo}${saliendo ? " toast-exit" : ""}`}>
      <span className="toast-icon">{iconos[toast.tipo]}</span>
      <span className="toast-msg">{toast.mensaje}</span>
      <button className="toast-close" onClick={onClose}>&times;</button>
    </div>
  );
}

export function useToast() {
  const ctx = useContext(ToastContext);
  if (!ctx) throw new Error("useToast debe usarse dentro de ToastProvider");
  return ctx;
}
