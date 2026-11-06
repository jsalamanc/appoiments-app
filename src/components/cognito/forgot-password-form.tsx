"use client";

import { zodResolver } from "@hookform/resolvers/zod";
import { invoke } from "@tauri-apps/api/core";
import { useState } from "react";
import { useForm } from "react-hook-form";
import { toast } from "sonner";

import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { cognitoForgotPasswordSchema, type CognitoForgotPasswordValues } from "@/lib/validations";

export function CognitoForgotPasswordForm({
  prefillEmail,
  onCodeSent,
  onBackToLogin,
}: {
  prefillEmail: string;
  onCodeSent: (email: string) => void;
  onBackToLogin: () => void;
}) {
  const [loading, setLoading] = useState(false);
  const {
    register,
    handleSubmit,
    formState: { errors },
  } = useForm<CognitoForgotPasswordValues>({
    resolver: zodResolver(cognitoForgotPasswordSchema),
    defaultValues: { email: prefillEmail },
  });

  const onSubmit = async (values: CognitoForgotPasswordValues) => {
    setLoading(true);
    try {
      await invoke("cognito_forgot_password", { input: { username: values.email } });
      toast.success("Si la cuenta existe, te enviamos un código para restablecer la contraseña");
      onCodeSent(values.email);
    } catch (err) {
      toast.error(err instanceof Error ? err.message : String(err));
    } finally {
      setLoading(false);
    }
  };

  return (
    <>
      <h1 className="mt-6 font-heading text-3xl font-semibold">
        Recupera tu contraseña
      </h1>
      <p className="mt-2 text-sm text-muted-foreground">
        Ingresa tu correo y te enviamos un código para restablecerla.
      </p>
      <form onSubmit={handleSubmit(onSubmit)} className="mt-8 space-y-4">
        <div className="space-y-2">
          <Label htmlFor="forgotEmail">Correo electrónico</Label>
          <Input
            id="forgotEmail"
            type="email"
            autoComplete="email"
            {...register("email")}
          />
          {errors.email && (
            <p className="text-xs text-destructive">{errors.email.message}</p>
          )}
        </div>
        <Button type="submit" className="w-full" disabled={loading}>
          {loading ? "Enviando..." : "Enviar código"}
        </Button>
      </form>
      <button
        type="button"
        onClick={onBackToLogin}
        className="mt-4 w-full text-center text-sm text-muted-foreground underline-offset-4 hover:underline"
      >
        Volver a iniciar sesión
      </button>
    </>
  );
}
