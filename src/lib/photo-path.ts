import { convertFileSrc } from "@tauri-apps/api/core";
import { appDataDir, join } from "@tauri-apps/api/path";

let cachedAppDataDir: Promise<string> | null = null;

function getAppDataDir() {
  if (!cachedAppDataDir) cachedAppDataDir = appDataDir();
  return cachedAppDataDir;
}

/**
 * Convierte un `photo_path` relativo (guardado en `products`/`services`/
 * `packages`, ej. `images/products/<uuid>.jpg`) en un src utilizable en un
 * `<img>`, resolviendo el directorio de datos de la app con
 * `@tauri-apps/api/path` y exponiéndolo vía `convertFileSrc` de
 * `@tauri-apps/api/core`. Devuelve null si no hay foto.
 */
export async function resolvePhotoSrc(
  photoPath: string | null | undefined
): Promise<string | null> {
  if (!photoPath) return null;
  const dataDir = await getAppDataDir();
  const absolute = await join(dataDir, photoPath);
  return convertFileSrc(absolute);
}
