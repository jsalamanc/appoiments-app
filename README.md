# Aura Studio — Centros Estéticos

App admin para centros estéticos: registro, login, agenda de citas, ventas e inventario.

## Stack

- Next.js (App Router) + TypeScript + Tailwind
- Supabase (Auth, Postgres, Storage)
- react-hook-form + Zod
- shadcn/ui + Framer Motion

## Setup

1. Instala dependencias:

```bash
npm install
```

2. Copia variables de entorno:

```bash
cp .env.example .env.local
```

3. En Supabase Dashboard → **SQL Editor**, ejecuta en orden:

- [`supabase/migrations/001_init.sql`](supabase/migrations/001_init.sql) (proyecto nuevo)
- [`supabase/migrations/002_catalog_status.sql`](supabase/migrations/002_catalog_status.sql) (si ya corriste el 001 antes de esta actualización)
- [`supabase/migrations/003_sales.sql`](supabase/migrations/003_sales.sql) (clientes y ventas)
- [`supabase/migrations/004_customer_document.sql`](supabase/migrations/004_customer_document.sql) (documento del cliente)
- [`supabase/migrations/005_inventory.sql`](supabase/migrations/005_inventory.sql) (stock y movimientos de productos)
- [`supabase/migrations/006_appointments.sql`](supabase/migrations/006_appointments.sql) (empleados y agenda de citas)
- [`supabase/migrations/007_center_modules.sql`](supabase/migrations/007_center_modules.sql) (módulos opcionales por centro)

4. (Recomendado para desarrollo) Authentication → Providers → Email → desactiva **Confirm email**.

5. Arranca la app:

```bash
npm run dev
```

## Flujo

1. Registra un centro (`/register`) — crea cuenta admin + fila en `centers`.
2. Entra al panel (`/admin`). Productos y ventas están siempre activos.
3. En `/admin/settings` activa los módulos que necesites (Agenda, Empleados, Servicios, Paquetes). Hoy son gratis.
4. Con Agenda activa, crea empleados y cita en `/admin/appointments` (al cobrar, abre Nueva venta precargada).
5. Gestiona el catálogo habilitado y registra ventas en `/admin/sales`.
