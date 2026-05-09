# CHANGELOG — Clouget Punto de Venta

Historial de mejoras, correcciones y nuevas funcionalidades. Cada entrada incluye fecha y versión publicada en GitHub Releases.

Repositorio: https://github.com/tecnomade/clouget-pos/releases

---

## v2.4.10 — 2026-05-09 🌲 STABLE
**ST-2.5 / 5 — Cascada tipo→marca→modelo en form de orden con + agregar inline.**

Completa la integración del catálogo en el flujo de creación/edición de órdenes. Sin necesidad de salir del form para configurar el catálogo.

### 🆕 Lo que entrega

#### Form de orden con selectores cascada inteligentes

3 nuevos campos que reemplazan los inputs de texto libre:

- **Tipo de equipo** — autocomplete del catálogo (`st_tipos_equipo`). Si hay tipos, los muestra con su emoji (`🚗 Vehículo`, `💻 Computadora`)
- **Marca** — autocomplete filtrado por el tipo seleccionado. Vacío si no se eligió tipo
- **Modelo** — autocomplete filtrado por la marca. Muestra años si están definidos: `Hilux (2018–2022)`

Cada uno con un botón **"+ Agregar al catálogo"** que aparece automáticamente cuando lo que escribiste no existe — crea la entrada inline y refresca el dropdown sin abrir Configuración.

#### Texto libre sigue funcionando

Si el catálogo está vacío o el user prefiere escribir libre, todo sigue funcionando como antes. Los campos `equipo_marca`, `equipo_modelo`, `tipo_equipo` se siguen guardando como TEXT. Cuando se elige del catálogo, además se guarda el ID (`tipo_equipo_id`, `marca_id`, `modelo_id`) — eso permite filtros del catálogo en el historial.

#### Validación dinámica de campos requeridos

Los campos **Placa**, **Kilometraje**, **Próximo recomendado**, **Serie** ahora se muestran/marcan como requeridos según los flags del tipo seleccionado en el catálogo:

```
Vehículo  → requiere_placa = true   → mostrar placa con *
Vehículo  → requiere_kilometraje = true → mostrar km
Computadora → requiere_serie = true → marcar serie con *
```

Antes era hardcoded a `tipo_equipo === "AUTOMOTRIZ"`. Ahora el admin define las reglas desde Configuración.

#### Indicador visual

El campo muestra un badge `✓ catálogo` verde cuando lo que tenés seleccionado es del catálogo (vs texto libre). Útil para auditoría rápida.

### 🛠 Backend

- `models/orden_servicio.rs` — 3 campos `Option<i64>` nuevos: `tipo_equipo_id`, `marca_id`, `modelo_id`
- `commands/servicio_tecnico.rs` — INSERT y UPDATE actualizados para guardar los 3 IDs
- 3 funciones de lectura (obtener/listar/buscar) actualizadas para devolverlos

### 🎨 Frontend

- `components/ComboCatalogoEquipo.tsx` (NUEVO, ~140 líneas) — combo input genérico con dropdown de sugerencias + botón "+" inline
- `services/api.ts` — tipo `OrdenServicio` con los 3 nuevos campos
- `pages/ServicioTecnicoPage.tsx`:
  - Reemplaza inputs marca/modelo por `<ComboCatalogoEquipo>`
  - Carga `stTipos`, `stMarcas`, `stModelos` en cascada
  - Bloque condicional placa/km basado en flags del tipo (no hardcoded)
  - Botones legacy de tipo solo se muestran como fallback si el catálogo está vacío

### 📦 Archivos tocados

- `src-tauri/src/models/orden_servicio.rs` — 3 campos opcionales
- `src-tauri/src/commands/servicio_tecnico.rs` — INSERT/UPDATE/SELECT actualizados
- `src/services/api.ts` — tipo extendido
- `src/components/ComboCatalogoEquipo.tsx` (NUEVO)
- `src/pages/ServicioTecnicoPage.tsx` — integración cascada + flags dinámicos

---

## v2.4.9 — 2026-05-09 🌳 STABLE
**ST-2 / 5 — Servicio Técnico: catálogo jerárquico equipos→marcas→modelos + historial filtrable.**

### 🆕 Lo que entrega

#### Catálogo jerárquico (botón "⚙ Configuración" en la página de Servicio Técnico)

Vista en árbol expandible de 3 niveles:

```
🚗 Vehículos          (15 órdenes)
   ├ Toyota           (8 órdenes)
   │  ├ Hilux 2020   (3 órd)
   │  ├ Corolla       (5 órd)
   │  └ + Modelo
   ├ Honda            (7 órdenes)
   │  └ ...
   └ + Marca
🏍 Motocicletas       (4 órdenes)
   └ ...
+ Nuevo tipo de equipo
```

- **3 tablas nuevas**: `st_tipos_equipo`, `st_marcas`, `st_modelos`
- Soft-delete (`activo=0`) — preserva referencias en órdenes históricas
- Cada tipo tiene flags: `requiere_placa`, `requiere_kilometraje`, `requiere_serie` (para validar campos del form de orden según el tipo)
- **Seed inicial** automático: Vehículo, Motocicleta, Computadora, Celular, Electrodoméstico, General
- Contador de órdenes asociadas en cada nodo
- Modal anidado para crear/editar tipo con flags de campos requeridos

#### Historial filtrable (botón "📜 Historial")

Modal full-screen con filtros multi-criterio:

| Filtro | Opciones |
|---|---|
| Cliente | búsqueda por nombre o cédula |
| Placa | match parcial |
| Serie | match parcial |
| Tipo / Marca / Modelo | cascada (la marca depende del tipo, el modelo de la marca) |
| Estado | RECIBIDO / DIAGNOSTICO / EN_REPARACION / LISTO / ENTREGADO / CANCELADA |
| Rango de fecha | desde / hasta |

Tabla de resultados con: número, fecha, cliente, equipo (marca/modelo), placa/serie, estado (badge color), monto. Click en fila → abre detalle de la orden directamente.

KPI superior: cantidad de órdenes + suma total $ filtrada.

#### Vinculación con órdenes existentes

Migración automática: agrega columnas opcionales `tipo_equipo_id`, `marca_id`, `modelo_id` a `ordenes_servicio`. Cuando el user use el catálogo en lugar de texto libre (ST-2.5 próximo), se guardan los IDs para mejor filtrado/historial.

### 🆕 14 comandos Tauri nuevos

```
st_listar_tipos_equipo / st_crear / st_actualizar / st_eliminar
st_listar_marcas / st_crear / st_actualizar / st_eliminar
st_listar_modelos / st_crear / st_actualizar / st_eliminar
st_listar_arbol_completo
st_historial_filtrable
```

Todos validan licencia con `requiere_modulo_servicio_tecnico` antes de ejecutar.

### 🛠 Backend

- `db/schema.rs` — 3 tablas + seed + ALTER `ordenes_servicio` con FKs opcionales
- `commands/servicio_tecnico_catalogo.rs` (NUEVO, ~430 líneas) — 14 comandos
- `commands/mod.rs` — registra el módulo
- `lib.rs` — registra los 14 comandos en invoke_handler

### 🎨 Frontend

- `components/ModalConfigServicioTecnico.tsx` (NUEVO) — vista en árbol expandible con CRUD inline
- `components/ModalHistorialServicioTecnico.tsx` (NUEVO) — filtros + tabla con resumen
- `services/api.ts` — wrappers TS de los 14 comandos + tipos `StTipoEquipo` / `StMarca` / `StModelo` / `StFiltrosHistorial`
- `pages/ServicioTecnicoPage.tsx` — 2 botones nuevos en barra superior: "📜 Historial" + "⚙ Configuración"

### 🔜 Próximos sub-sprints

- **ST-2.5** (próximo, v2.4.10): cascada tipo→marca→modelo en el form de orden con botón "+" para agregar inline sin abrir Configuración
- **ST-3** (v2.4.11): consultar SRI por ced/RUC desde el form de orden (mismo `consultar_identificacion` del POS)
- **ST-4** (v2.4.12): PDF A4 + Ticket 80mm con detección virtual/térmica
- **ST-5** (v2.4.13): abonos con holding en caja + botón cancelar orden + devolución + reportes

### 📦 Archivos tocados

- `src-tauri/src/db/schema.rs` — 3 tablas + seed + ALTER
- `src-tauri/src/commands/servicio_tecnico_catalogo.rs` (NUEVO)
- `src-tauri/src/commands/mod.rs` — declara módulo
- `src-tauri/src/lib.rs` — 14 comandos en invoke_handler
- `src/services/api.ts` — wrappers + tipos
- `src/components/ModalConfigServicioTecnico.tsx` (NUEVO, ~280 líneas)
- `src/components/ModalHistorialServicioTecnico.tsx` (NUEVO, ~200 líneas)
- `src/pages/ServicioTecnicoPage.tsx` — 2 botones + 2 modales

---

## v2.4.8 — 2026-05-09 🔧 STABLE
**ST-1 / 5 — Servicio Técnico ahora es módulo de licencia separado.**

