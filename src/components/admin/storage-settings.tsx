"use client";

import { invoke } from "@tauri-apps/api/core";
import { relaunch } from "@tauri-apps/plugin-process";
import { Check, Cloud, Database, HardDrive } from "lucide-react";
import { useCallback, useEffect, useState } from "react";
import { toast } from "sonner";

import {
  AlertDialog,
  AlertDialogAction,
  AlertDialogCancel,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogTitle,
} from "@/components/ui/alert-dialog";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { FadeIn } from "@/components/motion/fade-in";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";

type PostgresConfig = {
  host: string;
  port: number;
  database: string;
  username: string;
  sslmode: string;
};

type StorageMode =
  | { mode: "local" }
  | ({ mode: "postgres" } & PostgresConfig);

type SelectedMode = "local" | "postgres";

const EMPTY_PG_CONFIG: PostgresConfig = {
  host: "",
  port: 5432,
  database: "",
  username: "",
  sslmode: "prefer",
};

export function StorageSettings() {
  const [loading, setLoading] = useState(true);
  const [selected, setSelected] = useState<SelectedMode>("local");
  const [pgConfig, setPgConfig] = useState<PostgresConfig>(EMPTY_PG_CONFIG);
  const [password, setPassword] = useState("");
  const [testing, setTesting] = useState(false);
  const [switching, setSwitching] = useState(false);
  const [confirmOpen, setConfirmOpen] = useState(false);

  const load = useCallback(async () => {
    setLoading(true);
    try {
      const mode = await invoke<StorageMode>("storage_get_mode");
      setSelected(mode.mode);
      if (mode.mode === "postgres") {
        setPgConfig({
          host: mode.host,
          port: mode.port,
          database: mode.database,
          username: mode.username,
          sslmode: mode.sslmode,
        });
      }
    } catch (err) {
      toast.error(
        err instanceof Error
          ? err.message
          : "No se pudo cargar la configuración de almacenamiento"
      );
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    void load();
  }, [load]);

  const pgFieldsComplete =
    pgConfig.host.trim() !== "" &&
    pgConfig.database.trim() !== "" &&
    pgConfig.username.trim() !== "" &&
    password !== "";

  const testConnection = async () => {
    if (!pgFieldsComplete) {
      toast.error("Completa host, base de datos, usuario y contraseña");
      return;
    }
    setTesting(true);
    try {
      await invoke("storage_test_connection", { config: pgConfig, password });
      toast.success("Conexión exitosa");
    } catch (err) {
      toast.error(err instanceof Error ? err.message : "No se pudo conectar");
    } finally {
      setTesting(false);
    }
  };

  const applySwitch = async () => {
    setSwitching(true);
    try {
      if (selected === "local") {
        await invoke("storage_switch_mode", { mode: "local" });
      } else {
        await invoke("storage_switch_mode", {
          mode: "postgres",
          config: pgConfig,
          password,
        });
      }
      toast.success("Modo de almacenamiento actualizado. Reiniciando...");
      await relaunch();
    } catch (err) {
      toast.error(
        err instanceof Error
          ? err.message
          : "No se pudo cambiar el modo de almacenamiento"
      );
      setSwitching(false);
      setConfirmOpen(false);
    }
  };

  if (loading) return null;

  const canSave = selected === "local" || pgFieldsComplete;

  return (
    <div className="mx-auto max-w-5xl space-y-6">
      <FadeIn
        delay={0.12}
        className="rounded-3xl border border-border/70 bg-card shadow-sm p-5"
      >
        <h2 className="font-heading text-2xl font-semibold">
          Almacenamiento de datos
        </h2>
        <p className="mt-1 text-sm text-muted-foreground">
          Elige dónde vive la información de tu negocio. Cambiar de modo
          reinicia la aplicación.
        </p>

        <div className="mt-4 space-y-2">
          <ModeOption
            icon={<HardDrive className="size-5" />}
            title="Local"
            description="La app guarda todo en este equipo. No requiere internet ni configuración adicional."
            selected={selected === "local"}
            onSelect={() => setSelected("local")}
          />
          <ModeOption
            icon={<Database className="size-5" />}
            title="Base de datos propia"
            description="Conéctate a un servidor PostgreSQL que tú administras (local en red o en tu propia nube)."
            selected={selected === "postgres"}
            onSelect={() => setSelected("postgres")}
          />
          <ModeOption
            icon={<Cloud className="size-5" />}
            title="Nube"
            description="Base de datos gestionada por nosotros, con plan mensual."
            selected={false}
            disabled
            badge="Próximamente"
          />
        </div>

        {selected === "postgres" && (
          <div className="mt-5 space-y-4 rounded-2xl border border-border/60 p-4">
            <div className="grid gap-3 sm:grid-cols-2">
              <div className="space-y-2">
                <Label htmlFor="pg-host">Host</Label>
                <Input
                  id="pg-host"
                  placeholder="db.ejemplo.com"
                  value={pgConfig.host}
                  onChange={(e) =>
                    setPgConfig((c) => ({ ...c, host: e.target.value }))
                  }
                />
              </div>
              <div className="space-y-2">
                <Label htmlFor="pg-port">Puerto</Label>
                <Input
                  id="pg-port"
                  type="number"
                  value={pgConfig.port}
                  onChange={(e) =>
                    setPgConfig((c) => ({
                      ...c,
                      port: Number(e.target.value) || 0,
                    }))
                  }
                />
              </div>
              <div className="space-y-2">
                <Label htmlFor="pg-database">Base de datos</Label>
                <Input
                  id="pg-database"
                  value={pgConfig.database}
                  onChange={(e) =>
                    setPgConfig((c) => ({ ...c, database: e.target.value }))
                  }
                />
              </div>
              <div className="space-y-2">
                <Label htmlFor="pg-username">Usuario</Label>
                <Input
                  id="pg-username"
                  value={pgConfig.username}
                  onChange={(e) =>
                    setPgConfig((c) => ({ ...c, username: e.target.value }))
                  }
                />
              </div>
              <div className="space-y-2">
                <Label htmlFor="pg-password">Contraseña</Label>
                <Input
                  id="pg-password"
                  type="password"
                  value={password}
                  onChange={(e) => setPassword(e.target.value)}
                />
              </div>
              <div className="space-y-2">
                <Label htmlFor="pg-sslmode">SSL</Label>
                <select
                  id="pg-sslmode"
                  value={pgConfig.sslmode}
                  onChange={(e) =>
                    setPgConfig((c) => ({ ...c, sslmode: e.target.value }))
                  }
                  className="flex h-8 w-full rounded-lg border border-input bg-transparent px-2.5 text-sm outline-none focus-visible:border-ring focus-visible:ring-3 focus-visible:ring-ring/50"
                >
                  <option value="disable">disable</option>
                  <option value="prefer">prefer</option>
                  <option value="require">require</option>
                  <option value="verify-ca">verify-ca</option>
                  <option value="verify-full">verify-full</option>
                </select>
              </div>
            </div>

            <Button
              type="button"
              variant="secondary"
              disabled={testing || !pgFieldsComplete}
              onClick={() => void testConnection()}
            >
              {testing ? "Probando..." : "Probar conexión"}
            </Button>
          </div>
        )}

        <Button
          type="button"
          className="mt-5"
          disabled={!canSave}
          onClick={() => setConfirmOpen(true)}
        >
          Guardar y cambiar
        </Button>
      </FadeIn>

      <AlertDialog open={confirmOpen} onOpenChange={setConfirmOpen}>
        <AlertDialogContent>
          <AlertDialogHeader>
            <AlertDialogTitle>¿Cambiar el modo de almacenamiento?</AlertDialogTitle>
            <AlertDialogDescription>
              {selected === "local"
                ? "La app volverá a usar su base de datos local y se reiniciará."
                : "La app se conectará a la base de datos PostgreSQL indicada y se reiniciará. Los datos que tengas localmente no se copian automáticamente — esta base externa debe estar vacía o ya tener los datos que quieres usar."}
            </AlertDialogDescription>
          </AlertDialogHeader>
          <AlertDialogFooter>
            <AlertDialogCancel disabled={switching}>Cancelar</AlertDialogCancel>
            <AlertDialogAction
              disabled={switching}
              onClick={(e) => {
                e.preventDefault();
                void applySwitch();
              }}
            >
              {switching ? "Cambiando..." : "Confirmar y reiniciar"}
            </AlertDialogAction>
          </AlertDialogFooter>
        </AlertDialogContent>
      </AlertDialog>
    </div>
  );
}

type ModeOptionProps = {
  icon: React.ReactNode;
  title: string;
  description: string;
  selected: boolean;
  onSelect?: () => void;
  disabled?: boolean;
  badge?: string;
};

function ModeOption({
  icon,
  title,
  description,
  selected,
  onSelect,
  disabled,
  badge,
}: ModeOptionProps) {
  return (
    <button
      type="button"
      disabled={disabled}
      onClick={onSelect}
      className={`flex w-full items-start gap-3 rounded-2xl border px-4 py-3 text-left transition disabled:cursor-not-allowed disabled:opacity-60 ${
        selected
          ? "border-primary bg-primary/5"
          : "border-border/60 hover:bg-muted/40"
      }`}
    >
      <div className="mt-0.5 text-muted-foreground">{icon}</div>
      <div className="min-w-0 flex-1 space-y-1">
        <div className="flex items-center gap-2">
          <span className="font-medium">{title}</span>
          {badge && <Badge variant="outline">{badge}</Badge>}
        </div>
        <p className="text-sm text-muted-foreground">{description}</p>
      </div>
      {selected && <Check className="mt-0.5 size-5 shrink-0 text-primary" />}
    </button>
  );
}
