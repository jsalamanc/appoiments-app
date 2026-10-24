//! Dominio `sales`.
//!
//! NOTA DE COORDINACIÓN: `published_catalog` (abajo) pertenece al dominio
//! catálogo/inventario (otro ingeniero, en paralelo) — se conserva tal cual
//! aquí porque comparte este mismo archivo. El resto de las funciones
//! (`create_with_inventory`, `list`, `cash_day_summary`, `revenue_metrics`,
//! `link_to_appointment`) son del dominio `sales` propiamente dicho.

use std::collections::HashMap;

use rusqlite::{params, Connection};

use crate::commands::sales::{
    CashDaySummary, ListSalesResult, PaymentBreakdown, PaymentPeriods, PublishedCatalog,
    RevenueMetrics, SaleItemInput,
};
use crate::db::{new_id, now_iso};
use crate::error::{AppError, AppResult};
use crate::models::package::Package;
use crate::models::product::Product;
use crate::models::sale::{CustomerRef, Sale, SaleItem, SaleWithDetails};
use crate::models::service::Service;
use crate::util::pagination::{get_range, get_total_pages, CATALOG_PAGE_SIZE};
use crate::util::period::period_from;

// ---------------------------------------------------------------------------
// published_catalog (dominio catálogo/inventario — no tocar la lógica)
// ---------------------------------------------------------------------------

pub fn published_catalog(conn: &mut Connection) -> AppResult<PublishedCatalog> {
    let mut products_stmt =
        conn.prepare("SELECT * FROM products WHERE status = 'published' ORDER BY name ASC")?;
    let products = products_stmt
        .query_map([], Product::from_row)?
        .collect::<Result<Vec<_>, _>>()?;

    let mut services_stmt =
        conn.prepare("SELECT * FROM services WHERE status = 'published' ORDER BY name ASC")?;
    let services = services_stmt
        .query_map([], Service::from_row)?
        .collect::<Result<Vec<_>, _>>()?;

    let mut packages_stmt =
        conn.prepare("SELECT * FROM packages WHERE status = 'published' ORDER BY name ASC")?;
    let packages = packages_stmt
        .query_map([], Package::from_row)?
        .collect::<Result<Vec<_>, _>>()?;

    Ok(PublishedCatalog {
        products,
        services,
        packages,
    })
}

// ---------------------------------------------------------------------------
// Helpers compartidos del dominio `sales`
// ---------------------------------------------------------------------------

fn round2(value: f64) -> f64 {
    (value * 100.0).round() / 100.0
}

/// `EXISTS(...)` devuelve un entero 0/1 en SQLite; se lee como `i64` (en vez
/// de `bool`) para no depender de `FromSql for bool`, igual que el resto del
/// código ya hace (ver `emp_active` en `commands/customers.rs`).
fn row_exists(conn: &rusqlite::Connection, sql: &str, id: &str) -> AppResult<bool> {
    let count: i64 = conn.query_row(sql, params![id], |r| r.get(0))?;
    Ok(count != 0)
}

fn load_customer_ref(conn: &rusqlite::Connection, customer_id: &str) -> AppResult<CustomerRef> {
    let customer = conn.query_row(
        "SELECT id, name, email, phone, document FROM customers WHERE id = ?1",
        params![customer_id],
        |row| {
            Ok(CustomerRef {
                id: row.get(0)?,
                name: row.get(1)?,
                email: row.get(2)?,
                phone: row.get(3)?,
                document: row.get(4)?,
            })
        },
    )?;
    Ok(customer)
}

fn load_sale_items(conn: &rusqlite::Connection, sale_id: &str) -> AppResult<Vec<SaleItem>> {
    let mut stmt =
        conn.prepare("SELECT * FROM sale_items WHERE sale_id = ?1 ORDER BY created_at ASC")?;
    let items = stmt
        .query_map(params![sale_id], SaleItem::from_row)?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(items)
}

fn sale_to_details(conn: &rusqlite::Connection, sale: Sale) -> AppResult<SaleWithDetails> {
    let customers = load_customer_ref(conn, &sale.customer_id)?;
    let sale_items = load_sale_items(conn, &sale.id)?;
    Ok(SaleWithDetails {
        id: sale.id,
        customer_id: sale.customer_id,
        appointment_id: sale.appointment_id,
        payment_method: sale.payment_method,
        total: sale.total,
        sold_at: sale.sold_at,
        created_at: sale.created_at,
        customers,
        sale_items,
    })
}

// ---------------------------------------------------------------------------
// create_with_inventory
// ---------------------------------------------------------------------------

