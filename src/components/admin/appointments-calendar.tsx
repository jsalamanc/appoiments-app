"use client";

import {
  ChevronLeft,
  ChevronRight,
  Plus,
  Receipt,
  Users,
} from "lucide-react";
import Link from "next/link";
import { useRouter } from "next/navigation";
import { useCallback, useEffect, useMemo, useState } from "react";
import { toast } from "sonner";

import { AppointmentFormDialog } from "@/components/admin/appointment-form-dialog";
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
  addDays,
  endOfWeek,
  formatAppointmentStatus,
  isPastDay,
  isPastSlot,
  listAppointmentsInRange,
  nextBookableSlot,
  setAppointmentStatus,
  startOfWeek,
} from "@/lib/appointments";
import { getCurrentCenter } from "@/lib/catalog";
import { listEmployees } from "@/lib/employees";
import { formatCurrency, formatDateTime, formatDuration } from "@/lib/format";
import { isModuleEnabled, parseCenterModules } from "@/lib/modules";
import { createClient } from "@/lib/supabase/client";
import { cn } from "@/lib/utils";
import type {
  AppointmentStatus,
  AppointmentWithDetails,
  Employee,
  Service,
} from "@/types/database";

const DAY_START_HOUR = 8;
const DAY_END_HOUR = 20;
const HOUR_HEIGHT = 64;
const HOURS = Array.from(
  { length: DAY_END_HOUR - DAY_START_HOUR },
  (_, i) => DAY_START_HOUR + i
);

function dayLabel(date: Date) {
  return new Intl.DateTimeFormat("es-CO", {
    weekday: "short",
    day: "numeric",
    month: "short",
  }).format(date);
}

function weekRangeLabel(weekStart: Date) {
  const weekEnd = addDays(weekStart, 6);
  const fmt = new Intl.DateTimeFormat("es-CO", {
    day: "numeric",
    month: "short",
  });
  return `${fmt.format(weekStart)} – ${fmt.format(weekEnd)}`;
}

function appointmentTop(startsAt: string) {
  const d = new Date(startsAt);
  const minutes = d.getHours() * 60 + d.getMinutes();
  const startMinutes = DAY_START_HOUR * 60;
  return ((minutes - startMinutes) / 60) * HOUR_HEIGHT;
}

function appointmentHeight(startsAt: string, endsAt: string) {
  const start = new Date(startsAt).getTime();
  const end = new Date(endsAt).getTime();
  const hours = Math.max((end - start) / (1000 * 60 * 60), 0.5);
  return hours * HOUR_HEIGHT;
}

