-- Issue #2 Fase 6 — ver 0007_money_as_integers.sql (SQLite) para el
-- razonamiento. En Postgres no hace falta recrear tablas: ALTER COLUMN TYPE
-- re-valida los CHECK existentes automáticamente contra el nuevo tipo.
ALTER TABLE products ALTER COLUMN price TYPE BIGINT USING ROUND(price)::BIGINT;
ALTER TABLE products ALTER COLUMN cost TYPE BIGINT USING ROUND(cost)::BIGINT;

ALTER TABLE services ALTER COLUMN price TYPE BIGINT USING ROUND(price)::BIGINT;

ALTER TABLE packages ALTER COLUMN price TYPE BIGINT USING ROUND(price)::BIGINT;

ALTER TABLE sale_items ALTER COLUMN unit_price TYPE BIGINT USING ROUND(unit_price)::BIGINT;
ALTER TABLE sale_items ALTER COLUMN line_total TYPE BIGINT USING ROUND(line_total)::BIGINT;

ALTER TABLE sales ALTER COLUMN total TYPE BIGINT USING ROUND(total)::BIGINT;
ALTER TABLE sales ALTER COLUMN discount_amount TYPE BIGINT USING ROUND(discount_amount)::BIGINT;

ALTER TABLE sale_payments ALTER COLUMN amount TYPE BIGINT USING ROUND(amount)::BIGINT;

ALTER TABLE cash_sessions ALTER COLUMN opening_amount TYPE BIGINT USING ROUND(opening_amount)::BIGINT;
ALTER TABLE cash_sessions ALTER COLUMN counted_amount TYPE BIGINT USING ROUND(counted_amount)::BIGINT;
ALTER TABLE cash_sessions ALTER COLUMN expected_amount TYPE BIGINT USING ROUND(expected_amount)::BIGINT;
ALTER TABLE cash_sessions ALTER COLUMN difference TYPE BIGINT USING ROUND(difference)::BIGINT;
