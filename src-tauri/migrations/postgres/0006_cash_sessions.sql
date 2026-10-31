-- Issue #2 Fase 5, parte 2 — ver 0006_cash_sessions.sql (SQLite) para el
-- razonamiento del diseño.
CREATE TABLE cash_sessions (
  id TEXT PRIMARY KEY,
  opened_by TEXT REFERENCES users (id) ON DELETE SET NULL,
  opened_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  opening_amount DOUBLE PRECISION NOT NULL CHECK (opening_amount >= 0),
  closed_by TEXT REFERENCES users (id) ON DELETE SET NULL,
  closed_at TIMESTAMPTZ,
  counted_amount DOUBLE PRECISION CHECK (counted_amount IS NULL OR counted_amount >= 0),
  expected_amount DOUBLE PRECISION,
  difference DOUBLE PRECISION,
  notes TEXT,
  CHECK (
    (closed_at IS NULL AND counted_amount IS NULL AND expected_amount IS NULL AND difference IS NULL)
    OR (closed_at IS NOT NULL AND counted_amount IS NOT NULL AND expected_amount IS NOT NULL AND difference IS NOT NULL)
  )
);
CREATE INDEX cash_sessions_opened_at_idx ON cash_sessions (opened_at DESC);
