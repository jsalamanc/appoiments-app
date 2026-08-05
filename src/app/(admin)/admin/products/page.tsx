import { ProductsManager } from "@/components/admin/products-manager";
import { listProducts } from "@/lib/catalog-list";

export default async function ProductsPage({
  searchParams,
}: {
  searchParams: Promise<{ status?: string; page?: string }>;
}) {
  const params = await searchParams;
  const result = await listProducts(params);

  return (
    <ProductsManager
      products={result.items}
      status={result.status}
      counts={result.counts}
      page={result.page}
      totalPages={result.totalPages}
      total={result.total}
      pageSize={result.pageSize}
    />
  );
}
