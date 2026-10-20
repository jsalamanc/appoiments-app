"use client";

import { invoke } from "@tauri-apps/api/core";
import { UserPlus, UserX } from "lucide-react";
import { useCallback, useEffect, useState } from "react";
import { toast } from "sonner";

import { FadeIn } from "@/components/motion/fade-in";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { formatMemberRole } from "@/lib/roles";

type UserPublic = {
  id: string;
  name: string;
  username: string;
  role: "owner" | "staff";
  active: boolean;
};

export function TeamSettings() {
  const [users, setUsers] = useState<UserPublic[]>([]);
  const [loading, setLoading] = useState(true);
  const [creating, setCreating] = useState(false);
  const [name, setName] = useState("");
  const [username, setUsername] = useState("");
  const [password, setPassword] = useState("");

  const load = useCallback(async () => {
    setLoading(true);
    try {
      const rows = await invoke<UserPublic[]>("list_users");
      setUsers(rows);
    } catch (err) {
      toast.error(err instanceof Error ? err.message : "Error al cargar equipo");
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    void load();
  }, [load]);

  const createUser = async () => {
    if (!name.trim() || !username.trim() || password.length < 6) {
      toast.error("Completa nombre, usuario y una contraseña de al menos 6 caracteres");
      return;
    }
    setCreating(true);
    try {
      await invoke("create_staff_user", {
        input: { name, username, password, role: "staff" },
      });
      toast.success("Usuario creado");
      setName("");
      setUsername("");
      setPassword("");
      await load();
    } catch (err) {
      toast.error(err instanceof Error ? err.message : "No se pudo crear el usuario");
    } finally {
      setCreating(false);
    }
  };

  const toggleActive = async (user: UserPublic) => {
    try {
      await invoke("set_user_active", { userId: user.id, active: !user.active });
      toast.success(user.active ? "Usuario desactivado" : "Usuario activado");
      await load();
    } catch (err) {
      toast.error(err instanceof Error ? err.message : "No se pudo actualizar");
    }
  };

  return (
    <div className="mx-auto max-w-5xl space-y-6">
      <FadeIn
        delay={0.08}
        className="rounded-3xl border border-border/70 bg-card shadow-sm p-5"
      >
        <div className="mb-3 flex items-center gap-2">
          <UserPlus className="size-4 text-muted-foreground" />
          <h2 className="font-heading text-2xl font-semibold">Equipo</h2>
        </div>
        <p className="text-sm text-muted-foreground">
          Crea usuarios de staff para este equipo. Podrán operar agenda,
          clientes y ventas, pero no ajustes ni reportes.
        </p>

        <form
          onSubmit={(e) => {
            e.preventDefault();
            void createUser();
          }}
        >
          <div className="mt-4 grid gap-2 sm:grid-cols-3">
            <div className="space-y-2">
              <Label htmlFor="staff-name">Nombre</Label>
              <Input
                id="staff-name"
                value={name}
                onChange={(e) => setName(e.target.value)}
              />
            </div>
            <div className="space-y-2">
              <Label htmlFor="staff-username">Usuario</Label>
              <Input
                id="staff-username"
                value={username}
                onChange={(e) => setUsername(e.target.value)}
              />
            </div>
            <div className="space-y-2">
              <Label htmlFor="staff-password">Contraseña</Label>
              <Input
                id="staff-password"
                type="password"
                value={password}
                onChange={(e) => setPassword(e.target.value)}
              />
            </div>
          </div>
          <Button type="submit" className="mt-3" disabled={creating}>
            {creating ? "Creando..." : "Crear usuario"}
          </Button>
        </form>

        {loading ? (
          <p className="mt-4 text-sm text-muted-foreground">Cargando equipo...</p>
        ) : (
          <div className="mt-6 space-y-3">
            <h3 className="text-sm font-medium text-muted-foreground">
              Usuarios
            </h3>
            <ul className="space-y-2">
              {users.map((user) => (
                <li
                  key={user.id}
                  className="flex flex-wrap items-center justify-between gap-2 rounded-2xl border border-border/60 px-4 py-3 text-sm"
                >
                  <div>
                    <p className="font-medium">
                      {user.name}{" "}
                      <span className="text-muted-foreground">
                        @{user.username}
                      </span>
                    </p>
                    <div className="mt-1 flex gap-1">
                      <Badge variant="secondary">
                        {formatMemberRole(user.role)}
                      </Badge>
                      {!user.active && <Badge variant="outline">Inactivo</Badge>}
                    </div>
                  </div>
                  {user.role !== "owner" && (
                    <Button
                      type="button"
                      size="sm"
                      variant="ghost"
                      onClick={() => void toggleActive(user)}
                    >
                      <UserX data-icon="inline-start" />
                      {user.active ? "Desactivar" : "Activar"}
                    </Button>
                  )}
                </li>
              ))}
            </ul>
          </div>
        )}
      </FadeIn>
    </div>
  );
}
