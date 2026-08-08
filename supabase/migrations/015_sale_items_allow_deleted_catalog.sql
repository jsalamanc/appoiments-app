-- Permite borrar productos/servicios/paquetes sin romper historial.
--
-- Ventas: sale_items conservan name_snapshot; FKs quedan null (ON DELETE SET NULL).
-- Citas: appointments guardan service_name_snapshot; service_id puede quedar null.

-- 1) Ventas históricas (productos, servicios y paquetes)
alter table public.sale_items
  drop constraint if exists sale_items_ref_check;

alter table public.sale_items
  add constraint sale_items_ref_check check (
    (
      item_type = 'product'
      and service_id is null
      and package_id is null
    )
    or (
      item_type = 'service'
      and product_id is null
      and package_id is null
    )
    or (
      item_type = 'package'
      and product_id is null
      and service_id is null
    )
  );

-- 2) Citas: snapshot del servicio + permitir borrar el servicio
alter table public.appointments
  add column if not exists service_name_snapshot text;

update public.appointments a
set service_name_snapshot = coalesce(s.name, 'Servicio')
from public.services s
where a.service_id = s.id
  and (a.service_name_snapshot is null or a.service_name_snapshot = '');

update public.appointments
set service_name_snapshot = 'Servicio'
where service_name_snapshot is null or service_name_snapshot = '';

alter table public.appointments
  alter column service_name_snapshot set not null;

alter table public.appointments
  alter column service_id drop not null;

alter table public.appointments
  drop constraint if exists appointments_service_id_fkey;

alter table public.appointments
  add constraint appointments_service_id_fkey
  foreign key (service_id) references public.services (id) on delete set null;

-- 3) RPCs: siempre guardan el nombre del servicio al crear/editar
create or replace function public.create_appointment(
  p_employee_id uuid,
  p_customer_id uuid,
  p_service_id uuid,
  p_starts_at timestamptz,
  p_notes text default null
)
returns uuid
language plpgsql
security definer
set search_path = public
as $$
declare
  v_center_id uuid := public.current_center_id();
  v_duration integer;
  v_service_name text;
  v_ends_at timestamptz;
  v_id uuid;
begin
  if v_center_id is null then
    raise exception 'No se encontró el centro del usuario';
  end if;

  if p_starts_at is null then
    raise exception 'Fecha de inicio requerida';
  end if;

  if not exists (
    select 1 from public.employees e
    where e.id = p_employee_id and e.center_id = v_center_id and e.active
  ) then
    raise exception 'Empleado no disponible';
  end if;

  if not exists (
    select 1 from public.customers c
    where c.id = p_customer_id and c.center_id = v_center_id
  ) then
    raise exception 'Cliente no encontrado';
  end if;

  select duration_minutes, name into v_duration, v_service_name
  from public.services
  where id = p_service_id and center_id = v_center_id and status = 'published';

  if v_duration is null then
    raise exception 'Servicio no disponible';
  end if;

  v_ends_at := p_starts_at + make_interval(mins => v_duration);

  if public.appointment_has_overlap(v_center_id, p_employee_id, p_starts_at, v_ends_at, null) then
    raise exception 'El empleado ya tiene una cita en ese horario';
  end if;

  insert into public.appointments (
    center_id, employee_id, customer_id, service_id, service_name_snapshot,
    starts_at, ends_at, notes, status
  ) values (
    v_center_id, p_employee_id, p_customer_id, p_service_id, v_service_name,
    p_starts_at, v_ends_at, nullif(trim(coalesce(p_notes, '')), ''), 'scheduled'
  )
  returning id into v_id;

  return v_id;
end;
$$;

create or replace function public.update_appointment(
  p_appointment_id uuid,
  p_employee_id uuid,
  p_customer_id uuid,
  p_service_id uuid,
  p_starts_at timestamptz,
  p_notes text default null
)
returns uuid
language plpgsql
security definer
set search_path = public
as $$
declare
  v_center_id uuid := public.current_center_id();
  v_duration integer;
  v_service_name text;
  v_ends_at timestamptz;
  v_status text;
