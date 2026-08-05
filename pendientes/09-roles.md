# 09 — Roles (dueño / recepción / profesional)

**Estado:** parcial — dueño y recepción implementados; profesional diferido  
**Dependencia:** migraciones `008_center_members.sql`

## Qué es

Permitir que varias personas operen el mismo centro con permisos distintos:

- **Dueño:** todo (ajustes, módulos, reportes, catálogo, caja, equipo)
- **Recepción:** agenda, clientes, ventas, Hoy, catálogo operativo; sin ajustes ni reportes
- **Profesional:** (pendiente) ver su agenda y marcar estados; sin caja ni ajustes

## Implementado

1. Tabla `center_members` + `center_invites`
2. `current_center_id()` vía membresía
3. Invitación de recepción desde Ajustes
4. Sidebar / `OwnerGate` ocultan ajustes y reportes a recepción

## Pendiente

- Rol **profesional**
- Mostrar email del miembro (hoy se muestra id corto)
- Revocar invitaciones pendientes desde UI
