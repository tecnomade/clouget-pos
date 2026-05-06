# CHANGELOG — Clouget Punto de Venta

Historial de mejoras, correcciones y nuevas funcionalidades. Cada entrada incluye fecha y versión publicada en GitHub Releases.

Repositorio: https://github.com/tecnomade/clouget-pos/releases

---

## v2.3.63 — 2026-05-06 💵🐛 STABLE
**Descuentos por forma de pago + 3 fixes críticos.**

### 💵 Nueva feature: Descuentos automáticos por forma de pago

Permite configurar % de descuento automático según cómo paga el cliente. Caso típico Ecuador: incentivar pago en efectivo (sin comisión bancaria) o evitar pasar comisiones de tarjeta al cliente.

**Configuración** (admin → Configuración → "💵 Descuentos por forma de pago"):
- ☑ Activar descuentos automáticos
- % por método: Efectivo / Tarjeta / Transferencia / Crédito
- Aplicar sobre: Subtotal sin IVA (recomendado, no afecta IVA al SRI) o Total con IVA
- Monto mínimo de compra (opcional)

**POS**: cuando se activa, al elegir forma de pago el sistema calcula y muestra el descuento automáticamente:
```
Subtotal:        $100.00
IVA 15%:         $ 15.00
Total bruto:     $115.00 ───
✨ Descuento -5% por pago en EFECTIVO  -$5.75
TOTAL:           $109.25
```

**Pago MIXTO** NO aplica descuento (decisión por simplicidad — evita gaming del sistema).

Persistencia: el descuento se guarda en `ventas.descuento` (campo existente) con observación automática "Descuento -X% por pago en METODO" para trazabilidad en reportes.

**Pendiente Fase 2** (próxima versión): aplicar el mismo sistema al cobrar mesa en módulo Restaurante.

### 🐛 Fix crítico: items de mesa "desaparecen" al marcar como entregado

**Problema reportado**: usuario marca items como ENTREGADO desde pantalla de cocina y al volver a la mesa, los items habían desaparecido (mesa OCUPADA con $0.00 y "Sin items aún").

**Causa**: el query `rest_listar_mesas_con_estado` hacía LEFT JOIN simple a `rest_pedidos_abiertos` sin garantizar unicidad. Si por race condition o estado inconsistente había 2+ pedidos abiertos para la misma mesa, SQLite elegía aleatoriamente cuál mostrar — a veces uno vacío.

**Fix**:
- Subquery con `MAX(p.id)` garantiza que solo el pedido MÁS RECIENTE de cada mesa se muestre
- **Auto-limpieza idempotente**: pedidos abiertos vacíos (sin items) de más de 24h se cancelan automáticamente al cargar la página de mesas
- Sin afectar pedidos con items reales

### 🐛 Fix crítico: contador "transferencias por verificar" mostraba huérfanos

**Problema reportado**: el panel "Atención" del Dashboard mostraba "1 transferencia por verificar" aunque la única transferencia ya estaba marcada como Verificada.

**Causa**: en ventas MIXTAS (parte efectivo + parte transferencia), si admin verificaba la venta, se actualizaba `ventas.pago_estado='VERIFICADO'` pero la fila correspondiente en `pagos_venta` quedaba en `'REGISTRADO'`. El contador sumaba ambas tablas y contaba la huérfana.

**Fix**:
- `verificar_transferencia` ahora actualiza ambas tablas en cascada (origen='VENTA' también marca pagos_venta hijos; origen='PAGO_MIXTO' marca venta padre si todos los pagos están verificados)
- **Cleanup retroactivo idempotente** al cargar el contador: detecta huérfanos antiguos (creados antes de v2.3.63) y los marca como verificados
- Bonus: ventas anuladas con pago_estado='REGISTRADO' se marcan como 'NO_APLICA'

### ⌨️ UX fix: F10 (Nueva Venta) ahora pone focus en el buscador

**Problema reportado**: al presionar F10 después de cobrar, se abría la pantalla del POS pero el cajero tenía que hacer click manual en el buscador para empezar la siguiente venta.

**Fix**: agregado `setTimeout(50ms)` antes del `focus()` para esperar el re-render. Ahora el cursor va automáticamente al buscador y el cajero puede tipear inmediatamente. Bonus: si había texto previo, se selecciona todo (Ctrl+A automático).

### 🔒 UX fix anti-fuga: sin banner ruidoso al admin

**Problema reportado**: el banner "🔒 Modo anti-fuga ACTIVO" agregado en v2.3.62 generaba ruido visual al admin.

**Fix**: eliminado el banner. Comportamiento simplificado:
- Admin SIEMPRE ve el desglose verde con monto esperado (para auditoría)
- Cajeros NO ven el desglose si el toggle está activo, solo ven mensaje neutral "🔒 Conteo a ciegas — Ingresa el monto real contado"

### Cambios técnicos
- `src/utils/descuentoFormaPago.ts` (nuevo): helper puro TS con `leerConfigDescuento()` + `calcularDescuentoFormaPago()`. Cero dependencia backend.
- `src/pages/Configuracion.tsx`: nueva sección con toggle + 4 inputs % + radio buttons aplicar sobre + monto mínimo
- `src/pages/PuntoVenta.tsx`: state `configDescuento`, cálculo `descuentoFp`, visualización en panel de totales con badge verde, payload `descuento` + `observacion` automáticos
- `src/pages/CajaPage.tsx`: simplificación anti-fuga (sin banner)
- `src-tauri/src/restaurante/commands.rs`: subquery `MAX(p.id)` + auto-cleanup pedidos vacíos > 24h
- `src-tauri/src/commands/verificacion.rs`: cascada `verificar_transferencia` (VENTA↔PAGO_MIXTO) + cleanup retroactivo en `contar_transferencias_pendientes`

Verificado: `cargo check` OK, `tsc --noEmit` EXITCODE=0.

## v2.3.62 — 2026-05-05 🐛📄 STABLE
**Fix crítico Notas de Crédito + vista detalle + impresión universal + UX anti-fuga.**

Soluciona brechas críticas detectadas en auditoría del flujo de devoluciones / NC.

### 🔥 Fix crítico: NC SRI ahora afecta caja correctamente

**Problema**: cuando se hacía una NC SRI sobre una venta que se cobró en EFECTIVO, el sistema NO descontaba el dinero devuelto del `monto_esperado` de caja. Resultado: cierres de caja silenciosamente descuadrados por el monto reembolsado. Bug crítico que afectaba TODOS los clientes desde la primera versión.

