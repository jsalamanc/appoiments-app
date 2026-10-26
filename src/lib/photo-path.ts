import { convertFileSrc, invoke } from "@tauri-apps/api/core";
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
 *
 * `resolve_photo_cache` garantiza que el archivo exista en disco antes de
 * construir el src: en modo Local es un no-op (el archivo ya está ahí); en
 * modo "Base de datos propia" descarga del bucket configurado si hace falta
 * (ej. una segunda instalación apuntando al mismo Postgres/bucket). Ver
 * `src-tauri/src/commands/bucket.rs`.
 */
export async function resolvePhotoSrc(
  photoPath: string | null | undefined
): Promise<string | null> {
  if (!photoPath) return null;
  const cachedPath = await invoke<string>("resolve_photo_cache", { photoPath });
  const dataDir = await getAppDataDir();
  const absolute = await join(dataDir, cachedPath);
  return convertFileSrc(absolute);
}
