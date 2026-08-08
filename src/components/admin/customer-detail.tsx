"use client";

import { ArrowLeft, CalendarDays, Pencil, Receipt } from "lucide-react";
import Link from "next/link";
import { useRouter } from "next/navigation";
import { useEffect, useState } from "react";

import { CustomerFormDialog } from "@/components/admin/customer-form-dialog";
import { FadeIn } from "@/components/motion/fade-in";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import {
  formatAppointmentStatus,
  getAppointmentServiceName,
} from "@/lib/appointments";
import {
  formatCurrency,
  formatDateTime,
  formatPaymentMethod,
} from "@/lib/format";
import type {
  AppointmentWithDetails,
  Customer,
  SaleWithDetails,
} from "@/types/database";

type CustomerDetailProps = {
  customer: Customer;
  sales: SaleWithDetails[];
  appointments: AppointmentWithDetails[];
  appointmentsEnabled: boolean;
};

export function CustomerDetail({
  customer: initialCustomer,
  sales,
  appointments,
  appointmentsEnabled,
}: CustomerDetailProps) {
  const router = useRouter();
  const [customer, setCustomer] = useState(initialCustomer);
  const [editOpen, setEditOpen] = useState(false);

  useEffect(() => {
    setCustomer(initialCustomer);
  }, [initialCustomer]);

  return (
    <div className="mx-auto max-w-5xl space-y-6">
      <FadeIn className="space-y-4">
        <Button
          variant="ghost"
          size="sm"
          render={<Link href="/admin/customers" />}
        >
          <ArrowLeft data-icon="inline-start" />
          Volver a clientes
        </Button>
        <div className="flex flex-col gap-4 sm:flex-row sm:items-start sm:justify-between">
          <div>
            <p className="text-sm text-muted-foreground">Cliente</p>
            <h1 className="font-heading text-4xl font-semibold">
              {customer.name}
            </h1>
            <div className="mt-2 space-y-1 text-sm text-muted-foreground">
              <p>{customer.email}</p>
              <p>{customer.phone}</p>
              <p>Documento: {customer.document}</p>
            </div>
          </div>
          <Button variant="outline" onClick={() => setEditOpen(true)}>
            <Pencil data-icon="inline-start" />
            Editar datos
          </Button>
        </div>
      </FadeIn>

      <CustomerFormDialog
        open={editOpen}
        onOpenChange={setEditOpen}
        customer={customer}
        onSuccess={(updated) => {
          setCustomer(updated);
          router.refresh();
        }}
      />

      <FadeIn
        delay={0.05}
        className="rounded-3xl border border-border/70 bg-card/70 p-5"
      >
        <div className="mb-4 flex items-center gap-2">
          <Receipt className="size-4 text-muted-foreground" />
          <h2 className="font-heading text-2xl font-semibold">Ventas</h2>
        </div>
        {sales.length === 0 ? (
          <p className="text-sm text-muted-foreground">
            Este cliente aún no tiene ventas.
          </p>
        ) : (
          <div className="space-y-2">
            {sales.map((sale) => (
              <Link
                key={sale.id}
                href={`/admin/sales/history?highlight=${sale.id}`}
                className="flex flex-col gap-1 rounded-2xl border border-border/60 bg-background/50 px-4 py-3 transition hover:border-primary/40 sm:flex-row sm:items-center sm:justify-between"
              >
                <div>
                  <p className="font-medium">
                    {formatDateTime(sale.sold_at)}
                  </p>
                  <p className="text-sm text-muted-foreground">
                    {formatPaymentMethod(sale.payment_method)} ·{" "}
                    {sale.sale_items.length} ítem
                    {sale.sale_items.length === 1 ? "" : "s"}
                  </p>
                </div>
                <p className="font-semibold">
                  {formatCurrency(Number(sale.total))}
                </p>
              </Link>
            ))}
          </div>
        )}
      </FadeIn>

      {appointmentsEnabled && (
        <FadeIn
          delay={0.08}
          className="rounded-3xl border border-border/70 bg-card/70 p-5"
        >
          <div className="mb-4 flex items-center gap-2">
            <CalendarDays className="size-4 text-muted-foreground" />
            <h2 className="font-heading text-2xl font-semibold">Citas</h2>
          </div>
          {appointments.length === 0 ? (
            <p className="text-sm text-muted-foreground">
              Este cliente aún no tiene citas.
            </p>
          ) : (
            <div className="space-y-2">
              {appointments.map((appointment) => (
                <div
                  key={appointment.id}
                  className="flex flex-col gap-2 rounded-2xl border border-border/60 bg-background/50 px-4 py-3 sm:flex-row sm:items-center sm:justify-between"
                >
                  <div>
                    <p className="font-medium">
                      {formatDateTime(appointment.starts_at)}
                    </p>
                    <p className="text-sm text-muted-foreground">
                      {getAppointmentServiceName(appointment)} ·{" "}
                      {appointment.employees.name}
                    </p>
                  </div>
                  <Badge variant="secondary">
                    {formatAppointmentStatus(appointment.status)}
                  </Badge>
                </div>
              ))}
            </div>
          )}
        </FadeIn>
      )}
    </div>
  );
}
