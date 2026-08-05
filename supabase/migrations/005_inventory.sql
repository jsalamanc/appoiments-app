-- Inventario de productos (stock + movimientos)
-- Ejecutar en: Supabase Dashboard → SQL Editor
-- Solo aplica a productos; servicios/paquetes no consumen stock.
-- La tabla de movimientos deja base para historial UI y alertas futuras.

alter table public.products
  add column if not exists stock integer not null default 0
  check (stock >= 0);

create table if not exists public.inventory_movements (
  id uuid primary key default gen_random_uuid(),
  center_id uuid not null references public.centers (id) on delete cascade,
  product_id uuid not null references public.products (id) on delete cascade,
  movement_type text not null check (movement_type in ('initial', 'adjustment', 'sale')),
  quantity_delta integer not null,
  stock_after integer not null check (stock_after >= 0),
  sale_id uuid references public.sales (id) on delete set null,
  note text,
  created_at timestamptz not null default now(),
  constraint inventory_movements_delta_check check (quantity_delta <> 0)
);

create index if not exists inventory_movements_product_id_idx
  on public.inventory_movements (product_id, created_at desc);
create index if not exists inventory_movements_center_id_idx
  on public.inventory_movements (center_id, created_at desc);
create index if not exists inventory_movements_sale_id_idx
  on public.inventory_movements (sale_id);
create index if not exists products_stock_idx
  on public.products (center_id, stock);

alter table public.inventory_movements enable row level security;

drop policy if exists "Owners manage own inventory movements" on public.inventory_movements;
create policy "Owners manage own inventory movements"
  on public.inventory_movements for all
  using (center_id = public.current_center_id())
  with check (center_id = public.current_center_id());

-- Ajuste atómico de stock (alta / edición manual)
create or replace function public.set_product_stock(
  p_product_id uuid,
  p_stock integer,
  p_note text default null
)
returns integer
language plpgsql
security definer
set search_path = public
as $$
declare
  v_center_id uuid := public.current_center_id();
  v_product public.products%rowtype;
  v_delta integer;
  v_has_movements boolean;
  v_type text;
begin
  if v_center_id is null then
    raise exception 'No se encontró el centro del usuario';
  end if;

  if p_stock is null or p_stock < 0 then
    raise exception 'El stock debe ser un entero mayor o igual a 0';
  end if;

  select * into v_product
  from public.products
  where id = p_product_id and center_id = v_center_id
  for update;

  if not found then
    raise exception 'Producto no encontrado';
  end if;

  v_delta := p_stock - v_product.stock;
  if v_delta = 0 then
    return v_product.stock;
  end if;

  select exists (
    select 1 from public.inventory_movements m where m.product_id = p_product_id
  ) into v_has_movements;

  v_type := case when not v_has_movements then 'initial' else 'adjustment' end;

  update public.products
  set stock = p_stock
  where id = p_product_id;

  insert into public.inventory_movements (
    center_id,
    product_id,
    movement_type,
    quantity_delta,
    stock_after,
    note
  ) values (
    v_center_id,
    p_product_id,
    v_type,
    v_delta,
    p_stock,
    nullif(trim(coalesce(p_note, '')), '')
  );

  return p_stock;
end;
$$;

revoke all on function public.set_product_stock(uuid, integer, text) from public;
grant execute on function public.set_product_stock(uuid, integer, text) to authenticated;

-- Venta atómica: crea venta + descuenta stock solo de ítems tipo product
create or replace function public.create_sale_with_inventory(
  p_customer_id uuid,
  p_payment_method text,
  p_items jsonb
)
returns uuid
language plpgsql
security definer
set search_path = public
as $$
declare
  v_center_id uuid := public.current_center_id();
  v_item jsonb;
  v_item_type text;
  v_item_id uuid;
  v_name text;
  v_unit_price numeric(12, 2);
  v_quantity integer;
  v_total numeric(12, 2) := 0;
  v_sale_id uuid;
  v_status text;
  v_product_name text;
  v_stock integer;
  r record;
