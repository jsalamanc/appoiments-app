import { AppointmentsToday } from "@/components/admin/appointments-today";
import { ModuleGate } from "@/components/admin/module-gate";

export default function AppointmentsTodayPage() {
  return (
    <ModuleGate module="appointments">
      <AppointmentsToday />
    </ModuleGate>
  );
}
