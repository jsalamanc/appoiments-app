"use client";

import { AuditLog } from "@/components/admin/audit-log";
import { OwnerGate } from "@/components/admin/owner-gate";

export default function AuditPage() {
  return (
    <OwnerGate>
      <AuditLog />
    </OwnerGate>
  );
}
