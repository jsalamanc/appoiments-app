"use client";

import { getVersion } from "@tauri-apps/api/app";
import { useEffect, useState } from "react";
import { toast } from "sonner";

import { FadeIn } from "@/components/motion/fade-in";
import { Button } from "@/components/ui/button";
import { checkForUpdates, notifyUpdateAvailable } from "@/lib/updates";

export function UpdateSettings() {
  const [version, setVersion] = useState<string | null>(null);
  const [checking, setChecking] = useState(false);

  useEffect(() => {
    let cancelled = false;
    getVersion().then((v) => {
      if (!cancelled) setVersion(v);
    });
    return () => {
      cancelled = true;
    };
  }, []);

  const runCheck = async () => {
    setChecking(true);
    try {
      const update = await checkForUpdates();
      if (update) {
        notifyUpdateAvailable(update);
      } else {
        toast.success("Ya tienes la última versión");
      }
    } catch (err) {
      toast.error(
        err instanceof Error
          ? err.message
          : "No se pudo buscar actualizaciones"
      );
    } finally {
      setChecking(false);
    }
  };

  return (
    <div className="mx-auto max-w-5xl space-y-6">
      <FadeIn
        delay={0.1}
        className="rounded-3xl border border-border/70 bg-card shadow-sm p-5"
      >
        <h2 className="font-heading text-2xl font-semibold">
          Actualizaciones
        </h2>
        <p className="mt-1 text-sm text-muted-foreground">
          Versión instalada: {version ?? "…"}
        </p>
        <Button
          type="button"
          className="mt-3"
          disabled={checking}
          onClick={() => void runCheck()}
        >
          {checking ? "Buscando..." : "Buscar actualizaciones"}
        </Button>
      </FadeIn>
    </div>
  );
}
