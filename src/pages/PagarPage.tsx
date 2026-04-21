import { useState, useEffect } from "react";
import {
  resumenAcreedores,
  listarCuentasPagar,
  registrarPagoProveedor,
  historialPagosProveedor,
  listarCuentasBanco,
} from "../services/api";
import { useToast } from "../components/Toast";
import type { ResumenAcreedor, CuentaPorPagar, PagoProveedor, CuentaBanco } from "../types";

export default function PagarPage() {
  const { toastExito, toastError } = useToast();
  const [vista, setVista] = useState<"resumen" | "detalle" | "historial">("resumen");
  const [acreedores, setAcreedores] = useState<ResumenAcreedor[]>([]);
  const [cuentasProveedor, setCuentasProveedor] = useState<CuentaPorPagar[]>([]);
  const [proveedorNombre, setProveedorNombre] = useState("");
  const [proveedorId, setProveedorId] = useState<number | null>(null);

  // Historial de pagos
  const [pagosHistorial, setPagosHistorial] = useState<PagoProveedor[]>([]);
  const [cuentaHistorial, setCuentaHistorial] = useState<CuentaPorPagar | null>(null);

  // Pago form
  const [pagandoCuenta, setPagandoCuenta] = useState<number | null>(null);
  const [montoPago, setMontoPago] = useState("");
  const [formaPago, setFormaPago] = useState("EFECTIVO");
  const [numComprobante, setNumComprobante] = useState("");
  const [obsPago, setObsPago] = useState("");
  const [bancoId, setBancoId] = useState<number | undefined>(undefined);
  const [cuentasBanco, setCuentasBanco] = useState<CuentaBanco[]>([]);

  const totalDeuda = acreedores.reduce((s, a) => s + a.total_deuda, 0);
  const totalCuentas = acreedores.reduce((s, a) => s + a.num_cuentas, 0);

  const cargarResumen = async () => {
    try {
      setAcreedores(await resumenAcreedores());
    } catch (err) {
      toastError("Error: " + err);
    }
  };

  useEffect(() => {
    cargarResumen();
    listarCuentasBanco().then(setCuentasBanco).catch(() => {});
  }, []);

  const verCuentas = async (pId: number, nombre: string) => {
    try {
      const cuentas = await listarCuentasPagar(pId);
      setCuentasProveedor(cuentas);
      setProveedorNombre(nombre);
      setProveedorId(pId);
      setVista("detalle");
    } catch (err) {
      toastError("Error: " + err);
    }
  };

  const verHistorial = async (cuenta: CuentaPorPagar) => {
    try {
      const pagos = await historialPagosProveedor(cuenta.id!);
      setPagosHistorial(pagos);
      setCuentaHistorial(cuenta);
      setVista("historial");
    } catch (err) {
      toastError("Error: " + err);
    }
  };

  const resetPagoForm = () => {
    setPagandoCuenta(null);
    setMontoPago("");
    setFormaPago("EFECTIVO");
    setNumComprobante("");
    setObsPago("");
    setBancoId(undefined);
  };

  const handlePago = async (cuentaId: number) => {
    if (!montoPago || parseFloat(montoPago) <= 0) {
      toastError("Ingrese un monto valido");
      return;
    }
    try {
      await registrarPagoProveedor(
        cuentaId,
        parseFloat(montoPago),
        formaPago,
        numComprobante.trim() || undefined,
        obsPago.trim() || undefined,
        formaPago === "TRANSFERENCIA" ? bancoId : undefined,
      );
      toastExito("Pago registrado");
      resetPagoForm();
      // Refrescar cuentas del proveedor
      if (proveedorId) {
        const cuentas = await listarCuentasPagar(proveedorId);
        setCuentasProveedor(cuentas);
        if (cuentas.length === 0) {
          setVista("resumen");
        }
      }
      cargarResumen();
    } catch (err) {
      toastError("Error: " + err);
    }
  };

  // ==================== VISTA HISTORIAL ====================
  if (vista === "historial" && cuentaHistorial) {
    return (
      <>
        <div className="page-header">
          <div className="flex gap-2 items-center">
            <button className="btn btn-outline" onClick={() => {
              setVista("detalle");
              setCuentaHistorial(null);
              setPagosHistorial([]);
            }}>
              ← Volver
            </button>
            <h2>Historial de pagos - {cuentaHistorial.compra_numero ? `Compra #${cuentaHistorial.compra_numero}` : `Cuenta #${cuentaHistorial.id}`}</h2>
          </div>
          <span className="text-secondary">{cuentaHistorial.proveedor_nombre}</span>
        </div>
        <div className="page-body">
          <div className="flex gap-4 mb-4">
            <div className="card" style={{ flex: 1, maxWidth: 200 }}>
              <div className="card-body text-center">
                <span className="text-secondary" style={{ fontSize: 12 }}>Total</span>
                <div className="font-bold">${cuentaHistorial.monto_total.toFixed(2)}</div>
              </div>
            </div>
            <div className="card" style={{ flex: 1, maxWidth: 200 }}>
              <div className="card-body text-center">
                <span className="text-secondary" style={{ fontSize: 12 }}>Pagado</span>
                <div className="font-bold" style={{ color: "var(--color-success)" }}>
                  ${cuentaHistorial.monto_pagado.toFixed(2)}
                </div>
              </div>
            </div>
            <div className="card" style={{ flex: 1, maxWidth: 200 }}>
              <div className="card-body text-center">
                <span className="text-secondary" style={{ fontSize: 12 }}>Saldo</span>
                <div className="font-bold" style={{ color: cuentaHistorial.saldo > 0 ? "var(--color-danger)" : "var(--color-success)" }}>
                  ${cuentaHistorial.saldo.toFixed(2)}
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
                  <th>Banco</th>
                  <th>Comprobante</th>
                  <th>Observacion</th>
                </tr>
              </thead>
              <tbody>
                {pagosHistorial.length === 0 ? (
                  <tr>
                    <td colSpan={6} className="text-center text-secondary" style={{ padding: 40 }}>
                      Sin pagos registrados
                    </td>
                  </tr>
                ) : (
                  pagosHistorial.map((p) => (
                    <tr key={p.id}>
                      <td>
                        {p.fecha ? new Date(p.fecha).toLocaleDateString("es-EC", { day: "2-digit", month: "2-digit", year: "numeric", hour: "2-digit", minute: "2-digit" }) : "-"}
                      </td>
                      <td className="text-right font-bold">${p.monto.toFixed(2)}</td>
                      <td>
                        <span style={{
                          padding: "2px 8px", borderRadius: 4, fontSize: 11, fontWeight: 600,
                          background: p.forma_pago === "EFECTIVO" ? "rgba(34,197,94,0.15)" : "rgba(96,165,250,0.15)",
                          color: p.forma_pago === "EFECTIVO" ? "var(--color-success)" : "var(--color-primary)",
                        }}>
                          {p.forma_pago}
                        </span>
                      </td>
                      <td className="text-secondary">{p.banco_nombre || "-"}</td>
                      <td className="text-secondary">{p.numero_comprobante || "-"}</td>
                      <td className="text-secondary">{p.observacion || "-"}</td>
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

  // ==================== VISTA DETALLE POR PROVEEDOR ====================
  if (vista === "detalle") {
    const totalProveedor = cuentasProveedor.reduce((s, c) => s + c.saldo, 0);

    return (
      <>
        <div className="page-header">
          <div className="flex gap-2 items-center">
            <button className="btn btn-outline" onClick={() => { setVista("resumen"); cargarResumen(); }}>
              ← Volver
            </button>
            <h2>Cuentas por pagar - {proveedorNombre}</h2>
          </div>
          <span className="font-bold" style={{ color: "var(--color-danger)", fontSize: 18 }}>
            Deuda: ${totalProveedor.toFixed(2)}
          </span>
        </div>
        <div className="page-body">
          {cuentasProveedor.length === 0 ? (
            <div className="card">
              <div className="card-body text-center text-secondary" style={{ padding: 40 }}>
                Este proveedor no tiene cuentas pendientes
              </div>
            </div>
          ) : (
            <div style={{ display: "flex", flexDirection: "column", gap: 12 }}>
              {cuentasProveedor.map((cuenta) => (
                <div key={cuenta.id} className="card">
                  <div className="card-body">
                    <div className="flex justify-between items-center mb-2">
                      <div>
                        <strong>{cuenta.compra_numero ? `Compra #${cuenta.compra_numero}` : `Cuenta #${cuenta.id}`}</strong>
                        <span className="text-secondary" style={{ marginLeft: 12, fontSize: 12 }}>
                          {cuenta.created_at
                            ? new Date(cuenta.created_at).toLocaleDateString("es-EC")
                            : ""}
                        </span>
                        {cuenta.fecha_vencimiento && (
                          <span style={{
                            marginLeft: 12, fontSize: 11, padding: "2px 8px", borderRadius: 4,
                            background: new Date(cuenta.fecha_vencimiento) < new Date() ? "rgba(239,68,68,0.15)" : "rgba(250,204,21,0.15)",
                            color: new Date(cuenta.fecha_vencimiento) < new Date() ? "var(--color-danger)" : "var(--color-warning)",
                            fontWeight: 600,
                          }}>
                            Vence: {new Date(cuenta.fecha_vencimiento).toLocaleDateString("es-EC")}
                          </span>
                        )}
                      </div>
                      <div className="text-right">
                        <div className="text-secondary" style={{ fontSize: 12 }}>
                          Total: ${cuenta.monto_total.toFixed(2)} | Pagado: ${cuenta.monto_pagado.toFixed(2)}
                        </div>
                        <div className="font-bold" style={{ color: "var(--color-danger)", fontSize: 18 }}>
                          Saldo: ${cuenta.saldo.toFixed(2)}
                        </div>
                      </div>
                    </div>

                    {pagandoCuenta === cuenta.id ? (
                      <div style={{
                        background: "var(--color-bg)", padding: 12, borderRadius: "var(--radius)", marginTop: 8,
                      }}>
                        <div className="flex gap-2 items-end">
                          <div style={{ flex: 1 }}>
                            <label className="text-secondary" style={{ fontSize: 11 }}>Monto</label>
                            <input
                              className="input"
                              type="number"
                              step="0.01"
                              min="0.01"
                              max={cuenta.saldo}
                              placeholder={`Max: $${cuenta.saldo.toFixed(2)}`}
                              value={montoPago}
                              onChange={(e) => setMontoPago(e.target.value)}
                              autoFocus
                              onKeyDown={(e) => { if (e.key === "Enter") handlePago(cuenta.id!); }}
                            />
                          </div>
                          <div style={{ flex: 1 }}>
                            <label className="text-secondary" style={{ fontSize: 11 }}>Forma de pago</label>
                            <select className="input" value={formaPago} onChange={(e) => { setFormaPago(e.target.value); if (e.target.value !== "TRANSFERENCIA") setBancoId(undefined); }}>
                              <option value="EFECTIVO">Efectivo</option>
                              <option value="TRANSFERENCIA">Transferencia</option>
                              <option value="CHEQUE">Cheque</option>
                            </select>
                          </div>
                          {formaPago === "TRANSFERENCIA" && cuentasBanco.length > 0 && (
                            <div style={{ flex: 1 }}>
                              <label className="text-secondary" style={{ fontSize: 11 }}>Cuenta bancaria</label>
                              <select
                                className="input"
                                value={bancoId ?? ""}
                                onChange={(e) => setBancoId(e.target.value ? Number(e.target.value) : undefined)}
                              >
                                <option value="">-- Seleccionar --</option>
                                {cuentasBanco.filter(b => b.activa).map((b) => (
                                  <option key={b.id} value={b.id}>{b.nombre}{b.numero_cuenta ? ` (${b.numero_cuenta})` : ""}</option>
                                ))}
                              </select>
                            </div>
                          )}
                          <div style={{ flex: 1 }}>
                            <label className="text-secondary" style={{ fontSize: 11 }}>N. comprobante</label>
                            <input
                              className="input"
                              placeholder="Opcional"
                              value={numComprobante}
                              onChange={(e) => setNumComprobante(e.target.value)}
                            />
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
                        <div className="flex justify-between items-center mt-3">
                          <button
                            className="btn btn-outline"
                            style={{ fontSize: 12 }}
                            onClick={() => setMontoPago(cuenta.saldo.toFixed(2))}
                          >
                            Pagar todo (${cuenta.saldo.toFixed(2)})
                          </button>
                          <div className="flex gap-2">
                            <button className="btn btn-outline" onClick={resetPagoForm}>Cancelar</button>
                            <button className="btn btn-success" onClick={() => handlePago(cuenta.id!)}>
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
                          onClick={() => setPagandoCuenta(cuenta.id!)}
                        >
                          Pagar
                        </button>
                        {cuenta.monto_pagado > 0 && (
                          <button
                            className="btn btn-outline"
                            style={{ fontSize: 12 }}
                            onClick={() => verHistorial(cuenta)}
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

  // ==================== VISTA RESUMEN ====================
  return (
    <>
      <div className="page-header">
        <h2>Cuentas por Pagar</h2>
        {totalDeuda > 0 && (
          <span className="font-bold" style={{ color: "var(--color-danger)", fontSize: 16 }}>
            Total pendiente: ${totalDeuda.toFixed(2)}
          </span>
        )}
      </div>
      <div className="page-body">
        {/* Summary cards */}
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
              <span className="text-secondary" style={{ fontSize: 12 }}>Proveedores</span>
              <div className="text-xl font-bold">{acreedores.length}</div>
            </div>
          </div>
        </div>

        {/* Table of acreedores */}
        <div className="card">
          <table className="table">
            <thead>
              <tr>
                <th>Proveedor</th>
                <th className="text-center">Cuentas</th>
                <th className="text-right">Total Deuda</th>
                <th style={{ width: 120 }}></th>
              </tr>
            </thead>
            <tbody>
              {acreedores.length === 0 ? (
                <tr>
                  <td colSpan={4} className="text-center text-secondary" style={{ padding: 40 }}>
                    No hay cuentas pendientes con proveedores
                  </td>
                </tr>
              ) : (
                acreedores.map((a) => (
                  <tr key={a.proveedor_id}>
                    <td><strong>{a.proveedor_nombre}</strong></td>
                    <td className="text-center">{a.num_cuentas}</td>
                    <td className="text-right font-bold" style={{ color: "var(--color-danger)" }}>
                      ${a.total_deuda.toFixed(2)}
                    </td>
                    <td>
                      <button
                        className="btn btn-primary"
                        style={{ fontSize: 12 }}
                        onClick={() => verCuentas(a.proveedor_id, a.proveedor_nombre)}
                      >
                        Ver cuentas
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
