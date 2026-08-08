"use client";

import Link from "next/link";
import { usePathname } from "next/navigation";

import type { RevenuePeriod } from "@/lib/sales";
import { cn } from "@/lib/utils";

const OPTIONS: { value: RevenuePeriod; label: string }[] = [
  { value: "day", label: "Hoy" },
  { value: "week", label: "7 días" },
  { value: "month", label: "Mes" },
];

export function ReportPeriodFilter({ period }: { period: RevenuePeriod }) {
  const pathname = usePathname();

  return (
    <div className="print-hidden flex flex-wrap gap-1 rounded-2xl border border-border/70 bg-card shadow-sm p-1">
      {OPTIONS.map((option) => {
        const active = option.value === period;
        return (
          <Link
            key={option.value}
            href={`${pathname}?period=${option.value}`}
            className={cn(
              "rounded-xl px-3 py-1.5 text-sm transition",
              active
                ? "bg-primary text-primary-foreground"
                : "text-muted-foreground hover:bg-muted hover:text-foreground"
            )}
          >
            {option.label}
          </Link>
        );
      })}
    </div>
  );
}
