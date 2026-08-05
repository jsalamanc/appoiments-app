import { AppointmentsCalendar } from "@/components/admin/appointments-calendar";
import { ModuleGate } from "@/components/admin/module-gate";

export default function AppointmentsPage() {
  return (
    <ModuleGate module="appointments">
      <AppointmentsCalendar />
    </ModuleGate>
  );
}
