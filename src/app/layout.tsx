import type { Metadata } from "next";
import { Cormorant_Garamond, Nunito } from "next/font/google";
import { StorageMigrationListener } from "@/components/storage-migration-listener";
import { UpdateChecker } from "@/components/update-checker";
import { Toaster } from "@/components/ui/sonner";
import { WebviewZoomGuard } from "@/components/webview-zoom-guard";
import { loginDisplayFont, loginSansFont } from "@/lib/fonts";

import "./globals.css";

const APP_NAME = "Aura Studio";
const APP_DEFAULT_TITLE = "Aura Studio — Gestión de citas y negocios";
const APP_TITLE_TEMPLATE = "%s · Aura Studio";
const APP_DESCRIPTION =
  "Administra productos, servicios, citas y paquetes de tu negocio con elegancia.";

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
      className={`${display.variable} ${body.variable} ${loginDisplayFont.variable} ${loginSansFont.variable} h-full antialiased`}
    >
      <body className="flex min-h-full flex-col font-sans">
        <WebviewZoomGuard />
        <UpdateChecker />
        <StorageMigrationListener />
        {children}
        <Toaster richColors position="top-center" />
      </body>
    </html>
  );
}
