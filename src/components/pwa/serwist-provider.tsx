"use client";

import { SerwistProvider as Provider } from "@serwist/turbopack/react";
import type { ReactNode } from "react";

export function SerwistProvider({ children }: { children: ReactNode }) {
  return (
    <Provider
      swUrl="/serwist/sw.js"
      disable={process.env.NODE_ENV === "development"}
    >
      {children}
    </Provider>
  );
}
