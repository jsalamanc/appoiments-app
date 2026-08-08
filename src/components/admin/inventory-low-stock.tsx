"use client";

import { PackageOpen, Pencil } from "lucide-react";
import Image from "next/image";
import Link from "next/link";
import { useRouter } from "next/navigation";
import { useState } from "react";

import { EmptyState } from "@/components/admin/empty-state";
import { ProductFormDialog } from "@/components/admin/product-form-dialog";
import { FadeIn, StaggerChildren, StaggerItem } from "@/components/motion/fade-in";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { formatCurrency } from "@/lib/format";
import type { Product } from "@/types/database";

type InventoryLowStockProps = {
  outOfStock: Product[];
  lowStock: Product[];
  threshold: number;
};

function ProductRow({
  product,
  onEdit,
}: {
  product: Product;
  onEdit: (product: Product) => void;
}) {
  return (
    <article className="flex flex-col gap-3 rounded-3xl border border-border/70 bg-card shadow-sm p-4 sm:flex-row sm:items-center sm:justify-between">
      <div className="flex items-center gap-3">
        <div className="relative size-14 overflow-hidden rounded-2xl bg-muted">
          {product.photo_url ? (
            <Image
              src={product.photo_url}
              alt={product.name}
              fill
              className="object-cover"
              unoptimized
            />
          ) : (
            <div className="flex h-full items-center justify-center text-xs text-muted-foreground">
              —
            </div>
          )}
        </div>
        <div>
          <p className="font-heading text-xl font-semibold">{product.name}</p>
          <p className="text-sm text-muted-foreground">
            {formatCurrency(Number(product.price))}
          </p>
        </div>
      </div>
      <div className="flex items-center gap-3">
        <Badge variant={product.stock <= 0 ? "destructive" : "secondary"}>
          Stock: {product.stock}
        </Badge>
        <Button size="sm" variant="outline" onClick={() => onEdit(product)}>
          <Pencil data-icon="inline-start" />
          Editar
        </Button>
      </div>
    </article>
  );
}

export function InventoryLowStock({
  outOfStock,
  lowStock,
  threshold,
}: InventoryLowStockProps) {
  const router = useRouter();
  const [selected, setSelected] = useState<Product | null>(null);
  const [open, setOpen] = useState(false);
  const total = outOfStock.length + lowStock.length;

  const edit = (product: Product) => {
    setSelected(product);
    setOpen(true);
  };

  return (
    <div className="mx-auto max-w-5xl space-y-6">
      <FadeIn className="flex flex-col gap-4 sm:flex-row sm:items-end sm:justify-between">
        <div>
          <p className="text-sm text-muted-foreground">Catálogo</p>
          <h1 className="font-heading text-4xl font-semibold">Inventario</h1>
          <p className="mt-1 text-muted-foreground">
            Productos con stock bajo (≤ {threshold}) o sin stock.
          </p>
        </div>
        <Button variant="outline" render={<Link href="/admin/products" />}>
          Ver todos los productos
        </Button>
      </FadeIn>

      {total === 0 ? (
        <EmptyState
          title="Inventario en buen estado"
          description={`Ningún producto está en 0 ni por debajo de ${threshold} unidades.`}
          actionLabel="Ir a productos"
          onAction={() => router.push("/admin/products")}
        />
      ) : (
        <>
          {outOfStock.length > 0 && (
            <FadeIn delay={0.05} className="space-y-3">
              <div className="flex items-center gap-2">
                <PackageOpen className="size-4 text-destructive" />
                <h2 className="font-heading text-2xl font-semibold">
                  Sin stock ({outOfStock.length})
                </h2>
              </div>
              <StaggerChildren className="space-y-3">
                {outOfStock.map((product) => (
                  <StaggerItem key={product.id}>
                    <ProductRow product={product} onEdit={edit} />
                  </StaggerItem>
                ))}
              </StaggerChildren>
            </FadeIn>
          )}

          {lowStock.length > 0 && (
            <FadeIn delay={0.08} className="space-y-3">
              <h2 className="font-heading text-2xl font-semibold">
                Stock bajo ({lowStock.length})
              </h2>
              <StaggerChildren className="space-y-3">
                {lowStock.map((product) => (
                  <StaggerItem key={product.id}>
                    <ProductRow product={product} onEdit={edit} />
                  </StaggerItem>
                ))}
              </StaggerChildren>
            </FadeIn>
          )}
        </>
      )}

      <ProductFormDialog
        open={open}
        onOpenChange={setOpen}
        product={selected}
        onSuccess={() => router.refresh()}
      />
    </div>
  );
}
