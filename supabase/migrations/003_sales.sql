-- Módulo de ventas (POS admin)
-- Ejecutar en: Supabase Dashboard → SQL Editor

-- Customers (únicos por email dentro del centro)
create table public.customers (
  id uuid primary key default gen_random_uuid(),
  center_id uuid not null references public.centers (id) on delete cascade,
  name text not null,
  phone text not null,
  email text not null,
  document text not null,
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now(),
  constraint customers_email_format check (position('@' in email) > 1)
);

create unique index customers_center_email_unique
  on public.customers (center_id, lower(email));

create unique index customers_center_document_unique
  on public.customers (center_id, lower(document));

create index customers_center_id_idx on public.customers (center_id);

-- Sales
create table public.sales (
  id uuid primary key default gen_random_uuid(),
  center_id uuid not null references public.centers (id) on delete cascade,
  customer_id uuid not null references public.customers (id) on delete restrict,
  payment_method text not null check (payment_method in ('cash', 'transfer')),
  total numeric(12, 2) not null check (total >= 0),
  sold_at timestamptz not null default now(),
  created_at timestamptz not null default now()
);

create index sales_center_id_idx on public.sales (center_id);
create index sales_sold_at_idx on public.sales (center_id, sold_at desc);
create index sales_customer_id_idx on public.sales (customer_id);
create index sales_payment_method_idx on public.sales (center_id, payment_method);

-- Sale items (snapshot de nombre/precio)
create table public.sale_items (
  id uuid primary key default gen_random_uuid(),
  sale_id uuid not null references public.sales (id) on delete cascade,
  item_type text not null check (item_type in ('product', 'service', 'package')),
  product_id uuid references public.products (id) on delete set null,
  service_id uuid references public.services (id) on delete set null,
  package_id uuid references public.packages (id) on delete set null,
  name_snapshot text not null,
  unit_price numeric(12, 2) not null check (unit_price >= 0),
  quantity integer not null check (quantity >= 1),
  line_total numeric(12, 2) not null check (line_total >= 0),
  created_at timestamptz not null default now(),
  constraint sale_items_ref_check check (
    (item_type = 'product' and product_id is not null and service_id is null and package_id is null)
    or (item_type = 'service' and service_id is not null and product_id is null and package_id is null)
    or (item_type = 'package' and package_id is not null and product_id is null and service_id is null)
  )
);

create index sale_items_sale_id_idx on public.sale_items (sale_id);

create trigger customers_updated_at before update on public.customers
  for each row execute function public.set_updated_at();

-- RLS
alter table public.customers enable row level security;
alter table public.sales enable row level security;
alter table public.sale_items enable row level security;

create policy "Owners manage own customers"
  on public.customers for all
  using (center_id = public.current_center_id())
  with check (center_id = public.current_center_id());

create policy "Owners manage own sales"
  on public.sales for all
  using (center_id = public.current_center_id())
  with check (center_id = public.current_center_id());

create policy "Owners manage own sale items"
  on public.sale_items for all
  using (
    exists (
      select 1 from public.sales s
      where s.id = sale_id and s.center_id = public.current_center_id()
    )
  )
  with check (
    exists (
      select 1 from public.sales s
      where s.id = sale_id and s.center_id = public.current_center_id()
    )
  );
