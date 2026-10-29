//! Dominio `sales`.
//!
//! NOTA DE COORDINACIÓN: `published_catalog` (abajo) pertenece al dominio
//! catálogo/inventario (otro ingeniero, en paralelo) — se conserva tal cual
//! aquí porque comparte este mismo archivo. El resto de las funciones
//! (`create_with_inventory`, `list`, `cash_day_summary`, `revenue_metrics`,
//! `link_to_appointment`) son del dominio `sales` propiamente dicho.

use std::collections::HashMap;

use rusqlite::{params, Connection, OptionalExtension};

use crate::commands::sales::{
    CashDaySummary, ListSalesResult, PaymentBreakdown, PaymentInput, PaymentPeriods,
    PublishedCatalog, RevenueMetrics, SaleItemInput,
};
use crate::db::{new_id, now_iso};
use crate::error::{AppError, AppResult};
use crate::models::package::Package;
use crate::models::product::Product;
use crate::models::sale::{CustomerRef, Sale, SaleItem, SalePayment, SaleWithDetails};
use crate::models::service::Service;
use crate::util::pagination::{get_range, get_total_pages, CATALOG_PAGE_SIZE};

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

/// Valida la lista de pagos de una venta y devuelve la etiqueta resumen que
/// se guarda en `sales.payment_method`: el método único si hay uno solo, o
/// `"mixed"` si la venta se pagó con más de un medio (issue #2, Fase 2).
/// La suma de los montos debe coincidir exactamente con `total` — nunca se
/// asume que el frontend mandó el desglose correcto sin verificarlo.
fn validate_payments(payments: &[PaymentInput], total: f64) -> AppResult<String> {
    if payments.is_empty() {
        return Err(AppError::validation("Agrega al menos un medio de pago"));
    }
    let mut methods: std::collections::HashSet<&str> = std::collections::HashSet::new();
    for payment in payments {
        if !["cash", "transfer", "card"].contains(&payment.method.as_str()) {
            return Err(AppError::validation("Medio de pago inválido"));
        }
        if payment.amount <= 0.0 {
            return Err(AppError::validation(
                "El monto de cada medio de pago debe ser mayor a 0",
            ));
        }
        methods.insert(payment.method.as_str());
    }
    let paid = round2(payments.iter().map(|p| p.amount).sum());
    if paid != total {
        return Err(AppError::validation(
            "La suma de los medios de pago no coincide con el total de la venta",
        ));
    }
    Ok(if methods.len() == 1 {
        payments[0].method.clone()
    } else {
        "mixed".to_string()
    })
}

