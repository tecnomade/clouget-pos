import { useEffect } from "react";
import { useNavigate } from "react-router-dom";

export interface Shortcut {
  key: string;
  ctrl?: boolean;
  alt?: boolean;
  shift?: boolean;
  description: string;
  action: () => void;
  adminOnly?: boolean;
}

export function useKeyboardShortcuts(rol?: string) {
  const navigate = useNavigate();
  const esCajero = rol === "CAJERO";

  useEffect(() => {
    const shortcuts: Shortcut[] = [
      // Navegacion con F keys - disponibles para todos
      { key: "F1", description: "Ir a Punto de Venta", action: () => navigate("/pos") },
      { key: "F5", description: "Ir a Caja", action: () => navigate("/caja") },
      // Solo admin
      { key: "F2", description: "Ir a Productos", action: () => navigate("/productos"), adminOnly: true },
      { key: "F3", description: "Ir a Clientes", action: () => navigate("/clientes"), adminOnly: true },
      { key: "F4", description: "Ir a Ventas del dia", action: () => navigate("/ventas") },
      { key: "F6", description: "Ir a Configuracion", action: () => navigate("/config"), adminOnly: true },
      { key: "F7", description: "Ir a Gastos", action: () => navigate("/gastos"), adminOnly: true },
      { key: "F8", description: "Ir a Fiados", action: () => navigate("/cuentas"), adminOnly: true },
      // Acciones rapidas en POS - disponibles para todos
      // Usan CustomEvent para no depender del estado disabled del boton DOM
      { key: "F9", description: "Cobrar venta", action: () => {
        window.dispatchEvent(new CustomEvent("pos-cobrar"));
      }},
      { key: "F10", description: "Nueva venta", action: () => {
        window.dispatchEvent(new CustomEvent("pos-nueva-venta"));
      }},
      // Ctrl shortcuts - disponibles para todos
      { key: "b", ctrl: true, description: "Enfocar busqueda", action: () => {
        const input = document.querySelector("[data-action='busqueda']") as HTMLInputElement;
        input?.focus();
      }},
      { key: "n", ctrl: true, description: "Nuevo producto/cliente", action: () => {
        const btn = document.querySelector("[data-action='nuevo']") as HTMLButtonElement;
        btn?.click();
      }, adminOnly: true },
    ];

    const handler = (e: KeyboardEvent) => {
      for (const s of shortcuts) {
        // Filtrar por rol
        if (esCajero && s.adminOnly) continue;

        const keyMatch = e.key === s.key || e.key.toLowerCase() === s.key.toLowerCase();
        const ctrlMatch = !!s.ctrl === (e.ctrlKey || e.metaKey);
        const altMatch = !!s.alt === e.altKey;
        const shiftMatch = !!s.shift === e.shiftKey;

        if (keyMatch && ctrlMatch && altMatch && shiftMatch) {
          e.preventDefault();
          s.action();
          return;
        }
      }
    };

    window.addEventListener("keydown", handler);
    return () => window.removeEventListener("keydown", handler);
  }, [navigate, esCajero]);
}

// Exportar lista de shortcuts para mostrar en UI
export const SHORTCUTS_LIST = [
  { keys: "F1", description: "Punto de Venta" },
  { keys: "F2", description: "Productos" },
  { keys: "F3", description: "Clientes" },
  { keys: "F4", description: "Ventas del dia" },
  { keys: "F5", description: "Caja" },
  { keys: "F6", description: "Configuracion" },
  { keys: "F7", description: "Gastos" },
  { keys: "F8", description: "Fiados" },
  { keys: "F9", description: "Cobrar venta" },
  { keys: "F10", description: "Nueva venta" },
  { keys: "Ctrl+B", description: "Buscar producto" },
  { keys: "Ctrl+N", description: "Nuevo registro" },
  { keys: "Enter", description: "Agregar primer resultado / Cobrar" },
];
