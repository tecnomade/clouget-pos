# CLAUDE.md — Contexto para Claude Code

> Este archivo es leído automáticamente por Claude Code al abrir el repo.

---

## 🎯 Qué es este proyecto

**`clouget-pos`** — POS de escritorio para Ecuador (Tauri 2 + React 19 + TypeScript +
rusqlite). Target: pequeños comercios, restaurantes, talleres.

- **Repo**: `C:\proyectos\clouget-pos`
- **Versión actual**: ver `package.json` y `src-tauri/tauri.conf.json` (siempre iguales)
- **DB**: SQLite local en `%LOCALAPPDATA%/CloudgetPOS/clouget-pos.db`
- **Updater**: dos canales — `stable` y `beta` — ver `scripts/release.ps1`

Cumplimiento SRI Ecuador, multi-establecimiento, multi-almacén, retenciones,
servicio técnico, restaurante con mesas/comandas, etc.

---

## 🔗 Repo hermano (app móvil)

Existe un repo **separado** para la app móvil que se conecta a este POS:

```
C:\proyectos\clouget-pos-app
```

Stack: **Expo + React Native + TypeScript**. Consume el servidor HTTP embebido
de este POS (axum, puerto `8847`) — los endpoints viven en
`src-tauri/src/app_movil/http.rs` y `http_st.rs`.

### 🚫 Regla crítica: juntos pero no revueltos

**NUNCA modifiques archivos de la app móvil desde este chat.** Si necesitas
ajustar algo móvil:

1. Verifica primero que el cambio sea realmente del lado móvil y no algo que se
   pueda solucionar en el backend de este POS.
2. Si sí necesita la app móvil: **deja una nota clara** indicando qué archivo
   (`clouget-pos-app/app/...` o `clouget-pos-app/src/lib/api.ts`) y qué cambio
   se requiere. El usuario abrirá otra sesión en ese repo para implementarlo.
3. **No edites código del repo `clouget-pos-app` desde este chat.**

Excepción: **leer** archivos de la app móvil para entender qué endpoints
consume está perfecto y recomendado.

### Endpoints HTTP que consume la app móvil

Definidos en `src-tauri/src/app_movil/http.rs` (línea ~1539+). Si modificas un
endpoint, recuerda que puedes romper la app móvil — coordina con el otro repo.

---

## 🏗 Stack & convenciones clave

- **Frontend**: `src/` (React 19 + TypeScript + Vite 7 + react-router-dom)
- **Backend**: `src-tauri/src/` (Rust + axum + rusqlite)
- **Comandos Tauri**: `src-tauri/src/commands/*.rs` (registrados en `lib.rs`)
- **Schema**: `src-tauri/src/db/schema.rs` (migraciones self-healing con `ALTER TABLE` silently-failed)
- **API frontend**: `src/services/api.ts` (wrappers tipados de cada comando)
- **Tabs system**: `src/contexts/TabsContext.tsx` (v2.5.0+) con `display: none` para preservar estado
- **Event bus**: `window.dispatchEvent` para cross-tab communication (ver `clouget:venta-completada`, `clouget:compra-cambio`, etc.)
- **Atajos teclado**: `src/hooks/useKeyboardShortcuts.ts` (F1-F10, Ctrl+B, etc.)

### Convención semántica NV / Factura (v2.5.34+)

- `tipo_documento = 'NOTA_VENTA'` → siempre se muestra como "Nota de Venta", aunque tenga intentos SRI fallidos
- `tipo_documento = 'FACTURA'` → solo cuando `estado_sri = 'AUTORIZADA'`

La promoción NV → FACTURA ocurre **solo** cuando el SRI realmente autoriza
(dentro del UPDATE en `commands/sri.rs::emitir_factura_sri`).

---

## 🚢 Release

Script: `scripts/release.ps1`

```powershell
# Stable (todos los clientes)
powershell -ExecutionPolicy Bypass -File scripts\release.ps1

# Beta (solo testers con canal=beta)
powershell -ExecutionPolicy Bypass -File scripts\release.ps1 -Beta
```

Flujo automatizado: build Tauri → ZIP store-method → firma sign-tool → genera
`latest.json` → crea GitHub Release → para beta también actualiza el tag movible
`beta-channel`.

Para bumpear versión: actualizar **ambos** `package.json` y
`src-tauri/tauri.conf.json` antes de correr el script.

---

## 📝 Para historia y decisiones técnicas

- `CHANGELOG.md` — historial completo por versión, en español, con detalles
- Memorias del proyecto en `C:\Users\usuario1\.claude\projects\` (auto-cargadas)
