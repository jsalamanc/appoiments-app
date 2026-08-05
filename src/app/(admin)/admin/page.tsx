import Link from "next/link";
import {
  History,
  Package,
  Receipt,
  ShoppingBag,
  Sparkles,
} from "lucide-react";

import { RevenueMetrics } from "@/components/admin/revenue-metrics";
import { FadeIn, StaggerChildren, StaggerItem } from "@/components/motion/fade-in";
import { Button } from "@/components/ui/button";
import { getServerCenter } from "@/lib/supabase/center";
import { createClient } from "@/lib/supabase/server";
import { getRevenueMetrics } from "@/lib/sales-server";

export default async function AdminDashboardPage() {
  const supabase = await createClient();
  const center = await getServerCenter();

  const [products, services, packages, revenue] = await Promise.all([
    supabase.from("products").select("id", { count: "exact", head: true }),
    supabase.from("services").select("id", { count: "exact", head: true }),
    supabase.from("packages").select("id", { count: "exact", head: true }),
    getRevenueMetrics(),
  ]);

  const stats = [
    {
      label: "Productos",
      count: products.count ?? 0,
      href: "/admin/products",
      icon: ShoppingBag,
      tone: "bg-blush/50",
    },
    {
      label: "Servicios",
      count: services.count ?? 0,
      href: "/admin/services",
      icon: Sparkles,
      tone: "bg-mint/60",
    },
    {
      label: "Paquetes",
      count: packages.count ?? 0,
      href: "/admin/packages",
      icon: Package,
      tone: "bg-lavender/60",
    },
  ];

  return (
    <div className="mx-auto max-w-5xl space-y-8">
      <FadeIn>
        <p className="text-sm text-muted-foreground">Panel administrativo</p>
        <h1 className="font-heading text-4xl font-semibold tracking-tight">
          Hola, {center?.name ?? "admin"}
        </h1>
        <p className="mt-2 max-w-xl text-muted-foreground">
          Administra tu catálogo, registra ventas y revisa tus ingresos.
        </p>
      </FadeIn>

      <RevenueMetrics
        totalRevenue={revenue.totalRevenue}
        dayRevenue={revenue.dayRevenue}
        weekRevenue={revenue.weekRevenue}
        monthRevenue={revenue.monthRevenue}
        salesCount={revenue.salesCount}
        payments={revenue.payments}
      />

      <FadeIn
        delay={0.08}
        className="flex flex-wrap gap-2 rounded-3xl border border-border/70 bg-card/60 p-5"
      >
        <Button render={<Link href="/admin/sales" />}>
          <Receipt data-icon="inline-start" />
          Nueva venta
        </Button>
        <Button variant="outline" render={<Link href="/admin/sales/history" />}>
          <History data-icon="inline-start" />
          Historial
        </Button>
      </FadeIn>

      <StaggerChildren className="grid gap-4 sm:grid-cols-3">
        {stats.map((stat) => {
          const Icon = stat.icon;
          return (
            <StaggerItem key={stat.label}>
              <Link
                href={stat.href}
                className="block rounded-3xl border border-border/70 bg-card/70 p-5 transition hover:border-primary/40 hover:bg-card"
              >
                <div
                  className={`mb-4 inline-flex rounded-2xl p-3 ${stat.tone}`}
                >
                  <Icon className="size-5" />
                </div>
                <p className="text-sm text-muted-foreground">{stat.label}</p>
                <p className="font-heading text-4xl font-semibold">
                  {stat.count}
                </p>
              </Link>
            </StaggerItem>
          );
        })}
      </StaggerChildren>
    </div>
  );
}
