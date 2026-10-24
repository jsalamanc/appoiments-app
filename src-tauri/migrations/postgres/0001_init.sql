-- Aura Studio (PostgreSQL externo, self-hosted) — esquema inicial.
-- Espejo del esquema SQLite local (../0001_init.sql): un solo negocio por
-- conexión, sin center_id, sin RLS, sin planes. No se adaptaron las
-- migraciones legacy de supabase/migrations/*.sql (traían center_id, RLS y
-- planes multi-tenant que no aplican a este modo); este archivo se escribió
-- de cero a partir del esquema SQLite actual, que es la fuente de verdad.
--
-- Decisiones de mapeo de dialecto respecto al esquema SQLite:
--   * Los IDs siguen siendo TEXT (UUIDs generados por la app), igual que en
--     SQLite, para no requerir conversión en la capa de repositorios.
--   * INTEGER ... CHECK (col IN (0,1)) -> BOOLEAN.
--   * TEXT DEFAULT (strftime(...)) -> TIMESTAMPTZ DEFAULT now().
--   * instr(email, '@') > 1 -> position('@' in email) > 1.
--   * REAL -> DOUBLE PRECISION (mismo comportamiento de punto flotante que
--     SQLite REAL; no se introduce NUMERIC para no cambiar semántica).

CREATE TABLE business_profile (
  id SMALLINT PRIMARY KEY CHECK (id = 1),
  name TEXT NOT NULL,
  module_employees BOOLEAN NOT NULL DEFAULT FALSE,
  module_appointments BOOLEAN NOT NULL DEFAULT FALSE,
  module_services BOOLEAN NOT NULL DEFAULT FALSE,
  module_packages BOOLEAN NOT NULL DEFAULT FALSE,
  module_reports BOOLEAN NOT NULL DEFAULT FALSE,
  created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE users (
  id TEXT PRIMARY KEY,
  name TEXT NOT NULL,
  username TEXT NOT NULL UNIQUE,
  password_hash TEXT NOT NULL,
  role TEXT NOT NULL CHECK (role IN ('owner','staff')),
  active BOOLEAN NOT NULL DEFAULT TRUE,
  created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE sessions (
  token TEXT PRIMARY KEY,
  user_id TEXT NOT NULL REFERENCES users (id) ON DELETE CASCADE,
  created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  expires_at TIMESTAMPTZ NOT NULL
);
CREATE INDEX sessions_user_id_idx ON sessions (user_id);

CREATE TABLE customers (
  id TEXT PRIMARY KEY,
  name TEXT NOT NULL,
  phone TEXT NOT NULL,
  email TEXT NOT NULL,
  document TEXT NOT NULL,
  created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  CHECK (position('@' in email) > 1)
);
CREATE UNIQUE INDEX customers_email_unique ON customers (lower(email));
CREATE UNIQUE INDEX customers_document_unique ON customers (lower(document));
CREATE INDEX customers_name_idx ON customers (name);

CREATE TABLE products (
  id TEXT PRIMARY KEY,
  name TEXT NOT NULL,
  price DOUBLE PRECISION NOT NULL CHECK (price >= 0),
  photo_path TEXT,
  status TEXT NOT NULL DEFAULT 'draft' CHECK (status IN ('draft','published')),
  stock INTEGER NOT NULL DEFAULT 0 CHECK (stock >= 0),
  created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX products_status_idx ON products (status);
CREATE INDEX products_stock_idx ON products (stock);

CREATE TABLE services (
  id TEXT PRIMARY KEY,
  name TEXT NOT NULL,
  description TEXT,
  price DOUBLE PRECISION NOT NULL CHECK (price >= 0),
  duration_minutes INTEGER NOT NULL CHECK (duration_minutes > 0),
  photo_path TEXT,
  status TEXT NOT NULL DEFAULT 'draft' CHECK (status IN ('draft','published')),
  created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX services_status_idx ON services (status);

CREATE TABLE packages (
  id TEXT PRIMARY KEY,
  name TEXT NOT NULL,
  description TEXT,
  price DOUBLE PRECISION NOT NULL CHECK (price >= 0),
  photo_path TEXT,
  status TEXT NOT NULL DEFAULT 'draft' CHECK (status IN ('draft','published')),
  created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX packages_status_idx ON packages (status);

CREATE TABLE package_items (
  id TEXT PRIMARY KEY,
  package_id TEXT NOT NULL REFERENCES packages (id) ON DELETE CASCADE,
  item_type TEXT NOT NULL CHECK (item_type IN ('product','service')),
  product_id TEXT REFERENCES products (id) ON DELETE CASCADE,
  service_id TEXT REFERENCES services (id) ON DELETE CASCADE,
  quantity INTEGER NOT NULL DEFAULT 1 CHECK (quantity >= 1),
  created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  CHECK (
    (item_type = 'product' AND product_id IS NOT NULL AND service_id IS NULL)
    OR (item_type = 'service' AND service_id IS NOT NULL AND product_id IS NULL)
  )
);
CREATE INDEX package_items_package_id_idx ON package_items (package_id);

CREATE TABLE employees (
  id TEXT PRIMARY KEY,
  name TEXT NOT NULL,
  color TEXT NOT NULL DEFAULT '#b89bb8',
  active BOOLEAN NOT NULL DEFAULT TRUE,
  created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX employees_active_idx ON employees (active);

CREATE TABLE appointments (
  id TEXT PRIMARY KEY,
  employee_id TEXT NOT NULL REFERENCES employees (id) ON DELETE RESTRICT,
  customer_id TEXT NOT NULL REFERENCES customers (id) ON DELETE RESTRICT,
  service_id TEXT REFERENCES services (id) ON DELETE SET NULL,
  service_name_snapshot TEXT NOT NULL,
  starts_at TIMESTAMPTZ NOT NULL,
  ends_at TIMESTAMPTZ NOT NULL,
  status TEXT NOT NULL DEFAULT 'scheduled'
    CHECK (status IN ('scheduled','completed','cancelled','no_show')),
  notes TEXT,
  sale_id TEXT,
  created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  CHECK (ends_at > starts_at)
);
CREATE INDEX appointments_employee_starts_idx ON appointments (employee_id, starts_at);
CREATE INDEX appointments_starts_idx ON appointments (starts_at);
CREATE INDEX appointments_customer_id_idx ON appointments (customer_id);
CREATE INDEX appointments_status_idx ON appointments (status);

CREATE TABLE sales (
  id TEXT PRIMARY KEY,
  customer_id TEXT NOT NULL REFERENCES customers (id) ON DELETE RESTRICT,
  appointment_id TEXT REFERENCES appointments (id) ON DELETE SET NULL,
  payment_method TEXT NOT NULL CHECK (payment_method IN ('cash','transfer')),
  total DOUBLE PRECISION NOT NULL CHECK (total >= 0),
  sold_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX sales_sold_at_idx ON sales (sold_at DESC);
CREATE INDEX sales_customer_id_idx ON sales (customer_id);
CREATE INDEX sales_payment_method_idx ON sales (payment_method);
CREATE INDEX sales_appointment_id_idx ON sales (appointment_id);

CREATE TABLE sale_items (
  id TEXT PRIMARY KEY,
  sale_id TEXT NOT NULL REFERENCES sales (id) ON DELETE CASCADE,
  item_type TEXT NOT NULL CHECK (item_type IN ('product','service','package')),
  product_id TEXT REFERENCES products (id) ON DELETE SET NULL,
  service_id TEXT REFERENCES services (id) ON DELETE SET NULL,
  package_id TEXT REFERENCES packages (id) ON DELETE SET NULL,
  name_snapshot TEXT NOT NULL,
  unit_price DOUBLE PRECISION NOT NULL CHECK (unit_price >= 0),
  quantity INTEGER NOT NULL CHECK (quantity >= 1),
  line_total DOUBLE PRECISION NOT NULL CHECK (line_total >= 0),
  created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  CHECK (
    (item_type = 'product' AND service_id IS NULL AND package_id IS NULL)
    OR (item_type = 'service' AND product_id IS NULL AND package_id IS NULL)
    OR (item_type = 'package' AND product_id IS NULL AND service_id IS NULL)
  )
);
CREATE INDEX sale_items_sale_id_idx ON sale_items (sale_id);

CREATE TABLE inventory_movements (
  id TEXT PRIMARY KEY,
  product_id TEXT NOT NULL REFERENCES products (id) ON DELETE CASCADE,
  movement_type TEXT NOT NULL CHECK (movement_type IN ('initial','adjustment','sale')),
  quantity_delta INTEGER NOT NULL CHECK (quantity_delta <> 0),
  stock_after INTEGER NOT NULL CHECK (stock_after >= 0),
  sale_id TEXT REFERENCES sales (id) ON DELETE SET NULL,
  note TEXT,
  created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX inventory_movements_product_id_idx ON inventory_movements (product_id, created_at DESC);
CREATE INDEX inventory_movements_sale_id_idx ON inventory_movements (sale_id);
