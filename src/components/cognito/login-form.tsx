"use client";

import { zodResolver } from "@hookform/resolvers/zod";
import { invoke } from "@tauri-apps/api/core";
import { useState } from "react";
import { useForm } from "react-hook-form";
import { toast } from "sonner";

import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { cognitoLoginSchema, type CognitoLoginValues } from "@/lib/validations";

const DEFAULT_SUBTITLE =
  "Es la primera vez que abres Aura Studio en este equipo. Inicia sesión con la cuenta que administra tu negocio.";

export function CognitoLoginForm({
  prefillEmail,
  onLoggedIn,
  onSwitchToSignup,
  onForgotPassword,
  subtitle,
}: {
  prefillEmail: string;
  onLoggedIn: () => void;
  onSwitchToSignup: () => void;
  onForgotPassword: () => void;
  subtitle?: string;
}) {
  const [loading, setLoading] = useState(false);
  const {
    register,
    handleSubmit,
    formState: { errors },
  } = useForm<CognitoLoginValues>({
    resolver: zodResolver(cognitoLoginSchema),
    defaultValues: { email: prefillEmail, password: "" },
  });

  const onSubmit = async (values: CognitoLoginValues) => {
    setLoading(true);
    try {
      // Cognito usa el correo como usuario en este User Pool — se manda en
      // el campo `username` del comando porque así se llama el parámetro
      // de la API de Cognito, no porque sea distinto del correo.
      await invoke("cognito_login", {
        input: { username: values.email, password: values.password },
      });
      onLoggedIn();
    } catch (err) {
      toast.error(err instanceof Error ? err.message : String(err));
    } finally {
      setLoading(false);
    }
  };

  return (
    <>
      <h1 className="mt-6 font-heading text-3xl font-semibold">
        Inicia sesión
      </h1>
      <p className="mt-2 text-sm text-muted-foreground">
        {subtitle ?? DEFAULT_SUBTITLE}
      </p>
      <form onSubmit={handleSubmit(onSubmit)} className="mt-8 space-y-4">
        <div className="space-y-2">
          <Label htmlFor="cognitoLoginEmail">Correo electrónico</Label>
          <Input
            id="cognitoLoginEmail"
            type="email"
            autoComplete="email"
            {...register("email")}
          />
          {errors.email && (
            <p className="text-xs text-destructive">{errors.email.message}</p>
          )}
        </div>
        <div className="space-y-2">
          <Label htmlFor="cognitoLoginPassword">Contraseña</Label>
          <Input
            id="cognitoLoginPassword"
            type="password"
            autoComplete="current-password"
            {...register("password")}
          />
          {errors.password && (
            <p className="text-xs text-destructive">{errors.password.message}</p>
          )}
        </div>
        <Button type="submit" className="w-full" disabled={loading}>
          {loading ? "Entrando..." : "Entrar"}
        </Button>
      </form>
      <button
        type="button"
        onClick={onForgotPassword}
        className="mt-4 w-full text-center text-sm text-muted-foreground underline-offset-4 hover:underline"
      >
        ¿Olvidaste tu contraseña?
      </button>
      <button
        type="button"
        onClick={onSwitchToSignup}
        className="mt-2 w-full text-center text-sm text-muted-foreground underline-offset-4 hover:underline"
      >
        ¿No tienes cuenta? Regístrate
      </button>
    </>
  );
}
