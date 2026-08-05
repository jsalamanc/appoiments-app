import {
  CATALOG_PAGE_SIZE,
  getRange,
  getTotalPages,
  parseCatalogListQuery,
  type CatalogListQuery,
  type StatusFilter,
} from "@/lib/pagination";
import { createClient } from "@/lib/supabase/server";
import type { PackageWithItems, Product, Service } from "@/types/database";

export type CatalogCounts = {
  all: number;
  published: number;
  draft: number;
};

export type CatalogListResult<T> = {
  items: T[];
  total: number;
  page: number;
  totalPages: number;
  pageSize: number;
  status: StatusFilter;
  counts: CatalogCounts;
  query: CatalogListQuery;
};

async function getStatusCounts(table: "products" | "services" | "packages") {
  const supabase = await createClient();
  const [all, published, draft] = await Promise.all([
    supabase.from(table).select("id", { count: "exact", head: true }),
    supabase
      .from(table)
      .select("id", { count: "exact", head: true })
      .eq("status", "published"),
    supabase
      .from(table)
      .select("id", { count: "exact", head: true })
      .eq("status", "draft"),
  ]);

  return {
    all: all.count ?? 0,
    published: published.count ?? 0,
    draft: draft.count ?? 0,
  } satisfies CatalogCounts;
}

function clampPage(page: number, total: number, pageSize: number) {
  const totalPages = getTotalPages(total, pageSize);
  return Math.min(page, totalPages);
}

export async function listProducts(
  searchParams: { status?: string; page?: string }
): Promise<CatalogListResult<Product>> {
  const supabase = await createClient();
  const query = parseCatalogListQuery(searchParams);
  const counts = await getStatusCounts("products");
  const total = counts[query.status];
  const page = clampPage(query.page, total, CATALOG_PAGE_SIZE);
  const { from, to } = getRange(page);

  let request = supabase
    .from("products")
    .select("*", { count: "exact" })
    .order("created_at", { ascending: false })
    .range(from, to);

  if (query.status !== "all") {
    request = request.eq("status", query.status);
  }

  const { data, error } = await request;
  if (error) throw error;

  return {
    items: data ?? [],
    total,
    page,
    totalPages: getTotalPages(total),
    pageSize: CATALOG_PAGE_SIZE,
    status: query.status,
    counts,
    query: { ...query, page },
  };
}

export async function listServices(
  searchParams: { status?: string; page?: string }
): Promise<CatalogListResult<Service>> {
  const supabase = await createClient();
  const query = parseCatalogListQuery(searchParams);
  const counts = await getStatusCounts("services");
  const total = counts[query.status];
  const page = clampPage(query.page, total, CATALOG_PAGE_SIZE);
  const { from, to } = getRange(page);

  let request = supabase
    .from("services")
    .select("*", { count: "exact" })
    .order("created_at", { ascending: false })
    .range(from, to);

  if (query.status !== "all") {
    request = request.eq("status", query.status);
  }

  const { data, error } = await request;
  if (error) throw error;

  return {
    items: data ?? [],
    total,
    page,
    totalPages: getTotalPages(total),
    pageSize: CATALOG_PAGE_SIZE,
    status: query.status,
    counts,
    query: { ...query, page },
  };
}

export async function listPackages(
  searchParams: { status?: string; page?: string }
): Promise<CatalogListResult<PackageWithItems>> {
  const supabase = await createClient();
  const query = parseCatalogListQuery(searchParams);
  const counts = await getStatusCounts("packages");
  const total = counts[query.status];
  const page = clampPage(query.page, total, CATALOG_PAGE_SIZE);
  const { from, to } = getRange(page);

  let request = supabase
    .from("packages")
    .select(
      `
      *,
      package_items (
        *,
        products ( id, name, price ),
        services ( id, name, price, duration_minutes )
      )
    `,
      { count: "exact" }
    )
    .order("created_at", { ascending: false })
    .range(from, to);

  if (query.status !== "all") {
    request = request.eq("status", query.status);
  }

  const { data, error } = await request;
  if (error) throw error;

  return {
    items: (data as unknown as PackageWithItems[] | null) ?? [],
    total,
    page,
    totalPages: getTotalPages(total),
    pageSize: CATALOG_PAGE_SIZE,
    status: query.status,
    counts,
    query: { ...query, page },
  };
}
