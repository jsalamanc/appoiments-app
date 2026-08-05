import { ModuleGate } from "@/components/admin/module-gate";
import { ReportsNoShows } from "@/components/admin/reports-no-shows";
import { getNoShowsReport, getReportPeriod } from "@/lib/reports-server";

type PageProps = {
  searchParams: Promise<{ period?: string }>;
};

export default async function ReportsNoShowsPage({ searchParams }: PageProps) {
  const params = await searchParams;
  const { period } = getReportPeriod(params.period);
  const data = await getNoShowsReport(period);

  return (
    <ModuleGate module="appointments">
      <ReportsNoShows
        period={data.period}
        noShowTotal={data.noShowTotal}
        cancelledTotal={data.cancelledTotal}
        rows={data.rows}
        recent={data.recent}
      />
    </ModuleGate>
  );
}
