/**
 * v2.5.0 — Renderiza la página correspondiente a un path dado.
 * Centraliza la lista de rutas que antes estaba en main.tsx con <Route>.
 *
 * Cada PageRenderer rendereará una página completa. Cuando el sistema de tabs
 * mantiene varias instancias montadas (display:none), cada una usa su propio
 * componente de página por su path → state preservado por tab.
 */
import DashboardPage from "../pages/DashboardPage";
import PuntoVenta from "../pages/PuntoVenta";
import Productos from "../pages/Productos";
import Clientes from "../pages/Clientes";
import VentasDia from "../pages/VentasDia";
import CajaPage from "../pages/CajaPage";
import GastosPage from "../pages/GastosPage";
import CuentasPage from "../pages/CuentasPage";
import Configuracion from "../pages/Configuracion";
import InventarioPage from "../pages/InventarioPage";
import GuiasRemisionPage from "../pages/GuiasRemisionPage";
import ReportesPage from "../pages/ReportesPage";
import ComprasPage from "../pages/ComprasPage";
import PagarPage from "../pages/PagarPage";
import MovimientosBancariosPage from "../pages/MovimientosBancariosPage";
import SeriesPage from "../pages/SeriesPage";
import CaducidadPage from "../pages/CaducidadPage";
import ServicioTecnicoPage from "../pages/ServicioTecnicoPage";
import ContabilidadPage from "../pages/ContabilidadPage";
import MesasPage from "../restaurante/pages/MesasPage";
import CocinaPage from "../restaurante/pages/CocinaPage";
import ConfigMesasPage from "../restaurante/pages/ConfigMesasPage";

export default function PageRenderer({ path }: { path: string }) {
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
