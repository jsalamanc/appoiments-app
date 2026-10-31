use std::path::Path;

use rusqlite::Connection;
use rusqlite_migration::{Migrations, M};

const MIGRATION_0001: &str = include_str!("../migrations/0001_init.sql");
const MIGRATION_0002: &str = include_str!("../migrations/0002_sales_v2.sql");
const MIGRATION_0003: &str = include_str!("../migrations/0003_business_hours.sql");
const MIGRATION_0004: &str = include_str!("../migrations/0004_commissions.sql");
const MIGRATION_0005: &str =
    include_str!("../migrations/0005_customer_flexibility_and_product_cost.sql");
const MIGRATION_0006: &str = include_str!("../migrations/0006_cash_sessions.sql");
const MIGRATION_0007: &str = include_str!("../migrations/0007_money_as_integers.sql");
const MIGRATION_0008: &str = include_str!("../migrations/0008_customers_search.sql");

pub fn migrations() -> Migrations<'static> {
    Migrations::new(vec![
        M::up(MIGRATION_0001),
        M::up(MIGRATION_0002),
        M::up(MIGRATION_0003),
        M::up(MIGRATION_0004),
        M::up(MIGRATION_0005),
        M::up(MIGRATION_0006),
        M::up(MIGRATION_0007),
        M::up(MIGRATION_0008),
    ])
}

pub fn open(data_dir: &Path) -> rusqlite::Result<Connection> {
    std::fs::create_dir_all(data_dir).expect("no se pudo crear el directorio de datos");
    let db_path = data_dir.join("aura-studio.db");

    let mut conn = Connection::open(db_path)?;
    conn.pragma_update(None, "journal_mode", "WAL")?;

    // `foreign_keys` debe estar OFF mientras corren las migraciones: el
    // procedimiento de 12 pasos que varias migraciones usan para recrear una
    // tabla (SQLite no soporta ALTER de un CHECK/NOT NULL existente) hace un
    // DROP TABLE de la tabla vieja, y con foreign_keys=ON SQLite rechaza ese
    // DROP si alguna fila de otra tabla todavía la referencia con una FK
    // `ON DELETE RESTRICT` — es el primer paso del procedimiento oficial de
    // SQLite (https://sqlite.org/lang_altertable.html#otheralter) y no es
    // opcional: toggling el pragma *dentro* de una migración no sirve de
    // nada, porque rusqlite_migration ya corre cada migración en su propia
    // transacción y SQLite ignora cambios a `foreign_keys` con una
    // transacción abierta. Esto pasó desapercibido hasta ahora porque
    // ninguna migración anterior recreaba una tabla que fuera el padre de
    // una FK `ON DELETE RESTRICT` con filas reales apuntándole — la
    // migración 0005 (recrea `customers`, referenciada por
    // `appointments.customer_id`/`sales.customer_id` con `ON DELETE
    // RESTRICT`) fue la primera, y cualquier base de datos real con al
    // menos un cliente con una cita o venta no podía actualizarse: la app
    // se cerraba sola al abrir porque `to_latest()` fallaba y el
    // `.expect()` de abajo entraba en pánico antes de crear la ventana.
    conn.pragma_update(None, "foreign_keys", "OFF")?;
    migrations()
        .to_latest(&mut conn)
        .expect("fallo al aplicar migraciones de la base de datos local");
    conn.pragma_update(None, "foreign_keys", "ON")?;

    Ok(conn)
}

pub fn now_iso() -> String {
    time::OffsetDateTime::now_utc()
        .format(&time::format_description::well_known::Rfc3339)
        .unwrap_or_default()
}

/// Formatea un `TIMESTAMPTZ` leído de PostgreSQL con el mismo formato RFC3339
/// que usa `now_iso()` para SQLite, para que los modelos (`created_at`,
/// `updated_at`, etc., tipados como `String`) se vean idénticos sin importar
/// el backend de almacenamiento activo.
pub fn format_pg_timestamp(value: time::OffsetDateTime) -> String {
    value
        .format(&time::format_description::well_known::Rfc3339)
        .unwrap_or_default()
}