**Fix**: extraje la lógica de "calcular reembolso + crear retiro automático" en helper compartido `calcular_y_aplicar_reembolso()`. Ahora tanto `registrar_nota_credito` (SRI) como `crear_devolucion_interna` la usan idéntico:
- Lee `forma_pago` original (incluido MIXTO con desglose proporcional desde `pagos_venta`)
- Calcula desglose: efectivo / transferencia / crédito a devolver
- Si hay efectivo y caja abierta → crea `retiro_caja` con motivo "Devolución NC X — efectivo a cliente"
- Resta `monto_esperado` para mantener cierre cuadrado

### 💾 Persistencia del reembolso (auditoría futura)

**Problema**: el desglose calculado se mostraba al cajero pero NO se guardaba en BD. Si volvías a buscar la NC mañana, no sabías cómo se devolvió el dinero.

**Fix**: nuevas columnas en `notas_credito` (migración SQL idempotente):
- `tipo_devolucion` (`'PARCIAL'` | `'TOTAL'`)
- `monto_efectivo_devuelto`, `monto_transfer_devuelto`, `monto_credito_devuelto`
- `metodo_reembolso` (`'EFECTIVO'` | `'TRANSFER'` | `'CREDITO'` | `'MIXTO'`)
- `retiro_caja_id` (FK al retiro automático generado)

NCs antiguas (creadas antes de v2.3.62) muestran "Sin información de reembolso registrada" — sin afectar nada existente.

### 👁 Vista detalle de NC (nueva)

**Antes**: al hacer click en una NC del listado, no abría nada. Solo botones SRI/XML/RIDE. No podías ver qué items se devolvieron sin abrir el PDF.

