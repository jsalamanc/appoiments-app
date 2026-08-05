# 11 — Reportes exportables (Excel / PDF)

**Estado:** MVP hecho  
**Dependencia:** vistas de Reportes (`/admin/reports/*`)

## Qué es

Descargar o imprimir los reportes del admin (empleados, servicios/ticket, no-shows) para contabilidad, gerencia o archivo.

## Para qué sirve

- Compartir cifras fuera de la app
- Archivo mensual sin capturas de pantalla

## Alcance MVP (implementado)

1. **Excel:** descarga CSV (abre en Excel) del dataset del periodo — `src/lib/export-csv.ts`.
2. **PDF:** botón que abre el diálogo de impresión del navegador (`window.print`) con estilos `@media print` en `globals.css`.
3. Botones en las 3 vistas vía `ReportExportButtons`.

## Fuera de alcance (futuro)

- Generación PDF server-side (librería dedicada)
- Excel `.xlsx` nativo
- Envío automático por correo