pub fn new_id() -> String {
    uuid::Uuid::new_v4().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Reproduce un v0.4.0 real (solo migración 0001, aplicada vía
    /// `rusqlite_migration` para que el versionado quede igual que en la app
    /// de verdad) con datos típicos: un cliente con una cita completada
    /// vinculada a una venta, y otro cliente con una venta de producto
    /// suelta. Antes del fix, `to_latest()` fallaba con "FOREIGN KEY
    /// constraint failed" al llegar a la migración 0005 (recrea `customers`,
    /// que `appointments.customer_id`/`sales.customer_id` referencian con
    /// `ON DELETE RESTRICT`) en cualquier base de datos real con al menos un
    /// cliente así — exactamente el caso de cualquier instalación que ya
    /// hubiera usado la app antes de actualizar.
    #[test]
    fn upgrading_a_real_v040_database_does_not_fail() {
        let mut conn = Connection::open_in_memory().unwrap();
        conn.pragma_update(None, "foreign_keys", "ON").unwrap();

        Migrations::new(vec![M::up(MIGRATION_0001)])
            .to_latest(&mut conn)
            .unwrap();

        conn.execute_batch(
            "INSERT INTO business_profile (id, name, module_employees, module_appointments, module_services, module_packages, module_reports)
             VALUES (1, 'Salon Real', 1, 1, 1, 1, 1);
             INSERT INTO users (id, name, username, password_hash, role, active)
             VALUES ('user-1', 'Dueña', 'owner', 'hash', 'owner', 1);
             INSERT INTO customers (id, name, phone, email, document)
             VALUES ('cust-1', 'Cliente Uno', '3000000001', 'uno@test.com', 'DOC-1'),
                    ('cust-2', 'Cliente Dos', '3000000002', 'dos@test.com', 'DOC-2');
             INSERT INTO employees (id, name, color, active) VALUES ('emp-1', 'Ana', '#fff', 1);
             INSERT INTO services (id, name, price, duration_minutes, status)
             VALUES ('svc-1', 'Corte', 35000.0, 45, 'published');
             INSERT INTO products (id, name, price, status, stock)
             VALUES ('prod-1', 'Shampoo', 45000.0, 'published', 12);
             INSERT INTO appointments
                (id, employee_id, customer_id, service_id, service_name_snapshot, starts_at, ends_at, status, sale_id)
             VALUES ('appt-1', 'emp-1', 'cust-1', 'svc-1', 'Corte', '2026-01-10T10:00:00Z', '2026-01-10T10:45:00Z', 'completed', 'sale-1');
             INSERT INTO sales (id, customer_id, appointment_id, payment_method, total)
             VALUES ('sale-1', 'cust-1', 'appt-1', 'cash', 35000.0);
             INSERT INTO sale_items (id, sale_id, item_type, service_id, name_snapshot, unit_price, quantity, line_total)
             VALUES ('saleitem-1', 'sale-1', 'service', 'svc-1', 'Corte', 35000.0, 1, 35000.0);
             INSERT INTO sales (id, customer_id, appointment_id, payment_method, total)
             VALUES ('sale-2', 'cust-2', NULL, 'transfer', 90000.0);",
        )
        .unwrap();

        // Lo mismo que hace `open()`: foreign_keys OFF alrededor de
        // `to_latest()`, ON después.
        conn.pragma_update(None, "foreign_keys", "OFF").unwrap();
        migrations().to_latest(&mut conn).unwrap();
        conn.pragma_update(None, "foreign_keys", "ON").unwrap();

        let sales_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM sales", [], |r| r.get(0))
            .unwrap();
        assert_eq!(sales_count, 2);
        let total_sale1: i64 = conn
            .query_row("SELECT total FROM sales WHERE id = 'sale-1'", [], |r| r.get(0))
            .unwrap();
        assert_eq!(total_sale1, 35000, "el dinero debe sobrevivir la conversión a entero");
    }
}
