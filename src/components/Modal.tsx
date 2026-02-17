interface ModalProps {
  abierto: boolean;
  titulo: string;
  mensaje: string;
  tipo?: "peligro" | "normal";
  textoConfirmar?: string;
  textoCancelar?: string;
  onConfirmar: () => void;
  onCancelar: () => void;
}

export default function Modal({
  abierto,
  titulo,
  mensaje,
  tipo = "normal",
  textoConfirmar = "Confirmar",
  textoCancelar = "Cancelar",
  onConfirmar,
  onCancelar,
}: ModalProps) {
  if (!abierto) return null;

  return (
    <div className="modal-overlay" onClick={onCancelar}>
      <div className="modal-content" onClick={(e) => e.stopPropagation()}>
        <div className="modal-header">
          <h3>{titulo}</h3>
        </div>
        <div className="modal-body">
          <p>{mensaje}</p>
        </div>
        <div className="modal-footer">
          <button className="btn btn-outline" onClick={onCancelar}>
            {textoCancelar}
          </button>
          <button
            className={`btn ${tipo === "peligro" ? "btn-danger" : "btn-primary"}`}
            onClick={onConfirmar}
          >
            {textoConfirmar}
          </button>
        </div>
      </div>
    </div>
  );
}
