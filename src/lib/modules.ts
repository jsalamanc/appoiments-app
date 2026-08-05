import { getCurrentCenter } from "@/lib/catalog";
import {
  clampModulesToPlan,
  isModuleAllowedByPlan,
  parseAllowedModules,
} from "@/lib/plan-modules";
import { createClient } from "@/lib/supabase/client";
import type { Center, Json } from "@/types/database";

export type ModuleKey =
  | "employees"
  | "appointments"
  | "services"
  | "packages"
  | "public_booking"
  | "reports";

export type CenterModules = Record<ModuleKey, boolean>;

export const MODULE_KEYS: ModuleKey[] = [
  "appointments",
  "employees",
  "services",
  "packages",
  "public_booking",
  "reports",
];

export const MODULE_META: Record<
  ModuleKey,
  { label: string; description: string }
> = {
  appointments: {
    label: "Agenda",
    description:
      "Calendario de citas. Al activarla también habilita Empleados.",
  },
  employees: {
    label: "Empleados",
    description:
      "Profesionales y calendarios. Se puede usar sin Agenda.",
  },
  services: {
    label: "Servicios",
    description: "Tratamientos con duración y precio en el catálogo.",
  },
  packages: {
    label: "Paquetes",
    description: "Combos de productos y servicios a precio especial.",
  },
  public_booking: {
    label: "Portal de reservas",
    description:
      "Página pública para que los clientes agenden solos. Activa Agenda y Servicios.",
  },
  reports: {
    label: "Reportes",
    description:
      "Rendimiento por empleado, servicios/ticket, no-shows y exportes.",
  },
};

export const DEFAULT_MODULES: CenterModules = {
  employees: false,
  appointments: false,
  services: false,
  packages: false,
  public_booking: false,
  reports: false,
};

/** href de navegación → módulo requerido (null = siempre visible). */
export const MODULE_NAV: Record<string, ModuleKey | null> = {
  "/admin": null,
  "/admin/sales": null,
  "/admin/sales/history": null,
  "/admin/sales/cash": null,
  "/admin/products": null,
  "/admin/products/inventory": null,
  "/admin/customers": null,
  "/admin/settings": null,
  "/admin/appointments": "appointments",
  "/admin/appointments/today": "appointments",
  "/admin/employees": "employees",
  "/admin/services": "services",
  "/admin/packages": "packages",
  "/admin/reports/employees": "reports",
  "/admin/reports/services": "reports",
  "/admin/reports/no-shows": "reports",
};

export function parseCenterModules(
  value: Center["modules"] | Json | null | undefined
): CenterModules {
  const raw =
    value && typeof value === "object" && !Array.isArray(value)
      ? (value as Record<string, unknown>)
      : {};

  return {
    employees: Boolean(raw.employees),
    appointments: Boolean(raw.appointments),
    services: Boolean(raw.services),
    packages: Boolean(raw.packages),
    public_booking: Boolean(raw.public_booking),
    reports: Boolean(raw.reports),
  };
}

export function isModuleEnabled(
  modules: CenterModules | Center["modules"] | null | undefined,
  key: ModuleKey
) {
  return parseCenterModules(modules as Json)[key] === true;
}

/**
 * Reglas de dependencias:
 * - appointments ON ⇒ employees ON
 * - employees OFF ⇒ appointments OFF (y public_booking OFF)
 * - public_booking ON ⇒ appointments + services (+ employees)
 * - appointments OFF o services OFF ⇒ public_booking OFF
 */
export function normalizeModules(input: CenterModules): CenterModules {
  const next = { ...input };
  if (next.appointments) next.employees = true;
  if (!next.employees) next.appointments = false;
  if (next.public_booking) {
    next.appointments = true;
    next.services = true;
    next.employees = true;
  }
  if (!next.appointments || !next.services) {
    next.public_booking = false;
  }
  return next;
}

/** Toggle de un módulo respetando dependencias. */
export function toggleModule(
  current: CenterModules,
  key: ModuleKey,
  enabled: boolean
): CenterModules {
  const next = { ...current, [key]: enabled };
  if (key === "appointments" && enabled) {
    next.appointments = true;
    next.employees = true;
  }
  if (key === "employees" && !enabled) {
    next.employees = false;
    next.appointments = false;
    next.public_booking = false;
  }
  if (key === "appointments" && !enabled) {
    next.appointments = false;
    next.public_booking = false;
  }
  if (key === "services" && !enabled) {
    next.services = false;
    next.public_booking = false;
  }
  if (key === "public_booking" && enabled) {
    next.public_booking = true;
    next.appointments = true;
    next.services = true;
    next.employees = true;
  }
  return normalizeModules(next);
}

export async function updateCenterModules(modules: CenterModules) {
  const supabase = createClient();
  const center = await getCurrentCenter();
  if (!center) throw new Error("No se encontró el centro");

  const { data: plan, error: planError } = await supabase
    .from("plans")
    .select("id, allowed_modules")
    .eq("id", center.plan_id)
    .maybeSingle();

  if (planError) throw new Error(planError.message);

  const allowed = parseAllowedModules(
    plan?.allowed_modules,
    plan?.id ?? center.plan_id
  );

  for (const key of MODULE_KEYS) {
    if (modules[key] && !isModuleAllowedByPlan(allowed, key)) {
      throw new Error(
        `${MODULE_META[key].label} no está incluido en tu plan. Cambia de plan en Ajustes.`
      );
    }
  }

  const normalized = clampModulesToPlan(normalizeModules(modules), allowed);
  const { data, error } = await supabase
    .from("centers")
    .update({ modules: normalized as Center["modules"] })
    .eq("id", center.id)
    .select("*")
    .single();

  if (error) throw error;
  return data as Center;
}

export function moduleForPath(pathname: string): ModuleKey | null {
  if (MODULE_NAV[pathname] !== undefined) return MODULE_NAV[pathname];
  const match = Object.keys(MODULE_NAV)
    .filter((href) => href !== "/admin" && pathname.startsWith(href))
    .sort((a, b) => b.length - a.length)[0];
  return match ? MODULE_NAV[match] ?? null : null;
}
