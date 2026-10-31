-- Fase 3 del issue #2: horario de atención del negocio y disponibilidad por
-- empleado, validados al crear/editar una cita (antes se podía agendar a
-- cualquier hora de cualquier día).

-- Horario de atención, una fila por día de la semana (domingo=0 .. sábado=6,
-- misma convención que `Date.getDay()` en JS para no tener que traducir en
-- el frontend). Sembrado con un horario por defecto razonable; el dueño lo
-- ajusta desde Ajustes.
CREATE TABLE business_hours (
  weekday INTEGER PRIMARY KEY CHECK (weekday BETWEEN 0 AND 6),
  is_open INTEGER NOT NULL DEFAULT 1 CHECK (is_open IN (0,1)),
  opens_at TEXT,
  closes_at TEXT,
  CHECK ((is_open = 0 AND opens_at IS NULL AND closes_at IS NULL) OR (is_open = 1 AND opens_at IS NOT NULL AND closes_at IS NOT NULL))
);

INSERT INTO business_hours (weekday, is_open, opens_at, closes_at) VALUES
  (0, 0, NULL, NULL),
  (1, 1, '09:00', '19:00'),
  (2, 1, '09:00', '19:00'),
  (3, 1, '09:00', '19:00'),
  (4, 1, '09:00', '19:00'),
  (5, 1, '09:00', '19:00'),
  (6, 1, '09:00', '17:00');

-- Excepciones de disponibilidad por empleado — sparse: si no hay fila para
-- un empleado+día, ese empleado sigue el horario de negocio ese día. Una
-- fila con is_available=0 significa "no trabaja ese día"; con horas propias
-- (starts_at/ends_at no nulos), reemplaza el horario de negocio ese día.
CREATE TABLE employee_availability (
  id TEXT PRIMARY KEY,
  employee_id TEXT NOT NULL REFERENCES employees (id) ON DELETE CASCADE,
  weekday INTEGER NOT NULL CHECK (weekday BETWEEN 0 AND 6),
  is_available INTEGER NOT NULL DEFAULT 1 CHECK (is_available IN (0,1)),
  starts_at TEXT,
  ends_at TEXT,
  UNIQUE (employee_id, weekday)
);
CREATE INDEX employee_availability_employee_id_idx ON employee_availability (employee_id);
