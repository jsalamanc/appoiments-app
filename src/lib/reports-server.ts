import type {
  EmployeePerformanceRow,
  NoShowCustomerRow,
  ReportPeriod,
  TopServiceRow,
} from "@/lib/reports";
import { getPeriodRange } from "@/lib/sales";
import { createClient } from "@/lib/supabase/server";

export type {
  EmployeePerformanceRow,
  NoShowCustomerRow,
  ReportPeriod,
  TopServiceRow,
} from "@/lib/reports";
export { getReportPeriod } from "@/lib/reports";

export async function getEmployeePerformance(period: ReportPeriod = "month") {
  const supabase = await createClient();
  const { from } = getPeriodRange(period);

  const [employeesRes, appointmentsRes] = await Promise.all([
    supabase.from("employees").select("id, name, color, active").order("name"),
    supabase
      .from("appointments")
      .select("id, employee_id, status, sale_id, starts_at")
      .gte("starts_at", from),
  ]);

  if (employeesRes.error) throw employeesRes.error;
  if (appointmentsRes.error) throw appointmentsRes.error;

  const saleIds = (appointmentsRes.data ?? [])
    .map((a) => a.sale_id)
    .filter((id): id is string => Boolean(id));

  let salesById = new Map<string, number>();
  if (saleIds.length > 0) {
    const { data: sales, error } = await supabase
      .from("sales")
      .select("id, total")
      .in("id", saleIds);
    if (error) throw error;
    salesById = new Map(
      (sales ?? []).map((s) => [s.id, Number(s.total)])
    );
  }

  const rows: EmployeePerformanceRow[] = (employeesRes.data ?? []).map(
    (employee) => {
      const appts = (appointmentsRes.data ?? []).filter(
        (a) => a.employee_id === employee.id
      );
      const scheduled = appts.filter((a) => a.status === "scheduled").length;
      const completed = appts.filter((a) => a.status === "completed").length;
      const cancelled = appts.filter((a) => a.status === "cancelled").length;
      const noShow = appts.filter((a) => a.status === "no_show").length;
      const revenue = appts.reduce((sum, a) => {
        if (!a.sale_id) return sum;
        return sum + (salesById.get(a.sale_id) ?? 0);
      }, 0);

      return {
        employeeId: employee.id,
        name: employee.name,
        color: employee.color,
        scheduled,
        completed,
        cancelled,
        noShow,
        totalAppointments: appts.length,
        revenue,
      };
    }
  );

  rows.sort(
    (a, b) =>
      b.completed - a.completed ||
      b.revenue - a.revenue ||
      a.name.localeCompare(b.name)
  );

  return { period, from, rows };
}

export async function getTopServicesReport(period: ReportPeriod = "month") {
  const supabase = await createClient();
  const { from } = getPeriodRange(period);

  const { data: sales, error: salesError } = await supabase
    .from("sales")
    .select("id, total, sold_at")
    .gte("sold_at", from);
  if (salesError) throw salesError;

  const saleIds = (sales ?? []).map((s) => s.id);
  const salesCount = saleIds.length;
  const totalRevenue = (sales ?? []).reduce(
    (sum, s) => sum + Number(s.total),
    0
  );
  const averageTicket = salesCount === 0 ? 0 : totalRevenue / salesCount;

  if (saleIds.length === 0) {
    return {
      period,
      from,
      salesCount: 0,
      totalRevenue: 0,
      averageTicket: 0,
      rows: [] as TopServiceRow[],
    };
  }

  const { data: items, error: itemsError } = await supabase
    .from("sale_items")
    .select("service_id, name_snapshot, quantity, line_total, item_type, sale_id")
    .in("sale_id", saleIds)
    .eq("item_type", "service");
  if (itemsError) throw itemsError;

  const map = new Map<string, TopServiceRow>();
  for (const item of items ?? []) {
    const key = item.service_id ?? item.name_snapshot;
    const current = map.get(key) ?? {
      serviceId: item.service_id,
      name: item.name_snapshot,
      quantity: 0,
      revenue: 0,
    };
    current.quantity += item.quantity;
    current.revenue += Number(item.line_total);
    map.set(key, current);
  }

  const rows = [...map.values()].sort(
    (a, b) => b.quantity - a.quantity || b.revenue - a.revenue
  );

  return {
    period,
    from,
    salesCount,
    totalRevenue,
    averageTicket,
    rows,
  };
}

export async function getNoShowsReport(period: ReportPeriod = "month") {
  const supabase = await createClient();
  const { from } = getPeriodRange(period);

  type NoShowAppointmentRow = {
    id: string;
    status: "no_show" | "cancelled";
    starts_at: string;
    customer_id: string;
    customers: {
      id: string;
      name: string;
      email: string;
      phone: string;
    } | null;
  };

  const { data, error } = await supabase
    .from("appointments")
    .select(
      `
      id,
      status,
      starts_at,
      customer_id,
      customers ( id, name, email, phone )
    `
    )
    .gte("starts_at", from)
    .in("status", ["no_show", "cancelled"])
    .order("starts_at", { ascending: false });

  if (error) throw error;

  const appointments = (data ?? []) as unknown as NoShowAppointmentRow[];
  const byCustomer = new Map<string, NoShowCustomerRow>();
  let noShowTotal = 0;
  let cancelledTotal = 0;

  for (const row of appointments) {
    const customer = row.customers;
    if (!customer) continue;

    if (row.status === "no_show") noShowTotal += 1;
    if (row.status === "cancelled") cancelledTotal += 1;

    const current = byCustomer.get(customer.id) ?? {
      customerId: customer.id,
      name: customer.name,
      email: customer.email,
      phone: customer.phone,
      noShowCount: 0,
      cancelledCount: 0,
      lostCount: 0,
    };
    if (row.status === "no_show") current.noShowCount += 1;
    if (row.status === "cancelled") current.cancelledCount += 1;
    current.lostCount = current.noShowCount + current.cancelledCount;
    byCustomer.set(customer.id, current);
  }

  const rows = [...byCustomer.values()].sort(
    (a, b) => b.lostCount - a.lostCount || a.name.localeCompare(b.name)
  );

  return {
    period,
    from,
    noShowTotal,
    cancelledTotal,
    rows,
    recent: appointments.slice(0, 30),
  };
}
