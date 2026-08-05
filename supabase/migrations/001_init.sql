-- MVP Admin Centros Estéticos
-- Ejecutar en: Supabase Dashboard → SQL Editor

-- Extensions
create extension if not exists "pgcrypto";

-- Centers (un centro por admin)
create table public.centers (
  id uuid primary key default gen_random_uuid(),
  owner_id uuid not null unique references auth.users (id) on delete cascade,
  name text not null,
  slug text not null unique,
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now()
);

-- Products
create table public.products (
  id uuid primary key default gen_random_uuid(),
  center_id uuid not null references public.centers (id) on delete cascade,
  name text not null,
  price numeric(12, 2) not null check (price >= 0),
  photo_url text,
  status text not null default 'draft' check (status in ('draft', 'published')),
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now()
);

-- Services
create table public.services (
  id uuid primary key default gen_random_uuid(),
  center_id uuid not null references public.centers (id) on delete cascade,
  name text not null,
  description text,
  price numeric(12, 2) not null check (price >= 0),
  duration_minutes integer not null check (duration_minutes > 0),
  photo_url text,
  status text not null default 'draft' check (status in ('draft', 'published')),
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now()
);

-- Packages
create table public.packages (
  id uuid primary key default gen_random_uuid(),
  center_id uuid not null references public.centers (id) on delete cascade,
  name text not null,
  description text,
  price numeric(12, 2) not null check (price >= 0),
  photo_url text,
  status text not null default 'draft' check (status in ('draft', 'published')),
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now()
);

-- Package items (productos/servicios con cantidad)
create table public.package_items (
  id uuid primary key default gen_random_uuid(),
  package_id uuid not null references public.packages (id) on delete cascade,
  item_type text not null check (item_type in ('product', 'service')),
  product_id uuid references public.products (id) on delete cascade,
  service_id uuid references public.services (id) on delete cascade,
  quantity integer not null default 1 check (quantity >= 1),
  created_at timestamptz not null default now(),
  constraint package_items_ref_check check (
    (item_type = 'product' and product_id is not null and service_id is null)
    or (item_type = 'service' and service_id is not null and product_id is null)
  )
);

create index products_center_id_idx on public.products (center_id);
create index services_center_id_idx on public.services (center_id);
create index packages_center_id_idx on public.packages (center_id);
create index package_items_package_id_idx on public.package_items (package_id);
create index products_status_idx on public.products (center_id, status);
create index services_status_idx on public.services (center_id, status);
create index packages_status_idx on public.packages (center_id, status);

-- Helper: center del usuario autenticado
create or replace function public.current_center_id()
returns uuid
language sql
stable
security definer
set search_path = public
as $$
  select id from public.centers where owner_id = auth.uid() limit 1;
$$;

-- Slug helper
create or replace function public.slugify(value text)
returns text
language plpgsql
immutable
as $$
declare
  result text;
begin
  result := lower(coalesce(value, ''));
  result := regexp_replace(result, '[áàäâ]', 'a', 'g');
  result := regexp_replace(result, '[éèëê]', 'e', 'g');
  result := regexp_replace(result, '[íìïî]', 'i', 'g');
  result := regexp_replace(result, '[óòöô]', 'o', 'g');
  result := regexp_replace(result, '[úùüû]', 'u', 'g');
  result := regexp_replace(result, 'ñ', 'n', 'g');
  result := regexp_replace(result, '[^a-z0-9]+', '-', 'g');
  result := trim(both '-' from result);
  if result = '' then
    result := 'centro';
  end if;
  return result;
end;
$$;

-- Auto-create center on signup
create or replace function public.handle_new_user()
returns trigger
language plpgsql
security definer
set search_path = public
as $$
declare
  center_name text;
  base_slug text;
  final_slug text;
begin
  center_name := coalesce(nullif(trim(new.raw_user_meta_data->>'center_name'), ''), 'Mi Centro');
  base_slug := public.slugify(center_name);
  final_slug := base_slug || '-' || substr(replace(new.id::text, '-', ''), 1, 8);

  insert into public.centers (owner_id, name, slug)
  values (new.id, center_name, final_slug);

  return new;
end;
$$;

drop trigger if exists on_auth_user_created on auth.users;
create trigger on_auth_user_created
  after insert on auth.users
  for each row execute function public.handle_new_user();

-- updated_at trigger
create or replace function public.set_updated_at()
returns trigger
language plpgsql
as $$
begin
  new.updated_at = now();
  return new;
end;
$$;

create trigger centers_updated_at before update on public.centers
  for each row execute function public.set_updated_at();
create trigger products_updated_at before update on public.products
  for each row execute function public.set_updated_at();
create trigger services_updated_at before update on public.services
  for each row execute function public.set_updated_at();
create trigger packages_updated_at before update on public.packages
  for each row execute function public.set_updated_at();

-- RLS
alter table public.centers enable row level security;
alter table public.products enable row level security;
alter table public.services enable row level security;
alter table public.packages enable row level security;
alter table public.package_items enable row level security;

create policy "Owners can view own center"
  on public.centers for select
  using (owner_id = auth.uid());

create policy "Owners can update own center"
  on public.centers for update
  using (owner_id = auth.uid())
  with check (owner_id = auth.uid());

create policy "Owners manage own products"
  on public.products for all
  using (center_id = public.current_center_id())
  with check (center_id = public.current_center_id());

create policy "Owners manage own services"
  on public.services for all
  using (center_id = public.current_center_id())
  with check (center_id = public.current_center_id());

create policy "Owners manage own packages"
  on public.packages for all
  using (center_id = public.current_center_id())
  with check (center_id = public.current_center_id());

create policy "Owners manage own package items"
  on public.package_items for all
  using (
    exists (
      select 1 from public.packages p
      where p.id = package_id and p.center_id = public.current_center_id()
    )
  )
  with check (
    exists (
      select 1 from public.packages p
      where p.id = package_id and p.center_id = public.current_center_id()
    )
  );

-- Storage bucket for catalog images
insert into storage.buckets (id, name, public)
values ('catalog', 'catalog', true)
on conflict (id) do nothing;

create policy "Authenticated users can upload catalog images"
  on storage.objects for insert
  to authenticated
  with check (
    bucket_id = 'catalog'
    and (storage.foldername(name))[1] = public.current_center_id()::text
  );

create policy "Authenticated users can update own catalog images"
  on storage.objects for update
  to authenticated
  using (
    bucket_id = 'catalog'
    and (storage.foldername(name))[1] = public.current_center_id()::text
  );

create policy "Authenticated users can delete own catalog images"
  on storage.objects for delete
  to authenticated
  using (
    bucket_id = 'catalog'
    and (storage.foldername(name))[1] = public.current_center_id()::text
  );

create policy "Public can read catalog images"
  on storage.objects for select
  using (bucket_id = 'catalog');
