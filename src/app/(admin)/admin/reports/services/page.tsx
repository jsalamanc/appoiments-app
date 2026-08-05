import { ModuleGate } from "@/components/admin/module-gate";
import { ReportsServices } from "@/components/admin/reports-services";
import { getReportPeriod, getTopServicesReport } from "@/lib/reports-server";

type PageProps = {
  searchParams: Promise<{ period?: string }>;
};

export default async function ReportsServicesPage({ searchParams }: PageProps) {
  const params = await searchParams;
  const { period } = getReportPeriod(params.period);
  const data = await getTopServicesReport(period);

  return (
    <ModuleGate module="services">
      <ReportsServices
        period={data.period}
        salesCount={data.salesCount}
        totalRevenue={data.totalRevenue}
        averageTicket={data.averageTicket}
        rows={data.rows}
      />
    </ModuleGate>
  );
}
