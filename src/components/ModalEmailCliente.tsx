import { useState } from "react";

interface ModalEmailClienteProps {
  abierto: boolean;
  clienteNombre: string;
  ventaNumero: string;
  onEnviar: (email: string) => void;
  onOmitir: () => void;
  enviando?: boolean;
}

export default function ModalEmailCliente({
  abierto,
  clienteNombre,
  ventaNumero,
  onEnviar,
  onOmitir,
  enviando = false,
}: ModalEmailClienteProps) {
  const [email, setEmail] = useState("");
  const [error, setError] = useState("");

  if (!abierto) return null;

  const validarEmail = (e: string) => /^[^\s@]+@[^\s@]+\.[^\s@]+$/.test(e);

  const handleEnviar = () => {
    if (!email.trim()) {
      setError("Ingrese un email");
      return;
    }
    if (!validarEmail(email.trim())) {
      setError("Email invalido");
      return;
    }
    setError("");
    onEnviar(email.trim());
  };

  return (
    <div className="modal-overlay" onClick={onOmitir}>
      <div className="modal-content" onClick={(e) => e.stopPropagation()} style={{ maxWidth: 420 }}>
        <div className="modal-header">
          <h3>Enviar Factura por Email</h3>
        </div>
        <div className="modal-body">
          <p style={{ fontSize: 13, color: "#64748b", marginBottom: 12 }}>
            Factura <strong>{ventaNumero}</strong> autorizada por el SRI.
            {clienteNombre && <> Cliente: <strong>{clienteNombre}</strong>.</>}
          </p>
          <p style={{ fontSize: 13, marginBottom: 8 }}>
            Ingrese el email para enviar el RIDE (PDF) y XML:
          </p>
          <input
            className="input"
            type="email"
            placeholder="cliente@email.com"
            value={email}
            onChange={(e) => { setEmail(e.target.value); setError(""); }}
            onKeyDown={(e) => { if (e.key === "Enter") handleEnviar(); }}
            autoFocus
            disabled={enviando}
          />
          {error && <div style={{ color: "#dc2626", fontSize: 12, marginTop: 4 }}>{error}</div>}
        </div>
        <div className="modal-footer">
          <button className="btn btn-outline" onClick={onOmitir} disabled={enviando}>
            Omitir
          </button>
          <button className="btn btn-primary" onClick={handleEnviar} disabled={enviando}>
            {enviando ? "Enviando..." : "Enviar y Guardar"}
          </button>
        </div>
      </div>
    </div>
  );
}
