import Link from "next/link";
import { redirect } from "next/navigation";

import { FadeIn } from "@/components/motion/fade-in";
import { Button } from "@/components/ui/button";
import { isOwnerRole } from "@/lib/roles";
import { getServerCenterContext } from "@/lib/supabase/center";

export async function OwnerGate({ children }: { children: React.ReactNode }) {
  const ctx = await getServerCenterContext();
  if (!ctx) redirect("/login");

  if (isOwnerRole(ctx.role)) {
    return <>{children}</>;
  }

  return (
    <div className="mx-auto max-w-5xl">
      <FadeIn className="rounded-3xl border border-dashed border-border bg-card/50 px-6 py-16 text-center">
        <p className="text-sm text-muted-foreground">Acceso restringido</p>
        <h1 className="mt-2 font-heading text-3xl font-semibold">
          Solo el dueño puede ver esto
        </h1>
        <p className="mx-auto mt-2 max-w-md text-muted-foreground">
          Tu rol de recepción no incluye ajustes ni reportes.
        </p>
        <Button className="mt-6" render={<Link href="/admin" />}>
          Ir al panel
        </Button>
      </FadeIn>
    </div>
  );
}
