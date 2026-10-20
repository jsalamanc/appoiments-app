use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Product {
    pub id: String,
    pub name: String,
    pub price: f64,
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
            photo_path: row.get("photo_path")?,
            status: row.get("status")?,
            stock: row.get("stock")?,
            created_at: row.get("created_at")?,
            updated_at: row.get("updated_at")?,
        })
    }
}
