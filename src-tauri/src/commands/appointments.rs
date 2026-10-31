use serde::Deserialize;
use tauri::State;

use crate::app_db::AppDb;
use crate::commands::customers::{AppointmentWithDetails, ServiceRef};
use crate::error::AppResult;
use crate::models::appointment::Appointment;
use crate::repositories::appointments as repo;
use crate::state::AppState;

#[tauri::command]
pub async fn list_appointments_in_range(
    state: State<'_, AppState>,
    from: String,
    to: String,
    employee_id: Option<String>,
) -> AppResult<Vec<AppointmentWithDetails>> {
    state.require_user()?;
    match &state.db {
        AppDb::Sqlite(_) => {
            state
                .db
                .with_sqlite(move |conn| repo::list_in_range(conn, from, to, employee_id))
                .await
        }
        AppDb::Postgres(pool) => repo::postgres::list_in_range(pool, from, to, employee_id).await,
    }
}

/// `local_weekday`/`local_start_minutes` describen `starts_at` en la hora
/// LOCAL de quien agenda (domingo=0..sábado=6, minutos desde medianoche) —
/// el frontend ya tiene un `Date` en hora local antes de convertir
/// `starts_at` a UTC para enviarlo (ver `appointment-form-dialog.tsx`), así
/// que se manda ya resuelto en vez de reimplementar el cálculo de huso
/// horario en Rust (mismo criterio que la Fase 1 con `period_from`: el
/// backend no debe adivinar la zona horaria del proceso). El backend asume
/// que una cita no cruza la medianoche local para derivar la hora de fin a
/// partir de `local_start_minutes + duration_minutes`, sin volver a
/// convertir `ends_at` (ver `repositories::appointments::create`).
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppointmentInput {
    pub employee_id: String,
    pub customer_id: String,
    pub service_id: String,
    pub starts_at: String,
    pub notes: Option<String>,
    pub local_weekday: i64,
    pub local_start_minutes: i64,
}

#[tauri::command]
pub async fn create_appointment(
    state: State<'_, AppState>,
    input: AppointmentInput,
) -> AppResult<Appointment> {
    state.require_user()?;
    match &state.db {
        AppDb::Sqlite(_) => state.db.with_sqlite(move |conn| repo::create(conn, input)).await,
        AppDb::Postgres(pool) => repo::postgres::create(pool, input).await,
    }
}

#[tauri::command]
pub async fn update_appointment(
    state: State<'_, AppState>,
    id: String,
    input: AppointmentInput,
) -> AppResult<Appointment> {
    state.require_user()?;
    match &state.db {
        AppDb::Sqlite(_) => {
            state
                .db
                .with_sqlite(move |conn| repo::update(conn, id, input))
                .await
        }
        AppDb::Postgres(pool) => repo::postgres::update(pool, id, input).await,
    }
}

#[tauri::command]
pub async fn set_appointment_status(
    state: State<'_, AppState>,
    appointment_id: String,
    status: String,
) -> AppResult<Appointment> {
    state.require_user()?;
    match &state.db {
        AppDb::Sqlite(_) => {
            state
                .db
                .with_sqlite(move |conn| repo::set_status(conn, appointment_id, status))
                .await
        }
        AppDb::Postgres(pool) => repo::postgres::set_status(pool, appointment_id, status).await,
    }
}

/// Hoy `completeAppointmentWithSale` (src/lib/appointments.ts) hace dos
/// llamadas Supabase no-atómicas (update de `sales` y luego de
/// `appointments`). Aquí se vuelve una sola transacción SQLite: mejora de
/// correctitud pedida explícitamente por el plan de migración. Vive en el
/// dominio `appointments` (no `sales`) porque es la cita la que se marca
/// como completada; el wrapper en `src/lib/appointments.ts` es llamado por
/// `src/components/admin/sale-form.tsx` (dominio de ventas, sin tocar).
#[tauri::command]
pub async fn complete_appointment_with_sale(
    state: State<'_, AppState>,
    appointment_id: String,
    sale_id: String,
) -> AppResult<()> {
    state.require_user()?;
    match &state.db {
        AppDb::Sqlite(_) => {
            state
                .db
                .with_sqlite(move |conn| repo::complete_with_sale(conn, appointment_id, sale_id))
                .await
        }
        AppDb::Postgres(pool) => repo::postgres::complete_with_sale(pool, appointment_id, sale_id).await,
    }
}

/// Lectura mínima y de solo lectura para el selector de servicio del
/// formulario de citas (dominio `appointments`). `commands::services::list_services`
/// (dominio `services`) ya existe, pero pagina y también devuelve borradores;
/// este comando se queda separado e intencionalmente simple (sin paginar,
/// solo publicados) porque es todo lo que necesita el selector del
/// formulario de citas.
#[tauri::command]
pub async fn list_published_services_for_appointments(
    state: State<'_, AppState>,
) -> AppResult<Vec<ServiceRef>> {
    state.require_user()?;
    match &state.db {
        AppDb::Sqlite(_) => state.db.with_sqlite(repo::list_published_services).await,
        AppDb::Postgres(pool) => repo::postgres::list_published_services(pool).await,
    }
}
