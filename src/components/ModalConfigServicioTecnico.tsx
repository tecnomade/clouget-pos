/**
 * v2.4.9 — ST-2: Modal de configuración del catálogo jerárquico
 * tipos→marcas→modelos para el módulo Servicio Técnico.
 *
 * Vista en árbol expandible con CRUD inline. Soft-delete en backend.
 */

import { useEffect, useState } from "react";
import { useToast } from "./Toast";
import {
  stListarArbolCompleto,
  stCrearTipoEquipo, stActualizarTipoEquipo, stEliminarTipoEquipo,
  stCrearMarca, stEliminarMarca,
  stCrearModelo, stEliminarModelo,
  type StTipoEquipo,
} from "../services/api";

interface Props {
  onCerrar: () => void;
}

export default function ModalConfigServicioTecnico({ onCerrar }: Props) {
  const { toastExito, toastError } = useToast();
  const [arbol, setArbol] = useState<any[] | null>(null);
  const [expandidos, setExpandidos] = useState<Set<string>>(new Set());
  const [creandoTipo, setCreandoTipo] = useState(false);
  const [editandoTipo, setEditandoTipo] = useState<StTipoEquipo | null>(null);

  const cargar = async () => {
    try {
      const a = await stListarArbolCompleto();
      setArbol(a);
    } catch (err: any) {
      toastError("Error: " + (err?.message || err));
    }
  };

  useEffect(() => { cargar(); }, []);

  const toggleNodo = (key: string) => {
    setExpandidos(prev => {
      const next = new Set(prev);
      if (next.has(key)) next.delete(key); else next.add(key);
      return next;
    });
  };

  const handleAgregarMarca = async (tipoId: number) => {
    const nombre = prompt("Nombre de la nueva marca:");
    if (!nombre?.trim()) return;
    try {
      await stCrearMarca({ tipo_equipo_id: tipoId, nombre: nombre.trim(), activo: true });
      toastExito("Marca creada");
      await cargar();
    } catch (err: any) { toastError(err?.toString()); }
  };

  const handleAgregarModelo = async (marcaId: number) => {
    const nombre = prompt("Nombre del nuevo modelo:");
    if (!nombre?.trim()) return;
    const anioStr = prompt("Año desde (opcional, dejar vacío para sin año):");
    const anioDesde = anioStr?.trim() ? parseInt(anioStr) : null;
    try {
      await stCrearModelo({ marca_id: marcaId, nombre: nombre.trim(), anio_desde: anioDesde, anio_hasta: null, activo: true });
      toastExito("Modelo creado");
      await cargar();
    } catch (err: any) { toastError(err?.toString()); }
  };

  const handleEliminarTipo = async (id: number, nombre: string, count: number) => {
    if (count > 0) {
      if (!confirm(`"${nombre}" tiene ${count} órden(es) asociada(s). Al eliminar, las órdenes seguirán visibles pero ya no podrá usarse para nuevas. ¿Continuar?`)) return;
    } else {
      if (!confirm(`¿Eliminar "${nombre}"?`)) return;
    }
    try { await stEliminarTipoEquipo(id); toastExito("Eliminado"); await cargar(); }
    catch (err: any) { toastError(err?.toString()); }
  };

  const handleEliminarMarca = async (id: number, nombre: string, count: number) => {
    if (!confirm(`¿Eliminar marca "${nombre}"?${count > 0 ? ` (${count} órdenes asociadas seguirán visibles)` : ""}`)) return;
    try { await stEliminarMarca(id); toastExito("Marca eliminada"); await cargar(); }
    catch (err: any) { toastError(err?.toString()); }
  };

  const handleEliminarModelo = async (id: number, nombre: string, count: number) => {
    if (!confirm(`¿Eliminar modelo "${nombre}"?${count > 0 ? ` (${count} órdenes asociadas)` : ""}`)) return;
    try { await stEliminarModelo(id); toastExito("Modelo eliminado"); await cargar(); }
    catch (err: any) { toastError(err?.toString()); }
  };

  return (
    <div className="modal-overlay" onClick={onCerrar} style={{ zIndex: 100 }}>
      <div
        className="modal-content"
        onClick={e => e.stopPropagation()}
        style={{ maxWidth: 700, width: "100%", maxHeight: "90vh", display: "flex", flexDirection: "column" }}
      >
        <div className="modal-header" style={{ display: "flex", justifyContent: "space-between", alignItems: "center", padding: "14px 18px", borderBottom: "1px solid var(--color-border)" }}>
          <div>
            <h2 style={{ margin: 0, fontSize: 18 }}>⚙ Configuración Servicio Técnico</h2>
            <div style={{ fontSize: 12, color: "var(--color-text-secondary)", marginTop: 2 }}>
              Tipos de equipo · Marcas · Modelos (jerárquico)
            </div>
          </div>
          <button onClick={onCerrar} style={{ background: "transparent", border: "none", fontSize: 24, cursor: "pointer", color: "var(--color-text-muted)" }}>×</button>
        </div>

        <div className="modal-body" style={{ flex: 1, overflowY: "auto", padding: 16 }}>
          {!arbol ? (
            <div style={{ textAlign: "center", padding: 30 }}>Cargando...</div>
          ) : (
            <>
              <button
                className="btn btn-primary"
                onClick={() => { setEditandoTipo(null); setCreandoTipo(true); }}
                style={{ marginBottom: 12, padding: "6px 14px", fontSize: 12 }}
              >
                + Nuevo tipo de equipo
              </button>

              {arbol.length === 0 ? (
                <div style={{ padding: 30, textAlign: "center", color: "var(--color-text-secondary)", border: "2px dashed var(--color-border)", borderRadius: 8 }}>
                  Aún no hay tipos de equipo. Crea uno para empezar.
                </div>
              ) : (
                <div style={{ display: "flex", flexDirection: "column", gap: 6 }}>
                  {arbol.map(tipo => {
                    const tipoKey = `t-${tipo.id}`;
                    const tipoExpandido = expandidos.has(tipoKey);
                    return (
                      <div key={tipo.id} style={{ border: "1px solid var(--color-border)", borderRadius: 8, background: "var(--color-surface)" }}>
                        {/* Tipo de equipo */}
                        <div style={{ display: "flex", alignItems: "center", padding: "8px 10px", cursor: "pointer" }}
                             onClick={() => toggleNodo(tipoKey)}>
                          <span style={{ fontSize: 12, marginRight: 6, color: "var(--color-text-muted)" }}>{tipoExpandido ? "▼" : "▶"}</span>
                          <span style={{ fontSize: 18, marginRight: 8 }}>{tipo.icono}</span>
                          <span style={{ fontWeight: 600, flex: 1 }}>{tipo.nombre}</span>
                          <span style={{ fontSize: 11, color: "var(--color-text-muted)", marginRight: 10 }}>
                            {tipo.marcas.length} marca(s) · {tipo.ordenes_count} órden(es)
                          </span>
                          <button onClick={(e) => { e.stopPropagation(); setEditandoTipo({ ...tipo, activo: true }); setCreandoTipo(true); }}
                                  style={{ marginRight: 4, padding: "3px 8px", fontSize: 11, background: "transparent", border: "1px solid var(--color-border)", borderRadius: 4, cursor: "pointer", color: "var(--color-text)" }}>Editar</button>
                          <button onClick={(e) => { e.stopPropagation(); handleEliminarTipo(tipo.id, tipo.nombre, tipo.ordenes_count); }}
                                  style={{ padding: "3px 8px", fontSize: 11, background: "transparent", border: "1px solid var(--color-danger)", borderRadius: 4, cursor: "pointer", color: "var(--color-danger)" }}>×</button>
                        </div>

                        {/* Marcas (si tipo expandido) */}
                        {tipoExpandido && (
                          <div style={{ padding: "0 10px 8px 30px", borderTop: "1px solid var(--color-border)" }}>
                            <button onClick={() => handleAgregarMarca(tipo.id)}
                                    style={{ margin: "8px 0", padding: "3px 10px", fontSize: 11, background: "var(--color-primary)", color: "#fff", border: "none", borderRadius: 4, cursor: "pointer" }}>
                              + Marca
                            </button>
                            {tipo.marcas.length === 0 ? (
                              <div style={{ fontSize: 11, color: "var(--color-text-muted)", padding: 4 }}>Sin marcas</div>
                            ) : (
                              tipo.marcas.map((marca: any) => {
                                const marcaKey = `m-${marca.id}`;
                                const marcaExpandida = expandidos.has(marcaKey);
                                return (
                                  <div key={marca.id} style={{ marginTop: 4, border: "1px solid var(--color-border)", borderRadius: 6, background: "var(--color-surface-alt)" }}>
                                    <div style={{ display: "flex", alignItems: "center", padding: "6px 8px", cursor: "pointer" }}
                                         onClick={() => toggleNodo(marcaKey)}>
                                      <span style={{ fontSize: 11, marginRight: 6, color: "var(--color-text-muted)" }}>{marcaExpandida ? "▼" : "▶"}</span>
                                      <span style={{ fontSize: 13, flex: 1 }}>{marca.nombre}</span>
                                      <span style={{ fontSize: 10, color: "var(--color-text-muted)", marginRight: 8 }}>{marca.modelos.length} modelos</span>
                                      <button onClick={(e) => { e.stopPropagation(); handleEliminarMarca(marca.id, marca.nombre, marca.ordenes_count); }}
                                              style={{ padding: "2px 6px", fontSize: 11, background: "transparent", border: "1px solid var(--color-danger)", borderRadius: 4, cursor: "pointer", color: "var(--color-danger)" }}>×</button>
                                    </div>
                                    {marcaExpandida && (
                                      <div style={{ padding: "0 8px 6px 24px", borderTop: "1px solid var(--color-border)" }}>
                                        <button onClick={() => handleAgregarModelo(marca.id)}
                                                style={{ margin: "6px 0", padding: "2px 8px", fontSize: 10, background: "var(--color-primary)", color: "#fff", border: "none", borderRadius: 4, cursor: "pointer" }}>
                                          + Modelo
                                        </button>
                                        {marca.modelos.length === 0 ? (
                                          <div style={{ fontSize: 10, color: "var(--color-text-muted)", padding: 3 }}>Sin modelos</div>
                                        ) : (
                                          marca.modelos.map((modelo: any) => (
                                            <div key={modelo.id} style={{ display: "flex", alignItems: "center", padding: "4px 6px", fontSize: 12, gap: 6 }}>
                                              <span style={{ flex: 1 }}>
                                                {modelo.nombre}
                                                {modelo.anio_desde && (
                                                  <span style={{ color: "var(--color-text-muted)", marginLeft: 6, fontSize: 10 }}>
                                                    ({modelo.anio_desde}{modelo.anio_hasta ? `–${modelo.anio_hasta}` : ""})
                                                  </span>
                                                )}
                                              </span>
                                              <span style={{ fontSize: 10, color: "var(--color-text-muted)" }}>{modelo.ordenes_count} órd</span>
                                              <button onClick={() => handleEliminarModelo(modelo.id, modelo.nombre, modelo.ordenes_count)}
                                                      style={{ padding: "2px 6px", fontSize: 10, background: "transparent", border: "1px solid var(--color-danger)", borderRadius: 4, cursor: "pointer", color: "var(--color-danger)" }}>×</button>
                                            </div>
                                          ))
                                        )}
                                      </div>
                                    )}
                                  </div>
                                );
                              })
                            )}
                          </div>
                        )}
                      </div>
                    );
                  })}
                </div>
              )}
            </>
          )}
        </div>
      </div>

      {/* Modal crear/editar tipo */}
      {creandoTipo && (
        <ModalEditarTipo
          inicial={editandoTipo}
          onCerrar={() => setCreandoTipo(false)}
          onGuardado={async () => {
            setCreandoTipo(false);
            await cargar();
            toastExito(editandoTipo ? "Tipo actualizado" : "Tipo creado");
          }}
        />
      )}
    </div>
  );
}

