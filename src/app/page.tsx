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
import { CognitoAuthFlow } from "@/components/cognito/cognito-auth-flow";
import {
  loginSchema,
  onboardingSchema,
  recoverOwnerPasswordSchema,
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
  | "cognito"
  | "onboarding"
  | "login"
  | "recover-owner-password"
  | "redirecting";

// Mismo fondo para el Shell normal y el splash de carga (abajo) — un solo
// lugar para no desalinear las dos vistas si el fondo cambia más adelante.
const LOGIN_BACKGROUND_STYLE: React.CSSProperties = {
  backgroundImage: "url(/login-background.svg)",
  backgroundSize: "cover",
  backgroundPosition: "center",
  backgroundRepeat: "no-repeat",
};

/**
 * Se ve por una fracción de segundo al abrir la app, mientras `Home` decide a
 * qué stage ir (cognito/onboarding/login/admin) — antes de esto, la ventana
 * nativa mostraba un pantallazo en blanco (el color de fondo por defecto del
 * WebView) hasta que React termina de hidratar. El `backgroundColor` de
 * `tauri.conf.json` ya evita el blanco puro a nivel de ventana; esto cubre el
 * instante siguiente, ya con contenido real.
 */
function SplashScreen() {
  return (
    <main
      className="login-view relative flex min-h-screen items-center justify-center overflow-hidden bg-background text-foreground antialiased"
      style={LOGIN_BACKGROUND_STYLE}
    >
      <span className="font-heading text-6xl font-medium tracking-tight sm:text-7xl">
        Aura <span className="italic text-primary">Studio</span>
      </span>
    </main>
  );
}

function Shell({ children }: { children: React.ReactNode }) {
  return (
    <main
      className="login-view relative flex min-h-screen items-center justify-center overflow-hidden bg-background px-4 py-12 text-foreground antialiased"
      style={LOGIN_BACKGROUND_STYLE}
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
        setStage(cognitoState.session ? "onboarding" : "cognito");
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
    return <SplashScreen />;
  }

  return (
    <Shell>
      {stage === "cognito" ? (
        <CognitoAuthFlow onAuthenticated={() => setStage("onboarding")} />
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
