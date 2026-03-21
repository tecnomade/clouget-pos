import { useState, useEffect } from "react";
import {
  resumenDeudores,
  listarCuentasPendientes,
  obtenerCuentaDetalle,
  registrarPagoCuenta,
  listarCuentasBanco,
  crearCuentaBanco,
  actualizarCuentaBanco,
  desactivarCuentaBanco,
  confirmarPagoCuenta,
  rechazarPagoCuenta,
  contarPagosPendientes,
} from "../services/api";
import { open } from "@tauri-apps/plugin-dialog";
import { useSesion } from "../contexts/SesionContext";
import { useToast } from "../components/Toast";
import type { ResumenCliente, CuentaConCliente, CuentaDetalle, CuentaBanco } from "../types";

export default function CuentasPage() {
  const { toastExito, toastError } = useToast();
  const { esAdmin } = useSesion();
  const [vista, setVista] = useState<"resumen" | "detalle" | "historial">("resumen");
  const [deudores, setDeudores] = useState<ResumenCliente[]>([]);
  const [cuentasCliente, setCuentasCliente] = useState<CuentaConCliente[]>([]);
  const [clienteNombre, setClienteNombre] = useState("");
  const [clienteId, setClienteId] = useState<number | null>(null);

  // Detalle de cuenta (historial de pagos)
  const [cuentaDetalle, setCuentaDetalle] = useState<CuentaDetalle | null>(null);

  // Pago form
  const [pagandoCuenta, setPagandoCuenta] = useState<number | null>(null);
  const [montoPago, setMontoPago] = useState("");
  const [obsPago, setObsPago] = useState("");
  const [formaPago, setFormaPago] = useState("EFECTIVO");
  const [bancoId, setBancoId] = useState<number | undefined>(undefined);
  const [numComprobante, setNumComprobante] = useState("");
  const [comprobanteImagen, setComprobanteImagen] = useState("");

  // Bancos
  const [bancos, setBancos] = useState<CuentaBanco[]>([]);
  const [showBancosModal, setShowBancosModal] = useState(false);
  const [editandoBanco, setEditandoBanco] = useState<CuentaBanco | null>(null);
  const [bancoForm, setBancoForm] = useState({ nombre: "", tipo_cuenta: "", numero_cuenta: "", titular: "" });

  // Conteo de transferencias pendientes de confirmación (admin)
  const [pagosPendientesCount, setPagosPendientesCount] = useState(0);

  const totalDeuda = deudores.reduce((s, d) => s + d.total_deuda, 0);
  const totalCuentas = deudores.reduce((s, d) => s + d.num_cuentas, 0);

  const cargarResumen = async () => {
    try {
      setDeudores(await resumenDeudores());
    } catch (err) {
      toastError("Error: " + err);
    }
  };

  const cargarBancos = async () => {
    try {
      setBancos(await listarCuentasBanco());
    } catch {
      // Sin bancos disponibles
    }
  };

  const cargarPendientes = async () => {
    if (esAdmin) {
      try { setPagosPendientesCount(await contarPagosPendientes()); } catch { /* */ }
    }
  };

  useEffect(() => {
    cargarResumen();
    cargarBancos();
    cargarPendientes();
  }, []);

  const verDetalle = async (cId: number, nombre: string) => {
    try {
      const cuentas = await listarCuentasPendientes(cId);
      setCuentasCliente(cuentas);
      setClienteNombre(nombre);
      setClienteId(cId);
      setVista("detalle");
    } catch (err) {
      toastError("Error: " + err);
    }
  };

  const verHistorial = async (cuentaId: number) => {
    try {
      const detalle = await obtenerCuentaDetalle(cuentaId);
      setCuentaDetalle(detalle);
      setVista("historial");
    } catch (err) {
      toastError("Error: " + err);
    }
  };

  const resetPagoForm = () => {
    setPagandoCuenta(null);
    setMontoPago("");
    setObsPago("");
    setFormaPago("EFECTIVO");
    setBancoId(undefined);
    setNumComprobante("");
    setComprobanteImagen("");
  };

  const handlePago = async (cuentaId: number) => {
    if (!montoPago || parseFloat(montoPago) <= 0) {
      toastError("Ingrese un monto valido");
      return;
    }
    if (formaPago === "TRANSFERENCIA" && !bancoId) {
      toastError("Seleccione un banco para pagos por transferencia");
      return;
    }
    try {
      await registrarPagoCuenta({
        cuenta_id: cuentaId,
        monto: parseFloat(montoPago),
        observacion: obsPago.trim() || undefined,
        forma_pago: formaPago,
        banco_id: formaPago === "TRANSFERENCIA" ? bancoId : undefined,
        numero_comprobante: numComprobante.trim() || undefined,
        comprobante_imagen: comprobanteImagen || undefined,
      });
      toastExito(
        formaPago === "TRANSFERENCIA"
          ? "Transferencia registrada - pendiente de confirmacion del administrador"
          : "Pago registrado"
      );
      resetPagoForm();
      cargarPendientes();
      // Refrescar
      if (clienteId) {
        const cuentas = await listarCuentasPendientes(clienteId);
        setCuentasCliente(cuentas);
        if (cuentas.length === 0) {
          setVista("resumen");
        }
      }
      cargarResumen();
    } catch (err) {
      toastError("Error: " + err);
    }
  };

  const handleSeleccionarImagen = async () => {
    try {
      const path = await open({
        filters: [{ name: "Imagenes", extensions: ["png", "jpg", "jpeg", "webp"] }],
        multiple: false,
      });
      if (path) {
        setComprobanteImagen(path as string);
      }
    } catch (err) {
      toastError("Error al seleccionar imagen: " + err);
    }
  };

  // --- Bancos CRUD ---
  const handleGuardarBanco = async () => {
    if (!bancoForm.nombre.trim()) {
      toastError("El nombre del banco es requerido");
      return;
    }
    try {
      if (editandoBanco?.id) {
        await actualizarCuentaBanco(editandoBanco.id, {
          nombre: bancoForm.nombre.trim(),
          tipo_cuenta: bancoForm.tipo_cuenta.trim() || undefined,
          numero_cuenta: bancoForm.numero_cuenta.trim() || undefined,
          titular: bancoForm.titular.trim() || undefined,
          activa: true,
        });
        toastExito("Banco actualizado");
      } else {
        await crearCuentaBanco({
          nombre: bancoForm.nombre.trim(),
          tipo_cuenta: bancoForm.tipo_cuenta.trim() || undefined,
          numero_cuenta: bancoForm.numero_cuenta.trim() || undefined,
          titular: bancoForm.titular.trim() || undefined,
          activa: true,
        });
        toastExito("Banco creado");
      }
      setEditandoBanco(null);
      setBancoForm({ nombre: "", tipo_cuenta: "", numero_cuenta: "", titular: "" });
      cargarBancos();
    } catch (err) {
      toastError("Error: " + err);
    }
  };

  const handleEliminarBanco = async (id: number) => {
    if (!confirm("¿Desactivar esta cuenta bancaria?")) return;
    try {
      await desactivarCuentaBanco(id);
      toastExito("Banco desactivado");
      cargarBancos();
    } catch (err) {
      toastError("Error: " + err);
    }
  };

  // --- Vista Historial de pagos ---
  if (vista === "historial" && cuentaDetalle) {
    return (
      <>
        <div className="page-header">
          <div className="flex gap-2 items-center">
            <button className="btn btn-outline" onClick={() => {
              setVista("detalle");
              setCuentaDetalle(null);
            }}>
              ← Volver
            </button>
            <h2>Historial de pagos - Venta #{cuentaDetalle.venta_numero}</h2>
          </div>
          <span className="text-secondary">{cuentaDetalle.cliente_nombre}</span>
        </div>
        <div className="page-body">
          <div className="flex gap-4 mb-4">
            <div className="card" style={{ flex: 1, maxWidth: 200 }}>
              <div className="card-body text-center">
                <span className="text-secondary" style={{ fontSize: 12 }}>Total</span>
                <div className="font-bold">${cuentaDetalle.cuenta.monto_total.toFixed(2)}</div>
              </div>
            </div>
            <div className="card" style={{ flex: 1, maxWidth: 200 }}>
              <div className="card-body text-center">
                <span className="text-secondary" style={{ fontSize: 12 }}>Pagado</span>
                <div className="font-bold" style={{ color: "var(--color-success)" }}>
                  ${cuentaDetalle.cuenta.monto_pagado.toFixed(2)}
                </div>
              </div>
            </div>
            <div className="card" style={{ flex: 1, maxWidth: 200 }}>
              <div className="card-body text-center">
                <span className="text-secondary" style={{ fontSize: 12 }}>Saldo</span>
                <div className="font-bold" style={{ color: cuentaDetalle.cuenta.saldo > 0 ? "var(--color-danger)" : "var(--color-success)" }}>
                  ${cuentaDetalle.cuenta.saldo.toFixed(2)}
                </div>
              </div>
            </div>
          </div>

          <div className="card">
            <table className="table">
              <thead>
                <tr>
                  <th>Fecha</th>
                  <th className="text-right">Monto</th>
                  <th>Forma de pago</th>
                  <th>Estado</th>
                  <th>Banco / Comprobante</th>
                  <th>Observacion</th>
                  {esAdmin && <th style={{ width: 140 }}></th>}
                </tr>
              </thead>
              <tbody>
                {cuentaDetalle.pagos.length === 0 ? (
                  <tr>
                    <td colSpan={esAdmin ? 7 : 6} className="text-center text-secondary" style={{ padding: 40 }}>
                      Sin pagos registrados
                    </td>
                  </tr>
                ) : (
                  cuentaDetalle.pagos.map((p) => (
                    <tr key={p.id}>
                      <td>
                        {p.fecha ? new Date(p.fecha).toLocaleDateString("es-EC", { day: "2-digit", month: "2-digit", year: "numeric", hour: "2-digit", minute: "2-digit" }) : "-"}
                      </td>
                      <td className="text-right font-bold">${p.monto.toFixed(2)}</td>
                      <td>
                        <span style={{
                          padding: "2px 8px",
                          borderRadius: 4,
                          fontSize: 11,
                          fontWeight: 600,
                          background: p.forma_pago === "EFECTIVO" ? "#dcfce7" : "#dbeafe",
                          color: p.forma_pago === "EFECTIVO" ? "#166534" : "#1e40af",
                        }}>
                          {p.forma_pago === "EFECTIVO" ? "Efectivo" : "Transferencia"}
                        </span>
                      </td>
                      <td>
                        <span style={{
                          padding: "2px 8px", borderRadius: 4, fontSize: 11, fontWeight: 600,
                          background: p.estado === "CONFIRMADO" ? "#dcfce7" : p.estado === "PENDIENTE" ? "#fef3c7" : "#fee2e2",
                          color: p.estado === "CONFIRMADO" ? "#166534" : p.estado === "PENDIENTE" ? "#92400e" : "#991b1b",
                        }}>
                          {p.estado === "CONFIRMADO" ? "Confirmado" : p.estado === "PENDIENTE" ? "Pendiente" : "Rechazado"}
                        </span>
                      </td>
                      <td>
                        {p.banco_nombre && <span>{p.banco_nombre}</span>}
                        {p.numero_comprobante && <span className="text-secondary" style={{ marginLeft: 8, fontSize: 12 }}>#{p.numero_comprobante}</span>}
                      </td>
                      <td className="text-secondary">{p.observacion || "-"}</td>
                      {esAdmin && (
                        <td>
                          {p.estado === "PENDIENTE" && (
                            <div className="flex gap-1">
                              <button className="btn btn-success" style={{ fontSize: 11, padding: "2px 8px" }}
                                onClick={async () => {
                                  try {
                                    const det = await confirmarPagoCuenta(p.id!);
                                    setCuentaDetalle(det);
                                    cargarResumen();
                                    cargarPendientes();
                                    toastExito("Pago confirmado");
                                  } catch (err) { toastError("Error: " + err); }
                                }}>
                                Confirmar
                              </button>
                              <button className="btn btn-outline" style={{ fontSize: 11, padding: "2px 8px", color: "var(--color-danger)" }}
                                onClick={async () => {
                                  if (!confirm("¿Rechazar este pago por transferencia?")) return;
                                  try {
                                    const det = await rechazarPagoCuenta(p.id!);
                                    setCuentaDetalle(det);
                                    cargarResumen();
                                    cargarPendientes();
                                    toastExito("Pago rechazado");
                                  } catch (err) { toastError("Error: " + err); }
                                }}>
                                Rechazar
                              </button>
                            </div>
                          )}
                        </td>
                      )}
                    </tr>
                  ))
                )}
              </tbody>
            </table>
          </div>
        </div>
      </>
    );
  }

  // --- Vista detalle por cliente ---
  if (vista === "detalle") {
    const totalCliente = cuentasCliente.reduce((s, c) => s + c.cuenta.saldo, 0);

    return (
      <>
        <div className="page-header">
          <div className="flex gap-2 items-center">
            <button className="btn btn-outline" onClick={() => { setVista("resumen"); cargarResumen(); }}>
              ← Volver
            </button>
            <h2>Cuentas por cobrar - {clienteNombre}</h2>
          </div>
          <span className="font-bold" style={{ color: "var(--color-danger)", fontSize: 18 }}>
            Deuda: ${totalCliente.toFixed(2)}
          </span>
        </div>
        <div className="page-body">
          {cuentasCliente.length === 0 ? (
            <div className="card">
              <div className="card-body text-center text-secondary" style={{ padding: 40 }}>
                Este cliente no tiene cuentas pendientes
              </div>
            </div>
          ) : (
            <div style={{ display: "flex", flexDirection: "column", gap: 12 }}>
              {cuentasCliente.map((cc) => (
                <div key={cc.cuenta.id} className="card">
                  <div className="card-body">
                    <div className="flex justify-between items-center mb-2">
                      <div>
                        <strong>Venta #{cc.venta_numero}</strong>
                        <span className="text-secondary" style={{ marginLeft: 12, fontSize: 12 }}>
                          {cc.cuenta.created_at
                            ? new Date(cc.cuenta.created_at).toLocaleDateString("es-EC")
                            : ""}
                        </span>
                      </div>
                      <div className="text-right">
                        <div className="text-secondary" style={{ fontSize: 12 }}>
                          Total: ${cc.cuenta.monto_total.toFixed(2)} | Pagado: ${cc.cuenta.monto_pagado.toFixed(2)}
                        </div>
                        <div className="font-bold" style={{ color: "var(--color-danger)", fontSize: 18 }}>
                          Saldo: ${cc.cuenta.saldo.toFixed(2)}
                        </div>
                      </div>
                    </div>

                    {pagandoCuenta === cc.cuenta.id ? (
                      <div style={{
                        background: "var(--color-bg)", padding: 12, borderRadius: "var(--radius)", marginTop: 8,
                      }}>
                        {/* Fila 1: Monto + Forma de pago */}
                        <div className="flex gap-2 items-end">
                          <div style={{ flex: 1 }}>
                            <label className="text-secondary" style={{ fontSize: 11 }}>Monto</label>
                            <input
                              className="input"
                              type="number"
                              step="0.01"
                              min="0.01"
                              max={cc.cuenta.saldo}
                              placeholder={`Max: $${cc.cuenta.saldo.toFixed(2)}`}
                              value={montoPago}
                              onChange={(e) => setMontoPago(e.target.value)}
                              autoFocus
                              onKeyDown={(e) => { if (e.key === "Enter") handlePago(cc.cuenta.id!); }}
                            />
                          </div>
                          <div style={{ flex: 1 }}>
                            <label className="text-secondary" style={{ fontSize: 11 }}>Forma de pago</label>
                            <select
                              className="input"
                              value={formaPago}
                              onChange={(e) => {
                                setFormaPago(e.target.value);
                                if (e.target.value === "EFECTIVO") {
                                  setBancoId(undefined);
                                  setNumComprobante("");
                                  setComprobanteImagen("");
                                }
                              }}
                            >
                              <option value="EFECTIVO">Efectivo</option>
                              <option value="TRANSFERENCIA">Transferencia</option>
                            </select>
                          </div>
                          <div style={{ flex: 1 }}>
                            <label className="text-secondary" style={{ fontSize: 11 }}>Observacion</label>
                            <input
                              className="input"
                              placeholder="Opcional"
                              value={obsPago}
                              onChange={(e) => setObsPago(e.target.value)}
                            />
                          </div>
                        </div>

                        {/* Fila 2: Campos transferencia (condicional) */}
                        {formaPago === "TRANSFERENCIA" && (
                          <div className="flex gap-2 items-end mt-2">
                            <div style={{ flex: 1 }}>
                              <label className="text-secondary" style={{ fontSize: 11 }}>Banco *</label>
                              <select
                                className="input"
                                value={bancoId ?? ""}
                                onChange={(e) => setBancoId(e.target.value ? Number(e.target.value) : undefined)}
                              >
                                <option value="">Seleccione banco...</option>
                                {bancos.map((b) => (
                                  <option key={b.id} value={b.id}>{b.nombre}{b.numero_cuenta ? ` - ${b.numero_cuenta}` : ""}</option>
                                ))}
                              </select>
                            </div>
                            <div style={{ flex: 1 }}>
                              <label className="text-secondary" style={{ fontSize: 11 }}>N. comprobante</label>
                              <input
                                className="input"
                                placeholder="Opcional"
                                value={numComprobante}
                                onChange={(e) => setNumComprobante(e.target.value)}
                              />
                            </div>
                            <div style={{ paddingBottom: 0 }}>
                              <label className="text-secondary" style={{ fontSize: 11 }}>Imagen</label>
                              <button
                                className="btn btn-outline"
                                style={{ fontSize: 11, display: "block", width: "100%" }}
                                onClick={handleSeleccionarImagen}
                              >
                                {comprobanteImagen ? "Cambiar" : "Adjuntar"}
                              </button>
                              {comprobanteImagen && (
                                <span className="text-secondary" style={{ fontSize: 10 }}>Adjunto</span>
                              )}
                            </div>
                          </div>
                        )}

                        {/* Fila 3: Botones */}
                        <div className="flex justify-between items-center mt-3">
                          <button
                            className="btn btn-outline"
                            style={{ fontSize: 12 }}
                            onClick={() => setMontoPago(cc.cuenta.saldo.toFixed(2))}
                          >
                            Pagar todo (${cc.cuenta.saldo.toFixed(2)})
                          </button>
                          <div className="flex gap-2">
                            <button className="btn btn-outline" onClick={resetPagoForm}>
                              Cancelar
                            </button>
                            <button className="btn btn-success" onClick={() => handlePago(cc.cuenta.id!)}>
                              Registrar Pago
                            </button>
                          </div>
                        </div>
                      </div>
                    ) : (
                      <div className="flex gap-2 mt-2">
                        <button
                          className="btn btn-primary"
                          style={{ fontSize: 12 }}
                          onClick={() => setPagandoCuenta(cc.cuenta.id!)}
                        >
                          Registrar Pago
                        </button>
                        {cc.cuenta.monto_pagado > 0 && (
                          <button
                            className="btn btn-outline"
                            style={{ fontSize: 12 }}
                            onClick={() => verHistorial(cc.cuenta.id!)}
                          >
                            Ver pagos
                          </button>
                        )}
                      </div>
                    )}
                  </div>
                </div>
              ))}
            </div>
          )}
        </div>
      </>
    );
  }

  // --- Modal de gestion de bancos ---
  const bancosModal = showBancosModal && (
    <div style={{
      position: "fixed", inset: 0, background: "rgba(0,0,0,0.5)", zIndex: 1000,
      display: "flex", justifyContent: "center", alignItems: "center",
    }} onClick={() => setShowBancosModal(false)}>
      <div className="card" style={{ width: 560, maxHeight: "80vh", overflow: "auto" }} onClick={(e) => e.stopPropagation()}>
        <div className="card-body">
          <h3 style={{ marginBottom: 16 }}>Gestionar cuentas bancarias</h3>

          {/* Formulario */}
          <div style={{ background: "var(--color-bg)", padding: 12, borderRadius: "var(--radius)", marginBottom: 16 }}>
            <div className="flex gap-2">
              <div style={{ flex: 2 }}>
                <label className="text-secondary" style={{ fontSize: 11 }}>Nombre *</label>
                <input className="input" placeholder="Ej: Banco Pichincha" value={bancoForm.nombre}
                  onChange={(e) => setBancoForm({ ...bancoForm, nombre: e.target.value })} />
              </div>
              <div style={{ flex: 1 }}>
                <label className="text-secondary" style={{ fontSize: 11 }}>Tipo</label>
                <input className="input" placeholder="Ahorros" value={bancoForm.tipo_cuenta}
                  onChange={(e) => setBancoForm({ ...bancoForm, tipo_cuenta: e.target.value })} />
              </div>
            </div>
            <div className="flex gap-2 mt-2">
              <div style={{ flex: 1 }}>
                <label className="text-secondary" style={{ fontSize: 11 }}>N. cuenta</label>
                <input className="input" placeholder="2200123456" value={bancoForm.numero_cuenta}
                  onChange={(e) => setBancoForm({ ...bancoForm, numero_cuenta: e.target.value })} />
              </div>
              <div style={{ flex: 1 }}>
                <label className="text-secondary" style={{ fontSize: 11 }}>Titular</label>
                <input className="input" placeholder="Nombre del titular" value={bancoForm.titular}
                  onChange={(e) => setBancoForm({ ...bancoForm, titular: e.target.value })} />
              </div>
            </div>
            <div className="flex gap-2 mt-2">
              <button className="btn btn-primary" onClick={handleGuardarBanco}>
                {editandoBanco ? "Actualizar" : "Agregar banco"}
              </button>
              {editandoBanco && (
                <button className="btn btn-outline" onClick={() => {
                  setEditandoBanco(null);
                  setBancoForm({ nombre: "", tipo_cuenta: "", numero_cuenta: "", titular: "" });
                }}>
                  Cancelar
                </button>
              )}
            </div>
          </div>

          {/* Lista */}
          {bancos.length === 0 ? (
            <p className="text-secondary text-center" style={{ padding: 20 }}>
              No hay bancos registrados
            </p>
          ) : (
            <table className="table">
              <thead>
                <tr>
                  <th>Banco</th>
                  <th>Tipo</th>
                  <th>N. cuenta</th>
                  <th>Titular</th>
                  <th style={{ width: 100 }}></th>
                </tr>
              </thead>
              <tbody>
                {bancos.map((b) => (
                  <tr key={b.id}>
                    <td><strong>{b.nombre}</strong></td>
                    <td>{b.tipo_cuenta || "-"}</td>
                    <td>{b.numero_cuenta || "-"}</td>
                    <td>{b.titular || "-"}</td>
                    <td>
                      <div className="flex gap-1">
                        <button className="btn btn-outline" style={{ fontSize: 11, padding: "2px 6px" }}
                          onClick={() => {
                            setEditandoBanco(b);
                            setBancoForm({
                              nombre: b.nombre,
                              tipo_cuenta: b.tipo_cuenta || "",
                              numero_cuenta: b.numero_cuenta || "",
                              titular: b.titular || "",
                            });
                          }}>
                          Editar
                        </button>
                        <button className="btn btn-outline" style={{ fontSize: 11, padding: "2px 6px", color: "var(--color-danger)" }}
                          onClick={() => handleEliminarBanco(b.id!)}>
                          X
                        </button>
                      </div>
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          )}

          <div className="flex justify-end mt-4">
            <button className="btn btn-outline" onClick={() => setShowBancosModal(false)}>Cerrar</button>
          </div>
        </div>
      </div>
    </div>
  );

  // --- Vista resumen de deudores ---
  return (
    <>
      {bancosModal}
      <div className="page-header">
        <div className="flex gap-2 items-center">
          <h2>Cuentas por Cobrar</h2>
          {esAdmin && (
            <button className="btn btn-outline" style={{ fontSize: 12 }}
              onClick={() => setShowBancosModal(true)}>
              Gestionar bancos
            </button>
          )}
        </div>
        {totalDeuda > 0 && (
          <span className="font-bold" style={{ color: "var(--color-danger)", fontSize: 16 }}>
            Total pendiente: ${totalDeuda.toFixed(2)}
          </span>
        )}
      </div>
      <div className="page-body">
        {/* Resumen cards */}
        <div className="flex gap-4 mb-4">
          <div className="card" style={{ flex: 1, maxWidth: 200 }}>
            <div className="card-body text-center">
              <span className="text-secondary" style={{ fontSize: 12 }}>Total deuda</span>
              <div className="text-xl font-bold" style={{ color: "var(--color-danger)" }}>
                ${totalDeuda.toFixed(2)}
              </div>
            </div>
          </div>
          <div className="card" style={{ flex: 1, maxWidth: 200 }}>
            <div className="card-body text-center">
              <span className="text-secondary" style={{ fontSize: 12 }}>Cuentas pendientes</span>
              <div className="text-xl font-bold">{totalCuentas}</div>
            </div>
          </div>
          <div className="card" style={{ flex: 1, maxWidth: 200 }}>
            <div className="card-body text-center">
              <span className="text-secondary" style={{ fontSize: 12 }}>Deudores</span>
              <div className="text-xl font-bold">{deudores.length}</div>
            </div>
          </div>
          {esAdmin && pagosPendientesCount > 0 && (
            <div className="card" style={{ flex: 1, maxWidth: 220, border: "1px solid #fbbf24" }}>
              <div className="card-body text-center">
                <span className="text-secondary" style={{ fontSize: 12 }}>Transferencias pendientes</span>
                <div className="text-xl font-bold" style={{ color: "#92400e" }}>
                  {pagosPendientesCount}
                </div>
                <span style={{ fontSize: 10, color: "#92400e" }}>Requieren confirmacion</span>
              </div>
            </div>
          )}
        </div>

        {/* Tabla de deudores */}
        <div className="card">
          <table className="table">
            <thead>
              <tr>
                <th>Cliente</th>
                <th className="text-center">Cuentas</th>
                <th className="text-right">Total Deuda</th>
                <th style={{ width: 120 }}></th>
              </tr>
            </thead>
            <tbody>
              {deudores.length === 0 ? (
                <tr>
                  <td colSpan={4} className="text-center text-secondary" style={{ padding: 40 }}>
                    No hay cuentas pendientes
                  </td>
                </tr>
              ) : (
                deudores.map((d) => (
                  <tr key={d.cliente_id}>
                    <td><strong>{d.cliente_nombre}</strong></td>
                    <td className="text-center">{d.num_cuentas}</td>
                    <td className="text-right font-bold" style={{ color: "var(--color-danger)" }}>
                      ${d.total_deuda.toFixed(2)}
                    </td>
                    <td>
                      <button
                        className="btn btn-primary"
                        style={{ fontSize: 12 }}
                        onClick={() => verDetalle(d.cliente_id, d.cliente_nombre)}
                      >
                        Ver detalle
                      </button>
                    </td>
                  </tr>
                ))
              )}
            </tbody>
          </table>
        </div>
      </div>
    </>
  );
}