begin
  if v_center_id is null then
    raise exception 'No se encontró el centro del usuario';
  end if;

  if p_payment_method is null or p_payment_method not in ('cash', 'transfer') then
    raise exception 'Método de pago inválido';
  end if;

  if p_items is null or jsonb_typeof(p_items) <> 'array' or jsonb_array_length(p_items) = 0 then
    raise exception 'Agrega al menos un ítem a la venta';
  end if;

  if not exists (
    select 1 from public.customers c
    where c.id = p_customer_id and c.center_id = v_center_id
  ) then
    raise exception 'Cliente no encontrado';
  end if;

  -- Validar ítems y calcular total
  for v_item in select * from jsonb_array_elements(p_items)
  loop
    v_item_type := v_item->>'item_type';
    v_item_id := (v_item->>'item_id')::uuid;
    v_name := coalesce(nullif(trim(v_item->>'name'), ''), 'Ítem');
    v_unit_price := (v_item->>'unit_price')::numeric;
    v_quantity := (v_item->>'quantity')::integer;

    if v_item_type is null or v_item_type not in ('product', 'service', 'package') then
      raise exception 'Tipo de ítem inválido';
    end if;

    if v_item_id is null then
      raise exception 'Ítem inválido';
    end if;

    if v_unit_price is null or v_unit_price < 0 then
      raise exception 'Precio inválido en “%”', v_name;
    end if;

    if v_quantity is null or v_quantity < 1 then
      raise exception 'Cantidad inválida en “%”', v_name;
    end if;

    if v_item_type = 'product' then
      select status into v_status
      from public.products
      where id = v_item_id and center_id = v_center_id;
      if v_status is distinct from 'published' then
        raise exception 'El producto “%” no está disponible', v_name;
      end if;
    elsif v_item_type = 'service' then
      select status into v_status
      from public.services
      where id = v_item_id and center_id = v_center_id;
      if v_status is distinct from 'published' then
        raise exception 'El servicio “%” no está disponible', v_name;
      end if;
    else
      select status into v_status
      from public.packages
      where id = v_item_id and center_id = v_center_id;
      if v_status is distinct from 'published' then
        raise exception 'El paquete “%” no está disponible', v_name;
      end if;
    end if;

    v_total := v_total + round(v_unit_price * v_quantity, 2);
  end loop;

  -- Reservar / validar stock solo para productos directos (agrupado)
  for r in
    select
      (item->>'item_id')::uuid as product_id,
      sum((item->>'quantity')::integer) as qty,
      max(coalesce(nullif(trim(item->>'name'), ''), 'Producto')) as name
    from jsonb_array_elements(p_items) as item
    where item->>'item_type' = 'product'
    group by (item->>'item_id')::uuid
  loop
    select name, stock into v_product_name, v_stock
    from public.products
    where id = r.product_id and center_id = v_center_id
    for update;

    if not found then
      raise exception 'Producto no encontrado';
    end if;

    if v_stock < r.qty then
      raise exception 'Stock insuficiente de “%” (disponible: %, pedido: %)',
        coalesce(v_product_name, r.name), v_stock, r.qty;
    end if;
  end loop;

  insert into public.sales (center_id, customer_id, payment_method, total)
  values (v_center_id, p_customer_id, p_payment_method, v_total)
  returning id into v_sale_id;

  insert into public.sale_items (
    sale_id,
    item_type,
    product_id,
    service_id,
    package_id,
    name_snapshot,
    unit_price,
    quantity,
    line_total
  )
  select
    v_sale_id,
    item->>'item_type',
    case when item->>'item_type' = 'product' then (item->>'item_id')::uuid else null end,
    case when item->>'item_type' = 'service' then (item->>'item_id')::uuid else null end,
    case when item->>'item_type' = 'package' then (item->>'item_id')::uuid else null end,
    coalesce(nullif(trim(item->>'name'), ''), 'Ítem'),
    (item->>'unit_price')::numeric,
    (item->>'quantity')::integer,
    round(((item->>'unit_price')::numeric) * ((item->>'quantity')::integer), 2)
  from jsonb_array_elements(p_items) as item;

  -- Descontar stock y registrar movimientos de venta
  for r in
    select
      (item->>'item_id')::uuid as product_id,
      sum((item->>'quantity')::integer) as qty
    from jsonb_array_elements(p_items) as item
    where item->>'item_type' = 'product'
    group by (item->>'item_id')::uuid
  loop
    update public.products
    set stock = stock - r.qty
    where id = r.product_id
    returning stock into v_stock;

    insert into public.inventory_movements (
      center_id,
      product_id,
      movement_type,
      quantity_delta,
      stock_after,
      sale_id,
      note
    ) values (
      v_center_id,
      r.product_id,
      'sale',
      -r.qty,
      v_stock,
      v_sale_id,
      'Venta'
    );
  end loop;

  return v_sale_id;
end;
$$;

revoke all on function public.create_sale_with_inventory(uuid, text, jsonb) from public;
grant execute on function public.create_sale_with_inventory(uuid, text, jsonb) to authenticated;
