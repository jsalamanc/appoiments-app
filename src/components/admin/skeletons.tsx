"use client";

import { Skeleton } from "@/components/ui/skeleton";
import { cn } from "@/lib/utils";

/** Contenedor accesible: anuncia "Cargando" a lectores de pantalla y marca la
 * región como ocupada. Todos los skeletons de página lo usan como raíz. */
export function SkeletonRegion({
  className,
  label = "Cargando…",
  children,
}: {
  className?: string;
  label?: string;
  children: React.ReactNode;
}) {
  return (
    <div role="status" aria-busy="true" aria-live="polite" className={className}>
      <span className="sr-only">{label}</span>
      {children}
    </div>
  );
}

const card = "rounded-3xl border border-border/70 bg-card shadow-sm";

export function PageHeaderSkeleton({ withAction = false }: { withAction?: boolean }) {
  return (
    <div className="flex flex-col gap-4 sm:flex-row sm:items-end sm:justify-between">
      <div className="space-y-2">
        <Skeleton className="h-4 w-28" />
        <Skeleton className="h-10 w-64 max-w-full" />
        <Skeleton className="h-4 w-80 max-w-full" />
      </div>
      {withAction && <Skeleton className="h-9 w-36 rounded-lg" />}
    </div>
  );
}

export function StatCardsSkeleton({ count = 3 }: { count?: number }) {
  return (
    <div className="grid gap-4 sm:grid-cols-2 lg:grid-cols-3">
      {Array.from({ length: count }).map((_, i) => (
        <div key={i} className={cn(card, "space-y-3 p-5")}>
          <Skeleton className="size-11 rounded-2xl" />
          <Skeleton className="h-4 w-24" />
          <Skeleton className="h-9 w-32" />
        </div>
      ))}
    </div>
  );
}

export function ListSkeleton({
  rows = 5,
  className,
}: {
  rows?: number;
  className?: string;
}) {
  return (
    <ul className={cn("space-y-2", className)}>
      {Array.from({ length: rows }).map((_, i) => (
        <li
          key={i}
          className="flex items-center justify-between gap-3 rounded-2xl border border-border/60 bg-card px-4 py-3"
        >
          <div className="min-w-0 flex-1 space-y-2">
            <Skeleton className="h-4 w-1/3" />
            <Skeleton className="h-3 w-1/2" />
          </div>
          <Skeleton className="h-6 w-16 rounded-full" />
        </li>
      ))}
    </ul>
  );
}

export function TableSkeleton({
  rows = 6,
  cols = 4,
}: {
  rows?: number;
  cols?: number;
}) {
  return (
    <div className={cn(card, "overflow-hidden shadow-md")}>
      <div className="flex gap-4 border-b border-border/70 bg-black/[0.02] px-4 py-3">
        {Array.from({ length: cols }).map((_, i) => (
          <Skeleton key={i} className="h-4 flex-1" />
        ))}
      </div>
      {Array.from({ length: rows }).map((_, r) => (
        <div
          key={r}
          className="flex gap-4 border-b border-border/50 px-4 py-3.5 last:border-0"
        >
          {Array.from({ length: cols }).map((_, c) => (
            <Skeleton
              key={c}
              className={cn("h-4 flex-1", c === 0 && "max-w-[40%]")}
            />
          ))}
        </div>
      ))}
    </div>
  );
}

/** Esqueleto para secciones de Ajustes (tarjeta con título + filas). */
export function SettingsSectionSkeleton({ rows = 3 }: { rows?: number }) {
  return (
    <SkeletonRegion className={cn(card, "space-y-4 p-6")}>
      <Skeleton className="h-6 w-56" />
      <Skeleton className="h-4 w-80 max-w-full" />
      <div className="space-y-2 pt-2">
        {Array.from({ length: rows }).map((_, i) => (
          <Skeleton key={i} className="h-12 w-full rounded-2xl" />
        ))}
      </div>
    </SkeletonRegion>
  );
}

// ---------------------------------------------------------------------------
// Esqueletos de página completa
// ---------------------------------------------------------------------------

