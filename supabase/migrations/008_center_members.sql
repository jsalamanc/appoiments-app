-- Roles: dueño (owner) y recepción (reception)
-- Profesional y multi-sucursal quedan fuera de esta migración.
-- Ejecutar en: Supabase Dashboard → SQL Editor

-- Members
create table if not exists public.center_members (
  id uuid primary key default gen_random_uuid(),
  center_id uuid not null references public.centers (id) on delete cascade,
  user_id uuid not null references auth.users (id) on delete cascade,
  role text not null check (role in ('owner', 'reception')),
  created_at timestamptz not null default now(),
  unique (center_id, user_id),
  unique (user_id)
);

create index if not exists center_members_center_id_idx
  on public.center_members (center_id);
create index if not exists center_members_user_id_idx
  on public.center_members (user_id);

-- Invites (solo recepción por ahora)
create table if not exists public.center_invites (
  id uuid primary key default gen_random_uuid(),
  center_id uuid not null references public.centers (id) on delete cascade,
  email text,
  token text not null unique,
  role text not null default 'reception'
    check (role = 'reception'),
  created_by uuid not null references auth.users (id) on delete cascade,
  expires_at timestamptz not null default (now() + interval '7 days'),
  accepted_at timestamptz,
  accepted_by uuid references auth.users (id) on delete set null,
  created_at timestamptz not null default now()
);

create index if not exists center_invites_center_id_idx
  on public.center_invites (center_id);
create index if not exists center_invites_token_idx
  on public.center_invites (token);

-- Backfill owners as members
insert into public.center_members (center_id, user_id, role)
select c.id, c.owner_id, 'owner'
from public.centers c
on conflict (user_id) do nothing;

-- Resolve center via membership (1 membership per user for now)
create or replace function public.current_center_id()
returns uuid
language sql
stable
security definer
set search_path = public
as $$
  select center_id
  from public.center_members
  where user_id = auth.uid()
  limit 1;
$$;

create or replace function public.current_member_role()
returns text
language sql
stable
security definer
set search_path = public
as $$
  select role
  from public.center_members
  where user_id = auth.uid()
  limit 1;
$$;

create or replace function public.is_center_owner()
returns boolean
language sql
stable
security definer
set search_path = public
as $$
  select exists (
    select 1
    from public.center_members
    where user_id = auth.uid() and role = 'owner'
  );
$$;

-- RLS members / invites
alter table public.center_members enable row level security;
alter table public.center_invites enable row level security;

drop policy if exists "Members can view own membership" on public.center_members;
create policy "Members can view own membership"
  on public.center_members for select
  using (
    user_id = auth.uid()
    or (
      center_id = public.current_center_id()
      and public.is_center_owner()
    )
  );

drop policy if exists "Owners manage members" on public.center_members;
create policy "Owners manage members"
  on public.center_members for all
  using (
    center_id = public.current_center_id()
    and public.is_center_owner()
  )
  with check (
    center_id = public.current_center_id()
    and public.is_center_owner()
  );

drop policy if exists "Owners manage invites" on public.center_invites;
create policy "Owners manage invites"
  on public.center_invites for all
  using (
    center_id = public.current_center_id()
    and public.is_center_owner()
  )
  with check (
    center_id = public.current_center_id()
    and public.is_center_owner()
  );

-- Centers: members can view; only owner can update
drop policy if exists "Owners can view own center" on public.centers;
drop policy if exists "Owners can update own center" on public.centers;
drop policy if exists "Members can view their center" on public.centers;
drop policy if exists "Owners can update their center" on public.centers;

create policy "Members can view their center"
  on public.centers for select
  using (
    exists (
      select 1 from public.center_members m
      where m.center_id = centers.id and m.user_id = auth.uid()
    )
  );

create policy "Owners can update their center"
  on public.centers for update
  using (
    exists (
      select 1 from public.center_members m
      where m.center_id = centers.id
        and m.user_id = auth.uid()
        and m.role = 'owner'
    )
  )
  with check (
    exists (
      select 1 from public.center_members m
      where m.center_id = centers.id
        and m.user_id = auth.uid()
        and m.role = 'owner'
    )
  );

