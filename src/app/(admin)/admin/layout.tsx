import { NetworkStatusHeader } from "@/components/admin/network-status-header";
import { AdminSidebar } from "@/components/admin/sidebar";
import { getServerCenter } from "@/lib/supabase/center";

export default async function AdminLayout({
  children,
}: {
  children: React.ReactNode;
}) {
  const center = await getServerCenter();

  return (
    <div className="flex h-dvh flex-col overflow-hidden lg:flex-row">
      <AdminSidebar center={center} />
      <main className="min-h-0 flex-1 overflow-y-auto">
        <NetworkStatusHeader />
        <div className="px-4 py-6 md:px-8 md:py-8">{children}</div>
      </main>
    </div>
  );
}
