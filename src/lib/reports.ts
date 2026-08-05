import { getPeriodRange, type RevenuePeriod } from "@/lib/sales";

export type ReportPeriod = RevenuePeriod;

export type EmployeePerformanceRow = {
  employeeId: string;
  name: string;
  color: string;
  scheduled: number;
  completed: number;
  cancelled: number;
  noShow: number;
  totalAppointments: number;
  revenue: number;
};

export type TopServiceRow = {
  serviceId: string | null;
  name: string;
  quantity: number;
  revenue: number;
};

export type NoShowCustomerRow = {
  customerId: string;
  name: string;
  email: string;
  phone: string;
  noShowCount: number;
  cancelledCount: number;
  lostCount: number;
};

function parsePeriod(value?: string): ReportPeriod {
  if (value === "day" || value === "week" || value === "month") return value;
  return "month";
}

export function getReportPeriod(value?: string) {
  const period = parsePeriod(value);
  return { period, range: getPeriodRange(period) };
}
