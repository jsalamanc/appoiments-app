use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager, State};

use crate::app_db::AppDb;
use crate::error::{AppError, AppResult};
use crate::models::service::Service;
use crate::repositories::services as repo;
use crate::state::AppState;
use crate::util::catalog::CatalogCounts;
use crate::util::photos::{cleanup_old_photo, import_and_sync_photo};

#[derive(Debug, Serialize)]
pub struct ListServicesResult {
    pub items: Vec<Service>,
    pub total: i64,
    pub page: i64,
    pub total_pages: i64,
    pub page_size: i64,
    pub status: String,
    pub counts: CatalogCounts,
}

#[tauri::command]
pub async fn list_services(
    state: State<'_, AppState>,
    status: Option<String>,
    page: Option<i64>,
    page_size: Option<i64>,
) -> AppResult<ListServicesResult> {
    state.require_user()?;
    match &state.db {
        AppDb::Sqlite(_) => {
            state
                .db
                .with_sqlite(move |conn| repo::list(conn, status, page, page_size))
                .await
        }
        AppDb::Postgres(pool) => repo::postgres::list(pool, status, page, page_size).await,
    }
}

#[tauri::command]
pub async fn get_service(state: State<'_, AppState>, id: String) -> AppResult<Option<Service>> {
    state.require_user()?;
    match &state.db {
        AppDb::Sqlite(_) => state.db.with_sqlite(move |conn| repo::get(conn, id)).await,
        AppDb::Postgres(pool) => repo::postgres::get(pool, id).await,
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ServiceInput {
    pub name: String,
    pub description: Option<String>,
    pub price: i64,
    pub duration_minutes: i64,
    pub photo_path: Option<String>,
    pub status: String,
}

#[tauri::command]
pub async fn create_service(
    state: State<'_, AppState>,
    input: ServiceInput,
) -> AppResult<Service> {
    state.require_user()?;
    match &state.db {
        AppDb::Sqlite(_) => state.db.with_sqlite(move |conn| repo::create(conn, input)).await,
        AppDb::Postgres(pool) => repo::postgres::create(pool, input).await,
    }
}

/// Ver `products::update_product`: la foto vieja se borra solo después de
/// confirmar el `UPDATE`, y solo si de verdad cambió.
#[tauri::command]
pub async fn update_service(
    app: AppHandle,
    state: State<'_, AppState>,
    id: String,
    input: ServiceInput,
) -> AppResult<Service> {
    state.require_user()?;
    let old_photo_path = match &state.db {
        AppDb::Sqlite(_) => {
            let id = id.clone();
            state.db.with_sqlite(move |conn| repo::get(conn, id)).await?
        }
        AppDb::Postgres(pool) => repo::postgres::get(pool, id.clone()).await?,
    }
    .and_then(|s| s.photo_path);
    let new_photo_path = input.photo_path.clone();

    let updated = match &state.db {
        AppDb::Sqlite(_) => {
            state
                .db
                .with_sqlite(move |conn| repo::update(conn, id, input))
                .await
        }
        AppDb::Postgres(pool) => repo::postgres::update(pool, id, input).await,
    }?;

    if old_photo_path.is_some() && old_photo_path != new_photo_path {
        if let (Ok(data_dir), Ok(config_dir)) =
            (app.path().app_data_dir(), app.path().app_config_dir())
        {
            cleanup_old_photo(&data_dir, &config_dir, &state.db, old_photo_path.as_deref()).await;
        }
    }

    Ok(updated)
}

/// Ver `products::delete_product`: la foto se borra de disco/bucket después
/// de confirmar el `DELETE` (issue #18).
#[tauri::command]
pub async fn delete_service(app: AppHandle, state: State<'_, AppState>, id: String) -> AppResult<()> {
    state.require_user()?;
    let old_photo_path = match &state.db {
        AppDb::Sqlite(_) => {
            let id = id.clone();
            state.db.with_sqlite(move |conn| repo::get(conn, id)).await?
        }
        AppDb::Postgres(pool) => repo::postgres::get(pool, id.clone()).await?,
    }
    .and_then(|s| s.photo_path);

    match &state.db {
        AppDb::Sqlite(_) => state.db.with_sqlite(move |conn| repo::delete(conn, id)).await,
        AppDb::Postgres(pool) => repo::postgres::delete(pool, id).await,
    }?;

    if let (Ok(data_dir), Ok(config_dir)) = (app.path().app_data_dir(), app.path().app_config_dir())
    {
        cleanup_old_photo(&data_dir, &config_dir, &state.db, old_photo_path.as_deref()).await;
    }

    Ok(())
}

#[tauri::command]
pub async fn import_service_photo(
    app: AppHandle,
    state: State<'_, AppState>,
    service_id: String,
    source_file_path: String,
) -> AppResult<String> {
    state.require_user()?;
    match &state.db {
        AppDb::Sqlite(_) => {
            state
                .db
                .with_sqlite({
                    let service_id = service_id.clone();
                    move |conn| repo::assert_exists(conn, &service_id)
                })
                .await
        }
        AppDb::Postgres(pool) => repo::postgres::assert_exists(pool, &service_id).await,
    }?;

    let data_dir = app.path().app_data_dir().map_err(|e| {
        AppError::Internal(format!("no se pudo resolver el directorio de datos: {e}"))
    })?;
    let config_dir = app.path().app_config_dir().map_err(|e| {
        AppError::Internal(format!("no se pudo resolver el directorio de configuración: {e}"))
    })?;

    import_and_sync_photo(&data_dir, &config_dir, &state.db, "services", &source_file_path).await
}
