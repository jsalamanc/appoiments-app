use serde::{Deserialize, Serialize};

/// Horario de atención del negocio para un día de la semana — `weekday`
/// usa la misma convención que `Date.getDay()` en JS (domingo = 0 ..
/// sábado = 6) para que el frontend no tenga que traducir nada.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BusinessHours {
    pub weekday: i64,
    pub is_open: bool,
    pub opens_at: Option<String>,
    pub closes_at: Option<String>,
}

impl BusinessHours {
    pub fn from_row(row: &rusqlite::Row) -> rusqlite::Result<Self> {
        Ok(Self {
            weekday: row.get("weekday")?,
            is_open: row.get::<_, i64>("is_open")? != 0,
            opens_at: row.get("opens_at")?,
            closes_at: row.get("closes_at")?,
        })
    }

    pub fn from_pg_row(row: &sqlx::postgres::PgRow) -> crate::error::AppResult<Self> {
        use sqlx::Row;
        Ok(Self {
            weekday: row.try_get::<i16, _>("weekday")? as i64,
            is_open: row.try_get("is_open")?,
            opens_at: row.try_get("opens_at")?,
            closes_at: row.try_get("closes_at")?,
        })
    }
}

/// Excepción de disponibilidad de un empleado para un día de la semana —
/// sparse: si no hay fila para un empleado+día, sigue el horario de negocio
/// ese día (ver `repositories::business_hours`).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EmployeeAvailability {
    pub id: String,
    pub employee_id: String,
    pub weekday: i64,
    pub is_available: bool,
    pub starts_at: Option<String>,
    pub ends_at: Option<String>,
}

impl EmployeeAvailability {
    pub fn from_row(row: &rusqlite::Row) -> rusqlite::Result<Self> {
        Ok(Self {
            id: row.get("id")?,
            employee_id: row.get("employee_id")?,
            weekday: row.get("weekday")?,
            is_available: row.get::<_, i64>("is_available")? != 0,
            starts_at: row.get("starts_at")?,
            ends_at: row.get("ends_at")?,
        })
    }

    pub fn from_pg_row(row: &sqlx::postgres::PgRow) -> crate::error::AppResult<Self> {
        use sqlx::Row;
        Ok(Self {
            id: row.try_get("id")?,
            employee_id: row.try_get("employee_id")?,
            weekday: row.try_get::<i16, _>("weekday")? as i64,
            is_available: row.try_get("is_available")?,
            starts_at: row.try_get("starts_at")?,
            ends_at: row.try_get("ends_at")?,
        })
    }
}
