"use client";

import { motion } from "framer-motion";
import { useState } from "react";

import { FadeIn } from "@/components/motion/fade-in";
import { formatCurrency } from "@/lib/format";
import type { RevenuePeriod } from "@/lib/sales";
import { cn } from "@/lib/utils";

type PaymentStats = {
  cashCount: number;
  transferCount: number;
  cardCount: number;
  cashRevenue: number;
  transferRevenue: number;
  cardRevenue: number;
};

const PAYMENT_METHODS = [
  { key: "cash", label: "Efectivo", tone: "bg-blush/40", bar: "bg-blush" },
  { key: "transfer", label: "Transferencia", tone: "bg-mint/50", bar: "bg-mint" },
  { key: "card", label: "Tarjeta", tone: "bg-lavender/50", bar: "bg-lavender" },
] as const;

type PaymentPeriod = RevenuePeriod | "all";

type RevenueMetricsProps = {
  totalRevenue: number;
  dayRevenue: number;
  weekRevenue: number;
  monthRevenue: number;
  salesCount: number;
  payments: {
    day: PaymentStats;
    week: PaymentStats;
    month: PaymentStats;
    all: PaymentStats;
  };
};

const PERIODS: { value: RevenuePeriod; label: string }[] = [
  { value: "day", label: "Hoy" },
  { value: "week", label: "Semana" },
  { value: "month", label: "Mes" },
];

const PAYMENT_PERIODS: { value: PaymentPeriod; label: string }[] = [
  { value: "day", label: "Hoy" },
  { value: "week", label: "Semana" },
  { value: "month", label: "Mes" },
  { value: "all", label: "Todo" },
];

function periodHint(period: PaymentPeriod) {
  if (period === "day") return "Desde las 00:00 de hoy";
  if (period === "week") return "Últimos 7 días";
  if (period === "month") return "Mes calendario actual";
  return "Histórico completo";
}