Inicia el plan de mejora del módulo Servicio Técnico (5 sub-releases). Esta release lo separa de la licencia base como un **módulo opcional con costo adicional** (sugerido $150 setup + $5/mo).

### 🔄 Lo que cambia

- **Antes**: Servicio Técnico venía incluido en la licencia base
- **Ahora**: requiere `servicio_tecnico` en `licencia.modulos` para verse y usarse

### ✨ Auto-migración para clientes existentes

Si el cliente ya tiene órdenes de servicio creadas (`COUNT(*) FROM ordenes_servicio > 0`), al actualizar a v2.4.8 el POS **agrega automáticamente** `servicio_tecnico` a la licencia local. Así no se rompe a nadie. Idempotente.

```rust
[Migration v2.4.8] Modulo 'servicio_tecnico' agregado automaticamente a la licencia local (X ordenes preexistentes detectadas)
```

### 🛠 Backend

- `branding::tiene_modulo_servicio_tecnico()` (transversal Clouget+DigitalServer)
- `requiere_modulo_servicio_tecnico(&db)` agregado al inicio de **los 13 comandos** del módulo
- Auto-migración local en `lib.rs::run()`
- Demo ya incluía `servicio_tecnico` (no requirió cambio)

### 🎨 Frontend

- Sidebar oculta link "Servicio Técnico" si licencia no lo incluye (mismo patrón que Restaurante/App Móvil)
- Acepta tanto `licencia.modulos.includes('servicio_tecnico')` como el flag legacy `config.modulo_servicio_tecnico = "1"` para compatibilidad

### 🔐 Permisos reorganizados

Categoría nueva **`SERVICIO_TECNICO`** en Configuración → Usuarios → Permisos:

- `gestionar_servicio_tecnico` (movido de CORE) — todas las órdenes
- `ver_servicio_tecnico` (movido de CORE) — sólo asignadas
- `config_servicio_tecnico` (NUEVO) — configurar tipos/marcas/modelos (ST-2)
- `recibir_abonos_st` (NUEVO) — abonos en órdenes (ST-5)
- `retirar_holdings_caja` (NUEVO) — retirar dinero de holdings (ST-5)
- `cancelar_orden_servicio` (NUEVO) — cancelar órdenes (ST-5)

Los permisos sólo aparecen si la licencia tiene el módulo (filtrado automático por categoría).

### 🛍 Admin: checkbox "🔧 Servicio Técnico"

En crear/editar licencia (`admin.clouget.com`), nuevo checkbox al lado de los de Restaurante y App Móvil. Marcar/desmarcar para activar/desactivar el módulo.

### 🔜 Próximos sub-sprints

- **v2.4.9 — ST-2**: árbol jerárquico tipos→marcas→modelos + historial filtrable + agregar rápido
- **v2.4.10 — ST-3**: búsqueda cliente con SRI por ced/RUC desde la orden
- **v2.4.11 — ST-4**: PDF orden formato A4 + Ticket 80mm (con detección virtual/térmica)
- **v2.4.12 — ST-5**: abonos con holding en caja + botón cancelar orden + devolución + reportes

### 📦 Archivos tocados

- `src-tauri/src/branding.rs` — `tiene_modulo_servicio_tecnico()`
- `src-tauri/src/commands/servicio_tecnico.rs` — helper + 13 funciones validan licencia
- `src-tauri/src/lib.rs` — auto-migración para clientes con órdenes preexistentes
- `src-tauri/src/models/usuario.rs` — categoría `CAT_SERVICIO_TECNICO` + 4 permisos nuevos, 2 movidos
- `src/components/Layout.tsx` — sidebar lee `licencia_modulos.includes('servicio_tecnico')`
- `clouget-admin/src/index.html` — checkbox en crear/editar licencia

---

## v2.4.7 — 2026-05-08 🔧 STABLE
**Hotfix crítico: cobro de orden de servicio técnico con items con IVA — total mal calculado, ticket mostraba "solo el IVA".**

### 🐞 Síntoma reportado

Flujo: orden de servicio técnico → click "Cobrar" → agregar 2 items con IVA → cobrar con monto > total → imprimir desde Ventas. **El ticket impreso mostraba solo el IVA en el detalle**, sin la base de los items.

### 🔍 Causa raíz

En `cobrar_orden_servicio` (commands/servicio_tecnico.rs):

1. **Bug de cálculo**: cuando un item tenía IVA > 0%, **solo se sumaba el IVA al total** — la base del item NUNCA se acumulaba en ningún subtotal:

```rust
// ❌ ANTES
if iva_porc > 0.0 {
    iva_total += sub * (iva_porc / 100.0);   // ← solo agrega EL IVA
} else {
    subtotal_sin_iva += sub;
}
// La BASE del item con IVA se perdía → total = (servicio + items 0%) + IVA
```

2. **Bug de persistencia**: el `INSERT INTO ventas` guardaba `subtotal_con_iva = 0` hardcoded — perdiendo la base de los items con IVA en la DB.

Por eso el ticket impreso mostraba `Subtotal IVA: 0.00` y solo aparecía la línea del IVA — porque ESO era lo único que se había acumulado correctamente.

> ¿Por qué solo aparecía en algunos PCs y no en otros? Porque depende del flujo: si cobrás orden sin items o con items SIN IVA, el bug no aparece. Solo se manifiesta con items que tengan `iva_porcentaje > 0`.

### ✅ Fix

```rust
// ✅ AHORA
let mut subtotal_sin_iva: f64 = 0.0;   // base 0% + monto del servicio
let mut subtotal_con_iva: f64 = 0.0;   // base de items con IVA
let mut iva_total: f64 = 0.0;          // IVA acumulado

if monto_final > 0.0 {
    subtotal_sin_iva += monto_final;
}
for item in &items_repuestos {
    let sub = cant * precio;
    if iva_porc > 0.0 {
        subtotal_con_iva += sub;       // ← antes faltaba esta línea
        iva_total += sub * (iva_porc / 100.0);
    } else {
        subtotal_sin_iva += sub;
    }
}
let total = subtotal_sin_iva + subtotal_con_iva + iva_total;
```

Y el INSERT ahora guarda los 3 valores correctamente.

### Impacto

- **Ventas anteriores ya guardadas con el bug NO se corrigen automáticamente** — quedan en la DB con `subtotal_con_iva = 0` y total potencialmente erróneo. Si afectó a contabilidad, hay que corregirlas manualmente o anular y re-cobrar.
- **Cobros desde el POS normal NO están afectados** — el bug es exclusivo de `cobrar_orden_servicio`, que usa una lógica de cálculo propia distinta del flujo principal.

### 📦 Archivos tocados

- `src-tauri/src/commands/servicio_tecnico.rs` — fix `cobrar_orden_servicio` (~30 líneas refactor)

---

## v2.4.6 — 2026-05-08 📲 STABLE
**Endpoint `/auth/usuarios-disponibles` para selector de login en la app móvil.**

Esta release acompaña el lanzamiento de **`clouget-pos-app` v0.1** (repo aparte) — app Expo/React Native que ya consume todos los endpoints HTTP que veníamos construyendo (Sprints 3a/3b/3c).

### 🆕 Nuevo endpoint

`GET /api/v1/app/auth/usuarios-disponibles` (sin auth) — devuelve la lista de usuarios activos con permisos de app, para que la pantalla de login muestre **avatares con nombre** (UX mucho mejor que escribir un ID a ciegas).

Filtra a:
- Usuarios `ADMIN`, o
- Usuarios con al menos uno de: `atiende_mesas`, `ve_cocina`, `vende_piso`, `inventaria`, `dueno_dashboard`, `cobra_caja`

Solo expone `{ id, nombre, rol, es_admin }` — NO devuelve permisos (la app los recibe al hacer login con PIN).

### 📲 App móvil v0.1 publicada

Repo: `C:\proyectos\clouget-pos-app` (Expo + React Native + TypeScript). Soporta:

- ✅ **Buscar sucursal**: escanear QR o IP/puerto manual con ping de validación
- ✅ **Login PIN**: lista usuarios disponibles con avatares iniciales coloreados, teclado numérico de 6 dígitos custom
- ✅ **Tabs adaptables** según permisos: Inicio, Mesas (atiende_mesas), Cocina (ve_cocina), Vender (placeholder), Más
- ✅ **Mesas**: grid colorido con filtro por zona, estados (libre/ocupada/cuenta/unida), modal abrir pedido
- ✅ **Pedido detalle**: items agrupados, agregar via selector con búsqueda en vivo, enviar cocina, pedir cuenta, cobrar (modal forma de pago: efectivo/transfer/crédito), cancelar
- ✅ **Cocina**: comandas agrupadas por mesa con timer, marcar EN_PREPARACION → LISTO → ENTREGADO

Próximas versiones:
- v0.2 (Sprint 6): cocina responsive tablet, push notifications, dividir cuenta + unir mesas
- v0.3 (Sprint 7): vendedor de piso completo, inventarista, dashboard remoto

