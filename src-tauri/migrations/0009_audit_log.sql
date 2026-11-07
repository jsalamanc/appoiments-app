-- Bitácora de auditoría: una fila por acción relevante hecha por un usuario
-- (ventas, anulaciones, inventario, catálogo, citas, clientes, empleados,
-- caja, usuarios, configuración, inicios de sesión).
--
-- Decisiones de diseño:
--  * Sin llaves foráneas a propósito: el actor y la entidad se guardan como
--    texto copiado (`actor_name`, `actor_username`, `entity_id`) para que el
--    rastro sobreviva aunque después se borre el usuario o el registro.
--  * `before_json`/`after_json` guardan la foto completa de la fila antes y
--    después del cambio; `changes_json` solo los campos que cambiaron
--    ({campo: {from, to}}). Para borrados `after_json` es NULL y `before_json`
--    conserva todo lo que se eliminó.
--  * Solo inserción: los triggers rechazan UPDATE y DELETE, de modo que un
--    error de la aplicación no pueda reescribir el historial.
CREATE TABLE audit_log (
  id TEXT PRIMARY KEY,
  occurred_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
  actor_user_id TEXT,
  actor_name TEXT,
  actor_username TEXT,
  actor_role TEXT,
  action TEXT NOT NULL,
  entity_type TEXT NOT NULL,
  entity_id TEXT,
  summary TEXT NOT NULL,
  before_json TEXT,
  after_json TEXT,
  changes_json TEXT
);
CREATE INDEX audit_log_occurred_at_idx ON audit_log (occurred_at DESC);
CREATE INDEX audit_log_actor_idx ON audit_log (actor_user_id, occurred_at DESC);
CREATE INDEX audit_log_entity_idx ON audit_log (entity_type, entity_id);
CREATE INDEX audit_log_action_idx ON audit_log (action);

CREATE TRIGGER audit_log_no_update BEFORE UPDATE ON audit_log
BEGIN
  SELECT RAISE(ABORT, 'audit_log es de solo inserción');
END;

CREATE TRIGGER audit_log_no_delete BEFORE DELETE ON audit_log
BEGIN
  SELECT RAISE(ABORT, 'audit_log es de solo inserción');
END;
