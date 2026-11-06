import { relaunch } from "@tauri-apps/plugin-process";
import { check, type Update } from "@tauri-apps/plugin-updater";
import { toast } from "sonner";

// Id fijo para el toast de actualización: como se vuelve a chequear cada 15
// minutos (ver `UpdateChecker`), reusar el mismo id hace que cada chequeo
// reemplace el toast anterior en vez de apilar uno nuevo arriba del otro.
const UPDATE_TOAST_ID = "app-update-available";

export async function checkForUpdates(): Promise<Update | null> {
  return check();
}

export async function installUpdate(update: Update): Promise<void> {
  const toastId = toast.loading("Descargando actualización...", {
    id: UPDATE_TOAST_ID,
    position: "bottom-right",
  });
  try {
    await update.downloadAndInstall();
    toast.success("Actualización instalada. Reiniciando...", {
      id: toastId,
      position: "bottom-right",
    });
    await relaunch();
  } catch (err) {
    toast.error(
      err instanceof Error
        ? err.message
        : "No se pudo instalar la actualización",
      { id: toastId, position: "bottom-right" }
    );
  }
}

export function notifyUpdateAvailable(update: Update) {
  toast(`Nueva versión disponible: ${update.version}`, {
    id: UPDATE_TOAST_ID,
    duration: Infinity,
    position: "bottom-right",
    action: {
      label: "Descargar",
      onClick: () => void installUpdate(update),
    },
  });
}
