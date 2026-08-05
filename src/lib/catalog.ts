import { createClient } from "@/lib/supabase/client";
import type { Center, MemberRole } from "@/types/database";

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

function isMissingMembersTable(error: { code?: string; message?: string } | null) {
  if (!error) return false;
  return (
    error.code === "PGRST205" ||
    error.message?.includes("center_members") === true
  );
}

export async function getCurrentCenter() {
  const supabase = createClient();
  const {
    data: { user },
  } = await supabase.auth.getUser();

  if (!user) return null;

  const { data, error } = await supabase
    .from("center_members")
    .select("centers (*)")
    .eq("user_id", user.id)
    .maybeSingle();

  if (error) {
    if (isMissingMembersTable(error)) {
      const { data: center, error: ownerError } = await supabase
        .from("centers")
        .select("*")
        .eq("owner_id", user.id)
        .maybeSingle();
      if (ownerError) throw new Error(ownerError.message);
      return center;
    }
    throw new Error(error.message);
  }

  if (data?.centers) return data.centers as unknown as Center;

  const { data: center, error: ownerError } = await supabase
    .from("centers")
    .select("*")
    .eq("owner_id", user.id)
    .maybeSingle();
  if (ownerError) throw new Error(ownerError.message);
  return center;
}

export async function getCurrentMemberRole(): Promise<MemberRole | null> {
  const supabase = createClient();
  const { data, error } = await supabase.rpc("current_member_role");
  if (error) {
    if (
      error.code === "PGRST202" ||
      error.message?.includes("current_member_role")
    ) {
      const center = await getCurrentCenter();
      return center ? "owner" : null;
    }
    throw new Error(error.message);
  }
  if (data === "owner" || data === "reception") return data;
  return null;
}
