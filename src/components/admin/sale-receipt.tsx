"use client";

import { ArrowLeft, Printer } from "lucide-react";
import Link from "next/link";

import { Button } from "@/components/ui/button";
import {
  formatCurrency,
  formatDateTime,
  formatPaymentMethod,
  formatSaleItemType,
} from "@/lib/format";
import type { SaleWithDetails } from "@/types/database";

type SaleReceiptProps = {
  sale: SaleWithDetails;
  businessName: string;
};

export function SaleReceipt({ sale, businessName }: SaleReceiptProps) {
  const subtotal = sale.sale_items.reduce(
    (sum, item) => sum + Number(item.line_total),
    0
  );

  return (
    <div className="mx-auto max-w-md space-y-6 py-6">
      <div className="print-hidden flex items-center justify-between">
        <Button variant="outline" size="sm" render={<Link href="/admin/sales/history" />}>
          <ArrowLeft data-icon="inline-start" />
          Volver
        </Button>
        <Button size="sm" onClick={() => window.print()}>
          <Printer data-icon="inline-start" />
          Imprimir
        </Button>
      </div>

      <div className="space-y-4 rounded-3xl border border-border/70 bg-card p-6 shadow-sm">
        <div className="space-y-1 text-center">
          <h1 className="font-heading text-2xl font-semibold">{businessName}</h1>
          <p className="text-sm text-muted-foreground">Recibo de venta</p>
          {sale.voided_at && (
            <p className="font-medium text-destructive">VENTA ANULADA</p>
          )}
        </div>

        <div className="space-y-1 text-sm text-muted-foreground">
          <p>Fecha: {formatDateTime(sale.sold_at)}</p>
          <p>Cliente: {sale.customers.name}</p>
          {sale.customers.phone && <p>Teléfono: {sale.customers.phone}</p>}
          {sale.customers.email && <p>Correo: {sale.customers.email}</p>}
          {sale.customers.document && <p>Documento: {sale.customers.document}</p>}
        </div>

        <div className="space-y-2 border-y border-dashed border-border py-3">
          {sale.sale_items.map((item) => (
            <div key={item.id} className="flex items-start justify-between gap-3 text-sm">
              <div>
                <p className="font-medium">{item.name_snapshot}</p>
                <p className="text-xs text-muted-foreground">
                  {formatSaleItemType(item.item_type)} ·{" "}
                  {formatCurrency(Number(item.unit_price))} × {item.quantity}
                </p>
              </div>
              <p className="font-medium">{formatCurrency(Number(item.line_total))}</p>
            </div>
          ))}
        </div>

        <div className="space-y-1 text-sm">
          {sale.discount_amount > 0 && (
            <div className="flex items-center justify-between text-muted-foreground">
              <span>Subtotal</span>
              <span>{formatCurrency(subtotal)}</span>
            </div>
          )}
          {sale.discount_amount > 0 && (
            <div className="flex items-center justify-between text-muted-foreground">
              <span>Descuento</span>
              <span>-{formatCurrency(sale.discount_amount)}</span>
            </div>
          )}
          <div className="flex items-center justify-between text-lg font-semibold">
            <span>Total</span>
            <span>{formatCurrency(Number(sale.total))}</span>
          </div>
          <div className="flex items-center justify-between text-muted-foreground">
            <span>Medio de pago</span>
            <span>{formatPaymentMethod(sale.payment_method)}</span>
          </div>
          {sale.payments.length > 1 && (
            <div className="space-y-1 pt-1 text-xs text-muted-foreground">
              {sale.payments.map((payment) => (
                <div key={payment.id} className="flex items-center justify-between">
                  <span>{formatPaymentMethod(payment.method)}</span>
                  <span>{formatCurrency(Number(payment.amount))}</span>
                </div>
              ))}
            </div>
          )}
        </div>
      </div>
    </div>
  );
}
