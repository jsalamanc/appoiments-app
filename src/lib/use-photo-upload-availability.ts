"use client";

import { invoke } from "@tauri-apps/api/core";
import { useEffect, useState } from "react";

type StorageModeTag = { mode: "local" | "nube" };

/**
 * En modo local las fotos se guardan en disco, sin requisitos. En modo
 * "Nube" se suben a un bucket S3-compatible (ver
 * `util::photos::import_and_sync_photo` en Rust) — si todavía no hay uno
 * configurado, la subida fallaría a medio formulario, así que la UI debe
 * deshabilitar el selector de foto antes de llegar ahí.
 *
 * `true` mientras se resuelve (evita parpadear a "deshabilitado" en el
 * primer render); si la consulta falla, se asume disponible para no bloquear
 * el formulario por un problema ajeno a la foto en sí.
 */
export function usePhotoUploadAvailability() {
  const [available, setAvailable] = useState(true);

  useEffect(() => {
    let cancelled = false;
    (async () => {
      try {
        const mode = await invoke<StorageModeTag>("storage_get_mode");
        if (mode.mode === "local") {
          if (!cancelled) setAvailable(true);
          return;
        }
        const bucket = await invoke<unknown | null>("bucket_get_config");
        if (!cancelled) setAvailable(bucket !== null);
      } catch {
        if (!cancelled) setAvailable(true);
      }
    })();
    return () => {
      cancelled = true;
    };
  }, []);

  return available;
}
