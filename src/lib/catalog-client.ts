import { invoke } from "@tauri-apps/api/core";

import { getEnabledModules } from "@/lib/modules";
import type { Package, Product, Service } from "@/types/database";

export type SellableCatalog = {
  products: Product[];
  services: Service[];
  packages: Package[];
  draftCount: number;
  totalCount: number;
  error: string | null;
  modules: {
    services: boolean;
    packages: boolean;
  };
};

type PublishedCatalog = {
  products: Product[];
  services: Service[];
  packages: Package[];
};

type CountsOnly = {
  counts: { all: number; published: number; draft: number };
};

const ZERO_COUNTS: CountsOnly = { counts: { all: 0, published: 0, draft: 0 } };

/**
 * Picker de ítems del POS (ver `src/components/admin/sale-form.tsx`).
 * Antes consultaba Supabase directo; ahora usa `get_published_catalog`
 * (comando Rust, dominio catálogo) para los ítems publicados y los
 * `list_*` existentes (pidiendo solo `counts`, con `pageSize: 1`) para los
 * conteos totales que alimentan `draftCount`/`totalCount`. El módulo de
 * servicios/paquetes se sigue consultando con `get_enabled_modules`.
 */
export async function fetchSellableCatalog(): Promise<SellableCatalog> {
  const modules = await getEnabledModules();
  const servicesEnabled = modules.services;
  const packagesEnabled = modules.packages;

  let products: Product[] = [];
  let services: Service[] = [];
  let packages: Package[] = [];
  let totalCount = 0;
  let error: string | null = null;

  try {
    const [catalog, productCounts, serviceCounts, packageCounts] =
      await Promise.all([
        invoke<PublishedCatalog>("get_published_catalog"),
        invoke<CountsOnly>("list_products", {
          status: "all",
          page: 1,
          pageSize: 1,
        }),
        servicesEnabled
          ? invoke<CountsOnly>("list_services", {
              status: "all",
              page: 1,
              pageSize: 1,
            })
          : Promise.resolve(ZERO_COUNTS),
        packagesEnabled
          ? invoke<CountsOnly>("list_packages", {
              status: "all",
              page: 1,
              pageSize: 1,
            })
          : Promise.resolve(ZERO_COUNTS),
      ]);

    products = catalog.products;
    services = servicesEnabled ? catalog.services : [];
    packages = packagesEnabled ? catalog.packages : [];

    totalCount =
      productCounts.counts.all +
      serviceCounts.counts.all +
      packageCounts.counts.all;
  } catch (err) {
    error = err instanceof Error ? err.message : String(err);
  }

  const publishedCount = products.length + services.length + packages.length;
  const draftCount = totalCount - publishedCount;

  return {
    products,
    services,
    packages,
    draftCount,
    totalCount,
    error,
    modules: {
      services: servicesEnabled,
      packages: packagesEnabled,
    },
  };
}
