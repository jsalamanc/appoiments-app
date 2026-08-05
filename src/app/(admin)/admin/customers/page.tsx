import { CustomersManager } from "@/components/admin/customers-manager";
import { listCustomers } from "@/lib/customers-server";

export default async function CustomersPage({
  searchParams,
}: {
  searchParams: Promise<{ page?: string; q?: string }>;
}) {
  const params = await searchParams;
  const page = Number.parseInt(params.page ?? "1", 10) || 1;
  const result = await listCustomers({ page, q: params.q });

  return (
    <CustomersManager
      customers={result.items}
      page={result.page}
      totalPages={result.totalPages}
      total={result.total}
      pageSize={result.pageSize}
      q={result.q}
    />
  );
}
