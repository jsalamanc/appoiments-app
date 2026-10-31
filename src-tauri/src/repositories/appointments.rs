use rusqlite::{params, Connection, OptionalExtension};
use time::format_description::well_known::Rfc3339;

use crate::commands::appointments::AppointmentInput;
use crate::commands::customers::{AppointmentWithDetails, CustomerRef, EmployeeRef, ServiceRef};
use crate::db::{new_id, now_iso};
use crate::error::{AppError, AppResult};
use crate::models::appointment::Appointment;

const VALID_STATUSES: [&str; 4] = ["scheduled", "completed", "cancelled", "no_show"];

fn normalize_notes(notes: Option<String>) -> Option<String> {
    let trimmed = notes.unwrap_or_default().trim().to_string();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed)
    }
}

/// Replica `p_starts_at + make_interval(mins => v_duration)` de Postgres.
fn compute_ends_at(starts_at: &str, duration_minutes: i64) -> AppResult<String> {
    let start = time::OffsetDateTime::parse(starts_at, &Rfc3339)
        .map_err(|_| AppError::validation("Fecha de inicio inválida"))?;
    let end = start + time::Duration::minutes(duration_minutes);
    end.format(&Rfc3339)
        .map_err(|e| AppError::Internal(e.to_string()))
}

struct ServiceInfo {
    duration_minutes: i64,
    name: String,
}

/// Empleado debe existir y estar activo (igual que hoy en Postgres).
fn assert_employee_active(conn: &rusqlite::Connection, employee_id: &str) -> AppResult<()> {
    let exists: Option<i64> = conn
        .query_row(
            "SELECT 1 FROM employees WHERE id = ?1 AND active = 1",
            params![employee_id],
            |r| r.get::<_, i64>(0),
        )
        .optional()?;
    if exists.is_none() {
        return Err(AppError::validation("Empleado no disponible"));
    }
    Ok(())
}

fn assert_customer_exists(conn: &rusqlite::Connection, customer_id: &str) -> AppResult<()> {
    let exists: Option<i64> = conn
        .query_row(
            "SELECT 1 FROM customers WHERE id = ?1",
            params![customer_id],
            |r| r.get::<_, i64>(0),
        )
        .optional()?;
    if exists.is_none() {
        return Err(AppError::validation("Cliente no encontrado"));
    }
    Ok(())
}

/// El servicio debe existir y estar `published`; de ahí se toman
/// `duration_minutes` y `name` (para `service_name_snapshot`).
fn load_published_service(
    conn: &rusqlite::Connection,
    service_id: &str,
) -> AppResult<ServiceInfo> {
    let info = conn
        .query_row(
            "SELECT duration_minutes, name FROM services WHERE id = ?1 AND status = 'published'",
            params![service_id],
            |row| {
                Ok(ServiceInfo {
                    duration_minutes: row.get(0)?,
                    name: row.get(1)?,
                })
            },
        )
        .optional()?;
    info.ok_or_else(|| AppError::validation("Servicio no disponible"))
}

/// Mismo predicado que `appointment_has_overlap` en Postgres:
/// `starts_at < other.ends_at AND ends_at > other.starts_at`, para el mismo
/// empleado y solo contra citas `scheduled`.
fn appointment_has_overlap(
    conn: &rusqlite::Connection,
    employee_id: &str,
    starts_at: &str,
    ends_at: &str,
    exclude_id: Option<&str>,
) -> AppResult<bool> {
    let overlaps: i64 = conn.query_row(
        "SELECT EXISTS(
            SELECT 1 FROM appointments
            WHERE employee_id = ?1
              AND status = 'scheduled'
              AND starts_at < ?2
              AND ends_at > ?3
              AND (?4 IS NULL OR id <> ?4)
         )",
        params![employee_id, ends_at, starts_at, exclude_id],
        |r| r.get(0),
    )?;
    Ok(overlaps != 0)
}

