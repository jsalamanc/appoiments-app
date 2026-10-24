use rusqlite::{params, Connection};

use crate::commands::inventory::{ListInventoryMovementsResult, LowStockResult, LOW_STOCK_THRESHOLD};
use crate::error::AppResult;
use crate::models::inventory::InventoryMovement;
use crate::models::product::Product;
use crate::util::pagination::{get_range, get_total_pages, CATALOG_PAGE_SIZE};

pub fn list_low_stock(conn: &mut Connection, threshold: Option<i64>) -> AppResult<LowStockResult> {
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

pub fn list_movements(
    conn: &mut Connection,
    product_id: String,
    page: Option<i64>,
    page_size: Option<i64>,
) -> AppResult<ListInventoryMovementsResult> {
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
