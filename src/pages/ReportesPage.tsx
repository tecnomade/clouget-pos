import React, { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { reporteUtilidad, reporteBalance, reporteProductosRentabilidad, reporteIvaMensual,
  reporteCxcPorCliente, reporteCxcDetalleCliente, reporteCxpPorProveedor, reporteCxpDetalleProveedor,
  reporteInventarioValorizado, reporteKardexProducto, reporteKardexMulti, listarCategoriasSimple,
  exportarInventarioXlsx, exportarInventarioPdf, exportarTablaXlsx, exportarTablaPdf, reporteVentasPorCajero,
  reporteVentasFiltrable, reporteVentasFiltrosDisponibles,
  // v2.4.14: reportes de servicio tecnico
  stReporteCancelaciones, stReporteGarantiasActivas,
  // Reporte de compras (frontend-only, reusa comandos existentes)
  listarCompras, listarProveedores } from "../services/api";
import type { ReporteVentasResultado, VentaReporteRow, ResumenCancelaciones, ResumenGarantias } from "../services/api";
import type { Compra, Proveedor } from "../types";
import { save } from "@tauri-apps/plugin-dialog";
import { useToast } from "../components/Toast";
import ModalCorregirStockNegativo from "../components/ModalCorregirStockNegativo";
import DepositosEnTransito from "../components/DepositosEnTransito";
import { BarChart, Bar, XAxis, YAxis, CartesianGrid, Tooltip, ResponsiveContainer, PieChart, Pie, Cell } from "recharts";
import type { ReporteUtilidad, ReporteBalance, ProductoRentabilidad } from "../types";

type ReporteIva = {
  anio: number; mes: number; fecha_desde: string; fecha_hasta: string;
  ventas_0: number; ventas_15_base: number; iva_ventas: number;
  nc_base: number; nc_iva: number; iva_ventas_neto: number;
  compras_0: number; compras_15_base: number; iva_compras: number;
  iva_a_pagar: number; total_ventas: number; total_compras: number;
};

const MESES = [
  "Enero", "Febrero", "Marzo", "Abril", "Mayo", "Junio",
  "Julio", "Agosto", "Septiembre", "Octubre", "Noviembre", "Diciembre",
];

const hoy = () => new Date().toISOString().slice(0, 10);
const hace7 = () => { const d = new Date(); d.setDate(d.getDate() - 7); return d.toISOString().slice(0, 10); };
const inicioMes = () => { const d = new Date(); return `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, "0")}-01`; };
const inicioAnio = () => `${new Date().getFullYear()}-01-01`;

const COLORES_PIE = ["var(--color-primary)", "var(--color-success)", "var(--color-warning)", "#8b5cf6", "var(--color-danger)", "#06b6d4"];

export default function ReportesPage() {
  const { toastExito, toastError } = useToast();
  const [tab, setTab] = useState<"utilidad" | "balance" | "productos" | "iva" | "cxc" | "cxp" | "compras" | "depositos" | "inventario" | "valuacion" | "kardex" | "cajeros" | "ventas" | "gastos" | "cancelaciones_st" | "garantias_st">("utilidad");
  // v2.5.54: reporte de gastos
  const [gastosReporte, setGastosReporte] = useState<import("../services/api").ResumenGastos | null>(null);
  // v2.5.22: estado para reporte de valuación de inventario
  const [valuacionData, setValuacionData] = useState<any | null>(null);
  const [valuacionMetodo, setValuacionMetodo] = useState<"PMP" | "ULTIMO">("PMP");
  const [valuacionCargando, setValuacionCargando] = useState(false);
  // v2.4.14: reportes de Servicio Tecnico
  const [cancelacionesST, setCancelacionesST] = useState<ResumenCancelaciones | null>(null);
  const [garantiasST, setGarantiasST] = useState<ResumenGarantias | null>(null);
  // v2.4.15: gating — solo mostrar tabs ST si el modulo esta activo
  const [moduloSTActivo, setModuloSTActivo] = useState(false);
  // v2.4.15: filtros inteligentes para reportes ST
  const [filtroCancelaciones, setFiltroCancelaciones] = useState("");
  const [filtroGarantias, setFiltroGarantias] = useState("");
  const [cajerosData, setCajerosData] = useState<any>(null);
  // v2.3.70 — Reporte de ventas individuales filtrable
  const [ventasReporte, setVentasReporte] = useState<ReporteVentasResultado | null>(null);
  const [ventasFiltros, setVentasFiltros] = useState<{
    cajero: string;
    formaPago: string;
    tipoDocumento: string;
    categoriaId: number | null;
    incluirAnuladas: boolean;
  }>({ cajero: "", formaPago: "", tipoDocumento: "", categoriaId: null, incluirAnuladas: false });
  const [ventasOpciones, setVentasOpciones] = useState<{
    cajeros: string[]; formas_pago: string[]; tipos_documento: string[]; categorias: { id: number; nombre: string }[];
  } | null>(null);
  // Kardex multi-categoria
  const [kardexMultiData, setKardexMultiData] = useState<any | null>(null);
  // v2.5.25: buscador en kardex multi (filtra resultados ya cargados por nombre/codigo/usuario)
  const [kardexBusqueda, setKardexBusqueda] = useState("");
  const [categoriasMaestro, setCategoriasMaestro] = useState<Array<{ id: number; nombre: string }>>([]);
  const [kardexCatsSeleccionadas, setKardexCatsSeleccionadas] = useState<number[]>([]);
  const [kardexCargando, setKardexCargando] = useState(false);
  // CXC/CXP
  const [cxcResumen, setCxcResumen] = useState<any[]>([]);
  const [cxcClienteDetalle, setCxcClienteDetalle] = useState<{ cliente: any; cuentas: any[] } | null>(null);
  const [cxpResumen, setCxpResumen] = useState<any[]>([]);
  const [cxpProveedorDetalle, setCxpProveedorDetalle] = useState<{ proveedor: any; cuentas: any[] } | null>(null);
  const [busquedaCliente, setBusquedaCliente] = useState("");
  const [busquedaProveedor, setBusquedaProveedor] = useState("");
  // Reporte de Compras (frontend-only)
  const [comprasData, setComprasData] = useState<Compra[]>([]);
  const [comprasProveedores, setComprasProveedores] = useState<Proveedor[]>([]);
  const [comprasProveedorFiltro, setComprasProveedorFiltro] = useState<string>("TODOS");
  const [comprasAgrupacion, setComprasAgrupacion] = useState<"detalle" | "proveedor" | "fecha">("detalle");
  // Inventario
  const [inventario, setInventario] = useState<any | null>(null);
  const [kardexProducto, setKardexProducto] = useState<{ producto: any; movimientos: any[] } | null>(null);
  const [modalStockNeg, setModalStockNeg] = useState(false);
  const [kardexExpandido, setKardexExpandido] = useState<number | null>(null);
  const [busquedaInv, setBusquedaInv] = useState("");
  const [filtroEstado, setFiltroEstado] = useState<"TODOS" | "OK" | "BAJO" | "SIN_STOCK" | "STOCK_NEGATIVO">("TODOS");
  const [filtroCategoriaInv, setFiltroCategoriaInv] = useState<string>("TODAS");
  const [desde, setDesde] = useState(inicioMes());
  const [hasta, setHasta] = useState(hoy());
  const [utilidad, setUtilidad] = useState<ReporteUtilidad | null>(null);
  const [balance, setBalance] = useState<ReporteBalance | null>(null);
  const [productos, setProductos] = useState<ProductoRentabilidad[]>([]);
  const [cargando, setCargando] = useState(false);

  // Estado para reporte IVA mensual
  const anioActual = new Date().getFullYear();
  const mesActual = new Date().getMonth() + 1;
  const [ivaAnio, setIvaAnio] = useState<number>(anioActual);
  const [ivaMes, setIvaMes] = useState<number>(mesActual);
  const [iva, setIva] = useState<ReporteIva | null>(null);
  const [cargandoIva, setCargandoIva] = useState(false);

  const cargar = async () => {
    if (tab === "iva") return; // IVA se carga manualmente con botón
    setCargando(true);
    try {
      if (tab === "utilidad") setUtilidad(await reporteUtilidad(desde, hasta));
      else if (tab === "balance") setBalance(await reporteBalance(desde, hasta));
      else if (tab === "productos") setProductos(await reporteProductosRentabilidad(desde, hasta, 50));
      else if (tab === "cxc") {
        const r = await reporteCxcPorCliente();
        setCxcResumen(r);
        setCxcClienteDetalle(null);
      }
      else if (tab === "cxp") {
        const r = await reporteCxpPorProveedor();
        setCxpResumen(r);
        setCxpProveedorDetalle(null);
      }
      else if (tab === "compras") {
        // Cargar proveedores (para el filtro) + compras del rango en paralelo
        const [provs, compras] = await Promise.all([
          listarProveedores(),
          listarCompras(desde, hasta),
        ]);
        setComprasProveedores(provs);
        setComprasData(compras);
      }
      else if (tab === "inventario") {
        const inv = await reporteInventarioValorizado();
        setInventario(inv);
        setKardexProducto(null);
      }
      else if (tab === "cajeros") {
        const data = await reporteVentasPorCajero(desde, hasta);
        setCajerosData(data);
      }
      else if (tab === "cancelaciones_st") {
        try {
          const data = await stReporteCancelaciones(desde, hasta);
          setCancelacionesST(data);
        } catch (e: any) {
          // Modulo no habilitado u otro error: marcar vacio para que la UI lo muestre
          setCancelacionesST({ total_canceladas: 0, total_abonos_devueltos: 0, monto_total_devuelto: 0, ordenes: [] });
          if (!String(e).includes("modulo")) toastError("" + e);
        }
      }
      else if (tab === "garantias_st") {
        try {
          const data = await stReporteGarantiasActivas();
          setGarantiasST(data);
        } catch (e: any) {
          setGarantiasST({ total_activas: 0, total_por_vencer_30d: 0, ordenes: [] });
          if (!String(e).includes("modulo")) toastError("" + e);
        }
      }
      else if (tab === "gastos") {
        // v2.5.54: reporte de gastos del rango
        const { resumenGastosRango } = await import("../services/api");
        setGastosReporte(await resumenGastosRango(desde, hasta));
      }
      else if (tab === "ventas") {
        // Cargar opciones de filtro + datos en paralelo
        const [opciones, datos] = await Promise.all([
          reporteVentasFiltrosDisponibles(desde, hasta),
          reporteVentasFiltrable({
            fechaDesde: desde, fechaHasta: hasta,
            cajero: ventasFiltros.cajero || null,
            formaPago: ventasFiltros.formaPago || null,
            tipoDocumento: ventasFiltros.tipoDocumento || null,
            categoriaId: ventasFiltros.categoriaId,
            incluirAnuladas: ventasFiltros.incluirAnuladas,
          }),
        ]);
        setVentasOpciones(opciones);
        setVentasReporte(datos);
      }
    } catch (err) {
      toastError("Error: " + err);
    }
    setCargando(false);
  };

  useEffect(() => { cargar(); }, [tab, desde, hasta]);

  // Abrir una pestaña concreta al llegar desde otra página (ej. Compras →
  // "Reporte de Compras"). Funciona tanto al montar como si Reportes ya estaba
  // abierto (vía evento). Se limpia el flag para no re-aplicarlo después.
  useEffect(() => {
    const aplicarTabGuardado = () => {
      const t = sessionStorage.getItem("reportes_tab");
      if (t) {
        setTab(t as typeof tab);
        sessionStorage.removeItem("reportes_tab");
      }
    };
    aplicarTabGuardado();
    const handler = (e: Event) => {
      const detail = (e as CustomEvent).detail as string | undefined;
      if (detail) {
        setTab(detail as typeof tab);
        sessionStorage.removeItem("reportes_tab");
      }
    };
    window.addEventListener("clouget:reportes-tab", handler);
    return () => window.removeEventListener("clouget:reportes-tab", handler);
  }, []);

  // Cargar maestro de categorías al montar
  useEffect(() => {
    listarCategoriasSimple().then(setCategoriasMaestro).catch(() => {});
    // v2.4.15 / v2.4.17: chequear si modulo ST esta activo (para mostrar tabs).
    // Licencia es fuente de verdad — fallback al flag legacy si no hay licencia.
    invoke<any>("obtener_config")
      .then((cfg: any) => {
        const licStr = (cfg?.licencia_modulos || "").trim();
        const tieneLic = licStr !== "" && licStr !== "[]";
        try {
          const mods: string[] = tieneLic ? JSON.parse(licStr) : [];
          if (tieneLic) setModuloSTActivo(mods.includes("servicio_tecnico"));
          else setModuloSTActivo(cfg?.modulo_servicio_tecnico === "1");
        } catch { setModuloSTActivo(false); }
      })
      .catch(() => {});
  }, []);

  const cargarKardexMulti = async () => {
    setKardexCargando(true);
    try {
      const cats = kardexCatsSeleccionadas.length > 0 ? kardexCatsSeleccionadas : null;
      const data = await reporteKardexMulti(cats, null, desde, hasta);
      setKardexMultiData(data);
    } catch (err) { toastError("Error: " + err); }
    setKardexCargando(false);
  };

  // Helper generico: arma los datos del tab activo para exportar a XLSX/PDF
  type ExportDatos = {
    titulo: string;
    subtitulo: string | null;
    archivo: string;
    encabezados: string[];
    filas: string[][];
    cols_numericas?: number[];
  };
  const obtenerDatosTabla = (): ExportDatos | null => {
    const periodo = `${desde} a ${hasta}`;
    if (tab === "utilidad" && utilidad) {
      const u: any = utilidad;
      return {
        titulo: "Estado de Resultados",
        subtitulo: periodo,
        archivo: `estado-resultados-${desde}-${hasta}`,
        encabezados: ["Concepto", "Monto USD"],
        filas: [
          ["Ventas brutas", (u.ventas_brutas ?? 0).toFixed(2)],
          ["(-) Costo de ventas", (u.costo_ventas ?? 0).toFixed(2)],
          ["= Utilidad bruta", (u.utilidad_bruta ?? 0).toFixed(2)],
          [`Margen bruto`, `${(u.margen_bruto ?? 0).toFixed(2)}%`],
          ["(-) Gastos", (u.total_gastos ?? 0).toFixed(2)],
          ["(-) Devoluciones", (u.total_devoluciones ?? 0).toFixed(2)],
          ["= UTILIDAD NETA", (u.utilidad_neta ?? 0).toFixed(2)],
          ["Margen neto", `${(u.margen_neto ?? 0).toFixed(2)}%`],
          ["Num transacciones", String(u.num_ventas ?? 0)],
          ["Promedio por venta", (u.promedio_por_venta ?? 0).toFixed(2)],
        ],
        cols_numericas: [1],
      };
    }
    if (tab === "balance" && balance) {
      const b: any = balance;
      return {
        titulo: "Balance",
        subtitulo: periodo,
        archivo: `balance-${desde}-${hasta}`,
        encabezados: ["Concepto", "Monto USD"],
        filas: [
          ["Ventas totales", (b.total_ventas ?? 0).toFixed(2)],
          ["Cobros de credito", (b.total_cobros_credito ?? 0).toFixed(2)],
          ["Gastos", (b.total_gastos ?? 0).toFixed(2)],
          ["Pagos a proveedores", (b.total_pagos_proveedor ?? 0).toFixed(2)],
          ["Retiros de caja", (b.total_retiros ?? 0).toFixed(2)],
          ["UTILIDAD NETA", (b.utilidad_neta ?? 0).toFixed(2)],
        ],
        cols_numericas: [1],
      };
    }
    if (tab === "productos") {
      return {
        titulo: "Rentabilidad por Producto",
        subtitulo: periodo,
        archivo: `rentabilidad-productos-${desde}-${hasta}`,
        encabezados: ["Producto", "Unidades", "Ingresos", "Costo", "Utilidad", "Margen %"],
        filas: productos.map((p: any) => [
          p.nombre,
          String(p.unidades_vendidas ?? p.cantidad_vendida ?? 0),
          (p.total_vendido ?? p.ingresos ?? 0).toFixed(2),
          (p.costo_total ?? p.costo ?? 0).toFixed(2),
          (p.utilidad ?? 0).toFixed(2),
          `${(p.margen ?? 0).toFixed(2)}%`,
        ]),
        cols_numericas: [1, 2, 3, 4],
      };
    }
    if (tab === "cxc") {
      if (cxcClienteDetalle) {
        const cli = cxcClienteDetalle.cliente;
        return {
          titulo: `Cuentas por Cobrar - ${cli.cliente_nombre}`,
          subtitulo: `ID: ${cli.identificacion || "-"} · Tel: ${cli.telefono || "-"}`,
          archivo: `cxc-${cli.cliente_nombre.replace(/[^a-zA-Z0-9]/g, "_")}`,
          encabezados: ["Venta", "Fecha", "Total", "Pagado", "Saldo", "Estado", "Vencimiento", "Atraso"],
          filas: cxcClienteDetalle.cuentas.map((c: any) => [
            c.venta_numero || "", c.venta_fecha?.slice(0, 10) || "",
            c.monto_total.toFixed(2), c.monto_pagado.toFixed(2), c.saldo.toFixed(2),
            c.estado, c.fecha_vencimiento || "", String(c.dias_atraso ?? 0),
          ]),
          cols_numericas: [2, 3, 4],
        };
      }
      return {
        titulo: "Cuentas por Cobrar - Por Cliente",
        subtitulo: "Resumen de saldos pendientes",
        archivo: `cxc-por-cliente-${hoy()}`,
        encabezados: ["Cliente", "Identificacion", "Telefono", "Cuentas", "Facturado", "Pagado", "Saldo", "Vencido"],
        filas: cxcResumen.filter((c: any) => !busquedaCliente || c.cliente_nombre.toLowerCase().includes(busquedaCliente.toLowerCase()))
          .map((c: any) => [
            c.cliente_nombre, c.identificacion || "", c.telefono || "",
            String(c.num_cuentas),
            c.total_facturado.toFixed(2), c.total_pagado.toFixed(2),
            c.saldo_pendiente.toFixed(2), c.monto_vencido.toFixed(2),
          ]),
        cols_numericas: [4, 5, 6, 7],
      };
    }
    if (tab === "cxp") {
      if (cxpProveedorDetalle) {
        const p = cxpProveedorDetalle.proveedor;
        return {
          titulo: `Cuentas por Pagar - ${p.proveedor_nombre}`,
          subtitulo: `RUC: ${p.ruc || "-"} · Tel: ${p.telefono || "-"}`,
          archivo: `cxp-${p.proveedor_nombre.replace(/[^a-zA-Z0-9]/g, "_")}`,
          encabezados: ["Compra", "Fac.Prov", "Fecha", "Total", "Pagado", "Saldo", "Estado", "Vencim.", "Atraso"],
          filas: cxpProveedorDetalle.cuentas.map((c: any) => [
            c.compra_numero || "", c.numero_factura || "", c.compra_fecha?.slice(0, 10) || "",
            c.monto_total.toFixed(2), c.monto_pagado.toFixed(2), c.saldo.toFixed(2),
            c.estado, c.fecha_vencimiento || "", String(c.dias_atraso ?? 0),
          ]),
          cols_numericas: [3, 4, 5],
        };
      }
      return {
        titulo: "Cuentas por Pagar - Por Proveedor",
        subtitulo: "Resumen de saldos pendientes",
        archivo: `cxp-por-proveedor-${hoy()}`,
        encabezados: ["Proveedor", "RUC", "Telefono", "Cuentas", "Facturado", "Pagado", "Saldo", "Vencido"],
        filas: cxpResumen.filter((c: any) => !busquedaProveedor || c.proveedor_nombre.toLowerCase().includes(busquedaProveedor.toLowerCase()))
          .map((c: any) => [
            c.proveedor_nombre, c.ruc || "", c.telefono || "",
            String(c.num_cuentas),
            c.total_facturado.toFixed(2), c.total_pagado.toFixed(2),
            c.saldo_pendiente.toFixed(2), c.monto_vencido.toFixed(2),
          ]),
        cols_numericas: [4, 5, 6, 7],
      };
    }
    if (tab === "kardex" && kardexMultiData) {
      return {
        titulo: "Kardex Multi-producto",
        subtitulo: periodo,
        archivo: `kardex-${desde}-${hasta}`,
        encabezados: ["Fecha", "Producto", "Categoria", "Tipo", "Cantidad", "Stock Ant.", "Stock Nuevo", "Costo Un.", "Motivo"],
        filas: kardexMultiData.movimientos.map((m: any) => [
          m.fecha?.slice(0, 16).replace("T", " ") || "",
          m.nombre, m.categoria || "",
          m.tipo, String(m.cantidad), String(m.stock_anterior), String(m.stock_nuevo),
          m.costo_unitario != null ? m.costo_unitario.toFixed(2) : "",
          m.motivo || "",
        ]),
        cols_numericas: [4, 5, 6, 7],
      };
    }
    if (tab === "inventario") {
      if (kardexProducto) {
        return {
          titulo: `Kardex - ${kardexProducto.producto.nombre}`,
          subtitulo: `Stock actual: ${kardexProducto.producto.stock_actual}`,
          archivo: `kardex-${(kardexProducto.producto.codigo || kardexProducto.producto.nombre).replace(/[^a-zA-Z0-9]/g, "_")}`,
          encabezados: ["Fecha", "Tipo", "Cantidad", "Stock Ant.", "Stock Nuevo", "Costo Un.", "Motivo"],
          filas: kardexProducto.movimientos.map((m: any) => [
            m.fecha?.slice(0, 19).replace("T", " ") || "",
            m.tipo, String(m.cantidad), String(m.stock_anterior), String(m.stock_nuevo),
            m.costo_unitario ? m.costo_unitario.toFixed(2) : "",
            m.motivo || "",
          ]),
          cols_numericas: [2, 3, 4, 5],
        };
      }
      // Inventario valorizado: se maneja por backend con filtros
      return {
        titulo: "Inventario Valorizado",
        subtitulo: "Ver filtros en encabezado",
        archivo: `inventario-${hoy()}`,
        encabezados: [], filas: [], // backend se encarga
      };
    }
    if (tab === "compras") {
      const provNombre = comprasProveedorFiltro === "TODOS"
        ? "Todos los proveedores"
        : (comprasProveedores.find((p) => String(p.id) === comprasProveedorFiltro)?.nombre || "");
      return {
        titulo: "Reporte de Compras",
        subtitulo: `${periodo}${comprasProveedorFiltro !== "TODOS" ? ` · ${provNombre}` : ""}`,
        archivo: `compras-${desde}-${hasta}`,
        encabezados: ["Fecha", "N°", "Factura", "Proveedor", "Tipo", "Subtotal", "IVA", "Total"],
        filas: comprasFiltradas.map((c) => [
          comprasFechaCorta(c.fecha),
          c.numero || "",
          c.numero_factura || "",
          c.proveedor_nombre || "",
          c.tipo_documento || "",
          (c.subtotal ?? 0).toFixed(2),
          (c.iva ?? 0).toFixed(2),
          (c.total ?? 0).toFixed(2),
        ]),
        cols_numericas: [5, 6, 7],
      };
    }
    return null;
  };


  const verDetalleCxc = async (cliente: any) => {
    try {
      const cuentas = await reporteCxcDetalleCliente(cliente.cliente_id);
      setCxcClienteDetalle({ cliente, cuentas });
    } catch (err) { toastError("Error: " + err); }
  };

  const verDetalleCxp = async (proveedor: any) => {
    try {
      const cuentas = await reporteCxpDetalleProveedor(proveedor.proveedor_id);
      setCxpProveedorDetalle({ proveedor, cuentas });
    } catch (err) { toastError("Error: " + err); }
  };

  const verKardex = async (productoId: number) => {
    try {
      const k = await reporteKardexProducto(productoId, desde, hasta);
      setKardexProducto(k);
    } catch (err) { toastError("Error: " + err); }
  };

  const calcularIva = async () => {
    setCargandoIva(true);
    try {
      const r = await reporteIvaMensual(ivaAnio, ivaMes);
      setIva(r);
    } catch (err) {
      toastError("Error: " + err);
    }
    setCargandoIva(false);
  };

  const exportarIvaCsv = async () => {
    if (!iva) return;
    try {
      const nombreMes = MESES[iva.mes - 1] || String(iva.mes);
      const ruta = await save({
        defaultPath: `declaracion-iva-${iva.anio}-${String(iva.mes).padStart(2, "0")}.csv`,
        filters: [{ name: "CSV", extensions: ["csv"] }],
      });
      if (!ruta) return;
      const lineas = [
        `Declaracion IVA - ${nombreMes} ${iva.anio}`,
        `Periodo:,${iva.fecha_desde} a ${iva.fecha_hasta}`,
        ``,
        `Concepto,Monto USD`,
        `Ventas tarifa 0%,${iva.ventas_0.toFixed(2)}`,
        `Ventas tarifa 15% (base imponible),${iva.ventas_15_base.toFixed(2)}`,
        `IVA cobrado en ventas (debito fiscal),${iva.iva_ventas.toFixed(2)}`,
        `(-) Notas de credito base,${iva.nc_base.toFixed(2)}`,
        `(-) Notas de credito IVA,${iva.nc_iva.toFixed(2)}`,
        `IVA ventas neto,${iva.iva_ventas_neto.toFixed(2)}`,
        ``,
        `Compras tarifa 0%,${iva.compras_0.toFixed(2)}`,
        `Compras tarifa 15% (base),${iva.compras_15_base.toFixed(2)}`,
        `IVA pagado en compras (credito tributario),${iva.iva_compras.toFixed(2)}`,
        ``,
        `Total ventas,${iva.total_ventas.toFixed(2)}`,
        `Total compras,${iva.total_compras.toFixed(2)}`,
        `IVA A PAGAR AL SRI,${iva.iva_a_pagar.toFixed(2)}`,
      ];
      const contenido = lineas.join("\n");
      await invoke("guardar_archivo_texto", { ruta, contenido });
      toastExito("CSV exportado");
    } catch (e) {
      toastError("Error: " + e);
    }
  };

  const setPeriodo = (d: string, h: string) => { setDesde(d); setHasta(h); };

  const fmt = (n: number) => `$${n.toFixed(2)}`;
  const fmtPct = (n: number) => `${n.toFixed(1)}%`;

  // --- Reporte de Compras: helpers (frontend-only) ---
  const comprasFiltradas: Compra[] = comprasData.filter(
    (c) => comprasProveedorFiltro === "TODOS" || String(c.proveedor_id) === comprasProveedorFiltro
  );
  const comprasTotalGeneral = comprasFiltradas.reduce((s, c) => s + (c.total ?? 0), 0);
  const comprasFechaCorta = (f?: string) => {
    if (!f) return "";
    const d = new Date(f);
    return isNaN(d.getTime()) ? (f.slice(0, 10)) : d.toLocaleDateString("es-EC");
  };
  // Agrupa por una clave y devuelve [clave, compras[]] ordenado
  const agruparCompras = (
    keyFn: (c: Compra) => string,
    ordenDesc = false
  ): Array<[string, Compra[]]> => {
    const mapa = new Map<string, Compra[]>();
    for (const c of comprasFiltradas) {
      const k = keyFn(c);
      if (!mapa.has(k)) mapa.set(k, []);
      mapa.get(k)!.push(c);
    }
    const entradas = Array.from(mapa.entries());
    entradas.sort((a, b) => ordenDesc ? b[0].localeCompare(a[0]) : a[0].localeCompare(b[0]));
    return entradas;
  };

  // Helper generico para exportar CSV
  const exportarCsvGeneric = async (nombreArchivo: string, encabezados: string[], filas: (string | number)[][]) => {
    try {
      const ruta = await save({
        defaultPath: nombreArchivo,
        filters: [{ name: "CSV", extensions: ["csv"] }],
      });
      if (!ruta) return;
      const escape = (v: any) => {
        const s = String(v ?? "");
        return s.includes(",") || s.includes('"') || s.includes("\n")
          ? `"${s.replace(/"/g, '""')}"` : s;
      };
      const lineas = [encabezados.map(escape).join(",")];
      for (const f of filas) {
        lineas.push(f.map(escape).join(","));
      }
      await invoke("guardar_archivo_texto", { ruta, contenido: lineas.join("\n") });
      toastExito("CSV exportado");
    } catch (e) { toastError("Error: " + e); }
  };

  const exportarCxcCsv = () => {
    if (cxcClienteDetalle) {
      // Exportar detalle del cliente
      const cli = cxcClienteDetalle.cliente;
      exportarCsvGeneric(
        `cxc-${cli.cliente_nombre.replace(/[^a-zA-Z0-9]/g, "_")}.csv`,
        ["Venta", "Fecha", "Total", "Pagado", "Saldo", "Estado", "Vencimiento", "Dias atraso"],
        cxcClienteDetalle.cuentas.map((c: any) => [
          c.venta_numero || "", c.venta_fecha?.slice(0, 10) || "",
          c.monto_total.toFixed(2), c.monto_pagado.toFixed(2), c.saldo.toFixed(2),
          c.estado, c.fecha_vencimiento || "", c.dias_atraso,
        ])
      );
    } else {
      exportarCsvGeneric(
        `cxc-por-cliente-${hoy()}.csv`,
        ["Cliente", "Identificacion", "Telefono", "Num Cuentas", "Total Facturado", "Total Pagado", "Saldo Pendiente", "Vencido", "Proximo Venc.", "Ultimo Pago"],
        cxcResumen.map((c: any) => [
          c.cliente_nombre, c.identificacion || "", c.telefono || "",
          c.num_cuentas, c.total_facturado.toFixed(2), c.total_pagado.toFixed(2),
          c.saldo_pendiente.toFixed(2), c.monto_vencido.toFixed(2),
          c.proximo_vencimiento || "", c.ultimo_pago_fecha?.slice(0, 10) || "",
        ])
      );
    }
  };

  const exportarCxpCsv = () => {
    if (cxpProveedorDetalle) {
      const p = cxpProveedorDetalle.proveedor;
      exportarCsvGeneric(
        `cxp-${p.proveedor_nombre.replace(/[^a-zA-Z0-9]/g, "_")}.csv`,
        ["Compra", "Fac. Proveedor", "Fecha", "Total", "Pagado", "Saldo", "Estado", "Vencimiento", "Dias atraso"],
        cxpProveedorDetalle.cuentas.map((c: any) => [
          c.compra_numero || "", c.numero_factura || "", c.compra_fecha?.slice(0, 10) || "",
          c.monto_total.toFixed(2), c.monto_pagado.toFixed(2), c.saldo.toFixed(2),
          c.estado, c.fecha_vencimiento || "", c.dias_atraso,
        ])
      );
    } else {
      exportarCsvGeneric(
        `cxp-por-proveedor-${hoy()}.csv`,
        ["Proveedor", "RUC", "Telefono", "Num Cuentas", "Total Facturado", "Total Pagado", "Saldo Pendiente", "Vencido", "Proximo Venc.", "Ultimo Pago"],
        cxpResumen.map((c: any) => [
          c.proveedor_nombre, c.ruc || "", c.telefono || "",
          c.num_cuentas, c.total_facturado.toFixed(2), c.total_pagado.toFixed(2),
          c.saldo_pendiente.toFixed(2), c.monto_vencido.toFixed(2),
          c.proximo_vencimiento || "", c.ultimo_pago_fecha?.slice(0, 10) || "",
        ])
      );
    }
  };

  const exportarInventarioCsvFn = () => {
    if (kardexProducto) {
      const p = kardexProducto.producto;
      exportarCsvGeneric(
        `kardex-${(p.codigo || p.nombre).replace(/[^a-zA-Z0-9]/g, "_")}.csv`,
        ["Fecha", "Tipo", "Cantidad", "Stock Anterior", "Stock Nuevo", "Costo Unitario", "Motivo", "Usuario"],
        kardexProducto.movimientos.map((m: any) => [
          m.fecha?.slice(0, 19).replace("T", " ") || "",
          m.tipo, m.cantidad, m.stock_anterior, m.stock_nuevo,
          m.costo_unitario ? m.costo_unitario.toFixed(2) : "",
          m.motivo || "", m.usuario || "",
        ])
      );
    } else if (inventario) {
      // Respetar filtros activos al exportar
      const filtrados = inventario.productos
        .filter((p: any) => filtroEstado === "TODOS" || p.estado_stock === filtroEstado)
        .filter((p: any) => filtroCategoriaInv === "TODAS" || p.categoria === filtroCategoriaInv)
        .filter((p: any) => !busquedaInv || p.nombre.toLowerCase().includes(busquedaInv.toLowerCase()) || (p.codigo || "").toLowerCase().includes(busquedaInv.toLowerCase()));
      const sufijoFiltro = [
        filtroCategoriaInv !== "TODAS" ? filtroCategoriaInv.replace(/[^a-zA-Z0-9]/g, "_") : null,
        filtroEstado !== "TODOS" ? filtroEstado : null,
      ].filter(Boolean).join("-");
      exportarCsvGeneric(
        `inventario-valorizado${sufijoFiltro ? "-" + sufijoFiltro : ""}-${hoy()}.csv`,
        ["Codigo", "Producto", "Categoria", "Stock Actual", "Stock Minimo", "Precio Costo", "Precio Venta", "Valor Costo", "Valor Venta", "Utilidad Potencial", "Estado Stock"],
        filtrados.map((p: any) => [
          p.codigo || "", p.nombre, p.categoria || "",
          p.stock_actual, p.stock_minimo,
          p.precio_costo.toFixed(2), p.precio_venta.toFixed(2),
          p.valor_costo.toFixed(2), p.valor_venta.toFixed(2),
          p.utilidad_potencial.toFixed(2),
          p.estado_stock === "SIN_STOCK" ? "Sin stock" : p.estado_stock === "BAJO" ? "Bajo" : "OK",
        ])
      );
    }
  };




  return (
    <>
      <div className="page-header">
        <h2>Reportes</h2>
        <div className="flex gap-2 items-center">
          <button className="btn btn-outline" style={{ fontSize: 11, padding: "4px 10px" }} onClick={() => setPeriodo(hoy(), hoy())}>Hoy</button>
          <button className="btn btn-outline" style={{ fontSize: 11, padding: "4px 10px" }} onClick={() => setPeriodo(hace7(), hoy())}>7 dias</button>
          <button className="btn btn-outline" style={{ fontSize: 11, padding: "4px 10px" }} onClick={() => setPeriodo(inicioMes(), hoy())}>Este mes</button>
          <button className="btn btn-outline" style={{ fontSize: 11, padding: "4px 10px" }} onClick={() => setPeriodo(inicioAnio(), hoy())}>Este ano</button>
          <span style={{ color: "var(--color-text-secondary)", fontSize: 11 }}>|</span>
          <input type="date" className="input" style={{ fontSize: 12, padding: "4px 8px", width: 130 }} value={desde} onChange={e => setDesde(e.target.value)} />
          <span style={{ fontSize: 11, color: "var(--color-text-secondary)" }}>a</span>
          <input type="date" className="input" style={{ fontSize: 12, padding: "4px 8px", width: 130 }} value={hasta} onChange={e => setHasta(e.target.value)} />
          {/* Botones export contextuales: Excel y PDF */}
          <button className="btn btn-primary" style={{ fontSize: 11, padding: "4px 14px", fontWeight: 600 }}
            onClick={async () => {
              try {
                const result = obtenerDatosTabla();
                if (!result) { toastError("No hay datos para exportar"); return; }
                const ruta = await save({
                  defaultPath: `${result.archivo}.xlsx`,
                  filters: [{ name: "Excel", extensions: ["xlsx"] }]
                });
                if (!ruta) return;
                if (tab === "inventario" && !kardexProducto) {
                  await exportarInventarioXlsx(ruta, filtroCategoriaInv, busquedaInv || undefined, filtroEstado);
                } else {
                  await exportarTablaXlsx(ruta, result.titulo, result.subtitulo, result.encabezados, result.filas, result.cols_numericas || null);
                }
                toastExito("Excel generado");
              } catch (e) { toastError("Error: " + e); }
            }}>📊 Excel</button>
          <button className="btn btn-outline" style={{ fontSize: 11, padding: "4px 14px", fontWeight: 600 }}
            onClick={async () => {
              try {
                const result = obtenerDatosTabla();
                if (!result) { toastError("No hay datos para exportar"); return; }
                const ruta = await save({
                  defaultPath: `${result.archivo}.pdf`,
                  filters: [{ name: "PDF", extensions: ["pdf"] }]
                });
                if (!ruta) return;
                if (tab === "inventario" && !kardexProducto) {
                  await exportarInventarioPdf(ruta, filtroCategoriaInv, busquedaInv || undefined, filtroEstado);
                } else {
                  await exportarTablaPdf(ruta, result.titulo, result.subtitulo, result.encabezados, result.filas, true);
                }
                toastExito("PDF generado");
              } catch (e) { toastError("Error: " + e); }
            }}>📄 PDF</button>
        </div>
      </div>

      <div className="page-body" style={{ padding: 16 }}>
        {/* Tabs — v2.5.29: flex-wrap + padding reducido para que entren todos
            los tabs (incluyendo Cancelaciones ST / Garantías ST) sin desbordarse
            cuando el módulo de Servicio Técnico está activo. */}
        <div style={{ display: "flex", flexWrap: "wrap", gap: 6, marginBottom: 16 }}>
          {/* v2.4.15: tabs de Servicio Tecnico solo si el modulo esta activo */}
          {(([
            ["utilidad", "Estado de Resultados"],
            ["balance", "Balance"],
            ["ventas", "Ventas detalladas"],
            ["productos", "Rentabilidad por Producto"],
            ["iva", "Declaracion IVA"],
            ["cxc", "Cuentas por Cobrar"],
            ["cxp", "Cuentas por Pagar"],
            ["compras", "Compras"],
            ["depositos", "🏦 Depósitos en tránsito"],
            ["inventario", "Inventario"],
            ["valuacion", "💼 Valuación"],
            ["kardex", "Kardex Multi"],
            ["cajeros", "Cajeros"],
            ["gastos", "💸 Gastos"],
            ...(moduloSTActivo ? [
              ["cancelaciones_st", "🚫 Cancelaciones ST"],
              ["garantias_st", "🛡 Garantías ST"],
            ] : []),
          ] as const) as ReadonlyArray<readonly [typeof tab, string]>).map(([key, label]) => (
            <button key={key} className={`btn ${tab === key ? "btn-primary" : "btn-outline"}`}
              style={{ fontSize: 12, padding: "5px 10px", whiteSpace: "nowrap" }} onClick={() => setTab(key)}>
              {label}
            </button>
          ))}
        </div>

        {cargando && <div style={{ textAlign: "center", padding: 40, color: "var(--color-text-secondary)" }}>Cargando...</div>}

        {/* Estado de Resultados */}
        {!cargando && tab === "utilidad" && utilidad && (
          <div style={{ display: "flex", flexDirection: "column", gap: 16 }}>
            {/* KPIs */}
            <div style={{ display: "grid", gridTemplateColumns: "repeat(auto-fit, minmax(180px, 1fr))", gap: 12 }}>
              <KpiCard label="Ventas Brutas" valor={fmt(utilidad.ventas_brutas)} />
              <KpiCard label="Costo de Ventas" valor={fmt(utilidad.costo_ventas)} color="var(--color-danger)" />
              <KpiCard label="Utilidad Bruta" valor={fmt(utilidad.utilidad_bruta)} sub={`Margen: ${fmtPct(utilidad.margen_bruto)}`} color={utilidad.utilidad_bruta >= 0 ? "var(--color-success)" : "var(--color-danger)"} />
              <KpiCard label="Gastos" valor={fmt(utilidad.total_gastos)} color="var(--color-warning)" />
              <KpiCard label="Devoluciones" valor={fmt(utilidad.total_devoluciones)} color="var(--color-danger)" />
              <KpiCard label="Utilidad Neta" valor={fmt(utilidad.utilidad_neta)} sub={`Margen: ${fmtPct(utilidad.margen_neto)}`} color={utilidad.utilidad_neta >= 0 ? "var(--color-success)" : "var(--color-danger)"} destacado />
              <KpiCard label="Transacciones" valor={String(utilidad.num_ventas)} />
              <KpiCard label="Promedio/Venta" valor={fmt(utilidad.promedio_por_venta)} />
            </div>

            {/* Charts */}
            <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: 16 }}>
              {/* Utilidad por categoría (cada barra con color distinto) */}
              {utilidad.por_categoria.length > 0 && (
                <div className="card">
                  <div className="card-header">Utilidad por Categoria</div>
                  <div className="card-body" style={{ height: 250 }}>
                    <ResponsiveContainer>
                      <BarChart data={utilidad.por_categoria} layout="vertical" margin={{ left: 80 }}>
                        <CartesianGrid strokeDasharray="3 3" stroke="var(--color-border)" />
                        <XAxis type="number" tickFormatter={v => `$${v}`} style={{ fontSize: 11 }} />
                        <YAxis type="category" dataKey="categoria" style={{ fontSize: 11 }} width={75} />
                        <Tooltip formatter={(v: unknown) => fmt(Number(v))} />
                        <Bar dataKey="utilidad" radius={[0, 4, 4, 0]}>
                          {utilidad.por_categoria.map((_, i) => (
                            <Cell key={i} fill={COLORES_PIE[i % COLORES_PIE.length]} />
                          ))}
                        </Bar>
                      </BarChart>
                    </ResponsiveContainer>
                  </div>
                </div>
              )}

              {/* Gastos por categoría */}
              {utilidad.gastos_por_categoria.length > 0 && (
                <div className="card">
                  <div className="card-header">Gastos por Categoria</div>
                  <div className="card-body" style={{ height: 250 }}>
                    <ResponsiveContainer>
                      <PieChart>
                        <Pie data={utilidad.gastos_por_categoria} dataKey="monto" nameKey="categoria" cx="50%" cy="50%" outerRadius={80} label={({ name, percent }: { name?: string; percent?: number }) => `${name || ""} ${((percent || 0) * 100).toFixed(0)}%`} labelLine={{ stroke: "var(--color-text-secondary)" }}>
                          {utilidad.gastos_por_categoria.map((_, i) => <Cell key={i} fill={COLORES_PIE[i % COLORES_PIE.length]} />)}
                        </Pie>
                        <Tooltip formatter={(v: unknown) => fmt(Number(v))} />
                      </PieChart>
                    </ResponsiveContainer>
                  </div>
                </div>
              )}
            </div>
          </div>
        )}

        {/* Balance */}
        {!cargando && tab === "balance" && balance && (
          <div style={{ display: "flex", flexDirection: "column", gap: 16 }}>
            <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr 1fr", gap: 16 }}>
              {/* Ingresos */}
              <div className="card">
                <div className="card-header" style={{ color: "var(--color-success)", fontWeight: 700 }}>INGRESOS</div>
                <div className="card-body">
                  <BalanceRow label="Efectivo" valor={balance.ingresos_efectivo} />
                  <BalanceRow label="Transferencias" valor={balance.ingresos_transferencia} />
                  <BalanceRow label="Creditos cobrados" valor={balance.ingresos_credito_cobrado} />
                  <div style={{ borderTop: "2px solid var(--color-border)", marginTop: 8, paddingTop: 8 }}>
                    <BalanceRow label="TOTAL INGRESOS" valor={balance.total_ingresos} bold color="var(--color-success)" />
                  </div>
                </div>
              </div>

              {/* Egresos */}
              <div className="card">
                <div className="card-header" style={{ color: "var(--color-danger)", fontWeight: 700 }}>EGRESOS</div>
                <div className="card-body">
                  {balance.gastos_por_categoria.map(g => (
                    <BalanceRow key={g.categoria} label={g.categoria} valor={g.monto} />
                  ))}
                  {balance.total_devoluciones > 0 && <BalanceRow label="Devoluciones" valor={balance.total_devoluciones} />}
                  <div style={{ borderTop: "2px solid var(--color-border)", marginTop: 8, paddingTop: 8 }}>
                    <BalanceRow label="TOTAL EGRESOS" valor={balance.total_egresos} bold color="var(--color-danger)" />
                  </div>
                </div>
              </div>

              {/* Resultado */}
              <div className="card">
                <div className="card-header" style={{ fontWeight: 700 }}>RESULTADO</div>
                <div className="card-body">
                  <div style={{ textAlign: "center", padding: "20px 0" }}>
                    <div style={{ fontSize: 32, fontWeight: 800, color: balance.resultado >= 0 ? "var(--color-success)" : "var(--color-danger)" }}>
                      {fmt(balance.resultado)}
                    </div>
                    <div style={{ fontSize: 13, color: "var(--color-text-secondary)", marginTop: 4 }}>
                      {balance.resultado >= 0 ? "Ganancia del periodo" : "Perdida del periodo"}
                    </div>
                  </div>
                  <div style={{ borderTop: "1px solid var(--color-border)", paddingTop: 12, marginTop: 12 }}>
                    <BalanceRow label="Cuentas por cobrar" valor={balance.cuentas_por_cobrar} color="var(--color-warning)" />
                    <BalanceRow label="Valor inventario" valor={balance.valor_inventario} color="var(--color-primary)" />
                  </div>
                </div>
              </div>
            </div>
          </div>
        )}

        {/* Rentabilidad por Producto */}
        {!cargando && tab === "productos" && (
          <div className="card">
            <div className="card-header">
              Rentabilidad por Producto
              <span style={{ float: "right", fontSize: 12, color: "var(--color-text-secondary)" }}>{productos.length} productos</span>
            </div>
            <table className="table">
              <thead>
                <tr>
                  <th>Producto</th>
                  <th>Categoria</th>
                  <th className="text-right">Cant.</th>
                  <th className="text-right">Ingreso</th>
                  <th className="text-right">Costo</th>
                  <th className="text-right">Utilidad</th>
                  <th style={{ width: 120 }}>Margen</th>
                </tr>
              </thead>
              <tbody>
                {productos.length === 0 ? (
                  <tr><td colSpan={7} className="text-center text-secondary" style={{ padding: 40 }}>Sin datos para este periodo</td></tr>
                ) : productos.map((p, i) => (
                  <tr key={i}>
                    <td><strong>{p.nombre}</strong></td>
                    <td style={{ fontSize: 12, color: "var(--color-text-secondary)" }}>{p.categoria}</td>
                    <td className="text-right">{p.cantidad.toFixed(0)}</td>
                    <td className="text-right">{fmt(p.ingreso)}</td>
                    <td className="text-right">{fmt(p.costo)}</td>
                    <td className="text-right" style={{ fontWeight: 600, color: p.utilidad >= 0 ? "var(--color-success)" : "var(--color-danger)" }}>{fmt(p.utilidad)}</td>
                    <td>
                      <div style={{ display: "flex", alignItems: "center", gap: 6 }}>
                        <div style={{ flex: 1, height: 8, background: "var(--color-border)", borderRadius: 4, overflow: "hidden" }}>
                          <div style={{
                            width: `${Math.min(Math.max(p.margen, 0), 100)}%`, height: "100%", borderRadius: 4,
                            background: p.margen < 10 ? "var(--color-danger)" : p.margen < 25 ? "var(--color-warning)" : "var(--color-success)",
                          }} />
                        </div>
                        <span style={{ fontSize: 11, fontWeight: 600, minWidth: 40, textAlign: "right",
                          color: p.margen < 10 ? "var(--color-danger)" : p.margen < 25 ? "var(--color-warning)" : "var(--color-success)" }}>
                          {fmtPct(p.margen)}
                        </span>
                      </div>
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        )}

        {/* Declaracion IVA Mensual (Formulario 104 SRI) */}
        {tab === "iva" && (
          <div style={{ display: "flex", flexDirection: "column", gap: 16 }}>
            {/* Selectores de periodo */}
            <div className="card">
              <div className="card-body" style={{ display: "flex", gap: 12, alignItems: "center", flexWrap: "wrap" }}>
                <label style={{ fontSize: 13, fontWeight: 600 }}>Periodo:</label>
                <select
                  className="input"
                  style={{ fontSize: 13, padding: "6px 10px", width: 120 }}
                  value={ivaMes}
                  onChange={e => setIvaMes(Number(e.target.value))}
                >
                  {MESES.map((m, i) => (
                    <option key={i + 1} value={i + 1}>{m}</option>
                  ))}
                </select>
                <select
                  className="input"
                  style={{ fontSize: 13, padding: "6px 10px", width: 100 }}
                  value={ivaAnio}
                  onChange={e => setIvaAnio(Number(e.target.value))}
                >
                  {[2024, 2025, 2026, 2027].map(a => (
                    <option key={a} value={a}>{a}</option>
                  ))}
                </select>
                <button
                  className="btn btn-primary"
                  style={{ fontSize: 13, padding: "6px 16px" }}
                  onClick={calcularIva}
                  disabled={cargandoIva}
                >
                  {cargandoIva ? "Calculando..." : "Calcular"}
                </button>
                {iva && (
                  <button
                    className="btn btn-outline"
                    style={{ fontSize: 13, padding: "6px 16px" }}
                    onClick={exportarIvaCsv}
                  >
                    Exportar CSV
                  </button>
                )}
              </div>
            </div>

            {cargandoIva && (
              <div style={{ textAlign: "center", padding: 40, color: "var(--color-text-secondary)" }}>Calculando...</div>
            )}

            {!cargandoIva && iva && (
              <div className="card">
                <div className="card-header" style={{ fontWeight: 700 }}>
                  Declaracion de IVA - {MESES[iva.mes - 1]} {iva.anio}
                  <span style={{ float: "right", fontSize: 12, color: "var(--color-text-secondary)", fontWeight: 400 }}>
                    Periodo: {iva.fecha_desde} al {iva.fecha_hasta}
                  </span>
                </div>
                <div className="card-body" style={{ padding: "16px 20px" }}>
                  {/* Seccion Ventas */}
                  <div style={{ borderBottom: "2px solid var(--color-border)", paddingBottom: 6, marginBottom: 10, marginTop: 4 }}>
                    <span style={{ fontSize: 13, fontWeight: 700, color: "var(--color-success)" }}>VENTAS</span>
                  </div>
                  <IvaRow label="Ventas tarifa 0% (gravadas 0%)" valor={iva.ventas_0} />
                  <IvaRow label="Ventas tarifa 15% (base imponible)" valor={iva.ventas_15_base} />
                  <IvaRow label="IVA cobrado en ventas (debito fiscal)" valor={iva.iva_ventas} bold />
                  <IvaRow label="(-) Notas de credito base" valor={iva.nc_base} indent color="var(--color-warning)" />
                  <IvaRow label="(-) Notas de credito IVA" valor={iva.nc_iva} indent color="var(--color-warning)" />
                  <IvaRow label="IVA ventas neto" valor={iva.iva_ventas_neto} bold color="var(--color-success)" />

                  {/* Seccion Compras */}
                  <div style={{ borderBottom: "2px solid var(--color-border)", paddingBottom: 6, marginBottom: 10, marginTop: 20 }}>
                    <span style={{ fontSize: 13, fontWeight: 700, color: "var(--color-primary)" }}>COMPRAS</span>
                  </div>
                  <IvaRow label="Compras tarifa 0%" valor={iva.compras_0} />
                  <IvaRow label="Compras tarifa 15% (base)" valor={iva.compras_15_base} />
                  <IvaRow label="IVA pagado en compras (credito tributario)" valor={iva.iva_compras} bold color="var(--color-primary)" />

                  {/* Seccion Resumen */}
                  <div style={{ borderBottom: "2px solid var(--color-border)", paddingBottom: 6, marginBottom: 10, marginTop: 20 }}>
                    <span style={{ fontSize: 13, fontWeight: 700 }}>RESUMEN</span>
                  </div>
                  <IvaRow label="Total ventas" valor={iva.total_ventas} />
                  <IvaRow label="Total compras" valor={iva.total_compras} />

                  {/* IVA A PAGAR - destacado */}
                  <div
                    style={{
                      marginTop: 16,
                      padding: "14px 16px",
                      borderRadius: 8,
                      background: iva.iva_a_pagar > 0
                        ? "rgba(239, 68, 68, 0.15)"
                        : "rgba(34, 197, 94, 0.15)",
                      border: `2px solid ${iva.iva_a_pagar > 0 ? "var(--color-danger)" : "var(--color-success)"}`,
                    }}
                  >
                    <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
                      <div>
                        <div style={{
                          fontSize: 14,
                          fontWeight: 700,
                          color: iva.iva_a_pagar > 0 ? "var(--color-danger)" : "var(--color-success)",
                        }}>
                          {iva.iva_a_pagar > 0 ? "IVA A PAGAR AL SRI" : "CREDITO FISCAL A FAVOR"}
                        </div>
                        <div style={{ fontSize: 11, color: "var(--color-text-secondary)", marginTop: 2 }}>
                          (IVA ventas neto - IVA compras)
                        </div>
                      </div>
                      <div style={{
                        fontSize: 24,
                        fontWeight: 800,
                        color: iva.iva_a_pagar > 0 ? "var(--color-danger)" : "var(--color-success)",
                      }}>
                        {fmt(Math.abs(iva.iva_a_pagar))}
                      </div>
                    </div>
                  </div>

                  {/* Helper text */}
                  <div
                    style={{
                      marginTop: 20,
                      padding: 12,
                      borderRadius: 6,
                      background: "rgba(59, 130, 246, 0.1)",
                      fontSize: 12,
                      color: "var(--color-text-secondary)",
                      lineHeight: 1.5,
                    }}
                  >
                    <strong>Nota:</strong> Este reporte es una ayuda para preparar su declaracion mensual
                    (Formulario 104 del SRI). Consulte con su contador antes de presentar al SRI.
                  </div>
                </div>
              </div>
            )}

            {!cargandoIva && !iva && (
              <div className="card">
                <div className="card-body" style={{ textAlign: "center", padding: 40, color: "var(--color-text-secondary)" }}>
                  Seleccione un periodo y presione <strong>Calcular</strong> para generar el reporte de IVA mensual.
                </div>
              </div>
            )}
          </div>
        )}

        {/* TAB CXC */}
        {tab === "cxc" && (
          cxcClienteDetalle ? (
            <div>
              <div style={{ display: "flex", alignItems: "center", gap: 10, marginBottom: 12 }}>
                <button className="btn btn-outline" onClick={() => setCxcClienteDetalle(null)}>← Volver</button>
                <h3 style={{ margin: 0, flex: 1 }}>{cxcClienteDetalle.cliente.cliente_nombre}</h3>
                <span style={{ color: "var(--color-text-secondary)", fontSize: 12 }}>
                  {cxcClienteDetalle.cliente.identificacion} · {cxcClienteDetalle.cliente.telefono}
                </span>
                <button className="btn btn-outline" onClick={exportarCxcCsv} style={{ fontSize: 11 }}>📥 Exportar CSV</button>
              </div>
              <div className="card">
                <table className="table" style={{ width: "100%" }}>
                  <thead>
                    <tr>
                      <th>Venta</th><th>Fecha</th><th className="text-right">Total</th>
                      <th className="text-right">Pagado</th><th className="text-right">Saldo</th>
                      <th>Estado</th><th>Vencimiento</th><th className="text-right">Atraso</th>
                    </tr>
                  </thead>
                  <tbody>
                    {cxcClienteDetalle.cuentas.map((c: any) => (
                      <tr key={c.cuenta_id}>
                        <td>{c.venta_numero}</td>
                        <td>{c.venta_fecha?.slice(0, 10)}</td>
                        <td className="text-right">{fmt(c.monto_total)}</td>
                        <td className="text-right">{fmt(c.monto_pagado)}</td>
                        <td className="text-right" style={{ fontWeight: 700 }}>{fmt(c.saldo)}</td>
                        <td><span style={{ fontSize: 11, padding: "3px 10px", borderRadius: 4, fontWeight: 700, letterSpacing: 0.3,
                          background: c.estado === "VENCIDA" ? "#dc2626" : c.estado === "ABONADA" ? "#d97706" : c.estado === "PAGADA" ? "#16a34a" : "#2563eb",
                          color: "#ffffff",
                          border: c.estado === "VENCIDA" ? "1px solid #991b1b" : c.estado === "ABONADA" ? "1px solid #92400e" : c.estado === "PAGADA" ? "1px solid #166534" : "1px solid #1e40af",
                        }}>{c.estado}</span></td>
                        <td>{c.fecha_vencimiento}</td>
                        <td className="text-right" style={{ color: c.dias_atraso > 0 ? "var(--color-danger)" : "var(--color-text-secondary)", fontWeight: c.dias_atraso > 0 ? 700 : 400 }}>
                          {c.dias_atraso > 0 ? `+${c.dias_atraso}d` : "—"}
                        </td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              </div>
            </div>
          ) : (
            <div>
              <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: 12, gap: 10 }}>
                <span style={{ fontSize: 12, color: "var(--color-text-secondary)" }}>Click en un cliente para ver sus cuentas detalladas</span>
                <input className="input" placeholder="🔍 Buscar cliente..." style={{ flex: 1, maxWidth: 300, fontSize: 12 }}
                  value={busquedaCliente} onChange={(e) => setBusquedaCliente(e.target.value)} />
              </div>
              <div style={{ display: "grid", gridTemplateColumns: "repeat(3, 1fr)", gap: 12, marginBottom: 16 }}>
                <KpiCard label="Clientes con saldo" valor={String(cxcResumen.length)} />
                <KpiCard label="Saldo total pendiente" valor={fmt(cxcResumen.reduce((s, c) => s + c.saldo_pendiente, 0))} color="var(--color-warning)" />
                <KpiCard label="Vencido" valor={fmt(cxcResumen.reduce((s, c) => s + c.monto_vencido, 0))} color="var(--color-danger)" />
              </div>
              <div className="card">
                <table className="table" style={{ width: "100%" }}>
                  <thead>
                    <tr>
                      <th>Cliente</th><th>Identificacion</th><th>Telefono</th>
                      <th className="text-right">Cuentas</th><th className="text-right">Saldo</th>
                      <th className="text-right">Vencido</th><th>Proximo Venc.</th><th>Ultimo Pago</th><th></th>
                    </tr>
                  </thead>
                  <tbody>
                    {cxcResumen.length === 0 ? (
                      <tr><td colSpan={9} style={{ textAlign: "center", padding: 30, color: "var(--color-text-secondary)" }}>Sin cuentas pendientes</td></tr>
                    ) : cxcResumen
                      .filter((c: any) => !busquedaCliente || c.cliente_nombre.toLowerCase().includes(busquedaCliente.toLowerCase()) || (c.identificacion || "").includes(busquedaCliente))
                      .map((c: any) => (
                      <tr key={c.cliente_id} style={{ cursor: "pointer" }} onClick={() => verDetalleCxc(c)}>
                        <td style={{ fontWeight: 600 }}>{c.cliente_nombre}</td>
                        <td style={{ fontSize: 11 }}>{c.identificacion || "-"}</td>
                        <td style={{ fontSize: 11 }}>{c.telefono || "-"}</td>
                        <td className="text-right">{c.num_cuentas}</td>
                        <td className="text-right" style={{ fontWeight: 700 }}>{fmt(c.saldo_pendiente)}</td>
                        <td className="text-right" style={{ color: c.monto_vencido > 0 ? "var(--color-danger)" : "var(--color-text-secondary)", fontWeight: c.monto_vencido > 0 ? 700 : 400 }}>
                          {c.monto_vencido > 0 ? fmt(c.monto_vencido) : "—"}
                        </td>
                        <td style={{ fontSize: 11 }}>{c.proximo_vencimiento || "—"}</td>
                        <td style={{ fontSize: 11 }}>{c.ultimo_pago_fecha?.slice(0, 10) || "Sin pagos"}</td>
                        <td><button className="btn btn-outline" style={{ fontSize: 10, padding: "2px 8px" }}>Ver detalle</button></td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              </div>
            </div>
          )
        )}

        {/* TAB CXP */}
        {tab === "cxp" && (
          cxpProveedorDetalle ? (
            <div>
              <div style={{ display: "flex", alignItems: "center", gap: 10, marginBottom: 12 }}>
                <button className="btn btn-outline" onClick={() => setCxpProveedorDetalle(null)}>← Volver</button>
                <h3 style={{ margin: 0, flex: 1 }}>{cxpProveedorDetalle.proveedor.proveedor_nombre}</h3>
                <span style={{ color: "var(--color-text-secondary)", fontSize: 12 }}>
                  RUC: {cxpProveedorDetalle.proveedor.ruc} · {cxpProveedorDetalle.proveedor.telefono}
                </span>
                <button className="btn btn-outline" onClick={exportarCxpCsv} style={{ fontSize: 11 }}>📥 Exportar CSV</button>
              </div>
              <div className="card">
                <table className="table" style={{ width: "100%" }}>
                  <thead>
                    <tr>
                      <th>Compra</th><th>Fac. Proveedor</th><th>Fecha</th>
                      <th className="text-right">Total</th><th className="text-right">Pagado</th><th className="text-right">Saldo</th>
                      <th>Estado</th><th>Vencimiento</th><th className="text-right">Atraso</th>
                    </tr>
                  </thead>
                  <tbody>
                    {cxpProveedorDetalle.cuentas.map((c: any) => (
                      <tr key={c.cuenta_id}>
                        <td>{c.compra_numero}</td>
                        <td>{c.numero_factura || "-"}</td>
                        <td>{c.compra_fecha?.slice(0, 10)}</td>
                        <td className="text-right">{fmt(c.monto_total)}</td>
                        <td className="text-right">{fmt(c.monto_pagado)}</td>
                        <td className="text-right" style={{ fontWeight: 700 }}>{fmt(c.saldo)}</td>
                        <td><span style={{ fontSize: 11, padding: "3px 10px", borderRadius: 4, fontWeight: 700, letterSpacing: 0.3,
                          background: c.estado === "VENCIDA" ? "#dc2626" : c.estado === "ABONADA" ? "#d97706" : c.estado === "PAGADA" ? "#16a34a" : "#2563eb",
                          color: "#ffffff",
                          border: c.estado === "VENCIDA" ? "1px solid #991b1b" : c.estado === "ABONADA" ? "1px solid #92400e" : c.estado === "PAGADA" ? "1px solid #166534" : "1px solid #1e40af",
                        }}>{c.estado}</span></td>
                        <td>{c.fecha_vencimiento}</td>
                        <td className="text-right" style={{ color: c.dias_atraso > 0 ? "var(--color-danger)" : "var(--color-text-secondary)", fontWeight: c.dias_atraso > 0 ? 700 : 400 }}>
                          {c.dias_atraso > 0 ? `+${c.dias_atraso}d` : "—"}
                        </td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              </div>
            </div>
          ) : (
            <div>
              <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: 12, gap: 10 }}>
                <span style={{ fontSize: 12, color: "var(--color-text-secondary)" }}>Click en un proveedor para ver sus cuentas detalladas</span>
                <input className="input" placeholder="🔍 Buscar proveedor..." style={{ flex: 1, maxWidth: 300, fontSize: 12 }}
                  value={busquedaProveedor} onChange={(e) => setBusquedaProveedor(e.target.value)} />
              </div>
              <div style={{ display: "grid", gridTemplateColumns: "repeat(3, 1fr)", gap: 12, marginBottom: 16 }}>
                <KpiCard label="Proveedores con saldo" valor={String(cxpResumen.length)} />
                <KpiCard label="Saldo total pendiente" valor={fmt(cxpResumen.reduce((s, c) => s + c.saldo_pendiente, 0))} color="var(--color-warning)" />
                <KpiCard label="Vencido" valor={fmt(cxpResumen.reduce((s, c) => s + c.monto_vencido, 0))} color="var(--color-danger)" />
              </div>
              <div className="card">
                <table className="table" style={{ width: "100%" }}>
                  <thead>
                    <tr>
                      <th>Proveedor</th><th>RUC</th><th>Telefono</th>
                      <th className="text-right">Cuentas</th><th className="text-right">Saldo</th>
                      <th className="text-right">Vencido</th><th>Proximo Venc.</th><th>Ultimo Pago</th><th></th>
                    </tr>
                  </thead>
                  <tbody>
                    {cxpResumen.length === 0 ? (
                      <tr><td colSpan={9} style={{ textAlign: "center", padding: 30, color: "var(--color-text-secondary)" }}>Sin cuentas pendientes</td></tr>
                    ) : cxpResumen
                      .filter((c: any) => !busquedaProveedor || c.proveedor_nombre.toLowerCase().includes(busquedaProveedor.toLowerCase()) || (c.ruc || "").includes(busquedaProveedor))
                      .map((c: any) => (
                      <tr key={c.proveedor_id} style={{ cursor: "pointer" }} onClick={() => verDetalleCxp(c)}>
                        <td style={{ fontWeight: 600 }}>{c.proveedor_nombre}</td>
                        <td style={{ fontSize: 11 }}>{c.ruc || "-"}</td>
                        <td style={{ fontSize: 11 }}>{c.telefono || "-"}</td>
                        <td className="text-right">{c.num_cuentas}</td>
                        <td className="text-right" style={{ fontWeight: 700 }}>{fmt(c.saldo_pendiente)}</td>
                        <td className="text-right" style={{ color: c.monto_vencido > 0 ? "var(--color-danger)" : "var(--color-text-secondary)", fontWeight: c.monto_vencido > 0 ? 700 : 400 }}>
                          {c.monto_vencido > 0 ? fmt(c.monto_vencido) : "—"}
                        </td>
                        <td style={{ fontSize: 11 }}>{c.proximo_vencimiento || "—"}</td>
                        <td style={{ fontSize: 11 }}>{c.ultimo_pago_fecha?.slice(0, 10) || "Sin pagos"}</td>
                        <td><button className="btn btn-outline" style={{ fontSize: 10, padding: "2px 8px" }}>Ver detalle</button></td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              </div>
            </div>
          )
        )}

        {/* TAB INVENTARIO */}
        {tab === "inventario" && (
          kardexProducto ? (
            <div>
              <div style={{ display: "flex", alignItems: "center", gap: 10, marginBottom: 12 }}>
                <button
                  className="btn"
                  onClick={() => setKardexProducto(null)}
                  style={{
                    background: "var(--color-primary)",
                    color: "#fff",
                    fontWeight: 600,
                    border: "1px solid var(--color-primary)",
                    boxShadow: "0 1px 2px rgba(0,0,0,0.15)",
                  }}>
                  ← Volver al inventario
                </button>
                <h3 style={{ margin: 0, flex: 1 }}>Kardex: {kardexProducto.producto.nombre}</h3>
                <span style={{ color: "var(--color-text-secondary)", fontSize: 12 }}>
                  Stock actual: <strong>{kardexProducto.producto.stock_actual} {kardexProducto.producto.unidad_medida}</strong>
                </span>
                <button className="btn btn-outline" onClick={exportarInventarioCsvFn} style={{ fontSize: 11 }}>📥 Exportar CSV</button>
              </div>
              <div className="card">
                <table className="table" style={{ width: "100%" }}>
                  <thead>
                    <tr>
                      <th></th>
                      <th>Fecha</th><th>Tipo / Documento</th><th className="text-right">Cantidad</th>
                      <th className="text-right">Stock Anterior</th><th className="text-right">Stock Nuevo</th>
                      <th className="text-right">Costo</th><th>Motivo</th><th>Usuario</th>
                    </tr>
                  </thead>
                  <tbody>
                    {kardexProducto.movimientos.length === 0 ? (
                      <tr><td colSpan={9} style={{ textAlign: "center", padding: 30, color: "var(--color-text-secondary)" }}>Sin movimientos en este periodo</td></tr>
                    ) : kardexProducto.movimientos.map((m: any) => {
                      const expandido = kardexExpandido === m.id;
                      const tieneDetalle = m.documento || m.venta_cliente || m.compra_proveedor || m.venta_autorizacion;
                      const esVenta = m.tipo === "VENTA" || m.tipo === "VENTA_COMBO";
                      const esCompra = m.tipo?.startsWith("COMPRA") || m.tipo?.startsWith("INGRESO");
                      const aut = m.venta_estado_sri === "AUTORIZADA";
                      return (
                        <React.Fragment key={m.id}>
                          <tr style={{ cursor: tieneDetalle ? "pointer" : "default" }}
                              onClick={() => tieneDetalle && setKardexExpandido(expandido ? null : m.id)}>
                            <td style={{ width: 24, fontSize: 12, color: "var(--color-text-secondary)" }}>
                              {tieneDetalle ? (expandido ? "▼" : "▶") : ""}
                            </td>
                            <td style={{ fontSize: 11 }}>{m.fecha?.slice(0, 16).replace("T", " ")}</td>
                            <td>
                              <span style={{
                                fontSize: 10, padding: "2px 6px", borderRadius: 3,
                                background: esVenta ? "rgba(239,68,68,0.15)" : esCompra ? "rgba(34,197,94,0.15)" : "rgba(148,163,184,0.15)",
                                color: esVenta ? "var(--color-danger)" : esCompra ? "var(--color-success)" : "var(--color-text-secondary)"
                              }}>{m.tipo}</span>
                              {m.documento && (
                                <span style={{ marginLeft: 6, fontSize: 11, fontWeight: 600 }}>
                                  {m.documento}
                                </span>
                              )}
                              {esVenta && (
                                <span style={{ marginLeft: 6, fontSize: 9, padding: "1px 5px", borderRadius: 3,
                                  background: aut ? "rgba(34,197,94,0.15)" : "rgba(245,158,11,0.15)",
                                  color: aut ? "var(--color-success)" : "var(--color-warning)" }}>
                                  {aut ? "✓ SRI" : "Sin autorizar"}
                                </span>
                              )}
                            </td>
                            <td className="text-right" style={{ color: m.cantidad < 0 ? "var(--color-danger)" : "var(--color-success)", fontWeight: 600 }}>
                              {m.cantidad > 0 ? "+" : ""}{m.cantidad}
                            </td>
                            <td className="text-right">{m.stock_anterior}</td>
                            <td className="text-right" style={{ fontWeight: 600 }}>{m.stock_nuevo}</td>
                            <td className="text-right">{m.costo_unitario ? fmt(m.costo_unitario) : "-"}</td>
                            <td style={{ fontSize: 11 }}>{m.motivo || "-"}</td>
                            <td style={{ fontSize: 11 }}>{m.usuario || "-"}</td>
                          </tr>
                          {expandido && tieneDetalle && (
                            <tr>
                              <td colSpan={9} style={{ background: "var(--color-surface-alt)", padding: 12, fontSize: 11 }}>
                                {esVenta && (
                                  <div style={{ display: "grid", gridTemplateColumns: "repeat(auto-fit, minmax(220px, 1fr))", gap: 12 }}>
                                    {m.venta_cliente && <div><strong>Cliente:</strong> {m.venta_cliente}</div>}
                                    {m.venta_total != null && <div><strong>Total venta:</strong> {fmt(m.venta_total)}</div>}
                                    {m.venta_numero && <div><strong>N° interno:</strong> {m.venta_numero}</div>}
                                    {m.venta_numero_factura && <div><strong>N° factura SRI:</strong> {m.venta_numero_factura}</div>}
                                    {m.venta_autorizacion && (
                                      <div style={{ gridColumn: "1 / -1" }}>
                                        <strong>Autorización SRI:</strong> <span style={{ fontFamily: "monospace", fontSize: 10 }}>{m.venta_autorizacion}</span>
                                      </div>
                                    )}
                                    {m.venta_clave && (
                                      <div style={{ gridColumn: "1 / -1" }}>
                                        <strong>Clave de acceso:</strong> <span style={{ fontFamily: "monospace", fontSize: 10, wordBreak: "break-all" }}>{m.venta_clave}</span>
                                      </div>
                                    )}
                                  </div>
                                )}
                                {esCompra && (
                                  <div style={{ display: "grid", gridTemplateColumns: "repeat(auto-fit, minmax(220px, 1fr))", gap: 12 }}>
                                    {m.compra_proveedor && <div><strong>Proveedor:</strong> {m.compra_proveedor}</div>}
                                    {m.compra_total != null && <div><strong>Total compra:</strong> {fmt(m.compra_total)}</div>}
                                    {m.compra_numero && <div><strong>N° interno:</strong> {m.compra_numero}</div>}
                                    {m.compra_numero_factura && <div><strong>N° factura proveedor:</strong> {m.compra_numero_factura}</div>}
                                  </div>
                                )}
                              </td>
                            </tr>
                          )}
                        </React.Fragment>
                      );
                    })}
                  </tbody>
                </table>
              </div>
            </div>
          ) : inventario && (
            <div>
              <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: 12 }}>
                <span style={{ fontSize: 12, color: "var(--color-text-secondary)" }}>Click en "Kardex" para ver el detalle de movimientos de un producto</span>
                <button className="btn btn-outline" onClick={exportarInventarioCsvFn} style={{ fontSize: 11 }}>📥 Exportar CSV</button>
              </div>
              <div style={{ display: "grid", gridTemplateColumns: "repeat(4, 1fr)", gap: 12, marginBottom: 16 }}>
                <KpiCard label="Productos" valor={String(inventario.total_productos)} sub={`${inventario.total_unidades.toFixed(0)} unidades`} />
                <KpiCard label="Valor al costo" valor={fmt(inventario.valor_total_costo)} color="var(--color-text)" />
                <KpiCard label="Valor al precio venta" valor={fmt(inventario.valor_total_venta)} color="var(--color-primary)" />
                <KpiCard label="Utilidad potencial" valor={fmt(inventario.utilidad_potencial)} sub={fmtPct(inventario.valor_total_costo > 0 ? (inventario.utilidad_potencial / inventario.valor_total_costo * 100) : 0)} color="var(--color-success)" />
              </div>
              {(inventario.productos_sin_stock > 0 || inventario.productos_stock_bajo > 0 || ((inventario as any).productos_stock_negativo ?? 0) > 0) && (
                <div style={{ display: "flex", gap: 12, marginBottom: 12, flexWrap: "wrap" }}>
                  {((inventario as any).productos_stock_negativo ?? 0) > 0 && (
                    <div style={{ padding: "8px 14px", background: "rgba(220,38,38,0.15)", border: "1px solid rgba(220,38,38,0.5)", borderRadius: 6, fontSize: 12, color: "var(--color-danger)", fontWeight: 600, cursor: "pointer", display: "flex", alignItems: "center", gap: 8 }}
                         title="Clic para corregir: contar el stock real y ajustar en lote."
                         onClick={() => setModalStockNeg(true)}>
                      ⚠ <strong>{(inventario as any).productos_stock_negativo}</strong> productos con stock NEGATIVO
                      <span style={{ textDecoration: "underline", fontWeight: 700 }}>→ Corregir</span>
                    </div>
                  )}
                  {inventario.productos_sin_stock > 0 && (
                    <div style={{ padding: "8px 14px", background: "rgba(239,68,68,0.1)", border: "1px solid rgba(239,68,68,0.3)", borderRadius: 6, fontSize: 12, color: "var(--color-danger)" }}>
                      <strong>{inventario.productos_sin_stock}</strong> productos sin stock
                    </div>
                  )}
                  {inventario.productos_stock_bajo > 0 && (
                    <div style={{ padding: "8px 14px", background: "rgba(245,158,11,0.1)", border: "1px solid rgba(245,158,11,0.3)", borderRadius: 6, fontSize: 12, color: "var(--color-warning)" }}>
                      <strong>{inventario.productos_stock_bajo}</strong> productos con stock bajo
                    </div>
                  )}
                  <div style={{ padding: "8px 14px", background: "rgba(59,130,246,0.08)", borderRadius: 6, fontSize: 11, color: "var(--color-text-secondary)", flex: 1, minWidth: 240 }}>
                    💡 El valor del inventario excluye productos con stock negativo (cuentan como 0).
                  </div>
                </div>
              )}
              <div style={{ display: "flex", gap: 8, marginBottom: 8, flexWrap: "wrap" }}>
                <input className="input" style={{ flex: 1, minWidth: 200 }} placeholder="Buscar producto o codigo..."
                  value={busquedaInv} onChange={(e) => setBusquedaInv(e.target.value)} />
                <select className="input" style={{ width: 200 }}
                  value={filtroCategoriaInv} onChange={(e) => setFiltroCategoriaInv(e.target.value)}>
                  <option value="TODAS">Todas las categorias</option>
                  {Array.from(new Set(inventario.productos.map((p: any) => p.categoria).filter(Boolean)))
                    .sort()
                    .map((c: any) => <option key={c} value={c}>{c}</option>)}
                </select>
                <select className="input" style={{ width: 170 }}
                  value={filtroEstado} onChange={(e) => setFiltroEstado(e.target.value as any)}>
                  <option value="TODOS">Todos los estados</option>
                  <option value="OK">Stock OK</option>
                  <option value="BAJO">Stock bajo</option>
                  <option value="SIN_STOCK">Sin stock (=0)</option>
                  <option value="STOCK_NEGATIVO">Stock negativo (&lt;0)</option>
                </select>
                {(filtroCategoriaInv !== "TODAS" || filtroEstado !== "TODOS" || busquedaInv) && (
                  <button className="btn btn-outline" style={{ fontSize: 11 }}
                    onClick={() => { setFiltroCategoriaInv("TODAS"); setFiltroEstado("TODOS"); setBusquedaInv(""); }}>
                    Limpiar filtros
                  </button>
                )}
              </div>
              <div className="card">
                <table className="table" style={{ width: "100%" }}>
                  <thead>
                    <tr>
                      <th>Codigo</th><th>Producto</th><th>Categoria</th>
                      <th className="text-right">Stock</th><th className="text-right">Costo Un.</th>
                      <th className="text-right">Valor Costo</th><th className="text-right">Valor Venta</th>
                      <th className="text-right">Utilidad</th><th>Estado</th><th></th>
                    </tr>
                  </thead>
                  <tbody>
                    {inventario.productos
                      .filter((p: any) => filtroEstado === "TODOS" || p.estado_stock === filtroEstado)
                      .filter((p: any) => filtroCategoriaInv === "TODAS" || p.categoria === filtroCategoriaInv)
                      .filter((p: any) => !busquedaInv || p.nombre.toLowerCase().includes(busquedaInv.toLowerCase()) || (p.codigo || "").toLowerCase().includes(busquedaInv.toLowerCase()))
                      .map((p: any) => (
                      <tr key={p.id}>
                        <td style={{ fontSize: 11 }}>{p.codigo || "-"}</td>
                        <td style={{ fontWeight: 600 }}>{p.nombre}</td>
                        <td style={{ fontSize: 11 }}>{p.categoria || "-"}</td>
                        <td className="text-right" style={{ fontWeight: 600 }}>{p.stock_actual}</td>
                        <td className="text-right">{fmt(p.precio_costo)}</td>
                        <td className="text-right">{fmt(p.valor_costo)}</td>
                        <td className="text-right">{fmt(p.valor_venta)}</td>
                        <td className="text-right" style={{ color: "var(--color-success)" }}>{fmt(p.utilidad_potencial)}</td>
                        <td><span style={{ fontSize: 10, padding: "2px 6px", borderRadius: 3, fontWeight: 700,
                          background: p.estado_stock === "STOCK_NEGATIVO" ? "#dc2626" : p.estado_stock === "SIN_STOCK" ? "rgba(239,68,68,0.15)" : p.estado_stock === "BAJO" ? "rgba(245,158,11,0.15)" : "rgba(34,197,94,0.15)",
                          color: p.estado_stock === "STOCK_NEGATIVO" ? "#fff" : p.estado_stock === "SIN_STOCK" ? "var(--color-danger)" : p.estado_stock === "BAJO" ? "var(--color-warning)" : "var(--color-success)"
                        }}>{p.estado_stock === "STOCK_NEGATIVO" ? "⚠ Negativo" : p.estado_stock === "SIN_STOCK" ? "Sin stock" : p.estado_stock === "BAJO" ? "Bajo" : "OK"}</span></td>
                        <td><button className="btn btn-outline" style={{ fontSize: 10, padding: "2px 8px" }} onClick={() => verKardex(p.id)}>Kardex</button></td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              </div>
            </div>
          )
        )}

        {/* TAB VALUACIÓN DE INVENTARIO — v2.5.22 */}
        {tab === "valuacion" && (
          <div>
            <div style={{ marginBottom: 12, padding: 12, background: "var(--color-surface)", borderRadius: 6, border: "1px solid var(--color-border)" }}>
              <div style={{ display: "flex", alignItems: "center", gap: 12, flexWrap: "wrap" }}>
                <div style={{ fontSize: 12, fontWeight: 600 }}>Método de valuación:</div>
                <div style={{ display: "flex", gap: 6 }}>
                  <button type="button"
                    onClick={() => setValuacionMetodo("PMP")}
                    style={{
                      padding: "6px 14px", borderRadius: 4,
                      border: `1px solid ${valuacionMetodo === "PMP" ? "var(--color-primary)" : "var(--color-border)"}`,
                      background: valuacionMetodo === "PMP" ? "var(--color-primary)" : "transparent",
                      color: valuacionMetodo === "PMP" ? "#fff" : "var(--color-text)",
                      fontSize: 12, fontWeight: 600, cursor: "pointer",
                    }}>
                    📊 Promedio Ponderado (PMP)
                  </button>
                  <button type="button"
                    onClick={() => setValuacionMetodo("ULTIMO")}
                    style={{
                      padding: "6px 14px", borderRadius: 4,
                      border: `1px solid ${valuacionMetodo === "ULTIMO" ? "var(--color-primary)" : "var(--color-border)"}`,
                      background: valuacionMetodo === "ULTIMO" ? "var(--color-primary)" : "transparent",
                      color: valuacionMetodo === "ULTIMO" ? "#fff" : "var(--color-text)",
                      fontSize: 12, fontWeight: 600, cursor: "pointer",
                    }}>
                    🏷 Último precio de compra
                  </button>
                </div>
                <button className="btn btn-primary" style={{ marginLeft: "auto" }}
                  disabled={valuacionCargando}
                  onClick={async () => {
                    setValuacionCargando(true);
                    try {
                      const { reporteValuacionInventario } = await import("../services/api");
                      const data = await reporteValuacionInventario(valuacionMetodo);
                      setValuacionData(data);
                    } catch (err) { toastError("Error: " + err); }
                    setValuacionCargando(false);
                  }}>
                  {valuacionCargando ? "Calculando..." : "Generar reporte"}
                </button>
              </div>
              <div style={{ fontSize: 10, color: "var(--color-text-secondary)", marginTop: 8 }}>
                💡 <strong>PMP (Promedio Ponderado Móvil)</strong>: el costo se recalcula con cada compra, suavizando variaciones de precios. Recomendado por SRI para PyMEs. ·
                <strong>Último precio</strong>: usa el precio de la última compra registrada (modo "reposición").
              </div>
            </div>

            {valuacionData && (
              <>
                <div style={{ display: "grid", gridTemplateColumns: "repeat(4, 1fr)", gap: 12, marginBottom: 16 }}>
                  <KpiCard label="Productos" valor={String(valuacionData.totales.items)} />
                  <KpiCard label="Unidades totales" valor={valuacionData.totales.unidades.toFixed(0)} />
                  <KpiCard label="Valor inventario" valor={fmt(valuacionData.totales.valor_inventario)} color="var(--color-primary)" sub={`Método: ${valuacionData.metodo_descripcion}`} />
                  <KpiCard label="Utilidad potencial" valor={fmt(valuacionData.totales.utilidad_potencial)} color="var(--color-success)" sub={`Margen ${valuacionData.totales.margen_pct.toFixed(1)}%`} />
                </div>

                <div style={{ background: "var(--color-surface)", borderRadius: 6, border: "1px solid var(--color-border)", overflow: "auto" }}>
                  <table style={{ width: "100%", fontSize: 12, borderCollapse: "collapse" }}>
                    <thead>
                      <tr style={{ background: "var(--color-surface-alt)", textAlign: "left" }}>
                        <th style={{ padding: 8 }}>Código</th>
                        <th style={{ padding: 8 }}>Producto</th>
                        <th style={{ padding: 8 }}>Categoría</th>
                        <th style={{ padding: 8, textAlign: "right" }}>Stock</th>
                        <th style={{ padding: 8, textAlign: "right" }}>Costo unit.</th>
                        <th style={{ padding: 8, textAlign: "right" }}>Valor stock</th>
                        <th style={{ padding: 8, textAlign: "right" }}>Precio venta</th>
                        <th style={{ padding: 8, textAlign: "right" }}>Utilidad</th>
                        <th style={{ padding: 8, textAlign: "right" }}>Margen %</th>
                      </tr>
                    </thead>
                    <tbody>
                      {valuacionData.productos.length === 0 ? (
                        <tr><td colSpan={9} style={{ padding: 20, textAlign: "center", color: "var(--color-text-secondary)" }}>
                          Sin productos con stock para valuar.
                        </td></tr>
                      ) : valuacionData.productos.map((p: any) => (
                        <tr key={p.id} style={{ borderTop: "1px solid var(--color-border)" }}>
                          <td style={{ padding: 6 }}>{p.codigo || "-"}</td>
                          <td style={{ padding: 6 }}>{p.nombre}</td>
                          <td style={{ padding: 6 }}>{p.categoria || "-"}</td>
                          <td style={{ padding: 6, textAlign: "right" }}>{p.stock.toFixed(0)}</td>
                          <td style={{ padding: 6, textAlign: "right" }}>${p.costo_usado.toFixed(4)}</td>
                          <td style={{ padding: 6, textAlign: "right", fontWeight: 600 }}>${p.valor_inventario.toFixed(2)}</td>
                          <td style={{ padding: 6, textAlign: "right" }}>${p.precio_venta.toFixed(2)}</td>
                          <td style={{ padding: 6, textAlign: "right", color: "var(--color-success)" }}>${p.utilidad_potencial.toFixed(2)}</td>
                          <td style={{ padding: 6, textAlign: "right" }}>{p.margen_pct.toFixed(1)}%</td>
                        </tr>
                      ))}
                    </tbody>
                  </table>
                </div>
              </>
            )}

            {!valuacionData && !valuacionCargando && (
              <div style={{ padding: 32, textAlign: "center", color: "var(--color-text-secondary)" }}>
                Seleccioná un método y click "Generar reporte" para ver la valuación de tu inventario.
              </div>
            )}
          </div>
        )}

        {/* TAB KARDEX MULTI */}
        {tab === "kardex" && (
          <div>
            <div style={{ marginBottom: 12, padding: 12, background: "var(--color-surface)", borderRadius: 6, border: "1px solid var(--color-border)" }}>
              <div style={{ fontSize: 12, fontWeight: 600, marginBottom: 8 }}>
                Filtrar por categorías
              </div>
              <div style={{ display: "flex", flexWrap: "wrap", gap: 6, marginBottom: 10 }}>
                {/* v2.5.14: chip "Todas" siempre visible. Activo cuando lista vacía
                    (= sin filtro). Click → limpia las selecciones. */}
                <button type="button"
                  onClick={() => setKardexCatsSeleccionadas([])}
                  style={{
                    padding: "4px 10px", borderRadius: 4,
                    border: `1px solid ${kardexCatsSeleccionadas.length === 0 ? "var(--color-success)" : "var(--color-border)"}`,
                    background: kardexCatsSeleccionadas.length === 0 ? "var(--color-success)" : "transparent",
                    color: kardexCatsSeleccionadas.length === 0 ? "#fff" : "var(--color-text)",
                    fontSize: 11, fontWeight: 700, cursor: "pointer",
                  }}>
                  ✓ Todas
                </button>
                {categoriasMaestro.map(c => (
                  <button key={c.id} type="button"
                    onClick={() => {
                      setKardexCatsSeleccionadas(prev =>
                        prev.includes(c.id) ? prev.filter(x => x !== c.id) : [...prev, c.id]
                      );
                    }}
                    style={{
                      padding: "4px 10px", borderRadius: 4,
                      border: `1px solid ${kardexCatsSeleccionadas.includes(c.id) ? "var(--color-primary)" : "var(--color-border)"}`,
                      background: kardexCatsSeleccionadas.includes(c.id) ? "var(--color-primary)" : "transparent",
                      color: kardexCatsSeleccionadas.includes(c.id) ? "#fff" : "var(--color-text)",
                      fontSize: 11, fontWeight: 600, cursor: "pointer",
                    }}>{c.nombre}</button>
                ))}
              </div>
              {kardexCatsSeleccionadas.length > 0 && (
                <div style={{ fontSize: 10, color: "var(--color-text-secondary)", marginBottom: 8 }}>
                  💡 Filtrando por <strong>{kardexCatsSeleccionadas.length}</strong> categoría(s). Click "✓ Todas" para ver el inventario completo.
                </div>
              )}
              <div style={{ display: "flex", gap: 8, alignItems: "center", flexWrap: "wrap" }}>
                <span style={{ fontSize: 11, color: "var(--color-text-secondary)" }}>
                  Periodo: {desde} a {hasta}
                </span>
                {/* v2.5.27: buscador siempre visible (antes solo aparecía DESPUÉS de generar el reporte) */}
                <input className="input" style={{ flex: 1, minWidth: 220, fontSize: 12 }}
                  placeholder="🔍 Buscar en resultados (producto, motivo, usuario)..."
                  value={kardexBusqueda}
                  onChange={(e) => setKardexBusqueda(e.target.value)}
                  disabled={!kardexMultiData}
                  title={!kardexMultiData ? "Genera el reporte primero para poder buscar" : ""} />
                {kardexBusqueda && (
                  <button className="btn btn-outline" style={{ fontSize: 10, padding: "4px 10px" }}
                    onClick={() => setKardexBusqueda("")}>
                    Limpiar
                  </button>
                )}
                <button className="btn btn-primary" style={{ marginLeft: "auto" }}
                  onClick={cargarKardexMulti} disabled={kardexCargando}>
                  {kardexCargando ? "Cargando..." : "Generar Kardex"}
                </button>
              </div>
            </div>

            {kardexMultiData && (
              <>
                <div style={{ display: "grid", gridTemplateColumns: "repeat(4, 1fr)", gap: 12, marginBottom: 16 }}>
                  <KpiCard label="Movimientos" valor={String(kardexMultiData.total_movimientos)} />
                  <KpiCard label="Total Entradas" valor={kardexMultiData.total_entradas.toFixed(2)} sub={fmt(kardexMultiData.valor_entradas)} color="var(--color-success)" />
                  <KpiCard label="Total Salidas" valor={kardexMultiData.total_salidas.toFixed(2)} sub={fmt(kardexMultiData.valor_salidas)} color="var(--color-danger)" />
                  <KpiCard label="Movimiento neto" valor={(kardexMultiData.total_entradas - kardexMultiData.total_salidas).toFixed(2)} />
                </div>
                {/* contador de resultados (la búsqueda ya está arriba en la barra de filtros) */}
                <div style={{ marginBottom: 10, display: "flex", justifyContent: "flex-end" }}>
                  <span style={{ fontSize: 11, color: "var(--color-text-secondary)" }}>
                    {(() => {
                      const q = kardexBusqueda.trim().toLowerCase();
                      if (!q) return `${kardexMultiData.movimientos.length} movimientos`;
                      const filtrados = kardexMultiData.movimientos.filter((m: any) =>
                        (m.nombre || "").toLowerCase().includes(q) ||
                        (m.categoria || "").toLowerCase().includes(q) ||
                        (m.motivo || "").toLowerCase().includes(q) ||
                        (m.usuario || "").toLowerCase().includes(q) ||
                        (m.tipo || "").toLowerCase().includes(q)
                      );
                      return `${filtrados.length} de ${kardexMultiData.movimientos.length}`;
                    })()}
                  </span>
                </div>
                <div className="card">
                  <table className="table" style={{ width: "100%" }}>
                    <thead>
                      <tr>
                        <th>Fecha</th><th>Producto</th><th>Categoria</th><th>Tipo</th>
                        <th className="text-right">Cant.</th><th className="text-right">Stock Ant.</th>
                        <th className="text-right">Stock Nuevo</th><th className="text-right">Costo Un.</th>
                        <th>Motivo</th><th>Usuario</th>
                      </tr>
                    </thead>
                    <tbody>
                      {(() => {
                        const q = kardexBusqueda.trim().toLowerCase();
                        const lista = !q ? kardexMultiData.movimientos : kardexMultiData.movimientos.filter((m: any) =>
                          (m.nombre || "").toLowerCase().includes(q) ||
                          (m.categoria || "").toLowerCase().includes(q) ||
                          (m.motivo || "").toLowerCase().includes(q) ||
                          (m.usuario || "").toLowerCase().includes(q) ||
                          (m.tipo || "").toLowerCase().includes(q)
                        );
                        if (lista.length === 0) {
                          return <tr><td colSpan={10} style={{ textAlign: "center", padding: 30, color: "var(--color-text-secondary)" }}>
                            {q ? `Sin resultados para "${kardexBusqueda}"` : "Sin movimientos en este periodo y filtros"}
                          </td></tr>;
                        }
                        return lista.map((m: any) => (
                        <tr key={m.id}>
                          <td style={{ fontSize: 11 }}>{m.fecha?.slice(0, 16).replace("T", " ")}</td>
                          <td style={{ fontWeight: 600, fontSize: 11 }}>{m.nombre}</td>
                          <td style={{ fontSize: 10 }}>{m.categoria || "-"}</td>
                          <td><span style={{ fontSize: 9, padding: "2px 6px", borderRadius: 3,
                            background: m.tipo === "VENTA" ? "rgba(239,68,68,0.15)"
                              : m.tipo.includes("COMPRA") || m.tipo.includes("INGRESO") ? "rgba(34,197,94,0.15)"
                              : m.tipo === "ANULACION_VENTA" ? "rgba(34,197,94,0.15)"
                              : "rgba(148,163,184,0.15)",
                            color: m.tipo === "VENTA" ? "var(--color-danger)"
                              : m.tipo.includes("COMPRA") || m.tipo.includes("INGRESO") || m.tipo === "ANULACION_VENTA" ? "var(--color-success)"
                              : "var(--color-text-secondary)"
                          }}>{m.tipo}</span></td>
                          <td className="text-right" style={{ color: m.cantidad < 0 ? "var(--color-danger)" : "var(--color-success)", fontWeight: 600 }}>
                            {m.cantidad > 0 ? "+" : ""}{m.cantidad}
                          </td>
                          <td className="text-right">{m.stock_anterior}</td>
                          <td className="text-right" style={{ fontWeight: 600 }}>{m.stock_nuevo}</td>
                          <td className="text-right">{m.costo_unitario != null ? fmt(m.costo_unitario) : "-"}</td>
                          {/* v2.5.28: backend ahora resuelve motivo con LEFT JOIN a ventas/compras */}
                          <td style={{ fontSize: 11 }} title={m.motivo || ""}>{m.motivo || "-"}</td>
                          <td style={{ fontSize: 11 }}>{m.usuario || "-"}</td>
                        </tr>
                        ));
                      })()}
                    </tbody>
                  </table>
                </div>
              </>
            )}

            {!kardexMultiData && !kardexCargando && (
              <div className="card">
                <div className="card-body" style={{ textAlign: "center", padding: 40, color: "var(--color-text-secondary)" }}>
                  Selecciona filtros y presiona <strong>Generar Kardex</strong>
                </div>
              </div>
            )}
          </div>
        )}

        {/* TAB: Compras — reporte frontend-only (detalle / por proveedor / por fecha) */}
        {tab === "depositos" && (
          <DepositosEnTransito />
        )}

        {!cargando && tab === "compras" && (
          <div>
            {/* Filtros: proveedor + agrupación */}
            <div style={{ display: "flex", flexWrap: "wrap", alignItems: "center", gap: 10, marginBottom: 12 }}>
              <select className="input" style={{ fontSize: 12, padding: "5px 8px", minWidth: 220 }}
                value={comprasProveedorFiltro} onChange={(e) => setComprasProveedorFiltro(e.target.value)}>
                <option value="TODOS">Todos los proveedores</option>
                {comprasProveedores.map((p) => (
                  <option key={p.id} value={String(p.id)}>{p.nombre}</option>
                ))}
              </select>
              <select className="input" style={{ fontSize: 12, padding: "5px 8px", minWidth: 160 }}
                value={comprasAgrupacion} onChange={(e) => setComprasAgrupacion(e.target.value as typeof comprasAgrupacion)}>
                <option value="detalle">Detalle</option>
                <option value="proveedor">Por proveedor</option>
                <option value="fecha">Por fecha</option>
              </select>
              <span style={{ flex: 1 }} />
              <span style={{ fontSize: 12, color: "var(--color-text-secondary)" }}>
                {comprasFiltradas.length} compras · <strong style={{ color: "var(--color-text)" }}>{fmt(comprasTotalGeneral)}</strong>
              </span>
            </div>

            {comprasFiltradas.length === 0 ? (
              <div className="card">
                <div className="card-body" style={{ textAlign: "center", padding: 40, color: "var(--color-text-secondary)" }}>
                  Sin compras en el rango seleccionado.
                </div>
              </div>
            ) : comprasAgrupacion === "detalle" ? (
              <div className="card">
                <table className="table" style={{ width: "100%" }}>
                  <thead>
                    <tr>
                      <th>Fecha</th><th>N°</th><th>Factura</th><th>Proveedor</th><th>Tipo</th>
                      <th className="text-right">Subtotal</th><th className="text-right">IVA</th><th className="text-right">Total</th>
                    </tr>
                  </thead>
                  <tbody>
                    {comprasFiltradas.map((c) => (
                      <tr key={c.id}>
                        <td style={{ fontSize: 11 }}>{comprasFechaCorta(c.fecha)}</td>
                        <td>{c.numero}</td>
                        <td style={{ fontSize: 11 }}>{c.numero_factura || "—"}</td>
                        <td style={{ fontWeight: 600 }}>{c.proveedor_nombre || "—"}</td>
                        <td style={{ fontSize: 11 }}>{c.tipo_documento || "—"}</td>
                        <td className="text-right">{fmt(c.subtotal ?? 0)}</td>
                        <td className="text-right">{fmt(c.iva ?? 0)}</td>
                        <td className="text-right" style={{ fontWeight: 700 }}>{fmt(c.total ?? 0)}</td>
                      </tr>
                    ))}
                  </tbody>
                  <tfoot>
                    <tr style={{ borderTop: "2px solid var(--color-border)", fontWeight: 700 }}>
                      <td colSpan={5} className="text-right">TOTALES</td>
                      <td className="text-right">{fmt(comprasFiltradas.reduce((s, c) => s + (c.subtotal ?? 0), 0))}</td>
                      <td className="text-right">{fmt(comprasFiltradas.reduce((s, c) => s + (c.iva ?? 0), 0))}</td>
                      <td className="text-right" style={{ color: "var(--color-primary)" }}>{fmt(comprasTotalGeneral)}</td>
                    </tr>
                  </tfoot>
                </table>
              </div>
            ) : comprasAgrupacion === "proveedor" ? (
              <div style={{ display: "flex", flexDirection: "column", gap: 16 }}>
                {agruparCompras((c) => c.proveedor_nombre || "Sin proveedor").map(([nombre, grupo]) => {
                  const sub = grupo.reduce((s, c) => s + (c.subtotal ?? 0), 0);
                  const ivaG = grupo.reduce((s, c) => s + (c.iva ?? 0), 0);
                  const totG = grupo.reduce((s, c) => s + (c.total ?? 0), 0);
                  return (
                    <div className="card" key={nombre}>
                      <div className="card-header" style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
                        <span>{nombre} <span style={{ fontSize: 11, color: "var(--color-text-secondary)", fontWeight: 400 }}>({grupo.length} compras)</span></span>
                        <strong style={{ color: "var(--color-primary)" }}>{fmt(totG)}</strong>
                      </div>
                      <table className="table" style={{ width: "100%" }}>
                        <thead>
                          <tr>
                            <th>Fecha</th><th>N°</th><th>Factura</th><th>Tipo</th>
                            <th className="text-right">Subtotal</th><th className="text-right">IVA</th><th className="text-right">Total</th>
                          </tr>
                        </thead>
                        <tbody>
                          {grupo.map((c) => (
                            <tr key={c.id}>
                              <td style={{ fontSize: 11 }}>{comprasFechaCorta(c.fecha)}</td>
                              <td>{c.numero}</td>
                              <td style={{ fontSize: 11 }}>{c.numero_factura || "—"}</td>
                              <td style={{ fontSize: 11 }}>{c.tipo_documento || "—"}</td>
                              <td className="text-right">{fmt(c.subtotal ?? 0)}</td>
                              <td className="text-right">{fmt(c.iva ?? 0)}</td>
                              <td className="text-right" style={{ fontWeight: 700 }}>{fmt(c.total ?? 0)}</td>
                            </tr>
                          ))}
                        </tbody>
                        <tfoot>
                          <tr style={{ borderTop: "1px solid var(--color-border)", fontWeight: 700 }}>
                            <td colSpan={4} className="text-right">Subtotal proveedor</td>
                            <td className="text-right">{fmt(sub)}</td>
                            <td className="text-right">{fmt(ivaG)}</td>
                            <td className="text-right" style={{ color: "var(--color-primary)" }}>{fmt(totG)}</td>
                          </tr>
                        </tfoot>
                      </table>
                    </div>
                  );
                })}
                <div className="card">
                  <div className="card-body" style={{ display: "flex", justifyContent: "space-between", alignItems: "center", fontWeight: 700 }}>
                    <span>TOTAL GENERAL</span>
                    <span style={{ color: "var(--color-primary)" }}>{fmt(comprasTotalGeneral)}</span>
                  </div>
                </div>
              </div>
            ) : (
              <div style={{ display: "flex", flexDirection: "column", gap: 16 }}>
                {agruparCompras((c) => (c.fecha || "").slice(0, 10), true).map(([dia, grupo]) => {
                  const sub = grupo.reduce((s, c) => s + (c.subtotal ?? 0), 0);
                  const ivaG = grupo.reduce((s, c) => s + (c.iva ?? 0), 0);
                  const totG = grupo.reduce((s, c) => s + (c.total ?? 0), 0);
                  return (
                    <div className="card" key={dia || "sin-fecha"}>
                      <div className="card-header" style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
                        <span>{comprasFechaCorta(dia) || "Sin fecha"} <span style={{ fontSize: 11, color: "var(--color-text-secondary)", fontWeight: 400 }}>({grupo.length} compras)</span></span>
                        <strong style={{ color: "var(--color-primary)" }}>{fmt(totG)}</strong>
                      </div>
                      <table className="table" style={{ width: "100%" }}>
                        <thead>
                          <tr>
                            <th>N°</th><th>Factura</th><th>Proveedor</th><th>Tipo</th>
                            <th className="text-right">Subtotal</th><th className="text-right">IVA</th><th className="text-right">Total</th>
                          </tr>
                        </thead>
                        <tbody>
                          {grupo.map((c) => (
                            <tr key={c.id}>
                              <td>{c.numero}</td>
                              <td style={{ fontSize: 11 }}>{c.numero_factura || "—"}</td>
                              <td style={{ fontWeight: 600 }}>{c.proveedor_nombre || "—"}</td>
                              <td style={{ fontSize: 11 }}>{c.tipo_documento || "—"}</td>
                              <td className="text-right">{fmt(c.subtotal ?? 0)}</td>
                              <td className="text-right">{fmt(c.iva ?? 0)}</td>
                              <td className="text-right" style={{ fontWeight: 700 }}>{fmt(c.total ?? 0)}</td>
                            </tr>
                          ))}
                        </tbody>
                        <tfoot>
                          <tr style={{ borderTop: "1px solid var(--color-border)", fontWeight: 700 }}>
                            <td colSpan={4} className="text-right">Subtotal día</td>
                            <td className="text-right">{fmt(sub)}</td>
                            <td className="text-right">{fmt(ivaG)}</td>
                            <td className="text-right" style={{ color: "var(--color-primary)" }}>{fmt(totG)}</td>
                          </tr>
                        </tfoot>
                      </table>
                    </div>
                  );
                })}
                <div className="card">
                  <div className="card-body" style={{ display: "flex", justifyContent: "space-between", alignItems: "center", fontWeight: 700 }}>
                    <span>TOTAL GENERAL</span>
                    <span style={{ color: "var(--color-primary)" }}>{fmt(comprasTotalGeneral)}</span>
                  </div>
                </div>
              </div>
            )}
          </div>
        )}

        {/* TAB: Cajeros — ranking + tickets promedio */}
        {/* v2.5.54: Reporte de Gastos */}
        {!cargando && tab === "gastos" && gastosReporte && (
          <div>
            {/* KPIs */}
            <div style={{ display: "grid", gridTemplateColumns: "repeat(auto-fit, minmax(180px, 1fr))", gap: 12, marginBottom: 16 }}>
              <KpiCard label="Total gastos" valor={`$${gastosReporte.total.toFixed(2)}`} color="var(--color-danger)" />
              <KpiCard label="Cantidad" valor={String(gastosReporte.count)} />
              <KpiCard label="Promedio por gasto" valor={`$${gastosReporte.promedio.toFixed(2)}`} />
              <KpiCard label="Días con gastos" valor={String(gastosReporte.por_dia.length)} />
              {gastosReporte.por_dia.length > 1 && (
                <KpiCard label="Promedio diario"
                  valor={`$${(gastosReporte.total / gastosReporte.por_dia.length).toFixed(2)}`} />
              )}
            </div>

            {gastosReporte.count === 0 ? (
              <div className="card">
                <div className="card-body" style={{ textAlign: "center", padding: 40, color: "var(--color-text-secondary)" }}>
                  No hay gastos en este período.
                </div>
              </div>
            ) : (
              <>
                {/* Gráfica: gastos por día */}
                {gastosReporte.por_dia.length > 1 && (
                  <div className="card mb-4">
                    <div className="card-header">📈 Gastos por día</div>
                    <div className="card-body">
                      <ResponsiveContainer width="100%" height={260}>
                        <BarChart data={gastosReporte.por_dia.map(d => ({
                          dia: d.dia.slice(5), // MM-DD
                          total: d.total,
                          count: d.count,
                        }))}>
                          <CartesianGrid strokeDasharray="3 3" stroke="rgba(255,255,255,0.1)" />
                          <XAxis dataKey="dia" tick={{ fill: "var(--color-text-secondary)", fontSize: 11 }} />
                          <YAxis tick={{ fill: "var(--color-text-secondary)", fontSize: 11 }} />
                          <Tooltip
                            formatter={(value) => [`$${Number(value ?? 0).toFixed(2)}`, "Gastos"]}
                            contentStyle={{ background: "var(--color-surface)", border: "1px solid var(--color-border)" }} />
                          <Bar dataKey="total" fill="#ef4444" radius={[4, 4, 0, 0]} />
                        </BarChart>
                      </ResponsiveContainer>
                    </div>
                  </div>
                )}

                {/* Por categoría: tabla + pie */}
                {gastosReporte.por_categoria.length > 0 && (
                  <div className="card mb-4">
                    <div className="card-header">🏷 Por categoría</div>
                    <div className="card-body" style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: 16 }}>
                      <div>
                        <table className="table">
                          <thead>
                            <tr>
                              <th>Categoría</th>
                              <th className="text-right">Gastos</th>
                              <th className="text-right">Total</th>
                              <th className="text-right">%</th>
                            </tr>
                          </thead>
                          <tbody>
                            {gastosReporte.por_categoria.map((c) => (
                              <tr key={c.categoria}>
                                <td><strong>{c.categoria}</strong></td>
                                <td className="text-right">{c.count}</td>
                                <td className="text-right font-bold" style={{ color: "var(--color-danger)" }}>
                                  ${c.total.toFixed(2)}
                                </td>
                                <td className="text-right text-secondary">
                                  {gastosReporte.total > 0 ? ((c.total / gastosReporte.total) * 100).toFixed(1) : 0}%
                                </td>
                              </tr>
                            ))}
                          </tbody>
                        </table>
                      </div>
                      <div>
                        <ResponsiveContainer width="100%" height={260}>
                          <PieChart>
                            <Pie
                              data={gastosReporte.por_categoria}
                              dataKey="total"
                              nameKey="categoria"
                              cx="50%"
                              cy="50%"
                              outerRadius={90}
                              label={(e: any) => `${e.categoria}: $${e.total.toFixed(0)}`}>
                              {gastosReporte.por_categoria.map((_, idx) => {
                                const colors = ["#ef4444", "#f59e0b", "#10b981", "#3b82f6", "#8b5cf6", "#ec4899", "#06b6d4", "#84cc16"];
                                return <Cell key={idx} fill={colors[idx % colors.length]} />;
                              })}
                            </Pie>
                            <Tooltip formatter={(v) => `$${Number(v ?? 0).toFixed(2)}`} />
                          </PieChart>
                        </ResponsiveContainer>
                      </div>
                    </div>
                  </div>
                )}

                {/* Por usuario */}
                {gastosReporte.por_usuario.length > 0 && (
                  <div className="card mb-4">
                    <div className="card-header">👤 Por usuario</div>
                    <div className="card-body">
                      <table className="table">
                        <thead>
                          <tr>
                            <th>Usuario</th>
                            <th className="text-right">Gastos registrados</th>
                            <th className="text-right">Total</th>
                          </tr>
                        </thead>
                        <tbody>
                          {gastosReporte.por_usuario.map((u) => (
                            <tr key={u.usuario}>
                              <td><strong>{u.usuario}</strong></td>
                              <td className="text-right">{u.count}</td>
                              <td className="text-right font-bold" style={{ color: "var(--color-danger)" }}>
                                ${u.total.toFixed(2)}
                              </td>
                            </tr>
                          ))}
                        </tbody>
                      </table>
                    </div>
                  </div>
                )}
              </>
            )}
          </div>
        )}

        {tab === "cajeros" && cajerosData && (
          <div>
            {/* KPIs globales */}
            <div style={{ display: "grid", gridTemplateColumns: "repeat(5, 1fr)", gap: 12, marginBottom: 16 }}>
              <KpiCard label="Cajeros activos" valor={String(cajerosData.total_cajeros)} />
              <KpiCard label="Total ventas" valor={fmt(cajerosData.total_global)} color="var(--color-primary)" />
              <KpiCard label="N° transacciones" valor={String(cajerosData.num_ventas_global)} />
              <KpiCard label="Ticket promedio" valor={fmt(cajerosData.ticket_promedio_global)} color="var(--color-success)" />
              <KpiCard label="Descuadre neto" valor={fmt(cajerosData.descuadre_neto_global)}
                color={cajerosData.descuadre_neto_global < 0 ? "var(--color-danger)" : "var(--color-success)"} />
            </div>

            {cajerosData.cajeros.length === 0 ? (
              <div className="card" style={{ padding: 30, textAlign: "center", color: "var(--color-text-secondary)" }}>
                Sin ventas registradas en este período
              </div>
            ) : (
              <>
                {/* Ranking visual top 5 */}
                {cajerosData.cajeros.length > 1 && (
                  <div className="card" style={{ marginBottom: 12, padding: 14 }}>
                    <div style={{ fontSize: 13, fontWeight: 700, marginBottom: 10 }}>🏆 Ranking de cajeros (top 5)</div>
                    <ResponsiveContainer width="100%" height={220}>
                      <BarChart data={cajerosData.cajeros.slice(0, 5).map((c: any) => ({ nombre: c.cajero, total: c.total }))} margin={{ top: 5, right: 20, left: 0, bottom: 5 }}>
                        <CartesianGrid strokeDasharray="3 3" stroke="rgba(148,163,184,0.2)" />
                        <XAxis dataKey="nombre" stroke="var(--color-text-secondary)" tick={{ fontSize: 11 }} />
                        <YAxis stroke="var(--color-text-secondary)" tick={{ fontSize: 11 }} tickFormatter={(v) => `$${v}`} />
                        <Tooltip
                          contentStyle={{ background: "var(--color-surface)", border: "1px solid var(--color-border)" }}
                          formatter={(v: any) => [fmt(Number(v)), "Total ventas"]} />
                        <Bar dataKey="total" fill="var(--color-primary)" radius={[6, 6, 0, 0]} />
                      </BarChart>
                    </ResponsiveContainer>
                  </div>
                )}

                {/* Tabla detallada */}
                <div className="card">
                  <table className="table" style={{ width: "100%" }}>
                    <thead>
                      <tr>
                        <th style={{ width: 40 }}>#</th>
                        <th>Cajero</th>
                        <th className="text-right">Ventas</th>
                        <th className="text-right">Total</th>
                        <th className="text-right">Promedio/ticket</th>
                        <th className="text-right">Unidades</th>
                        <th>Top producto</th>
                        <th className="text-right">Efectivo</th>
                        <th className="text-right">Transfer.</th>
                        <th className="text-right">Crédito</th>
                        <th className="text-right">Cierres</th>
                        <th className="text-right">Descuadre</th>
                      </tr>
                    </thead>
                    <tbody>
                      {cajerosData.cajeros.map((c: any, idx: number) => (
                        <tr key={c.cajero}>
                          <td>
                            {idx === 0 ? <span title="1° lugar" style={{ fontSize: 14 }}>🥇</span> :
                             idx === 1 ? <span title="2° lugar" style={{ fontSize: 14 }}>🥈</span> :
                             idx === 2 ? <span title="3° lugar" style={{ fontSize: 14 }}>🥉</span> :
                             <span style={{ color: "var(--color-text-secondary)", fontSize: 11 }}>{idx + 1}</span>}
                          </td>
                          <td>
                            <strong>{c.cajero}</strong>
                            {c.num_facturas > 0 && (
                              <div style={{ fontSize: 10, color: "var(--color-text-secondary)", marginTop: 2 }}>
                                Facturas: {c.num_facturas_autorizadas}/{c.num_facturas} autorizadas
                              </div>
                            )}
                            {c.primera_venta && c.ultima_venta && (
                              <div style={{ fontSize: 10, color: "var(--color-text-secondary)" }}>
                                {c.primera_venta?.slice(0, 16).replace("T", " ")} → {c.ultima_venta?.slice(0, 16).replace("T", " ")}
                              </div>
                            )}
                          </td>
                          <td className="text-right">{c.num_ventas}</td>
                          <td className="text-right font-bold" style={{ color: "var(--color-primary)" }}>{fmt(c.total)}</td>
                          <td className="text-right" style={{ color: "var(--color-success)", fontWeight: 600 }}>{fmt(c.ticket_promedio)}</td>
                          <td className="text-right">{c.unidades_vendidas?.toFixed(0) || 0}</td>
                          <td style={{ fontSize: 11 }}>
                            {c.top_producto ? (
                              <>
                                <div>{c.top_producto}</div>
                                <div style={{ color: "var(--color-text-secondary)", fontSize: 10 }}>
                                  {c.top_producto_unidades?.toFixed(0)} unid.
                                </div>
                              </>
                            ) : "-"}
                          </td>
                          <td className="text-right" style={{ fontSize: 12 }}>{fmt(c.total_efectivo)}</td>
                          <td className="text-right" style={{ fontSize: 12 }}>{fmt(c.total_transfer)}</td>
                          <td className="text-right" style={{ fontSize: 12, color: c.total_credito > 0 ? "var(--color-warning)" : undefined }}>
                            {fmt(c.total_credito)}
                          </td>
                          <td className="text-right">{c.cajas_cerradas}</td>
                          <td className="text-right" style={{
                            color: c.descuadre_neto < 0 ? "var(--color-danger)" : c.descuadre_neto > 0 ? "var(--color-warning)" : "var(--color-text-secondary)",
                            fontWeight: Math.abs(c.descuadre_neto) > 0.01 ? 700 : undefined,
                          }}>
                            {c.descuadre_neto != null ? fmt(c.descuadre_neto) : "-"}
                          </td>
                        </tr>
                      ))}
                    </tbody>
                  </table>
                </div>

                {/* Tip */}
                <div style={{ marginTop: 10, padding: 10, fontSize: 11, color: "var(--color-text-secondary)", background: "var(--color-surface-alt)", borderRadius: 6 }}>
                  💡 Click en "Cajeros" para refrescar. Las columnas Cierres/Descuadre solo cuentan cierres en el período (cajero que cerró). Para ver el detalle de un cierre con audit log, ve a Caja → Historial caja.
                </div>
              </>
            )}
          </div>
        )}

        {/* v2.3.70 — Ventas detalladas filtrable */}
        {!cargando && tab === "ventas" && (
          <ReporteVentasFiltrable
            opciones={ventasOpciones}
            datos={ventasReporte}
            filtros={ventasFiltros}
            onFiltrosChange={setVentasFiltros}
            onAplicar={cargar}
            onExportar={async (formato) => {
              if (!ventasReporte) return;
              const ext = formato === "xlsx" ? "xlsx" : "pdf";
              const ruta = await save({
                defaultPath: `ventas_${desde}_${hasta}.${ext}`,
                filters: [{ name: formato === "xlsx" ? "Excel" : "PDF", extensions: [ext] }],
              });
              if (!ruta) return;
              const encabezados = [
                "Fecha", "Número", "Cliente", "Identif.", "Cajero",
                "Forma pago", "Tipo doc.", "Subtotal", "IVA", "Descuento", "Total", "Estado",
              ];
              const filas: string[][] = ventasReporte.ventas.map((v: VentaReporteRow) => [
                v.fecha.slice(0, 16),
                v.numero,
                v.cliente_nombre || "",
                v.cliente_identificacion || "",
                v.cajero,
                v.forma_pago,
                v.tipo_documento,
                v.subtotal_sin_iva.toFixed(2),
                v.iva.toFixed(2),
                v.descuento.toFixed(2),
                v.total.toFixed(2),
                v.anulada ? "ANULADA" : v.estado,
              ]);
              // Fila de totales
              filas.push([
                "", "", "", "", "", "", "TOTAL",
                "",
                ventasReporte.iva_global.toFixed(2),
                ventasReporte.descuento_global.toFixed(2),
                ventasReporte.total_global.toFixed(2),
                "",
              ]);
              const subtitulo = construirSubtituloVentas(desde, hasta, ventasFiltros, ventasOpciones);
              try {
                if (formato === "xlsx") {
                  await exportarTablaXlsx(ruta, "Reporte de Ventas", subtitulo, encabezados, filas, [7, 8, 9, 10]);
                } else {
                  await exportarTablaPdf(ruta, "Reporte de Ventas", subtitulo, encabezados, filas, true);
                }
                toastExito(`Reporte exportado a ${ruta}`);
              } catch (err) {
                toastError("Error exportando: " + err);
              }
            }}
          />
        )}

        {/* v2.4.14: Cancelaciones de Servicio Tecnico */}
        {!cargando && tab === "cancelaciones_st" && cancelacionesST && (() => {
          // v2.4.15: filtro inteligente — busca en orden, cliente, equipo, motivo, usuario
          const f = filtroCancelaciones.toLowerCase().trim();
          const ordenesFiltradas = !f ? cancelacionesST.ordenes : cancelacionesST.ordenes.filter(o =>
            o.numero.toLowerCase().includes(f)
            || (o.cliente_nombre || "").toLowerCase().includes(f)
            || (o.cliente_telefono || "").toLowerCase().includes(f)
            || o.equipo_descripcion.toLowerCase().includes(f)
            || (o.equipo_marca || "").toLowerCase().includes(f)
            || (o.equipo_modelo || "").toLowerCase().includes(f)
            || (o.usuario_cancelacion || "").toLowerCase().includes(f)
            || (o.observacion || "").toLowerCase().includes(f)
          );
          const sumaFiltrada = ordenesFiltradas.reduce((s, o) => s + o.monto_devuelto, 0);
          const abonosFiltrados = ordenesFiltradas.reduce((s, o) => s + o.abonos_devueltos, 0);
          return (
          <div style={{ display: "flex", flexDirection: "column", gap: 16 }}>
            {/* KPIs */}
            <div style={{ display: "grid", gridTemplateColumns: "repeat(auto-fit, minmax(180px, 1fr))", gap: 12 }}>
              <KpiCard label="Órdenes canceladas" valor={String(cancelacionesST.total_canceladas)}
                color="var(--color-danger)" />
              <KpiCard label="Abonos devueltos" valor={String(cancelacionesST.total_abonos_devueltos)}
                sub={`$${cancelacionesST.monto_total_devuelto.toFixed(2)} en total`}
                color="var(--color-warning)" />
              <KpiCard label="Período" valor={`${desde}`} sub={`hasta ${hasta}`} />
            </div>

            {/* v2.4.15: buscador */}
            <div style={{ position: "relative" }}>
              <input className="input"
                placeholder="🔎 Buscar por orden, cliente, equipo, motivo, usuario..."
                value={filtroCancelaciones}
                onChange={(e) => setFiltroCancelaciones(e.target.value)}
                style={{ width: "100%", paddingRight: filtroCancelaciones ? 30 : 8 }} />
              {filtroCancelaciones && (
                <button onClick={() => setFiltroCancelaciones("")}
                  style={{ position: "absolute", right: 6, top: "50%", transform: "translateY(-50%)", background: "transparent", border: "none", cursor: "pointer", fontSize: 16, color: "var(--color-text-secondary)" }}>×</button>
              )}
              {filtroCancelaciones && (
                <div style={{ fontSize: 11, color: "var(--color-text-secondary)", marginTop: 4 }}>
                  {ordenesFiltradas.length} de {cancelacionesST.ordenes.length} órdenes · {abonosFiltrados} abono(s) · ${sumaFiltrada.toFixed(2)}
                </div>
              )}
            </div>

            {/* Tabla */}
            <div style={{ overflowX: "auto", border: "1px solid var(--color-border)", borderRadius: 8 }}>
              <table style={{ width: "100%", borderCollapse: "collapse", fontSize: 12 }}>
                <thead>
                  <tr style={{ background: "var(--color-surface-alt)" }}>
                    <th style={thStyle}>Fecha cancelación</th>
                    <th style={thStyle}>Orden</th>
                    <th style={thStyle}>Cliente</th>
                    <th style={thStyle}>Equipo</th>
                    <th style={thStyle}>Canceló</th>
                    <th style={thStyle}>Motivo</th>
                    <th style={{ ...thStyle, textAlign: "center" }}>Abonos dev.</th>
                    <th style={{ ...thStyle, textAlign: "right" }}>Monto dev.</th>
                  </tr>
                </thead>
                <tbody>
                  {ordenesFiltradas.length === 0 ? (
                    <tr><td colSpan={8} style={{ padding: 24, textAlign: "center", color: "var(--color-text-secondary)" }}>
                      {filtroCancelaciones ? "Sin coincidencias" : "No hay órdenes canceladas en este período"}
                    </td></tr>
                  ) : (
                    ordenesFiltradas.map(o => (
                      <tr key={o.orden_id} style={{ borderTop: "1px solid var(--color-border)" }}>
                        <td style={tdStyle}>{(o.fecha_cancelacion || o.fecha_ingreso).slice(0, 16).replace("T", " ")}</td>
                        <td style={tdStyle}><strong>{o.numero}</strong></td>
                        <td style={tdStyle}>
                          {o.cliente_nombre || "—"}
                          {o.cliente_telefono && <div style={{ fontSize: 10, color: "var(--color-text-secondary)" }}>📞 {o.cliente_telefono}</div>}
                        </td>
                        <td style={tdStyle}>
                          {o.equipo_descripcion}
                          {(o.equipo_marca || o.equipo_modelo) && (
                            <div style={{ fontSize: 10, color: "var(--color-text-secondary)" }}>
                              {[o.equipo_marca, o.equipo_modelo].filter(Boolean).join(" · ")}
                            </div>
                          )}
                        </td>
                        <td style={tdStyle}>{o.usuario_cancelacion || "—"}</td>
                        <td style={tdStyle}>
                          {o.observacion || <span style={{ color: "var(--color-text-secondary)", fontStyle: "italic" }}>sin motivo</span>}
                        </td>
                        <td style={{ ...tdStyle, textAlign: "center" }}>
                          {o.abonos_devueltos > 0
                            ? <span style={{ color: "var(--color-warning)", fontWeight: 600 }}>{o.abonos_devueltos}</span>
                            : "—"}
                        </td>
                        <td style={{ ...tdStyle, textAlign: "right", fontWeight: 700, color: o.monto_devuelto > 0 ? "var(--color-warning)" : "var(--color-text-secondary)" }}>
                          ${o.monto_devuelto.toFixed(2)}
                        </td>
                      </tr>
                    ))
                  )}
                </tbody>
                {ordenesFiltradas.length > 0 && (
                  <tfoot>
                    <tr style={{ background: "var(--color-surface-alt)", fontWeight: 700 }}>
                      <td style={tdStyle} colSpan={6}>
                        TOTAL ({ordenesFiltradas.length}{filtroCancelaciones ? ` de ${cancelacionesST.total_canceladas}` : ""} canceladas)
                      </td>
                      <td style={{ ...tdStyle, textAlign: "center" }}>{abonosFiltrados}</td>
                      <td style={{ ...tdStyle, textAlign: "right", color: "var(--color-warning)" }}>${sumaFiltrada.toFixed(2)}</td>
                    </tr>
                  </tfoot>
                )}
              </table>
            </div>

            <div style={{ fontSize: 11, color: "var(--color-text-secondary)", padding: "8px 12px", background: "var(--color-surface-alt)", borderRadius: 6 }}>
              💡 Las órdenes canceladas conservan su historial. Los abonos en holding al momento de cancelar
              se devuelven automáticamente al cliente. Si se canceló por error, contacta al admin.
            </div>
          </div>
          );
        })()}

        {/* v2.4.14: Garantías activas de Servicio Tecnico */}
        {!cargando && tab === "garantias_st" && garantiasST && (() => {
          const f = filtroGarantias.toLowerCase().trim();
          const ordenesFiltradas = !f ? garantiasST.ordenes : garantiasST.ordenes.filter(o =>
            o.numero.toLowerCase().includes(f)
            || (o.cliente_nombre || "").toLowerCase().includes(f)
            || (o.cliente_telefono || "").toLowerCase().includes(f)
            || o.equipo_descripcion.toLowerCase().includes(f)
            || (o.equipo_marca || "").toLowerCase().includes(f)
            || (o.equipo_modelo || "").toLowerCase().includes(f)
            || (o.equipo_serie || "").toLowerCase().includes(f)
          );
          return (
          <div style={{ display: "flex", flexDirection: "column", gap: 16 }}>
            <div style={{ display: "grid", gridTemplateColumns: "repeat(auto-fit, minmax(180px, 1fr))", gap: 12 }}>
              <KpiCard label="Garantías activas" valor={String(garantiasST.total_activas)}
                color="var(--color-primary)" />
              <KpiCard label="Por vencer (≤30 días)" valor={String(garantiasST.total_por_vencer_30d)}
                color={garantiasST.total_por_vencer_30d > 0 ? "var(--color-warning)" : "var(--color-text-secondary)"} />
            </div>

            {/* v2.4.15: buscador */}
            <div style={{ position: "relative" }}>
              <input className="input"
                placeholder="🔎 Buscar por orden, cliente, teléfono, equipo, marca, modelo, serie..."
                value={filtroGarantias}
                onChange={(e) => setFiltroGarantias(e.target.value)}
                style={{ width: "100%", paddingRight: filtroGarantias ? 30 : 8 }} />
              {filtroGarantias && (
                <button onClick={() => setFiltroGarantias("")}
                  style={{ position: "absolute", right: 6, top: "50%", transform: "translateY(-50%)", background: "transparent", border: "none", cursor: "pointer", fontSize: 16, color: "var(--color-text-secondary)" }}>×</button>
              )}
              {filtroGarantias && (
                <div style={{ fontSize: 11, color: "var(--color-text-secondary)", marginTop: 4 }}>
                  {ordenesFiltradas.length} de {garantiasST.ordenes.length} garantías
                </div>
              )}
            </div>

            <div style={{ overflowX: "auto", border: "1px solid var(--color-border)", borderRadius: 8 }}>
              <table style={{ width: "100%", borderCollapse: "collapse", fontSize: 12 }}>
                <thead>
                  <tr style={{ background: "var(--color-surface-alt)" }}>
                    <th style={thStyle}>Orden</th>
                    <th style={thStyle}>Cliente</th>
                    <th style={thStyle}>Equipo</th>
                    <th style={thStyle}>Entrega</th>
                    <th style={{ ...thStyle, textAlign: "center" }}>Días gar.</th>
                    <th style={thStyle}>Vence</th>
                    <th style={{ ...thStyle, textAlign: "center" }}>Restantes</th>
                    <th style={{ ...thStyle, textAlign: "right" }}>Monto</th>
                  </tr>
                </thead>
                <tbody>
                  {ordenesFiltradas.length === 0 ? (
                    <tr><td colSpan={8} style={{ padding: 24, textAlign: "center", color: "var(--color-text-secondary)" }}>
                      {filtroGarantias ? "Sin coincidencias" : "No hay garantías activas en este momento"}
                    </td></tr>
                  ) : (
                    ordenesFiltradas.map(o => {
                      const colorRest = o.dias_restantes <= 7 ? "var(--color-danger)"
                        : o.dias_restantes <= 30 ? "var(--color-warning)"
                        : "var(--color-success)";
                      return (
                        <tr key={o.orden_id} style={{ borderTop: "1px solid var(--color-border)" }}>
                          <td style={tdStyle}><strong>{o.numero}</strong></td>
                          <td style={tdStyle}>
                            {o.cliente_nombre || "—"}
                            {o.cliente_telefono && <div style={{ fontSize: 10, color: "var(--color-text-secondary)" }}>📞 {o.cliente_telefono}</div>}
                          </td>
                          <td style={tdStyle}>
                            {o.equipo_descripcion}
                            {(o.equipo_marca || o.equipo_modelo || o.equipo_serie) && (
                              <div style={{ fontSize: 10, color: "var(--color-text-secondary)" }}>
                                {[o.equipo_marca, o.equipo_modelo, o.equipo_serie && `S/N: ${o.equipo_serie}`].filter(Boolean).join(" · ")}
                              </div>
                            )}
                          </td>
                          <td style={tdStyle}>{(o.fecha_entrega || "—").slice(0, 10)}</td>
                          <td style={{ ...tdStyle, textAlign: "center" }}>{o.garantia_dias}</td>
                          <td style={tdStyle}>{o.fecha_vence}</td>
                          <td style={{ ...tdStyle, textAlign: "center", fontWeight: 700, color: colorRest }}>
                            {o.dias_restantes} día{o.dias_restantes === 1 ? "" : "s"}
                          </td>
                          <td style={{ ...tdStyle, textAlign: "right" }}>${o.monto_final.toFixed(2)}</td>
                        </tr>
                      );
                    })
                  )}
                </tbody>
              </table>
            </div>

            <div style={{ fontSize: 11, color: "var(--color-text-secondary)", padding: "8px 12px", background: "var(--color-surface-alt)", borderRadius: 6 }}>
              💡 Lista órdenes entregadas con garantía aún vigente. Si el cliente vuelve por garantía,
              búscalo aquí para verificar fecha de entrega y días restantes.
            </div>
          </div>
          );
        })()}
      </div>

      {modalStockNeg && (
        <ModalCorregirStockNegativo
          onClose={() => setModalStockNeg(false)}
          onAplicado={() => { setModalStockNeg(false); cargar(); }}
        />
      )}
    </>
  );
}

// ─── v2.3.70 — Helper subtítulo del export ──────────────────────────────
function construirSubtituloVentas(
  desde: string,
  hasta: string,
  filtros: { cajero: string; formaPago: string; tipoDocumento: string; categoriaId: number | null; incluirAnuladas: boolean },
  opciones: { categorias: { id: number; nombre: string }[] } | null,
): string {
  const partes: string[] = [`Período: ${desde} al ${hasta}`];
  if (filtros.cajero) partes.push(`Cajero: ${filtros.cajero}`);
  if (filtros.formaPago) partes.push(`Forma pago: ${filtros.formaPago}`);
  if (filtros.tipoDocumento) partes.push(`Documento: ${filtros.tipoDocumento}`);
  if (filtros.categoriaId && opciones) {
    const cat = opciones.categorias.find(c => c.id === filtros.categoriaId);
    if (cat) partes.push(`Categoría: ${cat.nombre}`);
  }
  if (filtros.incluirAnuladas) partes.push("Incluye anuladas");
  return partes.join(" · ");
}

// ─── v2.3.70 — Componente Reporte Ventas Filtrable ──────────────────────
interface ReporteVentasProps {
  opciones: { cajeros: string[]; formas_pago: string[]; tipos_documento: string[]; categorias: { id: number; nombre: string }[] } | null;
  datos: ReporteVentasResultado | null;
  filtros: { cajero: string; formaPago: string; tipoDocumento: string; categoriaId: number | null; incluirAnuladas: boolean };
  onFiltrosChange: React.Dispatch<React.SetStateAction<{ cajero: string; formaPago: string; tipoDocumento: string; categoriaId: number | null; incluirAnuladas: boolean }>>;
  onAplicar: () => void;
  onExportar: (formato: "xlsx" | "pdf") => void;
}

function ReporteVentasFiltrable({ opciones, datos, filtros, onFiltrosChange, onAplicar, onExportar }: ReporteVentasProps) {
  if (!datos) {
    return (
      <div style={{ textAlign: "center", padding: 40, color: "var(--color-text-secondary)" }}>
        Click en "Aplicar" para cargar el reporte de ventas.
      </div>
    );
  }
  return (
    <div style={{ display: "flex", flexDirection: "column", gap: 14 }}>
      {/* Filtros */}
      <div style={{
        background: "var(--color-surface)",
        border: "1px solid var(--color-border)",
        borderRadius: 8,
        padding: 12,
        display: "grid",
        gridTemplateColumns: "repeat(auto-fit, minmax(170px, 1fr))",
        gap: 10,
      }}>
        <div>
          <label style={{ fontSize: 11, fontWeight: 600, display: "block", marginBottom: 4 }}>Cajero</label>
          <select
            className="input"
            value={filtros.cajero}
            onChange={e => onFiltrosChange(f => ({ ...f, cajero: e.target.value }))}
            style={{ width: "100%", fontSize: 12 }}
          >
            <option value="">— Todos —</option>
            {opciones?.cajeros.map(c => <option key={c} value={c}>{c}</option>)}
          </select>
        </div>
        <div>
          <label style={{ fontSize: 11, fontWeight: 600, display: "block", marginBottom: 4 }}>Forma de pago</label>
          <select
            className="input"
            value={filtros.formaPago}
            onChange={e => onFiltrosChange(f => ({ ...f, formaPago: e.target.value }))}
            style={{ width: "100%", fontSize: 12 }}
          >
            <option value="">— Todas —</option>
            {opciones?.formas_pago.map(f => <option key={f} value={f}>{f}</option>)}
          </select>
        </div>
        <div>
          <label style={{ fontSize: 11, fontWeight: 600, display: "block", marginBottom: 4 }}>Tipo documento</label>
          <select
            className="input"
            value={filtros.tipoDocumento}
            onChange={e => onFiltrosChange(f => ({ ...f, tipoDocumento: e.target.value }))}
            style={{ width: "100%", fontSize: 12 }}
          >
            <option value="">— Todos —</option>
            {opciones?.tipos_documento.map(t => <option key={t} value={t}>{t}</option>)}
          </select>
        </div>
        <div>
          <label style={{ fontSize: 11, fontWeight: 600, display: "block", marginBottom: 4 }}>Categoría</label>
          <select
            className="input"
            value={filtros.categoriaId ?? ""}
            onChange={e => onFiltrosChange(f => ({ ...f, categoriaId: e.target.value ? parseInt(e.target.value, 10) : null }))}
            style={{ width: "100%", fontSize: 12 }}
          >
            <option value="">— Todas —</option>
            {opciones?.categorias.map(c => <option key={c.id} value={c.id}>{c.nombre}</option>)}
          </select>
        </div>
        <div style={{ display: "flex", flexDirection: "column", justifyContent: "flex-end" }}>
          <label style={{ fontSize: 12, display: "flex", gap: 6, alignItems: "center", cursor: "pointer" }}>
            <input
              type="checkbox"
              checked={filtros.incluirAnuladas}
              onChange={e => onFiltrosChange(f => ({ ...f, incluirAnuladas: e.target.checked }))}
            />
            Incluir anuladas
          </label>
        </div>
        <div style={{ display: "flex", alignItems: "flex-end", gap: 6 }}>
          <button className="btn btn-primary" onClick={onAplicar} style={{ flex: 1, fontSize: 12 }}>
            🔍 Aplicar
          </button>
        </div>
      </div>

      {/* KPIs */}
      <div style={{ display: "grid", gridTemplateColumns: "repeat(auto-fit, minmax(160px, 1fr))", gap: 10 }}>
        <KpiCard label="Ventas en el período" valor={String(datos.num_ventas)} />
        <KpiCard label="Total facturado" valor={`$${datos.total_global.toFixed(2)}`} color="var(--color-success)" />
        <KpiCard label="Ticket promedio" valor={`$${datos.ticket_promedio.toFixed(2)}`} />
        <KpiCard label="IVA generado" valor={`$${datos.iva_global.toFixed(2)}`} />
        <KpiCard label="Descuentos" valor={`$${datos.descuento_global.toFixed(2)}`} color="var(--color-warning)" />
      </div>

      {/* Desglose por forma de pago */}
      {datos.por_forma_pago.length > 0 && (
        <div style={{
          display: "flex",
          flexWrap: "wrap",
          gap: 6,
          padding: 10,
          background: "var(--color-surface-alt)",
          borderRadius: 6,
          alignItems: "center",
        }}>
          <span style={{ fontSize: 11, fontWeight: 700, marginRight: 4 }}>Por forma de pago:</span>
          {datos.por_forma_pago.map(p => (
            <span key={p.forma_pago} style={{
              fontSize: 11,
              padding: "3px 8px",
              borderRadius: 12,
              background: "var(--color-surface)",
              border: "1px solid var(--color-border)",
            }}>
              <strong>{p.forma_pago}</strong>: ${p.total.toFixed(2)} ({p.num_ventas})
            </span>
          ))}
        </div>
      )}

      {/* Botones export */}
      <div style={{ display: "flex", gap: 8, justifyContent: "flex-end" }}>
        <button className="btn btn-outline" onClick={() => onExportar("xlsx")} style={{ fontSize: 12 }}>
          📊 Exportar Excel
        </button>
        <button className="btn btn-outline" onClick={() => onExportar("pdf")} style={{ fontSize: 12 }}>
          📄 Exportar PDF
        </button>
      </div>

      {/* Tabla */}
      <div style={{ overflowX: "auto", border: "1px solid var(--color-border)", borderRadius: 8 }}>
        <table style={{ width: "100%", borderCollapse: "collapse", fontSize: 12 }}>
          <thead>
            <tr style={{ background: "var(--color-surface-alt)" }}>
              <th style={thStyle}>Fecha</th>
              <th style={thStyle}>Número</th>
              <th style={thStyle}>Cliente</th>
              <th style={thStyle}>Cajero</th>
              <th style={thStyle}>Forma</th>
              <th style={thStyle}>Doc.</th>
              <th style={{ ...thStyle, textAlign: "right" }}>Subtotal</th>
              <th style={{ ...thStyle, textAlign: "right" }}>IVA</th>
              <th style={{ ...thStyle, textAlign: "right" }}>Desc.</th>
              <th style={{ ...thStyle, textAlign: "right" }}>Total</th>
              <th style={thStyle}>Estado</th>
            </tr>
          </thead>
          <tbody>
            {datos.ventas.length === 0 ? (
              <tr><td colSpan={11} style={{ padding: 24, textAlign: "center", color: "var(--color-text-secondary)" }}>Sin ventas para los filtros seleccionados</td></tr>
            ) : (
              datos.ventas.map(v => (
                <tr key={v.id} style={{ borderTop: "1px solid var(--color-border)", opacity: v.anulada ? 0.6 : 1 }}>
                  <td style={tdStyle}>{v.fecha.slice(0, 16).replace("T", " ")}</td>
                  <td style={tdStyle}>{v.numero}</td>
                  <td style={tdStyle}>
                    {v.cliente_nombre || "—"}
                    {v.cliente_identificacion && <div style={{ fontSize: 10, color: "var(--color-text-secondary)" }}>{v.cliente_identificacion}</div>}
                  </td>
                  <td style={tdStyle}>{v.cajero}</td>
                  <td style={tdStyle}>{v.forma_pago}</td>
                  <td style={tdStyle}>{v.tipo_documento}</td>
                  <td style={{ ...tdStyle, textAlign: "right" }}>${v.subtotal_sin_iva.toFixed(2)}</td>
                  <td style={{ ...tdStyle, textAlign: "right" }}>${v.iva.toFixed(2)}</td>
                  <td style={{ ...tdStyle, textAlign: "right" }}>${v.descuento.toFixed(2)}</td>
                  <td style={{ ...tdStyle, textAlign: "right", fontWeight: 700 }}>${v.total.toFixed(2)}</td>
                  <td style={tdStyle}>
                    {v.anulada
                      ? <span style={{ color: "var(--color-danger)", fontWeight: 600 }}>ANULADA</span>
                      : v.estado}
                  </td>
                </tr>
              ))
            )}
          </tbody>
          {datos.ventas.length > 0 && (
            <tfoot>
              <tr style={{ background: "var(--color-surface-alt)", fontWeight: 700 }}>
                <td style={tdStyle} colSpan={6}>TOTAL ({datos.num_ventas} ventas)</td>
                <td style={{ ...tdStyle, textAlign: "right" }}>${datos.ventas.reduce((s, v) => s + v.subtotal_sin_iva, 0).toFixed(2)}</td>
                <td style={{ ...tdStyle, textAlign: "right" }}>${datos.iva_global.toFixed(2)}</td>
                <td style={{ ...tdStyle, textAlign: "right" }}>${datos.descuento_global.toFixed(2)}</td>
                <td style={{ ...tdStyle, textAlign: "right", color: "var(--color-success)" }}>${datos.total_global.toFixed(2)}</td>
                <td style={tdStyle}></td>
              </tr>
            </tfoot>
          )}
        </table>
      </div>
    </div>
  );
}

const thStyle: React.CSSProperties = { padding: "8px 10px", textAlign: "left", fontWeight: 700, fontSize: 11, textTransform: "uppercase", letterSpacing: 0.4 };
const tdStyle: React.CSSProperties = { padding: "6px 10px" };

function KpiCard({ label, valor, sub, color, destacado }: { label: string; valor: string; sub?: string; color?: string; destacado?: boolean }) {
  return (
    <div className="card" style={destacado ? { border: "2px solid var(--color-success)" } : {}}>
      <div className="card-body" style={{ padding: "12px 16px" }}>
        <div style={{ fontSize: 11, color: "var(--color-text-secondary)", marginBottom: 4 }}>{label}</div>
        <div style={{ fontSize: 22, fontWeight: 700, color: color || "var(--color-text)" }}>{valor}</div>
        {sub && <div style={{ fontSize: 11, color: "var(--color-text-secondary)", marginTop: 2 }}>{sub}</div>}
      </div>
    </div>
  );
}

function BalanceRow({ label, valor, bold, color }: { label: string; valor: number; bold?: boolean; color?: string }) {
  return (
    <div className="flex justify-between" style={{ padding: "4px 0", fontWeight: bold ? 700 : 400 }}>
      <span>{label}</span>
      <span style={{ color: color || "var(--color-text)" }}>${valor.toFixed(2)}</span>
    </div>
  );
}

function IvaRow({ label, valor, bold, indent, color }: { label: string; valor: number; bold?: boolean; indent?: boolean; color?: string }) {
  return (
    <div
      className="flex justify-between"
      style={{
        padding: "6px 0",
        paddingLeft: indent ? 16 : 0,
        fontWeight: bold ? 700 : 400,
        fontSize: bold ? 14 : 13,
      }}
    >
      <span style={{ color: color || "var(--color-text)" }}>{label}</span>
      <span style={{ color: color || "var(--color-text)", fontFamily: "monospace" }}>${valor.toFixed(2)}</span>
    </div>
  );
}
