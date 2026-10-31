"use client";

import { Clock } from "lucide-react";
import { useEffect, useState } from "react";
import { toast } from "sonner";

import { FadeIn } from "@/components/motion/fade-in";
import { Button } from "@/components/ui/button";
import { Checkbox } from "@/components/ui/checkbox";
import { Input } from "@/components/ui/input";
import { getBusinessHours, updateBusinessHours, WEEKDAY_LABELS } from "@/lib/business-hours";
import type { BusinessHoursInput } from "@/types/database";

export function BusinessHoursSettings() {
  const [hours, setHours] = useState<BusinessHoursInput[]>([]);
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);

  useEffect(() => {
    let cancelled = false;
    getBusinessHours()
      .then((rows) => {
        if (cancelled) return;
        setHours(
          [...rows]
            .sort((a, b) => a.weekday - b.weekday)
            .map(({ weekday, isOpen, opensAt, closesAt }) => ({
              weekday,
              isOpen,
              opensAt,
              closesAt,
            }))
        );
      })
      .catch((err) => {
        if (cancelled) return;
        toast.error(err instanceof Error ? err.message : "Error al cargar el horario");
      })
      .finally(() => {
        if (!cancelled) setLoading(false);
      });
    return () => {
      cancelled = true;
    };
  }, []);

  const updateDay = (weekday: number, patch: Partial<BusinessHoursInput>) => {
    setHours((prev) =>
      prev.map((day) => (day.weekday === weekday ? { ...day, ...patch } : day))
    );
  };

  const save = async () => {
    setSaving(true);
    try {
      const saved = await updateBusinessHours(hours);
      setHours(
        [...saved]
          .sort((a, b) => a.weekday - b.weekday)
          .map(({ weekday, isOpen, opensAt, closesAt }) => ({
            weekday,
            isOpen,
            opensAt,
            closesAt,
          }))
      );
      toast.success("Horario de atención actualizado");
    } catch (err) {
      toast.error(err instanceof Error ? err.message : "No se pudo guardar el horario");
    } finally {
      setSaving(false);
    }
  };

  return (
    <div className="mx-auto max-w-5xl space-y-6">
      <FadeIn
        delay={0.08}
        className="rounded-3xl border border-border/70 bg-card shadow-sm p-5"
      >
        <div className="mb-3 flex items-center gap-2">
          <Clock className="size-4 text-muted-foreground" />
          <h2 className="font-heading text-2xl font-semibold">
            Horario de atención
          </h2>
        </div>
        <p className="text-sm text-muted-foreground">
          Define los días y horas en que el negocio atiende. Las citas no se
          podrán agendar fuera de este horario, salvo que un empleado tenga
          una excepción propia.
        </p>

        {loading ? (
          <p className="mt-4 text-sm text-muted-foreground">
            Cargando horario...
          </p>
        ) : (
          <div className="mt-6 space-y-2">
            {hours.map((day) => (
              <div
                key={day.weekday}
                className="flex flex-wrap items-center gap-3 rounded-2xl border border-border/60 px-4 py-3 text-sm"
              >
                <label className="flex w-36 shrink-0 items-center gap-2 font-medium">
                  <Checkbox
                    checked={day.isOpen}
                    onCheckedChange={(checked) =>
                      updateDay(day.weekday, {
                        isOpen: checked === true,
                        opensAt: checked === true ? (day.opensAt ?? "09:00") : null,
                        closesAt: checked === true ? (day.closesAt ?? "18:00") : null,
                      })
                    }
                  />
                  {WEEKDAY_LABELS[day.weekday]}
                </label>
                {day.isOpen ? (
                  <div className="flex items-center gap-2">
                    <Input
                      type="time"
                      className="w-32"
                      value={day.opensAt ?? ""}
                      onChange={(e) =>
                        updateDay(day.weekday, { opensAt: e.target.value })
                      }
                    />
                    <span className="text-muted-foreground">a</span>
                    <Input
                      type="time"
                      className="w-32"
                      value={day.closesAt ?? ""}
                      onChange={(e) =>
                        updateDay(day.weekday, { closesAt: e.target.value })
                      }
                    />
                  </div>
                ) : (
                  <span className="text-muted-foreground">Cerrado</span>
                )}
              </div>
            ))}
            <Button
              type="button"
              className="mt-2"
              disabled={saving}
              onClick={() => void save()}
            >
              {saving ? "Guardando..." : "Guardar horario"}
            </Button>
          </div>
        )}
      </FadeIn>
    </div>
  );
}
