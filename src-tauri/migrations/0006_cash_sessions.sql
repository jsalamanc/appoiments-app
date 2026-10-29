-- Issue #2 Fase 5, parte 2: apertura y cierre de caja. Una fila por turno de
-- caja — se abre con un monto inicial y se cierra con el monto contado en
-- físico; `expected_amount` (lo que debería haber: apertura + ventas en
-- efectivo del turno) y `difference` (contado - esperado) se calculan al
-- cerrar (ver `repositories::cash_sessions::close`), no se recalculan después.
--
-- Solo puede haber una sesión abierta a la vez — se valida en la capa de
-- aplicación (dentro de la misma transacción que abre una nueva) en vez de
-- con una restricción SQL, igual criterio que ya se usó para los medios de
-- pago y el horario de negocio: más simple de leer y de testear que un
-- índice único parcial sobre una expresión constante.
CREATE TABLE cash_sessions (
  id TEXT PRIMARY KEY,
  opened_by TEXT REFERENCES users (id) ON DELETE SET NULL,
  opened_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
  opening_amount REAL NOT NULL CHECK (opening_amount >= 0),
  closed_by TEXT REFERENCES users (id) ON DELETE SET NULL,
  closed_at TEXT,
  counted_amount REAL CHECK (counted_amount IS NULL OR counted_amount >= 0),
  expected_amount REAL,
  difference REAL,
  notes TEXT,
  CHECK (
    (closed_at IS NULL AND counted_amount IS NULL AND expected_amount IS NULL AND difference IS NULL)
    OR (closed_at IS NOT NULL AND counted_amount IS NOT NULL AND expected_amount IS NOT NULL AND difference IS NOT NULL)
  )
);
CREATE INDEX cash_sessions_opened_at_idx ON cash_sessions (opened_at DESC);
