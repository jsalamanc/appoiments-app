export type UserRole = "owner" | "staff";

export function isOwnerRole(role: UserRole | string | null | undefined) {
  return role === "owner";
}

export function isStaffRole(role: UserRole | string | null | undefined) {
  return role === "staff";
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

export function formatMemberRole(role: UserRole | string) {
  return role === "owner" ? "Dueño" : "Staff";
}
