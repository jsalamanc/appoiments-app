import { SaleForm } from "@/components/admin/sale-form";

export default async function SalesPage({
  searchParams,
}: {
  searchParams: Promise<{
    customerId?: string;
    serviceId?: string;
    appointmentId?: string;
  }>;
}) {
  const params = await searchParams;

  return (
    <SaleForm
      prefill={{
        customerId: params.customerId,
        serviceId: params.serviceId,
        appointmentId: params.appointmentId,
      }}
    />
  );
}
