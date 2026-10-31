# 09 — Roles (dueño / staff)

**Estado:** implementado (modelo simple, sin membresías)
**Dependencia:** ninguna — ya no aplica `008_center_members.sql` (arquitectura
Supabase/multi-tenant vieja, no existe en la edición actual)

## Qué es

Esta es la edición Tauri/Rust de escritorio: **un solo negocio por
instalación**, sin `center_id` ni `centers`/`plans`/`center_members`. Los
roles son una columna (`users.role`) con dos valores, no una tabla de
membresías:

- **`owner`:** todo (ajustes, módulos, reportes, comisiones, catálogo, caja,
  equipo, anular ventas).
- **`staff`:** agenda, clientes, ventas, catálogo operativo; sin ajustes,
  sin reportes, sin anular ventas. Es un solo rol operativo, no distingue
  "recepción" de "profesional".

## Implementado

1. `users.role` (`'owner' | 'staff'`, ver `src-tauri/src/models/user.rs`).
2. `AppState::require_user()`/`require_owner()` (`src-tauri/src/state.rs`) —
   cada comando de Tauri declara cuál exige.
3. El dueño crea cuentas de staff directamente (`create_staff_user`,
   `src-tauri/src/commands/auth.rs`) — sin invitación por correo, sin
   aceptación externa: es local, de escritorio.
4. `OwnerGate` (frontend) oculta ajustes/reportes/comisiones/anular venta a
   staff; el sidebar también filtra los enlaces correspondientes.

## Pendiente (si se retoma)

- Un rol intermedio tipo "profesional" (ver solo su propia agenda, marcar
  estados de sus citas, sin ver caja ni clientes de otros) — hoy no existe
  ninguna distinción dentro de `staff`.
- Desactivar/reactivar una cuenta de staff ya existe (`set_user_active`);
  falta poder cambiarle el rol sin recrearla.
