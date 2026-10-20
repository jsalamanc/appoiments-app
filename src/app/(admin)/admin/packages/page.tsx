"use client";

import { invoke } from "@tauri-apps/api/core";
import { useSearchParams } from "next/navigation";
import { useEffect, useState } from "react";

import { ModuleGate } from "@/components/admin/module-gate";
import { PackagesManager } from "@/components/admin/packages-manager";
import type { CatalogCounts, StatusFilter } from "@/lib/pagination";
import type { PackageWithItems, Product, Service } from "@/types/database";

type ListPackagesResult = {
  items: PackageWithItems[];
  total: number;
  page: number;
  total_pages: number;
  page_size: number;
  status: StatusFilter;
  counts: CatalogCounts;
};

type ListProductsResult = { items: Product[] };
type ListServicesResult = { items: Service[] };

/** Página grande para traer "todos" los productos/servicios que alimentan el
 * selector de ítems del formulario de paquete (antes era un `select('*')`
 * sin límite contra Supabase). */
const ALL_ITEMS_PAGE_SIZE = 1000;

export default function PackagesPage() {
  const searchParams = useSearchParams();
  const page = Number.parseInt(searchParams.get("page") ?? "1", 10) || 1;
  const status = (searchParams.get("status") as StatusFilter) ?? "all";

  const [result, setResult] = useState<ListPackagesResult | null>(null);
  const [products, setProducts] = useState<Product[]>([]);
  const [services, setServices] = useState<Service[]>([]);

  useEffect(() => {
    let cancelled = false;
    Promise.all([
      invoke<ListPackagesResult>("list_packages", { status, page, pageSize: 9 }),
      invoke<ListProductsResult>("list_products", {
        status: "all",
        page: 1,
        pageSize: ALL_ITEMS_PAGE_SIZE,
      }),
      invoke<ListServicesResult>("list_services", {
        status: "all",
        page: 1,
        pageSize: ALL_ITEMS_PAGE_SIZE,
      }),
    ]).then(([packagesRes, productsRes, servicesRes]) => {
      if (cancelled) return;
      setResult(packagesRes);
      setProducts(productsRes.items);
      setServices(servicesRes.items);
    });
    return () => {
      cancelled = true;
    };
  }, [page, status]);

  if (!result) return null;

  return (
    <ModuleGate module="packages">
      <PackagesManager
        packages={result.items}
        products={products}
        services={services}
        status={result.status}
        counts={result.counts}
        page={result.page}
        totalPages={result.total_pages}
        total={result.total}
        pageSize={result.page_size}
      />
    </ModuleGate>
  );
}
