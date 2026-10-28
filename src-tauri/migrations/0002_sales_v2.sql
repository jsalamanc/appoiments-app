-- Fase 2 del issue #2: trazabilidad de quién hizo la venta, anulación de
-- ventas (con reversa de inventario), tarjeta como medio de pago + pago
-- mixto (varios medios en una misma venta), y descuento por venta.
--
-- `sales` e `inventory_movements` tienen CHECK a nivel de columna fijados en
-- su CREATE TABLE original; SQLite no soporta ALTER de un CHECK existente,
-- así que se recrean siguiendo el procedimiento de 12 pasos documentado por
-- SQLite para cambios de esquema no soportados por ALTER TABLE directo
-- (https://sqlite.org/lang_altertable.html#otheralter). No hace falta
-- alternar `PRAGMA foreign_keys` a mano: rusqlite_migration ya corre cada
-- migración dentro de su propia transacción (alternarlo ahí sería un no-op),
-- y SQLite no exige que los padres de una FK existan en el momento del
-- DROP/CREATE, solo al hacer INSERT/UPDATE.

CREATE TABLE sales_new (
  id TEXT PRIMARY KEY,
  customer_id TEXT NOT NULL REFERENCES customers (id) ON DELETE RESTRICT,
  appointment_id TEXT REFERENCES appointments (id) ON DELETE SET NULL,
  user_id TEXT REFERENCES users (id) ON DELETE SET NULL,
  payment_method TEXT NOT NULL CHECK (payment_method IN ('cash','transfer','card','mixed')),
  total REAL NOT NULL CHECK (total >= 0),
  discount_amount REAL NOT NULL DEFAULT 0 CHECK (discount_amount >= 0),
  sold_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
  created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
  voided_at TEXT,
  void_reason TEXT
);

INSERT INTO sales_new (id, customer_id, appointment_id, payment_method, total, sold_at, created_at)
SELECT id, customer_id, appointment_id, payment_method, total, sold_at, created_at FROM sales;

DROP TABLE sales;
ALTER TABLE sales_new RENAME TO sales;

CREATE INDEX sales_sold_at_idx ON sales (sold_at DESC);
CREATE INDEX sales_customer_id_idx ON sales (customer_id);
CREATE INDEX sales_payment_method_idx ON sales (payment_method);
CREATE INDEX sales_appointment_id_idx ON sales (appointment_id);
CREATE INDEX sales_user_id_idx ON sales (user_id);
CREATE INDEX sales_voided_at_idx ON sales (voided_at);

CREATE TABLE inventory_movements_new (
  id TEXT PRIMARY KEY,
  product_id TEXT NOT NULL REFERENCES products (id) ON DELETE CASCADE,
  movement_type TEXT NOT NULL CHECK (movement_type IN ('initial','adjustment','sale','return')),
  quantity_delta INTEGER NOT NULL CHECK (quantity_delta <> 0),
  stock_after INTEGER NOT NULL CHECK (stock_after >= 0),
  sale_id TEXT REFERENCES sales (id) ON DELETE SET NULL,
  note TEXT,
  created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now'))
);

INSERT INTO inventory_movements_new
SELECT id, product_id, movement_type, quantity_delta, stock_after, sale_id, note, created_at
FROM inventory_movements;

DROP TABLE inventory_movements;
ALTER TABLE inventory_movements_new RENAME TO inventory_movements;

CREATE INDEX inventory_movements_product_id_idx ON inventory_movements (product_id, created_at DESC);
CREATE INDEX inventory_movements_sale_id_idx ON inventory_movements (sale_id);

-- Desglose de pagos por venta — una fila por medio de pago usado; una venta
-- normal tiene una sola fila, una venta con pago mixto tiene varias (ej.
-- mitad efectivo, mitad tarjeta). La suma de `amount` de una venta siempre
-- debe ser igual a `sales.total` (validado en la capa de aplicación).
CREATE TABLE sale_payments (
  id TEXT PRIMARY KEY,
  sale_id TEXT NOT NULL REFERENCES sales (id) ON DELETE CASCADE,
  method TEXT NOT NULL CHECK (method IN ('cash','transfer','card')),
  amount REAL NOT NULL CHECK (amount > 0),
  created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now'))
);
CREATE INDEX sale_payments_sale_id_idx ON sale_payments (sale_id);
