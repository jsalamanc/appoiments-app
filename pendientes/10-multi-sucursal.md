# 10 — Multi-sucursal

**Estado:** diferido (plan Business)  
**Dependencia:** Roles / memberships (`09-roles.md`) — hoy `user_id` es único en `center_members` (un centro por persona)

## Qué es

Una cuenta u organización puede operar **varios centros** (sucursales), con catálogo, agenda, clientes y caja separados por sucursal, y un selector de sucursal activa en el admin.

## Para qué sirve

- Cadenas o dueños con más de un local
- Reportes y operación por sede sin mezclar datos

## Alcance futuro sugerido (MVP)

1. Quitar `unique (user_id)` en `center_members` o modelar organización.
2. Selector de sucursal en layout admin (persistir sucursal activa).
3. `current_center_id` alineado con la sucursal elegida.
4. Clientes/catálogo por sucursal al inicio.

## Fuera de alcance por ahora

- No implementar selector ni multi-membership hasta retomarlo.
