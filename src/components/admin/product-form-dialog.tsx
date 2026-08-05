"use client";

import { zodResolver } from "@hookform/resolvers/zod";
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
import {
  deleteCatalogImage,
  getCurrentCenter,
  uploadCatalogImage,
} from "@/lib/catalog";
import { setProductStock } from "@/lib/inventory";
import { createClient } from "@/lib/supabase/client";
import { productSchema, type ProductValues } from "@/lib/validations";
import type { Product } from "@/types/database";

type ProductFormDialogProps = {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  product?: Product | null;
  onSuccess: () => void;
};

export function ProductFormDialog({
  open,
  onOpenChange,
  product,
  onSuccess,
}: ProductFormDialogProps) {
  const [photo, setPhoto] = useState<File | null>(null);
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
    defaultValues: { name: "", price: 0, stock: 0, status: "draft" },
  });

  const status = watch("status");

  useEffect(() => {
    if (open) {
      reset({
        name: product?.name ?? "",
        price: product?.price ?? 0,
        stock: product?.stock ?? 0,
        status: product?.status ?? "draft",
      });
      setPhoto(null);
      setClearPhoto(false);
    }
  }, [open, product, reset]);

  const save = async (values: ProductValues) => {
    setLoading(true);
    try {
      const supabase = createClient();
      const center = await getCurrentCenter();
      if (!center) throw new Error("No se encontró el centro");

      const previousUrl = product?.photo_url ?? null;
      let photoUrl = previousUrl;
      if (clearPhoto && !photo) photoUrl = null;
      if (photo) {
        photoUrl = await uploadCatalogImage(center.id, photo, "products");
      }

      const payload = {
        name: values.name,
        price: values.price,
        photo_url: photoUrl,
        status: values.status,
      };

      if (product) {
        const { error } = await supabase
          .from("products")
          .update(payload)
          .eq("id", product.id);
        if (error) throw error;

        if (values.stock !== product.stock) {
          await setProductStock(product.id, values.stock, "Ajuste manual");
        }

        toast.success(
          values.status === "published"
            ? "Producto actualizado y publicado"
            : "Producto guardado como borrador"
        );
      } else {
        const { data: created, error } = await supabase
          .from("products")
          .insert({
            center_id: center.id,
            ...payload,
            stock: 0,
          })
          .select("id")
          .single();
        if (error) throw error;

        if (values.stock > 0) {
          await setProductStock(created.id, values.stock, "Alta de producto");
        }

        toast.success(
          values.status === "published"
            ? "Producto creado y publicado"
            : "Producto guardado como borrador"
        );
      }

      if (previousUrl && previousUrl !== photoUrl) {
        await deleteCatalogImage(previousUrl);
      }

      onOpenChange(false);
      onSuccess();
    } catch (err) {
      toast.error(err instanceof Error ? err.message : "Error al guardar");
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
              value={photo}
              previewUrl={clearPhoto ? null : product?.photo_url}
              onChange={(file) => {
                setPhoto(file);
                setClearPhoto(file === null);
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
