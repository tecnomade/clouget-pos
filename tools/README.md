# Herramienta de Licencias - Clouget POS

La herramienta de licencias esta integrada en el proyecto Cargo de `src-tauri/`.
No necesitas crear ningun proyecto separado.

## Comandos (desde la carpeta src-tauri)

### 1. Generar claves (solo la primera vez)

```bash
cd C:\proyectos\clouget-pos\src-tauri
cargo run --bin licencia-tool -- generar-claves
```

Esto imprime:
- **Clave privada** → guardarla en un archivo seguro, NUNCA en git
- **Clave publica** (array Rust) → pegarla en `src/commands/licencia.rs`

### 2. Firmar licencia para un cliente

El cliente te envia su **codigo de maquina** (8 caracteres, ej: `A7F3B21E`) que ve en su pantalla de activacion.

```bash
cd C:\proyectos\clouget-pos\src-tauri
cargo run --bin licencia-tool -- firmar TU_CLAVE_PRIVADA "Tienda Don Juan" "juan@mail.com" A7F3B21E
```

La licencia generada SOLO funcionara en el equipo con ese codigo.

## Flujo completo de venta

1. Cliente instala Clouget POS
2. Ve pantalla de activacion con su **Codigo de Maquina**: `A7F3B21E`
3. Te escribe por WhatsApp: "Quiero comprar, mi codigo es A7F3B21E"
4. Recibes pago por transferencia
5. Generas licencia con el comando de arriba
6. Le envias la clave por WhatsApp
7. La pega en la app → activada!

Si intenta usar esa clave en OTRO equipo → "Esta licencia fue generada para otro equipo"

## Notas

- Si el cliente formatea su PC, el MachineGuid cambia. Genera otra licencia gratis (ya pago).
- Si cambia hardware pero no formatea, no hay problema (MachineGuid es del SO).
- La clave privada NUNCA debe estar en git ni compartirse.
