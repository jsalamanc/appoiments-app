import { parseAllowedModules } from "@/lib/plan-modules";
import type { CenterModules } from "@/lib/modules";
import { createClient } from "@/lib/supabase/server";
import type { Center, MemberRole, Plan } from "@/types/database";

export type ServerCenterContext = {
  center: Center;
  role: MemberRole;
  plan: Plan | null;
  planAllowed: CenterModules;
};

function isMissingMembersTable(error: { code?: string; message?: string } | null) {
  if (!error) return false;
  return (
    error.code === "PGRST205" ||
    error.message?.includes("center_members") === true
  );
}

async function loadPlan(
  supabase: Awaited<ReturnType<typeof createClient>>,
  planId: string | null | undefined
): Promise<Plan | null> {
  if (!planId) return null;
  const { data, error } = await supabase
    .from("plans")
    .select("*")
    .eq("id", planId)
    .maybeSingle();
  if (error) {
    // Columna/tabla aún no migrada
    if (
      error.code === "PGRST205" ||
      error.message?.includes("plans") ||
      error.message?.includes("allowed_modules")
    ) {
      return null;
    }
    throw new Error(error.message);
  }
  return data as Plan | null;
}

function toContext(
  center: Center,
  role: MemberRole,
  plan: Plan | null
): ServerCenterContext {
  return {
    center,
    role,
    plan,
    planAllowed: parseAllowedModules(plan?.allowed_modules, plan?.id ?? center.plan_id),
  };
}

async function getCenterAsOwner(
  supabase: Awaited<ReturnType<typeof createClient>>,
  userId: string
): Promise<ServerCenterContext | null> {
  const { data, error } = await supabase
    .from("centers")
    .select("*")
    .eq("owner_id", userId)
    .maybeSingle();

  if (error) throw new Error(error.message);
  if (!data) return null;

  const plan = await loadPlan(supabase, data.plan_id);
  return toContext(data, "owner", plan);
}

export async function getServerCenterContext(): Promise<ServerCenterContext | null> {
  const supabase = await createClient();
  const {
    data: { user },
  } = await supabase.auth.getUser();

  if (!user) return null;

  const { data, error } = await supabase
    .from("center_members")
    .select("role, centers (*)")
    .eq("user_id", user.id)
    .maybeSingle();

  if (error) {
    if (isMissingMembersTable(error)) {
      return getCenterAsOwner(supabase, user.id);
    }
    throw new Error(error.message);
  }

  if (!data?.centers) {
    return getCenterAsOwner(supabase, user.id);
  }

  const center = data.centers as unknown as Center;
  const plan = await loadPlan(supabase, center.plan_id);
  return toContext(center, data.role as MemberRole, plan);
}

export async function getServerCenter(): Promise<Center | null> {
  const ctx = await getServerCenterContext();
  return ctx?.center ?? null;
}
