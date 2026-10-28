"use client";

import { invoke } from "@tauri-apps/api/core";
import { useEffect, useState } from "react";

import { CashDay } from "@/components/admin/cash-day";
import { getPeriodRange } from "@/lib/sales";
import type { SaleWithDetails } from "@/types/database";

type PaymentBreakdown = {
  cash_count: number;
  transfer_count: number;
  card_count: number;
  cash_revenue: number;
  transfer_revenue: number;
  card_revenue: number;
};

type CashDaySummary = {
  sales: SaleWithDetails[];
  sales_count: number;
  total_revenue: number;
  payments: PaymentBreakdown;
  from: string;
};

export default function CashDayPage() {
  const [summary, setSummary] = useState<CashDaySummary | null>(null);
  const [reloadKey, setReloadKey] = useState(0);

  useEffect(() => {
    let cancelled = false;
    const { from } = getPeriodRange("day");
    invoke<CashDaySummary>("get_cash_day_summary", { from }).then((result) => {
      if (!cancelled) setSummary(result);
    });
    return () => {
      cancelled = true;
    };
  }, [reloadKey]);

  if (!summary) return null;

  return (
    <CashDay
      sales={summary.sales}
      salesCount={summary.sales_count}
      totalRevenue={summary.total_revenue}
      payments={{
        cashCount: summary.payments.cash_count,
        transferCount: summary.payments.transfer_count,
        cardCount: summary.payments.card_count,
        cashRevenue: summary.payments.cash_revenue,
        transferRevenue: summary.payments.transfer_revenue,
        cardRevenue: summary.payments.card_revenue,
      }}
      onVoided={() => setReloadKey((k) => k + 1)}
    />
  );
}
