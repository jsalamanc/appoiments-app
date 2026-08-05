"use client";

import Link from "next/link";

import {
  buildCatalogHref,
  type StatusFilter,
} from "@/lib/pagination";
import { cn } from "@/lib/utils";

const FILTERS: { value: StatusFilter; label: string }[] = [
  { value: "all", label: "Todos" },
  { value: "published", label: "Publicados" },
  { value: "draft", label: "Borradores" },
];

export function CatalogFilters({
  pathname,
  status,
  counts,
}: {
  pathname: string;
  status: StatusFilter;
  counts: { all: number; published: number; draft: number };
}) {
  return (
    <div className="flex flex-wrap gap-2">
      {FILTERS.map((filter) => {
        const active = status === filter.value;
        return (
          <Link
            key={filter.value}
            href={buildCatalogHref(pathname, {
              status: filter.value,
              page: 1,
            })}
            className={cn(
              "inline-flex items-center gap-2 rounded-full px-3 py-1.5 text-sm transition",
              active
                ? "bg-primary text-primary-foreground"
                : "bg-muted/70 text-muted-foreground hover:bg-muted hover:text-foreground"
            )}
          >
            {filter.label}
            <span
              className={cn(
                "rounded-full px-1.5 text-xs",
                active ? "bg-primary-foreground/20" : "bg-background/70"
              )}
            >
              {counts[filter.value]}
            </span>
          </Link>
        );
      })}
    </div>
  );
}
