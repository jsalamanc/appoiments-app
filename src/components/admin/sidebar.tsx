"use client";

import { motion } from "framer-motion";
import {
  CalendarDays,
  History,
  LayoutDashboard,
  LogOut,
  Menu,
  Package,
  Receipt,
  Settings,
  ShoppingBag,
  Sparkles,
  Users,
} from "lucide-react";
import Link from "next/link";
import { usePathname, useRouter } from "next/navigation";
import { useEffect, useMemo, useState } from "react";

import { Button } from "@/components/ui/button";
import {
  Sheet,
  SheetContent,
  SheetDescription,
  SheetHeader,
  SheetTitle,
} from "@/components/ui/sheet";
import {
  isModuleEnabled,
  parseCenterModules,
  type ModuleKey,
} from "@/lib/modules";
import { createClient } from "@/lib/supabase/client";
import { cn } from "@/lib/utils";
import type { Center } from "@/types/database";

type NavLink = {
  href: string;
  label: string;
  icon: typeof LayoutDashboard;
  exact?: boolean;
  module?: ModuleKey;
};

const links: NavLink[] = [
  { href: "/admin", label: "Panel", icon: LayoutDashboard, exact: true },
  {
    href: "/admin/appointments",
    label: "Agenda",
    icon: CalendarDays,
    exact: true,
    module: "appointments",
  },
  { href: "/admin/sales", label: "Nueva venta", icon: Receipt, exact: true },
  {
    href: "/admin/sales/history",
    label: "Historial",
    icon: History,
    exact: true,
  },
  { href: "/admin/employees", label: "Empleados", icon: Users, module: "employees" },
  { href: "/admin/products", label: "Productos", icon: ShoppingBag },
  { href: "/admin/services", label: "Servicios", icon: Sparkles, module: "services" },
  { href: "/admin/packages", label: "Paquetes", icon: Package, module: "packages" },
  { href: "/admin/settings", label: "Ajustes", icon: Settings },
];

function NavLinks({
  pathname,
  onNavigate,
  layoutId,
  visibleLinks,
}: {
  pathname: string;
  onNavigate?: () => void;
  layoutId: string;
  visibleLinks: NavLink[];
}) {
  return (
    <nav className="flex min-h-0 flex-1 flex-col gap-1 overflow-y-auto">
      {visibleLinks.map((link) => {
        const active = link.exact
          ? pathname === link.href
          : pathname.startsWith(link.href);
        const Icon = link.icon;

        return (
          <Link
            key={link.href}
            href={link.href}
            onClick={onNavigate}
            className={cn(
              "relative flex items-center gap-2 rounded-xl px-3 py-2.5 text-sm whitespace-nowrap transition",
              active
                ? "text-sidebar-accent-foreground"
                : "text-muted-foreground hover:bg-sidebar-accent/50 hover:text-foreground"
            )}
          >
            {active && (
              <motion.span
                layoutId={layoutId}
                className="absolute inset-0 rounded-xl bg-sidebar-accent"
                transition={{ type: "spring", stiffness: 380, damping: 30 }}
              />
            )}
            <Icon className="relative z-10 size-4" />
            <span className="relative z-10">{link.label}</span>
          </Link>
        );
      })}
    </nav>
  );
}

function Brand({ center }: { center: Center | null }) {
  return (
    <div className="px-2">
      <Link href="/admin" className="font-heading text-2xl font-semibold">
        Aura Studio
      </Link>
      <p className="mt-1 truncate text-sm text-muted-foreground">
        {center?.name ?? "Tu centro"}
      </p>
    </div>
  );
}

export function AdminSidebar({ center }: { center: Center | null }) {
  const pathname = usePathname();
  const router = useRouter();
  const [open, setOpen] = useState(false);

  const modules = useMemo(
    () => parseCenterModules(center?.modules),
    [center?.modules]
  );

  const visibleLinks = useMemo(
    () =>
      links.filter(
        (link) => !link.module || isModuleEnabled(modules, link.module)
      ),
    [modules]
  );

  useEffect(() => {
    setOpen(false);
  }, [pathname]);

  const signOut = async () => {
    const supabase = createClient();
    await supabase.auth.signOut();
    setOpen(false);
    router.push("/login");
    router.refresh();
  };

  return (
    <>
      <header className="flex shrink-0 items-center justify-between border-b border-sidebar-border bg-sidebar/80 px-4 py-3 backdrop-blur lg:hidden">
        <div className="min-w-0">
          <p className="font-heading text-xl font-semibold">Aura Studio</p>
          <p className="truncate text-xs text-muted-foreground">
            {center?.name ?? "Tu centro"}
          </p>
        </div>
        <Button
          type="button"
          variant="outline"
          size="icon"
          aria-label="Abrir menú"
          onClick={() => setOpen(true)}
        >
          <Menu />
        </Button>
      </header>

      <Sheet open={open} onOpenChange={setOpen}>
        <SheetContent
          side="left"
          className="w-[min(100%,20rem)] gap-0 bg-sidebar p-0 text-sidebar-foreground data-[side=left]:sm:max-w-xs"
        >
          <SheetHeader className="border-b border-sidebar-border px-4 py-5 text-left">
            <SheetTitle className="font-heading text-2xl font-semibold">
              Aura Studio
            </SheetTitle>
            <SheetDescription className="truncate">
              {center?.name ?? "Tu centro"}
            </SheetDescription>
          </SheetHeader>

          <div className="flex h-full min-h-0 flex-1 flex-col px-3 py-4">
            <NavLinks
              pathname={pathname}
              layoutId="admin-nav-mobile"
              onNavigate={() => setOpen(false)}
              visibleLinks={visibleLinks}
            />
            <Button
              variant="ghost"
              className="mt-4 shrink-0 justify-start"
              onClick={signOut}
            >
              <LogOut data-icon="inline-start" />
              Cerrar sesión
            </Button>
          </div>
        </SheetContent>
      </Sheet>

      <aside className="hidden h-full w-64 shrink-0 flex-col border-r border-sidebar-border bg-sidebar/80 px-4 py-5 backdrop-blur lg:flex">
        <div className="mb-8 shrink-0">
          <Brand center={center} />
        </div>

        <NavLinks
          pathname={pathname}
          layoutId="admin-nav-desktop"
          visibleLinks={visibleLinks}
        />

        <Button
          variant="ghost"
          className="mt-4 shrink-0 justify-start"
          onClick={signOut}
        >
          <LogOut data-icon="inline-start" />
          Cerrar sesión
        </Button>
      </aside>
    </>
  );
}
