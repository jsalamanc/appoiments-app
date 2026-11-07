"use client";

import { useEffect, useState } from "react";

import { ModuleGate } from "@/components/admin/module-gate";
import { OwnerGate } from "@/components/admin/owner-gate";
import { ReportsCustomerLtv } from "@/components/admin/reports-customer-ltv";
import { getCustomerLtvReport, type CustomerLtvReport } from "@/lib/analytics";
import { ReportPageSkeleton } from "@/components/admin/skeletons";
import { useMinLoadingElapsed } from "@/lib/use-min-loading";

const TOP_N = 20;

export default function ReportsCustomerLtvPage() {
  const [data, setData] = useState<CustomerLtvReport | null>(null);

  useEffect(() => {
    let cancelled = false;
    getCustomerLtvReport(TOP_N).then((r) => {
      if (!cancelled) setData(r);
    });
    return () => {
      cancelled = true;
    };
  }, []);

  const minElapsed = useMinLoadingElapsed();
  if (!data || !minElapsed) return <ReportPageSkeleton />;

  return (
    <OwnerGate>
      <ModuleGate module="reports">
        <ReportsCustomerLtv data={data} />
      </ModuleGate>
    </OwnerGate>
  );
}
