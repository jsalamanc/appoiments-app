-- Fase 2 del issue #2 — espejo en dialecto Postgres de
-- migrations/0002_sales_v2.sql. A diferencia de SQLite, Postgres sí permite
-- reemplazar un CHECK existente con ALTER TABLE directo, sin recrear la
-- tabla.

ALTER TABLE sales
  ADD COLUMN user_id TEXT REFERENCES users (id) ON DELETE SET NULL,
  ADD COLUMN discount_amount DOUBLE PRECISION NOT NULL DEFAULT 0 CHECK (discount_amount >= 0),
  ADD COLUMN voided_at TIMESTAMPTZ,
  ADD COLUMN void_reason TEXT;

ALTER TABLE sales DROP CONSTRAINT sales_payment_method_check;
ALTER TABLE sales ADD CONSTRAINT sales_payment_method_check
  CHECK (payment_method IN ('cash', 'transfer', 'card', 'mixed'));

CREATE INDEX sales_user_id_idx ON sales (user_id);
CREATE INDEX sales_voided_at_idx ON sales (voided_at);

ALTER TABLE inventory_movements DROP CONSTRAINT inventory_movements_movement_type_check;
ALTER TABLE inventory_movements ADD CONSTRAINT inventory_movements_movement_type_check
  CHECK (movement_type IN ('initial', 'adjustment', 'sale', 'return'));

-- Desglose de pagos por venta — ver comentario en migrations/0002_sales_v2.sql.
CREATE TABLE sale_payments (
  id TEXT PRIMARY KEY,
  sale_id TEXT NOT NULL REFERENCES sales (id) ON DELETE CASCADE,
  method TEXT NOT NULL CHECK (method IN ('cash', 'transfer', 'card')),
  amount DOUBLE PRECISION NOT NULL CHECK (amount > 0),
  created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX sale_payments_sale_id_idx ON sale_payments (sale_id);
