import { InventoryLowStock } from "@/components/admin/inventory-low-stock";
import { listLowStockProducts } from "@/lib/inventory-server";

export default async function InventoryPage() {
  const result = await listLowStockProducts();

  return (
    <InventoryLowStock
      outOfStock={result.outOfStock}
      lowStock={result.lowStock}
      threshold={result.threshold}
    />
  );
}
