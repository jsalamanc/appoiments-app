import { EmployeesManager } from "@/components/admin/employees-manager";
import { ModuleGate } from "@/components/admin/module-gate";
import { OwnerGate } from "@/components/admin/owner-gate";

export default function EmployeesPage() {
  return (
    <OwnerGate>
      <ModuleGate module="employees">
        <EmployeesManager />
      </ModuleGate>
    </OwnerGate>
  );
}
