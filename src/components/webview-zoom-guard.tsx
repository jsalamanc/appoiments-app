"use client";

import { useEffect } from "react";

/**
 * Esta es una app de escritorio (Tauri), no una página web: hacer zoom al
 * contenido no tiene sentido — la UI ya se adapta al tamaño de la ventana
 * del sistema operativo via CSS responsivo. `zoomHotkeysEnabled: false` en
 * `tauri.conf.json` ya bloquea Ctrl/Cmd +/-/0 a nivel de WebView, pero esa
 * opción de Tauri no cubre todos los gestos de zoom en todas las
 * plataformas (ej. pinch-to-zoom de trackpad en macOS, o Ctrl + rueda del
 * mouse) — este componente los bloquea a nivel de JS como refuerzo,
 * cualquiera sea la plataforma/WebView.
 */
export function WebviewZoomGuard() {
  useEffect(() => {
    const preventCtrlWheelZoom = (event: WheelEvent) => {
      if (event.ctrlKey || event.metaKey) {
        event.preventDefault();
      }
    };

    const preventZoomHotkeys = (event: KeyboardEvent) => {
      const isZoomKey = ["=", "+", "-", "_", "0"].includes(event.key);
      if ((event.ctrlKey || event.metaKey) && isZoomKey) {
        event.preventDefault();
      }
    };

    // `gesturestart`/`gesturechange` son eventos no estándar que solo
    // dispara WebKit (Safari/WKWebView en macOS) para el gesto de pinch del
    // trackpad — no existen en el DOM lib de TypeScript.
    const preventPinchZoom = (event: Event) => event.preventDefault();

    window.addEventListener("wheel", preventCtrlWheelZoom, { passive: false });
    window.addEventListener("keydown", preventZoomHotkeys);
    window.addEventListener("gesturestart", preventPinchZoom);
    window.addEventListener("gesturechange", preventPinchZoom);

    return () => {
      window.removeEventListener("wheel", preventCtrlWheelZoom);
      window.removeEventListener("keydown", preventZoomHotkeys);
      window.removeEventListener("gesturestart", preventPinchZoom);
      window.removeEventListener("gesturechange", preventPinchZoom);
    };
  }, []);

  return null;
}
