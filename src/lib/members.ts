import { createClient } from "@/lib/supabase/client";
import type { CenterInvite, CenterMember } from "@/types/database";

export async function listCenterMembers() {
  const supabase = createClient();
  const { data, error } = await supabase
    .from("center_members")
    .select("*")
    .order("created_at", { ascending: true });
  if (error) throw error;
  return (data ?? []) as CenterMember[];
}

export async function listCenterInvites() {
  const supabase = createClient();
  const { data, error } = await supabase
    .from("center_invites")
    .select("*")
    .is("accepted_at", null)
    .gt("expires_at", new Date().toISOString())
    .order("created_at", { ascending: false });
  if (error) throw error;
  return (data ?? []) as CenterInvite[];
}

export async function createReceptionInvite(email?: string) {
  const supabase = createClient();
  const { data, error } = await supabase.rpc("create_center_invite", {
    p_email: email?.trim() || null,
  });
  if (error) throw error;
  const row = Array.isArray(data) ? data[0] : data;
  if (!row) throw new Error("No se pudo crear la invitación");
  return row as { id: string; token: string; expires_at: string };
}

export async function revokeCenterMember(memberId: string) {
  const supabase = createClient();
  const { error } = await supabase.rpc("revoke_center_member", {
    p_member_id: memberId,
  });
  if (error) throw error;
}

export async function getInvitePreview(token: string) {
  const supabase = createClient();
  const { data, error } = await supabase.rpc("get_invite_preview", {
    p_token: token,
  });
  if (error) throw error;
  const row = Array.isArray(data) ? data[0] : data;
  return (row ?? null) as {
    center_name: string;
    email: string | null;
    role: string;
    expires_at: string;
    valid: boolean;
  } | null;
}