pub fn list_in_range(
    conn: &mut Connection,
    from: String,
    to: String,
    employee_id: Option<String>,
) -> AppResult<Vec<AppointmentWithDetails>> {
    let mut stmt = conn.prepare(
        "SELECT a.*,
                e.name AS emp_name, e.color AS emp_color, e.active AS emp_active,
                c.name AS cust_name, COALESCE(c.email, '') AS cust_email, c.phone AS cust_phone,
                COALESCE(c.document, '') AS cust_document,
                s.price AS svc_price, s.duration_minutes AS svc_duration, s.status AS svc_status
         FROM appointments a
         JOIN employees e ON e.id = a.employee_id
         JOIN customers c ON c.id = a.customer_id
         LEFT JOIN services s ON s.id = a.service_id
         WHERE a.starts_at >= ?1 AND a.starts_at < ?2
           AND (?3 IS NULL OR a.employee_id = ?3)
         ORDER BY a.starts_at ASC",
    )?;

    let rows = stmt.query_map(params![from, to, employee_id], |row| {
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
                id: row.get("customer_id")?,
                name: row.get("cust_name")?,
                email: row.get("cust_email")?,
                phone: row.get("cust_phone")?,
                document: row.get("cust_document")?,
            },
            services,
        })
    })?;

    Ok(rows.collect::<Result<Vec<_>, _>>()?)
}

pub fn create(conn: &mut Connection, input: AppointmentInput) -> AppResult<Appointment> {
    if input.starts_at.trim().is_empty() {
        return Err(AppError::validation("Fecha de inicio requerida"));
    }

    let tx = conn.transaction()?;

    assert_employee_active(&tx, &input.employee_id)?;
    assert_customer_exists(&tx, &input.customer_id)?;
    let service = load_published_service(&tx, &input.service_id)?;
    let ends_at = compute_ends_at(&input.starts_at, service.duration_minutes)?;
    crate::repositories::business_hours::assert_appointment_within_schedule(
        &tx,
        &input.employee_id,
        input.local_weekday,
        input.local_start_minutes,
        input.local_start_minutes + service.duration_minutes,
    )?;

    if appointment_has_overlap(&tx, &input.employee_id, &input.starts_at, &ends_at, None)? {
        return Err(AppError::validation(
            "El empleado ya tiene una cita en ese horario",
        ));
    }

    let id = new_id();
    let now = now_iso();
    let notes = normalize_notes(input.notes);

    tx.execute(
        "INSERT INTO appointments (
            id, employee_id, customer_id, service_id, service_name_snapshot,
            starts_at, ends_at, status, notes, created_at, updated_at
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 'scheduled', ?8, ?9, ?9)",
        params![
            id,
            input.employee_id,
            input.customer_id,
            input.service_id,
            service.name,
            input.starts_at,
            ends_at,
            notes,
            now,
        ],
    )?;

    let appointment = tx.query_row(
        "SELECT * FROM appointments WHERE id = ?1",
        params![id],
        Appointment::from_row,
    )?;

    tx.commit()?;
    Ok(appointment)
}

