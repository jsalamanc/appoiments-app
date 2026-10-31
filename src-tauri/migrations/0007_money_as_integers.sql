-- Issue #2 Fase 6: columnas de dinero como enteros (pesos exactos), no
-- REAL. El peso colombiano no tiene subunidad en uso práctico — la app ya
-- solo maneja valores enteros (ver `formatCurrency`,
-- `maximumFractionDigits: 0`) — así que no hace falta introducir
-- "centavos": basta con declarar la columna como INTEGER. ROUND(...) al
-- copiar los datos por si algún valor existente quedó con fracción por
-- aritmética de punto flotante previa.
--
-- `employee_commission_rates.rate_value` queda fuera a propósito: es de
-- doble uso (porcentaje O monto fijo según `rate_type`), y un porcentaje
-- (ej. 12.5%) necesita poder ser fraccionario — no es un monto de dinero en
-- sí mismo, es una tasa.
--
-- Todas las tablas con columnas de dinero tienen CHECK fijados en su
-- CREATE TABLE original, así que se recrean siguiendo el mismo
-- procedimiento de 12 pasos usado en 0002_sales_v2.sql.

CREATE TABLE products_new (
  id TEXT PRIMARY KEY,
  name TEXT NOT NULL,
  price INTEGER NOT NULL CHECK (price >= 0),
  cost INTEGER NOT NULL DEFAULT 0 CHECK (cost >= 0),
  photo_path TEXT,
  status TEXT NOT NULL DEFAULT 'draft' CHECK (status IN ('draft','published')),
  stock INTEGER NOT NULL DEFAULT 0 CHECK (stock >= 0),
  created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
  updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now'))
);
INSERT INTO products_new (id, name, price, cost, photo_path, status, stock, created_at, updated_at)
SELECT id, name, ROUND(price), ROUND(cost), photo_path, status, stock, created_at, updated_at FROM products;
DROP TABLE products;
ALTER TABLE products_new RENAME TO products;
CREATE INDEX products_status_idx ON products (status);
CREATE INDEX products_stock_idx ON products (stock);

CREATE TABLE services_new (
  id TEXT PRIMARY KEY,
  name TEXT NOT NULL,
  description TEXT,
  price INTEGER NOT NULL CHECK (price >= 0),
  duration_minutes INTEGER NOT NULL CHECK (duration_minutes > 0),
  photo_path TEXT,
  status TEXT NOT NULL DEFAULT 'draft' CHECK (status IN ('draft','published')),
  created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
  updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now'))
);
INSERT INTO services_new (id, name, description, price, duration_minutes, photo_path, status, created_at, updated_at)
SELECT id, name, description, ROUND(price), duration_minutes, photo_path, status, created_at, updated_at FROM services;
DROP TABLE services;
ALTER TABLE services_new RENAME TO services;
CREATE INDEX services_status_idx ON services (status);

CREATE TABLE packages_new (
  id TEXT PRIMARY KEY,
  name TEXT NOT NULL,
  description TEXT,
  price INTEGER NOT NULL CHECK (price >= 0),
  photo_path TEXT,
  status TEXT NOT NULL DEFAULT 'draft' CHECK (status IN ('draft','published')),
  created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
  updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now'))
);
INSERT INTO packages_new (id, name, description, price, photo_path, status, created_at, updated_at)
SELECT id, name, description, ROUND(price), photo_path, status, created_at, updated_at FROM packages;
DROP TABLE packages;
ALTER TABLE packages_new RENAME TO packages;
CREATE INDEX packages_status_idx ON packages (status);

CREATE TABLE sale_items_new (
  id TEXT PRIMARY KEY,
  sale_id TEXT NOT NULL REFERENCES sales (id) ON DELETE CASCADE,
  item_type TEXT NOT NULL CHECK (item_type IN ('product','service','package')),
  product_id TEXT REFERENCES products (id) ON DELETE SET NULL,
  service_id TEXT REFERENCES services (id) ON DELETE SET NULL,
  package_id TEXT REFERENCES packages (id) ON DELETE SET NULL,
  name_snapshot TEXT NOT NULL,
  unit_price INTEGER NOT NULL CHECK (unit_price >= 0),
  quantity INTEGER NOT NULL CHECK (quantity >= 1),
  line_total INTEGER NOT NULL CHECK (line_total >= 0),
  created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
  CHECK (
    (item_type = 'product' AND service_id IS NULL AND package_id IS NULL)
    OR (item_type = 'service' AND product_id IS NULL AND package_id IS NULL)
    OR (item_type = 'package' AND product_id IS NULL AND service_id IS NULL)
  )
);
INSERT INTO sale_items_new
SELECT id, sale_id, item_type, product_id, service_id, package_id, name_snapshot,
       ROUND(unit_price), quantity, ROUND(line_total), created_at
