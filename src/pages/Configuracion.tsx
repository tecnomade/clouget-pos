import { useState, useEffect } from "react";
import { obtenerConfig, guardarConfig, listarCategorias, crearCategoria, listarImpresorasCached, refrescarImpresoras, obtenerRutaDb, crearRespaldo, restaurarRespaldo, obtenerEstadoLicencia, listarUsuarios, crearUsuario, actualizarUsuario, eliminarUsuario, consultarEstadoSri, cargarCertificadoSri, cambiarAmbienteSri, validarSuscripcionSri, obtenerPlanesSri, crearPedidoSri, cargarLogoNegocio, eliminarLogoNegocio, listarListasPrecios, crearListaPrecio, actualizarListaPrecio, establecerListaDefault } from "../services/api";
import { save, open } from "@tauri-apps/plugin-dialog";
import { openUrl } from "@tauri-apps/plugin-opener";
import { useToast } from "../components/Toast";
import Modal from "../components/Modal";
import type { Categoria, LicenciaInfo, UsuarioInfo, EstadoSri, PlanSri, ConfigContratacion, PedidoCreado, ListaPrecio } from "../types";

export default function Configuracion() {
  const { toastExito, toastError } = useToast();
  const [config, setConfig] = useState<Record<string, string>>({});
  const [guardando, setGuardando] = useState(false);
  const [categorias, setCategorias] = useState<Categoria[]>([]);
  const [nuevaCat, setNuevaCat] = useState("");
  const [impresoras, setImpresoras] = useState<string[]>([]);
  const [refrescandoImpresoras, setRefrescandoImpresoras] = useState(false);
  const [rutaDb, setRutaDb] = useState("");
  const [respaldando, setRespaldando] = useState(false);
  const [mostrarModalRestaurar, setMostrarModalRestaurar] = useState(false);
  const [licencia, setLicencia] = useState<LicenciaInfo | null>(null);
  // SRI
  const [estadoSri, setEstadoSri] = useState<EstadoSri | null>(null);
  const [p12Password, setP12Password] = useState("");
  const [cargandoP12, setCargandoP12] = useState(false);
  const [verificandoSri, setVerificandoSri] = useState(false);
  // Contratacion planes SRI
  const [mostrarPlanes, setMostrarPlanes] = useState(false);
  const [planesSri, setPlanesSri] = useState<PlanSri[]>([]);
  const [configContratacion, setConfigContratacion] = useState<ConfigContratacion | null>(null);
  const [planSeleccionado, setPlanSeleccionado] = useState<PlanSri | null>(null);
  const [cargandoPlanes, setCargandoPlanes] = useState(false);
  const [creandoPedido, setCreandoPedido] = useState(false);
  const [pedidoCreado, setPedidoCreado] = useState<PedidoCreado | null>(null);
  // Listas de precios
  const [listasPrecios, setListasPrecios] = useState<ListaPrecio[]>([]);
  const [nuevaListaNombre, setNuevaListaNombre] = useState("");
  const [nuevaListaDesc, setNuevaListaDesc] = useState("");
  const [editandoListaId, setEditandoListaId] = useState<number | null>(null);
  const [editListaNombre, setEditListaNombre] = useState("");
  const [editListaDesc, setEditListaDesc] = useState("");
  // Cajeros
  const [usuarios, setUsuarios] = useState<UsuarioInfo[]>([]);
  const [mostrarFormCajero, setMostrarFormCajero] = useState(false);
  const [nuevoNombre, setNuevoNombre] = useState("");
  const [nuevoPin, setNuevoPin] = useState("");
  const [nuevoRol, setNuevoRol] = useState("CAJERO");
  const [editandoId, setEditandoId] = useState<number | null>(null);
  const [editPin, setEditPin] = useState("");
  // Ambiente SRI confirmation
  const [mostrarConfirmAmbiente, setMostrarConfirmAmbiente] = useState(false);
  const [ambientePendiente, setAmbientePendiente] = useState("");

  const cargarDatos = async () => {
    const [cfg, cats, imps, ruta, lic, usrs, sri, listas] = await Promise.all([
      obtenerConfig(), listarCategorias(), listarImpresorasCached(), obtenerRutaDb(), obtenerEstadoLicencia(), listarUsuarios().catch(() => []), consultarEstadoSri().catch(() => null), listarListasPrecios().catch(() => [])
    ]);
    setConfig(cfg);
    setCategorias(cats);
    setImpresoras(imps);
    setRutaDb(ruta);
    setLicencia(lic);
    setUsuarios(usrs);
    setEstadoSri(sri);
    setListasPrecios(listas);
  };

  useEffect(() => { cargarDatos(); }, []);

  const handleGuardar = async () => {
    setGuardando(true);
    try {
      await guardarConfig(config);
      toastExito("Configuración guardada");
    } catch (err) {
      toastError("Error: " + err);
    }
    setGuardando(false);
  };

  const handleCrearCategoria = async () => {
    if (!nuevaCat.trim()) return;
    try {
      await crearCategoria({ nombre: nuevaCat.trim(), activo: true });
      setNuevaCat("");
      setCategorias(await listarCategorias());
    } catch (err) {
      toastError("Error: " + err);
    }
  };

  const handleCrearRespaldo = async () => {
    try {
      setRespaldando(true);
      const fecha = new Date().toISOString().slice(0, 10);
      const destino = await save({
        defaultPath: `clouget-respaldo-${fecha}.db`,
        filters: [{ name: "Base de datos SQLite", extensions: ["db"] }],
      });
      if (!destino) return;
      await crearRespaldo(destino);
      toastExito("Respaldo creado exitosamente");
    } catch (err) {
      toastError("Error al crear respaldo: " + err);
    } finally {
      setRespaldando(false);
    }
  };

  const handleRestaurar = async () => {
    setMostrarModalRestaurar(false);
    try {
      const origen = await open({
        filters: [{ name: "Base de datos SQLite", extensions: ["db"] }],
        multiple: false,
      });
      if (!origen) return;
      const msg = await restaurarRespaldo(origen as string);
      toastExito(msg);
    } catch (err) {
      toastError("Error al restaurar: " + err);
    }
  };

  const update = (key: string, value: string) => {
    setConfig((prev) => ({ ...prev, [key]: value }));
  };

  return (
    <>
      <div className="page-header">
        <h2>Configuración</h2>
      </div>
      <div className="page-body">
        <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: 24, maxWidth: 900 }}>
          {/* Datos del negocio */}
          <div className="card">
            <div className="card-header">Datos del Negocio</div>
            <div className="card-body">
              <div style={{ display: "grid", gap: 16 }}>
                {/* Logo del negocio */}
                <div>
                  <label className="text-secondary" style={{ fontSize: 12, display: "block", marginBottom: 8 }}>Logo del negocio (RIDE)</label>
                  <div style={{ display: "flex", alignItems: "center", gap: 12 }}>
                    {config.logo_negocio ? (
                      <div style={{ position: "relative" }}>
                        <img
                          src={`data:image/png;base64,${config.logo_negocio}`}
                          alt="Logo"
                          style={{ maxWidth: 120, maxHeight: 60, objectFit: "contain", border: "1px solid var(--color-border)", borderRadius: "var(--radius)", padding: 4, background: "white" }}
                        />
                      </div>
                    ) : (
                      <div style={{ width: 120, height: 60, border: "2px dashed var(--color-border)", borderRadius: "var(--radius)", display: "flex", alignItems: "center", justifyContent: "center", color: "#94a3b8", fontSize: 11 }}>
                        Sin logo
                      </div>
                    )}
                    <div style={{ display: "flex", flexDirection: "column", gap: 4 }}>
                      <button className="btn btn-outline" style={{ fontSize: 11, padding: "4px 10px" }}
                        onClick={async () => {
                          try {
                            const path = await open({
                              filters: [{ name: "Imagenes", extensions: ["png", "jpg", "jpeg"] }],
                              multiple: false,
                            });
                            if (!path) return;
                            await cargarLogoNegocio(path as string);
                            const cfg = await obtenerConfig();
                            setConfig(cfg);
                            toastExito("Logo cargado");
                          } catch (err) {
                            toastError("Error: " + err);
                          }
                        }}>
                        {config.logo_negocio ? "Cambiar" : "Cargar logo"}
                      </button>
                      {config.logo_negocio && (
                        <button className="btn btn-outline" style={{ fontSize: 11, padding: "4px 10px", color: "#ef4444" }}
                          onClick={async () => {
                            try {
                              await eliminarLogoNegocio();
                              setConfig((prev) => { const c = { ...prev }; delete c.logo_negocio; return c; });
                              toastExito("Logo eliminado");
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
                    PNG o JPG, max 500KB. Aparece en la factura electronica (RIDE).
                  </span>
                </div>
                <div>
                  <label className="text-secondary" style={{ fontSize: 12 }}>Nombre del negocio</label>
                  <input className="input" value={config.nombre_negocio ?? ""}
                    onChange={(e) => update("nombre_negocio", e.target.value)} />
                </div>
                <div>
                  <label className="text-secondary" style={{ fontSize: 12 }}>RUC / RIMPE</label>
                  <input className="input" value={config.ruc ?? ""}
                    onChange={(e) => update("ruc", e.target.value)} />
                </div>
                <div>
                  <label className="text-secondary" style={{ fontSize: 12 }}>Dirección</label>
                  <input className="input" value={config.direccion ?? ""}
                    onChange={(e) => update("direccion", e.target.value)} />
                </div>
                <div>
                  <label className="text-secondary" style={{ fontSize: 12 }}>Teléfono</label>
                  <input className="input" value={config.telefono ?? ""}
                    onChange={(e) => update("telefono", e.target.value)} />
                </div>
                <div>
                  <label className="text-secondary" style={{ fontSize: 12 }}>Régimen tributario</label>
                  <select className="input" value={config.regimen ?? "RIMPE_POPULAR"}
                    onChange={(e) => update("regimen", e.target.value)}>
                    <option value="RIMPE_POPULAR">RIMPE - Negocio Popular</option>
                    <option value="RIMPE_EMPRENDEDOR">RIMPE - Emprendedor</option>
                    <option value="GENERAL">Régimen General</option>
                  </select>
                </div>
                <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: 16 }}>
                  <div>
                    <label className="text-secondary" style={{ fontSize: 12 }}>Establecimiento</label>
                    <input className="input" value={config.establecimiento ?? "001"}
                      onChange={(e) => update("establecimiento", e.target.value)} />
                  </div>
                  <div>
                    <label className="text-secondary" style={{ fontSize: 12 }}>Punto de emisión</label>
                    <input className="input" value={config.punto_emision ?? "001"}
                      onChange={(e) => update("punto_emision", e.target.value)} />
                  </div>
                </div>
                {(config.regimen === "RIMPE_EMPRENDEDOR" || config.regimen === "GENERAL") && (
                  <div style={{ background: "#f8fafc", padding: 12, borderRadius: "var(--radius)", border: "1px solid var(--color-border)" }}>
                    <label className="text-secondary" style={{ fontSize: 12, display: "block", marginBottom: 8, fontWeight: 600 }}>Secuenciales</label>
                    <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: 12 }}>
                      <div>
                        <label className="text-secondary" style={{ fontSize: 11 }}>Factura</label>
                        <input className="input" type="number" min={1}
                          value={config.secuencial_factura ?? "1"}
                          onChange={(e) => update("secuencial_factura", e.target.value)} />
                        <div style={{ fontSize: 10, color: "#94a3b8", marginTop: 2 }}>
                          {config.establecimiento ?? "001"}-{config.punto_emision ?? "001"}-{String(config.secuencial_factura ?? "1").padStart(9, "0")}
                        </div>
                      </div>
                      <div>
                        <label className="text-secondary" style={{ fontSize: 11 }}>Nota de Crédito</label>
                        <input className="input" type="number" min={1}
                          value={config.secuencial_nota_credito ?? "1"}
                          onChange={(e) => update("secuencial_nota_credito", e.target.value)} />
                        <div style={{ fontSize: 10, color: "#94a3b8", marginTop: 2 }}>
                          {config.establecimiento ?? "001"}-{config.punto_emision ?? "001"}-{String(config.secuencial_nota_credito ?? "1").padStart(9, "0")}
                        </div>
                      </div>
                    </div>
                  </div>
                )}
                <div>
                  <label className="text-secondary" style={{ fontSize: 12 }}>Timeout inactividad (minutos)</label>
                  <input className="input" type="number" min={0} max={120} value={config.timeout_inactividad ?? "15"}
                    onChange={(e) => update("timeout_inactividad", e.target.value)} />
                  <span className="text-secondary" style={{ fontSize: 11 }}>0 = desactivado. Cierra sesión tras inactividad.</span>
                </div>
              </div>
              <button
                className="btn btn-primary btn-lg mt-4"
                onClick={handleGuardar}
                disabled={guardando}
              >
                {guardando ? "Guardando..." : "Guardar Configuración"}
              </button>
            </div>
          </div>

          {/* Columna derecha */}
          <div style={{ display: "flex", flexDirection: "column", gap: 24 }}>

          {/* Impresora */}
          <div className="card">
            <div className="card-header">Impresora de Tickets</div>
            <div className="card-body">
              <div>
                <label className="text-secondary" style={{ fontSize: 12 }}>Impresora</label>
                <div className="flex gap-2">
                  <select className="input" style={{ flex: 1 }} value={config.impresora ?? ""}
                    onChange={(e) => update("impresora", e.target.value)}>
                    <option value="">-- Seleccionar impresora --</option>
                    {impresoras.map((imp) => (
                      <option key={imp} value={imp}>{imp}</option>
                    ))}
                  </select>
                  <button className="btn btn-outline" style={{ fontSize: 11, whiteSpace: "nowrap" }}
                    disabled={refrescandoImpresoras}
                    onClick={async () => {
                      setRefrescandoImpresoras(true);
                      try {
                        const nuevas = await refrescarImpresoras();
                        setImpresoras(nuevas);
                        toastExito("Impresoras actualizadas");
                      } catch (err) {
                        toastError("Error: " + err);
                      }
                      setRefrescandoImpresoras(false);
                    }}>
                    {refrescandoImpresoras ? "..." : "Refrescar"}
                  </button>
                </div>
              </div>
              <div className="mt-2">
                <label style={{ fontSize: 13, display: "flex", alignItems: "center", gap: 8, cursor: "pointer" }}>
                  <input type="checkbox"
                    checked={config.auto_imprimir === "1"}
                    onChange={(e) => update("auto_imprimir", e.target.checked ? "1" : "0")} />
                  Imprimir ticket automaticamente al vender
                </label>
              </div>
              <div className="mt-2">
                <label style={{ fontSize: 13, display: "flex", alignItems: "center", gap: 8, cursor: "pointer" }}>
                  <input type="checkbox"
                    checked={config.ticket_usar_pdf === "1"}
                    onChange={(e) => update("ticket_usar_pdf", e.target.checked ? "1" : "0")} />
                  Generar ticket como PDF (abrir en visor del sistema)
                </label>
                <span className="text-secondary" style={{ fontSize: 11, marginLeft: 28, display: "block" }}>
                  Usar cuando la impresora termica no esta disponible
                </span>
              </div>
              <button className="btn btn-primary mt-4" onClick={handleGuardar} disabled={guardando}>
                Guardar
              </button>
            </div>
          </div>

          {/* Categorías */}
          <div className="card">
            <div className="card-header">Categorías de Productos</div>
            <div className="card-body">
              <div className="flex gap-2 mb-4">
                <input
                  className="input"
                  placeholder="Nueva categoría..."
                  value={nuevaCat}
                  onChange={(e) => setNuevaCat(e.target.value)}
                  onKeyDown={(e) => { if (e.key === "Enter") handleCrearCategoria(); }}
                />
                <button className="btn btn-primary" onClick={handleCrearCategoria}>Agregar</button>
              </div>
              {categorias.length === 0 ? (
                <p className="text-secondary text-center" style={{ padding: 16 }}>
                  No hay categorías creadas
                </p>
              ) : (
                <div style={{ display: "flex", flexDirection: "column", gap: 4 }}>
                  {categorias.map((c) => (
                    <div
                      key={c.id}
                      style={{
                        padding: "8px 12px",
                        background: "var(--color-bg)",
                        borderRadius: "var(--radius)",
                        fontSize: 13,
                      }}
                    >
                      {c.nombre}
                    </div>
                  ))}
                </div>
              )}
            </div>
          </div>

          {/* Listas de Precios */}
          <div className="card">
            <div className="card-header">Listas de Precios</div>
            <div className="card-body">
              <p className="text-secondary" style={{ fontSize: 11, marginBottom: 12 }}>
                Defina tarifas diferentes (Público, Mayorista, etc.) y asígnelas a clientes.
              </p>
              <div style={{ display: "flex", gap: 8, marginBottom: 12 }}>
                <input
                  className="input"
                  placeholder="Nombre de lista..."
                  style={{ flex: 1 }}
                  value={nuevaListaNombre}
                  onChange={(e) => setNuevaListaNombre(e.target.value)}
                  onKeyDown={(e) => {
                    if (e.key === "Enter" && nuevaListaNombre.trim()) {
                      (async () => {
                        try {
                          await crearListaPrecio({ nombre: nuevaListaNombre.trim(), descripcion: nuevaListaDesc.trim() || undefined, es_default: false, activo: true });
                          setNuevaListaNombre("");
                          setNuevaListaDesc("");
                          setListasPrecios(await listarListasPrecios());
                          toastExito("Lista creada");
                        } catch (err) { toastError("Error: " + err); }
                      })();
                    }
                  }}
                />
                <button className="btn btn-primary" onClick={async () => {
                  if (!nuevaListaNombre.trim()) return;
                  try {
                    await crearListaPrecio({ nombre: nuevaListaNombre.trim(), descripcion: nuevaListaDesc.trim() || undefined, es_default: false, activo: true });
                    setNuevaListaNombre("");
                    setNuevaListaDesc("");
                    setListasPrecios(await listarListasPrecios());
                    toastExito("Lista creada");
                  } catch (err) { toastError("Error: " + err); }
                }}>Agregar</button>
              </div>
              {listasPrecios.length === 0 ? (
                <p className="text-secondary text-center" style={{ padding: 16 }}>
                  No hay listas de precios
                </p>
              ) : (
                <div style={{ display: "flex", flexDirection: "column", gap: 4 }}>
                  {listasPrecios.map((lp) => (
                    <div key={lp.id} style={{
                      padding: "8px 12px",
                      background: "var(--color-bg)",
                      borderRadius: "var(--radius)",
                      display: "flex",
                      alignItems: "center",
                      gap: 8,
                    }}>
                      {editandoListaId === lp.id ? (
                        <div style={{ flex: 1, display: "flex", gap: 6, alignItems: "center" }}>
                          <input className="input" style={{ flex: 1, fontSize: 12 }}
                            value={editListaNombre}
                            onChange={(e) => setEditListaNombre(e.target.value)} />
                          <input className="input" style={{ flex: 1, fontSize: 12 }}
                            placeholder="Descripción"
                            value={editListaDesc}
                            onChange={(e) => setEditListaDesc(e.target.value)} />
                          <button className="btn btn-primary" style={{ padding: "2px 8px", fontSize: 11 }}
                            onClick={async () => {
                              try {
                                await actualizarListaPrecio({ id: lp.id, nombre: editListaNombre.trim(), descripcion: editListaDesc.trim() || undefined, es_default: lp.es_default, activo: lp.activo });
                                setEditandoListaId(null);
                                setListasPrecios(await listarListasPrecios());
                                toastExito("Lista actualizada");
                              } catch (err) { toastError("Error: " + err); }
                            }}>OK</button>
                          <button className="btn btn-outline" style={{ padding: "2px 8px", fontSize: 11 }}
                            onClick={() => setEditandoListaId(null)}>x</button>
                        </div>
                      ) : (
                        <>
                          <div style={{ flex: 1 }}>
                            <span style={{ fontSize: 13, fontWeight: 600 }}>{lp.nombre}</span>
                            {lp.descripcion && (
                              <span className="text-secondary" style={{ fontSize: 11, marginLeft: 8 }}>{lp.descripcion}</span>
                            )}
                          </div>
                          {lp.es_default && (
                            <span style={{ fontSize: 10, background: "#dcfce7", color: "#166534", padding: "2px 8px", borderRadius: 4, fontWeight: 600 }}>
                              Por defecto
                            </span>
                          )}
                          {!lp.es_default && (
                            <button className="btn btn-outline" style={{ padding: "2px 8px", fontSize: 10 }}
                              onClick={async () => {
                                try {
                                  await establecerListaDefault(lp.id!);
                                  setListasPrecios(await listarListasPrecios());
                                  toastExito(`"${lp.nombre}" es ahora la lista por defecto`);
                                } catch (err) { toastError("Error: " + err); }
                              }}>
                              Defecto
                            </button>
                          )}
                          <button className="btn btn-outline" style={{ padding: "2px 8px", fontSize: 10 }}
                            onClick={() => {
                              setEditandoListaId(lp.id!);
                              setEditListaNombre(lp.nombre);
                              setEditListaDesc(lp.descripcion || "");
                            }}>
                            Editar
                          </button>
                        </>
                      )}
                    </div>
                  ))}
                </div>
              )}
            </div>
          </div>

          {/* Cajeros */}
          <div className="card">
            <div className="card-header flex justify-between items-center">
              <span>Cajeros</span>
              <button className="btn btn-primary" style={{ padding: "4px 12px", fontSize: 12 }}
                onClick={() => { setMostrarFormCajero(true); setEditandoId(null); setNuevoNombre(""); setNuevoPin(""); setNuevoRol("CAJERO"); }}>
                + Nuevo
              </button>
            </div>
            <div className="card-body" style={{ padding: 0 }}>
              {mostrarFormCajero && (
                <div style={{ padding: 12, borderBottom: "1px solid var(--color-border)", background: "#f8fafc" }}>
                  <div style={{ display: "grid", gap: 8 }}>
                    <input className="input" placeholder="Nombre del cajero"
                      value={nuevoNombre} onChange={(e) => setNuevoNombre(e.target.value)} />
                    <input className="input" placeholder="PIN (4-6 digitos)" type="password"
                      maxLength={6} value={nuevoPin}
                      onChange={(e) => { if (/^\d*$/.test(e.target.value)) setNuevoPin(e.target.value); }} />
                    <select className="input" value={nuevoRol} onChange={(e) => setNuevoRol(e.target.value)}>
                      <option value="CAJERO">Cajero</option>
                      <option value="ADMIN">Administrador</option>
                    </select>
                    <div className="flex gap-2">
                      <button className="btn btn-primary" style={{ flex: 1 }}
                        onClick={async () => {
                          try {
                            await crearUsuario({ nombre: nuevoNombre, pin: nuevoPin, rol: nuevoRol });
                            setMostrarFormCajero(false);
                            setUsuarios(await listarUsuarios());
                            toastExito("Cajero creado");
                          } catch (err) { toastError("Error: " + err); }
                        }}>
                        Crear
                      </button>
                      <button className="btn btn-outline" onClick={() => setMostrarFormCajero(false)}>Cancelar</button>
                    </div>
                  </div>
                </div>
              )}
              {usuarios.length === 0 ? (
                <p className="text-secondary text-center" style={{ padding: 16 }}>No hay usuarios</p>
              ) : (
                usuarios.map((u) => (
                  <div key={u.id} style={{
                    padding: "8px 12px", borderBottom: "1px solid var(--color-border)",
                    display: "flex", alignItems: "center", gap: 8,
                    opacity: u.activo ? 1 : 0.5,
                  }}>
                    <div style={{ flex: 1 }}>
                      <div style={{ fontSize: 13, fontWeight: 600 }}>{u.nombre}</div>
                      <span style={{
                        fontSize: 10, padding: "1px 6px", borderRadius: 3,
                        background: u.rol === "ADMIN" ? "#dbeafe" : "#f1f5f9",
                        color: u.rol === "ADMIN" ? "#1e40af" : "#475569",
                      }}>
                        {u.rol}
                      </span>
                      {!u.activo && <span style={{ fontSize: 10, color: "#ef4444", marginLeft: 6 }}>INACTIVO</span>}
                    </div>
                    {editandoId === u.id ? (
                      <div className="flex gap-2 items-center">
                        <input className="input" placeholder="Nuevo PIN" type="password" maxLength={6}
                          style={{ width: 100, fontSize: 12 }}
                          value={editPin}
                          onChange={(e) => { if (/^\d*$/.test(e.target.value)) setEditPin(e.target.value); }} />
                        <button className="btn btn-primary" style={{ padding: "2px 8px", fontSize: 11 }}
                          onClick={async () => {
                            try {
                              await actualizarUsuario(u.id, undefined, editPin || undefined);
                              setEditandoId(null);
                              setEditPin("");
                              toastExito("PIN actualizado");
                            } catch (err) { toastError("Error: " + err); }
                          }}>OK</button>
                        <button className="btn btn-outline" style={{ padding: "2px 8px", fontSize: 11 }}
                          onClick={() => { setEditandoId(null); setEditPin(""); }}>x</button>
                      </div>
                    ) : (
                      <div className="flex gap-2">
                        <button className="btn btn-outline" style={{ padding: "2px 8px", fontSize: 11 }}
                          onClick={() => { setEditandoId(u.id); setEditPin(""); }}>
                          PIN
                        </button>
                        {u.activo ? (
                          <button className="btn btn-outline" style={{ padding: "2px 8px", fontSize: 11, color: "#ef4444" }}
                            onClick={async () => {
                              try {
                                await eliminarUsuario(u.id);
                                setUsuarios(await listarUsuarios());
                                toastExito("Cajero desactivado");
                              } catch (err) { toastError("Error: " + err); }
                            }}>
                            Desactivar
                          </button>
                        ) : (
                          <button className="btn btn-outline" style={{ padding: "2px 8px", fontSize: 11, color: "#22c55e" }}
                            onClick={async () => {
                              try {
                                await actualizarUsuario(u.id, undefined, undefined, undefined, true);
                                setUsuarios(await listarUsuarios());
                                toastExito("Cajero activado");
                              } catch (err) { toastError("Error: " + err); }
                            }}>
                            Activar
                          </button>
                        )}
                      </div>
                    )}
                  </div>
                ))
              )}
            </div>
          </div>

          {/* Licencia */}
          {licencia && (
            <div className="card">
              <div className="card-header" style={{ background: "#f0fdf4", color: "#166534" }}>
                Licencia
                <span style={{ marginLeft: 8, fontSize: 11, background: "#dcfce7", padding: "2px 8px", borderRadius: 4, color: "#166534", fontWeight: 600 }}>
                  Activa
                </span>
              </div>
              <div className="card-body">
                <div style={{ display: "grid", gap: 8, fontSize: 13 }}>
                  <div className="flex justify-between">
                    <span className="text-secondary">Negocio:</span>
                    <span style={{ fontWeight: 600 }}>{licencia.negocio}</span>
                  </div>
                  {licencia.email && (
                    <div className="flex justify-between">
                      <span className="text-secondary">Email:</span>
                      <span>{licencia.email}</span>
                    </div>
                  )}
                  <div className="flex justify-between">
                    <span className="text-secondary">Tipo:</span>
                    <span style={{
                      background: "#dcfce7",
                      color: "#166534",
                      padding: "2px 8px",
                      borderRadius: 4,
                      fontSize: 12,
                      fontWeight: 600,
                    }}>
                      {licencia.tipo === "perpetua" ? "Licencia Perpetua" :
                       licencia.tipo === "anual" ? "Licencia Anual" : licencia.tipo}
                    </span>
                  </div>
                  {licencia.emitida && (
                    <div className="flex justify-between">
                      <span className="text-secondary">Activada:</span>
                      <span>{licencia.emitida}</span>
                    </div>
                  )}
                  <div className="flex justify-between">
                    <span className="text-secondary">Equipo:</span>
                    <span style={{ fontFamily: "monospace", letterSpacing: 1 }}>{licencia.machine_id}</span>
                  </div>
                </div>
              </div>
            </div>
          )}

          {/* Facturacion Electronica - solo si regimen no es RIMPE_POPULAR */}
          {config.regimen && config.regimen !== "RIMPE_POPULAR" && estadoSri && (
            <div className="card" style={{ borderColor: estadoSri.suscripcion_autorizada || estadoSri.suscripcion_es_lifetime ? "#16a34a" : estadoSri.modulo_activo ? "#3b82f6" : undefined }}>
              <div className="card-header" style={{
                background: estadoSri.suscripcion_autorizada || estadoSri.suscripcion_es_lifetime ? "#f0fdf4" : "#eff6ff",
                color: estadoSri.suscripcion_autorizada || estadoSri.suscripcion_es_lifetime ? "#166534" : "#1e40af",
              }}>
                Facturacion Electronica
                {estadoSri.suscripcion_autorizada && (
                  <span style={{ marginLeft: 8, fontSize: 11, background: "#dcfce7", padding: "2px 8px", borderRadius: 4, color: "#166534" }}>
                    {estadoSri.suscripcion_es_lifetime ? "Lifetime" : `Plan ${estadoSri.suscripcion_plan}`}
                  </span>
                )}
              </div>
              <div className="card-body">
                <div style={{ display: "grid", gap: 12, fontSize: 13 }}>

                  {/* Seccion 1: Estado de suscripcion */}
                  {estadoSri.suscripcion_autorizada ? (
                    <div style={{ background: "#f0fdf4", padding: 10, borderRadius: "var(--radius)", border: "1px solid #bbf7d0" }}>
                      <div className="flex justify-between items-center" style={{ marginBottom: 6 }}>
                        <span style={{ fontWeight: 600, color: "#166534" }}>Suscripcion activa</span>
                        <span style={{ fontSize: 11, background: "#dcfce7", padding: "2px 8px", borderRadius: 4, color: "#166534", fontWeight: 600 }}>
                          {estadoSri.suscripcion_es_lifetime ? "Lifetime" :
                           estadoSri.suscripcion_plan === "paquete" ? "Paquete" :
                           estadoSri.suscripcion_plan.charAt(0).toUpperCase() + estadoSri.suscripcion_plan.slice(1)}
                        </span>
                      </div>
                      {!estadoSri.suscripcion_es_lifetime && estadoSri.suscripcion_plan !== "paquete" && estadoSri.suscripcion_hasta && (
                        <div className="flex justify-between" style={{ fontSize: 12, color: "#15803d" }}>
                          <span>Valida hasta:</span>
                          <span style={{ fontWeight: 600 }}>{estadoSri.suscripcion_hasta}</span>
                        </div>
                      )}
                      {estadoSri.suscripcion_plan === "paquete" && estadoSri.suscripcion_docs_restantes != null && (
                        <div className="flex justify-between" style={{ fontSize: 12, color: "#15803d" }}>
                          <span>Documentos restantes:</span>
                          <span style={{ fontWeight: 600 }}>{estadoSri.suscripcion_docs_restantes}</span>
                        </div>
                      )}
                      {estadoSri.suscripcion_es_lifetime && (
                        <div style={{ fontSize: 12, color: "#15803d" }}>Facturas ilimitadas, sin fecha de expiracion</div>
                      )}
                    </div>
                  ) : (
                    <div style={{ background: estadoSri.facturas_usadas < estadoSri.facturas_gratis ? "#eff6ff" : "#fef2f2", padding: 10, borderRadius: "var(--radius)", border: `1px solid ${estadoSri.facturas_usadas < estadoSri.facturas_gratis ? "#bfdbfe" : "#fecaca"}` }}>
                      {estadoSri.facturas_usadas < estadoSri.facturas_gratis ? (
                        <>
                          <div style={{ fontWeight: 600, color: "#1e40af", marginBottom: 4 }}>
                            Prueba gratuita
                          </div>
                          <div style={{ fontSize: 12, color: "#3b82f6" }}>
                            {estadoSri.facturas_gratis - estadoSri.facturas_usadas} de {estadoSri.facturas_gratis} facturas gratis restantes
                          </div>
                          <div style={{ background: "#e2e8f0", borderRadius: 4, height: 6, overflow: "hidden", marginTop: 6 }}>
                            <div style={{
                              width: `${(estadoSri.facturas_usadas / Math.max(1, estadoSri.facturas_gratis)) * 100}%`,
                              height: "100%", background: "#3b82f6", borderRadius: 4,
                            }} />
                          </div>
                          {!mostrarPlanes && (
                            <button className="btn btn-outline" style={{ width: "100%", justifyContent: "center", fontSize: 12, marginTop: 8 }}
                              disabled={cargandoPlanes}
                              onClick={async () => {
                                try {
                                  setCargandoPlanes(true);
                                  const resp = await obtenerPlanesSri();
                                  setPlanesSri(resp.planes);
                                  setConfigContratacion(resp.config);
                                  setMostrarPlanes(true);
                                  setPlanSeleccionado(null);
                                  setPedidoCreado(null);
                                } catch (err) {
                                  toastError("Error cargando planes: " + err);
                                } finally {
                                  setCargandoPlanes(false);
                                }
                              }}>
                              {cargandoPlanes ? "Consultando..." : "Ver planes de suscripcion"}
                            </button>
                          )}
                        </>
                      ) : (
                        <>
                          <div style={{ fontWeight: 600, color: "#991b1b", marginBottom: 4 }}>
                            {estadoSri.suscripcion_mensaje || "Prueba gratuita agotada"}
                          </div>
                          <div style={{ fontSize: 12, color: "#b91c1c", marginBottom: 8 }}>
                            Adquiera una suscripcion para continuar emitiendo facturas electronicas.
                          </div>
                          {!mostrarPlanes && (
                            <button className="btn btn-primary" style={{ width: "100%", justifyContent: "center", fontSize: 13 }}
                              disabled={cargandoPlanes}
                              onClick={async () => {
                                try {
                                  setCargandoPlanes(true);
                                  const resp = await obtenerPlanesSri();
                                  setPlanesSri(resp.planes);
                                  setConfigContratacion(resp.config);
                                  setMostrarPlanes(true);
                                  setPlanSeleccionado(null);
                                  setPedidoCreado(null);
                                } catch (err) {
                                  toastError("Error cargando planes: " + err);
                                } finally {
                                  setCargandoPlanes(false);
                                }
                              }}>
                              {cargandoPlanes ? "Consultando..." : "Activar Facturacion Electronica"}
                            </button>
                          )}
                        </>
                      )}
                    </div>
                  )}

                  {/* Grid de planes disponibles */}
                  {mostrarPlanes && !estadoSri.suscripcion_autorizada && planesSri.length > 0 && !pedidoCreado && (
                    <>
                      <div style={{ fontWeight: 600, fontSize: 14, marginTop: 4 }}>Seleccione un plan:</div>
                      <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: 8 }}>
                        {planesSri.map((plan) => (
                          <div key={plan.clave}
                            onClick={() => setPlanSeleccionado(plan)}
                            style={{
                              padding: 12,
                              border: `2px solid ${planSeleccionado?.clave === plan.clave ? "#3b82f6" : plan.popular ? "#93c5fd" : "var(--color-border)"}`,
                              borderRadius: "var(--radius)",
                              cursor: "pointer",
                              position: "relative",
                              background: planSeleccionado?.clave === plan.clave ? "#eff6ff" : "white",
                              transition: "all 0.15s",
                            }}>
                            {plan.popular && (
                              <span style={{ position: "absolute", top: -8, right: 8, background: "#3b82f6", color: "white", fontSize: 9, padding: "1px 6px", borderRadius: 3, fontWeight: 700 }}>
                                Recomendado
                              </span>
                            )}
                            <div style={{ fontWeight: 800, fontSize: 18, color: "#0f172a" }}>${plan.precio}</div>
                            <div style={{ fontWeight: 600, fontSize: 13 }}>{plan.nombre}</div>
                            <div style={{ fontSize: 11, color: "#64748b", marginTop: 2 }}>{plan.descripcion}</div>
                            {plan.ahorro && (
                              <span style={{ fontSize: 10, color: "#16a34a", fontWeight: 700 }}>{plan.ahorro}</span>
                            )}
                          </div>
                        ))}
                      </div>

                      {/* Metodo de pago */}
                      {planSeleccionado && (
                        <div style={{ display: "grid", gap: 8, marginTop: 4 }}>
                          <div style={{ fontWeight: 600, fontSize: 13 }}>
                            Plan: {planSeleccionado.nombre} — ${planSeleccionado.precio}
                          </div>
                          <div style={{ fontWeight: 600, fontSize: 13 }}>Metodo de pago:</div>
                          <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: 8 }}>
                            {/* WhatsApp */}
                            {configContratacion?.whatsapp_numero && (
                              <button className="btn btn-success" style={{ fontSize: 12, justifyContent: "center" }}
                                onClick={async () => {
                                  const msg = encodeURIComponent(
                                    `Hola, quiero contratar Facturacion Electronica\n` +
                                    `Plan: ${planSeleccionado.nombre} ($${planSeleccionado.precio})\n` +
                                    `Negocio: ${licencia?.negocio || config.nombre_negocio || ""}\n` +
                                    `Machine ID: ${licencia?.machine_id || ""}`
                                  );
                                  try {
                                    await openUrl(`https://wa.me/${configContratacion.whatsapp_numero}?text=${msg}`);
                                  } catch {
                                    window.open(`https://wa.me/${configContratacion.whatsapp_numero}?text=${msg}`, "_blank");
                                  }
                                }}>
                                WhatsApp
                              </button>
                            )}
                            {/* Transferencia */}
                            {configContratacion?.banco_nombre && (
                              <button className="btn btn-primary" style={{ fontSize: 12, justifyContent: "center" }}
                                disabled={creandoPedido}
                                onClick={async () => {
                                  try {
                                    setCreandoPedido(true);
                                    const resp = await crearPedidoSri(
                                      planSeleccionado.clave,
                                      planSeleccionado.nombre,
                                      planSeleccionado.precio,
                                      "TRANSFERENCIA"
                                    );
                                    setPedidoCreado(resp);
                                  } catch (err) {
                                    toastError("Error: " + err);
                                  } finally {
                                    setCreandoPedido(false);
                                  }
                                }}>
                                {creandoPedido ? "Procesando..." : "Transferencia Bancaria"}
                              </button>
                            )}
                          </div>
                        </div>
                      )}
                    </>
                  )}

                  {/* Confirmacion de pedido / datos bancarios */}
                  {pedidoCreado && configContratacion && (
                    <div style={{ background: "#eff6ff", padding: 12, borderRadius: "var(--radius)", border: "1px solid #bfdbfe" }}>
                      <div style={{ fontWeight: 700, color: "#1e40af", marginBottom: 8, fontSize: 14 }}>
                        Pedido registrado
                      </div>
                      <div style={{ fontFamily: "monospace", fontWeight: 800, fontSize: 20, letterSpacing: 2, color: "#0c4a6e", background: "#f0f9ff", padding: "8px 12px", borderRadius: 6, textAlign: "center", border: "2px solid #bae6fd", marginBottom: 8 }}>
                        {pedidoCreado.referencia}
                      </div>
                      <div style={{ fontSize: 12, color: "#1e40af", marginBottom: 8 }}>
                        Incluya esta referencia en el detalle de su transferencia.
                      </div>
                      <div style={{ display: "grid", gap: 4, fontSize: 12 }}>
                        <div style={{ display: "flex", justifyContent: "space-between" }}>
                          <span style={{ color: "#64748b" }}>Banco:</span>
                          <span style={{ fontWeight: 600 }}>{configContratacion.banco_nombre}</span>
                        </div>
                        <div style={{ display: "flex", justifyContent: "space-between" }}>
                          <span style={{ color: "#64748b" }}>Cuenta:</span>
                          <span style={{ fontWeight: 600 }}>{configContratacion.banco_tipo_cuenta} - {configContratacion.banco_numero_cuenta}</span>
                        </div>
                        <div style={{ display: "flex", justifyContent: "space-between" }}>
                          <span style={{ color: "#64748b" }}>Titular:</span>
                          <span style={{ fontWeight: 600 }}>{configContratacion.banco_titular}</span>
                        </div>
                        <div style={{ display: "flex", justifyContent: "space-between" }}>
                          <span style={{ color: "#64748b" }}>CI/RUC:</span>
                          <span style={{ fontWeight: 600 }}>{configContratacion.banco_cedula_ruc}</span>
                        </div>
                        <div style={{ display: "flex", justifyContent: "space-between" }}>
                          <span style={{ color: "#64748b" }}>Monto:</span>
                          <span style={{ fontWeight: 700, color: "#0f172a" }}>${planSeleccionado?.precio?.toFixed(2)}</span>
                        </div>
                      </div>
                      {configContratacion.mensaje_transferencia && (
                        <div style={{ fontSize: 11, color: "#64748b", marginTop: 8, fontStyle: "italic" }}>
                          {configContratacion.mensaje_transferencia}
                        </div>
                      )}
                      <div style={{ fontSize: 12, color: "#1e40af", marginTop: 8, fontWeight: 600 }}>
                        Una vez realizado el deposito, su suscripcion se activara en maximo 24 horas.
                      </div>
                      <button className="btn btn-outline" style={{ width: "100%", justifyContent: "center", fontSize: 12, marginTop: 8 }}
                        disabled={verificandoSri}
                        onClick={async () => {
                          try {
                            setVerificandoSri(true);
                            const estado = await validarSuscripcionSri();
                            setEstadoSri(estado);
                            if (estado.suscripcion_autorizada) {
                              toastExito("Suscripcion activada!");
                              setMostrarPlanes(false);
                              setPedidoCreado(null);
                            } else {
                              toastError("Aun no confirmado. Intente mas tarde.");
                            }
                          } catch (err) { toastError("Error: " + err); }
                          finally { setVerificandoSri(false); }
                        }}>
                        {verificandoSri ? "Verificando..." : "Verificar si ya fue activada"}
                      </button>
                    </div>
                  )}

                  {/* Boton verificar suscripcion */}
                  <button className="btn btn-outline" style={{ fontSize: 12, justifyContent: "center" }}
                    disabled={verificandoSri}
                    onClick={async () => {
                      try {
                        setVerificandoSri(true);
                        const estado = await validarSuscripcionSri();
                        setEstadoSri(estado);
                        if (estado.suscripcion_autorizada) {
                          toastExito("Suscripcion verificada: " + (estado.suscripcion_es_lifetime ? "Lifetime" : `Plan ${estado.suscripcion_plan}`));
                        } else if (estado.facturas_usadas < estado.facturas_gratis) {
                          toastExito("Sin suscripcion. Tiene " + (estado.facturas_gratis - estado.facturas_usadas) + " facturas gratis.");
                        } else {
                          toastError(estado.suscripcion_mensaje || "Sin suscripcion activa");
                        }
                      } catch (err) { toastError("Error verificando: " + err); }
                      finally { setVerificandoSri(false); }
                    }}>
                    {verificandoSri ? "Verificando..." : "Verificar suscripcion"}
                  </button>

                  {/* Seccion 2: Certificado P12 */}
                  {(estadoSri.suscripcion_autorizada || estadoSri.facturas_usadas < estadoSri.facturas_gratis) && !estadoSri.certificado_cargado && (
                    <>
                      <hr style={{ border: "none", borderTop: "1px solid #e2e8f0", margin: "4px 0" }} />
                      <div style={{ background: "#fef3c7", padding: 10, borderRadius: "var(--radius)", color: "#92400e", fontSize: 12 }}>
                        Cargue su certificado digital (.p12) emitido por una entidad autorizada (Security Data, Uanataca, etc.)
                      </div>
                      <div>
                        <label className="text-secondary" style={{ fontSize: 12 }}>Password del certificado</label>
                        <input className="input" type="password" placeholder="Password del P12"
                          value={p12Password} onChange={(e) => setP12Password(e.target.value)} />
                      </div>
                      <button className="btn btn-primary" style={{ justifyContent: "center", width: "100%" }}
                        disabled={!p12Password || cargandoP12}
                        onClick={async () => {
                          try {
                            setCargandoP12(true);
                            const path = await open({
                              filters: [{ name: "Certificado P12", extensions: ["p12", "pfx"] }],
                              multiple: false,
                            });
                            if (!path) return;
                            const msg = await cargarCertificadoSri(path as string, p12Password);
                            setEstadoSri({ ...estadoSri, certificado_cargado: true, modulo_activo: true });
                            setConfig((prev) => ({ ...prev, sri_certificado_cargado: "1", sri_modulo_activo: "1" }));
                            setP12Password("");
                            toastExito(msg);
                          } catch (err) {
                            toastError("Error: " + err);
                          } finally {
                            setCargandoP12(false);
                          }
                        }}>
                        {cargandoP12 ? "Validando..." : "Seleccionar archivo .p12"}
                      </button>
                      {!p12Password && (
                        <div style={{ fontSize: 11, color: "#94a3b8", marginTop: 4, textAlign: "center" }}>
                          Ingrese la password del certificado para habilitar
                        </div>
                      )}
                    </>
                  )}

                  {/* Seccion 3: Configuracion completa (certificado + ambiente) */}
                  {estadoSri.certificado_cargado && (
                    <>
                      <hr style={{ border: "none", borderTop: "1px solid #e2e8f0", margin: "4px 0" }} />
                      <div className="flex justify-between items-center">
                        <span className="text-secondary">Certificado:</span>
                        <span style={{ color: "#16a34a", fontWeight: 600 }}>Cargado</span>
                      </div>
                      <div className="flex justify-between items-center">
                        <span className="text-secondary">Ambiente:</span>
                        <select className="input" style={{ width: 160, fontSize: 12 }}
                          value={estadoSri.ambiente}
                          onChange={(e) => {
                            if (e.target.value !== estadoSri.ambiente) {
                              setAmbientePendiente(e.target.value);
                              setMostrarConfirmAmbiente(true);
                            }
                          }}>
                          <option value="pruebas">Pruebas</option>
                          <option value="produccion">Produccion</option>
                        </select>
                      </div>
                      <div className="flex justify-between items-center">
                        <span className="text-secondary">Emisión automática:</span>
                        <label style={{ display: "flex", alignItems: "center", gap: 6, cursor: "pointer" }}>
                          <input type="checkbox"
                            checked={config.sri_emision_automatica === "1"}
                            onChange={(e) => {
                              const val = e.target.checked ? "1" : "0";
                              setConfig({ ...config, sri_emision_automatica: val });
                              guardarConfig({ sri_emision_automatica: val }).then(() => {
                                toastExito(e.target.checked ? "Emisión automática activada" : "Emisión automática desactivada");
                              }).catch((err) => toastError("Error: " + err));
                            }}
                          />
                          <span style={{ fontSize: 12, color: "var(--color-text-secondary)" }}>
                            {config.sri_emision_automatica === "1" ? "Sí" : "No"}
                          </span>
                        </label>
                      </div>
                      {config.sri_emision_automatica !== "1" && (
                        <p style={{ fontSize: 11, color: "var(--color-text-secondary)", margin: 0 }}>
                          Las facturas se emitirán manualmente desde Ventas del Día con el botón SRI.
                        </p>
                      )}
                      <div className="flex justify-between items-center">
                        <span className="text-secondary">Total facturas emitidas:</span>
                        <span style={{ fontWeight: 600 }}>{estadoSri.facturas_usadas}</span>
                      </div>
                      <button className="btn btn-outline" style={{ fontSize: 12 }}
                        onClick={async () => {
                          try {
                            setCargandoP12(true);
                            const path = await open({
                              filters: [{ name: "Certificado P12", extensions: ["p12", "pfx"] }],
                              multiple: false,
                            });
                            if (!path) return;
                            const pwd = prompt("Password del certificado P12:");
                            if (!pwd) return;
                            const msg = await cargarCertificadoSri(path as string, pwd);
                            toastExito(msg);
                          } catch (err) {
                            toastError("Error: " + err);
                          } finally {
                            setCargandoP12(false);
                          }
                        }}>
                        Cambiar certificado
                      </button>
                    </>
                  )}
                </div>
              </div>
            </div>
          )}

          {/* Respaldo */}
          <div className="card">
            <div className="card-header">Respaldo de Datos</div>
            <div className="card-body">
              <p style={{ fontSize: 12, color: "var(--color-text-secondary)", marginBottom: 12 }}>
                Cree respaldos periódicos de su base de datos para proteger su información.
              </p>
              <div style={{ display: "flex", flexDirection: "column", gap: 8 }}>
                <button
                  className="btn btn-primary"
                  onClick={handleCrearRespaldo}
                  disabled={respaldando}
                >
                  {respaldando ? "Creando respaldo..." : "📁 Crear Respaldo"}
                </button>
                <button
                  className="btn btn-outline"
                  onClick={() => setMostrarModalRestaurar(true)}
                >
                  📥 Restaurar Respaldo
                </button>
              </div>
              {rutaDb && (
                <p style={{ fontSize: 11, color: "var(--color-text-secondary)", marginTop: 12, wordBreak: "break-all" }}>
                  📍 Ubicación BD: {rutaDb}
                </p>
              )}
            </div>
          </div>

          </div>{/* cierre columna derecha */}
        </div>
      </div>

      <Modal
        abierto={mostrarModalRestaurar}
        titulo="Restaurar Respaldo"
        mensaje="¿Está seguro que desea restaurar un respaldo? Esto reemplazará TODOS los datos actuales. Se creará un respaldo automático antes de restaurar. Deberá reiniciar la aplicación después."
        tipo="peligro"
        textoConfirmar="Sí, restaurar"
        onConfirmar={handleRestaurar}
        onCancelar={() => setMostrarModalRestaurar(false)}
      />

      <Modal
        abierto={mostrarConfirmAmbiente}
        titulo="Cambiar Ambiente SRI"
        mensaje={ambientePendiente === "produccion"
          ? "¿Cambiar a PRODUCCION? Las facturas se enviarán al SRI real y tendrán validez tributaria. Asegúrese de que su certificado P12 sea de producción."
          : "¿Cambiar a PRUEBAS? Las facturas se enviarán al entorno de pruebas del SRI y NO tendrán validez tributaria."}
        tipo={ambientePendiente === "produccion" ? "peligro" : "normal"}
        textoConfirmar={`Sí, cambiar a ${ambientePendiente.toUpperCase()}`}
        onConfirmar={async () => {
          setMostrarConfirmAmbiente(false);
          try {
            await cambiarAmbienteSri(ambientePendiente);
            // Mark ambiente as needing re-confirmation in POS
            await guardarConfig({ sri_ambiente_confirmado: "0" });
            setEstadoSri({ ...estadoSri!, ambiente: ambientePendiente });
            toastExito(`Ambiente cambiado a ${ambientePendiente.toUpperCase()}`);
          } catch (err) { toastError("Error: " + err); }
        }}
        onCancelar={() => setMostrarConfirmAmbiente(false)}
      />
    </>
  );
}
