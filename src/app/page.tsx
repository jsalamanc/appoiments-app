"use client";

import { zodResolver } from "@hookform/resolvers/zod";
import { invoke } from "@tauri-apps/api/core";
import { motion } from "framer-motion";
import { useRouter } from "next/navigation";
import { useEffect, useState } from "react";
import { useForm } from "react-hook-form";
import { toast } from "sonner";

import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { loginDisplayFont, loginSansFont } from "@/lib/fonts";
import {
  cognitoConfirmSchema,
  cognitoForgotPasswordSchema,
  cognitoLoginSchema,
  cognitoResetPasswordSchema,
  cognitoSignupSchema,
  loginSchema,
  onboardingSchema,
  recoverOwnerPasswordSchema,
  type CognitoConfirmValues,
  type CognitoForgotPasswordValues,
  type CognitoLoginValues,
  type CognitoResetPasswordValues,
  type CognitoSignupValues,
  type LoginValues,
  type OnboardingValues,
  type RecoverOwnerPasswordValues,
} from "@/lib/validations";

const SESSION_STORAGE_KEY = "aura_session_token";

type LoginResult = {
  user: { id: string; name: string; username: string; role: string };
  session_token: string | null;
};

type CognitoInstallationState = {
  established: boolean;
  session: { sub: string; username: string; email: string | null; expiresAt: string } | null;
};

type Stage =
  | "loading"
  | "cognito-login"
  | "cognito-signup"
  | "cognito-confirm"
  | "cognito-forgot"
  | "cognito-reset"
  | "onboarding"
  | "login"
  | "recover-owner-password"
  | "redirecting";

function Shell({ children }: { children: React.ReactNode }) {
  return (
    <main
      className={`login-view ${loginDisplayFont.variable} ${loginSansFont.variable} relative flex min-h-screen items-center justify-center overflow-hidden bg-background px-4 py-12 text-foreground antialiased`}
    >
      {/* gradient blobs — mismo tratamiento que el hero de aura-studio-landing */}
      <div aria-hidden className="pointer-events-none absolute inset-0 -z-10">
        <div className="absolute left-1/2 top-[-18%] h-[36rem] w-[36rem] -translate-x-1/2 rounded-full bg-primary/25 blur-[110px]" />
        <div className="absolute -right-24 top-40 h-80 w-80 rounded-full bg-accent/50 blur-[100px]" />
        <div className="absolute -left-24 bottom-0 h-72 w-72 rounded-full bg-[oklch(0.72_0.1_80)]/30 blur-[100px]" />
      </div>

      <motion.div
        initial={{ opacity: 0, y: 20 }}
        animate={{ opacity: 1, y: 0 }}
        transition={{ duration: 0.45 }}
        className="w-full max-w-md rounded-3xl border border-border/70 bg-card/90 p-8 shadow-lg backdrop-blur"
      >
        <span className="font-heading text-2xl font-medium tracking-tight">
          Aura <span className="italic text-primary">Studio</span>
        </span>
        {children}
      </motion.div>
    </main>
  );
}

