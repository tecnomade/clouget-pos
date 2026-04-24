import { useState, useEffect, useRef, useMemo } from "react";
import NumericInput from "../components/NumericInput";
import { listarProductos, crearProducto, obtenerProducto, actualizarProducto, listarCategorias, crearCategoria, actualizarCategoria, eliminarCategoria, listarTiposUnidad, crearTipoUnidad, actualizarTipoUnidad, eliminarTipoUnidad, exportarInventarioCsv, listarListasPrecios, obtenerPreciosProducto, guardarPreciosProducto, cargarImagenProducto, leerImagenArchivo, eliminarImagenProducto, generarEtiquetasPdf, exportarPlantillaProductos, exportarProductosExcel, importarProductosExcel, eliminarProducto, listarSeriesProducto, registrarSeries, obtenerConfig, listarLotesProducto, registrarLoteCaducidad, eliminarLoteCaducidad, listarUnidadesProducto, guardarUnidadesProducto, listarComboGrupos, listarComboComponentes, guardarComboEstructura, buscarProductos } from "../services/api";
import { save, open } from "@tauri-apps/plugin-dialog";
import { useToast } from "../components/Toast";
import { useSesion } from "../contexts/SesionContext";
import type { ProductoBusqueda, Producto, Categoria, ListaPrecio, PrecioProducto } from "../types";

