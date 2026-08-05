import { PackagesManager } from "@/components/admin/packages-manager";
import { listPackages } from "@/lib/catalog-list";
import { createClient } from "@/lib/supabase/server";

export default async function PackagesPage({
  searchParams,
}: {
  searchParams: Promise<{ status?: string; page?: string }>;
}) {
  const params = await searchParams;
  const supabase = await createClient();

  const [result, productsRes, servicesRes] = await Promise.all([
    listPackages(params),
    supabase.from("products").select("*").order("name"),
    supabase.from("services").select("*").order("name"),
  ]);

  return (
    <PackagesManager
      packages={result.items}
      products={productsRes.data ?? []}
      services={servicesRes.data ?? []}
      status={result.status}
      counts={result.counts}
      page={result.page}
      totalPages={result.totalPages}
      total={result.total}
      pageSize={result.pageSize}
    />
  );
}
