import type { CatalogStatus } from "@/lib/catalog-status";

export const CATALOG_PAGE_SIZE = 9;

export type StatusFilter = "all" | CatalogStatus;

export type CatalogListQuery = {
  status: StatusFilter;
  page: number;
};

export function parseCatalogListQuery(input: {
  status?: string;
  page?: string;
}): CatalogListQuery {
  const status: StatusFilter =
    input.status === "draft" || input.status === "published"
      ? input.status
      : "all";

  const page = Math.max(1, Number.parseInt(input.page ?? "1", 10) || 1);

  return { status, page };
}

export function getTotalPages(total: number, pageSize = CATALOG_PAGE_SIZE) {
  return Math.max(1, Math.ceil(total / pageSize));
}

export function getRange(page: number, pageSize = CATALOG_PAGE_SIZE) {
  const from = (page - 1) * pageSize;
  const to = from + pageSize - 1;
  return { from, to };
}

export function buildCatalogHref(
  pathname: string,
  query: { status?: StatusFilter; page?: number }
) {
  const params = new URLSearchParams();
  if (query.status && query.status !== "all") {
    params.set("status", query.status);
  }
  if (query.page && query.page > 1) {
    params.set("page", String(query.page));
  }
  const qs = params.toString();
  return qs ? `${pathname}?${qs}` : pathname;
}
