import { redirect } from "next/navigation";

import { ModulesSettings } from "@/components/admin/modules-settings";
import { getServerCenter } from "@/lib/supabase/center";

export default async function SettingsPage() {
  const center = await getServerCenter();
  if (!center) redirect("/login");

  return <ModulesSettings center={center} />;
}
