"use client";

import { listen } from "@tauri-apps/api/event";
import { useEffect } from "react";
import { toast } from "sonner";

import {
  STORAGE_MIGRATION_EVENT,
  STORAGE_MIGRATION_TOAST_ID,
  type StorageMigrationProgress,
} from "@/lib/storage-migration";

/**
 * Montado en el layout raíz (no en Ajustes) a propósito: el toast de
 * progreso debe seguir visible y actualizándose aunque el usuario navegue a
 * otra pantalla mientras `storage_switch_mode` migra los datos locales a
 * Postgres/Nube — con mucho historial puede tardar varios minutos, al
 * insertar fila por fila (ver `migration::write_snapshot_to_postgres`).
 */
export function StorageMigrationListener() {
  useEffect(() => {
    let active = true;
    let unlisten: (() => void) | undefined;

    listen<StorageMigrationProgress>(STORAGE_MIGRATION_EVENT, (event) => {
      const { done, total } = event.payload;
      const percent = total > 0 ? Math.round((done / total) * 100) : 100;
      toast.loading(`Migrando datos a la nube: ${done}/${total} (${percent}%)`, {
        id: STORAGE_MIGRATION_TOAST_ID,
        duration: Infinity,
      });
    }).then((unlistenFn) => {
      if (active) {
        unlisten = unlistenFn;
      } else {
        unlistenFn();
      }
    });

    return () => {
      active = false;
      unlisten?.();
    };
  }, []);

  return null;
}
