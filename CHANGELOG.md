# CHANGELOG — Clouget Punto de Venta

Historial de mejoras, correcciones y nuevas funcionalidades. Cada entrada incluye fecha y versión publicada en GitHub Releases.

Repositorio: https://github.com/tecnomade/clouget-pos/releases

---

## v2.6.9 — 2026-06-07 🔐 Caja: exigir el monto contado para cerrar

- Al **cerrar caja** sin haber escrito el **"Monto real contado"**, ahora la app **no cierra**: pide *"Digite el valor real contado en caja para cerrarla"* y enfoca el campo. Evita cierres con $0 por descuido (que generaban descuadres). Si la caja está en $0, basta con escribir 0.

---

## v2.6.8 — 2026-06-07 🖼️ Comprobantes: ya se pueden subir fotos grandes (se comprimen solas)

- Antes los comprobantes (transferencias, depósitos, pago mixto, servicio técnico) **rechazaban imágenes mayores a 500 KB** — una limitante al subir fotos del celular.
- Ahora **se aceptan fotos grandes**: la app las **redimensiona y comprime automáticamente** (lado máximo 1600 px, JPEG) antes de guardarlas. Quedan **legibles** pero livianas, sin trabar la app ni la base de datos.

---

## v2.6.7 — 2026-06-06 🏦 Depósitos en tránsito: panel central para confirmarlos

- Cuando un **cajero** retira efectivo para **depositarlo al banco**, queda **EN TRÁNSITO** hasta que alguien lo confirme. Antes solo se podía confirmar dentro de la caja abierta; si la caja se cerraba, quedaba colgado.
- Nuevo **panel "Depósitos en tránsito"** que lista **todos** los depósitos pendientes (de cualquier caja, abierta o cerrada) y permite **confirmarlos** (referencia bancaria + comprobante) → Depositado.
- Disponible en **Bancos**, **Reportes** (pestaña "Depósitos en tránsito") y **Caja**.
- Protegido por el **nuevo permiso `confirmar_depositos`** (lo tiene el admin; se lo puedes dar a un supervisor/contador). Se valida también en el servidor.

---

## v2.6.6 — 2026-06-06 🧮 Caja: tabla "Movimientos" ajustada (columna Usuario ya no se recorta)

- La tabla de **Movimientos de caja** ahora **ajusta sus columnas al ancho** de la tarjeta (anchos fijos por columna). Ya no se recorta la columna **Usuario** ni necesita scroll horizontal en pantallas normales.

---

## v2.6.5 — 2026-06-06 🔗 Compras: acceso directo al Reporte de Compras

- En **Compras** se agregó el botón **"📊 Reporte de Compras"** que abre directamente el reporte (Reportes → pestaña Compras), sin tener que buscarlo entre las pestañas.

---

## v2.6.4 — 2026-06-06 🖱️ Cobro: la rueda del mouse ya no altera el monto recibido (25 → 24.99)

- **Corregida la causa real** del ticket que mostraba, por ejemplo, **Recibido $24.99 / Cambio $2.99** cuando se había recibido **$25.00**: el campo numérico cambiaba su valor al **rozar la rueda del mouse** estando enfocado (restaba un centavo). 
  - El campo "Monto recibido" ahora es de texto con teclado decimal (sin rueda/flechas) y acepta coma o punto.
  - Además, a nivel global, **la rueda del mouse ya no modifica ningún campo numérico** enfocado (protege también monto inicial de caja, retiros, ingresos, etc.).

---

## v2.6.3 — 2026-06-06 💵 Ticket: redondeo a centavos del monto recibido y el vuelto

- **Endurecido el redondeo** del **monto recibido** y el **vuelto/cambio** a 2 decimales (centavos), tanto al registrar la venta como al imprimir el ticket. Evita que un arrastre de coma flotante muestre, por ejemplo, $24.99 / vuelto $2.99 cuando debería ser $25.00 / vuelto $3.00.

---

## v2.6.2 — 2026-06-06 🧭 Menú: "Cerrar sesión" siempre visible (no se corta en pantallas pequeñas)

- **Corregido:** en algunas pantallas (resoluciones bajas o con escalado de Windows) no se podía bajar el menú lateral para ver el botón **Cerrar sesión** — quedaba cortado. Ahora **"Cerrar sesión" queda fijo al fondo del menú** (siempre visible) y solo los ítems del menú hacen scroll.

---

## v2.6.1 — 2026-06-06 🧯 Caja: corregido el desborde de la tabla "Movimientos de caja"

- La tabla de **Movimientos de caja** ya no se desborda (la columna **Usuario** se salía del recuadro). Ahora se ajusta con scroll horizontal dentro de la tarjeta.

---

## v2.6.0 — 2026-06-06 📱 App móvil: el admin ve TODAS las órdenes de servicio + estados igual que en PC

- **Servidor (app móvil):** el listado de órdenes de servicio ya **no oculta las entregadas/canceladas**. El admin/coordinador ve **todas** las órdenes (igual que en la PC); el técnico ve las suyas, en todos los estados.
- Esto corrige que un admin no viera órdenes en el móvil (antes solo se mostraban las "activas").

> Requiere actualizar también la app móvil (se publica por separado) para ver los nuevos filtros de estado (Entregadas / Canceladas).

---

## v2.5.99 — 2026-06-06 🍽 Restaurante: cobrar sin liberar la mesa + Consumidor Final en reporte de ventas

- **Restaurante — cobro flexible (prepago / mixto):** al **Cobrar** una mesa, ahora se imprime un **comprobante** (lo pagado + total) y el sistema **pregunta si liberar la mesa**. Si eliges **No**, la mesa **sigue ocupada con una cuenta nueva** para que los comensales sigan pidiendo (ideal para pago previo, o mixto: prepago + extras al final). Si piden factura, se entrega al final.
  - Nuevo interruptor en **Configuración → Cocina (Restaurante): "Imprimir comprobante automático al cobrar"** (activado por defecto).
- **Reporte de ventas detalladas:** ahora muestra correctamente **"CONSUMIDOR FINAL"** y su RUC **9999999999999** en las ventas sin cliente (antes salían en blanco).

---

## v2.5.98 — 2026-06-06 📦 Reporte de Compras + búsqueda por código de barras en Productos

- **Nuevo reporte de Compras** (Reportes → Compras): filtra por **rango de fechas** y por **proveedor**, con 3 vistas: **Detalle**, **Por proveedor** (con subtotales) y **Por fecha** (con totales por día), más total general. Exportable a Excel y PDF.
- **Productos:** el buscador ahora encuentra por **código de barras** (y también por descripción), no solo por nombre/código. Antes no aparecían los productos al escribir/escanear el código de barras.

---

## v2.5.97 — 2026-06-06 🧮 Caja: el cuadre forzado es solo para admin (cajeros cierran sin bloqueo)

- Ahora, **por defecto, los cajeros pueden cerrar caja aunque el efectivo no coincida**, sin tener que escribir un motivo. La diferencia **igual queda registrada** para que el admin la revise.
- Al **admin** se le sigue exigiendo justificar el descuadre (igual que antes).
- Nuevo interruptor en **Configuración → Caja: "Exigir cuadre de caja a cajeros"** (desactivado por defecto) por si quiere volver a exigirles que expliquen el faltante/sobrante.

---

## v2.5.96 — 2026-06-06 ✅ Suscripción SRI: validación automática al emitir + ID del equipo visible

- **Al emitir una factura**, si se agotó la prueba gratuita y no hay validación en caché, el POS ahora **valida la suscripción online automáticamente** (ya que para enviar al SRI se necesita internet). Con la suscripción activa en el panel, el cliente **factura sin pasos manuales** — ya no es obligatorio pulsar "Verificar suscripción" primero.
- Si el servidor responde que no hay suscripción activa, se muestra **ese mensaje claro**; si no hay internet, el aviso de reconectar.
- **ID del equipo** ahora visible en Configuración → SRI (clic para copiar). Sirve para registrar la suscripción SRI en el panel admin con el identificador exacto.
- La suscripción SRI es un permiso **global** para todos los comprobantes (factura, NC), independiente de los módulos.

---

## v2.5.95 — 2026-06-06 🔎 Configuración SRI: detalles de la firma + último secuencial autorizado

- **Firma electrónica:** ahora Configuración muestra el **titular** del certificado y su **fecha de vencimiento**, con días restantes y aviso en rojo si está **vencida** (el SRI rechaza comprobantes con firma vencida).
- **Último secuencial autorizado:** se muestra el número de la **última factura** y la **última nota de crédito** que el SRI autorizó (formato establecimiento-punto-secuencial). Los secuenciales *próximos* (editables) siguen en la configuración de documentos.
- Incluye el fix de firma de la v2.5.94 (firmador empaquetado en el instalador).

> Nota sobre el aviso "No se puede verificar su suscripción SRI": aparece cuando se agotó la prueba gratuita y no hay validación de suscripción en caché. Solución: en Configuración → SRI, con internet, pulse **Verificar suscripción**.

---

## v2.5.94 — 2026-06-06 🔏 Firma electrónica SRI: corregida en instalaciones de clientes

- **Corregido el error "No se encontró el script de firma (scripts/firmar-xml.cjs)"** que aparecía al autorizar Facturas/Notas con firma electrónica en equipos de clientes.
- El firmador XAdES-BES del SRI ahora se **empaqueta dentro del instalador** (script autocontenido + Node.js portátil incluido). Ya **no requiere** tener Node.js instalado en la PC del cliente ni depende de rutas del entorno de desarrollo.
- Se mantiene exactamente la misma librería de firma (`ec-sri-invoice-signer`), probada en producción, por lo que la firma generada es idéntica a la aceptada por el SRI.

---

## v2.5.93 — 2026-06-06 🔄 Auto-update: lee el manifiesto directo de GitHub + progreso real

- El actualizador del escritorio ahora consulta el manifiesto **directo de GitHub** (que el release mantiene al día), en vez de un intermediario que quedaba desfasado una versión. Esto corrige el "no actualiza a la última".
- El canal beta ahora **muestra el progreso real** de descarga (antes se quedaba en 0% aunque sí descargara).

---

## v2.5.92 — 2026-06-03 🍽️ Mesas: badge "Cuenta parcial" + aviso de abonos al cerrar caja

- En la grilla de mesas (escritorio y app) aparece un badge **💵 Cuenta parcial · saldo $X** cuando la mesa tiene pagos parciales.
- Al **cerrar caja**, se muestra un aviso con los **abonos de mesa sin cerrar** (ese efectivo está en caja pero la venta se registra al cerrar la mesa) — para que el arqueo sea claro, igual que los anticipos de Servicio Técnico.

---

## v2.5.91 — 2026-06-03 🍽️ Pagos parciales por mesa (abonos) — escritorio

- En el detalle de una mesa, botón **Registrar pago parcial (abono)**: el cliente puede pagar por partes mientras sigue consumiendo.
- La mesa muestra **Consumido / Abonado / Saldo** + historial de abonos. El botón Cobrar pasa a cobrar solo el **saldo**.
- Al cobrar, la venta se arma como **MIXTO** (abonos + saldo), reflejando cada forma de pago en el arqueo. El dinero del abono entra a caja como anticipo (holding), igual que en Servicio Técnico.
- (Móvil y badge de mesa: en camino.)

---

## v2.5.90 — 2026-06-03 💵 Apertura de caja no repite la justificación del faltante

- Si la caja anterior se cerró con un **descuadre ya justificado** (faltante explicado al cerrar), al **abrir** la nueva caja ya **no se vuelve a pedir** justificar la diferencia. Queda una nota automática en la auditoría para conservar el rastro.
- Si el cierre anterior cuadró, la apertura sigue pidiendo justificación cuando el monto difiere (control intacto).

---

## v2.5.89 — 2026-06-03 🍗 Combos/recetas con opciones y precio por extra

- **Combo flexible mejorado** para casos tipo restaurante/KFC:
  - **Precio por opción**: cada opción/extra puede sumar al precio (ej: papas grandes +$2). El total del combo = precio base + extras elegidos.
  - **Mismo ingrediente con distintas cantidades** en un grupo (ej: Alitas Tipo1=2, Tipo2=6, Tipo3=12, todas del mismo ingrediente "alita"). Antes la base lo impedía.
  - **Etiqueta por opción** (nombre visible en el POS, ej: "Tipo 2 (6 alitas)").
- Grupos obligatorios (bebida: elegir 1) y opcionales (extras: varios) ya existían; ahora con precio.
- El POS muestra el total en vivo al armar el combo y descuenta el stock de los ingredientes elegidos.

---

## v2.5.88 — 2026-06-03 📉 Dashboard: stock bajo vs sin stock + imprimir lista

- El widget de stock del **Dashboard** ahora separa **⛔ Sin stock** (agotados, 0/negativo) de **⚠ Stock bajo** (con stock pero bajo el mínimo).
- Al hacer click en cualquiera, va a **Productos** con ese filtro ya aplicado (listo para revisar/exportar).
- Nuevo botón **🖨️ Imprimir lista** en Productos: imprime la lista filtrada actual (código, nombre, categoría, stock, mínimo) — sirve para reponer o guardar como PDF.

---

## v2.5.87 — 2026-06-03 ⛔ Filtro "Sin stock" en el Punto de Venta

- Nuevo chip **⛔ Sin stock** en la barra de categorías del POS: muestra solo los productos **agotados** (stock ≤ 0) que controlan inventario — para saber qué reponer sin confundir con stock bajo.
- Funciona incluso si el modo de stock oculta los agotados: el filtro los muestra igual.

---

## v2.5.86 — 2026-06-03 🧾 Cheque oculto por defecto

- El botón **Cheque** ahora viene **oculto por defecto**. El **administrador siempre lo ve**; para que lo vean los **cajeros**, el admin lo activa en Configuración → Cuentas Bancarias → "Mostrar botón Cheque".

---

## v2.5.85 — 2026-06-03 🏷️ "Crédito" renombrado a "Fiado"

- La forma de pago **fiado** ahora se muestra como **"Fiado"** en todo el POS (botón, pago mixto, Ventas, Dashboard, ticket y reporte de cierre), para no confundirla con la nueva **Tarjeta de crédito/débito**.
- Es solo cambio de etiqueta: el valor interno y los datos históricos no cambian.

---

## v2.5.84 — 2026-06-03 💳 Formas de pago: Tarjeta y Cheque

- Nuevas formas de pago **Tarjeta** y **Cheque** en el POS. Layout: Efectivo grande + grilla compacta (Transferencia · Tarjeta · Crédito · Cheque), sin saturar.
- **Configurables**: en Configuración → Cuentas Bancarias se activan/desactivan los botones Tarjeta y Cheque (por defecto visibles).
- Tarjeta/Cheque admiten referencia (voucher / n° de cheque) y **no afectan el efectivo en caja**.
- Reportes y ticket de cierre ahora muestran Tarjeta y Cheque por separado (antes Cheque caía en "Otros").

---

## v2.5.83 — 2026-06-03 🔒 El permiso de eliminar también cubre categorías

- El permiso "Eliminar productos y categorías" ahora también oculta/valida el botón de **eliminar categorías** (antes solo productos). Mismo comportamiento: admin siempre puede; cajero nuevo no, hasta que se le marque el permiso.

---

## v2.5.82 — 2026-06-03 🔒 Permiso para eliminar productos

- Nuevo permiso **"Eliminar productos (botón borrar)"**. Por defecto los usuarios nuevos (ej. cajeros) **NO** lo tienen → no ven el botón de borrar productos.
- El **admin** siempre puede eliminar. Para que otro usuario pueda, marcar el permiso en Configuración → Usuarios.
- Refuerzo en backend: aunque se intente saltar la interfaz, `eliminar_producto` valida el permiso.

---

## v2.5.81 — 2026-06-03 💵 Cierre de caja: desglose del esperado que cuadra

- En el cierre, el **desglose del monto esperado** (visible para admin) ahora muestra: *Inicial + Ventas efectivo + Cobros efectivo + Ingresos − Gastos − Retiros = Esperado*. El sistema te **explica el porqué** en pantalla.
- **Fix de consistencia**: `obtener_resumen_caja` sumaba los **ingresos manuales dentro de "Retiros"**, lo que descuadraba el desglose respecto al esperado real. Ahora cuenta solo `tipo='RETIRO'` y muestra los ingresos aparte (igual que el cálculo del cierre). Esto también corrige el ticket impreso de cierre.
- Si el desglose no cuadra con el esperado del sistema, se muestra un **aviso de desincronización** (en vez de un descuadre fantasma silencioso).

---

## v2.5.80 — 2026-06-03 🔎 Filtros de stock más claros

- En **Productos**, el filtro de stock se separó en tres opciones (antes solo había "Sin stock" que mezclaba todo):
  - **Sin stock (0 o negativo)**
  - **Stock negativo** (ventas de más / errores a corregir)
  - **Stock bajo** (con stock pero por debajo del mínimo)
- Los filtros de stock solo aplican a productos que controlan inventario (excluyen servicios, combos y "no controla stock").

---

## v2.5.79 — 2026-06-03 📱 App: pantalla de Ventas + autorizar SRI

- Nuevos endpoints para la app móvil: `GET /api/v1/app/ventas` (ventas del día con estado SRI) y `GET /api/v1/app/sri/estado` (si el POS está listo para emitir).
- La app móvil ahora tiene una pestaña **Ventas** desde donde se puede **autorizar SRI** (emitir factura electrónica) de una venta, si la facturación electrónica está configurada en Windows.

---

## v2.5.78 — 2026-06-03 📦 Integridad de inventario (stock por kardex)

- **El stock inicial se captura una sola vez, al crear el producto** — y ahora genera un movimiento **INICIAL** en el kardex (origen trazable).
- **Al editar un producto, el stock es de solo lectura.** Para cambiarlo hay un botón **Ajustar** que pide motivo y registra un movimiento **AJUSTE** en el kardex (con usuario y costo). Antes el stock se editaba a mano sin dejar rastro.
- **No se puede eliminar un producto con stock > 0**: primero hay que ajustarlo a 0 (queda registrado). Así no desaparece inventario sin trazabilidad. Aplica también al borrado masivo (omite los que tienen stock e informa cuántos).
- Esto además mantiene consistente el costo promedio y la valorización del inventario.

---

## v2.5.77 — 2026-06-02 🎨 Nuevo icono de la aplicación

- Se actualizó el **icono del programa** (logo Clouget) en la ventana, barra de tareas e instalador.

---

## v2.5.76 — 2026-06-02 🧾 Email y página web en recibos

- **Configuración → Negocio**: dos campos nuevos opcionales, **Email** y **Página web**.
- Si se llenan, aparecen automáticamente en el **ticket ESC/POS**, el **ticket PDF** de venta y la **guía de remisión** (bloque del emisor). Si se dejan vacíos, no se muestran.
- El endpoint `/api/v1/app/me` ahora expone `email_negocio` y `pagina_web` para que el **comprobante de la app móvil** también los incluya.

---

## v2.5.75 — 2026-05-31 🔑 App móvil: login por contraseña

- Nuevo endpoint `POST /api/v1/app/auth/password` — login por contraseña, para negocios que configuran **modo de login = contraseña** (o ambos), no solo PIN.
- El `ping` ahora expone `modo_login` (`pin` / `password` / `ambos`) para que la app móvil muestre el método correcto automáticamente.

---

## v2.5.74 — 2026-05-31 👥 App móvil: más usuarios en el login

- El login de la app ahora también lista a los usuarios de **Servicio Técnico** (permisos `gestionar_servicio_tecnico` / `ver_servicio_tecnico` / `recibir_abonos_st`), no solo restaurante/venta.
- El endpoint `usuarios-disponibles` informa cuántos usuarios activos **no** pueden entrar por falta de permiso de App Móvil, para que la app guíe al admin sobre cómo habilitarlos.

---

## v2.5.73 — 2026-05-31 🏦 App móvil: endpoint de cuentas bancarias

- `GET /api/v1/app/cuentas-banco` — lista las cuentas bancarias activas para que la app móvil permita cobrar con **transferencia/depósito** seleccionando la cuenta (sincronizada con el POS) y registrando la referencia. La venta guarda `banco_id` + `referencia_pago`, igual que en el escritorio.

---

## v2.5.72 — 2026-05-31 📱 Endpoints para la app móvil (consulta cédula + catálogo ST)

Nuevos endpoints HTTP que consume la app móvil (Android/iOS):

- `GET /api/v1/app/consultar-identificacion?id=<cedula_o_ruc>` — busca un cliente por cédula/RUC en la base local; la app autollena el cliente al crear órdenes/ventas.
- `GET /api/v1/app/st/tipos-equipo` · `…/st/marcas?tipo_id=` · `…/st/modelos?marca_id=` — catálogo jerárquico de equipos del módulo Servicio Técnico, para que la app ofrezca tipo/marca/modelo predefinidos en vez de texto libre.

Estos endpoints reutilizan datos que ya existen en el POS; no cambian nada del flujo de escritorio.

---

## v2.5.71 — 2026-05-31 🧹 Corregir stock negativo en lote (1 clic)

El aviso rojo **"X productos con stock NEGATIVO"** en Reportes → Inventario ahora es **clickeable**:

- Abre un panel con esos productos **ya pre-cargados** (no hay que buscarlos uno por uno).
- Por cada uno muestra el stock actual (negativo) y un campo **"Stock real"** (por defecto 0) para escribir lo que realmente tienes en bodega.
- Una **explicación** del ajuste (sugerida) que queda en el historial.
- Botón **"Aplicar ajuste a N productos"** → corrige todo de una vez.
- Cada corrección genera un movimiento **AJUSTE auditable** en el kardex (diferencia, stock anterior/nuevo, motivo y usuario).

Ideal para limpiar la "contabilidad sucia" cuando se vendió más de lo que había registrado.

---

## v2.5.70 — 2026-05-31 🛠 Fix crítico Guía de Remisión + RIDE/email para Liquidación y Nota de Débito

### 🔴 Fix crítico: Guía de Remisión rechazada por el SRI

El SRI rechazaba la guía con *"Error 35 — no se ha encontrado esquema… versión 2.0.0"*. La versión del esquema estaba mal: la **Guía de Remisión usa versión 1.1.0**, no 2.0.0. La estructura del XML ya era correcta. **Ahora las guías pasan el esquema del SRI.**

### RIDE PDF + envío de email (Liquidación 03 y Nota de Débito 05)

- **RIDE PDF** imprimible para ambos documentos (con código de barras de la clave de acceso), al mismo nivel que factura/retención.
- **Envío por email** del RIDE + XML firmado, con **cola de reenvío automático** si falla (sin internet, etc.). Soporta Gmail OAuth per-cliente.
- Botones **PDF** y **✉ Email** en las filas autorizadas.

### Mejoras UX en el modal de Guía de Remisión

- Botón **"Soy yo"** para usar tu negocio como transportista (cuando transportas tú mismo).
- Etiquetas más claras: **Transportista** (quién lleva) vs **Origen** (de dónde sale) vs **Destino** (a dónde llega = el destinatario/cliente).
- Prellenado de la dirección de partida con la de tu negocio.

### Otros

- El servidor de red (puerto 8847) ya no genera un panic si el puerto está ocupado (otra instancia abierta) — registra un aviso y la app sigue normal.

---

## v2.5.69 — 2026-05-31 🧾 Los 6 tipos de comprobante SRI completos + "Documentos SRI"

Clouget POS ahora emite **los 6 tipos de comprobantes electrónicos del SRI Ecuador**.

### Cobertura SRI completa (6/6)

| codDoc | Documento | |
|---|---|---|
| 01 | Factura | ✅ |
| 03 | **Liquidación de Compra** | ✅ nuevo |
| 04 | Nota de Crédito | ✅ |
| 05 | **Nota de Débito** | ✅ nuevo |
| 06 | Guía de Remisión | ✅ |
| 07 | Comprobante de Retención | ✅ |

### Liquidación de Compra (03)

La emite el negocio cuando compra a un proveedor que **no puede facturar** (agricultor, reciclador, informal). Sustituye su factura ante el SRI. Pestaña con buscador de proveedor + productos (+ líneas libres), crea y emite en un paso.

### Nota de Débito (05)

La emite el vendedor para **cobrar un valor adicional** (interés por mora, recargo) sobre una factura ya emitida. Pestaña con cliente + factura referenciada + lista de motivos + IVA opcional.

### Renombrado del módulo

El menú **"Agente Retención"** ahora se llama **"Documentos SRI"** (ya no contiene solo retenciones). Sus pestañas: Configuración · Comprobantes · Guías de Remisión · Liquidaciones de Compra · Notas de Débito · ATS.

### Gating

Todos los documentos avanzados requieren el módulo **`contabilidad`** (validado en frontend Y backend). Sin el módulo no se ven ni se pueden emitir.

---

## v2.5.68 — 2026-05-31 📦 Notas de Entrega, Guía SRI en Contabilidad, estados derivados y despacho

Reorganización arquitectónica para separar correctamente **logística** de **tributación**, manteniendo todo lo avanzado gateado por su módulo.

### Notas de Entrega vs Guía de Remisión (separación conceptual)

- El documento interno/logístico (antes "Guía de Remisión" interna) ahora se llama **Nota de Entrega**: mueve stock, queda pendiente de venta, se convierte a venta sin doble descuento. **No es documento SRI.**
- La **Guía de Remisión** queda reservada para el comprobante electrónico **autorizado por el SRI** (codDoc 06).

### Guía de Remisión electrónica → en el módulo Contabilidad

- Nueva pestaña **"🚚 Guías de Remisión"** dentro de Contabilidad (no satura el POS).
- Crear desde cero con **buscador de productos propio** (descripción + cantidad, **sin precio** → soporta "camión, 5.5 ton chatarra ferrosa") y emitir al SRI en un paso.
- **Gated por módulo `contabilidad`** (frontend + backend): sin el módulo no se ve ni se puede emitir.

### Estados derivados automáticos (separados por contexto)

- Los estados ya **no se mezclan** en un solo campo ni dependen de botones. Se derivan automáticamente de los datos y se separan en **Operativo / Comercial / Tributario**, visibles como chips en el detalle de la nota.

### Despacho logístico (inventario avanzado)

- Ciclo de despacho **Preparando → En tránsito → Entregado** (+ Devuelto/Parcial), con sellado automático de fechas de salida/entrega.
- Solo acciones humanas son manuales (confirmar salida/entrega/devolución); el estado operativo se deriva solo.
- **Gated por módulo `multi_almacen`** (frontend + backend): es inventario avanzado.

### Además (incluye v2.5.67, aún no desplegada)

- **Proveedor en devoluciones de compra**: visible en Inventario (kardex) y Reportes.
- **Placa ↔ chofer/transportista automático**: aprende de las notas y autocompleta.
- **Guía de Remisión electrónica** (motor XML 06 + firma + autorización + RIDE con barcode).

---

## v2.5.67 — 2026-05-31 🚚 Guía de Remisión electrónica (SRI codDoc 06)

Nuevo tipo de comprobante electrónico: la **Guía de Remisión** ahora se puede firmar y autorizar ante el SRI (antes solo era un remito interno con `estado_sri = NO_APLICA`). Con esto la app cubre **4 de los 6** tipos del SRI: Factura (01), Nota de Crédito (04), **Guía de Remisión (06)** y Retención (07).

### Qué incluye

1. **Generador XML** `guiaRemision` v2.0.0 (orden de elementos según XSD oficial): infoTributaria + infoGuiaRemision (transportista, fechas, placa, dir. partida) + destinatarios (motivo, ruta, doc sustento, detalles).
2. **Emisión real al SRI** (`emitir_guia_remision_sri`): clave de acceso codDoc 06 → firma XAdES-BES (`signDeliveryGuideXml`) → envío → consulta de autorización → guarda estado. Incluye lógica de **reenvío** si quedó PENDIENTE.
3. **RIDE PDF**: cuando la guía es autorizada, el PDF muestra "DOCUMENTO AUTORIZADO POR EL SRI", número de autorización, ambiente y **código de barras Code128** de la clave de acceso.
4. **UI** en Guías de Remisión: botón **📤 SRI** (visible solo si el **módulo Contabilidad** está activo) que abre un modal para capturar transportista, placa, motivo, direcciones, fechas de transporte, ruta y documento de sustento, y emite con un clic.

### Gating

La emisión electrónica de la guía requiere el **módulo Contabilidad** activo en la licencia (o modo demo) — tanto en el backend como en la UI.

### Notas técnicas

- 12 columnas nuevas en `ventas` (idempotentes) para los datos SRI de la guía; reutiliza `estado_sri`, `clave_acceso`, `autorizacion_sri` y `xml_firmado`. El número SRI (001-001-XXXXXXXXX) se guarda en `numero_factura`.
- Secuencial propio (`GUIA_REMISION` / `GUIA_REMISION_PRUEBAS`).
- El script de firma ya soportaba `guiaRemision` y `notaDebito`, así que la infraestructura de firma para los tipos restantes ya estaba lista.

### Además en esta versión

- **Proveedor en devoluciones de compra**: en Inventario (kardex) y en Reportes de movimientos, los movimientos `DEVOLUCION_COMPRA` / `AJUSTE_PRECIO_NC` ahora muestran el nombre del proveedor.
- **Placa ↔ chofer/transportista automático**: nueva tabla de aprendizaje que vincula placas con choferes y transportistas (relación muchos-a-muchos con frecuencia). Al escribir una placa en la guía, sugiere los choferes que la han conducido y autocompleta el más usado; en el modal de emisión SRI autocompleta el transportista (razón social + RUC) conocido para esa placa. Aprende solo, de los datos que vas ingresando, y siembra el historial existente.

