"use client";

import { invoke } from "@tauri-apps/api/core";
import { LogOut, MoreVertical, Settings } from "lucide-react";
import { useRouter } from "next/navigation";

import { Avatar, AvatarFallback } from "@/components/ui/avatar";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { isOwnerRole } from "@/lib/roles";
import { useCurrentUser } from "@/lib/use-current-user";

const ROLE_LABELS: Record<string, string> = {
  owner: "Administrador",
  staff: "Staff",
};

function initials(name: string) {
  const parts = name.trim().split(/\s+/).filter(Boolean);
  if (parts.length === 0) return "?";
  if (parts.length === 1) return parts[0]!.slice(0, 2).toUpperCase();
  return (parts[0]![0] + parts[parts.length - 1]![0]).toUpperCase();
}

export function UserMenu() {
  const router = useRouter();
  const { user } = useCurrentUser();

  if (!user) return null;

  const roleLabel = ROLE_LABELS[user.role] ?? user.role;

  const signOut = async () => {
    await invoke("auth_logout", { sessionToken: null });
    try {
      localStorage.removeItem("aura_session_token");
    } catch {
      // localStorage puede no estar disponible; no es crítico.
    }
    router.push("/");
  };

  return (
    <DropdownMenu>
      <DropdownMenuTrigger className="flex items-center gap-2.5 rounded-full border border-border/60 bg-card/70 py-1.5 pr-2 pl-3.5 text-left transition hover:bg-card">
        <span className="min-w-0">
          <span className="block text-xs text-muted-foreground">{roleLabel}</span>
          <span className="block max-w-32 truncate text-sm font-semibold">
            {user.name}
          </span>
        </span>
        <Avatar>
          <AvatarFallback>{initials(user.name)}</AvatarFallback>
        </Avatar>
        <MoreVertical className="size-4 shrink-0 text-muted-foreground" />
      </DropdownMenuTrigger>
      <DropdownMenuContent align="end">
        {isOwnerRole(user.role) && (
          <DropdownMenuItem onClick={() => router.push("/admin/settings")}>
            <Settings data-icon="inline-start" />
            Ajustes
          </DropdownMenuItem>
        )}
        <DropdownMenuSeparator />
        <DropdownMenuItem variant="destructive" onClick={() => void signOut()}>
          <LogOut data-icon="inline-start" />
          Cerrar sesión
        </DropdownMenuItem>
      </DropdownMenuContent>
    </DropdownMenu>
  );
}
