"use client";

import { invoke } from "@tauri-apps/api/core";
import { useSearchParams } from "next/navigation";
import { useEffect, useState } from "react";

import { ModuleGate } from "@/components/admin/module-gate";
import { OwnerGate } from "@/components/admin/owner-gate";
import { ReportsEmployees } from "@/components/admin/reports-employees";
import { getReportPeriod } from "@/lib/reports";
import type { EmployeePerformanceRow, ReportPeriod } from "@/lib/reports";

type EmployeePerformanceReport = {
  period: ReportPeriod;
  rows: EmployeePerformanceRow[];
};

export default function ReportsEmployeesPage() {
  const searchParams = useSearchParams();
  const { period, range } = getReportPeriod(searchParams.get("period") ?? undefined);

  const [data, setData] = useState<EmployeePerformanceReport | null>(null);

  useEffect(() => {
    let cancelled = false;
    invoke<EmployeePerformanceReport>("get_employee_performance_report", {
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
        <ReportsEmployees period={data.period} rows={data.rows} />
      </ModuleGate>
    </OwnerGate>
  );
}
