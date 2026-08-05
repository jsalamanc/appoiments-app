import { EmployeesManager } from "@/components/admin/employees-manager";
import { ModuleGate } from "@/components/admin/module-gate";

export default function EmployeesPage() {
  return (
    <ModuleGate module="employees">
      <EmployeesManager />
    </ModuleGate>
  );
}