### 📦 Archivos tocados

- `src-tauri/src/app_movil/http.rs` — handler `auth_usuarios_disponibles` + ruta registrada

---

## v2.4.5 — 2026-05-08 🍳 STABLE
**Hotfix: Comanda de cocina ahora hereda configuración de impresión (PDF si virtual, ESC/POS si térmica).**

### 🛠 Bug fix

**Síntoma**: La comanda de cocina (al enviar pedido a cocina o al re-imprimir) siempre intentaba mandar bytes ESC/POS directos a la impresora `impresora_cocina` o `impresora` configurada. Si esa impresora era una **virtual** (Microsoft Print to PDF, OneNote, XPS, Fax) los bytes ESC/POS salían como basura ilegible. Si NO había impresora configurada, daba error en lugar de generar PDF.

**Causa**: el handler `rest_imprimir_comanda_cocina` no usaba el helper `impresora_es_virtual()` que la pre-cuenta sí usa. Faltaba paridad de comportamiento entre los 2 tickets de restaurante.

**Fix**: ahora la comanda sigue **exactamente** el mismo flujo que la pre-cuenta:
- 🖨 **Impresora térmica real** (POS-58, Epson TM, etc.) → bytes ESC/POS directos (formato 80mm con doble alto y emojis)
- 📄 **Impresora virtual** (Microsoft Print to PDF, OneNote, XPS, Fax) → genera PDF nativo legible y lo abre con el visor del sistema
- 📄 **Sin impresora configurada** → genera PDF y lo abre (antes: error)

### Implementación

- Nueva función `generar_comanda_cocina_pdf()` en `restaurante/printing.rs` (180 líneas) — equivalente PDF de `generar_comanda_cocina()` (que genera ESC/POS). Usa el mismo `genpdf` que la pre-cuenta, formato 80mm, fonts mesa GRANDE para leer desde lejos.
- `rest_imprimir_comanda_cocina` refactorizado: helper closure interno `imprimir_o_pdf` que decide ESC/POS vs PDF según la impresora. Aplica a los 3 caminos (modo separado cocina, modo separado barra, modo combinado ambos).
- Nombres de archivo PDF generado: `Comanda-🍳 Cocina-Mesa{X}-Ped{ID}.pdf` / `Comanda-🍷 Barra-...` / `Comanda-🍽 Comanda-...`

### 📦 Archivos tocados

- `src-tauri/src/restaurante/printing.rs` — nueva `generar_comanda_cocina_pdf` (~180 líneas)
- `src-tauri/src/restaurante/commands.rs` — refactor `rest_imprimir_comanda_cocina` con helper `imprimir_o_pdf`

---

## v2.4.4 — 2026-05-08 📷 STABLE
**Sprint 3c / 7 — mDNS broadcast + QR de emparejamiento + hotfix reporte ventas.**

Cierra la **Fase 3 del backend HTTP**. Con esta release, la app móvil (Sprint 5) puede encontrar el servidor de 3 maneras:

1. 🔍 **Auto-descubrimiento mDNS**: la app escanea la red y aparecen los POS de Clouget instantáneamente (servicio `_clouget-pos._tcp.local.`)
2. 📷 **Código QR**: el admin genera un QR desde Configuración → 📱 App Móvil, la app lo escanea con la cámara y queda configurada en 1 segundo
3. ⌨️ **Configuración manual** (alternativa): IP + puerto a mano

### 🆕 Sprint 3c

**Discovery mDNS automático** (`app_movil/discovery.rs`):
- El servidor se anuncia como `_clouget-pos._tcp.local.` con propiedades TXT (`negocio`, `version`, `restaurante`, `app_movil`, `api`)
- Hostname mDNS estable: `clouget-pos-<nombre-negocio>.local.`
- Se inicia automáticamente al arrancar el server HTTP (solo si la licencia tiene `app_movil`)
- Si la red no soporta mDNS (multicast bloqueado), no falla — la app cae al QR/manual

**QR de emparejamiento** (`app_generar_qr_emparejamiento`):
- Botón "📷 Generar código QR" en Configuración → App Móvil
- Modal muestra el QR (280×280 PNG) + datos visibles: IP, puerto, negocio, módulo restaurante
- El QR contiene JSON: `{ service, ip, port, negocio, restaurante, version }`
- **No incluye credenciales** (el PIN se pide después): si alguien fotografía el QR no puede loguearse
- El QR se puede regenerar las veces que quiera, no expira

**Auto-arranque del servidor HTTP**:
- Antes: el server solo arrancaba si `modo_red == "servidor"` (Multi-POS) y había token configurado
- Ahora: arranca también si la licencia tiene `app_movil` (sin token Multi-POS)
- En este caso `/api/v1/invoke` (Multi-POS) NO se monta — solo `/api/v1/app/*` (app móvil)
- Backward-compatible al 100% con instalaciones Multi-POS existentes

### 🛠 Hotfix incluido

**Reporte "Ventas detalladas" fallaba con `no such column: c.razon_social`**

La query usaba `COALESCE(c.razon_social, c.nombres, '')` pero la tabla `clientes` real solo tiene la columna `nombre` (singular). Era código heredado de una refactorización en clientes que nunca se aplicó.

Fix: `COALESCE(c.nombre, '') as cliente_nombre`. Sin esto el reporte fallaba al hacer click en "Aplicar" (apareció en producción).

### 🔜 Próximas fases

- **Sprint 4**: Admin panel — precios editables para los 4 combos de licencia
- **Sprint 5**: `clouget-pos-app` v0.1 (repo nuevo, Expo/React Native) — login PIN + mesas + pedido
- **Sprint 6**: App v0.2 — cocina responsive + push notifications + dividir/unir mesas
- **Sprint 7**: App v0.3 — vendedor de piso + inventarista + dashboard remoto

### 📦 Archivos tocados

- `src-tauri/Cargo.toml` — deps `mdns-sd = "0.11"`, `local-ip-address = "0.6"`
- `src-tauri/src/app_movil/discovery.rs` — mDNS broadcaster + helper IP local (NUEVO)
- `src-tauri/src/app_movil/commands.rs` — `app_generar_qr_emparejamiento` con `QrCode::to_colors()` + bitmap manual
- `src-tauri/src/app_movil/mod.rs` — declara submódulo discovery
- `src-tauri/src/lib.rs` — server arranca también con `app_movil`, lanza mDNS broadcast
- `src-tauri/src/server/mod.rs` — `/api/v1/invoke` solo se monta con token configurado
- `src-tauri/src/commands/reportes.rs` — fix columna `c.nombre` (era `razon_social/nombres`)
- `src/services/api.ts` — wrapper `appGenerarQrEmparejamiento` + tipo `QrEmparejamiento`
- `src/pages/Configuracion.tsx` — botón "📷 Generar código QR" + modal con la imagen

---

## v2.4.3 — 2026-05-07 🍽 STABLE
**Sprint 3b / 7 — Endpoints HTTP completos: pedidos, cocina, cobrar, dividir, unir mesas, vendedor piso.**

Esta release agrega los **19 endpoints HTTP que faltaban** para que la app móvil (próximo Sprint 5) pueda operar todo el flujo de mesero, cocinero, vendedor de piso y dividir/unir mesas. Junto con v2.4.2, el backend HTTP queda **funcionalmente completo** para la app v0.1.

### 🆕 Endpoints agregados (19 nuevos)

#### Pedidos (mesero)
| Método | Ruta | Permiso |
|---|---|---|
| POST | `/pedidos/abrir` | atiende_mesas |
| GET | `/pedidos/:id` | atiende_mesas o ve_cocina |
| GET | `/pedidos/mesa/:mesa_id` | atiende_mesas o ve_cocina |
| POST | `/pedidos/:id/items` | atiende_mesas |
| DELETE | `/pedidos/items/:item_id` | atiende_mesas |
| POST | `/pedidos/:id/enviar-cocina` | atiende_mesas |
| POST | `/pedidos/:id/pedir-cuenta` | atiende_mesas |
| POST | `/pedidos/:id/cancelar` | cancela_pedido |
| POST | `/pedidos/:id/cobrar` | cobra_caja |

El endpoint `cobrar` es un **combo atómico**: orquesta `registrar_venta` (vía dispatcher, reusando toda la lógica del POS desktop incluyendo SRI, secuenciales, kardex, banco/referencia) + `UPDATE rest_pedidos_abiertos SET estado='COBRADO'` que libera la mesa principal y todas las mesas extra automáticamente.

#### Unir mesas (grupos grandes)
| Método | Ruta | Permiso |
|---|---|---|
| POST | `/pedidos/:id/unir-mesas` | une_mesas |
| DELETE | `/pedidos/:pedido_id/mesas-extra/:mesa_id` | une_mesas |
| GET | `/pedidos/:id/mesas-libres-para-unir` | une_mesas |

Validación transaccional: si alguna mesa del lote falla, ninguna se une (mismo comportamiento que v2.3.68 desktop).

