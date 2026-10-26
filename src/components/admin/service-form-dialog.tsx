"use client";

import { zodResolver } from "@hookform/resolvers/zod";
import { invoke } from "@tauri-apps/api/core";
import { useEffect, useState } from "react";
import { useForm } from "react-hook-form";
import { toast } from "sonner";

import { CatalogStatusField } from "@/components/admin/catalog-status-field";
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { ImageUpload } from "@/components/ui/image-upload";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Textarea } from "@/components/ui/textarea";
import type { CatalogStatus } from "@/lib/catalog-status";
import { withPhotoRollback } from "@/lib/photo-path";
import { serviceSchema, type ServiceValues } from "@/lib/validations";
import type { Service } from "@/types/database";

type ServiceFormDialogProps = {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  service?: Service | null;
  onSuccess: () => void;
};

type ServiceInputPayload = {
  name: string;
  description: string | null;
  price: number;
  durationMinutes: number;
  photoPath: string | null;
  status: CatalogStatus;
};

export function ServiceFormDialog({
  open,
  onOpenChange,
  service,
  onSuccess,
}: ServiceFormDialogProps) {
  const [pendingPhoto, setPendingPhoto] = useState<string | null>(null);
  const [clearPhoto, setClearPhoto] = useState(false);
  const [loading, setLoading] = useState(false);

  const {
    register,
    handleSubmit,
    reset,
    setValue,
    watch,
    formState: { errors },
  } = useForm<ServiceValues>({
    resolver: zodResolver(serviceSchema),
    defaultValues: {
      name: "",
      description: "",
      price: 0,
      duration_minutes: 60,
      status: "draft",
    },
  });

  const status = watch("status");

  useEffect(() => {
    if (open) {
      reset({
        name: service?.name ?? "",
        description: service?.description ?? "",
        price: service?.price ?? 0,
        duration_minutes: service?.duration_minutes ?? 60,
        status: service?.status ?? "draft",
      });
      setPendingPhoto(null);
      setClearPhoto(false);
    }
  }, [open, service, reset]);

  const save = async (values: ServiceValues) => {
    setLoading(true);
    try {
      const basePhotoPath = clearPhoto ? null : service?.photo_path ?? null;

      if (service) {
        let photoPath = basePhotoPath;
        let freshPhotoPath: string | null = null;
        if (pendingPhoto) {
          freshPhotoPath = await invoke<string>("import_service_photo", {
            serviceId: service.id,
            sourceFilePath: pendingPhoto,
          });
          photoPath = freshPhotoPath;
        }

        const payload: ServiceInputPayload = {
          name: values.name,
          description: values.description || null,
          price: values.price,
          durationMinutes: values.duration_minutes,
          photoPath,
          status: values.status,
        };
        await withPhotoRollback(freshPhotoPath, () =>
          invoke("update_service", { id: service.id, input: payload })
        );

        toast.success(
          values.status === "published"
            ? "Servicio actualizado y publicado"
            : "Servicio guardado como borrador"
        );
      } else {
        const createPayload: ServiceInputPayload = {
          name: values.name,
          description: values.description || null,
          price: values.price,
          durationMinutes: values.duration_minutes,
          photoPath: null,
          status: values.status,
        };
        const created = await invoke<Service>("create_service", {
          input: createPayload,
        });

        if (pendingPhoto) {
          const photoPath = await invoke<string>("import_service_photo", {
            serviceId: created.id,
            sourceFilePath: pendingPhoto,
          });
          await withPhotoRollback(photoPath, () =>
            invoke("update_service", {
              id: created.id,
              input: { ...createPayload, photoPath },
            })
          );
        }

        toast.success(
          values.status === "published"
            ? "Servicio creado y publicado"
            : "Servicio guardado como borrador"
        );
      }

      onOpenChange(false);
      onSuccess();
    } catch (err) {
      toast.error(err instanceof Error ? err.message : String(err));
    } finally {
      setLoading(false);
    }
  };

  const submitWithStatus = (nextStatus: CatalogStatus) => {
    setValue("status", nextStatus, { shouldValidate: true });
    void handleSubmit((values) => save({ ...values, status: nextStatus }))();
  };

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="flex max-h-[90vh] flex-col gap-0 overflow-hidden p-0 sm:max-w-md">
        <DialogHeader className="shrink-0 px-4 pt-4">
          <DialogTitle>
            {service ? "Editar servicio" : "Nuevo servicio"}
          </DialogTitle>
          <DialogDescription>
            Nombre, precio y tiempo estimado son obligatorios.
          </DialogDescription>
        </DialogHeader>

        <form
          onSubmit={handleSubmit(save)}
          className="flex min-h-0 flex-1 flex-col"
        >
          <div className="space-y-4 overflow-y-auto px-4 py-4">
            <ImageUpload
              value={pendingPhoto}
              previewPath={clearPhoto ? null : service?.photo_path}
              onChange={(path) => {
                setPendingPhoto(path);
                setClearPhoto(path === null);
              }}
            />
            <div className="space-y-2">
              <Label htmlFor="service-name">Nombre</Label>
              <Input id="service-name" {...register("name")} />
              {errors.name && (
                <p className="text-xs text-destructive">{errors.name.message}</p>
              )}
            </div>
            <div className="space-y-2">
              <Label htmlFor="service-description">Descripción (opcional)</Label>
              <Textarea
                id="service-description"
                rows={3}
                {...register("description")}
              />
            </div>
            <div className="grid grid-cols-2 gap-3">
              <div className="space-y-2">
                <Label htmlFor="service-price">Precio</Label>
                <Input
                  id="service-price"
                  type="number"
                  min={0}
                  step="100"
                  {...register("price", { valueAsNumber: true })}
                />
                {errors.price && (
                  <p className="text-xs text-destructive">
                    {errors.price.message}
                  </p>
                )}
              </div>
              <div className="space-y-2">
                <Label htmlFor="service-duration">Tiempo (min)</Label>
                <Input
                  id="service-duration"
                  type="number"
                  min={5}
                  step={5}
                  {...register("duration_minutes", { valueAsNumber: true })}
                />
                {errors.duration_minutes && (
                  <p className="text-xs text-destructive">
                    {errors.duration_minutes.message}
                  </p>
                )}
              </div>
            </div>

            <CatalogStatusField
              id="service-status"
              value={status}
              onChange={(value) =>
                setValue("status", value, { shouldValidate: true })
              }
            />
          </div>

          <div className="shrink-0 space-y-2 border-t bg-muted/50 p-4">
            {service ? (
              <div className="flex flex-col-reverse gap-2 sm:flex-row sm:justify-end">
                <Button
                  type="button"
                  variant="outline"
                  onClick={() => onOpenChange(false)}
                >
                  Cancelar
                </Button>
                <Button type="submit" disabled={loading}>
                  {loading ? "Guardando..." : "Guardar cambios"}
                </Button>
              </div>
            ) : (
              <>
                <div className="grid grid-cols-1 gap-2 sm:grid-cols-2">
                  <Button
                    type="button"
                    variant="secondary"
                    disabled={loading}
                    onClick={() => submitWithStatus("draft")}
                  >
                    {loading ? "Guardando..." : "Guardar borrador"}
                  </Button>
                  <Button
                    type="button"
                    disabled={loading}
                    onClick={() => submitWithStatus("published")}
                  >
                    {loading ? "Publicando..." : "Guardar y publicar"}
                  </Button>
                </div>
                <Button
                  type="button"
                  variant="ghost"
                  className="w-full"
                  onClick={() => onOpenChange(false)}
                >
                  Cancelar
                </Button>
              </>
            )}
          </div>
        </form>
      </DialogContent>
    </Dialog>
  );
}
