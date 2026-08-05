import { getCurrentCenter } from "@/lib/catalog";
import { createClient } from "@/lib/supabase/client";
import type { Center, Json } from "@/types/database";

export type ModuleKey =
  | "employees"
  | "appointments"
  | "services"
  | "packages";

export type CenterModules = Record<ModuleKey, boolean>;

export const MODULE_KEYS: ModuleKey[] = [
  "appointments",
  "employees",
  "services",
  "packages",
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
};

export const DEFAULT_MODULES: CenterModules = {
  employees: false,
  appointments: false,
  services: false,
  packages: false,
};

/** href de navegación → módulo requerido (null = siempre visible). */
export const MODULE_NAV: Record<string, ModuleKey | null> = {
  "/admin": null,
  "/admin/sales": null,
  "/admin/sales/history": null,
  "/admin/products": null,
  "/admin/settings": null,
  "/admin/appointments": "appointments",
  "/admin/employees": "employees",
  "/admin/services": "services",
  "/admin/packages": "packages",
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
  };
}

export function isModuleEnabled(
  modules: CenterModules | Center["modules"] | null | undefined,
  key: ModuleKey
) {
  return parseCenterModules(modules as Json)[key] === true;
}

/**
 * Aplica reglas Agenda ↔ Empleados:
 * - appointments ON ⇒ employees ON
 * - employees OFF ⇒ appointments OFF
 */
export function normalizeModules(input: CenterModules): CenterModules {
  const next = { ...input };
  if (next.appointments) next.employees = true;
  if (!next.employees) next.appointments = false;
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
  }
  if (key === "appointments" && !enabled) {
    next.appointments = false;
    // employees permanece como estaba (puede seguir true)
  }
  return normalizeModules(next);
}

export async function updateCenterModules(modules: CenterModules) {
  const supabase = createClient();
  const center = await getCurrentCenter();
  if (!center) throw new Error("No se encontró el centro");

  const normalized = normalizeModules(modules);
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
