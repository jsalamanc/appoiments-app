# 10 — Multi-sucursal

**Estado:** no implementado — este documento describía la dependencia sobre
`center_members` (arquitectura Supabase/multi-tenant vieja), que no existe
en la edición Tauri/Rust de escritorio actual (un solo negocio, y hoy un
solo local, por instalación).

**Seguimiento:** issue [#28](https://github.com/jsalamanc/appoiments-app/issues/28)
("Soporte multi-sucursal para un mismo negocio") — replantea la idea sobre
la arquitectura actual en vez de sobre memberships de Supabase.

## Qué era

Una cuenta u organización podía operar varios centros (sucursales), con
catálogo, agenda, clientes y caja separados por sucursal, y un selector de
sucursal activa en el admin — dependía de quitar `unique(user_id)` en
`center_members`, una tabla que no existe en la edición actual.
