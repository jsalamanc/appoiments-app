"use client";

import { useRouter } from "next/navigation";
import { useEffect, useState } from "react";
import { toast } from "sonner";

import { FadeIn } from "@/components/motion/fade-in";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { formatCurrency } from "@/lib/format";
import {
  parsePlanBenefits,
  planPricePesos,
  updateCenterPlan,
} from "@/lib/plans";
import type { Plan, PlanId } from "@/types/database";

type PlanSettingsProps = {
  plans: Plan[];
  currentPlanId: string;
};

export function PlanSettings({ plans, currentPlanId }: PlanSettingsProps) {
  const router = useRouter();
  const [selectedId, setSelectedId] = useState(currentPlanId);
  const [savingId, setSavingId] = useState<string | null>(null);

  useEffect(() => {
    setSelectedId(currentPlanId);
  }, [currentPlanId]);

  const choose = async (planId: string) => {
    if (planId === selectedId) return;
    setSavingId(planId);
    try {
      await updateCenterPlan(planId as PlanId);
      setSelectedId(planId);
      toast.success("Plan actualizado");
      router.refresh();
    } catch (err) {
      toast.error(err instanceof Error ? err.message : "No se pudo cambiar el plan");
    } finally {
      setSavingId(null);
    }
  };

  return (
    <div className="mx-auto max-w-5xl space-y-6">
      <FadeIn>
        <p className="text-sm text-muted-foreground">Facturación</p>
        <h2 className="font-heading text-3xl font-semibold">Tu plan</h2>
        <p className="mt-1 text-muted-foreground">
          Sin cobro por ahora. Al cambiar de plan se desactivan los módulos que
          no incluye el plan nuevo.
        </p>
      </FadeIn>

      <FadeIn delay={0.05} className="grid gap-4 md:grid-cols-3">
        {plans.map((plan) => {
          const active = plan.id === selectedId;
          const benefits = parsePlanBenefits(plan.benefits);
          const busy = savingId === plan.id;

          return (
            <article
              key={plan.id}
              className={`flex flex-col rounded-3xl border p-5 ${
                active
                  ? "border-primary bg-primary/5"
                  : "border-border/70 bg-card shadow-sm"
              }`}
            >
              <div className="flex items-start justify-between gap-2">
                <h3 className="font-heading text-2xl font-semibold">
                  {plan.name}
                </h3>
                {active && <Badge>Plan actual</Badge>}
              </div>
              <p className="mt-2 font-heading text-3xl font-semibold">
                {plan.price_cents === 0
                  ? "Gratis"
                  : formatCurrency(planPricePesos(plan))}
              </p>
              {plan.price_cents > 0 && (
                <p className="text-xs text-muted-foreground">por mes</p>
              )}

              <ul className="mt-4 flex-1 space-y-2 text-sm text-muted-foreground">
                {benefits.map((benefit) => (
                  <li key={benefit} className="flex gap-2">
                    <span className="mt-1.5 size-1.5 shrink-0 rounded-full bg-primary" />
                    <span>{benefit}</span>
                  </li>
                ))}
              </ul>

              <Button
                className="mt-5 w-full"
                variant={active ? "secondary" : "default"}
                disabled={active || busy}
                onClick={() => void choose(plan.id)}
              >
                {active ? "Plan actual" : busy ? "Guardando..." : "Seleccionar"}
              </Button>
            </article>
          );
        })}
      </FadeIn>
    </div>
  );
}
