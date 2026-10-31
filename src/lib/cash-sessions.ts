import { invoke } from "@tauri-apps/api/core";

export type CashSession = {
  id: string;
  openedBy: string | null;
  openedAt: string;
  openingAmount: number;
  closedBy: string | null;
  closedAt: string | null;
  countedAmount: number | null;
  expectedAmount: number | null;
  difference: number | null;
  notes: string | null;
};

export async function getCurrentCashSession(): Promise<CashSession | null> {
  return invoke<CashSession | null>("get_current_cash_session");
}

export async function listCashSessions(limit?: number): Promise<CashSession[]> {
  return invoke<CashSession[]>("list_cash_sessions", { limit });
}

export async function openCashSession(
  openingAmount: number
): Promise<CashSession> {
  return invoke<CashSession>("open_cash_session", { openingAmount });
}

export async function closeCashSession(
  id: string,
  countedAmount: number,
  notes?: string
): Promise<CashSession> {
  return invoke<CashSession>("close_cash_session", {
    id,
    countedAmount,
    notes: notes?.trim() || null,
  });
}
