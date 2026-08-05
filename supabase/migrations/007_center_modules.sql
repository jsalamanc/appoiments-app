-- Módulos opcionales por centro (feature flags)
-- Ejecutar en: Supabase Dashboard → SQL Editor
-- Core siempre activo: productos + ventas.
-- Opcionales: employees, appointments, services, packages.
-- Default para centros nuevos: todos false.
-- Backfill: centros ya existentes quedan con todos true.

alter table public.centers
  add column if not exists modules jsonb not null default '{
    "employees": false,
    "appointments": false,
    "services": false,
    "packages": false
  }'::jsonb;

alter table public.centers
  add column if not exists plan_id text;

-- Centros existentes al momento de la migración: no romper funcionalidad actual
update public.centers
set modules = '{
  "employees": true,
  "appointments": true,
  "services": true,
  "packages": true
}'::jsonb;
