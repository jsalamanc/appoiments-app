import { getCurrentCenter } from "@/lib/catalog";
import { createClient } from "@/lib/supabase/client";
import type { Employee } from "@/types/database";

export async function listEmployees(options?: { activeOnly?: boolean }) {
  const supabase = createClient();
  let query = supabase.from("employees").select("*").order("name");
  if (options?.activeOnly) {
    query = query.eq("active", true);
  }
  const { data, error } = await query;
  if (error) throw error;
  return (data ?? []) as Employee[];
}

export async function createEmployee(input: {
  name: string;
  color: string;
  active: boolean;
}) {
  const supabase = createClient();
  const center = await getCurrentCenter();
  if (!center) throw new Error("No se encontró el centro");

  const { data, error } = await supabase
    .from("employees")
    .insert({
      center_id: center.id,
      name: input.name.trim(),
      color: input.color,
      active: input.active,
    })
    .select("*")
    .single();

  if (error) throw error;
  return data as Employee;
}

export async function updateEmployee(
  id: string,
  input: { name: string; color: string; active: boolean }
) {
  const supabase = createClient();
  const { data, error } = await supabase
    .from("employees")
    .update({
      name: input.name.trim(),
      color: input.color,
      active: input.active,
    })
    .eq("id", id)
    .select("*")
    .single();

  if (error) throw error;
  return data as Employee;
}

export async function deleteEmployee(id: string) {
  const supabase = createClient();
  const { error } = await supabase.from("employees").delete().eq("id", id);
  if (error) {
    if (error.code === "23503") {
      throw new Error(
        "No se puede eliminar: el empleado tiene citas asociadas. Desactívalo en su lugar."
      );
    }
    throw error;
  }
}
