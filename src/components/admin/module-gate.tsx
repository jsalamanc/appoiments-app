import Link from "next/link";
import { redirect } from "next/navigation";

import { FadeIn } from "@/components/motion/fade-in";
import { Button } from "@/components/ui/button";
import {
  isModuleEnabled,
  MODULE_META,
  parseCenterModules,
  type ModuleKey,
} from "@/lib/modules";
import { getServerCenter } from "@/lib/supabase/center";

type ModuleGateProps = {
  module: ModuleKey;
  children: React.ReactNode;
};

export async function ModuleGate({ module, children }: ModuleGateProps) {
  const center = await getServerCenter();
  if (!center) redirect("/login");

  const modules = parseCenterModules(center.modules);
  if (isModuleEnabled(modules, module)) {
    return <>{children}</>;
  }

  const meta = MODULE_META[module];

  return (
    <div className="mx-auto max-w-5xl">
      <FadeIn className="rounded-3xl border border-dashed border-border bg-card/50 px-6 py-16 text-center">
        <p className="text-sm text-muted-foreground">Módulo inactivo</p>
        <h1 className="mt-2 font-heading text-3xl font-semibold">
          {meta.label} no está activo
        </h1>
        <p className="mx-auto mt-2 max-w-md text-muted-foreground">
          Activa este módulo en Ajustes para usarlo. Hoy es gratis.
        </p>
        <Button className="mt-6" render={<Link href="/admin/settings" />}>
          Ir a Ajustes
        </Button>
      </FadeIn>
    </div>
  );
}
