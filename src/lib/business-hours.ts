import { invoke } from "@tauri-apps/api/core";

import type {
  BusinessHours,
  BusinessHoursInput,
  EmployeeAvailability,
  EmployeeAvailabilityInput,
} from "@/types/database";

/** Nombres de día para `weekday` (domingo=0 .. sábado=6, `Date.getDay()`). */
export const WEEKDAY_LABELS = [
  "Domingo",
  "Lunes",
  "Martes",
  "Miércoles",
  "Jueves",
  "Viernes",
  "Sábado",
];

export async function getBusinessHours(): Promise<BusinessHours[]> {
  return invoke<BusinessHours[]>("get_business_hours");
}

export async function updateBusinessHours(
  hours: BusinessHoursInput[]
): Promise<BusinessHours[]> {
  return invoke<BusinessHours[]>("update_business_hours", { hours });
}

export async function getEmployeeAvailability(
  employeeId: string
): Promise<EmployeeAvailability[]> {
  return invoke<EmployeeAvailability[]>("get_employee_availability", {
    employeeId,
  });
}

export async function updateEmployeeAvailability(
  employeeId: string,
  overrides: EmployeeAvailabilityInput[]
): Promise<EmployeeAvailability[]> {
  return invoke<EmployeeAvailability[]>("update_employee_availability", {
    employeeId,
    overrides,
  });
}