function ModalEditarTipo({
  inicial, onCerrar, onGuardado,
}: { inicial: StTipoEquipo | null; onCerrar: () => void; onGuardado: () => void }) {
  const { toastError } = useToast();
  const [form, setForm] = useState<StTipoEquipo>(inicial || {
    nombre: "",
    icono: "🔧",
    requiere_placa: false,
    requiere_kilometraje: false,
    requiere_serie: false,
    orden: 0,
    activo: true,
  });

  const guardar = async () => {
    if (!form.nombre.trim()) { toastError("Nombre requerido"); return; }
    try {
      if (inicial?.id) await stActualizarTipoEquipo(form);
      else await stCrearTipoEquipo(form);
      onGuardado();
    } catch (err: any) { toastError(err?.toString()); }
  };

  return (
    <div className="modal-overlay" style={{ zIndex: 200 }} onClick={onCerrar}>
      <div className="modal-content" onClick={e => e.stopPropagation()} style={{ maxWidth: 420 }}>
        <div className="modal-header" style={{ padding: "12px 18px", borderBottom: "1px solid var(--color-border)" }}>
          <h3 style={{ margin: 0, fontSize: 16 }}>{inicial?.id ? "Editar" : "Nuevo"} tipo de equipo</h3>
        </div>
        <div className="modal-body" style={{ padding: 16, display: "flex", flexDirection: "column", gap: 10 }}>
          <label style={{ fontSize: 12, fontWeight: 600 }}>
            Nombre
            <input className="input" autoFocus value={form.nombre} onChange={e => setForm({ ...form, nombre: e.target.value })} placeholder="Ej. Vehículo, Computadora..." />
          </label>
          <label style={{ fontSize: 12, fontWeight: 600 }}>
            Icono (emoji)
            <input className="input" value={form.icono} onChange={e => setForm({ ...form, icono: e.target.value })} maxLength={4} placeholder="🚗" />
          </label>
          <label style={{ fontSize: 12, fontWeight: 600 }}>
            Orden
            <input className="input" type="number" value={form.orden} onChange={e => setForm({ ...form, orden: parseInt(e.target.value) || 0 })} />
          </label>
          <div style={{ marginTop: 6, padding: 8, background: "var(--color-surface-alt)", borderRadius: 6 }}>
            <div style={{ fontSize: 11, fontWeight: 700, marginBottom: 6, textTransform: "uppercase", letterSpacing: 0.5, color: "var(--color-text-muted)" }}>Campos requeridos al ingresar equipo</div>
            <label style={{ fontSize: 12, display: "flex", gap: 6, cursor: "pointer", marginBottom: 4 }}>
              <input type="checkbox" checked={form.requiere_placa} onChange={e => setForm({ ...form, requiere_placa: e.target.checked })} />
              Placa (vehículos)
            </label>
            <label style={{ fontSize: 12, display: "flex", gap: 6, cursor: "pointer", marginBottom: 4 }}>
              <input type="checkbox" checked={form.requiere_kilometraje} onChange={e => setForm({ ...form, requiere_kilometraje: e.target.checked })} />
              Kilometraje
            </label>
            <label style={{ fontSize: 12, display: "flex", gap: 6, cursor: "pointer" }}>
              <input type="checkbox" checked={form.requiere_serie} onChange={e => setForm({ ...form, requiere_serie: e.target.checked })} />
              Número de serie (electrónicos)
            </label>
          </div>
        </div>
        <div className="modal-footer" style={{ padding: "12px 18px", borderTop: "1px solid var(--color-border)", display: "flex", gap: 8, justifyContent: "flex-end" }}>
          <button className="btn btn-outline" onClick={onCerrar}>Cancelar</button>
          <button className="btn btn-primary" onClick={guardar}>Guardar</button>
        </div>
      </div>
    </div>
  );
}
