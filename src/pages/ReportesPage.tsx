import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { reporteUtilidad, reporteBalance, reporteProductosRentabilidad, exportarVentasCsv, reporteIvaMensual,
  reporteCxcPorCliente, reporteCxcDetalleCliente, reporteCxpPorProveedor, reporteCxpDetalleProveedor,
  reporteInventarioValorizado, reporteKardexProducto, reporteKardexMulti, listarCategoriasSimple } from "../services/api";
import { save } from "@tauri-apps/plugin-dialog";
import { useToast } from "../components/Toast";
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
  const [tab, setTab] = useState<"utilidad" | "balance" | "productos" | "iva" | "cxc" | "cxp" | "inventario" | "kardex">("utilidad");
  // Kardex multi-categoria
  const [kardexMultiData, setKardexMultiData] = useState<any | null>(null);
  const [categoriasMaestro, setCategoriasMaestro] = useState<Array<{ id: number; nombre: string }>>([]);
  const [kardexCatsSeleccionadas, setKardexCatsSeleccionadas] = useState<number[]>([]);
  const [kardexCargando, setKardexCargando] = useState(false);
  // CXC/CXP
  const [cxcResumen, setCxcResumen] = useState<any[]>([]);
  const [cxcClienteDetalle, setCxcClienteDetalle] = useState<{ cliente: any; cuentas: any[] } | null>(null);
  const [cxpResumen, setCxpResumen] = useState<any[]>([]);
  const [cxpProveedorDetalle, setCxpProveedorDetalle] = useState<{ proveedor: any; cuentas: any[] } | null>(null);
  // Inventario
  const [inventario, setInventario] = useState<any | null>(null);
  const [kardexProducto, setKardexProducto] = useState<{ producto: any; movimientos: any[] } | null>(null);
  const [busquedaInv, setBusquedaInv] = useState("");
  const [filtroEstado, setFiltroEstado] = useState<"TODOS" | "OK" | "BAJO" | "SIN_STOCK">("TODOS");
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
      else if (tab === "inventario") {
        const inv = await reporteInventarioValorizado();
        setInventario(inv);
        setKardexProducto(null);
      }
    } catch (err) {
      toastError("Error: " + err);
    }
    setCargando(false);
  };

  useEffect(() => { cargar(); }, [tab, desde, hasta]);

  // Cargar maestro de categorías al montar
  useEffect(() => {
    listarCategoriasSimple().then(setCategoriasMaestro).catch(() => {});
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

  const exportarKardexMultiCsv = () => {
    if (!kardexMultiData) return;
    exportarCsvGeneric(
      `kardex-multi-${desde}-${hasta}.csv`,
      ["Fecha", "Producto", "Codigo", "Categoria", "Tipo", "Cantidad", "Stock Anterior", "Stock Nuevo", "Costo Un.", "Motivo", "Usuario"],
      kardexMultiData.movimientos.map((m: any) => [
        m.fecha?.slice(0, 19).replace("T", " ") || "",
        m.nombre, m.codigo || "", m.categoria || "",
        m.tipo, m.cantidad, m.stock_anterior, m.stock_nuevo,
        m.costo_unitario != null ? m.costo_unitario.toFixed(2) : "",
        m.motivo || "", m.usuario || "",
      ])
    );
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

  const exportarUtilidadCsv = () => {
    if (!utilidad) return;
    const u: any = utilidad;
    exportarCsvGeneric(
      `estado-resultados-${desde}-${hasta}.csv`,
      ["Concepto", "Monto USD"],
      [
        [`Estado de Resultados (${desde} a ${hasta})`, ""],
        ["", ""],
        ["Ventas brutas", (u.ventas_brutas ?? 0).toFixed(2)],
        ["(-) Costo de ventas", (u.costo_ventas ?? 0).toFixed(2)],
        ["= Utilidad bruta", (u.utilidad_bruta ?? 0).toFixed(2)],
        ["Margen bruto (%)", (u.margen_bruto ?? 0).toFixed(2)],
        ["", ""],
        ["(-) Gastos totales", (u.total_gastos ?? 0).toFixed(2)],
        ["(-) Devoluciones/NC", (u.total_devoluciones ?? 0).toFixed(2)],
        ["", ""],
        ["= UTILIDAD NETA", (u.utilidad_neta ?? 0).toFixed(2)],
        ["Margen neto (%)", (u.margen_neto ?? 0).toFixed(2)],
        ["", ""],
        ["Num transacciones", u.num_ventas ?? 0],
        ["Promedio por venta", (u.promedio_por_venta ?? 0).toFixed(2)],
      ]
    );
  };

  const exportarBalanceCsv = () => {
    if (!balance) return;
    const b: any = balance;
    exportarCsvGeneric(
      `balance-${desde}-${hasta}.csv`,
      ["Concepto", "Monto USD"],
      [
        [`Balance (${desde} a ${hasta})`, ""],
        ["", ""],
        ["ENTRADAS", ""],
        ["Ventas totales", (b.total_ventas ?? 0).toFixed(2)],
        ["Cobros de credito", (b.total_cobros_credito ?? 0).toFixed(2)],
        ["", ""],
        ["SALIDAS", ""],
        ["Gastos", (b.total_gastos ?? 0).toFixed(2)],
        ["Pagos a proveedores", (b.total_pagos_proveedor ?? 0).toFixed(2)],
        ["Retiros de caja", (b.total_retiros ?? 0).toFixed(2)],
        ["", ""],
        ["UTILIDAD NETA", (b.utilidad_neta ?? 0).toFixed(2)],
      ]
    );
  };

  const exportarProductosCsv = () => {
    exportarCsvGeneric(
      `rentabilidad-productos-${desde}-${hasta}.csv`,
      ["Producto", "Unidades vendidas", "Ingresos", "Costo", "Utilidad", "Margen %"],
      productos.map((p: any) => [
        p.nombre,
        p.unidades_vendidas ?? p.cantidad_vendida ?? 0,
        (p.total_vendido ?? p.ingresos ?? 0).toFixed(2),
        (p.costo_total ?? p.costo ?? 0).toFixed(2),
        (p.utilidad ?? 0).toFixed(2),
        `${(p.margen ?? 0).toFixed(2)}%`,
      ])
    );
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
          {/* Boton export contextual segun el tab activo */}
          <button className="btn btn-primary" style={{ fontSize: 11, padding: "4px 14px", fontWeight: 600 }}
            onClick={async () => {
              try {
                if (tab === "utilidad") { exportarUtilidadCsv(); return; }
                if (tab === "balance") { exportarBalanceCsv(); return; }
                if (tab === "productos") { exportarProductosCsv(); return; }
                if (tab === "cxc") { exportarCxcCsv(); return; }
                if (tab === "cxp") { exportarCxpCsv(); return; }
                if (tab === "inventario") { exportarInventarioCsvFn(); return; }
                if (tab === "kardex") { exportarKardexMultiCsv(); return; }
                if (tab === "iva") { exportarIvaCsv(); return; }
                // Fallback: export de ventas
                const ruta = await save({ defaultPath: `reporte-ventas-${desde}-${hasta}.csv`, filters: [{ name: "CSV", extensions: ["csv"] }] });
                if (ruta) { await exportarVentasCsv(desde, hasta, ruta); toastExito("CSV exportado"); }
              } catch (e) { toastError("Error: " + e); }
            }}>📥 Exportar CSV</button>
        </div>
      </div>

      <div className="page-body" style={{ padding: 16 }}>
        {/* Tabs */}
        <div className="flex gap-2 mb-4">
          {([
            ["utilidad", "Estado de Resultados"],
            ["balance", "Balance"],
            ["productos", "Rentabilidad por Producto"],
            ["iva", "Declaracion IVA"],
            ["cxc", "Cuentas por Cobrar"],
            ["cxp", "Cuentas por Pagar"],
            ["inventario", "Inventario"],
            ["kardex", "Kardex Multi"],
          ] as const).map(([key, label]) => (
            <button key={key} className={`btn ${tab === key ? "btn-primary" : "btn-outline"}`}
              style={{ fontSize: 13, padding: "6px 16px" }} onClick={() => setTab(key)}>
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
              {/* Utilidad por categoría */}
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
                        <Bar dataKey="utilidad" fill="var(--color-success)" radius={[0, 4, 4, 0]} />
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
                        <td><span style={{ fontSize: 10, padding: "2px 6px", borderRadius: 3,
                          background: c.estado === "VENCIDA" ? "rgba(239,68,68,0.15)" : c.estado === "ABONADA" ? "rgba(245,158,11,0.15)" : "rgba(59,130,246,0.15)",
                          color: c.estado === "VENCIDA" ? "var(--color-danger)" : c.estado === "ABONADA" ? "var(--color-warning)" : "var(--color-primary)"
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
              <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: 12 }}>
                <span style={{ fontSize: 12, color: "var(--color-text-secondary)" }}>Click en un cliente para ver sus cuentas detalladas</span>
                <button className="btn btn-outline" onClick={exportarCxcCsv} disabled={cxcResumen.length === 0} style={{ fontSize: 11 }}>📥 Exportar CSV</button>
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
                    ) : cxcResumen.map((c: any) => (
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
                        <td><span style={{ fontSize: 10, padding: "2px 6px", borderRadius: 3,
                          background: c.estado === "VENCIDA" ? "rgba(239,68,68,0.15)" : c.estado === "ABONADA" ? "rgba(245,158,11,0.15)" : "rgba(59,130,246,0.15)",
                          color: c.estado === "VENCIDA" ? "var(--color-danger)" : c.estado === "ABONADA" ? "var(--color-warning)" : "var(--color-primary)"
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
              <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: 12 }}>
                <span style={{ fontSize: 12, color: "var(--color-text-secondary)" }}>Click en un proveedor para ver sus cuentas detalladas</span>
                <button className="btn btn-outline" onClick={exportarCxpCsv} disabled={cxpResumen.length === 0} style={{ fontSize: 11 }}>📥 Exportar CSV</button>
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
                    ) : cxpResumen.map((c: any) => (
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
                <button className="btn btn-outline" onClick={() => setKardexProducto(null)}>← Volver al inventario</button>
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
                      <th>Fecha</th><th>Tipo</th><th className="text-right">Cantidad</th>
                      <th className="text-right">Stock Anterior</th><th className="text-right">Stock Nuevo</th>
                      <th className="text-right">Costo</th><th>Motivo</th><th>Usuario</th>
                    </tr>
                  </thead>
                  <tbody>
                    {kardexProducto.movimientos.length === 0 ? (
                      <tr><td colSpan={8} style={{ textAlign: "center", padding: 30, color: "var(--color-text-secondary)" }}>Sin movimientos en este periodo</td></tr>
                    ) : kardexProducto.movimientos.map((m: any) => (
                      <tr key={m.id}>
                        <td style={{ fontSize: 11 }}>{m.fecha?.slice(0, 16).replace("T", " ")}</td>
                        <td><span style={{ fontSize: 10, padding: "2px 6px", borderRadius: 3,
                          background: m.tipo === "VENTA" ? "rgba(239,68,68,0.15)" : m.tipo.includes("COMPRA") || m.tipo.includes("INGRESO") ? "rgba(34,197,94,0.15)" : "rgba(148,163,184,0.15)",
                          color: m.tipo === "VENTA" ? "var(--color-danger)" : m.tipo.includes("COMPRA") || m.tipo.includes("INGRESO") ? "var(--color-success)" : "var(--color-text-secondary)"
                        }}>{m.tipo}</span></td>
                        <td className="text-right" style={{ color: m.cantidad < 0 ? "var(--color-danger)" : "var(--color-success)", fontWeight: 600 }}>
                          {m.cantidad > 0 ? "+" : ""}{m.cantidad}
                        </td>
                        <td className="text-right">{m.stock_anterior}</td>
                        <td className="text-right" style={{ fontWeight: 600 }}>{m.stock_nuevo}</td>
                        <td className="text-right">{m.costo_unitario ? fmt(m.costo_unitario) : "-"}</td>
                        <td style={{ fontSize: 11 }}>{m.motivo || "-"}</td>
                        <td style={{ fontSize: 11 }}>{m.usuario || "-"}</td>
                      </tr>
                    ))}
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
              {(inventario.productos_sin_stock > 0 || inventario.productos_stock_bajo > 0) && (
                <div style={{ display: "flex", gap: 12, marginBottom: 12 }}>
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
                  <option value="SIN_STOCK">Sin stock</option>
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
                        <td><span style={{ fontSize: 10, padding: "2px 6px", borderRadius: 3,
                          background: p.estado_stock === "SIN_STOCK" ? "rgba(239,68,68,0.15)" : p.estado_stock === "BAJO" ? "rgba(245,158,11,0.15)" : "rgba(34,197,94,0.15)",
                          color: p.estado_stock === "SIN_STOCK" ? "var(--color-danger)" : p.estado_stock === "BAJO" ? "var(--color-warning)" : "var(--color-success)"
                        }}>{p.estado_stock === "SIN_STOCK" ? "Sin stock" : p.estado_stock === "BAJO" ? "Bajo" : "OK"}</span></td>
                        <td><button className="btn btn-outline" style={{ fontSize: 10, padding: "2px 8px" }} onClick={() => verKardex(p.id)}>Kardex</button></td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              </div>
            </div>
          )
        )}

        {/* TAB KARDEX MULTI */}
        {tab === "kardex" && (
          <div>
            <div style={{ marginBottom: 12, padding: 12, background: "var(--color-surface)", borderRadius: 6, border: "1px solid var(--color-border)" }}>
              <div style={{ fontSize: 12, fontWeight: 600, marginBottom: 8 }}>
                Filtros (selecciona categorias o deja vacio para todas)
              </div>
              <div style={{ display: "flex", flexWrap: "wrap", gap: 6, marginBottom: 10 }}>
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
                {kardexCatsSeleccionadas.length > 0 && (
                  <button className="btn btn-outline" style={{ fontSize: 10, padding: "2px 8px" }}
                    onClick={() => setKardexCatsSeleccionadas([])}>
                    Limpiar
                  </button>
                )}
              </div>
              <div style={{ display: "flex", gap: 8, alignItems: "center" }}>
                <span style={{ fontSize: 11, color: "var(--color-text-secondary)" }}>
                  Periodo: {desde} a {hasta}
                </span>
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
                      {kardexMultiData.movimientos.length === 0 ? (
                        <tr><td colSpan={10} style={{ textAlign: "center", padding: 30, color: "var(--color-text-secondary)" }}>Sin movimientos en este periodo y filtros</td></tr>
                      ) : kardexMultiData.movimientos.map((m: any) => (
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
                          <td style={{ fontSize: 11 }}>{m.motivo || "-"}</td>
                          <td style={{ fontSize: 11 }}>{m.usuario || "-"}</td>
                        </tr>
                      ))}
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
      </div>
    </>
  );
}

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