function FormProducto({
  onGuardar,
  onCancelar,
  productoEditar,
  categorias,
  listasPrecios,
  tiposUnidad,
  puedeVerCostos = true,
}: {
  onGuardar: () => void;
  onCancelar: () => void;
  productoEditar?: Producto;
  categorias: Categoria[];
  listasPrecios: ListaPrecio[];
  tiposUnidad?: Array<{ id: number; nombre: string; abreviatura: string }>;
  puedeVerCostos?: boolean;
}) {
  const { toastError } = useToast();
  const [form, setForm] = useState<Producto>(
    productoEditar ?? {
      nombre: "",
      precio_costo: 0,
      precio_venta: 0,
      iva_porcentaje: 0,
      incluye_iva: false,
      stock_actual: 0,
      stock_minimo: 0,
      unidad_medida: "UND",
      es_servicio: false,
      activo: true,
    }
  );
  const [mostrarInfoIva, setMostrarInfoIva] = useState(false);

  // Multi-unidad de venta (presentaciones)
  type PrecioListaUnidad = { lista_precio_id: number; precio: number };
  type UnidadProd = {
    id?: number;
    tipo_unidad_id?: number | null;
    nombre: string;
    abreviatura?: string;
    factor: number;
    precio: number;
    es_base: boolean;
    precios_lista?: PrecioListaUnidad[];
    _expandido?: boolean;
  };
  const [unidades, setUnidades] = useState<UnidadProd[]>([]);
  const [mostrarInfoUnidades, setMostrarInfoUnidades] = useState(false);
  const [tiposAgrupados, setTiposAgrupados] = useState<any[]>([]);

  // Cargar tipos agrupados del maestro
  useEffect(() => {
    import("../services/api").then(({ listarTiposUnidad }) => {
      listarTiposUnidad().then(ts => setTiposAgrupados(ts.filter((t: any) => t.es_agrupada))).catch(() => {});
    });
  }, []);
  const [preciosLista, setPreciosLista] = useState<Record<number, string>>({});
  const [seriesCount, setSeriesCount] = useState<{ disponible: number; vendido: number; total: number }>({ disponible: 0, vendido: 0, total: 0 });
  const [mostrarRegistrarSeries, setMostrarRegistrarSeries] = useState(false);
  const [seriesTexto, setSeriesTexto] = useState("");
  const [config, setConfig] = useState<Record<string, string>>({});
  const [lotes, setLotes] = useState<any[]>([]);
  const [nuevoLote, setNuevoLote] = useState("");
  const [nuevoLoteFecha, setNuevoLoteFecha] = useState("");
  const [nuevoLoteCantidad, setNuevoLoteCantidad] = useState("");
  const [nuevoLoteFechaElab, setNuevoLoteFechaElab] = useState("");
  // Combos
  const [comboGrupos, setComboGrupos] = useState<any[]>([]);
  const [comboComponentes, setComboComponentes] = useState<any[]>([]);
  const [comboBuscar, setComboBuscar] = useState("");
  const [comboBuscarRes, setComboBuscarRes] = useState<ProductoBusqueda[]>([]);
  const [comboBuscarGrupoId, setComboBuscarGrupoId] = useState<number | null | "raiz">("raiz"); // donde agregar el item
  // Cada grupo en el form usa un id temporal negativo si es nuevo (para asociar componentes antes de tener id real)
  const nextTempIdRef = useRef(-1);

  // Cargar config global (para detectar modulo_caducidad y default incluye_iva)
  useEffect(() => {
    obtenerConfig().then((cfg) => {
      setConfig(cfg);
      // Aplicar default "precio incluye IVA" solo en producto nuevo
      if (!productoEditar && cfg.producto_incluye_iva_default === "1") {
        setForm((prev) => ({ ...prev, incluye_iva: true }));
      }
    }).catch(() => {});
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  const recargarLotes = async () => {
    if (!productoEditar?.id) return;
    try {
      const lst = await listarLotesProducto(productoEditar.id);
      setLotes(lst);
    } catch { /* ignore */ }
  };

  // Cargar precios existentes al editar
  useEffect(() => {
    if (productoEditar?.id) {
      obtenerPreciosProducto(productoEditar.id).then((precios) => {
        const map: Record<number, string> = {};
        precios.forEach((p) => { map[p.lista_precio_id] = p.precio.toString(); });
        setPreciosLista(map);
      }).catch(() => {});
      // Cargar conteo de series si requiere_serie
      if (productoEditar.requiere_serie) {
        listarSeriesProducto(productoEditar.id).then((series) => {
          const disponible = series.filter((s: any) => s.estado === "DISPONIBLE").length;
          const vendido = series.filter((s: any) => s.estado === "VENDIDO").length;
          setSeriesCount({ disponible, vendido, total: series.length });
        }).catch(() => {});
      }
      // Cargar lotes si requiere_caducidad
      if (productoEditar.requiere_caducidad) {
        recargarLotes();
      }
      // Cargar componentes si es combo
      const tp = (productoEditar as any).tipo_producto;
      if (tp === "COMBO_FIJO" || tp === "COMBO_FLEXIBLE") {
        listarComboGrupos(productoEditar.id).then(setComboGrupos).catch(() => {});
        listarComboComponentes(productoEditar.id).then(setComboComponentes).catch(() => {});
      }
      // Cargar unidades / presentaciones del producto (incluye precios por lista)
      listarUnidadesProducto(productoEditar.id).then((us: any[]) => {
        setUnidades(us.map((u: any) => ({
          id: u.id, tipo_unidad_id: u.tipo_unidad_id ?? null,
          nombre: u.nombre, abreviatura: u.abreviatura ?? "",
          factor: u.factor, precio: u.precio, es_base: u.es_base,
          precios_lista: u.precios_lista || [],
        })));
      }).catch(() => {});
    }
  }, [productoEditar?.id]);

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    try {
      let productoId: number;
      if (form.id) {
        await actualizarProducto(form);
        productoId = form.id;
      } else {
        productoId = await crearProducto(form);
      }
      // Guardar precios por lista
      const preciosArr: PrecioProducto[] = [];
      for (const [listaId, precioStr] of Object.entries(preciosLista)) {
        const precio = parseFloat(precioStr);
        if (!isNaN(precio) && precio > 0) {
          preciosArr.push({ lista_precio_id: Number(listaId), producto_id: productoId, precio });
        }
      }
      if (preciosArr.length > 0) {
        await guardarPreciosProducto(productoId, preciosArr);
      }
      // Guardar unidades / presentaciones (solo las con nombre)
      const unidadesValidas = unidades.filter(u => u.nombre.trim() && u.factor > 0);
      if (unidadesValidas.length > 0 || form.id) {
        await guardarUnidadesProducto(productoId, unidadesValidas).catch(() => {});
      }
      // Guardar estructura de combo si aplica
      if (form.tipo_producto === "COMBO_FIJO" || form.tipo_producto === "COMBO_FLEXIBLE") {
        // Resolver producto_padre_id en cada componente y grupo (puede ser nuevo)
        const grpsToSave = comboGrupos.map(g => ({ ...g, producto_padre_id: productoId, id: g.id && g.id > 0 ? g.id : undefined }));
        const compsToSave = comboComponentes.map(c => ({ ...c, producto_padre_id: productoId, id: c.id && c.id > 0 ? c.id : undefined }));
        try {
          await guardarComboEstructura(productoId, grpsToSave as any, compsToSave as any);
        } catch (e) {
          toastError("Error guardando combo: " + e);
        }
      }
      onGuardar();
    } catch (err: any) {
      const errStr = String(err);
      // Manejo amigable del error de código de barras duplicado
      if (errStr.includes("DUPLICATE_BARCODE:")) {
        const partes = errStr.split("DUPLICATE_BARCODE:")[1]?.split(":") ?? [];
        const codigo = partes[0] || "";
        const productos = partes.slice(1).join(":") || "otro producto";
        toastError(`El código de barras "${codigo}" ya está asignado a: ${productos}`);
      } else if (errStr.includes("UNIQUE constraint failed: productos.codigo_barras")) {
        toastError("Ya existe otro producto con ese código de barras (puede estar inactivo). Use un código diferente o desactive el duplicado.");
      } else if (errStr.includes("UNIQUE constraint failed: productos.codigo")) {
        toastError("Ya existe otro producto con ese código. Use un código diferente.");
      } else {
        toastError("Error: " + err);
      }
    }
  };

  return (
    <form onSubmit={handleSubmit}
      onKeyDown={(e) => {
        // Ctrl+S o Ctrl+Enter: guardar
        if ((e.ctrlKey || e.metaKey) && (e.key === "s" || e.key === "S" || e.key === "Enter")) {
          e.preventDefault();
          handleSubmit(e as any);
          return;
        }
        // Prevenir submit accidental con Enter (lector de código de barras, etc.)
        if (e.key === "Enter" && (e.target as HTMLElement).tagName !== "TEXTAREA" && (e.target as HTMLElement).tagName !== "BUTTON") {
          e.preventDefault();
        }
      }}>
      {/* Botones de accion fijos arriba (acceso rapido sin scroll) */}
      <div style={{
        position: "sticky", top: 0, zIndex: 5,
        background: "var(--color-surface)",
        padding: "8px 0", marginBottom: 12,
        borderBottom: "1px solid var(--color-border)",
        display: "flex", justifyContent: "space-between", alignItems: "center", gap: 8,
      }}>
        <div style={{ fontSize: 13, fontWeight: 600, color: "var(--color-text)" }}>
          {form.id ? `Editar Producto: ${form.nombre}` : "Nuevo Producto"}
          <span style={{ fontSize: 10, color: "var(--color-text-secondary)", marginLeft: 8, fontWeight: 400 }}>
            Ctrl+S para guardar
          </span>
        </div>
        <div style={{ display: "flex", gap: 6 }}>
          <button type="button" className="btn btn-outline" onClick={onCancelar} style={{ padding: "6px 14px", fontSize: 12 }}>
            Cancelar
          </button>
          <button type="submit" className="btn btn-primary" style={{ padding: "6px 18px", fontSize: 12, fontWeight: 700 }}>
            {form.id ? "Actualizar" : "Guardar"}
          </button>
        </div>
      </div>
      <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: 16 }}>
        <div>
          <label className="text-secondary" style={{ fontSize: 12 }}>Nombre *</label>
          <input
            className="input"
            required
            value={form.nombre}
            onChange={(e) => setForm({ ...form, nombre: e.target.value })}
          />
        </div>
        <div>
          <label className="text-secondary" style={{ fontSize: 12 }}>Código</label>
          <input
            className="input"
            value={form.codigo ?? ""}
            onChange={(e) => setForm({ ...form, codigo: e.target.value || undefined })}
          />
        </div>
        <div>
          <label className="text-secondary" style={{ fontSize: 12 }}>Código de barras</label>
          <input
            className="input"
            value={form.codigo_barras ?? ""}
            onChange={(e) => setForm({ ...form, codigo_barras: e.target.value || undefined })}
            onKeyDown={(e) => {
              // Evitar que Enter (del lector de código de barras) submita el formulario
              if (e.key === "Enter") e.preventDefault();
            }}
          />
        </div>
        {/* Descripción ocupa ambas columnas (información extra, también busqueda en POS) */}
        <div style={{ gridColumn: "1 / -1" }}>
          <label className="text-secondary" style={{ fontSize: 12 }}>Descripción / información adicional</label>
          <textarea
            className="input"
            rows={2}
            placeholder="Información extra del producto (también usada para búsquedas en el POS si no hay coincidencia por nombre)"
            value={form.descripcion ?? ""}
            onChange={(e) => setForm({ ...form, descripcion: e.target.value || undefined })}
            style={{ resize: "vertical", minHeight: 50 }}
          />
        </div>
        <div>
          <label className="text-secondary" style={{ fontSize: 12 }}>Categoría</label>
          <select
            className="input"
            value={form.categoria_id ?? ""}
            onChange={(e) => setForm({ ...form, categoria_id: e.target.value ? Number(e.target.value) : undefined })}
          >
            <option value="">Sin categoría</option>
            {categorias.map((c) => (
              <option key={c.id} value={c.id}>{c.nombre}</option>
            ))}
          </select>
        </div>
        {puedeVerCostos ? (
          <div>
            <label className="text-secondary" style={{ fontSize: 12 }}>Precio costo</label>
            <NumericInput value={form.precio_costo} step={0.01} min={0}
              onChange={(v) => setForm({ ...form, precio_costo: v })} />
          </div>
        ) : (
          <div>
            <label className="text-secondary" style={{ fontSize: 12, color: "var(--color-text-secondary)" }} title="Sin permiso 'ver_costos'">Precio costo (oculto)</label>
            <input className="input" value="••••" readOnly disabled style={{ fontFamily: "monospace" }} />
          </div>
        )}
        <div>
          <label className="text-secondary" style={{ fontSize: 12 }}>Precio venta *</label>
          <NumericInput value={form.precio_venta} step={0.01} min={0}
            onChange={(v) => setForm({ ...form, precio_venta: v })} />
        </div>
        {!form.es_servicio && (
          <>
            <div>
              <label className="text-secondary" style={{ fontSize: 12 }}>Stock actual</label>
              <NumericInput value={form.stock_actual} step={1} min={0}
                onChange={(v) => setForm({ ...form, stock_actual: v })} />
            </div>
            <div>
              <label className="text-secondary" style={{ fontSize: 12 }}>Stock mínimo</label>
              <NumericInput value={form.stock_minimo} step={1} min={0}
                onChange={(v) => setForm({ ...form, stock_minimo: v })} />
            </div>
          </>
        )}
        <div>
          <label className="text-secondary" style={{ fontSize: 12 }}>IVA %</label>
          <select
            className="input"
            value={form.iva_porcentaje}
            onChange={(e) => setForm({ ...form, iva_porcentaje: parseFloat(e.target.value) })}
          >
            <option value={0}>0% (Sin IVA)</option>
            <option value={5}>5% (IVA reducido)</option>
            <option value={15}>15% (IVA)</option>
          </select>
          {/* Checkbox "Precio incluye IVA" + info */}
          <div style={{ marginTop: 6, display: "flex", alignItems: "center", gap: 6, position: "relative" }}>
            <label style={{ display: "flex", alignItems: "center", gap: 6, fontSize: 12, cursor: "pointer", flex: 1 }}>
              <input
                type="checkbox"
                checked={form.incluye_iva}
                onChange={(e) => setForm({ ...form, incluye_iva: e.target.checked })}
              />
              <span>Precio venta <strong>incluye</strong> IVA</span>
            </label>
            <button
              type="button"
              onClick={() => setMostrarInfoIva(!mostrarInfoIva)}
              title="Como funciona esto?"
              style={{
                background: "var(--color-primary)", color: "#fff", border: "none",
                borderRadius: "50%", width: 18, height: 18, fontSize: 11, fontWeight: 700,
                cursor: "pointer", lineHeight: 1, padding: 0, display: "flex",
                alignItems: "center", justifyContent: "center",
              }}>
              ?
            </button>
            {mostrarInfoIva && (
              <div
                onClick={() => setMostrarInfoIva(false)}
                style={{
                  position: "absolute", top: "100%", right: 0, marginTop: 4,
                  background: "var(--color-surface)", border: "1px solid var(--color-border)",
                  borderRadius: 6, padding: 12, fontSize: 11, lineHeight: 1.5,
                  width: 320, zIndex: 20, boxShadow: "0 4px 16px rgba(0,0,0,0.25)",
                  color: "var(--color-text)",
                }}>
                <div style={{ fontWeight: 700, marginBottom: 6, color: "var(--color-primary)" }}>
                  Como funciona "Precio incluye IVA"?
                </div>
                <div style={{ marginBottom: 6 }}>
                  <strong>Marcado (recomendado):</strong> El "Precio venta" que ingresas YA incluye el IVA.
                  El sistema lo desglosa automaticamente en la venta.
                </div>
                <div style={{ marginBottom: 6, padding: 6, background: "var(--color-surface-alt)", borderRadius: 4 }}>
                  Ejemplo con IVA 15%:<br/>
                  Precio venta: $11.50<br/>
                  → Base: $10.00 + IVA: $1.50 = $11.50
                </div>
                <div style={{ marginBottom: 6 }}>
                  <strong>Desmarcado:</strong> El precio NO incluye IVA. El sistema sumara el IVA encima del precio.
                </div>
                <div style={{ padding: 6, background: "var(--color-surface-alt)", borderRadius: 4 }}>
                  Ejemplo con IVA 15%:<br/>
                  Precio venta: $10.00<br/>
                  → Base: $10.00 + IVA: $1.50 = $11.50 (cobrado)
                </div>
                <div style={{ marginTop: 8, fontSize: 10, color: "var(--color-text-secondary)", textAlign: "center" }}>
                  Click para cerrar
                </div>
              </div>
            )}
          </div>
          {/* Desglose en vivo (solo si tiene IVA) */}
          {form.iva_porcentaje > 0 && form.precio_venta > 0 && (
            <div style={{ marginTop: 4, fontSize: 11, color: "var(--color-text-secondary)" }}>
              {form.incluye_iva ? (
                <>
                  Desglose: Base ${(form.precio_venta / (1 + form.iva_porcentaje / 100)).toFixed(4)} + IVA ${(form.precio_venta - form.precio_venta / (1 + form.iva_porcentaje / 100)).toFixed(4)} = <strong>${form.precio_venta.toFixed(2)}</strong>
                </>
              ) : (
                <>
                  Cliente paga: ${form.precio_venta.toFixed(2)} + IVA ${(form.precio_venta * form.iva_porcentaje / 100).toFixed(4)} = <strong>${(form.precio_venta * (1 + form.iva_porcentaje / 100)).toFixed(2)}</strong>
                </>
              )}
            </div>
          )}
        </div>
        <div>
          <label className="text-secondary" style={{ fontSize: 12 }}>Unidad de medida</label>
          <select
            className="input"
            value={form.unidad_medida}
            onChange={(e) => setForm({ ...form, unidad_medida: e.target.value })}
          >
            {tiposUnidad && tiposUnidad.length > 0 ? (
              tiposUnidad.map((u) => (
                <option key={u.id} value={u.abreviatura}>{u.nombre} ({u.abreviatura})</option>
              ))
            ) : (
              <>
                <option value="UND">Unidad</option>
                <option value="KG">Kilogramo</option>
                <option value="LB">Libra</option>
                <option value="LT">Litro</option>
                <option value="MT">Metro</option>
              </>
            )}
          </select>
        </div>
      </div>
      {/* Precios por lista */}
      {listasPrecios.length > 0 && (
        <div style={{ marginTop: 16, padding: 12, background: "var(--color-surface-alt, rgba(255,255,255,0.03))", borderRadius: "var(--radius)", border: "1px solid var(--color-border)" }}>
          <label className="text-secondary" style={{ fontSize: 12, display: "block", marginBottom: 8, fontWeight: 600 }}>
            Precios por lista
          </label>
          <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: 8 }}>
            {listasPrecios.map((lp) => (
              <div key={lp.id} style={{ display: "flex", alignItems: "center", gap: 8 }}>
                <span style={{ fontSize: 12, flex: 1 }}>
                  {lp.nombre}
                  {lp.es_default && <span style={{ fontSize: 10, color: "var(--color-success)", marginLeft: 4 }}>(defecto)</span>}
                </span>
                <input
                  className="input"
                  type="number"
                  step="0.01"
                  min="0"
                  placeholder={form.precio_venta.toFixed(2)}
                  style={{ width: 110, fontSize: 12 }}
                  value={preciosLista[lp.id!] ?? ""}
                  onChange={(e) => setPreciosLista({ ...preciosLista, [lp.id!]: e.target.value })}
                />
              </div>
            ))}
          </div>
          <span className="text-secondary" style={{ fontSize: 10, marginTop: 6, display: "block" }}>
            Deje vacío para usar el precio base (${form.precio_venta.toFixed(2)})
          </span>
        </div>
      )}

      {/* Presentaciones / Unidades de venta (multi-unidad v1.9.8) */}
      <div style={{ marginTop: 16, padding: 12, background: "var(--color-surface-alt, rgba(255,255,255,0.03))", borderRadius: "var(--radius)", border: "1px solid var(--color-border)", position: "relative" }}>
        <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between", marginBottom: 8 }}>
          <label className="text-secondary" style={{ fontSize: 12, fontWeight: 600, display: "flex", alignItems: "center", gap: 6 }}>
            Presentaciones (otras unidades de venta)
            <button type="button" onClick={() => setMostrarInfoUnidades(!mostrarInfoUnidades)}
              title="Como funciona?"
              style={{
                background: "var(--color-primary)", color: "#fff", border: "none",
                borderRadius: "50%", width: 16, height: 16, fontSize: 10, fontWeight: 700,
                cursor: "pointer", lineHeight: 1, padding: 0, display: "flex",
                alignItems: "center", justifyContent: "center",
              }}>?</button>
            <span style={{ fontSize: 10, color: "var(--color-text-secondary)", fontWeight: 400, marginLeft: 8 }}>
              (opcional - ej: SIXPACK, JABA, CAJA)
            </span>
          </label>
          {/* Dropdown para agregar desde el maestro */}
          <select
            className="input"
            style={{ width: 230, fontSize: 12 }}
            value=""
            onChange={(e) => {
              const tid = parseInt(e.target.value);
              if (!tid) return;
              const tipo = tiposAgrupados.find(t => t.id === tid);
              if (!tipo) return;
              // Evitar duplicados
              if (unidades.some(u => u.tipo_unidad_id === tid)) return;
              setUnidades([...unidades, {
                tipo_unidad_id: tid,
                nombre: tipo.nombre,
                abreviatura: tipo.abreviatura,
                factor: tipo.factor_default,
                precio: form.precio_venta * tipo.factor_default,
                es_base: false,
                precios_lista: [],
                _expandido: true,
              }]);
            }}>
            <option value="">+ Agregar presentacion...</option>
            {tiposAgrupados
              .filter(t => !unidades.some(u => u.tipo_unidad_id === t.id))
              .map(t => (
                <option key={t.id} value={t.id}>
                  {t.nombre} ({t.abreviatura}) × {t.factor_default}
                </option>
              ))}
            <option value="" disabled>──────────</option>
            <option value="-1" disabled>(Crea mas en pestaña Unidades)</option>
          </select>
        </div>
        {mostrarInfoUnidades && (
          <div onClick={() => setMostrarInfoUnidades(false)}
            style={{
              position: "absolute", top: 36, right: 12, marginTop: 4,
              background: "var(--color-surface)", border: "1px solid var(--color-border)",
              borderRadius: 6, padding: 12, fontSize: 11, lineHeight: 1.5,
              width: 360, zIndex: 30, boxShadow: "0 4px 16px rgba(0,0,0,0.25)",
            }}>
            <div style={{ fontWeight: 700, marginBottom: 6, color: "var(--color-primary)" }}>
              Como funciona?
            </div>
            <div style={{ marginBottom: 6 }}>
              1. <strong>Define las unidades agrupadas</strong> en la pestaña <strong>Unidades</strong> (SIXPACK, JABA, CAJA, BLISTER, etc.) con su factor default (cuantas unidades base contiene).
            </div>
            <div style={{ marginBottom: 6 }}>
              2. Aqui en cada producto solo <strong>seleccionas</strong> las presentaciones que vende y le pones el <strong>precio</strong>.
            </div>
            <div style={{ marginBottom: 6, padding: 6, background: "var(--color-surface-alt)", borderRadius: 4 }}>
              <strong>Cerveza Pilsener:</strong><br/>
              UND → $1.50 (precio base del producto)<br/>
              SIXPACK → $8.00 (factor 6, descuenta 6 del stock)<br/>
              JABA → $15.00 (factor 12)
            </div>
            <div style={{ marginBottom: 6 }}>
              Si tienes <strong>listas de precios</strong> (mayorista, etc) puedes definir precios distintos por presentacion para cada lista.
            </div>
            <div style={{ fontSize: 10, marginTop: 8, textAlign: "center", color: "var(--color-text-secondary)" }}>
              Click para cerrar
            </div>
          </div>
        )}
        {unidades.length === 0 ? (
          <div style={{ fontSize: 11, color: "var(--color-text-secondary)", textAlign: "center", padding: 8, fontStyle: "italic" }}>
            Sin presentaciones. Use el menu desplegable para agregar (opcional).
          </div>
        ) : (
          <div style={{ display: "flex", flexDirection: "column", gap: 6 }}>
            {unidades.map((u, idx) => (
              <div key={idx} style={{ background: "var(--color-surface)", border: "1px solid var(--color-border)", borderRadius: 6, overflow: "hidden" }}>
                {/* Cabecera de la presentacion */}
                <div style={{ display: "grid", gridTemplateColumns: "1.5fr 0.6fr 0.6fr 0.8fr 24px 24px", gap: 6, alignItems: "center", padding: "8px 10px" }}>
                  <span style={{ fontWeight: 600, fontSize: 13 }}>
                    {u.nombre} {u.abreviatura && <span style={{ fontSize: 11, color: "var(--color-text-secondary)" }}>({u.abreviatura})</span>}
                  </span>
                  <span style={{ fontSize: 11, color: "var(--color-text-secondary)", textAlign: "center" }}>
                    × {u.factor}
                  </span>
                  <span style={{ fontSize: 10, color: "var(--color-text-secondary)" }}>Precio:</span>
                  <input className="input" type="number" step="0.01" min="0" placeholder="0.00" style={{ fontSize: 12, textAlign: "right" }}
                    value={u.precio} onChange={(e) => {
                      const ar = [...unidades]; ar[idx] = { ...ar[idx], precio: parseFloat(e.target.value) || 0 }; setUnidades(ar);
                    }} />
                  <button type="button" title={u._expandido ? "Cerrar precios por lista" : "Definir precios por lista"}
                    style={{
                      background: u._expandido ? "var(--color-primary)" : "transparent",
                      color: u._expandido ? "#fff" : "var(--color-primary)",
                      border: "1px solid var(--color-primary)",
                      borderRadius: 4, fontSize: 11, padding: 0, width: 24, height: 24, cursor: "pointer", fontWeight: 700,
                    }}
                    onClick={() => {
                      const ar = [...unidades]; ar[idx] = { ...ar[idx], _expandido: !u._expandido }; setUnidades(ar);
                    }}>{u._expandido ? "−" : "≡"}</button>
                  <button type="button" title="Quitar presentacion"
                    style={{ background: "none", border: "none", cursor: "pointer", color: "var(--color-danger)", fontSize: 16, padding: 0 }}
                    onClick={() => setUnidades(unidades.filter((_, i) => i !== idx))}>×</button>
                </div>
                {/* Precios por lista (expandible) */}
                {u._expandido && listasPrecios.length > 0 && (
                  <div style={{ padding: "10px 12px", background: "var(--color-surface-alt)", borderTop: "1px solid var(--color-border)" }}>
                    <div style={{ fontSize: 10, color: "var(--color-text-secondary)", marginBottom: 6 }}>
                      Precios por lista para esta presentacion. Deje vacio para usar el precio default (${u.precio.toFixed(2)}).
                    </div>
                    <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: 6 }}>
                      {listasPrecios.map((lp) => {
                        const pl = (u.precios_lista || []).find(p => p.lista_precio_id === lp.id);
                        return (
                          <div key={lp.id} style={{ display: "flex", alignItems: "center", gap: 6 }}>
                            <span style={{ fontSize: 11, flex: 1 }}>
                              {lp.nombre}
                              {lp.es_default && <span style={{ fontSize: 9, color: "var(--color-success)", marginLeft: 4 }}>(defecto)</span>}
                            </span>
                            <input className="input" type="number" step="0.01" min="0"
                              placeholder={u.precio.toFixed(2)}
                              style={{ width: 90, fontSize: 11, textAlign: "right" }}
                              value={pl?.precio ?? ""}
                              onChange={(e) => {
                                const valor = e.target.value;
                                const ar = [...unidades];
                                let pls = [...(ar[idx].precios_lista || [])];
                                pls = pls.filter(p => p.lista_precio_id !== lp.id);
                                if (valor && parseFloat(valor) > 0) {
                                  pls.push({ lista_precio_id: lp.id!, precio: parseFloat(valor) });
                                }
                                ar[idx] = { ...ar[idx], precios_lista: pls }; setUnidades(ar);
                              }} />
                          </div>
                        );
                      })}
                    </div>
                  </div>
                )}
              </div>
            ))}
          </div>
        )}
      </div>

      {/* Imagen del producto */}
      <div style={{ marginTop: 16, padding: 12, background: "var(--color-surface-alt, rgba(255,255,255,0.03))", borderRadius: "var(--radius)", border: "1px solid var(--color-border)" }}>
        <label className="text-secondary" style={{ fontSize: 12, display: "block", marginBottom: 8, fontWeight: 600 }}>
          Imagen del producto
        </label>
        <div style={{ display: "flex", alignItems: "center", gap: 12 }}>
          {form.imagen ? (
            <img
              src={`data:image/png;base64,${form.imagen}`}
              alt={form.nombre}
              style={{ width: 80, height: 80, objectFit: "cover", border: "1px solid var(--color-border)", borderRadius: "var(--radius)" }}
            />
          ) : (
            <div style={{ width: 80, height: 80, border: "2px dashed var(--color-border)", borderRadius: "var(--radius)", display: "flex", alignItems: "center", justifyContent: "center", color: "var(--color-text-secondary)", fontSize: 11 }}>
              Sin imagen
            </div>
          )}
          <div style={{ display: "flex", flexDirection: "column", gap: 4 }}>
            <button type="button" className="btn btn-outline" style={{ fontSize: 11, padding: "4px 10px" }}
              onClick={async () => {
                try {
                  const path = await open({
                    filters: [{ name: "Imagenes", extensions: ["png", "jpg", "jpeg"] }],
                    multiple: false,
                  });
                  if (!path) return;
                  // Para producto nuevo (sin id): leer en memoria, se persiste al guardar.
                  // Para producto existente: actualiza DB de inmediato.
                  if (form.id) {
                    const b64 = await cargarImagenProducto(form.id, path as string);
                    setForm({ ...form, imagen: b64 });
                  } else {
                    const b64 = await leerImagenArchivo(path as string);
                    setForm({ ...form, imagen: b64 });
                  }
                } catch (err) {
                  toastError("Error: " + err);
                }
              }}>
              {form.imagen ? "Cambiar" : "Cargar imagen"}
            </button>
            {form.imagen && (
              <button type="button" className="btn btn-outline" style={{ fontSize: 11, padding: "4px 10px", color: "var(--color-danger)" }}
                onClick={async () => {
                  try {
                    // Si ya existe en DB, eliminar de DB. Si es nuevo, solo limpiar form.
                    if (form.id) await eliminarImagenProducto(form.id);
                    setForm({ ...form, imagen: undefined });
                  } catch (err) {
                    toastError("Error: " + err);
                  }
                }}>
                Quitar
              </button>
            )}
          </div>
        </div>
        <span className="text-secondary" style={{ fontSize: 10, marginTop: 4, display: "block" }}>
          PNG o JPG, max 500KB. Se muestra en modo tactil del POS.
        </span>
      </div>
      {/* Requiere número de serie */}
      <div style={{ marginTop: 16, padding: 12, background: "var(--color-surface-alt, rgba(255,255,255,0.03))", borderRadius: "var(--radius)", border: "1px solid var(--color-border)" }}>
        <label style={{ display: "flex", alignItems: "center", gap: 6, fontSize: 13, cursor: "pointer" }}>
          <input type="checkbox" checked={form.requiere_serie ?? false}
            onChange={e => setForm({ ...form, requiere_serie: e.target.checked })} />
          Requiere numero de serie
        </label>
        <label style={{ display: "flex", alignItems: "center", gap: 6, fontSize: 13, cursor: "pointer", marginTop: 8 }}>
          <input type="checkbox" checked={form.es_servicio ?? false}
            onChange={(e) => setForm({ ...form, es_servicio: e.target.checked, no_controla_stock: e.target.checked || (form.no_controla_stock ?? false) })} />
          Es un servicio (no se controla stock; SI se incluye en tickets y facturas)
        </label>
        <label style={{ display: "flex", alignItems: "center", gap: 6, fontSize: 13, cursor: form.es_servicio ? "not-allowed" : "pointer", marginTop: 8, opacity: form.es_servicio ? 0.6 : 1 }}>
          <input type="checkbox" checked={form.no_controla_stock ?? false}
            onChange={(e) => setForm({ ...form, no_controla_stock: e.target.checked })}
            disabled={form.es_servicio} />
          No controlar stock (productos a granel, digitales)
        </label>
        {config.modulo_caducidad === "1" && (
          <label style={{ display: "flex", alignItems: "center", gap: 6, fontSize: 13, cursor: "pointer", marginTop: 8 }}>
            <input type="checkbox" checked={form.requiere_caducidad ?? false}
              onChange={(e) => setForm({ ...form, requiere_caducidad: e.target.checked })} />
            Requiere control de caducidad (alimentos, medicinas)
          </label>
        )}
        <div style={{ marginTop: 12, paddingTop: 8, borderTop: "1px dashed var(--color-border)" }}>
          <label style={{ fontSize: 11, color: "var(--color-text-secondary)", fontWeight: 600 }}>Tipo de producto</label>
          <select className="input" style={{ marginTop: 4, fontSize: 13 }}
            value={form.tipo_producto || "SIMPLE"}
            onChange={(e) => setForm({ ...form, tipo_producto: e.target.value })}>
            <option value="SIMPLE">Simple (producto individual)</option>
            <option value="COMBO_FIJO">Combo / Kit fijo (canasta, paquete con componentes definidos)</option>
            <option value="COMBO_FLEXIBLE">Combo flexible (cliente elige: ej. plato + bebida + postre)</option>
          </select>
          <span style={{ fontSize: 10, color: "var(--color-text-secondary)", marginTop: 2, display: "block" }}>
            Los combos descuentan stock de sus componentes al vender. El precio del combo es independiente.
          </span>
        </div>

        {/* Panel de Componentes (visible solo si es combo) */}
        {(form.tipo_producto === "COMBO_FIJO" || form.tipo_producto === "COMBO_FLEXIBLE") && (
          <div style={{ marginTop: 16, padding: 12, background: "rgba(168,85,247,0.06)", border: "1px solid rgba(168,85,247,0.3)", borderRadius: "var(--radius)" }}>
            <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between", marginBottom: 8 }}>
              <div style={{ fontSize: 13, fontWeight: 700 }}>
                {form.tipo_producto === "COMBO_FIJO" ? "🎁 Componentes del Combo" : "🍽 Grupos del Combo Flexible"}
              </div>
              <span style={{ fontSize: 10, color: "var(--color-text-secondary)" }}>
                {form.tipo_producto === "COMBO_FIJO"
                  ? "Productos que forman parte del kit. Se descuentan del stock al vender."
                  : "El cliente elige componentes de cada grupo. Define mín/máx por grupo."}
              </span>
            </div>

            {/* COMBO_FLEXIBLE: lista de grupos */}
            {form.tipo_producto === "COMBO_FLEXIBLE" && (
              <div style={{ marginBottom: 10 }}>
                <button type="button" className="btn btn-outline" style={{ fontSize: 11, marginBottom: 6 }}
                  onClick={() => {
                    const id = nextTempIdRef.current--;
                    setComboGrupos([...comboGrupos, { id, producto_padre_id: form.id || 0, nombre: "Nuevo grupo", minimo: 1, maximo: 1, orden: comboGrupos.length }]);
                  }}>
                  + Agregar grupo
                </button>
                {comboGrupos.length === 0 && (
                  <div style={{ fontSize: 11, color: "var(--color-text-secondary)" }}>
                    Sin grupos. Crea uno (ej: "Plato", "Bebida", "Postre") y luego agrega componentes a cada uno.
                  </div>
                )}
                {comboGrupos.map((g) => {
                  const compsGrupo = comboComponentes.filter(c => c.grupo_id === g.id);
                  return (
                    <div key={g.id} style={{ marginTop: 6, padding: 8, background: "var(--color-surface)", borderRadius: 4, border: "1px solid var(--color-border)" }}>
                      <div style={{ display: "grid", gridTemplateColumns: "1fr 80px 80px auto", gap: 6, alignItems: "center", marginBottom: 6 }}>
                        <input className="input" style={{ fontSize: 12 }} placeholder="Nombre del grupo (ej: Plato)"
                          value={g.nombre}
                          onChange={(e) => setComboGrupos(comboGrupos.map(x => x.id === g.id ? { ...x, nombre: e.target.value } : x))} />
                        <input className="input" type="number" min="0" style={{ fontSize: 12 }} title="Mínimo a escoger" placeholder="Mín"
                          value={g.minimo}
                          onChange={(e) => setComboGrupos(comboGrupos.map(x => x.id === g.id ? { ...x, minimo: parseInt(e.target.value) || 0 } : x))} />
                        <input className="input" type="number" min="1" style={{ fontSize: 12 }} title="Máximo a escoger" placeholder="Máx"
                          value={g.maximo}
                          onChange={(e) => setComboGrupos(comboGrupos.map(x => x.id === g.id ? { ...x, maximo: parseInt(e.target.value) || 1 } : x))} />
                        <button type="button" className="btn btn-outline" style={{ fontSize: 10, padding: "2px 8px", color: "var(--color-danger)" }}
                          onClick={() => {
                            if (!confirm(`Eliminar grupo "${g.nombre}" y sus opciones?`)) return;
                            setComboGrupos(comboGrupos.filter(x => x.id !== g.id));
                            setComboComponentes(comboComponentes.filter(c => c.grupo_id !== g.id));
                          }}>Eliminar grupo</button>
                      </div>
                      {/* Opciones del grupo */}
                      <div style={{ paddingLeft: 16, fontSize: 11 }}>
                        {compsGrupo.length === 0 ? (
                          <div style={{ color: "var(--color-text-secondary)", marginBottom: 4 }}>Sin opciones aún.</div>
                        ) : compsGrupo.map((c, ix) => (
                          <div key={`${c.id ?? c.producto_hijo_id}-${ix}`} style={{ display: "flex", alignItems: "center", gap: 6, padding: "3px 0" }}>
                            <span style={{ flex: 1 }}>{c.hijo_nombre || `Producto #${c.producto_hijo_id}`}</span>
                            <input className="input" type="number" min="0.01" step="any" style={{ width: 70, fontSize: 11 }} title="Cantidad por unidad de combo"
                              value={c.cantidad}
                              onChange={(e) => setComboComponentes(comboComponentes.map(x => x === c ? { ...x, cantidad: parseFloat(e.target.value) || 1 } : x))} />
                            <span style={{ fontSize: 10, color: "var(--color-text-secondary)", width: 60 }}>{c.hijo_unidad_medida || ""}</span>
                            <button type="button" className="btn btn-outline" style={{ fontSize: 10, padding: "1px 6px", color: "var(--color-danger)" }}
                              onClick={() => setComboComponentes(comboComponentes.filter(x => x !== c))}>×</button>
                          </div>
                        ))}
                        <button type="button" className="btn btn-outline" style={{ fontSize: 10, padding: "2px 8px", marginTop: 4 }}
                          onClick={() => setComboBuscarGrupoId(g.id!)}>
                          + Agregar opción a este grupo
                        </button>
                      </div>
                    </div>
                  );
                })}
              </div>
            )}

            {/* COMBO_FIJO: lista plana de componentes */}
            {form.tipo_producto === "COMBO_FIJO" && (
              <div style={{ marginBottom: 10 }}>
                {comboComponentes.length === 0 ? (
                  <div style={{ fontSize: 11, color: "var(--color-text-secondary)", marginBottom: 6 }}>
                    Sin componentes. Agrega al menos uno usando el buscador de abajo.
                  </div>
                ) : (
                  <table style={{ width: "100%", fontSize: 12, borderCollapse: "collapse", marginBottom: 6 }}>
                    <thead>
                      <tr style={{ textAlign: "left" }}>
                        <th style={{ padding: "4px 6px", borderBottom: "1px solid var(--color-border)" }}>Componente</th>
                        <th style={{ padding: "4px 6px", borderBottom: "1px solid var(--color-border)", width: 90 }}>Cantidad</th>
                        <th style={{ padding: "4px 6px", borderBottom: "1px solid var(--color-border)", width: 60 }}>Stock</th>
                        <th style={{ padding: "4px 6px", borderBottom: "1px solid var(--color-border)", width: 40 }}></th>
                      </tr>
                    </thead>
                    <tbody>
                      {comboComponentes.map((c, ix) => (
                        <tr key={`${c.id ?? c.producto_hijo_id}-${ix}`}>
                          <td style={{ padding: "4px 6px" }}>
                            {c.hijo_nombre || `Producto #${c.producto_hijo_id}`}
                            {c.hijo_codigo && <span style={{ fontSize: 10, color: "var(--color-text-secondary)", marginLeft: 4 }}>({c.hijo_codigo})</span>}
                          </td>
                          <td style={{ padding: "4px 6px" }}>
                            <input className="input" type="number" min="0.01" step="any" style={{ width: 70, fontSize: 12 }}
                              value={c.cantidad}
                              onChange={(e) => setComboComponentes(comboComponentes.map((x, i) => i === ix ? { ...x, cantidad: parseFloat(e.target.value) || 1 } : x))} />
                          </td>
                          <td style={{ padding: "4px 6px", fontSize: 11, color: (c.hijo_stock_actual ?? 0) > 0 ? "var(--color-success)" : "var(--color-danger)" }}>
                            {c.hijo_stock_actual ?? 0}
                          </td>
                          <td style={{ padding: "4px 6px" }}>
                            <button type="button" className="btn btn-outline" style={{ fontSize: 10, padding: "1px 6px", color: "var(--color-danger)" }}
                              onClick={() => setComboComponentes(comboComponentes.filter((_, i) => i !== ix))}>×</button>
                          </td>
                        </tr>
                      ))}
                    </tbody>
                  </table>
                )}
                <button type="button" className="btn btn-outline" style={{ fontSize: 11 }}
                  onClick={() => setComboBuscarGrupoId("raiz")}>
                  + Agregar componente
                </button>
              </div>
            )}

            {/* Buscador de productos para agregar componente */}
            {comboBuscarGrupoId !== null && (
              <div style={{ marginTop: 10, padding: 8, background: "var(--color-surface)", borderRadius: 4, border: "1px solid var(--color-border)" }}>
                <div style={{ display: "flex", gap: 6, marginBottom: 4 }}>
                  <input className="input" style={{ flex: 1, fontSize: 12 }}
                    placeholder="Buscar producto por nombre o código..."
                    value={comboBuscar}
                    autoFocus
                    onChange={async (e) => {
                      const q = e.target.value;
                      setComboBuscar(q);
                      if (q.trim().length < 2) { setComboBuscarRes([]); return; }
                      try {
                        const r = await buscarProductos(q.trim());
                        // Excluir el propio producto y los que ya están agregados al mismo nivel
                        const excluidos = new Set(comboComponentes
                          .filter(c => (comboBuscarGrupoId === "raiz" ? c.grupo_id == null : c.grupo_id === comboBuscarGrupoId))
                          .map(c => c.producto_hijo_id));
                        if (form.id) excluidos.add(form.id);
                        setComboBuscarRes(r.filter(p => !excluidos.has(p.id)).slice(0, 10));
                      } catch { setComboBuscarRes([]); }
                    }} />
                  <button type="button" className="btn btn-outline" style={{ fontSize: 11 }}
                    onClick={() => { setComboBuscarGrupoId(null); setComboBuscar(""); setComboBuscarRes([]); }}>
                    Cerrar
                  </button>
                </div>
                {comboBuscarRes.map((p) => (
                  <div key={p.id} style={{ padding: "4px 6px", cursor: "pointer", fontSize: 12, borderBottom: "1px solid var(--color-border)" }}
                    onClick={() => {
                      setComboComponentes([
                        ...comboComponentes,
                        {
                          id: nextTempIdRef.current--,
                          producto_padre_id: form.id || 0,
                          producto_hijo_id: p.id,
                          cantidad: 1,
                          grupo_id: comboBuscarGrupoId === "raiz" ? null : (comboBuscarGrupoId as number),
                          orden: comboComponentes.length,
                          hijo_nombre: p.nombre,
                          hijo_codigo: p.codigo ?? undefined,
                          hijo_precio_venta: p.precio_venta,
                          hijo_stock_actual: p.stock_actual,
                          hijo_unidad_medida: undefined,
                        } as any
                      ]);
                      setComboBuscar("");
                      setComboBuscarRes([]);
                    }}>
                    <strong>{p.nombre}</strong>
                    {p.codigo && <span style={{ marginLeft: 6, color: "var(--color-text-secondary)" }}>({p.codigo})</span>}
                    <span style={{ float: "right", color: "var(--color-text-secondary)", fontSize: 11 }}>
                      Stock: {p.stock_actual} · ${p.precio_venta?.toFixed(2)}
                    </span>
                  </div>
                ))}
              </div>
            )}

            {/* Stats del combo */}
            {comboComponentes.length > 0 && (() => {
              const costoTotal = comboComponentes.reduce((s, c) => s + (c.hijo_precio_costo || 0) * c.cantidad, 0);
              const ventaSugerida = comboComponentes.reduce((s, c) => s + (c.hijo_precio_venta || 0) * c.cantidad, 0);
              const margen = form.precio_venta > 0 ? ((form.precio_venta - costoTotal) / form.precio_venta * 100) : 0;
              return (
                <div style={{ marginTop: 8, padding: 8, background: "rgba(0,0,0,0.04)", borderRadius: 4, fontSize: 11, display: "flex", gap: 14, flexWrap: "wrap" }}>
                  <span>Costo total componentes: <strong>${costoTotal.toFixed(2)}</strong></span>
                  <span>Suma precios venta individuales: <strong>${ventaSugerida.toFixed(2)}</strong></span>
                  <span>Precio combo: <strong>${form.precio_venta.toFixed(2)}</strong></span>
                  <span style={{ color: margen > 0 ? "var(--color-success)" : "var(--color-danger)" }}>
                    Margen: <strong>{margen.toFixed(1)}%</strong>
                  </span>
                </div>
              );
            })()}
          </div>
        )}
        <span className="text-secondary" style={{ fontSize: 10, marginTop: 4, display: "block" }}>
          Si activa, cada unidad necesita un numero de serie unico al vender.
        </span>
        {form.requiere_serie && form.id && (
          <div style={{ marginTop: 10, padding: 10, background: "var(--color-surface)", borderRadius: 6, border: "1px solid var(--color-border)" }}>
            <div style={{ display: "flex", gap: 16, alignItems: "center", fontSize: 12, marginBottom: 8 }}>
              <span>Disponibles: <strong style={{ color: "var(--color-success)" }}>{seriesCount.disponible}</strong></span>
              <span>Vendidos: <strong style={{ color: "var(--color-primary)" }}>{seriesCount.vendido}</strong></span>
              <span>Total: <strong>{seriesCount.total}</strong></span>
              <button type="button" className="btn btn-outline" style={{ fontSize: 11, padding: "2px 10px", marginLeft: "auto" }}
                onClick={() => setMostrarRegistrarSeries(!mostrarRegistrarSeries)}>
                {mostrarRegistrarSeries ? "Cerrar" : "+ Registrar series"}
              </button>
            </div>
            {mostrarRegistrarSeries && (
              <div>
                <textarea
                  className="input"
                  placeholder="Ingrese numeros de serie, uno por linea..."
                  value={seriesTexto}
                  onChange={e => setSeriesTexto(e.target.value)}
                  rows={4}
                  style={{ width: "100%", fontSize: 12, fontFamily: "monospace" }}
                />
                <div style={{ display: "flex", gap: 8, marginTop: 6, alignItems: "center" }}>
                  <button type="button" className="btn btn-primary" style={{ fontSize: 11, padding: "4px 12px" }}
                    onClick={async () => {
                      const serials = seriesTexto.split("\n").map(s => s.trim()).filter(s => s);
                      if (serials.length === 0) return;
                      try {
                        const res = await registrarSeries(form.id!, serials);
                        toastError(`${res.insertados} registrados, ${res.duplicados} duplicados`);
                        setSeriesTexto("");
                        setMostrarRegistrarSeries(false);
                        // Recargar conteo
                        const series = await listarSeriesProducto(form.id!);
                        const disponible = series.filter((s: any) => s.estado === "DISPONIBLE").length;
                        const vendido = series.filter((s: any) => s.estado === "VENDIDO").length;
                        setSeriesCount({ disponible, vendido, total: series.length });
                      } catch (err) { toastError("Error: " + err); }
                    }}>
                    Registrar
                  </button>
                  <span className="text-secondary" style={{ fontSize: 10 }}>
                    {seriesTexto.split("\n").filter(s => s.trim()).length} serie(s) a registrar
                  </span>
                </div>
              </div>
            )}
          </div>
        )}
      </div>

      {/* Lotes de caducidad */}
      {form.requiere_caducidad && form.id && (() => {
        const sumaLotes = lotes.reduce((a: number, l: any) => a + (Number(l.cantidad) || 0), 0);
        const stockActual = Number(form.stock_actual ?? 0);
        const disponible = stockActual - sumaLotes;
        const excede = sumaLotes > stockActual;
        return (
        <div style={{ marginTop: 16, padding: 12, background: "var(--color-surface-alt, rgba(255,255,255,0.03))", borderRadius: "var(--radius)", border: "1px solid var(--color-border)" }}>
          <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between", marginBottom: 8, gap: 12 }}>
            <div style={{ fontSize: 13, fontWeight: 600 }}>Lotes de caducidad</div>
            <div style={{ fontSize: 11, display: "flex", gap: 10 }}>
              <span className="text-secondary">Stock: <strong style={{ color: "var(--color-text)" }}>{stockActual}</strong></span>
              <span className="text-secondary">En lotes: <strong style={{ color: "var(--color-text)" }}>{sumaLotes}</strong></span>
              <span style={{ color: excede ? "var(--color-danger)" : disponible > 0 ? "var(--color-success)" : "var(--color-text-secondary)", fontWeight: 600 }}>
                Disponible: {disponible}
              </span>
            </div>
          </div>
          {excede && (
            <div style={{ padding: "6px 10px", background: "rgba(239,68,68,0.12)", border: "1px solid rgba(239,68,68,0.4)", borderRadius: 4, fontSize: 11, color: "var(--color-danger)", marginBottom: 8 }}>
              ⚠ Los lotes suman {sumaLotes} pero el stock actual es {stockActual}. Hay {sumaLotes - stockActual} unidades de mas en lotes — elimine o ajuste el stock para que coincida.
            </div>
          )}
          {lotes.length > 0 ? (
            <table style={{ width: "100%", fontSize: 12, borderCollapse: "collapse", marginBottom: 8 }}>
              <thead>
                <tr style={{ textAlign: "left" }}>
                  <th style={{ padding: "4px 8px", borderBottom: "1px solid var(--color-border)" }}>Lote</th>
                  <th style={{ padding: "4px 8px", borderBottom: "1px solid var(--color-border)" }}>Elaboracion</th>
                  <th style={{ padding: "4px 8px", borderBottom: "1px solid var(--color-border)" }}>Fecha caducidad</th>
                  <th style={{ padding: "4px 8px", borderBottom: "1px solid var(--color-border)" }}>Cantidad</th>
                  <th style={{ padding: "4px 8px", borderBottom: "1px solid var(--color-border)" }}></th>
                </tr>
              </thead>
              <tbody>
                {lotes.map((l) => (
                  <tr key={l.id}>
                    <td style={{ padding: "4px 8px" }}>{l.lote || "-"}</td>
                    <td style={{ padding: "4px 8px" }}>{l.fecha_elaboracion || "-"}</td>
                    <td style={{ padding: "4px 8px" }}>{l.fecha_caducidad}</td>
                    <td style={{ padding: "4px 8px" }}>{l.cantidad}</td>
                    <td style={{ padding: "4px 8px", textAlign: "right" }}>
                      <button type="button" className="btn btn-outline" style={{ fontSize: 11, padding: "2px 8px", color: "var(--color-danger)" }}
                        onClick={async () => {
                          if (!confirm("Eliminar este lote?")) return;
                          try {
                            await eliminarLoteCaducidad(l.id);
                            await recargarLotes();
                          } catch (err) { toastError("Error: " + err); }
                        }}>
                        Eliminar
                      </button>
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          ) : (
            <div className="text-secondary" style={{ fontSize: 11, marginBottom: 8 }}>No hay lotes registrados.</div>
          )}
          <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr 1fr 1fr auto", gap: 8, alignItems: "end" }}>
            <div>
              <label className="text-secondary" style={{ fontSize: 11 }}>Lote (opcional)</label>
              <input className="input" value={nuevoLote} onChange={(e) => setNuevoLote(e.target.value)} placeholder="LOT-001" style={{ fontSize: 12 }} />
            </div>
            <div>
              <label className="text-secondary" style={{ fontSize: 11 }}>Fecha elaboracion</label>
              <input type="date" className="input" value={nuevoLoteFechaElab} onChange={(e) => setNuevoLoteFechaElab(e.target.value)} style={{ fontSize: 12 }} />
            </div>
            <div>
              <label className="text-secondary" style={{ fontSize: 11 }}>Fecha caducidad *</label>
              <input type="date" className="input" value={nuevoLoteFecha} onChange={(e) => setNuevoLoteFecha(e.target.value)} style={{ fontSize: 12 }} />
            </div>
            <div>
              <label className="text-secondary" style={{ fontSize: 11 }}>Cantidad *</label>
              <input type="number" min="0" step="any" className="input" value={nuevoLoteCantidad} onChange={(e) => setNuevoLoteCantidad(e.target.value)} style={{ fontSize: 12 }} />
            </div>
            <button type="button" className="btn btn-primary" style={{ fontSize: 12 }}
              disabled={disponible <= 0}
              title={disponible <= 0 ? "No queda stock disponible para asignar a un nuevo lote" : ""}
              onClick={async () => {
                if (!nuevoLoteFecha || !nuevoLoteCantidad) {
                  toastError("Fecha y cantidad son requeridas");
                  return;
                }
                const cantNum = parseFloat(nuevoLoteCantidad);
                if (isNaN(cantNum) || cantNum <= 0) {
                  toastError("Cantidad invalida");
                  return;
                }
                if (cantNum > disponible) {
                  toastError(`Cantidad excede lo disponible (${disponible}). Si recibio mas unidades, registre una compra.`);
                  return;
                }
                try {
                  await registrarLoteCaducidad(form.id!, nuevoLote.trim() || null, nuevoLoteFecha, cantNum, undefined, undefined, nuevoLoteFechaElab || undefined);
                  setNuevoLote("");
                  setNuevoLoteFecha("");
                  setNuevoLoteCantidad("");
                  setNuevoLoteFechaElab("");
                  await recargarLotes();
                } catch (err) { toastError("Error: " + err); }
              }}>
              Agregar lote
            </button>
          </div>
        </div>
        );
      })()}

      <div className="flex gap-2 mt-4" style={{ justifyContent: "flex-end" }}>
        <button type="button" className="btn btn-outline" onClick={onCancelar}>
          Cancelar
        </button>
        <button type="submit" className="btn btn-primary">
          {form.id ? "Actualizar" : "Guardar"}
        </button>
      </div>
    </form>
  );
}

export default function Productos() {
  const { toastExito, toastError } = useToast();
  const { esAdmin: esAdminProd, tienePermiso: tienePermisoProd } = useSesion();
  const puedeVerCostos = esAdminProd || tienePermisoProd("ver_costos");
  const fileInputRef = useRef<HTMLInputElement>(null);
  const [importando, setImportando] = useState(false);
  const [productos, setProductos] = useState<ProductoBusqueda[]>([]);
  const [categorias, setCategorias] = useState<Categoria[]>([]);
  const [listasPrecios, setListasPrecios] = useState<ListaPrecio[]>([]);
  const [mostrarForm, setMostrarForm] = useState(false);
  const [productoEditar, setProductoEditar] = useState<Producto | undefined>();
  const [filtro, setFiltro] = useState("");
  const [filtroCategoriaId, setFiltroCategoriaId] = useState<number | null>(null);
  const [ordenamiento, setOrdenamiento] = useState<string>("nombre_asc");
  const [seleccionados, setSeleccionados] = useState<Set<number>>(new Set());
  const [vistaAgrupada, setVistaAgrupada] = useState(false);
  const [categoriasExpandidas, setCategoriasExpandidas] = useState<Set<string>>(new Set());
  const [mostrarEtiquetas, setMostrarEtiquetas] = useState(false);
  const [etiquetaIds, setEtiquetaIds] = useState<Set<number>>(new Set());
  const [etiquetaCantidad, setEtiquetaCantidad] = useState(1);
  const [etiquetaColumnas, setEtiquetaColumnas] = useState(3);
  const [etiquetaPrecio, setEtiquetaPrecio] = useState(true);
  const [etiquetaCodigo, setEtiquetaCodigo] = useState(true);
  const [generandoEtiquetas, setGenerandoEtiquetas] = useState(false);
  const [etiquetaPreset, setEtiquetaPreset] = useState("a4");
  const [etiquetaListaPrecio, setEtiquetaListaPrecio] = useState<number | undefined>();
  const [etiquetaAnchoMm, setEtiquetaAnchoMm] = useState(50);
  const [etiquetaAltoMm, setEtiquetaAltoMm] = useState(25);
  const [etiquetaMargenTop, setEtiquetaMargenTop] = useState(5);
  const [etiquetaMargenLeft, setEtiquetaMargenLeft] = useState(5);
  const [etiquetaBusqueda, setEtiquetaBusqueda] = useState("");

  // Pestañas
  const [tabActiva, setTabActiva] = useState<"productos" | "categorias" | "unidades">("productos");

  // CRUD Categorías
  const [editCatId, setEditCatId] = useState<number | null>(null);
  const [editCatNombre, setEditCatNombre] = useState("");
  const [nuevaCatNombre, setNuevaCatNombre] = useState("");

  // CRUD Tipos de Unidad
  const [tiposUnidad, setTiposUnidad] = useState<any[]>([]);
  const [editUnitId, setEditUnitId] = useState<number | null>(null);
  const [editUnitNombre, setEditUnitNombre] = useState("");
  const [editUnitAbrev, setEditUnitAbrev] = useState("");
  const [editUnitFactor, setEditUnitFactor] = useState<number>(1);
  const [editUnitAgrupada, setEditUnitAgrupada] = useState<boolean>(false);
  const [nuevoUnitNombre, setNuevoUnitNombre] = useState("");
  const [nuevoUnitAbrev, setNuevoUnitAbrev] = useState("");
  const [nuevoUnitFactor, setNuevoUnitFactor] = useState<string>("1");
  const [nuevoUnitAgrupada, setNuevoUnitAgrupada] = useState<boolean>(false);

  const toggleEtiquetaId = (id: number) => {
    setEtiquetaIds(prev => {
      const next = new Set(prev);
      if (next.has(id)) next.delete(id); else next.add(id);
      return next;
    });
  };

  const handleGenerarEtiquetas = async (preview = false) => {
    if (etiquetaIds.size === 0) { toastError("Seleccione al menos un producto"); return; }
    setGenerandoEtiquetas(true);
    try {
      const path = await generarEtiquetasPdf({
        producto_ids: Array.from(etiquetaIds),
        cantidad_por_producto: etiquetaCantidad,
        columnas: etiquetaColumnas,
        mostrar_precio: etiquetaPrecio,
        mostrar_codigo: etiquetaCodigo,
        lista_precio_id: etiquetaListaPrecio,
        preset: etiquetaPreset,
        ancho_mm: etiquetaPreset === "personalizado" ? etiquetaAnchoMm : undefined,
        alto_mm: etiquetaPreset === "personalizado" ? etiquetaAltoMm : undefined,
        margen_top_mm: etiquetaMargenTop,
        margen_left_mm: etiquetaMargenLeft,
      });
      toastExito(preview ? `Vista previa: ${path}` : `Etiquetas generadas: ${path}`);
      if (!preview) setMostrarEtiquetas(false);
    } catch (err) {
      toastError("Error generando etiquetas: " + err);
    } finally {
      setGenerandoEtiquetas(false);
    }
  };

  // Preset change handler - auto-set columns
  const handlePresetChange = (preset: string) => {
    setEtiquetaPreset(preset);
    switch (preset) {
      case "zebra_50x25":
      case "zebra_50x30":
      case "zebra_100x50":
      case "zebra_100x150":
        setEtiquetaColumnas(1);
        break;
      case "avery_65":
        setEtiquetaColumnas(5);
        break;
      case "avery_24":
        setEtiquetaColumnas(3);
        break;
      case "rollo_80":
        setEtiquetaColumnas(2);
        break;
    }
  };

  const cargarDatos = async () => {
    const [prods, cats, listas, units] = await Promise.all([listarProductos(true), listarCategorias(), listarListasPrecios().catch(() => []), listarTiposUnidad().catch(() => [])]);
    setProductos(prods);
    setCategorias(cats);
    setListasPrecios(listas);
    setTiposUnidad(units);
  };

  useEffect(() => {
    cargarDatos();
    // Si la URL tiene ?edit=ID, abrir ese producto automáticamente
    const params = new URLSearchParams(window.location.search);
    const editId = params.get("edit");
    if (editId) {
      const pid = parseInt(editId, 10);
      if (!isNaN(pid)) {
        obtenerProducto(pid).then((prod) => {
          setProductoEditar(prod);
          setMostrarForm(true);
          // Limpiar la URL
          window.history.replaceState({}, "", window.location.pathname);
        }).catch(() => {});
      }
    }
  }, []);

  const handleEditar = async (id: number) => {
    const prod = await obtenerProducto(id);
    setProductoEditar(prod);
    setMostrarForm(true);
  };

  const handleExportarCSV = async () => {
    try {
      const destino = await save({
        defaultPath: "inventario.csv",
        filters: [{ name: "CSV", extensions: ["csv"] }],
      });
      if (!destino) return;
      const msg = await exportarInventarioCsv(destino);
      toastExito(msg);
    } catch (err) {
      toastError("Error al exportar: " + err);
    }
  };

  const descargarExcel = (bytes: number[], nombre: string) => {
    const arr = new Uint8Array(bytes);
    const blob = new Blob([arr], { type: "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet" });
    const url = URL.createObjectURL(blob);
    const a = document.createElement("a");
    a.href = url; a.download = nombre; a.click();
    URL.revokeObjectURL(url);
  };

  const handlePlantilla = async () => {
    try {
      const bytes = await exportarPlantillaProductos();
      descargarExcel(bytes, "plantilla_productos.xlsx");
      toastExito("Plantilla descargada");
    } catch (err) { toastError("Error: " + err); }
  };

  const handleExportar = async () => {
    try {
      const bytes = await exportarProductosExcel();
      descargarExcel(bytes, `productos_${new Date().toISOString().slice(0,10)}.xlsx`);
      toastExito("Productos exportados");
    } catch (err) { toastError("Error: " + err); }
  };

  const handleImport = async (e: React.ChangeEvent<HTMLInputElement>) => {
    const file = e.target.files?.[0];
    if (!file) return;
    setImportando(true);
    try {
      const buffer = await file.arrayBuffer();
      const bytes = Array.from(new Uint8Array(buffer));
      const result = await importarProductosExcel(bytes);
      const lotesMsg = result.lotes_creados ? `, ${result.lotes_creados} lote(s) de caducidad` : "";
      toastExito(`Importación: ${result.creados} creados, ${result.actualizados} actualizados, ${result.errores} errores${lotesMsg}`);
      if (result.mensajes.length > 0) {
        result.mensajes.slice(0, 5).forEach(m => toastError(m));
      }
      if (result.warnings_caducidad && result.warnings_caducidad.length > 0) {
        const nombres = result.warnings_caducidad.slice(0, 5).join(", ");
        const extra = result.warnings_caducidad.length > 5 ? ` y ${result.warnings_caducidad.length - 5} mas` : "";
        toastError(`${result.warnings_caducidad.length} producto(s) con caducidad sin fecha (stock = 0): ${nombres}${extra}. Agregue lotes manualmente.`);
      }
      cargarDatos();
    } catch (err) { toastError("Error importando: " + err); }
    finally { setImportando(false); if (fileInputRef.current) fileInputRef.current.value = ""; }
  };

  const categoriaNombreFiltro = filtroCategoriaId !== null
    ? categorias.find(c => c.id === filtroCategoriaId)?.nombre ?? null
    : null;

  const productosFiltrados = useMemo(() => {
    let lista = productos.filter(p => {
      if (filtro && !p.nombre.toLowerCase().includes(filtro.toLowerCase()) &&
          !(p.codigo && p.codigo.toLowerCase().includes(filtro.toLowerCase()))) return false;
      if (categoriaNombreFiltro !== null && p.categoria_nombre !== categoriaNombreFiltro) return false;
      return true;
    });

    lista.sort((a, b) => {
      switch (ordenamiento) {
        case "nombre_desc": return b.nombre.localeCompare(a.nombre);
        case "precio_asc": return a.precio_venta - b.precio_venta;
        case "precio_desc": return b.precio_venta - a.precio_venta;
        case "stock_asc": return a.stock_actual - b.stock_actual;
        case "stock_desc": return b.stock_actual - a.stock_actual;
        case "recientes": return (b.id || 0) - (a.id || 0);
        default: return a.nombre.localeCompare(b.nombre);
      }
    });

    return lista;
  }, [productos, filtro, categoriaNombreFiltro, ordenamiento]);

  const productosAgrupados = useMemo(() => {
    const grupos: Record<string, typeof productosFiltrados> = {};
    for (const p of productosFiltrados) {
      const cat = p.categoria_nombre || "Sin categoría";
      if (!grupos[cat]) grupos[cat] = [];
      grupos[cat].push(p);
    }
    const sorted = Object.entries(grupos).sort(([a], [b]) => a.localeCompare(b));
    return sorted;
  }, [productosFiltrados]);

  const toggleCategoria = (cat: string) => {
    const s = new Set(categoriasExpandidas);
    if (s.has(cat)) s.delete(cat); else s.add(cat);
    setCategoriasExpandidas(s);
  };

  const expandirTodas = () => {
    setCategoriasExpandidas(new Set(productosAgrupados.map(([cat]) => cat)));
  };

  const contraerTodas = () => {
    setCategoriasExpandidas(new Set());
  };

  return (
    <>
      <div className="page-header">
        <div className="flex gap-2 items-center">
          <h2>Productos</h2>
          <div className="flex gap-1" style={{ marginLeft: 12 }}>
            {(["productos", "categorias", "unidades"] as const).map(tab => (
              <button key={tab} className={`btn ${tabActiva === tab ? "btn-primary" : "btn-outline"}`}
                style={{ fontSize: 11, padding: "4px 12px" }}
                onClick={() => setTabActiva(tab)}>
                {tab === "productos" ? `Lista (${productos.length})` : tab === "categorias" ? "Categorías" : "Unidades"}
              </button>
            ))}
          </div>
        </div>
        <div className="flex gap-2">
          {tabActiva === "productos" && <>
          <button className="btn btn-outline" style={{ fontSize: 11, padding: "4px 10px" }}
            onClick={() => setMostrarEtiquetas(true)}>
            Etiquetas
          </button>
          <button className="btn btn-outline" style={{ fontSize: 11, padding: "4px 10px" }}
            onClick={handlePlantilla}>
            Plantilla Excel
          </button>
          <button className="btn btn-outline" style={{ fontSize: 11, padding: "4px 10px" }}
            onClick={handleExportar}>
            Exportar Excel
          </button>
          <button className="btn btn-primary" style={{ fontSize: 11, padding: "4px 10px" }}
            onClick={() => fileInputRef.current?.click()} disabled={importando}>
            {importando ? "Importando..." : "Importar Excel"}
          </button>
          <input type="file" ref={fileInputRef} accept=".xlsx,.xls" style={{ display: "none" }} onChange={handleImport} />
          <button className="btn btn-outline" style={{ fontSize: 11, padding: "4px 10px" }}
            onClick={handleExportarCSV}>
            CSV Inventario
          </button>
          <button
            className="btn btn-primary"
            data-action="nuevo"
            onClick={() => {
              setProductoEditar(undefined);
              setMostrarForm(true);
            }}
          >
            + Nuevo Producto
          </button>
          </>}
        </div>
      </div>
      <div className="page-body">
        {/* Tab: Categorías */}
        {tabActiva === "categorias" && (
          <div className="card">
            <div className="card-header">Categorías ({categorias.length})</div>
            <div className="card-body">
              <div style={{ display: "flex", gap: 8, marginBottom: 16 }}>
                <input className="input" placeholder="Nueva categoría..." value={nuevaCatNombre}
                  onChange={e => setNuevaCatNombre(e.target.value)}
                  onKeyDown={e => { if (e.key === "Enter" && nuevaCatNombre.trim()) {
                    crearCategoria({ nombre: nuevaCatNombre.trim(), activo: true } as any).then(() => { toastExito("Categoría creada"); setNuevaCatNombre(""); cargarDatos(); }).catch((err: any) => toastError("" + err));
                  }}}
                  style={{ flex: 1 }} />
                <button className="btn btn-primary" disabled={!nuevaCatNombre.trim()}
                  onClick={() => {
                    crearCategoria({ nombre: nuevaCatNombre.trim(), activo: true } as any).then(() => { toastExito("Categoría creada"); setNuevaCatNombre(""); cargarDatos(); }).catch((err: any) => toastError("" + err));
                  }}>+ Agregar</button>
              </div>
              <table className="table">
                <thead><tr><th>Nombre</th><th style={{ width: 120 }}>Acciones</th></tr></thead>
                <tbody>
                  {categorias.map(c => (
                    <tr key={c.id}>
                      <td>
                        {editCatId === c.id ? (
                          <input className="input" value={editCatNombre} onChange={e => setEditCatNombre(e.target.value)}
                            onKeyDown={e => { if (e.key === "Enter") { actualizarCategoria(c.id!, editCatNombre).then(() => { toastExito("Actualizada"); setEditCatId(null); cargarDatos(); }).catch((err: any) => toastError("" + err)); }}}
                            autoFocus style={{ fontSize: 13 }} />
                        ) : c.nombre}
                      </td>
                      <td>
                        <div className="flex gap-1">
                          {editCatId === c.id ? (
                            <>
                              <button className="btn btn-primary" style={{ fontSize: 11, padding: "2px 8px" }}
                                onClick={() => actualizarCategoria(c.id!, editCatNombre).then(() => { toastExito("Actualizada"); setEditCatId(null); cargarDatos(); }).catch((err: any) => toastError("" + err))}>Guardar</button>
                              <button className="btn btn-outline" style={{ fontSize: 11, padding: "2px 8px" }}
                                onClick={() => setEditCatId(null)}>Cancelar</button>
                            </>
                          ) : (
                            <>
                              <button className="btn btn-outline" style={{ fontSize: 11, padding: "2px 8px" }}
                                onClick={() => { setEditCatId(c.id!); setEditCatNombre(c.nombre); }}>Editar</button>
                              <button className="btn btn-outline" style={{ fontSize: 11, padding: "2px 8px", color: "var(--color-danger)" }}
                                onClick={async () => {
                                  try {
                                    const res = await eliminarCategoria(c.id!);
                                    if (res.requiere_accion) {
                                      const opcion = prompt(`Esta categoría tiene ${res.productos} producto(s).\n\nEscriba:\n• "mover" para mover productos a General\n• "eliminar" para eliminar los productos\n• Cancele para no hacer nada`);
                                      if (!opcion) return;
                                      if (opcion.toLowerCase() === "mover") {
                                        await eliminarCategoria(c.id!, "mover");
                                        toastExito("Productos movidos a General, categoría eliminada");
                                      } else if (opcion.toLowerCase() === "eliminar") {
                                        if (confirm(`¿Está seguro de ELIMINAR ${res.productos} producto(s)? Esta acción no se puede deshacer.`)) {
                                          await eliminarCategoria(c.id!, "eliminar_productos");
                                          toastExito("Productos y categoría eliminados");
                                        }
                                      }
                                    } else {
                                      toastExito("Categoría eliminada");
                                    }
                                    cargarDatos();
                                  } catch (err: any) { toastError("" + err); }
                                }}>Eliminar</button>
                            </>
                          )}
                        </div>
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          </div>
        )}

        {/* Tab: Tipos de Unidad */}
        {tabActiva === "unidades" && (
          <div className="card">
            <div className="card-header">Tipos de Unidad ({tiposUnidad.length})</div>
            <div className="card-body">
              <p style={{ fontSize: 12, color: "var(--color-text-secondary)", marginBottom: 12 }}>
                Las unidades base (UND, KG, LT...) son la unidad minima de venta.
                Las unidades <strong>agrupadas</strong> (SIXPACK, JABA, CAJA...) contienen varias unidades base —
                se usa el factor para saber cuantas unidades base descontar del stock. Al editar un producto podras
                asignarle las unidades que vende, con su precio propio.
              </p>
              <div style={{ display: "grid", gridTemplateColumns: "2fr 0.8fr 0.8fr auto auto", gap: 8, marginBottom: 16, alignItems: "center" }}>
                <input className="input" placeholder="Nombre (ej: Sixpack)" value={nuevoUnitNombre}
                  onChange={e => setNuevoUnitNombre(e.target.value)} />
                <input className="input" placeholder="Abrev. (6PK)" value={nuevoUnitAbrev}
                  onChange={e => setNuevoUnitAbrev(e.target.value)} />
                <input className="input" type="number" step="0.01" min="1" placeholder="Factor (6)"
                  value={nuevoUnitFactor} onChange={e => setNuevoUnitFactor(e.target.value)}
                  disabled={!nuevoUnitAgrupada} />
                <label style={{ display: "flex", alignItems: "center", gap: 4, fontSize: 12, cursor: "pointer" }}>
                  <input type="checkbox" checked={nuevoUnitAgrupada}
                    onChange={e => { setNuevoUnitAgrupada(e.target.checked); if (!e.target.checked) setNuevoUnitFactor("1"); }} />
                  Agrupada
                </label>
                <button className="btn btn-primary" disabled={!nuevoUnitNombre.trim() || !nuevoUnitAbrev.trim()}
                  onClick={() => {
                    const factor = nuevoUnitAgrupada ? (parseFloat(nuevoUnitFactor) || 1) : 1;
                    crearTipoUnidad(nuevoUnitNombre.trim(), nuevoUnitAbrev.trim(), factor, nuevoUnitAgrupada)
                      .then(() => {
                        toastExito("Unidad creada");
                        setNuevoUnitNombre(""); setNuevoUnitAbrev(""); setNuevoUnitFactor("1"); setNuevoUnitAgrupada(false);
                        cargarDatos();
                      }).catch((err: any) => toastError("" + err));
                  }}>+ Agregar</button>
              </div>
              <table className="table" style={{ width: "100%" }}>
                <thead><tr>
                  <th>Nombre</th><th>Abreviatura</th><th className="text-right">Factor</th><th>Tipo</th><th style={{ width: 140 }}>Acciones</th>
                </tr></thead>
                <tbody>
                  {tiposUnidad.map((u: any) => (
                    <tr key={u.id} style={{ background: u.es_agrupada ? "rgba(59,130,246,0.05)" : "transparent" }}>
                      <td>
                        {editUnitId === u.id ? (
                          <input className="input" value={editUnitNombre} onChange={e => setEditUnitNombre(e.target.value)} autoFocus style={{ fontSize: 13 }} />
                        ) : <strong>{u.nombre}</strong>}
                      </td>
                      <td>
                        {editUnitId === u.id ? (
                          <input className="input" value={editUnitAbrev} onChange={e => setEditUnitAbrev(e.target.value)} style={{ fontSize: 13, width: 80 }} />
                        ) : u.abreviatura}
                      </td>
                      <td className="text-right">
                        {editUnitId === u.id ? (
                          <input className="input" type="number" step="0.01" min="1" value={editUnitFactor}
                            onChange={e => setEditUnitFactor(parseFloat(e.target.value) || 1)}
                            disabled={!editUnitAgrupada}
                            style={{ fontSize: 13, width: 80, textAlign: "right" }} />
                        ) : (u.es_agrupada ? <strong>×{u.factor_default}</strong> : "—")}
                      </td>
                      <td>
                        {editUnitId === u.id ? (
                          <label style={{ display: "flex", alignItems: "center", gap: 4, fontSize: 12, cursor: "pointer" }}>
                            <input type="checkbox" checked={editUnitAgrupada}
                              onChange={e => { setEditUnitAgrupada(e.target.checked); if (!e.target.checked) setEditUnitFactor(1); }} />
                            Agrupada
                          </label>
                        ) : (u.es_agrupada
                            ? <span style={{ fontSize: 10, padding: "2px 6px", borderRadius: 3, background: "rgba(59,130,246,0.15)", color: "var(--color-primary)", fontWeight: 600 }}>Agrupada</span>
                            : <span style={{ fontSize: 10, padding: "2px 6px", borderRadius: 3, background: "rgba(148,163,184,0.15)", color: "var(--color-text-secondary)" }}>Base</span>
                        )}
                      </td>
                      <td>
                        <div className="flex gap-1">
                          {editUnitId === u.id ? (
                            <>
                              <button className="btn btn-primary" style={{ fontSize: 11, padding: "2px 8px" }}
                                onClick={() => actualizarTipoUnidad(u.id, editUnitNombre, editUnitAbrev, editUnitFactor, editUnitAgrupada).then(() => { toastExito("Actualizado"); setEditUnitId(null); cargarDatos(); }).catch((err: any) => toastError("" + err))}>Guardar</button>
                              <button className="btn btn-outline" style={{ fontSize: 11, padding: "2px 8px" }}
                                onClick={() => setEditUnitId(null)}>Cancelar</button>
                            </>
                          ) : (
                            <>
                              <button className="btn btn-outline" style={{ fontSize: 11, padding: "2px 8px" }}
                                onClick={() => {
                                  setEditUnitId(u.id); setEditUnitNombre(u.nombre); setEditUnitAbrev(u.abreviatura);
                                  setEditUnitFactor(u.factor_default || 1); setEditUnitAgrupada(!!u.es_agrupada);
                                }}>Editar</button>
                              <button className="btn btn-outline" style={{ fontSize: 11, padding: "2px 8px", color: "var(--color-danger)" }}
                                onClick={() => { if (confirm("¿Eliminar tipo de unidad?")) eliminarTipoUnidad(u.id).then(() => { toastExito("Eliminado"); cargarDatos(); }).catch((err: any) => toastError("" + err)); }}>Eliminar</button>
                            </>
                          )}
                        </div>
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          </div>
        )}

        {/* Tab: Productos */}
        {tabActiva === "productos" && (mostrarForm ? (
          <div className="card">
            <div className="card-header">
              {productoEditar ? "Editar Producto" : "Nuevo Producto"}
            </div>
            <div className="card-body">
              <FormProducto
                productoEditar={productoEditar}
                categorias={categorias}
                listasPrecios={listasPrecios}
                tiposUnidad={tiposUnidad}
                puedeVerCostos={puedeVerCostos}
                onGuardar={() => {
                  setMostrarForm(false);
                  cargarDatos();
                }}
                onCancelar={() => setMostrarForm(false)}
              />
            </div>
          </div>
        ) : (
          <>
            <div style={{ display: "flex", gap: 8, marginBottom: 12, flexWrap: "wrap", alignItems: "center" }}>
              <input
                className="input"
                placeholder="Filtrar productos..."
                value={filtro}
                onChange={(e) => setFiltro(e.target.value)}
                style={{ flex: 1, minWidth: 150 }}
              />
              <select className="input" style={{ width: 160, fontSize: 12 }}
                value={filtroCategoriaId ?? ""}
                onChange={(e) => setFiltroCategoriaId(e.target.value ? Number(e.target.value) : null)}>
                <option value="">Todas las categorías</option>
                {categorias.map(c => <option key={c.id} value={c.id}>{c.nombre}</option>)}
              </select>
              <select className="input" style={{ width: 160, fontSize: 12 }}
                value={ordenamiento}
                onChange={(e) => setOrdenamiento(e.target.value)}>
                <option value="nombre_asc">Nombre A-Z</option>
                <option value="nombre_desc">Nombre Z-A</option>
                <option value="precio_asc">Precio menor</option>
                <option value="precio_desc">Precio mayor</option>
                <option value="stock_asc">Menor stock</option>
                <option value="stock_desc">Mayor stock</option>
                <option value="recientes">Más recientes</option>
              </select>
              <button className={`btn ${vistaAgrupada ? "btn-primary" : "btn-outline"}`}
                style={{ fontSize: 11, padding: "4px 12px" }}
                onClick={() => setVistaAgrupada(!vistaAgrupada)}>
                {vistaAgrupada ? "Vista Agrupada" : "Vista Normal"}
              </button>
              <span className="text-secondary" style={{ fontSize: 12 }}>
                {productosFiltrados.length} de {productos.length} producto(s)
              </span>
            </div>
            <div style={{ display: "flex", gap: 8, marginBottom: 8, flexWrap: "wrap", alignItems: "center" }}>
              {seleccionados.size > 0 && (
                <div style={{ display: "flex", gap: 8, alignItems: "center" }}>
                  <span style={{ fontSize: 12, color: "var(--color-text-secondary)" }}>{seleccionados.size} seleccionado(s)</span>
                  <button className="btn btn-danger" style={{ fontSize: 11, padding: "4px 12px" }}
                    onClick={async () => {
                      if (!confirm(`¿Eliminar ${seleccionados.size} producto(s)?`)) return;
                      try {
                        for (const id of seleccionados) { await eliminarProducto(id); }
                        toastExito(`${seleccionados.size} producto(s) eliminado(s)`);
                        setSeleccionados(new Set());
                        cargarDatos();
                      } catch (err) { toastError("Error: " + err); }
                    }}>
                    Eliminar seleccionados
                  </button>
                </div>
              )}
              {filtroCategoriaId !== null && (
                <button className="btn btn-danger" style={{ fontSize: 11, padding: "4px 12px" }}
                  onClick={async () => {
                    const cat = categorias.find(c => c.id === filtroCategoriaId);
                    if (!confirm(`¿Eliminar TODOS los productos de "${cat?.nombre}"? (${productosFiltrados.length} productos)`)) return;
                    try {
                      for (const p of productosFiltrados) { await eliminarProducto(p.id); }
                      toastExito(`${productosFiltrados.length} producto(s) eliminado(s)`);
                      setFiltroCategoriaId(null);
                      cargarDatos();
                    } catch (err) { toastError("Error: " + err); }
                  }}>
                  Eliminar categoría completa
                </button>
              )}
            </div>
            {vistaAgrupada ? (
              <div style={{ flex: 1, overflow: "auto" }}>
                <div style={{ display: "flex", gap: 6, marginBottom: 8 }}>
                  <button className="btn btn-outline" style={{ fontSize: 10, padding: "2px 8px" }} onClick={expandirTodas}>Expandir todas</button>
                  <button className="btn btn-outline" style={{ fontSize: 10, padding: "2px 8px" }} onClick={contraerTodas}>Contraer todas</button>
                </div>
                {productosAgrupados.map(([categoria, prods]) => (
                  <div key={categoria} className="card" style={{ marginBottom: 8 }}>
                    <div
                      style={{
                        display: "flex", justifyContent: "space-between", alignItems: "center",
                        padding: "10px 14px", cursor: "pointer",
                        background: "var(--color-surface-alt)", borderRadius: "var(--radius)",
                        fontWeight: 600, fontSize: 14,
                      }}
                      onClick={() => toggleCategoria(categoria)}
                    >
                      <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
                        <span>{categoriasExpandidas.has(categoria) ? "\u25BC" : "\u25B6"}</span>
                        <span>{categoria}</span>
                        <span style={{ fontSize: 11, color: "var(--color-text-secondary)", fontWeight: 400 }}>
                          ({prods.length} producto{prods.length !== 1 ? "s" : ""})
                        </span>
                      </div>
                      <div style={{ display: "flex", gap: 12, fontSize: 12, color: "var(--color-text-secondary)" }}>
                        <span>Total: ${prods.reduce((s, p) => s + p.stock_actual * p.precio_venta, 0).toFixed(2)}</span>
                        <span>Stock: {prods.reduce((s, p) => s + p.stock_actual, 0)}</span>
                      </div>
                    </div>
                    {categoriasExpandidas.has(categoria) && (
                      <table className="table" style={{ fontSize: 13 }}>
                        <thead>
                          <tr>
                            <th style={{ width: 30 }}>
                              <input type="checkbox"
                                checked={prods.every(p => seleccionados.has(p.id))}
                                onChange={(e) => {
                                  const s = new Set(seleccionados);
                                  prods.forEach(p => { if (e.target.checked) s.add(p.id); else s.delete(p.id); });
                                  setSeleccionados(s);
                                }} />
                            </th>
                            <th>CODIGO</th>
                            <th>NOMBRE</th>
                            {puedeVerCostos && <th className="text-right">COSTO</th>}
                            <th className="text-right">PRECIO</th>
                            {puedeVerCostos && <th className="text-right">MARGEN</th>}
                            <th className="text-right">STOCK</th>
                            <th style={{ width: 80 }}></th>
                          </tr>
                        </thead>
                        <tbody>
                          {prods.map(p => {
                            const costo = (p as any).precio_costo ?? 0;
                            const margen = p.precio_venta > 0 && costo > 0
                              ? ((p.precio_venta - costo) / p.precio_venta * 100)
                              : null;
                            return (
                            <tr key={p.id}>
                              <td>
                                <input type="checkbox" checked={seleccionados.has(p.id)}
                                  onChange={(e) => {
                                    const s = new Set(seleccionados);
                                    if (e.target.checked) s.add(p.id); else s.delete(p.id);
                                    setSeleccionados(s);
                                  }} />
                              </td>
                              <td style={{ fontSize: 11, color: "var(--color-text-secondary)" }}>{p.codigo || "-"}</td>
                              <td><strong>{p.nombre}</strong></td>
                              {puedeVerCostos && (
                                <td className="text-right" style={{ color: "var(--color-text-secondary)", fontSize: 12 }}>
                                  {costo > 0 ? `$${costo.toFixed(2)}` : "-"}
                                </td>
                              )}
                              <td className="text-right">${p.precio_venta.toFixed(2)}</td>
                              {puedeVerCostos && (
                                <td className="text-right" style={{
                                  fontSize: 11, fontWeight: 600,
                                  color: margen == null ? "var(--color-text-secondary)" : margen < 0 ? "var(--color-danger)" : margen < 15 ? "var(--color-warning)" : "var(--color-success)",
                                }}>
                                  {margen == null ? "-" : `${margen.toFixed(1)}%`}
                                </td>
                              )}
                              <td className="text-right" style={{
                                color: p.stock_actual <= 0 ? "var(--color-danger)" : p.stock_actual <= p.stock_minimo ? "var(--color-warning)" : undefined,
                                fontWeight: p.stock_actual <= p.stock_minimo ? 600 : undefined
                              }}>
                                {p.stock_actual}
                              </td>
                              <td>
                                <div style={{ display: "flex", gap: 4 }}>
                                  <button className="btn btn-outline" style={{ padding: "2px 8px", fontSize: 11 }}
                                    onClick={() => handleEditar(p.id)}>Editar</button>
                                  <button className="btn btn-danger" style={{ padding: "2px 6px", fontSize: 11 }}
                                    onClick={async () => {
                                      if (!confirm(`\u00bfEliminar "${p.nombre}"?`)) return;
                                      try {
                                        await eliminarProducto(p.id);
                                        toastExito("Eliminado");
                                        cargarDatos();
                                      } catch (err) { toastError("Error: " + err); }
                                    }}>x</button>
                                </div>
                              </td>
                            </tr>
                            );
                          })}
                        </tbody>
                      </table>
                    )}
                  </div>
                ))}
              </div>
            ) : (
              <div className="card">
                <table className="table">
                  <thead>
                    <tr>
                      <th style={{ width: 30 }}>
                        <input type="checkbox"
                          checked={seleccionados.size === productosFiltrados.length && productosFiltrados.length > 0}
                          onChange={(e) => {
                            if (e.target.checked) setSeleccionados(new Set(productosFiltrados.map(p => p.id)));
                            else setSeleccionados(new Set());
                          }} />
                      </th>
                      <th>Código</th>
                      <th>Nombre</th>
                      <th>Categoría</th>
                      {puedeVerCostos && <th className="text-right">Costo</th>}
                      <th className="text-right">Precio</th>
                      {puedeVerCostos && <th className="text-right">Margen</th>}
                      <th className="text-right">Stock</th>
                      <th></th>
                    </tr>
                  </thead>
                  <tbody>
                    {productosFiltrados.map((p) => {
                      const costo = (p as any).precio_costo ?? 0;
                      const margen = p.precio_venta > 0 && costo > 0
                        ? ((p.precio_venta - costo) / p.precio_venta * 100)
                        : null;
                      return (
                      <tr key={p.id}>
                        <td>
                          <input type="checkbox" checked={seleccionados.has(p.id)}
                            onChange={(e) => {
                              const s = new Set(seleccionados);
                              if (e.target.checked) s.add(p.id); else s.delete(p.id);
                              setSeleccionados(s);
                            }} />
                        </td>
                        <td>{p.codigo ?? "-"}</td>
                        <td><strong>{p.nombre}</strong></td>
                        <td className="text-secondary">{p.categoria_nombre ?? "-"}</td>
                        {puedeVerCostos && (
                          <td className="text-right" style={{ color: "var(--color-text-secondary)", fontSize: 12 }}>
                            {costo > 0 ? `$${costo.toFixed(2)}` : "-"}
                          </td>
                        )}
                        <td className="text-right">${p.precio_venta.toFixed(2)}</td>
                        {puedeVerCostos && (
                          <td className="text-right" style={{
                            fontSize: 11, fontWeight: 600,
                            color: margen == null ? "var(--color-text-secondary)" : margen < 0 ? "var(--color-danger)" : margen < 15 ? "var(--color-warning)" : "var(--color-success)",
                          }}>
                            {margen == null ? "-" : `${margen.toFixed(1)}%`}
                          </td>
                        )}
                        <td className="text-right">{p.stock_actual}</td>
                        <td>
                          <div className="flex gap-1">
                            <button className="btn btn-outline" onClick={() => handleEditar(p.id)}>
                              Editar
                            </button>
                            <button className="btn btn-danger" style={{ padding: "2px 8px", fontSize: 11 }}
                              onClick={async () => {
                                if (!confirm(`\u00bfEliminar "${p.nombre}"?`)) return;
                                try {
                                  await eliminarProducto(p.id);
                                  toastExito("Producto eliminado");
                                  cargarDatos();
                                } catch (err) { toastError("Error: " + err); }
                              }}>x</button>
                          </div>
                        </td>
                      </tr>
                      );
                    })}
                    {productosFiltrados.length === 0 && (
                      <tr>
                        <td colSpan={puedeVerCostos ? 9 : 7} className="text-center text-secondary" style={{ padding: 40 }}>
                          No hay productos registrados
                        </td>
                      </tr>
                    )}
                  </tbody>
                </table>
              </div>
            )}
          </>
        ))}
      </div>
      {/* Modal Etiquetas */}
      {mostrarEtiquetas && (() => {
        const isZebra = etiquetaPreset.startsWith("zebra_");
        const isAvery = etiquetaPreset.startsWith("avery_");
        const colsDisabled = isZebra || isAvery;
        const busquedaLower = etiquetaBusqueda.toLowerCase();
        const productosEtiqueta = productosFiltrados.filter(p =>
          !etiquetaBusqueda || p.nombre.toLowerCase().includes(busquedaLower)
          || (p.codigo && p.codigo.toLowerCase().includes(busquedaLower))
        );
        return (
        <div style={{ position: "fixed", inset: 0, background: "rgba(0,0,0,0.5)", display: "flex", alignItems: "center", justifyContent: "center", zIndex: 100 }}
          onClick={(e) => { if (e.target === e.currentTarget) setMostrarEtiquetas(false); }}>
          <div className="card" style={{ width: 650, maxHeight: "85vh", overflow: "auto" }}>
            <div className="card-header">
              <span>Generar Etiquetas con Codigo de Barras</span>
              <button className="btn btn-outline" style={{ padding: "2px 8px" }} onClick={() => setMostrarEtiquetas(false)}>x</button>
            </div>
            <div className="card-body">
              {/* Fila 1: Preset + Columnas + Cantidad */}
              <div style={{ display: "flex", gap: 10, marginBottom: 10, flexWrap: "wrap" }}>
                <div>
                  <label className="text-secondary" style={{ fontSize: 11 }}>Tipo de papel</label>
                  <select className="input" value={etiquetaPreset} onChange={(e) => handlePresetChange(e.target.value)}
                    style={{ width: 170 }}>
                    <option value="a4">A4 (210x297mm)</option>
                    <optgroup label="Zebra / Rollo">
                      <option value="zebra_50x25">Zebra 50x25mm</option>
                      <option value="zebra_50x30">Zebra 50x30mm</option>
                      <option value="zebra_100x50">Zebra 100x50mm</option>
                      <option value="zebra_100x150">Zebra 100x150mm</option>
                      <option value="rollo_80">Rollo 80mm</option>
                    </optgroup>
                    <optgroup label="A4 Adhesivo">
                      <option value="avery_65">Avery 65 (38x21mm)</option>
                      <option value="avery_24">Avery 24 (64x34mm)</option>
                    </optgroup>
                    <option value="personalizado">Personalizado</option>
                  </select>
                </div>
                <div>
                  <label className="text-secondary" style={{ fontSize: 11 }}>Columnas</label>
                  <select className="input" value={etiquetaColumnas} onChange={(e) => setEtiquetaColumnas(parseInt(e.target.value))}
                    style={{ width: 70 }} disabled={colsDisabled}>
                    {[1,2,3,4,5,6].map(n => <option key={n} value={n}>{n}</option>)}
                  </select>
                </div>
                <div>
                  <label className="text-secondary" style={{ fontSize: 11 }}>Cantidad c/u</label>
                  <input className="input" type="number" min={1} max={100} value={etiquetaCantidad}
                    onChange={(e) => setEtiquetaCantidad(parseInt(e.target.value) || 1)}
                    style={{ width: 70 }} />
                </div>
              </div>

              {/* Fila 2: Custom size + margins + lista precios */}
              <div style={{ display: "flex", gap: 10, marginBottom: 10, flexWrap: "wrap", alignItems: "flex-end" }}>
                {etiquetaPreset === "personalizado" && (
                  <>
                    <div>
                      <label className="text-secondary" style={{ fontSize: 11 }}>Ancho (mm)</label>
                      <input className="input" type="number" min={10} max={500} value={etiquetaAnchoMm}
                        onChange={(e) => setEtiquetaAnchoMm(parseInt(e.target.value) || 50)}
                        style={{ width: 70 }} />
                    </div>
                    <div>
                      <label className="text-secondary" style={{ fontSize: 11 }}>Alto (mm)</label>
                      <input className="input" type="number" min={10} max={500} value={etiquetaAltoMm}
                        onChange={(e) => setEtiquetaAltoMm(parseInt(e.target.value) || 25)}
                        style={{ width: 70 }} />
                    </div>
                  </>
                )}
                <div>
                  <label className="text-secondary" style={{ fontSize: 11 }}>Margen sup. (mm)</label>
                  <input className="input" type="number" min={0} max={30} value={etiquetaMargenTop}
                    onChange={(e) => setEtiquetaMargenTop(parseInt(e.target.value) || 0)}
                    style={{ width: 60 }} />
                </div>
                <div>
                  <label className="text-secondary" style={{ fontSize: 11 }}>Margen izq. (mm)</label>
                  <input className="input" type="number" min={0} max={30} value={etiquetaMargenLeft}
                    onChange={(e) => setEtiquetaMargenLeft(parseInt(e.target.value) || 0)}
                    style={{ width: 60 }} />
                </div>
                {listasPrecios.length > 0 && (
                  <div>
                    <label className="text-secondary" style={{ fontSize: 11 }}>Lista de precios</label>
                    <select className="input" value={etiquetaListaPrecio ?? ""} onChange={(e) => setEtiquetaListaPrecio(e.target.value ? parseInt(e.target.value) : undefined)}
                      style={{ width: 150 }}>
                      <option value="">Precio de venta</option>
                      {listasPrecios.map(lp => <option key={lp.id} value={lp.id}>{lp.nombre}</option>)}
                    </select>
                  </div>
                )}
              </div>

              {/* Fila 3: Checkboxes */}
              <div style={{ display: "flex", gap: 16, marginBottom: 12 }}>
                <label style={{ display: "flex", alignItems: "center", gap: 4, fontSize: 12, cursor: "pointer" }}>
                  <input type="checkbox" checked={etiquetaPrecio} onChange={(e) => setEtiquetaPrecio(e.target.checked)} />
                  Mostrar precio
                </label>
                <label style={{ display: "flex", alignItems: "center", gap: 4, fontSize: 12, cursor: "pointer" }}>
                  <input type="checkbox" checked={etiquetaCodigo} onChange={(e) => setEtiquetaCodigo(e.target.checked)} />
                  Mostrar codigo
                </label>
              </div>

              {/* Busqueda + Seleccionar todos */}
              <div style={{ display: "flex", gap: 8, marginBottom: 8, alignItems: "center" }}>
                <input className="input" placeholder="Buscar producto..." value={etiquetaBusqueda}
                  onChange={(e) => setEtiquetaBusqueda(e.target.value)}
                  style={{ flex: 1, fontSize: 12 }} />
                <button className="btn btn-outline btn-sm" onClick={() => setEtiquetaIds(new Set(productosEtiqueta.map(p => p.id)))}>
                  Todos
                </button>
                <button className="btn btn-outline btn-sm" onClick={() => setEtiquetaIds(new Set())}>
                  Ninguno
                </button>
                <span className="text-secondary" style={{ fontSize: 12 }}>
                  {etiquetaIds.size} sel.
                </span>
              </div>

              {/* Lista de productos */}
              <div style={{ maxHeight: 250, overflowY: "auto", border: "1px solid var(--color-border)", borderRadius: 8 }}>
                {productosEtiqueta.map((p) => (
                  <label key={p.id} style={{
                    display: "flex", alignItems: "center", gap: 8, padding: "5px 10px",
                    borderBottom: "1px solid var(--color-border)", cursor: "pointer", fontSize: 12,
                    background: etiquetaIds.has(p.id) ? "rgba(59, 130, 246, 0.1)" : undefined,
                  }}>
                    <input type="checkbox" checked={etiquetaIds.has(p.id)}
                      onChange={() => toggleEtiquetaId(p.id)} />
                    <span style={{ flex: 1 }}>{p.nombre}</span>
                    <span className="text-secondary" style={{ fontSize: 10 }}>{p.codigo || ""}</span>
                    <span style={{ fontWeight: 600 }}>${p.precio_venta.toFixed(2)}</span>
                  </label>
                ))}
                {productosEtiqueta.length === 0 && (
                  <div className="text-center text-secondary" style={{ padding: 20, fontSize: 12 }}>
                    No se encontraron productos
                  </div>
                )}
              </div>

              {/* Botones */}
              <div style={{ display: "flex", gap: 8, marginTop: 12 }}>
                <button className="btn btn-outline" style={{ flex: 1 }}
                  disabled={etiquetaIds.size === 0 || generandoEtiquetas}
                  onClick={() => handleGenerarEtiquetas(true)}>
                  Vista Previa
                </button>
                <button className="btn btn-primary" style={{ flex: 1 }}
                  disabled={etiquetaIds.size === 0 || generandoEtiquetas}
                  onClick={() => handleGenerarEtiquetas(false)}>
                  {generandoEtiquetas ? "Generando..." : `Generar PDF (${etiquetaIds.size} × ${etiquetaCantidad})`}
                </button>
              </div>
            </div>
          </div>
        </div>
        );
      })()}
    </>
  );
}
