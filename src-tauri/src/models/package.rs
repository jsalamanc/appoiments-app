use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Package {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub price: i64,
    pub photo_path: Option<String>,
    pub status: String,
    pub created_at: String,
    pub updated_at: String,
}

impl Package {
    pub fn from_row(row: &rusqlite::Row) -> rusqlite::Result<Self> {
        Ok(Self {
            id: row.get("id")?,
            name: row.get("name")?,
            description: row.get("description")?,
            price: row.get("price")?,
            photo_path: row.get("photo_path")?,
            status: row.get("status")?,
            created_at: row.get("created_at")?,
            updated_at: row.get("updated_at")?,
        })
    }

    /// Equivalente PostgreSQL de `from_row` (issue #9, Fase 2).
    pub fn from_pg_row(row: &sqlx::postgres::PgRow) -> crate::error::AppResult<Self> {
        use sqlx::Row;
        let created_at: time::OffsetDateTime = row.try_get("created_at")?;
        let updated_at: time::OffsetDateTime = row.try_get("updated_at")?;
        Ok(Self {
            id: row.try_get("id")?,
            name: row.try_get("name")?,
            description: row.try_get("description")?,
            price: row.try_get("price")?,
            photo_path: row.try_get("photo_path")?,
            status: row.try_get("status")?,
            created_at: crate::db::format_pg_timestamp(created_at),
            updated_at: crate::db::format_pg_timestamp(updated_at),
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageItem {
    pub id: String,
    pub package_id: String,
    pub item_type: String,
    pub product_id: Option<String>,
    pub service_id: Option<String>,
    pub quantity: i64,
    pub created_at: String,
}

impl PackageItem {
    pub fn from_row(row: &rusqlite::Row) -> rusqlite::Result<Self> {
        Ok(Self {
            id: row.get("id")?,
            package_id: row.get("package_id")?,
            item_type: row.get("item_type")?,
            product_id: row.get("product_id")?,
            service_id: row.get("service_id")?,
            quantity: row.get("quantity")?,
            created_at: row.get("created_at")?,
        })
    }

    /// Equivalente PostgreSQL de `from_row` (issue #9, Fase 2). `quantity` es
    /// `INTEGER` (4 bytes) en Postgres, decodificado como `i32` por `sqlx`;
    /// se castea a `i64` para que el modelo sea idéntico en ambos backends.
    pub fn from_pg_row(row: &sqlx::postgres::PgRow) -> crate::error::AppResult<Self> {
        use sqlx::Row;
        let created_at: time::OffsetDateTime = row.try_get("created_at")?;
        Ok(Self {
            id: row.try_get("id")?,
            package_id: row.try_get("package_id")?,
            item_type: row.try_get("item_type")?,
            product_id: row.try_get("product_id")?,
            service_id: row.try_get("service_id")?,
            quantity: row.try_get("quantity")?,
            created_at: crate::db::format_pg_timestamp(created_at),
        })
    }
}
