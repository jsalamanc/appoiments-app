import { relaunch } from "@tauri-apps/plugin-process";
import { check, type Update } from "@tauri-apps/plugin-updater";
import { toast } from "sonner";

export async function checkForUpdates(): Promise<Update | null> {
  return check();
}

export async function installUpdate(update: Update): Promise<void> {
  const toastId = toast.loading("Descargando actualización...");
  try {
    await update.downloadAndInstall();
    toast.success("Actualización instalada. Reiniciando...", { id: toastId });
    await relaunch();
  } catch (err) {
    toast.error(
      err instanceof Error
        ? err.message
        : "No se pudo instalar la actualización",
      { id: toastId }
    );
  }
}

export function notifyUpdateAvailable(update: Update) {
  toast(`Hay una nueva versión disponible: ${update.version}`, {
    duration: Infinity,
    action: {
      label: "Actualizar ahora",
      onClick: () => void installUpdate(update),
    },
  });
}
