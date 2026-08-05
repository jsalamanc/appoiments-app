-- Catálogo de planes Free / Starter / Growth
-- Ejecutar en: Supabase Dashboard → SQL Editor (después de 011)

create table if not exists public.plans (
  id text primary key,
  name text not null,
  price_cents integer not null default 0 check (price_cents >= 0),
  currency text not null default 'COP',
  benefits jsonb not null default '[]'::jsonb,
  sort_order integer not null default 0,
  active boolean not null default true,
  created_at timestamptz not null default now()
);

insert into public.plans (id, name, price_cents, currency, benefits, sort_order, active)
values
  (
    'free',
    'Free',
    0,
    'COP',
    '[
      "Productos e inventario",
      "Ventas y caja del día",
      "Clientes e historial"
    ]'::jsonb,
    1,
    true
  ),
  (
    'starter',
    'Starter',
    4900000,
    'COP',
    '[
      "Todo lo de Free",
      "Agenda de citas",
      "Empleados",
      "Servicios y paquetes"
    ]'::jsonb,
    2,
    true
  ),
  (
    'growth',
    'Growth',
    9900000,
    'COP',
    '[
      "Todo lo de Starter",
      "Portal público de reservas",
      "Reportes y exportes",
      "Equipo de recepción"
    ]'::jsonb,
    3,
    true
  )
on conflict (id) do update set
  name = excluded.name,
  price_cents = excluded.price_cents,
  currency = excluded.currency,
  benefits = excluded.benefits,
  sort_order = excluded.sort_order,
  active = excluded.active;

-- Limpiar valores inválidos antes de FK
update public.centers
set plan_id = 'free'
where plan_id is null
   or plan_id not in (select id from public.plans);

alter table public.centers
  alter column plan_id set default 'free';

do $$
begin
  if not exists (
    select 1
    from pg_constraint
    where conname = 'centers_plan_id_fkey'
  ) then
    alter table public.centers
      add constraint centers_plan_id_fkey
      foreign key (plan_id) references public.plans (id);
  end if;
end $$;

alter table public.centers
  alter column plan_id set not null;

alter table public.plans enable row level security;

drop policy if exists "Authenticated can read plans" on public.plans;
create policy "Authenticated can read plans"
  on public.plans for select
  to authenticated
  using (active = true);