/// El descuento se aplica sobre el subtotal ya calculado desde el catálogo
/// (nunca sobre un total que mande el frontend) — no puede ser negativo ni
/// mayor al subtotal.
fn validate_discount(discount_amount: f64, subtotal: f64) -> AppResult<()> {
    if discount_amount < 0.0 {
        return Err(AppError::validation("El descuento no puede ser negativo"));
    }
    if discount_amount > subtotal {
        return Err(AppError::validation(
            "El descuento no puede ser mayor al subtotal de la venta",
        ));
    }
    Ok(())
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
        "SELECT id, name, COALESCE(email, '') AS email, phone, COALESCE(document, '') AS document
         FROM customers WHERE id = ?1",
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

fn load_sale_payments(conn: &rusqlite::Connection, sale_id: &str) -> AppResult<Vec<SalePayment>> {
    let mut stmt =
        conn.prepare("SELECT * FROM sale_payments WHERE sale_id = ?1 ORDER BY created_at ASC")?;
    let payments = stmt
        .query_map(params![sale_id], SalePayment::from_row)?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(payments)
}

fn sale_to_details(conn: &rusqlite::Connection, sale: Sale) -> AppResult<SaleWithDetails> {
    let customers = load_customer_ref(conn, &sale.customer_id)?;
    let sale_items = load_sale_items(conn, &sale.id)?;
    let payments = load_sale_payments(conn, &sale.id)?;
    Ok(SaleWithDetails {
        id: sale.id,
        customer_id: sale.customer_id,
        appointment_id: sale.appointment_id,
        user_id: sale.user_id,
        payment_method: sale.payment_method,
        total: sale.total,
        discount_amount: sale.discount_amount,
        sold_at: sale.sold_at,
        created_at: sale.created_at,
        voided_at: sale.voided_at,
        void_reason: sale.void_reason,
        customers,
        sale_items,
        payments,
    })
}

// ---------------------------------------------------------------------------
// create_with_inventory
// ---------------------------------------------------------------------------

pub fn create_with_inventory(
    conn: &mut Connection,
    customer_id: String,
    user_id: Option<String>,
    items: Vec<SaleItemInput>,
    payments: Vec<PaymentInput>,
    discount_amount: Option<f64>,
) -> AppResult<String> {
    let discount_amount = discount_amount.unwrap_or(0.0);
    // --- Validación pura (sin tocar la DB) ---
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
    }

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

    // Cada ítem referenciado debe estar publicado — y de acá sale el
    // nombre/precio REAL que se usa para la venta (issue #2: antes se
    // confiaba ciegamente en `item.unit_price`/`item.name`, lo que manda el
    // frontend, sin contrastarlo contra el catálogo).
    let mut catalog: HashMap<(String, String), (String, f64)> = HashMap::new();
    for item in &items {
        let key = (item.item_type.clone(), item.item_id.clone());
        if catalog.contains_key(&key) {
            continue;
        }
        let found: Option<(String, f64)> = match item.item_type.as_str() {
            "product" => tx
                .query_row(
                    "SELECT name, price FROM products WHERE id = ?1 AND status = 'published'",
                    params![item.item_id],
                    |row| Ok((row.get(0)?, row.get(1)?)),
                )
                .optional()?,
            "service" => tx
                .query_row(
                    "SELECT name, price FROM services WHERE id = ?1 AND status = 'published'",
                    params![item.item_id],
                    |row| Ok((row.get(0)?, row.get(1)?)),
                )
                .optional()?,
            "package" => tx
                .query_row(
                    "SELECT name, price FROM packages WHERE id = ?1 AND status = 'published'",
                    params![item.item_id],
                    |row| Ok((row.get(0)?, row.get(1)?)),
                )
                .optional()?,
            _ => None,
        };
        let resolved = found.ok_or_else(|| {
            AppError::validation(format!(
                "\"{}\" ya no está disponible para la venta",
                item.name
            ))
        })?;
        catalog.insert(key, resolved);
    }

    let subtotal = round2(
        items
            .iter()
            .map(|item| {
                let (_, price) = &catalog[&(item.item_type.clone(), item.item_id.clone())];
                round2(price * item.quantity as f64)
            })
            .sum::<f64>(),
    );
    validate_discount(discount_amount, subtotal)?;
    let total = round2(subtotal - discount_amount);
    let payment_method = validate_payments(&payments, total)?;

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
        "INSERT INTO sales (
            id, customer_id, appointment_id, user_id, payment_method, total,
            discount_amount, sold_at, created_at
        ) VALUES (?1, ?2, NULL, ?3, ?4, ?5, ?6, ?7, ?7)",
        params![sale_id, customer_id, user_id, payment_method, total, discount_amount, now],
    )?;

    for payment in &payments {
        tx.execute(
            "INSERT INTO sale_payments (id, sale_id, method, amount, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![new_id(), sale_id, payment.method, payment.amount, now],
        )?;
    }

    for item in &items {
        let (name, unit_price) = &catalog[&(item.item_type.clone(), item.item_id.clone())];
        let line_total = round2(unit_price * item.quantity as f64);
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
                name,
                unit_price,
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
// void_sale
// ---------------------------------------------------------------------------

/// Anula una venta: revierte el stock de cada producto vendido (un
/// movimiento `'return'` por cada uno, igual de auditable que el `'sale'`
/// original) y marca la venta con `voided_at`/`void_reason`. No hace nada
/// con los productos vendidos dentro de un paquete, por la misma razón que
/// `create_with_inventory` no les descuenta stock al vender.
///
/// Rechaza si la venta no existe o si ya estaba anulada — anular dos veces
/// duplicaría la reversa de inventario.
pub fn void_sale(conn: &mut Connection, sale_id: String, reason: Option<String>) -> AppResult<()> {
    let tx = conn.transaction()?;

    let already_voided: bool = tx
        .query_row(
            "SELECT voided_at IS NOT NULL FROM sales WHERE id = ?1",
            params![sale_id],
            |r| r.get(0),
        )
        .optional()?
        .ok_or(AppError::NotFound)?;
    if already_voided {
        return Err(AppError::validation("Esta venta ya fue anulada"));
    }

    let sale_items = load_sale_items(&tx, &sale_id)?;
    let mut product_qty: HashMap<String, i64> = HashMap::new();
    for item in &sale_items {
        if item.item_type == "product" {
            if let Some(product_id) = &item.product_id {
                *product_qty.entry(product_id.clone()).or_insert(0) += item.quantity;
            }
        }
    }

    let now = now_iso();
    for (product_id, qty) in product_qty.iter() {
        tx.execute(
            "UPDATE products SET stock = stock + ?1, updated_at = ?2 WHERE id = ?3",
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
            ) VALUES (?1, ?2, 'return', ?3, ?4, ?5, ?6, ?7)",
            params![new_id(), product_id, qty, stock_after, sale_id, reason, now],
        )?;
    }

    tx.execute(
        "UPDATE sales SET voided_at = ?1, void_reason = ?2 WHERE id = ?3",
        params![now, reason, sale_id],
    )?;

    tx.commit()?;
    Ok(())
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
    let filter_payment = matches!(payment_method.as_str(), "cash" | "transfer" | "card" | "mixed");

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

/// `from` es la medianoche local de "hoy", ya calculada por el frontend
/// (`getPeriodRange` en `src/lib/sales.ts`, que usa el reloj/timezone del
/// equipo) — el backend ya no calcula ningún límite de tiempo por su cuenta
/// (issue #2: antes usaba `now - 24h`, una ventana móvil, no el día
/// calendario).
/// Fecha anterior a cualquier venta real — usada como límite inferior
/// cuando se quiere el total "de siempre" reusando la misma consulta que ya
/// filtra por `sold_at >= ?`, en vez de tener una rama de SQL aparte sin
/// filtro de fecha.
const BEGINNING_OF_TIME: &str = "0001-01-01T00:00:00Z";

/// Desglose de pagos por medio (cantidad + monto) para las ventas no
/// anuladas con `sold_at >= from` — issue #2, Fase 2: antes se leía
/// directamente `sales.payment_method`, que ahora es solo una etiqueta
/// resumen ("mixed" si hubo más de un medio); el desglose real sale de
/// `sale_payments`.
fn payment_breakdown(conn: &rusqlite::Connection, from: &str) -> AppResult<PaymentBreakdown> {
    let mut cash = (0i64, 0f64);
    let mut transfer = (0i64, 0f64);
    let mut card = (0i64, 0f64);

    let mut stmt = conn.prepare(
        "SELECT sp.method, COUNT(*), COALESCE(SUM(sp.amount), 0)
         FROM sale_payments sp
         JOIN sales s ON s.id = sp.sale_id
         WHERE s.sold_at >= ?1 AND s.voided_at IS NULL
         GROUP BY sp.method",
    )?;
    let rows = stmt
        .query_map(params![from], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, i64>(1)?,
                row.get::<_, f64>(2)?,
            ))
        })?
        .collect::<Result<Vec<_>, _>>()?;
    for (method, count, revenue) in rows {
        match method.as_str() {
            "cash" => cash = (count, revenue),
            "transfer" => transfer = (count, revenue),
            "card" => card = (count, revenue),
            _ => {}
        }
    }

    Ok(PaymentBreakdown {
        cash_count: cash.0,
        transfer_count: transfer.0,
        card_count: card.0,
        cash_revenue: round2(cash.1),
        transfer_revenue: round2(transfer.1),
        card_revenue: round2(card.1),
    })
}

