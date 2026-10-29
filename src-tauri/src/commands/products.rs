use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager, State};

use crate::app_db::AppDb;
use crate::error::{AppError, AppResult};
use crate::models::product::Product;
use crate::repositories::products as repo;
use crate::state::AppState;
use crate::util::catalog::CatalogCounts;
use crate::util::photos::{cleanup_old_photo, import_and_sync_photo};

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
pub async fn get_product(state: State<'_, AppState>, id: String) -> AppResult<Option<Product>> {
    state.require_user()?;
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
    pub cost: f64,
    pub photo_path: Option<String>,
    pub status: String,
}

#[tauri::command]
pub async fn create_product(
    state: State<'_, AppState>,
    input: ProductInput,
) -> AppResult<Product> {
    state.require_user()?;
    match &state.db {
        AppDb::Sqlite(_) => state.db.with_sqlite(move |conn| repo::create(conn, input)).await,
        AppDb::Postgres(pool) => repo::postgres::create(pool, input).await,
    }
}

/// Si la foto cambia (o se quita), la vieja se borra de disco/bucket después
/// de que el `UPDATE` se confirma con éxito — nunca antes, para no dejar la
/// fila apuntando a una foto que ya no existe si el `UPDATE` llegara a
/// fallar (ver `cleanup_old_photo`).
#[tauri::command]
pub async fn update_product(
    app: AppHandle,
    state: State<'_, AppState>,
    id: String,
    input: ProductInput,
) -> AppResult<Product> {
    state.require_user()?;
    let old_photo_path = match &state.db {
        AppDb::Sqlite(_) => {
            let id = id.clone();
            state.db.with_sqlite(move |conn| repo::get(conn, id)).await?
        }
        AppDb::Postgres(pool) => repo::postgres::get(pool, id.clone()).await?,
    }
    .and_then(|p| p.photo_path);
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

/// Borra la foto de disco/bucket después de que la fila se borra con éxito
/// (ver `cleanup_old_photo`) — issue #18, para no dejar imágenes huérfanas.
#[tauri::command]
pub async fn delete_product(app: AppHandle, state: State<'_, AppState>, id: String) -> AppResult<()> {
    state.require_user()?;
    let old_photo_path = match &state.db {
        AppDb::Sqlite(_) => {
            let id = id.clone();
            state.db.with_sqlite(move |conn| repo::get(conn, id)).await?
        }
        AppDb::Postgres(pool) => repo::postgres::get(pool, id.clone()).await?,
    }
    .and_then(|p| p.photo_path);

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

/// Replica la función Postgres `set_product_stock`: tipo de movimiento
/// `'initial'` si el producto nunca tuvo movimientos, si no `'adjustment'`;
/// no-op (no inserta movimiento) si el delta es 0. Corre en una transacción.
///
/// `unit_cost`, si se manda, se guarda como el nuevo `products.cost` — solo
/// cuando la entrada aumenta el stock (una entrada de mercancía nueva), no
/// cuando lo reduce (un ajuste a la baja no informa nada sobre el costo de
/// reposición). Es "último costo pagado", no un promedio ponderado: más
/// simple de calcular y de entender que llevar un costo promedio móvil, a
/// costa de precisión si el costo varía mucho entre compras — razonable para
/// el alcance de esta fase (issue #2, Fase 5).
#[tauri::command]
pub async fn set_product_stock(
    state: State<'_, AppState>,
    product_id: String,
    stock: i64,
    note: Option<String>,
    unit_cost: Option<f64>,
) -> AppResult<i64> {
    state.require_user()?;
    match &state.db {
        AppDb::Sqlite(_) => {
            state
                .db
                .with_sqlite(move |conn| repo::set_stock(conn, product_id, stock, note, unit_cost))
                .await
        }
        AppDb::Postgres(pool) => {
            repo::postgres::set_stock(pool, product_id, stock, note, unit_cost).await
        }
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
    state.require_user()?;
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
