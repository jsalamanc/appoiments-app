"use client";

import { Clock, Pencil, Plus, Trash2 } from "lucide-react";
import Image from "next/image";
import { useRouter } from "next/navigation";
import { useState } from "react";
import { toast } from "sonner";

import { CatalogFilters } from "@/components/admin/catalog-filters";
import { CatalogPagination } from "@/components/admin/catalog-pagination";
import { DeleteConfirm } from "@/components/admin/delete-confirm";
import { EmptyState } from "@/components/admin/empty-state";
import { ServiceFormDialog } from "@/components/admin/service-form-dialog";
import { StatusBadge } from "@/components/admin/status-badge";
import { FadeIn, StaggerChildren, StaggerItem } from "@/components/motion/fade-in";
import { Button } from "@/components/ui/button";
import { deleteCatalogImage } from "@/lib/catalog";
import type { CatalogCounts } from "@/lib/catalog-list";
import { formatCurrency, formatDuration } from "@/lib/format";
import type { StatusFilter } from "@/lib/pagination";
import { createClient } from "@/lib/supabase/client";
import type { Service } from "@/types/database";

const PATHNAME = "/admin/services";

type ServicesManagerProps = {
  services: Service[];
  status: StatusFilter;
  counts: CatalogCounts;
  page: number;
  totalPages: number;
  total: number;
  pageSize: number;
};

export function ServicesManager({
  services,
  status,
  counts,
  page,
  totalPages,
  total,
  pageSize,
}: ServicesManagerProps) {
  const router = useRouter();
  const [open, setOpen] = useState(false);
  const [selected, setSelected] = useState<Service | null>(null);
  const [toDelete, setToDelete] = useState<Service | null>(null);
  const [deleting, setDeleting] = useState(false);

  const refresh = () => router.refresh();
  const hasAny = counts.all > 0;

  const handleDelete = async () => {
    if (!toDelete) return;
    setDeleting(true);
    const supabase = createClient();
    const photoUrl = toDelete.photo_url;
    const { error } = await supabase
      .from("services")
      .delete()
      .eq("id", toDelete.id);

    if (error) {
      setDeleting(false);
      toast.error(error.message);
      return;
    }

    await deleteCatalogImage(photoUrl);
    setDeleting(false);
    toast.success("Servicio eliminado");
    setToDelete(null);
    refresh();
  };

  return (
    <div className="mx-auto max-w-5xl space-y-6">
      <FadeIn className="flex flex-col gap-4 sm:flex-row sm:items-end sm:justify-between">
        <div>
          <p className="text-sm text-muted-foreground">Catálogo</p>
          <h1 className="font-heading text-4xl font-semibold">Servicios</h1>
          <p className="mt-1 text-muted-foreground">
            Define tratamientos con precio y tiempo estimado.
          </p>
        </div>
        <Button
          onClick={() => {
            setSelected(null);
            setOpen(true);
          }}
        >
          <Plus data-icon="inline-start" />
          Nuevo servicio
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
          title="Aún no hay servicios"
          description="Agrega tu primer servicio: nombre, precio y duración estimada."
          actionLabel="Crear servicio"
          onAction={() => {
            setSelected(null);
            setOpen(true);
          }}
        />
      ) : services.length === 0 ? (
        <EmptyState
          title="No hay resultados"
          description="No hay servicios con ese estado. Prueba otro filtro."
        />
      ) : (
        <>
          <StaggerChildren className="grid gap-4 sm:grid-cols-2">
            {services.map((service) => (
              <StaggerItem key={service.id}>
                <article className="flex overflow-hidden rounded-3xl border border-border/70 bg-card shadow-sm">
                  <div className="relative w-28 shrink-0 bg-muted sm:w-36">
                    {service.photo_url ? (
                      <Image
                        src={service.photo_url}
                        alt={service.name}
                        fill
                        className="object-cover"
                        unoptimized
                      />
                    ) : (
                      <div className="flex h-full items-center justify-center bg-mint/50 px-2 text-center text-xs text-muted-foreground">
                        Sin foto
                      </div>
                    )}
                  </div>
                  <div className="flex flex-1 flex-col gap-3 p-4">
                    <div>
                      <div className="mb-1.5 flex items-start justify-between gap-2">
                        <h3 className="font-heading text-xl font-semibold">
                          {service.name}
                        </h3>
                        <StatusBadge status={service.status} />
                      </div>
                      {service.description && (
                        <p className="mt-1 line-clamp-2 text-sm text-muted-foreground">
                          {service.description}
                        </p>
                      )}
                      <div className="mt-2 flex flex-wrap items-center gap-3 text-sm">
                        <span>{formatCurrency(Number(service.price))}</span>
                        <span className="inline-flex items-center gap-1 text-muted-foreground">
                          <Clock className="size-3.5" />
                          {formatDuration(service.duration_minutes)}
                        </span>
                      </div>
                    </div>
                    <div className="mt-auto flex gap-2">
                      <Button
                        variant="outline"
                        size="sm"
                        onClick={() => {
                          setSelected(service);
                          setOpen(true);
                        }}
                      >
                        <Pencil data-icon="inline-start" />
                        Editar
                      </Button>
                      <Button
                        variant="ghost"
                        size="sm"
                        onClick={() => setToDelete(service)}
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

      <ServiceFormDialog
        open={open}
        onOpenChange={setOpen}
        service={selected}
        onSuccess={refresh}
      />

      <DeleteConfirm
        open={!!toDelete}
        onOpenChange={(value) => !value && setToDelete(null)}
        title="Eliminar servicio"
        description={`¿Seguro que quieres eliminar “${toDelete?.name}”?`}
        loading={deleting}
        onConfirm={handleDelete}
      />
    </div>
  );
}
