"use client";

import { Banknote, History } from "lucide-react";
import Link from "next/link";
import { useRouter } from "next/navigation";
import { useState } from "react";
import { toast } from "sonner";

import { EmptyState } from "@/components/admin/empty-state";
import { FadeIn } from "@/components/motion/fade-in";
import {
  AlertDialog,
  AlertDialogAction,
  AlertDialogCancel,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogTitle,
} from "@/components/ui/alert-dialog";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import {
  formatCurrency,
  formatDateTime,
  formatPaymentMethod,
  formatSaleItemType,
} from "@/lib/format";
import { isOwnerRole } from "@/lib/roles";
import { voidSale } from "@/lib/sales";
import { useCurrentUser } from "@/lib/use-current-user";
import type { SaleWithDetails } from "@/types/database";

type CashDayProps = {
  sales: SaleWithDetails[];
  salesCount: number;
  totalRevenue: number;
  payments: {
    cashCount: number;
    transferCount: number;
    cardCount: number;
    cashRevenue: number;
    transferRevenue: number;
    cardRevenue: number;
  };
  onVoided?: () => void;
};

export function CashDay({
  sales,
  salesCount,
  totalRevenue,
  payments,
  onVoided,
}: CashDayProps) {
  const router = useRouter();
  const [selected, setSelected] = useState<SaleWithDetails | null>(null);
  const [voidReason, setVoidReason] = useState("");
  const [confirmingVoid, setConfirmingVoid] = useState(false);
  const [voiding, setVoiding] = useState(false);
  const { user } = useCurrentUser();
  const canVoid = isOwnerRole(user?.role);

  const handleVoid = async () => {
    if (!selected) return;
    setVoiding(true);
    try {
      await voidSale(selected.id, voidReason.trim() || undefined);
      toast.success("Venta anulada");
      setConfirmingVoid(false);
      setSelected(null);
      setVoidReason("");
      onVoided?.();
    } catch (err) {
      toast.error(err instanceof Error ? err.message : "No se pudo anular la venta");
    } finally {
      setVoiding(false);
    }
  };
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
        className="grid gap-4 sm:grid-cols-2 lg:grid-cols-5"
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
        <div className="rounded-3xl border border-border/70 bg-card shadow-sm p-5">
          <p className="text-sm text-muted-foreground">Tarjeta</p>
          <p className="font-heading text-3xl font-semibold">
            {formatCurrency(payments.cardRevenue)}
          </p>
          <p className="text-xs text-muted-foreground">
            {payments.cardCount} venta{payments.cardCount === 1 ? "" : "s"}
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
                <p className="flex items-center gap-2 font-medium">
                  {sale.customers.name}
                  {sale.voided_at && <Badge variant="destructive">Anulada</Badge>}
                </p>
                <p className="text-sm text-muted-foreground">
                  {formatDateTime(sale.sold_at)} ·{" "}
                  {formatPaymentMethod(sale.payment_method)}
                </p>
              </div>
              <p
                className={
                  sale.voided_at
                    ? "font-semibold text-muted-foreground line-through"
                    : "font-semibold"
                }
              >
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
              {selected.voided_at && (
                <div className="rounded-xl border border-destructive/30 bg-destructive/5 px-3 py-2 text-destructive">
                  <p className="font-medium">Venta anulada</p>
                  {selected.void_reason && (
                    <p className="text-xs">{selected.void_reason}</p>
                  )}
                </div>
              )}
              <p>
                <span className="text-muted-foreground">Cliente: </span>
                {selected.customers.name}
              </p>
              <p>
                <span className="text-muted-foreground">Pago: </span>
                {formatPaymentMethod(selected.payment_method)}
              </p>
              {selected.discount_amount > 0 && (
                <p>
                  <span className="text-muted-foreground">Descuento: </span>
                  {formatCurrency(selected.discount_amount)}
                </p>
              )}
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
              {canVoid && !selected.voided_at && (
                <Button
                  type="button"
                  variant="destructive"
                  className="w-full"
                  onClick={() => setConfirmingVoid(true)}
                >
                  Anular venta
                </Button>
              )}
            </div>
          )}
        </DialogContent>
      </Dialog>

      <AlertDialog open={confirmingVoid} onOpenChange={setConfirmingVoid}>
        <AlertDialogContent>
          <AlertDialogHeader>
            <AlertDialogTitle>¿Anular esta venta?</AlertDialogTitle>
            <AlertDialogDescription>
              El stock de los productos vendidos vuelve a quedar disponible y
              la venta deja de contar en caja e ingresos. Esta acción no se
              puede deshacer.
            </AlertDialogDescription>
          </AlertDialogHeader>
          <div className="space-y-2">
            <Label htmlFor="void-reason">Motivo (opcional)</Label>
            <Input
              id="void-reason"
              value={voidReason}
              onChange={(e) => setVoidReason(e.target.value)}
              placeholder="Ej. el cliente se arrepintió"
            />
          </div>
          <AlertDialogFooter>
            <AlertDialogCancel disabled={voiding}>Cancelar</AlertDialogCancel>
            <AlertDialogAction
              disabled={voiding}
              onClick={(e) => {
                e.preventDefault();
                void handleVoid();
              }}
            >
              {voiding ? "Anulando..." : "Anular venta"}
            </AlertDialogAction>
          </AlertDialogFooter>
        </AlertDialogContent>
      </AlertDialog>
    </div>
  );
}
