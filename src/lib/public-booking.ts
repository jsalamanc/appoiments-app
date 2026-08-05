import { createClient } from "@/lib/supabase/client";
import { addDays, endOfDay, isPastSlot, startOfDay } from "@/lib/appointments";
import { customerSchema } from "@/lib/validations";

export type PublicService = {
  id: string;
  name: string;
  description: string | null;
  price: number;
  duration_minutes: number;
  photo_url: string | null;
};

export type PublicEmployee = {
  id: string;
  name: string;
  color: string;
};

export type PublicBookingCatalog = {
  available: boolean;
  reason?: string;
  center: { id?: string; name: string; slug: string };
  services: PublicService[];
  employees: PublicEmployee[];
};

export type BusyRange = { starts_at: string; ends_at: string };

const BOOKING_HOURS = [9, 10, 11, 12, 13, 14, 15, 16, 17];

export async function fetchPublicBookingCatalog(slug: string) {
  const supabase = createClient();
  const { data, error } = await supabase.rpc("get_public_booking_catalog", {
    p_slug: slug,
  });
  if (error) throw error;
  if (!data) return null;
  return data as unknown as PublicBookingCatalog;
}

export async function fetchPublicBusyRanges(
  slug: string,
  employeeId: string,
  day: Date
) {
  const supabase = createClient();
  const from = startOfDay(day).toISOString();
  const to = endOfDay(day).toISOString();
  const { data, error } = await supabase.rpc("get_public_busy_ranges", {
    p_slug: slug,
    p_employee_id: employeeId,
    p_from: from,
    p_to: to,
  });
  if (error) throw error;
  return (data ?? []) as unknown as BusyRange[];
}

function rangesOverlap(
  startA: number,
  endA: number,
  startB: number,
  endB: number
) {
  return startA < endB && endA > startB;
}

export function buildPublicSlots(input: {
  day: Date;
  durationMinutes: number;
  busy: BusyRange[];
  now?: Date;
}) {
  const now = input.now ?? new Date();
  const slots: Date[] = [];

  for (const hour of BOOKING_HOURS) {
    if (isPastSlot(input.day, hour, now)) continue;

    const start = new Date(input.day);
    start.setHours(hour, 0, 0, 0);
    const end = new Date(start.getTime() + input.durationMinutes * 60_000);

    // No permitir que termine después de las 18:00
    const dayEnd = new Date(input.day);
    dayEnd.setHours(18, 0, 0, 0);
    if (end.getTime() > dayEnd.getTime()) continue;

    const startMs = start.getTime();
    const endMs = end.getTime();
    const blocked = input.busy.some((range) =>
      rangesOverlap(
        startMs,
        endMs,
        new Date(range.starts_at).getTime(),
        new Date(range.ends_at).getTime()
      )
    );
    if (!blocked) slots.push(start);
  }

  return slots;
}

export function upcomingBookingDays(count = 14, from = new Date()) {
  const days: Date[] = [];
  let cursor = startOfDay(from);
  // si hoy ya pasó toda la jornada, empezar mañana
  while (days.length < count) {
    days.push(new Date(cursor));
    cursor = addDays(cursor, 1);
  }
  return days;
}

export async function createPublicBooking(input: {
  slug: string;
  employeeId: string;
  serviceId: string;
  startsAt: string;
  name: string;
  email: string;
  phone: string;
  document: string;
  notes?: string;
}) {
  const parsed = customerSchema.safeParse({
    name: input.name,
    email: input.email,
    phone: input.phone,
    document: input.document,
  });
  if (!parsed.success) {
    throw new Error(parsed.error.issues[0]?.message ?? "Datos inválidos");
  }

  const supabase = createClient();
  const { data, error } = await supabase.rpc("create_public_booking", {
    p_slug: input.slug,
    p_employee_id: input.employeeId,
    p_service_id: input.serviceId,
    p_starts_at: input.startsAt,
    p_customer_name: parsed.data.name,
    p_customer_email: parsed.data.email,
    p_customer_phone: parsed.data.phone,
    p_customer_document: parsed.data.document,
    p_notes: input.notes?.trim() || null,
  });
  if (error) throw error;
  return data as string;
}
