import { useState, useEffect } from "react";
import { listarClientes, crearCliente, actualizarCliente } from "../services/api";
import { useToast } from "../components/Toast";
import type { Cliente } from "../types";

export default function Clientes() {
  const { toastExito, toastError } = useToast();
  const [clientes, setClientes] = useState<Cliente[]>([]);
  const [mostrarForm, setMostrarForm] = useState(false);
  const [editando, setEditando] = useState<Cliente | undefined>();
  const [form, setForm] = useState<Cliente>({
    tipo_identificacion: "CEDULA",
    nombre: "",
    activo: true,
  });

  const cargar = async () => {
    setClientes(await listarClientes());
  };

  useEffect(() => { cargar(); }, []);

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

  return (
    <>
      <div className="page-header">
        <h2>Clientes ({clientes.length})</h2>
        <button className="btn btn-primary" onClick={abrirNuevo}>+ Nuevo Cliente</button>
      </div>
      <div className="page-body">
        {mostrarForm ? (
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
                    <input className="input" value={form.identificacion ?? ""}
                      onChange={(e) => setForm({ ...form, identificacion: e.target.value || undefined })} />
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
                  <div style={{ gridColumn: "1 / -1" }}>
                    <label className="text-secondary" style={{ fontSize: 12 }}>Dirección</label>
                    <input className="input" value={form.direccion ?? ""}
                      onChange={(e) => setForm({ ...form, direccion: e.target.value || undefined })} />
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
          <div className="card">
            <table className="table">
              <thead>
                <tr>
                  <th>Identificación</th>
                  <th>Nombre</th>
                  <th>Teléfono</th>
                  <th>Email</th>
                  <th></th>
                </tr>
              </thead>
              <tbody>
                {clientes.map((c) => (
                  <tr key={c.id}>
                    <td>{c.identificacion ?? "-"}</td>
                    <td><strong>{c.nombre}</strong></td>
                    <td className="text-secondary">{c.telefono ?? "-"}</td>
                    <td className="text-secondary">{c.email ?? "-"}</td>
                    <td>
                      <button className="btn btn-outline" onClick={() => abrirEditar(c)}>Editar</button>
                    </td>
                  </tr>
                ))}
                {clientes.length === 0 && (
                  <tr>
                    <td colSpan={5} className="text-center text-secondary" style={{ padding: 40 }}>
                      No hay clientes registrados
                    </td>
                  </tr>
                )}
              </tbody>
            </table>
          </div>
        )}
      </div>
    </>
  );
}