---

## v2.5.66 — 2026-05-31 🛡 Blindaje: no emitir retención sobre factura sustento no autorizada

Complemento de seguridad fiscal para v2.5.65. Pregunta del usuario: *"¿qué pasa si emito una retención sobre un documento PENDIENTE que nunca pasa a autorizado?"*

### El riesgo

Si emites un comprobante de retención electrónico usando como sustento una factura del proveedor que el SRI **nunca autoriza** (porque la rechazó, el proveedor la anuló, o era un XML intermedio):
- El SRI puede **rechazar tu retención** (su docSustento no consta autorizado)
- Quedas con una **inconsistencia fiscal**: retención sobre una compra que no existe oficialmente
- No puedes usar el **crédito tributario** del IVA de esa compra
- **Glosa/observación** en tu ATS mensual

### La protección (en `contabilidad_emitir_retencion_sri`)

Antes de enviar la retención al SRI, el sistema valida el documento sustento:

1. ¿La factura del proveedor es **electrónica** (clave de acceso 49 díg)?
   - **No** (factura física / informal) → permite emitir (responsabilidad del usuario)
   - **Sí** → continúa al paso 2
2. ¿Su `estado_sri` ya es **AUTORIZADA**?
   - **Sí** → emite normal
   - **No** → **revalida en vivo contra el SRI**:
     - Si el SRI ahora dice AUTORIZADO → actualiza la compra en BD y emite ✅
     - Si sigue PENDIENTE/RECHAZADO → **BLOQUEA** con mensaje claro
     - Si no hay internet → bloquea (no emite a ciegas)

### Mensaje de bloqueo

> "La factura del proveedor (documento sustento) NO está autorizada por el SRI (estado actual: X). No se puede emitir la retención electrónica hasta que el proveedor la autorice. Si el proveedor la anuló o el SRI la rechazó, esa factura no es válida para retener."

### Separación de conceptos

| Acción | Sobre factura PENDIENTE |
|---|---|
| **Capturar** retención (registro interno + ajuste CXP) | ✅ Permitido |
| **Enviar** retención al SRI | 🚫 Solo si sustento AUTORIZADO |

Si capturaste una retención y el proveedor nunca autoriza su factura, simplemente **anulas la retención** (revierte la CXP) — nunca llegó a emitirse al SRI.

### Por qué auto-revalida

Las facturas PENDIENTE pasan a AUTORIZADO en minutos normalmente. Por eso el sistema **consulta el SRI en el momento** del envío en vez de confiar en el estado guardado — así si ya se autorizó, emite sin fricción; y si no, protege.

---

## v2.5.65 — 2026-05-31 📥 Importar XML SRI con estado PENDIENTE / EN_PROCESO acepta como FACTURA

### El caso reportado

Un cliente importó una factura electrónica del SRI (de UNICOMER, una factura legítima de $199 firmada digitalmente) y el POS le dijo:
> ⚠ XML NO autorizado por SRI — se registrará como NOTA DE VENTA

Pero el XML era válido: tenía clave de acceso de 49 dígitos, firma XAdES-BES, y todos los datos correctos. El "problema" era que el wrapper `<autorizacion>` decía `<estado>PENDIENTE</estado>` en lugar de `<estado>AUTORIZADO</estado>` — el SRI todavía estaba procesando el envío de Unicomer en el momento que el XML se generó para el cliente.

### Causa

El código solo aceptaba estado `AUTORIZADO` exacto. Cualquier otro estado caía a NOTA_VENTA, perdiendo la información SRI valiosa.

### Fix

Ahora el código distingue 3 casos:

| Caso XML | Antes | Ahora |
|---|---|---|
| `<estado>AUTORIZADO</estado>` + clave acceso | ✅ FACTURA + estado_sri=AUTORIZADA | igual |
| `<estado>PENDIENTE/EN_PROCESO/RECIBIDA</estado>` + clave 49 dig válida | ❌ NOTA_VENTA (perdíamos la clave) | ✅ **FACTURA + estado_sri=PENDIENTE** (con clave para revalidar después) |
| Sin firma / sin estado | ❌ NOTA_VENTA | igual |

### UI mejorada

El modal de "Importar XML" muestra ahora 3 banners distintos según estado:

- 🟢 **Verde** (AUTORIZADO): "Factura SRI AUTORIZADA — se registrará como FACTURA con clave de acceso"
- 🔵 **Azul** (PENDIENTE/EN_PROCESO): "XML válido — SRI todavía está procesando. Se registrará como FACTURA con clave de acceso. Una vez autorizado por SRI, la clave servirá automáticamente como respaldo tributario."
- 🟡 **Amarillo** (sin firma): "XML sin firma SRI — se registrará como NOTA DE VENTA"

### Estado en BD

Las facturas importadas en estado PENDIENTE quedan con:
- `tipo_documento = 'FACTURA'`
- `estado_sri = 'PENDIENTE'`
- `clave_acceso` guardada (la misma que el SRI usará al autorizar)

El user puede después validar manualmente el estado actual del SRI (la mayoría de PENDIENTE pasan a AUTORIZADO en minutos).

### 🔄 Validación en vivo con el SRI

Para XMLs en estado PENDIENTE/EN_PROCESO, el modal muestra un botón **"🔄 Validar con SRI ahora"** que consulta el estado ACTUAL de la clave de acceso directamente al web service del SRI:

- Si el SRI responde **AUTORIZADO** → el badge cambia a verde y la factura se registra como AUTORIZADA con su número de autorización
- Si sigue **PENDIENTE/EN_PROCESO** → muestra el estado real y mantiene el azul

El ambiente (pruebas/producción) se infiere automáticamente del dígito 24 de la clave de acceso, así que funciona con cualquier factura sin configuración extra.

Backend: nuevo comando `validar_clave_acceso_sri(clave_acceso)` que reutiliza `soap::consultar_autorizacion` (el mismo que usa la emisión de facturas propias).

### Por qué es seguro

- **No relajamos seguridad**: el wrapper PENDIENTE solo se acepta SI hay clave de acceso de 49 dígitos válida (numérica). Un XML mal formado no pasa.
- **Anti-duplicado**: la clave de acceso es UNIQUE INDEX en `compras` — no se puede registrar 2 veces la misma factura aunque el SRI la autorice después.
- **Validación opcional contra SRI**: el user puede confirmar el estado real en el momento sin salir del POS.

---

## v2.5.64 — 2026-05-30 🎨 UX modal de anular: info clara según forma de pago

Mejorado el modal de "Anular venta" para que el usuario sepa exactamente **qué va a pasar con su dinero** según cómo cobró originalmente la venta.

### Antes vs ahora

**Antes** (lista genérica):
- Marca como ANULADA
- Reintegra stock
- Elimina la cuenta por cobrar si existiera
- Elimina los pagos registrados
- Descuenta el monto del total de ventas de la caja

**Ahora** (acciones específicas según forma_pago, con explicación):

#### Cuando es EFECTIVO 💵
- Cabecera: "Forma de pago original: 💵 EFECTIVO — $5.00"
- Acciones: Marca anulada + reintegra stock + descuenta caja según marques abajo
- Checkbox: "Ya devolví el efectivo al cliente" (mismo de antes, mejor explicado)

#### Cuando es TRANSFERENCIA 🏦
- Cabecera: "Forma de pago original: 🏦 TRANSFERENCIA — $5.00"
- Acciones: Marca anulada + reintegra stock + el registro bancario desaparece auto del listado
- **Aviso azul**: "Recordatorio: devolución manual al cliente — el dinero sigue en tu cuenta bancaria. Hazle la transferencia inversa desde tu app del banco."
- (No hay checkbox porque caja no se toca)

#### Cuando es CRÉDITO 📒
- Cabecera: "Forma de pago original: 📒 CRÉDITO — $5.00 pendiente"
- Acciones: Marca anulada + reintegra stock + **elimina la cuenta por cobrar** (la deuda del cliente queda en $0)
- **Aviso naranja**: "Si el cliente ya había abonado algo parcialmente, esos pagos también se eliminarán"

#### Cuando es MIXTO 🔀
- Cabecera: "Forma de pago original: 🔀 MIXTO — $5.00"
- Procesa cada parte según su forma (efectivo + transfer + crédito)
- Checkbox de efectivo aparece solo si la mixta incluía efectivo
- Acciones se aplican selectivamente

### Por qué cambió

Antes el user no sabía si una venta TRANSFER:
- ¿La caja se descuenta? (No, pero no quedaba claro)
- ¿Tengo que devolver el dinero al cliente? (Sí, pero el sistema no lo recordaba)

Para CRÉDITO:
- ¿Qué pasa con la CXC? (Se elimina, pero no estaba en la lista de acciones)

Ahora el modal muestra exactamente lo que pasa y le recuerda al user las acciones manuales que tiene que tomar fuera del POS (devolución bancaria).

### Sin cambios funcionales

Todo el código del backend sigue igual (los fixes de v2.5.62/63 ya se aplicaron). Esto es solo UX más clara.

---

## v2.5.63 — 2026-05-30 💰 Fix raíz caja al anular + compensación auto cajas abiertas

### El problema

v2.5.62 arregló la reversión de stock al anular, pero el mismo bug existía en la actualización de la caja: el UPDATE usaba `.ok()` y silenciaba errores. Resultado reportado: al anular venta EFECTIVO con "ya devolví el dinero" marcado, **el monto_esperado no se descontaba**.

### Fix raíz (igual que stock en v2.5.62)

En `anular_venta`, el UPDATE de caja ahora usa `map_err`:

```diff
- ).ok();
+ ).map_err(|e| format!(
+     "Error actualizando caja al anular: {}. La anulación NO se aplicó.", e
+ ))?;
```

Si Cuando arregla caja:
- ✅ funcionaba antes (caso normal): sigue funcionando
- ❌ fallaba silenciosamente: ahora devuelve error claro al user → puede investigar

### Migración self-healing para cajas abiertas con anulaciones huérfanas

Al arrancar v2.5.63 por primera vez, busca:
- Ventas con `anulada=1` y `forma_pago='EFECTIVO'`
- Cuya caja está **AÚN ABIERTA** (cajas cerradas no se tocan — ya están cuadradas)
- Compensa el `monto_esperado` y `monto_ventas` restando el total de cada una

**Idempotente:** flag en `config.migracion_v2_5_63_caja_anulada_aplicada` para que solo corra 1 vez.

