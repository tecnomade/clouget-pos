import { useState, useEffect, useRef } from "react";
import {
  listarClientes, crearCliente, actualizarCliente, eliminarCliente, listarListasPrecios, consultarIdentificacion,
  // v2.5.39
  listarCategoriasClientes, crearCategoriaCliente, actualizarCategoriaCliente, eliminarCategoriaCliente,
  exportarPlantillaClientes, exportarClientesExcel, importarClientesExcel,
} from "../services/api";
import type { CategoriaCliente } from "../services/api";
import { useToast } from "../components/Toast";
import { useTabActivated } from "../contexts/TabsContext";
import { useSesion } from "../contexts/SesionContext";
import type { Cliente, ListaPrecio } from "../types";

export default function Clientes() {
  const { toastExito, toastError } = useToast();
  const { esAdmin, tienePermiso } = useSesion();
  // Solo admin o usuarios con 'eliminar_clientes' ven los controles de borrado.
  const puedeEliminar = esAdmin || tienePermiso("eliminar_clientes");
  const [clientes, setClientes] = useState<Cliente[]>([]);
  const [seleccionados, setSeleccionados] = useState<Set<number>>(new Set());
  const [mostrarForm, setMostrarForm] = useState(false);
  const [editando, setEditando] = useState<Cliente | undefined>();
  const [listasPrecios, setListasPrecios] = useState<ListaPrecio[]>([]);
  const [form, setForm] = useState<Cliente>({
    tipo_identificacion: "CEDULA",
    nombre: "",
    activo: true,
  });
  const [consultandoSri, setConsultandoSri] = useState(false);
  const [busqueda, setBusqueda] = useState("");
  // v2.5.39: categorias + import/export
  const [categorias, setCategorias] = useState<CategoriaCliente[]>([]);
  const [tab, setTab] = useState<"clientes" | "categorias">("clientes");
  const [editandoCat, setEditandoCat] = useState<CategoriaCliente | null>(null);
  const [formCat, setFormCat] = useState<CategoriaCliente>({
    nombre: "", descripcion: "", permite_credito: false, dias_credito: 0,
    limite_credito: 0, descuento_pct: 0, requiere_ruc: false, es_default: false, activo: true,
  });
  const fileRef = useRef<HTMLInputElement>(null);

  const cargar = async () => {
    const [cls, listas, cats] = await Promise.all([
      listarClientes(),
      listarListasPrecios().catch(() => []),
      listarCategoriasClientes().catch(() => []),
    ]);
    setClientes(cls);
    setListasPrecios(listas);
    setCategorias(cats);
  };

  useEffect(() => { cargar(); }, []);

  // v2.5.3: refrescar lista al volver a esta pestaña (un cliente pudo crearse
  // desde POS o editarse desde otra tab).
  useTabActivated("/clientes", () => { cargar(); });

  const clientesFiltrados = clientes.filter((c) => {
    if (!busqueda.trim()) return true;
    const q = busqueda.toLowerCase();
    return (c.nombre?.toLowerCase().includes(q)) ||
      (c.identificacion?.toLowerCase().includes(q)) ||
      (c.email?.toLowerCase().includes(q)) ||
      (c.telefono?.toLowerCase().includes(q));
  });

  const manejarErrorEliminar = (nombre: string, err: unknown) => {
    const msg = String((err as Error)?.message || err);
    const m = msg.match(/BLOCK_DELETE_CREDITO:([\d.]+)/);
    if (m) {
      toastError(`"${nombre}" tiene crédito pendiente por $${m[1]}. Cobra o anula sus cuentas antes de eliminarlo.`);
    } else {
      toastError(`No se pudo eliminar "${nombre}": ${msg}`);
    }
  };

  const eliminarUno = async (c: Cliente) => {
    if (!confirm(`¿Eliminar al cliente "${c.nombre}"?\n\nSi tiene historial de ventas se desactiva (el historial no se pierde).`)) return;
    try {
      await eliminarCliente(c.id!);
      toastExito(`Cliente "${c.nombre}" eliminado`);
      setSeleccionados(prev => { const s = new Set(prev); s.delete(c.id!); return s; });
      cargar();
    } catch (err) {
      manejarErrorEliminar(c.nombre, err);
    }
  };

  const eliminarVarios = async () => {
    if (!confirm(`¿Eliminar ${seleccionados.size} cliente(s)?\n\nLos que tengan historial de ventas se desactivan (el historial no se pierde). Los que tengan crédito pendiente NO se eliminan.`)) return;
    let ok = 0;
    for (const id of seleccionados) {
      const c = clientes.find(x => x.id === id);
      try {
        await eliminarCliente(id);
        ok++;
      } catch (err) {
        manejarErrorEliminar(c?.nombre || `#${id}`, err);
      }
    }
    if (ok > 0) toastExito(`${ok} cliente(s) eliminado(s)`);
    setSeleccionados(new Set());
    cargar();
  };

  const abrirNuevo = () => {
    setEditando(undefined);
    setForm({ tipo_identificacion: "CEDULA", nombre: "", activo: true });
    setMostrarForm(true);
  };

  const abrirEditar = (c: Cliente) => {
    setEditando(c);
    setForm(c);
    setMostrarForm(true);
  };

  const guardar = async (e: React.FormEvent) => {
    e.preventDefault();
    try {
      if (editando?.id) {
        await actualizarCliente(form);
      } else {
        await crearCliente(form);
      }
      setMostrarForm(false);
      cargar();
      toastExito(editando?.id ? "Cliente actualizado" : "Cliente creado");
    } catch (err) {
      toastError("Error: " + err);
    }
  };

  // v2.5.39: handlers de import/export + descarga binaria como Productos
  const descargarExcel = (bytes: number[], nombre: string) => {
    const blob = new Blob([new Uint8Array(bytes)], {
      type: "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
    });
    const url = URL.createObjectURL(blob);
    const a = document.createElement("a");
    a.href = url;
    a.download = nombre;
    a.click();
    URL.revokeObjectURL(url);
  };
  const handlePlantilla = async () => {
    try {
      const bytes = await exportarPlantillaClientes();
      descargarExcel(bytes, "plantilla_clientes.xlsx");
      toastExito("Plantilla descargada");
    } catch (err) { toastError("Error: " + err); }
  };
  const handleExportar = async () => {
    try {
      const bytes = await exportarClientesExcel();
      descargarExcel(bytes, `clientes_${new Date().toISOString().slice(0, 10)}.xlsx`);
      toastExito("Clientes exportados");
    } catch (err) { toastError("Error: " + err); }
  };
  const handleImport = async (e: React.ChangeEvent<HTMLInputElement>) => {
    const file = e.target.files?.[0];
    if (!file) return;
    try {
      const arrBuf = await file.arrayBuffer();
      const bytes = Array.from(new Uint8Array(arrBuf));
      const res = await importarClientesExcel(bytes);
      toastExito(`Importación: ${res.creados} creados, ${res.actualizados} actualizados${res.errores > 0 ? `, ${res.errores} errores` : ""}`);
      if (res.errores > 0 && res.mensajes.length > 0) {
        console.warn("Errores de importación:", res.mensajes);
      }
      cargar();
    } catch (err) {
      toastError("Error: " + err);
    } finally {
      if (fileRef.current) fileRef.current.value = "";
    }
  };

  // v2.5.39: handlers de categorias
  const abrirNuevaCategoria = () => {
    setEditandoCat(null);
    setFormCat({
      nombre: "", descripcion: "", permite_credito: false, dias_credito: 0,
      limite_credito: 0, descuento_pct: 0, requiere_ruc: false, es_default: false, activo: true,
    });
  };
  const abrirEditarCategoria = (c: CategoriaCliente) => {
    setEditandoCat(c);
    setFormCat({ ...c });
  };
  const guardarCategoria = async (e: React.FormEvent) => {
    e.preventDefault();
    try {
      if (editandoCat?.id) {
        await actualizarCategoriaCliente({ ...formCat, id: editandoCat.id });
        toastExito("Categoría actualizada");
      } else {
        await crearCategoriaCliente(formCat);
        toastExito("Categoría creada");
      }
      setEditandoCat(null);
      cargar();
    } catch (err) {
      toastError("Error: " + err);
    }
  };
  const borrarCategoria = async (c: CategoriaCliente) => {
    if (!c.id) return;
    if (!confirm(`¿Eliminar categoría "${c.nombre}"?`)) return;
    try {
      await eliminarCategoriaCliente(c.id);
      toastExito("Categoría eliminada");
      cargar();
    } catch (err) {
      toastError("Error: " + err);
    }
  };

  return (
    <>
      <input ref={fileRef} type="file" accept=".xlsx,.xls" style={{ display: "none" }} onChange={handleImport} />
      <div className="page-header">
        <div style={{ display: "flex", alignItems: "center", gap: 12, flex: 1 }}>
          <h2 style={{ margin: 0 }}>{tab === "categorias" ? "Categorías de Clientes" : `Clientes (${clientes.length})`}</h2>
          {/* v2.5.39: tabs */}
          <div className="flex gap-1" style={{ marginLeft: 8 }}>
            <button className={`btn ${tab === "clientes" ? "btn-primary" : "btn-outline"}`}
              style={{ fontSize: 11, padding: "4px 10px" }} onClick={() => setTab("clientes")}>
              Clientes
            </button>
            <button className={`btn ${tab === "categorias" ? "btn-primary" : "btn-outline"}`}
              style={{ fontSize: 11, padding: "4px 10px" }} onClick={() => setTab("categorias")}>
              📋 Categorías ({categorias.length})
            </button>
          </div>
        </div>
        <div className="flex gap-2">
          {tab === "clientes" ? (
            <>
              <button className="btn btn-outline" style={{ fontSize: 11, padding: "5px 10px" }} onClick={handlePlantilla} title="Descargar plantilla XLSX vacía">
                📋 Plantilla
              </button>
              <button className="btn btn-outline" style={{ fontSize: 11, padding: "5px 10px" }} onClick={handleExportar} title="Exportar todos los clientes a XLSX">
                ⬇ Exportar
              </button>
              <button className="btn btn-outline" style={{ fontSize: 11, padding: "5px 10px" }} onClick={() => fileRef.current?.click()} title="Importar clientes desde XLSX">
                ⬆ Importar
              </button>
              <button className="btn btn-primary" onClick={abrirNuevo}>+ Nuevo Cliente</button>
            </>
          ) : (
            <button className="btn btn-primary" onClick={abrirNuevaCategoria}>+ Nueva Categoría</button>
          )}
        </div>
      </div>
      <div className="page-body">
        {tab === "categorias" ? (
          <CategoriaSection
            categorias={categorias}
            editando={editandoCat}
            form={formCat}
            setForm={setFormCat}
            listasPrecios={listasPrecios}
            onGuardar={guardarCategoria}
            onCancelar={() => setEditandoCat(null)}
            onEditar={abrirEditarCategoria}
            onBorrar={borrarCategoria}
            onNueva={abrirNuevaCategoria}
          />
        ) : mostrarForm ? (
          <div className="card">
            <div className="card-header">{editando ? "Editar Cliente" : "Nuevo Cliente"}</div>
            <div className="card-body">
              <form onSubmit={guardar}>
                <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: 16 }}>
                  <div>
                    <label className="text-secondary" style={{ fontSize: 12 }}>Tipo ID</label>
                    <select className="input" value={form.tipo_identificacion}
                      onChange={(e) => setForm({ ...form, tipo_identificacion: e.target.value })}>
                      <option value="CEDULA">Cédula</option>
                      <option value="RUC">RUC</option>
                      <option value="PASAPORTE">Pasaporte</option>
                    </select>
                  </div>
                  <div>
                    <label className="text-secondary" style={{ fontSize: 12 }}>Identificación</label>
                    <div className="flex gap-1">
                      <input className="input" style={{ flex: 1 }} value={form.identificacion ?? ""}
                        onChange={(e) => setForm({ ...form, identificacion: e.target.value || undefined })} />
                      {!editando && /^\d{10}(\d{3})?$/.test(form.identificacion ?? "") && (
                        <button type="button" className="btn btn-outline" style={{ fontSize: 11, padding: "4px 10px", whiteSpace: "nowrap" }}
                          disabled={consultandoSri}
                          onClick={async () => {
                            setConsultandoSri(true);
                            try {
                              const cliente = await consultarIdentificacion(form.identificacion!);
                              if (cliente.id) {
                                // Ya existe en la BD
                                toastExito(`Cliente encontrado: ${cliente.nombre}`);
                                setMostrarForm(false);
                                setEditando(cliente);
                                setForm(cliente);
                                setMostrarForm(true);
                              } else {
                                setForm({ ...form, nombre: cliente.nombre, direccion: cliente.direccion, tipo_identificacion: cliente.tipo_identificacion });
                                toastExito(`Datos encontrados: ${cliente.nombre}`);
                              }
                            } catch (err: any) {
                              toastError(err?.toString() || "No se encontró información");
                            } finally {
                              setConsultandoSri(false);
                            }
                          }}>
                          {consultandoSri ? "..." : "🔍 SRI"}
                        </button>
                      )}
                    </div>
                  </div>
                  <div style={{ gridColumn: "1 / -1" }}>
                    <label className="text-secondary" style={{ fontSize: 12 }}>Nombre *</label>
                    <input className="input" required value={form.nombre}
                      onChange={(e) => setForm({ ...form, nombre: e.target.value })} />
                  </div>
                  <div>
                    <label className="text-secondary" style={{ fontSize: 12 }}>Teléfono</label>
                    <input className="input" value={form.telefono ?? ""}
                      onChange={(e) => setForm({ ...form, telefono: e.target.value || undefined })} />
                  </div>
                  <div>
                    <label className="text-secondary" style={{ fontSize: 12 }}>Email</label>
                    <input className="input" type="email" value={form.email ?? ""}
                      onChange={(e) => setForm({ ...form, email: e.target.value || undefined })} />
                  </div>
                  <div>
                    <label className="text-secondary" style={{ fontSize: 12 }}>Dirección</label>
                    <input className="input" value={form.direccion ?? ""}
                      onChange={(e) => setForm({ ...form, direccion: e.target.value || undefined })} />
                  </div>
                  <div>
                    <label className="text-secondary" style={{ fontSize: 12 }}>Lista de precios</label>
                    <select className="input" value={form.lista_precio_id ?? ""}
                      onChange={(e) => setForm({ ...form, lista_precio_id: e.target.value ? Number(e.target.value) : undefined })}>
                      <option value="">Por defecto</option>
                      {listasPrecios.map((lp) => (
                        <option key={lp.id} value={lp.id}>{lp.nombre}{lp.es_default ? " (defecto)" : ""}</option>
                      ))}
                    </select>
                  </div>
                  {/* v2.5.39: Categoría con auto-fill de defaults */}
                  <div>
                    <label className="text-secondary" style={{ fontSize: 12 }}>Categoría</label>
                    <select className="input" value={form.categoria_id ?? ""}
                      onChange={(e) => {
                        const cid = e.target.value ? Number(e.target.value) : undefined;
                        const cat = categorias.find(c => c.id === cid);
                        if (cat) {
                          // Heredar defaults de la categoria seleccionada
                          setForm({
                            ...form,
                            categoria_id: cid,
                            permite_credito: cat.permite_credito,
                            dias_credito: cat.dias_credito,
                            limite_credito: cat.limite_credito,
                            descuento_pct: cat.descuento_pct,
                            lista_precio_id: cat.lista_precio_id ?? form.lista_precio_id,
                          });
                          toastExito(`Defaults aplicados de "${cat.nombre}"`);
                        } else {
                          setForm({ ...form, categoria_id: undefined });
                        }
                      }}>
                      <option value="">— Sin categoría —</option>
                      {categorias.map((c) => (
                        <option key={c.id} value={c.id}>
                          {c.nombre}{c.es_default ? " (default)" : ""}
                        </option>
                      ))}
                    </select>
                    <div style={{ fontSize: 10, color: "var(--color-text-secondary)", marginTop: 2 }}>
                      Al elegir categoría, se auto-llenan crédito, días, descuento. Puedes overridear abajo.
                    </div>
                  </div>
                </div>

                {/* v2.5.39: Campos de credito (heredables de categoria pero overrideables) */}
                <div style={{ marginTop: 16, padding: 12, background: "var(--color-surface-alt)", borderRadius: 6 }}>
                  <div style={{ fontSize: 12, fontWeight: 600, marginBottom: 8, color: "var(--color-text-secondary)" }}>
                    Configuración de crédito (heredada de categoría — overrideable)
                  </div>
                  <div style={{ display: "grid", gridTemplateColumns: "auto 1fr 1fr 1fr", gap: 12, alignItems: "end" }}>
                    <div>
                      <label style={{ display: "flex", alignItems: "center", gap: 6, fontSize: 12 }}>
                        <input type="checkbox" checked={!!form.permite_credito}
                          onChange={(e) => setForm({ ...form, permite_credito: e.target.checked })} />
                        Permite crédito
                      </label>
                    </div>
                    <div>
                      <label className="text-secondary" style={{ fontSize: 11 }}>Días crédito</label>
                      <input className="input" type="number" min="0"
                        disabled={!form.permite_credito}
                        value={form.dias_credito ?? 0}
                        onChange={(e) => setForm({ ...form, dias_credito: Number(e.target.value) || 0 })} />
                    </div>
                    <div>
                      <label className="text-secondary" style={{ fontSize: 11 }}>Límite crédito ($)</label>
                      <input className="input" type="number" min="0" step="0.01"
                        disabled={!form.permite_credito}
                        value={form.limite_credito ?? 0}
                        onChange={(e) => setForm({ ...form, limite_credito: Number(e.target.value) || 0 })} />
                    </div>
                    <div>
                      <label className="text-secondary" style={{ fontSize: 11 }}>Descuento (%)</label>
                      <input className="input" type="number" min="0" max="100" step="0.1"
                        value={form.descuento_pct ?? 0}
                        onChange={(e) => setForm({ ...form, descuento_pct: Number(e.target.value) || 0 })} />
                    </div>
                  </div>
                </div>

                <div className="flex gap-2 mt-4" style={{ justifyContent: "flex-end" }}>
                  <button type="button" className="btn btn-outline" onClick={() => setMostrarForm(false)}>Cancelar</button>
                  <button type="submit" className="btn btn-primary">{editando ? "Actualizar" : "Guardar"}</button>
                </div>
              </form>
            </div>
          </div>
        ) : (
          <>
          <div className="mb-4">
            <input
              className="input"
              data-action="busqueda"
              placeholder="Buscar por cédula, RUC o nombre... (Ctrl+B)"
              value={busqueda}
              onChange={(e) => setBusqueda(e.target.value)}
              style={{ maxWidth: 400 }}
              autoFocus
            />
          </div>
          {puedeEliminar && seleccionados.size > 0 && (
            <div style={{ display: "flex", alignItems: "center", gap: 10, marginBottom: 8 }}>
              <span style={{ fontSize: 12, color: "var(--color-text-secondary)" }}>{seleccionados.size} seleccionado(s)</span>
              <button className="btn btn-danger" style={{ fontSize: 12 }} onClick={eliminarVarios}>
                Eliminar seleccionados
              </button>
              <button className="btn btn-outline" style={{ fontSize: 12 }} onClick={() => setSeleccionados(new Set())}>
                Cancelar
              </button>
            </div>
          )}
          <div className="card">
            <table className="table">
              <thead>
                <tr>
                  {puedeEliminar && (
                    <th style={{ width: 32 }}>
                      <input
                        type="checkbox"
                        checked={clientesFiltrados.filter(c => c.id !== 1).length > 0 && clientesFiltrados.filter(c => c.id !== 1).every(c => seleccionados.has(c.id!))}
                        onChange={(e) => {
                          const s = new Set(seleccionados);
                          // Consumidor Final (id=1) nunca es seleccionable
                          clientesFiltrados.filter(c => c.id !== 1).forEach(c => {
                            if (e.target.checked) s.add(c.id!); else s.delete(c.id!);
                          });
                          setSeleccionados(s);
                        }}
                      />
                    </th>
                  )}
                  <th>Identificación</th>
                  <th>Nombre</th>
                  <th>Teléfono</th>
                  <th>Email</th>
                  <th>Lista precios</th>
                  <th></th>
                </tr>
              </thead>
              <tbody>
                {clientesFiltrados.map((c) => (
                  <tr key={c.id}>
                    {puedeEliminar && (
                      <td>
                        {c.id !== 1 && (
                          <input
                            type="checkbox"
                            checked={seleccionados.has(c.id!)}
                            onChange={(e) => {
                              const s = new Set(seleccionados);
                              if (e.target.checked) s.add(c.id!); else s.delete(c.id!);
                              setSeleccionados(s);
                            }}
                          />
                        )}
                      </td>
                    )}
                    <td>{c.identificacion ?? "-"}</td>
                    <td><strong>{c.nombre}</strong></td>
                    <td className="text-secondary">{c.telefono ?? "-"}</td>
                    <td className="text-secondary">{c.email ?? "-"}</td>
                    <td className="text-secondary">{c.lista_precio_nombre ?? "Por defecto"}</td>
                    <td style={{ whiteSpace: "nowrap" }}>
                      <button className="btn btn-outline" onClick={() => abrirEditar(c)}>Editar</button>
                      {puedeEliminar && c.id !== 1 && (
                        <button className="btn btn-outline" title="Eliminar cliente"
                          style={{ marginLeft: 6, color: "var(--color-danger)", borderColor: "var(--color-danger)" }}
                          onClick={() => eliminarUno(c)}>
                          ✕
                        </button>
                      )}
                    </td>
                  </tr>
                ))}
                {clientes.length === 0 && (
                  <tr>
                    <td colSpan={puedeEliminar ? 7 : 6} className="text-center text-secondary" style={{ padding: 40 }}>
                      No hay clientes registrados
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

// ─────────────────────────────────────────────────────────────────────────
// v2.5.39: Sección de Categorías de Clientes
// ─────────────────────────────────────────────────────────────────────────
function CategoriaSection({ categorias, editando, form, setForm, listasPrecios, onGuardar, onCancelar, onEditar, onBorrar, onNueva }: {
  categorias: CategoriaCliente[];
  editando: CategoriaCliente | null;
  form: CategoriaCliente;
  setForm: (c: CategoriaCliente) => void;
  listasPrecios: ListaPrecio[];
  onGuardar: (e: React.FormEvent) => void;
  onCancelar: () => void;
  onEditar: (c: CategoriaCliente) => void;
  onBorrar: (c: CategoriaCliente) => void;
  onNueva: () => void;
}) {
  // Form siempre visible al lado de la tabla
  return (
    <div style={{ display: "grid", gridTemplateColumns: "1fr 380px", gap: 16 }}>
      <div className="card">
        <div className="card-header">Categorías existentes</div>
        <table className="table">
          <thead>
            <tr>
              <th>Nombre</th>
              <th>Crédito</th>
              <th>Días</th>
              <th>Límite</th>
              <th>Desc %</th>
              <th></th>
            </tr>
          </thead>
          <tbody>
            {categorias.length === 0 && (
              <tr><td colSpan={6} className="text-center text-secondary" style={{ padding: 30 }}>
                No hay categorías. Crea la primera con "+ Nueva Categoría".
              </td></tr>
            )}
            {categorias.map(c => (
              <tr key={c.id}>
                <td>
                  <strong>{c.nombre}</strong>
                  {c.es_default && <span style={{ marginLeft: 6, fontSize: 9, padding: "1px 5px", borderRadius: 3, background: "rgba(34,197,94,0.15)", color: "var(--color-success)", fontWeight: 600 }}>DEFAULT</span>}
                  {c.descripcion && <div className="text-secondary" style={{ fontSize: 11 }}>{c.descripcion}</div>}
                </td>
                <td>{c.permite_credito ? "✓" : "—"}</td>
                <td>{c.dias_credito}</td>
                <td>${c.limite_credito.toFixed(2)}</td>
                <td>{c.descuento_pct}%</td>
                <td style={{ whiteSpace: "nowrap" }}>
                  <button className="btn btn-outline" style={{ fontSize: 10, padding: "2px 8px" }} onClick={() => onEditar(c)}>Editar</button>
                  {!c.es_default && (
                    <button className="btn btn-outline" style={{ fontSize: 10, padding: "2px 8px", marginLeft: 4, color: "var(--color-danger)", borderColor: "var(--color-danger)" }} onClick={() => onBorrar(c)}>Borrar</button>
                  )}
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>

      <div className="card" style={{ alignSelf: "flex-start" }}>
        <div className="card-header">{editando ? `Editar: ${editando.nombre}` : "Nueva categoría"}</div>
        <div className="card-body">
          <form onSubmit={onGuardar}>
            <div style={{ marginBottom: 10 }}>
              <label className="text-secondary" style={{ fontSize: 11 }}>Nombre *</label>
              <input className="input" required value={form.nombre}
                onChange={(e) => setForm({ ...form, nombre: e.target.value })} />
            </div>
            <div style={{ marginBottom: 10 }}>
              <label className="text-secondary" style={{ fontSize: 11 }}>Descripción</label>
              <input className="input" value={form.descripcion ?? ""}
                onChange={(e) => setForm({ ...form, descripcion: e.target.value || null })} />
            </div>
            <div style={{ marginBottom: 10 }}>
              <label className="text-secondary" style={{ fontSize: 11 }}>Lista de precios por defecto</label>
              <select className="input" value={form.lista_precio_id ?? ""}
                onChange={(e) => setForm({ ...form, lista_precio_id: e.target.value ? Number(e.target.value) : null })}>
                <option value="">— Default del sistema —</option>
                {listasPrecios.map(lp => (
                  <option key={lp.id} value={lp.id}>{lp.nombre}</option>
                ))}
              </select>
            </div>
            <div style={{ marginBottom: 10, padding: 10, background: "var(--color-surface-alt)", borderRadius: 6 }}>
              <label style={{ display: "flex", alignItems: "center", gap: 6, fontSize: 12, marginBottom: 8 }}>
                <input type="checkbox" checked={form.permite_credito}
                  onChange={(e) => setForm({ ...form, permite_credito: e.target.checked })} />
                Permite crédito por defecto
              </label>
              <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: 8 }}>
                <div>
                  <label className="text-secondary" style={{ fontSize: 10 }}>Días crédito</label>
                  <input className="input" type="number" min="0"
                    disabled={!form.permite_credito}
                    value={form.dias_credito}
                    onChange={(e) => setForm({ ...form, dias_credito: Number(e.target.value) || 0 })} />
                </div>
                <div>
                  <label className="text-secondary" style={{ fontSize: 10 }}>Límite ($)</label>
                  <input className="input" type="number" min="0" step="0.01"
                    disabled={!form.permite_credito}
                    value={form.limite_credito}
                    onChange={(e) => setForm({ ...form, limite_credito: Number(e.target.value) || 0 })} />
                </div>
              </div>
            </div>
            <div style={{ marginBottom: 10 }}>
              <label className="text-secondary" style={{ fontSize: 11 }}>Descuento default (%)</label>
              <input className="input" type="number" min="0" max="100" step="0.1"
                value={form.descuento_pct}
                onChange={(e) => setForm({ ...form, descuento_pct: Number(e.target.value) || 0 })} />
            </div>
            <div style={{ marginBottom: 10 }}>
              <label style={{ display: "flex", alignItems: "center", gap: 6, fontSize: 12 }}>
                <input type="checkbox" checked={form.requiere_ruc}
                  onChange={(e) => setForm({ ...form, requiere_ruc: e.target.checked })} />
                Requiere RUC (no acepta cédula simple)
              </label>
            </div>
            <div style={{ marginBottom: 10 }}>
              <label style={{ display: "flex", alignItems: "center", gap: 6, fontSize: 12 }}>
                <input type="checkbox" checked={form.es_default}
                  onChange={(e) => setForm({ ...form, es_default: e.target.checked })} />
                Categoría por defecto del sistema
              </label>
            </div>
            <div className="flex gap-2" style={{ justifyContent: "flex-end" }}>
              {editando && (
                <button type="button" className="btn btn-outline" style={{ fontSize: 11 }} onClick={onCancelar}>
                  Cancelar
                </button>
              )}
              <button type="submit" className="btn btn-primary" style={{ fontSize: 12 }}>
                {editando ? "Actualizar" : "Crear"}
              </button>
              {!editando && (
                <button type="button" className="btn btn-outline" style={{ fontSize: 11 }} onClick={onNueva}>
                  Limpiar
                </button>
              )}
            </div>
          </form>
        </div>
      </div>
    </div>
  );
}
