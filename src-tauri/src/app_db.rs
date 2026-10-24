//! La conexión de datos activa de la aplicación: SQLite local (hoy) o
//! PostgreSQL externo (self-hosted), según el modo elegido en
//! Ajustes → Almacenamiento de datos (ver [`crate::storage_config`]).
//!
//! Desde la Fase 1 del issue #9, `AppState.db` es un `AppDb` en vez de un
//! `Mutex<Connection>` crudo. El modo local sigue siendo igual de SQLite: el
//! único cambio es que los comandos acceden a la conexión a través de
//! [`AppDb::with_sqlite`] en vez de bloquear un mutex directamente, para que
//! la capa de `repositories/` pueda despachar a PostgreSQL en la Fase 2 sin
//! volver a tocar cada comando.

use std::sync::{Arc, Mutex};

use sqlx::migrate::Migrator;
use sqlx::postgres::{PgPool, PgPoolOptions};

use crate::error::{AppError, AppResult};
use crate::storage_config::PostgresConnectionConfig;

pub enum AppDb {
    Sqlite(Arc<Mutex<rusqlite::Connection>>),
    Postgres(PgPool),
}

impl AppDb {
    /// Ejecuta `f` contra la conexión SQLite local en un hilo bloqueante
    /// (`rusqlite` es síncrono), y devuelve su resultado ya en el `async fn`
    /// del comando que llama. Pensado para repositorios cuya única
    /// implementación hoy es SQLite; en la Fase 2, los repositorios con
    /// soporte PostgreSQL hacen su propio `match` sobre `AppDb` en vez de
    /// usar este helper.
    pub async fn with_sqlite<F, T>(&self, f: F) -> AppResult<T>
    where
        F: FnOnce(&mut rusqlite::Connection) -> AppResult<T> + Send + 'static,
        T: Send + 'static,
    {
        let AppDb::Sqlite(conn) = self else {
            return Err(AppError::Internal(
                "operación de SQLite invocada fuera del modo de almacenamiento local".into(),
            ));
        };
        let conn = conn.clone();
        tauri::async_runtime::spawn_blocking(move || {
            let mut guard = conn.lock().unwrap();
            f(&mut guard)
        })
        .await
        .map_err(|e| AppError::Internal(format!("tarea de base de datos cancelada: {e}")))?
    }
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
