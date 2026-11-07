"use client";

import { invoke } from "@tauri-apps/api/core";
import { save } from "@tauri-apps/plugin-dialog";
import { relaunch } from "@tauri-apps/plugin-process";
import { Check, Cloud, Download, HardDrive } from "lucide-react";
import { useCallback, useEffect, useState } from "react";
import { toast } from "sonner";

import { CognitoAuthFlow } from "@/components/cognito/cognito-auth-flow";
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
import { STORAGE_MIGRATION_TOAST_ID } from "@/lib/storage-migration";
import { ListSkeleton, SettingsSectionSkeleton, SkeletonRegion } from "@/components/admin/skeletons";
import { useMinLoadingElapsed } from "@/lib/use-min-loading";

type NubeConfig = {
  host: string;
  port: number;
  database: string;
  username: string;
  sslmode: string;
  businessId: string;
  businessName: string;
  installationId: string;
};

type StorageMode = { mode: "local" } | ({ mode: "nube" } & NubeConfig);

type SelectedMode = "local" | "nube";

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

// --- Negocios en la nube (ver src-tauri/src/vendor/mod.rs) ---------------------

type VendorBusiness = {
  id: string;
  businessName: string;
  subscriptionStatus: string | null;
  hasDatabase: boolean;
  installationId: string | null;
};

type VendorConnectPreview = { outcome: "connected" | "linkedElsewhere" };

