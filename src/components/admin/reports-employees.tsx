"use client";

import { EmptyState } from "@/components/admin/empty-state";
import { ReportExportButtons } from "@/components/admin/report-export-buttons";
import { ReportPeriodFilter } from "@/components/admin/report-period-filter";
import { FadeIn } from "@/components/motion/fade-in";
import { downloadCsv, periodLabel } from "@/lib/export-csv";
import { formatCurrency } from "@/lib/format";
import type { EmployeePerformanceRow } from "@/lib/reports";
import type { RevenuePeriod } from "@/lib/sales";

type ReportsEmployeesProps = {
  period: RevenuePeriod;
  rows: EmployeePerformanceRow[];
};

export function ReportsEmployees({ period, rows }: ReportsEmployeesProps) {
  const totalCompleted = rows.reduce((sum, r) => sum + r.completed, 0);
  const totalRevenue = rows.reduce((sum, r) => sum + r.revenue, 0);

  const exportExcel = () => {
    downloadCsv(
      `rendimiento-empleados-${periodLabel(period)}.csv`,
      [
        "Empleado",
        "Completadas",
        "Programadas",
        "No show",
        "Canceladas",
        "Ingresos",
      ],
      rows.map((row) => [
        row.name,
        row.completed,
        row.scheduled,
        row.noShow,
        row.cancelled,
        row.revenue,
      ])
    );
  };

  return (
    <div className="mx-auto max-w-5xl space-y-6">
      <FadeIn className="flex flex-col gap-4 sm:flex-row sm:items-end sm:justify-between">
        <div>
          <p className="text-sm text-muted-foreground">Reportes</p>
          <h1 className="font-heading text-4xl font-semibold">
            Rendimiento por empleado
          </h1>
          <p className="mt-1 text-muted-foreground">
            Citas y cobros vinculados a cada profesional en el periodo.
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

      <FadeIn delay={0.05} className="grid gap-4 sm:grid-cols-2">
        <div className="rounded-3xl border border-border/70 bg-card/70 p-5">
          <p className="text-sm text-muted-foreground">Citas completadas</p>
          <p className="font-heading text-3xl font-semibold">{totalCompleted}</p>
        </div>
        <div className="rounded-3xl border border-border/70 bg-card/70 p-5">
          <p className="text-sm text-muted-foreground">Ingresos cobrados</p>
          <p className="font-heading text-3xl font-semibold">
            {formatCurrency(totalRevenue)}
          </p>
        </div>
      </FadeIn>

      {rows.length === 0 ? (
        <EmptyState
          title="Sin empleados"
          description="Agrega empleados y agenda citas para ver el rendimiento."
        />
      ) : (
        <FadeIn delay={0.08} className="overflow-hidden rounded-3xl border border-border/70">
          <div className="overflow-x-auto">
            <table className="w-full min-w-[640px] text-left text-sm">
              <thead className="border-b border-border/70 bg-muted/40 text-muted-foreground">
                <tr>
                  <th className="px-4 py-3 font-medium">Empleado</th>
                  <th className="px-4 py-3 font-medium">Completadas</th>
                  <th className="px-4 py-3 font-medium">Programadas</th>
                  <th className="px-4 py-3 font-medium">No show</th>
                  <th className="px-4 py-3 font-medium">Canceladas</th>
                  <th className="px-4 py-3 font-medium">Ingresos</th>
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
                    <td className="px-4 py-3">{row.completed}</td>
                    <td className="px-4 py-3">{row.scheduled}</td>
                    <td className="px-4 py-3">{row.noShow}</td>
                    <td className="px-4 py-3">{row.cancelled}</td>
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
