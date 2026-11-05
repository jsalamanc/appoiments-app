"use client";

import { useSearchParams } from "next/navigation";
import { useEffect, useState } from "react";

import { ModuleGate } from "@/components/admin/module-gate";
import { OwnerGate } from "@/components/admin/owner-gate";
import { ReportsTrends } from "@/components/admin/reports-trends";
import { getRevenueTrendsReport, type RevenueTrendsReport } from "@/lib/analytics";
import { getReportPeriod } from "@/lib/reports";
import { getPreviousPeriodRange } from "@/lib/sales";

export default function ReportsTrendsPage() {
  const searchParams = useSearchParams();
  const { period, range } = getReportPeriod(searchParams.get("period") ?? undefined);

  const [data, setData] = useState<RevenueTrendsReport | null>(null);

  useEffect(() => {
    let cancelled = false;
    const { from: previousFrom } = getPreviousPeriodRange(period);
    getRevenueTrendsReport(period, range.from, previousFrom).then((r) => {
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
        <ReportsTrends period={period} data={data} />
      </ModuleGate>
    </OwnerGate>
  );
}
