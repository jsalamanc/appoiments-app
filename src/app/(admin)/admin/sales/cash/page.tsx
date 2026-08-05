import { CashDay } from "@/components/admin/cash-day";
import { getCashDaySummary } from "@/lib/sales-server";

export default async function CashDayPage() {
  const summary = await getCashDaySummary();

  return (
    <CashDay
      sales={summary.sales}
      salesCount={summary.salesCount}
      totalRevenue={summary.totalRevenue}
      payments={summary.payments}
    />
  );
}