export function RevenueMetrics({
  totalRevenue,
  dayRevenue,
  weekRevenue,
  monthRevenue,
  salesCount,
  payments,
}: RevenueMetricsProps) {
  const [period, setPeriod] = useState<RevenuePeriod>("day");
  const [paymentPeriod, setPaymentPeriod] = useState<PaymentPeriod>("day");

  const periodRevenue =
    period === "day"
      ? dayRevenue
      : period === "week"
        ? weekRevenue
        : monthRevenue;

  const paymentStats = payments[paymentPeriod];
  const paymentTotal =
    paymentStats.cashCount + paymentStats.transferCount + paymentStats.cardCount;
  const countFor = (key: (typeof PAYMENT_METHODS)[number]["key"]) =>
    key === "cash"
      ? paymentStats.cashCount
      : key === "transfer"
        ? paymentStats.transferCount
        : paymentStats.cardCount;
  const revenueFor = (key: (typeof PAYMENT_METHODS)[number]["key"]) =>
    key === "cash"
      ? paymentStats.cashRevenue
      : key === "transfer"
        ? paymentStats.transferRevenue
        : paymentStats.cardRevenue;
  const percentFor = (key: (typeof PAYMENT_METHODS)[number]["key"]) =>
    paymentTotal === 0 ? 0 : Math.round((countFor(key) / paymentTotal) * 100);

  return (
    <div className="space-y-4">
      <FadeIn className="grid gap-4 md:grid-cols-2">
        <div className="rounded-3xl border border-border/70 bg-card shadow-sm p-5">
          <p className="text-sm text-muted-foreground">Ingresos totales</p>
          <p className="mt-2 font-heading text-4xl font-semibold">
            {formatCurrency(totalRevenue)}
          </p>
          <p className="mt-2 text-sm text-muted-foreground">
            {salesCount} venta{salesCount === 1 ? "" : "s"} registrada
            {salesCount === 1 ? "" : "s"}
          </p>
        </div>

        <div className="rounded-3xl border border-border/70 bg-card shadow-sm p-5">
          <div className="flex flex-wrap items-center justify-between gap-2">
            <p className="text-sm text-muted-foreground">Ingresos por periodo</p>
            <div className="flex gap-1 rounded-full bg-muted/70 p-1">
              {PERIODS.map((item) => (
                <button
                  key={item.value}
                  type="button"
                  onClick={() => setPeriod(item.value)}
                  className={cn(
                    "rounded-full px-3 py-1 text-xs transition",
                    period === item.value
                      ? "bg-primary text-primary-foreground"
                      : "text-muted-foreground hover:text-foreground"
                  )}
                >
                  {item.label}
                </button>
              ))}
            </div>
          </div>
          <p className="mt-2 font-heading text-4xl font-semibold">
            {formatCurrency(periodRevenue)}
          </p>
          <p className="mt-2 text-sm text-muted-foreground">
            {periodHint(period)}
          </p>
        </div>
      </FadeIn>

      <FadeIn
        delay={0.06}
        className="rounded-3xl border border-border/70 bg-card shadow-sm p-5"
      >
        <div className="flex flex-col gap-3 sm:flex-row sm:items-end sm:justify-between">
          <div>
            <p className="text-sm text-muted-foreground">Métodos de pago</p>
            <h2 className="font-heading text-2xl font-semibold">
              Efectivo, transferencia y tarjeta
            </h2>
            <p className="mt-1 text-sm text-muted-foreground">
              {periodHint(paymentPeriod)}
            </p>
          </div>
          <div className="flex gap-1 rounded-full bg-muted/70 p-1">
            {PAYMENT_PERIODS.map((item) => (
              <button
                key={item.value}
                type="button"
                onClick={() => setPaymentPeriod(item.value)}
                className={cn(
                  "rounded-full px-3 py-1 text-xs transition",
                  paymentPeriod === item.value
                    ? "bg-primary text-primary-foreground"
                    : "text-muted-foreground hover:text-foreground"
                )}
              >
                {item.label}
              </button>
            ))}
          </div>
        </div>

        {paymentTotal === 0 ? (
          <p className="mt-6 rounded-2xl border border-dashed border-border px-4 py-8 text-center text-sm text-muted-foreground">
            No hay pagos en este periodo.
          </p>
        ) : (
          <div className="mt-6 space-y-5">
            <div className="flex items-center justify-between text-sm text-muted-foreground">
              <span>
                {paymentTotal} pago{paymentTotal === 1 ? "" : "s"}
              </span>
              <span>
                {formatCurrency(
                  paymentStats.cashRevenue +
                    paymentStats.transferRevenue +
                    paymentStats.cardRevenue
                )}
              </span>
            </div>

            <div className="flex h-4 overflow-hidden rounded-full bg-muted">
              {PAYMENT_METHODS.map((method) => (
                <motion.div
                  key={`${method.key}-${paymentPeriod}`}
                  className={cn("h-full", method.bar)}
                  initial={{ width: 0 }}
                  animate={{ width: `${percentFor(method.key)}%` }}
                  transition={{ duration: 0.55, ease: "easeOut" }}
                />
              ))}
            </div>

            <div className="grid gap-3 sm:grid-cols-3">
              {PAYMENT_METHODS.map((method) => (
                <div key={method.key} className={cn("rounded-2xl px-4 py-4", method.tone)}>
                  <div className="flex items-center justify-between gap-2">
                    <p className="text-sm text-muted-foreground">{method.label}</p>
                    <span className="text-xs text-muted-foreground">
                      {percentFor(method.key)}%
                    </span>
                  </div>
                  <p className="mt-1 font-heading text-3xl font-semibold">
                    {countFor(method.key)}
                  </p>
                  <p className="mt-1 text-sm text-muted-foreground">
                    {formatCurrency(revenueFor(method.key))}
                  </p>
                </div>
              ))}
            </div>
          </div>
        )}
      </FadeIn>
    </div>
  );
}