#### Dividir cuenta
| Método | Ruta | Permiso |
|---|---|---|
| POST | `/pedidos/:id/dividir` | divide_cuenta |
| GET | `/pedidos/:id/subcuentas` | (token) |
| POST | `/pedidos/:id/cancelar-division` | divide_cuenta |
| POST | `/subcuentas/:id/cobrar` | cobra_caja |

`/subcuentas/:id/cobrar` registra una venta al producto especial `_DIVISION_CUENTA_` por el monto de la sub-cuenta, marca la sub-cuenta como COBRADA, y si todas quedaron pagas cierra el pedido y libera mesas. Idéntico flujo a v2.3.69 desktop.

#### Cocina (cocinero)
| Método | Ruta | Permiso |
|---|---|---|
| GET | `/cocina/items` | ve_cocina |
| POST | `/cocina/items/:id/estado` | ve_cocina |

Body de `estado`: `{ estado: "PENDIENTE" \| "EN_PREPARACION" \| "LISTO" \| "ENTREGADO" }`. Con esto el cocinero en tablet/teléfono ve la lista en tiempo real (con minutos transcurridos) y marca cuando está listo.

#### Vendedor de piso (POS sin mesa)
| Método | Ruta | Permiso |
|---|---|---|
| POST | `/ventas` | vende_piso o cobra_caja |

Acepta el mismo payload que `registrar_venta` desktop. Útil para vendedor caminando con tablet o cobro inalámbrico — el item se vende desde el catálogo y la venta entra a la caja activa del POS.

### 🛠 Cambios técnicos

- 3 helpers internos del módulo restaurante refactorizados a `pub(crate)` para reuso desde HTTP:
  - `obtener_pedido_detalle(conn, pedido_id)`
  - `listar_mesas_con_estado_internal(conn)`
  - `listar_subcuentas_internal(conn, pedido_id)`
- `app_movil/http.rs` crece de ~440 a ~1100 líneas con los 19 handlers nuevos
- Cada handler valida en orden: licencia `app_movil` → token → permiso específico → módulo `restaurante` cuando aplica
- Para registrar venta (cobrar pedido / cobrar sub-cuenta / venta vendedor piso), reusa `dispatch_command("registrar_venta")` para no duplicar la lógica gigante (SRI, secuenciales, kardex, multi-almacén)
- Reparto de centavos en `dividir` mantiene precisión exacta (residuo a la última parte)

### 🔜 Próximas sub-fases

- **v2.4.4** (Sprint 3c): mDNS broadcast (`_clouget-pos._tcp.local.`) + comando para generar **QR de emparejamiento** que la app puede escanear para auto-configurar el servidor
- **Sprint 4**: Admin panel — precios editables para los 4 combos de licencia
- **Sprint 5**: `clouget-pos-app` v0.1 (repo nuevo) consume todo este backend

### 📦 Archivos tocados

- `src-tauri/src/restaurante/commands.rs` — 3 helpers a `pub(crate)`
- `src-tauri/src/app_movil/http.rs` — 19 handlers nuevos + 22 rutas registradas (~660 líneas agregadas)

---

## v2.4.2 — 2026-05-07 🌐 STABLE
**Sprint 3a / 7 — Backend HTTP completo para la app móvil + 2 hotfixes imagen.**

### 🛠 Hotfixes incluidos

**Hotfix 1 — Drag & drop de imagen no funcionaba en Tauri**

En Tauri el webview captura los eventos drag&drop a nivel SO y NO los entrega a React (`onDragOver`/`onDrop` no se disparan). Por eso solo el Ctrl+V (paste) funcionaba.

Fix: usar la API `getCurrentWebview().onDragDropEvent()` de Tauri 2 que entrega el path absoluto del archivo soltado. Detectamos si el cursor está sobre el cuadro de imagen comparando coordenadas con el `boundingRect` del container.

**Hotfix 2 — Imágenes >500KB ahora se aceptan y reducen automáticamente**

Antes: imagen > 500 KB era rechazada con error.
Ahora: acepta hasta **5 MB de input** y el backend optimiza:
1. Decodifica con `image` crate (PNG, JPG, GIF, BMP, WebP, etc.)
2. Si lado mayor > 1024 px → redimensiona con Lanczos3 (mantiene aspect ratio)
3. Re-encode como JPEG con calidad descendente (85→75→65→50→35) hasta entrar en 500 KB
4. Si tras todo eso no entra (improbable con 1024px JPEG q=35), error

Funciona en los 3 caminos: Cargar archivo, Pegar (Ctrl+V), Drag & drop. Formatos exóticos (SVG, HEIC) que `image` no decodifica siguen requiriendo entrada <500 KB (raros que excedan).



Esta release implementa toda la base que la app móvil (`clouget-pos-app`, repo aparte, próximo Sprint 5) necesita para hablar con el POS escritorio: auth con PIN, middleware de autorización por permisos, endpoints REST y panel de administración de dispositivos.

### 🆕 Lo que entrega

#### 1. **Auth con PIN** (`POST /api/v1/app/auth/pin`)
La app envía `{ usuario_id, pin, dispositivo_nombre, dispositivo_modelo, dispositivo_so }` y recibe un **token UUID v4 único por dispositivo**. El servidor valida:
- PIN contra `usuarios.pin_hash` (mismo hash que el login local)
- Que el usuario esté activo
- Que tenga **al menos un permiso de app** (`atiende_mesas`, `ve_cocina`, `vende_piso`, `inventaria`, `dueno_dashboard`, `cobra_caja`) o sea ADMIN

El token se persiste en la nueva tabla `app_tokens` con timestamp, dispositivo y push token (para Sprint 6).

#### 2. **Middleware de autorización**
`extract_app_session(headers, state)` valida el token en cada request, carga los permisos del usuario y bloquea automáticamente si la licencia no tiene `app_movil`. Helpers en handlers:
```rust
session.tiene("atiende_mesas")        // bool
session.requiere("divide_cuenta")?    // -> 403 si no tiene
```

#### 3. **6 endpoints REST funcionales**

| Método | Ruta | Auth | Permiso | Qué hace |
|---|---|---|---|---|
| GET | `/api/v1/app/ping` | No | — | Conectividad + nombre negocio + módulos |
| POST | `/api/v1/app/auth/pin` | No | — | Login PIN, devuelve token |
| POST | `/api/v1/app/auth/logout` | Token | — | Revoca el token actual |
| GET | `/api/v1/app/me` | Token | — | Usuario + permisos + módulos licencia |
| GET | `/api/v1/app/productos` | Token | — | Catálogo (con `?q=` opcional) |
| GET | `/api/v1/app/mesas` | Token | atiende_mesas o ve_cocina | Grid mesas (reusa lógica del POS) |

CORS habilitado (`Any`) — la app puede correr en cualquier origen y la auth la garantiza el token.

#### 4. **Panel de administración de dispositivos**
En **Configuración → 📱 App Móvil** ahora aparece:
- Lista de dispositivos emparejados (activos primero, revocados después)
- Por cada dispositivo: nombre, modelo, SO, último uso ("hace 5 min"), creado en
- Botón **Revocar** (marca `revoked = 1`, próximo request recibe 401 → app pide login)
- Botón **Eliminar** (borra del registro permanentemente)
- Refresh manual
- Datos de conexión sugeridos (IP local + puerto del servidor)

#### 5. **3 comandos Tauri admin**
- `app_listar_dispositivos()` → lista con JOIN a usuarios
- `app_revocar_dispositivo(id)` → soft revoke
- `app_eliminar_dispositivo(id)` → hard delete

### 🛠 Backend

- Nuevo módulo Rust `app_movil/` con 4 archivos: `mod.rs`, `schema.rs`, `http.rs`, `commands.rs`
- Tabla `app_tokens(id, usuario_id, token, dispositivo_*, push_token, created_at, last_used_at, revoked)` con FK CASCADE a usuarios
- `server/mod.rs` mergea las rutas con `.merge(app_movil::http::rutas())` y agrega `CorsLayer`
- `lib.rs` llama `app_movil::init()` al arranque (gateado por `branding::tiene_modulo_app_movil()`)
- 3 comandos Tauri registrados

### 🎨 Frontend

- `services/api.ts`: tipo `DispositivoApp` + 3 wrappers (`appListarDispositivos`, `appRevocarDispositivo`, `appEliminarDispositivo`)
- `Configuracion.tsx`: nuevo componente `PanelAppMovil` reemplaza el placeholder de v2.4.1

### 🔜 Próximas sub-fases

- **v2.4.3** (Sprint 3b): endpoints completos de pedidos (`POST /pedidos`, items, cocina, cobrar, dividir, unir)
- **v2.4.4** (Sprint 3c): mDNS broadcast + comando para generar QR de emparejamiento

### 📦 Archivos tocados

