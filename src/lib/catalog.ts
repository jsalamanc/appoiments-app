import { createClient } from "@/lib/supabase/client";

const CATALOG_PUBLIC_MARKER = "/object/public/catalog/";

export async function uploadCatalogImage(
  centerId: string,
  file: File,
  folder: "products" | "services" | "packages"
) {
  const supabase = createClient();
  const ext = file.name.split(".").pop()?.toLowerCase() || "jpg";
  const path = `${centerId}/${folder}/${crypto.randomUUID()}.${ext}`;

  const { error } = await supabase.storage.from("catalog").upload(path, file, {
    cacheControl: "3600",
    upsert: false,
  });

  if (error) throw error;

  const {
    data: { publicUrl },
  } = supabase.storage.from("catalog").getPublicUrl(path);

  return publicUrl;
}

/** Derives storage object path from a catalog public URL, or null if invalid. */
export function catalogImagePathFromUrl(photoUrl: string): string | null {
  try {
    const pathname = new URL(photoUrl).pathname;
    const markerIndex = pathname.indexOf(CATALOG_PUBLIC_MARKER);
    if (markerIndex === -1) return null;
    const path = decodeURIComponent(
      pathname.slice(markerIndex + CATALOG_PUBLIC_MARKER.length)
    );
    return path || null;
  } catch {
    return null;
  }
}

/**
 * Removes a catalog image from Storage. Never throws — callers should not
 * block DB success on orphan cleanup failures.
 */
export async function deleteCatalogImage(
  photoUrl: string | null | undefined
): Promise<void> {
  if (!photoUrl) return;

  try {
    const path = catalogImagePathFromUrl(photoUrl);
    if (!path) return;

    const center = await getCurrentCenter();
    if (!center) return;
    if (!path.startsWith(`${center.id}/`)) return;

    const supabase = createClient();
    const { error } = await supabase.storage.from("catalog").remove([path]);
    if (error) {
      console.warn("No se pudo eliminar la foto del storage:", error.message);
    }
  } catch (err) {
    console.warn("No se pudo eliminar la foto del storage:", err);
  }
}

export async function getCurrentCenter() {
  const supabase = createClient();
  const {
    data: { user },
  } = await supabase.auth.getUser();

  if (!user) return null;

  const { data, error } = await supabase
    .from("centers")
    .select("*")
    .eq("owner_id", user.id)
    .maybeSingle();

  if (error) throw error;
  return data;
}
