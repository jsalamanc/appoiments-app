use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Employee {
    pub id: String,
    pub name: String,
    pub color: String,
    pub active: bool,
    pub created_at: String,
    pub updated_at: String,
}

impl Employee {
    pub fn from_row(row: &rusqlite::Row) -> rusqlite::Result<Self> {
        Ok(Self {
            id: row.get("id")?,
            name: row.get("name")?,
            color: row.get("color")?,
            active: row.get::<_, i64>("active")? != 0,
            created_at: row.get("created_at")?,
            updated_at: row.get("updated_at")?,
        })
    }

    /// Equivalente PostgreSQL de `from_row` (issue #9, Fase 2). `active` ya
    /// es `BOOLEAN` nativo, sin el `!= 0` que hace falta en SQLite.
    pub fn from_pg_row(row: &sqlx::postgres::PgRow) -> crate::error::AppResult<Self> {
        use sqlx::Row;
        let created_at: time::OffsetDateTime = row.try_get("created_at")?;
        let updated_at: time::OffsetDateTime = row.try_get("updated_at")?;
        Ok(Self {
            id: row.try_get("id")?,
            name: row.try_get("name")?,
            color: row.try_get("color")?,
            active: row.try_get("active")?,
            created_at: crate::db::format_pg_timestamp(created_at),
            updated_at: crate::db::format_pg_timestamp(updated_at),
        })
    }
}
