"use client";

import { useEffect, useRef } from "react";

import { checkForUpdates, notifyUpdateAvailable } from "@/lib/updates";

/** Revisa en segundo plano, una vez por sesión, si hay una versión nueva. */
export function UpdateChecker() {
  const checked = useRef(false);

  useEffect(() => {
    if (checked.current) return;
    checked.current = true;

    checkForUpdates()
      .then((update) => {
        if (update) notifyUpdateAvailable(update);
      })
      .catch((err) => {
        console.warn("No se pudo buscar actualizaciones:", err);
      });
  }, []);

  return null;
}
