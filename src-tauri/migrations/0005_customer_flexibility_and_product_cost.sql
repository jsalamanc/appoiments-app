-- Issue #2 Fase 5, parte 1:
-- 1) `customers.email`/`document` pasan a opcionales — hoy una venta rápida
--    (ej. cliente de paso) obliga a inventar un correo/documento solo para
--    satisfacer un NOT NULL que no aporta nada. La unicidad se mantiene,
--    pero solo entre los valores que sí están presentes (dos clientes sin
--    correo no deben chocar entre sí).
-- 2) `products.cost` para poder calcular margen — se actualiza también al
--    registrar una entrada de stock con costo (ver `repositories::products`).
--
-- `customers` tiene un CHECK a nivel de tabla fijado en su CREATE TABLE
-- original y SQLite no soporta ALTER de un CHECK existente ni quitar un NOT
-- NULL con ALTER COLUMN, así que se recrea siguiendo el mismo procedimiento
-- de 12 pasos ya usado en 0002_sales_v2.sql.

CREATE TABLE customers_new (
  id TEXT PRIMARY KEY,
  name TEXT NOT NULL,
  phone TEXT NOT NULL,
  email TEXT,
  document TEXT,
  created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
  updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
  CHECK (email IS NULL OR instr(email, '@') > 1)
);

INSERT INTO customers_new (id, name, phone, email, document, created_at, updated_at)
SELECT id, name, phone, email, document, created_at, updated_at FROM customers;

DROP TABLE customers;
ALTER TABLE customers_new RENAME TO customers;

CREATE UNIQUE INDEX customers_email_unique ON customers (lower(email)) WHERE email IS NOT NULL;
CREATE UNIQUE INDEX customers_document_unique ON customers (lower(document)) WHERE document IS NOT NULL;
CREATE INDEX customers_name_idx ON customers (name);

ALTER TABLE products ADD COLUMN cost REAL NOT NULL DEFAULT 0 CHECK (cost >= 0);
