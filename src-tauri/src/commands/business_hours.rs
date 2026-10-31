//! Horario de atención del negocio y disponibilidad por empleado (issue #2,
//! Fase 3). `weekday` en toda la API es domingo=0 .. sábado=6, la misma
//! convención que `Date.getDay()` en JS, para que el frontend no tenga que
//! traducir nada.

use serde::Deserialize;
use tauri::State;

use crate::app_db::AppDb;
use crate::error::AppResult;
use crate::models::business_hours::{BusinessHours, EmployeeAvailability};
use crate::repositories::business_hours as repo;
use crate::state::AppState;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BusinessHoursInput {
    pub weekday: i64,
    pub is_open: bool,
    pub opens_at: Option<String>,
    pub closes_at: Option<String>,
}

#[tauri::command]
pub async fn get_business_hours(state: State<'_, AppState>) -> AppResult<Vec<BusinessHours>> {
    state.require_user()?;
    match &state.db {
        AppDb::Sqlite(_) => state.db.with_sqlite(repo::list_business_hours).await,
        AppDb::Postgres(pool) => repo::postgres::list_business_hours(pool).await,
    }
}

/// Reemplaza el horario completo (siempre se mandan los 7 días). Solo el
/// dueño puede configurarlo.
#[tauri::command]
pub async fn update_business_hours(
    state: State<'_, AppState>,
    hours: Vec<BusinessHoursInput>,
) -> AppResult<Vec<BusinessHours>> {
    state.require_owner()?;
    match &state.db {
        AppDb::Sqlite(_) => {
            state
                .db
                .with_sqlite(move |conn| repo::update_business_hours(conn, hours))
                .await
        }
        AppDb::Postgres(pool) => repo::postgres::update_business_hours(pool, hours).await,
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EmployeeAvailabilityInput {
    pub weekday: i64,
    pub is_available: bool,
    pub starts_at: Option<String>,
    pub ends_at: Option<String>,
}

#[tauri::command]
pub async fn get_employee_availability(
    state: State<'_, AppState>,
    employee_id: String,
) -> AppResult<Vec<EmployeeAvailability>> {
    state.require_user()?;
    match &state.db {
        AppDb::Sqlite(_) => {
            state
                .db
                .with_sqlite(move |conn| repo::list_employee_availability(conn, employee_id))
                .await
        }
        AppDb::Postgres(pool) => repo::postgres::list_employee_availability(pool, employee_id).await,
    }
}

/// Reemplaza las excepciones de un empleado (solo se guardan los días que
/// tengan una fila — ver `repositories::business_hours`). Solo el dueño
/// puede configurarlo, igual que crear/editar empleados.
#[tauri::command]
pub async fn update_employee_availability(
    state: State<'_, AppState>,
    employee_id: String,
    overrides: Vec<EmployeeAvailabilityInput>,
) -> AppResult<Vec<EmployeeAvailability>> {
    state.require_owner()?;
    match &state.db {
        AppDb::Sqlite(_) => {
            state
                .db
                .with_sqlite(move |conn| {
                    repo::update_employee_availability(conn, employee_id, overrides)
                })
                .await
        }
        AppDb::Postgres(pool) => {
            repo::postgres::update_employee_availability(pool, employee_id, overrides).await
        }
    }
}
