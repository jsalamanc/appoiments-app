import { notFound } from "next/navigation";

import { CustomerDetail } from "@/components/admin/customer-detail";
import { getCustomerDetail } from "@/lib/customers-server";
import { parseCenterModules } from "@/lib/modules";
import { canUseModule } from "@/lib/plan-modules";
import { getServerCenterContext } from "@/lib/supabase/center";

export default async function CustomerDetailPage({
  params,
}: {
  params: Promise<{ id: string }>;
}) {
  const { id } = await params;
  const [detail, ctx] = await Promise.all([
    getCustomerDetail(id),
    getServerCenterContext(),
  ]);

  if (!detail) notFound();

  const modules = parseCenterModules(ctx?.center.modules);

  return (
    <CustomerDetail
      customer={detail.customer}
      sales={detail.sales}
      appointments={detail.appointments}
      appointmentsEnabled={canUseModule(
        modules,
        ctx?.planAllowed,
        "appointments"
      )}
    />
  );
}
