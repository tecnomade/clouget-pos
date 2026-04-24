import React, { useState, useEffect } from "react";
import ReactDOM from "react-dom/client";
import { BrowserRouter, Routes, Route, Navigate } from "react-router-dom";
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
  const { sesion, setSesion, esAdmin, tienePermiso } = useSesion();

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
      {/* key={sesion.usuario_id} fuerza remount al cambiar de usuario:
          asi se resetea el historial de navegacion y todos los estados de pagina,
          evitando que un cajero quede atorado en una ruta que solo ve admin (ej. /config). */}
      <BrowserRouter key={sesion.usuario_id}>
        <Routes>
          <Route element={<Layout />}>
            {/* Rutas disponibles para todos los roles */}
            <Route path="/" element={<DashboardPage />} />
            <Route path="/pos" element={<PuntoVenta />} />
            <Route path="/caja" element={<CajaPage />} />
            <Route path="/ventas" element={<VentasDia />} />
            <Route path="/cuentas" element={<CuentasPage />} />
            {/* Rutas con permisos: cada una se renderiza si es admin O tiene el permiso correspondiente.
                Si el usuario no califica, el catch-all del final lo redirige al dashboard. */}
            {(esAdmin || tienePermiso("gestionar_productos")) && (
              <Route path="/productos" element={<Productos />} />
            )}
            {(esAdmin || tienePermiso("gestionar_clientes")) && (
              <Route path="/clientes" element={<Clientes />} />
            )}
            {(esAdmin || tienePermiso("ver_guias")) && (
              <Route path="/guias" element={<GuiasRemisionPage />} />
            )}
            {(esAdmin || tienePermiso("gestionar_gastos")) && (
              <Route path="/gastos" element={<GastosPage />} />
            )}
            {(esAdmin || tienePermiso("gestionar_compras")) && (
              <>
                <Route path="/compras" element={<ComprasPage />} />
                <Route path="/pagar" element={<PagarPage />} />
              </>
            )}
            {(esAdmin || tienePermiso("ver_movimientos_bancarios")) && (
              <Route path="/movimientos-bancarios" element={<MovimientosBancariosPage />} />
            )}
            {(esAdmin || tienePermiso("gestionar_inventario")) && (
              <>
                <Route path="/inventario" element={<InventarioPage />} />
                <Route path="/series" element={<SeriesPage />} />
                <Route path="/caducidad" element={<CaducidadPage />} />
              </>
            )}
            {(esAdmin || tienePermiso("gestionar_servicio_tecnico") || tienePermiso("ver_servicio_tecnico")) && (
              <Route path="/servicio-tecnico" element={<ServicioTecnicoPage />} />
            )}
            {(esAdmin || tienePermiso("ver_reportes")) && (
              <Route path="/reportes" element={<ReportesPage />} />
            )}
            {/* Configuración: solo admin */}
            {esAdmin && (
              <Route path="/config" element={<Configuracion />} />
            )}
            {/* Catch-all: cualquier ruta no encontrada (ej. cajero accede a /config admin)
                redirige al dashboard. Evita pantallas en blanco por rutas no autorizadas. */}
            <Route path="*" element={<Navigate to="/" replace />} />
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