pub fn update(conn: &mut Connection, id: String, input: AppointmentInput) -> AppResult<Appointment> {
    if input.starts_at.trim().is_empty() {
        return Err(AppError::validation("Fecha de inicio requerida"));
    }

    let tx = conn.transaction()?;

    let current_status: String = tx
        .query_row(
            "SELECT status FROM appointments WHERE id = ?1",
            params![id],
            |r| r.get::<_, String>(0),
        )
        .optional()?
        .ok_or(AppError::NotFound)?;

    if current_status != "scheduled" {
        return Err(AppError::validation(
            "Solo se pueden editar citas programadas",
        ));
    }

    assert_employee_active(&tx, &input.employee_id)?;
    assert_customer_exists(&tx, &input.customer_id)?;
    let service = load_published_service(&tx, &input.service_id)?;
    let ends_at = compute_ends_at(&input.starts_at, service.duration_minutes)?;
    crate::repositories::business_hours::assert_appointment_within_schedule(
        &tx,
        &input.employee_id,
        input.local_weekday,
        input.local_start_minutes,
        input.local_start_minutes + service.duration_minutes,
    )?;

    if appointment_has_overlap(
        &tx,
        &input.employee_id,
        &input.starts_at,
        &ends_at,
        Some(id.as_str()),
    )? {
        return Err(AppError::validation(
            "El empleado ya tiene una cita en ese horario",
        ));
    }

    let notes = normalize_notes(input.notes);

    tx.execute(
        "UPDATE appointments SET
            employee_id = ?1,
            customer_id = ?2,
            service_id = ?3,
            service_name_snapshot = ?4,
            starts_at = ?5,
            ends_at = ?6,
            notes = ?7,
            updated_at = ?8
         WHERE id = ?9",
        params![
            input.employee_id,
            input.customer_id,
            input.service_id,
            service.name,
            input.starts_at,
            ends_at,
            notes,
            now_iso(),
            id,
        ],
    )?;

    let appointment = tx.query_row(
        "SELECT * FROM appointments WHERE id = ?1",
        params![id],
        Appointment::from_row,
    )?;

    tx.commit()?;
    Ok(appointment)
}

pub fn set_status(
    conn: &mut Connection,
    appointment_id: String,
    status: String,
) -> AppResult<Appointment> {
    if !VALID_STATUSES.contains(&status.as_str()) {
        return Err(AppError::validation("Estado de cita inválido"));
    }

    let updated = conn.execute(
        "UPDATE appointments SET status = ?1, updated_at = ?2 WHERE id = ?3",
        params![status, now_iso(), appointment_id],
    )?;
    if updated == 0 {
        return Err(AppError::NotFound);
    }

    let appointment = conn.query_row(
        "SELECT * FROM appointments WHERE id = ?1",
        params![appointment_id],
        Appointment::from_row,
    )?;
    Ok(appointment)
}

/// Hoy `completeAppointmentWithSale` (src/lib/appointments.ts) hace dos
/// llamadas Supabase no-atómicas (update de `sales` y luego de
/// `appointments`). Aquí se vuelve una sola transacción SQLite: mejora de
/// correctitud pedida explícitamente por el plan de migración. Vive en el
/// dominio `appointments` (no `sales`) porque es la cita la que se marca
/// como completada; el wrapper en `src/lib/appointments.ts` es llamado por
/// `src/components/admin/sale-form.tsx` (dominio de ventas, sin tocar).
pub fn complete_with_sale(
    conn: &mut Connection,
    appointment_id: String,
    sale_id: String,
) -> AppResult<()> {
    let tx = conn.transaction()?;
    let now = now_iso();

    let updated_appt = tx.execute(
        "UPDATE appointments SET status = 'completed', sale_id = ?1, updated_at = ?2 WHERE id = ?3",
        params![sale_id, now, appointment_id],
    )?;
    if updated_appt == 0 {
        return Err(AppError::NotFound);
    }

    let updated_sale = tx.execute(
        "UPDATE sales SET appointment_id = ?1 WHERE id = ?2",
        params![appointment_id, sale_id],
    )?;
    if updated_sale == 0 {
        return Err(AppError::NotFound);
    }

    tx.commit()?;
    Ok(())
}

/// Lectura mínima y de solo lectura para el selector de servicio del
/// formulario de citas (dominio `appointments`). `repositories::services::list`
/// (dominio `services`) ya existe, pero pagina y también devuelve borradores;
/// este comando se queda separado e intencionalmente simple (sin paginar,
/// solo publicados) porque es todo lo que necesita el selector del
/// formulario de citas.
pub fn list_published_services(conn: &mut Connection) -> AppResult<Vec<ServiceRef>> {
    let mut stmt = conn.prepare(
        "SELECT id, name, price, duration_minutes, status FROM services
         WHERE status = 'published' ORDER BY name ASC",
    )?;
    let rows = stmt.query_map([], |row| {
        Ok(ServiceRef {
            id: row.get("id")?,
            name: row.get("name")?,
            price: row.get("price")?,
            duration_minutes: row.get("duration_minutes")?,
            status: row.get("status")?,
        })
    })?;
    Ok(rows.collect::<Result<Vec<_>, _>>()?)
}

