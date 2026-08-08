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
import { PackageFormDialog } from "@/components/admin/package-form-dialog";
import { StatusBadge } from "@/components/admin/status-badge";
import { FadeIn, StaggerChildren, StaggerItem } from "@/components/motion/fade-in";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { deleteCatalogImage } from "@/lib/catalog";
import type { CatalogCounts } from "@/lib/catalog-list";
import { formatCurrency } from "@/lib/format";
import type { StatusFilter } from "@/lib/pagination";
import { createClient } from "@/lib/supabase/client";
import type { PackageWithItems, Product, Service } from "@/types/database";

const PATHNAME = "/admin/packages";

type PackagesManagerProps = {
  packages: PackageWithItems[];
  products: Product[];
  services: Service[];
  status: StatusFilter;
  counts: CatalogCounts;
  page: number;
  totalPages: number;
  total: number;
  pageSize: number;
};

export function PackagesManager({
  packages,
  products,
  services,
  status,
  counts,
  page,
  totalPages,
  total,
  pageSize,
}: PackagesManagerProps) {
  const router = useRouter();
  const [open, setOpen] = useState(false);
  const [selected, setSelected] = useState<PackageWithItems | null>(null);
  const [toDelete, setToDelete] = useState<PackageWithItems | null>(null);
  const [deleting, setDeleting] = useState(false);

  const refresh = () => router.refresh();
  const canCreate = products.length > 0 || services.length > 0;
  const hasAny = counts.all > 0;

  const handleDelete = async () => {
    if (!toDelete) return;
    setDeleting(true);
    const supabase = createClient();
    const photoUrl = toDelete.photo_url;
    const { error } = await supabase
      .from("packages")
      .delete()
      .eq("id", toDelete.id);

    if (error) {
      setDeleting(false);
      toast.error(error.message);
      return;
    }

    await deleteCatalogImage(photoUrl);
    setDeleting(false);
    toast.success("Paquete eliminado");
    setToDelete(null);
    refresh();
  };

  return (
    <div className="mx-auto max-w-5xl space-y-6">
      <FadeIn className="flex flex-col gap-4 sm:flex-row sm:items-end sm:justify-between">
        <div>
          <p className="text-sm text-muted-foreground">Catálogo</p>
          <h1 className="font-heading text-4xl font-semibold">Paquetes</h1>
          <p className="mt-1 text-muted-foreground">
            Agrupa productos y servicios con cantidades bajo un precio especial.
          </p>
        </div>
        <Button
          disabled={!canCreate}
          onClick={() => {
            setSelected(null);
            setOpen(true);
          }}
        >
          <Plus data-icon="inline-start" />
          Nuevo paquete
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

      {!canCreate ? (
        <EmptyState
          title="Primero crea productos o servicios"
          description="Los paquetes necesitan al menos un producto o servicio existente para armarse."
        />
      ) : !hasAny ? (
        <EmptyState
          title="Aún no hay paquetes"
          description="Crea un paquete con uno o varios ítems y define tu propio precio."
          actionLabel="Crear paquete"
          onAction={() => {
            setSelected(null);
            setOpen(true);
          }}
        />
      ) : packages.length === 0 ? (
        <EmptyState
          title="No hay resultados"
          description="No hay paquetes con ese estado. Prueba otro filtro."
        />
      ) : (
        <>
          <StaggerChildren className="grid gap-4">
            {packages.map((pkg) => (
              <StaggerItem key={pkg.id}>
                <article className="flex flex-col overflow-hidden rounded-3xl border border-border/70 bg-card shadow-sm sm:flex-row">
                  <div className="relative h-40 shrink-0 bg-muted sm:h-auto sm:w-44">
                    {pkg.photo_url ? (
                      <Image
                        src={pkg.photo_url}
                        alt={pkg.name}
                        fill
                        className="object-cover"
                        unoptimized
                      />
                    ) : (
                      <div className="flex h-full min-h-40 items-center justify-center bg-lavender/50 text-sm text-muted-foreground">
                        Sin foto
                      </div>
                    )}
                  </div>
                  <div className="flex flex-1 flex-col gap-3 p-5">
                    <div className="flex flex-wrap items-start justify-between gap-3">
                      <div>
                        <div className="mb-1 flex items-center gap-2">
                          <h3 className="font-heading text-2xl font-semibold">
                            {pkg.name}
                          </h3>
                          <StatusBadge status={pkg.status} />
                        </div>
                        {pkg.description && (
                          <p className="mt-1 text-sm text-muted-foreground">
                            {pkg.description}
                          </p>
                        )}
                      </div>
                      <p className="font-heading text-2xl font-semibold">
                        {formatCurrency(Number(pkg.price))}
                      </p>
                    </div>

                    <div className="flex flex-wrap gap-2">
                      {pkg.package_items.map((item) => {
                        const label =
                          item.item_type === "product"
                            ? item.products?.name
                            : item.services?.name;
                        return (
                          <Badge key={item.id} variant="secondary">
                            {item.quantity}× {label ?? "Ítem"}
                          </Badge>
                        );
                      })}
                    </div>

                    <div className="mt-auto flex gap-2">
                      <Button
                        variant="outline"
                        size="sm"
                        onClick={() => {
                          setSelected(pkg);
                          setOpen(true);
                        }}
                      >
                        <Pencil data-icon="inline-start" />
                        Editar
                      </Button>
                      <Button
                        variant="ghost"
                        size="sm"
                        onClick={() => setToDelete(pkg)}
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

      <PackageFormDialog
        open={open}
        onOpenChange={setOpen}
        pkg={selected}
        products={products}
        services={services}
        onSuccess={refresh}
      />

      <DeleteConfirm
        open={!!toDelete}
        onOpenChange={(value) => !value && setToDelete(null)}
        title="Eliminar paquete"
        description={`¿Seguro que quieres eliminar “${toDelete?.name}”?`}
        loading={deleting}
        onConfirm={handleDelete}
      />
    </div>
  );
}
