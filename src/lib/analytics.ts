import { invoke } from "@tauri-apps/api/core";

export type RevenueTrendPoint = {
  label: string;
  bucketStart: string;
  revenue: number;
  salesCount: number;
};

export type RevenueTrendsReport = {
  period: string;
  from: string;
  previousFrom: string;
  currentRevenue: number;
  previousRevenue: number;
  currentSalesCount: number;
  previousSalesCount: number;
  deltaPercent: number | null;
  points: RevenueTrendPoint[];
};

export async function getRevenueTrendsReport(
  period: string,
  from: string,
  previousFrom: string
): Promise<RevenueTrendsReport> {
  return invoke<RevenueTrendsReport>("get_revenue_trends_report", {
    period,
    from,
    previousFrom,
  });
}

export type CustomerLtvRow = {
  customerId: string;
  name: string;
  phone: string;
  email: string;
  totalSpent: number;
  visitCount: number;
  avgDaysBetweenVisits: number | null;
  firstVisitAt: string;
  lastVisitAt: string;
};

export type CustomerLtvReport = {
  limit: number;
  totalCustomersWithSales: number;
  averageLtv: number;
  averageVisitCount: number;
  rows: CustomerLtvRow[];
};

export async function getCustomerLtvReport(limit: number): Promise<CustomerLtvReport> {
  return invoke<CustomerLtvReport>("get_customer_ltv_report", { limit });
}
