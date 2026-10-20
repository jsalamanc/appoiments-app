import type { Metadata } from "next";
import { Cormorant_Garamond, Nunito } from "next/font/google";
import { Toaster } from "@/components/ui/sonner";

import "./globals.css";

const APP_NAME = "Aura Studio";
const APP_DEFAULT_TITLE = "Aura Studio — Gestión para centros estéticos";
const APP_TITLE_TEMPLATE = "%s · Aura Studio";
const APP_DESCRIPTION =
  "Administra productos, servicios y paquetes de tu centro estético con elegancia.";

const display = Cormorant_Garamond({
  variable: "--font-display",
  subsets: ["latin"],
  weight: ["400", "500", "600", "700"],
});

const body = Nunito({
  variable: "--font-body",
  subsets: ["latin"],
  weight: ["400", "500", "600", "700"],
});

export const metadata: Metadata = {
  applicationName: APP_NAME,
  title: {
    default: APP_DEFAULT_TITLE,
    template: APP_TITLE_TEMPLATE,
  },
  description: APP_DESCRIPTION,
};

export default function RootLayout({
  children,
}: Readonly<{
  children: React.ReactNode;
}>) {
  return (
    <html
      lang="es"
      className={`${display.variable} ${body.variable} h-full antialiased`}
    >
      <body className="flex min-h-full flex-col font-sans">
        {children}
        <Toaster richColors position="top-center" />
      </body>
    </html>
  );
}
