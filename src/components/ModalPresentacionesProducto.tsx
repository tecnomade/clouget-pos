/**
 * v2.6.25 — Modal de gestion de presentaciones de compra de un producto.
 * Permite definir multiples presentaciones (Jaba x12, Six-pack, Caja x24...)
 * que despues se usan en el formulario de compra para ingresar cantidades
 * por bulto en lugar de por unidad.
 */
import { useEffect, useState } from "react";
import { useToast } from "./Toast";
import {
  listarPresentacionesProducto,
  guardarPresentacionesProducto,
  buscarProductos,
  listarPresentacionesUnicas,
} from "../services/api";
import type { ProductoPresentacion, ProductoBusqueda } from "../types";

interface Props {
  productoId: number;
  productoNombre: string;
  unidadBase?: string;
  onClose: () => void;
}

type Editable = ProductoPresentacion & { _key: number };

export default function ModalPresentacionesProducto({
  productoId,
  productoNombre,
  unidadBase = "UND",
  onClose,
}: Props) {
  const { toastExito, toastError } = useToast();
  const [presentaciones, setPresentaciones] = useState<Editable[]>([]);
  const [cargando, setCargando] = useState(true);
  const [guardando, setGuardando] = useState(false);
  // v2.6.26: aplicar las MISMAS presentaciones a otro producto (shortcut UX)
  const [aplicarOpen, setAplicarOpen] = useState(false);
  const [busquedaProd, setBusquedaProd] = useState("");
  const [resultadosBusqueda, setResultadosBusqueda] = useState<ProductoBusqueda[]>([]);
  const [buscando, setBuscando] = useState(false);
  // v2.6.25: catálogo de unidades agrupadas YA existentes (de otros productos +
  // tipos de unidad agrupados) para sugerir en el campo Nombre y autollenar factor.
  const [sugerencias, setSugerencias] = useState<{ nombre: string; factor: number }[]>([]);

  useEffect(() => {
    listarPresentacionesProducto(productoId)
      .then((rows) => {
        setPresentaciones(
          rows.map((r, i) => ({ ...r, _key: Date.now() + i })),
        );
      })
      .catch((e) => toastError("Error cargando presentaciones: " + String(e)))
      .finally(() => setCargando(false));
    // Cargar el catálogo de presentaciones/unidades agrupadas únicas.
    listarPresentacionesUnicas()
      .then((sugs: any[]) =>
        setSugerencias((sugs || []).map((s) => ({ nombre: s.nombre, factor: s.factor }))),
      )
      .catch(() => setSugerencias([]));
  }, [productoId]);

  const agregar = () => {
    setPresentaciones((prev) => [
      ...prev,
      {
        _key: Date.now() + Math.random(),
        producto_id: productoId,
        nombre: "",
        factor: 0,
        precio_costo: undefined,
        codigo_barras: undefined,
        activo: true,
        orden: prev.length,
      },
    ]);
  };

  const eliminar = (key: number) => {
    setPresentaciones((prev) => prev.filter((p) => p._key !== key));
  };

  const update = (key: number, patch: Partial<Editable>) => {
    setPresentaciones((prev) =>
      prev.map((p) => (p._key === key ? { ...p, ...patch } : p)),
    );
  };

  // Buscar productos al tipear (debounced en cada onChange con timeout natural del React)
  const buscar = async (termino: string) => {
    setBusquedaProd(termino);
    if (termino.trim().length < 2) {
      setResultadosBusqueda([]);
      return;
    }
    setBuscando(true);
    try {
      const lista = await buscarProductos(termino.trim());
      setResultadosBusqueda(lista.filter((p) => p.id !== productoId).slice(0, 8));
    } catch {
      setResultadosBusqueda([]);
    } finally {
      setBuscando(false);
    }
  };

  // Aplicar las MISMAS presentaciones (sin sus ids) a otro producto. No pisa las
  // existentes — agrega. Hace upsert silencioso usando el endpoint guardar_presentaciones.
  const aplicarAOtroProducto = async (otroId: number, otroNombre: string) => {
    if (presentaciones.length === 0) {
      toastError("Defini al menos una presentacion para poder duplicar");
      return;
    }
    try {
      // Cargar lo que ese producto ya tenia para no perderlo
      const existentes = await listarPresentacionesProducto(otroId);
      const nombresExistentes = new Set(existentes.map((p) => p.nombre.toLowerCase().trim()));
      const nuevas = presentaciones
        .filter((p) => !nombresExistentes.has(p.nombre.toLowerCase().trim()))
        .map((p) => ({
          ...p,
          id: undefined, // nuevo registro para el otro producto
          producto_id: otroId,
          // codigo_barras es unico por bulto, NO copiar
          codigo_barras: undefined,
        }));
      if (nuevas.length === 0) {
        toastError(`"${otroNombre}" ya tiene todas estas presentaciones`);
        return;
      }
      await guardarPresentacionesProducto(otroId, [...existentes, ...nuevas]);
      toastExito(`${nuevas.length} presentaciones aplicadas a "${otroNombre}"`);
      setAplicarOpen(false);
      setBusquedaProd("");
      setResultadosBusqueda([]);
    } catch (e) {
      toastError("Error: " + String(e));
    }
  };

  const guardar = async () => {
    // Validar
    for (let i = 0; i < presentaciones.length; i++) {
      const p = presentaciones[i];
      if (!p.nombre.trim()) {
        toastError(`Presentacion #${i + 1}: el nombre es obligatorio`);
        return;
      }
      if (!p.factor || p.factor <= 0) {
        toastError(`Presentacion '${p.nombre}': el factor debe ser mayor a 0`);
        return;
      }
    }
    setGuardando(true);
    try {
      const rows = await guardarPresentacionesProducto(
        productoId,
        presentaciones.map(({ _key, ...rest }, i) => ({
          ...rest,
          orden: i,
        })),
      );
      setPresentaciones(rows.map((r, i) => ({ ...r, _key: Date.now() + i })));
      toastExito(`${rows.length} presentaciones guardadas`);
    } catch (e) {
      toastError("Error guardando: " + String(e));
    } finally {
      setGuardando(false);
    }
  };

  return (
    <div className="modal-overlay" onClick={onClose}>
      <div
        className="modal-content"
        onClick={(e) => e.stopPropagation()}
        style={{ maxWidth: 850, width: "92%", maxHeight: "90vh", overflow: "auto" }}
      >
      <div style={{ padding: 20 }}>
        <h3 style={{ margin: 0, fontSize: 17, fontWeight: 700 }}>
          🎁 Presentaciones de compra
        </h3>
        <p
          style={{
            margin: "4px 0 14px 0",
            fontSize: 13,
            color: "var(--color-text-secondary)",
          }}
        >
          Producto: <strong>{productoNombre}</strong> &middot; Unidad base:{" "}
          <code>{unidadBase}</code>
        </p>
        <p
          style={{
            margin: "0 0 14px 0",
            fontSize: 12,
            color: "var(--color-text-secondary)",
            background: "var(--color-surface-alt)",
            padding: "8px 12px",
            borderRadius: 6,
            border: "1px solid var(--color-border)",
          }}
        >
          Si compras este producto en bultos (jaba, six-pack, caja...), defini aqui los
          factores de conversion. Al cargar una compra podras tipear "2 jabas" y el
          sistema entiende 24 {unidadBase.toLowerCase()}. Las ventas siempre se hacen en{" "}
          {unidadBase.toLowerCase()}, no se ven afectadas.
        </p>

        {cargando ? (
          <div style={{ padding: 30, textAlign: "center", color: "#94a3b8" }}>
            Cargando...
          </div>
        ) : (
          <>
            <table className="table" style={{ width: "100%", fontSize: 13 }}>
              <thead>
                <tr>
                  <th style={{ width: "30%" }}>Nombre</th>
                  <th style={{ width: "15%" }}>Factor</th>
                  <th style={{ width: "20%" }}>Precio costo ref.</th>
                  <th style={{ width: "20%" }}>Cod. barras (opcional)</th>
                  <th style={{ width: "8%" }}>Activo</th>
                  <th style={{ width: "7%" }}></th>
                </tr>
              </thead>
              <tbody>
                {presentaciones.length === 0 && (
                  <tr>
                    <td
                      colSpan={6}
                      style={{
                        padding: 20,
                        textAlign: "center",
                        color: "#94a3b8",
                      }}
                    >
                      Sin presentaciones definidas. Click "+ Agregar" para crear.
                    </td>
                  </tr>
                )}
                {presentaciones.map((p) => (
                  <tr key={p._key}>
                    <td>
                      <input
                        className="input"
                        style={{ width: "100%", fontSize: 13 }}
                        placeholder="Jaba x12"
                        list="presentaciones-sugeridas"
                        value={p.nombre}
                        onChange={(e) => {
                          const nombre = e.target.value;
                          // Si coincide con una agrupada ya existente, autollenar el factor
                          const match = sugerencias.find(
                            (s) =>
                              s.nombre.toLowerCase().trim() ===
                              nombre.toLowerCase().trim(),
                          );
                          update(
                            p._key,
                            match && match.factor > 0
                              ? { nombre, factor: match.factor }
                              : { nombre },
                          );
                        }}
                      />
                    </td>
                    <td>
                      <input
                        className="input"
                        style={{ width: "100%", fontSize: 13 }}
                        type="number"
                        min="0.01"
                        step="0.01"
                        placeholder="12"
                        value={p.factor || ""}
                        onChange={(e) =>
                          update(p._key, {
                            factor: Number(e.target.value) || 0,
                          })
                        }
                      />
                    </td>
                    <td>
                      <input
                        className="input"
                        style={{ width: "100%", fontSize: 13 }}
                        type="number"
                        min="0"
                        step="0.01"
                        placeholder="(opcional)"
                        value={p.precio_costo ?? ""}
                        onChange={(e) =>
                          update(p._key, {
                            precio_costo:
                              e.target.value === ""
                                ? undefined
                                : Number(e.target.value),
                          })
                        }
                      />
                    </td>
                    <td>
                      <input
                        className="input"
                        style={{ width: "100%", fontSize: 13 }}
                        placeholder="0780000001234"
                        value={p.codigo_barras ?? ""}
                        onChange={(e) =>
                          update(p._key, {
                            codigo_barras: e.target.value || undefined,
                          })
                        }
                      />
                    </td>
                    <td style={{ textAlign: "center" }}>
                      <input
                        type="checkbox"
                        checked={p.activo}
                        onChange={(e) =>
                          update(p._key, { activo: e.target.checked })
                        }
                      />
                    </td>
                    <td>
                      <button
                        className="btn btn-danger btn-sm"
                        onClick={() => eliminar(p._key)}
                        title="Eliminar"
                      >
                        ✕
                      </button>
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>

            {/* Sugerencias de unidades agrupadas ya existentes (otros productos
                + tipos de unidad agrupados). Elegir una autollena el factor;
                si no existe, el usuario simplemente escribe una nueva. */}
            <datalist id="presentaciones-sugeridas">
              {sugerencias.map((s, i) => (
                <option key={i} value={s.nombre}>
                  {`factor ${s.factor}`}
                </option>
              ))}
            </datalist>

            <div style={{ marginTop: 10, marginBottom: 10 }}>
              <button className="btn btn-outline btn-sm" onClick={agregar}>
                + Agregar presentacion
              </button>
            </div>

            {presentaciones.length > 0 && (
              <div
                style={{
                  fontSize: 12,
                  color: "var(--color-text-secondary)",
                  background: "var(--color-surface-alt)",
                  padding: 10,
                  borderRadius: 6,
                  marginBottom: 14,
                }}
              >
                Ejemplo de uso: si cargas una compra con la presentacion{" "}
                <strong>{presentaciones[0]?.nombre || "Jaba x12"}</strong> y
                tipeas cantidad <strong>2</strong>, el sistema entiende{" "}
                <strong>{2 * (presentaciones[0]?.factor || 12)}</strong>{" "}
                {unidadBase.toLowerCase()} al stock.
              </div>
            )}

            {/* v2.6.26: aplicar estas mismas presentaciones a otro producto */}
            {presentaciones.length > 0 && !aplicarOpen && (
              <div style={{ marginBottom: 12 }}>
                <button
                  type="button"
                  className="btn btn-outline btn-sm"
                  onClick={() => setAplicarOpen(true)}
                  title="Copiar esta misma lista de presentaciones a otro producto"
                >
                  📋 Aplicar las mismas a otro producto
                </button>
              </div>
            )}
            {aplicarOpen && (
              <div
                style={{
                  marginBottom: 14,
                  padding: 12,
                  background: "var(--color-surface-alt)",
                  border: "1px solid var(--color-border)",
                  borderRadius: 8,
                }}
              >
                <div style={{ fontSize: 13, fontWeight: 600, marginBottom: 8 }}>
                  Buscar producto destino
                </div>
                <input
                  className="input"
                  style={{ width: "100%", fontSize: 13, marginBottom: 8 }}
                  placeholder="Codigo, nombre, codigo de barras..."
                  value={busquedaProd}
                  autoFocus
                  onChange={(e) => buscar(e.target.value)}
                />
                {buscando && (
                  <div style={{ fontSize: 12, color: "#94a3b8" }}>Buscando...</div>
                )}
                {!buscando && resultadosBusqueda.length > 0 && (
                  <div
                    style={{
                      maxHeight: 200,
                      overflowY: "auto",
                      background: "white",
                      border: "1px solid var(--color-border)",
                      borderRadius: 6,
                    }}
                  >
                    {resultadosBusqueda.map((p) => (
                      <div
                        key={p.id}
                        onClick={() => aplicarAOtroProducto(p.id, p.nombre)}
                        style={{
                          padding: "8px 12px",
                          cursor: "pointer",
                          borderBottom: "1px solid var(--color-border)",
                          fontSize: 13,
                        }}
                        onMouseEnter={(e) =>
                          ((e.currentTarget as HTMLElement).style.background =
                            "var(--color-surface-hover)")
                        }
                        onMouseLeave={(e) =>
                          ((e.currentTarget as HTMLElement).style.background = "white")
                        }
                      >
                        <strong>{p.nombre}</strong>{" "}
                        <span style={{ color: "#94a3b8", fontSize: 11 }}>
                          ({p.codigo})
                        </span>
                      </div>
                    ))}
                  </div>
                )}
                <div style={{ marginTop: 8, display: "flex", gap: 6, justifyContent: "flex-end" }}>
                  <button
                    className="btn btn-outline btn-sm"
                    onClick={() => {
                      setAplicarOpen(false);
                      setBusquedaProd("");
                      setResultadosBusqueda([]);
                    }}
                  >
                    Cerrar
                  </button>
                </div>
              </div>
            )}

            <div
              style={{
                display: "flex",
                justifyContent: "flex-end",
                gap: 8,
              }}
            >
              <button
                className="btn btn-outline"
                onClick={onClose}
                disabled={guardando}
              >
                Cancelar
              </button>
              <button
                className="btn btn-primary"
                onClick={guardar}
                disabled={guardando}
              >
                {guardando ? "Guardando..." : "Guardar"}
              </button>
            </div>
          </>
        )}
      </div>
      </div>
    </div>
  );
}