-- Signup: owner (new center) OR reception (invite token)
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
  invite_token text;
  v_invite public.center_invites%rowtype;
  v_center_id uuid;
begin
  invite_token := nullif(trim(coalesce(new.raw_user_meta_data->>'invite_token', '')), '');

  if invite_token is not null then
    select * into v_invite
    from public.center_invites
    where token = invite_token
      and accepted_at is null
      and expires_at > now()
    for update;

    if not found then
      raise exception 'Invitación inválida o expirada';
    end if;

    if v_invite.email is not null
       and lower(trim(v_invite.email)) <> lower(trim(new.email)) then
      raise exception 'Esta invitación es para otro correo';
    end if;

    insert into public.center_members (center_id, user_id, role)
    values (v_invite.center_id, new.id, v_invite.role);

    update public.center_invites
    set accepted_at = now(), accepted_by = new.id
    where id = v_invite.id;

    return new;
  end if;

  center_name := coalesce(nullif(trim(new.raw_user_meta_data->>'center_name'), ''), 'Mi Centro');
  base_slug := public.slugify(center_name);
  final_slug := base_slug || '-' || substr(replace(new.id::text, '-', ''), 1, 8);

  insert into public.centers (owner_id, name, slug)
  values (new.id, center_name, final_slug)
  returning id into v_center_id;

  insert into public.center_members (center_id, user_id, role)
  values (v_center_id, new.id, 'owner');

  return new;
end;
$$;

-- Owner helpers
create or replace function public.create_center_invite(p_email text default null)
returns table (id uuid, token text, expires_at timestamptz)
language plpgsql
security definer
set search_path = public
as $$
declare
  v_center_id uuid := public.current_center_id();
  v_token text;
begin
  if v_center_id is null or not public.is_center_owner() then
    raise exception 'Solo el dueño puede invitar';
  end if;

  v_token := encode(gen_random_bytes(24), 'hex');

  return query
  insert into public.center_invites (center_id, email, token, role, created_by)
  values (
    v_center_id,
    nullif(trim(lower(coalesce(p_email, ''))), ''),
    v_token,
    'reception',
    auth.uid()
  )
  returning center_invites.id, center_invites.token, center_invites.expires_at;
end;
$$;

create or replace function public.revoke_center_member(p_member_id uuid)
returns void
language plpgsql
security definer
set search_path = public
as $$
declare
  v_center_id uuid := public.current_center_id();
  v_role text;
begin
  if v_center_id is null or not public.is_center_owner() then
    raise exception 'Solo el dueño puede quitar miembros';
  end if;

  select role into v_role
  from public.center_members
  where id = p_member_id and center_id = v_center_id;

  if v_role is null then
    raise exception 'Miembro no encontrado';
  end if;

  if v_role = 'owner' then
    raise exception 'No puedes quitar al dueño';
  end if;

  delete from public.center_members
  where id = p_member_id and center_id = v_center_id;
end;
$$;

create or replace function public.get_invite_preview(p_token text)
returns table (
  center_name text,
  email text,
  role text,
  expires_at timestamptz,
  valid boolean
)
language plpgsql
security definer
set search_path = public
as $$
begin
  return query
  select
    c.name,
    i.email,
    i.role,
    i.expires_at,
    (i.accepted_at is null and i.expires_at > now()) as valid
  from public.center_invites i
  join public.centers c on c.id = i.center_id
  where i.token = nullif(trim(p_token), '')
  limit 1;
end;
$$;

revoke all on function public.create_center_invite(text) from public;
revoke all on function public.revoke_center_member(uuid) from public;
revoke all on function public.get_invite_preview(text) from public;
revoke all on function public.current_member_role() from public;
revoke all on function public.is_center_owner() from public;

grant execute on function public.create_center_invite(text) to authenticated;
grant execute on function public.revoke_center_member(uuid) to authenticated;
grant execute on function public.get_invite_preview(text) to anon, authenticated;
grant execute on function public.current_member_role() to authenticated;
grant execute on function public.is_center_owner() to authenticated;
