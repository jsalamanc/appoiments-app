"use client";

import { useEffect, useState } from "react";

import { Badge } from "@/components/ui/badge";

export function NetworkStatusHeader() {
  const [online, setOnline] = useState(true);

  useEffect(() => {
    const sync = () => setOnline(navigator.onLine);
    sync();

    window.addEventListener("online", sync);
    window.addEventListener("offline", sync);
    return () => {
      window.removeEventListener("online", sync);
      window.removeEventListener("offline", sync);
    };
  }, []);

  return (
    <header className="sticky top-0 z-20 border-b border-border/50 bg-background/55 px-4 py-3 backdrop-blur-xl backdrop-saturate-150 md:px-8">
      <div className="mx-auto flex max-w-5xl items-center justify-end">
        {online ? (
          <Badge variant="secondary" className="gap-1.5">
            <span
              aria-hidden
              className="size-1.5 rounded-full bg-emerald-500"
            />
            Online
          </Badge>
        ) : (
          <Badge variant="destructive" className="gap-1.5">
            <span
              aria-hidden
              className="size-1.5 rounded-full bg-destructive"
            />
            Offline
          </Badge>
        )}
      </div>
    </header>
  );
}
