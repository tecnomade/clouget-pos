import React from "react";

interface State {
  hasError: boolean;
  error: Error | null;
}

export default class ErrorBoundary extends React.Component<
  { children: React.ReactNode },
  State
> {
  constructor(props: { children: React.ReactNode }) {
    super(props);
    this.state = { hasError: false, error: null };
  }

  static getDerivedStateFromError(error: Error): State {
    return { hasError: true, error };
  }

  componentDidCatch(error: Error, info: React.ErrorInfo) {
    console.error("[ErrorBoundary] Error capturado:", error, info.componentStack);
  }

  render() {
    if (this.state.hasError) {
      return (
        <div
          style={{
            minHeight: "100vh",
            display: "flex",
            alignItems: "center",
            justifyContent: "center",
            background: "var(--color-bg, #0f172a)",
            color: "white",
            padding: 32,
          }}
        >
          <div style={{ textAlign: "center", maxWidth: 500 }}>
            <h1 style={{ fontSize: 28, fontWeight: 800, marginBottom: 12 }}>
              CLOUGET
            </h1>
            <p style={{ color: "var(--color-danger)", fontSize: 16, marginBottom: 16 }}>
              Ocurrio un error inesperado
            </p>
            <pre
              style={{
                background: "var(--color-surface, #1e293b)",
                padding: 16,
                borderRadius: 8,
                fontSize: 12,
                textAlign: "left",
                overflow: "auto",
                maxHeight: 200,
                color: "var(--color-danger)",
                marginBottom: 16,
              }}
            >
              {this.state.error?.message || "Error desconocido"}
            </pre>
            <button
              onClick={() => window.location.reload()}
              style={{
                padding: "8px 24px",
                background: "var(--color-primary)",
                color: "white",
                border: "none",
                borderRadius: 6,
                fontSize: 14,
                fontWeight: 600,
                cursor: "pointer",
              }}
            >
              Recargar Aplicacion
            </button>
          </div>
        </div>
      );
    }

    return this.props.children;
  }
}
