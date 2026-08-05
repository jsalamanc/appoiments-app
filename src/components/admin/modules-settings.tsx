"use client";

import { useRouter } from "next/navigation";
import { useEffect, useState } from "react";
import { toast } from "sonner";

import { FadeIn } from "@/components/motion/fade-in";
import { Badge } from "@/components/ui/badge";
import { Label } from "@/components/ui/label";
import {
  MODULE_KEYS,
  MODULE_META,
  parseCenterModules,
  toggleModule,
  updateCenterModules,
  type CenterModules,
  type ModuleKey,
} from "@/lib/modules";
import {
  isModuleAllowedByPlan,
  planIdForModuleUnlock,
} from "@/lib/plan-modules";
import type { Center } from "@/types/database";

export function ModulesSettings({
  center,
  planAllowed,
}: {
  center: Center;
  planAllowed: CenterModules;
}) {
  const router = useRouter();
  const [modules, setModules] = useState<CenterModules>(() =>
    parseCenterModules(center.modules)
  );
  const [savingKey, setSavingKey] = useState<ModuleKey | null>(null);

  useEffect(() => {
    setModules(parseCenterModules(center.modules));
  }, [center.modules]);

  const onToggle = async (key: ModuleKey, enabled: boolean) => {
    if (enabled && !isModuleAllowedByPlan(planAllowed, key)) {
      toast.error(
        `${MODULE_META[key].label} no está en tu plan. Cambia a ${
          planIdForModuleUnlock(key) === "growth" ? "Growth" : "Starter"
        }.`
      );
      return;
    }

    const previous = modules;
    const next = toggleModule(modules, key, enabled);
    setModules(next);
    setSavingKey(key);
    try {
      await updateCenterModules(next);
      toast.success(
        enabled
          ? `${MODULE_META[key].label} activado`
          : `${MODULE_META[key].label} desactivado`
      );
      router.refresh();
    } catch (err) {
      setModules(previous);
      toast.error(err instanceof Error ? err.message : "No se pudo guardar");
    } finally {
      setSavingKey(null);
    }
  };

  return (
    <div className="mx-auto max-w-5xl space-y-6">
      <FadeIn>
        <p className="text-sm text-muted-foreground">Configuración</p>
        <h1 className="font-heading text-4xl font-semibold">Ajustes</h1>
        <p className="mt-1 text-muted-foreground">
          Activa los módulos incluidos en tu plan. Productos y ventas siempre
          están disponibles.
        </p>
      </FadeIn>

      <FadeIn
        delay={0.05}
        className="space-y-1 rounded-3xl border border-border/70 bg-card/70 p-2"
      >
        {MODULE_KEYS.map((key) => {
          const meta = MODULE_META[key];
          const enabled = modules[key];
          const busy = savingKey === key;
          const allowed = isModuleAllowedByPlan(planAllowed, key);
          const employeesLockedByAgenda =
            key === "employees" && modules.appointments && enabled;
          const lockedByPublicBooking =
            modules.public_booking &&
            enabled &&
            (key === "appointments" ||
              key === "services" ||
              key === "employees");
          const unlockPlan = planIdForModuleUnlock(key);

          return (
            <div
              key={key}
              className="flex items-start justify-between gap-4 rounded-2xl px-4 py-4 hover:bg-muted/40"
            >
              <div className="min-w-0 space-y-1">
                <div className="flex flex-wrap items-center gap-2">
                  <Label htmlFor={`module-${key}`} className="text-base">
                    {meta.label}
                  </Label>
                  {allowed ? (
                    <Badge variant="secondary">En tu plan</Badge>
                  ) : (
                    <Badge variant="outline">
                      Requiere {unlockPlan === "growth" ? "Growth" : "Starter"}
                    </Badge>
                  )}
                </div>
                <p className="text-sm text-muted-foreground">
                  {meta.description}
                </p>
                {!allowed && (
                  <p className="text-xs text-muted-foreground">
                    Cambia de plan más abajo para desbloquear este módulo.
                  </p>
                )}
                {allowed && employeesLockedByAgenda && !lockedByPublicBooking && (
                  <p className="text-xs text-muted-foreground">
                    Requerido por Agenda. Desactiva Agenda para poder apagar
                    Empleados.
                  </p>
                )}
                {allowed && lockedByPublicBooking && (
                  <p className="text-xs text-muted-foreground">
                    Requerido por Portal de reservas. Desactívalo primero.
                  </p>
                )}
              </div>
              <button
                id={`module-${key}`}
                type="button"
                role="switch"
                aria-checked={enabled && allowed}
                disabled={
                  busy ||
                  !allowed ||
                  employeesLockedByAgenda ||
                  lockedByPublicBooking
                }
                onClick={() => void onToggle(key, !enabled)}
                className={`relative mt-1 h-7 w-12 shrink-0 rounded-full transition focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring disabled:cursor-not-allowed disabled:opacity-60 ${
                  enabled && allowed ? "bg-primary" : "bg-muted"
                }`}
              >
                <span
                  className={`absolute top-0.5 left-0.5 size-6 rounded-full bg-background shadow transition ${
                    enabled && allowed ? "translate-x-5" : "translate-x-0"
                  }`}
                />
              </button>
            </div>
          );
        })}
      </FadeIn>
    </div>
  );
}