pub fn cash_day_summary(conn: &mut Connection, from: String) -> AppResult<CashDaySummary> {
    // Las ventas anuladas se siguen mostrando en el listado (para que quede
    // el rastro de que existieron), pero no cuentan en los totales ni en el
    // desglose de pagos.
    let mut stmt =
        conn.prepare("SELECT * FROM sales WHERE sold_at >= ?1 ORDER BY sold_at DESC")?;
    let sale_rows: Vec<Sale> = stmt
        .query_map(params![from], Sale::from_row)?
        .collect::<Result<Vec<_>, _>>()?;

    let mut sales = Vec::with_capacity(sale_rows.len());
    for sale in sale_rows {
        sales.push(sale_to_details(conn, sale)?);
    }

    let (sales_count, total_revenue): (i64, f64) = conn.query_row(
        "SELECT COUNT(*), COALESCE(SUM(total), 0) FROM sales WHERE sold_at >= ?1 AND voided_at IS NULL",
        params![from],
        |row| Ok((row.get(0)?, row.get(1)?)),
    )?;
    let payments = payment_breakdown(conn, &from)?;

    Ok(CashDaySummary {
        sales,
        sales_count,
        total_revenue: round2(total_revenue),
        payments,
        from,
    })
}

// ---------------------------------------------------------------------------
// revenue_metrics
// ---------------------------------------------------------------------------

