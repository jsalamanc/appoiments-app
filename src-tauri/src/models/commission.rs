use serde::{Deserialize, Serialize};

/// Tasa de comisión de un empleado — `service_id = None` es la tasa general
/// del empleado; `service_id = Some(..)` es una excepción puntual para ese
/// servicio, que prevalece sobre la general (ver
/// `repositories::commissions::resolve_rate`).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EmployeeCommissionRate {
    pub id: String,
    pub employee_id: String,
    pub service_id: Option<String>,
    pub rate_type: String,
    pub rate_value: f64,
}

impl EmployeeCommissionRate {
    pub fn from_row(row: &rusqlite::Row) -> rusqlite::Result<Self> {
        Ok(Self {
            id: row.get("id")?,
            employee_id: row.get("employee_id")?,
            service_id: row.get("service_id")?,
            rate_type: row.get("rate_type")?,
            rate_value: row.get("rate_value")?,
        })
    }

    pub fn from_pg_row(row: &sqlx::postgres::PgRow) -> crate::error::AppResult<Self> {
        use sqlx::Row;
        Ok(Self {
            id: row.try_get("id")?,
            employee_id: row.try_get("employee_id")?,
            service_id: row.try_get("service_id")?,
            rate_type: row.try_get("rate_type")?,
            rate_value: row.try_get("rate_value")?,
        })
    }
}
