import { notFound } from "next/navigation";

import { CustomerDetail } from "@/components/admin/customer-detail";
import { getCustomerDetail } from "@/lib/customers-server";
import { isModuleEnabled, parseCenterModules } from "@/lib/modules";
import { getServerCenter } from "@/lib/supabase/center";

export default async function CustomerDetailPage({
  params,
}: {
  params: Promise<{ id: string }>;
}) {
  const { id } = await params;
  const [detail, center] = await Promise.all([
    getCustomerDetail(id),
    getServerCenter(),
  ]);

  if (!detail) notFound();

  const modules = parseCenterModules(center?.modules);

  return (
    <CustomerDetail
      customer={detail.customer}
      sales={detail.sales}
      appointments={detail.appointments}
      appointmentsEnabled={isModuleEnabled(modules, "appointments")}
    />
  );
}
