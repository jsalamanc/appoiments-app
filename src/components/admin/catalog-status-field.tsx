"use client";

import { Label } from "@/components/ui/label";
import { CATALOG_STATUS_LABELS } from "@/lib/catalog-status";

type CatalogStatusFieldProps = {
  id: string;
  value: "draft" | "published";
  onChange: (value: "draft" | "published") => void;
};

export function CatalogStatusField({
  id,
  value,
  onChange,
}: CatalogStatusFieldProps) {
  return (
    <div className="space-y-2">
      <Label htmlFor={id}>Estado</Label>
      <select
        id={id}
        className="h-8 w-full rounded-lg border border-input bg-transparent px-2.5 text-sm"
        value={value}
        onChange={(e) =>
          onChange(e.target.value as "draft" | "published")
        }
      >
        <option value="draft">{CATALOG_STATUS_LABELS.draft}</option>
        <option value="published">{CATALOG_STATUS_LABELS.published}</option>
      </select>
      <p className="text-xs text-muted-foreground">
        Los borradores solo los ves tú; lo publicado estará listo para mostrar
        cuando abras el catálogo al público.
      </p>
    </div>
  );
}