FROM sale_items;
DROP TABLE sale_items;
ALTER TABLE sale_items_new RENAME TO sale_items;
CREATE INDEX sale_items_sale_id_idx ON sale_items (sale_id);

CREATE TABLE sales_new (
  id TEXT PRIMARY KEY,
  customer_id TEXT NOT NULL REFERENCES customers (id) ON DELETE RESTRICT,
  appointment_id TEXT REFERENCES appointments (id) ON DELETE SET NULL,
  user_id TEXT REFERENCES users (id) ON DELETE SET NULL,
  payment_method TEXT NOT NULL CHECK (payment_method IN ('cash','transfer','card','mixed')),
  total INTEGER NOT NULL CHECK (total >= 0),
  discount_amount INTEGER NOT NULL DEFAULT 0 CHECK (discount_amount >= 0),
  sold_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
  created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
  voided_at TEXT,
  void_reason TEXT
);
INSERT INTO sales_new
SELECT id, customer_id, appointment_id, user_id, payment_method, ROUND(total), ROUND(discount_amount),
       sold_at, created_at, voided_at, void_reason
FROM sales;
DROP TABLE sales;
ALTER TABLE sales_new RENAME TO sales;
CREATE INDEX sales_sold_at_idx ON sales (sold_at DESC);
CREATE INDEX sales_customer_id_idx ON sales (customer_id);
CREATE INDEX sales_payment_method_idx ON sales (payment_method);
CREATE INDEX sales_appointment_id_idx ON sales (appointment_id);
CREATE INDEX sales_user_id_idx ON sales (user_id);
CREATE INDEX sales_voided_at_idx ON sales (voided_at);

CREATE TABLE sale_payments_new (
  id TEXT PRIMARY KEY,
  sale_id TEXT NOT NULL REFERENCES sales (id) ON DELETE CASCADE,
  method TEXT NOT NULL CHECK (method IN ('cash','transfer','card')),
  amount INTEGER NOT NULL CHECK (amount > 0),
  created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now'))
);
INSERT INTO sale_payments_new
SELECT id, sale_id, method, ROUND(amount), created_at FROM sale_payments;
DROP TABLE sale_payments;
ALTER TABLE sale_payments_new RENAME TO sale_payments;
CREATE INDEX sale_payments_sale_id_idx ON sale_payments (sale_id);

CREATE TABLE cash_sessions_new (
  id TEXT PRIMARY KEY,
  opened_by TEXT REFERENCES users (id) ON DELETE SET NULL,
  opened_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
  opening_amount INTEGER NOT NULL CHECK (opening_amount >= 0),
  closed_by TEXT REFERENCES users (id) ON DELETE SET NULL,
  closed_at TEXT,
  counted_amount INTEGER CHECK (counted_amount IS NULL OR counted_amount >= 0),
  expected_amount INTEGER,
  difference INTEGER,
  notes TEXT,
  CHECK (
    (closed_at IS NULL AND counted_amount IS NULL AND expected_amount IS NULL AND difference IS NULL)
    OR (closed_at IS NOT NULL AND counted_amount IS NOT NULL AND expected_amount IS NOT NULL AND difference IS NOT NULL)
  )
);
INSERT INTO cash_sessions_new
SELECT id, opened_by, opened_at, ROUND(opening_amount), closed_by, closed_at,
       ROUND(counted_amount), ROUND(expected_amount), ROUND(difference), notes
FROM cash_sessions;
DROP TABLE cash_sessions;
ALTER TABLE cash_sessions_new RENAME TO cash_sessions;
CREATE INDEX cash_sessions_opened_at_idx ON cash_sessions (opened_at DESC);
