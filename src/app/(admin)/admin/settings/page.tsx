import { redirect } from "next/navigation";

import { ModulesSettings } from "@/components/admin/modules-settings";
import { OwnerGate } from "@/components/admin/owner-gate";
import { PlanSettings } from "@/components/admin/plan-settings";
import { TeamSettings } from "@/components/admin/team-settings";
import { canUseModule } from "@/lib/plan-modules";
import { parseCenterModules } from "@/lib/modules";
import { listPlansServer } from "@/lib/plans-server";
import { getServerCenterContext } from "@/lib/supabase/center";

export default async function SettingsPage() {
  const ctx = await getServerCenterContext();
  if (!ctx) redirect("/login");

  const plans = await listPlansServer();
  const modules = parseCenterModules(ctx.center.modules);

  return (
    <OwnerGate>
      <div className="space-y-10">
        <ModulesSettings
          center={ctx.center}
          planAllowed={ctx.planAllowed}
        />
        <PlanSettings
          plans={plans}
          currentPlanId={ctx.center.plan_id ?? "free"}
        />
        <TeamSettings
          bookingSlug={ctx.center.slug}
          publicBookingEnabled={canUseModule(
            modules,
            ctx.planAllowed,
            "public_booking"
          )}
        />
      </div>
    </OwnerGate>
  );
}
