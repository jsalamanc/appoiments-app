"use client";

import { useState } from "react";
import { toast } from "sonner";

import { Button } from "@/components/ui/button";
import { Checkbox } from "@/components/ui/checkbox";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { updateEmployeeAvailability, WEEKDAY_LABELS } from "@/lib/business-hours";
import type { Employee, EmployeeAvailability, EmployeeAvailabilityInput } from "@/types/database";

type DayOverride = EmployeeAvailabilityInput & { hasOverride: boolean };

function daysFromOverrides(overrides: EmployeeAvailability[]): DayOverride[] {
  const days: DayOverride[] = WEEKDAY_LABELS.map((_, weekday) => ({
    weekday,
    hasOverride: false,
    isAvailable: true,
    startsAt: null,
    endsAt: null,
  }));
  for (const override of overrides) {
    days[override.weekday] = {
      weekday: override.weekday,
      hasOverride: true,
      isAvailable: override.isAvailable,
      startsAt: override.startsAt,
      endsAt: override.endsAt,
    };
  }
  return days;
}

type EmployeeAvailabilityDialogProps = {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  employee: Employee | null;
  /** `null` mientras se está cargando desde el llamador (ver employees-manager.tsx). */
  overrides: EmployeeAvailability[] | null;
};

export function EmployeeAvailabilityDialog({
  open,
  onOpenChange,
  employee,
  overrides,
}: EmployeeAvailabilityDialogProps) {
  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="sm:max-w-lg">
        <DialogHeader>
          <DialogTitle>
            Disponibilidad {employee ? `de ${employee.name}` : ""}
          </DialogTitle>
          <DialogDescription>
            Por defecto sigue el horario de atención del negocio. Marca un día
            para darle una excepción propia (otro horario, o no disponible).
          </DialogDescription>
        </DialogHeader>

        {!employee || overrides === null ? (
          <p className="text-sm text-muted-foreground">Cargando...</p>
        ) : (
          <AvailabilityForm
            employeeId={employee.id}
            initialOverrides={overrides}
            onClose={() => onOpenChange(false)}
          />
        )}
      </DialogContent>
    </Dialog>
  );
}

/**
 * Se monta solo cuando `overrides` ya llegó (ver el ternario de arriba) —
 * así `useState(() => daysFromOverrides(initialOverrides))` siempre arranca
 * con los datos correctos sin necesitar un efecto que sincronice estado
 * derivado de props (evita el patrón que dispara
 * react-hooks/set-state-in-effect). Cada vez que el diálogo se vuelve a
 * abrir, el padre resetea `overrides` a `null` mientras recarga, lo que
 * desmonta esta instancia; al llegar los nuevos datos se monta una instancia
 * nueva con estado fresco.
 */
function AvailabilityForm({
  employeeId,
  initialOverrides,
  onClose,
}: {
  employeeId: string;
  initialOverrides: EmployeeAvailability[];
  onClose: () => void;
}) {
  const [days, setDays] = useState<DayOverride[]>(() =>
    daysFromOverrides(initialOverrides)
  );
  const [saving, setSaving] = useState(false);

  const updateDay = (weekday: number, patch: Partial<DayOverride>) => {
    setDays((prev) =>
      prev.map((day) => (day.weekday === weekday ? { ...day, ...patch } : day))
    );
  };

  const save = async () => {
    setSaving(true);
    try {
      const input: EmployeeAvailabilityInput[] = days
        .filter((day) => day.hasOverride)
        .map(({ weekday, isAvailable, startsAt, endsAt }) => ({
          weekday,
          isAvailable,
          startsAt: isAvailable ? startsAt : null,
          endsAt: isAvailable ? endsAt : null,
        }));
      await updateEmployeeAvailability(employeeId, input);
      toast.success("Disponibilidad actualizada");
      onClose();
    } catch (err) {
      toast.error(
        err instanceof Error ? err.message : "No se pudo guardar la disponibilidad"
      );
    } finally {
      setSaving(false);
    }
  };

  return (
    <>
      <div className="space-y-2">
        {days.map((day) => (
          <div
            key={day.weekday}
            className="space-y-2 rounded-2xl border border-border/60 px-4 py-3 text-sm"
          >
            <div className="flex items-center justify-between gap-3">
              <span className="font-medium">{WEEKDAY_LABELS[day.weekday]}</span>
              <label className="flex items-center gap-2 text-muted-foreground">
                <Checkbox
                  checked={day.hasOverride}
                  onCheckedChange={(checked) =>
                    updateDay(day.weekday, {
                      hasOverride: checked === true,
                      isAvailable: true,
                      startsAt: day.startsAt ?? "09:00",
                      endsAt: day.endsAt ?? "18:00",
                    })
                  }
                />
                Excepción
              </label>
            </div>

            {day.hasOverride && (
              <div className="flex flex-wrap items-center gap-3 pl-1">
                <label className="flex items-center gap-2">
                  <Checkbox
                    checked={day.isAvailable}
                    onCheckedChange={(checked) =>
                      updateDay(day.weekday, { isAvailable: checked === true })
                    }
                  />
                  Disponible
                </label>
                {day.isAvailable ? (
                  <div className="flex items-center gap-2">
                    <Input
                      type="time"
                      className="w-32"
                      value={day.startsAt ?? ""}
                      onChange={(e) =>
                        updateDay(day.weekday, { startsAt: e.target.value })
                      }
                    />
                    <span className="text-muted-foreground">a</span>
                    <Input
                      type="time"
                      className="w-32"
                      value={day.endsAt ?? ""}
                      onChange={(e) =>
                        updateDay(day.weekday, { endsAt: e.target.value })
                      }
                    />
                  </div>
                ) : (
                  <span className="text-muted-foreground">No atiende este día</span>
                )}
              </div>
            )}
          </div>
        ))}
      </div>

      <div className="flex justify-end gap-2 pt-2">
        <Button type="button" variant="outline" onClick={onClose}>
          Cancelar
        </Button>
        <Button type="button" disabled={saving} onClick={() => void save()}>
          {saving ? "Guardando..." : "Guardar"}
        </Button>
      </div>
    </>
  );
}
