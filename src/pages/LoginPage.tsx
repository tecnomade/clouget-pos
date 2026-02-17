import { useState } from "react";
import { iniciarSesion } from "../services/api";
import type { SesionActiva } from "../types";

interface Props {
  onLogin: (sesion: SesionActiva) => void;
}

export default function LoginPage({ onLogin }: Props) {
  const [pin, setPin] = useState("");
  const [error, setError] = useState("");
  const [shake, setShake] = useState(false);
  const [cargando, setCargando] = useState(false);

  const handleDigit = (d: string) => {
    if (pin.length >= 6) return;
    setPin((prev) => prev + d);
    setError("");
  };

  const handleDelete = () => {
    setPin((prev) => prev.slice(0, -1));
    setError("");
  };

  const handleSubmit = async () => {
    if (pin.length < 4) {
      setError("Ingrese al menos 4 digitos");
      return;
    }
    setCargando(true);
    try {
      const sesion = await iniciarSesion(pin);
      onLogin(sesion);
    } catch (err) {
      setError("PIN incorrecto");
      setShake(true);
      setTimeout(() => setShake(false), 500);
      setPin("");
    }
    setCargando(false);
  };

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key >= "0" && e.key <= "9") {
      handleDigit(e.key);
    } else if (e.key === "Backspace") {
      handleDelete();
    } else if (e.key === "Enter") {
      handleSubmit();
    }
  };

  return (
    <div
      style={{
        minHeight: "100vh",
        display: "flex",
        alignItems: "center",
        justifyContent: "center",
        background: "linear-gradient(135deg, #0f172a 0%, #1e293b 100%)",
        color: "white",
      }}
      tabIndex={0}
      onKeyDown={handleKeyDown}
      autoFocus
    >
      <div
        style={{
          width: 340,
          textAlign: "center",
        }}
      >
        {/* Logo */}
        <h1
          style={{
            fontSize: 36,
            fontWeight: 800,
            letterSpacing: 2,
            margin: "0 0 4px 0",
          }}
        >
          CLOUGET
        </h1>
        <p style={{ opacity: 0.5, fontSize: 13, margin: "0 0 40px 0" }}>
          Punto de Venta
        </p>

        {/* PIN dots */}
        <div
          className={shake ? "login-shake" : ""}
          style={{
            display: "flex",
            justifyContent: "center",
            gap: 12,
            marginBottom: 16,
          }}
        >
          {[0, 1, 2, 3, 4, 5].map((i) => (
            <div
              key={i}
              style={{
                width: 16,
                height: 16,
                borderRadius: "50%",
                border: "2px solid rgba(255,255,255,0.3)",
                background: i < pin.length ? "#3b82f6" : "transparent",
                transition: "background 0.15s",
              }}
            />
          ))}
        </div>

        {/* Error */}
        <div
          style={{
            height: 24,
            fontSize: 13,
            color: "#ef4444",
            marginBottom: 16,
          }}
        >
          {error}
        </div>

        {/* Teclado numerico */}
        <div
          style={{
            display: "grid",
            gridTemplateColumns: "repeat(3, 1fr)",
            gap: 8,
            maxWidth: 260,
            margin: "0 auto",
          }}
        >
          {[1, 2, 3, 4, 5, 6, 7, 8, 9].map((d) => (
            <button
              key={d}
              className="login-key"
              onClick={() => handleDigit(String(d))}
              disabled={cargando}
            >
              {d}
            </button>
          ))}
          <button
            className="login-key login-key-secondary"
            onClick={handleDelete}
            disabled={cargando}
          >
            ←
          </button>
          <button
            className="login-key"
            onClick={() => handleDigit("0")}
            disabled={cargando}
          >
            0
          </button>
          <button
            className="login-key login-key-enter"
            onClick={handleSubmit}
            disabled={cargando || pin.length < 4}
          >
            {cargando ? "..." : "OK"}
          </button>
        </div>

        <p
          style={{
            fontSize: 11,
            opacity: 0.3,
            marginTop: 32,
          }}
        >
          Ingrese su PIN para iniciar sesion
        </p>
      </div>

      <style>{`
        .login-key {
          width: 100%;
          height: 56px;
          border: 1px solid rgba(255,255,255,0.15);
          border-radius: 12px;
          background: rgba(255,255,255,0.06);
          color: white;
          font-size: 22px;
          font-weight: 600;
          cursor: pointer;
          transition: all 0.15s;
          display: flex;
          align-items: center;
          justify-content: center;
        }
        .login-key:hover:not(:disabled) {
          background: rgba(255,255,255,0.12);
          border-color: rgba(255,255,255,0.3);
        }
        .login-key:active:not(:disabled) {
          background: rgba(255,255,255,0.18);
          transform: scale(0.95);
        }
        .login-key:disabled {
          opacity: 0.4;
          cursor: default;
        }
        .login-key-secondary {
          font-size: 24px;
          color: rgba(255,255,255,0.6);
        }
        .login-key-enter {
          background: #3b82f6;
          border-color: #3b82f6;
          font-size: 16px;
        }
        .login-key-enter:hover:not(:disabled) {
          background: #2563eb;
          border-color: #2563eb;
        }
        .login-key-enter:disabled {
          background: rgba(59,130,246,0.3);
          border-color: rgba(59,130,246,0.3);
        }
        @keyframes shake {
          0%, 100% { transform: translateX(0); }
          20% { transform: translateX(-10px); }
          40% { transform: translateX(10px); }
          60% { transform: translateX(-10px); }
          80% { transform: translateX(10px); }
        }
        .login-shake {
          animation: shake 0.4s ease;
        }
      `}</style>
    </div>
  );
}
