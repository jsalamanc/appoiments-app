"use client";

import { ChevronLeft, ChevronRight, Search, UserRound } from "lucide-react";
import Link from "next/link";
import { useRouter } from "next/navigation";
import { useState } from "react";

import { EmptyState } from "@/components/admin/empty-state";
import { FadeIn, StaggerChildren, StaggerItem } from "@/components/motion/fade-in";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import type { Customer } from "@/types/database";

const PATHNAME = "/admin/customers";

type CustomersManagerProps = {
  customers: Customer[];
  page: number;
  totalPages: number;
  total: number;
  pageSize: number;
  q: string;
};

function href(page: number, q: string) {
  const params = new URLSearchParams();
  if (q) params.set("q", q);
  if (page > 1) params.set("page", String(page));
  const qs = params.toString();
  return qs ? `${PATHNAME}?${qs}` : PATHNAME;
}

export function CustomersManager({
  customers,
  page,
  totalPages,
  total,
  pageSize,
  q,
}: CustomersManagerProps) {
  const router = useRouter();
  const [search, setSearch] = useState(q);
  const from = total === 0 ? 0 : (page - 1) * pageSize + 1;
  const to = Math.min(page * pageSize, total);

  const submitSearch = (event: React.FormEvent) => {
    event.preventDefault();
    router.push(href(1, search.trim()));
  };

  return (
    <div className="mx-auto max-w-5xl space-y-6">
      <FadeIn>
        <p className="text-sm text-muted-foreground">Relación</p>
        <h1 className="font-heading text-4xl font-semibold">Clientes</h1>
        <p className="mt-1 text-muted-foreground">
          Consulta fichas, ventas y citas de tus clientes.
        </p>
      </FadeIn>

      <FadeIn delay={0.05}>
        <form
          onSubmit={submitSearch}
          className="flex flex-col gap-2 sm:flex-row"
        >
          <div className="relative flex-1">
            <Search className="pointer-events-none absolute top-1/2 left-2.5 size-4 -translate-y-1/2 text-muted-foreground" />
            <Input
              value={search}
              onChange={(e) => setSearch(e.target.value)}
              placeholder="Buscar por nombre, correo, documento o teléfono..."
              className="pl-8"
            />
          </div>
          <Button type="submit">Buscar</Button>
        </form>
      </FadeIn>

      {total === 0 ? (
        <EmptyState
          title={q ? "Sin resultados" : "Aún no hay clientes"}
          description={
            q
              ? "Prueba con otro término de búsqueda."
              : "Los clientes se crean al registrar una venta o una cita."
          }
        />
      ) : (
        <>
          <StaggerChildren className="space-y-3">
            {customers.map((customer) => (
              <StaggerItem key={customer.id}>
                <Link
                  href={`/admin/customers/${customer.id}`}
                  className="flex items-start gap-3 rounded-3xl border border-border/70 bg-card/70 p-4 transition hover:border-primary/40 hover:bg-card"
                >
                  <span className="inline-flex rounded-2xl bg-lavender/50 p-3">
                    <UserRound className="size-5" />
                  </span>
                  <div className="min-w-0 flex-1">
                    <p className="font-heading text-xl font-semibold">
                      {customer.name}
                    </p>
                    <p className="truncate text-sm text-muted-foreground">
                      {customer.email} · {customer.phone}
                    </p>
                    <p className="text-xs text-muted-foreground">
                      Doc: {customer.document}
                    </p>
                  </div>
                </Link>
              </StaggerItem>
            ))}
          </StaggerChildren>

          <FadeIn
            delay={0.08}
            className="flex flex-col gap-3 rounded-2xl border border-border/60 bg-muted/20 px-4 py-3 sm:flex-row sm:items-center sm:justify-between"
          >
            <p className="text-sm text-muted-foreground">
              Mostrando {from}–{to} de {total}
            </p>
            <div className="flex items-center gap-2">
              <Button
                variant="outline"
                size="sm"
                disabled={page <= 1}
                render={<Link href={href(page - 1, q)} />}
              >
                <ChevronLeft data-icon="inline-start" />
                Anterior
              </Button>
              <span className="min-w-16 text-center text-sm text-muted-foreground">
                {page} / {totalPages}
              </span>
              <Button
                variant="outline"
                size="sm"
                disabled={page >= totalPages}
                render={<Link href={href(page + 1, q)} />}
              >
                Siguiente
                <ChevronRight data-icon="inline-end" />
              </Button>
            </div>
          </FadeIn>
        </>
      )}
    </div>
  );
}
