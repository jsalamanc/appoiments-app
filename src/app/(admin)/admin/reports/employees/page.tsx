import { ModuleGate } from "@/components/admin/module-gate";
import { ReportsEmployees } from "@/components/admin/reports-employees";
import {
  getEmployeePerformance,
  getReportPeriod,
} from "@/lib/reports-server";

type PageProps = {
  searchParams: Promise<{ period?: string }>;
};

export default async function ReportsEmployeesPage({ searchParams }: PageProps) {
  const params = await searchParams;
  const { period } = getReportPeriod(params.period);
  const data = await getEmployeePerformance(period);

  return (
    <ModuleGate module="appointments">
      <ReportsEmployees period={data.period} rows={data.rows} />
    </ModuleGate>
  );
}
