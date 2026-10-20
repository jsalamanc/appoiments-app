"use client";

import { useEffect, useState } from "react";
import { toast } from "sonner";

import { FadeIn } from "@/components/motion/fade-in";
import { Label } from "@/components/ui/label";
import {
  getEnabledModules,
  MODULE_KEYS,
  MODULE_META,
  toggleModule,
  updateCenterModules,
  type CenterModules,
  type ModuleKey,
} from "@/lib/modules";

export function ModulesSettings() {
  const [modules, setModules] = useState<CenterModules | null>(null);
  const [savingKey, setSavingKey] = useState<ModuleKey | null>(null);

  useEffect(() => {
    let cancelled = false;
    getEnabledModules().then((m) => {
      if (!cancelled) setModules(m);
    });
    return () => {
      cancelled = true;
    };
  }, []);

  if (!modules) return null;

  const onToggle = async (key: ModuleKey, enabled: boolean) => {
    const previous = modules;
    const next = toggleModule(modules, key, enabled);
    setModules(next);
    setSavingKey(key);
    try {
      const saved = await updateCenterModules(next);
      setModules(saved);
      toast.success(
        enabled
          ? `${MODULE_META[key].label} activado`
          : `${MODULE_META[key].label} desactivado`
      );
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
          Activa los módulos que necesites. Productos y ventas siempre están
          disponibles.
        </p>
      </FadeIn>

      <FadeIn
        delay={0.05}
        className="space-y-1 rounded-3xl border border-border/70 bg-card shadow-sm p-2"
      >
        {MODULE_KEYS.map((key) => {
          const meta = MODULE_META[key];
          const enabled = modules[key];
          const busy = savingKey === key;
          const employeesLockedByAgenda =
            key === "employees" && modules.appointments && enabled;

          return (
            <div
              key={key}
              className="flex items-start justify-between gap-4 rounded-2xl px-4 py-4 hover:bg-muted/40"
            >
              <div className="min-w-0 space-y-1">
                <Label htmlFor={`module-${key}`} className="text-base">
                  {meta.label}
                </Label>
                <p className="text-sm text-muted-foreground">
                  {meta.description}
                </p>
                {employeesLockedByAgenda && (
                  <p className="text-xs text-muted-foreground">
                    Requerido por Agenda. Desactiva Agenda para poder apagar
                    Empleados.
                  </p>
                )}
              </div>
              <button
                id={`module-${key}`}
                type="button"
                role="switch"
                aria-checked={enabled}
                disabled={busy || employeesLockedByAgenda}
                onClick={() => void onToggle(key, !enabled)}
                className={`relative mt-1 h-7 w-12 shrink-0 rounded-full transition focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring disabled:cursor-not-allowed disabled:opacity-60 ${
                  enabled ? "bg-primary" : "bg-muted"
                }`}
              >
                <span
                  className={`absolute top-0.5 left-0.5 size-6 rounded-full bg-background shadow transition ${
                    enabled ? "translate-x-5" : "translate-x-0"
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
