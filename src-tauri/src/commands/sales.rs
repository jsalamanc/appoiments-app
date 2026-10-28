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

use crate::app_db::AppDb;
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
    state.require_user()?;
    match &state.db {
        AppDb::Sqlite(_) => state.db.with_sqlite(repo::published_catalog).await,
        AppDb::Postgres(pool) => repo::postgres::published_catalog(pool).await,
    }
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
    state.require_user()?;
    match &state.db {
        AppDb::Sqlite(_) => {
            state
                .db
                .with_sqlite(move |conn| {
                    repo::create_with_inventory(conn, customer_id, payment_method, items)
                })
                .await
        }
        AppDb::Postgres(pool) => {
            repo::postgres::create_with_inventory(pool, customer_id, payment_method, items).await
        }
    }
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
    state.require_user()?;
    match &state.db {
        AppDb::Sqlite(_) => {
            state
                .db
                .with_sqlite(move |conn| repo::list(conn, page, page_size, payment_method))
                .await
        }
        AppDb::Postgres(pool) => repo::postgres::list(pool, page, page_size, payment_method).await,
    }
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

/// `from` es la medianoche local de "hoy" ya calculada por el frontend
/// (`getPeriodRange("day")` en `src/lib/sales.ts`) — ver el comentario de
/// `repositories::sales::cash_day_summary` para el porqué (issue #2: antes
/// se calculaba en el backend como `now - 24h`, una ventana móvil en UTC,
/// no el día calendario en la hora local del negocio).
#[tauri::command]
pub async fn get_cash_day_summary(state: State<'_, AppState>, from: String) -> AppResult<CashDaySummary> {
    state.require_user()?;
    match &state.db {
        AppDb::Sqlite(_) => state.db.with_sqlite(move |conn| repo::cash_day_summary(conn, from)).await,
        AppDb::Postgres(pool) => repo::postgres::cash_day_summary(pool, from).await,
    }
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

/// Muestra ingresos totales del negocio — a diferencia de `get_cash_day_summary`
/// (operativo, lo necesita quien cierra caja), esto es información financiera
/// agregada y queda solo para el dueño (issue #2: antes cualquiera podía
/// llamarlo sin sesión).
#[tauri::command]
pub async fn get_revenue_metrics(
    state: State<'_, AppState>,
    day_from: String,
    week_from: String,
    month_from: String,
) -> AppResult<RevenueMetrics> {
    state.require_owner()?;
    match &state.db {
        AppDb::Sqlite(_) => {
            state
                .db
                .with_sqlite(move |conn| repo::revenue_metrics(conn, day_from, week_from, month_from))
                .await
        }
        AppDb::Postgres(pool) => repo::postgres::revenue_metrics(pool, day_from, week_from, month_from).await,
    }
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
    state.require_user()?;
    match &state.db {
        AppDb::Sqlite(_) => {
            state
                .db
                .with_sqlite(move |conn| repo::link_to_appointment(conn, sale_id, appointment_id))
                .await
        }
        AppDb::Postgres(pool) => repo::postgres::link_to_appointment(pool, sale_id, appointment_id).await,
    }
}
