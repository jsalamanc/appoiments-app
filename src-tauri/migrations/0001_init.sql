-- Aura Studio (edición local) — esquema SQLite inicial.
-- Un solo negocio por instalación: sin center_id, sin RLS, sin planes.

CREATE TABLE business_profile (
  id INTEGER PRIMARY KEY CHECK (id = 1),
  name TEXT NOT NULL,
  module_employees INTEGER NOT NULL DEFAULT 0 CHECK (module_employees IN (0,1)),
  module_appointments INTEGER NOT NULL DEFAULT 0 CHECK (module_appointments IN (0,1)),
  module_services INTEGER NOT NULL DEFAULT 0 CHECK (module_services IN (0,1)),
  module_packages INTEGER NOT NULL DEFAULT 0 CHECK (module_packages IN (0,1)),
  module_reports INTEGER NOT NULL DEFAULT 0 CHECK (module_reports IN (0,1)),
  created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
  updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now'))
);

CREATE TABLE users (
  id TEXT PRIMARY KEY,
  name TEXT NOT NULL,
  username TEXT NOT NULL UNIQUE,
  password_hash TEXT NOT NULL,
  role TEXT NOT NULL CHECK (role IN ('owner','staff')),
  active INTEGER NOT NULL DEFAULT 1 CHECK (active IN (0,1)),
  created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
  updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now'))
);

CREATE TABLE sessions (
  token TEXT PRIMARY KEY,
  user_id TEXT NOT NULL REFERENCES users (id) ON DELETE CASCADE,
  created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
  expires_at TEXT NOT NULL
);
CREATE INDEX sessions_user_id_idx ON sessions (user_id);

CREATE TABLE customers (
  id TEXT PRIMARY KEY,
  name TEXT NOT NULL,
  phone TEXT NOT NULL,
  email TEXT NOT NULL,
  document TEXT NOT NULL,
  created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
  updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
  CHECK (instr(email, '@') > 1)
);
CREATE UNIQUE INDEX customers_email_unique ON customers (lower(email));
CREATE UNIQUE INDEX customers_document_unique ON customers (lower(document));
CREATE INDEX customers_name_idx ON customers (name);

CREATE TABLE products (
  id TEXT PRIMARY KEY,
  name TEXT NOT NULL,
  price REAL NOT NULL CHECK (price >= 0),
  photo_path TEXT,
  status TEXT NOT NULL DEFAULT 'draft' CHECK (status IN ('draft','published')),
  stock INTEGER NOT NULL DEFAULT 0 CHECK (stock >= 0),
  created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
  updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now'))
);
CREATE INDEX products_status_idx ON products (status);
CREATE INDEX products_stock_idx ON products (stock);

CREATE TABLE services (
  id TEXT PRIMARY KEY,
  name TEXT NOT NULL,
  description TEXT,
  price REAL NOT NULL CHECK (price >= 0),
  duration_minutes INTEGER NOT NULL CHECK (duration_minutes > 0),
  photo_path TEXT,
  status TEXT NOT NULL DEFAULT 'draft' CHECK (status IN ('draft','published')),
  created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
  updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now'))
);
CREATE INDEX services_status_idx ON services (status);

CREATE TABLE packages (
  id TEXT PRIMARY KEY,
  name TEXT NOT NULL,
  description TEXT,
  price REAL NOT NULL CHECK (price >= 0),
  photo_path TEXT,
  status TEXT NOT NULL DEFAULT 'draft' CHECK (status IN ('draft','published')),
  created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
  updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now'))
);
CREATE INDEX packages_status_idx ON packages (status);

CREATE TABLE package_items (
  id TEXT PRIMARY KEY,
  package_id TEXT NOT NULL REFERENCES packages (id) ON DELETE CASCADE,
  item_type TEXT NOT NULL CHECK (item_type IN ('product','service')),
  product_id TEXT REFERENCES products (id) ON DELETE CASCADE,
  service_id TEXT REFERENCES services (id) ON DELETE CASCADE,
  quantity INTEGER NOT NULL DEFAULT 1 CHECK (quantity >= 1),
  created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
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
  active INTEGER NOT NULL DEFAULT 1 CHECK (active IN (0,1)),
  created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
  updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now'))
);
CREATE INDEX employees_active_idx ON employees (active);

CREATE TABLE appointments (
  id TEXT PRIMARY KEY,
  employee_id TEXT NOT NULL REFERENCES employees (id) ON DELETE RESTRICT,
  customer_id TEXT NOT NULL REFERENCES customers (id) ON DELETE RESTRICT,
  service_id TEXT REFERENCES services (id) ON DELETE SET NULL,
  service_name_snapshot TEXT NOT NULL,
  starts_at TEXT NOT NULL,
  ends_at TEXT NOT NULL,
  status TEXT NOT NULL DEFAULT 'scheduled'
    CHECK (status IN ('scheduled','completed','cancelled','no_show')),
  notes TEXT,
  sale_id TEXT,
  created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
  updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
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
  total REAL NOT NULL CHECK (total >= 0),
  sold_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
  created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now'))
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
  unit_price REAL NOT NULL CHECK (unit_price >= 0),
  quantity INTEGER NOT NULL CHECK (quantity >= 1),
  line_total REAL NOT NULL CHECK (line_total >= 0),
  created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
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
  created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now'))
);
CREATE INDEX inventory_movements_product_id_idx ON inventory_movements (product_id, created_at DESC);
CREATE INDEX inventory_movements_sale_id_idx ON inventory_movements (sale_id);
