-- Documento de identidad del cliente
-- Ejecutar en: Supabase Dashboard → SQL Editor

alter table public.customers
  add column if not exists document text;

update public.customers
set document = 'SIN-DOC-' || substr(replace(id::text, '-', ''), 1, 8)
where document is null or trim(document) = '';

alter table public.customers
  alter column document set not null;

create unique index if not exists customers_center_document_unique
  on public.customers (center_id, lower(document));
