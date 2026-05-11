// v2.4.14: miniatura de imagen de producto con lazy-load por viewport.
// El listado de productos puede tener miles de items. En vez de traer 1300+ base64
// (decenas de MB), `listar_productos` ahora devuelve `tiene_imagen: bool`. Este
// componente se monta para cada fila, observa cuando entra al viewport y solo
// entonces pide la imagen real al backend. Cachea por id en memoria de sesion.

import { useEffect, useRef, useState } from "react";
import { obtenerProducto } from "../services/api";

interface Props {
  productoId: number;
  tieneImagen: boolean;
  size?: number;
}

const cache = new Map<number, string | null>(); // id → base64 o null si error

export default function ProductoMiniatura({ productoId, tieneImagen, size = 36 }: Props) {
  const ref = useRef<HTMLDivElement>(null);
  const [b64, setB64] = useState<string | null>(() => cache.get(productoId) ?? null);
  const [visible, setVisible] = useState(false);

  // Observer: marca visible cuando entra al viewport
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
      { rootMargin: "100px" } // pre-cargar 100px antes de entrar a la vista
    );
    obs.observe(el);
    return () => obs.disconnect();
  }, [tieneImagen, productoId]);

  // Cuando esta visible y tiene_imagen, pedir
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

  const styleBase = {
    width: size, height: size, borderRadius: 6,
    border: "1px solid var(--color-border)",
    background: "var(--color-surface-alt)",
    display: "flex", alignItems: "center", justifyContent: "center",
    overflow: "hidden", flexShrink: 0,
  } as const;

  if (!tieneImagen) {
    return (
      <div ref={ref} style={{ ...styleBase, color: "var(--color-text-muted)", fontSize: 14 }} title="Sin imagen">
        📦
      </div>
    );
  }

  if (!b64) {
    return (
      <div ref={ref} style={{ ...styleBase, color: "var(--color-text-muted)", fontSize: 10 }}>
        ⏳
      </div>
    );
  }

  return (
    <div ref={ref} style={styleBase}>
      <img
        src={`data:image/png;base64,${b64}`}
        alt=""
        style={{ width: "100%", height: "100%", objectFit: "contain" }}
      />
    </div>
  );
}
