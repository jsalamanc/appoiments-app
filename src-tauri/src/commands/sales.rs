//! Dominio `sales`.
//!
//! NOTA DE COORDINACIÓN: `get_published_catalog` (abajo) pertenece al
//! dominio catálogo/inventario (otro ingeniero, en paralelo) — se conserva
//! tal cual aquí porque comparte este mismo archivo. El resto de los
//! comandos (`create_sale_with_inventory`, `list_sales`,
//! `get_cash_day_summary`, `get_revenue_metrics`,
//! `link_sale_to_appointment`) son del dominio `sales` propiamente dicho.

use serde::{Deserialize, Serialize};
use tauri::State;

use crate::error::AppResult;
use crate::models::package::Package;
use crate::models::product::Product;
use crate::models::sale::SaleWithDetails;
use crate::models::service::Service;
use crate::repositories::sales as repo;
use crate::state::AppState;

// ---------------------------------------------------------------------------
// get_published_catalog (dominio catálogo/inventario — no tocar la lógica)
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize)]
pub struct PublishedCatalog {
    pub products: Vec<Product>,
    pub services: Vec<Service>,
    pub packages: Vec<Package>,
}

#[tauri::command]
pub async fn get_published_catalog(state: State<'_, AppState>) -> AppResult<PublishedCatalog> {
    state.db.with_sqlite(repo::published_catalog).await
}

// ---------------------------------------------------------------------------
// create_sale_with_inventory
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
pub struct SaleItemInput {
    pub item_type: String,
    pub item_id: String,
    pub name: String,
    pub unit_price: f64,
    pub quantity: i64,
}

#[tauri::command]
pub async fn create_sale_with_inventory(
    state: State<'_, AppState>,
    customer_id: String,
    payment_method: String,
    items: Vec<SaleItemInput>,
) -> AppResult<String> {
    state
        .db
        .with_sqlite(move |conn| repo::create_with_inventory(conn, customer_id, payment_method, items))
        .await
}

// ---------------------------------------------------------------------------
// list_sales
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize)]
pub struct ListSalesResult {
    pub items: Vec<SaleWithDetails>,
    pub total: i64,
    pub page: i64,
    pub total_pages: i64,
    pub page_size: i64,
    pub payment_method: String,
}

#[tauri::command]
pub async fn list_sales(
    state: State<'_, AppState>,
    page: Option<i64>,
    page_size: Option<i64>,
    payment_method: Option<String>,
) -> AppResult<ListSalesResult> {
    state
        .db
        .with_sqlite(move |conn| repo::list(conn, page, page_size, payment_method))
        .await
}

// ---------------------------------------------------------------------------
// get_cash_day_summary
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize)]
pub struct PaymentBreakdown {
    pub cash_count: i64,
    pub transfer_count: i64,
    pub cash_revenue: f64,
    pub transfer_revenue: f64,
}

#[derive(Debug, Serialize)]
pub struct CashDaySummary {
    pub sales: Vec<SaleWithDetails>,
    pub sales_count: i64,
    pub total_revenue: f64,
    pub payments: PaymentBreakdown,
    pub from: String,
}

#[tauri::command]
pub async fn get_cash_day_summary(state: State<'_, AppState>) -> AppResult<CashDaySummary> {
    state.db.with_sqlite(repo::cash_day_summary).await
}

// ---------------------------------------------------------------------------
// get_revenue_metrics
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize)]
pub struct PaymentPeriods {
    pub day: PaymentBreakdown,
    pub week: PaymentBreakdown,
    pub month: PaymentBreakdown,
    pub all: PaymentBreakdown,
}

#[derive(Debug, Serialize)]
pub struct RevenueMetrics {
    pub total_revenue: f64,
    pub day_revenue: f64,
    pub week_revenue: f64,
    pub month_revenue: f64,
    pub sales_count: i64,
    pub payments: PaymentPeriods,
}

#[tauri::command]
pub async fn get_revenue_metrics(state: State<'_, AppState>) -> AppResult<RevenueMetrics> {
    state.db.with_sqlite(repo::revenue_metrics).await
}

// ---------------------------------------------------------------------------
// link_sale_to_appointment
// ---------------------------------------------------------------------------

#[tauri::command]
pub async fn link_sale_to_appointment(
    state: State<'_, AppState>,
    sale_id: String,
    appointment_id: String,
) -> AppResult<()> {
    state
        .db
        .with_sqlite(move |conn| repo::link_to_appointment(conn, sale_id, appointment_id))
        .await
}
