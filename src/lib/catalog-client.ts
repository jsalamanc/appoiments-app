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
 *
 * `search` se filtra en SQL (issue #31): antes se traía el catálogo
 * publicado completo sin tope y se filtraba en JS en cada tecla, lo que en
 * un catálogo grande significa cargar todo por IPC solo para mostrar 3
 * resultados. Sin `search`, el backend sigue acotando a `CATALOG_PICKER_CAP`.
 */
export async function fetchSellableCatalog(
  search?: string
): Promise<SellableCatalog> {
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
        invoke<PublishedCatalog>("get_published_catalog", { search }),
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

export type PublishedCatalogItems = {
  products: Product[];
  services: Service[];
  packages: Package[];
  error: string | null;
};

/**
 * Re-consulta solo los ítems publicados que coinciden con `search` — pensada
 * para el picker del POS cuando el usuario ya escribió algo en el buscador
 * (ver `src/components/admin/sale-form.tsx`). A diferencia de
 * `fetchSellableCatalog`, no vuelve a pedir `draftCount`/`totalCount`/módulos:
 * esos describen el catálogo completo, no el resultado de una búsqueda, y
 * recalcularlos a partir de un resultado filtrado daría un "X en borrador"
 * incorrecto mientras se busca.
 */
export async function fetchPublishedCatalogItems(
  search: string,
  modules: { services: boolean; packages: boolean }
): Promise<PublishedCatalogItems> {
  try {
    const catalog = await invoke<PublishedCatalog>("get_published_catalog", {
      search,
    });
    return {
      products: catalog.products,
      services: modules.services ? catalog.services : [],
      packages: modules.packages ? catalog.packages : [],
      error: null,
    };
  } catch (err) {
    return {
      products: [],
      services: [],
      packages: [],
      error: err instanceof Error ? err.message : String(err),
    };
  }
}
