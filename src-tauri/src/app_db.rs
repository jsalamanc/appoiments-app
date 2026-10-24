//! La conexión de datos activa de la aplicación: SQLite local (hoy) o
//! PostgreSQL externo (self-hosted), según el modo elegido en
//! Ajustes → Almacenamiento de datos (ver [`crate::storage_config`]).
//!
//! Introducido en la Fase 0 del issue #9 como preparación. Todavía no está
//! conectado a `AppState` ni a `lib.rs::setup` — esa es la Fase 1, que
//! reemplaza `AppState.db: Mutex<Connection>` por `AppDb::Sqlite(...)` sin
//! cambiar el comportamiento del modo local, y recién en la Fase 2 se arma el
//! `AppDb::Postgres(...)` a partir de lo que ya queda definido aquí.

use std::sync::Mutex;

use sqlx::migrate::Migrator;
use sqlx::postgres::{PgPool, PgPoolOptions};

use crate::storage_config::PostgresConnectionConfig;

pub enum AppDb {
    Sqlite(Mutex<rusqlite::Connection>),
    Postgres(PgPool),
}

/// Migraciones embebidas de `migrations/postgres/`, espejo en dialecto
/// PostgreSQL de `migrations/0001_init.sql` (ver ese directorio para el
/// detalle de las decisiones de mapeo de tipos).
static POSTGRES_MIGRATOR: Migrator = sqlx::migrate!("migrations/postgres");

/// Abre un pool de conexiones a PostgreSQL y corre las migraciones pendientes
/// contra él. Usado por `storage_test_connection`/`storage_switch_mode`
/// (Fase 3) para validar y preparar una base externa antes de adoptarla.
pub async fn open_postgres(
    config: &PostgresConnectionConfig,
    password: &str,
) -> Result<PgPool, sqlx::Error> {
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect_with(config.connect_options(password))
        .await?;

    POSTGRES_MIGRATOR.run(&pool).await?;

    Ok(pool)
}
