"use client";

import { zodResolver } from "@hookform/resolvers/zod";
import { invoke } from "@tauri-apps/api/core";
import { useEffect, useState } from "react";
import { useForm } from "react-hook-form";
import { toast } from "sonner";

import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { cognitoConfirmSchema, type CognitoConfirmValues } from "@/lib/validations";

export function CognitoConfirmForm({
  email,
  onConfirmed,
  onChangeEmail,
}: {
  email: string;
  onConfirmed: (email: string) => void;
  onChangeEmail: () => void;
}) {
  const [loading, setLoading] = useState(false);
  const [resendCooldown, setResendCooldown] = useState(0);
  const {
    register,
    handleSubmit,
    formState: { errors },
  } = useForm<CognitoConfirmValues>({
    resolver: zodResolver(cognitoConfirmSchema),
    defaultValues: { code: "" },
  });

  useEffect(() => {
    if (resendCooldown <= 0) return;
    const timer = setInterval(() => {
      setResendCooldown((value) => Math.max(0, value - 1));
    }, 1000);
    return () => clearInterval(timer);
  }, [resendCooldown]);

  const onSubmit = async (values: CognitoConfirmValues) => {
    setLoading(true);
    try {
      await invoke("cognito_confirm_signup", { input: { username: email, code: values.code } });
      toast.success("Cuenta confirmada, ya puedes iniciar sesión");
      onConfirmed(email);
    } catch (err) {
      toast.error(err instanceof Error ? err.message : String(err));
    } finally {
      setLoading(false);
    }
  };

  const onResend = async () => {
    try {
      await invoke("cognito_resend_code", { input: { username: email } });
      toast.success("Código reenviado");
      setResendCooldown(30);
    } catch (err) {
      toast.error(err instanceof Error ? err.message : String(err));
    }
  };

  return (
    <>
      <h1 className="mt-6 font-heading text-3xl font-semibold">
        Confirma tu correo
      </h1>
      <p className="mt-2 text-sm text-muted-foreground">
        Te enviamos un código a{" "}
        <span className="font-medium text-foreground">{email}</span>. Ingresa
        el código para confirmar la cuenta.
      </p>
      <form onSubmit={handleSubmit(onSubmit)} className="mt-8 space-y-4">
        <div className="space-y-2">
          <Label htmlFor="confirmCode">Código</Label>
          <Input id="confirmCode" autoComplete="one-time-code" {...register("code")} />
          {errors.code && (
            <p className="text-xs text-destructive">{errors.code.message}</p>
          )}
        </div>
        <Button type="submit" className="w-full" disabled={loading}>
          {loading ? "Confirmando..." : "Confirmar"}
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
      <button
        type="button"
        onClick={onChangeEmail}
        className="mt-2 w-full text-center text-sm text-muted-foreground underline-offset-4 hover:underline"
      >
        ¿Correo incorrecto? Regístrate de nuevo
      </button>
    </>
  );
}
