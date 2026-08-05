"use client";

import { CalendarDays, Receipt } from "lucide-react";
import Link from "next/link";
import { useRouter } from "next/navigation";
import { useCallback, useEffect, useState } from "react";
import { toast } from "sonner";

import { EmptyState } from "@/components/admin/empty-state";
import { FadeIn } from "@/components/motion/fade-in";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import {
  endOfDay,
  formatAppointmentStatus,
  listAppointmentsInRange,
  setAppointmentStatus,
  startOfDay,
} from "@/lib/appointments";
import { formatCurrency, formatDateTime, formatDuration } from "@/lib/format";
import type {
  AppointmentStatus,
  AppointmentWithDetails,
} from "@/types/database";

export function AppointmentsToday() {
  const router = useRouter();
  const [appointments, setAppointments] = useState<AppointmentWithDetails[]>(
    []
  );
  const [loading, setLoading] = useState(true);
  const [busyId, setBusyId] = useState<string | null>(null);

  const load = useCallback(async () => {
    setLoading(true);
    try {
      const now = new Date();
      const items = await listAppointmentsInRange({
        from: startOfDay(now).toISOString(),
        to: endOfDay(now).toISOString(),
      });
      setAppointments(items);
    } catch (err) {
      toast.error(err instanceof Error ? err.message : "Error al cargar citas");
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    void load();
  }, [load]);

  const changeStatus = async (
    appointment: AppointmentWithDetails,
    status: AppointmentStatus
  ) => {
    setBusyId(appointment.id);
    try {
      await setAppointmentStatus(appointment.id, status);
      toast.success(`Cita marcada como ${formatAppointmentStatus(status)}`);
      await load();
      router.refresh();
    } catch (err) {
      toast.error(err instanceof Error ? err.message : "No se pudo actualizar");
    } finally {
      setBusyId(null);
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
          <p className="text-sm text-muted-foreground">Agenda</p>
          <h1 className="font-heading text-4xl font-semibold">Hoy</h1>
          <p className="mt-1 capitalize text-muted-foreground">{todayLabel}</p>
        </div>
        <Button
          variant="outline"
          render={<Link href="/admin/appointments" />}
        >
          <CalendarDays data-icon="inline-start" />
          Ver semana
        </Button>
      </FadeIn>

      {loading ? (
        <p className="text-sm text-muted-foreground">Cargando citas de hoy...</p>
      ) : appointments.length === 0 ? (
        <EmptyState
          title="No hay citas para hoy"
          description="Cuando agenda citas para hoy, aparecerán aquí con acciones rápidas."
          actionLabel="Ir al calendario"
          onAction={() => router.push("/admin/appointments")}
        />
      ) : (
        <FadeIn delay={0.05} className="space-y-3">
          {appointments.map((appointment) => {
            const busy = busyId === appointment.id;
            const scheduled = appointment.status === "scheduled";
            return (
              <article
                key={appointment.id}
                className="rounded-3xl border border-border/70 bg-card/70 p-4"
              >
                <div className="flex flex-col gap-3 sm:flex-row sm:items-start sm:justify-between">
                  <div className="space-y-1">
                    <div className="flex flex-wrap items-center gap-2">
                      <p className="font-heading text-xl font-semibold">
                        {appointment.customers.name}
                      </p>
                      <Badge variant="secondary">
                        {formatAppointmentStatus(appointment.status)}
                      </Badge>
                    </div>
                    <p className="text-sm text-muted-foreground">
                      {formatDateTime(appointment.starts_at)} ·{" "}
                      {appointment.services.name} (
                      {formatDuration(appointment.services.duration_minutes)})
                    </p>
                    <p className="text-sm text-muted-foreground">
                      {appointment.employees.name} ·{" "}
                      {formatCurrency(Number(appointment.services.price))}
                    </p>
                    {appointment.notes && (
                      <p className="text-xs text-muted-foreground">
                        {appointment.notes}
                      </p>
                    )}
                  </div>

                  {scheduled && (
                    <div className="flex flex-col gap-2 sm:min-w-40">
                      <Button
                        size="sm"
                        onClick={() =>
                          router.push(
                            `/admin/sales?customerId=${appointment.customer_id}&serviceId=${appointment.service_id}&appointmentId=${appointment.id}`
                          )
                        }
                      >
                        <Receipt data-icon="inline-start" />
                        Cobrar
                      </Button>
                      <div className="grid grid-cols-2 gap-2">
                        <Button
                          size="sm"
                          variant="secondary"
                          disabled={busy}
                          onClick={() =>
                            void changeStatus(appointment, "no_show")
                          }
                        >
                          No asistió
                        </Button>
                        <Button
                          size="sm"
                          variant="ghost"
                          disabled={busy}
                          onClick={() =>
                            void changeStatus(appointment, "cancelled")
                          }
                        >
                          Cancelar
                        </Button>
                      </div>
                    </div>
                  )}
                </div>
              </article>
            );
          })}
        </FadeIn>
      )}
    </div>
  );
}
