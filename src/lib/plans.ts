import { getCurrentCenter } from "@/lib/catalog";
import {
  clampModulesToPlan,
  parseAllowedModules,
  PLAN_ALLOWED_FALLBACK,
} from "@/lib/plan-modules";
import {
  DEFAULT_MODULES,
  normalizeModules,
  parseCenterModules,
  type CenterModules,
} from "@/lib/modules";
import { createClient } from "@/lib/supabase/client";
import type { Center, Plan, PlanId } from "@/types/database";

export function planPricePesos(plan: Pick<Plan, "price_cents">) {
  return plan.price_cents / 100;
}

export function parsePlanBenefits(value: Plan["benefits"]): string[] {
  if (!Array.isArray(value)) return [];
  return value.filter((item): item is string => typeof item === "string");
}

export async function getCurrentPlanAllowed(): Promise<CenterModules> {
  const center = await getCurrentCenter();
  if (!center?.plan_id) return { ...DEFAULT_MODULES };

  const supabase = createClient();
  const { data, error } = await supabase
    .from("plans")
    .select("id, allowed_modules")
    .eq("id", center.plan_id)
    .maybeSingle();

  if (error || !data) {
    if (center.plan_id === "free" || center.plan_id === "starter" || center.plan_id === "growth") {
      return { ...PLAN_ALLOWED_FALLBACK[center.plan_id] };
    }
    return { ...DEFAULT_MODULES };
  }

  return parseAllowedModules(data.allowed_modules, data.id);
}

export async function listPlans(): Promise<Plan[]> {
  const supabase = createClient();
  const { data, error } = await supabase
    .from("plans")
    .select("*")
    .eq("active", true)
    .order("sort_order", { ascending: true });

  if (error) throw new Error(error.message);
  return (data ?? []) as Plan[];
}

/** null = sin límite. Fallback Free = 10 si aún no hay columna. */
export async function getCurrentProductLimit(): Promise<number | null> {
  const center = await getCurrentCenter();
  if (!center?.plan_id) return 10;

  const supabase = createClient();
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

export async function assertCanCreateProduct() {
  const max = await getCurrentProductLimit();
  if (max === null) return;

  const supabase = createClient();
  const { count, error } = await supabase
    .from("products")
    .select("id", { count: "exact", head: true });

  if (error) throw new Error(error.message);

  if ((count ?? 0) >= max) {
    throw new Error(
      `Tu plan Free permite máximo ${max} productos. Cambia a Starter o Growth para agregar más.`
    );
  }
}

export async function updateCenterPlan(planId: PlanId): Promise<Center> {
  const supabase = createClient();
  const center = await getCurrentCenter();
  if (!center) throw new Error("No se encontró el centro");

  const { data: plan, error: planError } = await supabase
    .from("plans")
    .select("*")
    .eq("id", planId)
    .maybeSingle();

  if (planError) throw new Error(planError.message);
  if (!plan) throw new Error("Plan no encontrado");

  const allowed = parseAllowedModules(plan.allowed_modules, plan.id);
  const clamped = clampModulesToPlan(
    normalizeModules(parseCenterModules(center.modules)),
    allowed
  );

  const { data, error } = await supabase
    .from("centers")
    .update({
      plan_id: planId,
      modules: clamped as Center["modules"],
    })
    .eq("id", center.id)
    .select("*")
    .single();

  if (error) throw new Error(error.message);
  return data as Center;
}
