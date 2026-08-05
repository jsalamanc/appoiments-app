import { ProductsManager } from "@/components/admin/products-manager";
import { listProducts } from "@/lib/catalog-list";
import { getServerProductLimit } from "@/lib/plans-server";

export default async function ProductsPage({
  searchParams,
}: {
  searchParams: Promise<{ status?: string; page?: string }>;
}) {
  const params = await searchParams;
  const [result, productLimit] = await Promise.all([
    listProducts(params),
    getServerProductLimit(),
  ]);

  return (
    <ProductsManager
      products={result.items}
      status={result.status}
      counts={result.counts}
      page={result.page}
      totalPages={result.totalPages}
      total={result.total}
      pageSize={result.pageSize}
      productLimit={productLimit}
    />
  );
}
