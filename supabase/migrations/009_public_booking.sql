-- Portal público de reserva por slug del centro
-- Ejecutar en: Supabase Dashboard → SQL Editor (después de 008)

create or replace function public.get_public_booking_catalog(p_slug text)
returns jsonb
language plpgsql
security definer
set search_path = public
as $$
declare
  v_center public.centers%rowtype;
  v_modules jsonb;
begin
  select * into v_center
  from public.centers
  where slug = nullif(trim(p_slug), '')
  limit 1;

  if not found then
    return null;
  end if;

  v_modules := coalesce(v_center.modules, '{}'::jsonb);

  if coalesce((v_modules->>'appointments')::boolean, false) is not true
     or coalesce((v_modules->>'services')::boolean, false) is not true then
    return jsonb_build_object(
      'available', false,
      'center', jsonb_build_object(
        'name', v_center.name,
        'slug', v_center.slug
      ),
      'reason', 'El centro no tiene agenda pública activa'
    );
  end if;

  return jsonb_build_object(
    'available', true,
    'center', jsonb_build_object(
      'id', v_center.id,
      'name', v_center.name,
      'slug', v_center.slug
    ),
    'services', coalesce((
      select jsonb_agg(
        jsonb_build_object(
          'id', s.id,
          'name', s.name,
          'description', s.description,
          'price', s.price,
          'duration_minutes', s.duration_minutes,
          'photo_url', s.photo_url
        )
        order by s.name
      )
      from public.services s
      where s.center_id = v_center.id and s.status = 'published'
    ), '[]'::jsonb),
    'employees', coalesce((
      select jsonb_agg(
        jsonb_build_object(
          'id', e.id,
          'name', e.name,
          'color', e.color
        )
        order by e.name
      )
      from public.employees e
      where e.center_id = v_center.id and e.active
    ), '[]'::jsonb)
  );
end;
$$;

create or replace function public.get_public_busy_ranges(
  p_slug text,
  p_employee_id uuid,
  p_from timestamptz,
  p_to timestamptz
)
returns jsonb
language plpgsql
security definer
set search_path = public
as $$
declare
  v_center_id uuid;
begin
  select c.id into v_center_id
  from public.centers c
  where c.slug = nullif(trim(p_slug), '')
  limit 1;

  if v_center_id is null then
    return '[]'::jsonb;
  end if;

  if not exists (
    select 1 from public.employees e
    where e.id = p_employee_id and e.center_id = v_center_id and e.active
  ) then
    return '[]'::jsonb;
  end if;

  return coalesce((
    select jsonb_agg(
      jsonb_build_object(
        'starts_at', a.starts_at,
        'ends_at', a.ends_at
      )
      order by a.starts_at
    )
    from public.appointments a
    where a.center_id = v_center_id
      and a.employee_id = p_employee_id
      and a.status = 'scheduled'
      and a.starts_at < p_to
      and a.ends_at > p_from
  ), '[]'::jsonb);
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

  select duration_minutes into v_duration
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
    center_id, employee_id, customer_id, service_id,
    starts_at, ends_at, notes, status
  ) values (
    v_center.id, p_employee_id, v_customer_id, p_service_id,
    p_starts_at, v_ends_at,
    nullif(trim(coalesce(p_notes, '')), ''),
    'scheduled'
  )
  returning id into v_id;

  return v_id;
end;
$$;

revoke all on function public.get_public_booking_catalog(text) from public;
revoke all on function public.get_public_busy_ranges(text, uuid, timestamptz, timestamptz) from public;
revoke all on function public.create_public_booking(text, uuid, uuid, timestamptz, text, text, text, text, text) from public;

grant execute on function public.get_public_booking_catalog(text) to anon, authenticated;
grant execute on function public.get_public_busy_ranges(text, uuid, timestamptz, timestamptz) to anon, authenticated;
grant execute on function public.create_public_booking(text, uuid, uuid, timestamptz, text, text, text, text, text) to anon, authenticated;
