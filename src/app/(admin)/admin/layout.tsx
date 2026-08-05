import { NetworkStatusHeader } from "@/components/admin/network-status-header";
import { AdminSidebar } from "@/components/admin/sidebar";
import { DEFAULT_MODULES } from "@/lib/modules";
import { getServerCenterContext } from "@/lib/supabase/center";

export default async function AdminLayout({
  children,
}: {
  children: React.ReactNode;
}) {
  const ctx = await getServerCenterContext();

  return (
    <div className="flex h-dvh flex-col overflow-hidden lg:flex-row">
      <AdminSidebar
        center={ctx?.center ?? null}
        role={ctx?.role ?? null}
        planAllowed={ctx?.planAllowed ?? DEFAULT_MODULES}
      />
      <main className="min-h-0 flex-1 overflow-y-auto">
        <div className="print-hidden">
          <NetworkStatusHeader />
        </div>
        <div className="px-4 py-6 md:px-8 md:py-8">{children}</div>
      </main>
    </div>
  );
}
