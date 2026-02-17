import { useState, useEffect } from "react";
import { listarProductos, crearProducto, obtenerProducto, actualizarProducto, listarCategorias, exportarInventarioCsv } from "../services/api";
import { save } from "@tauri-apps/plugin-dialog";
import { useToast } from "../components/Toast";
import type { ProductoBusqueda, Producto, Categoria } from "../types";

function FormProducto({
  onGuardar,
  onCancelar,
  productoEditar,
  categorias,
}: {
  onGuardar: () => void;
  onCancelar: () => void;
  productoEditar?: Producto;
  categorias: Categoria[];
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

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    try {
      if (form.id) {
        await actualizarProducto(form);
      } else {
        await crearProducto(form);
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
  const [mostrarForm, setMostrarForm] = useState(false);
  const [productoEditar, setProductoEditar] = useState<Producto | undefined>();
  const [filtro, setFiltro] = useState("");

  const cargarDatos = async () => {
    const [prods, cats] = await Promise.all([listarProductos(true), listarCategorias()]);
    setProductos(prods);
    setCategorias(cats);
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
    </>
  );
}
