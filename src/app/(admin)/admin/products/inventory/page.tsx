"use client";

import { invoke } from "@tauri-apps/api/core";
import { useEffect, useState } from "react";

import { InventoryLowStock } from "@/components/admin/inventory-low-stock";
import type { Product } from "@/types/database";

type LowStockResult = {
  products: Product[];
  out_of_stock: Product[];
  low_stock: Product[];
  threshold: number;
};

export default function InventoryPage() {
  const [result, setResult] = useState<LowStockResult | null>(null);
  const [reloadKey, setReloadKey] = useState(0);

  useEffect(() => {
    let cancelled = false;
    invoke<LowStockResult>("list_low_stock_products").then((r) => {
      if (!cancelled) setResult(r);
    });
    return () => {
      cancelled = true;
    };
  }, [reloadKey]);

  if (!result) return null;

  return (
    <InventoryLowStock
      outOfStock={result.out_of_stock}
      lowStock={result.low_stock}
      threshold={result.threshold}
      onRefresh={() => setReloadKey((k) => k + 1)}
    />
  );
}
