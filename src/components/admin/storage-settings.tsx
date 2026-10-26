"use client";

import { invoke } from "@tauri-apps/api/core";
import { open, save } from "@tauri-apps/plugin-dialog";
import { relaunch } from "@tauri-apps/plugin-process";
import { Check, Cloud, Database, Download, FileLock2, HardDrive, X } from "lucide-react";
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
import { Checkbox } from "@/components/ui/checkbox";
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

type MigrationSummary = {
  businessProfile: number;
  users: number;
  customers: number;
  employees: number;
  products: number;
  services: number;
  packages: number;
  packageItems: number;
  appointments: number;
  sales: number;
  saleItems: number;
  inventoryMovements: number;
  imagesMigrated: number;
};

type BucketProvider =
  | "aws_s3"
  | "cloudflare_r2"
  | "backblaze_b2"
  | "digital_ocean_spaces"
  | "custom";

type BucketConfig = {
  provider: BucketProvider;
  endpoint: string;
  region: string;
  bucket: string;
  prefix: string | null;
  accessKeyId: string;
  pathStyle: boolean;
};

const EMPTY_PG_CONFIG: PostgresConfig = {
  host: "",
  port: 5432,
  database: "",
  username: "",
  sslmode: "prefer",
};

const EMPTY_BUCKET_CONFIG: BucketConfig = {
  provider: "aws_s3",
  endpoint: "",
  region: "",
  bucket: "",
  prefix: null,
  accessKeyId: "",
  pathStyle: false,
};

// Solo para sugerir valores al elegir un proveedor en la UI — el cliente
// S3-compatible del backend es el mismo para todos (ver issue #18).
const PROVIDER_PRESETS: Record<
  BucketProvider,
  { label: string; endpointHint: string; defaultRegion: string; defaultPathStyle: boolean }
> = {
  aws_s3: {
    label: "AWS S3",
    endpointHint: "https://s3.<región>.amazonaws.com",
    defaultRegion: "us-east-1",
    defaultPathStyle: false,
  },
  cloudflare_r2: {
    label: "Cloudflare R2",
    endpointHint: "https://<account_id>.r2.cloudflarestorage.com",
    defaultRegion: "auto",
    defaultPathStyle: false,
  },
  backblaze_b2: {
    label: "Backblaze B2",
    endpointHint: "https://s3.<región>.backblazeb2.com",
    defaultRegion: "us-west-002",
    defaultPathStyle: false,
  },
  digital_ocean_spaces: {
    label: "DigitalOcean Spaces",
    endpointHint: "https://<región>.digitaloceanspaces.com",
    defaultRegion: "nyc3",
    defaultPathStyle: false,
  },
  custom: {
    label: "Otro (compatible con S3)",
    endpointHint: "https://mi-servidor-s3.ejemplo.com",
    defaultRegion: "",
    defaultPathStyle: true,
  },
};