pub fn create_with_inventory(
    conn: &mut Connection,
    customer_id: String,
    payment_method: String,
    items: Vec<SaleItemInput>,
) -> AppResult<String> {
    // --- Validación pura (sin tocar la DB) ---
    if payment_method != "cash" && payment_method != "transfer" {
        return Err(AppError::validation("Método de pago inválido"));
    }
    if items.is_empty() {
        return Err(AppError::validation("Agrega al menos un ítem a la venta"));
    }
    for item in &items {
        if !["product", "service", "package"].contains(&item.item_type.as_str()) {
            return Err(AppError::validation("Tipo de ítem inválido"));
        }
        if item.quantity < 1 {
            return Err(AppError::validation("La cantidad debe ser al menos 1"));
        }
        if item.unit_price < 0.0 {
            return Err(AppError::validation(
                "El precio unitario no puede ser negativo",
            ));
        }
    }

    let total = round2(
        items
            .iter()
            .map(|item| round2(item.unit_price * item.quantity as f64))
            .sum::<f64>(),
    );

    // Agrupa cantidades por producto distinto (solo ítems de tipo "product";
    // los productos dentro de un paquete no descuentan inventario, igual que
    // en el esquema actual).
    let mut product_qty: HashMap<String, i64> = HashMap::new();
    for item in &items {
        if item.item_type == "product" {
            *product_qty.entry(item.item_id.clone()).or_insert(0) += item.quantity;
        }
    }

    let tx = conn.transaction()?;

    // Cliente existe
    let customer_exists = row_exists(
        &tx,
        "SELECT EXISTS(SELECT 1 FROM customers WHERE id = ?1)",
        &customer_id,
    )?;
    if !customer_exists {
        return Err(AppError::validation("El cliente no existe"));
    }

    // Cada ítem referenciado debe estar publicado
    for item in &items {
        let published = match item.item_type.as_str() {
            "product" => row_exists(
                &tx,
                "SELECT EXISTS(SELECT 1 FROM products WHERE id = ?1 AND status = 'published')",
                &item.item_id,
            )?,
            "service" => row_exists(
                &tx,
                "SELECT EXISTS(SELECT 1 FROM services WHERE id = ?1 AND status = 'published')",
                &item.item_id,
            )?,
            "package" => row_exists(
                &tx,
                "SELECT EXISTS(SELECT 1 FROM packages WHERE id = ?1 AND status = 'published')",
                &item.item_id,
            )?,
            _ => false,
        };
        if !published {
            return Err(AppError::validation(format!(
                "\"{}\" ya no está disponible para la venta",
                item.name
            )));
        }
    }

    // Verifica stock suficiente para cada producto distinto, leyendo el stock
    // actual dentro de esta misma transacción para que el chequeo y el
    // descuento posterior sean consistentes.
    for (product_id, needed) in product_qty.iter() {
        let (name, stock): (String, i64) = tx.query_row(
            "SELECT name, stock FROM products WHERE id = ?1",
            params![product_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )?;
        if stock < *needed {
            return Err(AppError::validation(format!(
                "Stock insuficiente para {}",
                name
            )));
        }
    }

    // A partir de aquí no debe fallar: inserta venta + ítems, descuenta stock
    // e inserta movimientos de inventario.
    let sale_id = new_id();
    let now = now_iso();

    tx.execute(
        "INSERT INTO sales (id, customer_id, appointment_id, payment_method, total, sold_at, created_at)
         VALUES (?1, ?2, NULL, ?3, ?4, ?5, ?5)",
        params![sale_id, customer_id, payment_method, total, now],
    )?;

    for item in &items {
        let line_total = round2(item.unit_price * item.quantity as f64);
        let (product_id, service_id, package_id): (
            Option<&str>,
            Option<&str>,
            Option<&str>,
        ) = match item.item_type.as_str() {
            "product" => (Some(item.item_id.as_str()), None, None),
            "service" => (None, Some(item.item_id.as_str()), None),
            "package" => (None, None, Some(item.item_id.as_str())),
            _ => (None, None, None),
        };

        tx.execute(
            "INSERT INTO sale_items (
                id, sale_id, item_type, product_id, service_id, package_id,
                name_snapshot, unit_price, quantity, line_total, created_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
            params![
                new_id(),
                sale_id,
                item.item_type,
                product_id,
                service_id,
                package_id,
                item.name,
                item.unit_price,
                item.quantity,
                line_total,
                now
            ],
        )?;
    }

    for (product_id, qty) in product_qty.iter() {
        tx.execute(
            "UPDATE products SET stock = stock - ?1, updated_at = ?2 WHERE id = ?3",
            params![qty, now, product_id],
        )?;
        let stock_after: i64 = tx.query_row(
            "SELECT stock FROM products WHERE id = ?1",
            params![product_id],
            |r| r.get(0),
        )?;

        tx.execute(
            "INSERT INTO inventory_movements (
                id, product_id, movement_type, quantity_delta, stock_after, sale_id, note, created_at
            ) VALUES (?1, ?2, 'sale', ?3, ?4, ?5, NULL, ?6)",
            params![new_id(), product_id, -qty, stock_after, sale_id, now],
        )?;
    }

    tx.commit()?;

    Ok(sale_id)
}

