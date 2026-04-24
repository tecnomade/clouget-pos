# Correcciones después de v2.3.8

Registro detallado de bugs reportados por el usuario en pruebas y los fixes
aplicados en cada release de hotfix posterior a v2.3.8-beta.

---

## v2.3.9-beta (4 fixes)

### 1. 🐛 Cierre de caja: motivo descuadre no aparecía si monto vacío

**Síntoma reportado:** Al intentar cerrar caja con un descuadre, el sistema
pedía un motivo de mínimo 5 caracteres pero el campo textarea no era visible
en pantalla. El usuario no tenía dónde escribir.

**Causa:** En `CajaPage.tsx` la condición `if (!montoReal || Math.abs(dif) <= 0.01) return null`
devolvía `null` cuando `montoReal` estaba vacío (string ""). Esto ocultaba la
textarea aun cuando había descuadre real (esperado=$14.35, real vacío =
descuadre de -$14.35).

**Fix:** Cambiada la condición a `if (Math.abs(dif) <= 0.01) return null`. Ahora
el campo aparece apenas hay diferencia significativa entre esperado y real.

### 2. 🐛 Resetear datos (Zona de peligro): FOREIGN KEY constraint failed

**Síntoma reportado:** Al usar el botón "RESETEAR" en Configuración → Zona
de peligro, fallaba con error "Error reseteando datos: FOREIGN KEY constraint
failed" y no borraba los datos de prueba.

**Causa:** El comando `resetear_base_datos` en `commands/config.rs` solo
borraba un subset de tablas y no respetaba el orden de dependencias FK.
Faltaban tablas como `caja_eventos`, `lotes_caducidad`, `pagos_cuenta`,
`movimientos_inventario`, `transferencias`, `ordenes_servicio`, etc. — todas
introducidas en versiones recientes.

**Fix:** Reescrito el comando:
- `PRAGMA foreign_keys = OFF` durante el truncate y `ON` al final.
- Lista exhaustiva de tablas a borrar en orden dependiente (hijos → padres).
- Verificación previa de existencia vía `sqlite_master` para no romper en
  BDs viejas que no tengan alguna tabla.
- Recolección de errores por tabla y reporte agregado.

### 3. 🐛 Productos: precio_costo visible para todos los roles

**Síntoma reportado:** Cualquier usuario (incluso cajero) podía ver el precio
de costo en el formulario de Productos. Faltaba respetar el permiso `ver_costos`.

**Fix:**
- `Productos.tsx` ahora consume `useSesion()` y calcula
  `puedeVerCostos = esAdmin || tienePermiso("ver_costos")`.
- Pasa `puedeVerCostos` como prop al `FormProducto`.
- Si NO puede ver costos, el campo "Precio costo" se muestra como `••••`
  read-only y disabled, con tooltip explicando el motivo.

### 4. 🐛 POS no respetaba la lista de precios del cliente

**Síntoma reportado:** Cuando se asigna una lista de precios a un cliente
(ej. "Mayorista") y se selecciona en el POS, los precios no cambiaban al
agregar productos desde el grid (pantalla principal de productos del POS).

**Causa:** En `agregarAlCarrito` el cálculo de precio era:
`unidadElegida?.precio ?? producto.precio_lista ?? producto.precio_venta`.
Pero los productos del grid (`ProductoTactil`) NO traen `precio_lista`, así
que siempre caía a `precio_venta` ignorando la lista del cliente.

**Fix:** Ahora si el cliente tiene `lista_precio_id` y el producto no trae
`precio_lista`, se llama a `resolverPrecioProducto(producto.id, clienteId)`
para obtener el precio correcto de la lista del cliente.

### 5. ➕ Nuevo permiso `cambiar_lista_precio`

Agregado al sistema de permisos backend (`PERMISOS_DISPONIBLES` en
`models/usuario.rs`). La UI para cambiar lista en POS desde un selector
queda pendiente para próxima iteración.

---

## v2.3.10-beta (HOTFIX crítico)

### 6. 🔥 Pantalla en blanco al cambiar de usuario

**Síntoma reportado:** "Cuando salgo de una sesión abierta y abro un usuario
subordinado se queda pantalla en blanco actualmente y toca cerrar y abrir
la aplicación".

**Confirmación por log de consola:** `No routes matched location "/config"`.

**Causa raíz:** En `main.tsx` las rutas `/productos`, `/clientes`, `/config`,
etc. solo se renderizan condicionalmente cuando `sesion.rol === "ADMIN"`.
Cuando un admin estaba en `/config` y cerraba sesión, al loguear el cajero
las rutas admin desaparecían pero la URL seguía siendo `/config` → React
Router no encontraba match → no renderizaba nada → pantalla blanca.

