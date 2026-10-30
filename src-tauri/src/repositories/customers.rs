use rusqlite::{params, Connection};

use crate::commands::customers::{
    AppointmentWithDetails, CustomerDetail, CustomerInput, CustomerRef, EmployeeRef,
    ListCustomersResult, SaleItemRow, SaleWithDetails, ServiceRef,
};
use crate::db::{new_id, now_iso};
use crate::error::AppResult;
use crate::models::customer::Customer;
use crate::util::pagination::{get_range, get_total_pages, CATALOG_PAGE_SIZE};

pub fn list(
    conn: &mut Connection,
    page: Option<i64>,
    page_size: Option<i64>,
    q: Option<String>,
) -> AppResult<ListCustomersResult> {
    let page = page.unwrap_or(1).max(1);
    let page_size = page_size.unwrap_or(CATALOG_PAGE_SIZE);
    let q = q.unwrap_or_default().trim().to_string();

    let (total, items_unfiltered_sql_filter) = if q.is_empty() {
        let total: i64 = conn.query_row("SELECT COUNT(*) FROM customers", [], |r| r.get(0))?;
        (total, false)
    } else {
        let pattern = format!("%{}%", q.to_lowercase());
        let total: i64 = conn.query_row(
            "SELECT COUNT(*) FROM customers WHERE
                lower(name) LIKE ?1 OR lower(email) LIKE ?1 OR
                lower(document) LIKE ?1 OR lower(phone) LIKE ?1",
            params![pattern],
            |r| r.get(0),
        )?;
        (total, true)
    };

    let total_pages = get_total_pages(total, page_size);
    let safe_page = page.min(total_pages);
    let (limit, offset) = get_range(safe_page, page_size);

    let items = if items_unfiltered_sql_filter {
        let pattern = format!("%{}%", q.to_lowercase());
        let mut stmt = conn.prepare(
            "SELECT * FROM customers WHERE
                lower(name) LIKE ?1 OR lower(email) LIKE ?1 OR
                lower(document) LIKE ?1 OR lower(phone) LIKE ?1
             ORDER BY name ASC LIMIT ?2 OFFSET ?3",
        )?;
        let rows = stmt.query_map(params![pattern, limit, offset], Customer::from_row)?;
        rows.collect::<Result<Vec<_>, _>>()?
    } else {
        let mut stmt =
            conn.prepare("SELECT * FROM customers ORDER BY name ASC LIMIT ?1 OFFSET ?2")?;
        let rows = stmt.query_map(params![limit, offset], Customer::from_row)?;
        rows.collect::<Result<Vec<_>, _>>()?
    };

    Ok(ListCustomersResult {
        items,
        total,
        page: safe_page,
        total_pages,
        page_size,
        q,
    })
}

