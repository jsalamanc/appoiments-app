use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager, State};

use crate::app_db::AppDb;
use crate::error::{AppError, AppResult};
use crate::models::product::Product;
use crate::repositories::products as repo;
use crate::state::AppState;
use crate::util::catalog::CatalogCounts;
use crate::util::photos::import_and_sync_photo;

#[derive(Debug, Serialize)]
pub struct ListProductsResult {
    pub items: Vec<Product>,
    pub total: i64,
    pub page: i64,
    pub total_pages: i64,
    pub page_size: i64,
    pub status: String,
    pub counts: CatalogCounts,
}

#[tauri::command]
pub async fn list_products(
    state: State<'_, AppState>,
    status: Option<String>,
    page: Option<i64>,
    page_size: Option<i64>,
) -> AppResult<ListProductsResult> {
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
pub async fn get_product(state: State<'_, AppState>, id: String) -> AppResult<Option<Product>> {
    match &state.db {
        AppDb::Sqlite(_) => state.db.with_sqlite(move |conn| repo::get(conn, id)).await,
        AppDb::Postgres(pool) => repo::postgres::get(pool, id).await,
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProductInput {
    pub name: String,
    pub price: f64,
    pub photo_path: Option<String>,
    pub status: String,
}

#[tauri::command]
pub async fn create_product(
    state: State<'_, AppState>,
    input: ProductInput,
) -> AppResult<Product> {
    match &state.db {
        AppDb::Sqlite(_) => state.db.with_sqlite(move |conn| repo::create(conn, input)).await,
        AppDb::Postgres(pool) => repo::postgres::create(pool, input).await,
    }
}

#[tauri::command]
pub async fn update_product(
    state: State<'_, AppState>,
    id: String,
    input: ProductInput,
) -> AppResult<Product> {
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
pub async fn delete_product(state: State<'_, AppState>, id: String) -> AppResult<()> {
    match &state.db {
        AppDb::Sqlite(_) => state.db.with_sqlite(move |conn| repo::delete(conn, id)).await,
        AppDb::Postgres(pool) => repo::postgres::delete(pool, id).await,
    }
}

/// Replica la función Postgres `set_product_stock`: tipo de movimiento
/// `'initial'` si el producto nunca tuvo movimientos, si no `'adjustment'`;
/// no-op (no inserta movimiento) si el delta es 0. Corre en una transacción.
#[tauri::command]
pub async fn set_product_stock(
    state: State<'_, AppState>,
    product_id: String,
    stock: i64,
    note: Option<String>,
) -> AppResult<i64> {
    match &state.db {
        AppDb::Sqlite(_) => {
            state
                .db
                .with_sqlite(move |conn| repo::set_stock(conn, product_id, stock, note))
                .await
        }
        AppDb::Postgres(pool) => repo::postgres::set_stock(pool, product_id, stock, note).await,
    }
}

/// Copia la foto elegida por el usuario (file picker, ruta absoluta) al
/// directorio de datos de la app y devuelve la ruta relativa a guardar en
/// `photo_path` (el comando NO actualiza la fila; el frontend llama luego a
/// `update_product` con el valor devuelto). En modo "Base de datos propia"
/// también la sube al bucket configurado (ver `import_and_sync_photo`).
#[tauri::command]
pub async fn import_product_photo(
    app: AppHandle,
    state: State<'_, AppState>,
    product_id: String,
    source_file_path: String,
) -> AppResult<String> {
    match &state.db {
        AppDb::Sqlite(_) => {
            state
                .db
                .with_sqlite(move |conn| repo::assert_exists(conn, &product_id))
                .await?
        }
        AppDb::Postgres(pool) => repo::postgres::assert_exists(pool, &product_id).await?,
    }

    let data_dir = app.path().app_data_dir().map_err(|e| {
        AppError::Internal(format!("no se pudo resolver el directorio de datos: {e}"))
    })?;
    let config_dir = app.path().app_config_dir().map_err(|e| {
        AppError::Internal(format!("no se pudo resolver el directorio de configuración: {e}"))
    })?;

    import_and_sync_photo(&data_dir, &config_dir, &state.db, "products", &source_file_path).await
}
