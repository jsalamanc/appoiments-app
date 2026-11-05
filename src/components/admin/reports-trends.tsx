"use client";

import {
  CartesianGrid,
  Line,
  LineChart,
  ResponsiveContainer,
  Tooltip,
  XAxis,
  YAxis,
} from "recharts";

import { EmptyState } from "@/components/admin/empty-state";
import { ReportExportButtons } from "@/components/admin/report-export-buttons";
import { ReportPeriodFilter } from "@/components/admin/report-period-filter";
import { FadeIn } from "@/components/motion/fade-in";
import type { RevenueTrendsReport } from "@/lib/analytics";
import { downloadCsv, periodLabel } from "@/lib/export-csv";
import { formatCurrency } from "@/lib/format";
import type { RevenuePeriod } from "@/lib/sales";
import { cn } from "@/lib/utils";

type ReportsTrendsProps = {
  period: RevenuePeriod;
  data: RevenueTrendsReport;
};

function formatDeltaPercent(deltaPercent: number | null) {
  if (deltaPercent === null) return "Nuevo";
  const sign = deltaPercent > 0 ? "+" : "";
  return `${sign}${deltaPercent.toFixed(1)}%`;
}

export function ReportsTrends({ period, data }: ReportsTrendsProps) {
  const {
    currentRevenue,
    previousRevenue,
    currentSalesCount,
    previousSalesCount,
    deltaPercent,
    points,
  } = data;

  const exportExcel = () => {
    downloadCsv(
      `tendencias-${periodLabel(period)}.csv`,
      ["Periodo", "Ingresos", "Ventas"],
      points.map((point) => [point.label, point.revenue, point.salesCount])
    );
  };

  return (
    <div className="mx-auto max-w-5xl space-y-6">
      <FadeIn className="flex flex-col gap-4 sm:flex-row sm:items-end sm:justify-between">
        <div>
          <p className="text-sm text-muted-foreground">Reportes</p>
          <h1 className="font-heading text-4xl font-semibold">Tendencias</h1>
          <p className="mt-1 text-muted-foreground">
            Ingresos del periodo actual comparados con el periodo anterior, y
            su evolución dentro del periodo.
          </p>
        </div>
        <div className="flex flex-col items-stretch gap-2 sm:items-end">
          <ReportPeriodFilter period={period} />
          <ReportExportButtons
            onExportExcel={exportExcel}
            excelDisabled={points.length === 0}
          />
        </div>
      </FadeIn>

      <FadeIn delay={0.05} className="grid gap-4 sm:grid-cols-3">
        <div className="rounded-3xl border border-border/70 bg-card shadow-sm p-5">
          <p className="text-sm text-muted-foreground">Ingresos periodo actual</p>
          <p className="font-heading text-3xl font-semibold">
            {formatCurrency(currentRevenue)}
          </p>
          <p className="mt-1 text-xs text-muted-foreground">
            {currentSalesCount} {currentSalesCount === 1 ? "venta" : "ventas"}
          </p>
        </div>
        <div className="rounded-3xl border border-border/70 bg-card shadow-sm p-5">
          <p className="text-sm text-muted-foreground">Periodo anterior</p>
          <p className="font-heading text-3xl font-semibold">
            {formatCurrency(previousRevenue)}
          </p>
          <p className="mt-1 text-xs text-muted-foreground">
            {previousSalesCount} {previousSalesCount === 1 ? "venta" : "ventas"}
          </p>
        </div>
        <div className="rounded-3xl border border-border/70 bg-card shadow-sm p-5">
          <p className="text-sm text-muted-foreground">Variación</p>
          <p
            className={cn(
              "font-heading text-3xl font-semibold",
              deltaPercent === null
                ? "text-muted-foreground"
                : deltaPercent > 0
                  ? "text-emerald-600"
                  : deltaPercent < 0
                    ? "text-destructive"
                    : undefined
            )}
          >
            {formatDeltaPercent(deltaPercent)}
          </p>
          <p className="mt-1 text-xs text-muted-foreground">vs. periodo anterior</p>
        </div>
      </FadeIn>

      {points.length === 0 ? (
        <EmptyState
          title="Sin ventas en este periodo"
          description="Cuando registres ventas en este periodo vas a ver aquí la tendencia de ingresos."
        />
      ) : (
        <FadeIn
          delay={0.08}
          className="rounded-3xl border border-border/70 bg-card shadow-md p-5"
        >
          <div className="h-72 w-full">
            <ResponsiveContainer width="100%" height="100%">
              <LineChart data={points} margin={{ top: 8, right: 16, bottom: 0, left: 0 }}>
                <CartesianGrid strokeDasharray="3 3" className="stroke-border/70" />
                <XAxis
                  dataKey="label"
                  tick={{ fontSize: 12 }}
                  stroke="var(--muted-foreground)"
                />
                <YAxis
                  tick={{ fontSize: 12 }}
                  stroke="var(--muted-foreground)"
                  tickFormatter={(value: number) => formatCurrency(value)}
                  width={90}
                />
                <Tooltip
                  formatter={(value) => formatCurrency(Number(value ?? 0))}
                  labelStyle={{ color: "var(--foreground)" }}
                  contentStyle={{
                    backgroundColor: "var(--card)",
                    borderColor: "var(--border)",
                    borderRadius: "0.75rem",
                  }}
                />
                <Line
                  type="monotone"
                  dataKey="revenue"
                  name="Ingresos"
                  stroke="var(--primary)"
                  strokeWidth={2}
                  dot={{ r: 3 }}
                />
              </LineChart>
            </ResponsiveContainer>
          </div>
        </FadeIn>
      )}
    </div>
  );
}
