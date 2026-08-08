"use client";

import { EmptyState } from "@/components/admin/empty-state";
import { ReportExportButtons } from "@/components/admin/report-export-buttons";
import { ReportPeriodFilter } from "@/components/admin/report-period-filter";
import { FadeIn } from "@/components/motion/fade-in";
import { downloadCsv, periodLabel } from "@/lib/export-csv";
import { formatCurrency } from "@/lib/format";
import type { TopServiceRow } from "@/lib/reports";
import type { RevenuePeriod } from "@/lib/sales";

type ReportsServicesProps = {
  period: RevenuePeriod;
  salesCount: number;
  totalRevenue: number;
  averageTicket: number;
  rows: TopServiceRow[];
};

export function ReportsServices({
  period,
  salesCount,
  totalRevenue,
  averageTicket,
  rows,
}: ReportsServicesProps) {
  const exportExcel = () => {
    downloadCsv(
      `servicios-ticket-${periodLabel(period)}.csv`,
      ["#", "Servicio", "Cantidad", "Ingresos", "Ventas periodo", "Ticket promedio"],
      rows.map((row, index) => [
        index + 1,
        row.name,
        row.quantity,
        row.revenue,
        salesCount,
        Number(averageTicket.toFixed(2)),
      ])
    );
  };

  return (
    <div className="mx-auto max-w-5xl space-y-6">
      <FadeIn className="flex flex-col gap-4 sm:flex-row sm:items-end sm:justify-between">
        <div>
          <p className="text-sm text-muted-foreground">Reportes</p>
          <h1 className="font-heading text-4xl font-semibold">
            Servicios y ticket
          </h1>
          <p className="mt-1 text-muted-foreground">
            Servicios más vendidos y ticket promedio del periodo.
          </p>
        </div>
        <div className="flex flex-col items-stretch gap-2 sm:items-end">
          <ReportPeriodFilter period={period} />
          <ReportExportButtons
            onExportExcel={exportExcel}
            excelDisabled={rows.length === 0}
          />
        </div>
      </FadeIn>

      <FadeIn delay={0.05} className="grid gap-4 sm:grid-cols-3">
        <div className="rounded-3xl border border-border/70 bg-card shadow-sm p-5">
          <p className="text-sm text-muted-foreground">Ventas</p>
          <p className="font-heading text-3xl font-semibold">{salesCount}</p>
        </div>
        <div className="rounded-3xl border border-border/70 bg-card shadow-sm p-5">
          <p className="text-sm text-muted-foreground">Ingresos</p>
          <p className="font-heading text-3xl font-semibold">
            {formatCurrency(totalRevenue)}
          </p>
        </div>
        <div className="rounded-3xl border border-border/70 bg-card shadow-sm p-5">
          <p className="text-sm text-muted-foreground">Ticket promedio</p>
          <p className="font-heading text-3xl font-semibold">
            {formatCurrency(averageTicket)}
          </p>
        </div>
      </FadeIn>

      {rows.length === 0 ? (
        <EmptyState
          title="Sin servicios vendidos"
          description="Cuando cobres servicios en el POS, aparecerán aquí ordenados por cantidad."
        />
      ) : (
        <FadeIn delay={0.08} className="overflow-hidden rounded-3xl border border-border/70 bg-card shadow-md">
          <div className="overflow-x-auto">
            <table className="w-full min-w-[480px] text-left text-sm">
              <thead className="border-b border-border/70 bg-black/[0.02] text-muted-foreground">
                <tr>
                  <th className="px-4 py-3 font-medium">#</th>
                  <th className="px-4 py-3 font-medium">Servicio</th>
                  <th className="px-4 py-3 font-medium">Cantidad</th>
                  <th className="px-4 py-3 font-medium">Ingresos</th>
                </tr>
              </thead>
              <tbody>
                {rows.map((row, index) => (
                  <tr
                    key={row.serviceId ?? row.name}
                    className="border-b border-border/50 last:border-0"
                  >
                    <td className="px-4 py-3 text-muted-foreground">
                      {index + 1}
                    </td>
                    <td className="px-4 py-3 font-medium">{row.name}</td>
                    <td className="px-4 py-3">{row.quantity}</td>
                    <td className="px-4 py-3">{formatCurrency(row.revenue)}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        </FadeIn>
      )}
    </div>
  );
}