export function StorageSettings() {
  const [loading, setLoading] = useState(true);
  const [currentMode, setCurrentMode] = useState<SelectedMode>("local");
  const [selected, setSelected] = useState<SelectedMode>("local");
  const [pgConfig, setPgConfig] = useState<PostgresConfig>(EMPTY_PG_CONFIG);
  const [password, setPassword] = useState("");
  const [caCertPath, setCaCertPath] = useState<string | null>(null);
  const [migrateData, setMigrateData] = useState(true);
  const [testing, setTesting] = useState(false);
  const [bucketConfig, setBucketConfig] = useState<BucketConfig>(EMPTY_BUCKET_CONFIG);
  const [bucketSecret, setBucketSecret] = useState("");
  const [bucketTesting, setBucketTesting] = useState(false);
  const [switching, setSwitching] = useState(false);
  const [exporting, setExporting] = useState(false);
  const [confirmOpen, setConfirmOpen] = useState(false);

  const load = useCallback(async () => {
    setLoading(true);
    try {
      const mode = await invoke<StorageMode>("storage_get_mode");
      setCurrentMode(mode.mode);
      setSelected(mode.mode);
      if (mode.mode === "postgres") {
        setPgConfig({
          host: mode.host,
          port: mode.port,
          database: mode.database,
          username: mode.username,
          sslmode: mode.sslmode,
        });
        const bucket = await invoke<BucketConfig | null>("bucket_get_config");
        if (bucket) setBucketConfig(bucket);
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

  const needsCaCert =
    pgConfig.sslmode === "verify-ca" || pgConfig.sslmode === "verify-full";

  const bucketFieldsComplete =
    bucketConfig.endpoint.trim() !== "" &&
    bucketConfig.region.trim() !== "" &&
    bucketConfig.bucket.trim() !== "" &&
    bucketConfig.accessKeyId.trim() !== "" &&
    bucketSecret !== "";

  const pickCaCert = async () => {
    const selected = await open({
      multiple: false,
      filters: [{ name: "Certificado", extensions: ["pem", "crt", "cer"] }],
    });
    if (typeof selected === "string") {
      setCaCertPath(selected);
    }
  };

  const testConnection = async () => {
    if (!pgFieldsComplete) {
      toast.error("Completa host, base de datos, usuario y contraseña");
      return;
    }
    setTesting(true);
    try {
      await invoke("storage_test_connection", {
        config: pgConfig,
        password,
        caCertPath: needsCaCert ? caCertPath : null,
      });
      toast.success("Conexión exitosa");
    } catch (err) {
      toast.error(err instanceof Error ? err.message : "No se pudo conectar");
    } finally {
      setTesting(false);
    }
  };

  const testBucketConnection = async () => {
    if (!bucketFieldsComplete) {
      toast.error("Completa endpoint, región, bucket, access key y secret key del bucket");
      return;
    }
    setBucketTesting(true);
    try {
      await invoke("bucket_test_connection", { config: bucketConfig, secret: bucketSecret });
      toast.success("Conexión al bucket exitosa");
    } catch (err) {
      toast.error(err instanceof Error ? err.message : "No se pudo conectar al bucket");
    } finally {
      setBucketTesting(false);
    }
  };

  const exportBackup = async () => {
    const destination = await save({
      defaultPath: `aura-studio-respaldo-${new Date().toISOString().slice(0, 10)}.json`,
      filters: [{ name: "JSON", extensions: ["json"] }],
    });
    if (!destination) return;

    setExporting(true);
    try {
      await invoke("storage_export_local_data", { path: destination });
      toast.success("Respaldo guardado");
    } catch (err) {
      toast.error(
        err instanceof Error ? err.message : "No se pudo generar el respaldo"
      );
    } finally {
      setExporting(false);
    }
  };

  const applySwitch = async () => {
    setSwitching(true);
    try {
      let summary: MigrationSummary | null = null;
      if (selected === "local") {
        await invoke("storage_switch_mode", { mode: "local" });
      } else {
        summary = await invoke<MigrationSummary | null>("storage_switch_mode", {
          mode: "postgres",
          config: pgConfig,
          password,
          caCertPath: needsCaCert ? caCertPath : null,
          migrateData: currentMode === "local" ? migrateData : false,
          bucketConfig,
          bucketSecret,
        });
      }
      if (summary) {
        toast.success(
          `Datos migrados: ${summary.customers} clientes, ${summary.appointments} citas, ${summary.sales} ventas, ${summary.imagesMigrated} imágenes. Reiniciando...`
        );
      } else {
        toast.success("Modo de almacenamiento actualizado. Reiniciando...");
      }
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

  const canSave =
    selected === "local" || (pgFieldsComplete && bucketFieldsComplete);

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
          <>
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
                  onChange={(e) => {
                    const sslmode = e.target.value;
                    setPgConfig((c) => ({ ...c, sslmode }));
                    if (sslmode !== "verify-ca" && sslmode !== "verify-full") {
                      setCaCertPath(null);
                    }
                  }}
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

            {needsCaCert && (
              <div className="space-y-2">
                <Label>Certificado CA</Label>
                <p className="text-xs text-muted-foreground">
                  {pgConfig.sslmode === "verify-ca"
                    ? "verify-ca"
                    : "verify-full"}{" "}
                  necesita el certificado de la autoridad (CA) que firmó el
                  certificado de tu servidor para poder validarlo — normalmente
                  hace falta si usas un certificado autofirmado o de una CA
                  privada.
                </p>
                <div className="flex flex-wrap items-center gap-2">
                  <Button type="button" variant="secondary" onClick={() => void pickCaCert()}>
                    <FileLock2 data-icon="inline-start" />
                    {caCertPath ? "Cambiar certificado" : "Elegir certificado (.pem/.crt)"}
                  </Button>
                  {caCertPath && (
                    <>
                      <span className="truncate text-sm text-muted-foreground">
                        {caCertPath.split(/[\\/]/).pop()}
                      </span>
                      <Button
                        type="button"
                        variant="ghost"
                        size="sm"
                        onClick={() => setCaCertPath(null)}
                      >
                        <X data-icon="inline-start" />
                        Quitar
                      </Button>
                    </>
                  )}
                </div>
              </div>
            )}

            <Button
              type="button"
              variant="secondary"
              disabled={testing || !pgFieldsComplete}
              onClick={() => void testConnection()}
            >
              {testing ? "Probando..." : "Probar conexión"}
            </Button>
          </div>

          <div className="mt-4 space-y-4 rounded-2xl border border-border/60 p-4">
            <div>
              <h3 className="font-medium">Bucket de imágenes (obligatorio)</h3>
              <p className="text-xs text-muted-foreground">
                Las imágenes del catálogo se suben a este bucket S3-compatible
                (AWS S3, Cloudflare R2, Backblaze B2, DigitalOcean Spaces,
                MinIO u otro) en vez de quedar solo en este equipo —
                obligatorio para usar una base de datos propia.
              </p>
            </div>

            <div className="grid gap-3 sm:grid-cols-2">
              <div className="space-y-2">
                <Label htmlFor="bucket-provider">Proveedor</Label>
                <select
                  id="bucket-provider"
                  value={bucketConfig.provider}
                  onChange={(e) => {
                    const provider = e.target.value as BucketProvider;
                    const preset = PROVIDER_PRESETS[provider];
                    setBucketConfig((c) => ({
                      ...c,
                      provider,
                      region: c.region || preset.defaultRegion,
                      pathStyle: preset.defaultPathStyle,
                    }));
                  }}
                  className="flex h-8 w-full rounded-lg border border-input bg-transparent px-2.5 text-sm outline-none focus-visible:border-ring focus-visible:ring-3 focus-visible:ring-ring/50"
                >
                  {Object.entries(PROVIDER_PRESETS).map(([value, preset]) => (
                    <option key={value} value={value}>
                      {preset.label}
                    </option>
                  ))}
                </select>
              </div>
              <div className="space-y-2">
                <Label htmlFor="bucket-endpoint">Endpoint</Label>
                <Input
                  id="bucket-endpoint"
                  placeholder={PROVIDER_PRESETS[bucketConfig.provider].endpointHint}
                  value={bucketConfig.endpoint}
                  onChange={(e) =>
                    setBucketConfig((c) => ({ ...c, endpoint: e.target.value }))
                  }
                />
              </div>
              <div className="space-y-2">
                <Label htmlFor="bucket-region">Región</Label>
                <Input
                  id="bucket-region"
                  value={bucketConfig.region}
                  onChange={(e) =>
                    setBucketConfig((c) => ({ ...c, region: e.target.value }))
                  }
                />
              </div>
              <div className="space-y-2">
                <Label htmlFor="bucket-name">Bucket</Label>
                <Input
                  id="bucket-name"
                  value={bucketConfig.bucket}
                  onChange={(e) =>
                    setBucketConfig((c) => ({ ...c, bucket: e.target.value }))
                  }
                />
              </div>
              <div className="space-y-2">
                <Label htmlFor="bucket-prefix">Prefijo (opcional)</Label>
                <Input
                  id="bucket-prefix"
                  placeholder="catalogo"
                  value={bucketConfig.prefix ?? ""}
                  onChange={(e) =>
                    setBucketConfig((c) => ({ ...c, prefix: e.target.value || null }))
                  }
                />
              </div>
              <div className="space-y-2">
                <Label htmlFor="bucket-access-key">Access Key ID</Label>
                <Input
                  id="bucket-access-key"
                  value={bucketConfig.accessKeyId}
                  onChange={(e) =>
                    setBucketConfig((c) => ({ ...c, accessKeyId: e.target.value }))
                  }
                />
              </div>
              <div className="space-y-2">
                <Label htmlFor="bucket-secret">Secret Access Key</Label>
                <Input
                  id="bucket-secret"
                  type="password"
                  value={bucketSecret}
                  onChange={(e) => setBucketSecret(e.target.value)}
                />
              </div>
            </div>

            <label className="flex items-center gap-2 text-sm">
              <Checkbox
                checked={bucketConfig.pathStyle}
                onCheckedChange={(checked) =>
                  setBucketConfig((c) => ({ ...c, pathStyle: checked === true }))
                }
              />
              Direccionamiento por path (necesario para MinIO y algunos
              proveedores no-AWS)
            </label>

            <Button
              type="button"
              variant="secondary"
              disabled={bucketTesting || !bucketFieldsComplete}
              onClick={() => void testBucketConnection()}
            >
              {bucketTesting ? "Probando..." : "Probar conexión"}
            </Button>
          </div>
          </>
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
                : currentMode === "local" && migrateData
                  ? "La app se conectará a la base de datos PostgreSQL indicada, copiará tus datos locales hacia ella y se reiniciará."
                  : "La app se conectará a la base de datos PostgreSQL indicada y se reiniciará. Los datos que tengas localmente no se copian — esta base externa debe estar vacía o ya tener los datos que quieres usar."}
            </AlertDialogDescription>
          </AlertDialogHeader>
          {currentMode === "local" && selected === "postgres" && (
            <div className="space-y-3">
              <label className="flex items-start gap-2 text-sm">
                <Checkbox
                  className="mt-0.5"
                  checked={migrateData}
                  onCheckedChange={(checked) => setMigrateData(checked)}
                />
                <span>
                  Migrar mis datos locales a esta base de datos
                  <span className="block text-xs text-muted-foreground">
                    {migrateData
                      ? "Se copiarán tus clientes, empleados, servicios, citas, ventas y demás datos del negocio a esta base PostgreSQL (debe estar vacía), junto con las imágenes de tu catálogo al bucket configurado arriba."
                      : "La base de datos PostgreSQL quedará vacía: tendrás que crear tu negocio y cargar tus datos desde cero."}
                  </span>
                </span>
              </label>
              <Button
                type="button"
                variant="outline"
                className="w-fit"
                disabled={exporting}
                onClick={() => void exportBackup()}
              >
                <Download data-icon="inline-start" />
                {exporting ? "Generando respaldo..." : "Descargar respaldo de mis datos (JSON)"}
              </Button>
            </div>
          )}
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
