use tauri::{AppHandle, Manager, State};

use crate::bucket_config::{self, BucketConfig};
use crate::error::{AppError, AppResult};
use crate::state::AppState;
use crate::util::bucket::BucketClient;
use crate::util::photos::{cleanup_old_photo, validate_photo_path};

fn data_dir(app: &AppHandle) -> AppResult<std::path::PathBuf> {
    app.path()
        .app_data_dir()
        .map_err(|e| AppError::Internal(format!("no se pudo resolver el directorio de datos: {e}")))
}

fn config_dir(app: &AppHandle) -> AppResult<std::path::PathBuf> {
    app.path()
        .app_config_dir()
        .map_err(|e| AppError::Internal(format!("no se pudo resolver el directorio de configuración: {e}")))
}

/// Configuración del bucket guardada, sin el secreto — para precargar el
/// formulario de Ajustes. `None` si nunca se configuró uno.
#[tauri::command]
pub async fn bucket_get_config(app: AppHandle, state: State<'_, AppState>) -> AppResult<Option<BucketConfig>> {
    state.require_owner()?;
    Ok(bucket_config::load_bucket(&config_dir(&app)?))
}

/// Prueba un bucket con los datos ingresados en Ajustes (sube, descarga y
/// borra un objeto pequeño), sin persistir nada — mismo espíritu que
/// `storage_test_connection`.
#[tauri::command]
pub async fn bucket_test_connection(
    state: State<'_, AppState>,
    config: BucketConfig,
    secret: String,
) -> AppResult<()> {
    state.require_owner()?;
    BucketClient::new(&config, &secret)?.test_connection().await
}

/// Garantiza que `photo_path` exista en disco local antes de que el frontend
/// la resuelva con `convertFileSrc` (ver `src/lib/photo-path.ts`).
///
/// En modo Local, el archivo ya está ahí por construcción (no-op). En modo
/// "Base de datos propia", el disco local es solo una caché: si falta (ej.
/// una segunda instalación apuntando al mismo Postgres/bucket compartido), se
/// descarga del bucket a esa misma ruta relativa antes de devolverla.
#[tauri::command]
pub async fn resolve_photo_cache(
    app: AppHandle,
    state: State<'_, AppState>,
    photo_path: String,
) -> AppResult<String> {
    state.require_user()?;
    validate_photo_path(&photo_path)?;
    let data_dir = data_dir(&app)?;
    let local_path = data_dir.join(&photo_path);
    if local_path.exists() {
        return Ok(photo_path);
    }

    let cfg = bucket_config::load_bucket(&config_dir(&app)?).ok_or_else(|| {
        AppError::Internal(
            "la imagen no está en disco y no hay un bucket configurado para descargarla".to_string(),
        )
    })?;
    let secret = bucket_config::get_bucket_secret(&cfg)
        .map_err(|e| AppError::Internal(format!("no se pudo leer la credencial del bucket: {e}")))?;
    let client = BucketClient::new(&cfg, &secret)?;
    let bytes = client.get_object(&cfg.object_key(&photo_path)).await?;

    if let Some(parent) = local_path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| AppError::Internal(format!("no se pudo crear el directorio de caché: {e}")))?;
    }
    std::fs::write(&local_path, bytes)
        .map_err(|e| AppError::Internal(format!("no se pudo escribir la imagen descargada: {e}")))?;

    Ok(photo_path)
}

/// Borra una foto recién importada (`import_*_photo`) que nunca llegó a
/// guardarse en ninguna fila — el caso en que el `update_*` posterior falla
/// (validación, red) y el frontend, en vez de quedarse con un huérfano,
/// descarta explícitamente lo que acababa de subir. Cierra el último hueco
/// del issue #18: ver `cleanup_old_photo` para el resto de los casos
/// (reemplazo y borrado de un registro ya guardado).
#[tauri::command]
pub async fn discard_photo(app: AppHandle, state: State<'_, AppState>, photo_path: String) -> AppResult<()> {
    state.require_user()?;
    validate_photo_path(&photo_path)?;
    cleanup_old_photo(&data_dir(&app)?, &config_dir(&app)?, &state.db, Some(&photo_path)).await;
    Ok(())
}
