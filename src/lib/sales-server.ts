import {
  CATALOG_PAGE_SIZE,
  getRange,
  getTotalPages,
} from "@/lib/pagination";
import { getPeriodRange } from "@/lib/sales";
import { createClient } from "@/lib/supabase/server";
import type {
  Package,
  PaymentMethod,
  Product,
  SaleWithDetails,
  Service,
} from "@/types/database";

type SalePaymentRow = {
  total: number;
  payment_method: PaymentMethod;
};

function sumTotals(rows: { total: number }[] | null) {
  return (rows ?? []).reduce((acc, row) => acc + Number(row.total), 0);
}

function paymentBreakdown(rows: SalePaymentRow[] | null) {
  const sales = rows ?? [];
  const cashSales = sales.filter((sale) => sale.payment_method === "cash");
  const transferSales = sales.filter(
    (sale) => sale.payment_method === "transfer"
  );

  return {
    cashCount: cashSales.length,
    transferCount: transferSales.length,
    cashRevenue: sumTotals(cashSales),
    transferRevenue: sumTotals(transferSales),
  };
}

export async function getRevenueMetrics() {
  const supabase = await createClient();
  const dayRange = getPeriodRange("day");
  const weekRange = getPeriodRange("week");
  const monthRange = getPeriodRange("month");

  const [dayRes, weekRes, monthRes, allRes] = await Promise.all([
    supabase
      .from("sales")
      .select("total, payment_method")
      .gte("sold_at", dayRange.from),
    supabase
      .from("sales")
      .select("total, payment_method")
      .gte("sold_at", weekRange.from),
    supabase
      .from("sales")
      .select("total, payment_method")
      .gte("sold_at", monthRange.from),
    supabase.from("sales").select("total, payment_method"),
  ]);

  if (dayRes.error) throw dayRes.error;
  if (weekRes.error) throw weekRes.error;
  if (monthRes.error) throw monthRes.error;
  if (allRes.error) throw allRes.error;

  const daySales = (dayRes.data ?? []) as SalePaymentRow[];
  const weekSales = (weekRes.data ?? []) as SalePaymentRow[];
  const monthSales = (monthRes.data ?? []) as SalePaymentRow[];
  const allSales = (allRes.data ?? []) as SalePaymentRow[];

  return {
    totalRevenue: sumTotals(allSales),
    dayRevenue: sumTotals(daySales),
    weekRevenue: sumTotals(weekSales),
    monthRevenue: sumTotals(monthSales),
    salesCount: allSales.length,
    payments: {
      day: paymentBreakdown(daySales),
      week: paymentBreakdown(weekSales),
      month: paymentBreakdown(monthSales),
      all: paymentBreakdown(allSales),
    },
  };
}

export async function listSales(input: {
  page?: number;
  paymentMethod?: PaymentMethod | "all";
}) {
  const supabase = await createClient();
  const page = Math.max(1, input.page ?? 1);
  const paymentMethod = input.paymentMethod ?? "all";
  const pageSize = CATALOG_PAGE_SIZE;

  let countQuery = supabase
    .from("sales")
    .select("id", { count: "exact", head: true });
  if (paymentMethod !== "all") {
    countQuery = countQuery.eq("payment_method", paymentMethod);
  }
  const { count } = await countQuery;
  const total = count ?? 0;
  const totalPages = getTotalPages(total, pageSize);
  const safePage = Math.min(page, totalPages);
  const { from, to } = getRange(safePage, pageSize);

  let query = supabase
    .from("sales")
    .select(
      `
      *,
      customers ( id, name, email, phone, document ),
      sale_items ( * )
    `
    )
    .order("sold_at", { ascending: false })
    .range(from, to);

  if (paymentMethod !== "all") {
    query = query.eq("payment_method", paymentMethod);
  }

  const { data, error } = await query;
  if (error) throw error;

  return {
    items: (data as unknown as SaleWithDetails[] | null) ?? [],
    total,
    page: safePage,
    totalPages,
    pageSize,
    paymentMethod,
  };
}

export async function getPublishedCatalog() {
  const supabase = await createClient();
  const [products, services, packages] = await Promise.all([
    supabase.from("products").select("*").order("name"),
    supabase.from("services").select("*").order("name"),
    supabase.from("packages").select("*").order("name"),
  ]);

  if (products.error) throw products.error;
  if (services.error) throw services.error;
  if (packages.error) throw packages.error;

  const isPublished = (status: string | null | undefined) =>
    String(status ?? "").trim().toLowerCase() === "published";

  return {
    products: ((products.data ?? []) as Product[]).filter((item) =>
      isPublished(item.status)
    ),
    services: ((services.data ?? []) as Service[]).filter((item) =>
      isPublished(item.status)
    ),
    packages: ((packages.data ?? []) as Package[]).filter((item) =>
      isPublished(item.status)
    ),
  };
}
