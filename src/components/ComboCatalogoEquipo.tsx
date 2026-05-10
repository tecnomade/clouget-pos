/**
 * v2.4.10 — ST-2.5: Combo input con autocomplete del catálogo + botón "+"
 * para crear inline.
 *
 * Componente genérico usado en cascada para tipo→marca→modelo dentro del
 * form de orden de servicio. Soporta:
 *  - Selección del catálogo (dropdown con sugerencias)
 *  - Texto libre (por si no está en el catálogo)
 *  - Crear nuevo desde el mismo form (botón +) sin abrir Configuración
 */

import { useEffect, useRef, useState } from "react";
import { useToast } from "./Toast";

interface OpcionCatalogo {
  id: number;
  nombre: string;
}

interface Props {
  /** Etiqueta visible arriba del input */
  label: string;
  /** Texto libre actual (siempre se mantiene como fallback) */
  valorTexto: string;
  /** ID del catálogo si fue seleccionado del catálogo (NULL si texto libre) */
  valorId: number | null;
  /** Lista de opciones del catálogo (cargada por el padre) */
  opciones: OpcionCatalogo[];
  /** Callback cuando cambia. Si elige del catálogo: pasa id+nombre. Si escribe libre: pasa null+texto */
  onChange: (id: number | null, nombre: string) => void;
  /** Función que crea una nueva entrada en el catálogo. Recibe el nombre, devuelve el id creado. */
  onCrearNuevo?: (nombre: string) => Promise<number>;
  placeholder?: string;
  disabled?: boolean;
  required?: boolean;
}

export default function ComboCatalogoEquipo({
  label, valorTexto, valorId, opciones, onChange, onCrearNuevo,
  placeholder, disabled, required,
}: Props) {
  const { toastExito, toastError } = useToast();
  const [foco, setFoco] = useState(false);
  const [creando, setCreando] = useState(false);
  const containerRef = useRef<HTMLDivElement>(null);
  const inputRef = useRef<HTMLInputElement>(null);

  // Cerrar dropdown al click fuera
  useEffect(() => {
    if (!foco) return;
    const handler = (e: MouseEvent) => {
      if (!containerRef.current?.contains(e.target as Node)) {
        setFoco(false);
      }
    };
    setTimeout(() => document.addEventListener("mousedown", handler), 0);
    return () => document.removeEventListener("mousedown", handler);
  }, [foco]);

  const sugerencias = opciones.filter(o => {
    if (!valorTexto.trim()) return true;
    return o.nombre.toLowerCase().includes(valorTexto.toLowerCase().trim());
  }).slice(0, 30);

  const valorEnCatalogo = opciones.some(o => o.nombre.toLowerCase() === valorTexto.trim().toLowerCase());
  const puedeCrear = valorTexto.trim().length > 0 && !valorEnCatalogo && !!onCrearNuevo;

  const handleCrear = async () => {
    if (!onCrearNuevo || !valorTexto.trim()) return;
    setCreando(true);
    try {
      const nuevoId = await onCrearNuevo(valorTexto.trim());
      onChange(nuevoId, valorTexto.trim());
      toastExito(`"${valorTexto.trim()}" agregado al catálogo`);
      setFoco(false);
    } catch (err: any) {
      toastError(err?.toString() || "Error creando");
    } finally {
      setCreando(false);
    }
  };

  return (
    <div ref={containerRef} style={{ position: "relative" }}>
      <label style={{ fontSize: 12, fontWeight: 600, display: "flex", justifyContent: "space-between", alignItems: "center" }}>
        <span>
          {label} {required && <span style={{ color: "var(--color-danger)" }}>*</span>}
          {valorId && (
            <span title="Vinculado al catálogo" style={{
              marginLeft: 6, fontSize: 9, padding: "1px 5px", borderRadius: 4,
              background: "var(--color-success)", color: "#fff", fontWeight: 700,
            }}>✓ catálogo</span>
          )}
        </span>
        {puedeCrear && (
          <button
            type="button"
            onClick={handleCrear}
            disabled={creando || disabled}
            style={{
              fontSize: 10, padding: "2px 8px",
              background: "var(--color-primary)", color: "#fff",
              border: "none", borderRadius: 4, cursor: "pointer",
              opacity: creando ? 0.5 : 1,
            }}
            title={`Agregar "${valorTexto.trim()}" al catálogo`}
          >
            {creando ? "..." : `+ Agregar al catálogo`}
          </button>
        )}
      </label>
      <input
        ref={inputRef}
        className="input"
        value={valorTexto}
        placeholder={placeholder}
        disabled={disabled}
        onFocus={() => { if (!disabled) setFoco(true); }}
        onChange={(e) => {
          if (disabled) return;
          // Al editar texto, limpiamos el id (el valor ya no corresponde al item seleccionado)
          onChange(null, e.target.value);
          setFoco(true);
        }}
        autoComplete="off"
        style={disabled ? { opacity: 0.55, cursor: "not-allowed", background: "var(--color-surface-alt)" } : undefined}
      />
      {foco && opciones.length > 0 && (
        <div style={{
          position: "absolute", top: "100%", left: 0, right: 0,
          maxHeight: 200, overflowY: "auto",
          background: "var(--color-surface)", border: "1px solid var(--color-border)",
          borderRadius: 6, marginTop: 2, zIndex: 50,
          boxShadow: "0 4px 12px rgba(0,0,0,0.2)",
        }}>
          {sugerencias.length === 0 ? (
            <div style={{ padding: "8px 10px", fontSize: 12, color: "var(--color-text-muted)" }}>
              {valorTexto.trim() ? `Sin coincidencias. ${onCrearNuevo ? 'Click "+ Agregar al catálogo".' : ''}` : "Sin opciones"}
            </div>
          ) : (
            sugerencias.map(op => (
              <div
                key={op.id}
                onClick={() => { onChange(op.id, op.nombre); setFoco(false); }}
                style={{
                  padding: "6px 10px", fontSize: 13, cursor: "pointer",
                  background: valorId === op.id ? "var(--color-primary)" : "transparent",
                  color: valorId === op.id ? "#fff" : "var(--color-text)",
                }}
                onMouseEnter={e => {
                  if (valorId !== op.id) e.currentTarget.style.background = "var(--color-surface-alt)";
                }}
                onMouseLeave={e => {
                  if (valorId !== op.id) e.currentTarget.style.background = "transparent";
                }}
              >
                {op.nombre}
              </div>
            ))
          )}
        </div>
      )}
    </div>
  );
}
