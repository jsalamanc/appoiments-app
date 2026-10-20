import { invoke } from "@tauri-apps/api/core";

import type { Customer } from "@/types/database";

export function normalizeEmail(email: string) {
  return email.trim().toLowerCase();
}

export function normalizeDocument(document: string) {
  return document.trim();
}

export async function findCustomerByDocument(
  document: string
): Promise<Customer | null> {
  const normalized = normalizeDocument(document);
  if (!normalized) return null;
  return invoke<Customer | null>("find_customer_by_document", {
    document: normalized,
  });
}

export async function findCustomerByEmail(
  email: string
): Promise<Customer | null> {
  return invoke<Customer | null>("find_customer_by_email", {
    email: normalizeEmail(email),
  });
}

export async function findCustomerById(id: string): Promise<Customer | null> {
  return invoke<Customer | null>("find_customer_by_id", { id });
}

export async function createCustomer(input: {
  name: string;
  phone: string;
  email: string;
  document: string;
}): Promise<Customer> {
  return invoke<Customer>("create_customer", { input });
}

export async function ensureCustomer(input: {
  name: string;
  phone: string;
  email: string;
  document: string;
}): Promise<Customer> {
  const existing = await findCustomerByDocument(input.document);
  if (existing) return existing;
  return createCustomer(input);
}

export async function updateCustomer(
  id: string,
  input: {
    name: string;
    phone: string;
    email: string;
    document: string;
  }
): Promise<Customer> {
  return invoke<Customer>("update_customer", { id, input });
}