**Fix:**
- Catch-all `<Route path="*" element={<Navigate to="/" replace />} />`
  que redirige cualquier ruta no encontrada al dashboard.
- `BrowserRouter` ahora usa `key={sesion.usuario_id}` para forzar
  remount completo al cambiar de usuario. Esto resetea el historial de
  navegación y todos los estados de página, evitando datos stale del
  usuario anterior.

---

---

## v2.3.11-beta (5 fixes + auditoría completa de permisos)

### 7. 🔥 Cajero con permisos no podía abrir Productos / Reportes

**Síntoma reportado:** "Cuando va a productos usuario cajero que tiene permiso de
productos y ver precios costo le vuelve a pedir abrir caja" (= lo redirige al
home porque cae en el catch-all de v2.3.10).

**Causa:** En `main.tsx` las rutas `/productos`, `/clientes`, `/reportes`, etc.
se filtraban con `sesion.rol === "ADMIN"` ignorando los permisos. Aunque el
cajero tenía `gestionar_productos`, la ruta no se renderizaba → caía al
catch-all → redirigido a `/`.

**Fix:** Cada ruta admin-only se cambió a condicional por permiso:
```jsx
{(esAdmin || tienePermiso("gestionar_productos")) && <Route path="/productos" ... />}
{(esAdmin || tienePermiso("ver_reportes")) && <Route path="/reportes" ... />}
// etc.
```

### 8. 🐛 Lista de productos no mostraba precio de costo

**Síntoma reportado:** "Aún no ve precio costo" (en la lista principal de
Productos, además del form que se arregló en v2.3.9).

**Fix:** Si `esAdmin` o tiene permiso `ver_costos`, ahora se agregan dos
columnas a la tabla:
- **COSTO**: precio de costo del producto
- **MARGEN**: margen porcentual con color (rojo si < 0, ámbar si < 15%, verde
  si ≥ 15%)

### 9. 🐛 No había forma de reimprimir reporte de cierre

**Síntoma reportado:** "Aún no permite reimprimir el reporte de sesiones de
caja de forma individual".

**Fix:** En el drawer "Historial de caja" → tab "Sesiones", al expandir una
sesión cerrada aparecen 2 botones nuevos:
- **🖨 Reimprimir ticket** — usa `imprimir_reporte_caja` (ESC/POS térmica)
- **📄 Reporte PDF A4** — usa `imprimir_reporte_caja_pdf` (incluye
  trazabilidad agregada en v2.3.3)

### 10. ➕ Selector de lista de precios global en POS

**Pedido:** "¿Dónde cambio la lista de precio?".

**Fix:** Header del POS ahora tiene un selector "Lista: ..." visible si
`esAdmin` o tiene el permiso `cambiar_lista_precio`. Permite forzar una
lista distinta a la del cliente. Default "Auto" usa la lista del cliente o
precio_venta default. Al cambiar la selección se recalcula el carrito
automáticamente. Botón × para volver a Auto.

### 11. ➕ Auditoría completa de permisos (2 nuevos)

**Pedido:** "Revisar bien los permisos para los usuarios subordinados".

**Permisos nuevos:**
- `gestionar_gastos` — necesario para ver `/gastos` (antes era admin-only).
- `ver_pagos_pendientes_admin` — para confirmar/rechazar transferencias en CXC.
- `cambiar_lista_precio` — selector lista en POS (introducido en v2.3.9, UI ahora).

**Sincronización:**
- `Layout.tsx` y `main.tsx` ahora usan **la misma matriz** rol/permiso → ruta.
  Antes había desincronización (Layout filtraba por permiso pero las rutas
  solo por rol).
- Soporte para **permiso alternativo** (`permisoAlt`): por ej.
  `/servicio-tecnico` se ve si tiene `gestionar_servicio_tecnico` O
  `ver_servicio_tecnico`.

---

## Pendientes después de v2.3.11 (siguiente iteración)

| # | Tarea | Origen |
|---|---|---|
| 1 | Mejorar plantilla de import productos + tests internos | reportado |
| 2 | Re-validar bug "cajero no muestra productos" tras v2.3.10/v2.3.11 | reportado |

---

## Notas de desarrollo

### Sobre el bug "blanco al cambiar usuario"
Este patrón de **rutas condicionales por rol sin catch-all** es un bug latente
en SPAs con react-router. Recomendación general: siempre incluir un
catch-all que redirija al home, especialmente en apps con sesiones
multi-rol.

### Sobre `resetear_base_datos`
Cada nueva tabla transaccional que se agregue en el futuro debe sumarse
al array `tablas_a_borrar` del comando. Considerar mover esta lógica a
un módulo central que pueda mantenerse junto a las migraciones de schema.
