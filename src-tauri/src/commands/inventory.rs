use serde::Serialize;
use tauri::State;

use crate::app_db::AppDb;
use crate::error::AppResult;
use crate::models::inventory::InventoryMovement;
use crate::models::product::Product;
use crate::repositories::inventory as repo;
use crate::state::AppState;

/// Productos con stock menor o igual se consideran "stock bajo" (puerto de
/// `LOW_STOCK_THRESHOLD` en `src/lib/inventory.ts`).
pub const LOW_STOCK_THRESHOLD: i64 = 5;

#[derive(Debug, Serialize)]
pub struct LowStockResult {
    pub products: Vec<Product>,
    pub out_of_stock: Vec<Product>,
    pub low_stock: Vec<Product>,
    pub threshold: i64,
}

#[tauri::command]
pub async fn list_low_stock_products(
    state: State<'_, AppState>,
    threshold: Option<i64>,
) -> AppResult<LowStockResult> {
    state.require_user()?;
    match &state.db {
        AppDb::Sqlite(_) => {
            state
                .db
                .with_sqlite(move |conn| repo::list_low_stock(conn, threshold))
                .await
        }
        AppDb::Postgres(pool) => repo::postgres::list_low_stock(pool, threshold).await,
    }
}

#[derive(Debug, Serialize)]
pub struct ListInventoryMovementsResult {
    pub items: Vec<InventoryMovement>,
    pub total: i64,
    pub page: i64,
    pub total_pages: i64,
    pub page_size: i64,
}

#[tauri::command]
pub async fn list_inventory_movements(
    state: State<'_, AppState>,
    product_id: String,
    page: Option<i64>,
    page_size: Option<i64>,
) -> AppResult<ListInventoryMovementsResult> {
    state.require_user()?;
    match &state.db {
        AppDb::Sqlite(_) => {
            state
                .db
                .with_sqlite(move |conn| repo::list_movements(conn, product_id, page, page_size))
                .await
        }
        AppDb::Postgres(pool) => repo::postgres::list_movements(pool, product_id, page, page_size).await,
    }
}
