use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InventoryMovement {
    pub id: String,
    pub product_id: String,
    pub movement_type: String,
    pub quantity_delta: i64,
    pub stock_after: i64,
    pub sale_id: Option<String>,
    pub note: Option<String>,
    pub created_at: String,
}

impl InventoryMovement {
    pub fn from_row(row: &rusqlite::Row) -> rusqlite::Result<Self> {
        Ok(Self {
            id: row.get("id")?,
            product_id: row.get("product_id")?,
            movement_type: row.get("movement_type")?,
            quantity_delta: row.get("quantity_delta")?,
            stock_after: row.get("stock_after")?,
            sale_id: row.get("sale_id")?,
            note: row.get("note")?,
            created_at: row.get("created_at")?,
        })
    }

    /// Equivalente PostgreSQL de `from_row` (issue #9, Fase 2).
    /// `quantity_delta`/`stock_after` son `INTEGER` (4 bytes) en Postgres,
    /// decodificados como `i32` por `sqlx`; se castean a `i64` para que el
    /// modelo sea idéntico en ambos backends.
    pub fn from_pg_row(row: &sqlx::postgres::PgRow) -> crate::error::AppResult<Self> {
        use sqlx::Row;
        let created_at: time::OffsetDateTime = row.try_get("created_at")?;
        Ok(Self {
            id: row.try_get("id")?,
            product_id: row.try_get("product_id")?,
            movement_type: row.try_get("movement_type")?,
            quantity_delta: row.try_get("quantity_delta")?,
            stock_after: row.try_get("stock_after")?,
            sale_id: row.try_get("sale_id")?,
            note: row.try_get("note")?,
            created_at: crate::db::format_pg_timestamp(created_at),
        })
    }
}