- `src-tauri/src/app_movil/mod.rs` — declara submódulos + init
- `src-tauri/src/app_movil/schema.rs` — tabla `app_tokens` (NUEVO)
- `src-tauri/src/app_movil/http.rs` — 6 handlers + middleware (NUEVO, ~440 líneas)
- `src-tauri/src/app_movil/commands.rs` — 3 comandos Tauri (NUEVO)
- `src-tauri/src/server/mod.rs` — merge de rutas + CORS
- `src-tauri/src/lib.rs` — init módulo + registro de comandos
- `src/services/api.ts` — wrappers TS
- `src/pages/Configuracion.tsx` — `PanelAppMovil` con lista dispositivos

---

## v2.4.1 — 2026-05-07 📱 STABLE
**Sprint 2 / 7 — Módulo `app_movil` en licencia + 4 hotfixes.**

### 🆕 Sprint 2: Módulo `app_movil` separado de `restaurante`

Hoy hay 8 módulos de licencia: `multi_pos`, `multi_almacen`, `backup_cloud`, `backup_premium`, `servicio_tecnico`, `sri_ilimitado`, `restaurante` y ahora **`app_movil`** (transversal — disponible en marcas Clouget y DigitalServer).

Esto habilita los 4 combos de licencia que se vienen comercializando:

| Módulos | Caso | Próximo precio sugerido |
|---|---|---|
| `[]` | POS básico (perpetua) | $80-120 |
| `["restaurante"]` | Restaurante sin app | actual + $5/mo |
| `["app_movil"]` | POS + app (vendedor piso, inventarista, dueño dashboard) | $5-8/mo |
| `["restaurante", "app_movil"]` | Caso completo (meseros + cocineros + admin) | $10-12/mo |

**Cambios visibles:**
- Nueva sección **📱 App Móvil** en Configuración (visible solo si licencia tiene `app_movil`)
- Lista cuántos usuarios tienen permisos relevantes (atiende_mesas, ve_cocina, vende_piso, inventaria, dueno_dashboard)
- Avisa el estado de la app (en construcción — Sprint 3 entrega los endpoints HTTP, Sprint 5 entrega la app)
- Modo **demo** ahora incluye `app_movil` (todos los módulos activos)

**Backend:**
- `branding::tiene_modulo_app_movil()` (transversal a Clouget y DigitalServer)
- Nuevo módulo Rust `app_movil/mod.rs` con `requiere_modulo_app_movil()` (helper de validación de licencia, base para Sprint 3)
- `commands/demo.rs` y `commands/licencia.rs` agregan `app_movil` a la lista de módulos del demo

### 🛠 Hotfixes incluidos

#### 1. Dashboard "Sin ventas hoy" falso por UTC
**Síntoma**: A partir de las ~7-8pm en Ecuador (UTC-5), el widget "Últimas ventas del día" decía "Sin ventas hoy" aunque la gráfica de 7 días Y el "Top 10 productos del día" mostraran ventas hechas hoy.

**Causa**: `date('now')` en SQLite devuelve UTC, pero las ventas se guardan con `datetime('now', 'localtime')`. Por la noche UTC ya es del día siguiente → no matchea.

**Fix**: usar `date('now', 'localtime')` en `ultimas_ventas_dia` y `resumen_diario_ayer`.

#### 2. Restaurante: auto-limpieza de pedidos vacíos abandonados con desfase horario
**Síntoma menor**: la auto-limpieza diaria de pedidos abandonados (>24h, sin items) en restaurante usaba `julianday('now')` sin localtime → desfase de 5h en Ecuador (no rompía nada visible pero técnicamente incorrecto).

**Fix**: `julianday('now', 'localtime')` para que coincida con `julianday(fecha_apertura)` ya en localtime.

#### 3. Productos: imagen ahora se puede pegar (Ctrl+V), arrastrar (drag&drop) y soporta más formatos
**Antes**: solo PNG/JPG por archivo.

**Ahora**:
- 📋 **Ctrl+V** para pegar imagen del portapapeles (de captura de pantalla, navegador, etc.)
- 🖱️ **Drag & drop** arrastrando archivo desde explorador o navegador
- 🎨 Formatos extra: **WebP, GIF, BMP, AVIF, SVG, ICO, HEIC** además de PNG/JPG
- Detección automática del mime type para mostrar correctamente
- Indicador visual claro: el cuadro se ilumina al arrastrar encima ("📥 Suelta aquí")

**Backend nuevo**: `guardar_imagen_producto_b64(id, base64)` acepta el b64 directo (con o sin prefijo `data:image/xxx;base64,`), valida tamaño 500 KB y persiste.

**Frontend**: extraído a componente reutilizable `ImagenProductoPicker` que centraliza los 3 modos de carga (file picker, paste, drag&drop).

#### 4. Productos: "Eliminar categoría completa" / "Eliminar seleccionados" fallaba con FOREIGN KEY constraint failed
**Síntoma**: al intentar eliminar productos que tenían historial (compras, kardex, combos, series, lotes, multi-precios, multi-almacén, multi-unidad) el DELETE físico fallaba con `FOREIGN KEY constraint failed`. Como el botón hacía un loop, el primer error detenía toda la operación → "ni uno solo se eliminaba".

**Causa**: `eliminar_producto` solo chequeaba referencias en `venta_detalles`. Si no había ventas pero SÍ había compras o kardex, intentaba DELETE directo y se rompía.

**Fix**:
- `eliminar_producto`: cambia a estrategia "intenta DELETE; si falla con FK → soft delete (`activo=0`) liberando códigos para que puedan reusarse"
- `eliminar_categoria` con acción "eliminar productos": ya no usa DELETE masivo, ahora itera con el helper que cae a soft delete cuando es necesario
- `eliminar_categoria`: si la categoría tiene productos soft-deleted que aún apuntan a ella, libera referencias (`SET categoria_id = NULL`) y reintenta el DELETE

### 📦 Archivos tocados

**Sprint 2:**
- `src-tauri/src/branding.rs` — `tiene_modulo_app_movil()`
- `src-tauri/src/app_movil/mod.rs` — módulo nuevo con helper de licencia
- `src-tauri/src/lib.rs` — declaración del módulo
- `src-tauri/src/commands/demo.rs` y `commands/licencia.rs` — `app_movil` en demo
- `src/pages/Configuracion.tsx` — nueva sección "📱 App Móvil"

**Hotfixes:**
- `src-tauri/src/commands/reportes.rs` — fix UTC `ultimas_ventas_dia`, `resumen_diario_ayer`
- `src-tauri/src/restaurante/commands.rs` — fix UTC auto-limpieza
- `src-tauri/src/commands/productos.rs` — `guardar_imagen_producto_b64`, refactor `eliminar_producto` + `eliminar_categoria`
- `src/services/api.ts` — wrapper `guardarImagenProductoB64`
- `src/pages/Productos.tsx` — componente `ImagenProductoPicker` con paste/drag&drop

---

## v2.4.0 — 2026-05-07 🔐 STABLE
**Sprint 1 / 7 — Permisos agrupados por categoría + base para app móvil.**

Inicia el camino hacia la **app móvil** (clouget-pos-app, repo aparte): meseros con PIN, cocineros en tablet, vendedores de piso, inventaristas, dueño con dashboard remoto. Pero esa app necesita primero un sistema de permisos fino — eso es lo que entrega esta release.

### 🔐 Lo que cambia para el usuario

En **Configuración → Usuarios → Permisos**, los checkboxes ahora aparecen agrupados por categoría con un encabezado claro:

```
POS Escritorio                ← siempre visible
  ☐ Editar precio   ☐ Editar IVA  ☐ Aplicar descuentos ...

🍽 Módulo Restaurante          ← solo si licencia tiene `restaurante`
  ☐ Atiende mesas    ☐ Ver pantalla cocina  ☐ Dividir cuenta ...

📱 App Móvil                   ← solo si licencia tiene `app_movil`
  ☐ Vendedor de piso  ☐ Inventarista  ☐ Dueño/Dashboard ...
```

Si la licencia NO tiene módulo restaurante o app_movil, esas categorías **no aparecen** (no se pueden marcar permisos inválidos). Si no tiene ninguno de los dos, aparece un tip sugiriendo activarlos.

### 🆕 Permisos nuevos (categoría RESTAURANTE)

- `atiende_mesas` — abre/edita pedidos en mesas
- `ve_cocina` — pantalla de cocina + marcar items LISTOS
- `imprime_comandas` — reimprimir comandas
- `divide_cuenta` — dividir cuenta en sub-cuentas (v2.3.69)
- `une_mesas` — unir mesas para grupos grandes (v2.3.68)
- `cancela_pedido` — cancelar pedido sin cobrar (libera mesa)
- `config_mesas` — configurar zonas y mesas

### 🆕 Permisos nuevos (categoría APP_MOVIL)

- `vende_piso` — vendedor de piso (toma pedidos en la app y envía a caja)
- `inventaria` — conteo físico de inventario con la app
- `dueno_dashboard` — dueño/admin ve dashboard remoto en la app
- `cobra_caja` — puede cobrar desde la app (vende y cobra él mismo)

