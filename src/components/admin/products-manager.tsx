"use client";

import { Pencil, Plus, Trash2 } from "lucide-react";
import Image from "next/image";
import { useRouter } from "next/navigation";
import { useState } from "react";
import { toast } from "sonner";

import { CatalogFilters } from "@/components/admin/catalog-filters";
import { CatalogPagination } from "@/components/admin/catalog-pagination";
import { DeleteConfirm } from "@/components/admin/delete-confirm";
import { EmptyState } from "@/components/admin/empty-state";
import { ProductFormDialog } from "@/components/admin/product-form-dialog";
import { StatusBadge } from "@/components/admin/status-badge";
import { FadeIn, StaggerChildren, StaggerItem } from "@/components/motion/fade-in";
import { Button } from "@/components/ui/button";
import { deleteCatalogImage } from "@/lib/catalog";
import type { CatalogCounts } from "@/lib/catalog-list";
import { formatCurrency } from "@/lib/format";
import type { StatusFilter } from "@/lib/pagination";
import { createClient } from "@/lib/supabase/client";
import type { Product } from "@/types/database";

const PATHNAME = "/admin/products";

type ProductsManagerProps = {
  products: Product[];
  status: StatusFilter;
  counts: CatalogCounts;
  page: number;
  totalPages: number;
  total: number;
  pageSize: number;
  /** null = sin límite */
  productLimit: number | null;
};

export function ProductsManager({
  products,
  status,
  counts,
  page,
  totalPages,
  total,
  pageSize,
  productLimit,
}: ProductsManagerProps) {
  const router = useRouter();
  const [open, setOpen] = useState(false);
  const [selected, setSelected] = useState<Product | null>(null);
  const [toDelete, setToDelete] = useState<Product | null>(null);
  const [deleting, setDeleting] = useState(false);

  const refresh = () => router.refresh();
  const hasAny = counts.all > 0;
  const atLimit =
    productLimit !== null && counts.all >= productLimit;

  const openCreate = () => {
    if (atLimit) {
      toast.error(
        `Tu plan Free permite máximo ${productLimit} productos. Cambia a Starter o Growth para agregar más.`
      );
      return;
    }
    setSelected(null);
    setOpen(true);
  };

  const handleDelete = async () => {
    if (!toDelete) return;
    setDeleting(true);
    const supabase = createClient();
    const photoUrl = toDelete.photo_url;
    const { error } = await supabase
      .from("products")
      .delete()
      .eq("id", toDelete.id);

    if (error) {
      setDeleting(false);
      toast.error(error.message);
      return;
    }

    await deleteCatalogImage(photoUrl);
    setDeleting(false);
    toast.success("Producto eliminado");
    setToDelete(null);
    refresh();
  };

  return (
    <div className="mx-auto max-w-5xl space-y-6">
      <FadeIn className="flex flex-col gap-4 sm:flex-row sm:items-end sm:justify-between">
        <div>
          <p className="text-sm text-muted-foreground">Catálogo</p>
          <h1 className="font-heading text-4xl font-semibold">Productos</h1>
          <p className="mt-1 text-muted-foreground">
            Sube productos para venta o para incluirlos en paquetes.
            {productLimit !== null && (
              <>
                {" "}
                Plan Free: {counts.all}/{productLimit} productos.
              </>
            )}
          </p>
        </div>
        <Button onClick={openCreate} disabled={atLimit}>
          <Plus data-icon="inline-start" />
          Nuevo producto
        </Button>
      </FadeIn>

      {hasAny && (
        <FadeIn delay={0.05}>
          <CatalogFilters
            pathname={PATHNAME}
            status={status}
            counts={counts}
          />
        </FadeIn>
      )}

      {!hasAny ? (
        <EmptyState
          title="Aún no hay productos"
          description="Crea tu primer producto con nombre, precio y una foto opcional."
          actionLabel={atLimit ? undefined : "Crear producto"}
          onAction={atLimit ? undefined : openCreate}
        />
      ) : products.length === 0 ? (
        <EmptyState
          title="No hay resultados"
          description="No hay productos con ese estado. Prueba otro filtro."
        />
      ) : (
        <>
          <StaggerChildren className="grid gap-4 sm:grid-cols-2 lg:grid-cols-3">
            {products.map((product) => (
              <StaggerItem key={product.id}>
                <article className="overflow-hidden rounded-3xl border border-border/70 bg-card/70">
                  <div className="relative h-40 bg-muted">
                    {product.photo_url ? (
                      <Image
                        src={product.photo_url}
                        alt={product.name}
                        fill
                        className="object-cover"
                        unoptimized
                      />
                    ) : (
                      <div className="flex h-full items-center justify-center bg-blush/40 text-sm text-muted-foreground">
                        Sin foto
                      </div>
                    )}
                  </div>
                  <div className="space-y-3 p-4">
                    <div>
                      <div className="mb-1.5 flex items-start justify-between gap-2">
                        <h3 className="font-heading text-xl font-semibold">
                          {product.name}
                        </h3>
                        <StatusBadge status={product.status} />
                      </div>
                      <p className="text-muted-foreground">
                        {formatCurrency(Number(product.price))}
                      </p>
                      <p className="text-sm text-muted-foreground">
                        Stock: {product.stock}
                      </p>
                    </div>
                    <div className="flex gap-2">
                      <Button
                        variant="outline"
                        size="sm"
                        onClick={() => {
                          setSelected(product);
                          setOpen(true);
                        }}
                      >
                        <Pencil data-icon="inline-start" />
                        Editar
                      </Button>
                      <Button
                        variant="ghost"
                        size="sm"
                        onClick={() => setToDelete(product)}
                      >
                        <Trash2 data-icon="inline-start" />
                        Eliminar
                      </Button>
                    </div>
                  </div>
                </article>
              </StaggerItem>
            ))}
          </StaggerChildren>

          <CatalogPagination
            pathname={PATHNAME}
            status={status}
            page={page}
            totalPages={totalPages}
            total={total}
            pageSize={pageSize}
          />
        </>
      )}

      <ProductFormDialog
        open={open}
        onOpenChange={setOpen}
        product={selected}
        onSuccess={refresh}
      />

      <DeleteConfirm
        open={!!toDelete}
        onOpenChange={(value) => !value && setToDelete(null)}
        title="Eliminar producto"
        description={`¿Seguro que quieres eliminar “${toDelete?.name}”?`}
        loading={deleting}
        onConfirm={handleDelete}
      />
    </div>
  );
}
