-- Módulo reports (reportes del admin), gratis y conmutable
-- Ejecutar en: Supabase Dashboard → SQL Editor (después de 010)

alter table public.centers
  alter column modules set default '{
    "employees": false,
    "appointments": false,
    "services": false,
    "packages": false,
    "public_booking": false,
    "reports": false
  }'::jsonb;

update public.centers
set modules = coalesce(modules, '{}'::jsonb) || '{"reports": false}'::jsonb
where modules->>'reports' is null;
