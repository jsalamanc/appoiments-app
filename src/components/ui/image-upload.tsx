"use client";

import { open } from "@tauri-apps/plugin-dialog";
import { ImagePlus, X } from "lucide-react";
import Image from "next/image";

import { Button } from "@/components/ui/button";
import { usePhotoSrc } from "@/lib/use-photo-src";
import { cn } from "@/lib/utils";

type ImageUploadProps = {
  /** Ruta absoluta del archivo recién elegido en el file picker nativo,
   * pendiente de copiarse al directorio de datos de la app (aún no se ha
   * llamado a `import_*_photo`). */
  value?: string | null;
  /** `photo_path` relativo ya guardado en la base (si se está editando). */
  previewPath?: string | null;
  onChange: (sourceFilePath: string | null) => void;
  className?: string;
};

export function ImageUpload({
  value,
  previewPath,
  onChange,
  className,
}: ImageUploadProps) {
  const savedSrc = usePhotoSrc(value ? null : previewPath);
  const pendingFileName = value ? value.split(/[\\/]/).pop() : null;

  const pick = async () => {
    const selected = await open({
      multiple: false,
      filters: [
        { name: "Imágenes", extensions: ["png", "jpg", "jpeg", "gif", "webp"] },
      ],
    });
    if (typeof selected === "string") {
      onChange(selected);
    }
  };

  return (
    <div className={cn("space-y-2", className)}>
      <button
        type="button"
        onClick={() => void pick()}
        className={cn(
          "relative flex h-36 w-full items-center justify-center overflow-hidden rounded-2xl border border-dashed border-border bg-muted/40 transition hover:border-primary/50 hover:bg-muted/70",
          (savedSrc || pendingFileName) && "border-solid"
        )}
      >
        {pendingFileName ? (
          <div className="flex flex-col items-center gap-1 px-4 text-center text-muted-foreground">
            <ImagePlus className="size-6 text-primary" />
            <span className="text-sm">{pendingFileName}</span>
            <span className="text-xs">Se guardará al confirmar</span>
          </div>
        ) : savedSrc ? (
          <Image
            src={savedSrc}
            alt="Vista previa"
            fill
            className="object-cover"
            unoptimized
          />
        ) : (
          <div className="flex flex-col items-center gap-2 text-muted-foreground">
            <ImagePlus className="size-6 text-primary" />
            <span className="text-sm">Foto opcional</span>
          </div>
        )}
      </button>
      {(value || previewPath) && (
        <Button
          type="button"
          variant="ghost"
          size="sm"
          onClick={() => onChange(null)}
        >
          <X data-icon="inline-start" />
          Quitar foto
        </Button>
      )}
    </div>
  );
}