> Estos permisos **ya existen en el sistema** pero solo se vuelven útiles cuando la app móvil esté disponible (Sprint 5). Hoy se pueden asignar para preparar usuarios anticipadamente.

### 🔍 Por qué este orden

La app móvil es el destino final (Sprints 5-7), pero antes hace falta:
1. **Sprint 1** (esta release) — permisos finos + categorización ← **estamos aquí**
2. **Sprint 2** — módulo `app_movil` separado en la licencia
3. **Sprint 3** — endpoints HTTP completos del POS escritorio (hoy son stub)
4. **Sprint 4** — admin panel con precios editables para los 4 combos de licencia
5. **Sprint 5-7** — la app en sí (repo aparte `clouget-pos-app`)

### 🛠 Backend

- `models/usuario.rs`: `PERMISOS_DISPONIBLES` ahora es `&[(key, label, categoria)]` con 3 categorías canónicas (`CAT_CORE`, `CAT_RESTAURANTE`, `CAT_APP_MOVIL`)
- 11 permisos nuevos: 7 de restaurante + 4 de app móvil
- `obtener_permisos_disponibles` devuelve `Vec<(String, String, String)>`

### 🎨 Frontend

- `services/api.ts`: tipo de retorno actualizado a `[string, string, string][]`
- `Configuracion.tsx`: render de checkboxes refactorizado para agrupar por categoría con headings y filtrar según `config.licencia_modulos`
- Tip informativo si no tiene módulos extras

### 📦 Archivos tocados

- `src-tauri/src/models/usuario.rs` — categorías + permisos nuevos
- `src-tauri/src/commands/usuarios.rs` — firma del command
- `src/services/api.ts` — wrapper TS
- `src/pages/Configuracion.tsx` — UI agrupada y filtrada

---

## v2.3.70 — 2026-05-07 📊 STABLE
**Reporte de ventas detalladas filtrable con export Excel/PDF.**

Nueva pestaña en `/reportes` que lista cada venta individual del período con filtros multi-criterio. Antes solo había reportes agregados (utilidad, balance, top productos, IVA, CxC, CxP, inventario, kardex, cajeros) — faltaba poder ver el listado plano de ventas para auditoría, conciliación y comprobación de cajeros/categorías.

### 🎯 Caso de uso

- "Quiero ver todas las ventas que hizo Juan en transferencia esta semana"
- "Quiero el detalle de las ventas de la categoría Bebidas en abril para conciliar con bodega"
- "Quiero exportar a Excel todas las facturas del mes para mi contadora"
- "Quiero las ventas anuladas del trimestre"

### 🔍 Filtros disponibles

- **Rango de fecha** (desde/hasta) — heredado del header común de reportes
- **Cajero** — selector con los usuarios que tuvieron ventas en el rango
- **Forma de pago** — EFECTIVO, TRANSFERENCIA, CRÉDITO, etc.
- **Tipo documento** — NOTA_VENTA, FACTURA, NOTA_CREDITO
- **Categoría** — filtra ventas que tengan al menos un item de esa categoría (EXISTS subquery)
- **Incluir anuladas** — checkbox (default OFF)

Los selectores se cargan dinámicamente con valores ÚNICOS que aparecen en el rango (no muestra opciones vacías).

### 📊 KPIs y desglose

Encima de la tabla:
- 5 KPIs: número de ventas, total facturado, ticket promedio, IVA generado, descuentos
- Chips por forma de pago: cada forma con su total y número de ventas

### 📋 Tabla de ventas

11 columnas: fecha, número, cliente (con identificación), cajero, forma de pago, tipo doc, subtotal, IVA, descuento, total y estado. Footer con totales agregados. Las anuladas se muestran con opacidad reducida y badge "ANULADA".

### 📁 Export

Botones Excel (.xlsx) y PDF (apaisado por defecto) reutilizando `exportar_tabla_xlsx` / `exportar_tabla_pdf`. El subtítulo del archivo incluye automáticamente todos los filtros aplicados (período + cajero + forma + tipo + categoría + flag anuladas).

### 🛠 Backend

- `reporte_ventas_filtrable(fecha_desde, fecha_hasta, cajero?, cliente_id?, forma_pago?, tipo_documento?, categoria_id?, incluir_anuladas?)` — construcción dinámica del WHERE con `params_from_iter`
- `reporte_ventas_filtros_disponibles(fecha_desde, fecha_hasta)` — devuelve cajeros / formas / tipos / categorías que aparecen en el rango (alimenta los selectores)
- Filtro por categoría via `EXISTS` subquery contra `venta_detalles + productos` (evita duplicar ventas que tienen varios items de la misma categoría)
- KPIs y desglose por forma de pago calculados en el mismo command (un solo round-trip)

### 🎨 Frontend

- Nueva pestaña **"Ventas detalladas"** en `/reportes` (3ra después de Estado de Resultados y Balance)
- `ReporteVentasFiltrable`: bloque de filtros (grid auto-fit), KPIs, chips por forma de pago, tabla scrolleable, footer con totales
- Reuso completo de `KpiCard`, `exportarTablaXlsx`, `exportarTablaPdf` ya existentes
- Helper `construirSubtituloVentas` que documenta los filtros aplicados en el archivo exportado

### 📦 Archivos tocados

- `src-tauri/src/commands/reportes.rs` — 2 comandos nuevos (~140 líneas)
- `src-tauri/src/lib.rs` — registro
- `src/services/api.ts` — tipos `FiltrosReporteVentas`, `VentaReporteRow`, `ReporteVentasResultado` + 2 wrappers
- `src/pages/ReportesPage.tsx` — nueva tab + componente `ReporteVentasFiltrable` + helper subtítulo

---

## v2.3.69 — 2026-05-07 ✂️ STABLE
**Restaurante: Dividir cuenta — completa el trío de features pedidas.**

Tercera y última feature del paquete restaurante solicitado. Las tres features (`v2.3.67` comandas a cocina, `v2.3.68` unir mesas, `v2.3.69` dividir cuenta) cubren los flujos clave que el cliente real reclamó.

### ✂️ Caso de uso

Un grupo de 4 personas come junto y quieren pagar por separado. Antes había que cobrar todo a una sola persona; ahora el mesero divide la cuenta en N partes iguales y cada comensal paga la suya con su propia forma de pago (efectivo, tarjeta, transferencia, crédito).

### Cómo se usa

1. Cuando el pedido tenga items y esté listo para cobrar, click en **✂️ Dividir cuenta entre varios** (debajo del botón Cobrar)
2. Modal pregunta **número de partes** (2 a 20). Default = número de comensales del pedido. Total se divide en partes iguales (la última lleva el residuo del redondeo: $100/3 → $33.33, $33.33, $33.34)
3. Click **✂️ Dividir** → la sección Cobrar se reemplaza por la **lista de sub-cuentas** con su monto y botón **💰 Cobrar** independiente
4. Cada vez que se cobra una sub-cuenta:
   - Aparece modal de forma de pago (mismo flujo que cobrar normal: efectivo / transfer / crédito)
   - Se genera una **nota de venta independiente** con el monto exacto
   - La sub-cuenta queda marcada `COBRADA` con el número de venta visible
5. Cuando **TODAS** las sub-cuentas están cobradas → el pedido se cierra y la(s) mesa(s) se liberan automáticamente
6. Mientras NINGUNA esté cobrada, se puede **Cancelar división** para volver al cobro normal

### Detalles técnicos importantes

- **Producto especial** `_DIVISION_CUENTA_` (auto-creado en `seed_default`): es_servicio=1, IVA 0%, oculto del POS normal. Cada venta de sub-cuenta usa este producto con `precio_unitario = monto de la parte`
- **Observación de cada venta**: incluye "Mesa X · Pedido #Y · Sub-cuenta i/N" y `info_adicional` con el detalle de items reales del pedido (trazabilidad)
- **Número de venta visible**: cada sub-cuenta cobrada muestra su número (ej. NV-001-001-000000042) junto a la forma de pago

### ⚠️ Limitación conocida (MVP)

El stock de los items reales del pedido **NO se descuenta** — es el tradeoff del approach simple. Aceptable para restaurantes pequeños donde el inventario fino no es crítico. Para descuento de stock + IVA desglosado por sub-cuenta haría falta refactorizar `registrar_venta` para soportar pagos múltiples sobre una sola venta (queda como mejora futura).

### 🛠 Backend

- **Schema**: tabla `rest_subcuentas(id, pedido_id, numero, total, estado, forma_pago, banco_id, referencia_pago, venta_id, fecha_cobro)` con FK CASCADE al pedido
- **Producto especial** auto-creado en `seed_default()`: codigo='_DIVISION_CUENTA_', es_servicio=1, IVA 0
- **Comandos Tauri**:
  - `rest_dividir_cuenta(pedido_id, n_partes)` — crea N sub-cuentas con reparto en centavos
  - `rest_listar_subcuentas(pedido_id)` — JOIN con cuentas_banco y ventas
  - `rest_cancelar_division(pedido_id)` — solo si NINGUNA cobrada
  - `rest_marcar_subcuenta_cobrada(subcuenta_id, venta_id, forma_pago, banco_id?, referencia?)` — auto-cierra el pedido si todas las sub-cuentas quedan cobradas
  - `rest_producto_division_id()` — devuelve el ID del producto especial
