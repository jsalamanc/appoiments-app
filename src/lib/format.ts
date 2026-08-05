import type { PaymentMethod, SaleItemType } from "@/types/database";

export function formatCurrency(value: number) {
  return new Intl.NumberFormat("es-CO", {
    style: "currency",
    currency: "COP",
    maximumFractionDigits: 0,
  }).format(value);
}

export function formatDuration(minutes: number) {
  if (minutes < 60) return `${minutes} min`;
  const hours = Math.floor(minutes / 60);
  const rest = minutes % 60;
  if (rest === 0) return `${hours} h`;
  return `${hours} h ${rest} min`;
}

export function formatPaymentMethod(method: PaymentMethod) {
  return method === "cash" ? "Efectivo" : "Transferencia";
}

export function formatSaleItemType(type: SaleItemType) {
  if (type === "product") return "Producto";
  if (type === "service") return "Servicio";
  return "Paquete";
}

export function formatDateTime(value: string) {
  return new Intl.DateTimeFormat("es-CO", {
    dateStyle: "medium",
    timeStyle: "short",
  }).format(new Date(value));
}
