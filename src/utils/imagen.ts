/**
 * Comprime/redimensiona una imagen en el cliente antes de guardarla como base64.
 *
 * Pensado para comprobantes (transferencias, depósitos): permite que el usuario
 * suba fotos grandes de celular (3-8 MB) sin que falle ni se ponga lento — la
 * imagen se reescala a un lado máximo y se recodifica como JPEG, quedando
 * legible pero liviana (típicamente < 600 KB).
 *
 * @param file    Archivo de imagen seleccionado por el usuario.
 * @param maxLado Lado máximo (px) del lado más largo. Default 1600 (legible para recibos).
 * @param calidad Calidad JPEG 0..1. Default 0.82.
 * @returns base64 data URL (image/jpeg), o el original si no se pudo procesar.
 */
export async function comprimirImagen(
  file: File,
  maxLado = 1600,
  calidad = 0.82,
): Promise<string> {
  const leerComoDataUrl = (f: File) =>
    new Promise<string>((resolve, reject) => {
      const r = new FileReader();
      r.onload = () => resolve(r.result as string);
      r.onerror = reject;
      r.readAsDataURL(f);
    });

  const dataUrl = await leerComoDataUrl(file);

  // Si no es imagen rasterizable (ej. PDF) o algo falla, devolver el original.
  if (!file.type.startsWith("image/")) return dataUrl;

  try {
    const img = await new Promise<HTMLImageElement>((resolve, reject) => {
      const i = new Image();
      i.onload = () => resolve(i);
      i.onerror = reject;
      i.src = dataUrl;
    });

    let { width, height } = img;
    if (width <= 0 || height <= 0) return dataUrl;

    // Escalar manteniendo proporción si excede el lado máximo
    if (width > maxLado || height > maxLado) {
      const escala = maxLado / Math.max(width, height);
      width = Math.round(width * escala);
      height = Math.round(height * escala);
    }

    const canvas = document.createElement("canvas");
    canvas.width = width;
    canvas.height = height;
    const ctx = canvas.getContext("2d");
    if (!ctx) return dataUrl;
    // Fondo blanco (por si la imagen tiene transparencia → JPEG no la soporta)
    ctx.fillStyle = "#ffffff";
    ctx.fillRect(0, 0, width, height);
    ctx.drawImage(img, 0, 0, width, height);

    const comprimida = canvas.toDataURL("image/jpeg", calidad);
    // Quedarse con la más pequeña (por si el original ya era menor)
    return comprimida.length < dataUrl.length ? comprimida : dataUrl;
  } catch {
    return dataUrl;
  }
}
