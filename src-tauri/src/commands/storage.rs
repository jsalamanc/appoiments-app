use std::time::Duration;

use sqlx::postgres::PgPoolOptions;
use tauri::{AppHandle, Manager, State};

use crate::app_db;
use crate::app_db::AppDb;
use crate::error::{AppError, AppResult};
use crate::export;
use crate::migration::{self, MigrationSummary};
use crate::state::AppState;
use crate::storage_config::{self, PostgresConnectionConfig, StorageMode};

const CONNECT_TIMEOUT: Duration = Duration::from_secs(5);

fn config_dir(app: &AppHandle) -> AppResult<std::path::PathBuf> {
    app.path()
        .app_config_dir()
        .map_err(|e| AppError::Internal(format!("no se pudo resolver el directorio de configuración: {e}")))
}

/// Modo de almacenamiento activo, para que Ajustes sepa qué mostrar
/// seleccionado al cargar la página. No incluye la contraseña (vive aparte,
/// en el keychain del SO) — `StorageMode` ya está diseñado para serializarse
/// de forma segura.
#[tauri::command]
pub async fn storage_get_mode(app: AppHandle, state: State<'_, AppState>) -> AppResult<StorageMode> {
    state.require_owner()?;
    Ok(storage_config::load(&config_dir(&app)?))
}

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
    ca_cert_path: Option<String>,
) -> AppResult<()> {
    state.require_owner()?;

    let ca_cert_path = ca_cert_path.map(std::path::PathBuf::from);
    let pool = PgPoolOptions::new()
        .max_connections(1)
        .acquire_timeout(CONNECT_TIMEOUT)
        .connect_with(config.connect_options(&password, ca_cert_path.as_deref()))
        .await
        .map_err(|e| AppError::validation(format!("No se pudo conectar: {e}")))?;

    let result = sqlx::query("SELECT 1")
        .execute(&pool)
        .await
        .map_err(|e| AppError::validation(format!("Conexión establecida pero la prueba falló: {e}")));

    pool.close().await;
    result.map(|_| ())
}

/// Cambia el modo de almacenamiento activo. Para `postgres`, se conecta y se
/// corren las migraciones de esquema contra el destino, y opcionalmente se
/// migran los datos locales (issue #11) — todo ANTES de persistir nada — si
/// algo falla, `storage.json` queda exactamente como estaba (no hace falta un
/// "revert" explícito, el orden de las operaciones ya lo garantiza). El
/// frontend es responsable de reiniciar la app (`relaunch()` del plugin
/// `process`) después de un cambio exitoso: `AppState` ya quedó construido
/// contra el backend anterior y no se puede reemplazar en caliente.
#[tauri::command]
pub async fn storage_switch_mode(
    app: AppHandle,
    state: State<'_, AppState>,
    mode: String,
    config: Option<PostgresConnectionConfig>,
    password: Option<String>,
    ca_cert_path: Option<String>,
    migrate_data: Option<bool>,
) -> AppResult<Option<MigrationSummary>> {
    state.require_owner()?;
    let dir = config_dir(&app)?;

    match mode.as_str() {
        "local" => {
            storage_config::save(&dir, &StorageMode::Local)
                .map_err(|e| AppError::Internal(format!("no se pudo guardar la configuración: {e}")))?;
            // Sin certificado CA en modo local: evita que una reconfiguración
            // futura a Postgres herede en silencio un certificado viejo.
            storage_config::delete_ca_cert(&dir)
                .map_err(|e| AppError::Internal(format!("no se pudo limpiar el certificado CA: {e}")))?;
            Ok(None)
        }
        "postgres" => {
            let config = config.ok_or_else(|| AppError::validation("Faltan los datos de conexión"))?;
            let password = password.ok_or_else(|| AppError::validation("Falta la contraseña"))?;

            // El certificado se copia a la carpeta de configuración ANTES de
            // probar la conexión, para validar contra la copia que de verdad
            // va a usar la app en cada arranque (no el archivo original, que
            // podría moverse o borrarse después).
            let saved_ca_cert = match &ca_cert_path {
                Some(source) => Some(
                    storage_config::save_ca_cert(&dir, std::path::Path::new(source)).map_err(|e| {
                        AppError::validation(format!("No se pudo leer el certificado CA: {e}"))
                    })?,
                ),
                None => {
                    storage_config::delete_ca_cert(&dir).map_err(|e| {
                        AppError::Internal(format!("no se pudo limpiar el certificado CA: {e}"))
                    })?;
                    None
                }
            };

            let pg_pool = app_db::open_postgres(&config, &password, saved_ca_cert.as_deref())
                .await
                .map_err(|e| AppError::validation(format!("No se pudo preparar la base de datos: {e}")))?;

            // Guard de seguridad del lado del servidor, independiente de lo
            // que mande la UI: la migración automática solo tiene sentido
            // (y solo se ofreció) yendo de Local hacia Postgres.
            let should_migrate = migrate_data.unwrap_or(false) && matches!(&state.db, AppDb::Sqlite(_));
            let migration_summary = if should_migrate {
                let summary = migration::migrate_local_to_postgres(&state.db, &pg_pool)
                    .await
                    .map_err(|e| AppError::validation(format!("No se pudo migrar los datos locales: {e}")))?;
                Some(summary)
            } else {
                None
            };

            storage_config::set_password(&config, &password)
                .map_err(|e| AppError::Internal(format!("no se pudo guardar la contraseña: {e}")))?;
            storage_config::save(&dir, &StorageMode::Postgres(config))
                .map_err(|e| AppError::Internal(format!("no se pudo guardar la configuración: {e}")))?;

            Ok(migration_summary)
        }
        _ => Err(AppError::validation("Modo de almacenamiento inválido")),
    }
}

/// Descarga un respaldo JSON de los datos locales (issue #9, Fase 4). Solo
/// disponible en modo Local — el modo Postgres es responsabilidad del propio
/// servidor del usuario. No incluye `sessions` ni `password_hash` (ver
/// `crate::export`). `path` es elegido por el usuario en un diálogo nativo
/// de "Guardar como" en el frontend, no una ruta arbitraria del backend.
#[tauri::command]
pub async fn storage_export_local_data(state: State<'_, AppState>, path: String) -> AppResult<()> {
    state.require_owner()?;

    match &state.db {
        AppDb::Sqlite(_) => {
            let json = state.db.with_sqlite(export::export_all).await?;
            std::fs::write(&path, json)
                .map_err(|e| AppError::Internal(format!("no se pudo escribir el archivo: {e}")))
        }
        AppDb::Postgres(_) => Err(AppError::validation(
            "El respaldo a JSON solo está disponible en modo local",
        )),
    }
}
