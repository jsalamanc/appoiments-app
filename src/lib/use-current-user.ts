"use client";

import { invoke } from "@tauri-apps/api/core";
import { useEffect, useState } from "react";

export type CurrentUser = { id: string; role: string } | null;

/**
 * Usuario de la sesión actual, para decisiones de UI que dependen del rol
 * (ej. mostrar "Anular venta" solo al dueño) sin tener que envolver toda la
 * página en `OwnerGate`. El backend revalida el rol de todas formas en cada
 * comando — esto es solo para no mostrarle a staff un botón que el backend
 * va a rechazar.
 */
export function useCurrentUser() {
  const [user, setUser] = useState<CurrentUser>(null);
  const [loaded, setLoaded] = useState(false);

  useEffect(() => {
    let cancelled = false;
    invoke<CurrentUser>("auth_current_user").then((u) => {
      if (!cancelled) {
        setUser(u);
        setLoaded(true);
      }
    });
    return () => {
      cancelled = true;
    };
  }, []);

  return { user, loaded };
}
