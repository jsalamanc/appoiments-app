# 11 — Reportes exportables (Excel / PDF)

**Estado:** implementado
**Dependencia:** vistas de Reportes (`/admin/reports/*`)

## Qué es

Descargar o imprimir los reportes del admin para contabilidad, gerencia o
archivo.

## Para qué sirve

- Compartir cifras fuera de la app
- Archivo mensual sin capturas de pantalla

## Alcance implementado

1. **Excel:** descarga CSV (abre en Excel) del dataset del periodo —
   `src/lib/export-csv.ts`.
2. **PDF:** botón que abre el diálogo de impresión del navegador
   (`window.print()`) con estilos `@media print` en `globals.css`
   (`.print-hidden` oculta sidebar/filtros/botones en la impresión).
3. Botones reutilizables vía `ReportExportButtons`
   (`src/components/admin/report-export-buttons.tsx`), usados en las 4
   vistas de reportes: empleados, servicios, no-shows y comisiones
   (`src/components/admin/reports-{employees,services,no-shows,commissions}.tsx`).
4. El recibo de venta imprimible (`/admin/sales/receipt`, issue #2 Fase 5)
   reusa el mismo patrón `window.print()` + `.print-hidden`, aunque no es
   técnicamente un "reporte".

## Fuera de alcance (futuro)

- Generación PDF server-side (librería dedicada)
- Excel `.xlsx` nativo
- Envío automático por correo
