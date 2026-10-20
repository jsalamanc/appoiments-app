"use client";

import { useSearchParams } from "next/navigation";

import { SaleForm } from "@/components/admin/sale-form";

export default function SalesPage() {
  const searchParams = useSearchParams();

  return (
    <SaleForm
      prefill={{
        customerId: searchParams.get("customerId") ?? undefined,
        serviceId: searchParams.get("serviceId") ?? undefined,
        appointmentId: searchParams.get("appointmentId") ?? undefined,
      }}
    />
  );
}
