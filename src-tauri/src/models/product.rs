use serde::{Deserialize, Serialize};

/// Issue #2, Fase 5: `cost` es lo que le cuesta el producto al negocio (no
/// lo que cobra) — permite calcular margen. Se actualiza manualmente desde
/// el formulario de producto, igual que `price`; opcionalmente también se
/// actualiza al registrar una entrada de stock con costo (ver
/// `repositories::products::set_stock`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Product {
    pub id: String,
    pub name: String,
    pub price: f64,
    pub cost: f64,
    pub photo_path: Option<String>,
    pub status: String,
    pub stock: i64,
    pub created_at: String,
    pub updated_at: String,
}

impl Product {
    pub fn from_row(row: &rusqlite::Row) -> rusqlite::Result<Self> {
        Ok(Self {
            id: row.get("id")?,
            name: row.get("name")?,
            price: row.get("price")?,
            cost: row.get("cost")?,
            photo_path: row.get("photo_path")?,
            status: row.get("status")?,
            stock: row.get("stock")?,
            created_at: row.get("created_at")?,
            updated_at: row.get("updated_at")?,
        })
    }

    /// Equivalente PostgreSQL de `from_row` (issue #9, Fase 2). `stock` es
    /// `INTEGER` (4 bytes) en Postgres, por lo que `sqlx` lo decodifica como
    /// `i32`; se castea a `i64` para que el modelo sea idéntico en ambos
    /// backends. Ver `crate::db::format_pg_timestamp` para el porqué del
    /// formateo de `created_at`/`updated_at`.
    pub fn from_pg_row(row: &sqlx::postgres::PgRow) -> crate::error::AppResult<Self> {
        use sqlx::Row;
        let created_at: time::OffsetDateTime = row.try_get("created_at")?;
        let updated_at: time::OffsetDateTime = row.try_get("updated_at")?;
        Ok(Self {
            id: row.try_get("id")?,
            name: row.try_get("name")?,
            price: row.try_get("price")?,
            cost: row.try_get("cost")?,
            photo_path: row.try_get("photo_path")?,
            status: row.try_get("status")?,
            stock: row.try_get("stock")?,
            created_at: crate::db::format_pg_timestamp(created_at),
            updated_at: crate::db::format_pg_timestamp(updated_at),
        })
    }
}
