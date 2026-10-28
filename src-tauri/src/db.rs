use std::path::Path;

use rusqlite::Connection;
use rusqlite_migration::{Migrations, M};

const MIGRATION_0001: &str = include_str!("../migrations/0001_init.sql");
const MIGRATION_0002: &str = include_str!("../migrations/0002_sales_v2.sql");

pub fn migrations() -> Migrations<'static> {
    Migrations::new(vec![M::up(MIGRATION_0001), M::up(MIGRATION_0002)])
}

pub fn open(data_dir: &Path) -> rusqlite::Result<Connection> {
    std::fs::create_dir_all(data_dir).expect("no se pudo crear el directorio de datos");
    let db_path = data_dir.join("aura-studio.db");

    let mut conn = Connection::open(db_path)?;
    conn.pragma_update(None, "foreign_keys", "ON")?;
    conn.pragma_update(None, "journal_mode", "WAL")?;

    migrations()
        .to_latest(&mut conn)
        .expect("fallo al aplicar migraciones de la base de datos local");

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
