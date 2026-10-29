use serde::{Deserialize, Serialize};

/// Issue #2, Fase 5: apertura y cierre de caja. `expected_amount`/`difference`
/// se calculan una sola vez al cerrar (ver `repositories::cash_sessions::close`)
/// y quedan fijos — no se recalculan después aunque se anule una venta ya
/// incluida en un turno cerrado.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CashSession {
    pub id: String,
    pub opened_by: Option<String>,
    pub opened_at: String,
    pub opening_amount: f64,
    pub closed_by: Option<String>,
    pub closed_at: Option<String>,
    pub counted_amount: Option<f64>,
    pub expected_amount: Option<f64>,
    pub difference: Option<f64>,
    pub notes: Option<String>,
}

impl CashSession {
    pub fn from_row(row: &rusqlite::Row) -> rusqlite::Result<Self> {
        Ok(Self {
            id: row.get("id")?,
            opened_by: row.get("opened_by")?,
            opened_at: row.get("opened_at")?,
            opening_amount: row.get("opening_amount")?,
            closed_by: row.get("closed_by")?,
            closed_at: row.get("closed_at")?,
            counted_amount: row.get("counted_amount")?,
            expected_amount: row.get("expected_amount")?,
            difference: row.get("difference")?,
            notes: row.get("notes")?,
        })
    }

    pub fn from_pg_row(row: &sqlx::postgres::PgRow) -> crate::error::AppResult<Self> {
        use sqlx::Row;
        let opened_at: time::OffsetDateTime = row.try_get("opened_at")?;
        let closed_at: Option<time::OffsetDateTime> = row.try_get("closed_at")?;
        Ok(Self {
            id: row.try_get("id")?,
            opened_by: row.try_get("opened_by")?,
            opened_at: crate::db::format_pg_timestamp(opened_at),
            opening_amount: row.try_get("opening_amount")?,
            closed_by: row.try_get("closed_by")?,
            closed_at: closed_at.map(crate::db::format_pg_timestamp),
            counted_amount: row.try_get("counted_amount")?,
            expected_amount: row.try_get("expected_amount")?,
            difference: row.try_get("difference")?,
            notes: row.try_get("notes")?,
        })
    }
}
