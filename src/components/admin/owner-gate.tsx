"use client";

import { invoke } from "@tauri-apps/api/core";
import Link from "next/link";
import { useEffect, useState } from "react";

import { FadeIn } from "@/components/motion/fade-in";
import { Button } from "@/components/ui/button";
import { isOwnerRole } from "@/lib/roles";

type CurrentUser = { role: string } | null;

export function OwnerGate({ children }: { children: React.ReactNode }) {
  const [user, setUser] = useState<CurrentUser>(null);
  const [loaded, setLoaded] = useState(false);

  useEffect(() => {
    let cancelled = false;
    invoke<CurrentUser>("auth_current_user").then((u) => {
      if (!cancelled) {
        setUser(u);
        setLoaded(true);
      }
    });
    return () => {
      cancelled = true;
    };
  }, []);

  if (!loaded) return null;

  if (isOwnerRole(user?.role)) {
    return <>{children}</>;
  }

  return (
    <div className="mx-auto max-w-5xl">
      <FadeIn className="rounded-3xl border border-dashed border-border bg-card shadow-sm px-6 py-16 text-center">
        <p className="text-sm text-muted-foreground">Acceso restringido</p>
        <h1 className="mt-2 font-heading text-3xl font-semibold">
          Solo el dueño puede ver esto
        </h1>
        <p className="mx-auto mt-2 max-w-md text-muted-foreground">
          Tu rol de staff no incluye ajustes ni reportes.
        </p>
        <Button className="mt-6" render={<Link href="/admin" />}>
          Ir al panel
        </Button>
      </FadeIn>
    </div>
  );
}
