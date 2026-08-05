"use client";

import { ImagePlus, X } from "lucide-react";
import Image from "next/image";
import { useEffect, useRef, useState } from "react";

import { Button } from "@/components/ui/button";
import { cn } from "@/lib/utils";

type ImageUploadProps = {
  value?: File | null;
  previewUrl?: string | null;
  onChange: (file: File | null) => void;
  className?: string;
};

export function ImageUpload({
  value,
  previewUrl,
  onChange,
  className,
}: ImageUploadProps) {
  const inputRef = useRef<HTMLInputElement>(null);
  const [localPreview, setLocalPreview] = useState<string | null>(null);

  useEffect(() => {
    if (!value) {
      setLocalPreview(null);
      return;
    }
    const url = URL.createObjectURL(value);
    setLocalPreview(url);
    return () => URL.revokeObjectURL(url);
  }, [value]);

  const src = localPreview || previewUrl || null;

  return (
    <div className={cn("space-y-2", className)}>
      <button
        type="button"
        onClick={() => inputRef.current?.click()}
        className={cn(
          "relative flex h-36 w-full items-center justify-center overflow-hidden rounded-2xl border border-dashed border-border bg-muted/40 transition hover:border-primary/50 hover:bg-muted/70",
          src && "border-solid"
        )}
      >
        {src ? (
          <Image
            src={src}
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
      <input
        ref={inputRef}
        type="file"
        accept="image/*"
        className="hidden"
        onChange={(e) => onChange(e.target.files?.[0] ?? null)}
      />
      {src && (
        <Button
          type="button"
          variant="ghost"
          size="sm"
          onClick={() => {
            onChange(null);
            if (inputRef.current) inputRef.current.value = "";
          }}
        >
          <X data-icon="inline-start" />
          Quitar foto
        </Button>
      )}
    </div>
  );
}
