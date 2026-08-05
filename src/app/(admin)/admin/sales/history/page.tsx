import { SalesHistory } from "@/components/admin/sales-history";
import { listSales } from "@/lib/sales-server";
import type { PaymentMethod } from "@/types/database";

export default async function SalesHistoryPage({
  searchParams,
}: {
  searchParams: Promise<{
    page?: string;
    payment?: string;
    highlight?: string;
  }>;
}) {
  const params = await searchParams;
  const payment =
    params.payment === "cash" || params.payment === "transfer"
      ? (params.payment as PaymentMethod)
      : "all";
  const page = Number.parseInt(params.page ?? "1", 10) || 1;

  const result = await listSales({
    page,
    paymentMethod: payment,
  });

  return (
    <SalesHistory
      sales={result.items}
      page={result.page}
      totalPages={result.totalPages}
      total={result.total}
      pageSize={result.pageSize}
      paymentMethod={result.paymentMethod}
      highlightId={params.highlight}
    />
  );
}
