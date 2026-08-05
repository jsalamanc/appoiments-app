import { createClient } from "@/lib/supabase/server";
import type { Center } from "@/types/database";

export async function getServerCenter(): Promise<Center | null> {
  const supabase = await createClient();
  const {
    data: { user },
  } = await supabase.auth.getUser();

  if (!user) return null;

  const { data } = await supabase
    .from("centers")
    .select("*")
    .eq("owner_id", user.id)
    .maybeSingle();

  return data;
}
