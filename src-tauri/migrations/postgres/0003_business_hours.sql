-- Fase 3 del issue #2 — espejo en dialecto Postgres de
-- migrations/0003_business_hours.sql.

CREATE TABLE business_hours (
  weekday SMALLINT PRIMARY KEY CHECK (weekday BETWEEN 0 AND 6),
  is_open BOOLEAN NOT NULL DEFAULT true,
  opens_at TEXT,
  closes_at TEXT,
  CHECK ((is_open = false AND opens_at IS NULL AND closes_at IS NULL) OR (is_open = true AND opens_at IS NOT NULL AND closes_at IS NOT NULL))
);

INSERT INTO business_hours (weekday, is_open, opens_at, closes_at) VALUES
  (0, false, NULL, NULL),
  (1, true, '09:00', '19:00'),
  (2, true, '09:00', '19:00'),
  (3, true, '09:00', '19:00'),
  (4, true, '09:00', '19:00'),
  (5, true, '09:00', '19:00'),
  (6, true, '09:00', '17:00');

CREATE TABLE employee_availability (
  id TEXT PRIMARY KEY,
  employee_id TEXT NOT NULL REFERENCES employees (id) ON DELETE CASCADE,
  weekday SMALLINT NOT NULL CHECK (weekday BETWEEN 0 AND 6),
  is_available BOOLEAN NOT NULL DEFAULT true,
  starts_at TEXT,
  ends_at TEXT,
  UNIQUE (employee_id, weekday)
);
CREATE INDEX employee_availability_employee_id_idx ON employee_availability (employee_id);
