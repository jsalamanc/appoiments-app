-- Borrado lógico del catálogo: eliminar un producto, servicio o paquete ya
-- no borra la fila, solo la marca con `deleted_at`. Así el historial de
-- ventas, citas, inventario y auditoría que apunta a ese registro sigue
-- intacto. Las pantallas, el selector de ventas y los reportes de catálogo
-- ignoran las filas con `deleted_at` (ver `WHERE deleted_at IS NULL`).
ALTER TABLE products ADD COLUMN deleted_at TEXT;
ALTER TABLE services ADD COLUMN deleted_at TEXT;
ALTER TABLE packages ADD COLUMN deleted_at TEXT;

CREATE INDEX products_deleted_at_idx ON products (deleted_at);
CREATE INDEX services_deleted_at_idx ON services (deleted_at);
CREATE INDEX packages_deleted_at_idx ON packages (deleted_at);
