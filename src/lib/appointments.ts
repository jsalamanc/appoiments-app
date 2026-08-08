import { createClient } from "@/lib/supabase/client";
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
}) {
  const supabase = createClient();
  let query = supabase
    .from("appointments")
    .select(
      `
      *,
      employees ( id, name, color, active ),
      customers ( id, name, email, phone, document ),
      services ( id, name, price, duration_minutes, status )
    `
    )
    .gte("starts_at", input.from)
    .lt("starts_at", input.to)
    .order("starts_at", { ascending: true });

  if (input.employeeId) {
    query = query.eq("employee_id", input.employeeId);
  }

  const { data, error } = await query;
  if (error) throw error;
  return (data ?? []) as unknown as AppointmentWithDetails[];
}

export async function createAppointment(input: {
  employeeId: string;
  customerId: string;
  serviceId: string;
  startsAt: string;
  notes?: string;
}) {
  const supabase = createClient();
  const { data, error } = await supabase.rpc("create_appointment", {
    p_employee_id: input.employeeId,
    p_customer_id: input.customerId,
    p_service_id: input.serviceId,
    p_starts_at: input.startsAt,
    p_notes: input.notes ?? null,
  });
  if (error) throw new Error(error.message);
  return data as string;
}

export async function updateAppointment(input: {
  appointmentId: string;
  employeeId: string;
  customerId: string;
  serviceId: string;
  startsAt: string;
  notes?: string;
}) {
  const supabase = createClient();
  const { data, error } = await supabase.rpc("update_appointment", {
    p_appointment_id: input.appointmentId,
    p_employee_id: input.employeeId,
    p_customer_id: input.customerId,
    p_service_id: input.serviceId,
    p_starts_at: input.startsAt,
    p_notes: input.notes ?? null,
  });
  if (error) throw new Error(error.message);
  return data as string;
}

export async function setAppointmentStatus(
  appointmentId: string,
  status: AppointmentStatus
) {
  const supabase = createClient();
  const { error } = await supabase
    .from("appointments")
    .update({ status })
    .eq("id", appointmentId);
  if (error) throw error;
}

export async function completeAppointmentWithSale(
  appointmentId: string,
  saleId: string
) {
  const supabase = createClient();

  const { error: saleError } = await supabase
    .from("sales")
    .update({ appointment_id: appointmentId })
    .eq("id", saleId);
  if (saleError) throw saleError;

  const { error: apptError } = await supabase
    .from("appointments")
    .update({ status: "completed", sale_id: saleId })
    .eq("id", appointmentId);
  if (apptError) throw apptError;
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
