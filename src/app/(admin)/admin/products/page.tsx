"use client";

import { invoke } from "@tauri-apps/api/core";
import { useSearchParams } from "next/navigation";
import { useEffect, useState } from "react";

import { ProductsManager } from "@/components/admin/products-manager";
import type { CatalogCounts, StatusFilter } from "@/lib/pagination";
import type { Product } from "@/types/database";

type ListResult = {
  items: Product[];
  total: number;
  page: number;
  total_pages: number;
  page_size: number;
  status: StatusFilter;
  counts: CatalogCounts;
};

export default function ProductsPage() {
  const searchParams = useSearchParams();
  const page = Number.parseInt(searchParams.get("page") ?? "1", 10) || 1;
  const status = (searchParams.get("status") as StatusFilter) ?? "all";

  const [result, setResult] = useState<ListResult | null>(null);

  useEffect(() => {
    let cancelled = false;
    invoke<ListResult>("list_products", { status, page, pageSize: 9 }).then(
      (r) => {
        if (!cancelled) setResult(r);
      }
    );
    return () => {
      cancelled = true;
    };
  }, [page, status]);

  if (!result) return null;

  return (
    <ProductsManager
      products={result.items}
      status={result.status}
      counts={result.counts}
      page={result.page}
      totalPages={result.total_pages}
      total={result.total}
      pageSize={result.page_size}
    />
  );
}
