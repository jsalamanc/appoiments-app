import { LOW_STOCK_THRESHOLD } from "@/lib/inventory";
import { createClient } from "@/lib/supabase/server";
import type { Product } from "@/types/database";

export async function listLowStockProducts(threshold = LOW_STOCK_THRESHOLD) {
  const supabase = await createClient();
  const { data, error } = await supabase
    .from("products")
    .select("*")
    .lte("stock", threshold)
    .order("stock", { ascending: true })
    .order("name", { ascending: true });

  if (error) throw error;

  const products = (data ?? []) as Product[];
  return {
    products,
    outOfStock: products.filter((p) => p.stock <= 0),
    lowStock: products.filter((p) => p.stock > 0 && p.stock <= threshold),
    threshold,
  };
}