export function AppointmentsCalendar() {
  const router = useRouter();
  const [weekStart, setWeekStart] = useState(() => startOfWeek(new Date()));
  const [employeeFilter, setEmployeeFilter] = useState<string>("all");
  const [employees, setEmployees] = useState<Employee[]>([]);
  const [services, setServices] = useState<Service[]>([]);
  const [appointments, setAppointments] = useState<AppointmentWithDetails[]>(
    []
  );
  const [loading, setLoading] = useState(true);
  const [formOpen, setFormOpen] = useState(false);
  const [selected, setSelected] = useState<AppointmentWithDetails | null>(null);
  const [defaultStartsAt, setDefaultStartsAt] = useState<Date | null>(null);
  const [detail, setDetail] = useState<AppointmentWithDetails | null>(null);
  const [statusLoading, setStatusLoading] = useState(false);
  const [servicesModuleEnabled, setServicesModuleEnabled] = useState(true);

  const days = useMemo(
    () => Array.from({ length: 7 }, (_, i) => addDays(weekStart, i)),
    [weekStart]
  );

  const load = useCallback(async () => {
    setLoading(true);
    try {
      const supabase = createClient();
      const center = await getCurrentCenter();
      const modules = parseCenterModules(center?.modules);
      const servicesOn = isModuleEnabled(modules, "services");
      setServicesModuleEnabled(servicesOn);

      const [emps, servicesRes, appts] = await Promise.all([
        listEmployees(),
        servicesOn
          ? supabase
              .from("services")
              .select("*")
              .eq("status", "published")
              .order("name")
          : Promise.resolve({ data: [] as Service[], error: null }),
        listAppointmentsInRange({
          from: weekStart.toISOString(),
          to: endOfWeek(weekStart).toISOString(),
          employeeId: employeeFilter === "all" ? null : employeeFilter,
        }),
      ]);

      if (servicesRes.error) throw servicesRes.error;
      setEmployees(emps);
      setServices((servicesRes.data ?? []) as Service[]);
      setAppointments(appts);
    } catch (err) {
      toast.error(err instanceof Error ? err.message : "Error al cargar agenda");
    } finally {
      setLoading(false);
    }
  }, [weekStart, employeeFilter]);

  useEffect(() => {
    void load();
  }, [load]);

  const activeEmployees = employees.filter((e) => e.active);

  const openCreate = (day: Date, hour: number) => {
    if (isPastSlot(day, hour)) {
      toast.error("No puedes agendar en horarios pasados");
      return;
    }
    const start = new Date(day);
    start.setHours(hour, 0, 0, 0);
    setSelected(null);
    setDefaultStartsAt(start);
    setFormOpen(true);
  };

  const openEdit = (appointment: AppointmentWithDetails) => {
    setDetail(null);
    setSelected(appointment);
    setDefaultStartsAt(null);
    setFormOpen(true);
  };

  const changeStatus = async (status: AppointmentStatus) => {
    if (!detail) return;
    setStatusLoading(true);
    try {
      await setAppointmentStatus(detail.id, status);
      toast.success(`Cita marcada como ${formatAppointmentStatus(status)}`);
      setDetail(null);
      await load();
    } catch (err) {
      toast.error(err instanceof Error ? err.message : "No se pudo actualizar");
    } finally {
      setStatusLoading(false);
    }
  };

  const appointmentsForDay = (day: Date) => {
    const dayKey = day.toDateString();
    return appointments.filter(
      (a) => new Date(a.starts_at).toDateString() === dayKey
    );
  };

  return (
    <div className="mx-auto max-w-6xl space-y-6">
      <FadeIn className="flex flex-col gap-4 lg:flex-row lg:items-end lg:justify-between">
        <div>
          <p className="text-sm text-muted-foreground">Agenda</p>
          <h1 className="font-heading text-4xl font-semibold">Citas</h1>
          <p className="mt-1 text-muted-foreground">
            Agenda por empleado y cobra al finalizar con un clic.
          </p>
        </div>
        <Button
          onClick={() => {
            setSelected(null);
            setDefaultStartsAt(nextBookableSlot());
            setFormOpen(true);
          }}
          disabled={
            activeEmployees.length === 0 ||
            services.length === 0 ||
            !servicesModuleEnabled
          }
        >
          <Plus data-icon="inline-start" />
          Nueva cita
        </Button>
      </FadeIn>

      {activeEmployees.length === 0 && (
        <FadeIn className="rounded-3xl border border-dashed border-border bg-card/50 p-5 text-sm text-muted-foreground">
          Primero crea un{" "}
          <Link href="/admin/employees" className="underline">
            empleado
          </Link>{" "}
          para poder agendar.
        </FadeIn>
      )}

      {!servicesModuleEnabled && (
        <FadeIn className="rounded-3xl border border-dashed border-border bg-card/50 p-5 text-sm text-muted-foreground">
          El módulo de Servicios está apagado.{" "}
          <Link href="/admin/settings" className="underline">
            Actívalo en Ajustes
          </Link>{" "}
          para poder agendar citas.
        </FadeIn>
      )}

      {servicesModuleEnabled && services.length === 0 && (
        <FadeIn className="rounded-3xl border border-dashed border-border bg-card/50 p-5 text-sm text-muted-foreground">
          Publica al menos un{" "}
          <Link href="/admin/services" className="underline">
            servicio
          </Link>{" "}
          para agendar citas.
        </FadeIn>
      )}

      <FadeIn
        delay={0.05}
        className="flex flex-col gap-3 rounded-3xl border border-border/70 bg-card/70 p-4 sm:flex-row sm:items-center sm:justify-between"
      >
        <div className="flex items-center gap-2">
          <Button
            type="button"
            variant="outline"
            size="icon"
            onClick={() => setWeekStart((d) => addDays(d, -7))}
          >
            <ChevronLeft />
          </Button>
          <Button
            type="button"
            variant="outline"
            size="sm"
            onClick={() => setWeekStart(startOfWeek(new Date()))}
          >
            Hoy
          </Button>
          <Button
            type="button"
            variant="outline"
            size="icon"
            onClick={() => setWeekStart((d) => addDays(d, 7))}
          >
            <ChevronRight />
          </Button>
          <p className="ml-2 text-sm font-medium capitalize">
            {weekRangeLabel(weekStart)}
          </p>
        </div>

        <div className="flex flex-wrap items-center gap-2">
          <Users className="size-4 text-muted-foreground" />
          <select
            className="h-9 rounded-xl border border-input bg-transparent px-3 text-sm"
            value={employeeFilter}
            onChange={(e) => setEmployeeFilter(e.target.value)}
          >
            <option value="all">Todos los empleados</option>
            {employees.map((employee) => (
              <option key={employee.id} value={employee.id}>
                {employee.name}
                {!employee.active ? " (inactivo)" : ""}
              </option>
            ))}
          </select>
        </div>
      </FadeIn>

      <FadeIn delay={0.08} className="overflow-hidden rounded-3xl border border-border/70 bg-card/60">
        {loading ? (
          <p className="p-6 text-sm text-muted-foreground">Cargando agenda...</p>
        ) : (
          <div className="overflow-x-auto">
            <div
              className="grid min-w-[840px]"
              style={{
                gridTemplateColumns: `3.5rem repeat(7, minmax(0, 1fr))`,
              }}
            >
              <div className="border-b border-border/60" />
              {days.map((day) => {
                const pastDay = isPastDay(day);
                return (
                  <div
                    key={day.toISOString()}
                    className={cn(
                      "border-b border-l border-border/60 px-2 py-3 text-center",
                      pastDay && "bg-muted/40"
                    )}
                  >
                    <p
                      className={cn(
                        "text-xs font-medium capitalize",
                        pastDay
                          ? "text-muted-foreground/60 line-through"
                          : "text-muted-foreground"
                      )}
                    >
                      {dayLabel(day)}
                    </p>
                    {pastDay && (
                      <p className="mt-0.5 text-[10px] text-muted-foreground/70">
                        Pasado
                      </p>
                    )}
                  </div>
                );
              })}

              <div className="relative">
                {HOURS.map((hour) => (
                  <div
                    key={hour}
                    className="border-b border-border/40 pr-2 text-right text-[11px] text-muted-foreground"
                    style={{ height: HOUR_HEIGHT }}
                  >
                    {String(hour).padStart(2, "0")}:00
                  </div>
                ))}
              </div>

              {days.map((day) => {
                const dayAppts = appointmentsForDay(day);
                const pastDay = isPastDay(day);
                return (
                  <div
                    key={`col-${day.toISOString()}`}
                    className={cn(
                      "relative border-l border-border/60",
                      pastDay && "bg-muted/25"
                    )}
                    style={{ height: HOURS.length * HOUR_HEIGHT }}
                  >
                    {HOURS.map((hour) => {
                      const pastSlot = isPastSlot(day, hour);
                      return (
                        <button
                          key={hour}
                          type="button"
                          disabled={pastSlot}
                          className={cn(
                            "absolute right-0 left-0 border-b border-border/30 transition",
                            pastSlot
                              ? "cursor-not-allowed bg-muted/30"
                              : "hover:bg-muted/40"
                          )}
                          style={{
                            top: (hour - DAY_START_HOUR) * HOUR_HEIGHT,
                            height: HOUR_HEIGHT,
                          }}
                          onClick={() => openCreate(day, hour)}
                          aria-label={
                            pastSlot
                              ? `Horario pasado ${dayLabel(day)} ${hour}:00`
                              : `Agendar ${dayLabel(day)} ${hour}:00`
                          }
                        />
                      );
                    })}

                    {dayAppts.map((appointment) => {
                      const top = appointmentTop(appointment.starts_at);
                      const height = appointmentHeight(
                        appointment.starts_at,
                        appointment.ends_at
                      );
                      const muted = appointment.status !== "scheduled";
                      return (
                        <button
                          key={appointment.id}
                          type="button"
                          className={cn(
                            "absolute right-1 left-1 z-10 overflow-hidden rounded-lg border px-1.5 py-1 text-left shadow-sm transition hover:brightness-95",
                            muted && "opacity-55"
                          )}
                          style={{
                            top: Math.max(top, 0),
                            height: Math.max(height - 2, 28),
                            backgroundColor: `${appointment.employees.color}33`,
                            borderColor: appointment.employees.color,
                          }}
                          onClick={(e) => {
                            e.stopPropagation();
                            setDetail(appointment);
                          }}
                        >
                          <p className="truncate text-[11px] font-semibold leading-tight">
                            {appointment.customers.name}
                          </p>
                          <p className="truncate text-[10px] text-muted-foreground">
                            {appointment.services.name}
                          </p>
                        </button>
                      );
                    })}
                  </div>
                );
              })}
            </div>
          </div>
        )}
      </FadeIn>

      <AppointmentFormDialog
        open={formOpen}
        onOpenChange={setFormOpen}
        appointment={selected}
        defaultStartsAt={defaultStartsAt}
        defaultEmployeeId={
          employeeFilter !== "all" ? employeeFilter : activeEmployees[0]?.id
        }
        employees={employees}
        services={services}
        onSuccess={() => void load()}
      />

      <Dialog
        open={!!detail}
        onOpenChange={(open) => !open && setDetail(null)}
      >
        <DialogContent className="sm:max-w-md">
          <DialogHeader>
            <DialogTitle>Detalle de cita</DialogTitle>
            <DialogDescription>
              {detail ? formatDateTime(detail.starts_at) : ""}
            </DialogDescription>
          </DialogHeader>

          {detail && (
            <div className="space-y-4">
              <div className="space-y-1 text-sm">
                <p>
                  <span className="text-muted-foreground">Cliente: </span>
                  {detail.customers.name}
                </p>
                <p>
                  <span className="text-muted-foreground">Servicio: </span>
                  {detail.services.name} ·{" "}
                  {formatCurrency(Number(detail.services.price))} ·{" "}
                  {formatDuration(detail.services.duration_minutes)}
                </p>
                <p>
                  <span className="text-muted-foreground">Empleado: </span>
                  {detail.employees.name}
                </p>
                {detail.notes && (
                  <p>
                    <span className="text-muted-foreground">Notas: </span>
                    {detail.notes}
                  </p>
                )}
                <div className="pt-1">
                  <Badge variant="secondary">
                    {formatAppointmentStatus(detail.status)}
                  </Badge>
                </div>
              </div>

              <div className="flex flex-col gap-2">
                {detail.status === "scheduled" && (
                  <>
                    <Button
                      onClick={() => {
                        router.push(
                          `/admin/sales?customerId=${detail.customer_id}&serviceId=${detail.service_id}&appointmentId=${detail.id}`
                        );
                      }}
                    >
                      <Receipt data-icon="inline-start" />
                      Cobrar
                    </Button>
                    <Button
                      variant="outline"
                      disabled={new Date(detail.starts_at).getTime() < Date.now()}
                      onClick={() => openEdit(detail)}
                    >
                      Editar
                    </Button>
                    <div className="grid grid-cols-2 gap-2">
                      <Button
                        variant="secondary"
                        disabled={statusLoading}
                        onClick={() => void changeStatus("no_show")}
                      >
                        No asistió
                      </Button>
                      <Button
                        variant="ghost"
                        disabled={statusLoading}
                        onClick={() => void changeStatus("cancelled")}
                      >
                        Cancelar cita
                      </Button>
                    </div>
                  </>
                )}
                {detail.status !== "scheduled" && (
                  <Button variant="outline" onClick={() => setDetail(null)}>
                    Cerrar
                  </Button>
                )}
              </div>
            </div>
          )}
        </DialogContent>
      </Dialog>
    </div>
  );
}
