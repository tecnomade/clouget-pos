/**
 * v2.5.0 — Registro central de metadatos de tabs (título, icono).
 * Usado por TabsContainer (para abrir tabs desde URL) y por Layout (sidebar
 * usa esto para abrir tabs con metadata correcta).
 */
export interface TabMetadata {
  title: string;
  icon: string; // emoji
}

export const TABS_METADATA: Record<string, TabMetadata> = {
  "/":                      { title: "Inicio",                  icon: "🏠" },
  "/pos":                   { title: "Venta",                   icon: "🛒" },
  "/caja":                  { title: "Caja",                    icon: "💵" },
  "/ventas":                { title: "Ventas",                  icon: "🧾" },
  "/cuentas":               { title: "Cobrar",                  icon: "💰" },
  "/productos":             { title: "Productos",               icon: "📦" },
  "/clientes":              { title: "Clientes",                icon: "👥" },
  "/guias":                 { title: "Notas de Entrega",        icon: "🚚" },
  "/gastos":                { title: "Gastos",                  icon: "💸" },
  "/compras":               { title: "Compras",                 icon: "🛍" },
  "/pagar":                 { title: "Pagar",                   icon: "💳" },
  "/movimientos-bancarios": { title: "Bancos",                  icon: "🏦" },
  "/inventario":            { title: "Inventario",              icon: "📊" },
  "/series":                { title: "Series",                  icon: "🔢" },
  "/caducidad":             { title: "Caducidad",               icon: "📅" },
  "/servicio-tecnico":      { title: "Servicio",                icon: "🔧" },
  "/contabilidad":          { title: "Contabilidad",            icon: "🧮" },
  "/reportes":              { title: "Reportes",                icon: "📈" },
  "/mesas":                 { title: "Mesas",                   icon: "🍴" },
  "/cocina":                { title: "Cocina",                  icon: "🍳" },
  "/config-mesas":          { title: "Config Mesas",            icon: "⚙" },
  "/config":                { title: "Configuración",           icon: "⚙" },
};

export function getTabMetadata(path: string): TabMetadata | null {
  return TABS_METADATA[path] || null;
}
