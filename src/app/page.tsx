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
import {
  loginSchema,
  onboardingSchema,
  type LoginValues,
  type OnboardingValues,
} from "@/lib/validations";

const SESSION_STORAGE_KEY = "aura_session_token";

type LoginResult = {
  user: { id: string; name: string; username: string; role: string };
  session_token: string | null;
};

function Shell({ children }: { children: React.ReactNode }) {
  return (
    <main className="flex min-h-screen items-center justify-center px-4 py-12">
      <motion.div
        initial={{ opacity: 0, y: 20 }}
        animate={{ opacity: 1, y: 0 }}
        transition={{ duration: 0.45 }}
        className="w-full max-w-md rounded-3xl border border-border/80 bg-card p-8 shadow-md backdrop-blur"
      >
        <span className="font-heading text-2xl font-semibold tracking-wide">
          Aura Studio
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
            placeholder="Spa Belleza Natural"
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

function LoginForm() {
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
    </>
  );
}

export default function Home() {
  const router = useRouter();
  const [stage, setStage] = useState<
    "loading" | "onboarding" | "login" | "redirecting"
  >("loading");

  useEffect(() => {
    let cancelled = false;

    (async () => {
      const complete = await invoke<boolean>("onboarding_is_complete");
      if (cancelled) return;

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
      {stage === "onboarding" ? (
        <OnboardingForm onDone={() => router.push("/admin")} />
      ) : (
        <LoginForm />
      )}
    </Shell>
  );
}
