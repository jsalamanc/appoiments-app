"use client";

import { ChevronLeft, ChevronRight } from "lucide-react";
import Link from "next/link";

import { Button } from "@/components/ui/button";
import {
  buildCatalogHref,
  type StatusFilter,
} from "@/lib/pagination";

export function CatalogPagination({
  pathname,
  status,
  page,
  totalPages,
  total,
  pageSize,
}: {
  pathname: string;
  status: StatusFilter;
  page: number;
  totalPages: number;
  total: number;
  pageSize: number;
}) {
  if (total === 0) return null;

  const from = (page - 1) * pageSize + 1;
  const to = Math.min(page * pageSize, total);
  const prevHref = buildCatalogHref(pathname, { status, page: page - 1 });
  const nextHref = buildCatalogHref(pathname, { status, page: page + 1 });

  return (
    <div className="flex flex-col gap-3 rounded-2xl border border-border/70 bg-card shadow-sm px-4 py-3 sm:flex-row sm:items-center sm:justify-between">
      <p className="text-sm text-muted-foreground">
        Mostrando {from}–{to} de {total}
      </p>
      <div className="flex items-center gap-2">
        {page <= 1 ? (
          <Button variant="outline" size="sm" disabled>
            <ChevronLeft data-icon="inline-start" />
            Anterior
          </Button>
        ) : (
          <Button variant="outline" size="sm" render={<Link href={prevHref} />}>
            <ChevronLeft data-icon="inline-start" />
            Anterior
          </Button>
        )}
        <span className="min-w-16 text-center text-sm text-muted-foreground">
          {page} / {totalPages}
        </span>
        {page >= totalPages ? (
          <Button variant="outline" size="sm" disabled>
            Siguiente
            <ChevronRight data-icon="inline-end" />
          </Button>
        ) : (
          <Button variant="outline" size="sm" render={<Link href={nextHref} />}>
            Siguiente
            <ChevronRight data-icon="inline-end" />
          </Button>
        )}
      </div>
    </div>
  );
}
