"use client";

import { EmptyState } from "@/components/admin/empty-state";
import { ReportExportButtons } from "@/components/admin/report-export-buttons";
import { ReportPeriodFilter } from "@/components/admin/report-period-filter";
import { FadeIn } from "@/components/motion/fade-in";
import type { EmployeeCommissionsReport } from "@/lib/commissions";
import { downloadCsv, periodLabel } from "@/lib/export-csv";
import { formatCurrency } from "@/lib/format";
import type { RevenuePeriod } from "@/lib/sales";

type ReportsCommissionsProps = {
  period: RevenuePeriod;
  data: EmployeeCommissionsReport;
};

export function ReportsCommissions({ period, data }: ReportsCommissionsProps) {
  const { rows, totalCommission } = data;

  const exportExcel = () => {
    downloadCsv(
      `comisiones-${periodLabel(period)}.csv`,
      ["Empleado", "Citas completadas", "Ingresos", "Comisión"],
      rows.map((row) => [
        row.name,
        row.completedAppointments,
        row.revenue,
        row.commissionAmount,
      ])
    );
  };

  return (
    <div className="mx-auto max-w-5xl space-y-6">
      <FadeIn className="flex flex-col gap-4 sm:flex-row sm:items-end sm:justify-between">
        <div>
          <p className="text-sm text-muted-foreground">Reportes</p>
          <h1 className="font-heading text-4xl font-semibold">Comisiones</h1>
          <p className="mt-1 text-muted-foreground">
            Cálculo de comisión por empleado sobre las citas completadas del
            periodo, según las tasas configuradas en cada empleado.
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

      <FadeIn delay={0.05} className="grid gap-4 sm:grid-cols-1">
        <div className="rounded-3xl border border-border/70 bg-card shadow-sm p-5">
          <p className="text-sm text-muted-foreground">Comisión total a pagar</p>
          <p className="font-heading text-3xl font-semibold">
            {formatCurrency(totalCommission)}
          </p>
        </div>
      </FadeIn>

      {rows.length === 0 ? (
        <EmptyState
          title="Sin empleados"
          description="Agrega empleados y configura una tasa de comisión para verla aquí."
        />
      ) : (
        <FadeIn delay={0.08} className="overflow-hidden rounded-3xl border border-border/70 bg-card shadow-md">
          <div className="overflow-x-auto">
            <table className="w-full min-w-[640px] text-left text-sm">
              <thead className="border-b border-border/70 bg-black/[0.02] text-muted-foreground">
                <tr>
                  <th className="px-4 py-3 font-medium">Empleado</th>
                  <th className="px-4 py-3 font-medium">Citas completadas</th>
                  <th className="px-4 py-3 font-medium">Ingresos</th>
                  <th className="px-4 py-3 font-medium">Comisión</th>
                </tr>
              </thead>
              <tbody>
                {rows.map((row) => (
                  <tr
                    key={row.employeeId}
                    className="border-b border-border/50 last:border-0"
                  >
                    <td className="px-4 py-3">
                      <div className="flex items-center gap-2">
                        <span
                          className="size-2.5 shrink-0 rounded-full print-hidden"
                          style={{ backgroundColor: row.color }}
                        />
                        <span className="font-medium">{row.name}</span>
                      </div>
                    </td>
                    <td className="px-4 py-3">{row.completedAppointments}</td>
                    <td className="px-4 py-3">{formatCurrency(row.revenue)}</td>
                    <td className="px-4 py-3 font-medium">
                      {formatCurrency(row.commissionAmount)}
                    </td>
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
