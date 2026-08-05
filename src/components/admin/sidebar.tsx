"use client";

import { motion } from "framer-motion";
import {
  Banknote,
  BarChart3,
  CalendarClock,
  CalendarDays,
  ChartColumn,
  ChevronDown,
  History,
  LayoutDashboard,
  LogOut,
  Menu,
  Package,
  PackageOpen,
  Receipt,
  Settings,
  ShoppingBag,
  Sparkles,
  UserRound,
  UserX,
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
  type CenterModules,
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

type NavGroup = {
  id: string;
  label: string;
  items: NavLink[];
};

const primaryLinks: NavLink[] = [
  { href: "/admin", label: "Panel", icon: LayoutDashboard, exact: true },
  { href: "/admin/sales", label: "Nueva venta", icon: Receipt, exact: true },
  {
    href: "/admin/appointments/today",
    label: "Hoy",
    icon: CalendarClock,
    exact: true,
    module: "appointments",
  },
];

const navGroups: NavGroup[] = [
  {
    id: "ventas",
    label: "Ventas",
    items: [
      {
        href: "/admin/sales/cash",
        label: "Caja del día",
        icon: Banknote,
        exact: true,
      },
      {
        href: "/admin/sales/history",
        label: "Historial",
        icon: History,
        exact: true,
      },
    ],
  },
  {
    id: "agenda",
    label: "Agenda",
    items: [
      {
        href: "/admin/appointments",
        label: "Calendario",
        icon: CalendarDays,
        exact: true,
        module: "appointments",
      },
    ],
  },
  {
    id: "catalogo",
    label: "Catálogo",
    items: [
      {
        href: "/admin/products",
        label: "Productos",
        icon: ShoppingBag,
        exact: true,
      },
      {
        href: "/admin/products/inventory",
        label: "Inventario",
        icon: PackageOpen,
        exact: true,
      },
      {
        href: "/admin/services",
        label: "Servicios",
        icon: Sparkles,
        module: "services",
      },
      {
        href: "/admin/packages",
        label: "Paquetes",
        icon: Package,
        module: "packages",
      },
    ],
  },
  {
    id: "personas",
    label: "Personas",
    items: [
      { href: "/admin/customers", label: "Clientes", icon: UserRound },
      {
        href: "/admin/employees",
        label: "Empleados",
        icon: Users,
        module: "employees",
      },
    ],
  },
  {
    id: "reportes",
    label: "Reportes",
    items: [
      {
        href: "/admin/reports/employees",
        label: "Por empleado",
        icon: ChartColumn,
        exact: true,
        module: "appointments",
      },
      {
        href: "/admin/reports/services",
        label: "Servicios",
        icon: BarChart3,
        exact: true,
        module: "services",
      },
      {
        href: "/admin/reports/no-shows",
        label: "No-shows",
        icon: UserX,
        exact: true,
        module: "appointments",
      },
    ],
  },
];

function isLinkVisible(link: NavLink, modules: CenterModules) {
  return !link.module || isModuleEnabled(modules, link.module);
}

function isLinkActive(pathname: string, link: NavLink) {
  return link.exact
    ? pathname === link.href
    : pathname.startsWith(link.href);
}

function filterLinks(links: NavLink[], modules: CenterModules) {
  return links.filter((link) => isLinkVisible(link, modules));
}

function NavLinkItem({
  link,
  pathname,
  onNavigate,
  layoutId,
  nested,
}: {
  link: NavLink;
  pathname: string;
  onNavigate?: () => void;
  layoutId: string;
  nested?: boolean;
}) {
  const active = isLinkActive(pathname, link);
  const Icon = link.icon;

  return (
    <Link
      href={link.href}
      onClick={onNavigate}
      className={cn(
        "relative flex items-center gap-2 rounded-xl text-sm whitespace-nowrap transition",
        nested ? "px-3 py-2" : "px-3 py-2.5",
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
}

function NavPanel({
  pathname,
  onNavigate,
  layoutId,
  modules,
}: {
  pathname: string;
  onNavigate?: () => void;
  layoutId: string;
  modules: CenterModules;
}) {
  const visiblePrimary = useMemo(
    () => filterLinks(primaryLinks, modules),
    [modules]
  );

  const visibleGroups = useMemo(
    () =>
      navGroups
        .map((group) => ({
          ...group,
          items: filterLinks(group.items, modules),
        }))
        .filter((group) => group.items.length > 0),
    [modules]
  );

  const activeGroupId = useMemo(() => {
    for (const group of visibleGroups) {
      if (group.items.some((link) => isLinkActive(pathname, link))) {
        return group.id;
      }
    }
    return null;
  }, [pathname, visibleGroups]);

  const [openGroups, setOpenGroups] = useState<Record<string, boolean>>({});

  useEffect(() => {
    if (!activeGroupId) return;
    setOpenGroups((prev) =>
      prev[activeGroupId] ? prev : { ...prev, [activeGroupId]: true }
    );
  }, [activeGroupId]);

  const toggleGroup = (id: string) => {
    setOpenGroups((prev) => ({ ...prev, [id]: !prev[id] }));
  };

  return (
    <div className="flex min-h-0 flex-1 flex-col">
      <nav className="flex min-h-0 flex-1 flex-col gap-4 overflow-y-auto">
        <div className="flex flex-col gap-1">
          {visiblePrimary.map((link) => (
            <NavLinkItem
              key={link.href}
              link={link}
              pathname={pathname}
              onNavigate={onNavigate}
              layoutId={layoutId}
            />
          ))}
        </div>

        <div className="flex flex-col gap-2">
          {visibleGroups.map((group) => {
            const open = openGroups[group.id] ?? false;
            return (
              <div key={group.id} className="flex flex-col gap-0.5">
                <button
                  type="button"
                  onClick={() => toggleGroup(group.id)}
                  className="flex w-full items-center justify-between rounded-lg px-3 py-1.5 text-left text-xs font-medium tracking-wide text-muted-foreground uppercase transition hover:text-foreground"
                  aria-expanded={open}
                >
                  <span>{group.label}</span>
                  <ChevronDown
                    className={cn(
                      "size-3.5 transition-transform",
                      open && "rotate-180"
                    )}
                  />
                </button>
                {open && (
                  <div className="flex flex-col gap-0.5 pb-1">
                    {group.items.map((link) => (
                      <NavLinkItem
                        key={link.href}
                        link={link}
                        pathname={pathname}
                        onNavigate={onNavigate}
                        layoutId={layoutId}
                        nested
                      />
                    ))}
                  </div>
                )}
              </div>
            );
          })}
        </div>
      </nav>

      <div className="mt-3 shrink-0 space-y-1 border-t border-sidebar-border pt-3">
        <NavLinkItem
          link={{
            href: "/admin/settings",
            label: "Ajustes",
            icon: Settings,
          }}
          pathname={pathname}
          onNavigate={onNavigate}
          layoutId={layoutId}
        />
      </div>
    </div>
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
            <NavPanel
              pathname={pathname}
              layoutId="admin-nav-mobile"
              onNavigate={() => setOpen(false)}
              modules={modules}
            />
            <Button
              variant="ghost"
              className="mt-2 shrink-0 justify-start"
              onClick={signOut}
            >
              <LogOut data-icon="inline-start" />
              Cerrar sesión
            </Button>
          </div>
        </SheetContent>
      </Sheet>

      <aside className="hidden h-full w-64 shrink-0 flex-col border-r border-sidebar-border bg-sidebar/80 px-4 py-5 backdrop-blur lg:flex">
        <div className="mb-6 shrink-0">
          <Brand center={center} />
        </div>

        <NavPanel
          pathname={pathname}
          layoutId="admin-nav-desktop"
          modules={modules}
        />

        <Button
          variant="ghost"
          className="mt-2 shrink-0 justify-start"
          onClick={signOut}
        >
          <LogOut data-icon="inline-start" />
          Cerrar sesión
        </Button>
      </aside>
    </>
  );
}
