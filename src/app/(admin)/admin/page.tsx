"use client";

import { invoke } from "@tauri-apps/api/core";
import Link from "next/link";
import {
  Banknote,
  CalendarClock,
  CalendarDays,
  History,
  Package,
  PackageOpen,
  Receipt,
  ShoppingBag,
  Sparkles,
  UserRound,
  Users,
} from "lucide-react";
import { useEffect, useState } from "react";

import { RevenueMetrics } from "@/components/admin/revenue-metrics";
import { FadeIn, StaggerChildren, StaggerItem } from "@/components/motion/fade-in";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { type CenterModules, type ModuleKey, DEFAULT_MODULES } from "@/lib/modules";
import { getPeriodRange } from "@/lib/sales";
import { cn } from "@/lib/utils";

type StatCard = {
  label: string;
  count: number;
  href: string;
  icon: typeof ShoppingBag;
  tone: string;
  module?: ModuleKey;
};

type PaymentBreakdown = {
  cash_count: number;
  transfer_count: number;
  card_count: number;
  cash_revenue: number;
  transfer_revenue: number;
  card_revenue: number;
};

type RevenueMetricsData = {
  total_revenue: number;
  day_revenue: number;
  week_revenue: number;
  month_revenue: number;
  sales_count: number;
  payments: {
    day: PaymentBreakdown;
    week: PaymentBreakdown;
    month: PaymentBreakdown;
    all: PaymentBreakdown;
  };
};

function toPaymentStats(p: PaymentBreakdown) {
  return {
    cashCount: p.cash_count,
    transferCount: p.transfer_count,
    cardCount: p.card_count,
    cashRevenue: p.cash_revenue,
    transferRevenue: p.transfer_revenue,
    cardRevenue: p.card_revenue,
  };
}

const FAR_PAST = "1970-01-01T00:00:00.000Z";
const FAR_FUTURE = "2999-01-01T00:00:00.000Z";

