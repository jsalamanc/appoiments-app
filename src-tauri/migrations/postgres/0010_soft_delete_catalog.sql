-- Equivalente Postgres de ../0010_soft_delete_catalog.sql — borrado lógico
-- del catálogo para conservar el historial de ventas, citas e inventario.
ALTER TABLE products ADD COLUMN deleted_at TIMESTAMPTZ;
ALTER TABLE services ADD COLUMN deleted_at TIMESTAMPTZ;
ALTER TABLE packages ADD COLUMN deleted_at TIMESTAMPTZ;

CREATE INDEX products_deleted_at_idx ON products (deleted_at);
CREATE INDEX services_deleted_at_idx ON services (deleted_at);
CREATE INDEX packages_deleted_at_idx ON packages (deleted_at);
