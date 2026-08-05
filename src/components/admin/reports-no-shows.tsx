"use client";

import Link from "next/link";

import { EmptyState } from "@/components/admin/empty-state";
import { ReportExportButtons } from "@/components/admin/report-export-buttons";
import { ReportPeriodFilter } from "@/components/admin/report-period-filter";
import { FadeIn } from "@/components/motion/fade-in";
import { Badge } from "@/components/ui/badge";
import { downloadCsv, periodLabel } from "@/lib/export-csv";
import { formatDateTime } from "@/lib/format";
import type { NoShowCustomerRow } from "@/lib/reports";
import type { RevenuePeriod } from "@/lib/sales";

type RecentRow = {
  id: string;
  status: "no_show" | "cancelled";
  starts_at: string;
  customers: { id: string; name: string; email: string; phone: string } | null;
};

type ReportsNoShowsProps = {
  period: RevenuePeriod;
  noShowTotal: number;
  cancelledTotal: number;
  rows: NoShowCustomerRow[];
  recent: RecentRow[];
};

export function ReportsNoShows({
  period,
  noShowTotal,
  cancelledTotal,
  rows,
  recent,
}: ReportsNoShowsProps) {
  const exportExcel = () => {
    downloadCsv(
      `no-shows-cancelaciones-${periodLabel(period)}.csv`,
      [
        "Cliente",
        "Email",
        "Telefono",
        "No-show",
        "Canceladas",
        "Total",
        "No-shows periodo",
        "Cancelaciones periodo",
      ],
      rows.map((row) => [
        row.name,
        row.email,
        row.phone,
        row.noShowCount,
        row.cancelledCount,
        row.lostCount,
        noShowTotal,
        cancelledTotal,
      ])
    );
  };

  return (
    <div className="mx-auto max-w-5xl space-y-6">
      <FadeIn className="flex flex-col gap-4 sm:flex-row sm:items-end sm:justify-between">
        <div>
          <p className="text-sm text-muted-foreground">Reportes</p>
          <h1 className="font-heading text-4xl font-semibold">
            No-shows y cancelaciones
          </h1>
          <p className="mt-1 text-muted-foreground">
            Clientes con citas perdidas o canceladas en el periodo.
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
          <p className="text-sm text-muted-foreground">No-shows</p>
          <p className="font-heading text-3xl font-semibold">{noShowTotal}</p>
        </div>
        <div className="rounded-3xl border border-border/70 bg-card/70 p-5">
          <p className="text-sm text-muted-foreground">Cancelaciones</p>
          <p className="font-heading text-3xl font-semibold">
            {cancelledTotal}
          </p>
        </div>
      </FadeIn>

      {rows.length === 0 ? (
        <EmptyState
          title="Sin pérdidas en el periodo"
          description="No hay no-shows ni cancelaciones registradas en este rango."
        />
      ) : (
        <>
          <FadeIn delay={0.08} className="overflow-hidden rounded-3xl border border-border/70">
            <div className="border-b border-border/70 px-4 py-3">
              <h2 className="font-heading text-lg font-semibold">
                Por cliente
              </h2>
            </div>
            <div className="overflow-x-auto">
              <table className="w-full min-w-[520px] text-left text-sm">
                <thead className="border-b border-border/70 bg-muted/40 text-muted-foreground">
                  <tr>
                    <th className="px-4 py-3 font-medium">Cliente</th>
                    <th className="px-4 py-3 font-medium">No-show</th>
                    <th className="px-4 py-3 font-medium">Canceladas</th>
                    <th className="px-4 py-3 font-medium">Total</th>
                  </tr>
                </thead>
                <tbody>
                  {rows.map((row) => (
                    <tr
                      key={row.customerId}
                      className="border-b border-border/50 last:border-0"
                    >
                      <td className="px-4 py-3">
                        <Link
                          href={`/admin/customers/${row.customerId}`}
                          className="font-medium hover:underline"
                        >
                          {row.name}
                        </Link>
                        <p className="text-xs text-muted-foreground">
                          {row.phone || row.email}
                        </p>
                      </td>
                      <td className="px-4 py-3">{row.noShowCount}</td>
                      <td className="px-4 py-3">{row.cancelledCount}</td>
                      <td className="px-4 py-3 font-medium">{row.lostCount}</td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          </FadeIn>

          {recent.length > 0 && (
            <FadeIn delay={0.1} className="space-y-3">
              <h2 className="font-heading text-lg font-semibold">
                Últimas citas
              </h2>
              <ul className="space-y-2">
                {recent.map((item) => (
                  <li
                    key={item.id}
                    className="flex flex-wrap items-center justify-between gap-2 rounded-2xl border border-border/70 bg-card/60 px-4 py-3 text-sm"
                  >
                    <div>
                      <p className="font-medium">
                        {item.customers?.name ?? "Cliente"}
                      </p>
                      <p className="text-muted-foreground">
                        {formatDateTime(item.starts_at)}
                      </p>
                    </div>
                    <Badge
                      variant={
                        item.status === "no_show" ? "destructive" : "secondary"
                      }
                    >
                      {item.status === "no_show" ? "No-show" : "Cancelada"}
                    </Badge>
                  </li>
                ))}
              </ul>
            </FadeIn>
          )}
        </>
      )}
    </div>
  );
}
