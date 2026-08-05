import { ServicesManager } from "@/components/admin/services-manager";
import { listServices } from "@/lib/catalog-list";

export default async function ServicesPage({
  searchParams,
}: {
  searchParams: Promise<{ status?: string; page?: string }>;
}) {
  const params = await searchParams;
  const result = await listServices(params);

  return (
    <ServicesManager
      services={result.items}
      status={result.status}
      counts={result.counts}
      page={result.page}
      totalPages={result.totalPages}
      total={result.total}
      pageSize={result.pageSize}
    />
  );
}
