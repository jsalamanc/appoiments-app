import {
  CATALOG_PAGE_SIZE,
  getRange,
  getTotalPages,
} from "@/lib/pagination";
import { createClient } from "@/lib/supabase/server";
import type {
  AppointmentWithDetails,
  Customer,
  SaleWithDetails,
} from "@/types/database";

export async function listCustomers(input: {
  page?: number;
  q?: string;
}) {
  const supabase = await createClient();
  const page = Math.max(1, input.page ?? 1);
  const pageSize = CATALOG_PAGE_SIZE;
  const q = input.q?.trim() ?? "";

  let countQuery = supabase
    .from("customers")
    .select("id", { count: "exact", head: true });
  let listQuery = supabase
    .from("customers")
    .select("*")
    .order("name", { ascending: true });

  if (q) {
    const pattern = `%${q}%`;
    countQuery = countQuery.or(
      `name.ilike.${pattern},email.ilike.${pattern},document.ilike.${pattern},phone.ilike.${pattern}`
    );
    listQuery = listQuery.or(
      `name.ilike.${pattern},email.ilike.${pattern},document.ilike.${pattern},phone.ilike.${pattern}`
    );
  }

  const { count, error: countError } = await countQuery;
  if (countError) throw countError;

  const total = count ?? 0;
  const totalPages = getTotalPages(total, pageSize);
  const safePage = Math.min(page, totalPages);
  const { from, to } = getRange(safePage, pageSize);

  const { data, error } = await listQuery.range(from, to);
  if (error) throw error;

  return {
    items: (data ?? []) as Customer[],
    total,
    page: safePage,
    totalPages,
    pageSize,
    q,
  };
}

export async function getCustomerDetail(id: string) {
  const supabase = await createClient();

  const { data: customer, error } = await supabase
    .from("customers")
    .select("*")
    .eq("id", id)
    .maybeSingle();

  if (error) throw error;
  if (!customer) return null;

  const [salesRes, appointmentsRes] = await Promise.all([
    supabase
      .from("sales")
      .select(
        `
        *,
        customers ( id, name, email, phone, document ),
        sale_items ( * )
      `
      )
      .eq("customer_id", id)
      .order("sold_at", { ascending: false })
      .limit(20),
    supabase
      .from("appointments")
      .select(
        `
        *,
        employees ( id, name, color, active ),
        customers ( id, name, email, phone, document ),
        services ( id, name, price, duration_minutes, status )
      `
      )
      .eq("customer_id", id)
      .order("starts_at", { ascending: false })
      .limit(20),
  ]);

  if (salesRes.error) throw salesRes.error;
  // appointments table may be unused if module off; ignore missing gracefully
  const appointments = appointmentsRes.error
    ? []
    : ((appointmentsRes.data ?? []) as unknown as AppointmentWithDetails[]);

  return {
    customer: customer as Customer,
    sales: (salesRes.data as unknown as SaleWithDetails[] | null) ?? [],
    appointments,
  };
}
