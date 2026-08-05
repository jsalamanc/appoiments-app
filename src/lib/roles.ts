import type { MemberRole } from "@/types/database";

export type { MemberRole };

export function isOwnerRole(role: MemberRole | null | undefined) {
  return role === "owner";
}

export function isReceptionRole(role: MemberRole | null | undefined) {
  return role === "reception";
}

/** Rutas visibles solo para dueño. */
export function isOwnerOnlyPath(pathname: string) {
  if (pathname === "/admin/settings" || pathname.startsWith("/admin/settings/")) {
    return true;
  }
  if (pathname.startsWith("/admin/reports")) {
    return true;
  }
  return false;
}

export function formatMemberRole(role: MemberRole) {
  return role === "owner" ? "Dueño" : "Recepción";
}
