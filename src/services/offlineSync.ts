import { invoke } from "@tauri-apps/api/core";

// Comandos que se pueden encolar offline (escrituras)
const COMANDOS_ENCOLABLES = [
  "registrar_venta",
  "crear_gasto",
  "registrar_pago_cuenta",
  "abrir_caja",
  "cerrar_caja",
];

// Estado de conexión global
let _online = true;
let _onStatusChange: ((online: boolean, pendientes: number) => void) | null = null;
let _syncInterval: ReturnType<typeof setInterval> | null = null;

export function isOnline() {
  return _online;
}

export function setOnline(val: boolean) {
  _online = val;
  notifyStatus();
}

export function onConnectionStatusChange(cb: (online: boolean, pendientes: number) => void) {
  _onStatusChange = cb;
}

async function notifyStatus() {
  if (!_onStatusChange) return;
  try {
    const count = await invoke<number>("contar_cola_offline");
    _onStatusChange(_online, count);
  } catch {
    _onStatusChange(_online, 0);
  }
}

/** Verifica si un comando puede ser encolado offline */
export function esComandoEncolable(command: string): boolean {
  return COMANDOS_ENCOLABLES.includes(command);
}

/** Encola una operación para sincronizar después */
export async function encolarOperacion(command: string, args: Record<string, unknown>): Promise<void> {
  await invoke("encolar_operacion", {
    comando: command,
    paramsJson: JSON.stringify(args),
  });
  notifyStatus();
}

/** Intenta sincronizar operaciones pendientes con el servidor */
export async function sincronizarCola(servidorUrl: string, token: string): Promise<{ enviadas: number; errores: number }> {
  const pendientes = await invoke<Array<{
    id: number;
    comando: string;
    params_json: string;
  }>>("listar_cola_offline");

  let enviadas = 0;
  let errores = 0;

  for (const op of pendientes) {
    try {
      const response = await fetch(`${servidorUrl}/api/v1/invoke`, {
        method: "POST",
        headers: {
          "Content-Type": "application/json",
          Authorization: `Bearer ${token}`,
        },
        body: JSON.stringify({
          command: op.comando,
          args: JSON.parse(op.params_json),
        }),
      });

      const data = await response.json();
      if (data.ok) {
        await invoke("marcar_operacion_enviada", { id: op.id });
        enviadas++;
      } else {
        await invoke("marcar_operacion_error", {
          id: op.id,
          error: data.error || "Error desconocido",
        });
        errores++;
      }
    } catch (err) {
      // Si falla la red, dejar de intentar
      break;
    }
  }

  notifyStatus();
  return { enviadas, errores };
}

/** Sincroniza el cache de productos desde el servidor */
export async function sincronizarCacheProductos(servidorUrl: string, token: string): Promise<number> {
  const response = await fetch(`${servidorUrl}/api/v1/invoke`, {
    method: "POST",
    headers: {
      "Content-Type": "application/json",
      Authorization: `Bearer ${token}`,
    },
    body: JSON.stringify({ command: "listar_productos", args: {} }),
  });

  const data = await response.json();
  if (!data.ok) throw new Error(data.error);

  const count = await invoke<number>("sincronizar_cache_productos", {
    productosJson: JSON.stringify(data.data),
  });

  return count;
}

/** Reserva secuenciales del servidor para uso offline */
export async function reservarSecuenciales(
  servidorUrl: string,
  token: string,
  establecimiento: string,
  puntoEmision: string,
  tipoDocumento: string,
  cantidad: number = 50,
): Promise<void> {
  const response = await fetch(`${servidorUrl}/api/v1/invoke`, {
    method: "POST",
    headers: {
      "Content-Type": "application/json",
      Authorization: `Bearer ${token}`,
    },
    body: JSON.stringify({
      command: "reservar_secuenciales",
      args: { establecimiento, puntoEmision, tipoDocumento, cantidad },
    }),
  });

  const data = await response.json();
  if (!data.ok) throw new Error(data.error);

  await invoke("guardar_secuenciales_reservados", {
    tipoDocumento,
    desde: data.data.desde,
    hasta: data.data.hasta,
  });
}

/** Inicia el servicio de sincronización background */
export function iniciarSyncService(servidorUrl: string, token: string) {
  if (_syncInterval) return;

  // Intentar sync cada 10 segundos
  _syncInterval = setInterval(async () => {
    try {
      // Ping al servidor
      const res = await fetch(`${servidorUrl}/api/v1/ping`, {
        signal: AbortSignal.timeout(3000),
      });
      const body = await res.text();

      if (body === "clouget-pos-server") {
        if (!_online) {
          setOnline(true);
          // Reconectado: sincronizar cola
          await sincronizarCola(servidorUrl, token);
        }
      }
    } catch {
      if (_online) {
        setOnline(false);
      }
    }
  }, 10000);
}

/** Detiene el servicio de sincronización */
export function detenerSyncService() {
  if (_syncInterval) {
    clearInterval(_syncInterval);
    _syncInterval = null;
  }
}