// ---------------------------------------------------------------------------
// list
// ---------------------------------------------------------------------------

pub fn list(
    conn: &mut Connection,
    page: Option<i64>,
    page_size: Option<i64>,
    payment_method: Option<String>,
) -> AppResult<ListSalesResult> {
    let page = page.unwrap_or(1).max(1);
    let page_size = page_size.unwrap_or(CATALOG_PAGE_SIZE);
    let payment_method = payment_method.unwrap_or_else(|| "all".to_string());
    let filter_payment = matches!(payment_method.as_str(), "cash" | "transfer");

    let total: i64 = if filter_payment {
        conn.query_row(
            "SELECT COUNT(*) FROM sales WHERE payment_method = ?1",
            params![payment_method],
            |r| r.get(0),
        )?
    } else {
        conn.query_row("SELECT COUNT(*) FROM sales", [], |r| r.get(0))?
    };

    let total_pages = get_total_pages(total, page_size);
    let safe_page = page.min(total_pages);
    let (limit, offset) = get_range(safe_page, page_size);

    let sale_rows: Vec<Sale> = if filter_payment {
        let mut stmt = conn.prepare(
            "SELECT * FROM sales WHERE payment_method = ?1
             ORDER BY sold_at DESC LIMIT ?2 OFFSET ?3",
        )?;
        let rows = stmt.query_map(params![payment_method, limit, offset], Sale::from_row)?;
        rows.collect::<Result<Vec<_>, _>>()?
    } else {
        let mut stmt =
            conn.prepare("SELECT * FROM sales ORDER BY sold_at DESC LIMIT ?1 OFFSET ?2")?;
        let rows = stmt.query_map(params![limit, offset], Sale::from_row)?;
        rows.collect::<Result<Vec<_>, _>>()?
    };

    let mut items = Vec::with_capacity(sale_rows.len());
    for sale in sale_rows {
        items.push(sale_to_details(conn, sale)?);
    }

    Ok(ListSalesResult {
        items,
        total,
        page: safe_page,
        total_pages,
        page_size,
        payment_method,
    })
}

// ---------------------------------------------------------------------------
// cash_day_summary
// ---------------------------------------------------------------------------

pub fn cash_day_summary(conn: &mut Connection) -> AppResult<CashDaySummary> {
    let from = period_from("day");

    let mut stmt =
        conn.prepare("SELECT * FROM sales WHERE sold_at >= ?1 ORDER BY sold_at DESC")?;
    let sale_rows: Vec<Sale> = stmt
        .query_map(params![from], Sale::from_row)?
        .collect::<Result<Vec<_>, _>>()?;

    let mut sales = Vec::with_capacity(sale_rows.len());
    for sale in sale_rows {
        sales.push(sale_to_details(conn, sale)?);
    }

    let sales_count = sales.len() as i64;
    let total_revenue = round2(sales.iter().map(|s| s.total).sum());
    let cash_count = sales
        .iter()
        .filter(|s| s.payment_method == "cash")
        .count() as i64;
    let transfer_count = sales
        .iter()
        .filter(|s| s.payment_method == "transfer")
        .count() as i64;
    let cash_revenue = round2(
        sales
            .iter()
            .filter(|s| s.payment_method == "cash")
            .map(|s| s.total)
            .sum(),
    );
    let transfer_revenue = round2(
        sales
            .iter()
            .filter(|s| s.payment_method == "transfer")
            .map(|s| s.total)
            .sum(),
    );

    Ok(CashDaySummary {
        sales,
        sales_count,
        total_revenue,
        payments: PaymentBreakdown {
            cash_count,
            transfer_count,
            cash_revenue,
            transfer_revenue,
        },
        from,
    })
}

// ---------------------------------------------------------------------------
// revenue_metrics
// ---------------------------------------------------------------------------

