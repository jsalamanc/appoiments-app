"use client";

import { invoke } from "@tauri-apps/api/core";
import { useSearchParams } from "next/navigation";
import { useEffect, useState } from "react";

import { SalesHistory } from "@/components/admin/sales-history";
import type { Sale, SaleWithDetails } from "@/types/database";

type PaymentFilter = Sale["payment_method"] | "all";

type ListSalesResult = {
  items: SaleWithDetails[];
  total: number;
  page: number;
  total_pages: number;
  page_size: number;
  payment_method: PaymentFilter;
};

const VALID_FILTERS: PaymentFilter[] = ["cash", "transfer", "card", "mixed"];

export default function SalesHistoryPage() {
  const searchParams = useSearchParams();
  const rawPayment = searchParams.get("payment");
  const payment: PaymentFilter = VALID_FILTERS.includes(rawPayment as PaymentFilter)
    ? (rawPayment as PaymentFilter)
    : "all";
  const page = Number.parseInt(searchParams.get("page") ?? "1", 10) || 1;
  const highlight = searchParams.get("highlight") ?? undefined;

  const [result, setResult] = useState<ListSalesResult | null>(null);
  const [reloadKey, setReloadKey] = useState(0);

  useEffect(() => {
    let cancelled = false;
    invoke<ListSalesResult>("list_sales", {
      page,
      pageSize: 9,
      paymentMethod: payment,
    }).then((r) => {
      if (!cancelled) setResult(r);
    });
    return () => {
      cancelled = true;
    };
  }, [page, payment, reloadKey]);

  if (!result) return null;

  return (
    <SalesHistory
      sales={result.items}
      page={result.page}
      totalPages={result.total_pages}
      total={result.total}
      pageSize={result.page_size}
      paymentMethod={result.payment_method}
      highlightId={highlight}
      onVoided={() => setReloadKey((k) => k + 1)}
    />
  );
}
