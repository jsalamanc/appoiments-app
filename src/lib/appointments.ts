import { invoke } from "@tauri-apps/api/core";

import type {
  AppointmentStatus,
  AppointmentWithDetails,
} from "@/types/database";

export function startOfDay(date: Date) {
  const d = new Date(date);
  d.setHours(0, 0, 0, 0);
  return d;
}

export function endOfDay(date: Date) {
  const d = startOfDay(date);
  d.setDate(d.getDate() + 1);
  return d;
}

export function startOfWeek(date: Date) {
  const d = startOfDay(date);
  const day = d.getDay();
  const diff = day === 0 ? -6 : 1 - day;
  d.setDate(d.getDate() + diff);
  return d;
}

export function addDays(date: Date, days: number) {
  const d = new Date(date);
  d.setDate(d.getDate() + days);
  return d;
}

export function endOfWeek(date: Date) {
  const start = startOfWeek(date);
  const end = addDays(start, 7);
  return end;
}

export function toLocalInputValue(date: Date) {
  const pad = (n: number) => String(n).padStart(2, "0");
  return `${date.getFullYear()}-${pad(date.getMonth() + 1)}-${pad(date.getDate())}T${pad(date.getHours())}:${pad(date.getMinutes())}`;
}

/** Día completo anterior a hoy (ignora la hora). */
export function isPastDay(date: Date, now = new Date()) {
  const day = new Date(date);
  day.setHours(0, 0, 0, 0);
  const today = new Date(now);
  today.setHours(0, 0, 0, 0);
  return day.getTime() < today.getTime();
}

/** Slot (día + hora) ya pasado. */
export function isPastSlot(day: Date, hour: number, now = new Date()) {
  const slot = new Date(day);
  slot.setHours(hour, 0, 0, 0);
  return slot.getTime() < now.getTime();
}

export function nextBookableSlot(now = new Date()) {
  const slot = new Date(now);
  slot.setMinutes(0, 0, 0);
  if (slot.getTime() <= now.getTime()) {
    slot.setHours(slot.getHours() + 1);
  }
  return slot;
}

export async function listAppointmentsInRange(input: {
  from: string;
  to: string;
  employeeId?: string | null;
}): Promise<AppointmentWithDetails[]> {
  return invoke<AppointmentWithDetails[]>("list_appointments_in_range", {
    from: input.from,
    to: input.to,
    employeeId: input.employeeId ?? null,
  });
}

export async function createAppointment(input: {
  employeeId: string;
  customerId: string;
  serviceId: string;
  startsAt: string;
  notes?: string;
  localWeekday: number;
  localStartMinutes: number;
}) {
  return invoke<AppointmentWithDetails>("create_appointment", {
    input: {
      employeeId: input.employeeId,
      customerId: input.customerId,
      serviceId: input.serviceId,
      startsAt: input.startsAt,
      notes: input.notes ?? null,
      localWeekday: input.localWeekday,
      localStartMinutes: input.localStartMinutes,
    },
  });
}

export async function updateAppointment(input: {
  appointmentId: string;
  employeeId: string;
  customerId: string;
  serviceId: string;
  startsAt: string;
  notes?: string;
  localWeekday: number;
  localStartMinutes: number;
}) {
  return invoke<AppointmentWithDetails>("update_appointment", {
    id: input.appointmentId,
    input: {
      employeeId: input.employeeId,
      customerId: input.customerId,
      serviceId: input.serviceId,
      startsAt: input.startsAt,
      notes: input.notes ?? null,
      localWeekday: input.localWeekday,
      localStartMinutes: input.localStartMinutes,
    },
  });
}

export async function setAppointmentStatus(
  appointmentId: string,
  status: AppointmentStatus
) {
  await invoke("set_appointment_status", { appointmentId, status });
}

export async function completeAppointmentWithSale(
  appointmentId: string,
  saleId: string
) {
  await invoke("complete_appointment_with_sale", {
    appointmentId,
    saleId,
  });
}

export function formatAppointmentStatus(status: AppointmentStatus) {
  if (status === "scheduled") return "Programada";
  if (status === "completed") return "Completada";
  if (status === "cancelled") return "Cancelada";
  return "No asistió";
}

export function getAppointmentServiceName(appointment: AppointmentWithDetails) {
  return (
    appointment.services?.name ??
    appointment.service_name_snapshot ??
    "Servicio"
  );
}

export function getAppointmentServiceDurationMinutes(
  appointment: AppointmentWithDetails
) {
  if (appointment.services?.duration_minutes) {
    return appointment.services.duration_minutes;
  }
  const ms =
    new Date(appointment.ends_at).getTime() -
    new Date(appointment.starts_at).getTime();
  return Math.max(Math.round(ms / 60000), 0);
}

export function getAppointmentServicePrice(
  appointment: AppointmentWithDetails
) {
  if (appointment.services?.price == null) return null;
  return Number(appointment.services.price);
}

/** Forma mínima de servicio que necesita el selector del formulario de citas. */
export type AppointmentServiceOption = {
  id: string;
  name: string;
  price: number;
  duration_minutes: number;
  status: string;
};

/**
 * Lista de servicios publicados para el selector del formulario de citas.
 * El dominio `services` (propiedad de otro ingeniero) aún no expone su
 * `list_services` paginado; este wrapper llama a un comando de solo lectura
 * definido en el dominio `appointments`
 * (`list_published_services_for_appointments`, ver
 * src-tauri/src/commands/appointments.rs) para no bloquear el calendario en
 * ese trabajo paralelo.
 */
export async function listPublishedServicesForAppointments() {
  return invoke<AppointmentServiceOption[]>(
    "list_published_services_for_appointments"
  );
}
