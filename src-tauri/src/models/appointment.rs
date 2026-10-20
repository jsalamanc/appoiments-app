use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Appointment {
    pub id: String,
    pub employee_id: String,
    pub customer_id: String,
    pub service_id: Option<String>,
    pub service_name_snapshot: String,
    pub starts_at: String,
    pub ends_at: String,
    pub status: String,
    pub notes: Option<String>,
    pub sale_id: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

impl Appointment {
    pub fn from_row(row: &rusqlite::Row) -> rusqlite::Result<Self> {
        Ok(Self {
            id: row.get("id")?,
            employee_id: row.get("employee_id")?,
            customer_id: row.get("customer_id")?,
            service_id: row.get("service_id")?,
            service_name_snapshot: row.get("service_name_snapshot")?,
            starts_at: row.get("starts_at")?,
            ends_at: row.get("ends_at")?,
            status: row.get("status")?,
            notes: row.get("notes")?,
            sale_id: row.get("sale_id")?,
            created_at: row.get("created_at")?,
            updated_at: row.get("updated_at")?,
        })
    }
}
