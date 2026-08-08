import Link from "next/link";
import { redirect } from "next/navigation";

import { FadeIn } from "@/components/motion/fade-in";
import { Button } from "@/components/ui/button";
import { MODULE_META, parseCenterModules, type ModuleKey } from "@/lib/modules";
import {
  canUseModule,
  isModuleAllowedByPlan,
  planIdForModuleUnlock,
} from "@/lib/plan-modules";
import { getServerCenterContext } from "@/lib/supabase/center";

type ModuleGateProps = {
  module: ModuleKey;
  children: React.ReactNode;
};

export async function ModuleGate({ module, children }: ModuleGateProps) {
  const ctx = await getServerCenterContext();
  if (!ctx) redirect("/login");

  const modules = parseCenterModules(ctx.center.modules);
  if (canUseModule(modules, ctx.planAllowed, module)) {
    return <>{children}</>;
  }

  const meta = MODULE_META[module];
  const allowedByPlan = isModuleAllowedByPlan(ctx.planAllowed, module);
  const unlockPlan = planIdForModuleUnlock(module);

  return (
    <div className="mx-auto max-w-5xl">
      <FadeIn className="rounded-3xl border border-dashed border-border bg-card shadow-sm px-6 py-16 text-center">
        <p className="text-sm text-muted-foreground">
          {allowedByPlan ? "Módulo inactivo" : "No incluido en tu plan"}
        </p>
        <h1 className="mt-2 font-heading text-3xl font-semibold">
          {meta.label} no está disponible
        </h1>
        <p className="mx-auto mt-2 max-w-md text-muted-foreground">
          {allowedByPlan
            ? "Activa este módulo en Ajustes para usarlo."
            : `Este módulo requiere el plan ${unlockPlan === "growth" ? "Growth" : "Starter"} o superior.`}
        </p>
        <Button className="mt-6" render={<Link href="/admin/settings" />}>
          Ir a Ajustes
        </Button>
      </FadeIn>
    </div>
  );
}