- **Validaciones**: división solo si pedido ABIERTO/CUENTA_PEDIDA, mínimo 2 / máximo 20 partes, total > 0

### 🎨 Frontend

- **PedidoDetalle**:
  - Botón discreto **✂️ Dividir cuenta entre varios** debajo del botón Cobrar (solo si hay items y NO está dividido aún)
  - Cuando está dividido: oculta botón Cobrar y muestra una **caja con lista de sub-cuentas** (parte i/N, monto, botón Cobrar individual o badge COBRADA)
  - Cobro de sub-cuenta usa el `ModalCobro` existente (reuso completo)
  - Botón **Cancelar división** visible solo si ninguna sub-cuenta cobrada
- **ModalDividirCuenta** nuevo: input numérico con +/− (2-20), preview "cada parte paga $X", warning sobre limitaciones

### 📦 Archivos tocados

- `src-tauri/src/restaurante/schema.rs` — tabla `rest_subcuentas` + producto especial en seed
- `src-tauri/src/restaurante/models.rs` — `Subcuenta`, `ResultadoCobroSubcuenta`
- `src-tauri/src/restaurante/commands.rs` — 5 comandos nuevos + helper `listar_subcuentas_internal`
- `src-tauri/src/lib.rs` — registro de comandos
- `src/restaurante/types.ts`, `src/restaurante/api.ts` — mirror TS
- `src/restaurante/components/PedidoDetalle.tsx` — UI sub-cuentas + ModalDividirCuenta

---

## v2.3.68 — 2026-05-07 🔗 STABLE  
*(release inmediatamente anterior a v2.3.69 — el mismo día)*
**Restaurante: Unir mesas para grupos grandes (2 de 3 features pedidas).**

Segunda feature de las 3 solicitadas. Próxima: **v2.3.69 (dividir cuenta)**.

### 🔗 Caso de uso

Llega un grupo grande de 10 personas y ninguna mesa los acomoda sola. El mesero abre pedido en una mesa "principal" (ej. Mesa 1) y une mesas adicionales (ej. Mesa 2 y Mesa 3) al mismo pedido. Todos los items van al mismo ticket, todas las mesas se liberan juntas al cobrar.

### Cómo se usa

1. **Abrir pedido** en cualquier mesa libre (esa será la "principal")
2. En el drawer del pedido, click en **🔗 Unir mesas**
3. Modal muestra todas las **mesas LIBRES** agrupadas por zona — seleccionar las que ocupará el grupo
4. Click **🔗 Unir** → las mesas quedan vinculadas al pedido
5. **Indicadores visuales**:
   - **Mesa principal**: badge `🔗 +N` sobre el nombre
   - **Mesas extra (unidas)**: borde azul, label "UNIDA", muestran "🔗 Unida a Mesa X"
   - **Click en mesa extra** → abre el pedido principal (mismo flujo)
6. Header del drawer muestra todas las mesas del grupo + capacidad total efectiva
7. Click `×` en cada badge de mesa unida → **desunir** (libera esa mesa, sus items quedan en la principal)
8. Al **cobrar** o **cancelar** el pedido: TODAS las mesas (principal + unidas) se liberan automáticamente

### Reglas de validación

- Solo se pueden unir mesas **LIBRES** (sin pedido propio activo y sin estar ya unidas a otro pedido)
- No se puede unir la mesa principal a sí misma
- Una mesa extra **NO** puede tener pedido propio (al unirse pierde esa capacidad hasta liberarse)
- Solo se permite unir/desunir mientras el pedido esté **ABIERTO** o **CUENTA_PEDIDA**

### 🛠 Backend

- **Schema**: nueva tabla `rest_pedido_mesas_extra(pedido_id, mesa_id, fecha_union)` con FK CASCADE al pedido
- **Comandos**:
  - `rest_unir_mesas(pedido_id, mesas_ids: number[])` — transaccional, valida todas antes de insertar
  - `rest_desunir_mesa(pedido_id, mesa_id)` — solo si pedido sigue activo
  - `rest_listar_mesas_libres_para_unir(pedido_id)` — filtra disponibles
- **Modificado** `rest_listar_mesas_con_estado`: query con COALESCE(pedido_propio, pedido_extra) — una mesa extra hereda el estado del pedido principal y trae `mesa_principal_id` + `mesa_principal_nombre`
- **Modificado** `PedidoDetalle`: ahora incluye `mesas_extra: MesaResumen[]` y `capacidad_total: number`
- **Liberación automática**: al pasar el pedido a COBRADO o CANCELADO, las mesas extra se sueltan sin lógica adicional (la query filtra solo pedidos ABIERTO/CUENTA_PEDIDA)

### 🎨 Frontend

- **MesasPage**: card de mesa extra muestra borde azul + "🔗 Unida a Mesa X" + click abre el pedido principal. Card de mesa principal muestra badge "🔗 +N" sobre el nombre
- **PedidoDetalle**: header con lista de mesas unidas (chips desunibles), footer con botón "🔗 Unir mesas (N)", modal `ModalUnirMesas` con grid agrupado por zona y multi-select
- **Total abierto** del header de MesasPage: NO duplica el total cuando una mesa está unida (solo la principal acumula)

### 📦 Archivos tocados

- `src-tauri/src/restaurante/schema.rs` — tabla `rest_pedido_mesas_extra`
- `src-tauri/src/restaurante/models.rs` — `MesaResumen`, `MesaConEstado.mesa_principal_*`, `PedidoDetalle.mesas_extra` + `capacidad_total`
- `src-tauri/src/restaurante/commands.rs` — 3 comandos nuevos + query mesas con extras
- `src-tauri/src/lib.rs` — registro de comandos
- `src/restaurante/types.ts`, `src/restaurante/api.ts` — mirror TS
- `src/restaurante/components/PedidoDetalle.tsx` — UI unir mesas + ModalUnirMesas
- `src/restaurante/pages/MesasPage.tsx` — visualización de mesas unidas en grid

---

## v2.3.67 — 2026-05-07 🍳 STABLE
**Restaurante: Imprimir comandas a cocina (1 de 3 features pedidas).**

Primera de las 3 features que el cliente pidió para llevar el módulo Restaurante a nivel profesional. Las próximas: **v2.3.68 (unir mesas)** y **v2.3.69 (dividir cuenta)**.

### 🍳 Comandas automáticas a cocina

**Caso de uso real**: el mesero envía pedido a cocina → ticket impreso aparece automáticamente en la impresora de cocina → el cocinero lo lee y empieza a preparar.

### Cómo funciona

1. **Click "🔔 Enviar cocina"** en el drawer del pedido (como antes)
2. **Automáticamente**: el sistema marca los items como enviados Y manda a imprimir la comanda en la impresora de cocina configurada
3. **Toast de confirmación**: "X items enviados a cocina · 🍽 Comanda impresa"
4. Si falla la impresora (no configurada, offline), warning en lugar de error — el flujo NO se rompe

### Diseño del ticket de comanda

```
━━━━━━━━━━━━━━━━━
   🍳 COCINA
━━━━━━━━━━━━━━━━━
  MESA: Mesa 5 (Salón)
━━━━━━━━━━━━━━━━━
 Mesero: Juan
 Hora: 13:42:18 · Pedido #42
 (Restaurante El Bosque)
─────────────────
 2x  Hamburguesa BBQ
     ↳ sin cebolla

 1x  Ensalada César

 1x  Papas Fritas
─────────────────
 3 item(s) — 13:42:18
━━━━━━━━━━━━━━━━━
```

Características clave:
- **Sin precios** (cocina no necesita verlos)
- **Cantidades en negrita doble alto** — se leen desde lejos
- **Observaciones destacadas** ("sin cebolla", "término medio") con flecha + indentadas
- **Mesa enorme** en la cabecera para identificar rápido
- **Items DIRECTO ignorados** (bebidas embotelladas no van a cocina)

### Configuración (Configuración → 🍳 Cocina)

- **Impresora de cocina** (opcional): puede ser distinta a la del POS principal. Si dejás "misma del POS", usa la principal.
- **Toggle "Imprimir tickets separados (Cocina y Barra)"**:
  - **OFF** (default): 1 ticket combinado con tag `[BARRA]` en cada item de barra
  - **ON**: 2 tickets independientes (uno cocina, uno barra) — útil si son áreas físicas distintas con impresoras dedicadas
- **Impresora de barra** (solo si modo separado activo): impresora dedicada para items de barra. Si vacío, usa la de cocina.

### Re-imprimir comanda

Si la impresora se atascó o el ticket se perdió, hay un link pequeño debajo del botón "Enviar cocina":

> 🖨 Reimprimir comanda

Imprime la comanda completa con TODOS los items del pedido (no solo los nuevos).