/** Contenido estándar de carga: 3 tarjetas de resumen + tabla. Es el diseño
 * de "Caja del día" y se usa como carga por defecto en toda la app. */
export function ContentSkeleton() {
  return (
    <div className="space-y-6">
      <StatCardsSkeleton count={3} />
      <TableSkeleton rows={5} cols={4} />
    </div>
  );
}

/** Página completa estándar: encabezado con botón + contenido estándar. Lo
 * usan los reportes, los listados, el catálogo y los gates (módulo/rol)
 * mientras resuelven. */
export function ReportPageSkeleton() {
  return (
    <SkeletonRegion className="mx-auto max-w-5xl space-y-6">
      <PageHeaderSkeleton withAction />
      <ContentSkeleton />
    </SkeletonRegion>
  );
}

export const PageSkeleton = ReportPageSkeleton;
export const CatalogPageSkeleton = ReportPageSkeleton;
export const TablePageSkeleton = ReportPageSkeleton;
export const DetailPageSkeleton = ReportPageSkeleton;

/** Recibo imprimible. */
export function ReceiptSkeleton() {
  return (
    <SkeletonRegion className="mx-auto max-w-md space-y-4">
      <div className={cn(card, "space-y-4 p-6")}>
        <Skeleton className="mx-auto h-6 w-40" />
        <Skeleton className="mx-auto h-4 w-56" />
        <div className="space-y-3 pt-4">
          {Array.from({ length: 4 }).map((_, i) => (
            <div key={i} className="flex justify-between gap-4">
              <Skeleton className="h-4 w-1/2" />
              <Skeleton className="h-4 w-16" />
            </div>
          ))}
        </div>
        <Skeleton className="ml-auto h-6 w-28" />
      </div>
    </SkeletonRegion>
  );
}

/** Panel principal. */
export function DashboardSkeleton() {
  return (
    <SkeletonRegion className="mx-auto max-w-5xl space-y-8">
      <div className="space-y-2">
        <Skeleton className="h-4 w-36" />
        <Skeleton className="h-10 w-72 max-w-full" />
        <Skeleton className="h-4 w-96 max-w-full" />
      </div>
      <RevenueMetricsSkeleton />
      <Skeleton className="h-20 w-full rounded-3xl" />
      <StatCardsSkeleton count={5} />
    </SkeletonRegion>
  );
}

/** Tarjeta de ingresos del panel (se carga aparte, solo para el dueño). */
export function RevenueMetricsSkeleton() {
  return (
    <div
      role="status"
      aria-busy="true"
      className={cn(card, "space-y-4 p-5")}
    >
      <span className="sr-only">Cargando ingresos…</span>
      <div className="flex items-center justify-between">
        <Skeleton className="h-6 w-40" />
        <Skeleton className="h-8 w-48 rounded-full" />
      </div>
      <Skeleton className="h-12 w-56" />
      <div className="grid gap-3 sm:grid-cols-3">
        {Array.from({ length: 3 }).map((_, i) => (
          <Skeleton key={i} className="h-16 w-full rounded-2xl" />
        ))}
      </div>
    </div>
  );
}

/** Calendario semanal de citas. */
export function CalendarSkeleton() {
  return (
    <div role="status" aria-busy="true" className="min-w-[840px] p-3">
      <span className="sr-only">Cargando agenda…</span>
      <div
        className="grid gap-2"
        style={{ gridTemplateColumns: "3.5rem repeat(7, minmax(0, 1fr))" }}
      >
        <div />
        {Array.from({ length: 7 }).map((_, i) => (
          <Skeleton key={i} className="h-10 w-full rounded-xl" />
        ))}
        {Array.from({ length: 6 }).map((_, r) => (
          <div key={r} className="contents">
            <Skeleton className="h-4 w-10 self-center" />
            {Array.from({ length: 7 }).map((_, c) => (
              <Skeleton
                key={c}
                className={cn(
                  "h-14 w-full rounded-xl",
                  (r + c) % 3 === 0 ? "opacity-100" : "opacity-40"
                )}
              />
            ))}
          </div>
        ))}
      </div>
    </div>
  );
}
