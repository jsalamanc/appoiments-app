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
}
