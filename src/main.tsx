import React, { useState, useEffect } from "react";
import ReactDOM from "react-dom/client";
import { BrowserRouter } from "react-router-dom";
import { ToastProvider } from "./components/Toast";
import { SesionProvider, useSesion } from "./contexts/SesionContext";
import { DemoProvider } from "./contexts/DemoContext";
import { TabsProvider } from "./contexts/TabsContext";
import Layout from "./components/Layout";
import TabsContainer from "./components/TabsContainer";
import LicenciaPage from "./pages/LicenciaPage";
import LoginPage from "./pages/LoginPage";
import { FEATURES } from "./config/branding";
import { getTabMetadata } from "./config/tabsRegistry";
import { obtenerEstadoLicencia, obtenerSesionActual, obtenerConfig, configurarModoRed } from "./services/api";
import { iniciarSyncService, sincronizarCacheProductos, reservarSecuenciales } from "./services/offlineSync";
import ConnectionStatus from "./components/ConnectionStatus";
import type { LicenciaInfo } from "./types";
import ErrorBoundary from "./components/ErrorBoundary";
import "./styles/global.css";

/** Helper: ¿la licencia tiene este módulo activo? */
function hasModulo(lic: LicenciaInfo | null, modulo: string): boolean {
  return !!lic?.modulos?.includes(modulo);
}

// Auto-select all text in number inputs on focus (global)
document.addEventListener("focusin", (e) => {
  const target = e.target as HTMLInputElement;
  if (target?.tagName === "INPUT" && target.type === "number") {
    setTimeout(() => target.select(), 0);
  }
});

// v2.6.4: evitar que la RUEDA del mouse cambie el valor de un <input type="number">
// enfocado (causaba bugs tipo "tecleé 25 pero quedó 24.99" al rozar el scroll).
// Al hacer scroll sobre el campo enfocado, lo desenfocamos: el valor no cambia y
// la página sigue desplazándose normalmente.
document.addEventListener("wheel", (e) => {
  const el = document.activeElement as HTMLInputElement | null;
  if (el && el.tagName === "INPUT" && el.type === "number" && el === e.target) {
    el.blur();
  }
}, { passive: true });

function AppGate() {
  const [licencia, setLicencia] = useState<LicenciaInfo | null>(null);
  const [verificando, setVerificando] = useState(true);
  // v2.5.0: feature flag de tabs (toggle en Configuración)
  const [tabsEnabled, setTabsEnabled] = useState<boolean>(true);
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
      Promise.all([obtenerEstadoLicencia(), obtenerSesionActual(), obtenerConfig()])
        .then(([lic, ses, cfg]) => {
          setLicencia(lic);
          if (ses) setSesion(ses);
          // v2.5.0: leer toggle de tabs (default ON, valor "0" lo desactiva)
          setTabsEnabled((cfg.tabs_enabled ?? "1") !== "0");
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

  // v2.5.0: función que valida si un path es accesible para la sesión actual.
  // Replica la lógica de los <Route> con permisos del flujo viejo.
  const canAccessPath = (path: string): boolean => {
    // Rutas accesibles para todos los roles
    const todos = ["/", "/pos", "/caja", "/ventas", "/cuentas"];
    if (todos.includes(path)) return true;
    // Rutas con permiso (admin bypassa)
    const reglas: Record<string, string[]> = {
      "/productos":             ["gestionar_productos"],
      "/clientes":              ["gestionar_clientes"],
      "/guias":                 ["ver_guias"],
      "/gastos":                ["gestionar_gastos"],
      "/compras":               ["gestionar_compras"],
      "/pagar":                 ["gestionar_compras"],
      "/movimientos-bancarios": ["ver_movimientos_bancarios"],
      "/inventario":            ["gestionar_inventario"],
      "/series":                ["gestionar_inventario"],
      "/caducidad":             ["gestionar_inventario"],
      "/servicio-tecnico":      ["gestionar_servicio_tecnico", "ver_servicio_tecnico"],
      "/reportes":              ["ver_reportes"],
    };
    if (reglas[path]) {
      if (esAdmin) return true;
      return reglas[path].some(p => tienePermiso(p));
    }
    // Restaurante: solo si build + licencia
    if (path === "/mesas" || path === "/cocina") {
      return FEATURES.restaurante && hasModulo(licencia, "restaurante");
    }
    if (path === "/config-mesas") {
      return esAdmin && FEATURES.restaurante && hasModulo(licencia, "restaurante");
    }
    // Configuración: solo admin
    if (path === "/config") return esAdmin;
    return false;
  };

  return (
    <DemoProvider>
      {/* key={sesion.usuario_id} fuerza remount al cambiar de usuario:
          asi se resetea el historial de navegacion y todos los estados de pagina,
          evitando que un cajero quede atorado en una ruta que solo ve admin. */}
      <BrowserRouter key={sesion.usuario_id}>
        {/* v2.5.0: TabsProvider envuelve toda la app. Si tabsEnabled=false, actúa
            como pass-through (modo clásico single-page). scope=usuario_id para
            que cada user tenga su propio set de tabs (no se filtran tabs entre
            cajeros que comparten la misma PC). */}
        <TabsProvider enabled={tabsEnabled} scope={sesion.usuario_id}>
          <Layout>
            <TabsContainer
              canAccessPath={canAccessPath}
              resolveMetadata={(p) => getTabMetadata(p)}
            />
          </Layout>
        </TabsProvider>
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
