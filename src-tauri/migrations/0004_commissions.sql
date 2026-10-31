-- Issue #22 / #2 Fase 4: comisión por empleado sobre las citas completadas
-- (vía la venta ya vinculada, mismo join que usa `employee_performance`).
--
-- Una sola tabla cubre la tasa general del empleado (`service_id IS NULL`) y
-- las excepciones por servicio puntual (`service_id` presente). Un UNIQUE
-- normal sobre (employee_id, service_id) no serviría para impedir más de
-- una tasa general por empleado, porque SQL trata cada NULL como distinto
-- en una restricción UNIQUE — por eso se usan dos índices únicos parciales
-- en su lugar, uno por caso.
CREATE TABLE employee_commission_rates (
  id TEXT PRIMARY KEY,
  employee_id TEXT NOT NULL REFERENCES employees (id) ON DELETE CASCADE,
  service_id TEXT REFERENCES services (id) ON DELETE CASCADE,
  rate_type TEXT NOT NULL CHECK (rate_type IN ('percentage','fixed')),
  rate_value REAL NOT NULL CHECK (rate_value >= 0)
);

CREATE UNIQUE INDEX employee_commission_rates_default_idx
  ON employee_commission_rates (employee_id)
  WHERE service_id IS NULL;

CREATE UNIQUE INDEX employee_commission_rates_service_idx
  ON employee_commission_rates (employee_id, service_id)
  WHERE service_id IS NOT NULL;

CREATE INDEX employee_commission_rates_employee_id_idx
  ON employee_commission_rates (employee_id);