function CognitoSignupForm({
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

function CognitoConfirmForm({
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

function CognitoLoginForm({
  prefillEmail,
  onLoggedIn,
  onSwitchToSignup,
  onForgotPassword,
}: {
  prefillEmail: string;
  onLoggedIn: () => void;
  onSwitchToSignup: () => void;
  onForgotPassword: () => void;
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
        Es la primera vez que abres Aura Studio en este equipo. Inicia sesión
        con la cuenta que administra tu negocio.
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

function CognitoForgotPasswordForm({
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

function CognitoResetPasswordForm({
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

function OnboardingForm({ onDone }: { onDone: () => void }) {
  const [loading, setLoading] = useState(false);
  const {
    register,
    handleSubmit,
    formState: { errors },
  } = useForm<OnboardingValues>({
    resolver: zodResolver(onboardingSchema),
    defaultValues: {
      businessName: "",
      ownerName: "",
      ownerUsername: "",
      ownerPassword: "",
      confirmPassword: "",
    },
  });

  const onSubmit = async (values: OnboardingValues) => {
    setLoading(true);
    try {
      await invoke("onboarding_complete_setup", {
        input: {
          businessName: values.businessName,
          ownerName: values.ownerName,
          ownerUsername: values.ownerUsername,
          ownerPassword: values.ownerPassword,
        },
      });
      toast.success("Negocio configurado. Bienvenida a Aura Studio");
      onDone();
    } catch (err) {
      toast.error(err instanceof Error ? err.message : String(err));
    } finally {
      setLoading(false);
    }
  };

  return (
    <>
      <h1 className="mt-6 font-heading text-3xl font-semibold">
        Configura tu negocio
      </h1>
      <p className="mt-2 text-sm text-muted-foreground">
        Es la primera vez que abres Aura Studio en este equipo. Crea tu
        negocio y tu usuario de dueño/a.
      </p>
      <form onSubmit={handleSubmit(onSubmit)} className="mt-8 space-y-4">
        <div className="space-y-2">
          <Label htmlFor="businessName">Nombre del negocio</Label>
          <Input
            id="businessName"
            placeholder="Nombre de tu negocio"
            {...register("businessName")}
          />
          {errors.businessName && (
            <p className="text-xs text-destructive">
              {errors.businessName.message}
            </p>
          )}
        </div>
        <div className="space-y-2">
          <Label htmlFor="ownerName">Tu nombre</Label>
          <Input id="ownerName" {...register("ownerName")} />
          {errors.ownerName && (
            <p className="text-xs text-destructive">
              {errors.ownerName.message}
            </p>
          )}
        </div>
        <div className="space-y-2">
          <Label htmlFor="ownerUsername">Usuario</Label>
          <Input
            id="ownerUsername"
            autoComplete="username"
            {...register("ownerUsername")}
          />
          {errors.ownerUsername && (
            <p className="text-xs text-destructive">
              {errors.ownerUsername.message}
            </p>
          )}
        </div>
        <div className="space-y-2">
          <Label htmlFor="ownerPassword">Contraseña</Label>
          <Input
            id="ownerPassword"
            type="password"
            autoComplete="new-password"
            {...register("ownerPassword")}
          />
          {errors.ownerPassword && (
            <p className="text-xs text-destructive">
              {errors.ownerPassword.message}
            </p>
          )}
        </div>
        <div className="space-y-2">
          <Label htmlFor="confirmPassword">Confirmar contraseña</Label>
          <Input
            id="confirmPassword"
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
          {loading ? "Creando..." : "Crear negocio"}
        </Button>
      </form>
    </>
  );
}

function LoginForm({ onForgotPassword }: { onForgotPassword: () => void }) {
  const router = useRouter();
  const [loading, setLoading] = useState(false);
  const {
    register,
    handleSubmit,
    formState: { errors },
  } = useForm<LoginValues>({
    resolver: zodResolver(loginSchema),
    defaultValues: { username: "", password: "", rememberMe: true },
  });

  const onSubmit = async (values: LoginValues) => {
    setLoading(true);
    try {
      const result = await invoke<LoginResult>("auth_login", {
        username: values.username,
        password: values.password,
        rememberMe: values.rememberMe,
      });
      if (result.session_token) {
        try {
          localStorage.setItem(SESSION_STORAGE_KEY, result.session_token);
        } catch {
          // localStorage puede no estar disponible; no es crítico.
        }
      }
      toast.success("Bienvenida de nuevo");
      router.push("/admin");
    } catch (err) {
      toast.error(err instanceof Error ? err.message : String(err));
    } finally {
      setLoading(false);
    }
  };

  return (
    <>
      <h1 className="mt-6 font-heading text-3xl font-semibold">
        Iniciar sesión
      </h1>
      <p className="mt-2 text-sm text-muted-foreground">
        Accede al panel de tu negocio.
      </p>
      <form onSubmit={handleSubmit(onSubmit)} className="mt-8 space-y-4">
        <div className="space-y-2">
          <Label htmlFor="username">Usuario</Label>
          <Input
            id="username"
            autoComplete="username"
            {...register("username")}
          />
          {errors.username && (
            <p className="text-xs text-destructive">
              {errors.username.message}
            </p>
          )}
        </div>
        <div className="space-y-2">
          <Label htmlFor="password">Contraseña</Label>
          <Input
            id="password"
            type="password"
            autoComplete="current-password"
            {...register("password")}
          />
          {errors.password && (
            <p className="text-xs text-destructive">
              {errors.password.message}
            </p>
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
    </>
  );
}

function RecoverOwnerPasswordForm({ onBackToLogin }: { onBackToLogin: () => void }) {
  const [loading, setLoading] = useState(false);
  const {
    register,
    handleSubmit,
    formState: { errors },
  } = useForm<RecoverOwnerPasswordValues>({
    resolver: zodResolver(recoverOwnerPasswordSchema),
    defaultValues: {
      cognitoEmail: "",
      cognitoPassword: "",
      localUsername: "",
      newPassword: "",
      confirmPassword: "",
    },
  });

  const onSubmit = async (values: RecoverOwnerPasswordValues) => {
    setLoading(true);
    try {
      await invoke("auth_recover_owner_password", {
        input: {
          cognitoEmail: values.cognitoEmail,
          cognitoPassword: values.cognitoPassword,
          localUsername: values.localUsername,
          newPassword: values.newPassword,
        },
      });
      toast.success("Contraseña actualizada, ya puedes iniciar sesión");
      onBackToLogin();
    } catch (err) {
      toast.error(err instanceof Error ? err.message : String(err));
    } finally {
      setLoading(false);
    }
  };

  return (
    <>
      <h1 className="mt-6 font-heading text-3xl font-semibold">
        Recupera el acceso del dueño
      </h1>
      <p className="mt-2 text-sm text-muted-foreground">
        Vuelve a iniciar sesión con la cuenta de Aura Studio que configuró
        este negocio para poder definir una contraseña local nueva.
      </p>
      <form onSubmit={handleSubmit(onSubmit)} className="mt-8 space-y-4">
        <div className="space-y-2">
          <Label htmlFor="recoverCognitoEmail">Correo de tu cuenta de Aura Studio</Label>
          <Input
            id="recoverCognitoEmail"
            type="email"
            autoComplete="email"
            {...register("cognitoEmail")}
          />
          {errors.cognitoEmail && (
            <p className="text-xs text-destructive">{errors.cognitoEmail.message}</p>
          )}
        </div>
        <div className="space-y-2">
          <Label htmlFor="recoverCognitoPassword">Contraseña de tu cuenta de Aura Studio</Label>
          <Input
            id="recoverCognitoPassword"
            type="password"
            autoComplete="current-password"
            {...register("cognitoPassword")}
          />
          {errors.cognitoPassword && (
            <p className="text-xs text-destructive">{errors.cognitoPassword.message}</p>
          )}
        </div>
        <div className="space-y-2">
          <Label htmlFor="recoverLocalUsername">Usuario dueño a recuperar</Label>
          <Input
            id="recoverLocalUsername"
            autoComplete="username"
            {...register("localUsername")}
          />
          {errors.localUsername && (
            <p className="text-xs text-destructive">{errors.localUsername.message}</p>
          )}
        </div>
        <div className="space-y-2">
          <Label htmlFor="recoverNewPassword">Nueva contraseña</Label>
          <Input
            id="recoverNewPassword"
            type="password"
            autoComplete="new-password"
            {...register("newPassword")}
          />
          {errors.newPassword && (
            <p className="text-xs text-destructive">{errors.newPassword.message}</p>
          )}
        </div>
        <div className="space-y-2">
          <Label htmlFor="recoverConfirmPassword">Confirmar contraseña</Label>
          <Input
            id="recoverConfirmPassword"
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
          {loading ? "Recuperando..." : "Recuperar acceso"}
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

export default function Home() {
  const router = useRouter();
  const [stage, setStage] = useState<Stage>("loading");
  const [cognitoPendingEmail, setCognitoPendingEmail] = useState("");

  useEffect(() => {
    let cancelled = false;

    (async () => {
      const [cognitoState, complete] = await Promise.all([
        invoke<CognitoInstallationState>("cognito_installation_state"),
        invoke<boolean>("onboarding_is_complete"),
      ]);
      if (cancelled) return;

      if (!complete && !cognitoState.established) {
        // Instalación genuinamente nueva: el gate de Cognito se pide acá,
        // una sola vez. Una instalación que ya tenía el onboarding completo
        // de antes de este cambio nunca llega a esta rama.
        setStage(cognitoState.session ? "onboarding" : "cognito-login");
        return;
      }

      if (!complete) {
        setStage("onboarding");
        return;
      }

      let token: string | null = null;
      try {
        token = localStorage.getItem(SESSION_STORAGE_KEY);
      } catch {
        token = null;
      }

      if (token) {
        const user = await invoke<{ id: string } | null>(
          "auth_resume_session",
          { sessionToken: token }
        );
        if (!cancelled && user) {
          setStage("redirecting");
          router.replace("/admin");
          return;
        }
      }

      if (!cancelled) setStage("login");
    })();

    return () => {
      cancelled = true;
    };
  }, [router]);

  if (stage === "loading" || stage === "redirecting") {
    return <Shell>{null}</Shell>;
  }

  return (
    <Shell>
      {stage === "cognito-signup" ? (
        <CognitoSignupForm
          onNeedsConfirmation={(email) => {
            setCognitoPendingEmail(email);
            setStage("cognito-confirm");
          }}
          onConfirmedDirectly={(email) => {
            setCognitoPendingEmail(email);
            setStage("cognito-login");
          }}
          onSwitchToLogin={() => setStage("cognito-login")}
        />
      ) : stage === "cognito-confirm" ? (
        <CognitoConfirmForm
          email={cognitoPendingEmail}
          onConfirmed={(email) => {
            setCognitoPendingEmail(email);
            setStage("cognito-login");
          }}
          onChangeEmail={() => setStage("cognito-signup")}
        />
      ) : stage === "cognito-login" ? (
        <CognitoLoginForm
          prefillEmail={cognitoPendingEmail}
          onLoggedIn={() => setStage("onboarding")}
          onSwitchToSignup={() => setStage("cognito-signup")}
          onForgotPassword={() => setStage("cognito-forgot")}
        />
      ) : stage === "cognito-forgot" ? (
        <CognitoForgotPasswordForm
          prefillEmail={cognitoPendingEmail}
          onCodeSent={(email) => {
            setCognitoPendingEmail(email);
            setStage("cognito-reset");
          }}
          onBackToLogin={() => setStage("cognito-login")}
        />
      ) : stage === "cognito-reset" ? (
        <CognitoResetPasswordForm
          email={cognitoPendingEmail}
          onReset={() => setStage("cognito-login")}
        />
      ) : stage === "onboarding" ? (
        <OnboardingForm onDone={() => router.push("/admin")} />
      ) : stage === "recover-owner-password" ? (
        <RecoverOwnerPasswordForm onBackToLogin={() => setStage("login")} />
      ) : (
        <LoginForm onForgotPassword={() => setStage("recover-owner-password")} />
      )}
    </Shell>
  );
}
