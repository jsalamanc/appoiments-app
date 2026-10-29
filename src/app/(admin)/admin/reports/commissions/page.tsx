"use client";

import { useSearchParams } from "next/navigation";
import { useEffect, useState } from "react";

import { ModuleGate } from "@/components/admin/module-gate";
import { OwnerGate } from "@/components/admin/owner-gate";
import { ReportsCommissions } from "@/components/admin/reports-commissions";
import { getEmployeeCommissionsReport, type EmployeeCommissionsReport } from "@/lib/commissions";
import { getReportPeriod } from "@/lib/reports";

export default function ReportsCommissionsPage() {
  const searchParams = useSearchParams();
  const { period, range } = getReportPeriod(searchParams.get("period") ?? undefined);

  const [data, setData] = useState<EmployeeCommissionsReport | null>(null);

  useEffect(() => {
    let cancelled = false;
    getEmployeeCommissionsReport(period, range.from).then((r) => {
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
        <ReportsCommissions period={period} data={data} />
      </ModuleGate>
    </OwnerGate>
  );
}
