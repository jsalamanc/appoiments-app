use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Service {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub price: f64,
    pub duration_minutes: i64,
    pub photo_path: Option<String>,
    pub status: String,
    pub created_at: String,
    pub updated_at: String,
}

impl Service {
    pub fn from_row(row: &rusqlite::Row) -> rusqlite::Result<Self> {
        Ok(Self {
            id: row.get("id")?,
            name: row.get("name")?,
            description: row.get("description")?,
            price: row.get("price")?,
            duration_minutes: row.get("duration_minutes")?,
            photo_path: row.get("photo_path")?,
            status: row.get("status")?,
            created_at: row.get("created_at")?,
            updated_at: row.get("updated_at")?,
        })
    }

    /// Equivalente PostgreSQL de `from_row` (issue #9, Fase 2). `duration_minutes`
    /// es `INTEGER` (4 bytes) en Postgres, decodificado como `i32` por `sqlx`;
    /// se castea a `i64` para que el modelo sea idéntico en ambos backends.
    pub fn from_pg_row(row: &sqlx::postgres::PgRow) -> crate::error::AppResult<Self> {
        use sqlx::Row;
        let created_at: time::OffsetDateTime = row.try_get("created_at")?;
        let updated_at: time::OffsetDateTime = row.try_get("updated_at")?;
        Ok(Self {
            id: row.try_get("id")?,
            name: row.try_get("name")?,
            description: row.try_get("description")?,
            price: row.try_get("price")?,
            duration_minutes: row.try_get("duration_minutes")?,
            photo_path: row.try_get("photo_path")?,
            status: row.try_get("status")?,
            created_at: crate::db::format_pg_timestamp(created_at),
            updated_at: crate::db::format_pg_timestamp(updated_at),
        })
    }
}
