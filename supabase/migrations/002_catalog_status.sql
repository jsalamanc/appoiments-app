-- Estado borrador / publicado para catálogo
-- Ejecutar en: Supabase Dashboard → SQL Editor
-- (si ya corriste 001_init.sql)

alter table public.products
  add column if not exists status text not null default 'draft'
  check (status in ('draft', 'published'));

alter table public.services
  add column if not exists status text not null default 'draft'
  check (status in ('draft', 'published'));

alter table public.packages
  add column if not exists status text not null default 'draft'
  check (status in ('draft', 'published'));

create index if not exists products_status_idx on public.products (center_id, status);
create index if not exists services_status_idx on public.services (center_id, status);
create index if not exists packages_status_idx on public.packages (center_id, status);
