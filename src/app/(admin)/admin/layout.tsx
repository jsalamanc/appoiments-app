"use client";

import { invoke } from "@tauri-apps/api/core";
import { useRouter } from "next/navigation";
import { useEffect, useState } from "react";

import { NetworkStatusHeader } from "@/components/admin/network-status-header";
import { AdminSidebar } from "@/components/admin/sidebar";

type CurrentUser = { id: string } | null;

export default function AdminLayout({
  children,
}: {
  children: React.ReactNode;
}) {
  const router = useRouter();
  const [ready, setReady] = useState(false);
  const [authorized, setAuthorized] = useState(false);

  useEffect(() => {
    let cancelled = false;
    invoke<CurrentUser>("auth_current_user").then((user) => {
      if (cancelled) return;
      if (!user) {
        router.replace("/");
        return;
      }
      setAuthorized(true);
      setReady(true);
    });
    return () => {
      cancelled = true;
    };
  }, [router]);

  if (!ready || !authorized) return null;

  return (
    <div className="flex h-dvh flex-col overflow-hidden lg:flex-row">
      <AdminSidebar />
      <main className="min-h-0 flex-1 overflow-y-auto">
        <div className="print-hidden">
          <NetworkStatusHeader />
        </div>
        <div className="px-4 py-6 md:px-8 md:py-8">{children}</div>
      </main>
    </div>
  );
}