pub fn revenue_metrics(conn: &mut Connection) -> AppResult<RevenueMetrics> {
    let day_from = period_from("day");
    let week_from = period_from("week");
    let month_from = period_from("month");

    let metrics = conn.query_row(
        "SELECT
            COALESCE(SUM(total), 0),
            COALESCE(SUM(CASE WHEN sold_at >= ?1 THEN total ELSE 0 END), 0),
            COALESCE(SUM(CASE WHEN sold_at >= ?2 THEN total ELSE 0 END), 0),
            COALESCE(SUM(CASE WHEN sold_at >= ?3 THEN total ELSE 0 END), 0),
            COUNT(*),
            COALESCE(SUM(CASE WHEN sold_at >= ?1 AND payment_method = 'cash' THEN 1 ELSE 0 END), 0),
            COALESCE(SUM(CASE WHEN sold_at >= ?1 AND payment_method = 'transfer' THEN 1 ELSE 0 END), 0),
            COALESCE(SUM(CASE WHEN sold_at >= ?1 AND payment_method = 'cash' THEN total ELSE 0 END), 0),
            COALESCE(SUM(CASE WHEN sold_at >= ?1 AND payment_method = 'transfer' THEN total ELSE 0 END), 0),
            COALESCE(SUM(CASE WHEN sold_at >= ?2 AND payment_method = 'cash' THEN 1 ELSE 0 END), 0),
            COALESCE(SUM(CASE WHEN sold_at >= ?2 AND payment_method = 'transfer' THEN 1 ELSE 0 END), 0),
            COALESCE(SUM(CASE WHEN sold_at >= ?2 AND payment_method = 'cash' THEN total ELSE 0 END), 0),
            COALESCE(SUM(CASE WHEN sold_at >= ?2 AND payment_method = 'transfer' THEN total ELSE 0 END), 0),
            COALESCE(SUM(CASE WHEN sold_at >= ?3 AND payment_method = 'cash' THEN 1 ELSE 0 END), 0),
            COALESCE(SUM(CASE WHEN sold_at >= ?3 AND payment_method = 'transfer' THEN 1 ELSE 0 END), 0),
            COALESCE(SUM(CASE WHEN sold_at >= ?3 AND payment_method = 'cash' THEN total ELSE 0 END), 0),
            COALESCE(SUM(CASE WHEN sold_at >= ?3 AND payment_method = 'transfer' THEN total ELSE 0 END), 0),
            COALESCE(SUM(CASE WHEN payment_method = 'cash' THEN 1 ELSE 0 END), 0),
            COALESCE(SUM(CASE WHEN payment_method = 'transfer' THEN 1 ELSE 0 END), 0),
            COALESCE(SUM(CASE WHEN payment_method = 'cash' THEN total ELSE 0 END), 0),
            COALESCE(SUM(CASE WHEN payment_method = 'transfer' THEN total ELSE 0 END), 0)
         FROM sales",
        params![day_from, week_from, month_from],
        |row| {
            let day = PaymentBreakdown {
                cash_count: row.get(5)?,
                transfer_count: row.get(6)?,
                cash_revenue: row.get(7)?,
                transfer_revenue: row.get(8)?,
            };
            let week = PaymentBreakdown {
                cash_count: row.get(9)?,
                transfer_count: row.get(10)?,
                cash_revenue: row.get(11)?,
                transfer_revenue: row.get(12)?,
            };
            let month = PaymentBreakdown {
                cash_count: row.get(13)?,
                transfer_count: row.get(14)?,
                cash_revenue: row.get(15)?,
                transfer_revenue: row.get(16)?,
            };
            let all = PaymentBreakdown {
                cash_count: row.get(17)?,
                transfer_count: row.get(18)?,
                cash_revenue: row.get(19)?,
                transfer_revenue: row.get(20)?,
            };

            Ok(RevenueMetrics {
                total_revenue: row.get(0)?,
                day_revenue: row.get(1)?,
                week_revenue: row.get(2)?,
                month_revenue: row.get(3)?,
                sales_count: row.get(4)?,
                payments: PaymentPeriods { day, week, month, all },
            })
        },
    )?;

    Ok(metrics)
}

// ---------------------------------------------------------------------------
// link_to_appointment
// ---------------------------------------------------------------------------

pub fn link_to_appointment(
    conn: &mut Connection,
    sale_id: String,
    appointment_id: String,
) -> AppResult<()> {
    let tx = conn.transaction()?;

    let sale_exists = row_exists(
        &tx,
        "SELECT EXISTS(SELECT 1 FROM sales WHERE id = ?1)",
        &sale_id,
    )?;
    if !sale_exists {
        return Err(AppError::validation("La venta no existe"));
    }

    let appointment_exists = row_exists(
        &tx,
        "SELECT EXISTS(SELECT 1 FROM appointments WHERE id = ?1)",
        &appointment_id,
    )?;
    if !appointment_exists {
        return Err(AppError::validation("La cita no existe"));
    }

    tx.execute(
        "UPDATE sales SET appointment_id = ?1 WHERE id = ?2",
        params![appointment_id, sale_id],
    )?;
    tx.execute(
        "UPDATE appointments SET sale_id = ?1, updated_at = ?2 WHERE id = ?3",
        params![sale_id, now_iso(), appointment_id],
    )?;

    tx.commit()?;
    Ok(())
}
