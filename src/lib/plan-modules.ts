import type { CenterModules, ModuleKey } from "@/lib/modules";
import type { Json, Plan, PlanId } from "@/types/database";

const MODULE_KEYS: ModuleKey[] = [
  "appointments",
  "employees",
  "services",
  "packages",
  "public_booking",
  "reports",
];

const EMPTY_MODULES: CenterModules = {
  employees: false,
  appointments: false,
  services: false,
  packages: false,
  public_booking: false,
  reports: false,
};

/** Fallback si la columna allowed_modules aún no existe en BD. */
export const PLAN_ALLOWED_FALLBACK: Record<PlanId, CenterModules> = {
  free: { ...EMPTY_MODULES },
  starter: {
    employees: true,
    appointments: true,
    services: true,
    packages: true,
    public_booking: false,
    reports: false,
  },
  growth: {
    employees: true,
    appointments: true,
    services: true,
    packages: true,
    public_booking: true,
    reports: true,
  },
};

export function parseAllowedModules(
  value: Plan["allowed_modules"] | Json | null | undefined,
  planId?: string | null
): CenterModules {
  const raw =
    value && typeof value === "object" && !Array.isArray(value)
      ? (value as Record<string, unknown>)
      : null;

  if (!raw) {
    if (planId === "starter" || planId === "growth" || planId === "free") {
      return { ...PLAN_ALLOWED_FALLBACK[planId] };
    }
    return { ...EMPTY_MODULES };
  }

  return {
    employees: Boolean(raw.employees),
    appointments: Boolean(raw.appointments),
    services: Boolean(raw.services),
    packages: Boolean(raw.packages),
    public_booking: Boolean(raw.public_booking),
    reports: Boolean(raw.reports),
  };
}

export function isModuleAllowedByPlan(
  allowed: CenterModules | null | undefined,
  key: ModuleKey
) {
  return Boolean(allowed?.[key]);
}

/** Activo en el centro Y permitido por el plan. */
export function canUseModule(
  centerModules: CenterModules | null | undefined,
  planAllowed: CenterModules | null | undefined,
  key: ModuleKey
) {
  const raw =
    centerModules && typeof centerModules === "object"
      ? (centerModules as Record<string, unknown>)
      : {};
  const enabled = Boolean(raw[key]);
  return enabled && isModuleAllowedByPlan(planAllowed, key);
}

/** Apaga módulos que el plan no incluye. */
export function clampModulesToPlan(
  modules: CenterModules,
  allowed: CenterModules
): CenterModules {
  const next = { ...modules };
  for (const key of MODULE_KEYS) {
    if (!allowed[key]) next[key] = false;
  }
  return next;
}

export function planIdForModuleUnlock(key: ModuleKey): PlanId {
  if (key === "public_booking" || key === "reports") return "growth";
  return "starter";
}