/// Implementación PostgreSQL (issue #9, Fase 2). Los helpers de validación
/// (`assert_employee_active`, `assert_customer_exists`, `load_published_service`,
/// `appointment_has_overlap`) son genéricos sobre `PgExecutor` para poder
/// correr tanto contra el pool como dentro de una transacción con el mismo
/// código. `starts_at`/`ends_at` llegan como `String` desde el frontend (y
/// las helpers puras `compute_ends_at`/`normalize_notes` del módulo padre se
/// reusan tal cual, sin cambios), pero para comparar/guardar contra columnas
/// `TIMESTAMPTZ` hace falta parsearlas a `time::OffsetDateTime` antes de
/// pasarlas como parámetro — a diferencia de SQLite, donde todo es texto. En
/// `update` se agrega `FOR UPDATE` al leer el estado actual: con SQLite había
/// una sola conexión mutex-serializada, pero con Postgres puede haber varias
/// instalaciones escribiendo a la vez, y sin el lock de fila dos ediciones
/// concurrentes de la misma cita podrían pisarse.
pub mod postgres {
    use sqlx::postgres::PgRow;
    use sqlx::{PgExecutor, PgPool, Row};
    use time::format_description::well_known::Rfc3339;

    use crate::commands::appointments::AppointmentInput;
    use crate::commands::customers::{AppointmentWithDetails, CustomerRef, EmployeeRef, ServiceRef};
    use crate::db::{format_pg_timestamp, new_id};
    use crate::error::{AppError, AppResult};
    use crate::models::appointment::Appointment;

    use super::{compute_ends_at, normalize_notes, ServiceInfo, VALID_STATUSES};

    const APPOINTMENT_COLUMNS: &str = "id, employee_id, customer_id, service_id, \
        service_name_snapshot, starts_at, ends_at, status, notes, sale_id, created_at, updated_at";

    fn parse_rfc3339(value: &str) -> AppResult<time::OffsetDateTime> {
        time::OffsetDateTime::parse(value, &Rfc3339)
            .map_err(|_| AppError::validation("Fecha inválida"))
    }

    fn ts(row: &PgRow, col: &str) -> AppResult<String> {
        let value: time::OffsetDateTime = row.try_get(col)?;
        Ok(format_pg_timestamp(value))
    }

    fn map_appointment(row: &PgRow) -> AppResult<Appointment> {
        Ok(Appointment {
            id: row.try_get("id")?,
            employee_id: row.try_get("employee_id")?,
            customer_id: row.try_get("customer_id")?,
            service_id: row.try_get("service_id")?,
            service_name_snapshot: row.try_get("service_name_snapshot")?,
            starts_at: ts(row, "starts_at")?,
            ends_at: ts(row, "ends_at")?,
            status: row.try_get("status")?,
            notes: row.try_get("notes")?,
            sale_id: row.try_get("sale_id")?,
            created_at: ts(row, "created_at")?,
            updated_at: ts(row, "updated_at")?,
        })
    }

    fn map_appointment_with_details(row: &PgRow) -> AppResult<AppointmentWithDetails> {
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

        Ok(AppointmentWithDetails {
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
            customers: CustomerRef {
                id: row.try_get("customer_id")?,
                name: row.try_get("cust_name")?,
                email: row.try_get("cust_email")?,
                phone: row.try_get("cust_phone")?,
                document: row.try_get("cust_document")?,
            },
            services,
        })
    }

