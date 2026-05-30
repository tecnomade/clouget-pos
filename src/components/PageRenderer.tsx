/**
 * v2.5.0 — Renderiza la página correspondiente a un path dado.
 * Centraliza la lista de rutas que antes estaba en main.tsx con <Route>.
 *
 * Cada PageRenderer rendereará una página completa. Cuando el sistema de tabs
 * mantiene varias instancias montadas (display:none), cada una usa su propio
 * componente de página por su path → state preservado por tab.
 *
 * v2.5.60: lazy-load de todas las páginas EXCEPTO PuntoVenta (caso más común).
 * Esto reduce el bundle inicial de ~600KB a ~200KB. Páginas con dependencias
 * pesadas (Recharts en Dashboard/Reportes, genpdf-like en Etiquetas, etc) solo
 * cargan cuando el usuario abre esa pestaña. Mejora notable en PCs lentos:
 *   - tiempo de arranque más rápido
 *   - menos memoria ocupada al inicio
 *   - actualización vía updater más rápida (instalador chunkificado)
 */
import { lazy, Suspense } from "react";
import PuntoVenta from "../pages/PuntoVenta"; // Página más usada — carga inmediata

// Páginas con dependencias pesadas (recharts ~400KB)
const DashboardPage = lazy(() => import("../pages/DashboardPage"));
const ReportesPage = lazy(() => import("../pages/ReportesPage"));
const ContabilidadPage = lazy(() => import("../pages/ContabilidadPage"));

// Páginas de uso medio
const Productos = lazy(() => import("../pages/Productos"));
const Clientes = lazy(() => import("../pages/Clientes"));
const VentasDia = lazy(() => import("../pages/VentasDia"));
const CajaPage = lazy(() => import("../pages/CajaPage"));
const GastosPage = lazy(() => import("../pages/GastosPage"));
const CuentasPage = lazy(() => import("../pages/CuentasPage"));
const Configuracion = lazy(() => import("../pages/Configuracion"));
const InventarioPage = lazy(() => import("../pages/InventarioPage"));
const GuiasRemisionPage = lazy(() => import("../pages/GuiasRemisionPage"));
const ComprasPage = lazy(() => import("../pages/ComprasPage"));
const PagarPage = lazy(() => import("../pages/PagarPage"));
const MovimientosBancariosPage = lazy(() => import("../pages/MovimientosBancariosPage"));
const SeriesPage = lazy(() => import("../pages/SeriesPage"));
const CaducidadPage = lazy(() => import("../pages/CaducidadPage"));
const ServicioTecnicoPage = lazy(() => import("../pages/ServicioTecnicoPage"));
const MesasPage = lazy(() => import("../restaurante/pages/MesasPage"));
const CocinaPage = lazy(() => import("../restaurante/pages/CocinaPage"));
const ConfigMesasPage = lazy(() => import("../restaurante/pages/ConfigMesasPage"));

function PageFallback() {
  return (
    <div style={{
      display: "flex", alignItems: "center", justifyContent: "center",
      height: "60vh", color: "var(--color-text-secondary)", fontSize: 14,
    }}>
      Cargando…
    </div>
  );
}

function resolverPagina(path: string) {
  switch (path) {
    case "/":                return <DashboardPage />;
    case "/pos":             return <PuntoVenta />;
    case "/caja":            return <CajaPage />;
    case "/ventas":          return <VentasDia />;
    case "/cuentas":         return <CuentasPage />;
    case "/productos":       return <Productos />;
    case "/clientes":        return <Clientes />;
    case "/guias":           return <GuiasRemisionPage />;
    case "/gastos":          return <GastosPage />;
    case "/compras":         return <ComprasPage />;
    case "/pagar":           return <PagarPage />;
    case "/movimientos-bancarios": return <MovimientosBancariosPage />;
    case "/inventario":      return <InventarioPage />;
    case "/series":          return <SeriesPage />;
    case "/caducidad":       return <CaducidadPage />;
    case "/servicio-tecnico":return <ServicioTecnicoPage />;
    case "/contabilidad":    return <ContabilidadPage />;
    case "/reportes":        return <ReportesPage />;
    case "/mesas":           return <MesasPage />;
    case "/cocina":          return <CocinaPage />;
    case "/config-mesas":    return <ConfigMesasPage />;
    case "/config":          return <Configuracion />;
    default:
      // Catch-all → Dashboard
      return <DashboardPage />;
  }
}

export default function PageRenderer({ path }: { path: string }) {
  return (
    <Suspense fallback={<PageFallback />}>
      {resolverPagina(path)}
    </Suspense>
  );
}
