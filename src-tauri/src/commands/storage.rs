use std::time::Duration;

use sqlx::postgres::PgPoolOptions;
use tauri::State;

use crate::error::{AppError, AppResult};
use crate::state::AppState;
use crate::storage_config::PostgresConnectionConfig;

const CONNECT_TIMEOUT: Duration = Duration::from_secs(5);

/// Prueba una conexión a PostgreSQL con los datos ingresados en Ajustes, sin
/// persistir nada (ni `storage.json` ni la contraseña en el keychain) y sin
/// correr migraciones. Se usa antes de `storage_switch_mode` (Fase 3) para
/// darle al usuario feedback inmediato de si el servidor es alcanzable y las
/// credenciales son válidas.
///
/// Solo el owner puede cambiar el modo de almacenamiento, igual que el resto
/// de Ajustes (`update_business_profile`, `update_enabled_modules`): se
/// revalida aquí y no solo se oculta en la UI.
#[tauri::command]
pub async fn storage_test_connection(
    state: State<'_, AppState>,
    config: PostgresConnectionConfig,
    password: String,
) -> AppResult<()> {
    state.require_owner()?;

    let pool = PgPoolOptions::new()
        .max_connections(1)
        .acquire_timeout(CONNECT_TIMEOUT)
        .connect_with(config.connect_options(&password))
        .await
        .map_err(|e| AppError::validation(format!("No se pudo conectar: {e}")))?;

    let result = sqlx::query("SELECT 1")
        .execute(&pool)
        .await
        .map_err(|e| AppError::validation(format!("Conexión establecida pero la prueba falló: {e}")));

    pool.close().await;
    result.map(|_| ())
}
