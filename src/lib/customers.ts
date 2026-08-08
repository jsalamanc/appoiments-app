import { getCurrentCenter } from "@/lib/catalog";
import { createClient } from "@/lib/supabase/client";
import type { Customer } from "@/types/database";

export function normalizeEmail(email: string) {
  return email.trim().toLowerCase();
}

export function normalizeDocument(document: string) {
  return document.trim();
}

export async function findCustomerByEmail(
  email: string
): Promise<Customer | null> {
  const supabase = createClient();
  const center = await getCurrentCenter();
  if (!center) throw new Error("No se encontró el centro");

  const { data, error } = await supabase
    .from("customers")
    .select("*")
    .eq("center_id", center.id)
    .eq("email", normalizeEmail(email))
    .maybeSingle();

  if (error) throw error;
  return data;
}

export async function findCustomerByDocument(
  document: string
): Promise<Customer | null> {
  const supabase = createClient();
  const center = await getCurrentCenter();
  if (!center) throw new Error("No se encontró el centro");

  const normalized = normalizeDocument(document);
  if (!normalized) return null;

  const { data, error } = await supabase
    .from("customers")
    .select("*")
    .eq("center_id", center.id)
    .ilike("document", normalized)
    .maybeSingle();

  if (error) throw error;
  return data;
}

export async function findCustomerById(id: string): Promise<Customer | null> {
  const supabase = createClient();
  const center = await getCurrentCenter();
  if (!center) throw new Error("No se encontró el centro");

  const { data, error } = await supabase
    .from("customers")
    .select("*")
    .eq("center_id", center.id)
    .eq("id", id)
    .maybeSingle();

  if (error) throw error;
  return data;
}

export async function createCustomer(input: {
  name: string;
  phone: string;
  email: string;
  document: string;
}): Promise<Customer> {
  const supabase = createClient();
  const center = await getCurrentCenter();
  if (!center) throw new Error("No se encontró el centro");

  const payload = {
    center_id: center.id,
    name: input.name.trim(),
    phone: input.phone.trim(),
    email: normalizeEmail(input.email),
    document: normalizeDocument(input.document),
  };

  const { data, error } = await supabase
    .from("customers")
    .insert(payload)
    .select("*")
    .single();

  if (error) {
    if (error.code === "23505") {
      const message = error.message.toLowerCase();
      if (message.includes("document")) {
        throw new Error("Ya existe un cliente con ese documento");
      }
      throw new Error("Ya existe un cliente con ese correo");
    }
    throw error;
  }

  return data;
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
  const supabase = createClient();
  const center = await getCurrentCenter();
  if (!center) throw new Error("No se encontró el centro");

  const payload = {
    name: input.name.trim(),
    phone: input.phone.trim(),
    email: normalizeEmail(input.email),
    document: normalizeDocument(input.document),
  };

  const { data, error } = await supabase
    .from("customers")
    .update(payload)
    .eq("id", id)
    .eq("center_id", center.id)
    .select("*")
    .single();

  if (error) {
    if (error.code === "23505") {
      const message = error.message.toLowerCase();
      if (message.includes("document")) {
        throw new Error("Ya existe un cliente con ese documento");
      }
      throw new Error("Ya existe un cliente con ese correo");
    }
    throw error;
  }

  return data;
}
