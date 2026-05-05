# CHANGELOG — Clouget Punto de Venta

Historial de mejoras, correcciones y nuevas funcionalidades. Cada entrada incluye fecha y versión publicada en GitHub Releases.

Repositorio: https://github.com/tecnomade/clouget-pos/releases

---

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
