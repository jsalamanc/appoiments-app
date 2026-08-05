import { createClient } from "@/lib/supabase/client";

/** Ajusta el stock de un producto y registra movimiento (initial/adjustment). */
export async function setProductStock(
  productId: string,
  stock: number,
  note?: string
) {
  const supabase = createClient();
  const { data, error } = await supabase.rpc("set_product_stock", {
    p_product_id: productId,
    p_stock: stock,
    p_note: note ?? null,
  });

  if (error) throw error;
  return data as number;
}
