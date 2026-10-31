# 12 — Portal público de reserva

**Estado:** no implementado en la edición actual — este documento describía
la arquitectura Supabase/multi-tenant vieja (RPCs, `center_id`, página
pública `/r/[slug]` por centro), que no existe en la edición Tauri/Rust de
escritorio actual (un solo negocio por instalación, sin servidor público).

**Seguimiento:** issue [#26](https://github.com/jsalamanc/appoiments-app/issues/26)
("Agenda pública de autoreserva para clientes") — ahí se replantea qué
necesitaría esta funcionalidad en la arquitectura actual (un backend propio
accesible desde afuera, o el modo "base de datos propia"/Postgres ya
existente como base), no el enfoque original de RPCs de Supabase descrito
aquí originalmente.

## Qué era

Página pública `/r/[slug]` donde el cliente elegía servicio, profesional,
horario y dejaba sus datos para crear una cita — dependía de RPCs
(`get_public_booking_catalog`, `get_public_busy_ranges`,
`create_public_booking`) y de las migraciones `009`/`010` de la arquitectura
Supabase, ninguna de las cuales existe hoy.
