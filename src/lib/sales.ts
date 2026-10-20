import { invoke } from "@tauri-apps/api/core";

import type { PaymentMethod, SaleItemType } from "@/types/database";

export type CartLineInput = {
  item_type: SaleItemType;
  item_id: string;
  name: string;
  unit_price: number;
  quantity: number;
};

export type RevenuePeriod = "day" | "week" | "month";

export function cartLineTotal(unitPrice: number, quantity: number) {
  return Number((unitPrice * quantity).toFixed(2));
}

export function cartTotal(items: CartLineInput[]) {
  return items.reduce(
    (sum, item) => sum + cartLineTotal(item.unit_price, item.quantity),
    0
  );
}

export function getPeriodRange(period: RevenuePeriod, now = new Date()) {
  const to = now.toISOString();

  if (period === "day") {
    const start = new Date(now);
    start.setHours(0, 0, 0, 0);
    return { from: start.toISOString(), to };
  }

  if (period === "week") {
    const start = new Date(now);
    start.setDate(start.getDate() - 6);
    start.setHours(0, 0, 0, 0);
    return { from: start.toISOString(), to };
  }

  const start = new Date(now.getFullYear(), now.getMonth(), 1);
  start.setHours(0, 0, 0, 0);
  return { from: start.toISOString(), to };
}

export async function createSale(input: {
  customerId: string;
  paymentMethod: PaymentMethod;
  items: CartLineInput[];
}): Promise<string> {
  if (input.items.length === 0) {
    throw new Error("Agrega al menos un ítem a la venta");
  }

  return invoke<string>("create_sale_with_inventory", {
    customerId: input.customerId,
    paymentMethod: input.paymentMethod,
    items: input.items.map((item) => ({
      item_type: item.item_type,
      item_id: item.item_id,
      name: item.name,
      unit_price: item.unit_price,
      quantity: item.quantity,
    })),
  });
}
