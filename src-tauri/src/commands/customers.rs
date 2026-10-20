use rusqlite::params;
use serde::{Deserialize, Serialize};
use tauri::State;

use crate::db::{new_id, now_iso};
use crate::error::AppResult;
use crate::models::customer::Customer;
use crate::state::AppState;
use crate::util::pagination::{get_range, get_total_pages, CATALOG_PAGE_SIZE};

#[derive(Debug, Serialize)]
pub struct ListCustomersResult {
    pub items: Vec<Customer>,
    pub total: i64,
    pub page: i64,
    pub total_pages: i64,
    pub page_size: i64,
    pub q: String,
}

#[tauri::command]
pub fn list_customers(
    state: State<AppState>,
    page: Option<i64>,
    page_size: Option<i64>,
    q: Option<String>,
) -> AppResult<ListCustomersResult> {
    let conn = state.db.lock().unwrap();
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

#[derive(Debug, Serialize)]
pub struct CustomerRef {
    pub id: String,
    pub name: String,
    pub email: String,
    pub phone: String,
    pub document: String,
}

#[derive(Debug, Serialize)]
pub struct SaleItemRow {
    pub id: String,
    pub sale_id: String,
    pub item_type: String,
    pub product_id: Option<String>,
    pub service_id: Option<String>,
    pub package_id: Option<String>,
    pub name_snapshot: String,
    pub unit_price: f64,
    pub quantity: i64,
    pub line_total: f64,
    pub created_at: String,
}

#[derive(Debug, Serialize)]
pub struct SaleWithDetails {
    pub id: String,
    pub customer_id: String,
    pub appointment_id: Option<String>,
    pub payment_method: String,
    pub total: f64,
    pub sold_at: String,
    pub created_at: String,
    pub customers: CustomerRef,
    pub sale_items: Vec<SaleItemRow>,
}

#[derive(Debug, Serialize)]
pub struct EmployeeRef {
    pub id: String,
    pub name: String,
    pub color: String,
    pub active: bool,
}

#[derive(Debug, Serialize)]
pub struct ServiceRef {
    pub id: String,
    pub name: String,
    pub price: f64,
    pub duration_minutes: i64,
    pub status: String,
}

#[derive(Debug, Serialize)]
pub struct AppointmentWithDetails {
    pub id: String,
    pub employee_id: String,
    pub customer_id: String,
    pub service_id: Option<String>,
    pub service_name_snapshot: String,
    pub starts_at: String,
    pub ends_at: String,
    pub status: String,
    pub notes: Option<String>,
    pub sale_id: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub employees: EmployeeRef,
    pub customers: CustomerRef,
    pub services: Option<ServiceRef>,
}

#[derive(Debug, Serialize)]
pub struct CustomerDetail {
    pub customer: Customer,
    pub sales: Vec<SaleWithDetails>,
    pub appointments: Vec<AppointmentWithDetails>,
}

#[tauri::command]
pub fn get_customer_detail(
    state: State<AppState>,
    id: String,
) -> AppResult<Option<CustomerDetail>> {
    let conn = state.db.lock().unwrap();

    let customer = match conn.query_row(
        "SELECT * FROM customers WHERE id = ?1",
        params![id],
        Customer::from_row,
    ) {
        Ok(c) => c,
        Err(rusqlite::Error::QueryReturnedNoRows) => return Ok(None),
        Err(e) => return Err(e.into()),
    };

    let customer_ref = CustomerRef {
        id: customer.id.clone(),
        name: customer.name.clone(),
        email: customer.email.clone(),
        phone: customer.phone.clone(),
        document: customer.document.clone(),
    };

    let mut sales_stmt = conn.prepare(
        "SELECT * FROM sales WHERE customer_id = ?1 ORDER BY sold_at DESC LIMIT 20",
    )?;
    let sale_rows: Vec<(String, Option<String>, String, f64, String, String)> = sales_stmt
        .query_map(params![id], |row| {
            Ok((
                row.get::<_, String>("id")?,
                row.get::<_, Option<String>>("appointment_id")?,
                row.get::<_, String>("payment_method")?,
                row.get::<_, f64>("total")?,
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
            customers: CustomerRef {
                id: customer_ref.id.clone(),
                name: customer_ref.name.clone(),
                email: customer_ref.email.clone(),
                phone: customer_ref.phone.clone(),
                document: customer_ref.document.clone(),
            },
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
                customers: CustomerRef {
                    id: customer_ref.id.clone(),
                    name: customer_ref.name.clone(),
                    email: customer_ref.email.clone(),
                    phone: customer_ref.phone.clone(),
                    document: customer_ref.document.clone(),
                },
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

#[derive(Debug, Deserialize)]
pub struct CustomerInput {
    pub name: String,
    pub phone: String,
    pub email: String,
    pub document: String,
}

fn normalize_email(email: &str) -> String {
    email.trim().to_lowercase()
}

#[tauri::command]
pub fn create_customer(state: State<AppState>, input: CustomerInput) -> AppResult<Customer> {
    let conn = state.db.lock().unwrap();
    let id = new_id();
    let now = now_iso();
    let email = normalize_email(&input.email);
    let document = input.document.trim().to_string();

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

#[tauri::command]
pub fn update_customer(
    state: State<AppState>,
    id: String,
    input: CustomerInput,
) -> AppResult<Customer> {
    let conn = state.db.lock().unwrap();
    let email = normalize_email(&input.email);
    let document = input.document.trim().to_string();

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

#[tauri::command]
pub fn find_customer_by_document(
    state: State<AppState>,
    document: String,
) -> AppResult<Option<Customer>> {
    let conn = state.db.lock().unwrap();
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

#[tauri::command]
pub fn find_customer_by_email(
    state: State<AppState>,
    email: String,
) -> AppResult<Option<Customer>> {
    let conn = state.db.lock().unwrap();
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

#[tauri::command]
pub fn find_customer_by_id(
    state: State<AppState>,
    id: String,
) -> AppResult<Option<Customer>> {
    let conn = state.db.lock().unwrap();
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
