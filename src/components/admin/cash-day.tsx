"use client";

import { Banknote, History } from "lucide-react";
import Link from "next/link";
import { useRouter } from "next/navigation";
import { useState } from "react";

import { EmptyState } from "@/components/admin/empty-state";
import { FadeIn } from "@/components/motion/fade-in";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import {
  formatCurrency,
  formatDateTime,
  formatPaymentMethod,
  formatSaleItemType,
} from "@/lib/format";
import type { SaleWithDetails } from "@/types/database";

type CashDayProps = {
  sales: SaleWithDetails[];
  salesCount: number;
  totalRevenue: number;
  payments: {
    cashCount: number;
    transferCount: number;
    cashRevenue: number;
    transferRevenue: number;
  };
};

export function CashDay({
  sales,
  salesCount,
  totalRevenue,
  payments,
}: CashDayProps) {
  const router = useRouter();
  const [selected, setSelected] = useState<SaleWithDetails | null>(null);
  const todayLabel = new Intl.DateTimeFormat("es-CO", {
    weekday: "long",
    day: "numeric",
    month: "long",
  }).format(new Date());

  return (
    <div className="mx-auto max-w-5xl space-y-6">
      <FadeIn className="flex flex-col gap-4 sm:flex-row sm:items-end sm:justify-between">
        <div>
          <p className="text-sm text-muted-foreground">Ventas</p>
          <h1 className="font-heading text-4xl font-semibold">Caja del día</h1>
          <p className="mt-1 capitalize text-muted-foreground">{todayLabel}</p>
        </div>
        <div className="flex flex-wrap gap-2">
          <Button render={<Link href="/admin/sales" />}>Nueva venta</Button>
          <Button
            variant="outline"
            render={<Link href="/admin/sales/history" />}
          >
            <History data-icon="inline-start" />
            Historial
          </Button>
        </div>
      </FadeIn>

      <FadeIn
        delay={0.05}
        className="grid gap-4 sm:grid-cols-2 lg:grid-cols-4"
      >
        <div className="rounded-3xl border border-border/70 bg-card shadow-sm p-5">
          <p className="text-sm text-muted-foreground">Total cobrado</p>
          <p className="font-heading text-3xl font-semibold">
            {formatCurrency(totalRevenue)}
          </p>
        </div>
        <div className="rounded-3xl border border-border/70 bg-card shadow-sm p-5">
          <p className="text-sm text-muted-foreground">Ventas</p>
          <p className="font-heading text-3xl font-semibold">{salesCount}</p>
        </div>
        <div className="rounded-3xl border border-border/70 bg-card shadow-sm p-5">
          <p className="text-sm text-muted-foreground">Efectivo</p>
          <p className="font-heading text-3xl font-semibold">
            {formatCurrency(payments.cashRevenue)}
          </p>
          <p className="text-xs text-muted-foreground">
            {payments.cashCount} venta{payments.cashCount === 1 ? "" : "s"}
          </p>
        </div>
        <div className="rounded-3xl border border-border/70 bg-card shadow-sm p-5">
          <p className="text-sm text-muted-foreground">Transferencia</p>
          <p className="font-heading text-3xl font-semibold">
            {formatCurrency(payments.transferRevenue)}
          </p>
          <p className="text-xs text-muted-foreground">
            {payments.transferCount} venta
            {payments.transferCount === 1 ? "" : "s"}
          </p>
        </div>
      </FadeIn>

      {sales.length === 0 ? (
        <EmptyState
          title="Sin ventas hoy"
          description="Cuando registres ventas, aparecerán aquí para el cierre de caja."
          actionLabel="Nueva venta"
          onAction={() => router.push("/admin/sales")}
        />
      ) : (
        <FadeIn delay={0.08} className="space-y-3">
          <div className="flex items-center gap-2">
            <Banknote className="size-4 text-muted-foreground" />
            <h2 className="font-heading text-2xl font-semibold">
              Movimientos de hoy
            </h2>
          </div>
          {sales.map((sale) => (
            <button
              key={sale.id}
              type="button"
              onClick={() => setSelected(sale)}
              className="flex w-full flex-col gap-2 rounded-3xl border border-border/70 bg-card shadow-sm px-4 py-3 text-left transition hover:border-primary/40 sm:flex-row sm:items-center sm:justify-between"
            >
              <div>
                <p className="font-medium">{sale.customers.name}</p>
                <p className="text-sm text-muted-foreground">
                  {formatDateTime(sale.sold_at)} ·{" "}
                  {formatPaymentMethod(sale.payment_method)}
                </p>
              </div>
              <p className="font-semibold">
                {formatCurrency(Number(sale.total))}
              </p>
            </button>
          ))}
        </FadeIn>
      )}

      <Dialog
        open={!!selected}
        onOpenChange={(open) => !open && setSelected(null)}
      >
        <DialogContent className="sm:max-w-md">
          <DialogHeader>
            <DialogTitle>Detalle de venta</DialogTitle>
            <DialogDescription>
              {selected ? formatDateTime(selected.sold_at) : ""}
            </DialogDescription>
          </DialogHeader>
          {selected && (
            <div className="space-y-3 text-sm">
              <p>
                <span className="text-muted-foreground">Cliente: </span>
                {selected.customers.name}
              </p>
              <p>
                <span className="text-muted-foreground">Pago: </span>
                {formatPaymentMethod(selected.payment_method)}
              </p>
              <div className="space-y-2">
                {selected.sale_items.map((item) => (
                  <div
                    key={item.id}
                    className="flex items-center justify-between gap-2 rounded-xl border border-border/60 px-3 py-2"
                  >
                    <div>
                      <p className="font-medium">{item.name_snapshot}</p>
                      <Badge variant="outline">
                        {formatSaleItemType(item.item_type)}
                      </Badge>
                    </div>
                    <p>
                      {item.quantity} × {formatCurrency(Number(item.unit_price))}
                    </p>
                  </div>
                ))}
              </div>
              <p className="text-right text-lg font-semibold">
                Total {formatCurrency(Number(selected.total))}
              </p>
            </div>
          )}
        </DialogContent>
      </Dialog>
    </div>
  );
}
