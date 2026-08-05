import { Badge } from "@/components/ui/badge";
import {
  CATALOG_STATUS_LABELS,
  type CatalogStatus,
} from "@/lib/catalog-status";
import { cn } from "@/lib/utils";

export function StatusBadge({
  status,
  className,
}: {
  status: CatalogStatus;
  className?: string;
}) {
  return (
    <Badge
      variant={status === "published" ? "secondary" : "outline"}
      className={cn(
        status === "published"
          ? "bg-mint/70 text-accent-foreground"
          : "bg-cream/80 text-muted-foreground",
        className
      )}
    >
      {CATALOG_STATUS_LABELS[status]}
    </Badge>
  );
}
