"use client";

import { useEffect, useMemo, useState } from "react";
import { toast } from "sonner";

import { FadeIn } from "@/components/motion/fade-in";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Textarea } from "@/components/ui/textarea";
import { formatCurrency, formatDateTime, formatDuration } from "@/lib/format";
import {
  buildPublicSlots,
  createPublicBooking,
  fetchPublicBookingCatalog,
  fetchPublicBusyRanges,
  upcomingBookingDays,
  type PublicBookingCatalog,
  type PublicEmployee,
  type PublicService,
} from "@/lib/public-booking";
import { cn } from "@/lib/utils";

type Step = "service" | "employee" | "slot" | "details" | "done";

export function PublicBooking({ slug }: { slug: string }) {
  const [catalog, setCatalog] = useState<PublicBookingCatalog | null>(null);
  const [loading, setLoading] = useState(true);
  const [step, setStep] = useState<Step>("service");
  const [service, setService] = useState<PublicService | null>(null);
  const [employee, setEmployee] = useState<PublicEmployee | null>(null);
  const [day, setDay] = useState<Date>(() => upcomingBookingDays(1)[0]);
  const [slots, setSlots] = useState<Date[]>([]);
  const [loadingSlots, setLoadingSlots] = useState(false);
  const [startsAt, setStartsAt] = useState<Date | null>(null);
  const [submitting, setSubmitting] = useState(false);
  const [name, setName] = useState("");
  const [email, setEmail] = useState("");
  const [phone, setPhone] = useState("");
  const [document, setDocument] = useState("");
  const [notes, setNotes] = useState("");

  const days = useMemo(() => upcomingBookingDays(14), []);

  useEffect(() => {
    let cancelled = false;
    (async () => {
      setLoading(true);
      try {
        const data = await fetchPublicBookingCatalog(slug);
        if (!cancelled) setCatalog(data);
      } catch (err) {
        if (!cancelled) {
          toast.error(
            err instanceof Error ? err.message : "No se pudo cargar el portal"
          );
        }
      } finally {
        if (!cancelled) setLoading(false);
      }
    })();
    return () => {
      cancelled = true;
    };
  }, [slug]);

  useEffect(() => {
    if (!employee || !service || step !== "slot") return;
    let cancelled = false;
    (async () => {
      setLoadingSlots(true);
      try {
        const busy = await fetchPublicBusyRanges(slug, employee.id, day);
        if (cancelled) return;
        setSlots(
          buildPublicSlots({
            day,
            durationMinutes: service.duration_minutes,
            busy,
          })
        );
      } catch (err) {
        if (!cancelled) {
          toast.error(
            err instanceof Error ? err.message : "No se pudieron cargar horarios"
          );
          setSlots([]);
        }
      } finally {
        if (!cancelled) setLoadingSlots(false);
      }
    })();
    return () => {
      cancelled = true;
    };
  }, [slug, employee, service, day, step]);

  const submit = async () => {
    if (!service || !employee || !startsAt) return;
    setSubmitting(true);
    try {
      await createPublicBooking({
        slug,
        employeeId: employee.id,
        serviceId: service.id,
        startsAt: startsAt.toISOString(),
        name,
        email,
        phone,
        document,
        notes,
      });
      setStep("done");
      toast.success("Reserva creada");
    } catch (err) {
      toast.error(err instanceof Error ? err.message : "No se pudo reservar");
    } finally {
      setSubmitting(false);
    }
  };

  if (loading) {
    return (
      <main className="mx-auto flex min-h-screen max-w-2xl items-center justify-center px-4">
        <p className="text-sm text-muted-foreground">Cargando reservas...</p>
      </main>
    );
  }

  if (!catalog) {
    return (
      <main className="mx-auto flex min-h-screen max-w-2xl items-center justify-center px-4">
        <div className="text-center">
          <h1 className="font-heading text-3xl font-semibold">Centro no encontrado</h1>
          <p className="mt-2 text-muted-foreground">
            Revisa el enlace de reserva con el centro.
          </p>
        </div>
      </main>
    );
  }

  if (!catalog.available) {
    return (
      <main className="mx-auto flex min-h-screen max-w-2xl items-center justify-center px-4">
        <div className="text-center">
          <p className="text-sm text-muted-foreground">{catalog.center.name}</p>
          <h1 className="mt-2 font-heading text-3xl font-semibold">
            Reservas no disponibles
          </h1>
          <p className="mt-2 text-muted-foreground">
            {catalog.reason ?? "Intenta más tarde o contacta al centro."}
          </p>
        </div>
      </main>
    );
  }

  if (catalog.services.length === 0 || catalog.employees.length === 0) {
    return (
      <main className="mx-auto flex min-h-screen max-w-2xl items-center justify-center px-4">
        <div className="text-center">
          <p className="text-sm text-muted-foreground">{catalog.center.name}</p>
          <h1 className="mt-2 font-heading text-3xl font-semibold">
            Aún no hay agenda pública
          </h1>
          <p className="mt-2 text-muted-foreground">
            El centro necesita servicios publicados y profesionales activos.
          </p>
        </div>
      </main>
    );
  }

  if (step === "done" && service && employee && startsAt) {
    return (
      <main className="mx-auto min-h-screen max-w-2xl px-4 py-12">
        <FadeIn className="rounded-3xl border border-border/70 bg-card/80 p-8 text-center">
          <p className="text-sm text-muted-foreground">{catalog.center.name}</p>
          <h1 className="mt-2 font-heading text-4xl font-semibold">
            ¡Reserva confirmada!
          </h1>
          <p className="mt-4 text-muted-foreground">
            {service.name} con {employee.name}
          </p>
          <p className="mt-1 font-medium">{formatDateTime(startsAt.toISOString())}</p>
          <p className="mt-6 text-sm text-muted-foreground">
            Te esperamos. Si necesitas cambiar la cita, contacta al centro.
          </p>
        </FadeIn>
      </main>
    );
  }

  return (
    <main className="mx-auto min-h-screen max-w-2xl px-4 py-10">
      <FadeIn>
        <p className="text-sm text-muted-foreground">Reservar en</p>
        <h1 className="font-heading text-4xl font-semibold">
          {catalog.center.name}
        </h1>
        <p className="mt-2 text-muted-foreground">
          Elige servicio, profesional y horario.
        </p>
      </FadeIn>

      <FadeIn delay={0.05} className="mt-8 space-y-6">
        {step === "service" && (
          <section className="space-y-3">
            <h2 className="font-heading text-2xl font-semibold">Servicio</h2>
            <ul className="space-y-2">
              {catalog.services.map((item) => (
                <li key={item.id}>
                  <button
                    type="button"
                    onClick={() => {
                      setService(item);
                      setEmployee(null);
                      setStartsAt(null);
                      setStep("employee");
                    }}
                    className="w-full rounded-3xl border border-border/70 bg-card/70 p-4 text-left transition hover:border-primary/40"
                  >
                    <p className="font-heading text-xl font-semibold">
                      {item.name}
                    </p>
                    <p className="text-sm text-muted-foreground">
                      {formatDuration(item.duration_minutes)} ·{" "}
                      {formatCurrency(Number(item.price))}
                    </p>
                  </button>
                </li>
              ))}
            </ul>
          </section>
        )}

        {step === "employee" && service && (
          <section className="space-y-3">
            <Button type="button" variant="ghost" size="sm" onClick={() => setStep("service")}>
              ← Cambiar servicio
            </Button>
            <h2 className="font-heading text-2xl font-semibold">Profesional</h2>
            <p className="text-sm text-muted-foreground">
              Para {service.name}
            </p>
            <ul className="space-y-2">
              {catalog.employees.map((item) => (
                <li key={item.id}>
                  <button
                    type="button"
                    onClick={() => {
                      setEmployee(item);
                      setStartsAt(null);
                      setStep("slot");
                    }}
                    className="flex w-full items-center gap-3 rounded-3xl border border-border/70 bg-card/70 p-4 text-left transition hover:border-primary/40"
                  >
                    <span
                      className="size-3 rounded-full"
                      style={{ backgroundColor: item.color }}
                    />
                    <span className="font-heading text-xl font-semibold">
                      {item.name}
                    </span>
                  </button>
                </li>
              ))}
            </ul>
          </section>
        )}

        {step === "slot" && service && employee && (
          <section className="space-y-4">
            <Button type="button" variant="ghost" size="sm" onClick={() => setStep("employee")}>
              ← Cambiar profesional
            </Button>
            <h2 className="font-heading text-2xl font-semibold">Horario</h2>
            <p className="text-sm text-muted-foreground">
              {service.name} · {employee.name}
            </p>

            <div className="flex gap-2 overflow-x-auto pb-1">
              {days.map((d) => {
                const active = d.toDateString() === day.toDateString();
                const label = new Intl.DateTimeFormat("es-CO", {
                  weekday: "short",
                  day: "numeric",
                  month: "short",
                }).format(d);
                return (
                  <button
                    key={d.toISOString()}
                    type="button"
                    onClick={() => {
                      setDay(d);
                      setStartsAt(null);
                    }}
                    className={cn(
                      "shrink-0 rounded-2xl border px-3 py-2 text-sm capitalize",
                      active
                        ? "border-primary bg-primary text-primary-foreground"
                        : "border-border/70 bg-card/70"
                    )}
                  >
                    {label}
                  </button>
                );
              })}
            </div>

            {loadingSlots ? (
              <p className="text-sm text-muted-foreground">Cargando horarios...</p>
            ) : slots.length === 0 ? (
              <p className="text-sm text-muted-foreground">
                No hay horarios libres este día.
              </p>
            ) : (
              <div className="grid grid-cols-3 gap-2 sm:grid-cols-4">
                {slots.map((slot) => {
                  const active = startsAt?.getTime() === slot.getTime();
                  const label = new Intl.DateTimeFormat("es-CO", {
                    hour: "2-digit",
                    minute: "2-digit",
                  }).format(slot);
                  return (
                    <button
                      key={slot.toISOString()}
                      type="button"
                      onClick={() => setStartsAt(slot)}
                      className={cn(
                        "rounded-2xl border px-3 py-2 text-sm",
                        active
                          ? "border-primary bg-primary text-primary-foreground"
                          : "border-border/70 bg-card/70"
                      )}
                    >
                      {label}
                    </button>
                  );
                })}
              </div>
            )}

            <Button
              type="button"
              disabled={!startsAt}
              onClick={() => setStep("details")}
            >
              Continuar
            </Button>
          </section>
        )}

        {step === "details" && service && employee && startsAt && (
          <section className="space-y-4">
            <Button type="button" variant="ghost" size="sm" onClick={() => setStep("slot")}>
              ← Cambiar horario
            </Button>
            <h2 className="font-heading text-2xl font-semibold">Tus datos</h2>
            <p className="text-sm text-muted-foreground">
              {service.name} · {employee.name} ·{" "}
              {formatDateTime(startsAt.toISOString())}
            </p>

            <div className="space-y-3">
              <div className="space-y-2">
                <Label htmlFor="name">Nombre</Label>
                <Input id="name" value={name} onChange={(e) => setName(e.target.value)} />
              </div>
              <div className="space-y-2">
                <Label htmlFor="email">Correo</Label>
                <Input
                  id="email"
                  type="email"
                  value={email}
                  onChange={(e) => setEmail(e.target.value)}
                />
              </div>
              <div className="space-y-2">
                <Label htmlFor="phone">Teléfono</Label>
                <Input
                  id="phone"
                  type="tel"
                  value={phone}
                  onChange={(e) => setPhone(e.target.value)}
                />
              </div>
              <div className="space-y-2">
                <Label htmlFor="document">Documento</Label>
                <Input
                  id="document"
                  value={document}
                  onChange={(e) => setDocument(e.target.value)}
                />
              </div>
              <div className="space-y-2">
                <Label htmlFor="notes">Notas (opcional)</Label>
                <Textarea
                  id="notes"
                  value={notes}
                  onChange={(e) => setNotes(e.target.value)}
                />
              </div>
            </div>

            <Button
              type="button"
              disabled={submitting}
              onClick={() => void submit()}
            >
              {submitting ? "Reservando..." : "Confirmar reserva"}
            </Button>
          </section>
        )}
      </FadeIn>
    </main>
  );
}
