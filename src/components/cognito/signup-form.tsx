"use client";

import { zodResolver } from "@hookform/resolvers/zod";
import { invoke } from "@tauri-apps/api/core";
import { useState } from "react";
import { useForm } from "react-hook-form";
import { toast } from "sonner";

import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { cognitoSignupSchema, type CognitoSignupValues } from "@/lib/validations";

export function CognitoSignupForm({
  onNeedsConfirmation,
  onConfirmedDirectly,
  onSwitchToLogin,
}: {
  onNeedsConfirmation: (email: string) => void;
  onConfirmedDirectly: (email: string) => void;
  onSwitchToLogin: () => void;
}) {
  const [loading, setLoading] = useState(false);
  const {
    register,
    handleSubmit,
    formState: { errors },
  } = useForm<CognitoSignupValues>({
    resolver: zodResolver(cognitoSignupSchema),
    defaultValues: { email: "", password: "", confirmPassword: "" },
  });

  const onSubmit = async (values: CognitoSignupValues) => {
    setLoading(true);
    try {
      // El User Pool usa el correo como usuario (`username_attributes =
      // ["email"]` en el backend) — por eso `username` y `email` llevan el
      // mismo valor acá, no hay un "usuario" separado que elegir.
      const result = await invoke<{ needsConfirmation: boolean }>("cognito_signup", {
        input: {
          username: values.email,
          email: values.email,
          password: values.password,
        },
      });
      if (result.needsConfirmation) {
        toast.success("Te enviamos un código de confirmación a tu correo");
        onNeedsConfirmation(values.email);
      } else {
        onConfirmedDirectly(values.email);
      }
    } catch (err) {
      toast.error(err instanceof Error ? err.message : String(err));
    } finally {
      setLoading(false);
    }
  };

  return (
    <>
      <h1 className="mt-6 font-heading text-3xl font-semibold">
        Crea tu cuenta
      </h1>
      <p className="mt-2 text-sm text-muted-foreground">
        Esta cuenta administra tu negocio y tu suscripción en la nube.
      </p>
      <form onSubmit={handleSubmit(onSubmit)} className="mt-8 space-y-4">
        <div className="space-y-2">
          <Label htmlFor="cognitoSignupEmail">Correo electrónico</Label>
          <Input
            id="cognitoSignupEmail"
            type="email"
            autoComplete="email"
            {...register("email")}
          />
          {errors.email && (
            <p className="text-xs text-destructive">{errors.email.message}</p>
          )}
        </div>
        <div className="space-y-2">
          <Label htmlFor="cognitoSignupPassword">Contraseña</Label>
          <Input
            id="cognitoSignupPassword"
            type="password"
            autoComplete="new-password"
            {...register("password")}
          />
          {errors.password && (
            <p className="text-xs text-destructive">{errors.password.message}</p>
          )}
        </div>
        <div className="space-y-2">
          <Label htmlFor="cognitoSignupConfirmPassword">Confirmar contraseña</Label>
          <Input
            id="cognitoSignupConfirmPassword"
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
          {loading ? "Creando..." : "Crear cuenta"}
        </Button>
      </form>
      <button
        type="button"
        onClick={onSwitchToLogin}
        className="mt-4 w-full text-center text-sm text-muted-foreground underline-offset-4 hover:underline"
      >
        ¿Ya tienes cuenta? Inicia sesión
      </button>
    </>
  );
}
