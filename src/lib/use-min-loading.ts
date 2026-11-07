import { useEffect, useState } from "react";

/** Tiempo mínimo (ms) que se muestra un skeleton de carga. Con datos locales
 * (SQLite) las consultas terminan en pocos milisegundos y el skeleton
 * parpadearía sin que el usuario lo note; este mínimo hace que se perciba la
 * carga. Ajusta o pon 0 para desactivarlo. */
export const SKELETON_MIN_MS = 350;

/** `false` hasta que pasa `ms` desde que el componente se monta. Úsalo junto
 * con el estado de carga real: `loading || !minElapsed` → mostrar skeleton. */
export function useMinLoadingElapsed(ms: number = SKELETON_MIN_MS) {
  const [elapsed, setElapsed] = useState(ms <= 0);

  useEffect(() => {
    if (ms <= 0) return;
    const timer = setTimeout(() => setElapsed(true), ms);
    return () => clearTimeout(timer);
  }, [ms]);

  return elapsed;
}
