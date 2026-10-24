"use client";

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
        <TeamSettings />
        <StorageSettings />
        <UpdateSettings />
      </div>
    </OwnerGate>
  );
}
