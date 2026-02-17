import { useState, useEffect } from "react";
import { resumenDeudores, listarCuentasPendientes, registrarPagoCuenta } from "../services/api";
import { useToast } from "../components/Toast";
import type { ResumenCliente, CuentaConCliente } from "../types";

export default function CuentasPage() {
  const { toastExito, toastError } = useToast();
  const [vista, setVista] = useState<"resumen" | "detalle">("resumen");
  const [deudores, setDeudores] = useState<ResumenCliente[]>([]);
  const [cuentasCliente, setCuentasCliente] = useState<CuentaConCliente[]>([]);
  const [clienteNombre, setClienteNombre] = useState("");
  const [clienteId, setClienteId] = useState<number | null>(null);

  // Pago form
  const [pagandoCuenta, setPagandoCuenta] = useState<number | null>(null);
  const [montoPago, setMontoPago] = useState("");
  const [obsPago, setObsPago] = useState("");

  const totalDeuda = deudores.reduce((s, d) => s + d.total_deuda, 0);
  const totalCuentas = deudores.reduce((s, d) => s + d.num_cuentas, 0);

  const cargarResumen = async () => {
    try {
      setDeudores(await resumenDeudores());
    } catch (err) {
      toastError("Error: " + err);
    }
  };

  useEffect(() => { cargarResumen(); }, []);

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

  const handlePago = async (cuentaId: number) => {
    if (!montoPago || parseFloat(montoPago) <= 0) {
      toastError("Ingrese un monto valido");
      return;
    }
    try {
      await registrarPagoCuenta({
        cuenta_id: cuentaId,
        monto: parseFloat(montoPago),
        observacion: obsPago.trim() || undefined,
      });
      toastExito("Pago registrado");
      setPagandoCuenta(null);
      setMontoPago("");
      setObsPago("");
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

  // Vista detalle por cliente
  if (vista === "detalle") {
    const totalCliente = cuentasCliente.reduce((s, c) => s + c.cuenta.saldo, 0);

    return (
      <>
        <div className="page-header">
          <div className="flex gap-2 items-center">
            <button className="btn btn-outline" onClick={() => { setVista("resumen"); cargarResumen(); }}>
              ← Volver
            </button>
            <h2>Fiados - {clienteNombre}</h2>
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
                        <div className="flex gap-2 items-center">
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
                            <label className="text-secondary" style={{ fontSize: 11 }}>Observacion</label>
                            <input
                              className="input"
                              placeholder="Opcional"
                              value={obsPago}
                              onChange={(e) => setObsPago(e.target.value)}
                            />
                          </div>
                          <div style={{ paddingTop: 16 }}>
                            <div className="flex gap-2">
                              <button className="btn btn-outline" onClick={() => { setPagandoCuenta(null); setMontoPago(""); setObsPago(""); }}>
                                Cancelar
                              </button>
                              <button className="btn btn-success" onClick={() => handlePago(cc.cuenta.id!)}>
                                Registrar Pago
                              </button>
                            </div>
                          </div>
                        </div>
                        <button
                          className="btn btn-outline mt-2"
                          style={{ fontSize: 12 }}
                          onClick={() => setMontoPago(cc.cuenta.saldo.toFixed(2))}
                        >
                          Pagar todo (${cc.cuenta.saldo.toFixed(2)})
                        </button>
                      </div>
                    ) : (
                      <button
                        className="btn btn-primary mt-2"
                        style={{ fontSize: 12 }}
                        onClick={() => setPagandoCuenta(cc.cuenta.id!)}
                      >
                        Registrar Pago
                      </button>
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

  // Vista resumen de deudores
  return (
    <>
      <div className="page-header">
        <h2>Cuentas por Cobrar</h2>
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
