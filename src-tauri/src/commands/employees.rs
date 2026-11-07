use serde::Deserialize;
use tauri::State;

use crate::app_db::AppDb;
use crate::audit;
use crate::error::AppResult;
use crate::models::employee::Employee;
use crate::repositories::employees as repo;
use crate::state::AppState;

#[tauri::command]
pub async fn list_employees(
    state: State<'_, AppState>,
    active_only: Option<bool>,
) -> AppResult<Vec<Employee>> {
    state.require_user()?;
    match &state.db {
        AppDb::Sqlite(_) => {
            state
                .db
                .with_sqlite(move |conn| repo::list(conn, active_only))
                .await
        }
        AppDb::Postgres(pool) => repo::postgres::list(pool, active_only).await,
    }
}

#[derive(Debug, Deserialize)]
pub struct EmployeeInput {
    pub name: String,
    pub color: String,
    pub active: bool,
}

#[tauri::command]
pub async fn create_employee(
    state: State<'_, AppState>,
    input: EmployeeInput,
) -> AppResult<Employee> {
    let actor = state.require_owner()?;
    let tracker = audit::Tracker::start(&state.db, &actor, "employee.create", "employees", None, &[]).await;
    let result = match &state.db {
        AppDb::Sqlite(_) => state.db.with_sqlite(move |conn| repo::create(conn, input)).await,
        AppDb::Postgres(pool) => repo::postgres::create(pool, input).await,
    }?;
    tracker.finish_created("Creó empleado", &result.id).await;
    Ok(result)
}

#[tauri::command]
pub async fn update_employee(
    state: State<'_, AppState>,
    id: String,
    input: EmployeeInput,
) -> AppResult<Employee> {
    let actor = state.require_owner()?;
    let tracker = audit::Tracker::start(&state.db, &actor, "employee.update", "employees", Some(&id), &[]).await;
    let result = match &state.db {
        AppDb::Sqlite(_) => {
            state
                .db
                .with_sqlite(move |conn| repo::update(conn, id, input))
                .await
        }
        AppDb::Postgres(pool) => repo::postgres::update(pool, id, input).await,
    }?;
    tracker.finish("Actualizó empleado").await;
    Ok(result)
}

/// Falla con un mensaje claro si el empleado tiene citas asociadas: el
/// `ON DELETE RESTRICT` de `appointments.employee_id` dispara una violación
/// de llave foránea. SQLite no incluye el nombre de la columna en el mensaje
/// de error de una violación de FK (a diferencia de UNIQUE), así que
/// `AppError::from(rusqlite::Error)` (ver src-tauri/src/error.rs) solo puede
/// dar un mensaje genérico de conflicto; aquí lo reemplazamos por el mismo
/// mensaje que ya mostraba la UI en la era Supabase, guiando a desactivar en
/// vez de eliminar.
#[tauri::command]
pub async fn delete_employee(state: State<'_, AppState>, id: String) -> AppResult<()> {
    let actor = state.require_owner()?;
    let tracker = audit::Tracker::start(&state.db, &actor, "employee.delete", "employees", Some(&id), &[("employee_commission_rates", "employee_id"), ("employee_availability", "employee_id")]).await;
    let result = match &state.db {
        AppDb::Sqlite(_) => state.db.with_sqlite(move |conn| repo::delete(conn, id)).await,
        AppDb::Postgres(pool) => repo::postgres::delete(pool, id).await,
    }?;
    tracker.finish_deleted("Eliminó empleado").await;
    Ok(result)
}