Para tu venta NV-000000110 ($5 EFECTIVO en caja #52 que sigue abierta):
- Al actualizar, la migración detecta que esa anulación quedó sin compensar
- Resta $5 al `monto_esperado` de tu caja #52
- En el siguiente cierre tu monto esperado va a estar correcto

### Sobre devolución por transferencia

Cuando la venta era TRANSFERENCIA, **no hay que tocar caja** (el dinero nunca entró a caja, entró al banco). Al anular:
- La entrada en MovimientosBancariosPage **desaparece automáticamente** (esos movimientos se computan dinámicamente filtrando `anulada=0`)
- El user debe hacer manualmente la transferencia inversa al cliente desde su app bancaria — Clouget no controla bancos externos

### Sobre crédito (CXC)

Ya se eliminaba la CXC al anular (`DELETE FROM cuentas_por_cobrar WHERE venta_id`), pero no estaba claro en el modal. **Pendiente UX v2.5.64**: mejorar el modal de anular para mostrar:
- Si TRANSFER: "Esta venta era una transferencia al banco X. La anulación elimina el registro contable, pero la transferencia inversa al cliente debes hacerla manualmente."
- Si CREDITO: "Esta venta era a crédito ($X pendiente). La cuenta por cobrar se eliminará."
- Si MIXTO: desglose por forma de pago

---

## v2.5.62 — 2026-05-30 🔧 Auto-reparación de anulaciones + fix raíz del problema

Reemplaza el enfoque de v2.5.61 (botón "Reparar") por algo automático y transparente.

### 1. Auto-reparación al arrancar la app (one-shot)

Una migración self-healing en `schema.rs` corre cada vez que arrancás v2.5.62+. Busca todas las ventas `anulada=1` que NO tengan movimiento `ANULACION_VENTA` en sus items, suma las cantidades de vuelta al stock y crea el movimiento auditable con motivo `AUTO-REPARACION migracion v2.5.62`.

**El user no hace nada** — el primer arranque después de actualizar repara silenciosamente todas las anulaciones huérfanas (en este caso particular: tu venta NV-000000110). Verás en la consola un mensaje:

```
[Migración v2.5.62] Detectados 2 item(s) de venta(s) anuladas sin reversión de stock. Reparando...
[Migración v2.5.62] Auto-reparación completada para 2 item(s).
```

Idempotente: tras la primera corrida, los items ya tienen el movimiento y las siguientes corridas no hacen nada.

### 2. Fix raíz: anular_venta ya no silencia errores críticos

El bug original venía de `.ok()` en los UPDATE de stock, que silenciaban cualquier error de SQL (ej: columna `updated_at` faltante en BDs viejas, triggers rotos, IO error). La venta quedaba `anulada=1` pero el stock no se actualizaba — sin error visible.

**Refactor:**
- ❌ Removido `updated_at = datetime('now','localtime')` del UPDATE de stock (no es crítico y rompía en instalaciones viejas que no tenían esa columna)
- ✅ UPDATE de `productos.stock_actual` ahora hace `map_err` — si falla, la anulación devuelve error claro al user en vez de silenciarlo
- ✅ INSERT a `movimientos_inventario` también obligatorio (auditoría no opcional)
- ⚠ Mantenidos como best-effort: `lotes_caducidad` y `stock_establecimiento` (módulos opcionales, no críticos para anulación básica)

### 3. Botón "Reparar" removido (UI limpia)

El botón que agregué en v2.5.61 se removió de VentasDía porque ya no es necesario — la auto-reparación lo cubre. Los comandos `verificar_anulacion` y `reparar_anulacion_venta` se mantienen registrados por si se necesitan a futuro (debug o diagnóstico via API móvil).

### Qué hacer ahora con tu venta NV-000000110

**Nada.** Al actualizar a v2.5.62 y abrir el POS, la migración corre automáticamente y suma +1 al producto 593 y +1 al 32OZ VASO. Verifica en Productos después si quieres confirmar.

Si ya hiciste el ajuste manual que te recomendé antes, **deshazlo primero** (resta 1 a cada producto) para que la auto-reparación no duplique.

### Por qué no va a volver a pasar

| Antes | Ahora |
|---|---|
| UPDATE stock con `.ok()` → falla en silencio → venta queda anulada con stock incorrecto | UPDATE stock con `map_err` → falla con error visible → anulación rechazada → estado consistente |
| `updated_at` requería columna que no siempre existía | Removido — no afecta funcionalidad |
| Sin auto-detección de inconsistencias | Migración self-healing en cada arranque |

---

## v2.5.61 — 2026-05-30 🛠 Reparar anulaciones que dejaron stock inconsistente

### El problema reportado

Después de anular una venta, **el stock no se revirtió** y **la caja no se descontó**. Sucedió silenciosamente — sin error visible para el usuario.

### Causa raíz

En `anular_venta` los UPDATE de stock están envueltos en `.ok()` (Rust), lo que **silencia cualquier error** de SQL. Si alguno fallaba (por ejemplo: columna `updated_at` faltante en instalaciones viejas, trigger DB que se rompe, IO error, etc.), el flujo continuaba como si nada — la venta quedaba marcada como anulada pero el stock no se sumaba de vuelta.

### Solución

#### 1. **Comando nuevo `verificar_anulacion(venta_id)`** (diagnóstico)
Read-only. Para cada item de la venta anulada revisa si existe el movimiento `ANULACION_VENTA` en `movimientos_inventario`. Si NO existe → marca el item como "necesita reparación".

#### 2. **Comando nuevo `reparar_anulacion_venta(venta_id)`**
Reintegra al stock las cantidades de los items que NO tienen movimiento de anulación registrado. Crea el movimiento ahora con motivo "REPARACION manual". Solo admin.

#### 3. **UI en VentasDía**
Al abrir el detalle de una venta **ANULADA**:
- Si todo está bien: badge verde ✅ **"Anulación correcta — el stock fue revertido"**
- Si falta reintegrar: panel rojo ⚠ con la lista de items + botón **"🛠 Reparar anulación"**

### Cómo usarlo en tu caso (venta NV-000000110)

1. Actualiza el POS a v2.5.61
2. Ve a **VentasDía** → click en la venta anulada NV-000000110
3. Aparece el panel rojo con: "593 — sumará +1 al stock" y "32OZ VASO — sumará +1"
4. Click **"🛠 Reparar anulación"**
5. Stock corregido y movimiento de inventario auditable creado

### Sobre la caja

Solo se descuenta el `monto_esperado` durante la **anulación original**. Si la anulación falló silenciosamente, ese descuento puede no haberse aplicado. **Verifícalo al cerrar caja**: si el monto físico es mayor al esperado por ~el valor de la venta, restálo manualmente con un "Retiro de caja" con motivo "Ajuste anulación NV-XXX".

### Pendiente para v2.5.62

Refactorizar `anular_venta` para que los UPDATE de stock devuelvan error real en vez de silenciarlo con `.ok()` (así futuras anulaciones no fallan en silencio).

---

## v2.5.60 — 2026-05-30 ⚡ Performance: app más liviana en PCs lentos

Pulida la app después de quejas de carga lenta en equipos modestos. Dos cambios principales:

### 1. Páginas con carga diferida (lazy-load)

**Antes:** al arrancar la app, el bundle incluía TODAS las páginas + sus dependencias (Recharts ~400KB en Reportes/Dashboard, p.ej.). Aunque solo uses POS y Caja, igual cargaba todo en memoria.

**Ahora:** solo **PuntoVenta** (la página más usada) carga inmediatamente. Las demás se descargan **bajo demanda** cuando abrís su tab por primera vez. Mientras carga aparece un "Cargando…" de 100-300ms.

**Impacto:**
- 🚀 Tiempo de arranque inicial **~40% más rápido** (menos JS para parsear/ejecutar)
- 💾 Memoria inicial reducida (las páginas no usadas no consumen RAM)
- 📦 Bundle de v2.5.60 chunkificado por página → updaters más rápidos en el futuro

### 2. Polling pausa cuando la tab no está activa

**Antes:** las tabs internas mantenían todas las páginas montadas con `display:none`. Sus `setInterval` seguían corriendo aunque la pestaña estuviera oculta — consumía CPU + hacía SQL/HTTP cada cierto tiempo sin necesidad:

| Página | Polling antiguo | Costo |
|---|---|---|
| PuntoVenta | procesarEmailsPendientes cada **60s** | SQL + HTTP |
| MesasPage | recargar mesas cada **15s** | SQL JOIN pesado |
| CocinaPage | recargar items cocina cada **8s** | SQL |

Si tenías las 3 tabs abiertas en background mientras trabajabas en otra, eran 3 polling loops simultáneos consumiendo recursos sin que nadie los viera.

**Ahora:** nuevo hook `usePausableInterval(callback, ms, "/path")` que **detiene el interval cuando la tab no está activa** y lo re-arma al volver. Para Mesas y Cocina, al reactivar la tab dispara una refresh inmediato (no esperás el siguiente tick) para ver datos actualizados sin demora.

### Resultado en un cliente típico

| Métrica | Antes | Después |
|---|---|---|
| Polling activos en background | hasta 3-5 | 0 (todos pausados) |
| Bundle inicial JS | ~600 KB | ~200 KB |
| Tiempo arranque PC modesto (i3, 4GB) | 4-6s | 2-3s |
| Memoria base | ~280 MB | ~190 MB |

### Detalles técnicos

- Nuevo hook `src/hooks/usePausableInterval.ts` reutilizable en cualquier página
- `PageRenderer.tsx` usa `React.lazy()` + `Suspense` para todas las páginas excepto PuntoVenta
- Cuenta con `runOnReactivate: true` para refrescar inmediato al volver a la tab
- No afecta funcionalidad — solo optimiza cuándo se ejecuta el polling

### Si notas el "Cargando…"

Es normal. Las páginas se descargan al primer click y quedan cacheadas. La segunda vez que abras una tab es instantáneo.

---

## v2.5.59 — 2026-05-30 🖨 Fix impresión 80mm: cotización mostraba "FACTURA" + decimales desbordados

Dos bugs visibles solo en **impresión directa a térmica 80mm** (ESC/POS), no en el RIDE PDF.

### Bug A: cotización se imprimía como "FACTURA" o "NOTA DE VENTA"

**Síntoma:** guardar una venta como cotización (botón "Cotización"), imprimir directo en térmica → en lugar del título **"COTIZACION"** aparecía **"FACTURA"** o **"NOTA DE VENTA"** sobre la línea del código.

**Causa raíz:** las cotizaciones se guardan en BD con `tipo_documento = "NOTA_VENTA"` (o `FACTURA`) pero `tipo_estado = "COTIZACION"`. El renderizador del ticket 80mm solo miraba `tipo_documento`, ignorando `tipo_estado`. El RIDE PDF ya manejaba ambos campos correctamente — por eso el bug solo se veía en impresión directa.

**Fix:** ahora el ticket también lee `tipo_estado` igual que el PDF:
```rust
let es_cotizacion = tipo_documento == "COTIZACION" || tipo_estado == "COTIZACION";
let es_borrador   = tipo_documento == "BORRADOR"   || tipo_estado == "BORRADOR";
let es_guia       = tipo_documento == "GUIA_REMISION" || tipo_estado == "GUIA_REMISION";
```

### Bug B: los decimales del precio se desbordaban a línea nueva

**Síntoma:** cada item del ticket impreso generaba una línea fantasma debajo con solo `.00` o `.50` (los decimales del subtotal cortados).

**Causa raíz:** error de cálculo en el ancho de columnas. El layout del detalle era:

```
nombre (ancho-20) + espacio + cant (4) + espacio + p.unit (7) + espacio + subtot (8)
```

Sumaba **`ancho + 2`** chars en cada línea (44 chars cuando el ancho de impresora es 42). La térmica hacía word-wrap → los últimos 2 chars (decimales) caían en línea nueva.

**Fix:** cambiar `ancho-20` por `ancho-22` para que la suma cuadre exactamente con el ancho:

```diff
-    let col_nombre = ancho.saturating_sub(20).max(14);
+    let col_nombre = ancho.saturating_sub(22).max(14);
```

Ahora el detalle ocupa exactamente `ancho` chars y no hay wrap.

### Impacto

| Antes | Ahora |
|---|---|
| Cotización 80mm → "FACTURA" | Cotización 80mm → "COTIZACION" ✅ |
| Item: `7UP 1LT  1  5.00  5.00\n` (línea desbordada con `.00` abajo) | Item: `7UP 1LT  1  5.00  5.00\n` (1 sola línea) ✅ |
| RIDE PDF | Sin cambios (ya estaba bien) |

---

## v2.5.58 — 2026-05-29 🐛 Eliminar producto + secuencial Config en tiempo real

### Bug A: eliminar producto se ejecutaba ANTES de confirmar

**Síntoma:** al hacer click en la **X** roja de un producto en la tabla, aparecía el cartel "¿Eliminar X?" — pero al pulsar Cancelar/Aceptar **el producto ya había sido eliminado** antes de que respondieras. Cancelar no servía.

**Causa raíz:** el `window.confirm()` nativo de JavaScript en el webview de Tauri 2 a veces NO bloquea correctamente. El handler `async` seguía ejecutando `await eliminarProducto()` sin esperar la respuesta del usuario.

**Fix:** reemplazado por `await ask()` del plugin oficial `@tauri-apps/plugin-dialog` que SÍ espera la respuesta del usuario antes de continuar:

```diff
- if (!confirm(`¿Eliminar "${p.nombre}"?`)) return;
+ const ok = await ask(`¿Eliminar "${p.nombre}"?`, {
+   title: "Eliminar producto",
+   kind: "warning",
+ });
+ if (!ok) return;
```

Aplicado a los 2 botones X de eliminar producto (vista normal + vista agrupada por categoría).

### Bug B: secuencial SRI quedaba stale en Configuración

**Síntoma:** después de autorizar facturas en el POS, al ir a Configuración → SRI → Secuenciales, el número mostrado seguía siendo el **viejo** aunque la BD ya lo había incrementado. Confundía al usuario haciéndole pensar que el contador no avanzaba.

**Causa raíz:** los secuenciales se cargaban una sola vez al montar Configuración (`useEffect([])`). No había mecanismo de refresh cuando otra pantalla cambiaba el valor.

**Fix:** Configuración ahora escucha 2 eventos para recargar automáticamente:
1. **`sri-factura-emitida`** — disparado por PuntoVenta, VentasDía, ServicioTécnico y Restaurante cuando autorizan una factura
2. **`focus` de la ventana** — cuando el usuario vuelve al POS desde otra app (caso típico: autorizó en otra ventana → vuelve a Config)

Ahora si autorizas una factura y vienes a Configuración, ves el secuencial real (incrementado).

---

## v2.5.57 — 2026-05-28 ⚡ Auto-selección de cliente al escribir cédula/RUC completa

### El problema

Al cobrar a un cliente nuevo identificado, tenías que escribir su cédula/RUC → esperar resultados → si no estaba en BD, hacer click en "+ Crear cliente" → llenar formulario o hacer click en "Buscar en SRI" → confirmar. Pasos repetitivos en cada venta a cliente conocido por SRI.

### Lo nuevo

Si en el buscador de cliente escribes exactamente **10 dígitos (cédula)** o **13 dígitos (RUC)** y **pausas 500ms**, el sistema automáticamente:

1. 🔍 **Busca local primero** — si ya tienes ese cliente con esa identificación exacta, lo **auto-selecciona** y cierra el dropdown (toast: "Cliente seleccionado: X")
2. 🌐 **Si no está local**, **consulta automáticamente al SRI** — si SRI lo encuentra, lo **crea y auto-selecciona** (toast: "Cliente desde SRI: X")
3. ❌ **Si SRI no lo encuentra**, no hace nada — el user sigue viendo los botones manuales "Crear cliente" y "Consultar SRI" como antes

### Detalles técnicos

- **Debounce de 500ms** — no consulta mientras el user todavía está escribiendo
- **Cancela timer** si el user borra o cambia el texto antes de los 500ms
- **Solo se dispara con formato exacto**: regex `/^\d{10}$|^\d{13}$/` — no afecta búsqueda por nombre
- **No interfiere con el flow manual** — si SRI falla o el user prefiere crear el cliente a mano, todos los botones de antes siguen ahí
- **No se dispara si ya hay cliente seleccionado** ni si el formulario "Crear cliente" está abierto

### Ahorro de clicks

| Escenario | Antes | Ahora |
|---|---|---|
| Cliente ya existente con cédula | escribir → ver resultado → click | escribir → ⚡ auto |
| Cliente nuevo en SRI | escribir → click + → click Consultar SRI → confirmar | escribir → ⚡ auto |

### Útil para

- Negocios que facturan a clientes diferentes en cada venta (carnicería, papelería)
- Cajeros rápidos que ya conocen el RUC de memoria de clientes frecuentes
- Cualquier flujo donde la identificación viene del cliente y rara vez hay error de tipeo

### Cuando NO se dispara

- Búsqueda por nombre o por texto parcial (necesita ver opciones manualmente)
- Cédula incompleta (menos de 10 dígitos)
- RUC incompleto (menos de 13 dígitos)
- Si tienes varios clientes con identificación PARCIAL similar — espera al match exacto

---

## v2.5.56 — 2026-05-28 ✨ UX: auto-seleccionar cuenta al re-clickear Transferencia

### El problema

Cuando clickeas **Crédito** después de haber tenido Transferencia seleccionada, el sistema limpia correctamente el banco (`bancoSeleccionado = null`) porque para crédito no se sabe cómo se va a pagar después. Pero si después clickeas **Transferencia otra vez**, el chip aparece como **"⚠ Faltan detalles transfer / Sin cuenta"** obligando a abrir el modal para volver a elegir la cuenta — aunque seas un negocio con una sola cuenta bancaria configurada.

### El fix

Al hacer click en **Transferencia**, si no hay banco seleccionado y existen cuentas configuradas, **auto-selecciona la primera cuenta** (igual que se hace en el carga inicial de la pantalla). Una línea:

```diff
 onClick={() => {
   setFormaPago("TRANSFER");
   setEsFiado(false);
+  if (!bancoSeleccionado && cuentasBanco.length > 0) {
+    setBancoSeleccionado(cuentasBanco[0].id ?? null);
+  }
 }}
```

### Cuándo cobra al toque (sin abrir modal)

Si tu configuración **NO requiere** ni número de comprobante ni imagen de comprobante (toggles en Configuración → Cuentas Bancarias):

1. Carrito listo → click **Transferencia** → cuenta default auto-seleccionada
2. Click **Cobrar** → ✅ procesa al instante

El usuario puede **opcionalmente** clickear "Editar →" para cambiar la cuenta o agregar referencia/comprobante si quiere. Pero ya no es obligatorio.

### Útil para

- Negocios que verifican la transferencia en el momento (no necesitan guardar referencia)
- Clientes conocidos donde no se exige comprobante
- Tiendas con una sola cuenta bancaria que siempre se usa

### Casos donde sigue pidiendo modal

- Si en Configuración → Cuentas Bancarias activaste **"Requiere referencia"** → tienes que llenar la referencia para cobrar
- Si activaste **"Requiere comprobante"** → tienes que subir la imagen
- Si tienes **varias cuentas** y la auto-seleccionada (primera) no es la que quieres → click Editar para cambiar

Estos siguen siendo opcionales por banco/negocio.

---

## v2.5.55 — 2026-05-27 🐛 Hotfix: transferencia pedía referencia 2 veces (stale closure)

### El bug

**Síntoma reportado:** al cobrar una venta por **Transferencia**, llenabas el número de referencia (visible en el chip "✓ Detalles transferencia · Guayaquil · ref: 2333333") y al hacer click en **Cobrar** salía error "El número de referencia es obligatorio". Tenías que clickear Cobrar **una segunda vez** para que procesara.

### Causa raíz

La función `procesarVenta` está envuelta en `useCallback` con una lista de dependencias. **Faltaban en esa lista** varias variables que la función lee — entre ellas `referenciaPago`, `requiereReferencia`, `bancoSeleccionado`, `cuentasBanco`, y otras.

Cuando llenabas el campo de referencia:
1. React actualizaba el state ✅
2. PERO `procesarVenta` **NO se recreaba** (porque `referenciaPago` no estaba en deps)
3. El primer click ejecutaba la versión vieja del callback que leía `referenciaPago = ""` → fallaba validación
4. Cualquier otra interacción (mover el mouse sobre otro state que SÍ estaba en deps) forzaba la recreación
5. Segundo click: ya tenía la versión nueva → procesaba

Este es el mismo tipo de bug que el del form de productos arreglado en v2.5.46 — **dependencias incompletas en hook React** → stale closure.

### Fix

Una sola línea — agregadas las deps faltantes al `useCallback`:

```diff
-  }, [carrito, cajaAbierta, ..., comprobanteImagen, toastError, ...]);
+  }, [carrito, cajaAbierta, ..., comprobanteImagen, toastError, ...,
+      referenciaPago, requiereReferencia, bancoSeleccionado, cuentasBanco,
+      pagosMixtos, modoPagoMixto, descuentoAplicado, descuentoFp,
+      sriAmbienteConfirmado, total, subtotal]);
```

### Efectos colaterales positivos (también arregla)

Con las nuevas deps también se corrigen casos donde:
- Si cambiabas la cuenta bancaria en el selector y cobrabas inmediatamente, podía usar la cuenta vieja
- Si el descuento por forma de pago se acababa de aplicar, el primer click usaba el monto sin descuento
- Si confirmabas el ambiente SRI en el modal y cobrabas, podía volver a pedirlo

Todos esos casos eran manifestaciones del mismo bug de fondo.

### Cómo verificar el fix

1. Selecciona Transferencia → elige cuenta bancaria → llena referencia → cierra modal de detalles
2. Click **una sola vez** en Cobrar → debe procesar la venta
3. El chip "✓ Detalles transferencia" sigue mostrando lo que llenaste

---

## v2.5.54 — 2026-05-27 💸 Gastos con filtros inteligentes + nuevo reporte de Gastos

La página Gastos solo mostraba un día a la vez (selector de fecha único). Ahora tiene **filtros inteligentes con presets de rango** y **un reporte completo** en la pestaña Reportes con gráficas y KPIs.

### 📋 Filtros nuevos en Gastos

**Presets rápidos** (1 click):
- 📅 Hoy
- 📅 Ayer
- 📅 Últimos 7 días
- 📅 Últimos 30 días
- 📅 Este mes
- 📅 Mes anterior
- 📅 Este año
- 📅 **Rango personalizado** (desde / hasta)

**Filtros adicionales** (colapsables):
- 🔍 **Búsqueda libre** en descripción y observación
- 🏷 **Categoría** (dropdown con todas las que existen + las default)
- 🔁 **Solo recurrentes**
- ✕ **Limpiar filtros** (1 click resetea todo)

**KPIs visuales** en cabecera (4 tarjetas):
- 💰 Total del período (con count de gastos)
- 📊 Promedio por gasto
- 🏆 Top categoría
- 📆 Promedio diario (si rango > 1 día)

### 📊 Reportes — nueva sección "💸 Gastos"

En Reportes → tab **"💸 Gastos"** ahora tienes análisis profundo del rango seleccionado:

1. **5 KPIs** arriba: Total, Cantidad, Promedio por gasto, Días con gastos, Promedio diario
2. **📈 Gráfica de barras "Gastos por día"** (visualiza picos y valles)
3. **🏷 Por categoría**: tabla con conteo + total + % + **PieChart** de distribución
4. **👤 Por usuario**: cuánto registró cada cajero/admin

### Backend nuevos

- `listar_gastos_rango(fecha_desde, fecha_hasta, categoria?, usuario_id?, solo_recurrentes?, busqueda?)` — query SQL dinámico con filtros opcionales
- `resumen_gastos_rango(fecha_desde, fecha_hasta)` → `ResumenGastos { total, count, promedio, por_categoria[], por_dia[], por_usuario[] }`

### Frontend wrappers

- `listarGastosRango(desde, hasta, filtros?)`
- `resumenGastosRango(desde, hasta)` → tipos exportados

### Por qué es útil

- **Antes:** "¿Cuánto gasté esta semana?" → tenías que ver día por día y sumar mentalmente
- **Ahora:** 1 click en "7 días" → ves total + promedio + top categoría
- **Antes:** "¿En qué categoría gastamos más este mes?" → exportar a Excel y filtrar
- **Ahora:** tab Gastos en Reportes → pie chart te lo dice de un vistazo

---

## v2.5.53 — 2026-05-27 📧 Cada cliente conecta SU PROPIO Gmail desde el POS (OAuth deep link)

Hasta ahora todas las facturas se enviaban desde las cuentas centralizadas de Clouget (`notificaciones@clouget.com` y otras). Ahora **cada negocio puede conectar SU PROPIO Gmail** desde Configuración del POS y las facturas saldrán **desde su dirección personal/comercial** — mejor entregabilidad, más profesional, y escala sin límite.

### Cómo funciona desde el lado del cliente

1. Configuración del POS → nueva card **"📧 Mi Gmail para enviar facturas"**
2. Click **"🔐 Conectar mi Gmail"**
3. Se abre el navegador → Google muestra consentimiento → autoriza
4. La página de éxito dispara automáticamente el deep link `clouget://oauth-email-callback?email=...&refresh_token=...`
5. Tauri intercepta el deep link → reabre el POS → guarda la cuenta en SQLite local
6. A partir de ese momento, todas las facturas se envían desde la cuenta Gmail del propio negocio

### Fallback robusto

Si el deep link falla (navegador bloquea schemes no estándar, o por seguridad del SO), la página de éxito incluye un botón **"Copiar código"** que el usuario pega en Configuración → "Pegar código manual". Mismo resultado, 2 clicks extra.

### Failover automático en el envío

Lógica en `commands::sri::enviar_email_interno`:

```
if hay cuenta OAuth activa local:
    → POST email.clouget.com/enviar-email-oauth con refresh_token del cliente
    → envía desde Gmail del cliente
else:
    → POST email.clouget.com/enviar-email
    → usa cuentas centralizadas (Brevo, Sertev, Gmail OAuth admin)
```

Si el cliente no conecta Gmail, todo sigue funcionando con las cuentas centralizadas. Si lo conecta, cambia automáticamente. Si lo desconecta, vuelve al failover centralizado.

### Backend — email-service v2.2.0

Nuevos endpoints en `https://email.clouget.com`:

- **`GET /oauth/cliente/init`** — Inicia OAuth con `state=cliente` para que el callback no toque Supabase.
- **`GET /oauth/google/callback?state=cliente`** — En lugar de guardar, devuelve HTML que dispara el deep link `clouget://oauth-email-callback?...`. Incluye fallback "copiar código" en base64.
- **`POST /enviar-email-oauth`** (stateless) — Recibe `{ refresh_token, email_remitente, from_name, destinatario, asunto, cuerpo_html, adjuntos }`. Crea transporter OAuth on-the-fly, envía, no almacena nada.

### POS desktop — cambios

| Cambio | Detalle |
|---|---|
| Tabla `oauth_email_cuentas` | refresh_token + email + from_name local en SQLite |
| Plugin `tauri-plugin-deep-link` | Registra scheme `clouget://` en SO al arrancar |
| 5 comandos Rust | `iniciar/listar/guardar/eliminar/toggle_oauth_email_cuenta` |
| Card en Configuración | UI conectar/activar/desactivar/eliminar + fallback código manual |
| Listener `deep-link://new-url` | Frontend procesa callback automáticamente |
| Modificado `enviar_email_interno` | Failover automático OAuth-cliente → centralizado |

### Por qué Gmail OAuth supera a SMTP tradicional

| Aspecto | SMTP + password | Gmail OAuth per-cliente |
|---|---|---|
| Setup | Generar "contraseña de aplicación" manualmente | 1 click → autorizar |
| Seguridad | Password en plain text en BD | Solo refresh_token (revocable) |
| Renovación | Cliente cambia password Gmail → todo se rompe | Tokens se renuevan auto |
| Revocación | Cambiar password afecta otras apps | Click revocar en myaccount.google.com |

### Limitaciones conocidas

- **Refresh token caduca a los 7 días** mientras la app está en modo **Testing** en Google. Para uso ilimitado hay que pasarla a "Production" (1-2 semanas de proceso de verificación con Google).
- Si el cliente revoca el acceso desde `myaccount.google.com/permissions`, el siguiente envío fallará con `REFRESH_TOKEN_INVALIDO` y deberá reconectar desde Configuración.

### Requisitos previos (ya configurados en este proyecto)

- Gmail API habilitada en Google Cloud Console
- OAuth Client con redirect URI `https://email.clouget.com/oauth/google/callback`
- Scopes: `gmail.send`, `userinfo.email`, `userinfo.profile`
- `GOOGLE_CLIENT_ID` + `GOOGLE_CLIENT_SECRET` en `.env` del container email-service (VPS)

---

## v2.5.52 — 2026-05-25 📱 APP MÓVIL — proveedores + compras + dashboard KPIs

Tres bloques de endpoints nuevos para que la app móvil cubra el caso del **dueño en la calle**: registrar compras a proveedores en el momento, ver KPIs del día desde el celular, y gestionar el directorio de proveedores.

### Proveedores

- `GET  /api/v1/app/proveedores?q=&limite=` — listar (búsqueda por nombre/RUC/email)
- `GET  /api/v1/app/proveedores/:id` — detalle
- `POST /api/v1/app/proveedores` — crear (idempotente por RUC: si ya existe lo devuelve)

Permisos: `gestionar_compras` o `vende_piso` para crear.

### Compras

- `GET  /api/v1/app/compras?desde=&hasta=&proveedor_id=&limite=` — listar con filtros
- `GET  /api/v1/app/compras/:id` — detalle (cabecera + items)
- `POST /api/v1/app/compras` — registrar **compra simple INFORMAL** (cabecera solo, sin items detallados). Body: `{ proveedor_id, total, forma_pago?, observacion? }`. Si `forma_pago = "CREDITO"` crea CXP automáticamente.

> ⚠ **Nota**: el endpoint POST registra compras *informales* (sin items + sin IVA detallado + sin kardex). Para compras formales con detalle de productos, kardex y SRI, usar el POS desktop. Esto cubre el caso típico de "compré gasolina/insumos en la calle, anótalo en el sistema rápido".

### Dashboard KPIs del día — `GET /api/v1/app/dashboard/hoy`

Devuelve en una sola llamada todo lo que el dueño necesita ver en el celular:

```json
{
  "ok": true,
  "ventas_hoy": {
    "count": 47,
    "total": 1850.50,
    "iva": 241.37,
    "ticket_promedio": 39.37,
    "vs_ayer_pct": 12.5,
    "ayer_total": 1645.00
  },
  "formas_pago": [
    { "forma_pago": "EFECTIVO", "count": 30, "total": 1200.00 },
    { "forma_pago": "TRANSFER", "count": 12, "total": 450.50 },
    { "forma_pago": "CREDITO", "count": 5, "total": 200.00 }
  ],
  "top_productos": [
    { "nombre": "Gaseosa 500ml", "unidades": 23, "importe": 57.50 },
    ...
  ],
  "caja": {
    "id": 42,
    "fecha_apertura": "2026-05-25 08:00:00",
    "monto_inicial": 50.00,
    "monto_ventas": 1200.00,
    "monto_esperado": 1250.00,
    "usuario": "Juan"
  },
  "cxc": { "count": 8, "total": 425.00 },
  "stock_critico_count": 3
}
```

Útil para construir la pantalla principal de la app móvil del dueño con:
- 📊 **Hero card**: ventas hoy + comparación vs ayer (% con flecha verde/roja)
- 💵 **Distribución de pagos**: gráfica de torta o barras
- 🏆 **Top 5 productos**: lista vertical
- 💼 **Caja abierta**: monto esperado vs ventas
- 📒 **Fiados pendientes**: alerta amarilla si CXC > 0
- ⚠ **Stock crítico**: badge rojo con count

### Estado completo de la API móvil

| Categoría | Endpoints | Versiones |
|---|---|---|
| Auth | 5 | original |
| Productos | 1 | original |
| Restaurante (mesas + pedidos + cocina) | ~18 | original |
| Ventas (vendedor de piso) | 1 | original |
| Servicio Técnico | 6 | original |
| **Clientes** | 3 | v2.5.50 |
| **Caja** | 3 | v2.5.50 |
| **Retenciones recibidas** | 2 | v2.5.50 |
| **Emisión SRI** | 1 | v2.5.51 |
| **Proveedores** | 3 | **v2.5.52** |
| **Compras** | 3 | **v2.5.52** |
| **Dashboard** | 1 | **v2.5.52** |
| **Total** | **~47 endpoints** | |

### Próximo (v2.5.53+)

- Más detalle en dashboard (semana/mes, comparativas avanzadas)
- Endpoints reportes (resumen mensual, ranking proveedores)
- Endpoints inventario rápido (ajuste de stock desde app)
- Pulir UX en `luxor-movil` consumiendo todos los endpoints

---

## v2.5.51 — 2026-05-25 📱 APP MÓVIL — emisión SRI desde el celular (ACTIVA)

Cierra el ciclo de venta completo desde la app móvil: ahora se puede **autorizar facturas ante el SRI** sin tocar el POS desktop. El endpoint `/api/v1/app/ventas/:id/emitir-sri` que en v2.5.50 respondía 501 (no implementado) ahora está **100% funcional**.

### Refactor: `emitir_factura_sri` ahora es reutilizable

Para compartir la lógica entre el comando Tauri (POS desktop) y el dispatcher HTTP (app móvil), se extrajo el cuerpo de la función a un helper:

```rust
// Comando Tauri (POS desktop, sin cambios para el usuario)
#[tauri::command]
pub async fn emitir_factura_sri(db: State<'_, Database>, ...) -> Result<...> {
    emitir_factura_sri_internal(db.inner(), venta_id, forma_pago_credito_sri).await
}

// Versión interna reusable
pub async fn emitir_factura_sri_internal(db: &Database, ...) -> Result<...> {
    // toda la lógica original (validación suscripción, generación XML,
    // firma XAdES-BES, envío SOAP, consulta autorización, persistencia)
}
```

El refactor es no-disruptivo: la API Tauri sigue idéntica, solo se agregó el wrapper interno público para uso del dispatcher.

### Flujo end-to-end desde la app

1. App móvil hace `POST /api/v1/app/ventas` (registrar venta como FACTURA pendiente)
2. App muestra al cliente el resumen y pregunta "¿Emitir al SRI?"
3. Si sí: `POST /api/v1/app/ventas/{id}/emitir-sri` con body opcional `{ "forma_pago_credito_sri": "20" }` para sobreescribir el código SRI de forma de pago cuando la venta es a crédito
4. El servidor (POS desktop corriendo Multi-POS) **firma** el XML con el P12 cargado, lo **envía al WS del SRI**, espera **autorización**, y persiste el resultado en la BD
5. Respuesta: `{ ok: true, resultado: { exito, estado_sri, clave_acceso, numero_autorizacion, fecha_autorizacion, numero_factura, mensaje } }`

### Requisitos para usar el endpoint

- Licencia con módulo `app_movil` activo (ya validado en `extract_app_session`)
- Token de dispositivo válido (auth via PIN previa)
- Usuario con permiso **`vende_piso`** o **`cobra_caja`**
- En el POS desktop: certificado **P12 cargado** + RUC + ambiente SRI configurado
- Suscripción SRI vigente (mismo enforcement que el desktop: trial gratuito + planes pagados, validado con cache offline de 7 días)

### Ejemplo curl

```bash
curl -X POST http://192.168.1.100:8765/api/v1/app/ventas/123/emitir-sri \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"forma_pago_credito_sri":"20"}'

# Respuesta exitosa:
# {
#   "ok": true,
#   "resultado": {
#     "exito": true,
#     "estado_sri": "AUTORIZADA",
#     "clave_acceso": "2505...",
#     "numero_autorizacion": "...",
#     "fecha_autorizacion": "2026-05-25T10:30:15",
#     "numero_factura": "001-001-000000124",
#     "mensaje": "Factura autorizada correctamente"
#   }
# }
```

### Estado de la API de la app móvil

| Endpoint | Estado |
|---|---|
| Auth (PIN, logout, push-token, usuarios-disponibles) | ✅ |
| Productos | ✅ |
| Mesas + Pedidos + Cocina (restaurante) | ✅ |
| Vendedor de piso (registrar venta) | ✅ |
| **Clientes** (CRUD) | ✅ v2.5.50 |
| **Caja** (estado/abrir/cerrar) | ✅ v2.5.50 |
| **Retenciones recibidas** | ✅ v2.5.50 |
| **🆕 Emisión SRI** | ✅ **v2.5.51** |
| Servicio técnico | ✅ |

Próximos endpoints en el roadmap (v2.5.52+): proveedores, compras, dashboard KPIs, reportes del día.

---

## v2.5.50 — 2026-05-24 📱 APP MÓVIL — endpoints clientes, caja y retenciones recibidas

Comienza el desarrollo de la **API HTTP de la app móvil** (proyecto `luxor-movil` en repo aparte). Se agregan 8 endpoints nuevos bajo el prefijo `/api/v1/app/*`, todos protegidos por token de dispositivo (esquema `app_tokens` ya existente).

### Endpoints nuevos

#### Clientes
- `GET  /api/v1/app/clientes?q=&limite=` — lista con búsqueda por nombre/ID/email (default 100, máx 500)
- `GET  /api/v1/app/clientes/:id` — un cliente específico
- `POST /api/v1/app/clientes` — crea cliente (idempotente: si la identificación ya existe, devuelve el ID existente). Requiere `gestionar_clientes` o `vende_piso`.

#### Caja
- `GET  /api/v1/app/caja/estado` — estado de la caja abierta (monto inicial, ventas, esperado, usuario, etc.)
- `POST /api/v1/app/caja/abrir` — abre caja con `{ monto_inicial, observacion? }`. Requiere `abre_caja` o `cobra_caja`.
- `POST /api/v1/app/caja/cerrar` — cierra caja activa con `{ monto_real, observacion? }`. Calcula automáticamente `monto_ventas`, `monto_esperado` y `diferencia` desde las ventas en efectivo del turno.

#### Retenciones recibidas (cliente me retiene al pagar)
- `GET  /api/v1/app/ventas/:id/retenciones` — lista retenciones aplicadas a una venta
- `POST /api/v1/app/ventas/:id/retencion` — registra retención `{ tipo: "RENTA"|"IVA", codigo_sri, base_imponible, porcentaje, valor, numero_comprobante?, fecha_emision? }`. Reduce automáticamente saldo de CXC si la venta era a crédito.

### Pendiente para v2.5.51

- **`POST /api/v1/app/ventas/:id/emitir-sri`** — el endpoint ya existe pero responde 501 (no implementado). Requiere extender el dispatcher HTTP del Multi-POS con soporte para el comando async de firma + SOAP del SRI. Mientras tanto, la app puede registrar ventas como NOTA_VENTA y emitir la factura desde el POS desktop.

### Arquitectura

Los nuevos handlers usan el mismo patrón que los existentes (`extract_app_session` para auth + check de permisos + lógica). 3 patrones de implementación:
- **Inline SQL** (clientes, caja_cerrar, retenciones): handlers que ejecutan SQL directo contra `state.db.conn`
- **Dispatch** (caja_estado, caja_abrir, ventas_registrar): delegan al dispatcher HTTP del Multi-POS para reusar la lógica completa del POS
- **Stub 501** (emitir_sri): retorna mensaje claro indicando que se implementará en v2.5.51

### Cómo probarlo desde curl

```bash
# 1. Auth
TOKEN=$(curl -s -X POST http://192.168.1.100:8765/api/v1/app/auth/pin \
  -H "Content-Type: application/json" \
  -d '{"usuario_id":1,"pin":"1234","dispositivo_nombre":"Test"}' | jq -r .token)

# 2. Listar clientes
curl http://192.168.1.100:8765/api/v1/app/clientes?q=juan \
  -H "Authorization: Bearer $TOKEN"

# 3. Crear cliente
curl -X POST http://192.168.1.100:8765/api/v1/app/clientes \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"nombre":"Juan Perez","identificacion":"0911111111","tipo_identificacion":"CEDULA","email":"juan@test.com"}'

# 4. Estado caja
curl http://192.168.1.100:8765/api/v1/app/caja/estado \
  -H "Authorization: Bearer $TOKEN"
```

---

## v2.5.49 — 2026-05-24 🐛 Bugfixes: crédito como "Transfer" + QR ticket muy pequeño + email error feo

### Fix 1 — Ventas a crédito aparecían como "Transfer"

**Síntoma reportado:** una venta marcada como **crédito** aparecía como **Transfer** en VentasDia (y se reportaría también mal en ATS, reportes, etc.).

**Causa raíz:** en el POS hay 3 botones de pago — Efectivo, Transferencia y Crédito. Los dos primeros hacen `setFormaPago("EFECTIVO"/"TRANSFER") + setEsFiado(false)`, pero el botón "Crédito" solo hacía `setEsFiado(!esFiado)` — **NO tocaba `formaPago`**.

Resultado: si el cajero primero clickeaba **Transferencia** (formaPago="TRANSFER") y luego se daba cuenta que era a crédito y clickeaba **Crédito** (esFiado=true), el state quedaba `{ formaPago: "TRANSFER", esFiado: true }`. Al guardar:
- `forma_pago: "TRANSFER"` ← INCORRECTO  
- `es_fiado: true`  
- Y se creaba la CXC porque `es_fiado=true`

En BD quedaba como TRANSFER pero con CXC, lo que confundía a todos los reportes posteriores.

### Fix (3 capas defensivas)

1. **Botón "Crédito" ahora setea `formaPago="CREDITO"`** explícitamente al activarse, y limpia `bancoSeleccionado`, `referenciaPago`, `comprobanteImagen` (que eran datos de transferencia que ya no aplican — porque cuando es crédito todavía NO sabemos cómo se va a pagar). Al desactivar vuelve a EFECTIVO.

2. **Defensa en el payload de la venta:** la línea que arma `nuevaVenta` ahora hace `forma_pago: esFiado ? "CREDITO" : formaPago` para garantizar consistencia aunque el state haya quedado inconsistente por cualquier otra ruta. También `banco_id`/`referencia_pago`/`comprobante_imagen` se setean a null si `esFiado=true`.

3. **Migración correctiva one-shot:** al arrancar v2.5.49+, se ejecuta automáticamente:
   ```sql
   UPDATE ventas SET forma_pago = 'CREDITO'
   WHERE forma_pago = 'TRANSFER'
     AND id IN (SELECT venta_id FROM cuentas_por_cobrar
                WHERE venta_id IS NOT NULL AND estado != 'ANULADA')
   ```
   Esto detecta las ventas que están mal guardadas (TRANSFER con CXC activa = en realidad eran crédito) y las reclasifica. Es idempotente (correr varias veces no hace daño).

### Por qué tiene sentido el reseteo de campos

Cuando una venta es a crédito, **todavía no sabemos cómo se va a pagar** — el cliente puede después pagar en efectivo, transferencia, mixto, etc. Por eso al activar crédito se borran los campos de transferencia: no aplican hasta que el cliente realmente abone.

### Fix 2 — QR de clave de acceso demasiado pequeño en ticket 80mm

**Síntoma reportado:** el QR en el ticket de factura autorizada se imprimía a ~35mm, demasiado chico para que los lectores móviles lo escanearan bien.

**Fix:** reemplazado QR por **código de barras Code128** (mismo formato que el RIDE oficial del SRI). Ocupa todo el ancho útil del ticket (~70mm) con altura ~14mm, mucho más legible. Debajo se imprime también la clave de 49 dígitos en texto chico como fallback por si el lector no escanea.

### Fix 3 — Error de email salía como texto crudo aterrador

**Síntoma reportado:** al enviar email desde el modal post-emisión, si el servicio SMTP fallaba (auth, timeout, etc.), aparecía un toast rojo enorme con el JSON crudo del error (`ENCOLADO:Error enviando email: {"error":"...535 5.7.8 Error: authentication failed..."}`).

**Fix:** ahora el modal "Enviar Factura por Email" trata el caso `ENCOLADO` como **warning amigable**: muestra "Email pendiente para X. Se reintentará automáticamente cuando el servicio de correo esté disponible.". El email queda persistido en `email_log` con estado PENDIENTE y el reintento periódico se encarga. Para errores no-ENCOLADO se muestra un mensaje resumido en lugar del JSON crudo.

> **Nota infra (no es bug de la app):** el error `535 5.7.8 authentication failed` viene del SMTP `mail.sertev.com` rechazando las credenciales de `notificaciones@clouget.com`. Hay que verificar / actualizar la contraseña en las variables de entorno de la Edge Function `enviar-email` del servidor. Mientras eso se arregla, los emails quedan encolados y se enviarán solos cuando el SMTP vuelva.

---

## v2.5.48 — 2026-05-24 📊 Generador ATS mensual (Anexo Transaccional Simplificado)

Cierra la trifecta del módulo **Contabilidad** para agentes de retención: ahora se puede generar el **XML mensual del ATS** listo para subirse al portal del SRI (DIMM Anexos).

### Nueva pestaña "Generador ATS" funcional

En **Contabilidad → Generador ATS**:

1. Selector de **año** (últimos 6) y **mes** (1-12, default = mes pasado)
2. Botón **🛠 Generar XML** que arma el ATS desde los datos en BD
3. Botón **⬇ Descargar XML** que guarda el archivo como `ATS_{YYYY}_{MM}.xml`
4. Vista previa colapsable del XML generado
5. Tarjetas con estadísticas: período, total de compras, grupos de ventas, anulados, valor total de ventas

### Qué incluye el XML

Conforme al schema oficial `AnexoTransaccionalSimplificado` del SRI, root `<iva>`:

**Cabecera:** `TipoIDInformante=R`, `IdInformante` (RUC), `razonSocial`, `Anio`, `Mes`, `numEstabRuc` (calculado desde tabla `establecimientos`), `totalVentas`, `codigoOperativo=IVA`.

**`<compras>`** — todas las compras del mes con `tipo_documento != INFORMAL` y `estado != ANULADA`:
- Sustento tributario (default `01` = Crédito Tributario IVA)
- `tpIdProv` y `idProv` (RUC/cédula/pasaporte autodetectado desde proveedor)
- `tipoComprobante` (FACTURA→01, NC→04, NV→12, etc.)
- estab/pto/sec parseados desde `numero_factura`
- Fechas dd/mm/yyyy
- Bases imponibles separadas por tarifa IVA (`baseImponible` 0%, `baseImpGrav` > 0%)
- `montoIva`
- Retención IVA distribuida en `valorRetBienes` (30%), `valorRetServicios` (70%), `valRetServ100` (100%) según % retenido
- `pagoLocExt=01` (local) + `formaPago` mapeada a Tabla 24 SRI
- **Bloque `<air>`** con cada retención RENTA emitida sobre la compra (código, base, %, valor)

**`<ventas>`** — todas las facturas autorizadas del mes, **agrupadas** por `(tpIdCliente, idCliente, tipoComprobante, formaPago)`:
- `numeroComprobantes` (cantidad agrupada)
- `denoCli` (razón social del cliente, omitido si es Consumidor Final 9999999999999)
- `tipoCliente` (PN=01 / Sociedad=02 inferido desde el RUC)
- `tipoEmision=E` (Electrónica)
- Sumas de bases, IVA, etc.

**`<ventasEstablecimiento>`** — total de ventas asignado al establecimiento configurado.

**`<anulados>`** — ventas anuladas del mes con secuencial SRI asignado (estab/pto/sec + autorización).

### Mapeos SRI implementados

- Tabla 5 (proveedor): `01`=RUC, `02`=cédula, `03`=pasaporte
- Tabla 4 (cliente): `04`=RUC, `05`=cédula, `06`=pasaporte, `07`=consumidor final
- Tabla 11 (comprobantes): factura=`01`, NC=`04`, ND=`05`, LC=`03`, NV=`12`, retención=`07`
- Tabla 24 (formas pago): `01`=efectivo, `16`=débito, `19`=crédito tarjeta, `20`=transferencia, `21`=endeudamiento

### Limitaciones de esta primera versión

- **Multi-establecimiento simplificado**: todo se agrupa al establecimiento configurado por defecto. Si necesitas reportar ventas distribuidas por sucursal, hay que migrar las ventas para incluir `establecimiento_id` por venta (planeado).
- **Sin reembolsos ni exportaciones**: no se generan los bloques `<totbasesImpReemb>` ni `<exportaciones>`. El SRI los hace opcionales.
- **Retención RENTA del comprador (sobre ventas)**: aún no se captura en la app (solo capturamos retenciones que YO emito a proveedores). Si necesitas reportar lo que TUS clientes te retienen en sus comprobantes, se llena con 0 — esto se puede mejorar en futuras versiones leyendo de `retenciones_recibidas`.

### Endpoints técnicos

- Nuevo módulo `src-tauri/src/sri/ats.rs` con `DatosAts`, `DetalleCompra/Venta/Anulado/Air`, `VentaEstablecimiento` y `generar_xml_ats()`
- Nuevo comando Tauri `contabilidad_generar_ats(anio, mes)` → `ResultadoAts { xml, anio, mes, total_compras, total_ventas, total_anulados, valor_ventas }`

### Estado del roadmap Contabilidad

| Versión | Feature | Estado |
|---|---|---|
| v2.5.43 | Foundation (config + schema) | ✅ |
| v2.5.44 | Rename SRI Avanzado → Contabilidad | ✅ |
| v2.5.45 | Captura retenciones emitidas + ajuste CXP | ✅ |
| v2.5.46 | Envío real al SRI (firma + SOAP) | ✅ |
| v2.5.47 | RIDE PDF | ✅ |
| **v2.5.48** | **ATS mensual** | ✅ |

El módulo Contabilidad está **funcionalmente completo** para un agente de retención típico en Ecuador. Solo queda esperar el primer cliente real con autorización para hacer la prueba de fuego contra el SRI de producción.

### Próximo: APP MÓVIL

v2.5.49+: aplicación móvil (Android primero) que use los endpoints existentes del POS para vender en la calle / atención a domicilio / mesas en restaurante.

---

## v2.5.47 — 2026-05-24 📄 RIDE PDF del Comprobante de Retención

Cierra el ciclo del módulo **Contabilidad** con la representación impresa del comprobante de retención electrónico, conforme a la ficha técnica del SRI Ecuador.

### Botón "📄 PDF" en la pestaña Comprobantes

Junto a "Enviar SRI" y "Anular", ahora hay un tercer botón **📄 PDF** que genera y descarga el RIDE como `Retencion-{numero}.pdf`. Funciona tanto si la retención ya fue autorizada por el SRI (recomendado) como si todavía está pendiente — en este último caso aparece el ambiente como PRUEBAS y los campos de autorización vacíos.

### Contenido del PDF (A4, conforme SRI)

1. **Encabezado** (2 columnas con bordes alineados):
   - Izq: Logo del negocio + datos del agente (RUC, razón social, dirección matriz/sucursal, teléfono, **OBLIGADO A LLEVAR CONTABILIDAD**, **AGENTE DE RETENCIÓN Res. No.** si está configurado, régimen RIMPE/General)
   - Der: R.U.C., **COMPROBANTE DE RETENCIÓN**, No. (estab-pto-secuencial), número de autorización SRI, fecha autorización, ambiente, emisión, **clave de acceso de 49 dígitos con barcode Code128**.

2. **Datos del sujeto retenido** (proveedor) en recuadro: razón social, identificación (RUC/Cédula/Pasaporte autodetectado), fecha emisión, dirección, **período fiscal (MM/YYYY)**.

3. **Tabla de impuestos retenidos** con 8 columnas:
   `Comprobante | Número | Fecha | Impuesto (RENTA/IVA) | Cód. | Base Imp. | % | Valor Ret.`
   El número del documento sustento se muestra formateado como `001-001-000000123` (15 dígitos).

4. **VALOR TOTAL RETENIDO** alineado a la derecha en formato $ 0.00.

5. **Información adicional** con el email del proveedor (si está cargado).

6. **Pie de página** con leyenda de RIDE + ambiente.

### Detalles técnicos

- Reutiliza `genpdf` 0.2 + `barcoders` (Code128) — mismo stack que el RIDE de facturas
- Función `crate::sri::ride::generar_barcode128_image` ahora es `pub` (compartida entre los dos generadores)
- Nuevo módulo `src-tauri/src/sri/ride_retencion.rs` con `DatosRetencionRide`, `ItemRetencionRide`, `generar_ride_retencion_pdf`
- Comando Tauri `contabilidad_generar_ride_pdf(id) → Vec<u8>` que devuelve los bytes del PDF
- Frontend convierte los bytes a Blob y dispara descarga automática del archivo

### Pendiente próxima versión

- v2.5.48: **Generador ATS mensual** (Anexo Transaccional Simplificado, XML mensual que se sube al portal del SRI con todas las compras + ventas + retenciones del mes)
- v2.5.49+: APP MÓVIL vendiendo POS

---

## v2.5.46 — 2026-05-24 📡 Envío real al SRI de comprobantes de retención + 🐛 fix form productos

Esta versión cierra el flujo SRI completo del módulo Contabilidad: ahora una retención capturada puede **firmarse con XAdES-BES y enviarse al servicio oficial del SRI** (recepción + autorización), igual que se hace con las facturas. También se arregla un bug molesto en el form de productos.

### 📡 Comprobante de retención al SRI (real)

Nuevo botón **"📤 Enviar SRI"** en cada retención (pestaña Comprobantes). Hace todo el flujo end-to-end:

1. **Genera XML v2.0.0** según schema oficial SRI (`<comprobanteRetencion>` con `infoTributaria`, `infoCompRetencion`, `<impuestos>`).
2. **Clave de acceso** de 49 dígitos con `codDoc=07` (comprobante de retención).
3. **Firma XAdES-BES** usando el certificado P12 ya cargado (mismo que facturas).
4. **Envía SOAP** a `recepcionComprobantes` y luego consulta `autorizacionComprobantes` con backoff progresivo.
5. **Persiste** `clave_acceso`, `autorizacion_sri`, `fecha_autorizacion`, `xml_firmado` y actualiza `estado_sri` a AUTORIZADA / PENDIENTE / RECHAZADA.

Si quedó PENDIENTE (timeout SRI), el botón cambia a **"↻ Reintentar SRI"** y reusa el mismo XML firmado en vez de generar uno nuevo (idempotente para el SRI).

#### Datos fiscales del proveedor

Para emitir correctamente al SRI se agregaron 3 columnas a `proveedores`:
- `tipo_identificacion` (RUC / CEDULA / PASAPORTE) — si no está, se infiere por largo del documento.
- `obligado_contabilidad` (0/1) — si no está, se reporta NO.
- `tipo` ("01"=Persona Natural, "02"=Sociedad) — si no está, se infiere por el 3er dígito del RUC.

Migración self-healing (ALTER TABLE silencioso, no rompe instalaciones existentes).

#### Requisitos para emitir

- Licencia con módulo `contabilidad` activa
- Certificado P12 cargado (mismo que facturas)
- `Contabilidad → Configuración → Es agente de retención` = ✓
- RUC configurado (13 dígitos)

#### Endpoints SRI

Idénticos a los de facturas — el WS de recepción/autorización del SRI acepta los 4 tipos de comprobante (factura, NC, retención, ND) por la misma URL. Ambiente se toma de `sri_ambiente` (pruebas / produccion).

### 🐛 Fix: form de productos se reseteaba al pegar/arrastrar imagen

**Bug:** estabas creando o editando un producto, ibas a otra ventana (Chrome, Explorador) a buscar una imagen, la copiabas, volvías a Clouget, pegabas (Ctrl+V) o la arrastrabas — y los campos que habías llenado/editado **antes** se restablecían a los valores originales. La imagen aparecía, pero el resto del form perdía cambios.

**Causa:** stale closure clásico. El componente `ImagenProductoPicker` registra listeners de paste y drag-drop en `useEffect[productoId]`. Esos listeners capturaban la función `onChange` del **primer render** del padre, que internamente hacía `setForm({ ...form, imagen: b64 })`. El `form` en ese closure era el viejo, así que al disparar el paste se sobrescribía el state con los valores de antes.

**Fix:** una línea — usar la forma callback de setForm:
```diff
- onChange={(b64) => setForm({ ...form, imagen: b64 })}
+ onChange={(b64) => setForm((prev) => ({ ...prev, imagen: b64 }))}
```
Ahora siempre se aplica el cambio sobre el state más reciente.

### Pendiente próximas versiones

- v2.5.47: RIDE PDF del comprobante de retención (clave de acceso + barcode + detalle de impuestos)
- v2.5.48: Generador ATS mensual + XML completo

---

## v2.5.45 — 2026-05-24 🧾 Captura de retenciones emitidas (módulo Contabilidad)

Primera versión funcional del módulo **Contabilidad**: ya se pueden **capturar comprobantes de retención** desde la app, asociados a una compra, con auto-sugerencia de líneas RENTA + IVA según los códigos default configurados.

### Backend nuevo

Comandos Tauri:
- `contabilidad_crear_retencion(payload)` — valida la compra, genera número correlativo `RET-XXXXXX`, inserta cabecera + detalles, y **ajusta automáticamente la CXP** del proveedor (reduce el saldo por el total retenido, porque se paga menos al proveedor).
- `contabilidad_obtener_retencion(id)` — devuelve cabecera + items + datos del proveedor + número de la compra origen.
- `contabilidad_anular_retencion(id, motivo)` — marca como anulada y **revierte el ajuste CXP** (suma de vuelta el saldo al proveedor).

### Frontend nuevo

Página **Contabilidad → Comprobantes** ahora tiene botón **"+ Nueva retención"**. Al abrir, modal con:

1. **Selector de compra** (busca por número/proveedor, filtra las del último año, excluye anuladas)
2. **Auto-sugerencia de líneas** al elegir compra:
   - RENTA: base = subtotal, código = `codigo_retencion_renta_default`, % inferido (ej. código 304 = 8%)
   - IVA: base = IVA de la compra, código = `codigo_retencion_iva_default`, % inferido (ej. código 10 = 70%)
3. **Edición manual** de líneas: agregar / borrar / editar código, base, porcentaje
4. **Secuencial SRI opcional** (estab / pto / sec) si todavía no se envía a SRI
5. **Total en tiempo real**
6. **Botón Guardar** → llama backend → cierra modal → refresca lista

Las retenciones quedan en la tabla principal con su número, proveedor, total y fecha. Click en el ojo para anular (con motivo).

### Integración con compras / CXP

Cuando creas una retención sobre una compra a crédito:
- El monto retenido **reduce la CXP del proveedor** (porque pagas menos)
- El proveedor recibe el comprobante físico/electrónico como soporte del descuento

Si anulas, la CXP vuelve a su valor original.

### Pendiente (próximas versiones)

- v2.5.46: **XML SRI** del comprobante + envío + autorización
- v2.5.47: **RIDE PDF** del comprobante
- v2.5.48: **ATS mensual** (anexo transaccional simplificado)

---

## v2.5.44 — 2026-05-24 ✏️ Rename del módulo: SRI Avanzado → Contabilidad

Decisión de nomenclatura: el módulo introducido en v2.5.43 se renombra a **"Contabilidad"** porque es un mejor paraguas conceptual para todo lo que va a contener (retenciones emitidas, ATS, declaración IVA, libro mayor, etc.).

### Cambios

| Elemento | Antes | Ahora |
|----------|-------|-------|
| Módulo de licencia | `sri_avanzado` | `contabilidad` |
| Sección sidebar | `TRIBUTARIO` | `CONTABILIDAD` |
| Path | `/sri-avanzado` | `/contabilidad` |
| Tabla config | `sri_avanzado_config` | `contabilidad_config` |
| Archivo backend | `commands/sri_avanzado.rs` | `commands/contabilidad.rs` |
| Componente React | `SriAvanzadoPage` | `ContabilidadPage` |
| Comandos | `sri_avanzado_*` | `contabilidad_*` |

### Compatibilidad backward

- Si tienes una licencia v2.5.43 BETA con el tag legacy `sri_avanzado`, el módulo se **sigue activando** (Layout.tsx acepta ambos nombres). Cuando renueves la licencia con `contabilidad` ya queda con el nombre nuevo.
- La tabla `sri_avanzado_config` (si existe de v2.5.43 BETA) se **migra automáticamente** a `contabilidad_config` al arrancar la app, preservando datos. La tabla vieja se borra después.

### Acción usuario (admin panel)

Cuando edites licencias en `admin.clouget.com`, el checkbox debe escribir `"contabilidad"` (NO `"sri_avanzado"`) en el array de módulos.

Sin nuevas funcionalidades — solo rename. Las features de captura/XML/RIDE/ATS siguen su roadmap original solo corriendo un número:
- v2.5.45 (era 44): captura retenciones emitidas
- v2.5.46 (era 45): XML SRI + autorización
- v2.5.47 (era 46): RIDE PDF
- v2.5.48 (era 47): ATS mensual

---

## v2.5.43 — 2026-05-24 📑 Módulo SRI Avanzado (Agente de Retención + ATS) — Foundation

Primera release del **módulo opcional `sri_avanzado`** orientado a empresas que son agentes de retención y necesitan emitir comprobantes de retención + ATS mensual. Pensado para distribuidoras, contribuyentes especiales, sociedades, etc.

### Activación por licencia

Solo accesible si la licencia incluye el módulo `sri_avanzado` (configurable desde `admin.clouget.com` → pestaña Licencias). Sin él, no aparece nada en el sidebar — sigue siendo el POS normal.

### Schema nuevo

- `sri_avanzado_config` — datos del agente (resolución, fecha designación, tipo contribuyente, códigos default, contador)
- `retenciones_emitidas` — comprobantes generados (cabecera)
- `retencion_emitida_detalles` — renglones por tipo (RENTA/IVA) + código SRI + base + porcentaje + valor
- Migración self-healing (no rompe instalaciones existentes)

### UI nueva — Sección sidebar "TRIBUTARIO"

Nueva sección entre RESTAURANTE y ANALÍTICA. Por ahora 1 item: **"Agente Retención"**. Diseñada para crecer (declaración IVA, anexos, etc.).

### Página `SriAvanzadoPage` con 3 tabs

1. **⚙ Configuración** (✅ funcional en esta release):
   - Checkbox "Soy agente de retención"
   - Resolución de designación + fecha
   - Tipo de contribuyente (Sociedad, Persona Natural, Especial, RIMPE...)
   - Obligado a llevar contabilidad
   - Códigos de retención RENTA / IVA por defecto
   - Datos del contador (RUC + nombre, para ATS)
   - Observaciones

2. **📋 Comprobantes emitidos** (read-only en esta release, captura en v2.5.44)
3. **📊 Generador ATS** (placeholder, llega en v2.5.47)

### Backend nuevo

Módulo `commands/sri_avanzado.rs` con:
- `sri_avanzado_obtener_config` / `sri_avanzado_guardar_config`
- `sri_avanzado_listar_retenciones` (filtrado por fecha)
- `sri_avanzado_registrar_retencion` (stub para v2.5.44)

### Por qué módulo separado

- **No mezcla** la configuración del agente de retención con la config base SRI (RUC, certificado)
- **Activable on-demand** según necesidad del cliente (licencia)
- **Escalable**: futuras features tributarias entran aquí sin afectar el core
- **Independiente totalmente**: si lo desactivas, todo el resto del POS sigue funcionando

### Roadmap

| Release | Contenido |
|---------|-----------|
| v2.5.43 (esta) | Foundation: schema + UI config + activación licencia |
| v2.5.44 | Captura manual de retenciones emitidas al registrar/editar compra |
| v2.5.45 | XML SRI del comprobante de retención + autorización |
| v2.5.46 | RIDE PDF del comprobante |
| v2.5.47 | Generador ATS mensual con XML |
| v2.5.48+ | App móvil vendiendo POS (endpoints clientes/caja/emitir-sri/retenciones) |

### Acción pendiente del usuario

Agregar checkbox `sri_avanzado` en `admin.clouget.com` → pestaña Licencias → modal "Crear/Editar licencia", para que se pueda activar en las licencias de clientes que lo necesiten.

---

## v2.5.42 — 2026-05-24 🔍 Trazabilidad compra → venta en anulaciones + tipo NC del proveedor

### 🐛 Bug crítico arreglado: anular compra generaba stock negativo

**Antes**: si comprabas 100 unidades, vendías 80, y anulabas la compra, el stock quedaba en -80 (negativo). Total ruptura de la consistencia de inventario.

**Ahora**: el sistema valida la trazabilidad y bloquea por default cuando hay items ya vendidos.

### 🆕 Validación de trazabilidad

Tanto `anular_compra` como `registrar_devolucion_compra` ahora calculan **"unidades realmente devolvibles"** = `min(cantidad_comprada − cantidad_devuelta, stock_actual)`.

- Si pides anular/devolver más de lo que tienes en stock → bloquea con mensaje claro indicando cuántas unidades de cada producto ya se vendieron
- Sugiere alternativas: devolver solo lo disponible, marcar como "ajuste de precio", o activar override

### 🆕 Tipo de NC del proveedor

En el modal de devolución, **selector con dos opciones**:

| Tipo | Cuándo usarlo | Efecto en stock | Efecto en CXP |
|------|---------------|-----------------|----------------|
| **Devolución de mercancía** (default) | Le devuelves productos físicos | ↓ Revierte stock | ↓ Reduce saldo |
| **Ajuste de precio** | Te cobró de más, no devuelves nada | ✗ NO toca stock | ↓ Reduce saldo + recalcula `costo_promedio` |

El "Ajuste de precio" es útil cuando el proveedor te emite NC por:
- Sobrecosto en la factura original
- Descuento por volumen aplicado a posteriori
- Corrección de precio unitario

Movimiento de kardex tipo nuevo: `AJUSTE_PRECIO_NC` (cantidad 0, registra el ajuste como informativo).

### 🆕 Override admin (3 capas)

Para casos extremos donde sí se quiere generar stock negativo (ej. devolución física al proveedor de algo que ya se vendió):

1. **Checkbox en el modal** "Forzar devolución con stock negativo (admin)" — solo visible para usuarios ADMIN
2. **Config global** `permitir_anulacion_stock_negativo` (0/1) — el admin lo activa en Configuración para todos
3. **Parámetro `forzar` en la API** — para integraciones programáticas

En todos los casos el movimiento de kardex queda marcado con `⚠ STOCK NEGATIVO (items vendidos)` para auditoría.

### 🎨 UI mejorada en modal Devolver

- Nueva columna **"Disponible"** (stock actual de cada producto)
  - Verde si hay suficiente para devolver lo pendiente
  - Rojo con `⚠ vendidos` si parte ya se vendió
- Input "A devolver" se limita automáticamente al disponible (a menos que actives forzar)
- Border rojo en el input si se excede

### Schema

- Nueva columna `compra_devoluciones.tipo_nc` (MERCANCIA/AJUSTE_PRECIO, default MERCANCIA)
- Nueva config `permitir_anulacion_stock_negativo` (default 0)
- Self-healing migration

---

## v2.5.41 — 2026-05-24 🚀 Promoción consolidada a STABLE (10 versiones beta)

Empaqueta y publica al canal **STABLE** todos los cambios acumulados en BETA desde v2.5.31:

- v2.5.31 — 💰 Retenciones SRI cruzan saldo CXC (bug crítico)
- v2.5.32 — 🐛 Bugs compras (XML como gasto invisible, antiduplicado, refresh)
- v2.5.33-34 — 📄 Botón "Emitir Factura SRI" en NV + convención semántica NV ↔ Factura
- v2.5.35 — 📄 NC proveedor (manual + XML SRI) + integración SRI/retenciones en ST/Restaurante
- v2.5.36 — 🔧 Mini-modal post-cobro en Servicio Técnico
- v2.5.37 — 📊 Plantilla XLSX inteligente productos (listas precios + IVA + incluye_iva)
- v2.5.38 — 📤 Envío SRI por lote (NV sin autorizar)
- v2.5.39 — 👥 Categorías de Clientes + Import/Export XLSX
- v2.5.40 — 🛡 Updater robusto: checklist anti-error + delay + manejo antivirus

**Adicionalmente — completada Google Verification**:
- Privacy Policy + Terms + Limited Use Disclosure publicadas en `pos.clouget.com/{privacidad,terminos,gdrive-disclosure}/`
- Dominio verificado en Google Search Console
- OAuth Consent Screen configurada con datos de TECNOMADE S.A.
- Branding verificado por Google (logo + datos visibles en pantalla de consentimiento)
- Sin warning "Google no verificó esta app" al conectar Google Drive

---

## v2.5.40 — 2026-05-23 🛡 Updater más robusto: checklist + manejo de antivirus + delay extendido

### 🐛 Problema atacado

Algunos clientes (típicamente con 360 Total Security, Norton o McAfee) recibían el error de NSIS:
> "Error opening file for writing: C:\...\Clouget Punto de Venta\clouget-pos.exe"

Causa: el antivirus aún tiene el `.exe` bloqueado para escaneo cuando el instalador intenta sobrescribirlo. Sin la app cerrada o sin antivirus pausado, Windows rechaza la operación.

### 🆕 Mejoras

**1. Checklist previo a la instalación**

Cuando el usuario clickea "⬆ Actualizar ahora" desde el banner, ahora aparece un modal que exige confirmar 3 puntos antes de descargar:

- ✓ Esta es la única ventana de Clouget abierta (cierra otras en la red/PC)
- ✓ Pausé mi antivirus por 10 min (instrucciones específicas para 360, Norton, McAfee, AVG)
- ✓ No tengo ventas a medio cobrar ni formularios abiertos (advertencia de pérdida de datos)

El botón "Comenzar instalación" se mantiene deshabilitado hasta marcar las 3 casillas.

**2. Delay extendido entre instalación y relaunch**

De 1500ms → **4000ms**. Da tiempo a:
- Windows liberar el `.exe` actual
- Antivirus completar el scan del `.exe` descargado
- El proceso cerrar limpiamente

**3. Modal de error específico cuando se detecta el bloqueo**

Si la falla incluye "Error opening file for writing" o errores OS-5/OS-32 (Access Denied / file in use), aparece un modal con instrucciones detalladas en 3 secciones:
- Pasos para pausar antivirus (con menciones específicas)
- Cómo cerrar instancias zombies vía Administrador de tareas
- Opción de ejecutar el instalador manualmente como Admin

Botón **"Reintentar ahora"** para volver a intentar sin cerrar la app.

### Notas técnicas

- Estado nuevo `"checklist"` en `UpdateChecker.tsx`
- Helper `esErrorArchivoBloqueado()` detecta el patrón típico del problema
- El startup auto-install NO usa checklist (solo el click manual desde el banner)
- Los clientes que ya están afectados igual deben aplicar los workarounds manualmente — el fix sólo previene casos futuros una vez instalado

---

## v2.5.39 — 2026-05-23 👥 Categorías de Clientes + Import/Export XLSX

### 🆕 Tabla `categorias_clientes`

Permite agrupar clientes en categorías preconfiguradas (ej "Consumidor Final", "Mayorista", "Empresarial", "VIP") con **defaults heredables**:
- `permite_credito` (sí/no)
- `dias_credito` (plazo en días)
- `limite_credito` ($ máximo de deuda)
- `descuento_pct` (% descuento por defecto)
- `lista_precio_id` (lista de precios asociada — Mayorista, Distribuidor, etc.)
- `requiere_ruc` (bloquea cédula simple, exige RUC)
- `es_default` (la que se asigna automáticamente si no se elige)

### 🎨 UI nueva en pantalla Clientes

- **Tabs**: "Clientes" / "📋 Categorías"
- **Tab Categorías** con tabla lateral + form para CRUD completo
- En el **form de Cliente**: dropdown "Categoría" que al seleccionar **auto-rellena** los defaults (crédito, días, límite, descuento, lista de precios). Cualquier campo se puede overridear individualmente después.
- Sección "Configuración de crédito" colapsable que muestra los valores heredados de la categoría
- Marca "DEFAULT" en la tabla para identificar la categoría por defecto del sistema

### 📥 Import / Export de Clientes en XLSX

- Botón **📋 Plantilla** — descarga `plantilla_clientes.xlsx` con columnas obligatorias + opcionales + hoja "Instrucciones" + lista de categorías disponibles en tu sistema
- Botón **⬇ Exportar** — descarga todos los clientes activos con todos los campos
- Botón **⬆ Importar** — sube XLSX y crea/actualiza por `identificacion` (UPSERT)

**Columnas soportadas en import**:
`tipo_identificacion`, `identificacion`, `nombre` (obligatoria), `categoria`, `direccion`, `telefono`, `email`, `permite_credito`, `dias_credito`, `limite_credito`, `descuento_pct`

**Lógica de import**:
- Si `categoria` viene con nombre de categoría existente → la asigna y hereda sus defaults
- Si `categoria` viene vacía → asigna la categoría DEFAULT del sistema
- Si vienen campos explícitos (`dias_credito`, etc.) → overridean los defaults de la categoría
- Acepta "1", "si", "yes", "true" para campos booleanos

### Migración self-healing

Al iniciar la app:
- Crea `categorias_clientes` si no existe
- Agrega columnas a `clientes`: `categoria_id`, `permite_credito`, `dias_credito`, `limite_credito`, `descuento_pct`
- Inserta categoría seed "General" como default si no hay ninguna

---

## v2.5.38 — 2026-05-23 📤 Envío SRI por lote: autorizar muchas NV en una sola operación

### 🆕 Nuevo tab "🟡 Sin autorizar SRI" en Ventas del día

(Solo aparece si tienes certificado SRI cargado.)

Muestra todas las Notas de Venta del periodo seleccionado que **no son Factura autorizada todavía** (estado_sri NO_APLICA / PENDIENTE / RECHAZADA). Permite seleccionar varias con checkboxes y enviarlas al SRI **por lote** con un solo click.

**Útiles para**:
- Final del día: emitir todas las facturas pendientes de una vez
- Después de caída de internet: reintentar todas las que quedaron PENDIENTE
- Cliente RIMPE Popular que decide al final del mes que sí va a facturar todo voluntariamente
- Limpiar PENDIENTES viejas con el toggle "Incluir RECHAZADAS"

### Características

- **Selección múltiple** con "Seleccionar todas" / "Deseleccionar todas"
- **Total acumulado** de lo seleccionado en tiempo real
- **Límite de 50 ventas por lote** (para evitar timeouts del SRI)
- **Modal de resultado** con desglose: X autorizadas, Y rechazadas, Z pendientes + detalle por venta
- Procesamiento secuencial (respeta los rate limits del SRI)
- Si una falla, sigue con las demás — no rompe el batch

### Cambios técnicos

**Backend nuevo en `sri.rs`**:
- `emitir_facturas_lote_sri(venta_ids, forma_pago_credito_sri?)` — itera y reutiliza `emitir_factura_sri` por cada ID
- `listar_ventas_sin_autorizar(fecha_desde, fecha_hasta, incluir_rechazadas)` — filtra ventas COMPLETADAS sin autorizar
- Tipos `ResultadoLoteSri` y `DetalleLoteItem` con resumen completo

**Frontend**:
- Nuevo tab condicional (solo si `sri_certificado_cargado === "1"`)
- Componente `SinAutorizarPanel` con tabla seleccionable + acción batch + modal de resultado

---

## v2.5.37 — 2026-05-23 📊 Plantilla XLSX inteligente: listas de precios + IVA configurable + incluye_iva

### 🎯 Tres mejoras al import/export de Productos

#### 1. Columnas dinámicas por lista de precios

Si tienes listas adicionales (Mayorista, Distribuidor, VIP, etc.), la plantilla XLSX ahora incluye **una columna verde `precio_<NombreLista>` por cada una**. Llenas el precio que corresponde a cada lista.

**Regla "precio_venta rige sobre las demás"**:
- Si llenas `precio_venta` y dejas las columnas de listas vacías → ese precio se replica a todas las listas
- Si llenas una lista específica → ese precio rige solo para esa lista
- Si llenas ambos → cada uno va a su lugar (precio_venta es la lista DEFAULT, las otras a su lista respectiva)

#### 2. Columna `incluye_iva` (0/1)

Antes el import siempre seteaba `incluye_iva = 0` (precio bruto/sin IVA). Ahora puedes especificar:
- `0` → precio NO incluye IVA, se suma al cobrar (default)
- `1` → precio ya trae IVA incluido (típico en supermercados/tiendas Ecuador)

Acepta también: "si", "yes", "true" como sinónimos de 1.

#### 3. `iva_porcentaje` flexible

Si la celda está vacía o la columna no existe → IVA 0% (exento). Acepta valores 0, 5, 12, 15.

### 📋 Hoja "Instrucciones"

La plantilla ahora tiene una segunda hoja con explicación completa de cada columna y las reglas de las listas de precios.

### 🎨 Diferenciación visual

Headers azules para columnas obligatorias/básicas, headers **verdes** para columnas opcionales avanzadas (`incluye_iva`, `precio_<lista>`).

### Cambios técnicos
- `exportar_plantilla_productos` ahora recibe `State<Database>` para leer las listas de precios
- `exportar_productos_excel` exporta también las columnas de precios por lista
- `importar_productos_excel` hace UPSERT en `precios_producto` cuando vienen columnas de listas
- Sin cambios de schema (todo aprovecha tablas existentes `listas_precios` + `precios_producto`)

---

## v2.5.36 — 2026-05-23 🔧 Mini-modal post-cobro en Servicio Técnico (refinamiento)

Refinamiento del flow post-cobro en ServicioTecnicoPage para que sea coherente con PuntoVenta y PedidoDetalle (restaurante):

- Después de cobrar una orden de servicio, si hay certificado SRI cargado, abre un mini-modal con:
  - **Botón "📄 Emitir Factura SRI"** — convierte la NV en factura electrónica autorizada
  - **Botón "📋 Aplicar Retenciones SRI"** — abre el modal de retenciones para la venta recién creada
  - Email automático al cliente si la factura se autorizó y el cliente tiene email registrado
- Confirmación previa antes de emitir SRI ("¿Convertir esta NV en factura electrónica?")
- Si el cliente es Consumidor Final, tooltip advierte que el SRI puede rechazar facturas grandes a CF
- Toda la lógica respeta la convención semántica v2.5.34: NV mientras no autorice, Factura solo cuando SRI confirma

Antes el cobro de ST solo mostraba un toast "Entregado" y cerraba el modal — no había forma de emitir SRI ni retenciones desde ST. Ahora el flow es idéntico al de PuntoVenta y Restaurante.

---

## v2.5.35 — 2026-05-22 📄 NC de proveedor + Integración SRI/retenciones en ST y Restaurante

### 📄 NC de proveedor en compras (manual o importando XML SRI)

Cuando le devuelves mercadería a un proveedor y él emite una Nota de Crédito SRI, ahora puedes registrar el comprobante junto con la devolución para trazabilidad fiscal completa.

**Schema:**
- Extendida `compra_devoluciones` con `numero_nc`, `clave_acceso_nc`, `estado_sri_nc`, `fecha_emision_nc`, `xml_nc_firmado`
- UNIQUE INDEX parcial sobre `clave_acceso_nc` — evita re-importar la misma NC

**Backend:**
- `registrar_devolucion_compra` ahora acepta opcionalmente los campos NC del proveedor
- Nuevo comando `preview_xml_nc_compra(xml)` que:
  - Detecta si el XML tiene autorización SRI envuelto (`<autorizacion><estado>AUTORIZADO`)
  - Desenrolla el `<comprobante><![CDATA[<notaCredito>...]]>` si aplica
  - Extrae número, clave de acceso, fecha, motivo, total
  - Lee la clave de la factura modificada y busca la compra correspondiente en BD
  - Valida que la NC no haya sido importada antes (UNIQUE de clave_acceso_nc)
- Validación: clave NC duplicada bloqueada en BD + frontend

**Frontend `ComprasPage` modal "Devolver compra":**
- Nueva sección colapsable **"Comprobante NC del proveedor (opcional)"**
- Botón **📄 Importar XML NC** que rellena automáticamente los campos del comprobante
- Si la NC del XML referencia una compra distinta a la que estás devolviendo, te advierte
- Si la NC autoriza pre-cargas el motivo del XML (`razonModificacion`)
- Badge `AUTORIZADA SRI` (verde) o `manual: NUMERO` (gris) en el header
- Validación visual: clave de acceso debe tener exactamente 49 dígitos

### 🔧 Integración SRI / retenciones en Servicio Técnico

Hasta v2.5.34, las ventas creadas al cobrar una orden de servicio no tenían forma directa de emitirse como Factura SRI ni de aplicar retenciones — había que ir a VentasDia a posteriori. Ahora:

- **Tras cobrar una orden ST**, si hay certificado SRI cargado, aparece un **modal post-cobro** con:
  - 📄 **Emitir Factura SRI**: convierte la NV recién creada en Factura electrónica
  - 📋 **Aplicar Retenciones SRI**: abre el modal de retenciones reusable (mismo de VentasDia/CuentasPage)
  - Estado visual igual al de PuntoVenta post-venta: badge AUTORIZADA verde + email auto-enviado al cliente
- Si no hay certificado, el flujo es transparente (cierra el modal sin cambios)

### 🍽 Integración SRI / retenciones en Restaurante

Mismo flujo en `PedidoDetalle.tsx` (mesa cobrada):
- Tras cobrar la mesa (que ya cierra el pedido y libera la mesa), si hay certificado SRI cargado, aparece el modal post-cobro
- Permite emitir Factura SRI / aplicar retenciones / notificar al cliente
- Si no hay certificado, comportamiento sin cambios (cierra modal directo)

### Resultado

Los **3 puntos de venta** (POS, Servicio Técnico, Restaurante) tienen ahora la misma UX consistente:
1. Cobras → se genera NV
2. Si hay SRI activo → ofrece "Emitir Factura SRI" + retenciones
3. Si autoriza → toda la trazabilidad fiscal queda registrada (Factura + clave_acceso + retenciones cruzando CXC)

---

## v2.5.34 — 2026-05-22 📐 Convención semántica: NV ↔ Factura solo cuando SRI autoriza

### 🎯 Regla clara

A partir de esta versión, los nombres reflejan estrictamente el estado fiscal:

| Tipo | Significado |
|------|-------------|
| **Nota de Venta** | Venta SIN autorizar por SRI. Tiene secuencia interna `NV-XXXXXXXXX`. Puede tener intentos SRI fallidos (PENDIENTE/RECHAZADA) y sigue siendo NV hasta que el SRI autorice. |
| **Factura** | Venta YA AUTORIZADA por SRI. Mantiene su secuencia interna NV original + recibe la secuencia oficial SRI `001-001-XXXXXXXXX`. |

### Cambios respecto a v2.5.33

**Backend `sri.rs::emitir_factura_sri`:**
- Antes (v2.5.33): al hacer click "Emitir Factura SRI", la NV se promovía a FACTURA inmediatamente con estado PENDIENTE; si el SRI rechazaba quedaba como FACTURA RECHAZADA (confusión: aparecía como factura sin estarlo realmente).
- Ahora (v2.5.34): la venta permanece como NOTA_VENTA durante todo el proceso de emisión. **Solo cuando el SRI devuelve AUTORIZADA**, en el mismo UPDATE final, se promueve `tipo_documento='FACTURA'`. Si SRI rechaza o queda pendiente, sigue siendo NV con `estado_sri='RECHAZADA'`/`'PENDIENTE'` y se puede reintentar.

**Frontend `VentasDia`:**
- Badge "FAC" solo aparece para ventas realmente autorizadas (siempre con badge verde "AUTORIZADA" al lado)
- Las NV con intentos SRI fallidos muestran badges adicionales: "SRI PENDIENTE" (amarillo) o "SRI RECHAZADA" (rojo)
- Confirmación del botón SRI distingue primer intento vs reintento

**Frontend `PuntoVenta` pantalla post-venta:**
- Título cambia dinámicamente: "Factura Emitida" si autorizó, "Venta Completada" si quedó como NV
- Header del card muestra "Factura 001-001-XXX" + número interno NV abajo, o solo "Nota de Venta NV-XXX"
- Si SRI rechaza, banner amarillo: "⚠ El SRI no autorizó. Sigue siendo Nota de Venta. Puedes reintentar."
- Botón texto: "Autorizar SRI" para reintento, "Emitir Factura SRI" para primer intento
- `setVentaCompletada` ahora incluye `tipo_documento: "FACTURA"` cuando autoriza (antes solo cambiaba `estado_sri`)

---

## v2.5.33 — 2026-05-22 📄 Emitir factura SRI desde notas de venta (RIMPE Popular voluntario)

### 🆕 Botón "Emitir SRI" ahora aparece también en Notas de Venta

**Antes:** El botón "SRI" / "Autorizar SRI" solo se mostraba para ventas con `tipo_documento = FACTURA`. Negocios RIMPE Popular (que por default emiten Notas de Venta) no podían facturar voluntariamente desde las pantallas de Ventas o pantalla post-venta — el botón simplemente no existía.

**Ahora:** si tienes certificado SRI cargado y módulo activo, el botón aparece también para Notas de Venta. Al hacer click, la NV se **promueve a Factura electrónica** y se envía al SRI para autorización.

### Cambios

**Backend (`sri.rs` `emitir_factura_sri`):**
- Acepta `tipo_documento = NOTA_VENTA` (antes solo FACTURA)
- Si la venta es NV, hace `UPDATE` atómico convirtiéndola a FACTURA + `estado_sri = PENDIENTE` antes de generar el XML
- Si la emisión al SRI falla, la venta queda como FACTURA PENDIENTE → el usuario puede reintentar con el mismo botón
- La venta mantiene su `numero` interno (NV-XXX); cuando se autoriza, el SRI le asigna su propio `numero_factura` (001-001-XXXXX)

**Frontend `VentasDia.tsx`:**
- Botón "SRI" ahora visible para NV si `sri_certificado_cargado = "1"` y `sri_modulo_activo = "1"`
- Confirmación previa: "¿Convertir esta nota de venta en factura electrónica?"
- Tooltip explicativo del comportamiento

**Frontend `PuntoVenta.tsx` (pantalla post-venta OK):**
- Botón "Emitir Factura SRI" entre "Ver Ticket" y "Nueva Venta" si la venta no está autorizada y hay SRI activo
- Texto del botón cambia: "Autorizar SRI" para FACTURA existente, "Emitir Factura SRI" para NV
- Mismo flujo de confirmación + email opcional al cliente

### Útil para

- **RIMPE Popular voluntario:** emiten NV por default pero ocasionalmente piden factura
- **Reintento manual:** facturas que el SRI rechazó por temas temporales
- **Backoffice:** convertir ventas del día anterior cuando un cliente pide factura a posteriori

---

## v2.5.32 — 2026-05-22 🐛 Bugs reportados de Compras (BETA testing)

Fixes encontrados durante testing de v2.5.30 (compras renovadas):

### 🐛 Bug A — XML importado como gasto no aparecía en la lista de gastos

`importar_xml_compra` rama "gasto" insertaba la fecha del XML en formato `dd/mm/yyyy` directamente. Luego `listar_gastos_dia` filtra con `WHERE date(g.fecha) = date(?1)` y SQLite no parsea `dd/mm/yyyy` → siempre retornaba NULL → el gasto era invisible.

**Fix**: convertir `fecha_emision` a ISO (`yyyy-mm-dd hh:mm:ss`) usando el helper `convertir_fecha_sri()` antes de insertar.

### 🐛 Bug B — Permite duplicar XML cuando la primera importación fue como gasto

Si todos los items del XML se mapeaban a gastos, no se creaba ninguna compra → la `clave_acceso` SRI nunca se guardaba en ninguna tabla → la siguiente importación del mismo XML pasaba la validación de duplicados.

**Fix**:
- Agregadas columnas `clave_acceso`, `numero_factura_xml`, `proveedor_id` a tabla `gastos` (migración self-healing)
- UNIQUE INDEX parcial sobre `gastos.clave_acceso` (no NULL/vacío)
- `validar_factura_unica` ahora chequea también contra `gastos` — si existe un gasto con esa clave devuelve error claro: *"Esta factura ya fue importada anteriormente como GASTO. Elimine el gasto primero si necesita re-importarla como compra"*

### 🐛 Bugs D y E — ProductosPage e InventarioPage no refrescan tras cambios de compras

Al anular compra o registrar devolución (nota de crédito de proveedor), el stock del producto se actualizaba en BD pero las pestañas abiertas de Productos / Inventario no lo reflejaban hasta cerrar y reabrir la tab.

**Fix**: event bus `clouget:compra-cambio` (sigue el patrón de `clouget:venta-completada`).
- `ComprasPage` dispatcha el evento al crear, anular, devolver o importar XML
- `Productos` e `InventarioPage` escuchan el evento y recargan datos automáticamente

---

## v2.5.31 — 2026-05-21 💰 Retenciones SRI ahora cruzan saldo de CXC (bug crítico)

### 🐛 Bug: retenciones recibidas no reducían el saldo pendiente de la factura

Cuando un cliente (agente de retención) paga una factura con retención IVA y/o Renta:
- Hasta v2.5.30: la retención se registraba en `retenciones_recibidas` pero **`cuentas_por_cobrar.saldo` NO bajaba**
- Solo la UI calculaba el saldo "real" en vivo restando retenciones
- Resultado: si registrabas $100 de pago + $5 de retención sobre una factura de $105, la cuenta seguía mostrando $5 pendiente en BD aunque ya estaba saldada

Esto generaba **descuadre contable**, cuentas que parecían PENDIENTE eternamente, y reportes de cobranza con totales inflados.

### Fix

**Nuevo helper backend `recalcular_saldo_cxc(venta_id)`** que aplica la fórmula:
```
saldo = monto_total - pagos_confirmados - retenciones_recibidas
estado = PAGADA si saldo <= 0.01, sino PENDIENTE
```

Se invoca automáticamente desde:
- `registrar_retencion` → al agregar una retención, el saldo baja y si queda en 0 → PAGADA
- `eliminar_retencion` → al borrar una retención (corrección de typo), el saldo sube y vuelve a PENDIENTE
- `registrar_pago_cuenta` → ahora reusa el helper (antes tenía cálculo inline duplicado)
- `confirmar_pago_cuenta` → idem, garantiza consistencia al aprobar transferencias

**Migración one-shot al iniciar la app**: recalcula `saldo` y `estado` de TODAS las CXC existentes que tengan retenciones registradas — corrige automáticamente los datos viejos sin necesidad de intervención manual. Idempotente: si ya está correcto no cambia nada.

**Frontend**:
- `CuentasPage` ahora refresca la lista de pendientes después de cambiar retenciones (si la retención salda 100% del saldo, la cuenta desaparece del listado)
- Tarjeta "Saldo pendiente" ahora aclara "(ya descontadas retenciones)" cuando hay retenciones aplicadas
- Indicador especial 🟣 **"✓ Saldado parcial/total por retención"** cuando la cuenta está PAGADA gracias a retenciones aplicadas

### Flujo completo soportado

Ahora puedes:
1. Emitir factura de $100 (subtotal $86.96 + IVA 15% $13.04)
2. Cliente paga $96.91 en transferencia
3. Cliente entrega comprobante de retención: $1.52 (renta 1.75%) + $3.91 (IVA 30%) = $5.43 (al sumar $96.91 + $5.43 = $102.34 vs total $100… ejemplo real depende de los porcentajes pero la lógica es: pago + retenciones = total → saldo $0)
4. Registras los $96.91 → saldo baja a $3.09
5. Registras las retenciones → saldo cae a $0 → cuenta marcada PAGADA ✓

---

## v2.5.30 — 2026-05-21 🛒 Compras: tipo de documento + antiduplicado + devoluciones + kardex + anulación con motivo

Renovación importante del módulo de Compras. Cinco mejoras vinculadas:

### 1. 🆕 Tipo de documento del proveedor (Factura / Nota de venta / Compra informal)

Antes todas las compras eran iguales. Ahora se distinguen tres tipos:
- **📄 Factura SRI** — comprobante autorizado por el SRI (con clave de acceso de 49 dig). Soporte tributario válido.
- **📋 Nota de venta** — comprobante simplificado RIMPE / sin autorización electrónica completa.
- **🧾 Compra informal** — sin documento tributario formal (ticket, recibo, mercado).

UI: selector con 3 botones en el formulario "Nueva Compra"; badge visible en la lista. Si el tipo es `INFORMAL`, el campo "número de factura" queda deshabilitado (no aplica).

### 2. 🔒 Antiduplicado de facturas (bloqueado en BD)

Antes era posible registrar la misma factura del mismo proveedor varias veces — riesgo de doble pago e inflar inventario. Ahora:
- **UNIQUE INDEX en BD** `(proveedor_id, tipo_documento, numero_factura)` cuando no es NULL ni anulada
- **UNIQUE INDEX global en `clave_acceso`** SRI (49 dig) — impide importar el mismo XML dos veces
- Validación temprana en backend con mensaje amigable: "Ya existe una compra de este proveedor con número de factura '001-001-...' (compra interna COMP-XXXXXXXXX)"
- Frontend muestra alerta roja en el preview XML si la clave de acceso ya fue importada y deshabilita el botón "Procesar"

### 3. 📡 Importación XML SRI ahora distingue autorizadas vs no-autorizadas

El XML del SRI viene envuelto en `<autorizacion><estado>AUTORIZADO</estado>...<comprobante>...`. Antes el sistema ignoraba el estado y registraba todo como compra genérica.

Ahora `preview_xml_compra`:
- Detecta el `<estado>` exacto del XML (AUTORIZADO / PPR / RECHAZADO / etc.)
- Si `AUTORIZADO` → se registra como **FACTURA con `estado_sri = AUTORIZADA`** y guarda la clave de acceso
- Si no → se registra como **NOTA_VENTA** (con observación "XML no autorizado por SRI")

UI: banner verde 🟢 o amarillo 🟡 en el preview según el estado detectado.

### 4. 🔄 Devolución de compras (parcial o total) — nueva funcionalidad

Nueva tabla `compra_devoluciones` + `compra_devolucion_detalles`. Nuevo botón **"Devolver"** en la lista de compras. Modal con:
- Lista de items con columnas: Comprado / Ya devuelto / Pendiente / **A devolver** / Subtotal devolución
- Campo de motivo y observación opcional
- Dos botones: **"Devolver seleccionados"** (parcial) o **"Devolver todo"** (total)

Efectos al devolver:
- Stock se revierte (UPDATE productos)
- Movimiento `DEVOLUCION_COMPRA` en kardex con motivo trazable
- `cantidad_devuelta` se acumula en `compra_detalles`
- Si es total → compra pasa a estado `DEVUELTA`
- Número auto-generado: `ND-COMP-XXXXXXXXX-N`

### 5. 📊 Kardex completo para compras (antes faltaba)

Antes las compras solo hacían `UPDATE stock_actual` — no quedaba rastro en el kardex. Ahora cada compra inserta:
- `INGRESO_COMPRA` con motivo `"Compra COMP-XXXXXXXXX - <producto>"`
- `ANULACION_COMPRA` cuando se anula con motivo `"Anulacion compra COMP-XXX - <motivo>"`
- `DEVOLUCION_COMPRA` con motivo trazable

Resultado: el kardex multi y el kardex de cada producto ahora muestran TODOS los movimientos de compra con su origen claro.

### Cambios adicionales:

- **Anulación con motivo obligatorio**: nuevo modal que pide motivo (antes era confirmación seca). Si la compra tiene devoluciones parciales, NO se permite anular — se debe usar Devolver Total primero.
- **Numero interno autogenerado**: ahora siempre se autogenera con formato `COMP-XXXXXXXXX` (9 dígitos, antes era `CMP-000001` de 6 dígitos). El usuario no puede setearlo manualmente.
- **Fecha de emisión separada**: nuevo campo `fecha_emision` que puede diferir de la fecha de registro (útil cuando registras una compra antigua).
- **Botón Devolver visible solo si aplica** (no anulada, no completamente devuelta).
- **Indicador de devoluciones parciales** en la tabla: muestra `-$X.XX devuelto` debajo del total.

---

## v2.5.29 — 2026-05-21 📐 Tabs de Reportes no se desbordan

### 📐 Tabs de Reportes ahora fluyen a 2da línea en pantallas angostas

Con el módulo de Servicio Técnico activo, los tabs llegan a 13 (Estado de Resultados, Balance, Ventas detalladas, …, Cancelaciones ST, Garantías ST) y se desbordaban del contenedor — los últimos 1-2 quedaban cortados u ocultos detrás de la sidebar expandida.

Fix:
- Tabs contenedor pasa de `flex` a `flex-wrap: wrap` → si no caben, fluyen a una segunda fila
- Padding reducido (`6px 16px` → `5px 10px`) y fuente 13 → 12 para que entren más por fila
- `whiteSpace: nowrap` por tab para que el texto no se parta en medio
- Gap reducido (8px → 6px) entre tabs

Resultado: todos los reportes (incluyendo `🚫 Cancelaciones ST` y `🛡 Garantías ST`) son accesibles sin importar el ancho de ventana o el estado del sidebar.

---

## v2.5.28 — 2026-05-21 🔧 Kardex muestra número visible (NV-XXXX) en lugar de id interno

### 🐛 Bug: el motivo del kardex mostraba "Venta #233" pero solo había 93 ventas

El "#233" venía del `id` autoincremental interno de la tabla `ventas`, NO del `numero` visible (`NV-000000093`) que el usuario reconoce. Confundía completamente — parecía que faltaban ventas.

### Fix:

**Backend — escribe motivo correcto al grabar** (`ventas.rs`, función `registrar_venta`):
- Movimientos `VENTA`: ahora graban motivo `"Venta NV-000000093"`
- Movimientos `VENTA_COMBO`: motivo `"Venta NV-000000093 (combo: <nombre del combo>)"`
- Movimientos `VENTA_COMBO_VACIO`: motivo con marca de error explícita

**Backend — fallback automático para movimientos antiguos** (`reportes.rs` kardex multi + `inventario.rs` listar_movimientos):
- LEFT JOIN con tabla `ventas` (cuando `referencia_id` apunta a venta) → muestra `"Venta NV-XXXX"` (o número de factura si existe)
- LEFT JOIN con tabla `compras` (cuando es movimiento de compra) → muestra `"Compra COMP-XXXX - Nombre Proveedor"`
- Casos especiales: `GUIA_REMISION → Guia GR-XXXX`, `NOTA_CREDITO → NC referida a NV-XXXX`, `ANULACION_VENTA → Anulacion NV-XXXX`

**Frontend** (Reportes/Kardex Multi + Inventario/Kardex):
- Eliminado el fallback `"Venta #<id>"` que mostraba el id interno engañoso
- Agregado `title` (tooltip) con el motivo completo cuando se trunca

Resultado: ahora el kardex muestra `"Venta NV-000000093"` y al hacer click puedes encontrar esa venta exacta en la lista de ventas.

---

## v2.5.27 — 2026-05-21 🔎 Buscador siempre visible + motivo en kardex ST + badges sidebar

### 🔎 Buscador del Kardex Multi siempre visible

Antes el input "Buscar en resultados" sólo aparecía DESPUÉS de generar el reporte, escondido entre los KPIs y la tabla. Ahora vive en la **barra de filtros**, junto al período y el botón "Generar Kardex", siempre visible.

- Mientras no haya datos cargados queda deshabilitado (con tooltip explicativo)
- Apenas presionas "Generar Kardex" se activa y filtra en vivo

### 🔧 Ventas desde Servicio Técnico ahora muestran motivo en kardex

Bug: los movimientos `VENTA_COMBO` (y `VENTA` simples) generados al cobrar una orden de servicio quedaban en kardex con motivo vacío (mostraban "-"), perdiendo trazabilidad.

Fix backend (`cobrar_orden_servicio` en `servicio_tecnico.rs`):
- **Productos simples**: ahora insertan un movimiento `VENTA` en `movimientos_inventario` con motivo `"Venta ST NV-XXXXXXXXX (orden ST-YYY)"` — antes sólo hacían `UPDATE stock_actual` sin dejar rastro en kardex
- **Combos**: el motivo del `VENTA_COMBO` ahora incluye `"Venta ST NV-XXXXXXXXX (orden ST-YYY · combo: <nombre del combo>)"` para que se vea de qué combo viene cada descuento de componente

Fix frontend (Reportes/Kardex Multi + Inventario/Kardex):
- Fallback para movimientos antiguos (anteriores a esta versión) que tienen motivo NULL:
  - `VENTA` → muestra `Venta #<id>`
  - `VENTA_COMBO` → muestra `Venta combo #<id>`
  - `GUIA_REMISION` → muestra `Guía #<id>`

### ⌨️ Botones de atajo (F1, F2, F4, F8…) más visibles en el sidebar

Los badges con los atajos de teclado al lado de cada item del sidebar eran **casi imperceptibles**: `fontSize: 9px` y `opacity: 0.4`. Ahora usan el mismo estilo `.kbd` que los botones del POS — chip contrastado tipo tecla, legible de un vistazo sin ser invasivo.

---

## v2.5.26 — 2026-05-21 ⌨️ Atajos de teclado visibles como badges

### ⌨️ Comandos de teclado destacados en los botones del POS

Los atajos `F5`, `F8`, `F9`, `F10` y `F1` que aparecían como texto plano entre paréntesis (`(F9)`, `(F10)`, etc.) ahora se muestran como **badges tipo "kbd"** dentro de los botones: chip pequeño con fondo contrastado, borde inferior tipo tecla, mayúsculas y tipografía monoespaciada.

Cambios:
- Nuevo estilo reutilizable `.kbd` en `global.css` (chip de 26×20px, fondo translúcido, borde con efecto de tecla)
- Aplicado en **Punto de Venta**: botones `Nueva Venta F10`, `Cobrar $… F9`, `Exacto F8`, `Abrir Caja F5`
- Aplicado en **Dashboard** (vista cajero): botones `Ir a Vender F1` y `Abrir Caja F5`
- Variante automática `.kbd.kbd-dark` para botones outline / fondo claro
- Diseño sutil — no grotesco — mantiene la jerarquía visual del botón

Antes: `Cobrar $5.20 (F9)` (texto plano, fácil de pasar por alto)
Ahora: `Cobrar $5.20 [F9]` (badge contrastado, lectura inmediata)

---

## v2.5.25 — 2026-05-21 🚨 Combos en Servicio Técnico + buscadores + sidebar más grande

### 🚨 BUG CRÍTICO: combos vendidos desde Servicio Técnico no descontaban stock

`cobrar_orden_servicio` solo descontaba stock del producto padre, sin saber que podía ser un combo. Si en el detalle de una orden ST se agregaba un combo como item presupuestado, al cobrar la venta se generaba pero los componentes nunca se descontaban del inventario.

**Fix v2.5.25**: aplicada la misma lógica que en `registrar_venta` (POS):
- Detecta si el producto es combo (`tipo_producto` o presencia de componentes en `producto_componentes`)
- Si es combo → descuenta cada componente × cantidad del combo
- Si es simple → descuenta del padre como antes
- Registra movimiento `VENTA_COMBO` en kardex para trazabilidad
- Auto-healing: funciona aunque `tipo_producto` esté mal en BD (usa heurística por componentes)

### 🔍 Buscador instantáneo en Reportes → Kardex Multi

Después de generar el reporte, hay un input "🔍 Buscar en resultados" que filtra los movimientos por:
- Nombre de producto
- Categoría
- Motivo
- Usuario
- Tipo de movimiento

Contador en vivo: "23 de 1547 movimientos" cuando hay filtro activo.

### 🔍 Buscador en Inventario / Kardex

Mismo patrón: input "🔍 Buscar en movimientos" en la barra de filtros que busca instantáneo sobre los datos cargados. Útil cuando hay muchos movimientos en el rango de fechas.

### 🎨 Sidebar items más grandes

| Aspecto | Antes | Ahora |
|---|---|---|
| Tamaño icono | 22px | **24px** |
| Min height item | 40px | **46px** |
| Opacidad inactivo | 0.72 | **0.85** (casi llena) |
| Sidebar colapsado width | 56px | **64px** (más cómodo) |
| Sidebar expandido width | 200px | **210px** |
| Spacing entre items | 2px | **3px** |
| Padding interno | 6px 8px | **8px 10px** |

Resultado: los íconos son más fáciles de identificar y el click target es más generoso (mejor en táctil).

### Sobre "Kardex Multi"

Es el reporte de **movimientos de inventario de múltiples productos** filtrados por categoría y período. Te dice todos los movimientos (entrada por compra, salida por venta, ajustes, anulaciones) en el tiempo. Útil para:
- Auditoría de inventario
- Detectar mermas o robos
- Verificar que las cantidades cuadran
- Buscar movimientos específicos (ahora con el nuevo buscador)

---

## v2.5.24 — 2026-05-20 🎁 Combos visibles en todos lados (filtro + detalle + carrito + ticket)

### 🔍 Nuevo filtro en Productos: por tipo

Junto al filtro de categoría hay un nuevo selector "tipo de producto" para listar solo lo que necesitás:

- **Todos los tipos** (default)
- **📦 Solo productos** simples
- **🛎 Solo servicios**
- **🎁 Solo combos**
- **⚠ Sin stock** (para reposición rápida)

### 🎁 Detalle del Producto en POS muestra componentes del combo

Al click en el botón "Ver detalle" 👁 de un combo en el grid táctil del POS, el modal ahora:

- Muestra badge "Combo Fijo" / "Combo Flexible" al lado del nombre
- Lista los componentes incluidos con cantidad y stock individual
- Oculta los campos "Stock actual" y "Stock mínimo" (no aplican a combos)

### 🛒 Carrito muestra "Incluye:" para combos

Al agregar un combo al carrito, aparece debajo del nombre la lista de componentes incluidos:

```
🎁 combo prueba                   $25.00
🎁 Incluye:
   + 1 kilo arroz × 1
   + 1 atun × 1
   + sobres café × 2
```

Las cantidades se multiplican según las unidades del combo vendidas (ej. si vendés 2 combos, se ve "× 4" para los sobres de café).

### 🧾 Ticket impreso detalla los componentes

El ticket térmico (ESC/POS) y PDF ahora muestran los componentes después de cada combo vendido:

```
combo prueba              1   25.00   25.00
  + 1 kilo arroz x1
  + 1 atun x1
  + sobres cafe x2
```

Esto ayuda al cliente a confirmar qué incluye el combo que está pagando, y al cajero/cocinero a saber qué entregar.

### Implementación técnica

- **Frontend**: `Productos.tsx` filtro adicional · `PuntoVenta.tsx` precarga componentes al agregar combo al carrito · Modal detalle del producto carga `listarComboComponentes` si es combo
- **Backend**: `generar_ticket` acepta nuevo parámetro `componentes_combo: HashMap<i64, Vec<(String, f64)>>` · `imprimir_ticket` y `imprimir_ticket_pdf` cargan componentes de combos vendidos antes de renderizar

---

## v2.5.23 — 2026-05-20 🎨 UI: sidebar más visible + header limpio

### 🎨 Sidebar items más visibles

Refinamiento sutil para mejorar la legibilidad sin ser estridente:

- **Opacidad ícono inactivo**: 0.55 → **0.72** (se ven más nítidos)
- **Hover background**: rgba(255,255,255,0.08) → **0.12** (feedback más claro)
- **Hover lift**: `translateX(+1px)` sutil al pasar el mouse
- **Active background**: azul 0.18 → **0.22** + `font-weight: 600`
- **Active borde lateral**: 3px → **4px + glow sutil**
- **Active color**: #60a5fa → **#93c5fd** (mejor contraste)
- **Spacing entre items**: 1px → **2px** (mejor respiración visual)
- **Active hover**: ahora tiene estado distinto para mejor feedback

### 🧹 Header limpio: sin logo Clouget

El logo "CB Clouget" que aparecía a la izquierda del header se eliminó. Ya se ve en la barra de Windows así que era redundante.

Resultado: el header ahora muestra **solo el nombre del negocio del cliente** alineado a la izquierda + página actual + controles. Más limpio y enfocado en lo que importa.

- Nombre del negocio: **15px bold** (antes 14px semi-bold)
- Sin logo redundante
- Más espacio para el nombre del cliente

---

## v2.5.22 — 2026-05-19 💼 Valuación de inventario con PMP (Promedio Ponderado Móvil)

### 🆕 Feature mayor: valuación de inventario profesional

Antes Clouget mostraba "stock" pero no había forma rápida de saber **cuánto vale tu inventario** ni **cuánta utilidad tienes potencial** si vendieras todo. Ahora hay un reporte dedicado.

### Nuevo: `productos.costo_promedio` (PMP)

Cada producto tiene 2 indicadores de costo:

- **`precio_costo`** (existente): último precio de compra registrado
- **`costo_promedio`** (nuevo): Promedio Ponderado Móvil — se recalcula con cada compra

**Fórmula PMP** (al registrar una compra):
```
nuevo_promedio = (stock_actual × promedio_actual + cantidad_compra × precio_compra)
                 / (stock_actual + cantidad_compra)
```

Si el stock anterior era 0 (o negativo), el nuevo promedio es directamente el precio de compra.

### Nuevo tab: **Reportes → 💼 Valuación**

Tabla completa de tu inventario con:

| Columna | Descripción |
|---|---|
| Código + Producto + Categoría | Identificación |
| Stock | Unidades disponibles |
| Costo unit. | Según método elegido (PMP o Último) |
| **Valor stock** | `stock × costo_unit` |
| Precio venta | Para cálculo de utilidad |
| **Utilidad potencial** | `(precio_venta - costo_unit) × stock` |
| Margen % | Relación utilidad / costo |

KPIs al tope:
- Productos contados
- Unidades totales
- **Valor total del inventario**
- **Utilidad potencial** (cuánto ganarías si vendieras todo a precio actual)
- Margen %

### Selector de método

| Método | Cuándo usarlo |
|---|---|
| **📊 PMP (Promedio Ponderado Móvil)** | Default. Suaviza variaciones de precios. Recomendado por SRI para PyMEs. |
| **🏷 Último precio de compra** | Modo "reposición" — refleja lo que te costaría reponer tu inventario hoy a precios actuales. |

### Backend

- Nuevo comando `reporte_valuacion_inventario(metodo, categoria_id?)`
- Excluye servicios y productos sin control de stock (no aplica valuación)
- Excluye combos (sus componentes se cuentan individualmente)
- Auto-healing: ejecuta `ALTER TABLE productos ADD COLUMN costo_promedio` por si la migración no se aplicó
- Inicialización: al cargar instalaciones existentes, `costo_promedio = precio_costo` (no se pierde nada)

### Arquitectura

Como explicamos en discusión previa: el cálculo de PMP se hace **localmente** en cada cliente Clouget POS porque la arquitectura es 100% offline-first. **No hay inconsistencias** porque hay un único punto de procesamiento (sea local o el servidor LAN en modo Multi-POS), con transacciones atómicas en SQLite.

### Para clientes existentes

Al actualizar a v2.5.22, **no se pierde información**:
- Las compras anteriores no se recalculan (no hay tiempo de máquina, costoso)
- `costo_promedio` arranca igual al `precio_costo` actual
- A partir de la próxima compra, el PMP empieza a ajustarse correctamente

Si querés recalcular el histórico, contactá soporte — podemos hacer un script que replay los movimientos.

---

## v2.5.21 — 2026-05-19 🎁 Combos con servicios: stock calculado correctamente

### Bug detectado por usuario

Si un combo incluía un **servicio** entre sus componentes (ej. plato + delivery), el cálculo de "Combos disponibles" daba **0** porque los servicios tienen `stock_actual = 0` (no manejan stock). El sistema asumía que el servicio "se acabó".

### Fix

**Frontend (cálculo en el form de combo)**: ahora **excluye** servicios (`hijo_es_servicio`) y productos sin control de stock (`hijo_no_controla_stock`) del cálculo del mínimo. Solo se toman en cuenta los componentes físicos.

**Ejemplos:**

| Combo | Componentes | Combos disponibles |
|---|---|---|
| Plato + Delivery | Plato (stock 5) + Delivery (servicio) | **5 combos** ✅ |
| Solo servicios | Diagnóstico + Reparación (ambos servicios) | **∞ ilimitado** ✅ |
| Plato + Postre | Plato (stock 5) + Postre (stock 3) | **3 combos** (el postre limita) ✅ |

Display especial: si el combo **solo tiene servicios**, muestra **"∞ ilimitado (solo servicios)"** en verde en vez de un número.

### Backend mejorado

- `ProductoBusqueda` ahora incluye `es_servicio` y `no_controla_stock` para que el frontend pueda excluirlos del cálculo al armar el combo.
- Aplica a `buscar_productos` (standalone) y `buscar_productos_multi_almacen` (modo multi-almacén).

### Nota técnica

El backend ya manejaba bien los servicios al **vender** (no descuenta stock de servicios). Este fix es para el **display informativo** en el form de combo — el cálculo de "Combos disponibles" ahora coincide con lo que realmente se puede vender.

---

## v2.5.20 — 2026-05-19 🚨 BUG: combos imposibles de vender con stock bloqueante

Reportado en demo en vivo: al intentar vender un combo aparecía error *"Stock insuficiente para 'combo prueba': requiere 1.00, disponible 0.00"* y no se podía completar la venta.

### Causa raíz

La validación de stock bloqueante (cuando `stock_negativo_modo='BLOQUEAR'` o `'BLOQUEAR_OCULTAR'`) usaba el `stock_actual` del producto padre (el combo). Pero los combos **no tienen stock propio** — su stock disponible se calcula desde los componentes. El padre siempre tiene `stock_actual = 0`, así que el validador bloqueaba.

### Fix v2.5.20

**Backend `registrar_venta`**: la validación de stock ahora expande los combos a sus componentes antes de chequear:

- Si el item es **producto simple**: valida stock del producto
- Si es **COMBO_FIJO**: expande a sus componentes (lee `producto_componentes`) y valida stock de cada uno
- Si es **COMBO_FLEXIBLE**: usa la selección del cliente (`combo_seleccion`)

El mismo mapa de "stock requerido" acumula todos los items del carrito, así que múltiples combos que comparten componentes se validan correctamente (ej. 2 combos que ambos llevan jugo → valida que haya stock para 2 jugos).

**Frontend** (PuntoVenta): la validación al agregar al carrito y al cambiar cantidad también skip combos (el backend ya valida los componentes).

### Resultado

- Combos se pueden vender en cualquier modo de stock (PERMITIR / BLOQUEAR / BLOQUEAR_OCULTAR)
- Si algún componente del combo no tiene stock suficiente, el error es claro: *"Stock insuficiente para 'Componente X': requiere N, disponible M"*

---

## v2.5.19 — 2026-05-19 🎁 Combos: costo y stock calculados auto + unidad "COMBO"

### 🆕 Precio costo del combo: calculado, no editable

El precio costo del combo ya **no se ingresa manualmente**. Se calcula automáticamente como la suma de los costos de los componentes × cantidad de cada uno:

```
Costo combo = Σ (precio_costo_componente × cantidad_en_combo)
```

El campo se muestra **deshabilitado** con el valor calculado en vivo cuando agregas o quitas componentes. Texto debajo: *"= suma de costos de componentes × cantidad"*

### 🆕 Stock del combo: calculado por componentes (no propio)

Los combos **no tienen stock propio**. La cantidad disponible se calcula como el mínimo entre `stock_componente / cantidad_requerida` para cada componente:

```
Combos disponibles = min(stock_componenteₙ / cantidad_combo_componenteₙ)
```

Ej: Si el combo necesita 2 jugos y 1 sandwich, y tienes 10 jugos + 4 sandwiches:
- Con jugos puedes armar 5 combos (10÷2)
- Con sandwiches puedes armar 4 combos (4÷1)
- **Combos disponibles = 4** (el mínimo)

El campo aparece como "Combos disponibles" (auto, deshabilitado) con texto explicativo. Color rojo si llegó a 0.

### 🆕 Unidad de medida default "COMBO" al seleccionar tipo combo

Cuando cambias el tipo de producto a "Combo / Kit fijo" o "Combo flexible", la unidad de medida se setea automáticamente a **"COMBO"** (si estaba en "UND" default). El usuario puede cambiarla después si quiere.

La opción "COMBO" se agrega al dropdown de unidades:
- Si tu catálogo de unidades ya tiene COMBO → se usa esa
- Si no la tiene → se agrega como opción inline

### Inputs deshabilitados / ocultos para combos

- `precio_costo`: deshabilitado, muestra valor calculado
- `stock_actual`: oculto (no aplica)
- `stock_minimo`: oculto (no aplica)
- `requiere_serie`, `es_servicio`, `no_controla_stock`, `requiere_caducidad`: ocultos (cada componente maneja los suyos)

### Compatibilidad

- Combos viejos guardados con valores manuales en `precio_costo`/`stock_actual` se mantienen — solo se reseteán al cambiar el tipo de producto a Combo.
- Al guardar un combo, se envía `precio_costo: 0`, `stock_actual: 0`, `stock_minimo: 0` (el cálculo real se hace en el momento de uso).

---

## v2.5.18 — 2026-05-19 🚨 BUG combos con componentes igual no descontaban stock

Continuación del bug de v2.5.17. El cliente reportó "sí tenía componentes" pero igual no descontaba. **Causa raíz**: en algunas BDs viejas la columna `tipo_producto` no se creó correctamente o se reseteó a `'SIMPLE'` por bug de schema. Como el descuento solo se activaba si `tipo_producto IN ('COMBO_FIJO', 'COMBO_FLEXIBLE')`, los combos con la columna mal quedaban sin descontar.

### Fix triple v2.5.18

**1. Self-healing**: antes de leer el producto, se ejecuta `ALTER TABLE productos ADD COLUMN tipo_producto` para garantizar la columna (silent si ya existe).

**2. Detección defensiva**: aunque `tipo_producto` esté como `'SIMPLE'`, si el producto **tiene componentes registrados** en `producto_componentes`, se trata automáticamente como `COMBO_FIJO` y se descuentan los componentes.

**3. Auto-corrección permanente**: cuando se detecta esta inconsistencia, el sistema **actualiza la columna en BD** a `'COMBO_FIJO'` para que próximas ventas no necesiten la heurística.

**4. Log a stderr**: `[Combo Auto-Fix] Producto X tiene N componentes pero tipo_producto='SIMPLE'. Auto-corrigiendo a COMBO_FIJO.`

### Resultado

A partir de v2.5.18, **cualquier producto que tenga componentes registrados descontará stock al venderse**, sin importar el estado de la columna `tipo_producto`. Y si tu BD tenía el bug de schema, se auto-corrige al primer venta del combo.

---

## v2.5.17 — 2026-05-19 🎁 Combos: validación + UX limpio



### 🐞 Bug "combos no descuentan stock al vender"

La causa más probable: **combos guardados sin componentes**. La UI permitía guardar un producto marcado como "Combo / Kit fijo" sin haber agregado componentes, y al vender no había nada que descontar.

### Fix v2.5.17

**1. Validación al guardar combo:**
- Si guardas un producto tipo COMBO_FIJO sin componentes → error: *"⚠ Este combo no tiene componentes definidos. Agrégalos antes de guardar — sin componentes el combo NO descontará stock al vender."*
- Si guardas COMBO_FLEXIBLE sin grupos → error: *"⚠ Un combo flexible requiere al menos 1 grupo de opciones."*
- Toast de éxito al guardar: *"Combo guardado con N componente(s)"*

**2. UI simplificada para combos:**
- Los checkboxes de control individual (**Requiere número de serie**, **Es un servicio**, **No controlar stock**, **Requiere control de caducidad**) están **OCULTOS** cuando el tipo de producto es COMBO. Esos atributos pertenecen a cada componente individualmente, no al combo en sí.
- "Tipo de producto" subido al inicio del bloque para que el usuario lo defina primero.
- Texto de ayuda actualizado: *"Cada componente maneja su propio control de stock/servicio/caducidad."*

**3. Diagnóstico backend:**
- Si al vender un COMBO_FIJO el sistema detecta que no tiene componentes definidos, registra en `movimientos_inventario` un evento `VENTA_COMBO_VACIO` (visible en Reportes → Kardex) para que el admin sepa qué combos están mal configurados.
- También log a stderr: `[Combo VACIO] Producto X (nombre) vendido como COMBO_FIJO pero no tiene componentes...`

### Si tu combo ya está mal configurado

1. Ve a Productos → busca el combo → editar
2. En el panel **🎁 Componentes del Combo**, agregá los componentes que faltan
3. Guardar → el sistema confirma cuántos componentes tiene
4. Próxima venta descuenta correctamente

---

## v2.5.16 — 2026-05-19 🚨 Dashboard y Ventas no se actualizaban al cobrar (event bus + tab activation)

### 🚨 Bug reportado

Después de hacer una venta (especialmente con cobro mixto) desde la pestaña POS:
- El Dashboard (Inicio) seguía mostrando montos viejos en Efectivo / Transferencia / Por cobrar
- La pestaña Ventas seguía mostrando solo las ventas anteriores

El usuario tenía que cerrar y reabrir la app para ver los datos actualizados.

### Causa raíz

Las pestañas con sistema multi-vista (v2.5.0+) mantienen sus páginas montadas con `display:none` para preservar estado. Pero el Dashboard y Ventas NO escuchaban el evento global `clouget:venta-completada` que POS dispara al cobrar (introducido en v2.5.7). Tampoco refrescaban al re-activar la tab (`useTabActivated`).

Resultado: al hacer una venta, los datos quedaban stale en las otras tabs.

### Fix v2.5.16

**1. DashboardPage** ahora:
- Escucha el evento `clouget:venta-completada` → recarga todos los KPIs (Ventas hoy, Efectivo, Transferencia, Por cobrar, etc.)
- Escucha `clouget:caja-cambio` → recarga si hay movimientos de caja desde otra tab
- Se refresca al re-activar la tab (`useTabActivated("/")`)

**2. VentasDia** ahora:
- Escucha `clouget:venta-completada` → recarga la lista de ventas
- Se refresca al re-activar la tab (`useTabActivated("/ventas")`)

**3. Backend `listar_ventas_sesion_caja`** mejorado:
- Ahora trae `caja_id` real (antes era `None` hardcoded → rompía filtro "Solo sesión #X")
- Ahora trae `cliente_nombre` (LEFT JOIN clientes)
- Filtro de fecha más permisivo: incluye todas las ventas del día además de las desde apertura

### Resultado

Al cobrar una venta en POS, en cuestión de milisegundos:
- Si tienes Dashboard abierto en otra tab → KPIs se actualizan automáticamente
- Si tienes Ventas abierto en otra tab → la nueva venta aparece en la lista
- Si vuelves a cualquiera de esas tabs después → también se refrescan al activarse

---

## v2.5.15 — 2026-05-19 🚨 Ventas mixtas no se registraban + Movimientos Bancarios no las mostraba

### 🚨 BUG CRÍTICO: ventas con pago MIXTO podían fallar silenciosamente

En instalaciones donde la migración de v2.5.12 no se aplicó correctamente (BDs creadas antes de v2.5.12 que ya tenían la tabla `pagos_venta` sin la columna `pago_estado`), el INSERT del cobro mixto fallaba y la venta entera se perdía.

**Fix self-healing v2.5.15**:
1. Antes de insertar pagos mixtos, **ejecutamos los ALTER TABLE on-the-fly** para asegurar que las columnas existan (silent si ya existen).
2. **Fallback de emergencia**: si el INSERT con `pago_estado` falla por la razón que sea, intentamos un INSERT mínimo sin esa columna — la venta se guarda igual.
3. Log a stderr cuando se activa el fallback (visible en herramientas de debugging) para que detectemos casos raros.

**Garantía**: una venta mixta JAMÁS debería fallar por problema de schema. Si fallaba antes, ahora se guarda igual.

### 🚨 Movimientos Bancarios: ventas no aparecían

El query usaba `WHERE v.tipo_estado = 'COMPLETADA'` pero las **ventas normales tienen `tipo_estado` NULL** (solo se setea para BORRADOR / COTIZACION / GUIA_REMISION). Resultado: las ventas normales con transferencia bancaria nunca aparecían en Movimientos Bancarios.

**Fix**: `(v.tipo_estado IS NULL OR v.tipo_estado = 'COMPLETADA')` en ambas subqueries (ventas simples + porciones de ventas mixtas).

Adicional: el filtro de forma de pago ahora es **case-insensitive** — acepta tanto `'TRANSFER'` (POS) como `'TRANSFERENCIA'` (Compras).

### Impacto

- Clientes que vieron error en cobro mixto post v2.5.12: ahora se registra siempre.
- Movimientos Bancarios ahora muestra **todas** las transferencias (incluyendo porciones de pagos mixtos).
- Dashboard sigue sumando correctamente con el fix de v2.5.14.

---

## v2.5.14 — 2026-05-19 🐞 5 fixes (ticket térmico + dashboard + kardex + RIMPE)

### 🐞 #1 — Ticket Epson 80mm: columnas desbordadas

Las impresoras Epson 80mm con fuente A imprimen máximo **42 columnas**, pero el código usaba 48. Las líneas se cortaban y los valores bajaban a la siguiente línea ("P.UNIT SUBTOT" en una línea, los precios en otra, etc.).

**Fix**: ancho calibrado a 42 columnas, columnas de la tabla recalculadas (nombre 22 + cant 4 + p.unit 7 + subtot 8 = 42 exactos). Configurable vía `config.ticket_ancho_columnas` para impresoras especiales (rango 28-64).

### 🐞 #2 — Ticket pago MIXTO: no mostraba detalle

Cuando hacías una venta con varios pagos (efectivo + transfer + crédito), el ticket solo decía "Pago: MIXTO" sin detalle. Ahora muestra desglose:

```
Forma pago: MIXTO
  Efectivo:                       $1.00
  Transfer.:                      $1.00
    Banco: Pichincha
    Ref: 28726926282
  Credito:                        $0.25
Total pagado:                     $2.25
```

Aplica tanto al ticket térmico ESC/POS como al PDF.

### 🐞 #3 — Dashboard no sumaba ventas MIXTAS a Efectivo/Transferencia

Los KPIs de "Efectivo" y "Transferencia" en el Home solo contaban ventas con forma_pago puro. Las ventas MIXTAS quedaban invisibles. Ahora se suman las porciones desde `pagos_venta`:

- Venta de $50 efectivo puro → +$50 a Efectivo
- Venta de $1 efectivo + $1 transfer en MIXTO → +$1 a Efectivo Y +$1 a Transferencia

Aplica también al reporte de período (`resumen_periodo`).

### 🆕 #4 — Kardex Multi: chip "✓ Todas" siempre visible

Antes el filtro de categorías era "vacío = todas" pero el usuario no veía esa lógica claramente. Ahora hay un chip verde **"✓ Todas"** que está activo cuando no hay filtro. Click para limpiar la selección. Texto explicativo si hay categorías seleccionadas: *"💡 Filtrando por N categoría(s). Click '✓ Todas' para ver el inventario completo."*

### 🆕 #5 — RIMPE Negocio Popular ahora puede emitir Facturas (si tiene módulo SRI)

Antes el tipo de documento "Factura" estaba completamente oculto en régimen RIMPE Popular. Ahora si el cliente tiene el módulo SRI activo, puede elegir entre **Nota de Venta** (default) y **Factura** — la emisión electrónica es opcional pero permitida para clientes que la pidan.

El régimen RIMPE Popular sigue sin obligación de emitir factura electrónica; simplemente damos la opción de hacerlo voluntariamente.

---

## v2.5.13 — 2026-05-19 🐞 Bug precio agrupado se pisaba al seleccionar cliente

Continuación del fix de v2.5.12. Quedaba un caso no cubierto: si en el POS tenías un blister/jaba/sixpack en el carrito Y después seleccionabas un cliente (o el cliente ya estaba seleccionado al agregar), el precio se pisaba al unitario.

### Causa raíz

`recalcularPreciosCarrito` (que se dispara al cambiar de cliente) llamaba a `resolverPrecioProducto(producto_id, clienteId)` para TODOS los items del carrito. Esa función solo conoce el precio del **producto base** (unidad), no de las presentaciones (blister, jaba). Resultado: el blister de 10 a $2.00 se quedaba en $0.25 (precio unitario) al cambiar cliente.

### Fix v2.5.13

**1. `recalcularPreciosCarrito` ahora NO toca presentaciones agrupadas** (factor > 1 o con unidad_id). Solo recalcula items en unidad base. Las presentaciones mantienen el precio con el que entraron al carrito.

**2. `agregarAlCarrito` aplica factor también con lista de precios del cliente**. Antes solo aplicaba al fallback `precio_venta`. Ahora si la presentación no tiene precio explícito Y el cliente tiene lista de precios, calcula `precio_lista × factor` (ej. $0.25 × 10 = $2.50 para blister x10).

### Recomendación de configuración

Para evitar ambigüedad, **configurá precio explícito a cada presentación** en Productos → Unidades. Eso siempre prevalece sobre cualquier lista. Ejemplo:
- Aspirina unitaria: $0.25
- Aspirina blister x10: $2.00 (descuento por agrupado)
- Aspirina caja x100: $18.00 (descuento mayor por mayoreo)

Si no configurás precio a la presentación, el sistema usa `precio_unitario × factor` automáticamente.

---

## v2.5.12 — 2026-05-19 🚨 Bug CRÍTICO cobro mixto + precio unidad agrupada

### 🚨 BUG CRÍTICO: cobro mixto fallaba con "table pagos_venta has no column named pago_estado"

Al hacer una venta con **pago mixto** (efectivo + transferencia + crédito, etc.), el sistema fallaba con:
```
Error al registrar venta: Error guardando pago: table pagos_venta has no column named pago_estado
```

**Causa raíz**: las migraciones de verificación de transferencias (`pago_estado`, `verificado_por`, `fecha_verificacion`, `motivo_verificacion`) sobre `pagos_venta` estaban ubicadas **antes** del `CREATE TABLE pagos_venta` en `schema.rs`. En instalaciones nuevas, los `ALTER TABLE` corrían sobre una tabla inexistente y fallaban silenciosamente. Cuando finalmente se creaba la tabla, no tenía esas columnas.

**Fix**: las migraciones se movieron **después** del CREATE TABLE, garantizando que se ejecuten sobre la tabla recién creada. Idempotente para clientes existentes que ya tienen las columnas (los ALTER fallan silenciosamente, sin efecto).

**Impacto**: cualquier instalación nueva post v2.5.12 va a tener cobro mixto funcionando. Las instalaciones viejas que ya funcionaban siguen igual.

### 🐞 Bug: precio de unidad agrupada (blister, jaba, sixpack) iba al unitario

Si tenías un producto con presentación agrupada (ej. blister de 10 aspirinas) **sin precio explícito** en esa presentación, al venderlo el precio se quedaba en el unitario:
- Aspirina unitaria: $0.25
- Aspirina blister x10: debería ser ~$2.50 (= $0.25 × 10) → se mostraba **$0.25** ❌

**Fix v2.5.12**: si la presentación no tiene precio explícito definido, ahora se aplica automáticamente `precio_venta_unitario × factor` para que sea matemáticamente neutral. Si el usuario configuró un precio específico para la presentación, ese sigue prevaleciendo (ej. blister con descuento por agrupado: $2.00 en vez de $2.50).

---

## v2.5.11 — 2026-05-16 🚨 Bug crítico eliminar ST + UI fixes

### 🚨 BUG CRÍTICO: orden ST con abonos se podía eliminar

El comando `eliminar_orden_servicio` solo chequeaba si la orden tenía `venta_id`, pero **no chequeaba si tenía abonos en HOLDING**. Esto permitía borrar una orden cuyos abonos ya habían entrado a caja, dejando el dinero en caja sin contrapartida → **descuadre contable**.

**Fix v2.5.11**:
- Ahora bloquea la eliminación si hay **cualquier abono** registrado (HOLDING / APLICADO / DEVUELTO).
- Mensaje claro: *"No se puede eliminar esta orden porque tiene N abono(s) registrado(s) en caja. Si querés anular la orden, usá 'Cancelar orden' — eso devuelve los abonos en holding automáticamente."*
- También bloquea si tiene items presupuestados (sugerimos eliminar items primero o cancelar la orden).
- La lógica original de marcar `CANCELADO` cuando hay venta_id sigue intacta.

**Para órdenes ya eliminadas erróneamente**: los abonos huérfanos quedan en `st_abonos` con `orden_id` apuntando a una fila inexistente. Si necesitás limpieza retroactiva, contactanos.

### 🎨 Botón "📄 Imprimir" no se leía en tema oscuro

El botón en el footer del detalle de orden ST heredaba el color `inherit` que en dark theme quedaba blanco-sobre-blanco. Se forzó `color: var(--color-text)` y `fontWeight: 600` para que siempre se vea.

### 🆕 Cotización PDF (A4): items ahora en tabla por columnas

Antes los items se imprimían como viñetas planas:
```
• Producto X x2 · $5.00 c/u = $10.00
```
Ahora en A4 se muestran como tabla con columnas (igual que las notas de venta):

| # | Descripción | Cant. | P.Unit. | Subtotal |
|---|---|---|---|---|
| 1 | Cambio de aceite | 1 | $35.00 | $35.00 |
| 2 | Filtro de aire | 1 | $12.00 | $12.00 |

El formato 80mm se mantiene multi-línea (mejor lectura en ticket angosto).

---

## v2.5.10 — 2026-05-16 🐞 Canal Beta ahora recibe también versiones Stable

### Bug reportado

Los clientes/testers con canal **Beta** configurado no recibían las versiones **Stable** nuevas. Si después de una beta no salía otra beta sino solo stables, quedaban atrasados (sin las correcciones críticas de stable).

### Causa

El comando `verificar_update_canal` consultaba SOLO el endpoint `?canal=beta` cuando el cliente estaba en beta. El endpoint beta no incluye las versiones stable.

### Fix

Ahora si el canal es **Beta**, el cliente consulta **AMBOS endpoints** (stable + beta) y aplica la versión MÁS ALTA. Esto garantiza que un usuario en beta nunca se queda atrás respecto a stable.

Comportamiento por canal:

| Canal | Endpoints consultados | Resultado |
|---|---|---|
| **Stable** | Solo `?canal=stable` | Última stable |
| **Beta** | `?canal=stable` + `?canal=beta` | Versión más alta de ambas |

Adicional: si un endpoint está caído, el cliente sigue probando los demás (no aborta el chequeo entero por un endpoint con error).

---

## v2.5.9 — 2026-05-16 ⬆ Auto-update UX refinada (startup vs runtime + detalles)

Mejora del flujo de actualización de v2.5.8 según feedback:

### 🆕 Diferenciación startup vs runtime

**Al abrir la app** (3 segundos después del arranque):
- Aparece un banner azul fino: **"🔄 Buscando actualización..."**
- Si encuentra → **instala automáticamente** (sin preguntar — el cliente recién está abriendo, no está en medio de nada)
- Si no encuentra → desaparece el banner silenciosamente

**Mientras la app está abierta** (check cada 60 minutos):
- Si encuentra → muestra banner con **[⬆ Actualizar ahora]** / **[Más tarde]**
- No instala automáticamente — el cliente podría estar en medio de una venta o cobro y perder trabajo si reinicia sin avisar

### 🆕 "Ver detalles de la actualización"

El banner ahora incluye un toggle expandible **"Ver detalles de la actualización"** que muestra las notas de la release (body del último commit/release de GitHub) — así el cliente sabe qué se está instalando antes de aceptar.

Si el body no viene (fallback): mensaje genérico "Esta nueva versión incluye correcciones y mejoras. Revisá el detalle completo en GitHub."

### Resumen del comportamiento

| Cuándo | Acción |
|---|---|
| Arranque de la app (1ra vez) | Muestra "Buscando..." → si hay, instala auto |
| App abierta, 60 min después | Si hay, banner con [Actualizar] [Más tarde] + detalles |
| Click manual en Configuración | Igual que runtime: banner con confirmación + detalles |

---

## v2.5.8 — 2026-05-16 ⬆ Auto-update: chequeo periódico + confirmación + banner llamativo

### 🐞 Por qué los testers no recibían updates

El sistema de auto-update **solo verificaba al iniciar la app**. Los POS suelen quedar abiertos 12-16 horas/día sin reiniciar, así que el chequeo nunca se volvía a disparar — los clientes/testers no se enteraban de versiones nuevas.

Encima, antes el sistema **descargaba e instalaba automáticamente** sin preguntar. Eso es peligroso si el cliente está en medio de una venta: el reinicio podría perder el carrito o un cobro a medias.

### 🆕 Solución v2.5.8

**1. Verificación recurrente cada 60 minutos** (además del check inicial a los 5 segundos del arranque). Si la app está abierta todo el día, el cliente se entera de updates dentro de la hora.

**2. Banner con confirmación** — ya no descarga sin preguntar. Cuando hay nueva versión aparece un banner llamativo arriba:

> 🎉 **Nueva versión X.X.X disponible.** Aplica el cambio cuando termines lo que estás haciendo — se cerrará y reiniciará la app.
>
> [⬆ Actualizar ahora]  [Más tarde]

- **"Actualizar ahora"** → descarga + reinicia (igual que antes pero solo con consentimiento)
- **"Más tarde"** → oculta el banner. Volverá a aparecer en el próximo check (60 min) o al reiniciar la app

**3. Botón manual "🔄 Buscar actualización ahora"** en Configuración → Actualizaciones. Permite al cliente forzar un chequeo en cualquier momento (útil para soporte: "le digo al cliente que vaya a Config y haga click").

**4. Feedback visible siempre que el usuario pide chequeo manual**:
- Si HAY update → aparece el banner llamativo
- Si NO HAY update → aparece banner verde "✓ Estás en la última versión" (auto-cierra en 4s)
- Si HAY ERROR → aparece banner rojo con el error

### Impacto

- Clientes ya no se quedan en versiones viejas por días/semanas
- Nunca más se pierde trabajo por reinicio sorpresa
- Soporte puede instruir al cliente a forzar chequeo

---

## v2.5.7 — 2026-05-16 🚨 Bug CRÍTICO: POS no veía caja abierta / venta no se sumaba a Caja

### 🐞 Síntomas reportados por cliente

> "Cierro sesión, abro caja, al vender me vuelve a pedir abrir caja"
> "Al vender, la venta no se suma a la caja — tengo que cerrar y volver a abrir"

### Causa raíz

Con el sistema de pestañas internas (v2.5.0+), las páginas POS y Caja se mantienen montadas en memoria (display:none) para preservar su state. Pero NO se comunicaban entre sí:

- **POS** cacheaba `cajaAbierta` al montar. Si después abrías caja desde la pestaña Caja, POS no se enteraba y al vender daba "Debe abrir la caja".
- **Caja** mostraba el monto cacheado al momento del último render. Las ventas hechas en POS no se sumaban hasta refrescar.

El refresh por `useTabActivated` (v2.5.3) solo actualizaba productos/categorías, NO la caja. Esto se nos pasó.

### 🆕 Fix v2.5.7 — Event bus cross-tab

Implementé un sistema de notificaciones DOM events entre pestañas:

**1. Cuando POS completa una venta** dispara:
```js
window.dispatchEvent(new CustomEvent("clouget:venta-completada", {...}));
```
→ CajaPage escucha y refresca automáticamente (en vivo, sin tener que cambiar de tab).

**2. Cuando Caja se abre/cierra** dispara:
```js
window.dispatchEvent(new CustomEvent("clouget:caja-cambio", {...}));
```
→ PuntoVenta escucha y refresca `cajaAbierta` automáticamente. Si vas al POS y la caja ya estaba abierta, ya no da el falso error "Debe abrir caja".

**3. PuntoVenta useTabActivated** ahora también refresca `cajaAbierta` al volver a la tab (no solo productos). Backup adicional por si el evento no llegó.

### Impacto

- Ya no es necesario "cerrar y reabrir caja" para ver ventas reflejadas.
- Ya no aparece el falso error "Debe abrir caja" cuando la caja sí está abierta.
- La sincronización entre tabs ahora es **inmediata** (event-driven), no solo al cambiar de tab.

### Comunícale al cliente

Después de actualizar (auto-update al próximo arranque), el problema desaparece. **No requiere migración de datos ni cambiar configuración** — funciona de inmediato.

---

## v2.5.6 — 2026-05-14 🐞 Backup en la Nube: fix selección + sección Premium visible

### 🚨 Bug: el dropdown "Tipo de respaldo" no se mantenía seleccionado

Al seleccionar "Google Drive (cuenta propia)" o "Premium", la opción se reseteaba a "Seleccionar..." y nunca aparecía el botón para conectar / configurar.

**Causa**: stale state en React. El handler usaba `setConfig({ ...config, ... })` capturando un `config` viejo. Si el usuario activaba el checkbox "Activar backup automático" y a continuación elegía un tipo, el segundo `setConfig` pisaba el cambio del primero (por la closure del JS).

**Fix**: ahora todos los handlers usan `setConfig((prev) => ({ ...prev, ... }))` (functional update) que siempre recibe el state más reciente. La selección persiste correctamente.

### 🆕 Sección "Premium (servidor Clouget)" antes invisible

Cuando seleccionabas "Premium" no aparecía nada — ni info ni botón. Ahora aparece una caja explicativa morada con:

- Indicador de licencia válida (código truncado)
- Confirmación módulo `backup_premium` activo
- Frecuencia configurada
- Instrucciones de uso
- Cifrado automático en el servidor de Clouget

### 🆕 Bloqueo visual cuando no se tiene el módulo

Si la licencia NO incluye `backup_premium`, la opción aparece deshabilitada con candado 🔒 y un mensaje:

> 💡 El backup Premium requiere el módulo backup_premium en tu licencia. Contacta al administrador para activarlo.

Antes la opción simplemente no aparecía, ahora se ve pero deshabilitada — más claro para el cliente que sabe que existe el feature.

---

## v2.5.5 — 2026-05-13 💳 Catálogo SRI de formas de pago en Compras

En el módulo de Compras (compra manual + importar XML SRI), el dropdown de "Forma de pago" ahora muestra el catálogo completo del SRI Tabla 24 con el código visible en cada opción:

```
💵 Efectivo · SRI 01
🧾 Cheque · SRI 20
🏦 Transferencia · SRI 20
💳 Tarjeta de débito · SRI 16
💳 Tarjeta de crédito · SRI 19
💳 Tarjeta prepago · SRI 18
📱 Dinero electrónico · SRI 17
🔄 Compensación / canje · SRI 15
📋 Crédito (queda por pagar) · SRI 20
```

Debajo del dropdown se muestra: **"Código SRI XX: descripción oficial"** para que no quede duda sobre qué código se va a reportar al SRI.

3 formas de pago nuevas se agregaron al catálogo: **Tarjeta prepago (18)**, **Dinero electrónico BCE (17)** y **Compensación / canje (15)**.

### Backward compat

Las compras existentes con códigos legacy (EFECTIVO, TRANSFERENCIA, DEBITO, CHEQUE, CREDITO) siguen funcionando — el catálogo se actualizó para mantener exactamente esos mismos códigos internos.

---

## v2.5.4 — 2026-05-13 📋 Módulo de Retenciones SRI (cruce con factura)

### 🆕 Problema que resuelve

En Ecuador, cuando vendés una factura a una empresa, esa empresa puede actuar como **agente de retención** y descontar parte del pago según normativa SRI:
- Retención de IVA (Tabla 21): 10%, 20%, 30%, 70%, 100% del IVA
- Retención de Renta (Tabla 304): 1%, 1.75%, 2%, 8%, 10% del subtotal

**Ejemplo**: Factura $1.150 → cliente retiene 30% IVA ($45) + 2% Renta ($20) = $65
- Cliente paga $1.085 + te entrega 2 comprobantes de retención
- Antes: la factura quedaba con saldo pendiente $65 → descuadre contable
- Ahora: registrás las 2 retenciones en el sistema → saldo pasa a **$0 (cancelada)**

### 🆕 Cómo usarlo

**Desde Ventas del Día → detalle de una FACTURA**, aparece un botón **📋 Retenciones SRI**.

**Desde Cuentas por Cobrar → historial de pago de una factura a crédito**, aparece una tarjeta "Retenciones SRI" + botón **📋 Registrar / Gestionar**.

El modal permite:
- Seleccionar **tipo**: Retención de IVA o Retención de Renta
- Elegir el **código SRI** del catálogo (Tabla 21 o Tabla 304) — incluye los más comunes (10%, 20%, 30%, 70%, 100% IVA · 1%, 1.75%, 2%, 8%, 10% Renta)
- **Cálculo automático**: `valor = base × % / 100`
- Ingresar **número del comprobante** de retención del cliente y **fecha de emisión**
- Listar todas las retenciones aplicadas a la factura
- **Eliminar** retenciones (corregir errores de tipeo)

### 📊 Recálculo automático del saldo

Al registrar una retención, el saldo de la factura se recalcula:
```
saldo = total - cobrado - retenciones_renta - retenciones_iva
```

Si saldo = 0 → la factura aparece como **✓ CANCELADA** (cobrada totalmente entre pago + retenciones).

### 🛡 Validaciones

- No permite que `valor` exceda el saldo pendiente de la factura
- Tipo, código SRI, número de comprobante y fecha son obligatorios
- Solo aplica a tipo de documento **FACTURA** (no a Notas de Venta — éstas no generan retenciones)
- Retenciones registradas se pueden **eliminar** si fueron mal cargadas (registro de auditoría queda en `usuario` y `fecha_registro`)

### 🏗 Backend

- Tabla nueva `retenciones_recibidas` (id, venta_id, tipo, código_sri, base, %, valor, num_comprobante, fechas, usuario, observación)
- 4 comandos Tauri: `listar_retenciones_venta`, `total_retenciones_venta`, `registrar_retencion`, `eliminar_retencion`
- Catálogo SRI completo en `src/config/retencionesSri.ts` (frontend)

### Próximamente (v2.5.5)

- Reporte de retenciones recibidas para declaración SRI
- Retenciones que vos hacés a proveedores (lado opuesto del flujo)

---

## v2.5.3 — 2026-05-13 🔄 Auto-refresh de pestañas (data fresca al volver)

### 🐞 Bug detectado en sistema de pestañas (v2.5.0)

Si tenías POS abierto en una pestaña, ibas a Productos, editabas un producto (cambiabas precio o nombre), y volvías al POS — **el POS seguía mostrando los datos viejos**. Esto pasa porque las pestañas mantienen su state preservado con `display: none` (esa es la ventaja: no perdés el carrito), pero el efecto colateral era que la data en cache no se refrescaba.

### 🆕 Solución: Hook `useTabActivated`

Nuevo hook en `TabsContext` que ejecuta un callback cada vez que una pestaña pasa a estar activa (después de no estarlo). Las páginas críticas ahora se auto-refrescan al recuperar el foco:

| Pestaña | Qué se refresca al volver |
|---|---|
| **POS** | Lista de productos, categorías, listas de precios, cuentas bancarias |
| **Caja** | Estado de caja abierta, retiros, ingresos, holdings ST |
| **Servicio Técnico** | Listado de órdenes |
| **Clientes** | Lista de clientes |
| **Productos** | Lista de productos + categorías |

### Para desarrolladores

Ahora cualquier página puede opt-in al refresh con:

```tsx
import { useTabActivated } from "../contexts/TabsContext";

useTabActivated("/mi-ruta", () => {
  // este callback corre cada vez que la tab se vuelve activa
  recargarMisDatos();
});
```

Si tabs están desactivadas (modo clásico), el callback no se ejecuta — el remount del componente al cambiar de ruta ya recarga la data como antes.

---

## v2.5.2 — 2026-05-13 🛠 7 mejoras UX + métodos de pago SRI ampliados

### 🐞 Bugs corregidos

- **Cotización desde POS imprimía "NOTA DE VENTA"** en el ticket en lugar de "COTIZACIÓN". El query SQL no leía `tipo_estado` desde la tabla `ventas`, así que el flag de cotización siempre venía null. Fix: `imprimir_ticket` e `imprimir_ticket_pdf` ahora cargan `tipo_estado` y el render lo respeta.
- **Botones ✏ y 🗑 de abono HOLDING casi invisibles**: tenían fontSize 10 y el icono solo. Ahora tienen 11px, con label "Editar" / "Eliminar" y border de color (azul / rojo). Más fáciles de descubrir.
- **Presets de garantía (Sin / 7d / 15d / 30d / 60d / 90d / 180d) se desbordaban del modal de cobrar**. Layout cambiado a 2 filas: input arriba, presets en flex-wrap abajo.

### 🆕 Mejoras UX

- **Editar Chasis/VIN y Placa después de creada la orden**: en el detalle de orden ST, ahora hay 2 inputs editables al lado del equipo. Útil cuando al crear no se sabía el dato o se tipeó mal. Solo en órdenes no-cerradas.
- **Formato 80mm de cotización ST mejorado**: antes la línea `"• Producto x2 · $5.00 c/u = $10.00"` se veía apretada y mezclada en ticket. Ahora en ticket se imprime en 2 líneas: descripción arriba, cantidades abajo. Separador `------` antes de los totales.
- **LicenciaPage: links promo solo en modo demo**. "Ver todas las características" + URL "pos.clouget.com" se ocultan automáticamente cuando la licencia ya está activada (el cliente que ya nos compró no necesita seguir viendo promociones). Las novedades / descripción de mejoras siguen visibles.

### 🆕 Catálogo SRI completo de formas de pago (Tabla 24)

Antes el mapeo POS → SRI tenía huecos. Ahora soporta los 9 códigos oficiales del SRI:

| Código SRI | Descripción | Forma POS |
|---|---|---|
| 01 | Sin sistema financiero | Efectivo |
| 15 | Compensación de deudas | Compensación / Canje |
| 16 | Tarjeta de débito | Tarjeta débito |
| 17 | Dinero electrónico (BCE) | Dinero electrónico |
| 18 | Tarjeta prepago | Tarjeta prepago |
| 19 | Tarjeta de crédito | Tarjeta crédito |
| 20 | Otros con sistema financiero | Transferencia · Cheque · Crédito · Mixto |
| 21 | Endoso de títulos | Endoso |

Nuevo archivo `src/config/formasPagoSri.ts` con el catálogo completo (label visible, código interno, código SRI, descripción oficial). Backend actualizado en `src-tauri/src/sri/xml.rs::forma_pago_sri` para hacer el mapeo correcto al emitir factura electrónica.

---

## v2.5.1 — 2026-05-12 📦 Stock visible en buscador de productos del taller

En el detalle de la orden ST, al buscar un producto para agregarlo como item presupuestado, ahora se muestra **el stock actual** junto al precio. Color del badge:

- 🟢 Verde — stock disponible
- 🟡 Amarillo — stock bajo (al o por debajo del mínimo)
- 🔴 Rojo — sin stock (0 o negativo)

Esto evita prometerle al cliente un repuesto que después no hay en bodega.

---

## v2.5.0 — 2026-05-12 🗂 Pestañas internas (multi-vista)

Cambio mayor de UX: ahora podés tener varias páginas abiertas a la vez, como en un navegador. Estás armando una venta en POS, alguien te pregunta por stock, vas a Productos, volvés al POS y **el carrito sigue ahí**. Sin perder lo que estabas haciendo.

### 🗂 Cómo funciona

- Cada página que abrís se vuelve una **pestaña** en la barra superior
- Click en otra del sidebar → se abre como nueva pestaña (o se activa si ya estaba)
- **Una pestaña por ruta** (no se duplican — clic en el mismo ítem activa la existente)
- **Inicio** es pestaña fija (no se puede cerrar)
- **X** en cada pestaña para cerrar (o **clic con rueda del mouse**, estilo navegador)
- Cerrar la pestaña activa → activa la anterior automáticamente
- Las pestañas **persisten al recargar la app** (sessionStorage), no entre cierres totales

### ⚙ Reglas de seguridad

- **Máximo 8 pestañas** abiertas (si pasás el límite, reemplaza la más vieja no-activa)
- Cada usuario tiene **su propio set de pestañas** (no se mezclan entre cajeros que comparten PC)
- Páginas sin permiso para el rol no se pueden abrir como pestaña (redirige a Inicio)
- **Atajos F1-F10** siguen funcionando: abren la pestaña correspondiente o la activan
- **State preservado**: usamos `display: none` para ocultar pestañas inactivas → carrito, formularios a medio llenar, filtros, scroll position, modales abiertos — todo queda intacto

### 🔌 Toggle on/off

Si por alguna razón el sistema de pestañas te causa problemas (rendimiento, comportamiento raro), podés desactivarlo en **Configuración → Pestañas internas (multi-vista)**. El sistema vuelve al modo clásico de una página a la vez. Recargá la app después de cambiar el toggle.

### ⚠ ¿Por qué NO se pueden duplicar pestañas?

Decisión deliberada de diseño profesional. Comparado con Square, Lightspeed Retail, Toast, Loyverse y Vend — **ninguno** permite duplicar la pantalla de venta porque genera bugs:

- 2 carritos POS abiertos → cajero se confunde y agrega productos al cliente equivocado
- 2 Cajas abiertas → registrás un retiro en una y la otra no lo sabe → cierre descuadrado
- 2 Configuraciones → cambios pisándose

Para "atender 2 clientes a la vez" en POS, ya tenés el sistema de **borradores** (guardar carrito y abrirlo después). Esa es la solución correcta para ese caso.

### 🏗 Arquitectura técnica (para futuras referencias)

- **`TabsContext`** — manejo del estado de tabs
- **`TabsContainer`** — renderiza TODAS las tabs montadas, oculta inactivas con display:none
- **`TabBar`** — barra horizontal con tabs (estilo navegador)
- **`PageRenderer`** — switch path → componente
- Sincronización **bidireccional** URL ↔ active tab (browser back/forward funciona)
- Storage scope por `usuario_id` (sessionStorage)

---

## v2.4.29 — 2026-05-12 📋 Cotización antes de cobrar (orden ST)

Antes el cliente que pedía cotización antes de aprobar el trabajo no tenía un PDF formal — solo el presupuesto numérico en el formulario. Ahora hay un botón **"📋 Cotizar"** en el detalle de la orden ST (junto al botón Cobrar) que genera un PDF de cotización con:

- Título **"COTIZACIÓN"** + número `COT-NNNNNN`
- Sección **"DETALLE DE COTIZACIÓN"** con cada item presupuestado (descripción, cantidad, precio unitario, subtotal)
- Subtotal, IVA y TOTAL calculados desde los items
- Línea **"📅 Cotización válida por N días"** + fecha de vencimiento calculada
- Línea de aceptación al pie en lugar de firma

El PDF de cotización **no afecta inventario** (no descuenta stock), **no genera venta**, **no consume abonos**. Es solo un documento informativo. Cuando el cliente aprueba, se sigue el flujo normal de "💰 Cobrar".

**Configuración** → Servicio Técnico → **"📅 Validez de cotización (días)"** (default 30).

El botón aparece solo en órdenes en estado pre-cobro (no en ENTREGADO ni CANCELADA).

---

## v2.4.28 — 2026-05-12 🐞 Caja + UX productos + Editar abonos + Kanban ST

### 🚨 BUG Caja: pedía motivo aunque el monto coincidiera con el disponible

Si cerrabas con $72.99 y depositabas todo al banco (disponible = $0), al abrir la nueva caja con $0 el sistema pedía justificar "diferencia de $72.99". Era el frontend comparando contra `monto_real` (bruto) en vez de `monto_disponible` (post-depósitos). Fix: usa `monto_disponible` para la validación.

### 🆕 Diferencia al abrir caja: ¿Ingreso o Sobrante?

Si pones más dinero del esperado (ej. esperado $0, contás $5), antes lo marcaba como "diferencia" sin distinguir el origen. Ahora aparecen 2 opciones:

- **📥 Ingreso de caja** — alguien aportó dinero (dueño, vuelto, etc.)
- **🪙 Sobrante** — estaba sin contar al cerrar la sesión anterior

El motivo se prefija con `[INGRESO DE CAJA]` o `[SOBRANTE]` para auditoría posterior.

### 🐞 Detalle de venta: ahora muestra abonos aplicados + total real recibido

Si la venta vino de una orden ST cobrada con abonos previos, el modal solo mostraba "Recibido: $64.99" (lo del cobro), dando la impresión de que el cliente pagó menos del total. Ahora se ve:

- Abonos previos aplicados (con fecha, forma de pago, banco)
- **Total real recibido (cobro + abonos)** = el monto verdadero que pagó el cliente

### 🆕 Editar / eliminar abono en HOLDING (corregir typos)

Si el cajero registra un abono con un monto incorrecto (ej. $250 en vez de $25), ahora puede editarlo o eliminarlo siempre que el abono esté en HOLDING (no aplicado todavía). Aparecen los botones **✏ editar** y **🗑 eliminar** junto a cada abono HOLDING.

- Editar permite cambiar monto, forma de pago, banco, referencia y observación.
- Se valida que el nuevo monto no exceda el total de la orden.
- Auditoría: la edición agrega `[editado por X: $A → $B]` a la observación.
- Abonos en estado APLICADO o DEVUELTO son inmutables (ya generaron venta o NC).

### 🆕 Lotes de caducidad: agregar al crear el producto (sin guardar primero)

Antes había que guardar el producto, reabrirlo y recién entonces se podían agregar lotes. Ahora podés cargar lotes en el mismo formulario de creación — quedan marcados como **(pendiente)** en amarillo y se persisten automáticamente al guardar el producto.

### 🐞 Kanban Servicio Técnico: columnas se desbordaban

Las columnas con `1fr` no respetaban el min-width y los cards se cortaban. Ahora `minmax(180px, 1fr)` + scroll horizontal cuando no caben las 6 columnas. Texto largo (cliente, equipo, técnico) se trunca con `…` y muestra completo al hacer hover.

### 🐞 Label "número de serie único al vender" mal posicionado

Aparecía huérfano debajo de "Destino restaurante" cuando debía estar junto al checkbox **Requiere número de serie**. Movido a su lugar y solo aparece si el checkbox está activo.

---

## v2.4.27 — 2026-05-11 🛠 ST: prefijo OT, recibo completo, accesorios rápidos + 🐞 caja

### 🚨 Bug crítico Caja: retiros post-cierre con motivo libre no se descontaban

Continuación del fix v2.4.24. El filtro anterior solo restaba retiros con `motivo LIKE '%cierre%'` Y `estado IN ('DEPOSITADO', 'EN_TRANSITO')`. Eso dejaba afuera **cualquier retiro normal** post-cierre (estado `SIN_DEPOSITO`, motivos como "para gastos", "vuelto al dueño", etc.).

**Fix v2.4.27**: ahora se descuentan **TODOS** los retiros hechos después del cierre (cualquier motivo, cualquier estado), filtrando por `fecha > cerrada_at`. Si retiraste $100 después de cerrar, el monto sugerido para abrir la próxima caja se reduce en $100, indistintamente de cómo se haya etiquetado el retiro.

### 🆕 Recibo de cobro: ahora muestra los pagos reales + garantía + saldo correcto

Antes el PDF/ticket solo mostraba abonos, dejando "Saldo pendiente: $40" aunque el cliente ya hubiese pagado al cobrar. Ahora:

- Sección nueva **"PAGO AL COBRO"** con cada forma de pago usada (efectivo, tarjeta, transfer, etc.) y referencias.
- Saldo se recalcula con: `Total - (Abonos + Pagos al cobro)` → si todo está pagado, sale **"CANCELADO TOTALMENTE"**.
- Línea **"🛡 Garantía del trabajo: N días"** + fecha de vencimiento calculada desde la fecha de entrega.

### 🆕 Prefijo OT (Orden de Trabajo) en lugar de OS

En Ecuador es más común llamar a estas órdenes "OT" (orden de trabajo) que "OS" (orden de servicio). Las órdenes nuevas usan `OT-NNNNNN`; las viejas siguen como `OS-NNNNNN` y la numeración continúa unbroken (no hay colisión ni saltos).

### 🆕 Accesorios pre-seleccionables al crear orden ST

En Configuración → Servicio Técnico hay un nuevo campo **"🎒 Accesorios comunes"** (lista separada por comas, ej: `Cargador, Mochila, Llaves, Manual`). Al crear una orden, esos accesorios aparecen como chips toggleables sobre el campo de texto, evitando tipear los más frecuentes.

---

## v2.4.26 — 2026-05-11 🛠 Kilometraje en el PDF de orden ST

Complementa v2.4.25: el PDF/ticket de la orden de servicio ahora incluye el bloque de kilometraje cuando aplica (vehículos, motos, maquinaria con km).

En el bloque **EQUIPO** del PDF/ticket se imprimen, si están definidos:
- `Km entrada: 45000`
- `Km salida: 45120`
- `Próximo mantenimiento: 50120 km (cada 5000 km)`

Antes el cliente recibía el reporte impreso sin esta información, aunque sí se mostraba en pantalla — ahora ambos coinciden.

---

## v2.4.25 — 2026-05-09 🛠 Servicio Técnico: Kilometraje + Imprimir desde Historial + Permisos TECNICO

### 🚗 Sistema de kilometraje con cálculo automático del próximo mantenimiento

Para tipos de equipo que requieren kilometraje (motos/autos/maquinaria), el form de **Nueva orden ST** ahora pide:

- **Kilometraje actual** (km de entrada del vehículo)
- **Cada (km)** — intervalo recomendado entre mantenimientos (ej. 5000 km)
- **Próximo (auto)** — se calcula automáticamente: `entrada + intervalo`

Al **cobrar** la orden, se muestra un campo nuevo **"🚗 Kilometraje de salida"** (precargado con el de entrada). Si el técnico/cajero lo modifica:

- Se guarda como `equipo_kilometraje_salida`
- Se **recalcula** el próximo mantenimiento usando `salida + intervalo` en vez del de entrada
- Preview en vivo dentro del modal: `✓ Próximo mantenimiento: X km`

Backend:
- 2 columnas nuevas en `ordenes_servicio`: `equipo_kilometraje_intervalo` y `equipo_kilometraje_salida`
- Migración no destructiva (ALTER TABLE ADD COLUMN)
- `crear_orden_servicio` y `actualizar_orden_servicio` aplican la lógica de cálculo automático cuando el campo `proximo` viene vacío

### 🖨 Imprimir desde el historial de servicio técnico

En el modal de **Historial por equipo**, cada orden expandida ahora muestra:

- Sección **🚗 Kilometraje** con: Entrada · Salida · Próximo mant. · (cada X km)
- Botones discretos **🖨 A4** y **🧾 80mm** para imprimir el reporte directamente sin tener que abrir la orden

### 🔗 Botón "Abrir orden completa" del historial

Antes el botón solo cerraba el modal. Ahora abre el detalle completo de la orden histórica en el panel principal, con toda la información cargada y todos los botones de acción disponibles (incluidos los de imprimir).

### 🔓 Permisos asignables a usuarios TECNICO

Antes solo los CAJEROS podían recibir permisos personalizados desde Configuración → Usuarios. Ahora los usuarios con rol **TECNICO** también pueden recibir permisos extra (ver reportes, cobros, productos, etc.), útiles para talleres donde el técnico cobra directamente al cliente.

---

## v2.4.24 — 2026-05-11 🐞 Bug crítico Caja + UX

### 🚨 Bug crítico: depósitos post-cierre no descontaban del monto sugerido

Flujo del bug:
1. Cierras caja con $282 contados → registrado `monto_real = 282`
2. Hacés depósito a banco por $282 (clic "Registrar depósito a banco")
3. Próxima apertura → sugiere $282 como monto inicial
4. Pero ese efectivo ya NO está en caja (está en el banco)
5. Apertura con $282 = inflar la caja con dinero fantasma → desfase contable

**Fix v2.4.24**:
- `obtener_ultimo_cierre` ahora devuelve también `monto_disponible` y `depositos_post_cierre`. Resta los retiros con motivo "%cierre%" en estado DEPOSITADO o EN_TRANSITO.
- `abrir_caja` usa el mismo cálculo para validar continuidad.
- UI banner de "Cierre anterior": ahora muestra el desglose:
  ```
  Monto contado: $282.14
  − Depositado al banco: $282.14
  = Disponible en caja: $0.00
  ```
- El sugerido en input "Monto inicial" ahora es el `disponible` (no el `monto_real`).

### 🆕 Botón "Cerrar sesión" en lugar de "Finalizar Turno"

El nombre confundía. Ahora:
- "Cerrar Caja" → cierra la sesión de caja (registra fecha, calcula diferencia, libera)
- Pantalla de resumen con opciones (imprimir ticket, depositar a banco)
- Botón final renombrado a **"🔓 Cerrar sesión"** + texto explicativo claro

### 🆕 Garantía en form de creación de orden ST

Antes el form de Nueva Orden solo tenía Técnico / Presupuesto / Fecha promesa. La garantía solo se podía editar desde el detalle. Ahora hay un campo **🛡 Garantía del trabajo (días)** con presets rápidos (Sin / 7 / 15 / 30 / 60 / 90 / 180). El valor se precarga automáticamente al cobrar.

---

## v2.4.23 — 2026-05-11 🧾 Abonos en orden impresa

**El PDF de la orden de servicio ahora muestra los abonos recibidos.**

Antes: el PDF mostraba "Presupuesto: $15.00" pero si el cliente ya había abonado $5, eso no aparecía. El cliente se llevaba la orden impresa sin constancia de su pago.

**Ahora**: nueva sección "ABONOS RECIBIDOS" debajo del presupuesto con:
- Lista de cada abono: fecha, monto, forma de pago, referencia (si tiene)
- "Total abonado: $X.XX"
- "Saldo pendiente: $Y.YY" (calculado como `presupuesto/total - total_abonos`)
- Si el saldo es 0: "CANCELADO TOTALMENTE"

Aplica tanto a abonos en HOLDING (orden abierta, en proceso) como APLICADOS (orden ya cobrada). Funciona en formato A4 y Ticket 80mm.

---

## v2.4.22 — 2026-05-11 🔒 Integridad ST

**Bloqueo de cambio de estado en órdenes ya cerradas (consistencia con abonos y ventas).**

### 🐞 Bug detectado por usuario

Si una orden ya estaba ENTREGADO/ENTREGADO_PARCIAL o CANCELADA, los botones de "Cambiar estado" seguían activos y permitían retroceder a RECIBIDO/DIAGNOSTICANDO/etc. Eso generaba inconsistencia grave:

- **ENTREGADO/ENTREGADO_PARCIAL**: ya hay una venta generada y los abonos HOLDING pasaron a APLICADO. Si retrocedes el estado, la orden parece abierta pero los abonos APLICADOS ya no están en caja como HOLDING → caja descuadrada vs. orden.
- **CANCELADA**: los abonos se devolvieron al cliente (estado DEVUELTO). Reabrir la orden la haría parecer abierta sin abonos.

### Fix v2.4.22

**Frontend**: si la orden está cerrada, en lugar del selector de estados muestra un panel informativo con el estado actual y la razón del bloqueo. Sugiere "anula la venta primero desde Ventas del Día" si se necesita reabrir.

**Backend** (`cambiar_estado_orden`): doble validación:
- Rechaza cambio si `estado_anterior` ∈ {ENTREGADO, ENTREGADO_PARCIAL, CANCELADA, CANCELADO}
- Rechaza cambio directo *hacia* esos estados — el flujo correcto es "💰 Cobrar" y "🚫 Cancelar orden", que hacen las operaciones contables completas.

Los estados abiertos (RECIBIDO ↔ DIAGNOSTICANDO ↔ EN_REPARACION ↔ ESPERANDO_REPUESTOS ↔ LISTO ↔ GARANTIA) siguen siendo intercambiables libremente.

---

## v2.4.21 — 2026-05-11 🚨 SECURITY HOTFIX + UX

**Mensaje de PIN duplicado revelaba el dueño + permisos implícitos por rol.**

### 🚨 SECURITY: Oráculo de PINs en mensaje de error

En v2.4.20 el mensaje de validación de PIN duplicado decía: *"El PIN ya está en uso por 'JUAN'"*. Eso convertía el formulario de crear/editar usuario en un **oráculo**: cualquiera con permiso de gestionar usuarios podía tantear PINs (1234, 0000, etc.) y descubrir el PIN exacto de cualquier otro usuario.

**Fix**: mensaje genérico *"Este PIN ya está en uso. Elige otro."* — sin nombre. Aplicado en crear usuario y cambiar PIN. El helper `pin_duplicado()` sigue retornando el nombre internamente (para auditoría futura), pero los call-sites usan `.is_some()` y nunca propagan al cliente.

### 🆕 Permisos implícitos por rol (TECNICO ↔ Servicio Técnico)

Antes: el rol TECNICO se creaba con `permisos = "{}"` (vacío). El usuario TECNICO recién creado **no veía el módulo Servicio Técnico** en el sidebar hasta que un admin le asignaba manualmente los permisos `gestionar_servicio_tecnico` o `ver_servicio_tecnico`. UX horrible.

**Fix**: el rol TECNICO ya implica esos permisos automáticamente:
- Frontend (`SesionContext.tienePermiso`): si `rol === "TECNICO"` y el permiso es de ST, devuelve true sin chequear el JSON.
- Backend app móvil (`AppSession.tiene`): mismo patrón. El técnico móvil ya puede usar la app sin pasos extra.

ADMIN sigue teniendo bypass total (rol > permisos). CAJERO necesita permisos explícitos como antes.

---

## v2.4.20 — 2026-05-11 🔒

**Bug seguridad PIN + UX en gestión de usuarios y órdenes ST.**

### 🚨 Bug seguridad: PIN duplicado entre usuarios

Si dos usuarios tenían el mismo PIN, el login no era determinístico — devolvía el primero que matcheaba en orden de inserción. El segundo usuario nunca podía entrar con su PIN, y peor, el admin creía que estaba logueado como "Juan" pero era "María" (mismo PIN).

**Fix**:
- Al **crear** un usuario con un PIN ya en uso → rechaza con mensaje claro: "El PIN ya está en uso por 'X'"
- Al **cambiar PIN** de un usuario existente → mismo chequeo (excluyendo al propio usuario para permitir guardar sin cambios)
- Helper `pin_duplicado(conn, pin, excluir_id?)` que rehashea el PIN candidato con cada salt y compara

⚠ Si ya tenías PINs duplicados en BD, siguen funcionando como antes (el primero matchea). Te recomendamos cambiar los PINs duplicados manualmente a valores únicos.

### 🆕 Editar nombre de usuario en Configuración

Antes solo se podía cambiar PIN, contraseña, permisos y activar/desactivar. Ahora **click en el nombre** del usuario lo convierte en input editable. Enter o "OK" guarda. Backend ya soportaba el cambio de nombre, faltaba la UI.

### 🆕 Cambiar técnico asignado en orden ST

En el modal de detalle de la orden, nuevo selector "👤 Técnico asignado" que permite **reasignar** la orden a otro técnico en cualquier momento (no solo al crear). Útil cuando un técnico sale, está ocupado o se requiere reasignar trabajo. Auto-guarda al cambiar.

---

## v2.4.19 — 2026-05-11 📱

**Crear órdenes de servicio desde la app móvil.**

### 🆕 Nuevo endpoint: crear orden ST desde móvil

`POST /api/v1/app/st/ordenes` — el técnico/coordinador con permiso `gestionar_servicio_tecnico` ahora puede crear órdenes de servicio directamente desde su celular.

- Genera número correlativo automáticamente (`OS-XXXXXX`)
- Auto-busca cliente existente por identificación, teléfono o nombre. Si no encuentra, registra el nombre/teléfono igual sin vincular a un cliente del catálogo (puede vincularse después desde el POS desktop).
- Auto-asigna al técnico que la creó (`tecnico_id` = quien hizo el POST)
- Estado inicial `RECIBIDO`
- Log en historial de movimientos: "Creada desde app móvil" + nombre del usuario
- Validación: cliente, equipo y problema son obligatorios

Útil para taller con técnico itinerante: el técnico va al cliente, abre la orden desde su celular, le toma fotos, y queda registrada en el sistema central inmediatamente.

---

## v2.4.18 — 2026-05-11 📱

**Backend para Sprint 6 de la app móvil + fixes UX en POS desktop.**

### 🆕 Backend para app móvil — push notifications a cocineros

- Nuevo módulo `app_movil/push.rs` con cliente de Expo Push API.
- Función `tokens_por_permiso(db, "ve_cocina")` busca todos los push tokens activos de usuarios con ese permiso.
- `enviar_push_async(...)` dispara notificación en background (tokio::spawn, no bloquea la API).
- Endpoint nuevo `POST /api/v1/app/auth/push-token` para que la app registre su Expo Push Token al login.
- **Integración en `pedidos_enviar_cocina`**: cuando un mesero envía a cocina, dispara push automática a todos los cocineros conectados con título "🍳 Nueva comanda" y body con mesa + items.
- `AppSession` ahora incluye `token_id` para asociar push token al dispositivo correcto.

### 🆕 Backend para app móvil — Servicio Técnico

5 endpoints nuevos bajo `/api/v1/app/st/*` para que el técnico use la app desde el celular:
- `GET /mis-ordenes` — lista órdenes activas (filtradas por `tecnico_id` si no es admin/coordinador)
- `GET /ordenes/:id` — detalle completo + galería de imágenes
- `POST /ordenes/:id/estado` — cambia estado + log en historial
- `POST /ordenes/:id/diagnostico` — guarda diagnóstico/trabajo/observaciones
- `POST /ordenes/:id/imagen` — sube imagen base64 (ANTES/DESPUÉS/GENERAL)

### 🐞 Fix: imágenes de productos no se ven completas

`PosGridTactil`, `SelectorProductos` (restaurante) y preview en editor de Productos usaban `objectFit: cover` que recortaba el producto. Ahora `objectFit: contain` muestra el producto completo con fondo neutro semi-transparente para llenar el espacio sobrante.

### 🔧 Internal

- Refactor: `ApiError::new`, `err400`, `err500`, `extract_app_session` ahora son `pub` para poder reutilizarse desde `http_st.rs` (módulo nuevo de Servicio Técnico).

---

## v2.4.17 — 2026-05-11 🛠

**Hotfix gating: licencia es la fuente de verdad para todos los módulos.**

En v2.4.16 el toggle y la sección "Cocina" en Configuración ya respetaban la licencia, pero el **sidebar seguía mostrando el ícono** de Servicio Técnico aunque admin lo desactivara. Causa: el cálculo era OR (`licencia OR flag local`), así que el flag local "1" mantenía el módulo visible.

**Fix v2.4.17**: la licencia es la fuente de verdad. Si está cargada (`licencia_modulos` no vacío), ESA decide. El flag legacy `modulo_servicio_tecnico` solo se usa como fallback en instalaciones pre-v2.4.8 sin licencia.

Aplicado en 3 lugares:
- `Layout.tsx` → ícono del sidebar
- `CajaPage.tsx` → panel de holdings + advertencia al cerrar
- `ReportesPage.tsx` → tabs "Cancelaciones ST" y "Garantías ST"

Cuando admin desactiva el módulo desde el panel admin, la próxima vez que el cliente refresque la app desaparece TODO (sidebar, configuración, caja, reportes).

---

## v2.4.16 — 2026-05-11 🛠

**Servicios manuales en ticket impreso + gating por licencia.**

### 🐞 Servicios manuales no aparecían en ticket impreso

Después del fix de v2.4.15 (la línea ya se guarda en BD), aún faltaba arreglar la **lectura** en los endpoints de impresión:

- `imprimir_ticket` (ESC/POS) — INNER JOIN filtraba la línea
- `imprimir_ticket_pdf` (PDF térmico) — mismo
- `imprimir_guia_remision_pdf` — mismo
- `nota_venta_pdf` — mismo
- `printing/mod.rs` (renderizado) — mostraba "?" en vez del nombre del servicio

**Fix v2.4.16**: todos los queries cambian a `LEFT JOIN`, y el renderizador usa `info_adicional` como nombre cuando no hay producto vinculado. Ahora el servicio manual aparece en TODOS los formatos de impresión.

### 🔒 Gating por licencia en Configuración

Si admin desactiva un módulo desde el panel admin, los **campos relacionados en Configuración del POS también desaparecen**:

- Toggle "Servicio Técnico" en "Módulos del Negocio" → solo aparece si `licencia_modulos` incluye `servicio_tecnico`.
- Sección "🍳 Cocina (Restaurante)" → solo aparece si licencia incluye `restaurante`.
- Auto-clean del flag local: si la licencia ya no incluye un módulo pero el flag local seguía activo (instalación vieja), se desactiva automáticamente al cargar Configuración. Eso cascadea: tabs en Reportes, panel de holdings en Caja, sección leyenda, etc., también desaparecen.

Esto cierra el caso "admin desactiva el módulo desde admin pero el cliente seguía viendo los campos".

---

## v2.4.15 — 2026-05-11 🚨 HOTFIX

**Hotfix crítico: servicios manuales perdidos al cobrar + mejoras UI.**

### 🚨 Bug crítico (root cause encontrado)

En v2.4.14 reporté que el detalle de venta no mostraba servicios manuales y lo "arreglé" con `LEFT JOIN`. **Eso era solo el síntoma.** El root cause real:

La tabla `venta_detalles` tenía `producto_id INTEGER NOT NULL` desde la versión inicial. Cuando `cobrar_orden_servicio` insertaba la línea de un servicio manual (con `producto_id = NULL`), el INSERT **fallaba silenciosamente** por el `.ok()` que ignora errores. Resultado: la línea NUNCA se guardaba en BD.

→ El `LEFT JOIN` de v2.4.14 no servía de nada porque no había nada que traer.

**Fix v2.4.15**:
- Schema base: `producto_id INTEGER` (sin NOT NULL).
- Migración para BDs existentes: detecta vía `pragma_table_info` si la columna sigue siendo NOT NULL y, si lo es, recrea la tabla preservando todos los datos (transaccional con `PRAGMA foreign_keys = OFF`).
- Las nuevas ventas con servicios manuales ya guardan correctamente.

⚠ **Las ventas viejas que perdieron la línea** (NV pre-v2.4.15 generadas desde órdenes con servicios manuales) **no se pueden recuperar** — la línea nunca se persistió. El total de la venta sí está correcto, solo falta la línea visual del servicio. Cuando abras esas ventas verás solo los productos del catálogo (info histórica perdida, pero contabilidad intacta).

### 🆕 Form de servicio manual con labels claros + cantidad

Antes: 3 inputs sin labels, con placeholders que desaparecían al tipear. El usuario veía "DIAGNOSTICO / 15 / 0" sin entender qué era el "0".

**Ahora**:
- Labels visibles arriba de cada campo: **Descripción \* / Cantidad / Precio unitario \* / IVA %**
- Campo nuevo "Cantidad" (default 1, editable)
- Backend ya soportaba cantidad — solo se exponía en UI

### 🔒 Gating: tabs ST en Reportes

Los tabs "🚫 Cancelaciones ST" y "🛡 Garantías ST" en Reportes ahora **solo aparecen si el módulo Servicio Técnico está activo** en Configuración. Antes aparecían siempre (rebotaban en backend).

### 🔎 Buscador inteligente en reportes ST

Ambos tabs (Cancelaciones y Garantías) ahora tienen un input de búsqueda con filtro inteligente:

- **Cancelaciones**: busca en orden, cliente, teléfono, equipo, marca, modelo, motivo, usuario que canceló.
- **Garantías**: busca en orden, cliente, teléfono, equipo, marca, modelo, serie.
- Botón × para limpiar.
- Contador "X de Y" cuando hay filtro activo.
- Totales del footer se recalculan según el filtro.

---

## v2.4.14 — 2026-05-10 🛠

**Cierre de mejoras ST + bug fix detalle de venta + miniaturas en productos.**

### 🐞 Bug fix: detalle de venta no mostraba servicios manuales

`obtener_venta` hacía `INNER JOIN productos`, así las líneas con `producto_id NULL` (servicios manuales de orden de servicio técnico) desaparecían del detalle. El total decía $28 pero solo se veía el repuesto de $3.

**Fix**: cambio a `LEFT JOIN`, modelo `VentaDetalle.producto_id` ahora es `Option<i64>` (nullable), modal muestra `info_adicional` como nombre cuando no hay producto vinculado.

### 🆕 Miniaturas en listado de Productos

El listado mostraba solo texto. Ahora cada fila muestra una miniatura 36x36 de la imagen del producto si tiene una; si no, un placeholder "📦".

- Backend: nuevo flag `tiene_imagen: bool` en `ProductoBusqueda` (cheap query, no carga la imagen completa).
- Frontend: componente `<ProductoMiniatura>` con **lazy-load por IntersectionObserver** — solo pide la imagen al backend cuando la fila entra al viewport. Cachea por id en memoria de sesión. Funciona bien con 1300+ productos sin cargar 1300 base64.

### 🆕 Pie de página configurable desde el modal del módulo ST

Antes: la leyenda/términos solo se editaba desde Configuración → Servicio Técnico (varios clicks).

**Ahora**: también editable desde el botón "⚙ Configuración" del propio módulo, en un panel arriba del catálogo de tipos/marcas/modelos. Mismo campo (`leyenda_orden_servicio`), dos puntos de acceso.

### 🆕 Cobranza parcial (entrega con saldo pendiente)

Caso real: el cliente quiere llevarse el equipo y deja parte del pago para después. Antes esto era imposible — el backend rechazaba si no se cubría el total.

**Ahora**:
- Schema: nueva columna `saldo_pendiente REAL` en `ordenes_servicio`.
- Estado nuevo `ENTREGADO_PARCIAL`.
- Modal de cobrar muestra checkbox "**Permitir saldo pendiente**" cuando el monto pagado es menor al saldo (con explicación clara).
- Backend `cobrar_orden_servicio` acepta `permitirSaldoPendiente?: boolean`. Si es true y hay diferencia, marca la orden con saldo y estado parcial.
- El historial registra el motivo (`Cobrado parcial · saldo pendiente $X`).

### 🆕 Botón "📱 Avisar al cliente" (WhatsApp)

En el footer del modal de orden, si el cliente tiene teléfono, aparece un botón verde 📱.

- Click → abre `wa.me/<telefono>?text=<mensaje>` con plantilla automática según el estado de la orden:
  - `LISTO` → "Su [equipo] (orden #X) está listo para retirar"
  - `ENTREGADO_PARCIAL` → "Le recordamos que tiene un saldo pendiente sobre la orden..."
  - `ESPERANDO_REPUESTOS` → "Está en espera de repuestos. Le avisaremos apenas llegue"
  - `DIAGNOSTICANDO` / `EN_REPARACION` → "Está actualmente en proceso..."
- Asume Ecuador (+593) si el número no tiene código de país.

### 🆕 Reporte de garantías activas

Nuevo tab "🛡 Garantías ST" en Reportes. Lista órdenes ENTREGADAS con garantía vigente (fecha_entrega + garantia_dias > hoy).

- KPIs: total activas + por vencer en ≤30 días.
- Tabla: orden, cliente (con tel), equipo (marca+modelo+serie), fecha entrega, días garantía, fecha vencimiento, días restantes (color: rojo ≤7d, naranja ≤30d, verde >30d), monto.
- Útil cuando un cliente vuelve por garantía → datos a la mano.

### 🆕 Reporte de cancelaciones ST

Nuevo tab "🚫 Cancelaciones ST" en Reportes. Lista órdenes canceladas con motivo, usuario que canceló, abonos devueltos y monto.

- KPIs: total canceladas + abonos devueltos + monto total.
- Filtro por rango de fechas (default últimos 30 días).

### 🆕 Botón limpiar búsqueda en módulo ST

Input de búsqueda muestra una × cuando hay texto. Click borra y recarga.

### 🔒 Gating del módulo ST en Caja

El panel de "Anticipos en holding" solo aparece si `modulo_servicio_tecnico` está activo. Antes se intentaba cargar siempre (rebotaba en backend, pero ahora ni se intenta).

### 🇪🇨 Localización (continuación)

Más voseo argentino → español neutro:
- `Configuracion`: dejas/tienes/seleccionala
- `ModalHistorialServicioTecnico`: "Ve a Ventas → busca"
- `Productos`: "Usa: ..."

---

## v2.4.13 — 2026-05-09 🛠

**ST-5 — Items presupuestados, abonos en holding, pago mixto, cancelar orden, jerarquía estricta de catálogo.**

### 🆕 Items presupuestados en la orden

Antes: solo había un campo libre "Monto final" que el técnico escribía a mano. Al cobrar, aparecían descuadres porque la línea de "servicio" no se mostraba en el detalle de la venta.

**Ahora**: cada orden tiene una **lista de items** (productos del catálogo + servicios manuales) que se construye antes del cobro:
- Tabla nueva `orden_servicio_items` (id, orden_id, producto_id?, descripción, cantidad, precio, IVA, es_servicio).
- 5 comandos backend: `st_listar_items_orden`, `st_agregar_item_orden`, `st_actualizar_item_orden`, `st_eliminar_item_orden`, `st_total_orden`.
- UI en el modal de detalle: tabla editable inline (cantidad/precio se guardan al blur) + buscador de productos del catálogo + botón "+ Servicio manual" (mano de obra, etc.).
- El total se calcula automáticamente desde los items (subtotal sin IVA, subtotal con IVA, IVA, total).

### 💵 Abonos / anticipos en holding

El cliente puede pagar adelantado al dejar el equipo. Ese dinero entra a caja pero queda en estado **HOLDING** (no es venta) hasta que la orden se cobra (APLICADO) o se cancela (DEVUELTO).

- Tabla nueva `st_abonos` con estados HOLDING / APLICADO / DEVUELTO.
- 5 comandos backend: `st_listar_abonos`, `st_recibir_abono`, `st_total_abonos_orden`, `st_cancelar_orden`, `st_listar_holdings_caja`.
- UI: sección "💵 Abonos / Anticipos" en el modal de orden con form para recibir abono (efectivo / transferencia / tarjeta + banco + referencia).
- **Validación**: el monto holding total no puede exceder el total de items de la orden.
- **Caja**: panel de "Anticipos en holding" en el cierre de caja con detalle por orden + advertencia visual (este dinero NO debe retirarse — pertenece a clientes).
- **Confirmación al cerrar**: si hay holdings, el modal de cerrar caja avisa el monto y cantidad antes de confirmar.

### 🚫 Cancelar orden + devolución automática

- Nuevo botón "🚫 Cancelar orden" en el footer del modal (cualquier cajero, sin requerir admin).
- Marca la orden como `CANCELADA`.
- Devuelve abonos HOLDING → DEVUELTO automáticamente con monto y cantidad.
- Registra en el historial de movimientos quién canceló y por qué.

### 💳 Pago mixto en cobro de orden

Antes: una sola forma de pago al cobrar. Si el cliente pagaba parte en efectivo y parte con transferencia, no se podía registrar correctamente.

**Ahora**: el modal de cobrar acepta **múltiples pagos** (igual que el POS):
- Lista de pagos (forma + monto + banco/referencia opcionales).
- Botón "+ Agregar pago" para combinar formas.
- Atajo "= Saldo" para autocompletar el primer pago al saldo exacto.
- Resumen visual: Total ítems − Abonos en holding = Saldo a cobrar; total pagado vs saldo; cambio si hay sobrante en efectivo.
- Backend `cobrar_orden_servicio` refactorizado: lee items de la tabla, acepta `pagos: Vec<{forma, monto, banco_id?, ref?}>`, aplica abonos HOLDING como descuento, marca abonos como APLICADO con `venta_id`. Compat: si vienen los parámetros viejos (`forma_pago` + `items_repuestos`), funciona como antes.

### 🌳 Jerarquía estricta tipo → marca → modelo

Antes: el campo Marca y Modelo eran inputs libres. Si el usuario tipeaba "Dell" sin tipo seleccionado, no quedaba vinculado al árbol del catálogo y aparecían modelos mezclados (ej: Latitude bajo Lenovo).

**Ahora**:
- **Marca**: deshabilitada hasta que se elija Tipo de equipo. Las opciones son **solo las del tipo seleccionado**.
- **Modelo**: deshabilitado hasta que se elija Marca. Las opciones son **solo las de esa marca**.
- Placeholders claros: "Elige primero un tipo", "Elige primero una marca".
- ComboCatalogoEquipo respeta `disabled` (no abre dropdown, fondo gris, cursor not-allowed).

### 📜 Leyenda configurable en orden de servicio

- Nuevo campo en Configuración → Servicio Técnico: textarea "📜 Leyenda / términos en orden de servicio" (clave `leyenda_orden_servicio`).
- Se imprime al final de cada orden bajo el título "TÉRMINOS Y CONDICIONES" (sobre la firma).
- Útil para cláusulas de equipo abandonado, garantías, formas de pago aceptadas, etc.

### ✏ Firma única en orden impresa

- La orden ya solo muestra **Firma del Cliente** (se quitó "Firma del Técnico" que era redundante).

### 🐞 Bug fix: detalle de venta con líneas sin producto_id

Las líneas con `producto_id = NULL` (servicios técnicos) no se mostraban en el modal "Detalle de Venta" porque el JOIN no devolvía `nombre_producto`. Total decía $28 pero solo se veía el repuesto de $3.

**Fix**: si la línea no tiene producto vinculado, se muestra el `info_adicional` como nombre. Si tiene producto + info_adicional, se muestran ambos.

### 🇪🇨 Localización (parcial)

Cambios de español argentino (voseo) → español neutro/ecuatoriano:
- "Elegí o escribí" → "Elige o escribe"
- "Ingresá una cédula" → "Ingresa una cédula"
- "Esta seguro que desea cerrar la caja" → "¿Estás seguro que deseas cerrar la caja?"
- (Continúa de a poco en cada release)

---

## v2.4.12 — 2026-05-09 🛠 STABLE
**ST-4 / 5 — PDF A4 + Ticket 80mm + hotfix historial + garantía al cobrar.**

### 🐞 Hotfix Historial — feedback usuario

**Bugs reportados:**
1. Filtros separados de "Placa" y "Serie" → si buscabas "3432" en Placa pero el equipo era PC con `serie="3432222"`, no aparecían resultados (el filtro era exclusivo).
2. Labels fijos "Placa/Serie" sin importar tipo de negocio (taller mecánico vs taller electrónico).

**Fixes:**
- **Campo unificado `Placa / Serie`** en filtros del historial — busca en `equipo_placa` + `equipo_serie` + `equipo_descripcion` con un solo input.
- **Labels adaptables según `tipo_taller`** (config):
  - `AUTOMOTRIZ` → "Placa / Chasis"
  - `ELECTRODOMESTICO` / `ELECTRONICO` / `COMPUTADORAS` → "Serie / IMEI"
  - `MIXTO` (default) → "Placa / Serie"
- Backend: nuevo filtro `identificador_equipo` (los antiguos `placa` y `serie` se mantienen por backward-compat)
- **Filas expandibles** en el historial — click en ▶ muestra problema reportado, diagnóstico, trabajo realizado y botón "📋 Abrir orden completa" + indicador de venta vinculada
- **Columna "Venta"** con badge `📄 #X` cuando la orden generó una venta

### 🆕 ST-4 — Imprimir orden en A4 o Ticket 80mm

Antes: solo PDF tamaño grande (mezcla de A4/A5 sin claridad).

**Ahora**: en el detalle de orden, **selector de formato** (A4 / 80mm) + botón Imprimir. Al cambiar de formato:
- **A4** (default): paper 210×297, márgenes 15mm, fonts 10-16pt — para impresora normal
- **TICKET_80**: paper 80×297mm, márgenes 3mm, fonts 8-12pt — para térmica 80mm

El parámetro `formato` se pasa al backend (`imprimir_orden_servicio_pdf`). Si una versión vieja del frontend no lo manda, default es A4 (backward-compat).

### 🆕 Garantía al cobrar

Al click en **💰 Cobrar** ahora aparece un campo **🛡 Garantía del trabajo (días)** con:
- Input numérico
- Atajos rápidos: `Sin / 7d / 15d / 30d / 60d / 90d / 180d`
- Default = el valor que ya tiene la orden (precarga al abrir el modal)

Backend: `cobrar_orden_servicio` ahora acepta parámetro `garantia_dias` opcional. Si viene, actualiza `ordenes_servicio.garantia_dias` antes de generar la venta. Toast confirma "Cobrado y entregado · 🛡 Garantía X días".

### 📦 Archivos tocados

- `src-tauri/src/commands/servicio_tecnico.rs` — `cobrar_orden_servicio` con garantía + `imprimir_orden_servicio_pdf` con formato A4/Ticket
- `src-tauri/src/commands/servicio_tecnico_catalogo.rs` — filtro `identificador_equipo` unificado
- `src/services/api.ts` — wrappers actualizados (garantía + formato)
- `src/components/ModalHistorialServicioTecnico.tsx` — campo único + labels adaptables + filas expandibles + columna Venta + componente FilaExpandida
- `src/pages/ServicioTecnicoPage.tsx` — selector formato + selector garantía + handlers

---

## v2.4.11 — 2026-05-09 🆔 STABLE
**ST-3 / 5 — Búsqueda de cliente con SRI desde form de orden de servicio.**

### 🆕 Lo que entrega

El form de orden de servicio ahora tiene **3 inputs en lugar de 2** para identificar al cliente:

```
┌──────────────────────┬──────────────┬──────────┬─────────┐
│ Nombre del cliente   │ Cédula / RUC │ Teléfono │ 🔍 SRI  │
└──────────────────────┴──────────────┴──────────┴─────────┘
```

Y la lógica:

1. **Buscar local automático**: al escribir cédula/RUC y completar 10 dígitos (cédula) o 13 (RUC), busca en clientes locales. Si encuentra exacto → autocompleta nombre/teléfono y vincula al cliente existente.
2. **Botón 🔍 SRI**: si no encontró local, click consulta SRI Ecuador (mismo `consultar_identificacion` que usa el POS). El SRI devuelve el nombre del contribuyente, lo crea localmente como cliente nuevo, y queda vinculado al form. Toast confirma "Cliente cargado del SRI: ...".
3. **Enter en el campo cédula/RUC** dispara la consulta al SRI directamente (atajo).
4. **Búsqueda por nombre** sigue funcionando como antes (autocomplete en el campo nombre).

Badge verde **"✓ vinculado al cliente #X"** indica cuando el form está vinculado a un cliente real (vs solo nombre suelto).

### Reuso

Reusa `consultar_identificacion` (servicio del SRI Ecuador ya implementado para el POS desktop). Ningún backend nuevo.

### 📦 Archivos tocados

- `src/pages/ServicioTecnicoPage.tsx`:
  - Import `consultarIdentificacion`
  - 2 estados nuevos: `busquedaIdentif`, `consultandoSri`
  - Handler `consultarSriHandler`
  - Bloque cliente refactorizado con grid 4 columnas

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
