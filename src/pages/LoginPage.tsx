import { useState, useEffect } from "react";
import { iniciarSesion, obtenerConfig, listarUsuariosLogin } from "../services/api";
import type { SesionActiva } from "../types";

interface Novedad {
  titulo: string;
  descripcion: string;
  icono: string;
}

interface PromoData {
  imagen_url?: string;
  novedades: Novedad[];
  links: { label: string; url: string }[];
  version_actual?: string;
}

const NOVEDADES_DEFAULT: PromoData = {
  novedades: [
    { titulo: "Multi-Terminal en Red", descripcion: "Conecte varios puntos de venta a una base de datos centralizada", icono: "🖥️" },
    { titulo: "Respaldo en la Nube", descripcion: "Respalde automaticamente su base de datos en Google Drive o servidor Clouget", icono: "☁️" },
    { titulo: "Multi-Almacen", descripcion: "Gestione stock por establecimiento y venda entre locales", icono: "📦" },
    { titulo: "Consulta Cedula/RUC", descripcion: "Busque datos de clientes automaticamente desde el SRI", icono: "🔍" },
  ],
  links: [
    { label: "Ver todas las caracteristicas", url: "https://pos.clouget.com" },
    { label: "Tutoriales y guias", url: "https://pos.clouget.com/tutoriales" },
  ],
};

interface Props {
  onLogin: (sesion: SesionActiva) => void;
  esDemo?: boolean;
}

