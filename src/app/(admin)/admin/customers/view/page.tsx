"use client";

import { invoke } from "@tauri-apps/api/core";
import { useRouter, useSearchParams } from "next/navigation";
import { useEffect, useState } from "react";

import { CustomerDetail } from "@/components/admin/customer-detail";
import type {
  AppointmentWithDetails,
  Customer,
  SaleWithDetails,
} from "@/types/database";

type Detail = {
  customer: Customer;
  sales: SaleWithDetails[];
  appointments: AppointmentWithDetails[];
};

type ModulesFlags = { appointments: boolean };

export default function CustomerDetailPage() {
  const router = useRouter();
  const id = useSearchParams().get("id");
  const [detail, setDetail] = useState<Detail | null>(null);
  const [appointmentsEnabled, setAppointmentsEnabled] = useState(false);
  const [notFound, setNotFound] = useState(false);

  useEffect(() => {
    if (!id) {
      setNotFound(true);
      return;
    }
    let cancelled = false;
    Promise.all([
      invoke<Detail | null>("get_customer_detail", { id }),
      invoke<ModulesFlags>("get_enabled_modules"),
    ])
      .then(([d, modules]) => {
        if (cancelled) return;
        if (!d) {
          setNotFound(true);
          return;
        }
        setDetail(d);
        setAppointmentsEnabled(modules.appointments);
      })
      .catch(() => {
        if (!cancelled) setNotFound(true);
      });
    return () => {
      cancelled = true;
    };
  }, [id]);

  useEffect(() => {
    if (notFound) router.replace("/admin/customers");
  }, [notFound, router]);

  if (notFound || !detail) return null;

  return (
    <CustomerDetail
      customer={detail.customer}
      sales={detail.sales}
      appointments={detail.appointments}
      appointmentsEnabled={appointmentsEnabled}
    />
  );
}
