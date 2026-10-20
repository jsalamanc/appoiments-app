use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Customer {
    pub id: String,
    pub name: String,
    pub phone: String,
    pub email: String,
    pub document: String,
    pub created_at: String,
    pub updated_at: String,
}

impl Customer {
    pub fn from_row(row: &rusqlite::Row) -> rusqlite::Result<Self> {
        Ok(Self {
            id: row.get("id")?,
            name: row.get("name")?,
            phone: row.get("phone")?,
            email: row.get("email")?,
            document: row.get("document")?,
            created_at: row.get("created_at")?,
            updated_at: row.get("updated_at")?,
        })
    }
}
