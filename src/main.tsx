import React, { useState, useEffect } from "react";
import ReactDOM from "react-dom/client";
import { BrowserRouter, Routes, Route } from "react-router-dom";
import { ToastProvider } from "./components/Toast";
import { SesionProvider, useSesion } from "./contexts/SesionContext";
import Layout from "./components/Layout";
import DashboardPage from "./pages/DashboardPage";
import PuntoVenta from "./pages/PuntoVenta";
import Productos from "./pages/Productos";
import Clientes from "./pages/Clientes";
import VentasDia from "./pages/VentasDia";
import CajaPage from "./pages/CajaPage";
import GastosPage from "./pages/GastosPage";
import CuentasPage from "./pages/CuentasPage";
import Configuracion from "./pages/Configuracion";
import InventarioPage from "./pages/InventarioPage";
import LicenciaPage from "./pages/LicenciaPage";
import LoginPage from "./pages/LoginPage";
import { obtenerEstadoLicencia, obtenerSesionActual } from "./services/api";
import type { LicenciaInfo } from "./types";
import "./styles/global.css";

function AppGate() {
  const [licencia, setLicencia] = useState<LicenciaInfo | null>(null);
  const [verificando, setVerificando] = useState(true);
  const { sesion, setSesion } = useSesion();

  useEffect(() => {
    Promise.all([obtenerEstadoLicencia(), obtenerSesionActual()])
      .then(([lic, ses]) => {
        setLicencia(lic);
        if (ses) setSesion(ses);
        setVerificando(false);
      })
      .catch(() => {
        setVerificando(false);
      });
  }, []);

  if (verificando) {
    return (
      <div
        style={{
          minHeight: "100vh",
          display: "flex",
          alignItems: "center",
          justifyContent: "center",
          background: "#0f172a",
          color: "white",
          fontSize: 18,
        }}
      >
        <div style={{ textAlign: "center" }}>
          <h1 style={{ fontSize: 32, fontWeight: 800, margin: "0 0 8px 0" }}>CLOUGET</h1>
          <p style={{ opacity: 0.6, margin: 0 }}>Cargando...</p>
        </div>
      </div>
    );
  }

  if (!licencia) {
    return <LicenciaPage onActivada={(lic) => setLicencia(lic)} />;
  }

  if (!sesion) {
    return <LoginPage onLogin={(s) => setSesion(s)} />;
  }

  return (
    <BrowserRouter>
      <Routes>
        <Route element={<Layout />}>
          {/* Rutas disponibles para todos los roles */}
          <Route path="/" element={<DashboardPage />} />
          <Route path="/pos" element={<PuntoVenta />} />
          <Route path="/caja" element={<CajaPage />} />
          <Route path="/ventas" element={<VentasDia />} />
          {/* Rutas solo para ADMIN */}
          {sesion.rol === "ADMIN" && (
            <>
              <Route path="/productos" element={<Productos />} />
              <Route path="/clientes" element={<Clientes />} />
              <Route path="/gastos" element={<GastosPage />} />
              <Route path="/cuentas" element={<CuentasPage />} />
              <Route path="/inventario" element={<InventarioPage />} />
              <Route path="/config" element={<Configuracion />} />
            </>
          )}
        </Route>
      </Routes>
    </BrowserRouter>
  );
}

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <ToastProvider>
      <SesionProvider>
        <AppGate />
      </SesionProvider>
    </ToastProvider>
  </React.StrictMode>
);
