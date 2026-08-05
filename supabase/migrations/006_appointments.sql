-- Agenda de citas (empleados + appointments)
-- Ejecutar en: Supabase Dashboard → SQL Editor
-- Solo agenda admin; varios calendarios por empleado.

create table if not exists public.employees (
  id uuid primary key default gen_random_uuid(),
  center_id uuid not null references public.centers (id) on delete cascade,
  name text not null,
  color text not null default '#b89bb8'
    check (color ~ '^#[0-9A-Fa-f]{6}$'),
  active boolean not null default true,
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now()
);

create index if not exists employees_center_id_idx on public.employees (center_id);
create index if not exists employees_center_active_idx on public.employees (center_id, active);

create trigger employees_updated_at before update on public.employees
  for each row execute function public.set_updated_at();

alter table public.employees enable row level security;

drop policy if exists "Owners manage own employees" on public.employees;
create policy "Owners manage own employees"
  on public.employees for all
  using (center_id = public.current_center_id())
  with check (center_id = public.current_center_id());

-- Appointments
create table if not exists public.appointments (
  id uuid primary key default gen_random_uuid(),
  center_id uuid not null references public.centers (id) on delete cascade,
  employee_id uuid not null references public.employees (id) on delete restrict,
  customer_id uuid not null references public.customers (id) on delete restrict,
  service_id uuid not null references public.services (id) on delete restrict,
  starts_at timestamptz not null,
  ends_at timestamptz not null,
  status text not null default 'scheduled'
    check (status in ('scheduled', 'completed', 'cancelled', 'no_show')),
  notes text,
  sale_id uuid references public.sales (id) on delete set null,
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now(),
  constraint appointments_range_check check (ends_at > starts_at)
);

create index if not exists appointments_center_starts_idx
  on public.appointments (center_id, starts_at);
create index if not exists appointments_employee_starts_idx
  on public.appointments (employee_id, starts_at);
create index if not exists appointments_customer_id_idx
  on public.appointments (customer_id);
create index if not exists appointments_status_idx
  on public.appointments (center_id, status);

create trigger appointments_updated_at before update on public.appointments
  for each row execute function public.set_updated_at();

alter table public.appointments enable row level security;

drop policy if exists "Owners manage own appointments" on public.appointments;
create policy "Owners manage own appointments"
  on public.appointments for all
  using (center_id = public.current_center_id())
  with check (center_id = public.current_center_id());

-- Link sales → appointment (walk-ins leave null)
alter table public.sales
  add column if not exists appointment_id uuid references public.appointments (id) on delete set null;

create index if not exists sales_appointment_id_idx on public.sales (appointment_id);

-- Overlap helper
create or replace function public.appointment_has_overlap(
  p_center_id uuid,
  p_employee_id uuid,
  p_starts_at timestamptz,
  p_ends_at timestamptz,
  p_exclude_id uuid default null
)
returns boolean
language sql
stable
security definer
set search_path = public
as $$
  select exists (
    select 1
    from public.appointments a
    where a.center_id = p_center_id
      and a.employee_id = p_employee_id
      and a.status = 'scheduled'
      and (p_exclude_id is null or a.id <> p_exclude_id)
      and a.starts_at < p_ends_at
      and a.ends_at > p_starts_at
  );
$$;

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

  select duration_minutes into v_duration
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
    center_id, employee_id, customer_id, service_id,
    starts_at, ends_at, notes, status
  ) values (
    v_center_id, p_employee_id, p_customer_id, p_service_id,
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

  select duration_minutes into v_duration
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
    starts_at = p_starts_at,
    ends_at = v_ends_at,
    notes = nullif(trim(coalesce(p_notes, '')), '')
  where id = p_appointment_id;

  return p_appointment_id;
end;
$$;

revoke all on function public.appointment_has_overlap(uuid, uuid, timestamptz, timestamptz, uuid) from public;
revoke all on function public.create_appointment(uuid, uuid, uuid, timestamptz, text) from public;
revoke all on function public.update_appointment(uuid, uuid, uuid, uuid, timestamptz, text) from public;

grant execute on function public.appointment_has_overlap(uuid, uuid, timestamptz, timestamptz, uuid) to authenticated;
grant execute on function public.create_appointment(uuid, uuid, uuid, timestamptz, text) to authenticated;
grant execute on function public.update_appointment(uuid, uuid, uuid, uuid, timestamptz, text) to authenticated;
