"use client";

import { useEffect, useState } from "react";

import { resolvePhotoSrc } from "@/lib/photo-path";

/**
 * Resuelve un `photo_path` relativo (columna de `products`/`services`/
 * `packages`) al src que necesita un `<img>`/`next/image`, vía
 * `convertFileSrc`. Null mientras no hay foto o aún se está resolviendo.
 */
export function usePhotoSrc(photoPath: string | null | undefined) {
  const [src, setSrc] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;
    if (!photoPath) {
      setSrc(null);
      return;
    }
    resolvePhotoSrc(photoPath).then((resolved) => {
      if (!cancelled) setSrc(resolved);
    });
    return () => {
      cancelled = true;
    };
  }, [photoPath]);

  return src;
}
