import { invoke } from "@tauri-apps/api/core";

import type { PaymentMethod, SaleItemType, SaleWithDetails } from "@/types/database";

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

// Ventana inmediatamente anterior a la actual, de la misma duración/tipo de
// calendario — usada para el delta "+12% vs periodo anterior" del reporte de
// tendencias (issue #23). El límite superior del periodo anterior es
// siempre el `from` del periodo actual, así que no hace falta mandar un "to"
// explícito al backend (ver `get_revenue_trends_report`).
//
// Para "mes" se compara contra el mes calendario anterior COMPLETO (ej. todo
// octubre vs. lo que va de noviembre), no contra los mismos días
// transcurridos — es lo que la gente espera al leer "vs. mes pasado", aunque
// a inicio de mes el % pueda verse artificialmente bajo.
export function getPreviousPeriodRange(period: RevenuePeriod, now = new Date()) {
  const current = getPeriodRange(period, now);

  if (period === "day") {
    const start = new Date(current.from);
    start.setDate(start.getDate() - 1);
    return { from: start.toISOString() };
  }

  if (period === "week") {
    const start = new Date(current.from);
    start.setDate(start.getDate() - 7);
    return { from: start.toISOString() };
  }

  const currentStart = new Date(current.from);
  const start = new Date(currentStart.getFullYear(), currentStart.getMonth() - 1, 1);
  start.setHours(0, 0, 0, 0);
  return { from: start.toISOString() };
}

export type PaymentLineInput = {
  method: PaymentMethod;
  amount: number;
};

export async function createSale(input: {
  customerId: string;
  items: CartLineInput[];
  payments: PaymentLineInput[];
  discountAmount?: number;
}): Promise<string> {
  if (input.items.length === 0) {
    throw new Error("Agrega al menos un ítem a la venta");
  }
  if (input.payments.length === 0) {
    throw new Error("Agrega al menos un medio de pago");
  }

  return invoke<string>("create_sale_with_inventory", {
    customerId: input.customerId,
    items: input.items.map((item) => ({
      item_type: item.item_type,
      item_id: item.item_id,
      name: item.name,
      unit_price: item.unit_price,
      quantity: item.quantity,
    })),
    payments: input.payments.map((payment) => ({
      method: payment.method,
      amount: payment.amount,
    })),
    discountAmount: input.discountAmount,
  });
}

// Anula una venta ya registrada — revierte el stock vendido y la saca de
// caja/ingresos (issue #2, Fase 2). Solo el dueño puede hacerlo.
export async function voidSale(saleId: string, reason?: string): Promise<void> {
  await invoke("void_sale", { saleId, reason });
}

/** Venta individual con todos sus detalles, para el recibo imprimible
 * (issue #2, Fase 5). */
export async function getSale(id: string): Promise<SaleWithDetails | null> {
  return invoke<SaleWithDetails | null>("get_sale", { id });
}
