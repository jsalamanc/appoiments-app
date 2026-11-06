"use client";

import { zodResolver } from "@hookform/resolvers/zod";
import { invoke } from "@tauri-apps/api/core";
import { useEffect, useState } from "react";
import { useForm } from "react-hook-form";
import { toast } from "sonner";

import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { cognitoResetPasswordSchema, type CognitoResetPasswordValues } from "@/lib/validations";

export function CognitoResetPasswordForm({
  email,
  onReset,
}: {
  email: string;
  onReset: () => void;
}) {
  const [loading, setLoading] = useState(false);
  const [resendCooldown, setResendCooldown] = useState(0);
  const {
    register,
    handleSubmit,
    formState: { errors },
  } = useForm<CognitoResetPasswordValues>({
    resolver: zodResolver(cognitoResetPasswordSchema),
    defaultValues: { code: "", newPassword: "", confirmPassword: "" },
  });

  useEffect(() => {
    if (resendCooldown <= 0) return;
    const timer = setInterval(() => {
      setResendCooldown((value) => Math.max(0, value - 1));
    }, 1000);
    return () => clearInterval(timer);
  }, [resendCooldown]);

  const onSubmit = async (values: CognitoResetPasswordValues) => {
    setLoading(true);
    try {
      await invoke("cognito_confirm_forgot_password", {
        input: { username: email, code: values.code, newPassword: values.newPassword },
      });
      toast.success("Contraseña actualizada, ya puedes iniciar sesión");
      onReset();
    } catch (err) {
      toast.error(err instanceof Error ? err.message : String(err));
    } finally {
      setLoading(false);
    }
  };

  const onResend = async () => {
    try {
      await invoke("cognito_forgot_password", { input: { username: email } });
      toast.success("Código reenviado");
      setResendCooldown(30);
    } catch (err) {
      toast.error(err instanceof Error ? err.message : String(err));
    }
  };

  return (
    <>
      <h1 className="mt-6 font-heading text-3xl font-semibold">
        Ingresa el código
      </h1>
      <p className="mt-2 text-sm text-muted-foreground">
        Revisa tu correo y define una nueva contraseña.
      </p>
      <form onSubmit={handleSubmit(onSubmit)} className="mt-8 space-y-4">
        <div className="space-y-2">
          <Label htmlFor="resetCode">Código</Label>
          <Input id="resetCode" autoComplete="one-time-code" {...register("code")} />
          {errors.code && (
            <p className="text-xs text-destructive">{errors.code.message}</p>
          )}
        </div>
        <div className="space-y-2">
          <Label htmlFor="resetNewPassword">Nueva contraseña</Label>
          <Input
            id="resetNewPassword"
            type="password"
            autoComplete="new-password"
            {...register("newPassword")}
          />
          {errors.newPassword && (
            <p className="text-xs text-destructive">{errors.newPassword.message}</p>
          )}
        </div>
        <div className="space-y-2">
          <Label htmlFor="resetConfirmPassword">Confirmar contraseña</Label>
          <Input
            id="resetConfirmPassword"
            type="password"
            autoComplete="new-password"
            {...register("confirmPassword")}
          />
          {errors.confirmPassword && (
            <p className="text-xs text-destructive">
              {errors.confirmPassword.message}
            </p>
          )}
        </div>
        <Button type="submit" className="w-full" disabled={loading}>
          {loading ? "Guardando..." : "Restablecer contraseña"}
        </Button>
      </form>
      <button
        type="button"
        onClick={onResend}
        disabled={resendCooldown > 0}
        className="mt-4 w-full text-center text-sm text-muted-foreground underline-offset-4 hover:underline disabled:opacity-50"
      >
        {resendCooldown > 0
          ? `Reenviar código (${resendCooldown}s)`
          : "Reenviar código"}
      </button>
    </>
  );
}
