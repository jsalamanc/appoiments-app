use rusqlite::{params, Connection};

use crate::commands::reports::{
    EmployeePerformanceReport, EmployeePerformanceRow, NoShowCustomerRow, NoShowsReport,
    RecentCustomerRef, RecentNoShowRow, TopServiceRow, TopServicesReport,
};
use crate::error::AppResult;

/// Puerto de `getEmployeePerformance` (src/lib/reports-server.ts).
///
/// Nota: el JS original NO filtra por `employees.active` (selecciona el
/// campo pero nunca lo usa para filtrar ni lo expone en
/// `EmployeePerformanceRow`), así que esta consulta tampoco filtra —
/// replica el comportamiento real observado, no la descripción informal
/// de "solo empleados activos".
/// `from` ya viene calculado por el frontend (medianoche local calendario
/// del período pedido, ver `repositories::sales::cash_day_summary`).
pub fn employee_performance(
    conn: &mut Connection,
    period: String,
    from: String,
) -> AppResult<EmployeePerformanceReport> {
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
pub fn top_services(
    conn: &mut Connection,
    period: String,
    from: String,
) -> AppResult<TopServicesReport> {
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
pub fn no_shows(
    conn: &mut Connection,
    period: String,
    from: String,
) -> AppResult<NoShowsReport> {
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

/// Implementación PostgreSQL (issue #9, Fase 2) — misma lógica de agregación
/// que arriba, adaptada al driver async `sqlx`.
///
/// IMPORTANTE: `starts_at`/`sold_at` son `TIMESTAMPTZ` en Postgres (no texto
/// como en SQLite), así que comparar contra la cadena RFC3339 que manda el
/// frontend como `from` requiere parsearla primero a `time::OffsetDateTime`
/// y bindearla nativamente — nunca castear la columna a texto
/// (`col::text >= $1`), porque la representación textual por defecto de
/// Postgres para `timestamptz` ("2024-01-15 10:30:00+00") no es RFC3339 y una
/// comparación lexicográfica contra ella no es confiablemente equivalente a
/// una comparación cronológica.
pub mod postgres {
    use sqlx::{PgPool, Row};

    use crate::commands::reports::{
        EmployeePerformanceReport, EmployeePerformanceRow, NoShowCustomerRow, NoShowsReport,
        RecentCustomerRef, RecentNoShowRow, TopServiceRow, TopServicesReport,
    };
    use crate::db::format_pg_timestamp;
    use crate::error::{AppError, AppResult};

    fn parse_from(from: &str) -> AppResult<time::OffsetDateTime> {
        time::OffsetDateTime::parse(from, &time::format_description::well_known::Rfc3339)
            .map_err(|e| AppError::Internal(e.to_string()))
    }

    pub async fn employee_performance(
        pool: &PgPool,
        period: String,
        from: String,
    ) -> AppResult<EmployeePerformanceReport> {
        let from = parse_from(&from)?;

        let rows = sqlx::query(
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
                ON a.employee_id = e.id AND a.starts_at >= $1
             LEFT JOIN sales s ON s.id = a.sale_id
             GROUP BY e.id, e.name, e.color
             ORDER BY completed DESC, revenue DESC, e.name ASC",
        )
        .bind(from)
        .fetch_all(pool)
        .await?;

        let rows = rows
            .iter()
            .map(|row| -> AppResult<EmployeePerformanceRow> {
                Ok(EmployeePerformanceRow {
                    employee_id: row.try_get("employee_id")?,
                    name: row.try_get("name")?,
                    color: row.try_get("color")?,
                    scheduled: row.try_get("scheduled")?,
                    completed: row.try_get("completed")?,
                    cancelled: row.try_get("cancelled")?,
                    no_show: row.try_get("no_show")?,
                    total_appointments: row.try_get("total_appointments")?,
                    revenue: row.try_get("revenue")?,
                })
            })
            .collect::<AppResult<Vec<_>>>()?;

        Ok(EmployeePerformanceReport {
            period,
            from: format_pg_timestamp(from),
            rows,
        })
    }

    pub async fn top_services(
        pool: &PgPool,
        period: String,
        from: String,
    ) -> AppResult<TopServicesReport> {
        let from = parse_from(&from)?;

        let totals_row = sqlx::query(
            "SELECT COUNT(*) AS sales_count, COALESCE(SUM(total), 0.0) AS total_revenue
             FROM sales WHERE sold_at >= $1",
        )
        .bind(from)
        .fetch_one(pool)
        .await?;
        let sales_count: i64 = totals_row.try_get("sales_count")?;
        let total_revenue: f64 = totals_row.try_get("total_revenue")?;

        let average_ticket = if sales_count == 0 {
            0.0
        } else {
            total_revenue / sales_count as f64
        };

        // A diferencia de SQLite, Postgres exige que toda columna en el
        // SELECT esté en el GROUP BY o dentro de una función de agregación;
        // como el GROUP BY agrupa por COALESCE(service_id, name_snapshot) y
        // no por service_id directamente, hay que envolverlo en MIN(...)
        // igual que ya se hacía con name_snapshot (dentro de cada grupo,
        // service_id es siempre el mismo valor o siempre NULL).
        let rows = sqlx::query(
            "SELECT
                MIN(si.service_id) AS service_id,
                MIN(si.name_snapshot) AS name,
                SUM(si.quantity) AS quantity,
                SUM(si.line_total) AS revenue
             FROM sale_items si
             JOIN sales s ON s.id = si.sale_id
             WHERE si.item_type = 'service' AND s.sold_at >= $1
             GROUP BY COALESCE(si.service_id, si.name_snapshot)
             ORDER BY quantity DESC, revenue DESC",
        )
        .bind(from)
        .fetch_all(pool)
        .await?;

        let rows = rows
            .iter()
            .map(|row| -> AppResult<TopServiceRow> {
                Ok(TopServiceRow {
                    service_id: row.try_get("service_id")?,
                    name: row.try_get("name")?,
                    quantity: row.try_get("quantity")?,
                    revenue: row.try_get("revenue")?,
                })
            })
            .collect::<AppResult<Vec<_>>>()?;

        Ok(TopServicesReport {
            period,
            from: format_pg_timestamp(from),
            sales_count,
            total_revenue,
            average_ticket,
            rows,
        })
    }

    pub async fn no_shows(
        pool: &PgPool,
        period: String,
        from: String,
    ) -> AppResult<NoShowsReport> {
        let from = parse_from(&from)?;

        let totals_row = sqlx::query(
            "SELECT
                COUNT(CASE WHEN status = 'no_show' THEN 1 END) AS no_show_total,
                COUNT(CASE WHEN status = 'cancelled' THEN 1 END) AS cancelled_total
             FROM appointments
             WHERE starts_at >= $1 AND status IN ('no_show', 'cancelled')",
        )
        .bind(from)
        .fetch_one(pool)
        .await?;
        let no_show_total: i64 = totals_row.try_get("no_show_total")?;
        let cancelled_total: i64 = totals_row.try_get("cancelled_total")?;

        let rows = sqlx::query(
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
             WHERE a.starts_at >= $1 AND a.status IN ('no_show', 'cancelled')
             GROUP BY c.id, c.name, c.email, c.phone
             ORDER BY lost_count DESC, c.name ASC",
        )
        .bind(from)
        .fetch_all(pool)
        .await?;

        let rows = rows
            .iter()
            .map(|row| -> AppResult<NoShowCustomerRow> {
                Ok(NoShowCustomerRow {
                    customer_id: row.try_get("customer_id")?,
                    name: row.try_get("name")?,
                    email: row.try_get("email")?,
                    phone: row.try_get("phone")?,
                    no_show_count: row.try_get("no_show_count")?,
                    cancelled_count: row.try_get("cancelled_count")?,
                    lost_count: row.try_get("lost_count")?,
                })
            })
            .collect::<AppResult<Vec<_>>>()?;

        let recent_rows = sqlx::query(
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
             WHERE a.starts_at >= $1 AND a.status IN ('no_show', 'cancelled')
             ORDER BY a.starts_at DESC
             LIMIT 30",
        )
        .bind(from)
        .fetch_all(pool)
        .await?;

        let recent = recent_rows
            .iter()
            .map(|row| -> AppResult<RecentNoShowRow> {
                let starts_at: time::OffsetDateTime = row.try_get("starts_at")?;
                Ok(RecentNoShowRow {
                    id: row.try_get("id")?,
                    status: row.try_get("status")?,
                    starts_at: format_pg_timestamp(starts_at),
                    customers: Some(RecentCustomerRef {
                        id: row.try_get("cust_id")?,
                        name: row.try_get("cust_name")?,
                        email: row.try_get("cust_email")?,
                        phone: row.try_get("cust_phone")?,
                    }),
                })
            })
            .collect::<AppResult<Vec<_>>>()?;

        Ok(NoShowsReport {
            period,
            from: format_pg_timestamp(from),
            no_show_total,
            cancelled_total,
            rows,
            recent,
        })
    }
}
