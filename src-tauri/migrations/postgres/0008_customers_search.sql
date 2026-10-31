-- Issue #31: equivalente Postgres de ../0008_customers_search.sql. La
-- búsqueda de clientes (`lower(col) LIKE '%texto%'`) no puede usar un índice
-- B-tree por el comodín inicial. `pg_trgm` sí permite que el planner use un
-- índice GIN para acelerar `LIKE`/`ILIKE` con comodines en ambos extremos.

CREATE EXTENSION IF NOT EXISTS pg_trgm;

CREATE INDEX customers_name_trgm_idx ON customers USING gin (lower(name) gin_trgm_ops);
CREATE INDEX customers_email_trgm_idx ON customers USING gin (lower(email) gin_trgm_ops);
CREATE INDEX customers_document_trgm_idx ON customers USING gin (lower(document) gin_trgm_ops);
CREATE INDEX customers_phone_trgm_idx ON customers USING gin (lower(phone) gin_trgm_ops);
