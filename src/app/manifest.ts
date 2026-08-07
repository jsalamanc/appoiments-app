import type { MetadataRoute } from "next";

export default function manifest(): MetadataRoute.Manifest {
  return {
    name: "Aura Studio — Gestión para centros estéticos",
    short_name: "Aura Studio",
    description:
      "Administra productos, servicios y paquetes de tu centro estético con elegancia.",
    start_url: "/",
    display: "standalone",
    orientation: "portrait-primary",
    background_color: "#faf6f8",
    theme_color: "#b89bb8",
    lang: "es",
    categories: ["business", "productivity"],
    icons: [
      {
        src: "/icons/icon-192x192.png",
        sizes: "192x192",
        type: "image/png",
      },
      {
        src: "/icons/icon-512x512.png",
        sizes: "512x512",
        type: "image/png",
      },
      {
        src: "/icons/icon-maskable-192x192.png",
        sizes: "192x192",
        type: "image/png",
        purpose: "maskable",
      },
      {
        src: "/icons/icon-maskable-512x512.png",
        sizes: "512x512",
        type: "image/png",
        purpose: "maskable",
      },
    ],
  };
}
