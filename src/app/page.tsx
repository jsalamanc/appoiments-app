"use client";

import { motion } from "framer-motion";
import { ArrowRight, Sparkles } from "lucide-react";
import Link from "next/link";

import { Button } from "@/components/ui/button";

export default function HomePage() {
  return (
    <main className="relative flex min-h-screen flex-col overflow-hidden">
      <header className="mx-auto flex w-full max-w-6xl items-center justify-between px-6 py-6">
        <span className="font-heading text-2xl font-semibold tracking-wide text-foreground">
          Aura Studio
        </span>
        <div className="flex items-center gap-2">
          <Button variant="ghost" render={<Link href="/login" />}>
            Iniciar sesión
          </Button>
          <Button render={<Link href="/register" />}>Crear cuenta</Button>
        </div>
      </header>

      <section className="relative mx-auto flex w-full max-w-6xl flex-1 flex-col justify-center px-6 pb-20 pt-8">
        <motion.div
          initial={{ opacity: 0, y: 24 }}
          animate={{ opacity: 1, y: 0 }}
          transition={{ duration: 0.6, ease: "easeOut" }}
          className="max-w-2xl"
        >
          <p className="mb-4 inline-flex items-center gap-2 rounded-full bg-lavender/70 px-3 py-1 text-sm text-foreground/80">
            <Sparkles className="size-3.5" />
            Para centros estéticos
          </p>
          <h1 className="font-heading text-5xl leading-[1.1] font-semibold tracking-tight sm:text-6xl md:text-7xl">
            Aura Studio
          </h1>
          <p className="mt-5 max-w-lg text-lg text-muted-foreground">
            Gestiona productos, servicios y paquetes de tu centro con una
            experiencia suave, clara y hecha para brillar sin gritar.
          </p>
          <div className="mt-8 flex flex-wrap gap-3">
            <Button size="lg" render={<Link href="/register" />}>
              Empezar gratis
              <ArrowRight data-icon="inline-end" />
            </Button>
            <Button
              size="lg"
              variant="outline"
              render={<Link href="/login" />}
            >
              Ya tengo cuenta
            </Button>
          </div>
        </motion.div>

        <motion.div
          aria-hidden
          initial={{ opacity: 0, scale: 0.92 }}
          animate={{ opacity: 1, scale: 1 }}
          transition={{ duration: 0.8, delay: 0.15, ease: "easeOut" }}
          className="pointer-events-none absolute right-0 top-1/2 hidden h-[420px] w-[420px] -translate-y-1/2 md:block"
        >
          <div className="absolute inset-8 rounded-full bg-lavender/50 blur-2xl" />
          <div className="absolute inset-20 rounded-full bg-mint/60 blur-xl" />
          <div className="absolute inset-32 rounded-full bg-cream/80 blur-lg" />
        </motion.div>
      </section>
    </main>
  );
}
