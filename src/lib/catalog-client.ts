import { getCurrentCenter } from "@/lib/catalog";
import { isModuleEnabled, parseCenterModules } from "@/lib/modules";
import { createClient } from "@/lib/supabase/client";
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

export async function fetchSellableCatalog(): Promise<SellableCatalog> {
  const supabase = createClient();
  const center = await getCurrentCenter();
  const modules = parseCenterModules(center?.modules);
  const servicesEnabled = isModuleEnabled(modules, "services");
  const packagesEnabled = isModuleEnabled(modules, "packages");

  const [productsRes, servicesRes, packagesRes] = await Promise.all([
    supabase.from("products").select("*").order("name"),
    servicesEnabled
      ? supabase.from("services").select("*").order("name")
      : Promise.resolve({ data: [] as Service[], error: null }),
    packagesEnabled
      ? supabase.from("packages").select("*").order("name")
      : Promise.resolve({ data: [] as Package[], error: null }),
  ]);

  const error =
    productsRes.error?.message ||
    servicesRes.error?.message ||
    packagesRes.error?.message ||
    null;

  const allProducts = productsRes.data ?? [];
  const allServices = servicesRes.data ?? [];
  const allPackages = packagesRes.data ?? [];

  const isPublished = (status: string | null | undefined) =>
    String(status ?? "").trim().toLowerCase() === "published";

  const products = allProducts.filter((item) => isPublished(item.status));
  const services = allServices.filter((item) => isPublished(item.status));
  const packages = allPackages.filter((item) => isPublished(item.status));

  const totalCount = allProducts.length + allServices.length + allPackages.length;
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
