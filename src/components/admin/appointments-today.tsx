"use client";

import {
  CalendarDays,
  ChevronLeft,
  ChevronRight,
  Receipt,
  Search,
} from "lucide-react";
import Link from "next/link";
import { useRouter } from "next/navigation";
import { useCallback, useEffect, useMemo, useState } from "react";
import { toast } from "sonner";

import { EmptyState } from "@/components/admin/empty-state";
import { FadeIn } from "@/components/motion/fade-in";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import {
  endOfDay,
  formatAppointmentStatus,
  getAppointmentServiceDurationMinutes,
  getAppointmentServiceName,
  getAppointmentServicePrice,
  listAppointmentsInRange,
  setAppointmentStatus,
  startOfDay,
} from "@/lib/appointments";
import { formatCurrency, formatDateTime, formatDuration } from "@/lib/format";
import { CATALOG_PAGE_SIZE, getTotalPages } from "@/lib/pagination";
import type {
  AppointmentStatus,
  AppointmentWithDetails,
} from "@/types/database";

function normalizeDocument(value: string) {
  return value.toLowerCase().replace(/[\s.\-]/g, "");
}

export function AppointmentsToday() {
  const router = useRouter();
  const [appointments, setAppointments] = useState<AppointmentWithDetails[]>(
    []
  );
  const [loading, setLoading] = useState(true);
  const [busyId, setBusyId] = useState<string | null>(null);
  const [documentQuery, setDocumentQuery] = useState("");
  const [page, setPage] = useState(1);

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

  const filtered = useMemo(() => {
    const q = normalizeDocument(documentQuery.trim());
    if (!q) return appointments;
    return appointments.filter((appointment) =>
      normalizeDocument(appointment.customers.document).includes(q)
    );
  }, [appointments, documentQuery]);

  const total = filtered.length;
  const totalPages = getTotalPages(total, CATALOG_PAGE_SIZE);
  const safePage = Math.min(page, totalPages);

  useEffect(() => {
    setPage(1);
  }, [documentQuery]);

  useEffect(() => {
    if (page > totalPages) setPage(totalPages);
  }, [page, totalPages]);

  const pageItems = useMemo(() => {
    const from = (safePage - 1) * CATALOG_PAGE_SIZE;
    return filtered.slice(from, from + CATALOG_PAGE_SIZE);
  }, [filtered, safePage]);

  const rangeFrom = total === 0 ? 0 : (safePage - 1) * CATALOG_PAGE_SIZE + 1;
  const rangeTo = Math.min(safePage * CATALOG_PAGE_SIZE, total);
  const hasDocumentFilter = documentQuery.trim().length > 0;

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

      <FadeIn delay={0.04}>
        <div className="relative">
          <Search className="pointer-events-none absolute top-1/2 left-2.5 size-4 -translate-y-1/2 text-muted-foreground" />
          <Input
            value={documentQuery}
            onChange={(e) => setDocumentQuery(e.target.value)}
            placeholder="Buscar reserva por documento del cliente..."
            className="pl-8"
            autoComplete="off"
          />
        </div>
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
      ) : total === 0 ? (
        <EmptyState
          title="Sin reservas con ese documento"
          description="Revisa el número o limpia la búsqueda para ver todas las citas de hoy."
          actionLabel="Limpiar búsqueda"
          onAction={() => setDocumentQuery("")}
        />
      ) : (
        <>
          <FadeIn delay={0.05} className="space-y-3">
            {hasDocumentFilter && (
              <p className="text-sm text-muted-foreground">
                {total} reserva{total === 1 ? "" : "s"} con documento coincidente
              </p>
            )}
            {pageItems.map((appointment) => {
              const busy = busyId === appointment.id;
              const scheduled = appointment.status === "scheduled";
              return (
                <article
                  key={appointment.id}
                  className="rounded-3xl border border-border/70 bg-card shadow-sm p-4"
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
                        Doc: {appointment.customers.document}
                      </p>
                      <p className="text-sm text-muted-foreground">
                        {formatDateTime(appointment.starts_at)} ·{" "}
                        {getAppointmentServiceName(appointment)} (
                        {formatDuration(
                          getAppointmentServiceDurationMinutes(appointment)
                        )}
                        )
                      </p>
                      <p className="text-sm text-muted-foreground">
                        {appointment.employees.name}
                        {getAppointmentServicePrice(appointment) != null && (
                          <>
                            {" "}
                            ·{" "}
                            {formatCurrency(
                              getAppointmentServicePrice(appointment)!
                            )}
                          </>
                        )}
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
                          onClick={() => {
                            const params = new URLSearchParams({
                              customerId: appointment.customer_id,
                              appointmentId: appointment.id,
                            });
                            if (appointment.service_id) {
                              params.set("serviceId", appointment.service_id);
                            }
                            router.push(`/admin/sales?${params.toString()}`);
                          }}
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

          {total > 0 && (
            <FadeIn
              delay={0.08}
              className="flex flex-col gap-3 rounded-2xl border border-border/60 bg-muted/20 px-4 py-3 sm:flex-row sm:items-center sm:justify-between"
            >
              <p className="text-sm text-muted-foreground">
                Mostrando {rangeFrom}–{rangeTo} de {total}
              </p>
              <div className="flex items-center gap-2">
                <Button
                  variant="outline"
                  size="sm"
                  disabled={safePage <= 1}
                  onClick={() => setPage((current) => Math.max(1, current - 1))}
                >
                  <ChevronLeft data-icon="inline-start" />
                  Anterior
                </Button>
                <span className="min-w-16 text-center text-sm text-muted-foreground">
                  {safePage} / {totalPages}
                </span>
                <Button
                  variant="outline"
                  size="sm"
                  disabled={safePage >= totalPages}
                  onClick={() =>
                    setPage((current) => Math.min(totalPages, current + 1))
                  }
                >
                  Siguiente
                  <ChevronRight data-icon="inline-end" />
                </Button>
              </div>
            </FadeIn>
          )}
        </>
      )}
    </div>
  );
}
