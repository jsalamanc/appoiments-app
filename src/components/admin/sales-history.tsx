"use client";

import Link from "next/link";
import { useMemo, useState } from "react";

import { FadeIn, StaggerChildren, StaggerItem } from "@/components/motion/fade-in";
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
import type { PaymentMethod, SaleWithDetails } from "@/types/database";
import { ChevronLeft, ChevronRight } from "lucide-react";

const PATHNAME = "/admin/sales/history";

type SalesHistoryProps = {
  sales: SaleWithDetails[];
  page: number;
  totalPages: number;
  total: number;
  pageSize: number;
  paymentMethod: PaymentMethod | "all";
  highlightId?: string;
};

const FILTERS: { value: PaymentMethod | "all"; label: string }[] = [
  { value: "all", label: "Todos" },
  { value: "cash", label: "Efectivo" },
  { value: "transfer", label: "Transferencia" },
];

function paymentHref(method: PaymentMethod | "all", page = 1) {
  const params = new URLSearchParams();
  if (method !== "all") params.set("payment", method);
  if (page > 1) params.set("page", String(page));
  const qs = params.toString();
  return qs ? `${PATHNAME}?${qs}` : PATHNAME;
}

export function SalesHistory({
  sales,
  page,
  totalPages,
  total,
  pageSize,
  paymentMethod,
  highlightId,
}: SalesHistoryProps) {
  const [selected, setSelected] = useState<SaleWithDetails | null>(
    () => sales.find((sale) => sale.id === highlightId) ?? null
  );

  const from = total === 0 ? 0 : (page - 1) * pageSize + 1;
  const to = Math.min(page * pageSize, total);

  const highlighted = useMemo(
    () => new Set(highlightId ? [highlightId] : []),
    [highlightId]
  );

  return (
    <div className="mx-auto max-w-5xl space-y-6">
      <FadeIn className="flex flex-col gap-4 sm:flex-row sm:items-end sm:justify-between">
        <div>
          <p className="text-sm text-muted-foreground">Ventas</p>
          <h1 className="font-heading text-4xl font-semibold">Historial</h1>
          <p className="mt-1 text-muted-foreground">
            Consulta las transacciones registradas en tu centro.
          </p>
        </div>
        <Button render={<Link href="/admin/sales" />}>Nueva venta</Button>
      </FadeIn>

      <FadeIn delay={0.05} className="flex flex-wrap gap-2">
        {FILTERS.map((filter) => {
          const active = paymentMethod === filter.value;
          return (
            <Link
              key={filter.value}
              href={paymentHref(filter.value)}
              className={`rounded-full px-3 py-1.5 text-sm transition ${
                active
                  ? "bg-primary text-primary-foreground"
                  : "bg-muted/70 text-muted-foreground hover:bg-muted hover:text-foreground"
              }`}
            >
              {filter.label}
            </Link>
          );
        })}
      </FadeIn>

      {sales.length === 0 ? (
        <FadeIn className="rounded-3xl border border-dashed border-border bg-card/50 px-6 py-16 text-center">
          <h3 className="font-heading text-2xl font-semibold">
            Sin transacciones
          </h3>
          <p className="mt-2 text-muted-foreground">
            Cuando registres ventas aparecerán aquí.
          </p>
        </FadeIn>
      ) : (
        <>
          <StaggerChildren className="space-y-3">
            {sales.map((sale) => (
              <StaggerItem key={sale.id}>
                <button
                  type="button"
                  onClick={() => setSelected(sale)}
                  className={`w-full rounded-3xl border px-4 py-4 text-left transition hover:border-primary/40 hover:bg-card ${
                    highlighted.has(sale.id)
                      ? "border-primary bg-primary/5"
                      : "border-border/70 bg-card/70"
                  }`}
                >
                  <div className="flex flex-col gap-3 sm:flex-row sm:items-center sm:justify-between">
                    <div>
                      <p className="font-heading text-xl font-semibold">
                        {sale.customers?.name ?? "Cliente"}
                      </p>
                      <p className="text-sm text-muted-foreground">
                        {sale.customers?.email} · {formatDateTime(sale.sold_at)}
                      </p>
                    </div>
                    <div className="flex items-center gap-3">
                      <Badge variant="secondary">
                        {formatPaymentMethod(sale.payment_method)}
                      </Badge>
                      <p className="font-heading text-2xl font-semibold">
                        {formatCurrency(Number(sale.total))}
                      </p>
                    </div>
                  </div>
                </button>
              </StaggerItem>
            ))}
          </StaggerChildren>

          <div className="flex flex-col gap-3 rounded-2xl border border-border/70 bg-card/50 px-4 py-3 sm:flex-row sm:items-center sm:justify-between">
            <p className="text-sm text-muted-foreground">
              Mostrando {from}–{to} de {total}
            </p>
            <div className="flex items-center gap-2">
              {page <= 1 ? (
                <Button variant="outline" size="sm" disabled>
                  <ChevronLeft data-icon="inline-start" />
                  Anterior
                </Button>
              ) : (
                <Button
                  variant="outline"
                  size="sm"
                  render={
                    <Link href={paymentHref(paymentMethod, page - 1)} />
                  }
                >
                  <ChevronLeft data-icon="inline-start" />
                  Anterior
                </Button>
              )}
              <span className="min-w-16 text-center text-sm text-muted-foreground">
                {page} / {totalPages}
              </span>
              {page >= totalPages ? (
                <Button variant="outline" size="sm" disabled>
                  Siguiente
                  <ChevronRight data-icon="inline-end" />
                </Button>
              ) : (
                <Button
                  variant="outline"
                  size="sm"
                  render={
                    <Link href={paymentHref(paymentMethod, page + 1)} />
                  }
                >
                  Siguiente
                  <ChevronRight data-icon="inline-end" />
                </Button>
              )}
            </div>
          </div>
        </>
      )}

      <Dialog
        open={!!selected}
        onOpenChange={(open) => !open && setSelected(null)}
      >
        <DialogContent className="max-h-[90vh] overflow-y-auto sm:max-w-lg">
          <DialogHeader>
            <DialogTitle>Detalle de venta</DialogTitle>
            <DialogDescription>
              {selected
                ? `${selected.customers?.name} · ${formatDateTime(selected.sold_at)}`
                : ""}
            </DialogDescription>
          </DialogHeader>
          {selected && (
            <div className="space-y-4">
              <div className="grid gap-2 text-sm">
                <p>
                  <span className="text-muted-foreground">Documento: </span>
                  {selected.customers?.document}
                </p>
                <p>
                  <span className="text-muted-foreground">Correo: </span>
                  {selected.customers?.email}
                </p>
                <p>
                  <span className="text-muted-foreground">Teléfono: </span>
                  {selected.customers?.phone}
                </p>
                <p>
                  <span className="text-muted-foreground">Pago: </span>
                  {formatPaymentMethod(selected.payment_method)}
                </p>
              </div>
              <div className="space-y-2">
                {selected.sale_items.map((item) => (
                  <div
                    key={item.id}
                    className="flex items-start justify-between gap-3 rounded-2xl border border-border/60 px-3 py-2"
                  >
                    <div>
                      <p className="font-medium">{item.name_snapshot}</p>
                      <p className="text-xs text-muted-foreground">
                        {formatSaleItemType(item.item_type)} ·{" "}
                        {formatCurrency(Number(item.unit_price))} ×{" "}
                        {item.quantity}
                      </p>
                    </div>
                    <p className="font-medium">
                      {formatCurrency(Number(item.line_total))}
                    </p>
                  </div>
                ))}
              </div>
              <div className="flex items-center justify-between rounded-2xl bg-mint/40 px-4 py-3">
                <span>Total</span>
                <span className="font-heading text-2xl font-semibold">
                  {formatCurrency(Number(selected.total))}
                </span>
              </div>
            </div>
          )}
        </DialogContent>
      </Dialog>
    </div>
  );
}
