"use client";

import { EmptyState } from "@/components/admin/empty-state";
import { ReportExportButtons } from "@/components/admin/report-export-buttons";
import { FadeIn } from "@/components/motion/fade-in";
import type { CustomerLtvReport } from "@/lib/analytics";
import { downloadCsv } from "@/lib/export-csv";
import { formatCurrency, formatDateTime } from "@/lib/format";

type ReportsCustomerLtvProps = {
  data: CustomerLtvReport;
};

function formatFrequency(avgDaysBetweenVisits: number | null) {
  if (avgDaysBetweenVisits === null) return "—";
  return `cada ${Math.round(avgDaysBetweenVisits)} días`;
}

export function ReportsCustomerLtv({ data }: ReportsCustomerLtvProps) {
  const { rows, totalCustomersWithSales, averageLtv, averageVisitCount, limit } = data;

  const exportExcel = () => {
    downloadCsv(
      "clientes-ltv.csv",
      ["Cliente", "Total gastado", "Visitas", "Frecuencia (días)", "Última visita"],
      rows.map((row) => [
        row.name,
        row.totalSpent,
        row.visitCount,
        row.avgDaysBetweenVisits === null ? "" : Math.round(row.avgDaysBetweenVisits),
        row.lastVisitAt,
      ])
    );
  };

  return (
    <div className="mx-auto max-w-5xl space-y-6">
      <FadeIn className="flex flex-col gap-4 sm:flex-row sm:items-end sm:justify-between">
        <div>
          <p className="text-sm text-muted-foreground">Reportes</p>
          <h1 className="font-heading text-4xl font-semibold">Clientes (LTV)</h1>
          <p className="mt-1 text-muted-foreground">
            Valor histórico total y frecuencia de visita por cliente, desde
            siempre — no cambia con el periodo seleccionado en otros reportes.
          </p>
        </div>
        <div className="flex flex-col items-stretch gap-2 sm:items-end">
          <ReportExportButtons
            onExportExcel={exportExcel}
            excelDisabled={rows.length === 0}
          />
        </div>
      </FadeIn>

      <FadeIn delay={0.05} className="grid gap-4 sm:grid-cols-3">
        <div className="rounded-3xl border border-border/70 bg-card shadow-sm p-5">
          <p className="text-sm text-muted-foreground">LTV promedio</p>
          <p className="font-heading text-3xl font-semibold">
            {formatCurrency(averageLtv)}
          </p>
        </div>
        <div className="rounded-3xl border border-border/70 bg-card shadow-sm p-5">
          <p className="text-sm text-muted-foreground">Visitas promedio</p>
          <p className="font-heading text-3xl font-semibold">
            {averageVisitCount.toFixed(1)}
          </p>
        </div>
        <div className="rounded-3xl border border-border/70 bg-card shadow-sm p-5">
          <p className="text-sm text-muted-foreground">Clientes con compras</p>
          <p className="font-heading text-3xl font-semibold">{totalCustomersWithSales}</p>
        </div>
      </FadeIn>

      {rows.length === 0 ? (
        <EmptyState
          title="Sin ventas todavía"
          description="Cuando registres ventas vas a ver aquí el ranking de clientes por valor histórico."
        />
      ) : (
        <FadeIn delay={0.08} className="overflow-hidden rounded-3xl border border-border/70 bg-card shadow-md">
          <div className="overflow-x-auto">
            <table className="w-full min-w-[640px] text-left text-sm">
              <thead className="border-b border-border/70 bg-black/[0.02] text-muted-foreground">
                <tr>
                  <th className="px-4 py-3 font-medium">Cliente</th>
                  <th className="px-4 py-3 font-medium">Total gastado</th>
                  <th className="px-4 py-3 font-medium">Visitas</th>
                  <th className="px-4 py-3 font-medium">Frecuencia</th>
                  <th className="px-4 py-3 font-medium">Última visita</th>
                </tr>
              </thead>
              <tbody>
                {rows.map((row) => (
                  <tr key={row.customerId} className="border-b border-border/50 last:border-0">
                    <td className="px-4 py-3 font-medium">{row.name}</td>
                    <td className="px-4 py-3">{formatCurrency(row.totalSpent)}</td>
                    <td className="px-4 py-3">{row.visitCount}</td>
                    <td className="px-4 py-3">{formatFrequency(row.avgDaysBetweenVisits)}</td>
                    <td className="px-4 py-3">{formatDateTime(row.lastVisitAt)}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
          <p className="border-t border-border/70 px-4 py-3 text-xs text-muted-foreground">
            Mostrando el top {limit} de {totalCustomersWithSales} clientes con compras.
          </p>
        </FadeIn>
      )}
    </div>
  );
}
