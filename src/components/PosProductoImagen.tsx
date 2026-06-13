// Imagen de producto para las cards del grid de venta, con lazy-load por viewport.
// El listado tactil (`listar_productos_tactil`) ya NO devuelve la imagen base64
// (eran decenas de MB), solo el flag `tiene_imagen`. Este componente observa
// cuando la card entra a la vista y solo entonces pide la imagen real, cacheada
// por id en memoria de sesion. Mientras carga (o si no tiene), muestra la inicial
// del producto como en el diseno original.

import { useEffect, useRef, useState } from "react";
import { obtenerProducto } from "../services/api";

interface Props {
  productoId: number;
  tieneImagen: boolean;
  nombre: string;
}

// Cache compartido id -> base64 (o null si no hay/falla)
const cache = new Map<number, string | null>();

export default function PosProductoImagen({ productoId, tieneImagen, nombre }: Props) {
  const ref = useRef<HTMLDivElement>(null);
  const [b64, setB64] = useState<string | null>(() => cache.get(productoId) ?? null);
  const [visible, setVisible] = useState(false);

  useEffect(() => {
    if (!tieneImagen || cache.has(productoId)) {
      setVisible(true);
      return;
    }
    const el = ref.current;
    if (!el) return;
    const obs = new IntersectionObserver(
      (entries) => {
        if (entries[0].isIntersecting) {
          setVisible(true);
          obs.disconnect();
        }
      },
      { rootMargin: "150px" }
    );
    obs.observe(el);
    return () => obs.disconnect();
  }, [tieneImagen, productoId]);

  useEffect(() => {
    if (!visible || !tieneImagen) return;
    if (cache.has(productoId)) {
      setB64(cache.get(productoId) ?? null);
      return;
    }
    obtenerProducto(productoId)
      .then((p) => {
        const img = (p as any).imagen || null;
        cache.set(productoId, img);
        setB64(img);
      })
      .catch(() => {
        cache.set(productoId, null);
        setB64(null);
      });
  }, [visible, tieneImagen, productoId]);

  // Inicial del producto (fallback y placeholder mientras carga)
  const inicial = (
    <div
      ref={ref}
      style={{
        width: "100%", height: "100%",
        background: "linear-gradient(135deg, rgba(59,130,246,0.15) 0%, rgba(59,130,246,0.05) 100%)",
        display: "flex", alignItems: "center", justifyContent: "center",
        color: "var(--color-primary)", fontSize: 56, fontWeight: 800,
      }}
    >
      {nombre.charAt(0).toUpperCase()}
    </div>
  );

  if (!tieneImagen || !b64) return inicial;

  return (
    <img
      ref={ref as any}
      src={`data:image/png;base64,${b64}`}
      alt={nombre}
      loading="lazy"
      decoding="async"
      style={{
        width: "100%", height: "100%", objectFit: "contain",
        display: "block",
        background: "rgba(255,255,255,0.06)",
      }}
    />
  );
}
