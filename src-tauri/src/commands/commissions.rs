//! Comisión por empleado sobre las citas completadas (issue #22, #2 Fase 4).
//! Ver `commands::reports::get_employee_commissions_report` para el reporte
//! que calcula el monto a partir de estas tasas.

use serde::Deserialize;
use tauri::State;

use crate::app_db::AppDb;
use crate::error::AppResult;
use crate::models::commission::EmployeeCommissionRate;
use crate::repositories::commissions as repo;
use crate::state::AppState;

/// `service_id = None` es la tasa general del empleado; `Some(id)` es una
/// excepción para ese servicio puntual.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CommissionRateInput {
    pub service_id: Option<String>,
    pub rate_type: String,
    pub rate_value: f64,
}

#[tauri::command]
pub async fn get_employee_commission_rates(
    state: State<'_, AppState>,
    employee_id: String,
) -> AppResult<Vec<EmployeeCommissionRate>> {
    state.require_owner()?;
    match &state.db {
        AppDb::Sqlite(_) => {
            state
                .db
                .with_sqlite(move |conn| repo::list_employee_commission_rates(conn, employee_id))
                .await
        }
        AppDb::Postgres(pool) => {
            repo::postgres::list_employee_commission_rates(pool, employee_id).await
        }
    }
}

/// Reemplaza todas las tasas del empleado (igual semántica que
/// `update_employee_availability`): se manda la lista completa, incluyendo
/// como máximo una entrada con `serviceId: null` (la tasa general).
#[tauri::command]
pub async fn update_employee_commission_rates(
    state: State<'_, AppState>,
    employee_id: String,
    rates: Vec<CommissionRateInput>,
) -> AppResult<Vec<EmployeeCommissionRate>> {
    state.require_owner()?;
    match &state.db {
        AppDb::Sqlite(_) => {
            state
                .db
                .with_sqlite(move |conn| {
                    repo::update_employee_commission_rates(conn, employee_id, rates)
                })
                .await
        }
        AppDb::Postgres(pool) => {
            repo::postgres::update_employee_commission_rates(pool, employee_id, rates).await
        }
    }
}
