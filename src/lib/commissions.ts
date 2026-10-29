import { invoke } from "@tauri-apps/api/core";

export type CommissionRateType = "percentage" | "fixed";

export type EmployeeCommissionRate = {
  id: string;
  employeeId: string;
  serviceId: string | null;
  rateType: CommissionRateType;
  rateValue: number;
};

export type CommissionRateInput = {
  serviceId: string | null;
  rateType: CommissionRateType;
  rateValue: number;
};

export type EmployeeCommissionRow = {
  employeeId: string;
  name: string;
  color: string;
  completedAppointments: number;
  revenue: number;
  commissionAmount: number;
};

export type EmployeeCommissionsReport = {
  period: string;
  from: string;
  totalCommission: number;
  rows: EmployeeCommissionRow[];
};

export async function getEmployeeCommissionRates(
  employeeId: string
): Promise<EmployeeCommissionRate[]> {
  return invoke<EmployeeCommissionRate[]>("get_employee_commission_rates", {
    employeeId,
  });
}

export async function updateEmployeeCommissionRates(
  employeeId: string,
  rates: CommissionRateInput[]
): Promise<EmployeeCommissionRate[]> {
  return invoke<EmployeeCommissionRate[]>("update_employee_commission_rates", {
    employeeId,
    rates,
  });
}

export async function getEmployeeCommissionsReport(
  period: string,
  from: string
): Promise<EmployeeCommissionsReport> {
  return invoke<EmployeeCommissionsReport>("get_employee_commissions_report", {
    period,
    from,
  });
}
