"use client";

import { invoke } from "@tauri-apps/api/core";
import { useSearchParams } from "next/navigation";
import { useEffect, useState } from "react";

import { ModuleGate } from "@/components/admin/module-gate";
import { OwnerGate } from "@/components/admin/owner-gate";
import { ReportsNoShows } from "@/components/admin/reports-no-shows";
import { getReportPeriod } from "@/lib/reports";
import type { NoShowCustomerRow, ReportPeriod } from "@/lib/reports";

type RecentNoShowRow = {
  id: string;
  status: "no_show" | "cancelled";
  starts_at: string;
  customers: { id: string; name: string; email: string; phone: string } | null;
};

type NoShowsReport = {
  period: ReportPeriod;
  noShowTotal: number;
  cancelledTotal: number;
  rows: NoShowCustomerRow[];
  recent: RecentNoShowRow[];
};

export default function ReportsNoShowsPage() {
  const searchParams = useSearchParams();
  const { period } = getReportPeriod(searchParams.get("period") ?? undefined);

  const [data, setData] = useState<NoShowsReport | null>(null);

  useEffect(() => {
    let cancelled = false;
    invoke<NoShowsReport>("get_no_shows_report", { period }).then((r) => {
      if (!cancelled) setData(r);
    });
    return () => {
      cancelled = true;
    };
  }, [period]);

  if (!data) return null;

  return (
    <OwnerGate>
      <ModuleGate module="reports">
        <ReportsNoShows
          period={data.period}
          noShowTotal={data.noShowTotal}
          cancelledTotal={data.cancelledTotal}
          rows={data.rows}
          recent={data.recent}
        />
      </ModuleGate>
    </OwnerGate>
  );
}