/// `day_from`/`week_from`/`month_from` ya vienen calculados por el frontend
/// como medianoche local calendario (ver `cash_day_summary` arriba). Las
/// ventas anuladas no cuentan en ningún total (issue #2, Fase 2).
pub fn revenue_metrics(
    conn: &mut Connection,
    day_from: String,
    week_from: String,
    month_from: String,
) -> AppResult<RevenueMetrics> {
    let (sales_count, total_revenue): (i64, f64) = conn.query_row(
        "SELECT COUNT(*), COALESCE(SUM(total), 0) FROM sales WHERE voided_at IS NULL",
        [],
        |row| Ok((row.get(0)?, row.get(1)?)),
    )?;
    let revenue_since = |from: &str| -> AppResult<f64> {
        conn.query_row(
            "SELECT COALESCE(SUM(total), 0) FROM sales WHERE sold_at >= ?1 AND voided_at IS NULL",
            params![from],
            |r| r.get(0),
        )
        .map_err(AppError::from)
    };
    let day_revenue = revenue_since(&day_from)?;
    let week_revenue = revenue_since(&week_from)?;
    let month_revenue = revenue_since(&month_from)?;

    let day = payment_breakdown(conn, &day_from)?;
    let week = payment_breakdown(conn, &week_from)?;
    let month = payment_breakdown(conn, &month_from)?;
    let all = payment_breakdown(conn, BEGINNING_OF_TIME)?;

    Ok(RevenueMetrics {
        total_revenue: round2(total_revenue),
        day_revenue: round2(day_revenue),
        week_revenue: round2(week_revenue),
        month_revenue: round2(month_revenue),
        sales_count,
        payments: PaymentPeriods { day, week, month, all },
    })
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

/// Implementación PostgreSQL (issue #9, Fase 2). `row_exists` queda genérico
/// sobre `PgExecutor` porque se usa tanto contra el pool como dentro de las
/// transacciones de `create_with_inventory`/`link_to_appointment`;
/// `load_customer_ref`/`load_sale_items`/`sale_to_details` solo se usan a
/// nivel de pool (igual que en la versión SQLite, donde tampoco corren dentro
/// de una transacción), así que toman `&PgPool` directo.
pub mod postgres {
    use std::collections::HashMap;

    use sqlx::{PgExecutor, PgPool, Row};

    use crate::commands::sales::{
        CashDaySummary, ListSalesResult, PaymentBreakdown, PaymentInput, PaymentPeriods,
        PublishedCatalog, RevenueMetrics, SaleItemInput,
    };
    use crate::db::new_id;
    use crate::error::{AppError, AppResult};
    use crate::models::package::Package;
    use crate::models::product::Product;
    use crate::models::sale::{CustomerRef, Sale, SaleItem, SalePayment, SaleWithDetails};
    use crate::models::service::Service;
    use crate::util::pagination::{get_range, get_total_pages, CATALOG_PAGE_SIZE};

    use super::{round2, validate_discount, validate_payments};

    pub async fn published_catalog(pool: &PgPool) -> AppResult<PublishedCatalog> {
        let product_rows = sqlx::query("SELECT * FROM products WHERE status = 'published' ORDER BY name ASC")
            .fetch_all(pool)
            .await?;
        let products = product_rows
            .iter()
            .map(Product::from_pg_row)
            .collect::<AppResult<Vec<_>>>()?;

        let service_rows = sqlx::query("SELECT * FROM services WHERE status = 'published' ORDER BY name ASC")
            .fetch_all(pool)
            .await?;
        let services = service_rows
            .iter()
            .map(Service::from_pg_row)
            .collect::<AppResult<Vec<_>>>()?;

        let package_rows = sqlx::query("SELECT * FROM packages WHERE status = 'published' ORDER BY name ASC")
            .fetch_all(pool)
            .await?;
        let packages = package_rows
            .iter()
            .map(Package::from_pg_row)
            .collect::<AppResult<Vec<_>>>()?;

        Ok(PublishedCatalog {
            products,
            services,
            packages,
        })
    }

    async fn row_exists<'e, E>(executor: E, sql: &str, id: &str) -> AppResult<bool>
    where
        E: PgExecutor<'e>,
    {
        let exists: bool = sqlx::query_scalar(sql).bind(id).fetch_one(executor).await?;
        Ok(exists)
    }

    async fn load_customer_ref(pool: &PgPool, customer_id: &str) -> AppResult<CustomerRef> {
        let row = sqlx::query(
            "SELECT id, name, COALESCE(email, '') AS email, phone, COALESCE(document, '') AS document
             FROM customers WHERE id = $1",
        )
        .bind(customer_id)
        .fetch_one(pool)
        .await?;
        Ok(CustomerRef {
            id: row.try_get(0)?,
            name: row.try_get(1)?,
            email: row.try_get(2)?,
            phone: row.try_get(3)?,
            document: row.try_get(4)?,
        })
    }

    async fn load_sale_items(pool: &PgPool, sale_id: &str) -> AppResult<Vec<SaleItem>> {
        let rows = sqlx::query("SELECT * FROM sale_items WHERE sale_id = $1 ORDER BY created_at ASC")
            .bind(sale_id)
            .fetch_all(pool)
            .await?;
        rows.iter().map(SaleItem::from_pg_row).collect()
    }

    async fn load_sale_payments(pool: &PgPool, sale_id: &str) -> AppResult<Vec<SalePayment>> {
        let rows = sqlx::query("SELECT * FROM sale_payments WHERE sale_id = $1 ORDER BY created_at ASC")
            .bind(sale_id)
            .fetch_all(pool)
            .await?;
        rows.iter().map(SalePayment::from_pg_row).collect()
    }

    /// El frontend manda el límite de tiempo ya calculado como string
    /// RFC3339 (medianoche local calendario, ver `cash_day_summary` arriba);
    /// acá se parsea a `OffsetDateTime` para comparar nativamente contra
    /// columnas `TIMESTAMPTZ`. Comparar como texto (`sold_at::text >= $1`)
    /// sería incorrecto: el formato de texto por defecto de Postgres para
    /// `timestamptz` no es RFC3339 (usa espacio en vez de `T`, `+00` en vez
    /// de `Z`), así que la comparación lexicográfica no sería equivalente a
    /// la comparación cronológica real.
    fn parse_from(from: &str) -> AppResult<time::OffsetDateTime> {
        time::OffsetDateTime::parse(from, &time::format_description::well_known::Rfc3339)
            .map_err(|e| AppError::Internal(e.to_string()))
    }

    async fn sale_to_details(pool: &PgPool, sale: Sale) -> AppResult<SaleWithDetails> {
        let customers = load_customer_ref(pool, &sale.customer_id).await?;
        let sale_items = load_sale_items(pool, &sale.id).await?;
        let payments = load_sale_payments(pool, &sale.id).await?;
        Ok(SaleWithDetails {
            id: sale.id,
            customer_id: sale.customer_id,
            appointment_id: sale.appointment_id,
            user_id: sale.user_id,
            payment_method: sale.payment_method,
            total: sale.total,
            discount_amount: sale.discount_amount,
            sold_at: sale.sold_at,
            created_at: sale.created_at,
            voided_at: sale.voided_at,
            void_reason: sale.void_reason,
            customers,
            sale_items,
            payments,
        })
    }

    pub async fn create_with_inventory(
        pool: &PgPool,
        customer_id: String,
        user_id: Option<String>,
        items: Vec<SaleItemInput>,
        payments: Vec<PaymentInput>,
        discount_amount: Option<f64>,
    ) -> AppResult<String> {
        let discount_amount = discount_amount.unwrap_or(0.0);
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
        }

        let mut product_qty: HashMap<String, i64> = HashMap::new();
        for item in &items {
            if item.item_type == "product" {
                *product_qty.entry(item.item_id.clone()).or_insert(0) += item.quantity;
            }
        }

        let mut tx = pool.begin().await?;

        let customer_exists = row_exists(
            &mut *tx,
            "SELECT EXISTS(SELECT 1 FROM customers WHERE id = $1)",
            &customer_id,
        )
        .await?;
        if !customer_exists {
            return Err(AppError::validation("El cliente no existe"));
        }

        // Cada ítem referenciado debe estar publicado — y de acá sale el
        // nombre/precio REAL que se usa para la venta (issue #2: antes se
        // confiaba en `item.unit_price`/`item.name` del frontend).
        let mut catalog: HashMap<(String, String), (String, f64)> = HashMap::new();
        for item in &items {
            let key = (item.item_type.clone(), item.item_id.clone());
            if catalog.contains_key(&key) {
                continue;
            }
            let row = match item.item_type.as_str() {
                "product" => {
                    sqlx::query("SELECT name, price FROM products WHERE id = $1 AND status = 'published'")
                        .bind(&item.item_id)
                        .fetch_optional(&mut *tx)
                        .await?
                }
                "service" => {
                    sqlx::query("SELECT name, price FROM services WHERE id = $1 AND status = 'published'")
                        .bind(&item.item_id)
                        .fetch_optional(&mut *tx)
                        .await?
                }
                "package" => {
                    sqlx::query("SELECT name, price FROM packages WHERE id = $1 AND status = 'published'")
                        .bind(&item.item_id)
                        .fetch_optional(&mut *tx)
                        .await?
                }
                _ => None,
            };
            let found = row
                .map(|row| -> AppResult<(String, f64)> {
                    Ok((row.try_get("name")?, row.try_get("price")?))
                })
                .transpose()?;
            let resolved = found.ok_or_else(|| {
                AppError::validation(format!(
                    "\"{}\" ya no está disponible para la venta",
                    item.name
                ))
            })?;
            catalog.insert(key, resolved);
        }

        let subtotal = round2(
            items
                .iter()
                .map(|item| {
                    let (_, price) = &catalog[&(item.item_type.clone(), item.item_id.clone())];
                    round2(price * item.quantity as f64)
                })
                .sum::<f64>(),
        );
        validate_discount(discount_amount, subtotal)?;
        let total = round2(subtotal - discount_amount);
        let payment_method = validate_payments(&payments, total)?;

        for (product_id, needed) in product_qty.iter() {
            let row = sqlx::query("SELECT name, stock FROM products WHERE id = $1")
                .bind(product_id)
                .fetch_one(&mut *tx)
                .await?;
            let name: String = row.try_get("name")?;
            let stock: i64 = row.try_get("stock")?;
            if stock < *needed {
                return Err(AppError::validation(format!(
                    "Stock insuficiente para {}",
                    name
                )));
            }
        }

        let sale_id = new_id();

        sqlx::query(
            "INSERT INTO sales (
                id, customer_id, appointment_id, user_id, payment_method, total, discount_amount
            ) VALUES ($1, $2, NULL, $3, $4, $5, $6)",
        )
        .bind(&sale_id)
        .bind(&customer_id)
        .bind(&user_id)
        .bind(&payment_method)
        .bind(total)
        .bind(discount_amount)
        .execute(&mut *tx)
        .await?;

        for payment in &payments {
            sqlx::query(
                "INSERT INTO sale_payments (id, sale_id, method, amount) VALUES ($1, $2, $3, $4)",
            )
            .bind(new_id())
            .bind(&sale_id)
            .bind(&payment.method)
            .bind(payment.amount)
            .execute(&mut *tx)
            .await?;
        }

        for item in &items {
            let (name, unit_price) = &catalog[&(item.item_type.clone(), item.item_id.clone())];
            let line_total = round2(unit_price * item.quantity as f64);
            let (product_id, service_id, package_id): (Option<&str>, Option<&str>, Option<&str>) =
                match item.item_type.as_str() {
                    "product" => (Some(item.item_id.as_str()), None, None),
                    "service" => (None, Some(item.item_id.as_str()), None),
                    "package" => (None, None, Some(item.item_id.as_str())),
                    _ => (None, None, None),
                };

            sqlx::query(
                "INSERT INTO sale_items (
                    id, sale_id, item_type, product_id, service_id, package_id,
                    name_snapshot, unit_price, quantity, line_total
                ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)",
            )
            .bind(new_id())
            .bind(&sale_id)
            .bind(&item.item_type)
            .bind(product_id)
            .bind(service_id)
            .bind(package_id)
            .bind(name)
            .bind(unit_price)
            .bind(item.quantity)
            .bind(line_total)
            .execute(&mut *tx)
            .await?;
        }

        for (product_id, qty) in product_qty.iter() {
            sqlx::query("UPDATE products SET stock = stock - $1, updated_at = now() WHERE id = $2")
                .bind(qty)
                .bind(product_id)
                .execute(&mut *tx)
                .await?;

            let stock_after: i64 = sqlx::query_scalar("SELECT stock FROM products WHERE id = $1")
                .bind(product_id)
                .fetch_one(&mut *tx)
                .await?;

            sqlx::query(
                "INSERT INTO inventory_movements (
                    id, product_id, movement_type, quantity_delta, stock_after, sale_id, note
                ) VALUES ($1, $2, 'sale', $3, $4, $5, NULL)",
            )
            .bind(new_id())
            .bind(product_id)
            .bind(-qty)
            .bind(stock_after)
            .bind(&sale_id)
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;

        Ok(sale_id)
    }

    /// Ver `super::void_sale` — misma lógica, dialecto Postgres.
    pub async fn void_sale(pool: &PgPool, sale_id: String, reason: Option<String>) -> AppResult<()> {
        let mut tx = pool.begin().await?;

        let already_voided: Option<bool> =
            sqlx::query_scalar("SELECT voided_at IS NOT NULL FROM sales WHERE id = $1")
                .bind(&sale_id)
                .fetch_optional(&mut *tx)
                .await?;
        match already_voided {
            None => return Err(AppError::NotFound),
            Some(true) => return Err(AppError::validation("Esta venta ya fue anulada")),
            Some(false) => {}
        }

        let sale_items = load_sale_items(pool, &sale_id).await?;
        let mut product_qty: HashMap<String, i64> = HashMap::new();
        for item in &sale_items {
            if item.item_type == "product" {
                if let Some(product_id) = &item.product_id {
                    *product_qty.entry(product_id.clone()).or_insert(0) += item.quantity;
                }
            }
        }

        for (product_id, qty) in product_qty.iter() {
            sqlx::query("UPDATE products SET stock = stock + $1, updated_at = now() WHERE id = $2")
                .bind(qty)
                .bind(product_id)
                .execute(&mut *tx)
                .await?;

            let stock_after: i64 = sqlx::query_scalar("SELECT stock FROM products WHERE id = $1")
                .bind(product_id)
                .fetch_one(&mut *tx)
                .await?;

            sqlx::query(
                "INSERT INTO inventory_movements (
                    id, product_id, movement_type, quantity_delta, stock_after, sale_id, note
                ) VALUES ($1, $2, 'return', $3, $4, $5, $6)",
            )
            .bind(new_id())
            .bind(product_id)
            .bind(qty)
            .bind(stock_after)
            .bind(&sale_id)
            .bind(&reason)
            .execute(&mut *tx)
            .await?;
        }

        sqlx::query("UPDATE sales SET voided_at = now(), void_reason = $1 WHERE id = $2")
            .bind(&reason)
            .bind(&sale_id)
            .execute(&mut *tx)
            .await?;

        tx.commit().await?;
        Ok(())
    }

    pub async fn list(
        pool: &PgPool,
        page: Option<i64>,
        page_size: Option<i64>,
        payment_method: Option<String>,
    ) -> AppResult<ListSalesResult> {
        let page = page.unwrap_or(1).max(1);
        let page_size = page_size.unwrap_or(CATALOG_PAGE_SIZE);
        let payment_method = payment_method.unwrap_or_else(|| "all".to_string());
        let filter_payment = matches!(payment_method.as_str(), "cash" | "transfer" | "card" | "mixed");

        let total: i64 = if filter_payment {
            sqlx::query_scalar("SELECT COUNT(*) FROM sales WHERE payment_method = $1")
                .bind(&payment_method)
                .fetch_one(pool)
                .await?
        } else {
            sqlx::query_scalar("SELECT COUNT(*) FROM sales")
                .fetch_one(pool)
                .await?
        };

        let total_pages = get_total_pages(total, page_size);
        let safe_page = page.min(total_pages);
        let (limit, offset) = get_range(safe_page, page_size);

        let sale_rows = if filter_payment {
            sqlx::query(
                "SELECT * FROM sales WHERE payment_method = $1
                 ORDER BY sold_at DESC LIMIT $2 OFFSET $3",
            )
            .bind(&payment_method)
            .bind(limit)
            .bind(offset)
            .fetch_all(pool)
            .await?
        } else {
            sqlx::query("SELECT * FROM sales ORDER BY sold_at DESC LIMIT $1 OFFSET $2")
                .bind(limit)
                .bind(offset)
                .fetch_all(pool)
                .await?
        };

        let mut items = Vec::with_capacity(sale_rows.len());
        for row in &sale_rows {
            let sale = Sale::from_pg_row(row)?;
            items.push(sale_to_details(pool, sale).await?);
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

    /// Ver `super::payment_breakdown` — misma lógica, dialecto Postgres.
    async fn payment_breakdown(pool: &PgPool, from: time::OffsetDateTime) -> AppResult<PaymentBreakdown> {
        let rows = sqlx::query(
            "SELECT sp.method AS method, COUNT(*) AS cnt, COALESCE(SUM(sp.amount), 0) AS revenue
             FROM sale_payments sp
             JOIN sales s ON s.id = sp.sale_id
             WHERE s.sold_at >= $1 AND s.voided_at IS NULL
             GROUP BY sp.method",
        )
        .bind(from)
        .fetch_all(pool)
        .await?;

        let mut cash = (0i64, 0f64);
        let mut transfer = (0i64, 0f64);
        let mut card = (0i64, 0f64);
        for row in &rows {
            let method: String = row.try_get("method")?;
            let count: i64 = row.try_get("cnt")?;
            let revenue: f64 = row.try_get("revenue")?;
            match method.as_str() {
                "cash" => cash = (count, revenue),
                "transfer" => transfer = (count, revenue),
                "card" => card = (count, revenue),
                _ => {}
            }
        }

        Ok(PaymentBreakdown {
            cash_count: cash.0,
            transfer_count: transfer.0,
            card_count: card.0,
            cash_revenue: round2(cash.1),
            transfer_revenue: round2(transfer.1),
            card_revenue: round2(card.1),
        })
    }

    pub async fn cash_day_summary(pool: &PgPool, from: String) -> AppResult<CashDaySummary> {
        let from_dt = parse_from(&from)?;

        let sale_rows = sqlx::query("SELECT * FROM sales WHERE sold_at >= $1 ORDER BY sold_at DESC")
            .bind(from_dt)
            .fetch_all(pool)
            .await?;

        let mut sales = Vec::with_capacity(sale_rows.len());
        for row in &sale_rows {
            let sale = Sale::from_pg_row(row)?;
            sales.push(sale_to_details(pool, sale).await?);
        }

        let totals_row = sqlx::query(
            "SELECT COUNT(*) AS cnt, COALESCE(SUM(total), 0) AS revenue
             FROM sales WHERE sold_at >= $1 AND voided_at IS NULL",
        )
        .bind(from_dt)
        .fetch_one(pool)
        .await?;
        let sales_count: i64 = totals_row.try_get("cnt")?;
        let total_revenue: f64 = totals_row.try_get("revenue")?;

        let payments = payment_breakdown(pool, from_dt).await?;

        Ok(CashDaySummary {
            sales,
            sales_count,
            total_revenue: round2(total_revenue),
            payments,
            from,
        })
    }

    async fn revenue_since(pool: &PgPool, from: time::OffsetDateTime) -> AppResult<f64> {
        let revenue: f64 = sqlx::query_scalar(
            "SELECT COALESCE(SUM(total), 0) FROM sales WHERE sold_at >= $1 AND voided_at IS NULL",
        )
        .bind(from)
        .fetch_one(pool)
        .await?;
        Ok(revenue)
    }

    pub async fn revenue_metrics(
        pool: &PgPool,
        day_from: String,
        week_from: String,
        month_from: String,
    ) -> AppResult<RevenueMetrics> {
        let day_from = parse_from(&day_from)?;
        let week_from = parse_from(&week_from)?;
        let month_from = parse_from(&month_from)?;

        let totals_row = sqlx::query(
            "SELECT COUNT(*) AS cnt, COALESCE(SUM(total), 0) AS revenue FROM sales WHERE voided_at IS NULL",
        )
        .fetch_one(pool)
        .await?;
        let sales_count: i64 = totals_row.try_get("cnt")?;
        let total_revenue: f64 = totals_row.try_get("revenue")?;

        let day_revenue = revenue_since(pool, day_from).await?;
        let week_revenue = revenue_since(pool, week_from).await?;
        let month_revenue = revenue_since(pool, month_from).await?;

        let day = payment_breakdown(pool, day_from).await?;
        let week = payment_breakdown(pool, week_from).await?;
        let month = payment_breakdown(pool, month_from).await?;
        let all = payment_breakdown(pool, time::OffsetDateTime::UNIX_EPOCH).await?;

        Ok(RevenueMetrics {
            total_revenue: round2(total_revenue),
            day_revenue: round2(day_revenue),
            week_revenue: round2(week_revenue),
            month_revenue: round2(month_revenue),
            sales_count,
            payments: PaymentPeriods { day, week, month, all },
        })
    }

    pub async fn link_to_appointment(
        pool: &PgPool,
        sale_id: String,
        appointment_id: String,
    ) -> AppResult<()> {
        let mut tx = pool.begin().await?;

        let sale_exists = row_exists(
            &mut *tx,
            "SELECT EXISTS(SELECT 1 FROM sales WHERE id = $1)",
            &sale_id,
        )
        .await?;
        if !sale_exists {
            return Err(AppError::validation("La venta no existe"));
        }

        let appointment_exists = row_exists(
            &mut *tx,
            "SELECT EXISTS(SELECT 1 FROM appointments WHERE id = $1)",
            &appointment_id,
        )
        .await?;
        if !appointment_exists {
            return Err(AppError::validation("La cita no existe"));
        }

        sqlx::query("UPDATE sales SET appointment_id = $1 WHERE id = $2")
            .bind(&appointment_id)
            .bind(&sale_id)
            .execute(&mut *tx)
            .await?;
        sqlx::query("UPDATE appointments SET sale_id = $1, updated_at = now() WHERE id = $2")
            .bind(&sale_id)
            .bind(&appointment_id)
            .execute(&mut *tx)
            .await?;

        tx.commit().await?;
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

    fn insert_customer(conn: &Connection, id: &str) {
        conn.execute(
            "INSERT INTO customers (id, name, phone, email, document)
             VALUES (?1, 'Cliente de prueba', '3000000000', 'cliente@test.com', 'DOC-1')",
            params![id],
        )
        .unwrap();
    }

    fn insert_published_product(conn: &Connection, id: &str, name: &str, price: f64, stock: i64) {
        conn.execute(
            "INSERT INTO products (id, name, price, status, stock)
             VALUES (?1, ?2, ?3, 'published', ?4)",
            params![id, name, price, stock],
        )
        .unwrap();
    }

    fn cash_payment(amount: f64) -> Vec<PaymentInput> {
        vec![PaymentInput {
            method: "cash".to_string(),
            amount,
        }]
    }

    fn product_item(item_id: &str, name: &str, unit_price: f64, quantity: i64) -> SaleItemInput {
        SaleItemInput {
            item_type: "product".to_string(),
            item_id: item_id.to_string(),
            name: name.to_string(),
            unit_price,
            quantity,
        }
    }

    /// Issue #2: `create_with_inventory` no debe confiar en el
    /// `unit_price`/`name` que manda el frontend — debe recalcularlos desde
    /// `products`/`services`/`packages`, para que manipular esos campos (ej.
    /// desde las devtools) no cambie el monto real cobrado.
    #[test]
    fn create_with_inventory_ignores_client_supplied_price() {
        let mut conn = open_test_db();
        insert_customer(&conn, "cust-1");
        insert_published_product(&conn, "prod-1", "Shampoo real", 50.0, 10);

        let sale_id = create_with_inventory(
            &mut conn,
            "cust-1".to_string(),
            None,
            vec![product_item("prod-1", "Nombre falso", 1.0, 2)],
            cash_payment(100.0),
            None,
        )
        .unwrap();

        let total: f64 = conn
            .query_row("SELECT total FROM sales WHERE id = ?1", params![sale_id], |r| r.get(0))
            .unwrap();
        assert_eq!(total, 100.0, "debe usar el precio real (50.0 x 2), no el manipulado (1.0 x 2)");

        let (name, unit_price): (String, f64) = conn
            .query_row(
                "SELECT name_snapshot, unit_price FROM sale_items WHERE sale_id = ?1",
                params![sale_id],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .unwrap();
        assert_eq!(name, "Shampoo real");
        assert_eq!(unit_price, 50.0);

        let stock: i64 = conn
            .query_row("SELECT stock FROM products WHERE id = 'prod-1'", [], |r| r.get(0))
            .unwrap();
        assert_eq!(stock, 8, "el stock debe descontarse por la cantidad vendida");
    }

    #[test]
    fn create_with_inventory_rejects_unpublished_item() {
        let mut conn = open_test_db();
        insert_customer(&conn, "cust-1");
        conn.execute(
            "INSERT INTO products (id, name, price, status, stock)
             VALUES ('prod-draft', 'Borrador', 10.0, 'draft', 5)",
            [],
        )
        .unwrap();

        let result = create_with_inventory(
            &mut conn,
            "cust-1".to_string(),
            None,
            vec![product_item("prod-draft", "Borrador", 10.0, 1)],
            cash_payment(10.0),
            None,
        );

        assert!(result.is_err(), "no debe poder vender un producto en borrador");
    }

    /// Issue #2, Fase 2: si la suma de los medios de pago no coincide con el
    /// total de la venta, se rechaza — no se puede "pagar de menos" mandando
    /// un desglose que no cuadra.
    #[test]
    fn create_with_inventory_rejects_payments_that_dont_match_total() {
        let mut conn = open_test_db();
        insert_customer(&conn, "cust-1");
        insert_published_product(&conn, "prod-1", "Shampoo", 50.0, 10);

        let result = create_with_inventory(
            &mut conn,
            "cust-1".to_string(),
            None,
            vec![product_item("prod-1", "Shampoo", 50.0, 1)],
            cash_payment(30.0),
            None,
        );

        assert!(result.is_err(), "30.0 pagado no cubre el total de 50.0");
    }

    /// Issue #2, Fase 2: pago mixto — varios medios cuya suma sí cubre el
    /// total se aceptan, y `payment_method` queda como "mixed".
    #[test]
    fn create_with_inventory_accepts_mixed_payment() {
        let mut conn = open_test_db();
        insert_customer(&conn, "cust-1");
        insert_published_product(&conn, "prod-1", "Shampoo", 100.0, 10);

        let sale_id = create_with_inventory(
            &mut conn,
            "cust-1".to_string(),
            None,
            vec![product_item("prod-1", "Shampoo", 100.0, 1)],
            vec![
                PaymentInput { method: "cash".to_string(), amount: 60.0 },
                PaymentInput { method: "card".to_string(), amount: 40.0 },
            ],
            None,
        )
        .unwrap();

        let payment_method: String = conn
            .query_row(
                "SELECT payment_method FROM sales WHERE id = ?1",
                params![sale_id],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(payment_method, "mixed");

        let payment_rows: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM sale_payments WHERE sale_id = ?1",
                params![sale_id],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(payment_rows, 2);
    }

    /// Issue #2, Fase 2: el descuento se resta del subtotal calculado desde
    /// el catálogo, nunca de un total que mande el frontend.
    #[test]
    fn create_with_inventory_applies_discount_to_catalog_subtotal() {
        let mut conn = open_test_db();
        insert_customer(&conn, "cust-1");
        insert_published_product(&conn, "prod-1", "Shampoo", 100.0, 10);

        let sale_id = create_with_inventory(
            &mut conn,
            "cust-1".to_string(),
            None,
            vec![product_item("prod-1", "Shampoo", 100.0, 1)],
            cash_payment(90.0),
            Some(10.0),
        )
        .unwrap();

        let (total, discount): (f64, f64) = conn
            .query_row(
                "SELECT total, discount_amount FROM sales WHERE id = ?1",
                params![sale_id],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .unwrap();
        assert_eq!(total, 90.0);
        assert_eq!(discount, 10.0);
    }

    #[test]
    fn create_with_inventory_rejects_discount_larger_than_subtotal() {
        let mut conn = open_test_db();
        insert_customer(&conn, "cust-1");
        insert_published_product(&conn, "prod-1", "Shampoo", 50.0, 10);

        let result = create_with_inventory(
            &mut conn,
            "cust-1".to_string(),
            None,
            vec![product_item("prod-1", "Shampoo", 50.0, 1)],
            cash_payment(0.0),
            Some(60.0),
        );

        assert!(result.is_err(), "el descuento no puede superar el subtotal");
    }

    /// Issue #2, Fase 2: anular una venta revierte el stock vendido y la
    /// marca como anulada; no se puede anular dos veces.
    #[test]
    fn void_sale_reverses_stock_and_marks_voided() {
        let mut conn = open_test_db();
        insert_customer(&conn, "cust-1");
        insert_published_product(&conn, "prod-1", "Shampoo", 50.0, 10);

        let sale_id = create_with_inventory(
            &mut conn,
            "cust-1".to_string(),
            None,
            vec![product_item("prod-1", "Shampoo", 50.0, 3)],
            cash_payment(150.0),
            None,
        )
        .unwrap();

        let stock_after_sale: i64 = conn
            .query_row("SELECT stock FROM products WHERE id = 'prod-1'", [], |r| r.get(0))
            .unwrap();
        assert_eq!(stock_after_sale, 7);

        void_sale(&mut conn, sale_id.clone(), Some("Cliente se arrepintió".to_string())).unwrap();

        let stock_after_void: i64 = conn
            .query_row("SELECT stock FROM products WHERE id = 'prod-1'", [], |r| r.get(0))
            .unwrap();
        assert_eq!(stock_after_void, 10, "el stock debe volver al nivel original");

        let voided_at: Option<String> = conn
            .query_row("SELECT voided_at FROM sales WHERE id = ?1", params![sale_id], |r| r.get(0))
            .unwrap();
        assert!(voided_at.is_some());

        let result = void_sale(&mut conn, sale_id, None);
        assert!(result.is_err(), "no se puede anular una venta ya anulada");
    }

    /// Issue #2, Fase 2: una venta anulada no debe contar en la caja del
    /// día ni en los ingresos.
    #[test]
    fn voided_sale_excluded_from_cash_day_summary() {
        let mut conn = open_test_db();
        insert_customer(&conn, "cust-1");
        insert_published_product(&conn, "prod-1", "Shampoo", 50.0, 10);

        let sale_id = create_with_inventory(
            &mut conn,
            "cust-1".to_string(),
            None,
            vec![product_item("prod-1", "Shampoo", 50.0, 1)],
            cash_payment(50.0),
            None,
        )
        .unwrap();
        void_sale(&mut conn, sale_id, None).unwrap();

        let summary = cash_day_summary(&mut conn, "0001-01-01T00:00:00Z".to_string()).unwrap();
        assert_eq!(summary.sales_count, 0);
        assert_eq!(summary.total_revenue, 0.0);
        assert_eq!(summary.payments.cash_count, 0);
        assert_eq!(summary.sales.len(), 1, "la venta anulada se sigue listando, solo no cuenta en los totales");
    }
}