**Ahora**: nuevo botón **👁** en cada fila → abre `ModalDetalleNc`:
- Header con número, motivo, fecha, cliente, factura original, badge de estado SRI
- Tabla de items devueltos con cantidades, precios y subtotales
- **Sección "💵 Reembolso al cliente"** con desglose visual (3 cards: Efectivo / Transfer / Crédito)
- Indicador si se generó retiro automático de caja (#)
- Aviso si transferencia: "el reembolso lo realiza admin manualmente desde su app bancaria"
- Botones **🖨 Térmica** y **📄 PDF** para imprimir

### 🖨 Impresión universal de NC

**Antes**: el botón RIDE PDF solo aparecía para NC SRI autorizadas. Las devoluciones internas NO tenían forma de imprimir comprobante físico.

**Ahora**:
- Nuevo comando `imprimir_ticket_nc(nc_id)` → ESC/POS térmica para CUALQUIER NC
- Botón 📄 PDF disponible para autorizadas Y devoluciones internas
- El cliente siempre sale con comprobante físico

### 🔒 UX fix anti-fuga: aviso al admin cuando el modo está activo

**Problema reportado**: admin activa el toggle "Ocultar monto esperado a cajeros", abre Caja, ve el monto esperado y piensa "no funciona".

**Fix**: ahora cuando el modo anti-fuga está activo y el admin abre Caja, aparece un banner azul punteado:
> 🔒 **Modo anti-fuga ACTIVO** — Los cajeros NO ven este desglose. Vos sí (admin) para auditoría.

### Cambios técnicos
- `src-tauri/src/db/mod.rs`: 6 ALTER TABLE notas_credito (idempotentes con `.ok()`)
- `src-tauri/src/commands/ventas.rs`:
  - Nueva función helper `calcular_y_aplicar_reembolso()` (lógica compartida NC SRI + interna)
  - `registrar_nota_credito` ahora aplica el helper (fix crítico de caja)
  - `crear_devolucion_interna` refactorizada para usar el helper (sin duplicación)
  - Ambas persisten desglose en columnas nuevas
  - Nuevo `obtener_nota_credito(nc_id)` con header + items + datos venta original + reembolso
  - `listar_notas_credito` retorna también el desglose para mostrar en listado
- `src-tauri/src/commands/impresion.rs`: nuevo `imprimir_ticket_nc()` que reutiliza `printing::generar_ticket` adaptando NC a struct Venta con tipo_documento='NOTA_CREDITO'
- `src-tauri/src/lib.rs`: registrados nuevos comandos
- `src/services/api.ts`: wrappers `obtenerNotaCredito`, `imprimirTicketNc`
- `src/components/ModalDetalleNc.tsx` (nuevo, ~280 líneas): vista completa de detalle
- `src/pages/VentasDia.tsx`: state `verDetalleNcId`, botón 👁 en cada fila, botón PDF disponible para devoluciones internas
- `src/pages/CajaPage.tsx`: banner aviso anti-fuga visible al admin cuando el toggle está activo

Verificado: `cargo check` OK, `tsc --noEmit` EXITCODE=0.

## v2.3.61 — 2026-05-05 ✨ STABLE
**Fase 2 polish premium**: dashboard rediseñado + sistema de diseño consistente.

Continúa el rediseño UI iniciado en v2.3.59, llevándolo a nivel "premium SaaS" (Stripe / Linear). 100% visual, sin tocar lógica de negocio.

### 💰 KPI Hero (estilo Stripe)
**Antes**: 6 cards iguales del mismo tamaño, todos compitiendo por atención.

**Ahora**:
- **1 Hero card** prominente arriba con el número MÁS importante (Ventas Hoy) en 36px
- Comparación vs ayer con badge ↑12% / ↓5% en color contextual (verde/rojo)
- Contexto adicional: "9 transacciones · ticket promedio $5.20 · utilidad $39.02"
- Ícono decorativo 💰 al lado (sutil, 56px con opacidad 15%)
- **3 cards secundarios** abajo (Efectivo, Transferencia, Por cobrar) con ícono propio
- Hover: lift sutil + sombra mejorada (estilo Linear)

### 📦 Stock Bajo más visual
**Antes**: lista plana con barras de progreso, header simple "Stock Bajo (1301)".

**Ahora**:
- **Chips de severidad** en el header: 🔴 X sin stock + 🟠 Y crítico
- Barras de progreso con color contextual (rojo=agotado, naranja=crítico, verde=OK)
- Cantidades como **badges coloreados** (no solo texto)
- Estado vacío celebratorio: "✨ Stock OK — Todos los productos con stock suficiente"
- **Botón "Ver los X restantes →"** si hay más de 8 productos con stock bajo
- Transiciones suaves en las barras

### 🎨 Sistema de diseño consistente
- **Sombras nuevas estilo Stripe/Linear**: 2 capas sutiles en vez de border prominente
  - `--shadow`: 1px+3px sutil (cards default)
  - `--shadow-md`: 2px+8px (cards hover/elevated)
  - `--shadow-lg`: 4px+24px (modals, drawers)
  - `--shadow-hover`: estado hover de cards interactivas
- **Radius consistente**: `--radius` 10px (default), `--radius-sm` 6px (chips), `--radius-lg` 14px (hero)
- **Tipografía con escala**: H1 22px / H2 18px / H3 15px / body 14px / caption 12px / micro 10-11px uppercase
- **Card-header**: bordes más delgados (1px en vez de 2px) para look más refinado

### ✨ Animaciones sutiles
- `.anim-fade-up`: cards aparecen con fade + slide up (320ms cubic-bezier)
- `.anim-fade-in`: aparición simple (250ms)
- `.skeleton`: shimmer animado para estados de carga (en lugar de "Cargando..." plano)
- `.kpi-card:hover`: lift de 1px + sombra mejorada
- `prefers-reduced-motion`: respeta accesibilidad del usuario

### 🌓 Tema dark refinado
- Sombras dark theme con 2 capas (más realistas)
- Mantiene contraste sin ser "duras"

### Cambios técnicos
- `src/styles/global.css`:
  - Variables CSS: nuevas `--radius-sm/lg`, sombras refactoradas
  - `.card`: transición + sombras nuevas
  - `.kpi-card`: hover lift dedicado
  - Sistema tipográfico (h1-h3 con sizes definidos)
  - Keyframes `anim-fade-up`, `anim-fade-in`, `anim-skeleton`
  - Clases reutilizables `.anim-fade-up`, `.skeleton`
  - Media query `prefers-reduced-motion`
- `src/pages/DashboardPage.tsx`:
  - KPI Hero card grande estilo Stripe con comparativo
  - 3 KPIs secundarios con íconos
  - Stock Bajo: chips de severidad + estado vacío celebratorio + botón "ver más"
- `src/pages/DashboardPage.tsx::KpiCard`: prop `icon` opcional + estilos refinados

Verificado: `tsc --noEmit` EXITCODE=0. Solo UI/UX, cero impacto backend.

## v2.3.60 — 2026-05-05 🐛🔒 STABLE
**5 fixes + 1 feature de seguridad** (anti-fuga en cierre de caja).

### 🐛 Bugs corregidos

1. **Imágenes no se mostraban en módulo Restaurante** (selector productos): faltaba el prefix `data:image/png;base64,` que SÍ usa el POS normal. Ahora se muestran iguales que en el POS, con fallback de inicial estilizada cuando no hay imagen.

2. **Cobrar mesa con TRANSFER no permitía elegir cuenta bancaria**: ahora al click en "🏦 Transfer." abre un sub-modal con selector de banco + referencia + aviso de verificación. Mismo flujo que el POS normal — la transferencia queda registrada en `/movimientos-bancarios` y aparece en panel de verificación admin.

3. **Sidebar expandido no permitía scroll** — items inferiores (Operaciones, Reportes, Cerrar sesión) quedaban cortados sin acceso. Causa: `overflow:visible` para mostrar pseudo-element del indicador activo bloqueaba el scroll. Fix: indicador activo ahora con `box-shadow inset` (no se sale del item) + `overflow-y:auto` siempre activo. Bonus: scrollbar sutil estilo Linear.

4. **Contador "transferencias por verificar" mostraba transferencias YA verificadas**: el query contaba TODAS las transferencias `REGISTRADO` sin límite de tiempo, incluyendo las de pruebas viejas que el usuario olvidó. Ahora limita a últimos 60 días para mantener consistencia con el filtro "Este mes" de `/movimientos-bancarios`.

### 🔒 Feature nueva: Ocultar monto esperado a cajeros (anti-fuga)

**Problema real**: si el cajero cobra de más a un cliente y se queda con la diferencia, viendo el "monto esperado" puede "ajustar" su conteo para que cuadre exactamente, ocultando el faltante.

**Solución**: nueva opción en **Configuración → Sistema → Control y Seguridad**:
- ☑ **🔒 Ocultar monto esperado a cajeros (anti-fuga)**

Cuando se activa:
- **Cajeros NO ven** el desglose verde con el monto esperado al cerrar caja. Solo ven mensaje neutral "Conteo a ciegas — Ingresa el monto real contado en caja".
- **Cuentan el efectivo a ciegas** y el sistema detecta diferencias.
- **Admin SIEMPRE ve** la información completa (no se oculta para él).

Esto evita que un cajero deshonesto sepa cuánto debe ajustar para que "cuadre".

### Cambios técnicos
- `src/restaurante/components/SelectorProductos.tsx`: prefix base64 + fallback inicial
- `src/restaurante/components/PedidoDetalle.tsx`: ModalCobro con sub-vista TRANSFER (selector banco + referencia + obtenerConfig para `transferencia_requiere_referencia`); handleCobrar pasa `banco_id` y `referencia_pago` al payload
- `src/components/Layout.tsx`: sidebar `overflowY:auto` + `overflowX:hidden` siempre
- `src/styles/global.css`: indicador activo con `box-shadow inset 3px 0 0 #60a5fa`; scrollbar `::-webkit-scrollbar` estilo Linear
- `src-tauri/src/commands/verificacion.rs::contar_transferencias_pendientes`: filtro `DATE(fecha) >= DATE('now', '-60 days')`
- `src/pages/Configuracion.tsx`: toggle `ocultar_monto_esperado_caja` en sección Control y Seguridad
- `src/pages/CajaPage.tsx`: state `ocultarMontoEsperado` + `ocultarParaCajero`; condicional en bloque verde de cierre

Verificado: `cargo check` OK (16 warnings preexistentes), `tsc --noEmit` EXITCODE=0.

## v2.3.59 — 2026-05-05 🎨 STABLE
**Rediseño UI: sidebar agrupado + header limpio + dashboard humanizado.**

Mejoras 100% visuales/UX siguiendo principios de apps modernas (Linear, Notion, Stripe). Sin tocar lógica de negocio, base de datos ni backend.

### 🗂️ Sidebar agrupado con expandir/colapsar
Antes: 14+ íconos sueltos sin agrupar — saturado y difícil de escanear.

Ahora:
- **Items agrupados visualmente** en 7 secciones lógicas:
  - PRINCIPAL (Inicio)
  - VENTAS (Venta POS, Ventas día, Cobrar, Guías)
  - GESTIÓN (Productos, Clientes, Inventario, Series, Caducidad)
  - COMPRAS (Compras, Pagar, Bancos)
  - OPERACIONES (Gastos, Servicio Técnico)
  - RESTAURANTE (Mesas, Cocina) — solo si módulo activo
  - ANALÍTICA (Reportes)
- **Modo colapsado** (default, 56px): íconos + separadores sutiles entre grupos
- **Modo expandido** (200px): íconos + labels + headers de grupos en uppercase + atajos visibles
- **Botón toggle** (chevron arriba) alterna estados, **persistente en localStorage**
- **Indicador activo** mejorado: barra azul de 3px a la izquierda del item activo (estilo Linear)
- Atajos F1-F10 funcionan idéntico en ambos modos

### 🏷️ Header limpio (sin logo redundante)
Antes: el logo CLOUGET aparecía DOS veces (barra de Windows + header) — redundancia visual clásica.

Ahora (estilo Notion/Linear):
- Logo Windows mantiene branding (barra de título)
- En el header solo: **logo pequeño 18px (botón "home") + NOMBRE DEL NEGOCIO + página actual** como breadcrumb
- Ejemplo: `🟦 FERMAGRI · Caja` en vez de `🟦 CLOUGET Punto de Venta`
- Le da contexto útil al usuario: sabe en qué empresa está y dónde
- Aprovecha el espacio para info útil en lugar de duplicar branding

### 👋 Dashboard con saludo personalizado
Antes: `Inicio` + fecha plana `2026-05-05` arriba.

Ahora:
- **Saludo dinámico según hora**: "Buenos días/tardes/noches, [Nombre Usuario]" 👋
- **Fecha en español natural**: "martes 5 de mayo · Caja abierta desde 8:30 a.m."
- Estado de caja visible y contextual (verde si abierta, rojo si cerrada)

### 🔔 Panel "Atención" reemplaza "Acciones Rápidas"
Antes: card con 4 botones (POS, Ventas, Caja, Productos) que duplicaban el sidebar.

Ahora: panel inteligente que muestra **solo lo que requiere acción**:
- 🏦 Transferencias por verificar
- ⏰ Pagos vencidos a proveedores
- 💵 Pendiente de cobro a clientes (fiados)
- 📅 Lotes vencidos
- ⚠ Lotes por vencer pronto
- 💰 Estado caja (con monto vendido si abierta, "Abrir →" si cerrada)
- ✨ Si nada pendiente: mensaje positivo "Todo al día"

Cada alerta es **clickeable** y navega directo a la página correspondiente. Lateral colorido por severidad (rojo/naranja/azul).

### Cambios técnicos
- `src/components/Layout.tsx`:
  - `navItems` con campo `group: GroupKey`
  - Render del sidebar agrupado con headers/separadores condicionales
  - State `sidebarExpandido` persistente + CSS variable `--sidebar-width`
  - State `nombreNegocio` (lee de config) + `tituloPagina` (mapea ruta)
  - Header rediseñado con breadcrumb
- `src/styles/global.css`:
  - `.sidebar-compact` con width fijo eliminado (ahora dinámico via inline style)
  - `.main-content` margin-left usa CSS variable
  - `.sidebar-compact .nav-item` ajustado para soportar ambos modos
  - `.nav-item.active` con barra lateral azul (estilo Linear)
- `src/pages/DashboardPage.tsx`:
  - Funciones `saludoHora()`, `fechaNatural()`, `horaCorta()`
  - Nuevos states `caducidadVencidos`, `caducidadPorVencer`, `transferenciasPendientes`
  - Header rediseñado
  - Panel "Atención" con array dinámico de alertas

Verificado: `tsc --noEmit` EXITCODE=0. Solo UI/UX, sin tocar backend ni lógica.

## v2.3.58 — 2026-05-05 🚀 STABLE
**Promoción a STABLE de los 5 cambios validados en canal beta.**

Esta versión consolida en canal estable los cambios que se probaron durante varios días en canal BETA. Resumen ejecutivo:

### 🔥 Hotfix crítico (urgente para todos)
**Fechas de caducidad importadas como serial Excel** — al importar productos desde Excel donde la columna fecha_caducidad tenía formato Fecha (no Texto), se guardaba el número serial Excel crudo (ej. "46265") en vez de la fecha real ("2026-06-28"). Resultado: lotes con "días restantes: -2,414,893" y estado "Vencido" para productos buenos.

**Fix triple**:
- ✅ **Botón "🔧 Reparar fechas"** en página Caducidad. Si detectamos lotes con fechas-bug (días < -100000), aparece destacado en amarillo. Click → corrige TODOS los lotes en 1 segundo. Idempotente.
- ✅ **Importer Excel arreglado** para futuras importaciones — detecta DateTime/Float/Int en rango Excel serial y convierte a YYYY-MM-DD automáticamente.
- ✅ **Validación al guardar lote** con `chrono::NaiveDate` — previene que el bug entre por cualquier ruta.

### 🍴 Módulo Restaurante (nuevo, opcional)
Sistema completo para restaurantes/cafeterías/bares — solo visible si tu licencia tiene el módulo "restaurante" activo (sin el módulo, no se ve nada nuevo).

**Funcionalidades**:
- Mesas y zonas con estados visuales (libre/ocupada/cuenta pedida)
- Pedidos por mesa con comandas a cocina
- Pantalla cocina (TV/tablet) con flujo PENDIENTE → EN COCINA → LISTO → ENTREGADO
- **Despacho directo** por producto (bebidas embotelladas, snacks no van a cocina)
- **Pre-cuenta impresa** al pedir cuenta (con auto-detección de impresora térmica vs PDF nativo)
- Cobrar mesa delega a `registrar_venta` → SRI, combos, IVA, secuenciales, stock, kardex funcionan automáticamente

**Activación**: desde admin.clouget.com → Licencias → Editar Módulos → ✅ Restaurante.

### 🎯 Resumen de cambios incluidos (v2.3.54 a v2.3.58 unificados)
| Categoría | Cambio |
|---|---|
| ✨ Nuevo | Módulo Restaurante completo (mesas, cocina, comandas) |
| ✨ Nuevo | Brand flag para variantes DigitalServer POS |
| 💎 Mejora | Despacho directo por producto + pre-cuenta impresa |
| 💎 Mejora | Pre-cuenta auto-genera PDF si impresora es virtual |
| 🧹 UX | Ocultar selector "Destino Restaurante" si módulo inactivo |
| 🐛 Hotfix | Reparación + import correcto de fechas Excel serial |

### 📥 Para todos los clientes (con o sin Restaurante)
- ✅ **Recibirán el botón "🔧 Reparar fechas"** automáticamente al actualizar
- ✅ **Sus importaciones Excel futuras** ya no rompen fechas
- ✅ **Su sistema de stock/SRI/combos/cierre de caja** intactos — cero cambios visibles
- 🔒 **Si NO tienen módulo Restaurante**: no ven menú Mesas, Cocina ni opciones de pre-cuenta. El módulo está estrictamente gateado por licencia.

### 🔧 Cambios técnicos consolidados (referencia para soporte)
- `src-tauri/src/utils.rs`: `excel_serial_to_iso()`, `parse_posible_serial_excel()`
- `src-tauri/src/branding.rs` (nuevo): brand flag compile-time Clouget vs DigitalServer
- `src-tauri/src/restaurante/` (nuevo): mod, schema, models, commands, http stub, printing (ESC/POS + PDF)
- `src-tauri/src/db/mod.rs`: migración `ALTER TABLE productos ADD COLUMN destino_preparacion`
- `src-tauri/src/commands/productos.rs`: importer Excel con `get_fecha()`, validación NaiveDate, comando `reparar_fechas_caducidad`
- `src/restaurante/`: pages (Mesas, Cocina, ConfigMesas) + components (PedidoDetalle, SelectorProductos)
- `src/main.tsx` + `src/components/Layout.tsx`: rutas + nav items gateados por brand+licencia
- `src/pages/Productos.tsx`: selector "Destino" condicional
- `src/pages/CaducidadPage.tsx`: botón Reparar fechas

### Versiones beta superadas
v2.3.54-beta, v2.3.55-beta, v2.3.56-beta, v2.3.57-beta, v2.3.58-beta — todas consolidadas en este release stable.

## v2.3.58-beta — 2026-05-05 🐛📅
**Hotfix crítico: fechas de caducidad importadas como serial Excel.**

Bug histórico detectado en cliente real: al importar productos desde Excel donde la columna "fecha_caducidad" tenía formato **Fecha** en Excel (no Texto), la librería `calamine` devolvía el valor como `Data::DateTime/Float` con el número serial Excel (días desde 1899-12-30). El código hacía `.to_string()` y guardaba **"46265"** en lugar de **"2026-06-28"** en `lotes_caducidad.fecha_caducidad`. Resultado: lotes con "días restantes: -2,414,893" y estado "Vencido" para productos buenos.

### Fix triple

**1. Importer Excel ahora detecta y convierte fechas correctamente** (futuro):
- Nuevo helper `get_fecha()` en `importar_productos_excel` que distingue celdas Fecha de Texto.
- Si la celda viene como `Data::DateTime/DateTimeIso/Float/Int` con valor en rango Excel serial (30000-100000) → convierte a `YYYY-MM-DD` con `excel_serial_to_iso()`.
- Si viene como `Data::String` que es número puro en rango → también convierte.
- Si ya es string `YYYY-MM-DD` válido → usa tal cual.

**2. Comando nuevo `reparar_fechas_caducidad`** (presente):
- Recorre todos los lotes en `lotes_caducidad`.
- Detecta `fecha_caducidad` o `fecha_elaboracion` que sean números puros entre 30000-100000.
- Convierte y hace `UPDATE` atómico.
- **Idempotente**: re-ejecutarlo no causa problema (los ya arreglados ya no matchean el patrón).
- Retorna `{ revisados, reparados, ejemplos }` para auditoría.

**3. Validación al guardar lote** (defensa en profundidad):
- `registrar_lote_caducidad` ahora valida que `fecha_caducidad` y `fecha_elaboracion` parseen como `YYYY-MM-DD` válido con `chrono::NaiveDate`.
- Si no, error claro: *"Fecha de caducidad invalida: '46265'. Formato esperado: YYYY-MM-DD"*.
- Previene que el bug vuelva por cualquier otra ruta de entrada.

### UX

- Botón **"🔧 Reparar fechas"** en página Caducidad (esquina superior derecha junto a "Exportar CSV").
- Si detectamos automáticamente lotes con `dias_restantes < -100000` (claramente bug): el botón aparece **destacado en amarillo con ⚠** invitando a clickearlo.
- Al click: confirmación + ejecución + toast con resultado: *"Reparados X de Y lotes ✓"*.

### Cambios técnicos
- `src-tauri/src/utils.rs`: nuevos helpers `excel_serial_to_iso(f64) -> Option<String>` (compatible con bug del 1900) y `parse_posible_serial_excel(&str) -> Option<f64>`.
- `src-tauri/src/commands/productos.rs`:
  - `importar_productos_excel`: closure `get_fecha()` para columnas de fecha
  - `registrar_lote_caducidad`: validación `chrono::NaiveDate::parse_from_str` antes de INSERT
  - Nuevo `reparar_fechas_caducidad` Tauri command
- `src-tauri/src/lib.rs`: registrado nuevo comando
- `src/services/api.ts`: wrapper `repararFechasCaducidad()`
- `src/pages/CaducidadPage.tsx`: handler + botón + detector `tieneFechasBug`

## v2.3.57-beta — 2026-05-05 🧹
**UX: ocultar selector "Destino (Restaurante)" en Productos cuando el módulo no está activo.**

Antes: la sección "🍴 Destino (Restaurante)" aparecía siempre al editar un producto, incluso para clientes que no tienen el módulo Restaurante en su licencia. Confundía porque mostraba opciones que no aplicaban.

Ahora: la sección **solo aparece** si:
1. El build incluye el módulo (`FEATURES.restaurante`, true en Clouget, false en DigitalServer)
2. La licencia activa tiene `"restaurante"` en `licencia_modulos`

Si las dos condiciones no se cumplen, la sección queda oculta y el producto mantiene `destino_preparacion = 'COCINA'` por default sin que el usuario tenga que verlo.

Sin cambios técnicos en backend — solo UI condicional en `src/pages/Productos.tsx` con helper `moduloRestauranteActivo(config.licencia_modulos)`.

## v2.3.56-beta — 2026-05-05 🐛
**Hotfix Restaurante: pre-cuenta auto-detecta impresora virtual y genera PDF nativo.**

Bug detectado en v2.3.55-beta: cuando el cliente tenía configurada una "impresora virtual" (Microsoft Print to PDF, OneNote, XPS, Fax) en lugar de impresora térmica real, la pre-cuenta se enviaba como bytes ESC/POS crudos a esa impresora virtual, generando un PDF ilegible con caracteres binarios.

**Solución**: el comando `rest_imprimir_pre_cuenta` ahora detecta automáticamente el tipo de impresora:
- **Impresora térmica real** (POS-58, Epson TM-T20, etc.) → ESC/POS bytes (igual que antes)
- **Impresora virtual** (PDF/OneNote/XPS/Fax) o **sin impresora configurada** → genera PDF nativo legible con `genpdf` y lo abre con el visor del sistema

El PDF generado es 80mm de ancho (mismo formato que tickets POS) e incluye toda la info: cabecera negocio, datos mesa (mesero, comensales, hora apertura, # pedido), items agrupados con observaciones, total prominente y aviso "ESTE DOCUMENTO NO ES UN COMPROBANTE FISCAL".

Cambios técnicos:
- `restaurante/printing.rs`: nueva función `generar_pre_cuenta_pdf()` con genpdf (similar a `sri::ride::generar_ticket_pdf`)
- `restaurante/commands.rs::rest_imprimir_pre_cuenta`: helper `impresora_es_virtual()` + branch automático ESC/POS vs PDF
- Sin cambios en frontend — la transición es transparente.

## v2.3.55-beta — 2026-05-05 🍴
**Restaurante: despacho directo + pre-cuenta impresa** — UX completa para flujo real.

Resuelve dos brechas críticas detectadas en la v2.3.54-beta cuando se usaba el módulo Restaurante con clientes reales:

### 1. 📦 Despacho directo por producto (Opción A)
- **Nuevo campo en cada producto: "Destino (Restaurante)"** con 3 opciones:
  - 🍳 **Cocina** (default, comportamiento anterior — preparado por cocinero, aparece en /cocina)
  - 🍷 **Barra** (cocteles, café preparado — también va a /cocina, badge violeta)
  - 📦 **Despacho directo** (bebidas embotelladas, snacks, postres en exhibición — el mesero los toma del mostrador)
- **Items DIRECTO no aparecen en /cocina**: se insertan en el pedido ya marcados como `enviado_cocina=1, estado_cocina='ENTREGADO'`. El cocinero/parrillero ya no ve la Coca-Cola ni el agua entre los items que tiene que preparar.
- **Badge visual en pedido**: items DIRECTO se ven con fondo verde claro y badge "📦 DIRECTO". Items BARRA con badge "🍷 BARRA NUEVO" → "🍷 EN BARRA".
- **Items DIRECTO se pueden eliminar** (no como los items COCINA enviados, que no se pueden borrar). Si el mesero se equivocó al agregar la Coca, la borra.
- **Migración SQL safe**: `ALTER TABLE productos ADD COLUMN destino_preparacion TEXT NOT NULL DEFAULT 'COCINA'`. Productos existentes mantienen comportamiento anterior automáticamente.
- **Configuración en pantalla Productos**: nuevo selector debajo del tipo de producto. Editas cada producto una vez y queda configurado para siempre.

### 2. 📄 Pre-cuenta impresa al "Pedir cuenta"
- Al click en **"Pedir cuenta"**, el sistema ahora **automáticamente imprime un ticket "PRE-CUENTA"** en la impresora térmica configurada (la misma del POS).
- Ticket incluye: nombre negocio + logo (si está cargado), datos de mesa (nombre, zona, mesero, comensales, hora apertura, # pedido), detalle de items con observaciones, total, y aviso prominente: **"ESTE DOCUMENTO NO ES UN COMPROBANTE FISCAL — Solicite su factura al pagar"**.
- La pre-cuenta es **solo informativa**. El comprobante fiscal real (Nota de Venta o Factura SRI) se sigue generando al cobrar (botón "💰 Cobrar"), igual que antes — sin cambios al flujo de cobro ni al sistema SRI.
- **Nuevo botón "🖨 Reimprimir cuenta"** aparece después de pedir cuenta. Si el cliente la pierde o quiere otra copia, la reimprimís sin afectar nada.
- Si NO hay impresora configurada, el botón "Pedir cuenta" igual marca la mesa como CUENTA_PEDIDA y muestra warning, pero no rompe el flujo.

### 3. 🚫 Bloqueo de agregar items con cuenta pedida (con confirmación)
- Después de pedir cuenta, el botón "+ Agregar productos" cambia su texto a **"+ Agregar productos (mesa pidió cuenta)"** y al click pide confirmación: *"Esta mesa ya pidió la cuenta y la pre-cuenta fue impresa. Si agregas más productos, deberás reimprimir la pre-cuenta. ¿Continuar?"*
- Esto evita el caso real donde el cliente ve la pre-cuenta, paga, y después el sistema le cobra más.
- Si el mesero confirma, agrega el item normalmente y el botón "Reimprimir cuenta" sigue disponible para emitir una pre-cuenta actualizada.

### Cambios técnicos
- **Backend**:
  - `db/mod.rs`: migración ALTER TABLE productos (idempotente, .ok())
  - `models/producto.rs`: campo `destino_preparacion` con default 'COCINA'
  - `commands/productos.rs`: crear/actualizar/obtener leen el campo nuevo
  - `restaurante/commands.rs`: `rest_agregar_item` lee destino → si DIRECTO inserta marcado como entregado; `rest_eliminar_item` permite borrar items DIRECTO; `rest_imprimir_pre_cuenta` (nuevo) reutiliza `printing/mod.rs`
  - `restaurante/printing.rs` (nuevo): `generar_pre_cuenta()` — ticket ESC/POS estilo restaurante con cabecera negocio + datos mesa + items agrupados + totales + aviso fiscal
  - `printing/mod.rs`: helpers (`linea_separador_simple/doble`, `linea_monto`, `format_cantidad`, `logo_to_raster_pub`) ahora públicos para reutilizar
  - `server/dispatch.rs`: SELECT productos también trae `destino_preparacion`
- **Frontend**:
  - `types/index.ts`: campo `destino_preparacion?: string` en Producto
  - `restaurante/types.ts`: campo `destino_preparacion?: string` en PedidoItem
  - `pages/Productos.tsx`: selector "Destino (Restaurante)" debajo de tipo_producto
  - `restaurante/api.ts`: nuevo wrapper `imprimirPreCuenta(pedidoId)`
  - `restaurante/components/PedidoDetalle.tsx`:
    - `handlePedirCuenta` ahora también llama `imprimirPreCuenta` (con fallback warning si falla impresora)
    - `handleReimprimirPreCuenta` (nuevo)
    - Botón "+ Agregar productos" pide confirmación si CUENTA_PEDIDA
    - Botón "Pedir cuenta" se reemplaza por "🖨 Reimprimir cuenta" cuando estado=CUENTA_PEDIDA
    - `ItemRow`: badges DIRECTO/BARRA + colores fondo distintos + permitir eliminar items DIRECTO

### Cero impacto en POS normal
- Productos existentes: mantienen `destino='COCINA'` por default. Sin cambios visibles si no usas Restaurante.
- Sistema de ventas, SRI, combos, kardex, cierre de caja: intactos.
- Solo se ven cambios si:
  1. El build incluye el módulo (`branding::BRAND.tiene_modulo_restaurante()`) — sí en Clouget, no en DigitalServer
  2. La licencia tiene `"restaurante"` en módulos (admin lo asigna por cliente)

## v2.3.54-beta — 2026-05-05 🍴
**Nuevo módulo: Restaurante** (mesas, comandas, cocina) — versión BETA para early adopters.

Pensado para restaurantes, cafeterías, bares, food trucks. Convierte Clouget POS en un sistema completo de restaurante con flujo natural de mesa→pedido→cocina→cobro.

### Backend (Fase 1)
- **Tablas nuevas**: `rest_zonas`, `rest_mesas`, `rest_pedidos_abiertos`, `rest_pedido_items` (todas con prefijo `rest_` para no chocar con el resto del schema). Incluye seed inicial: 1 zona "Salón" con 6 mesas de capacidad 4.
- **21 comandos Tauri**: CRUD de zonas/mesas + flujo completo de pedido (abrir, agregar items con observación tipo "sin cebolla", enviar a cocina, marcar listo, pedir cuenta, cobrar, cancelar).
- **Brand flag compile-time**: `src-tauri/src/branding.rs` permite generar build de **DigitalServer POS** que NO incluya este módulo (solo Clouget lo lleva). Doble capa de control: brand (qué EXISTE en binario) + license module (qué está ACTIVO por cliente).
- Cada comando valida que la licencia activa tenga el módulo `"restaurante"` antes de operar.

### UI Desktop (Fase 2)
- **Página /mesas**: grid visual de mesas con auto-refresh 15s. Estados con código de color: 🟢 LIBRE, 🟢 OCUPADA con total y minutos abierta, 🟡 CUENTA PEDIDA. Filtro por zona, badge de items pendientes en cocina, botón flotante para configurar.
- **Página /cocina**: vista TV/tablet con items pendientes agrupados por mesa. Código de color por antigüedad (rojo si >15min). Click en item cycla estado: PENDIENTE → EN COCINA → LISTO → ENTREGADO. Auto-refresh 8s.
- **Página /config-mesas** (solo admin): CRUD de zonas (con paleta de 8 colores) + mesas (asignación de zona, capacidad).
- **Drawer "Detalle pedido"**: items agrupados con badges (NUEVO, EN COCINA, LISTO), botones de acción (Agregar productos, Enviar cocina, Pedir cuenta, Cobrar con 4 formas de pago, Cancelar).
- **Modal selector de productos**: grid táctil con búsqueda + filtro por categoría. Click=agregar 1, click-derecho/📝=agregar con observación.

### Integración con sistema existente (cero rework)
- **Cobrar mesa delega a `registrar_venta`**: combos fijos/flexibles, IVA, SRI, secuenciales, descuento de stock, validación de caja abierta — todo funciona idéntico al POS normal porque NO se reimplementa, se reutiliza.
- Después del cobro, `rest_cerrar_pedido` vincula la venta con el pedido (campo `venta_id`) y libera la mesa.
- La venta queda con observación automática: `Mesa: Mesa 1 (Salón) · Pedido #123` para trazabilidad desde Ventas del Día.

### Activación
- **Modo demo**: viene activo automáticamente — los íconos 🍴 Mesas y 🍳 Cocina aparecen en sidebar al activar Modo Demo.
- **Licencia real**: el módulo `"restaurante"` se asigna por cliente desde admin.clouget.com (precio sugerido: +$99 sobre los $199 base = $298 plan Restaurante).
- Si el cliente no tiene el módulo en su licencia, los nav items NO aparecen y las rutas no se registran.

### Próximas fases (próximas versiones)
- **Fase 3**: app móvil para meseros (React Native + Expo, repo separado `clouget-mesero`) → conexión por WiFi local al PC servidor, mDNS auto-discovery, login con PIN.
- Imprimir ticket cocina automático al "Enviar cocina"
- Sonido de notificación en CocinaPage
- Soporte para combos flexibles en SelectorProductos
- Dividir cuenta entre comensales

## v2.3.53 — 2026-05-02
**Ticket de cierre de caja: Resumido vs Detallado** (ahorra papel)
- Al imprimir el cierre se pregunta si se quiere ticket Resumido (sin lista de ventas) o Detallado (con cada venta).
- El Resumido queda en ~10–15 cm de papel; el Detallado mantiene el formato actual con todo el listado.
- Aplica a impresión térmica y PDF.

## v2.3.52 — 2026-05-02
**Hotfix: monto recibido = 0 ahora se asume "exacto"** (UX flujo rápido)
- Cuando el cajero presiona "Cobrar" sin tipear nada en monto recibido, el sistema asume que recibió el monto exacto. Antes salía error de "monto menor al total".
- La validación anti-fraude sigue activa: si el cajero tipea un valor > 0 menor al total y no marca como crédito/mixto, se bloquea con explicación.

## v2.3.51 — 2026-05-02
**Hotfix: detalle de movimientos bancarios con datos completos**
- Corregidas queries SQL que usaban columnas inexistentes (cl.cedula_ruc → cl.identificacion). El error "no such column" al expandir filas en Movimientos Bancarios queda resuelto.
- Pago a proveedor ahora muestra factura número y fecha vía JOIN correcto con tabla compras.

## v2.3.50 — 2026-05-01
**Cierre de auditoría modulo caja/ventas (Med + Low)**
- Anular venta efectivo: nuevo checkbox "¿Devolviste el efectivo al cliente?" para que la caja refleje el caso real (devolución vs error contable).
- Backend valida monto recibido suficiente (anti deuda fantasma).
- Cobros de cuentas por cobrar ya NO inflan `monto_ventas` de la caja.

## v2.3.49 — 2026-05-01
**3 fixes críticos detectados en auditoría**
- Anular una venta que ya tiene Nota de Crédito → BLOQUEADO (antes duplicaba stock).
- Anular venta efectivo ahora revierte `monto_esperado` (antes quedaba "efectivo fantasma").
- Nota de Crédito SRI también escribe en kardex.

## v2.3.48 — 2026-05-01
**Devolución mejorada**
- Devolución ahora registra movimiento en kardex (antes el stock subía pero no se veía en Inventario).
- Nueva opción "Stock" por item: marcar si el cliente devuelve el producto físicamente. Desmarcar si solo se devuelve dinero (compensación, dañado, descuento).

## v2.3.47 — 2026-05-01
**Gastos con trazabilidad**
- Lista de gastos muestra ahora la sesión de caja (`#N` con icono 🟢/🔒 según abierta/cerrada) y el usuario que lo registró.
- Botón eliminar deshabilitado visualmente para gastos de cajas cerradas.

## v2.3.46 — 2026-05-01
**+ Ingreso a Caja** (admin)
- Nuevo botón "+ Ingreso a Caja" en CajaPage para registrar entradas manuales (compensaciones, ajustes, aporte de socio, devolución de gasto erróneo de caja anterior).
- Solo admin. Motivo obligatorio. Suma al monto esperado.

## v2.3.45 — 2026-05-01
**Anti-fraude: gastos de cajas cerradas inmutables**
- No se puede eliminar un gasto cuya caja ya fue cerrada. Mensaje explica que para corregir hay que registrar un ingreso de compensación en la caja actual.

## v2.3.44 — 2026-05-01
**Fix descuadre fantasma por gastos**
- Los gastos ahora actualizan correctamente el `monto_esperado` en tiempo real (antes solo lo restaban en el cálculo recalculado, generando descuadre falso al cerrar).
- `cerrar_caja` ahora SIEMPRE usa el valor recalculado (única fuente de verdad).

## v2.3.43 — 2026-04-30
**Vehículos y direcciones de cliente con autocompletar**
- Modal Guía de Remisión: dropdown con placas y choferes guardados de uso anterior.
- Cliente identificado: dropdown con sus direcciones de entrega previas + opción de agregar nueva (se guarda automáticamente).

## v2.3.42 — 2026-04-30
**Editar precios al facturar**
- Modal "Facturar" permite editar precio unitario y descuento por item al convertir guía → venta.
- Si la guía está PENDIENTE, también permite editar cantidad (con ajuste de stock automático).
- Si está ENTREGADA, cantidad bloqueada (ya fue al cliente).

## v2.3.41 — 2026-04-30
**Hard-block: guía nunca al carrito**
- Si por algún flujo se intenta cargar una guía al carrito de POS, se bloquea con toast de error. Previene el doble descuento de stock.

## v2.3.40 — 2026-04-30
**Documentos Recientes: botón Facturar con modal completo**
- En el panel "Documentos Recientes" del POS, cambiar "Convertir" por "💰 Facturar" con modal completo (forma de pago, banco, referencia).
- Antes "Convertir" cargaba al carrito y al cobrar duplicaba stock.

## v2.3.39 — 2026-04-30
**Fix: Guías mostraban Consumidor Final aunque tenían cliente real**
- Query de listado de guías ahora hace JOIN con clientes para retornar el nombre. Antes el frontend caía a "Consumidor Final" por fallback.

## v2.3.38 — 2026-04-30
**UX: alerta de descuadre solo aparece tras ingresar monto**
- La alerta roja "Descuadre" en el cierre de caja ya no aparece por defecto (cuando el campo monto está vacío). Solo cuando el cajero ingresa un valor que difiere del esperado.

## v2.3.37 — 2026-04-30
**Hotfix: comando movimientos bancarios no registrado en lib.rs**
- Corregido error "Command obtener_detalle_movimiento_bancario not found" al expandir filas.

## v2.3.36 — 2026-04-30
**FIX BUG GRAVE: doble descuento de stock guía → factura**
- `convertir_guia_a_venta` refactorizado: ahora crea NUEVA venta vinculada a la guía SIN volver a descontar stock.
- Acepta guías PENDIENTE o ENTREGADA (antes solo PENDIENTE → cajero terminaba creando venta nueva en POS, duplicando stock).
- Guía origen queda con estado FACTURADA.
- Nueva pestaña "Facturadas" en Guías de Remisión.

## v2.3.35 — 2026-04-30
**Devolución descuenta caja automáticamente con mensaje claro**
- Al hacer una devolución, la caja se actualiza según forma de pago original:
  - EFECTIVO → registra retiro automático "Devolución NC X — efectivo a cliente"
  - TRANSFER → mensaje "haz transfer inversa al cliente desde tu app del banco"
  - CRÉDITO → reduce el saldo (no devuelve dinero)
  - MIXTO → proporcional según componentes

## v2.3.34 — 2026-04-30
**Ventas vinculadas a sesión de caja**
- Cada venta ahora se vincula a la caja en la que se hizo (columna `caja_id`).
- VentasDia muestra "Sesión de caja: #N" en el detalle.
- Nuevo filtro "Solo sesión #N" para ver solo ventas del turno actual.
- Banner explicativo: "Esta pantalla muestra todas las ventas del día sin importar las sesiones de caja".

## v2.3.33 — 2026-04-30
**Movimientos Bancarios expandibles + verificación de transferencias**
- Click en cada fila para ver detalle del documento (cliente, items, comprobante).
- Filtro por tipo (Ventas / Retiros / Pagos / Cobros).
- Nuevo flujo: transferencias se marcan como "Por verificar" (cajero) o "Verificada" (admin), trazables.
- Admin puede aprobar o rechazar transferencias desde la fila expandida.

## v2.3.32 — 2026-04-30
**Resumen post-cierre con depósitos visibles + auto-refresh**
- Card "Resumen de Cierre de Caja" ahora muestra los depósitos a banco hechos después del cierre, con auto-refresh cada vez que se registra uno.
- Calcula "Efectivo restante en caja" en tiempo real.

## v2.3.31 — 2026-04-30
**Auto-refresh CajaPage + comprobante en pago mixto**
- Listener focus + visibility: la caja se recarga automáticamente al volver a la pestaña/ventana.
- Modal "Agregar pago mixto" para TRANSFER ahora permite subir comprobante (igual que el flujo simple).
- Migración `pagos_venta` para incluir `comprobante_imagen`.

## v2.3.30 — 2026-04-30
**Reportes detallados + fix monto esperado MIXTO + UI comprobante**
- Cierre de caja con desglose completo: monto inicial, ventas EFECTIVO, cobros, gastos, retiros, otras formas de pago.
- Fix: ventas mixtas ahora aportan correctamente solo su porción EFECTIVO al monto esperado (antes inflaban con el total).
- Comprobante de transferencia visible y descargable desde VentasDia.

---

## Versiones anteriores (resumen)

### v2.3.x previas (abril 2026)
- v2.3.27 — Botón "Ajustar caja a $0" para admin con descuadre arrastrado.
- v2.3.26 — Migración limpia de retiros viejos del demo al iniciar.
- v2.3.25 — Demo balanceado + validación de gastos contra disponible.
- v2.3.21 — 3 bugs críticos del cajero (descuento sin permiso, fiados invisibles, cierre auto-logout).
- v2.3.20 — SRI siempre visible + reimprimir reportes en descuadres.
- v2.3.19 — Listas de precios en modal del item con permiso.
- v2.3.16 — POS limpio: lista de precios y precio dentro del modal del item.

### v2.x mayores (marzo 2026)
- v2.3.0 — Caja anti-fraude (PIN supervisor, depósito, auditoría completa).
- v2.2.x — Multi-POS en red, multi-almacén, backup cloud.

### v1.x (febrero 2026 e inicios)
- v1.8.x — Tooltips flotantes, demo data ampliada, ergonomía POS.
- v0.5.x — Info adicional por item, transferencia bancaria con referencia, etiquetas de productos, lista oferta limitada.
- v0.4.x — Módulos de licencia (multi-POS, multi-almacén, backup), Google Drive backup OAuth2.
- v0.3.x — Dashboard con widgets, gráficas Recharts.
- v0.2.x — RIDE PDF facturación electrónica SRI.
- v0.1.x — Multi-POS en red (Fase 1-5), licencias online.

---

## Convenciones de versionado

- **Major.Minor.Patch-beta** mientras está en testing con clientes piloto.
- Cada release publica binarios firmados en GitHub Releases.
- Promoción a estable se hace desde admin.clouget.com (canal stable de auto-actualizador).

## Cómo actualizar

La app se auto-actualiza al canal estable cuando hay una nueva versión promovida desde admin. Para forzar manualmente: descargar el `setup.exe` desde la página de releases y ejecutarlo.
