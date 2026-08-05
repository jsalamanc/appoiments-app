export type CatalogStatus = "draft" | "published";

export const CATALOG_STATUS_LABELS: Record<CatalogStatus, string> = {
  draft: "Borrador",
  published: "Publicado",
};

export function isCatalogStatus(value: string): value is CatalogStatus {
  return value === "draft" || value === "published";
}
