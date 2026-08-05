-- Matriz plan → módulos permitidos
-- Ejecutar en: Supabase Dashboard → SQL Editor (después de 012)

alter table public.plans
  add column if not exists allowed_modules jsonb not null default '{
    "employees": false,
    "appointments": false,
    "services": false,
    "packages": false,
    "public_booking": false,
    "reports": false
  }'::jsonb;

update public.plans
set allowed_modules = '{
  "employees": false,
  "appointments": false,
  "services": false,
  "packages": false,
  "public_booking": false,
  "reports": false
}'::jsonb
where id = 'free';

update public.plans
set allowed_modules = '{
  "employees": true,
  "appointments": true,
  "services": true,
  "packages": true,
  "public_booking": false,
  "reports": false
}'::jsonb
where id = 'starter';

update public.plans
set allowed_modules = '{
  "employees": true,
  "appointments": true,
  "services": true,
  "packages": true,
  "public_booking": true,
  "reports": true
}'::jsonb
where id = 'growth';

-- Apagar en cada centro módulos que su plan no permite
update public.centers c
set modules = jsonb_build_object(
  'employees',
    coalesce((c.modules->>'employees')::boolean, false)
    and coalesce((p.allowed_modules->>'employees')::boolean, false),
  'appointments',
    coalesce((c.modules->>'appointments')::boolean, false)
    and coalesce((p.allowed_modules->>'appointments')::boolean, false),
  'services',
    coalesce((c.modules->>'services')::boolean, false)
    and coalesce((p.allowed_modules->>'services')::boolean, false),
  'packages',
    coalesce((c.modules->>'packages')::boolean, false)
    and coalesce((p.allowed_modules->>'packages')::boolean, false),
  'public_booking',
    coalesce((c.modules->>'public_booking')::boolean, false)
    and coalesce((p.allowed_modules->>'public_booking')::boolean, false),
  'reports',
    coalesce((c.modules->>'reports')::boolean, false)
    and coalesce((p.allowed_modules->>'reports')::boolean, false)
)
from public.plans p
where p.id = c.plan_id;