begin
  if v_center_id is null then
    raise exception 'No se encontró el centro del usuario';
  end if;

  select status into v_status
  from public.appointments
  where id = p_appointment_id and center_id = v_center_id
  for update;

  if not found then
    raise exception 'Cita no encontrada';
  end if;

  if v_status <> 'scheduled' then
    raise exception 'Solo se pueden editar citas programadas';
  end if;

  if not exists (
    select 1 from public.employees e
    where e.id = p_employee_id and e.center_id = v_center_id and e.active
  ) then
    raise exception 'Empleado no disponible';
  end if;

  if not exists (
    select 1 from public.customers c
    where c.id = p_customer_id and c.center_id = v_center_id
  ) then
    raise exception 'Cliente no encontrado';
  end if;

  select duration_minutes, name into v_duration, v_service_name
  from public.services
  where id = p_service_id and center_id = v_center_id and status = 'published';

  if v_duration is null then
    raise exception 'Servicio no disponible';
  end if;

  v_ends_at := p_starts_at + make_interval(mins => v_duration);

  if public.appointment_has_overlap(
    v_center_id, p_employee_id, p_starts_at, v_ends_at, p_appointment_id
  ) then
    raise exception 'El empleado ya tiene una cita en ese horario';
  end if;

  update public.appointments
  set
    employee_id = p_employee_id,
    customer_id = p_customer_id,
    service_id = p_service_id,
    service_name_snapshot = v_service_name,
    starts_at = p_starts_at,
    ends_at = v_ends_at,
    notes = nullif(trim(coalesce(p_notes, '')), '')
  where id = p_appointment_id;

  return p_appointment_id;
end;
$$;

create or replace function public.create_public_booking(
  p_slug text,
  p_employee_id uuid,
  p_service_id uuid,
  p_starts_at timestamptz,
  p_customer_name text,
  p_customer_email text,
  p_customer_phone text,
  p_customer_document text,
  p_notes text default null
)
returns uuid
language plpgsql
security definer
set search_path = public
as $$
declare
  v_center public.centers%rowtype;
  v_modules jsonb;
  v_duration integer;
  v_service_name text;
  v_ends_at timestamptz;
  v_customer_id uuid;
  v_email text;
  v_document text;
  v_id uuid;
begin
  select * into v_center
  from public.centers
  where slug = nullif(trim(p_slug), '')
  limit 1;

  if not found then
    raise exception 'Centro no encontrado';
  end if;

  v_modules := coalesce(v_center.modules, '{}'::jsonb);

  if coalesce((v_modules->>'public_booking')::boolean, false) is not true then
    raise exception 'El portal de reservas no está activo';
  end if;

  if coalesce((v_modules->>'appointments')::boolean, false) is not true
     or coalesce((v_modules->>'services')::boolean, false) is not true then
    raise exception 'Las reservas públicas no están disponibles';
  end if;

  if p_starts_at is null or p_starts_at <= now() then
    raise exception 'Elige un horario futuro';
  end if;

  if length(trim(coalesce(p_customer_name, ''))) < 2 then
    raise exception 'Nombre requerido';
  end if;

  v_email := lower(trim(coalesce(p_customer_email, '')));
  if position('@' in v_email) < 2 then
    raise exception 'Correo inválido';
  end if;

  if length(trim(coalesce(p_customer_phone, ''))) < 7 then
    raise exception 'Teléfono requerido';
  end if;

  v_document := trim(coalesce(p_customer_document, ''));
  if length(v_document) < 5 then
    raise exception 'Documento requerido';
  end if;

  if not exists (
    select 1 from public.employees e
    where e.id = p_employee_id and e.center_id = v_center.id and e.active
  ) then
    raise exception 'Profesional no disponible';
  end if;

  select duration_minutes, name into v_duration, v_service_name
  from public.services
  where id = p_service_id and center_id = v_center.id and status = 'published';

  if v_duration is null then
    raise exception 'Servicio no disponible';
  end if;

  v_ends_at := p_starts_at + make_interval(mins => v_duration);

  if public.appointment_has_overlap(
    v_center.id, p_employee_id, p_starts_at, v_ends_at, null
  ) then
    raise exception 'Ese horario ya no está disponible';
  end if;

  select id into v_customer_id
  from public.customers
  where center_id = v_center.id and lower(document) = lower(v_document)
  limit 1;

  if v_customer_id is null then
    select id into v_customer_id
    from public.customers
    where center_id = v_center.id and lower(email) = v_email
    limit 1;
  end if;

  if v_customer_id is null then
    insert into public.customers (
      center_id, name, email, phone, document
    ) values (
      v_center.id,
      trim(p_customer_name),
      v_email,
      trim(p_customer_phone),
      v_document
    )
    returning id into v_customer_id;
  else
    update public.customers
    set
      name = trim(p_customer_name),
      email = v_email,
      phone = trim(p_customer_phone),
      document = v_document
    where id = v_customer_id;
  end if;

  insert into public.appointments (
    center_id, employee_id, customer_id, service_id, service_name_snapshot,
    starts_at, ends_at, notes, status
  ) values (
    v_center.id, p_employee_id, v_customer_id, p_service_id, v_service_name,
    p_starts_at, v_ends_at,
    nullif(trim(coalesce(p_notes, '')), ''),
    'scheduled'
  )
  returning id into v_id;

  return v_id;
end;
$$;
