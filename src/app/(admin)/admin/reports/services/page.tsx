"use client";

import { invoke } from "@tauri-apps/api/core";
import { useSearchParams } from "next/navigation";
import { useEffect, useState } from "react";

import { ModuleGate } from "@/components/admin/module-gate";
import { OwnerGate } from "@/components/admin/owner-gate";
import { ReportsServices } from "@/components/admin/reports-services";
import { getReportPeriod } from "@/lib/reports";
import type { ReportPeriod, TopServiceRow } from "@/lib/reports";

type TopServicesReport = {
  period: ReportPeriod;
  salesCount: number;
  totalRevenue: number;
  averageTicket: number;
  rows: TopServiceRow[];
};

export default function ReportsServicesPage() {
  const searchParams = useSearchParams();
  const { period, range } = getReportPeriod(searchParams.get("period") ?? undefined);

  const [data, setData] = useState<TopServicesReport | null>(null);

  useEffect(() => {
    let cancelled = false;
    invoke<TopServicesReport>("get_top_services_report", {
      period,
      from: range.from,
    }).then((r) => {
      if (!cancelled) setData(r);
    });
    return () => {
      cancelled = true;
    };
  }, [period, range.from]);

  if (!data) return null;

  return (
    <OwnerGate>
      <ModuleGate module="reports">
        <ReportsServices
          period={data.period}
          salesCount={data.salesCount}
          totalRevenue={data.totalRevenue}
          averageTicket={data.averageTicket}
          rows={data.rows}
        />
      </ModuleGate>
    </OwnerGate>
  );
}
