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
    if input.cost < 0.0 {
        return Err(AppError::validation("El costo no puede ser negativo"));
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
        "INSERT INTO products (id, name, price, cost, photo_path, status, stock, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, 0, ?7, ?7)",
        params![
            id,
            input.name.trim(),
            input.price,
            input.cost,
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
        "UPDATE products SET name = ?1, price = ?2, cost = ?3, photo_path = ?4, status = ?5, updated_at = ?6
         WHERE id = ?7",
        params![
            input.name.trim(),
            input.price,
            input.cost,
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
    unit_cost: Option<f64>,
) -> AppResult<i64> {
    if stock < 0 {
        return Err(AppError::validation("El stock no puede ser negativo"));
    }
    if let Some(cost) = unit_cost {
        if cost < 0.0 {
            return Err(AppError::validation("El costo no puede ser negativo"));
        }
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

    // El costo solo se actualiza en una entrada (delta > 0) — una reducción
    // de stock no informa nada sobre el costo de reposición (ver doc del
    // comando `set_product_stock`).
    if let Some(cost) = unit_cost {
        if delta > 0 {
            tx.execute(
                "UPDATE products SET stock = ?1, cost = ?2, updated_at = ?3 WHERE id = ?4",
                params![stock, cost, now, product_id],
            )?;
        } else {
            tx.execute(
                "UPDATE products SET stock = ?1, updated_at = ?2 WHERE id = ?3",
                params![stock, now, product_id],
            )?;
        }
    } else {
        tx.execute(
            "UPDATE products SET stock = ?1, updated_at = ?2 WHERE id = ?3",
            params![stock, now, product_id],
        )?;
    }

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

/// Implementación PostgreSQL (issue #9, Fase 2) — mismas reglas que arriba,
/// adaptadas al driver async `sqlx`. `products.stock` e
/// `inventory_movements.quantity_delta`/`stock_after` son `BIGINT` en
/// Postgres (ver `migrations/postgres/0001_init.sql`), por lo que `sqlx` los
/// decodifica nativamente como `i64`, sin necesidad de castear. Se usa
/// `RETURNING` en los `INSERT`/`UPDATE` para evitar una segunda consulta de
/// relectura, y `updated_at = now()` del lado de Postgres.
pub mod postgres {
    use sqlx::PgPool;

    use crate::commands::products::{ListProductsResult, ProductInput};
    use crate::db::new_id;
    use crate::error::{AppError, AppResult};
    use crate::models::product::Product;
    use crate::util::catalog::{normalize_status, pick_count, CatalogCounts};
    use crate::util::pagination::{get_range, get_total_pages, CATALOG_PAGE_SIZE};

    use super::validate_input;

    /// Equivalente Postgres de `util::catalog::status_counts` (esa función
    /// toma un `rusqlite::Connection`, así que no se reutiliza acá). `table`
    /// es siempre un literal fijo pasado por el código, nunca por el usuario.
    async fn status_counts(pool: &PgPool, table: &str) -> AppResult<CatalogCounts> {
        let all: i64 = sqlx::query_scalar(&format!("SELECT COUNT(*) FROM {table}"))
            .fetch_one(pool)
            .await?;
        let published: i64 = sqlx::query_scalar(&format!(
            "SELECT COUNT(*) FROM {table} WHERE status = 'published'"
        ))
        .fetch_one(pool)
        .await?;
        let draft: i64 = sqlx::query_scalar(&format!(
            "SELECT COUNT(*) FROM {table} WHERE status = 'draft'"
        ))
        .fetch_one(pool)
        .await?;
        Ok(CatalogCounts {
            all,
            published,
            draft,
        })
    }

    pub async fn list(
        pool: &PgPool,
        status: Option<String>,
        page: Option<i64>,
        page_size: Option<i64>,
    ) -> AppResult<ListProductsResult> {
        let status = normalize_status(status);
        let page = page.unwrap_or(1).max(1);
        let page_size = page_size.unwrap_or(CATALOG_PAGE_SIZE);

        let counts = status_counts(pool, "products").await?;
        let total = pick_count(&counts, &status);
        let total_pages = get_total_pages(total, page_size);
        let safe_page = page.min(total_pages);
        let (limit, offset) = get_range(safe_page, page_size);

        let rows = if status == "all" {
            sqlx::query("SELECT * FROM products ORDER BY created_at DESC LIMIT $1 OFFSET $2")
                .bind(limit)
                .bind(offset)
                .fetch_all(pool)
                .await?
        } else {
            sqlx::query(
                "SELECT * FROM products WHERE status = $1 ORDER BY created_at DESC LIMIT $2 OFFSET $3",
            )
            .bind(&status)
            .bind(limit)
            .bind(offset)
            .fetch_all(pool)
            .await?
        };

        let items = rows
            .iter()
            .map(Product::from_pg_row)
            .collect::<AppResult<Vec<_>>>()?;

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

    pub async fn get(pool: &PgPool, id: String) -> AppResult<Option<Product>> {
        let row = sqlx::query("SELECT * FROM products WHERE id = $1")
            .bind(id)
            .fetch_optional(pool)
            .await?;
        row.as_ref().map(Product::from_pg_row).transpose()
    }

    pub async fn create(pool: &PgPool, input: ProductInput) -> AppResult<Product> {
        validate_input(&input)?;

        let row = sqlx::query(
            "INSERT INTO products (id, name, price, cost, photo_path, status, stock)
             VALUES ($1, $2, $3, $4, $5, $6, 0)
             RETURNING *",
        )
        .bind(new_id())
        .bind(input.name.trim())
        .bind(input.price)
        .bind(input.cost)
        .bind(&input.photo_path)
        .bind(&input.status)
        .fetch_one(pool)
        .await?;

        Product::from_pg_row(&row)
    }

    pub async fn update(pool: &PgPool, id: String, input: ProductInput) -> AppResult<Product> {
        validate_input(&input)?;

        let row = sqlx::query(
            "UPDATE products SET name = $1, price = $2, cost = $3, photo_path = $4, status = $5, updated_at = now()
             WHERE id = $6
             RETURNING *",
        )
        .bind(input.name.trim())
        .bind(input.price)
        .bind(input.cost)
        .bind(&input.photo_path)
        .bind(&input.status)
        .bind(&id)
        .fetch_optional(pool)
        .await?
        .ok_or(AppError::NotFound)?;

        Product::from_pg_row(&row)
    }

    pub async fn delete(pool: &PgPool, id: String) -> AppResult<()> {
        let result = sqlx::query("DELETE FROM products WHERE id = $1")
            .bind(id)
            .execute(pool)
            .await?;
        if result.rows_affected() == 0 {
            return Err(AppError::NotFound);
        }
        Ok(())
    }

    /// Mismas reglas que `super::set_stock`, corriendo en una transacción
    /// `sqlx` (ver `sales::postgres::create_with_inventory` para el patrón
    /// general de transacciones en este módulo).
    pub async fn set_stock(
        pool: &PgPool,
        product_id: String,
        stock: i64,
        note: Option<String>,
        unit_cost: Option<f64>,
    ) -> AppResult<i64> {
        if stock < 0 {
            return Err(AppError::validation("El stock no puede ser negativo"));
        }
        if let Some(cost) = unit_cost {
            if cost < 0.0 {
                return Err(AppError::validation("El costo no puede ser negativo"));
            }
        }

        let mut tx = pool.begin().await?;

        let old_stock: i64 = sqlx::query_scalar("SELECT stock FROM products WHERE id = $1")
            .bind(&product_id)
            .fetch_optional(&mut *tx)
            .await?
            .ok_or(AppError::NotFound)?;

        let delta = stock - old_stock;
        if delta == 0 {
            tx.commit().await?;
            return Ok(old_stock);
        }

        let previous_movements: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM inventory_movements WHERE product_id = $1",
        )
        .bind(&product_id)
        .fetch_one(&mut *tx)
        .await?;
        let movement_type = if previous_movements == 0 {
            "initial"
        } else {
            "adjustment"
        };

        // El costo solo se actualiza en una entrada (delta > 0) — ver doc de
        // `super::set_stock`.
        if let (Some(cost), true) = (unit_cost, delta > 0) {
            sqlx::query("UPDATE products SET stock = $1, cost = $2, updated_at = now() WHERE id = $3")
                .bind(stock)
                .bind(cost)
                .bind(&product_id)
                .execute(&mut *tx)
                .await?;
        } else {
            sqlx::query("UPDATE products SET stock = $1, updated_at = now() WHERE id = $2")
                .bind(stock)
                .bind(&product_id)
                .execute(&mut *tx)
                .await?;
        }

        sqlx::query(
            "INSERT INTO inventory_movements
                (id, product_id, movement_type, quantity_delta, stock_after, sale_id, note)
             VALUES ($1, $2, $3, $4, $5, NULL, $6)",
        )
        .bind(new_id())
        .bind(&product_id)
        .bind(movement_type)
        .bind(delta)
        .bind(stock)
        .bind(&note)
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;
        Ok(stock)
    }

    pub async fn assert_exists(pool: &PgPool, product_id: &str) -> AppResult<()> {
        let exists: bool =
            sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM products WHERE id = $1)")
                .bind(product_id)
                .fetch_one(pool)
                .await?;
        if !exists {
            return Err(AppError::NotFound);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn open_test_db() -> Connection {
        let mut conn = Connection::open_in_memory().unwrap();
        crate::db::migrations().to_latest(&mut conn).unwrap();
        conn
    }

    fn input(price: f64, cost: f64) -> ProductInput {
        ProductInput {
            name: "Shampoo".to_string(),
            price,
            cost,
            photo_path: None,
            status: "draft".to_string(),
        }
    }

    #[test]
    fn create_stores_cost() {
        let mut conn = open_test_db();

        let product = create(&mut conn, input(50.0, 20.0)).unwrap();

        assert_eq!(product.cost, 20.0);
    }

    #[test]
    fn create_rejects_negative_cost() {
        let mut conn = open_test_db();

        let result = create(&mut conn, input(50.0, -1.0));

        assert!(result.is_err(), "el costo no puede ser negativo");
    }

    #[test]
    fn set_stock_updates_cost_when_stock_increases_with_unit_cost() {
        let mut conn = open_test_db();
        let product = create(&mut conn, input(50.0, 20.0)).unwrap();

        set_stock(&mut conn, product.id.clone(), 10, None, Some(25.0)).unwrap();

        let updated = get(&mut conn, product.id).unwrap().unwrap();
        assert_eq!(updated.cost, 25.0, "una entrada de stock con costo debe actualizar el costo del producto");
    }

    #[test]
    fn set_stock_ignores_unit_cost_when_stock_decreases() {
        let mut conn = open_test_db();
        let product = create(&mut conn, input(50.0, 20.0)).unwrap();
        set_stock(&mut conn, product.id.clone(), 10, None, Some(25.0)).unwrap();

        set_stock(&mut conn, product.id.clone(), 5, None, Some(99.0)).unwrap();

        let updated = get(&mut conn, product.id).unwrap().unwrap();
        assert_eq!(updated.cost, 25.0, "una reducción de stock no debe cambiar el costo de reposición");
    }

    #[test]
    fn set_stock_rejects_negative_unit_cost() {
        let mut conn = open_test_db();
        let product = create(&mut conn, input(50.0, 20.0)).unwrap();

        let result = set_stock(&mut conn, product.id, 10, None, Some(-5.0));

        assert!(result.is_err());
    }
}
