"use client";

import { invoke } from "@tauri-apps/api/core";
import { useSearchParams } from "next/navigation";
import { useEffect, useState } from "react";

import { SaleReceipt } from "@/components/admin/sale-receipt";
import { getSale } from "@/lib/sales";
import type { SaleWithDetails } from "@/types/database";
import { ReceiptSkeleton } from "@/components/admin/skeletons";
import { useMinLoadingElapsed } from "@/lib/use-min-loading";

type BusinessProfile = { name: string };

export default function SaleReceiptPage() {
  const id = useSearchParams().get("id");
  const [sale, setSale] = useState<SaleWithDetails | null>(null);
  const [business, setBusiness] = useState<BusinessProfile | null>(null);
  const [loadFailed, setLoadFailed] = useState(false);

  useEffect(() => {
    if (!id) return;
    let cancelled = false;
    Promise.all([getSale(id), invoke<BusinessProfile>("get_business_profile")])
      .then(([s, profile]) => {
        if (cancelled) return;
        if (!s) {
          setLoadFailed(true);
          return;
        }
        setSale(s);
        setBusiness(profile);
      })
      .catch(() => {
        if (!cancelled) setLoadFailed(true);
      });
    return () => {
      cancelled = true;
    };
  }, [id]);

  const minElapsed = useMinLoadingElapsed();
  if (!id || loadFailed) {
    return (
      <div className="mx-auto max-w-md py-10 text-center text-muted-foreground">
        No se encontró la venta.
      </div>
    );
  }

  if (!sale || !business || !minElapsed) return <ReceiptSkeleton />;

  return <SaleReceipt sale={sale} businessName={business.name} />;
}
