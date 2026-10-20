use rusqlite::params;
use serde::Serialize;
use tauri::State;

use crate::error::AppResult;
use crate::models::inventory::InventoryMovement;
use crate::models::product::Product;
use crate::state::AppState;
use crate::util::pagination::{get_range, get_total_pages, CATALOG_PAGE_SIZE};

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
pub fn list_low_stock_products(
    state: State<AppState>,
    threshold: Option<i64>,
) -> AppResult<LowStockResult> {
    let conn = state.db.lock().unwrap();
    let threshold = threshold.unwrap_or(LOW_STOCK_THRESHOLD);

    let mut stmt =
        conn.prepare("SELECT * FROM products WHERE stock <= ?1 ORDER BY stock ASC, name ASC")?;
    let products = stmt
        .query_map(params![threshold], Product::from_row)?
        .collect::<Result<Vec<_>, _>>()?;

    let out_of_stock: Vec<Product> = products.iter().filter(|p| p.stock <= 0).cloned().collect();
    let low_stock: Vec<Product> = products
        .iter()
        .filter(|p| p.stock > 0 && p.stock <= threshold)
        .cloned()
        .collect();

    Ok(LowStockResult {
        products,
        out_of_stock,
        low_stock,
        threshold,
    })
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
pub fn list_inventory_movements(
    state: State<AppState>,
    product_id: String,
    page: Option<i64>,
    page_size: Option<i64>,
) -> AppResult<ListInventoryMovementsResult> {
    let conn = state.db.lock().unwrap();
    let page = page.unwrap_or(1).max(1);
    let page_size = page_size.unwrap_or(CATALOG_PAGE_SIZE);

    let total: i64 = conn.query_row(
        "SELECT COUNT(*) FROM inventory_movements WHERE product_id = ?1",
        params![product_id],
        |r| r.get(0),
    )?;
    let total_pages = get_total_pages(total, page_size);
    let safe_page = page.min(total_pages);
    let (limit, offset) = get_range(safe_page, page_size);

    let mut stmt = conn.prepare(
        "SELECT * FROM inventory_movements WHERE product_id = ?1
         ORDER BY created_at DESC LIMIT ?2 OFFSET ?3",
    )?;
    let items = stmt
        .query_map(params![product_id, limit, offset], InventoryMovement::from_row)?
        .collect::<Result<Vec<_>, _>>()?;

    Ok(ListInventoryMovementsResult {
        items,
        total,
        page: safe_page,
        total_pages,
        page_size,
    })
}
