use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager, State};

use crate::error::{AppError, AppResult};
use crate::models::service::Service;
use crate::repositories::services as repo;
use crate::state::AppState;
use crate::util::catalog::CatalogCounts;
use crate::util::photos::import_photo;

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
    state
        .db
        .with_sqlite(move |conn| repo::list(conn, status, page, page_size))
        .await
}

#[tauri::command]
pub async fn get_service(state: State<'_, AppState>, id: String) -> AppResult<Option<Service>> {
    state.db.with_sqlite(move |conn| repo::get(conn, id)).await
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ServiceInput {
    pub name: String,
    pub description: Option<String>,
    pub price: f64,
    pub duration_minutes: i64,
    pub photo_path: Option<String>,
    pub status: String,
}

#[tauri::command]
pub async fn create_service(
    state: State<'_, AppState>,
    input: ServiceInput,
) -> AppResult<Service> {
    state.db.with_sqlite(move |conn| repo::create(conn, input)).await
}

#[tauri::command]
pub async fn update_service(
    state: State<'_, AppState>,
    id: String,
    input: ServiceInput,
) -> AppResult<Service> {
    state
        .db
        .with_sqlite(move |conn| repo::update(conn, id, input))
        .await
}

#[tauri::command]
pub async fn delete_service(state: State<'_, AppState>, id: String) -> AppResult<()> {
    state.db.with_sqlite(move |conn| repo::delete(conn, id)).await
}

#[tauri::command]
pub async fn import_service_photo(
    app: AppHandle,
    state: State<'_, AppState>,
    service_id: String,
    source_file_path: String,
) -> AppResult<String> {
    state
        .db
        .with_sqlite(move |conn| repo::assert_exists(conn, &service_id))
        .await?;

    let data_dir = app.path().app_data_dir().map_err(|e| {
        AppError::Internal(format!("no se pudo resolver el directorio de datos: {e}"))
    })?;

    import_photo(&data_dir, "services", &source_file_path)
}