    async fn assert_employee_active<'e, E>(executor: E, employee_id: &str) -> AppResult<()>
    where
        E: PgExecutor<'e>,
    {
        let exists: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM employees WHERE id = $1 AND active = TRUE)",
        )
        .bind(employee_id)
        .fetch_one(executor)
        .await?;
        if !exists {
            return Err(AppError::validation("Empleado no disponible"));
        }
        Ok(())
    }

    async fn assert_customer_exists<'e, E>(executor: E, customer_id: &str) -> AppResult<()>
    where
        E: PgExecutor<'e>,
    {
        let exists: bool =
            sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM customers WHERE id = $1)")
                .bind(customer_id)
                .fetch_one(executor)
                .await?;
        if !exists {
            return Err(AppError::validation("Cliente no encontrado"));
        }
        Ok(())
    }

    async fn load_published_service<'e, E>(executor: E, service_id: &str) -> AppResult<ServiceInfo>
    where
        E: PgExecutor<'e>,
    {
        let row =
            sqlx::query("SELECT duration_minutes, name FROM services WHERE id = $1 AND status = 'published'")
                .bind(service_id)
                .fetch_optional(executor)
                .await?
                .ok_or_else(|| AppError::validation("Servicio no disponible"))?;

        Ok(ServiceInfo {
            duration_minutes: row.try_get("duration_minutes")?,
            name: row.try_get("name")?,
        })
    }

    async fn appointment_has_overlap<'e, E>(
        executor: E,
        employee_id: &str,
        starts_at: time::OffsetDateTime,
        ends_at: time::OffsetDateTime,
        exclude_id: Option<&str>,
    ) -> AppResult<bool>
    where
        E: PgExecutor<'e>,
    {
        let overlaps: bool = sqlx::query_scalar(
            "SELECT EXISTS(
                SELECT 1 FROM appointments
                WHERE employee_id = $1
                  AND status = 'scheduled'
                  AND starts_at < $2
                  AND ends_at > $3
                  AND ($4::text IS NULL OR id <> $4)
             )",
        )
        .bind(employee_id)
        .bind(ends_at)
        .bind(starts_at)
        .bind(exclude_id)
        .fetch_one(executor)
        .await?;
        Ok(overlaps)
    }

    pub async fn list_in_range(
        pool: &PgPool,
        from: String,
        to: String,
        employee_id: Option<String>,
    ) -> AppResult<Vec<AppointmentWithDetails>> {
        let from_dt = parse_rfc3339(&from)?;
        let to_dt = parse_rfc3339(&to)?;

        let rows = sqlx::query(
            "SELECT a.id, a.employee_id, a.customer_id, a.service_id, a.service_name_snapshot,
                    a.starts_at, a.ends_at, a.status, a.notes, a.sale_id, a.created_at, a.updated_at,
                    e.name AS emp_name, e.color AS emp_color, e.active AS emp_active,
                    c.name AS cust_name, COALESCE(c.email, '') AS cust_email, c.phone AS cust_phone,
                    COALESCE(c.document, '') AS cust_document,
                    s.price AS svc_price, s.duration_minutes AS svc_duration, s.status AS svc_status
             FROM appointments a
             JOIN employees e ON e.id = a.employee_id
             JOIN customers c ON c.id = a.customer_id
             LEFT JOIN services s ON s.id = a.service_id
             WHERE a.starts_at >= $1 AND a.starts_at < $2
               AND ($3::text IS NULL OR a.employee_id = $3)
             ORDER BY a.starts_at ASC",
        )
        .bind(from_dt)
        .bind(to_dt)
        .bind(&employee_id)
        .fetch_all(pool)
        .await?;

        rows.iter().map(map_appointment_with_details).collect()
    }

    pub async fn create(pool: &PgPool, input: AppointmentInput) -> AppResult<Appointment> {
        if input.starts_at.trim().is_empty() {
            return Err(AppError::validation("Fecha de inicio requerida"));
        }

        let mut tx = pool.begin().await?;

        assert_employee_active(&mut *tx, &input.employee_id).await?;
        assert_customer_exists(&mut *tx, &input.customer_id).await?;
        let service = load_published_service(&mut *tx, &input.service_id).await?;
        let ends_at_str = compute_ends_at(&input.starts_at, service.duration_minutes)?;

        let starts_at_dt = parse_rfc3339(&input.starts_at)?;
        let ends_at_dt = parse_rfc3339(&ends_at_str)?;

        crate::repositories::business_hours::postgres::assert_appointment_within_schedule(
            &mut tx,
            &input.employee_id,
            input.local_weekday,
            input.local_start_minutes,
            input.local_start_minutes + service.duration_minutes,
        )
        .await?;

        if appointment_has_overlap(&mut *tx, &input.employee_id, starts_at_dt, ends_at_dt, None)
            .await?
        {
            return Err(AppError::validation(
                "El empleado ya tiene una cita en ese horario",
            ));
        }

        let notes = normalize_notes(input.notes);

        let row = sqlx::query(&format!(
            "INSERT INTO appointments (
                id, employee_id, customer_id, service_id, service_name_snapshot,
                starts_at, ends_at, status, notes
             ) VALUES ($1, $2, $3, $4, $5, $6, $7, 'scheduled', $8)
             RETURNING {APPOINTMENT_COLUMNS}"
        ))
        .bind(new_id())
        .bind(&input.employee_id)
        .bind(&input.customer_id)
        .bind(&input.service_id)
        .bind(&service.name)
        .bind(starts_at_dt)
        .bind(ends_at_dt)
        .bind(&notes)
        .fetch_one(&mut *tx)
        .await?;

        let appointment = map_appointment(&row)?;
        tx.commit().await?;
        Ok(appointment)
    }

    pub async fn update(
        pool: &PgPool,
        id: String,
        input: AppointmentInput,
    ) -> AppResult<Appointment> {
        if input.starts_at.trim().is_empty() {
            return Err(AppError::validation("Fecha de inicio requerida"));
        }

        let mut tx = pool.begin().await?;

        let current_status: String = sqlx::query_scalar(
            "SELECT status FROM appointments WHERE id = $1 FOR UPDATE",
        )
        .bind(&id)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or(AppError::NotFound)?;

        if current_status != "scheduled" {
            return Err(AppError::validation(
                "Solo se pueden editar citas programadas",
            ));
        }

        assert_employee_active(&mut *tx, &input.employee_id).await?;
        assert_customer_exists(&mut *tx, &input.customer_id).await?;
        let service = load_published_service(&mut *tx, &input.service_id).await?;
        let ends_at_str = compute_ends_at(&input.starts_at, service.duration_minutes)?;

        let starts_at_dt = parse_rfc3339(&input.starts_at)?;
        let ends_at_dt = parse_rfc3339(&ends_at_str)?;

        crate::repositories::business_hours::postgres::assert_appointment_within_schedule(
            &mut tx,
            &input.employee_id,
            input.local_weekday,
            input.local_start_minutes,
            input.local_start_minutes + service.duration_minutes,
        )
        .await?;

        if appointment_has_overlap(
            &mut *tx,
            &input.employee_id,
            starts_at_dt,
            ends_at_dt,
            Some(id.as_str()),
        )
        .await?
        {
            return Err(AppError::validation(
                "El empleado ya tiene una cita en ese horario",
            ));
        }

        let notes = normalize_notes(input.notes);

        let row = sqlx::query(&format!(
            "UPDATE appointments SET
                employee_id = $1,
                customer_id = $2,
                service_id = $3,
                service_name_snapshot = $4,
                starts_at = $5,
                ends_at = $6,
                notes = $7,
                updated_at = now()
             WHERE id = $8
             RETURNING {APPOINTMENT_COLUMNS}"
        ))
        .bind(&input.employee_id)
        .bind(&input.customer_id)
        .bind(&input.service_id)
        .bind(&service.name)
        .bind(starts_at_dt)
        .bind(ends_at_dt)
        .bind(&notes)
        .bind(&id)
        .fetch_one(&mut *tx)
        .await?;

        let appointment = map_appointment(&row)?;
        tx.commit().await?;
        Ok(appointment)
    }

    pub async fn set_status(
        pool: &PgPool,
        appointment_id: String,
        status: String,
    ) -> AppResult<Appointment> {
        if !VALID_STATUSES.contains(&status.as_str()) {
            return Err(AppError::validation("Estado de cita inválido"));
        }

        let row = sqlx::query(&format!(
            "UPDATE appointments SET status = $1, updated_at = now() WHERE id = $2
             RETURNING {APPOINTMENT_COLUMNS}"
        ))
        .bind(&status)
        .bind(&appointment_id)
        .fetch_optional(pool)
        .await?
        .ok_or(AppError::NotFound)?;

        map_appointment(&row)
    }

    pub async fn complete_with_sale(
        pool: &PgPool,
        appointment_id: String,
        sale_id: String,
    ) -> AppResult<()> {
        let mut tx = pool.begin().await?;

        let updated_appt = sqlx::query(
            "UPDATE appointments SET status = 'completed', sale_id = $1, updated_at = now() WHERE id = $2",
        )
        .bind(&sale_id)
        .bind(&appointment_id)
        .execute(&mut *tx)
        .await?
        .rows_affected();
        if updated_appt == 0 {
            return Err(AppError::NotFound);
        }

        let updated_sale = sqlx::query("UPDATE sales SET appointment_id = $1 WHERE id = $2")
            .bind(&appointment_id)
            .bind(&sale_id)
            .execute(&mut *tx)
            .await?
            .rows_affected();
        if updated_sale == 0 {
            return Err(AppError::NotFound);
        }

        tx.commit().await?;
        Ok(())
    }

    pub async fn list_published_services(pool: &PgPool) -> AppResult<Vec<ServiceRef>> {
        let rows = sqlx::query(
            "SELECT id, name, price, duration_minutes, status FROM services
             WHERE status = 'published' ORDER BY name ASC",
        )
        .fetch_all(pool)
        .await?;

        rows.iter()
            .map(|row| -> AppResult<ServiceRef> {
                Ok(ServiceRef {
                    id: row.try_get("id")?,
                    name: row.try_get("name")?,
                    price: row.try_get("price")?,
                    duration_minutes: row.try_get("duration_minutes")?,
                    status: row.try_get("status")?,
                })
            })
            .collect()
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

    fn seed_fixtures(conn: &Connection) {
        conn.execute(
            "INSERT INTO employees (id, name, color, active) VALUES ('emp-1', 'Ana', '#fff', 1)",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO customers (id, name, phone, email, document)
             VALUES ('cust-1', 'Cliente', '3000000000', 'cliente@test.com', 'DOC-1')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO services (id, name, price, duration_minutes, status)
             VALUES ('svc-1', 'Corte', 20.0, 60, 'published')",
            [],
        )
        .unwrap();
    }

    /// 2026-01-10 es sábado (weekday 6 en la convención de `Date.getDay()`),
    /// dentro del horario de negocio sembrado por la migración (sáb.
    /// 09:00-17:00) — los tests tratan la hora UTC del literal como si ya
    /// fuera la hora local, igual que haría el frontend real al mandar
    /// `local_weekday`/`local_start_minutes` ya resueltos.
    fn appointment_input(starts_at: &str, local_start_minutes: i64) -> AppointmentInput {
        AppointmentInput {
            employee_id: "emp-1".to_string(),
            customer_id: "cust-1".to_string(),
            service_id: "svc-1".to_string(),
            starts_at: starts_at.to_string(),
            notes: None,
            local_weekday: 6,
            local_start_minutes,
        }
    }

    /// El servicio dura 60 minutos; agendar una segunda cita que empieza a
    /// mitad de la primera debe rechazarse por solapamiento con el mismo
    /// empleado.
    #[test]
    fn create_rejects_overlapping_appointment_for_same_employee() {
        let mut conn = open_test_db();
        seed_fixtures(&conn);

        create(&mut conn, appointment_input("2026-01-10T10:00:00Z", 10 * 60)).unwrap();
        let result = create(&mut conn, appointment_input("2026-01-10T10:30:00Z", 10 * 60 + 30));

        assert!(result.is_err(), "debe rechazar una cita que se solapa con otra del mismo empleado");
    }

    #[test]
    fn create_allows_back_to_back_appointments() {
        let mut conn = open_test_db();
        seed_fixtures(&conn);

        create(&mut conn, appointment_input("2026-01-10T10:00:00Z", 10 * 60)).unwrap();
        let result = create(&mut conn, appointment_input("2026-01-10T11:00:00Z", 11 * 60));

        assert!(result.is_ok(), "una cita que empieza justo cuando termina la anterior no debe solaparse");
    }

    /// Issue #2, Fase 3: la migración siembra el domingo (weekday 0) como
    /// cerrado — no debería poder agendarse una cita ese día sin una
    /// excepción explícita del empleado.
    #[test]
    fn create_rejects_appointment_when_business_is_closed() {
        let mut conn = open_test_db();
        seed_fixtures(&conn);

        let mut input = appointment_input("2026-01-11T10:00:00Z", 10 * 60);
        input.local_weekday = 0; // domingo

        let result = create(&mut conn, input);

        assert!(result.is_err(), "no debe poder agendar un domingo si el negocio está cerrado");
    }

    /// El sábado abre 09:00-17:00 (sembrado por la migración) — una cita que
    /// terminaría después del cierre debe rechazarse.
    #[test]
    fn create_rejects_appointment_outside_business_hours() {
        let mut conn = open_test_db();
        seed_fixtures(&conn);

        let result = create(&mut conn, appointment_input("2026-01-10T18:00:00Z", 18 * 60));

        assert!(result.is_err(), "18:00-19:00 cae fuera del horario de cierre (17:00)");
    }

    /// Una excepción de disponibilidad del empleado reemplaza el horario de
    /// negocio ese día: si el negocio está cerrado un domingo pero el
    /// empleado tiene una excepción habilitándolo, la cita sí debe poder
    /// agendarse dentro de esa ventana propia.
    #[test]
    fn create_allows_appointment_within_employee_override_on_closed_business_day() {
        let mut conn = open_test_db();
        seed_fixtures(&conn);
        conn.execute(
            "INSERT INTO employee_availability (id, employee_id, weekday, is_available, starts_at, ends_at)
             VALUES ('avail-1', 'emp-1', 0, 1, '10:00', '14:00')",
            [],
        )
        .unwrap();

        let mut input = appointment_input("2026-01-11T10:00:00Z", 10 * 60);
        input.local_weekday = 0;

        let result = create(&mut conn, input);

        assert!(result.is_ok(), "la excepción del empleado debe habilitar el domingo aunque el negocio esté cerrado");
    }

    /// Una excepción con `is_available = 0` bloquea al empleado ese día
    /// aunque el negocio esté abierto.
    #[test]
    fn create_rejects_appointment_when_employee_marked_unavailable() {
        let mut conn = open_test_db();
        seed_fixtures(&conn);
        conn.execute(
            "INSERT INTO employee_availability (id, employee_id, weekday, is_available)
             VALUES ('avail-1', 'emp-1', 6, 0)",
            [],
        )
        .unwrap();

        let result = create(&mut conn, appointment_input("2026-01-10T10:00:00Z", 10 * 60));

        assert!(result.is_err(), "el empleado no debe poder atender un día marcado como no disponible");
    }
}
