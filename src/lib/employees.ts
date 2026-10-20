import { invoke } from "@tauri-apps/api/core";

import type { Employee } from "@/types/database";

export async function listEmployees(options?: {
  activeOnly?: boolean;
}): Promise<Employee[]> {
  return invoke<Employee[]>("list_employees", {
    activeOnly: options?.activeOnly ?? false,
  });
}

export async function createEmployee(input: {
  name: string;
  color: string;
  active: boolean;
}): Promise<Employee> {
  return invoke<Employee>("create_employee", { input });
}

export async function updateEmployee(
  id: string,
  input: { name: string; color: string; active: boolean }
): Promise<Employee> {
  return invoke<Employee>("update_employee", { id, input });
}

export async function deleteEmployee(id: string): Promise<void> {
  await invoke("delete_employee", { id });
}