### Cambios técnicos
- `src-tauri/src/restaurante/printing.rs`:
  - `enum DestinoComanda { Cocina, Barra, Ambos }`
  - `generar_comanda_cocina(...)` retorna `Option<Vec<u8>>` (None si no hay items que imprimir según el filtro)
  - Items DIRECTO siempre filtrados out
- `src-tauri/src/restaurante/commands.rs::rest_imprimir_comanda_cocina(pedido_id, items_ids?)`:
  - Si `items_ids` viene, solo imprime esos (auto al enviar cocina)
  - Si None, imprime todos (re-imprimir)
  - Resuelve impresora: `impresora_cocina` → fallback a `impresora` principal
  - Modo separado: 2 tickets independientes (cocina + barra)
- `src-tauri/src/lib.rs`: registrado nuevo comando
- `src/restaurante/api.ts`: wrapper `imprimirComandaCocina(pedidoId, itemsIds?)`
- `src/restaurante/components/PedidoDetalle.tsx`:
  - `handleEnviarCocina` ahora llama `imprimirComandaCocina(pedidoId, itemIds)` después de enviar
  - `handleReimprimirComanda` (nuevo) llama sin `itemsIds`
  - Botón pequeño "🖨 Reimprimir comanda" debajo de "Enviar cocina" si hay items ya enviados
- `src/pages/Configuracion.tsx`: nueva sección "🍳 Cocina (Restaurante)" con selector de impresora + toggles

Verificado: cargo check OK, tsc EXITCODE=0.

### Próximas features de Restaurante (planificadas)

- **v2.3.68** — 🔗 Unir mesas (combinar 2+ mesas en 1 pedido para grupos grandes)
- **v2.3.69** — ✂️ Dividir cuenta (cobrar 1 mesa en N sub-cuentas independientes)

## v2.3.66 — 2026-05-06 🧭 STABLE
**UX flow transferencias: navegación inteligente desde el modal a la fecha exacta.**

### Problema reportado

El usuario tenía una transferencia de **abril** pendiente de verificar. Al hacer click en la alerta del Dashboard se abría Movimientos Bancarios con filtro "Este mes" (mayo) y la transferencia NO aparecía. El usuario tenía que cambiar manualmente el período a abril para encontrarla.

### Fix

**Modal de transferencias pendientes** (v2.3.64 + v2.3.66):
- Cada fila ahora tiene botón **"Ir →"** (admin y cajero) que navega a Movimientos Bancarios con la fecha EXACTA de esa transferencia + filtro "Por verificar" preconfigurado
- Botón **"Forzar"** (solo admin) para limpiar badges fantasma — sin cambios

**MovimientosBancariosPage** (nuevo):
- Lee URL params: `?desde=YYYY-MM-DD&hasta=YYYY-MM-DD&estado=REGISTRADO`
- Aplica filtros automáticamente al montar
- **Chip visible** con el filtro de estado activo: "⚠ Filtrando por estado: Por verificar [✕ Quitar filtro]"
- Filtro combinado con tipo (Todos/Ventas/Retiros caja/etc.)

### Resultado

```
ANTES:
1. Click "1 transferencia por verificar" → Bancos (filtro mes actual)
2. No aparece → confusión
3. Cambiar período a abril manualmente
4. Buscar la transferencia
5. Verificar

AHORA:
1. Click alerta → Modal con detalle
2. Click "Ir →" → Bancos filtrado en la fecha exacta + estado=Por verificar
3. La transferencia aparece directamente
4. Verificar
```

### Cambios técnicos
- `src/components/ModalTransferenciasPendientes.tsx`: `useNavigate` + handler `handleIrAVerificar` que navega con URL params; columna "Acciones" combina "Ir" + "Forzar"
- `src/pages/MovimientosBancariosPage.tsx`:
  - `useSearchParams` para leer `desde`, `hasta`, `estado`
  - State `filtroEstado` con valor inicial desde URL
  - `useMemo` `movimientosFiltrados` aplica filtro tipo + estado
  - Chip visual con filtro activo + botón quitar

Verificado: tsc EXITCODE=0.

## v2.3.65 — 2026-05-06 🔒 STABLE
**Hotfix anti-fuga: toast del descuadre revelaba el monto exacto al cajero.**

### 🔥 Fix crítico

**Problema reportado**: aún con el toggle anti-fuga activo y la alerta visual de descuadre oculta (v2.3.64), cuando el cajero presionaba "Cerrar Caja" con un monto incorrecto, aparecía un toast de error:

> ❌ "Hay un descuadre de $-36.82. Debe explicar el motivo (mínimo 5 caracteres)."

Eso le revelaba el monto exacto del faltante. El cajero deshonesto podía:
1. Ingresar un valor cualquiera (ej. "1")
2. Click "Cerrar"
3. Leer el toast: "Hay un descuadre de $-36.82"
4. Sumar 36.82 al valor ingresado
5. Volver a cerrar y cuadrar perfecto
6. Faltante real ocultado

**Fix**: cuando modo anti-fuga activo + usuario es CAJERO (no admin):
- Toast genérico **sin monto**: *"El monto contado no coincide con lo registrado. Escribe una observación (mínimo 5 caracteres) en el campo de abajo y vuelve a cerrar caja."*
- El campo "Motivo del descuadre" sigue oculto
- El cajero usa el campo "Observación adicional" (siempre visible) como motivo
- El backend recibe esa observación como motivo del descuadre para que admin la vea al revisar
- Admin sigue viendo toda la info completa (sin cambios para él)

### Resultado

Ahora el cajero NUNCA puede saber el monto del descuadre — ni en pantalla ni en toast. Si ingresa mal, solo sabe que "no coincide" pero no por cuánto. Con la herramienta deshonesta de "ir ajustando hasta cuadrar" eliminada por completo.

### Cambios técnicos
- `src/pages/CajaPage.tsx::intentarCerrarCaja`:
  - Branching según `ocultarParaCajero`: mensaje genérico vs específico
  - Si anti-fuga activo, valida `observacion` (no `motivoDescuadre`) ya que el campo de motivo está oculto
  - El motivo final pasado al backend usa `observacion` para que admin lo vea al revisar el cierre

Verificado: `tsc --noEmit` EXITCODE=0.

## v2.3.64 — 2026-05-06 🔍🔒 STABLE
**Modal de diagnóstico transferencias + fix anti-fuga descuadre.**

### 🔒 Fix crítico: descuadre delataba el monto esperado al cajero (anti-fuga)

**Problema reportado**: aún con el toggle anti-fuga activo, cuando el cajero ingresaba un monto en "Monto real contado en caja", aparecía la alerta "⚠ Descuadre: -$42.82 (faltante)" + el motivo obligatorio. Eso le permitía al cajero ir aumentando el monto poco a poco hasta llegar al "exacto" — exactamente lo que la feature buscaba PREVENIR.

**Fix**: en modo anti-fuga, el cajero NUNCA ve la alerta de descuadre ni el campo "Motivo del descuadre". Solo ve "Monto real contado en caja" + botón Cerrar Caja. Cuenta a ciegas, envía, y el admin audita después.

**Bonus**: también se eliminó el banner ruidoso "🔒 Conteo a ciegas" — el cajero solo ve el input limpio, sin pistas que delaten la feature.

### 🔍 Modal de diagnóstico de transferencias pendientes

**Problema reportado** (recurrente desde v2.3.60): el badge "1 transferencia por verificar" del Dashboard sigue apareciendo aunque el usuario verificó todas. El cleanup automático no las pesca cuando la venta padre también está REGISTRADO.

**Fix**: nuevo modal que se abre al click en la alerta del Dashboard. Muestra **exactamente qué está contando** el sistema:
- Lista completa de transferencias pendientes (sin filtro de fecha)
- Por cada una: # venta, fecha, monto, cliente, tipo (VENTA o MIXTO)
- Botón **"Forzar verificar"** (solo admin) — último recurso si el cleanup no resuelve

Esto resuelve la frustración del usuario: ahora ve qué hay, decide si es real o fantasma, y si es fantasma lo limpia con 1 click.

### Cambios técnicos
- `src/pages/CajaPage.tsx`: condicional `if (ocultarParaCajero) return null;` antes de mostrar la alerta de descuadre + sin banner anti-fuga
- `src-tauri/src/commands/verificacion.rs`:
  - Nuevo `detalle_transferencias_pendientes()` retorna lista detallada sin filtro de fecha
  - Nuevo `forzar_marcar_transferencia_verificada(origen, id, motivo)` para admin
- `src-tauri/src/lib.rs`: registrar nuevos comandos
- `src/services/api.ts`: wrappers `detalleTransferenciasPendientes`, `forzarMarcarTransferenciaVerificada`
- `src/components/ModalTransferenciasPendientes.tsx` (nuevo): modal con tabla + acción forzar
- `src/pages/DashboardPage.tsx`: alerta de transferencias ahora abre modal en vez de navegar; refresh automático del contador después de forzar

Verificado: cargo check OK, tsc EXITCODE=0.

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
