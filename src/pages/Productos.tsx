import { useState, useEffect } from "react";
import { listarProductos, crearProducto, obtenerProducto, actualizarProducto, listarCategorias, exportarInventarioCsv, listarListasPrecios, obtenerPreciosProducto, guardarPreciosProducto, cargarImagenProducto, eliminarImagenProducto, generarEtiquetasPdf } from "../services/api";
import { save, open } from "@tauri-apps/plugin-dialog";
import { useToast } from "../components/Toast";
import type { ProductoBusqueda, Producto, Categoria, ListaPrecio, PrecioProducto } from "../types";

function FormProducto({
  onGuardar,
  onCancelar,
  productoEditar,
  categorias,
  listasPrecios,
}: {
  onGuardar: () => void;
  onCancelar: () => void;
  productoEditar?: Producto;
  categorias: Categoria[];
  listasPrecios: ListaPrecio[];
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
  const [preciosLista, setPreciosLista] = useState<Record<number, string>>({});

  // Cargar precios existentes al editar
  useEffect(() => {
    if (productoEditar?.id) {
      obtenerPreciosProducto(productoEditar.id).then((precios) => {
        const map: Record<number, string> = {};
        precios.forEach((p) => { map[p.lista_precio_id] = p.precio.toString(); });
        setPreciosLista(map);
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
      onGuardar();
    } catch (err) {
      toastError("Error: " + err);
    }
  };

  return (
    <form onSubmit={handleSubmit}>
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
        <div>
          <label className="text-secondary" style={{ fontSize: 12 }}>Precio costo</label>
          <input
            className="input"
            type="number"
            step="0.01"
            value={form.precio_costo}
            onChange={(e) => setForm({ ...form, precio_costo: parseFloat(e.target.value) || 0 })}
          />
        </div>
        <div>
          <label className="text-secondary" style={{ fontSize: 12 }}>Precio venta *</label>
          <input
            className="input"
            type="number"
            step="0.01"
            required
            value={form.precio_venta}
            onChange={(e) => setForm({ ...form, precio_venta: parseFloat(e.target.value) || 0 })}
          />
        </div>
        <div>
          <label className="text-secondary" style={{ fontSize: 12 }}>Stock actual</label>
          <input
            className="input"
            type="number"
            step="0.01"
            value={form.stock_actual}
            onChange={(e) => setForm({ ...form, stock_actual: parseFloat(e.target.value) || 0 })}
          />
        </div>
        <div>
          <label className="text-secondary" style={{ fontSize: 12 }}>Stock mínimo</label>
          <input
            className="input"
            type="number"
            step="0.01"
            value={form.stock_minimo}
            onChange={(e) => setForm({ ...form, stock_minimo: parseFloat(e.target.value) || 0 })}
          />
        </div>
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
        </div>
        <div>
          <label className="text-secondary" style={{ fontSize: 12 }}>Unidad de medida</label>
          <select
            className="input"
            value={form.unidad_medida}
            onChange={(e) => setForm({ ...form, unidad_medida: e.target.value })}
          >
            <option value="UND">Unidad</option>
            <option value="KG">Kilogramo</option>
            <option value="LB">Libra</option>
            <option value="LT">Litro</option>
            <option value="MT">Metro</option>
          </select>
        </div>
      </div>
      {/* Precios por lista */}
      {listasPrecios.length > 0 && (
        <div style={{ marginTop: 16, padding: 12, background: "#f8fafc", borderRadius: "var(--radius)", border: "1px solid var(--color-border)" }}>
          <label className="text-secondary" style={{ fontSize: 12, display: "block", marginBottom: 8, fontWeight: 600 }}>
            Precios por lista
          </label>
          <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: 8 }}>
            {listasPrecios.map((lp) => (
              <div key={lp.id} style={{ display: "flex", alignItems: "center", gap: 8 }}>
                <span style={{ fontSize: 12, flex: 1 }}>
                  {lp.nombre}
                  {lp.es_default && <span style={{ fontSize: 10, color: "#16a34a", marginLeft: 4 }}>(defecto)</span>}
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
      {/* Imagen del producto */}
      <div style={{ marginTop: 16, padding: 12, background: "#f8fafc", borderRadius: "var(--radius)", border: "1px solid var(--color-border)" }}>
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
            <div style={{ width: 80, height: 80, border: "2px dashed var(--color-border)", borderRadius: "var(--radius)", display: "flex", alignItems: "center", justifyContent: "center", color: "#94a3b8", fontSize: 11 }}>
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
                  if (form.id) {
                    const b64 = await cargarImagenProducto(form.id, path as string);
                    setForm({ ...form, imagen: b64 });
                  } else {
                    toastError("Guarde el producto primero, luego agregue la imagen");
                  }
                } catch (err) {
                  toastError("Error: " + err);
                }
              }}>
              {form.imagen ? "Cambiar" : "Cargar imagen"}
            </button>
            {form.imagen && (
              <button type="button" className="btn btn-outline" style={{ fontSize: 11, padding: "4px 10px", color: "#ef4444" }}
                onClick={async () => {
                  try {
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
  const [productos, setProductos] = useState<ProductoBusqueda[]>([]);
  const [categorias, setCategorias] = useState<Categoria[]>([]);
  const [listasPrecios, setListasPrecios] = useState<ListaPrecio[]>([]);
  const [mostrarForm, setMostrarForm] = useState(false);
  const [productoEditar, setProductoEditar] = useState<Producto | undefined>();
  const [filtro, setFiltro] = useState("");
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
    const [prods, cats, listas] = await Promise.all([listarProductos(true), listarCategorias(), listarListasPrecios().catch(() => [])]);
    setProductos(prods);
    setCategorias(cats);
    setListasPrecios(listas);
  };

  useEffect(() => {
    cargarDatos();
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

  const productosFiltrados = filtro
    ? productos.filter(
        (p) =>
          p.nombre.toLowerCase().includes(filtro.toLowerCase()) ||
          (p.codigo && p.codigo.toLowerCase().includes(filtro.toLowerCase()))
      )
    : productos;

  return (
    <>
      <div className="page-header">
        <h2>Productos ({productos.length})</h2>
        <div className="flex gap-2">
          <button className="btn btn-outline" style={{ fontSize: 11, padding: "4px 10px" }}
            onClick={() => setMostrarEtiquetas(true)}>
            Etiquetas
          </button>
          <button className="btn btn-outline" style={{ fontSize: 11, padding: "4px 10px" }}
            onClick={handleExportarCSV}>
            CSV
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
        </div>
      </div>
      <div className="page-body">
        {mostrarForm ? (
          <div className="card">
            <div className="card-header">
              {productoEditar ? "Editar Producto" : "Nuevo Producto"}
            </div>
            <div className="card-body">
              <FormProducto
                productoEditar={productoEditar}
                categorias={categorias}
                listasPrecios={listasPrecios}
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
            <input
              className="input mb-4"
              placeholder="Filtrar productos..."
              value={filtro}
              onChange={(e) => setFiltro(e.target.value)}
            />
            <div className="card">
              <table className="table">
                <thead>
                  <tr>
                    <th>Código</th>
                    <th>Nombre</th>
                    <th>Categoría</th>
                    <th className="text-right">Precio</th>
                    <th className="text-right">Stock</th>
                    <th></th>
                  </tr>
                </thead>
                <tbody>
                  {productosFiltrados.map((p) => (
                    <tr key={p.id}>
                      <td>{p.codigo ?? "-"}</td>
                      <td><strong>{p.nombre}</strong></td>
                      <td className="text-secondary">{p.categoria_nombre ?? "-"}</td>
                      <td className="text-right">${p.precio_venta.toFixed(2)}</td>
                      <td className="text-right">{p.stock_actual}</td>
                      <td>
                        <button className="btn btn-outline" onClick={() => handleEditar(p.id)}>
                          Editar
                        </button>
                      </td>
                    </tr>
                  ))}
                  {productosFiltrados.length === 0 && (
                    <tr>
                      <td colSpan={6} className="text-center text-secondary" style={{ padding: 40 }}>
                        No hay productos registrados
                      </td>
                    </tr>
                  )}
                </tbody>
              </table>
            </div>
          </>
        )}
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
                    borderBottom: "1px solid #f1f5f9", cursor: "pointer", fontSize: 12,
                    background: etiquetaIds.has(p.id) ? "#eff6ff" : undefined,
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
