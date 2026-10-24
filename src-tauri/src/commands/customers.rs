use serde::{Deserialize, Serialize};
use tauri::State;

use crate::error::AppResult;
use crate::models::customer::Customer;
use crate::repositories::customers as repo;
use crate::state::AppState;

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
pub async fn list_customers(
    state: State<'_, AppState>,
    page: Option<i64>,
    page_size: Option<i64>,
    q: Option<String>,
) -> AppResult<ListCustomersResult> {
    state
        .db
        .with_sqlite(move |conn| repo::list(conn, page, page_size, q))
        .await
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
pub async fn get_customer_detail(
    state: State<'_, AppState>,
    id: String,
) -> AppResult<Option<CustomerDetail>> {
    state
        .db
        .with_sqlite(move |conn| repo::get_detail(conn, id))
        .await
}

#[derive(Debug, Deserialize)]
pub struct CustomerInput {
    pub name: String,
    pub phone: String,
    pub email: String,
    pub document: String,
}

#[tauri::command]
pub async fn create_customer(
    state: State<'_, AppState>,
    input: CustomerInput,
) -> AppResult<Customer> {
    state.db.with_sqlite(move |conn| repo::create(conn, input)).await
}

#[tauri::command]
pub async fn update_customer(
    state: State<'_, AppState>,
    id: String,
    input: CustomerInput,
) -> AppResult<Customer> {
    state
        .db
        .with_sqlite(move |conn| repo::update(conn, id, input))
        .await
}

#[tauri::command]
pub async fn find_customer_by_document(
    state: State<'_, AppState>,
    document: String,
) -> AppResult<Option<Customer>> {
    state
        .db
        .with_sqlite(move |conn| repo::find_by_document(conn, document))
        .await
}

#[tauri::command]
pub async fn find_customer_by_email(
    state: State<'_, AppState>,
    email: String,
) -> AppResult<Option<Customer>> {
    state
        .db
        .with_sqlite(move |conn| repo::find_by_email(conn, email))
        .await
}

#[tauri::command]
pub async fn find_customer_by_id(
    state: State<'_, AppState>,
    id: String,
) -> AppResult<Option<Customer>> {
    state
        .db
        .with_sqlite(move |conn| repo::find_by_id(conn, id))
        .await
}
