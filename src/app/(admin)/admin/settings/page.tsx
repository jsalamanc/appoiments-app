"use client";

import { BusinessHoursSettings } from "@/components/admin/business-hours-settings";
import { ModulesSettings } from "@/components/admin/modules-settings";
import { OwnerGate } from "@/components/admin/owner-gate";
import { StorageSettings } from "@/components/admin/storage-settings";
import { TeamSettings } from "@/components/admin/team-settings";
import { UpdateSettings } from "@/components/admin/update-settings";

export default function SettingsPage() {
  return (
    <OwnerGate>
      <div className="space-y-10">
        <ModulesSettings />
        <BusinessHoursSettings />
        <TeamSettings />
        <StorageSettings />
        <UpdateSettings />
      </div>
    </OwnerGate>
  );
}
