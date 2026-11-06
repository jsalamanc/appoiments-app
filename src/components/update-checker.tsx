"use client";

import { useEffect, useRef } from "react";

import { checkForUpdates, notifyUpdateAvailable } from "@/lib/updates";

const RECHECK_INTERVAL_MS = 15 * 60 * 1000;

/**
 * Revisa si hay una versión nueva al abrir la app (haya o no una sesión
 * iniciada -- por eso vive en el layout raíz y no en el del panel de admin,
 * donde solo se enteraba quien ya había iniciado sesión) y después cada 15
 * minutos mientras siga abierta, para no depender de que alguien se acuerde
 * de ir a buscarla a mano.
 */
export function UpdateChecker() {
  const checkedOnce = useRef(false);

  useEffect(() => {
    let cancelled = false;

    const runCheck = () => {
      checkForUpdates()
        .then((update) => {
          if (!cancelled && update) notifyUpdateAvailable(update);
        })
        .catch((err) => {
          console.warn("No se pudo buscar actualizaciones:", err);
        });
    };

    if (!checkedOnce.current) {
      checkedOnce.current = true;
      runCheck();
    }

    const interval = setInterval(runCheck, RECHECK_INTERVAL_MS);
    return () => {
      cancelled = true;
      clearInterval(interval);
    };
  }, []);

  return null;
}
