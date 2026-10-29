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
import type { CatalogStatus } from "@/lib/catalog-status";
import { withPhotoRollback } from "@/lib/photo-path";
import { productSchema, type ProductValues } from "@/lib/validations";
import type { Product } from "@/types/database";

type ProductFormDialogProps = {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  product?: Product | null;
  onSuccess: () => void;
};

type ProductInputPayload = {
  name: string;
  price: number;
  cost: number;
  photoPath: string | null;
  status: CatalogStatus;
};

export function ProductFormDialog({
  open,
  onOpenChange,
  product,
  onSuccess,
}: ProductFormDialogProps) {
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
  } = useForm<ProductValues>({
    resolver: zodResolver(productSchema),
    defaultValues: { name: "", price: 0, cost: 0, stock: 0, status: "draft" },
  });

  const status = watch("status");

  useEffect(() => {
    if (open) {
      reset({
        name: product?.name ?? "",
        price: product?.price ?? 0,
        cost: product?.cost ?? 0,
        stock: product?.stock ?? 0,
        status: product?.status ?? "draft",
      });
      setPendingPhoto(null);
      setClearPhoto(false);
    }
  }, [open, product, reset]);

  const save = async (values: ProductValues) => {
    setLoading(true);
    try {
      const basePhotoPath = clearPhoto ? null : product?.photo_path ?? null;

      if (product) {
        let photoPath = basePhotoPath;
        let freshPhotoPath: string | null = null;
        if (pendingPhoto) {
          freshPhotoPath = await invoke<string>("import_product_photo", {
            productId: product.id,
            sourceFilePath: pendingPhoto,
          });
          photoPath = freshPhotoPath;
        }

        const payload: ProductInputPayload = {
          name: values.name,
          price: values.price,
          cost: values.cost,
          photoPath,
          status: values.status,
        };
        await withPhotoRollback(freshPhotoPath, () =>
          invoke("update_product", { id: product.id, input: payload })
        );

        if (values.stock !== product.stock) {
          const isRestock = values.stock > product.stock;
          await invoke("set_product_stock", {
            productId: product.id,
            stock: values.stock,
            note: "Ajuste manual",
            unitCost: isRestock ? values.cost : null,
          });
        }

        toast.success(
          values.status === "published"
            ? "Producto actualizado y publicado"
            : "Producto guardado como borrador"
        );
      } else {
        const createPayload: ProductInputPayload = {
          name: values.name,
          price: values.price,
          cost: values.cost,
          photoPath: null,
          status: values.status,
        };
        const created = await invoke<Product>("create_product", {
          input: createPayload,
        });

        if (pendingPhoto) {
          const photoPath = await invoke<string>("import_product_photo", {
            productId: created.id,
            sourceFilePath: pendingPhoto,
          });
          await withPhotoRollback(photoPath, () =>
            invoke("update_product", {
              id: created.id,
              input: { ...createPayload, photoPath },
            })
          );
        }

        if (values.stock > 0) {
          await invoke("set_product_stock", {
            productId: created.id,
            stock: values.stock,
            note: "Alta de producto",
            unitCost: values.cost,
          });
        }

        toast.success(
          values.status === "published"
            ? "Producto creado y publicado"
            : "Producto guardado como borrador"
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
            {product ? "Editar producto" : "Nuevo producto"}
          </DialogTitle>
          <DialogDescription>
            Foto opcional. Nombre, precio y stock son obligatorios.
          </DialogDescription>
        </DialogHeader>

        <form
          onSubmit={handleSubmit(save)}
          className="flex min-h-0 flex-1 flex-col"
        >
          <div className="space-y-4 overflow-y-auto px-4 py-4">
            <ImageUpload
              value={pendingPhoto}
              previewPath={clearPhoto ? null : product?.photo_path}
              onChange={(path) => {
                setPendingPhoto(path);
                setClearPhoto(path === null);
              }}
            />
            <div className="space-y-2">
              <Label htmlFor="product-name">Nombre</Label>
              <Input id="product-name" {...register("name")} />
              {errors.name && (
                <p className="text-xs text-destructive">{errors.name.message}</p>
              )}
            </div>
            <div className="space-y-2">
              <Label htmlFor="product-price">Precio</Label>
              <Input
                id="product-price"
                type="number"
                min={0}
                step="100"
                {...register("price", { valueAsNumber: true })}
              />
              {errors.price && (
                <p className="text-xs text-destructive">{errors.price.message}</p>
              )}
            </div>
            <div className="space-y-2">
              <Label htmlFor="product-cost">Costo (opcional)</Label>
              <Input
                id="product-cost"
                type="number"
                min={0}
                step="100"
                {...register("cost", { valueAsNumber: true })}
              />
              {errors.cost && (
                <p className="text-xs text-destructive">{errors.cost.message}</p>
              )}
              <p className="text-xs text-muted-foreground">
                Lo que te cuesta a ti, para calcular margen — no se muestra al
                cliente.
              </p>
            </div>
            <div className="space-y-2">
              <Label htmlFor="product-stock">Stock</Label>
              <Input
                id="product-stock"
                type="number"
                min={0}
                step={1}
                {...register("stock", { valueAsNumber: true })}
              />
              {errors.stock && (
                <p className="text-xs text-destructive">{errors.stock.message}</p>
              )}
            </div>

            <CatalogStatusField
              id="product-status"
              value={status}
              onChange={(value) =>
                setValue("status", value, { shouldValidate: true })
              }
            />
          </div>

          <div className="shrink-0 space-y-2 border-t bg-muted/50 p-4">
            {product ? (
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
