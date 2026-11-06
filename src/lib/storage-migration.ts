/** Compartido entre `storage-settings.tsx` (quien dispara la migración) y
 * `storage-migration-listener.tsx` (montado en el layout raíz, quien la
 * muestra) para que ambos usen el mismo nombre de evento de Tauri y el mismo
 * `id` de toast de `sonner` — así las actualizaciones de progreso reemplazan
 * en el mismo toast en vez de apilar uno nuevo por evento. */
export const STORAGE_MIGRATION_EVENT = "storage-migration-progress";
export const STORAGE_MIGRATION_TOAST_ID = "storage-migration-progress";

export type StorageMigrationProgress = {
  done: number;
  total: number;
};
