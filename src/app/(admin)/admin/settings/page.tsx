"use client";

import { ModulesSettings } from "@/components/admin/modules-settings";
import { OwnerGate } from "@/components/admin/owner-gate";
import { TeamSettings } from "@/components/admin/team-settings";

export default function SettingsPage() {
  return (
    <OwnerGate>
      <div className="space-y-10">
        <ModulesSettings />
        <TeamSettings />
      </div>
    </OwnerGate>
  );
}
