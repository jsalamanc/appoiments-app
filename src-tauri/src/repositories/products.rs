use rusqlite::{params, Connection};

use crate::commands::products::{ListProductsResult, ProductInput};
use crate::db::{new_id, now_iso};
use crate::error::{AppError, AppResult};
use crate::models::product::Product;
use crate::util::catalog::{normalize_status, pick_count, status_counts};
use crate::util::pagination::{get_range, get_total_pages, CATALOG_PAGE_SIZE};

pub fn list(
    conn: &mut Connection,
    status: Option<String>,
    page: Option<i64>,
    page_size: Option<i64>,
) -> AppResult<ListProductsResult> {
    let status = normalize_status(status);
    let page = page.unwrap_or(1).max(1);
    let page_size = page_size.unwrap_or(CATALOG_PAGE_SIZE);

    let counts = status_counts(conn, "products")?;
    let total = pick_count(&counts, &status);
    let total_pages = get_total_pages(total, page_size);
    let safe_page = page.min(total_pages);
    let (limit, offset) = get_range(safe_page, page_size);

    let items = if status == "all" {
        let mut stmt =
            conn.prepare("SELECT * FROM products ORDER BY created_at DESC LIMIT ?1 OFFSET ?2")?;
        let rows = stmt.query_map(params![limit, offset], Product::from_row)?;
        rows.collect::<Result<Vec<_>, _>>()?
    } else {
        let mut stmt = conn.prepare(
            "SELECT * FROM products WHERE status = ?1 ORDER BY created_at DESC LIMIT ?2 OFFSET ?3",
        )?;
        let rows = stmt.query_map(params![status, limit, offset], Product::from_row)?;
        rows.collect::<Result<Vec<_>, _>>()?
    };

    Ok(ListProductsResult {
        items,
        total,
        page: safe_page,
        total_pages,
        page_size,
        status,
        counts,
    })
}

pub fn get(conn: &mut Connection, id: String) -> AppResult<Option<Product>> {
    let result = conn.query_row(
        "SELECT * FROM products WHERE id = ?1",
        params![id],
        Product::from_row,
    );
    match result {
        Ok(p) => Ok(Some(p)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(e.into()),
    }
}

pub fn validate_input(input: &ProductInput) -> AppResult<()> {
    if input.name.trim().is_empty() {
        return Err(AppError::validation("El nombre es obligatorio"));
    }
    if input.price < 0.0 {
        return Err(AppError::validation("El precio no puede ser negativo"));
    }
    if input.status != "draft" && input.status != "published" {
        return Err(AppError::validation("Estado inválido"));
    }
    Ok(())
}

pub fn create(conn: &mut Connection, input: ProductInput) -> AppResult<Product> {
    validate_input(&input)?;

    let id = new_id();
    let now = now_iso();

    conn.execute(
        "INSERT INTO products (id, name, price, photo_path, status, stock, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, 0, ?6, ?6)",
        params![
            id,
            input.name.trim(),
            input.price,
            input.photo_path,
            input.status,
            now
        ],
    )?;

    let product = conn.query_row(
        "SELECT * FROM products WHERE id = ?1",
        params![id],
        Product::from_row,
    )?;
    Ok(product)
}

pub fn update(conn: &mut Connection, id: String, input: ProductInput) -> AppResult<Product> {
    validate_input(&input)?;

    conn.execute(
        "UPDATE products SET name = ?1, price = ?2, photo_path = ?3, status = ?4, updated_at = ?5
         WHERE id = ?6",
        params![
            input.name.trim(),
            input.price,
            input.photo_path,
            input.status,
            now_iso(),
            id
        ],
    )?;

    let product = conn.query_row(
        "SELECT * FROM products WHERE id = ?1",
        params![id],
        Product::from_row,
    )?;
    Ok(product)
}

pub fn delete(conn: &mut Connection, id: String) -> AppResult<()> {
    let affected = conn.execute("DELETE FROM products WHERE id = ?1", params![id])?;
    if affected == 0 {
        return Err(AppError::NotFound);
    }
    Ok(())
}

/// Replica la función Postgres `set_product_stock`: tipo de movimiento
/// `'initial'` si el producto nunca tuvo movimientos, si no `'adjustment'`;
/// no-op (no inserta movimiento) si el delta es 0. Corre en una transacción.
pub fn set_stock(
    conn: &mut Connection,
    product_id: String,
    stock: i64,
    note: Option<String>,
) -> AppResult<i64> {
    if stock < 0 {
        return Err(AppError::validation("El stock no puede ser negativo"));
    }

    let tx = conn.transaction()?;

    let old_stock: i64 = tx
        .query_row(
            "SELECT stock FROM products WHERE id = ?1",
            params![product_id],
            |r| r.get(0),
        )
        .map_err(|e| match e {
            rusqlite::Error::QueryReturnedNoRows => AppError::NotFound,
            other => other.into(),
        })?;

    let delta = stock - old_stock;
    if delta == 0 {
        tx.commit()?;
        return Ok(old_stock);
    }

    let previous_movements: i64 = tx.query_row(
        "SELECT COUNT(*) FROM inventory_movements WHERE product_id = ?1",
        params![product_id],
        |r| r.get(0),
    )?;
    let movement_type = if previous_movements == 0 {
        "initial"
    } else {
        "adjustment"
    };
    let now = now_iso();

    tx.execute(
        "UPDATE products SET stock = ?1, updated_at = ?2 WHERE id = ?3",
        params![stock, now, product_id],
    )?;

    tx.execute(
        "INSERT INTO inventory_movements
            (id, product_id, movement_type, quantity_delta, stock_after, sale_id, note, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, NULL, ?6, ?7)",
        params![new_id(), product_id, movement_type, delta, stock, note, now],
    )?;

    tx.commit()?;
    Ok(stock)
}

pub fn assert_exists(conn: &mut Connection, product_id: &str) -> AppResult<()> {
    let exists: i64 = conn.query_row(
        "SELECT COUNT(*) FROM products WHERE id = ?1",
        params![product_id],
        |r| r.get(0),
    )?;
    if exists == 0 {
        return Err(AppError::NotFound);
    }
    Ok(())
}
