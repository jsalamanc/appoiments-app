use serde::Serialize;
use tauri::State;

use crate::app_db::AppDb;
use crate::error::AppResult;
use crate::repositories::reports as repo;
use crate::state::AppState;

// ---------------------------------------------------------------------
// get_employee_performance_report
// ---------------------------------------------------------------------

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EmployeePerformanceRow {
    pub employee_id: String,
    pub name: String,
    pub color: String,
    pub scheduled: i64,
    pub completed: i64,
    pub cancelled: i64,
    pub no_show: i64,
    pub total_appointments: i64,
    pub revenue: i64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EmployeePerformanceReport {
    pub period: String,
    pub from: String,
    pub rows: Vec<EmployeePerformanceRow>,
}

/// Puerto de `getEmployeePerformance` (src/lib/reports-server.ts).
///
/// Nota: el JS original NO filtra por `employees.active` (selecciona el
/// campo pero nunca lo usa para filtrar ni lo expone en
/// `EmployeePerformanceRow`), así que esta consulta tampoco filtra —
/// replica el comportamiento real observado, no la descripción informal
/// de "solo empleados activos".
#[tauri::command]
pub async fn get_employee_performance_report(
    state: State<'_, AppState>,
    period: String,
    from: String,
) -> AppResult<EmployeePerformanceReport> {
    state.require_owner()?;
    match &state.db {
        AppDb::Sqlite(_) => {
            state
                .db
                .with_sqlite(move |conn| repo::employee_performance(conn, period, from))
                .await
        }
        AppDb::Postgres(pool) => repo::postgres::employee_performance(pool, period, from).await,
    }
}

// ---------------------------------------------------------------------
// get_employee_commissions_report
// ---------------------------------------------------------------------

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EmployeeCommissionRow {
    pub employee_id: String,
    pub name: String,
    pub color: String,
    pub completed_appointments: i64,
    pub revenue: i64,
    /// A diferencia de `revenue`, queda como `f64`: una comisión por
    /// porcentaje (`rate_type = "percentage"`) puede dar un resultado
    /// fraccionario (ej. 12.5% de $1.001.000) — no es un monto guardado en
    /// la base de datos, solo un cálculo de reporte.
    pub commission_amount: f64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EmployeeCommissionsReport {
    pub period: String,
    pub from: String,
    pub total_commission: f64,
    pub rows: Vec<EmployeeCommissionRow>,
}

/// Issue #22 / #2 Fase 4. Reusa el mismo universo de citas que
/// `get_employee_performance_report` (citas completadas con venta vinculada
/// no anulada) para calcular cuánto se le debe a cada empleado según las
/// tasas configuradas en `commands::commissions`.
#[tauri::command]
pub async fn get_employee_commissions_report(
    state: State<'_, AppState>,
    period: String,
    from: String,
) -> AppResult<EmployeeCommissionsReport> {
    state.require_owner()?;
    match &state.db {
        AppDb::Sqlite(_) => {
            state
                .db
                .with_sqlite(move |conn| repo::employee_commissions(conn, period, from))
                .await
        }
        AppDb::Postgres(pool) => repo::postgres::employee_commissions(pool, period, from).await,
    }
}

// ---------------------------------------------------------------------
// get_top_services_report
// ---------------------------------------------------------------------

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TopServiceRow {
    pub service_id: Option<String>,
    pub name: String,
    pub quantity: i64,
    pub revenue: i64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TopServicesReport {
    pub period: String,
    pub from: String,
    pub sales_count: i64,
    pub total_revenue: i64,
    /// `f64`: el ticket promedio es una división (`total_revenue /
    /// sales_count`) que casi siempre da un resultado fraccionario.
    pub average_ticket: f64,
    pub rows: Vec<TopServiceRow>,
}

/// Puerto de `getTopServicesReport` (src/lib/reports-server.ts).
///
/// `salesCount`/`totalRevenue`/`averageTicket` se calculan sobre TODAS las
/// ventas del periodo (igual que el JS original, que no filtra por
/// `item_type`); `rows` agrupa únicamente los `sale_items` de tipo
/// `service`, por `service_id` o, si el servicio fue borrado
/// (`ON DELETE SET NULL`), por `name_snapshot`.
///
/// Nota: cuando un mismo `service_id` tiene más de un `name_snapshot`
/// distinto en el periodo (el servicio fue renombrado entre ventas), el JS
/// original usa el nombre del primer ítem encontrado en el orden de
/// retorno de Postgres (no determinista). Aquí se usa `MIN(name_snapshot)`
/// para un resultado determinista; la agrupación/cantidad/ingresos son
/// idénticos.
#[tauri::command]
pub async fn get_top_services_report(
    state: State<'_, AppState>,
    period: String,
    from: String,
) -> AppResult<TopServicesReport> {
    state.require_owner()?;
    match &state.db {
        AppDb::Sqlite(_) => {
            state
                .db
                .with_sqlite(move |conn| repo::top_services(conn, period, from))
                .await
        }
        AppDb::Postgres(pool) => repo::postgres::top_services(pool, period, from).await,
    }
}

// ---------------------------------------------------------------------
// get_no_shows_report
// ---------------------------------------------------------------------

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NoShowCustomerRow {
    pub customer_id: String,
    pub name: String,
    pub email: String,
    pub phone: String,
    pub no_show_count: i64,
    pub cancelled_count: i64,
    pub lost_count: i64,
}

/// Referencia de cliente embebida en `recent`. A propósito NO lleva
/// `#[serde(rename_all = "camelCase")]`: el frontend (`reports-no-shows.tsx`,
/// tipo `RecentRow`) espera el mismo shape "estilo Supabase" que devolvía
/// la consulta original (`starts_at`, `customers: {...}`), no camelCase.
#[derive(Debug, Serialize)]
pub struct RecentCustomerRef {
    pub id: String,
    pub name: String,
    pub email: String,
    pub phone: String,
}

#[derive(Debug, Serialize)]
pub struct RecentNoShowRow {
    pub id: String,
    pub status: String,
    pub starts_at: String,
    pub customers: Option<RecentCustomerRef>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NoShowsReport {
    pub period: String,
    pub from: String,
    pub no_show_total: i64,
    pub cancelled_total: i64,
    pub rows: Vec<NoShowCustomerRow>,
    pub recent: Vec<RecentNoShowRow>,
}

/// Puerto de `getNoShowsReport` (src/lib/reports-server.ts).
///
/// `noShowTotal`/`cancelledTotal` cuentan TODAS las citas del periodo con
/// status `no_show`/`cancelled` (no solo las primeras 30 de `recent`),
/// igual que el JS original que itera la lista completa antes de hacer
/// `.slice(0, 30)`. `customer_id` es `NOT NULL` con
/// `REFERENCES customers (id) ON DELETE RESTRICT`, así que a diferencia
/// del JS (que descartaba citas sin cliente embebido, caso que podía darse
/// vía RLS/Postgrest) aquí el `JOIN` con `customers` siempre encuentra
/// fila — `customers` en la respuesta es `Option` solo para igualar el
/// tipo del frontend, pero siempre viene `Some`.
#[tauri::command]
pub async fn get_no_shows_report(
    state: State<'_, AppState>,
    period: String,
    from: String,
) -> AppResult<NoShowsReport> {
    state.require_owner()?;
    match &state.db {
        AppDb::Sqlite(_) => {
            state
                .db
                .with_sqlite(move |conn| repo::no_shows(conn, period, from))
                .await
        }
        AppDb::Postgres(pool) => repo::postgres::no_shows(pool, period, from).await,
    }
}

// ---------------------------------------------------------------------
// get_revenue_trends_report
// ---------------------------------------------------------------------

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RevenueTrendPoint {
    pub label: String,
    pub bucket_start: String,
    pub revenue: i64,
    pub sales_count: i64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RevenueTrendsReport {
    pub period: String,
    pub from: String,
    pub previous_from: String,
    pub current_revenue: i64,
    pub previous_revenue: i64,
    pub current_sales_count: i64,
    pub previous_sales_count: i64,
    /// `None` cuando `previous_revenue == 0` (el % de crecimiento no está
    /// definido/es infinito — el frontend muestra "Nuevo" en vez de un %).
    pub delta_percent: Option<f64>,
    pub points: Vec<RevenueTrendPoint>,
}

/// Issue #23: ingresos del periodo actual vs. el periodo anterior inmediato
/// (`previous_from` lo calcula el frontend, ver `getPreviousPeriodRange` en
/// `src/lib/sales.ts`), más una serie de sub-periodos ("buckets") dentro del
/// periodo actual para graficar la tendencia. El periodo anterior es
/// siempre `[previous_from, from)` — igual que el resto de los reportes, el
/// periodo actual no tiene límite superior (`sold_at >= from`).
#[tauri::command]
pub async fn get_revenue_trends_report(
    state: State<'_, AppState>,
    period: String,
    from: String,
    previous_from: String,
) -> AppResult<RevenueTrendsReport> {
    state.require_owner()?;
    match &state.db {
        AppDb::Sqlite(_) => {
            state
                .db
                .with_sqlite(move |conn| repo::revenue_trends(conn, period, from, previous_from))
                .await
        }
        AppDb::Postgres(pool) => {
            repo::postgres::revenue_trends(pool, period, from, previous_from).await
        }
    }
}

// ---------------------------------------------------------------------
// get_customer_ltv_report
// ---------------------------------------------------------------------

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CustomerLtvRow {
    pub customer_id: String,
    pub name: String,
    pub phone: String,
    pub email: String,
    pub total_spent: i64,
    pub visit_count: i64,
    /// `None` cuando el cliente solo tiene una visita (no hay intervalo que promediar).
    pub avg_days_between_visits: Option<f64>,
    pub first_visit_at: String,
    pub last_visit_at: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CustomerLtvReport {
    pub limit: i64,
    pub total_customers_with_sales: i64,
    pub average_ltv: f64,
    pub average_visit_count: f64,
    /// Ya ordenados por `total_spent` descendente y truncados a `limit`.
    pub rows: Vec<CustomerLtvRow>,
}

/// Issue #23: valor de cliente (LTV) histórico total y frecuencia de visita,
/// sin filtro de periodo — a diferencia del resto de los reportes, "lifetime
/// value" es por definición desde siempre. "Visita" = una venta no anulada
/// (`sales.sold_at`), no una cita: toda venta tiene `customer_id`
/// obligatorio y es el registro real de una transacción, mientras que una
/// cita puede terminar en no-show o no tener venta vinculada.
#[tauri::command]
pub async fn get_customer_ltv_report(
    state: State<'_, AppState>,
    limit: i64,
) -> AppResult<CustomerLtvReport> {
    state.require_owner()?;
    match &state.db {
        AppDb::Sqlite(_) => {
            state
                .db
                .with_sqlite(move |conn| repo::customer_ltv(conn, limit))
                .await
        }
        AppDb::Postgres(pool) => repo::postgres::customer_ltv(pool, limit).await,
    }
}
