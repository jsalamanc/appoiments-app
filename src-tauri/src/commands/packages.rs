use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager, State};

use crate::app_db::AppDb;
use crate::error::{AppError, AppResult};
use crate::repositories::packages as repo;
use crate::state::AppState;
use crate::util::catalog::CatalogCounts;
use crate::util::photos::import_photo;

#[derive(Debug, Serialize)]
pub struct ProductRef {
    pub id: String,
    pub name: String,
    pub price: f64,
}

#[derive(Debug, Serialize)]
pub struct ServiceRef {
    pub id: String,
    pub name: String,
    pub price: f64,
    pub duration_minutes: i64,
}

#[derive(Debug, Serialize)]
pub struct PackageItemWithRefs {
    pub id: String,
    pub package_id: String,
    pub item_type: String,
    pub product_id: Option<String>,
    pub service_id: Option<String>,
    pub quantity: i64,
    pub created_at: String,
    pub products: Option<ProductRef>,
    pub services: Option<ServiceRef>,
}

#[derive(Debug, Serialize)]
pub struct PackageWithItems {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub price: f64,
    pub photo_path: Option<String>,
    pub status: String,
    pub created_at: String,
    pub updated_at: String,
    pub package_items: Vec<PackageItemWithRefs>,
}

#[derive(Debug, Serialize)]
pub struct ListPackagesResult {
    pub items: Vec<PackageWithItems>,
    pub total: i64,
    pub page: i64,
    pub total_pages: i64,
    pub page_size: i64,
    pub status: String,
    pub counts: CatalogCounts,
}

#[tauri::command]
pub async fn list_packages(
    state: State<'_, AppState>,
    status: Option<String>,
    page: Option<i64>,
    page_size: Option<i64>,
) -> AppResult<ListPackagesResult> {
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
pub async fn get_package(
    state: State<'_, AppState>,
    id: String,
) -> AppResult<Option<PackageWithItems>> {
    match &state.db {
        AppDb::Sqlite(_) => state.db.with_sqlite(move |conn| repo::get(conn, id)).await,
        AppDb::Postgres(pool) => repo::postgres::get(pool, id).await,
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PackageItemInput {
    pub item_type: String,
    pub item_id: String,
    pub quantity: i64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PackageInput {
    pub name: String,
    pub description: Option<String>,
    pub price: f64,
    pub photo_path: Option<String>,
    pub status: String,
    pub items: Vec<PackageItemInput>,
}

#[tauri::command]
pub async fn create_package(
    state: State<'_, AppState>,
    input: PackageInput,
) -> AppResult<PackageWithItems> {
    match &state.db {
        AppDb::Sqlite(_) => {
            state
                .db
                .with_sqlite(move |conn| repo::create(conn, input))
                .await
        }
        AppDb::Postgres(pool) => repo::postgres::create(pool, input).await,
    }
}

#[tauri::command]
pub async fn update_package(
    state: State<'_, AppState>,
    id: String,
    input: PackageInput,
) -> AppResult<PackageWithItems> {
    match &state.db {
        AppDb::Sqlite(_) => {
            state
                .db
                .with_sqlite(move |conn| repo::update(conn, id, input))
                .await
        }
        AppDb::Postgres(pool) => repo::postgres::update(pool, id, input).await,
    }
}

#[tauri::command]
pub async fn delete_package(state: State<'_, AppState>, id: String) -> AppResult<()> {
    match &state.db {
        AppDb::Sqlite(_) => state.db.with_sqlite(move |conn| repo::delete(conn, id)).await,
        AppDb::Postgres(pool) => repo::postgres::delete(pool, id).await,
    }
}

#[tauri::command]
pub async fn import_package_photo(
    app: AppHandle,
    state: State<'_, AppState>,
    package_id: String,
    source_file_path: String,
) -> AppResult<String> {
    match &state.db {
        AppDb::Sqlite(_) => {
            state
                .db
                .with_sqlite(move |conn| repo::assert_exists(conn, &package_id))
                .await?;
        }
        AppDb::Postgres(pool) => {
            repo::postgres::assert_exists(pool, &package_id).await?;
        }
    }

    let data_dir = app.path().app_data_dir().map_err(|e| {
        AppError::Internal(format!("no se pudo resolver el directorio de datos: {e}"))
    })?;

    import_photo(&data_dir, "packages", &source_file_path)
}
