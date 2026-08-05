-- Límite de productos por plan (Free: 10; Starter/Growth: sin límite)
-- Ejecutar en: Supabase Dashboard → SQL Editor (después de 013)

alter table public.plans
  add column if not exists max_products integer
  check (max_products is null or max_products >= 0);

update public.plans
set
  max_products = 10,
  benefits = '[
    "Hasta 10 productos",
    "Ventas y caja del día",
    "Clientes e historial"
  ]'::jsonb
where id = 'free';

update public.plans
set max_products = null
where id in ('starter', 'growth');

create or replace function public.enforce_product_plan_limit()
returns trigger
language plpgsql
security definer
set search_path = public
as $$
declare
  v_plan_id text;
  v_max integer;
  v_count integer;
begin
  select c.plan_id, p.max_products
  into v_plan_id, v_max
  from public.centers c
  join public.plans p on p.id = c.plan_id
  where c.id = new.center_id;

  if v_max is null then
    return new;
  end if;

  select count(*)::integer into v_count
  from public.products
  where center_id = new.center_id;

  if v_count >= v_max then
    raise exception 'Tu plan Free permite máximo % productos. Cambia a Starter o Growth para agregar más.', v_max;
  end if;

  return new;
end;
$$;

drop trigger if exists products_enforce_plan_limit on public.products;
create trigger products_enforce_plan_limit
  before insert on public.products
  for each row execute function public.enforce_product_plan_limit();
