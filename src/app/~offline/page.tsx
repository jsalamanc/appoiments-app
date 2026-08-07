import Link from "next/link";

import { Button } from "@/components/ui/button";

export default function OfflinePage() {
  return (
    <main className="flex min-h-screen flex-col items-center justify-center px-6 text-center">
      <p className="font-heading text-sm tracking-wide text-muted-foreground">
        Aura Studio
      </p>
      <h1 className="mt-3 font-heading text-4xl font-semibold tracking-tight">
        Sin conexión
      </h1>
      <p className="mt-4 max-w-md text-muted-foreground">
        No hay conexión a internet. Revisa tu red e intenta de nuevo cuando
        vuelvas a estar en línea.
      </p>
      <Button className="mt-8" render={<Link href="/" />}>
        Reintentar
      </Button>
    </main>
  );
}
