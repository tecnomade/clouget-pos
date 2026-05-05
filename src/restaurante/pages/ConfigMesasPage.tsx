/**
 * ConfigMesasPage — CRUD de Zonas y Mesas del restaurante.
 *
 * Layout simple: dos columnas (zonas a la izquierda, mesas a la derecha).
 * Acceso: solo admin (filtrado en routing).
 */

import { useState, useEffect, useCallback, useMemo } from "react";
import { useToast } from "../../components/Toast";
import {
  listarZonas,
  crearZona,
  actualizarZona,
  eliminarZona,
  listarMesasConEstado,
  crearMesa,
  actualizarMesa,
  eliminarMesa,
} from "../api";
import type { Zona, Mesa, MesaConEstado } from "../types";

const COLORES_ZONA = [
  "#3b82f6", // azul
  "#10b981", // verde
  "#f59e0b", // naranja
  "#ec4899", // rosa
  "#8b5cf6", // violeta
  "#ef4444", // rojo
  "#14b8a6", // teal
  "#0ea5e9", // celeste
];

export default function ConfigMesasPage() {
  const { toastExito, toastError } = useToast();
  const [zonas, setZonas] = useState<Zona[]>([]);
  const [mesas, setMesas] = useState<MesaConEstado[]>([]);
  const [cargando, setCargando] = useState(true);

  // Forms
  const [editandoZona, setEditandoZona] = useState<Zona | null>(null);
  const [editandoMesa, setEditandoMesa] = useState<Mesa | null>(null);

  const cargar = useCallback(async () => {
    try {
      const [z, m] = await Promise.all([listarZonas(), listarMesasConEstado()]);
      setZonas(z);
      setMesas(m);
    } catch (err: any) {
      toastError("Error: " + (err?.message || err));
    } finally {
      setCargando(false);
    }
  }, [toastError]);

  useEffect(() => {
    cargar();
  }, [cargar]);

  // ─── Acciones zona ─────────────────────────────────────────────────────

  const handleNuevaZona = () => {
    setEditandoZona({
      nombre: "",
      color: COLORES_ZONA[zonas.length % COLORES_ZONA.length],
      orden: zonas.length,
      activa: true,
    });
  };

  const handleGuardarZona = async () => {
    if (!editandoZona) return;
    if (!editandoZona.nombre.trim()) {
      toastError("Nombre requerido");
      return;
    }
    try {
      if (editandoZona.id) {
        await actualizarZona(editandoZona);
        toastExito("Zona actualizada");
      } else {
        await crearZona({
          nombre: editandoZona.nombre.trim(),
          color: editandoZona.color,
          orden: editandoZona.orden,
          activa: editandoZona.activa,
        });
        toastExito("Zona creada");
      }
      setEditandoZona(null);
      await cargar();
    } catch (err: any) {
      toastError(err?.message || String(err));
    }
  };

  const handleEliminarZona = async (zona: Zona) => {
    const usadasPor = mesas.filter((m) => m.zona_id === zona.id).length;
    if (usadasPor > 0) {
      if (!confirm(`Esta zona tiene ${usadasPor} mesa(s). ¿Desactivarla? Las mesas quedarán sin zona pero seguirán activas.`)) return;
    } else {
      if (!confirm(`¿Eliminar zona "${zona.nombre}"?`)) return;
    }
    try {
      await eliminarZona(zona.id!);
      toastExito("Zona eliminada");
      await cargar();
    } catch (err: any) {
      toastError(err?.message || String(err));
    }
  };

  // ─── Acciones mesa ─────────────────────────────────────────────────────

  const handleNuevaMesa = () => {
    const ultimaMesa = mesas[mesas.length - 1];
    setEditandoMesa({
      nombre: `Mesa ${mesas.length + 1}`,
      zona_id: ultimaMesa?.zona_id ?? zonas[0]?.id ?? null,
      capacidad: 4,
      orden: mesas.length,
      activa: true,
    });
  };

  const handleGuardarMesa = async () => {
    if (!editandoMesa) return;
    if (!editandoMesa.nombre.trim()) {
      toastError("Nombre requerido");
      return;
    }
    try {
      if (editandoMesa.id) {
        await actualizarMesa(editandoMesa);
        toastExito("Mesa actualizada");
      } else {
        await crearMesa({
          nombre: editandoMesa.nombre.trim(),
          zona_id: editandoMesa.zona_id ?? null,
          capacidad: editandoMesa.capacidad,
          orden: editandoMesa.orden,
          activa: editandoMesa.activa,
        });
        toastExito("Mesa creada");
      }
      setEditandoMesa(null);
      await cargar();
    } catch (err: any) {
      toastError(err?.message || String(err));
    }
  };

  const handleEliminarMesa = async (mesa: MesaConEstado) => {
    if (mesa.estado !== "LIBRE") {
      toastError("No se puede eliminar una mesa con pedido abierto");
      return;
    }
    if (!confirm(`¿Eliminar "${mesa.nombre}"?`)) return;
    try {
      await eliminarMesa(mesa.id);
      toastExito("Mesa eliminada");
      await cargar();
    } catch (err: any) {
      toastError(err?.message || String(err));
    }
  };

  const mesasPorZona = useMemo(() => {
    const m = new Map<number | null, MesaConEstado[]>();
    for (const x of mesas) {
      const k = x.zona_id;
      if (!m.has(k)) m.set(k, []);
      m.get(k)!.push(x);
    }
    return m;
  }, [mesas]);

  if (cargando) {
    return (
      <div style={{ padding: 32, textAlign: "center", color: "var(--color-text-muted)" }}>
        Cargando configuración...
      </div>
    );
  }

  return (
    <div style={{ padding: "16px 20px", display: "flex", flexDirection: "column", gap: 16 }}>
      {/* Header */}
      <header>
        <h1 style={{ margin: 0, fontSize: 22, fontWeight: 700 }}>Configuración Restaurante</h1>
        <p style={{ margin: "4px 0 0 0", fontSize: 13, color: "var(--color-text-muted)" }}>
          Define las zonas y mesas de tu negocio.
        </p>
      </header>

      <div style={{ display: "grid", gridTemplateColumns: "minmax(260px, 1fr) 2fr", gap: 16 }}>
        {/* ─── ZONAS ──────────────────────────────────────────────── */}
        <div className="card">
          <div className="card-header" style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
            <strong>Zonas ({zonas.length})</strong>
            <button className="btn btn-primary" onClick={handleNuevaZona} style={{ padding: "4px 10px", fontSize: 12 }}>
              + Nueva
            </button>
          </div>
          <div className="card-body" style={{ padding: 0 }}>
            {zonas.length === 0 ? (
              <div style={{ padding: 20, textAlign: "center", color: "var(--color-text-muted)", fontSize: 13 }}>
                Sin zonas. Crea una para empezar.
              </div>
            ) : (
              zonas.map((z) => (
                <div
                  key={z.id}
                  style={{
                    display: "flex",
                    alignItems: "center",
                    gap: 8,
                    padding: "10px 14px",
                    borderTop: "1px solid var(--color-border)",
                  }}
                >
                  <span
                    style={{
                      width: 18,
                      height: 18,
                      background: z.color,
                      borderRadius: 4,
                      flexShrink: 0,
                    }}
                  />
                  <div style={{ flex: 1, minWidth: 0 }}>
                    <strong style={{ fontSize: 14 }}>{z.nombre}</strong>
                    <div style={{ fontSize: 11, color: "var(--color-text-muted)" }}>
                      {(mesasPorZona.get(z.id!) || []).length} mesa(s)
                    </div>
                  </div>
                  <button
                    onClick={() => setEditandoZona(z)}
                    title="Editar"
                    style={{
                      background: "transparent",
                      border: "1px solid var(--color-border)",
                      borderRadius: 4,
                      padding: "4px 8px",
                      fontSize: 12,
                      cursor: "pointer",
                      color: "var(--color-text)",
                    }}
                  >
                    ✎
                  </button>
                  <button
                    onClick={() => handleEliminarZona(z)}
                    title="Eliminar"
                    style={{
                      background: "transparent",
                      border: "1px solid var(--color-border)",
                      borderRadius: 4,
                      padding: "4px 8px",
                      fontSize: 12,
                      cursor: "pointer",
                      color: "var(--color-danger)",
                    }}
                  >
                    🗑
                  </button>
                </div>
              ))
            )}
          </div>
        </div>

        {/* ─── MESAS ──────────────────────────────────────────────── */}
        <div className="card">
          <div className="card-header" style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
            <strong>Mesas ({mesas.length})</strong>
            <button
              className="btn btn-primary"
              onClick={handleNuevaMesa}
              disabled={zonas.length === 0}
              style={{ padding: "4px 10px", fontSize: 12 }}
              title={zonas.length === 0 ? "Crea una zona primero" : ""}
            >
              + Nueva mesa
            </button>
          </div>
          <div className="card-body" style={{ padding: 0 }}>
            {mesas.length === 0 ? (
              <div style={{ padding: 20, textAlign: "center", color: "var(--color-text-muted)", fontSize: 13 }}>
                Sin mesas configuradas.
              </div>
            ) : (
              <table style={{ width: "100%", borderCollapse: "collapse", fontSize: 13 }}>
                <thead>
                  <tr style={{ background: "var(--color-surface-hover)", borderBottom: "1px solid var(--color-border)" }}>
                    <th style={{ padding: "8px 12px", textAlign: "left", fontSize: 11, fontWeight: 600, color: "var(--color-text-muted)" }}>Mesa</th>
                    <th style={{ padding: "8px 12px", textAlign: "left", fontSize: 11, fontWeight: 600, color: "var(--color-text-muted)" }}>Zona</th>
                    <th style={{ padding: "8px 12px", textAlign: "left", fontSize: 11, fontWeight: 600, color: "var(--color-text-muted)" }}>Capacidad</th>
                    <th style={{ padding: "8px 12px", textAlign: "left", fontSize: 11, fontWeight: 600, color: "var(--color-text-muted)" }}>Estado</th>
                    <th style={{ padding: "8px 12px", textAlign: "right", fontSize: 11, fontWeight: 600, color: "var(--color-text-muted)" }}>Acciones</th>
                  </tr>
                </thead>
                <tbody>
                  {mesas.map((m) => (
                    <tr key={m.id} style={{ borderTop: "1px solid var(--color-border)" }}>
                      <td style={{ padding: "8px 12px", fontWeight: 600 }}>{m.nombre}</td>
                      <td style={{ padding: "8px 12px" }}>
                        {m.zona_color && (
                          <span
                            style={{
                              display: "inline-block",
                              width: 8,
                              height: 8,
                              background: m.zona_color,
                              borderRadius: "50%",
                              marginRight: 6,
                            }}
                          />
                        )}
                        {m.zona_nombre || <em style={{ color: "var(--color-text-muted)" }}>Sin zona</em>}
                      </td>
                      <td style={{ padding: "8px 12px" }}>{m.capacidad} pax</td>
                      <td style={{ padding: "8px 12px" }}>
                        <span
                          style={{
                            fontSize: 10,
                            fontWeight: 700,
                            padding: "2px 6px",
                            borderRadius: 4,
                            background: m.estado === "LIBRE" ? "var(--color-success)" : "var(--color-warning)",
                            color: "#fff",
                          }}
                        >
                          {m.estado}
                        </span>
                      </td>
                      <td style={{ padding: "8px 12px", textAlign: "right" }}>
                        <button
                          onClick={() =>
                            setEditandoMesa({
                              id: m.id,
                              zona_id: m.zona_id,
                              nombre: m.nombre,
                              capacidad: m.capacidad,
                              orden: m.orden,
                              activa: true,
                            })
                          }
                          style={{
                            background: "transparent",
                            border: "1px solid var(--color-border)",
                            borderRadius: 4,
                            padding: "3px 7px",
                            marginRight: 4,
                            fontSize: 12,
                            cursor: "pointer",
                            color: "var(--color-text)",
                          }}
                        >
                          ✎
                        </button>
                        <button
                          onClick={() => handleEliminarMesa(m)}
                          disabled={m.estado !== "LIBRE"}
                          style={{
                            background: "transparent",
                            border: "1px solid var(--color-border)",
                            borderRadius: 4,
                            padding: "3px 7px",
                            fontSize: 12,
                            cursor: m.estado === "LIBRE" ? "pointer" : "not-allowed",
                            color: "var(--color-danger)",
                            opacity: m.estado === "LIBRE" ? 1 : 0.4,
                          }}
                        >
                          🗑
                        </button>
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
            )}
          </div>
        </div>
      </div>

      {/* ─── Modal editar zona ──────────────────────────────────── */}
      {editandoZona && (
        <div className="modal-overlay" onClick={() => setEditandoZona(null)}>
          <div
            className="modal-content"
            style={{ maxWidth: 400 }}
            onClick={(e) => e.stopPropagation()}
          >
            <div className="modal-header">
              <h3 style={{ margin: 0 }}>{editandoZona.id ? "Editar zona" : "Nueva zona"}</h3>
            </div>
            <div className="modal-body" style={{ display: "flex", flexDirection: "column", gap: 12 }}>
              <label style={{ fontSize: 13, fontWeight: 600 }}>
                Nombre
                <input
                  autoFocus
                  value={editandoZona.nombre}
                  onChange={(e) => setEditandoZona({ ...editandoZona, nombre: e.target.value })}
                  className="input"
                  placeholder="Salón, Terraza, Barra..."
                  style={{ width: "100%", marginTop: 4 }}
                />
              </label>
              <div>
                <label style={{ fontSize: 13, fontWeight: 600, display: "block", marginBottom: 6 }}>Color</label>
                <div style={{ display: "flex", gap: 6, flexWrap: "wrap" }}>
                  {COLORES_ZONA.map((c) => (
                    <button
                      key={c}
                      onClick={() => setEditandoZona({ ...editandoZona, color: c })}
                      style={{
                        width: 32,
                        height: 32,
                        background: c,
                        border: editandoZona.color === c ? "3px solid var(--color-text)" : "1px solid var(--color-border)",
                        borderRadius: 6,
                        cursor: "pointer",
                      }}
                    />
                  ))}
                </div>
              </div>
            </div>
            <div className="modal-footer" style={{ display: "flex", gap: 8, justifyContent: "flex-end" }}>
              <button className="btn btn-outline" onClick={() => setEditandoZona(null)}>
                Cancelar
              </button>
              <button className="btn btn-primary" onClick={handleGuardarZona}>
                Guardar
              </button>
            </div>
          </div>
        </div>
      )}

      {/* ─── Modal editar mesa ──────────────────────────────────── */}
      {editandoMesa && (
        <div className="modal-overlay" onClick={() => setEditandoMesa(null)}>
          <div
            className="modal-content"
            style={{ maxWidth: 400 }}
            onClick={(e) => e.stopPropagation()}
          >
            <div className="modal-header">
              <h3 style={{ margin: 0 }}>{editandoMesa.id ? "Editar mesa" : "Nueva mesa"}</h3>
            </div>
            <div className="modal-body" style={{ display: "flex", flexDirection: "column", gap: 12 }}>
              <label style={{ fontSize: 13, fontWeight: 600 }}>
                Nombre
                <input
                  autoFocus
                  value={editandoMesa.nombre}
                  onChange={(e) => setEditandoMesa({ ...editandoMesa, nombre: e.target.value })}
                  className="input"
                  placeholder="Mesa 1, Barra-1..."
                  style={{ width: "100%", marginTop: 4 }}
                />
              </label>
              <label style={{ fontSize: 13, fontWeight: 600 }}>
                Zona
                <select
                  value={editandoMesa.zona_id ?? ""}
                  onChange={(e) =>
                    setEditandoMesa({
                      ...editandoMesa,
                      zona_id: e.target.value ? parseInt(e.target.value, 10) : null,
                    })
                  }
                  className="input"
                  style={{ width: "100%", marginTop: 4 }}
                >
                  <option value="">— Sin zona —</option>
                  {zonas.map((z) => (
                    <option key={z.id} value={z.id}>
                      {z.nombre}
                    </option>
                  ))}
                </select>
              </label>
              <label style={{ fontSize: 13, fontWeight: 600 }}>
                Capacidad (personas)
                <input
                  type="number"
                  min="1"
                  max="50"
                  value={editandoMesa.capacidad}
                  onChange={(e) => setEditandoMesa({ ...editandoMesa, capacidad: parseInt(e.target.value, 10) || 1 })}
                  className="input"
                  style={{ width: "100%", marginTop: 4 }}
                />
              </label>
            </div>
            <div className="modal-footer" style={{ display: "flex", gap: 8, justifyContent: "flex-end" }}>
              <button className="btn btn-outline" onClick={() => setEditandoMesa(null)}>
                Cancelar
              </button>
              <button className="btn btn-primary" onClick={handleGuardarMesa}>
                Guardar
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
