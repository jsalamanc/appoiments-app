"use client";

import { Copy, Link2, Trash2, UserPlus } from "lucide-react";
import { useRouter } from "next/navigation";
import { useCallback, useEffect, useState } from "react";
import { toast } from "sonner";

import { FadeIn } from "@/components/motion/fade-in";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import {
  createReceptionInvite,
  listCenterInvites,
  listCenterMembers,
  revokeCenterMember,
} from "@/lib/members";
import { formatMemberRole } from "@/lib/roles";
import type { CenterInvite, CenterMember } from "@/types/database";

export function TeamSettings({
  bookingSlug,
  publicBookingEnabled,
}: {
  bookingSlug: string;
  publicBookingEnabled: boolean;
}) {
  const router = useRouter();
  const [members, setMembers] = useState<CenterMember[]>([]);
  const [invites, setInvites] = useState<CenterInvite[]>([]);
  const [email, setEmail] = useState("");
  const [loading, setLoading] = useState(true);
  const [creating, setCreating] = useState(false);
  const [lastInviteUrl, setLastInviteUrl] = useState<string | null>(null);
  const [origin, setOrigin] = useState("");

  useEffect(() => {
    setOrigin(window.location.origin);
  }, []);

  const bookingUrl = origin
    ? `${origin}/r/${bookingSlug}`
    : `/r/${bookingSlug}`;

  const load = useCallback(async () => {
    setLoading(true);
    try {
      const [memberRows, inviteRows] = await Promise.all([
        listCenterMembers(),
        listCenterInvites(),
      ]);
      setMembers(memberRows);
      setInvites(inviteRows);
    } catch (err) {
      toast.error(err instanceof Error ? err.message : "Error al cargar equipo");
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    void load();
  }, [load]);

  const createInvite = async () => {
    setCreating(true);
    try {
      const invite = await createReceptionInvite(email || undefined);
      const url = `${window.location.origin}/register?invite=${invite.token}`;
      setLastInviteUrl(url);
      await navigator.clipboard.writeText(url).catch(() => undefined);
      toast.success("Invitación creada. Link copiado.");
      setEmail("");
      await load();
      router.refresh();
    } catch (err) {
      toast.error(err instanceof Error ? err.message : "No se pudo invitar");
    } finally {
      setCreating(false);
    }
  };

  const copy = async (value: string, label: string) => {
    try {
      await navigator.clipboard.writeText(value);
      toast.success(`${label} copiado`);
    } catch {
      toast.error("No se pudo copiar");
    }
  };

  const revoke = async (memberId: string) => {
    try {
      await revokeCenterMember(memberId);
      toast.success("Acceso de recepción removido");
      await load();
    } catch (err) {
      toast.error(err instanceof Error ? err.message : "No se pudo remover");
    }
  };

  return (
    <div className="mx-auto max-w-5xl space-y-6">
      <FadeIn delay={0.05} className="rounded-3xl border border-border/70 bg-card/70 p-5">
        <div className="mb-3 flex items-center gap-2">
          <Link2 className="size-4 text-muted-foreground" />
          <h2 className="font-heading text-2xl font-semibold">
            Portal de reservas
          </h2>
        </div>
        <p className="text-sm text-muted-foreground">
          Comparte este enlace para que tus clientes agenden sin entrar al
          admin. Activa el módulo “Portal de reservas” arriba (hoy es gratis).
        </p>
        {!publicBookingEnabled && (
          <p className="mt-3 text-sm text-destructive">
            El portal está desactivado. Actívalo en la lista de módulos para
            aceptar reservas públicas.
          </p>
        )}
        {publicBookingEnabled && (
          <Badge variant="secondary" className="mt-3">
            Portal activo
          </Badge>
        )}
        <div className="mt-4 flex flex-col gap-2 sm:flex-row">
          <Input
            readOnly
            value={publicBookingEnabled ? bookingUrl : "Activa el módulo para obtener el enlace"}
            disabled={!publicBookingEnabled}
            className="font-mono text-sm"
          />
          <Button
            type="button"
            variant="outline"
            disabled={!publicBookingEnabled}
            onClick={() => void copy(bookingUrl, "Enlace de reservas")}
          >
            <Copy data-icon="inline-start" />
            Copiar
          </Button>
        </div>
      </FadeIn>

      <FadeIn delay={0.08} className="rounded-3xl border border-border/70 bg-card/70 p-5">
        <div className="mb-3 flex items-center gap-2">
          <UserPlus className="size-4 text-muted-foreground" />
          <h2 className="font-heading text-2xl font-semibold">Equipo</h2>
        </div>
        <p className="text-sm text-muted-foreground">
          Invita a recepción. Podrán operar agenda, clientes y ventas, pero no
          ajustes ni reportes.
        </p>

        <div className="mt-4 space-y-2">
          <Label htmlFor="invite-email">Correo (opcional)</Label>
          <div className="flex flex-col gap-2 sm:flex-row">
            <Input
              id="invite-email"
              type="email"
              placeholder="recepcion@tu-centro.com"
              value={email}
              onChange={(e) => setEmail(e.target.value)}
            />
            <Button
              type="button"
              disabled={creating}
              onClick={() => void createInvite()}
            >
              {creating ? "Creando..." : "Crear invitación"}
            </Button>
          </div>
        </div>

        {lastInviteUrl && (
          <div className="mt-4 rounded-2xl border border-border/60 bg-background/60 p-3">
            <p className="text-xs text-muted-foreground">Último link</p>
            <p className="mt-1 break-all font-mono text-sm">{lastInviteUrl}</p>
          </div>
        )}

        {loading ? (
          <p className="mt-4 text-sm text-muted-foreground">Cargando equipo...</p>
        ) : (
          <div className="mt-6 space-y-3">
            <h3 className="text-sm font-medium text-muted-foreground">
              Miembros
            </h3>
            <ul className="space-y-2">
              {members.map((member) => (
                <li
                  key={member.id}
                  className="flex flex-wrap items-center justify-between gap-2 rounded-2xl border border-border/60 px-4 py-3 text-sm"
                >
                  <div>
                    <p className="font-medium font-mono text-xs sm:text-sm">
                      {member.user_id.slice(0, 8)}…
                    </p>
                    <Badge variant="secondary" className="mt-1">
                      {formatMemberRole(member.role)}
                    </Badge>
                  </div>
                  {member.role === "reception" && (
                    <Button
                      type="button"
                      size="sm"
                      variant="ghost"
                      onClick={() => void revoke(member.id)}
                    >
                      <Trash2 data-icon="inline-start" />
                      Quitar
                    </Button>
                  )}
                </li>
              ))}
            </ul>

            {invites.length > 0 && (
              <>
                <h3 className="pt-2 text-sm font-medium text-muted-foreground">
                  Invitaciones pendientes
                </h3>
                <ul className="space-y-2">
                  {invites.map((invite) => {
                    const url = `${origin}/register?invite=${invite.token}`;
                    return (
                      <li
                        key={invite.id}
                        className="flex flex-wrap items-center justify-between gap-2 rounded-2xl border border-border/60 px-4 py-3 text-sm"
                      >
                        <div>
                          <p className="font-medium">
                            {invite.email || "Sin correo fijo"}
                          </p>
                          <p className="text-xs text-muted-foreground">
                            Expira {new Date(invite.expires_at).toLocaleString("es-CO")}
                          </p>
                        </div>
                        <Button
                          type="button"
                          size="sm"
                          variant="outline"
                          onClick={() => void copy(url, "Link de invitación")}
                        >
                          <Copy data-icon="inline-start" />
                          Copiar link
                        </Button>
                      </li>
                    );
                  })}
                </ul>
              </>
            )}
          </div>
        )}
      </FadeIn>
    </div>
  );
}
