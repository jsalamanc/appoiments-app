use rusqlite::params;
use serde::Serialize;
use tauri::State;

use crate::error::AppResult;
use crate::state::AppState;
use crate::util::period::period_from;

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
    pub revenue: f64,
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
pub fn get_employee_performance_report(
    state: State<AppState>,
    period: String,
) -> AppResult<EmployeePerformanceReport> {
    let conn = state.db.lock().unwrap();
    let from = period_from(&period);

    let mut stmt = conn.prepare(
        "SELECT
            e.id AS employee_id,
            e.name AS name,
            e.color AS color,
            COUNT(CASE WHEN a.status = 'scheduled' THEN 1 END) AS scheduled,
            COUNT(CASE WHEN a.status = 'completed' THEN 1 END) AS completed,
            COUNT(CASE WHEN a.status = 'cancelled' THEN 1 END) AS cancelled,
            COUNT(CASE WHEN a.status = 'no_show' THEN 1 END) AS no_show,
            COUNT(a.id) AS total_appointments,
            COALESCE(SUM(s.total), 0.0) AS revenue
         FROM employees e
         LEFT JOIN appointments a
            ON a.employee_id = e.id AND a.starts_at >= ?1
         LEFT JOIN sales s ON s.id = a.sale_id
         GROUP BY e.id, e.name, e.color
         ORDER BY completed DESC, revenue DESC, e.name ASC",
    )?;

    let rows = stmt
        .query_map(params![from], |row| {
            Ok(EmployeePerformanceRow {
                employee_id: row.get("employee_id")?,
                name: row.get("name")?,
                color: row.get("color")?,
                scheduled: row.get("scheduled")?,
                completed: row.get("completed")?,
                cancelled: row.get("cancelled")?,
                no_show: row.get("no_show")?,
                total_appointments: row.get("total_appointments")?,
                revenue: row.get("revenue")?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;

    Ok(EmployeePerformanceReport { period, from, rows })
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
    pub revenue: f64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TopServicesReport {
    pub period: String,
    pub from: String,
    pub sales_count: i64,
    pub total_revenue: f64,
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
pub fn get_top_services_report(
    state: State<AppState>,
    period: String,
) -> AppResult<TopServicesReport> {
    let conn = state.db.lock().unwrap();
    let from = period_from(&period);

    let (sales_count, total_revenue): (i64, f64) = conn.query_row(
        "SELECT COUNT(*), COALESCE(SUM(total), 0.0) FROM sales WHERE sold_at >= ?1",
        params![from],
        |row| Ok((row.get(0)?, row.get(1)?)),
    )?;

    let average_ticket = if sales_count == 0 {
        0.0
    } else {
        total_revenue / sales_count as f64
    };

    let mut stmt = conn.prepare(
        "SELECT
            si.service_id AS service_id,
            MIN(si.name_snapshot) AS name,
            SUM(si.quantity) AS quantity,
            SUM(si.line_total) AS revenue
         FROM sale_items si
         JOIN sales s ON s.id = si.sale_id
         WHERE si.item_type = 'service' AND s.sold_at >= ?1
         GROUP BY COALESCE(si.service_id, si.name_snapshot)
         ORDER BY quantity DESC, revenue DESC",
    )?;

    let rows = stmt
        .query_map(params![from], |row| {
            Ok(TopServiceRow {
                service_id: row.get("service_id")?,
                name: row.get("name")?,
                quantity: row.get("quantity")?,
                revenue: row.get("revenue")?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;

    Ok(TopServicesReport {
        period,
        from,
        sales_count,
        total_revenue,
        average_ticket,
        rows,
    })
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
pub fn get_no_shows_report(state: State<AppState>, period: String) -> AppResult<NoShowsReport> {
    let conn = state.db.lock().unwrap();
    let from = period_from(&period);

    let (no_show_total, cancelled_total): (i64, i64) = conn.query_row(
        "SELECT
            COUNT(CASE WHEN status = 'no_show' THEN 1 END),
            COUNT(CASE WHEN status = 'cancelled' THEN 1 END)
         FROM appointments
         WHERE starts_at >= ?1 AND status IN ('no_show', 'cancelled')",
        params![from],
        |row| Ok((row.get(0)?, row.get(1)?)),
    )?;

    let mut rows_stmt = conn.prepare(
        "SELECT
            c.id AS customer_id,
            c.name AS name,
            c.email AS email,
            c.phone AS phone,
            COUNT(CASE WHEN a.status = 'no_show' THEN 1 END) AS no_show_count,
            COUNT(CASE WHEN a.status = 'cancelled' THEN 1 END) AS cancelled_count,
            COUNT(a.id) AS lost_count
         FROM appointments a
         JOIN customers c ON c.id = a.customer_id
         WHERE a.starts_at >= ?1 AND a.status IN ('no_show', 'cancelled')
         GROUP BY c.id, c.name, c.email, c.phone
         ORDER BY lost_count DESC, c.name ASC",
    )?;
    let rows = rows_stmt
        .query_map(params![from], |row| {
            Ok(NoShowCustomerRow {
                customer_id: row.get("customer_id")?,
                name: row.get("name")?,
                email: row.get("email")?,
                phone: row.get("phone")?,
                no_show_count: row.get("no_show_count")?,
                cancelled_count: row.get("cancelled_count")?,
                lost_count: row.get("lost_count")?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;

    let mut recent_stmt = conn.prepare(
        "SELECT
            a.id AS id,
            a.status AS status,
            a.starts_at AS starts_at,
            c.id AS cust_id,
            c.name AS cust_name,
            c.email AS cust_email,
            c.phone AS cust_phone
         FROM appointments a
         JOIN customers c ON c.id = a.customer_id
         WHERE a.starts_at >= ?1 AND a.status IN ('no_show', 'cancelled')
         ORDER BY a.starts_at DESC
         LIMIT 30",
    )?;
    let recent = recent_stmt
        .query_map(params![from], |row| {
            Ok(RecentNoShowRow {
                id: row.get("id")?,
                status: row.get("status")?,
                starts_at: row.get("starts_at")?,
                customers: Some(RecentCustomerRef {
                    id: row.get("cust_id")?,
                    name: row.get("cust_name")?,
                    email: row.get("cust_email")?,
                    phone: row.get("cust_phone")?,
                }),
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;

    Ok(NoShowsReport {
        period,
        from,
        no_show_total,
        cancelled_total,
        rows,
        recent,
    })
}