pub fn get_detail(conn: &mut Connection, id: String) -> AppResult<Option<CustomerDetail>> {
    let customer = match conn.query_row(
        "SELECT * FROM customers WHERE id = ?1",
        params![id],
        Customer::from_row,
    ) {
        Ok(c) => c,
        Err(rusqlite::Error::QueryReturnedNoRows) => return Ok(None),
        Err(e) => return Err(e.into()),
    };

    let customer_ref = to_customer_ref(&customer);

    let mut sales_stmt =
        conn.prepare("SELECT * FROM sales WHERE customer_id = ?1 ORDER BY sold_at DESC LIMIT 20")?;
    let sale_rows: Vec<(String, Option<String>, String, i64, String, String)> = sales_stmt
        .query_map(params![id], |row| {
            Ok((
                row.get::<_, String>("id")?,
                row.get::<_, Option<String>>("appointment_id")?,
                row.get::<_, String>("payment_method")?,
                row.get::<_, i64>("total")?,
                row.get::<_, String>("sold_at")?,
                row.get::<_, String>("created_at")?,
            ))
        })?
        .collect::<Result<Vec<_>, _>>()?;

    let mut sales = Vec::new();
    for (sale_id, appointment_id, payment_method, total, sold_at, created_at) in sale_rows {
        let mut items_stmt =
            conn.prepare("SELECT * FROM sale_items WHERE sale_id = ?1 ORDER BY created_at ASC")?;
        let items = items_stmt
            .query_map(params![sale_id], |row| {
                Ok(SaleItemRow {
                    id: row.get("id")?,
                    sale_id: row.get("sale_id")?,
                    item_type: row.get("item_type")?,
                    product_id: row.get("product_id")?,
                    service_id: row.get("service_id")?,
                    package_id: row.get("package_id")?,
                    name_snapshot: row.get("name_snapshot")?,
                    unit_price: row.get("unit_price")?,
                    quantity: row.get("quantity")?,
                    line_total: row.get("line_total")?,
                    created_at: row.get("created_at")?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        sales.push(SaleWithDetails {
            id: sale_id,
            customer_id: id.clone(),
            appointment_id,
            payment_method,
            total,
            sold_at,
            created_at,
            customers: customer_ref.clone(),
            sale_items: items,
        });
    }

    let mut appt_stmt = conn.prepare(
        "SELECT a.*, e.name as emp_name, e.color as emp_color, e.active as emp_active,
                s.price as svc_price, s.duration_minutes as svc_duration, s.status as svc_status
         FROM appointments a
         JOIN employees e ON e.id = a.employee_id
         LEFT JOIN services s ON s.id = a.service_id
         WHERE a.customer_id = ?1
         ORDER BY a.starts_at DESC LIMIT 20",
    )?;
    let appointments = appt_stmt
        .query_map(params![id], |row| {
            let service_id: Option<String> = row.get("service_id")?;
            let services = if service_id.is_some() {
                Some(ServiceRef {
                    id: service_id.clone().unwrap(),
                    name: row.get("service_name_snapshot")?,
                    price: row.get("svc_price")?,
                    duration_minutes: row.get("svc_duration")?,
                    status: row.get("svc_status")?,
                })
            } else {
                None
            };
            Ok(AppointmentWithDetails {
                id: row.get("id")?,
                employee_id: row.get("employee_id")?,
                customer_id: row.get("customer_id")?,
                service_id,
                service_name_snapshot: row.get("service_name_snapshot")?,
                starts_at: row.get("starts_at")?,
                ends_at: row.get("ends_at")?,
                status: row.get("status")?,
                notes: row.get("notes")?,
                sale_id: row.get("sale_id")?,
                created_at: row.get("created_at")?,
                updated_at: row.get("updated_at")?,
                employees: EmployeeRef {
                    id: row.get("employee_id")?,
                    name: row.get("emp_name")?,
                    color: row.get("emp_color")?,
                    active: row.get::<_, i64>("emp_active")? != 0,
                },
                customers: customer_ref.clone(),
                services,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;

    Ok(Some(CustomerDetail {
        customer,
        sales,
        appointments,
    }))
}

/// Vacío (tras recortar espacios) se trata igual que ausente — así un campo
/// de formulario dejado en blanco se guarda como `NULL`, no como cadena
/// vacía (que violaría el `CHECK` de formato de email y rompería la
/// comparación de unicidad "ignorando mayúsculas" para el documento).
fn normalize_optional(value: &Option<String>) -> Option<String> {
    let trimmed = value.as_deref().unwrap_or("").trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

fn normalize_optional_email(value: &Option<String>) -> Option<String> {
    normalize_optional(value).map(|e| e.to_lowercase())
}

/// Vista mínima de `customer` para embeber en otros DTOs — `CustomerRef`
/// sigue exigiendo `String` (no `Option<String>`) porque cambiar eso se
/// propagaría a cada `join` que embebe datos de cliente (citas, ventas,
/// reportes); en su lugar, esos `String` simplemente quedan vacíos cuando el
/// cliente no tiene correo/documento (ver los `COALESCE(..., '')` en
/// `repositories::{appointments,sales,reports}`).
fn to_customer_ref(customer: &Customer) -> CustomerRef {
    CustomerRef {
        id: customer.id.clone(),
        name: customer.name.clone(),
        email: customer.email.clone().unwrap_or_default(),
        phone: customer.phone.clone(),
        document: customer.document.clone().unwrap_or_default(),
    }
}

fn normalize_email(email: &str) -> String {
    email.trim().to_lowercase()
}

pub fn create(conn: &mut Connection, input: CustomerInput) -> AppResult<Customer> {
    let id = new_id();
    let now = now_iso();
    let email = normalize_optional_email(&input.email);
    let document = normalize_optional(&input.document);

    conn.execute(
        "INSERT INTO customers (id, name, phone, email, document, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?6)",
        params![id, input.name.trim(), input.phone.trim(), email, document, now],
    )?;

    let customer = conn.query_row(
        "SELECT * FROM customers WHERE id = ?1",
        params![id],
        Customer::from_row,
    )?;
    Ok(customer)
}

pub fn update(conn: &mut Connection, id: String, input: CustomerInput) -> AppResult<Customer> {
    let email = normalize_optional_email(&input.email);
    let document = normalize_optional(&input.document);

    conn.execute(
        "UPDATE customers SET name = ?1, phone = ?2, email = ?3, document = ?4, updated_at = ?5
         WHERE id = ?6",
        params![input.name.trim(), input.phone.trim(), email, document, now_iso(), id],
    )?;

    let customer = conn.query_row(
        "SELECT * FROM customers WHERE id = ?1",
        params![id],
        Customer::from_row,
    )?;
    Ok(customer)
}

pub fn find_by_document(conn: &mut Connection, document: String) -> AppResult<Option<Customer>> {
    let normalized = document.trim();
    if normalized.is_empty() {
        return Ok(None);
    }
    let result = conn.query_row(
        "SELECT * FROM customers WHERE lower(document) = ?1",
        params![normalized.to_lowercase()],
        Customer::from_row,
    );
    match result {
        Ok(c) => Ok(Some(c)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(e.into()),
    }
}

pub fn find_by_email(conn: &mut Connection, email: String) -> AppResult<Option<Customer>> {
    let normalized = normalize_email(&email);
    if normalized.is_empty() {
        return Ok(None);
    }
    let result = conn.query_row(
        "SELECT * FROM customers WHERE lower(email) = ?1",
        params![normalized],
        Customer::from_row,
    );
    match result {
        Ok(c) => Ok(Some(c)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(e.into()),
    }
}

pub fn find_by_id(conn: &mut Connection, id: String) -> AppResult<Option<Customer>> {
    let result = conn.query_row(
        "SELECT * FROM customers WHERE id = ?1",
        params![id],
        Customer::from_row,
    );
    match result {
        Ok(c) => Ok(Some(c)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(e.into()),
    }
}

/// Implementación PostgreSQL (issue #9, Fase 2) — mismas reglas que arriba,
/// adaptadas al driver async `sqlx`. Las columnas de texto/booleano decodifican
/// igual que en SQLite; las de timestamp (`TIMESTAMPTZ`) se decodifican como
/// `time::OffsetDateTime` y se formatean a RFC3339 con
/// `db::format_pg_timestamp` para que los modelos sigan viendo un `String`
/// idéntico sin importar el backend activo. Se usa `RETURNING` en los
/// `INSERT`/`UPDATE` para evitar una segunda consulta de relectura.
pub mod postgres {
    use sqlx::postgres::PgRow;
    use sqlx::{PgPool, Row};

    use crate::commands::customers::{
        AppointmentWithDetails, CustomerDetail, CustomerInput, EmployeeRef,
        ListCustomersResult, SaleItemRow, SaleWithDetails, ServiceRef,
    };
    use crate::db::{format_pg_timestamp, new_id};
    use crate::error::{AppError, AppResult};
    use crate::models::customer::Customer;
    use crate::util::pagination::{get_range, get_total_pages, CATALOG_PAGE_SIZE};

    fn ts(row: &PgRow, col: &str) -> AppResult<String> {
        let value: time::OffsetDateTime = row.try_get(col)?;
        Ok(format_pg_timestamp(value))
    }

    fn map_customer(row: &PgRow) -> AppResult<Customer> {
        Ok(Customer {
            id: row.try_get("id")?,
            name: row.try_get("name")?,
            phone: row.try_get("phone")?,
            email: row.try_get("email")?,
            document: row.try_get("document")?,
            created_at: ts(row, "created_at")?,
            updated_at: ts(row, "updated_at")?,
        })
    }

    use super::{normalize_optional, normalize_optional_email};

    fn normalize_email(email: &str) -> String {
        email.trim().to_lowercase()
    }

    const CUSTOMER_COLUMNS: &str = "id, name, phone, email, document, created_at, updated_at";

    pub async fn list(
        pool: &PgPool,
        page: Option<i64>,
        page_size: Option<i64>,
        q: Option<String>,
    ) -> AppResult<ListCustomersResult> {
        let page = page.unwrap_or(1).max(1);
        let page_size = page_size.unwrap_or(CATALOG_PAGE_SIZE);
        let q = q.unwrap_or_default().trim().to_string();

        let (total, filtered): (i64, bool) = if q.is_empty() {
            let total: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM customers")
                .fetch_one(pool)
                .await?;
            (total, false)
        } else {
            let pattern = format!("%{}%", q.to_lowercase());
            let total: i64 = sqlx::query_scalar(
                "SELECT COUNT(*) FROM customers WHERE
                    lower(name) LIKE $1 OR lower(email) LIKE $1 OR
                    lower(document) LIKE $1 OR lower(phone) LIKE $1",
            )
            .bind(pattern)
            .fetch_one(pool)
            .await?;
            (total, true)
        };

        let total_pages = get_total_pages(total, page_size);
        let safe_page = page.min(total_pages);
        let (limit, offset) = get_range(safe_page, page_size);

        let rows = if filtered {
            let pattern = format!("%{}%", q.to_lowercase());
            sqlx::query(&format!(
                "SELECT {CUSTOMER_COLUMNS} FROM customers WHERE
                    lower(name) LIKE $1 OR lower(email) LIKE $1 OR
                    lower(document) LIKE $1 OR lower(phone) LIKE $1
                 ORDER BY name ASC LIMIT $2 OFFSET $3"
            ))
            .bind(pattern)
            .bind(limit)
            .bind(offset)
            .fetch_all(pool)
            .await?
        } else {
            sqlx::query(&format!(
                "SELECT {CUSTOMER_COLUMNS} FROM customers ORDER BY name ASC LIMIT $1 OFFSET $2"
            ))
            .bind(limit)
            .bind(offset)
            .fetch_all(pool)
            .await?
        };

        let items = rows.iter().map(map_customer).collect::<AppResult<Vec<_>>>()?;

        Ok(ListCustomersResult {
            items,
            total,
            page: safe_page,
            total_pages,
            page_size,
            q,
        })
    }

    pub async fn get_detail(pool: &PgPool, id: String) -> AppResult<Option<CustomerDetail>> {
        let customer_row = sqlx::query(&format!(
            "SELECT {CUSTOMER_COLUMNS} FROM customers WHERE id = $1"
        ))
        .bind(&id)
        .fetch_optional(pool)
        .await?;

        let Some(customer_row) = customer_row else {
            return Ok(None);
        };
        let customer = map_customer(&customer_row)?;

        let customer_ref = super::to_customer_ref(&customer);

        let sale_rows = sqlx::query(
            "SELECT id, appointment_id, payment_method, total, sold_at, created_at
             FROM sales WHERE customer_id = $1 ORDER BY sold_at DESC LIMIT 20",
        )
        .bind(&id)
        .fetch_all(pool)
        .await?;

        let mut sales = Vec::new();
        for row in &sale_rows {
            let sale_id: String = row.try_get("id")?;
            let appointment_id: Option<String> = row.try_get("appointment_id")?;
            let payment_method: String = row.try_get("payment_method")?;
            let total: i64 = row.try_get("total")?;
            let sold_at = ts(row, "sold_at")?;
            let created_at = ts(row, "created_at")?;

            let item_rows = sqlx::query(
                "SELECT id, sale_id, item_type, product_id, service_id, package_id,
                        name_snapshot, unit_price, quantity, line_total, created_at
                 FROM sale_items WHERE sale_id = $1 ORDER BY created_at ASC",
            )
            .bind(&sale_id)
            .fetch_all(pool)
            .await?;

            let items = item_rows
                .iter()
                .map(|r| -> AppResult<SaleItemRow> {
                    Ok(SaleItemRow {
                        id: r.try_get("id")?,
                        sale_id: r.try_get("sale_id")?,
                        item_type: r.try_get("item_type")?,
                        product_id: r.try_get("product_id")?,
                        service_id: r.try_get("service_id")?,
                        package_id: r.try_get("package_id")?,
                        name_snapshot: r.try_get("name_snapshot")?,
                        unit_price: r.try_get("unit_price")?,
                        quantity: r.try_get("quantity")?,
                        line_total: r.try_get("line_total")?,
                        created_at: ts(r, "created_at")?,
                    })
                })
                .collect::<AppResult<Vec<_>>>()?;

            sales.push(SaleWithDetails {
                id: sale_id,
                customer_id: id.clone(),
                appointment_id,
                payment_method,
                total,
                sold_at,
                created_at,
                customers: customer_ref.clone(),
                sale_items: items,
            });
        }

        let appt_rows = sqlx::query(
            "SELECT a.id, a.employee_id, a.customer_id, a.service_id, a.service_name_snapshot,
                    a.starts_at, a.ends_at, a.status, a.notes, a.sale_id, a.created_at, a.updated_at,
                    e.name as emp_name, e.color as emp_color, e.active as emp_active,
                    s.price as svc_price, s.duration_minutes as svc_duration, s.status as svc_status
             FROM appointments a
             JOIN employees e ON e.id = a.employee_id
             LEFT JOIN services s ON s.id = a.service_id
             WHERE a.customer_id = $1
             ORDER BY a.starts_at DESC LIMIT 20",
        )
        .bind(&id)
        .fetch_all(pool)
        .await?;

        let mut appointments = Vec::new();
        for row in &appt_rows {
            let service_id: Option<String> = row.try_get("service_id")?;
            let services = if service_id.is_some() {
                Some(ServiceRef {
                    id: service_id.clone().unwrap(),
                    name: row.try_get("service_name_snapshot")?,
                    price: row.try_get("svc_price")?,
                    duration_minutes: row.try_get("svc_duration")?,
                    status: row.try_get("svc_status")?,
                })
            } else {
                None
            };

            appointments.push(AppointmentWithDetails {
                id: row.try_get("id")?,
                employee_id: row.try_get("employee_id")?,
                customer_id: row.try_get("customer_id")?,
                service_id,
                service_name_snapshot: row.try_get("service_name_snapshot")?,
                starts_at: ts(row, "starts_at")?,
                ends_at: ts(row, "ends_at")?,
                status: row.try_get("status")?,
                notes: row.try_get("notes")?,
                sale_id: row.try_get("sale_id")?,
                created_at: ts(row, "created_at")?,
                updated_at: ts(row, "updated_at")?,
                employees: EmployeeRef {
                    id: row.try_get("employee_id")?,
                    name: row.try_get("emp_name")?,
                    color: row.try_get("emp_color")?,
                    active: row.try_get("emp_active")?,
                },
                customers: customer_ref.clone(),
                services,
            });
        }

        Ok(Some(CustomerDetail {
            customer,
            sales,
            appointments,
        }))
    }

    pub async fn create(pool: &PgPool, input: CustomerInput) -> AppResult<Customer> {
        let email = normalize_optional_email(&input.email);
        let document = normalize_optional(&input.document);

        let row = sqlx::query(&format!(
            "INSERT INTO customers (id, name, phone, email, document)
             VALUES ($1, $2, $3, $4, $5)
             RETURNING {CUSTOMER_COLUMNS}"
        ))
        .bind(new_id())
        .bind(input.name.trim())
        .bind(input.phone.trim())
        .bind(&email)
        .bind(&document)
        .fetch_one(pool)
        .await?;

        map_customer(&row)
    }

    pub async fn update(pool: &PgPool, id: String, input: CustomerInput) -> AppResult<Customer> {
        let email = normalize_optional_email(&input.email);
        let document = normalize_optional(&input.document);

        let row = sqlx::query(&format!(
            "UPDATE customers SET name = $1, phone = $2, email = $3, document = $4, updated_at = now()
             WHERE id = $5
             RETURNING {CUSTOMER_COLUMNS}"
        ))
        .bind(input.name.trim())
        .bind(input.phone.trim())
        .bind(&email)
        .bind(&document)
        .bind(&id)
        .fetch_optional(pool)
        .await?
        .ok_or(AppError::NotFound)?;

        map_customer(&row)
    }

    pub async fn find_by_document(
        pool: &PgPool,
        document: String,
    ) -> AppResult<Option<Customer>> {
        let normalized = document.trim();
        if normalized.is_empty() {
            return Ok(None);
        }
        let row = sqlx::query(&format!(
            "SELECT {CUSTOMER_COLUMNS} FROM customers WHERE lower(document) = $1"
        ))
        .bind(normalized.to_lowercase())
        .fetch_optional(pool)
        .await?;

        row.as_ref().map(map_customer).transpose()
    }

    pub async fn find_by_email(pool: &PgPool, email: String) -> AppResult<Option<Customer>> {
        let normalized = normalize_email(&email);
        if normalized.is_empty() {
            return Ok(None);
        }
        let row = sqlx::query(&format!(
            "SELECT {CUSTOMER_COLUMNS} FROM customers WHERE lower(email) = $1"
        ))
        .bind(normalized)
        .fetch_optional(pool)
        .await?;

        row.as_ref().map(map_customer).transpose()
    }

    pub async fn find_by_id(pool: &PgPool, id: String) -> AppResult<Option<Customer>> {
        let row = sqlx::query(&format!("SELECT {CUSTOMER_COLUMNS} FROM customers WHERE id = $1"))
            .bind(id)
            .fetch_optional(pool)
            .await?;

        row.as_ref().map(map_customer).transpose()
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

    fn input(name: &str, email: Option<&str>, document: Option<&str>) -> CustomerInput {
        CustomerInput {
            name: name.to_string(),
            phone: "3000000000".to_string(),
            email: email.map(str::to_string),
            document: document.map(str::to_string),
        }
    }

    #[test]
    fn create_allows_missing_email_and_document() {
        let mut conn = open_test_db();

        let customer = create(&mut conn, input("Cliente de paso", None, None)).unwrap();

        assert_eq!(customer.email, None);
        assert_eq!(customer.document, None);
    }

    #[test]
    fn create_allows_multiple_customers_without_email_or_document() {
        let mut conn = open_test_db();

        create(&mut conn, input("Cliente 1", None, None)).unwrap();
        let result = create(&mut conn, input("Cliente 2", None, None));

        assert!(result.is_ok(), "dos clientes sin correo/documento no deben chocar entre sí");
    }

    #[test]
    fn create_treats_blank_email_as_missing() {
        let mut conn = open_test_db();

        let customer = create(&mut conn, input("Cliente", Some("   "), None)).unwrap();

        assert_eq!(customer.email, None, "un email en blanco debe guardarse como NULL, no como cadena vacía");
    }

    #[test]
    fn create_still_rejects_duplicate_email_when_present() {
        let mut conn = open_test_db();
        create(&mut conn, input("Cliente 1", Some("ana@test.com"), None)).unwrap();

        let result = create(&mut conn, input("Cliente 2", Some("ANA@test.com"), None));

        assert!(result.is_err(), "la unicidad de email debe seguir aplicando (sin distinguir mayúsculas) cuando sí hay valor");
    }

    #[test]
    fn create_still_rejects_duplicate_document_when_present() {
        let mut conn = open_test_db();
        create(&mut conn, input("Cliente 1", None, Some("DOC-1"))).unwrap();

        let result = create(&mut conn, input("Cliente 2", None, Some("doc-1")));

        assert!(result.is_err(), "la unicidad de documento debe seguir aplicando cuando sí hay valor");
    }

    #[test]
    fn create_rejects_malformed_email() {
        let mut conn = open_test_db();

        let result = create(&mut conn, input("Cliente", Some("no-es-un-email"), None));

        assert!(result.is_err(), "un email presente sigue debiendo tener formato válido");
    }
}
