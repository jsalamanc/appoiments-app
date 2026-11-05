import { Fraunces, Geist } from "next/font/google";

/**
 * Tipografía del landing (aura-studio-landing), migrada primero a la vista
 * de login/onboarding (ver `.login-view` en `globals.css`). Deliberadamente
 * separada de las fuentes del resto de la app (`layout.tsx`: Cormorant
 * Garamond + Nunito) — variables con nombre propio para no pisarlas.
 */
export const loginDisplayFont = Fraunces({
  variable: "--font-login-display",
  subsets: ["latin"],
  axes: ["opsz", "SOFT", "WONK"],
  style: ["normal", "italic"],
});

export const loginSansFont = Geist({
  variable: "--font-login-sans",
  subsets: ["latin"],
});
