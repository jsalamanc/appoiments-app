import Link from "next/link";
import {
  CalendarDays,
  History,
  Package,
  Receipt,
  ShoppingBag,
  Sparkles,
  Users,
} from "lucide-react";

import { RevenueMetrics } from "@/components/admin/revenue-metrics";
import { FadeIn, StaggerChildren, StaggerItem } from "@/components/motion/fade-in";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import {
  isModuleEnabled,
  parseCenterModules,
  type ModuleKey,
} from "@/lib/modules";
import { getRevenueMetrics } from "@/lib/sales-server";
import { getServerCenter } from "@/lib/supabase/center";
import { createClient } from "@/lib/supabase/server";
import { cn } from "@/lib/utils";

type StatCard = {
  label: string;
  count: number;
  href: string;
  icon: typeof ShoppingBag;
  tone: string;
  module?: ModuleKey;
};

export default async function AdminDashboardPage() {
  const supabase = await createClient();
  const center = await getServerCenter();
  const modules = parseCenterModules(center?.modules);

  const [products, services, packages, employees, appointments, revenue] =
    await Promise.all([
      supabase.from("products").select("id", { count: "exact", head: true }),
      isModuleEnabled(modules, "services")
        ? supabase.from("services").select("id", { count: "exact", head: true })
        : Promise.resolve({ count: 0 }),
      isModuleEnabled(modules, "packages")
        ? supabase.from("packages").select("id", { count: "exact", head: true })
        : Promise.resolve({ count: 0 }),
      isModuleEnabled(modules, "employees")
        ? supabase.from("employees").select("id", { count: "exact", head: true })
        : Promise.resolve({ count: 0 }),
      isModuleEnabled(modules, "appointments")
        ? supabase
            .from("appointments")
            .select("id", { count: "exact", head: true })
            .eq("status", "scheduled")
        : Promise.resolve({ count: 0 }),
      getRevenueMetrics(),
    ]);

  const stats: StatCard[] = [
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
      module: "services",
    },
    {
      label: "Paquetes",
      count: packages.count ?? 0,
      href: "/admin/packages",
      icon: Package,
      tone: "bg-lavender/60",
      module: "packages",
    },
    {
      label: "Empleados",
      count: employees.count ?? 0,
      href: "/admin/employees",
      icon: Users,
      tone: "bg-cream/80",
      module: "employees",
    },
    {
      label: "Citas programadas",
      count: appointments.count ?? 0,
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
        <Button variant="ghost" render={<Link href="/admin/settings" />}>
          Ajustes de módulos
        </Button>
      </FadeIn>

      <StaggerChildren className="grid gap-4 sm:grid-cols-2 lg:grid-cols-3">
        {stats.map((stat) => {
          const Icon = stat.icon;
          const enabled =
            !stat.module || isModuleEnabled(modules, stat.module);
          const href = enabled ? stat.href : "/admin/settings";

          return (
            <StaggerItem key={stat.label}>
              <Link
                href={href}
                className={cn(
                  "block rounded-3xl border border-border/70 p-5 transition",
                  enabled
                    ? "bg-card/70 hover:border-primary/40 hover:bg-card"
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
