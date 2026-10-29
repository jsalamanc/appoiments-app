-- Issue #2 Fase 5, parte 1 — ver 0005_customer_flexibility_and_product_cost.sql
-- (SQLite) para el razonamiento. En Postgres no hace falta recrear la
-- tabla: se puede ALTER directamente.

ALTER TABLE customers ALTER COLUMN email DROP NOT NULL;
ALTER TABLE customers ALTER COLUMN document DROP NOT NULL;

-- El CHECK de formato de email se declaró sin nombre en 0001_init.sql, así
-- que Postgres le asignó un nombre autogenerado — se busca dinámicamente en
-- vez de asumirlo, y se reemplaza por una versión que tolera NULL.
DO $$
DECLARE
  existing_check text;
BEGIN
  SELECT conname INTO existing_check
  FROM pg_constraint
  WHERE conrelid = 'customers'::regclass
    AND contype = 'c'
    AND pg_get_constraintdef(oid) LIKE '%email%';

  IF existing_check IS NOT NULL THEN
    EXECUTE format('ALTER TABLE customers DROP CONSTRAINT %I', existing_check);
  END IF;
END $$;

ALTER TABLE customers
  ADD CONSTRAINT customers_email_format_check
  CHECK (email IS NULL OR position('@' in email) > 1);

DROP INDEX customers_email_unique;
DROP INDEX customers_document_unique;
CREATE UNIQUE INDEX customers_email_unique ON customers (lower(email)) WHERE email IS NOT NULL;
CREATE UNIQUE INDEX customers_document_unique ON customers (lower(document)) WHERE document IS NOT NULL;

ALTER TABLE products ADD COLUMN cost DOUBLE PRECISION NOT NULL DEFAULT 0 CHECK (cost >= 0);