const SUBSCRIPTION_LABELS: Record<string, string> = {
  trialing: "Prueba",
  active: "Activa",
  past_due: "Pago atrasado",
  canceled: "Cancelada",
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
  const minElapsed = useMinLoadingElapsed();
  const [currentMode, setCurrentMode] = useState<SelectedMode>("local");
  const [selected, setSelected] = useState<SelectedMode>("local");
  const [migrateData, setMigrateData] = useState(true);
  // Por default false a propósito: reemplazar borra TODO lo que haya en el
  // destino (local o remoto) antes de escribir los datos nuevos — el usuario
  // tiene que marcarlo explícitamente, nunca es el comportamiento implícito.
  const [replaceData, setReplaceData] = useState(false);
  const [bucketConfig, setBucketConfig] = useState<BucketConfig>(EMPTY_BUCKET_CONFIG);
  const [bucketSecret, setBucketSecret] = useState("");
  const [bucketTesting, setBucketTesting] = useState(false);
  const [switching, setSwitching] = useState(false);
  const [exporting, setExporting] = useState(false);
  const [confirmOpen, setConfirmOpen] = useState(false);

  // --- Estado del panel de Nube ---
  const [cognitoEstablished, setCognitoEstablished] = useState<boolean | null>(null);
  const [businesses, setBusinesses] = useState<VendorBusiness[] | null>(null);
  const [businessesLoading, setBusinessesLoading] = useState(false);
  const [newBusinessName, setNewBusinessName] = useState("");
  const [creatingBusiness, setCreatingBusiness] = useState(false);
  const [selectedBusiness, setSelectedBusiness] = useState<VendorBusiness | null>(null);
  const [connectingBusinessId, setConnectingBusinessId] = useState<string | null>(null);
  const [connectOutcome, setConnectOutcome] = useState<"connected" | "linkedElsewhere" | null>(null);
  const [unlinking, setUnlinking] = useState(false);

  const load = useCallback(async () => {
    setLoading(true);
    try {
      const mode = await invoke<StorageMode>("storage_get_mode");
      setCurrentMode(mode.mode);
      setSelected(mode.mode);
      if (mode.mode === "nube") {
        // Ya conectada a un negocio: se muestra como "elegido" sin que haga
        // falta volver a tocar nada — el usuario solo ve esto si entra a
        // Ajustes mientras ya está en modo Nube.
        setSelectedBusiness({
          id: mode.businessId,
          businessName: mode.businessName,
          subscriptionStatus: null,
          hasDatabase: true,
          installationId: mode.installationId,
        });
        setConnectOutcome("connected");
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

  // Al entrar al panel de Nube, confirmar si ya hay una cuenta de Aura
  // Studio vinculada a esta instalación (puede que sí, de una vez anterior,
  // o porque ya se usa Nube) antes de decidir si mostrar el login.
  useEffect(() => {
    if (selected !== "nube" || cognitoEstablished !== null) return;
    invoke<{ established: boolean }>("cognito_installation_state")
      .then((state) => setCognitoEstablished(state.established))
      .catch(() => setCognitoEstablished(false));
  }, [selected, cognitoEstablished]);

  const loadBusinesses = useCallback(async () => {
    setBusinessesLoading(true);
    try {
      const list = await invoke<VendorBusiness[]>("vendor_list_businesses");
      setBusinesses(list);
    } catch (err) {
      toast.error(err instanceof Error ? err.message : "No se pudieron cargar tus negocios");
    } finally {
      setBusinessesLoading(false);
    }
  }, []);

  useEffect(() => {
    if (selected === "nube" && cognitoEstablished && businesses === null) {
      void loadBusinesses();
    }
  }, [selected, cognitoEstablished, businesses, loadBusinesses]);

  const connectToBusiness = useCallback(async (business: VendorBusiness) => {
    setConnectingBusinessId(business.id);
    setConnectOutcome(null);
    try {
      const preview = await invoke<VendorConnectPreview>("vendor_connect_business", {
        businessId: business.id,
      });
      setSelectedBusiness(business);
      setConnectOutcome(preview.outcome);
    } catch (err) {
      toast.error(err instanceof Error ? err.message : "No se pudo conectar con ese negocio");
    } finally {
      setConnectingBusinessId(null);
    }
  }, []);

  const createBusiness = async () => {
    const businessName = newBusinessName.trim();
    if (!businessName) return;
    setCreatingBusiness(true);
    try {
      const created = await invoke<VendorBusiness>("vendor_create_business", {
        input: { businessName },
      });
      setBusinesses((list) => [...(list ?? []), created]);
      setNewBusinessName("");
      toast.success("Negocio creado");
      await connectToBusiness(created);
    } catch (err) {
      toast.error(err instanceof Error ? err.message : "No se pudo crear el negocio");
    } finally {
      setCreatingBusiness(false);
    }
  };

  const unlinkAndRetry = async (business: VendorBusiness) => {
    setUnlinking(true);
    try {
      await invoke("vendor_unlink_installation", { businessId: business.id });
      toast.success("Instalación desvinculada");
      await connectToBusiness(business);
    } catch (err) {
      toast.error(err instanceof Error ? err.message : "No se pudo desvincular esa instalación");
    } finally {
      setUnlinking(false);
    }
  };

  const bucketFieldsComplete =
    bucketConfig.endpoint.trim() !== "" &&
    bucketConfig.region.trim() !== "" &&
    bucketConfig.bucket.trim() !== "" &&
    bucketConfig.accessKeyId.trim() !== "" &&
    bucketSecret !== "";

  // En modo Nube el bucket es opcional: el usuario puede guardar sin
  // configurar uno y simplemente no podrá subir fotos del catálogo hasta que
  // lo haga (ver `usePhotoUploadAvailability`). Dejarlo completamente vacío
  // es una elección válida; dejarlo a medias no lo es.
  const bucketFieldsEmpty =
    bucketConfig.endpoint.trim() === "" &&
    bucketConfig.region.trim() === "" &&
    bucketConfig.bucket.trim() === "" &&
    bucketConfig.accessKeyId.trim() === "" &&
    bucketSecret === "";

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

  // El diálogo de confirmación ya se cerró antes de llamar a esto (ver el
  // `onClick` de "Confirmar y reiniciar" más abajo) para no bloquear la
  // navegación mientras la migración corre — puede tardar varios minutos en
  // un negocio con mucho historial. El progreso en sí lo muestra
  // `StorageMigrationListener` (montado en el layout raíz, no acá), a partir
  // de los eventos `storage-migration-progress` que emite el backend; esta
  // función solo necesita el toast inicial (antes de que lleguen los
  // primeros eventos) y el resultado final, ambos con el mismo `id` para que
  // reemplacen al toast de progreso en vez de apilarse encima.
  const applySwitch = async () => {
    const willMigrate =
      (selected !== "local" && currentMode === "local" && migrateData) ||
      (selected === "local" && currentMode !== "local" && migrateData);
    if (willMigrate) {
      toast.loading("Conectando...", {
        id: STORAGE_MIGRATION_TOAST_ID,
        duration: Infinity,
      });
    }
    setSwitching(true);
    try {
      let summary: MigrationSummary | null = null;
      if (selected === "local") {
        const migrating = currentMode !== "local" && migrateData;
        summary = await invoke<MigrationSummary | null>("storage_switch_mode", {
          mode: "local",
          migrateData: migrating,
          replaceData: migrating && replaceData,
        });
      } else {
        if (!selectedBusiness) throw new Error("Elige un negocio primero");
        const migrating = currentMode === "local" && migrateData;
        summary = await invoke<MigrationSummary | null>("storage_switch_mode", {
          mode: "nube",
          nubeBusinessId: selectedBusiness.id,
          nubeBusinessName: selectedBusiness.businessName,
          migrateData: migrating,
          replaceData: migrating && replaceData,
          // El bucket es opcional en Nube: si se dejó todo vacío a propósito
          // (bucketFieldsEmpty), se manda null en vez del objeto con strings
          // vacíos para que el backend lo trate como "sin bucket todavía" y
          // no como datos incompletos.
          bucketConfig: bucketFieldsEmpty ? null : bucketConfig,
          bucketSecret: bucketFieldsEmpty ? null : bucketSecret,
        });
      }
      const successMessage = summary
        ? `Datos migrados: ${summary.customers} clientes, ${summary.appointments} citas, ${summary.sales} ventas, ${summary.imagesMigrated} imágenes. Reiniciando...`
        : "Modo de almacenamiento actualizado. Reiniciando...";
      if (willMigrate) {
        toast.success(successMessage, { id: STORAGE_MIGRATION_TOAST_ID });
      } else {
        toast.success(successMessage);
      }
      await relaunch();
    } catch (err) {
      const message =
        err instanceof Error ? err.message : "No se pudo cambiar el modo de almacenamiento";
      if (willMigrate) {
        toast.error(message, { id: STORAGE_MIGRATION_TOAST_ID });
      } else {
        toast.error(message);
      }
    } finally {
      setSwitching(false);
    }
  };

  if (loading || !minElapsed) return <SettingsSectionSkeleton rows={3} />;

  const canSave =
    selected === "local" ||
    (selected === "nube" &&
      connectOutcome === "connected" &&
      (bucketFieldsComplete || bucketFieldsEmpty));

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
            icon={<Cloud className="size-5" />}
            title="Nube"
            description="Conecta un negocio a la base de datos gestionada de tu cuenta de Aura Studio."
            selected={selected === "nube"}
            onSelect={() => setSelected("nube")}
          />
        </div>

        {selected === "nube" && (
          <div className="mt-5 space-y-4 rounded-2xl border border-border/60 p-4">
            {cognitoEstablished === null ? (
              <p className="text-sm text-muted-foreground">
                Verificando tu cuenta de Aura Studio...
              </p>
            ) : !cognitoEstablished ? (
              <CognitoAuthFlow
                onAuthenticated={() => setCognitoEstablished(true)}
                copy={{
                  loginSubtitle:
                    "Inicia sesión con tu cuenta de Aura Studio para ver y conectar tus negocios en la nube.",
                }}
              />
            ) : (
              <>
                <div>
                  <h3 className="font-medium">Tus negocios</h3>
                  <p className="text-xs text-muted-foreground">
                    Elige un negocio para conectarlo, o crea uno nuevo.
                  </p>
                </div>

                {businessesLoading ? (
                  <SkeletonRegion label="Cargando negocios…">
                    <ListSkeleton rows={3} />
                  </SkeletonRegion>
                ) : (
                  <div className="space-y-2">
                    {(businesses ?? []).map((business) => {
                      const isSelected = selectedBusiness?.id === business.id;
                      const isConnecting = connectingBusinessId === business.id;
                      return (
                        <div
                          key={business.id}
                          className={`rounded-xl border p-3 ${
                            isSelected && connectOutcome === "connected"
                              ? "border-primary bg-primary/5"
                              : "border-border/60"
                          }`}
                        >
                          <button
                            type="button"
                            disabled={isConnecting}
                            onClick={() => void connectToBusiness(business)}
                            className="flex w-full items-center justify-between gap-2 text-left disabled:opacity-60"
                          >
                            <span className="flex flex-wrap items-center gap-2">
                              <span className="font-medium">{business.businessName}</span>
                              {business.subscriptionStatus && (
                                <Badge variant="outline">
                                  {SUBSCRIPTION_LABELS[business.subscriptionStatus] ??
                                    business.subscriptionStatus}
                                </Badge>
                              )}
                              {business.hasDatabase && (
                                <span className="text-xs text-muted-foreground">
                                  ya tiene base de datos
                                </span>
                              )}
                            </span>
                            {isConnecting ? (
                              <span className="text-xs text-muted-foreground">Conectando...</span>
                            ) : (
                              isSelected &&
                              connectOutcome === "connected" && (
                                <Check className="size-4 shrink-0 text-primary" />
                              )
                            )}
                          </button>
                          {isSelected && connectOutcome === "linkedElsewhere" && (
                            <div className="mt-2 rounded-lg bg-destructive/10 p-2 text-xs text-destructive">
                              Este negocio ya está conectado desde otro equipo.
                              <div>
                                <Button
                                  type="button"
                                  size="sm"
                                  variant="outline"
                                  className="mt-2"
                                  disabled={unlinking}
                                  onClick={() => void unlinkAndRetry(business)}
                                >
                                  {unlinking
                                    ? "Desvinculando..."
                                    : "Desvincular ese equipo y conectar este"}
                                </Button>
                              </div>
                            </div>
                          )}
                        </div>
                      );
                    })}
                    {businesses !== null && businesses.length === 0 && (
                      <p className="text-sm text-muted-foreground">
                        Todavía no tienes negocios en la nube.
                      </p>
                    )}
                  </div>
                )}

                <div className="flex items-center gap-2">
                  <Input
                    placeholder="Nombre del nuevo negocio"
                    value={newBusinessName}
                    onChange={(e) => setNewBusinessName(e.target.value)}
                  />
                  <Button
                    type="button"
                    variant="secondary"
                    disabled={creatingBusiness || !newBusinessName.trim()}
                    onClick={() => void createBusiness()}
                  >
                    {creatingBusiness ? "Creando..." : "Crear negocio"}
                  </Button>
                </div>

                {connectOutcome === "connected" && (
                  <BucketConfigSection
                    bucketConfig={bucketConfig}
                    setBucketConfig={setBucketConfig}
                    bucketSecret={bucketSecret}
                    setBucketSecret={setBucketSecret}
                    bucketTesting={bucketTesting}
                    bucketFieldsComplete={bucketFieldsComplete}
                    onTestConnection={() => void testBucketConnection()}
                  />
                )}
              </>
            )}
          </div>
        )}

        <Button
          type="button"
          className="mt-5"
          disabled={!canSave || switching}
          onClick={() => setConfirmOpen(true)}
        >
          {switching ? "Cambiando..." : "Guardar y cambiar"}
        </Button>
      </FadeIn>

      <AlertDialog open={confirmOpen} onOpenChange={setConfirmOpen}>
        <AlertDialogContent>
          <AlertDialogHeader>
            <AlertDialogTitle>¿Cambiar el modo de almacenamiento?</AlertDialogTitle>
            <AlertDialogDescription>
              {selected === "local"
                ? currentMode !== "local" && migrateData
                  ? "La app copiará tus datos hacia una base local nueva y vacía, volverá a usarla y se reiniciará."
                  : "La app volverá a usar su base de datos local y se reiniciará."
                : currentMode === "local" && migrateData
                  ? `La app se conectará al negocio "${selectedBusiness?.businessName}" en la nube, copiará tus datos locales hacia él y se reiniciará.`
                  : `La app se conectará al negocio "${selectedBusiness?.businessName}" en la nube y se reiniciará. Los datos que tengas localmente no se copian.`}
            </AlertDialogDescription>
          </AlertDialogHeader>
          {currentMode === "local" && selected === "nube" && (
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
                      ? "Se copiarán tus clientes, empleados, servicios, citas, ventas y demás datos del negocio a esta base, junto con las imágenes de tu catálogo al bucket configurado arriba. Una vez migrados, la base local se vacía — esta base pasa a ser la única fuente de verdad."
                      : "La base de datos quedará vacía: tendrás que crear tu negocio y cargar tus datos desde cero."}
                  </span>
                </span>
              </label>
              {migrateData && (
                <label className="flex items-start gap-2 text-sm">
                  <Checkbox
                    className="mt-0.5"
                    checked={replaceData}
                    onCheckedChange={(checked) => setReplaceData(checked)}
                  />
                  <span>
                    Esta base ya tiene datos: reemplazarlos con los de Local
                    <span className="block text-xs text-destructive">
                      {replaceData
                        ? "Se borrará TODO lo que haya en esta base antes de copiar tus datos de Local — no se puede deshacer. Si no estás seguro, descarga primero el respaldo de abajo."
                        : "Si no la marcas y la base ya tiene datos, la migración se va a rechazar."}
                    </span>
                  </span>
                </label>
              )}
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
          {currentMode !== "local" && selected === "local" && (
            <div className="space-y-3">
              <label className="flex items-start gap-2 text-sm">
                <Checkbox
                  className="mt-0.5"
                  checked={migrateData}
                  onCheckedChange={(checked) => setMigrateData(checked)}
                />
                <span>
                  Traer mis datos de vuelta a este equipo
                  <span className="block text-xs text-muted-foreground">
                    {migrateData
                      ? "Se copiarán tus clientes, empleados, servicios, citas, ventas y demás datos del negocio a una base local nueva, junto con las imágenes de tu catálogo si hay un bucket configurado."
                      : "La base local quedará vacía: tendrás que crear tu negocio y cargar tus datos desde cero."}
                  </span>
                </span>
              </label>
              {migrateData && (
                <label className="flex items-start gap-2 text-sm">
                  <Checkbox
                    className="mt-0.5"
                    checked={replaceData}
                    onCheckedChange={(checked) => setReplaceData(checked)}
                  />
                  <span>
                    Ya tengo datos en Local: reemplazarlos con los de esta base
                    <span className="block text-xs text-destructive">
                      {replaceData
                        ? "Se borrará TODO lo que haya en tu base local antes de traer los datos — no se puede deshacer. Esta pantalla no puede respaldar la base local mientras está conectada a una base remota; si te importa lo que hay ahí, respáldalo por tu cuenta antes de continuar."
                        : "Si no la marcas y tu base local ya tiene datos, la migración se va a rechazar."}
                    </span>
                  </span>
                </label>
              )}
            </div>
          )}
          <AlertDialogFooter>
            <AlertDialogCancel disabled={switching}>Cancelar</AlertDialogCancel>
            <AlertDialogAction
              disabled={switching}
              onClick={(e) => {
                e.preventDefault();
                // Cierra el diálogo de inmediato en vez de esperar a que
                // `applySwitch` resuelva: la migración puede tardar varios
                // minutos y el diálogo, al ser modal, bloquearía el resto de
                // la app mientras tanto. El progreso se sigue mostrando en el
                // toast global (`StorageMigrationListener`), sin importar a
                // qué pantalla se navegue mientras tanto.
                setConfirmOpen(false);
                void applySwitch();
              }}
            >
              Confirmar y reiniciar
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

/**
 * Formulario de bucket S3-compatible para las fotos del catálogo del modo
 * "Nube" (issue #18: el backend gestionado no provee almacenamiento de
 * imágenes propio). El bucket es opcional: se puede guardar sin
 * configurarlo, solo que mientras tanto no se podrán subir fotos del
 * catálogo.
 */
function BucketConfigSection({
  bucketConfig,
  setBucketConfig,
  bucketSecret,
  setBucketSecret,
  bucketTesting,
  bucketFieldsComplete,
  onTestConnection,
}: {
  bucketConfig: BucketConfig;
  setBucketConfig: React.Dispatch<React.SetStateAction<BucketConfig>>;
  bucketSecret: string;
  setBucketSecret: (value: string) => void;
  bucketTesting: boolean;
  bucketFieldsComplete: boolean;
  onTestConnection: () => void;
}) {
  return (
    <div className="mt-4 space-y-4 rounded-2xl border border-border/60 p-4">
      <div>
        <h3 className="font-medium">Bucket de imágenes (opcional)</h3>
        <p className="text-xs text-muted-foreground">
          Las imágenes del catálogo se suben a este bucket S3-compatible (AWS
          S3, Cloudflare R2, Backblaze B2, DigitalOcean Spaces, MinIO u otro)
          en vez de quedar solo en este equipo. Puedes guardar sin
          configurarlo, pero no podrás subir fotos de productos, servicios ni
          paquetes hasta que lo hagas.
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
        Direccionamiento por path (necesario para MinIO y algunos proveedores
        no-AWS)
      </label>

      <Button
        type="button"
        variant="secondary"
        disabled={bucketTesting || !bucketFieldsComplete}
        onClick={onTestConnection}
      >
        {bucketTesting ? "Probando..." : "Probar conexión"}
      </Button>
    </div>
  );
}
