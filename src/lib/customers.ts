import { invoke } from "@tauri-apps/api/core";

import type { Customer } from "@/types/database";

export function normalizeEmail(email: string) {
  return email.trim().toLowerCase();
}

export function normalizeDocument(document: string) {
  return document.trim();
}

/** Issue #2, Fase 5: un campo vacío se manda como `null` (ausente), nunca
 * como cadena vacía — el backend ya trata ambos igual, pero mandar `null`
 * explícito evita violar el formato de email en el backend si alguna vez
 * deja de ser tolerante con cadenas vacías. */
function toOptional(value: string | null | undefined): string | null {
  const trimmed = value?.trim() ?? "";
  return trimmed ? trimmed : null;
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

type CustomerInput = {
  name: string;
  phone: string;
  email?: string | null;
  document?: string | null;
};

export async function createCustomer(input: CustomerInput): Promise<Customer> {
  return invoke<Customer>("create_customer", {
    input: {
      name: input.name,
      phone: input.phone,
      email: toOptional(input.email),
      document: toOptional(input.document),
    },
  });
}

export async function ensureCustomer(input: CustomerInput): Promise<Customer> {
  const document = toOptional(input.document);
  if (document) {
    const existing = await findCustomerByDocument(document);
    if (existing) return existing;
  }
  return createCustomer(input);
}

export async function updateCustomer(
  id: string,
  input: CustomerInput
): Promise<Customer> {
  return invoke<Customer>("update_customer", {
    id,
    input: {
      name: input.name,
      phone: input.phone,
      email: toOptional(input.email),
      document: toOptional(input.document),
    },
  });
}