export default function LoginPage({ onLogin, esDemo }: Props) {
  const [pin, setPin] = useState("");
  const [error, setError] = useState("");
  const [shake, setShake] = useState(false);
  const [cargando, setCargando] = useState(false);
  const [promo, setPromo] = useState<PromoData>(NOVEDADES_DEFAULT);
  // Modo login: pin, password, ambos
  const [modoLogin, setModoLogin] = useState<string>("pin");
  const [modoActivo, setModoActivo] = useState<"pin" | "password">("pin"); // para tabs en modo 'ambos'
  const [passwordInput, setPasswordInput] = useState("");
  const [usuariosLista, setUsuariosLista] = useState<[number, string][]>([]);
  const [usuarioSeleccionado, setUsuarioSeleccionado] = useState<number>(0);

  useEffect(() => {
    // Cargar modo_login desde config
    obtenerConfig().then((cfg) => {
      const modo = cfg.modo_login || "pin";
      setModoLogin(modo);
      if (modo === "password") {
        setModoActivo("password");
      }
    }).catch(() => {});

    // Cargar lista de usuarios para modo password
    listarUsuariosLogin().then((usrs) => {
      setUsuariosLista(usrs);
      if (usrs.length > 0) {
        setUsuarioSeleccionado(usrs[0][0]);
      }
    }).catch(() => {});

    // Intentar cargar promo desde Supabase (si hay internet)
    const SUPABASE_URL = "https://zakquzflkvfqflqnxpxj.supabase.co";
    const SUPABASE_ANON = "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJpc3MiOiJzdXBhYmFzZSIsInJlZiI6Inpha3F1emZsa3ZmcWZscW54cHhqIiwicm9sZSI6ImFub24iLCJpYXQiOjE3MzY2MDcxNjQsImV4cCI6MjA1MjE4MzE2NH0.sxaKNMkNguqQnvmUXh2JVRjqXDDqgsKb2LKPSGFp9bE";
    fetch(`${SUPABASE_URL}/rest/v1/configuracion_global?clave=eq.login_promo&select=valor`, {
      headers: { apikey: SUPABASE_ANON, Authorization: `Bearer ${SUPABASE_ANON}` },
    })
      .then((r) => r.json())
      .then((rows: { valor: string }[]) => {
        if (rows?.[0]?.valor) {
          const data = JSON.parse(rows[0].valor) as PromoData & { habilitado?: boolean };
          if (data.habilitado === false) {
            setPromo({ ...NOVEDADES_DEFAULT, novedades: [], links: [] }); // ocultar
          } else if (data.novedades?.length) {
            setPromo(data);
          }
        }
      })
      .catch(() => {
        // Sin internet, usar default local
      });
  }, []);

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

  const handleLoginPassword = async () => {
    if (!passwordInput || passwordInput.length < 6) {
      setError("Ingrese una contraseña valida (min 6 caracteres)");
      return;
    }
    if (!usuarioSeleccionado) {
      setError("Seleccione un usuario");
      return;
    }
    setCargando(true);
    try {
      const sesion = await iniciarSesion("", passwordInput);
      onLogin(sesion);
    } catch (err: any) {
      setError(String(err) || "Contraseña incorrecta");
      setShake(true);
      setTimeout(() => setShake(false), 500);
      setPasswordInput("");
    }
    setCargando(false);
  };

  const handleKeyDown = (e: React.KeyboardEvent) => {
    // Only handle numeric keypad in PIN mode
    const enModoPinActivo = modoLogin === "pin" || (modoLogin === "ambos" && modoActivo === "pin");
    if (!enModoPinActivo) return;
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
        background: "linear-gradient(135deg, #0f172a 0%, #1e293b 100%)",
        color: "white",
      }}
      tabIndex={0}
      onKeyDown={handleKeyDown}
      autoFocus
    >
      {/* Panel izquierdo - PIN */}
      <div
        style={{
          width: promo.novedades.length > 0 ? 380 : "100%",
          minWidth: promo.novedades.length > 0 ? 380 : undefined,
          display: "flex",
          alignItems: "center",
          justifyContent: "center",
          flexDirection: "column",
        }}
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

        {/* Tabs para modo 'ambos' */}
        {modoLogin === "ambos" && (
          <div style={{ display: "flex", gap: 0, marginBottom: 24, borderRadius: 8, overflow: "hidden", border: "1px solid rgba(255,255,255,0.15)" }}>
            <button
              onClick={() => { setModoActivo("pin"); setError(""); setPasswordInput(""); }}
              style={{
                flex: 1, padding: "8px 16px", fontSize: 13, fontWeight: 600, cursor: "pointer",
                border: "none",
                background: modoActivo === "pin" ? "var(--color-primary)" : "rgba(255,255,255,0.06)",
                color: "white",
              }}
            >PIN</button>
            <button
              onClick={() => { setModoActivo("password"); setError(""); setPin(""); }}
              style={{
                flex: 1, padding: "8px 16px", fontSize: 13, fontWeight: 600, cursor: "pointer",
                border: "none",
                background: modoActivo === "password" ? "var(--color-primary)" : "rgba(255,255,255,0.06)",
                color: "white",
              }}
            >Contrasena</button>
          </div>
        )}

        {/* Error */}
        <div
          style={{
            height: 24,
            fontSize: 13,
            color: "var(--color-danger)",
            marginBottom: 16,
          }}
        >
          {error}
        </div>

        {/* PIN mode */}
        {modoActivo === "pin" && (
          <>
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
                    background: i < pin.length ? "var(--color-primary)" : "transparent",
                    transition: "background 0.15s",
                  }}
                />
              ))}
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
                marginTop: 24,
              }}
            >
              Ingrese su PIN para iniciar sesion
            </p>

            {modoLogin === "pin" && (
              <button
                onClick={() => setModoActivo("password")}
                style={{
                  marginTop: 12, background: "none", border: "none", color: "rgba(255,255,255,0.4)",
                  fontSize: 12, cursor: "pointer", textDecoration: "underline",
                }}
              >
                Iniciar con usuario y contraseña
              </button>
            )}
          </>
        )}

        {/* Password mode */}
        {(modoLogin === "password" || modoActivo === "password") && (
          <div className={shake ? "login-shake" : ""} style={{ display: "flex", flexDirection: "column", gap: 12, maxWidth: 280, margin: "0 auto" }}>
            <div style={{ textAlign: "left" }}>
              <label style={{ fontSize: 12, opacity: 0.6, marginBottom: 4, display: "block" }}>Usuario</label>
              <select
                style={{
                  width: "100%", padding: "10px 12px", borderRadius: 8,
                  border: "1px solid rgba(255,255,255,0.15)",
                  background: "rgba(255,255,255,0.06)",
                  color: "white", fontSize: 14,
                }}
                value={usuarioSeleccionado}
                onChange={(e) => setUsuarioSeleccionado(Number(e.target.value))}
              >
                {usuariosLista.map(([id, nombre]) => (
                  <option key={id} value={id} style={{ background: "#1e293b", color: "white" }}>{nombre}</option>
                ))}
              </select>
            </div>
            <div style={{ textAlign: "left" }}>
              <label style={{ fontSize: 12, opacity: 0.6, marginBottom: 4, display: "block" }}>Contrasena</label>
              <input
                type="password"
                style={{
                  width: "100%", padding: "10px 12px", borderRadius: 8,
                  border: "1px solid rgba(255,255,255,0.15)",
                  background: "rgba(255,255,255,0.06)",
                  color: "white", fontSize: 14,
                  boxSizing: "border-box",
                }}
                value={passwordInput}
                onChange={(e) => { setPasswordInput(e.target.value); setError(""); }}
                onKeyDown={(e) => { if (e.key === "Enter") handleLoginPassword(); }}
                autoFocus
                placeholder="Ingrese su contrasena"
              />
            </div>
            <button
              className="login-key login-key-enter"
              style={{ width: "100%", marginTop: 8 }}
              onClick={handleLoginPassword}
              disabled={cargando || !passwordInput}
            >
              {cargando ? "..." : "Iniciar Sesion"}
            </button>

            {modoLogin === "pin" && (
              <button
                onClick={() => { setModoActivo("pin"); setError(""); setPasswordInput(""); }}
                style={{
                  marginTop: 12, background: "none", border: "none", color: "rgba(255,255,255,0.4)",
                  fontSize: 12, cursor: "pointer", textDecoration: "underline",
                }}
              >
                Volver a PIN
              </button>
            )}
          </div>
        )}

        {esDemo && (
          <div
            style={{
              marginTop: 16,
              padding: "10px 16px",
              background: "rgba(245, 158, 11, 0.15)",
              border: "1px solid rgba(245, 158, 11, 0.3)",
              borderRadius: 8,
              fontSize: 12,
              color: "var(--color-warning)",
              lineHeight: 1.5,
            }}
          >
            <strong>Demo:</strong> Admin PIN <strong>1234</strong> | Cajero PIN <strong>0000</strong>
          </div>
        )}
      </div>
      </div>

      {/* Panel derecho - Promociones y Novedades (solo si hay contenido) */}
      {promo.novedades.length > 0 && (
      <div
        style={{
          flex: 1,
          display: "flex",
          flexDirection: "column",
          justifyContent: "center",
          padding: "40px 48px",
          background: "linear-gradient(135deg, rgba(37,99,235,0.08) 0%, rgba(59,130,246,0.04) 100%)",
          borderLeft: "1px solid rgba(255,255,255,0.06)",
          overflow: "auto",
        }}
      >
        {/* Header */}
        <div style={{ marginBottom: 32 }}>
          <h2 style={{ fontSize: 22, fontWeight: 700, margin: "0 0 6px 0", color: "rgba(255,255,255,0.9)" }}>
            Novedades
          </h2>
          <p style={{ fontSize: 13, opacity: 0.5, margin: 0 }}>
            Ultimas mejoras de Clouget POS
          </p>
        </div>

        {/* Novedades Grid */}
        <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: 12, marginBottom: 28 }}>
          {promo.novedades.map((n, i) => (
            <div
              key={i}
              style={{
                padding: "16px 14px",
                background: "rgba(255,255,255,0.04)",
                border: "1px solid rgba(255,255,255,0.08)",
                borderRadius: 10,
                transition: "background 0.2s",
              }}
            >
              <div style={{ fontSize: 24, marginBottom: 8 }}>{n.icono}</div>
              <div style={{ fontSize: 13, fontWeight: 600, color: "rgba(255,255,255,0.85)", marginBottom: 4 }}>
                {n.titulo}
              </div>
              <div style={{ fontSize: 11, color: "rgba(255,255,255,0.45)", lineHeight: 1.4 }}>
                {n.descripcion}
              </div>
            </div>
          ))}
        </div>

        {/* Links */}
        <div style={{ display: "flex", flexDirection: "column", gap: 8 }}>
          {promo.links.map((link, i) => (
            <a
              key={i}
              href={link.url}
              target="_blank"
              rel="noopener noreferrer"
              style={{
                display: "flex",
                alignItems: "center",
                gap: 8,
                padding: "10px 14px",
                background: "rgba(59,130,246,0.1)",
                border: "1px solid rgba(59,130,246,0.2)",
                borderRadius: 8,
                color: "var(--color-primary-light, #93c5fd)",
                fontSize: 13,
                fontWeight: 500,
                textDecoration: "none",
                transition: "background 0.2s",
              }}
            >
              <span style={{ fontSize: 14 }}>→</span>
              {link.label}
            </a>
          ))}
        </div>

        {/* Footer */}
        <div style={{ marginTop: 32, fontSize: 11, opacity: 0.25 }}>
          pos.clouget.com
        </div>
      </div>
      )}

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
          background: var(--color-primary);
          border-color: var(--color-primary);
          font-size: 16px;
        }
        .login-key-enter:hover:not(:disabled) {
          background: var(--color-primary-hover, #2563eb);
          border-color: var(--color-primary-hover, #2563eb);
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
