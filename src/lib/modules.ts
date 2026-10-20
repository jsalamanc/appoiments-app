import { invoke } from "@tauri-apps/api/core";

export type ModuleKey =
  | "employees"
  | "appointments"
  | "services"
  | "packages"
  | "reports";

export type CenterModules = Record<ModuleKey, boolean>;

export const MODULE_KEYS: ModuleKey[] = [
  "appointments",
  "employees",
  "services",
  "packages",
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
    description: "Profesionales y calendarios. Se puede usar sin Agenda.",
  },
  services: {
    label: "Servicios",
    description: "Tratamientos con duración y precio en el catálogo.",
  },
  packages: {
    label: "Paquetes",
    description: "Combos de productos y servicios a precio especial.",
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

export function isModuleEnabled(
  modules: CenterModules | null | undefined,
  key: ModuleKey
) {
  return Boolean(modules?.[key]);
}

/**
 * Reglas de dependencias (sin chequeo de plan: todos los módulos son
 * libres en la edición local):
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
    next.employees = true;
  }
  if (key === "employees" && !enabled) {
    next.appointments = false;
  }
  return normalizeModules(next);
}

export async function getEnabledModules(): Promise<CenterModules> {
  return invoke<CenterModules>("get_enabled_modules");
}

export async function updateCenterModules(
  modules: CenterModules
): Promise<CenterModules> {
  return invoke<CenterModules>("update_enabled_modules", { modules });
}

export function moduleForPath(pathname: string): ModuleKey | null {
  if (MODULE_NAV[pathname] !== undefined) return MODULE_NAV[pathname];
  const match = Object.keys(MODULE_NAV)
    .filter((href) => href !== "/admin" && pathname.startsWith(href))
    .sort((a, b) => b.length - a.length)[0];
  return match ? MODULE_NAV[match] ?? null : null;
}
