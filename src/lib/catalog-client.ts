import { createClient } from "@/lib/supabase/client";
import type { Package, Product, Service } from "@/types/database";

export type SellableCatalog = {
  products: Product[];
  services: Service[];
  packages: Package[];
  draftCount: number;
  totalCount: number;
  error: string | null;
};

export async function fetchSellableCatalog(): Promise<SellableCatalog> {
  const supabase = createClient();

  const [productsRes, servicesRes, packagesRes] = await Promise.all([
    supabase.from("products").select("*").order("name"),
    supabase.from("services").select("*").order("name"),
    supabase.from("packages").select("*").order("name"),
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
  };
}
