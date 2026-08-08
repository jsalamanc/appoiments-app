"use client";

import { standardSchemaResolver } from "@hookform/resolvers/standard-schema";
import { motion } from "framer-motion";
import Link from "next/link";
import { useRouter, useSearchParams } from "next/navigation";
import { Suspense, useEffect, useState } from "react";
import { useForm } from "react-hook-form";
import { toast } from "sonner";

import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { getInvitePreview } from "@/lib/members";
import { createClient } from "@/lib/supabase/client";
import {
  registerInviteSchema,
  registerSchema,
  type RegisterValues,
} from "@/lib/validations";
import { z } from "zod";

type InviteValues = z.infer<typeof registerInviteSchema>;

function RegisterForm() {
  const router = useRouter();
  const searchParams = useSearchParams();
  const inviteToken = searchParams.get("invite")?.trim() || "";
  const [loading, setLoading] = useState(false);
  const [inviteCenter, setInviteCenter] = useState<string | null>(null);
  const [inviteEmail, setInviteEmail] = useState<string | null>(null);
  const [inviteError, setInviteError] = useState<string | null>(null);
  const [checkingInvite, setCheckingInvite] = useState(Boolean(inviteToken));

  const ownerForm = useForm<RegisterValues>({
    resolver: standardSchemaResolver(registerSchema),
    defaultValues: {
      centerName: "",
      email: "",
      password: "",
      confirmPassword: "",
    },
  });

  const inviteForm = useForm<InviteValues>({
    resolver: standardSchemaResolver(registerInviteSchema),
    defaultValues: {
      email: "",
      password: "",
      confirmPassword: "",
    },
  });

  useEffect(() => {
    if (!inviteToken) return;
    let cancelled = false;
    (async () => {
      setCheckingInvite(true);
      try {
        const preview = await getInvitePreview(inviteToken);
        if (cancelled) return;
        if (!preview || !preview.valid) {
          setInviteError("Invitación inválida o expirada");
          return;
        }
        setInviteCenter(preview.center_name);
        setInviteEmail(preview.email);
        if (preview.email) {
          inviteForm.setValue("email", preview.email);
        }
      } catch {
        if (!cancelled) setInviteError("No se pudo validar la invitación");
      } finally {
        if (!cancelled) setCheckingInvite(false);
      }
    })();
    return () => {
      cancelled = true;
    };
  }, [inviteToken, inviteForm]);

  const onOwnerSubmit = async (values: RegisterValues) => {
    setLoading(true);
    const supabase = createClient();
    const { data, error } = await supabase.auth.signUp({
      email: values.email,
      password: values.password,
      options: {
        data: {
          center_name: values.centerName,
        },
      },
    });
    setLoading(false);

    if (error) {
      toast.error(error.message);
      return;
    }

    if (data.session) {
      toast.success("Cuenta creada. Bienvenida a Aura Studio");
      router.push("/admin");
      router.refresh();
      return;
    }

    toast.success(
      "Cuenta creada. Revisa tu correo para confirmar e inicia sesión."
    );
    router.push("/login");
  };

  const onInviteSubmit = async (values: InviteValues) => {
    setLoading(true);
    const supabase = createClient();
    const { data, error } = await supabase.auth.signUp({
      email: values.email,
      password: values.password,
      options: {
        data: {
          invite_token: inviteToken,
        },
      },
    });
    setLoading(false);

    if (error) {
      toast.error(error.message);
      return;
    }

    if (data.session) {
      toast.success("Te uniste al equipo de recepción");
      router.push("/admin");
      router.refresh();
      return;
    }

    toast.success(
      "Cuenta creada. Confirma tu correo e inicia sesión para entrar."
    );
    router.push("/login");
  };

  if (inviteToken) {
    return (
      <main className="flex min-h-screen items-center justify-center px-4 py-12">
        <motion.div
          initial={{ opacity: 0, y: 20 }}
          animate={{ opacity: 1, y: 0 }}
          transition={{ duration: 0.45 }}
          className="w-full max-w-md rounded-3xl border border-border/80 bg-card p-8 shadow-md backdrop-blur"
        >
          <Link
            href="/"
            className="font-heading text-2xl font-semibold tracking-wide"
          >
            Aura Studio
          </Link>
          <h1 className="mt-6 font-heading text-3xl font-semibold">
            Unirte como recepción
          </h1>
          {checkingInvite ? (
            <p className="mt-2 text-sm text-muted-foreground">
              Validando invitación...
            </p>
          ) : inviteError ? (
            <p className="mt-2 text-sm text-destructive">{inviteError}</p>
          ) : (
            <p className="mt-2 text-sm text-muted-foreground">
              Te unirás a <strong>{inviteCenter}</strong> con rol de recepción.
            </p>
          )}

          {!checkingInvite && !inviteError && (
            <form
              onSubmit={inviteForm.handleSubmit(onInviteSubmit)}
              className="mt-8 space-y-4"
            >
              <div className="space-y-2">
                <Label htmlFor="email">Correo</Label>
                <Input
                  id="email"
                  type="email"
                  autoComplete="email"
                  disabled={Boolean(inviteEmail)}
                  {...inviteForm.register("email")}
                />
                {inviteForm.formState.errors.email && (
                  <p className="text-xs text-destructive">
                    {inviteForm.formState.errors.email.message}
                  </p>
                )}
              </div>
              <div className="space-y-2">
                <Label htmlFor="password">Contraseña</Label>
                <Input
                  id="password"
                  type="password"
                  autoComplete="new-password"
                  {...inviteForm.register("password")}
                />
                {inviteForm.formState.errors.password && (
                  <p className="text-xs text-destructive">
                    {inviteForm.formState.errors.password.message}
                  </p>
                )}
              </div>
              <div className="space-y-2">
                <Label htmlFor="confirmPassword">Confirmar contraseña</Label>
                <Input
                  id="confirmPassword"
                  type="password"
                  autoComplete="new-password"
                  {...inviteForm.register("confirmPassword")}
                />
                {inviteForm.formState.errors.confirmPassword && (
                  <p className="text-xs text-destructive">
                    {inviteForm.formState.errors.confirmPassword.message}
                  </p>
                )}
              </div>
              <Button type="submit" className="w-full" disabled={loading}>
                {loading ? "Creando cuenta..." : "Unirme al equipo"}
              </Button>
            </form>
          )}

          <p className="mt-6 text-center text-sm text-muted-foreground">
            ¿Ya tienes cuenta?{" "}
            <Link
              href="/login"
              className="text-foreground underline-offset-4 hover:underline"
            >
              Inicia sesión
            </Link>
          </p>
        </motion.div>
      </main>
    );
  }

  return (
    <main className="flex min-h-screen items-center justify-center px-4 py-12">
      <motion.div
        initial={{ opacity: 0, y: 20 }}
        animate={{ opacity: 1, y: 0 }}
        transition={{ duration: 0.45 }}
        className="w-full max-w-md rounded-3xl border border-border/80 bg-card p-8 shadow-md backdrop-blur"
      >
        <Link
          href="/"
          className="font-heading text-2xl font-semibold tracking-wide"
        >
          Aura Studio
        </Link>
        <h1 className="mt-6 font-heading text-3xl font-semibold">
          Crear cuenta admin
        </h1>
        <p className="mt-2 text-sm text-muted-foreground">
          Registra tu centro estético y empieza a cargar tu catálogo.
        </p>

        <form
          onSubmit={ownerForm.handleSubmit(onOwnerSubmit)}
          className="mt-8 space-y-4"
        >
          <div className="space-y-2">
            <Label htmlFor="centerName">Nombre del centro</Label>
            <Input
              id="centerName"
              placeholder="Spa Belleza Natural"
              {...ownerForm.register("centerName")}
            />
            {ownerForm.formState.errors.centerName && (
              <p className="text-xs text-destructive">
                {ownerForm.formState.errors.centerName.message}
              </p>
            )}
          </div>
          <div className="space-y-2">
            <Label htmlFor="email">Correo</Label>
            <Input
              id="email"
              type="email"
              autoComplete="email"
              placeholder="admin@tu-centro.com"
              {...ownerForm.register("email")}
            />
            {ownerForm.formState.errors.email && (
              <p className="text-xs text-destructive">
                {ownerForm.formState.errors.email.message}
              </p>
            )}
          </div>
          <div className="space-y-2">
            <Label htmlFor="password">Contraseña</Label>
            <Input
              id="password"
              type="password"
              autoComplete="new-password"
              {...ownerForm.register("password")}
            />
            {ownerForm.formState.errors.password && (
              <p className="text-xs text-destructive">
                {ownerForm.formState.errors.password.message}
              </p>
            )}
          </div>
          <div className="space-y-2">
            <Label htmlFor="confirmPassword">Confirmar contraseña</Label>
            <Input
              id="confirmPassword"
              type="password"
              autoComplete="new-password"
              {...ownerForm.register("confirmPassword")}
            />
            {ownerForm.formState.errors.confirmPassword && (
              <p className="text-xs text-destructive">
                {ownerForm.formState.errors.confirmPassword.message}
              </p>
            )}
          </div>
          <Button type="submit" className="w-full" disabled={loading}>
            {loading ? "Creando cuenta..." : "Crear cuenta"}
          </Button>
        </form>

        <p className="mt-6 text-center text-sm text-muted-foreground">
          ¿Ya tienes cuenta?{" "}
          <Link
            href="/login"
            className="text-foreground underline-offset-4 hover:underline"
          >
            Inicia sesión
          </Link>
        </p>
      </motion.div>
    </main>
  );
}

export default function RegisterPage() {
  return (
    <Suspense
      fallback={
        <main className="flex min-h-screen items-center justify-center px-4">
          <p className="text-sm text-muted-foreground">Cargando...</p>
        </main>
      }
    >
      <RegisterForm />
    </Suspense>
  );
}
