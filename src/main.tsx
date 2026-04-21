import React, { useState, useEffect } from "react";
import ReactDOM from "react-dom/client";
import { BrowserRouter, Routes, Route } from "react-router-dom";
import { ToastProvider } from "./components/Toast";
import { SesionProvider, useSesion } from "./contexts/SesionContext";
import { DemoProvider } from "./contexts/DemoContext";
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
import GuiasRemisionPage from "./pages/GuiasRemisionPage";
import ReportesPage from "./pages/ReportesPage";
import ComprasPage from "./pages/ComprasPage";
import PagarPage from "./pages/PagarPage";
import MovimientosBancariosPage from "./pages/MovimientosBancariosPage";
import SeriesPage from "./pages/SeriesPage";
import CaducidadPage from "./pages/CaducidadPage";
import ServicioTecnicoPage from "./pages/ServicioTecnicoPage";
import LicenciaPage from "./pages/LicenciaPage";
import LoginPage from "./pages/LoginPage";
import { obtenerEstadoLicencia, obtenerSesionActual, obtenerConfig, configurarModoRed } from "./services/api";
import { iniciarSyncService, sincronizarCacheProductos, reservarSecuenciales } from "./services/offlineSync";
import ConnectionStatus from "./components/ConnectionStatus";
import type { LicenciaInfo } from "./types";
import ErrorBoundary from "./components/ErrorBoundary";
import "./styles/global.css";

// Auto-select all text in number inputs on focus (global)
document.addEventListener("focusin", (e) => {
  const target = e.target as HTMLInputElement;
  if (target?.tagName === "INPUT" && target.type === "number") {
    setTimeout(() => target.select(), 0);
  }
});

function AppGate() {
  const [licencia, setLicencia] = useState<LicenciaInfo | null>(null);
  const [verificando, setVerificando] = useState(true);
  const { sesion, setSesion } = useSesion();

  useEffect(() => {
    // Inicializar modo red antes de cualquier otra llamada
    obtenerConfig().then(async (cfg) => {
      if (cfg.modo_red === 'cliente' && cfg.servidor_url && cfg.servidor_token) {
        configurarModoRed('cliente', cfg.servidor_url, cfg.servidor_token);
        // Iniciar servicio de sincronización background
        iniciarSyncService(cfg.servidor_url, cfg.servidor_token);
        // Obtener licencia y módulos del servidor + sincronizar cache
        try {
          // Obtener módulos de la licencia del servidor
          const resp = await fetch(`${cfg.servidor_url}/api/v1/invoke`, {
            method: 'POST',
            headers: { 'Content-Type': 'application/json', 'Authorization': `Bearer ${cfg.servidor_token}` },
            body: JSON.stringify({ command: 'obtener_licencia_servidor', args: {} }),
          });
          const licData = await resp.json();
          if (licData.ok && licData.data?.modulos) {
            // Guardar módulos del servidor en config local del cliente
            const { guardarConfig: gc } = await import('./services/api');
            await gc({ licencia_modulos: JSON.stringify(licData.data.modulos) });
          }

          await sincronizarCacheProductos(cfg.servidor_url, cfg.servidor_token);
          const est = cfg.terminal_establecimiento || '001';
          const pe = cfg.terminal_punto_emision || '001';
          await reservarSecuenciales(cfg.servidor_url, cfg.servidor_token, est, pe, 'NOTA_VENTA', 50);
        } catch { /* offline desde el inicio */ }
      }
    }).catch(() => {}).finally(() => {
      // Luego verificar licencia y sesion
      Promise.all([obtenerEstadoLicencia(), obtenerSesionActual()])
        .then(([lic, ses]) => {
          setLicencia(lic);
          if (ses) setSesion(ses);
          setVerificando(false);
        })
        .catch(() => {
          setVerificando(false);
        });
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
    return <LoginPage onLogin={(s) => setSesion(s)} esDemo={licencia.tipo === "demo"} />;
  }

  return (
    <DemoProvider>
      <BrowserRouter>
        <Routes>
          <Route element={<Layout />}>
            {/* Rutas disponibles para todos los roles */}
            <Route path="/" element={<DashboardPage />} />
            <Route path="/pos" element={<PuntoVenta />} />
            <Route path="/caja" element={<CajaPage />} />
            <Route path="/ventas" element={<VentasDia />} />
            <Route path="/cuentas" element={<CuentasPage />} />
            {/* Rutas solo para ADMIN */}
            {sesion.rol === "ADMIN" && (
              <>
                <Route path="/productos" element={<Productos />} />
                <Route path="/clientes" element={<Clientes />} />
                <Route path="/guias" element={<GuiasRemisionPage />} />
                <Route path="/gastos" element={<GastosPage />} />
                <Route path="/compras" element={<ComprasPage />} />
                <Route path="/pagar" element={<PagarPage />} />
                <Route path="/movimientos-bancarios" element={<MovimientosBancariosPage />} />
                <Route path="/inventario" element={<InventarioPage />} />
                <Route path="/series" element={<SeriesPage />} />
                <Route path="/caducidad" element={<CaducidadPage />} />
                <Route path="/servicio-tecnico" element={<ServicioTecnicoPage />} />
                <Route path="/reportes" element={<ReportesPage />} />
                <Route path="/config" element={<Configuracion />} />
              </>
            )}
          </Route>
        </Routes>
      </BrowserRouter>
      <ConnectionStatus />
    </DemoProvider>
  );
}

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <ErrorBoundary>
      <ToastProvider>
        <SesionProvider>
          <AppGate />
        </SesionProvider>
      </ToastProvider>
    </ErrorBoundary>
  </React.StrictMode>
);
