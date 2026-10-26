"use client";

import { zodResolver } from "@hookform/resolvers/zod";
import { invoke } from "@tauri-apps/api/core";
import { Plus, Trash2 } from "lucide-react";
import { useEffect, useState } from "react";
import { useFieldArray, useForm } from "react-hook-form";
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
import { packageSchema, type PackageValues } from "@/lib/validations";
import type { PackageWithItems, Product, Service } from "@/types/database";

type PackageFormDialogProps = {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  pkg?: PackageWithItems | null;
  products: Product[];
  services: Service[];
  onSuccess: () => void;
};

type PackageInputPayload = {
  name: string;
  description: string | null;
  price: number;
  photoPath: string | null;
  status: CatalogStatus;
  items: { itemType: "product" | "service"; itemId: string; quantity: number }[];
};

export function PackageFormDialog({
  open,
  onOpenChange,
  pkg,
  products,
  services,
  onSuccess,
}: PackageFormDialogProps) {
  const [pendingPhoto, setPendingPhoto] = useState<string | null>(null);
  const [clearPhoto, setClearPhoto] = useState(false);
  const [loading, setLoading] = useState(false);

  const {
    register,
    control,
    handleSubmit,
    reset,
    watch,
    setValue,
    formState: { errors },
  } = useForm<PackageValues>({
    resolver: zodResolver(packageSchema),
    defaultValues: {
      name: "",
      description: "",
      price: 0,
      status: "draft",
      items: [{ item_type: "service", item_id: "", quantity: 1 }],
    },
  });

  const { fields, append, remove } = useFieldArray({
    control,
    name: "items",
  });

  const status = watch("status");

  useEffect(() => {
    if (open) {
      reset({
        name: pkg?.name ?? "",
        description: pkg?.description ?? "",
        price: pkg?.price ?? 0,
        status: pkg?.status ?? "draft",
        items:
          pkg?.package_items?.map((item) => ({
            item_type: item.item_type,
            item_id:
              item.item_type === "product"
                ? item.product_id ?? ""
                : item.service_id ?? "",
            quantity: item.quantity,
          })) ?? [{ item_type: "service" as const, item_id: "", quantity: 1 }],
      });
      setPendingPhoto(null);
      setClearPhoto(false);
    }
  }, [open, pkg, reset]);

  const save = async (values: PackageValues) => {
    setLoading(true);
    try {
      const basePhotoPath = clearPhoto ? null : pkg?.photo_path ?? null;
      let photoPath = basePhotoPath;
      let freshPhotoPath: string | null = null;
      if (pendingPhoto && pkg) {
        freshPhotoPath = await invoke<string>("import_package_photo", {
          packageId: pkg.id,
          sourceFilePath: pendingPhoto,
        });
        photoPath = freshPhotoPath;
      }

      const payload: PackageInputPayload = {
        name: values.name,
        description: values.description || null,
        price: values.price,
        photoPath,
        status: values.status,
        items: values.items.map((item) => ({
          itemType: item.item_type,
          itemId: item.item_id,
          quantity: item.quantity,
        })),
      };

      if (pkg) {
        await withPhotoRollback(freshPhotoPath, () =>
          invoke("update_package", { id: pkg.id, input: payload })
        );
      } else {
        const created = await invoke<{ id: string }>("create_package", {
          input: payload,
        });

        if (pendingPhoto) {
          const createdPhotoPath = await invoke<string>(
            "import_package_photo",
            {
              packageId: created.id,
              sourceFilePath: pendingPhoto,
            }
          );
          await withPhotoRollback(createdPhotoPath, () =>
            invoke("update_package", {
              id: created.id,
              input: { ...payload, photoPath: createdPhotoPath },
            })
          );
        }
      }

      toast.success(
        values.status === "published"
          ? pkg
            ? "Paquete actualizado y publicado"
            : "Paquete creado y publicado"
          : "Paquete guardado como borrador"
      );
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
      <DialogContent className="flex max-h-[90vh] flex-col gap-0 overflow-hidden p-0 sm:max-w-lg">
        <DialogHeader className="shrink-0 px-4 pt-4">
          <DialogTitle>{pkg ? "Editar paquete" : "Nuevo paquete"}</DialogTitle>
          <DialogDescription>
            Combina productos y servicios con cantidades bajo un precio propio.
          </DialogDescription>
        </DialogHeader>

        <form
          onSubmit={handleSubmit(save)}
          className="flex min-h-0 flex-1 flex-col"
        >
          <div className="space-y-4 overflow-y-auto px-4 py-4">
            <ImageUpload
              value={pendingPhoto}
              previewPath={clearPhoto ? null : pkg?.photo_path}
              onChange={(path) => {
                setPendingPhoto(path);
                setClearPhoto(path === null);
              }}
            />

            <div className="space-y-2">
              <Label htmlFor="package-name">Nombre</Label>
              <Input id="package-name" {...register("name")} />
              {errors.name && (
                <p className="text-xs text-destructive">{errors.name.message}</p>
              )}
            </div>

            <div className="space-y-2">
              <Label htmlFor="package-description">Descripción (opcional)</Label>
              <Textarea
                id="package-description"
                rows={3}
                {...register("description")}
              />
            </div>

            <div className="space-y-2">
              <Label htmlFor="package-price">Precio del paquete</Label>
              <Input
                id="package-price"
                type="number"
                min={0}
                step="100"
                {...register("price", { valueAsNumber: true })}
              />
              {errors.price && (
                <p className="text-xs text-destructive">{errors.price.message}</p>
              )}
            </div>

            <div className="space-y-3">
              <div className="flex items-center justify-between">
                <Label>Ítems del paquete</Label>
                <Button
                  type="button"
                  variant="outline"
                  size="sm"
                  onClick={() =>
                    append({ item_type: "service", item_id: "", quantity: 1 })
                  }
                >
                  <Plus data-icon="inline-start" />
                  Agregar
                </Button>
              </div>

              {fields.map((field, index) => {
                const itemType = watch(`items.${index}.item_type`);
                const options =
                  itemType === "product"
                    ? products.map((p) => ({ id: p.id, label: p.name }))
                    : services.map((s) => ({ id: s.id, label: s.name }));

                return (
                  <div
                    key={field.id}
                    className="grid gap-2 rounded-2xl border border-border/70 bg-muted/30 p-3 sm:grid-cols-[120px_1fr_90px_auto]"
                  >
                    <select
                      className="h-8 rounded-lg border border-input bg-transparent px-2 text-sm"
                      {...register(`items.${index}.item_type`, {
                        onChange: () =>
                          setValue(`items.${index}.item_id`, "", {
                            shouldValidate: true,
                          }),
                      })}
                    >
                      <option value="service">Servicio</option>
                      <option value="product">Producto</option>
                    </select>

                    <select
                      className="h-8 rounded-lg border border-input bg-transparent px-2 text-sm"
                      {...register(`items.${index}.item_id`)}
                    >
                      <option value="">Seleccionar...</option>
                      {options.map((option) => (
                        <option key={option.id} value={option.id}>
                          {option.label}
                        </option>
                      ))}
                    </select>

                    <Input
                      type="number"
                      min={1}
                      placeholder="Cant."
                      {...register(`items.${index}.quantity`, {
                        valueAsNumber: true,
                      })}
                    />

                    <Button
                      type="button"
                      variant="ghost"
                      size="icon-sm"
                      disabled={fields.length === 1}
                      onClick={() => remove(index)}
                    >
                      <Trash2 />
                    </Button>

                    {(errors.items?.[index]?.item_id ||
                      errors.items?.[index]?.quantity) && (
                      <p className="text-xs text-destructive sm:col-span-4">
                        {errors.items?.[index]?.item_id?.message ||
                          errors.items?.[index]?.quantity?.message}
                      </p>
                    )}
                  </div>
                );
              })}

              {errors.items?.root && (
                <p className="text-xs text-destructive">
                  {errors.items.root.message}
                </p>
              )}
              {typeof errors.items?.message === "string" && (
                <p className="text-xs text-destructive">{errors.items.message}</p>
              )}
            </div>

            <CatalogStatusField
              id="package-status"
              value={status}
              onChange={(value) =>
                setValue("status", value, { shouldValidate: true })
              }
            />
          </div>

          <div className="shrink-0 space-y-2 border-t bg-muted/50 p-4">
            {pkg ? (
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
