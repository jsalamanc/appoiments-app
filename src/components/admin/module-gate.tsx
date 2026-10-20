"use client";

import { invoke } from "@tauri-apps/api/core";
import Link from "next/link";
import { useEffect, useState } from "react";

import { FadeIn } from "@/components/motion/fade-in";
import { Button } from "@/components/ui/button";
import { MODULE_META, type ModuleKey } from "@/lib/modules";

type ModuleGateProps = {
  module: ModuleKey;
  children: React.ReactNode;
};

type ModulesFlags = Record<
  "employees" | "appointments" | "services" | "packages" | "reports",
  boolean
>;

export function ModuleGate({ module, children }: ModuleGateProps) {
  const [modules, setModules] = useState<ModulesFlags | null>(null);

  useEffect(() => {
    let cancelled = false;
    invoke<ModulesFlags>("get_enabled_modules").then((m) => {
      if (!cancelled) setModules(m);
    });
    return () => {
      cancelled = true;
    };
  }, []);

  if (!modules) return null;
  if (modules[module]) {
    return <>{children}</>;
  }

  const meta = MODULE_META[module];

  return (
    <div className="mx-auto max-w-5xl">
      <FadeIn className="rounded-3xl border border-dashed border-border bg-card shadow-sm px-6 py-16 text-center">
        <p className="text-sm text-muted-foreground">Módulo inactivo</p>
        <h1 className="mt-2 font-heading text-3xl font-semibold">
          {meta.label} no está disponible
        </h1>
        <p className="mx-auto mt-2 max-w-md text-muted-foreground">
          Activa este módulo en Ajustes para usarlo.
        </p>
        <Button className="mt-6" render={<Link href="/admin/settings" />}>
          Ir a Ajustes
        </Button>
      </FadeIn>
    </div>
  );
}
