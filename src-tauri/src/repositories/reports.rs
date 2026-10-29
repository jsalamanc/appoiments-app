use std::collections::HashMap;

use rusqlite::{params, Connection};

use crate::commands::reports::{
    EmployeeCommissionRow, EmployeeCommissionsReport, EmployeePerformanceReport,
    EmployeePerformanceRow, NoShowCustomerRow, NoShowsReport, RecentCustomerRef,
    RecentNoShowRow, TopServiceRow, TopServicesReport,
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
         LEFT JOIN sales s ON s.id = a.sale_id AND s.voided_at IS NULL
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

/// Issue #22 / #2 Fase 4: comisión por empleado sobre las citas completadas
/// cuya venta vinculada no esté anulada — mismo universo de citas que
/// `employee_performance` (`a.status = 'completed'`, `a.starts_at >= from`,
/// `s.voided_at IS NULL`), pero resolviendo la tasa de comisión (excepción
/// por servicio si existe, si no la general del empleado, si no ninguna) y
/// calculando el monto en Rust en vez de SQL — con solo tres tipos de tasa
/// posibles por cita (sin tasa / porcentaje / fijo) hacerlo en SQL exigiría
/// un CASE anidado difícil de leer para un beneficio de rendimiento
/// irrelevante en esta escala (negocio pequeño, tabla de tasas minúscula).
pub fn employee_commissions(
    conn: &mut Connection,
    period: String,
    from: String,
) -> AppResult<EmployeeCommissionsReport> {
    let employees: Vec<(String, String, String)> = {
        let mut stmt = conn.prepare("SELECT id, name, color FROM employees")?;
        let rows = stmt
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))?
            .collect::<Result<Vec<_>, _>>()?;
        rows
    };

    let rate_map = crate::repositories::commissions::load_rate_map(conn)?;

    let mut stats: HashMap<String, (i64, f64)> = HashMap::new();
    let mut commission: HashMap<String, f64> = HashMap::new();
    {
        let mut stmt = conn.prepare(
            "SELECT a.employee_id, a.service_id, s.total
             FROM appointments a
             JOIN sales s ON s.id = a.sale_id AND s.voided_at IS NULL
             WHERE a.status = 'completed' AND a.starts_at >= ?1",
        )?;
        let rows = stmt.query_map(params![from], |row| {
            let employee_id: String = row.get(0)?;
            let service_id: Option<String> = row.get(1)?;
            let sale_total: f64 = row.get(2)?;
            Ok((employee_id, service_id, sale_total))
        })?;

        for row in rows {
            let (employee_id, service_id, sale_total) = row?;
            let entry = stats.entry(employee_id.clone()).or_insert((0, 0.0));
            entry.0 += 1;
            entry.1 += sale_total;

            if let Some(rate) = crate::repositories::commissions::resolve_rate(
                &rate_map,
                &employee_id,
                service_id.as_deref(),
            ) {
                let amount = crate::repositories::commissions::commission_amount(rate, sale_total);
                *commission.entry(employee_id).or_insert(0.0) += amount;
            }
        }
    }

    let mut rows: Vec<EmployeeCommissionRow> = employees
        .into_iter()
        .map(|(id, name, color)| {
            let (completed_appointments, revenue) = stats.get(&id).copied().unwrap_or((0, 0.0));
            let commission_amount = commission.get(&id).copied().unwrap_or(0.0);
            EmployeeCommissionRow {
                employee_id: id,
                name,
                color,
                completed_appointments,
                revenue,
                commission_amount,
            }
        })
        .collect();

    rows.sort_by(|a, b| {
        b.commission_amount
            .partial_cmp(&a.commission_amount)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| b.revenue.partial_cmp(&a.revenue).unwrap_or(std::cmp::Ordering::Equal))
            .then_with(|| a.name.cmp(&b.name))
    });

    let total_commission = rows.iter().map(|r| r.commission_amount).sum();

    Ok(EmployeeCommissionsReport {
        period,
        from,
        total_commission,
        rows,
    })
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
        "SELECT COUNT(*), COALESCE(SUM(total), 0.0) FROM sales WHERE sold_at >= ?1 AND voided_at IS NULL",
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
         WHERE si.item_type = 'service' AND s.sold_at >= ?1 AND s.voided_at IS NULL
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
            COALESCE(c.email, '') AS email,
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
            COALESCE(c.email, '') AS cust_email,
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
    use std::collections::HashMap;

    use sqlx::{PgPool, Row};

    use crate::commands::reports::{
        EmployeeCommissionRow, EmployeeCommissionsReport, EmployeePerformanceReport,
        EmployeePerformanceRow, NoShowCustomerRow, NoShowsReport, RecentCustomerRef,
        RecentNoShowRow, TopServiceRow, TopServicesReport,
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
             LEFT JOIN sales s ON s.id = a.sale_id AND s.voided_at IS NULL
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

    /// Ver `super::employee_commissions` — misma lógica, dialecto Postgres.
    pub async fn employee_commissions(
        pool: &PgPool,
        period: String,
        from: String,
    ) -> AppResult<EmployeeCommissionsReport> {
        let from_ts = parse_from(&from)?;

        let employees = sqlx::query("SELECT id, name, color FROM employees")
            .fetch_all(pool)
            .await?;
        let employees = employees
            .iter()
            .map(|row| -> AppResult<(String, String, String)> {
                Ok((
                    row.try_get("id")?,
                    row.try_get("name")?,
                    row.try_get("color")?,
                ))
            })
            .collect::<AppResult<Vec<_>>>()?;

        let rate_map = crate::repositories::commissions::postgres::load_rate_map(pool).await?;

        let appointment_rows = sqlx::query(
            "SELECT a.employee_id, a.service_id, s.total
             FROM appointments a
             JOIN sales s ON s.id = a.sale_id AND s.voided_at IS NULL
             WHERE a.status = 'completed' AND a.starts_at >= $1",
        )
        .bind(from_ts)
        .fetch_all(pool)
        .await?;

        let mut stats: HashMap<String, (i64, f64)> = HashMap::new();
        let mut commission: HashMap<String, f64> = HashMap::new();
        for row in &appointment_rows {
            let employee_id: String = row.try_get("employee_id")?;
            let service_id: Option<String> = row.try_get("service_id")?;
            let sale_total: f64 = row.try_get("total")?;

            let entry = stats.entry(employee_id.clone()).or_insert((0, 0.0));
            entry.0 += 1;
            entry.1 += sale_total;

            if let Some(rate) = crate::repositories::commissions::resolve_rate(
                &rate_map,
                &employee_id,
                service_id.as_deref(),
            ) {
                let amount = crate::repositories::commissions::commission_amount(rate, sale_total);
                *commission.entry(employee_id).or_insert(0.0) += amount;
            }
        }

        let mut rows: Vec<EmployeeCommissionRow> = employees
            .into_iter()
            .map(|(id, name, color)| {
                let (completed_appointments, revenue) = stats.get(&id).copied().unwrap_or((0, 0.0));
                let commission_amount = commission.get(&id).copied().unwrap_or(0.0);
                EmployeeCommissionRow {
                    employee_id: id,
                    name,
                    color,
                    completed_appointments,
                    revenue,
                    commission_amount,
                }
            })
            .collect();

        rows.sort_by(|a, b| {
            b.commission_amount
                .partial_cmp(&a.commission_amount)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| {
                    b.revenue
                        .partial_cmp(&a.revenue)
                        .unwrap_or(std::cmp::Ordering::Equal)
                })
                .then_with(|| a.name.cmp(&b.name))
        });

        let total_commission = rows.iter().map(|r| r.commission_amount).sum();

        Ok(EmployeeCommissionsReport {
            period,
            from: format_pg_timestamp(from_ts),
            total_commission,
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
             FROM sales WHERE sold_at >= $1 AND voided_at IS NULL",
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
             WHERE si.item_type = 'service' AND s.sold_at >= $1 AND s.voided_at IS NULL
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
                COALESCE(c.email, '') AS email,
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
                COALESCE(c.email, '') AS cust_email,
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::commissions::CommissionRateInput;

    fn open_test_db() -> Connection {
        let mut conn = Connection::open_in_memory().unwrap();
        crate::db::migrations().to_latest(&mut conn).unwrap();
        conn
    }

    fn seed_employee(conn: &Connection, id: &str, name: &str) {
        conn.execute(
            "INSERT INTO employees (id, name, color, active) VALUES (?1, ?2, '#fff', 1)",
            params![id, name],
        )
        .unwrap();
    }

    fn seed_customer(conn: &Connection) {
        conn.execute(
            "INSERT INTO customers (id, name, phone, email, document)
             VALUES ('cust-1', 'Cliente', '3000000000', 'cliente@test.com', 'DOC-1')",
            [],
        )
        .unwrap();
    }

    fn seed_service(conn: &Connection, id: &str) {
        conn.execute(
            "INSERT INTO services (id, name, price, duration_minutes, status)
             VALUES (?1, 'Corte', 20.0, 60, 'published')",
            params![id],
        )
        .unwrap();
    }

    /// Crea una cita completada con una venta vinculada no anulada por
    /// `total`, igual al único camino de atribución de ingresos a un
    /// empleado que existe hoy (`appointments.employee_id` ->
    /// `appointments.sale_id` -> `sales.total`, ver `employee_performance`).
    fn seed_completed_appointment(
        conn: &Connection,
        appointment_id: &str,
        employee_id: &str,
        service_id: &str,
        starts_at: &str,
        ends_at: &str,
        total: f64,
        voided: bool,
    ) {
        let sale_id = format!("sale-{appointment_id}");
        conn.execute(
            "INSERT INTO sales (id, customer_id, payment_method, total, voided_at)
             VALUES (?1, 'cust-1', 'cash', ?2, ?3)",
            params![sale_id, total, if voided { Some("2026-01-01T00:00:00Z") } else { None }],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO appointments
                (id, employee_id, customer_id, service_id, service_name_snapshot, starts_at, ends_at, status, sale_id)
             VALUES (?1, ?2, 'cust-1', ?3, 'Corte', ?4, ?5, 'completed', ?6)",
            params![appointment_id, employee_id, service_id, starts_at, ends_at, sale_id],
        )
        .unwrap();
    }

    fn set_default_rate(conn: &mut Connection, employee_id: &str, rate_type: &str, rate_value: f64) {
        crate::repositories::commissions::update_employee_commission_rates(
            conn,
            employee_id.to_string(),
            vec![CommissionRateInput {
                service_id: None,
                rate_type: rate_type.to_string(),
                rate_value,
            }],
        )
        .unwrap();
    }

    #[test]
    fn employee_commissions_is_zero_without_configured_rate() {
        let mut conn = open_test_db();
        seed_employee(&conn, "emp-1", "Ana");
        seed_customer(&conn);
        seed_service(&conn, "svc-1");
        seed_completed_appointment(&conn, "appt-1", "emp-1", "svc-1", "2026-01-10T10:00:00Z", "2026-01-10T11:00:00Z", 100.0, false);

        let report = employee_commissions(&mut conn, "month".to_string(), "2026-01-01T00:00:00Z".to_string()).unwrap();

        let row = report.rows.iter().find(|r| r.employee_id == "emp-1").unwrap();
        assert_eq!(row.completed_appointments, 1);
        assert_eq!(row.revenue, 100.0);
        assert_eq!(row.commission_amount, 0.0, "sin tasa configurada no debe haber comisión");
    }

    #[test]
    fn employee_commissions_applies_default_percentage_rate() {
        let mut conn = open_test_db();
        seed_employee(&conn, "emp-1", "Ana");
        seed_customer(&conn);
        seed_service(&conn, "svc-1");
        seed_completed_appointment(&conn, "appt-1", "emp-1", "svc-1", "2026-01-10T10:00:00Z", "2026-01-10T11:00:00Z", 200.0, false);
        set_default_rate(&mut conn, "emp-1", "percentage", 10.0);

        let report = employee_commissions(&mut conn, "month".to_string(), "2026-01-01T00:00:00Z".to_string()).unwrap();

        let row = report.rows.iter().find(|r| r.employee_id == "emp-1").unwrap();
        assert_eq!(row.commission_amount, 20.0);
        assert_eq!(report.total_commission, 20.0);
    }

    #[test]
    fn employee_commissions_service_override_takes_precedence() {
        let mut conn = open_test_db();
        seed_employee(&conn, "emp-1", "Ana");
        seed_customer(&conn);
        seed_service(&conn, "svc-1");
        seed_completed_appointment(&conn, "appt-1", "emp-1", "svc-1", "2026-01-10T10:00:00Z", "2026-01-10T11:00:00Z", 200.0, false);
        set_default_rate(&mut conn, "emp-1", "percentage", 10.0);
        crate::repositories::commissions::update_employee_commission_rates(
            &mut conn,
            "emp-1".to_string(),
            vec![
                CommissionRateInput { service_id: None, rate_type: "percentage".to_string(), rate_value: 10.0 },
                CommissionRateInput { service_id: Some("svc-1".to_string()), rate_type: "fixed".to_string(), rate_value: 15.0 },
            ],
        )
        .unwrap();

        let report = employee_commissions(&mut conn, "month".to_string(), "2026-01-01T00:00:00Z".to_string()).unwrap();

        let row = report.rows.iter().find(|r| r.employee_id == "emp-1").unwrap();
        assert_eq!(row.commission_amount, 15.0, "la excepción por servicio debe ganarle a la tasa general");
    }

    #[test]
    fn employee_commissions_excludes_voided_sale() {
        let mut conn = open_test_db();
        seed_employee(&conn, "emp-1", "Ana");
        seed_customer(&conn);
        seed_service(&conn, "svc-1");
        seed_completed_appointment(&conn, "appt-1", "emp-1", "svc-1", "2026-01-10T10:00:00Z", "2026-01-10T11:00:00Z", 200.0, true);
        set_default_rate(&mut conn, "emp-1", "percentage", 10.0);

        let report = employee_commissions(&mut conn, "month".to_string(), "2026-01-01T00:00:00Z".to_string()).unwrap();

        let row = report.rows.iter().find(|r| r.employee_id == "emp-1").unwrap();
        assert_eq!(row.completed_appointments, 0);
        assert_eq!(row.revenue, 0.0);
        assert_eq!(row.commission_amount, 0.0, "una venta anulada no debe generar comisión");
    }

    #[test]
    fn employee_commissions_excludes_appointments_before_from() {
        let mut conn = open_test_db();
        seed_employee(&conn, "emp-1", "Ana");
        seed_customer(&conn);
        seed_service(&conn, "svc-1");
        seed_completed_appointment(&conn, "appt-1", "emp-1", "svc-1", "2025-12-01T10:00:00Z", "2025-12-01T11:00:00Z", 200.0, false);
        set_default_rate(&mut conn, "emp-1", "percentage", 10.0);

        let report = employee_commissions(&mut conn, "month".to_string(), "2026-01-01T00:00:00Z".to_string()).unwrap();

        let row = report.rows.iter().find(|r| r.employee_id == "emp-1").unwrap();
        assert_eq!(row.commission_amount, 0.0, "una cita anterior al período no debe contar");
    }
}
