import { createClient } from "@/lib/supabase/server";
import { getServerCenter } from "@/lib/supabase/center";
import type { Plan } from "@/types/database";

export async function listPlansServer(): Promise<Plan[]> {
  const supabase = await createClient();
  const { data, error } = await supabase
    .from("plans")
    .select("*")
    .eq("active", true)
    .order("sort_order", { ascending: true });

  if (error) throw new Error(error.message);
  return (data ?? []) as Plan[];
}

/** null = sin límite */
export async function getServerProductLimit(): Promise<number | null> {
  const center = await getServerCenter();
  if (!center?.plan_id) return 10;

  const supabase = await createClient();
  const { data, error } = await supabase
    .from("plans")
    .select("id, max_products")
    .eq("id", center.plan_id)
    .maybeSingle();

  if (error || !data) {
    return center.plan_id === "free" ? 10 : null;
  }

  return data.max_products ?? null;
}
