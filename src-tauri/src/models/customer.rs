use serde::{Deserialize, Serialize};

/// Issue #2, Fase 5: `email`/`document` son opcionales — una venta rápida
/// (cliente de paso) no debe obligar a inventar un correo o documento solo
/// para satisfacer un campo requerido que no aporta nada en ese caso. La
/// unicidad (`customers_email_unique`/`customers_document_unique`) solo
/// aplica entre los valores que sí están presentes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Customer {
    pub id: String,
    pub name: String,
    pub phone: String,
    pub email: Option<String>,
    pub document: Option<String>,
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
