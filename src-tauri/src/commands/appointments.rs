use rusqlite::{params, OptionalExtension};
use serde::Deserialize;
use tauri::State;
use time::format_description::well_known::Rfc3339;

use crate::commands::customers::{AppointmentWithDetails, CustomerRef, EmployeeRef, ServiceRef};
use crate::db::{new_id, now_iso};
use crate::error::{AppError, AppResult};
use crate::models::appointment::Appointment;
use crate::state::AppState;

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

#[tauri::command]
pub fn list_appointments_in_range(
    state: State<AppState>,
    from: String,
    to: String,
    employee_id: Option<String>,
) -> AppResult<Vec<AppointmentWithDetails>> {
    let conn = state.db.lock().unwrap();

    let mut stmt = conn.prepare(
        "SELECT a.*,
                e.name AS emp_name, e.color AS emp_color, e.active AS emp_active,
                c.name AS cust_name, c.email AS cust_email, c.phone AS cust_phone,
                c.document AS cust_document,
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

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppointmentInput {
    pub employee_id: String,
    pub customer_id: String,
    pub service_id: String,
    pub starts_at: String,
    pub notes: Option<String>,
}

#[tauri::command]
pub fn create_appointment(
    state: State<AppState>,
    input: AppointmentInput,
) -> AppResult<Appointment> {
    if input.starts_at.trim().is_empty() {
        return Err(AppError::validation("Fecha de inicio requerida"));
    }

    let mut conn = state.db.lock().unwrap();
    let tx = conn.transaction()?;

    assert_employee_active(&tx, &input.employee_id)?;
    assert_customer_exists(&tx, &input.customer_id)?;
    let service = load_published_service(&tx, &input.service_id)?;
    let ends_at = compute_ends_at(&input.starts_at, service.duration_minutes)?;

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

#[tauri::command]
pub fn update_appointment(
    state: State<AppState>,
    id: String,
    input: AppointmentInput,
) -> AppResult<Appointment> {
    if input.starts_at.trim().is_empty() {
        return Err(AppError::validation("Fecha de inicio requerida"));
    }

    let mut conn = state.db.lock().unwrap();
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

#[tauri::command]
pub fn set_appointment_status(
    state: State<AppState>,
    appointment_id: String,
    status: String,
) -> AppResult<Appointment> {
    if !VALID_STATUSES.contains(&status.as_str()) {
        return Err(AppError::validation("Estado de cita inválido"));
    }

    let conn = state.db.lock().unwrap();
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
#[tauri::command]
pub fn complete_appointment_with_sale(
    state: State<AppState>,
    appointment_id: String,
    sale_id: String,
) -> AppResult<()> {
    let mut conn = state.db.lock().unwrap();
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
/// formulario de citas (dominio `appointments`). El dominio `services`
/// (propiedad de otro ingeniero, fuera de este alcance) definirá su propio
/// `list_services` paginado más adelante; este comando es intencionalmente
/// independiente (nombre distinto, sin paginar) para no bloquear `employees`
/// + `appointments` en ese trabajo paralelo. Ver reporte final para detalles.
#[tauri::command]
pub fn list_published_services_for_appointments(
    state: State<AppState>,
) -> AppResult<Vec<ServiceRef>> {
    let conn = state.db.lock().unwrap();
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