export default function AdminDashboardPage() {
  const [businessName, setBusinessName] = useState<string | null>(null);
  const [modules, setModules] = useState<CenterModules>(DEFAULT_MODULES);
  const [counts, setCounts] = useState({
    products: 0,
    services: 0,
    packages: 0,
    employees: 0,
    appointments: 0,
  });
  const [revenue, setRevenue] = useState<RevenueMetricsData | null>(null);

  useEffect(() => {
    let cancelled = false;

    (async () => {
      const [profile, enabledModules] = await Promise.all([
        invoke<{ name: string }>("get_business_profile"),
        invoke<CenterModules>("get_enabled_modules"),
      ]);
      if (cancelled) return;
      setBusinessName(profile.name);
      setModules(enabledModules);

      const [products, services, packages, employees, appointments] = await Promise.all([
        invoke<{ counts: { all: number } }>("list_products", { pageSize: 1 }),
        enabledModules.services
          ? invoke<{ counts: { all: number } }>("list_services", { pageSize: 1 })
          : Promise.resolve({ counts: { all: 0 } }),
        enabledModules.packages
          ? invoke<{ counts: { all: number } }>("list_packages", { pageSize: 1 })
          : Promise.resolve({ counts: { all: 0 } }),
        enabledModules.employees
          ? invoke<unknown[]>("list_employees", {})
          : Promise.resolve([]),
        enabledModules.appointments
          ? invoke<{ status: string }[]>("list_appointments_in_range", {
              from: FAR_PAST,
              to: FAR_FUTURE,
            })
          : Promise.resolve([]),
      ]);

      if (cancelled) return;
      setCounts({
        products: products.counts.all,
        services: services.counts.all,
        packages: packages.counts.all,
        employees: employees.length,
        appointments: appointments.filter((a) => a.status === "scheduled").length,
      });

      // Solo el dueño puede ver los ingresos (get_revenue_metrics exige
      // require_owner en el backend) — el staff ve el resto del panel
      // normalmente, simplemente sin esta tarjeta.
      try {
        const revenueMetrics = await invoke<RevenueMetricsData>("get_revenue_metrics", {
          dayFrom: getPeriodRange("day").from,
          weekFrom: getPeriodRange("week").from,
          monthFrom: getPeriodRange("month").from,
        });
        if (!cancelled) setRevenue(revenueMetrics);
      } catch {
        // Staff: sin permiso para ver ingresos, se omite la tarjeta.
      }
    })();

    return () => {
      cancelled = true;
    };
  }, []);

  const usable = (key: ModuleKey) => modules[key];

  const stats: StatCard[] = [
    {
      label: "Productos",
      count: counts.products,
      href: "/admin/products",
      icon: ShoppingBag,
      tone: "bg-blush/50",
    },
    {
      label: "Servicios",
      count: counts.services,
      href: "/admin/services",
      icon: Sparkles,
      tone: "bg-mint/60",
      module: "services",
    },
    {
      label: "Paquetes",
      count: counts.packages,
      href: "/admin/packages",
      icon: Package,
      tone: "bg-lavender/60",
      module: "packages",
    },
    {
      label: "Empleados",
      count: counts.employees,
      href: "/admin/employees",
      icon: Users,
      tone: "bg-cream/80",
      module: "employees",
    },
    {
      label: "Citas programadas",
      count: counts.appointments,
      href: "/admin/appointments",
      icon: CalendarDays,
      tone: "bg-secondary/70",
      module: "appointments",
    },
  ];

  return (
    <div className="mx-auto max-w-5xl space-y-8">
      <FadeIn>
        <p className="text-sm text-muted-foreground">Panel administrativo</p>
        <h1 className="font-heading text-4xl font-semibold tracking-tight">
          Hola, {businessName ?? "admin"}
        </h1>
        <p className="mt-2 max-w-xl text-muted-foreground">
          Administra tu catálogo, registra ventas y revisa tus ingresos.
        </p>
      </FadeIn>

      {revenue && (
        <RevenueMetrics
          totalRevenue={revenue.total_revenue}
          dayRevenue={revenue.day_revenue}
          weekRevenue={revenue.week_revenue}
          monthRevenue={revenue.month_revenue}
          salesCount={revenue.sales_count}
          payments={{
            day: toPaymentStats(revenue.payments.day),
            week: toPaymentStats(revenue.payments.week),
            month: toPaymentStats(revenue.payments.month),
            all: toPaymentStats(revenue.payments.all),
          }}
        />
      )}

      <FadeIn
        delay={0.08}
        className="flex flex-wrap gap-2 rounded-3xl border border-border/70 bg-card shadow-sm p-5"
      >
        <Button render={<Link href="/admin/sales" />}>
          <Receipt data-icon="inline-start" />
          Nueva venta
        </Button>
        <Button variant="outline" render={<Link href="/admin/sales/cash" />}>
          <Banknote data-icon="inline-start" />
          Caja del día
        </Button>
        <Button variant="outline" render={<Link href="/admin/customers" />}>
          <UserRound data-icon="inline-start" />
          Clientes
        </Button>
        <Button
          variant="outline"
          render={<Link href="/admin/products/inventory" />}
        >
          <PackageOpen data-icon="inline-start" />
          Inventario
        </Button>
        {usable("appointments") && (
          <Button
            variant="outline"
            render={<Link href="/admin/appointments/today" />}
          >
            <CalendarClock data-icon="inline-start" />
            Citas de hoy
          </Button>
        )}
        <Button variant="ghost" render={<Link href="/admin/sales/history" />}>
          <History data-icon="inline-start" />
          Historial
        </Button>
        <Button variant="ghost" render={<Link href="/admin/settings" />}>
          Ajustes
        </Button>
      </FadeIn>

      <StaggerChildren className="grid gap-4 sm:grid-cols-2 lg:grid-cols-3">
        {stats.map((stat) => {
          const Icon = stat.icon;
          const enabled = !stat.module || usable(stat.module);
          const href = enabled ? stat.href : "/admin/settings";

          return (
            <StaggerItem key={stat.label}>
              <Link
                href={href}
                className={cn(
                  "block rounded-3xl border border-border/70 p-5 transition",
                  enabled
                    ? "bg-card shadow-sm hover:border-primary/40 hover:bg-card"
                    : "bg-muted/30 hover:border-border"
                )}
              >
                <div className="mb-4 flex items-start justify-between gap-2">
                  <div
                    className={cn(
                      "inline-flex rounded-2xl p-3",
                      enabled ? stat.tone : "bg-muted"
                    )}
                  >
                    <Icon
                      className={cn(
                        "size-5",
                        !enabled && "text-muted-foreground"
                      )}
                    />
                  </div>
                  {stat.module && (
                    <Badge variant={enabled ? "secondary" : "outline"}>
                      {enabled ? "Activo" : "Deshabilitado"}
                    </Badge>
                  )}
                </div>
                <p className="text-sm text-muted-foreground">{stat.label}</p>
                {enabled ? (
                  <p className="font-heading text-4xl font-semibold">
                    {stat.count}
                  </p>
                ) : (
                  <div className="mt-1 space-y-1">
                    <p className="font-heading text-2xl font-semibold text-muted-foreground">
                      —
                    </p>
                    <p className="text-xs text-muted-foreground">
                      Métrica no disponible: el módulo está deshabilitado.
                      Actívalo en Ajustes.
                    </p>
                  </div>
                )}
              </Link>
            </StaggerItem>
          );
        })}
      </StaggerChildren>
    </div>
  );
}
